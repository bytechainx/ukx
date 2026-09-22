//! 序列标识、观测值对象与缺失取值的具名表达。
//!
//! 观测值对象的形状遵循 `contracts/source-library-contract.md` §2.1：
//! 源条目标识 + 业务期间 + 值 + 源侧单位 + 频率 + 修订标识。
//!
//! **禁止**在本模块实现派生公式（净流动性、利差、Credit Impulse、z-score 等）；
//! 也**禁止**把缺失值静默转成 0 —— 缺失由 [`UkCbValue::Absent`] 具名表达。

use super::{Frequency, Period, UkCbIadbCode, UkCbUnit};
use crate::error::{UkCbError, UkCbResult};

/// 序列标识。
///
/// **原样保留**源侧标识（含清单 §1.2 的 6 个关键码与其余源声明的标识）；
/// 本层不解释其含义、不映射、**不扩集**。需要用关键码判定时走
/// [`UkCbIadbCode::from_code`]（严格 6 个种子）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UkCbSeriesId(String);

impl UkCbSeriesId {
    /// 构造并校验序列标识。
    ///
    /// 允许的字符：ASCII 字母、数字、`_`、`.`、`-`；其余一律拒绝。
    ///
    /// # Errors
    ///
    /// 为空 / 全空白 / 含非允许字符时返回 [`UkCbError::Invalid`]。
    pub fn parse(text: &str) -> UkCbResult<Self> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(UkCbError::Missing("序列标识".to_owned()));
        }
        if !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
        {
            return Err(UkCbError::Invalid(format!(
                "序列标识只允许 ASCII 字母 / 数字 / `_` / `.` / `-`，收到 {text:?}"
            )));
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// 标识文本。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 当且仅当本标识属于清单 §1.2 的 6 个关键码时返回该码。
    #[must_use]
    pub fn iadb_code(&self) -> Option<UkCbIadbCode> {
        UkCbIadbCode::from_code(&self.0).ok()
    }
}

impl std::fmt::Display for UkCbSeriesId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 观测取值的**具名**缺失原因。
///
/// 本源清单未固定空值标记的字面量，故本层只区分「源行为空」与「源给出非数值标记」
/// 两类，**不解释**后者的字面含义（不猜「cancelled」「provisional」等语义）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UkCbAbsence {
    /// 源行该字段为空。
    BlankInSource,
    /// 源行给出非数值标记；本层不解释其字面含义。
    ExplicitMarker,
}

/// 观测取值。
///
/// **不得**把缺失静默转 0：缺失必须走 [`UkCbValue::Absent`]。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UkCbValue {
    /// 有值。
    Present(f64),
    /// 缺失，附带具名原因。
    Absent(UkCbAbsence),
}

impl UkCbValue {
    /// 取值；缺失时为 `None`（**不**返回 0）。
    #[must_use]
    pub fn present(self) -> Option<f64> {
        match self {
            Self::Present(value) => Some(value),
            Self::Absent(_) => None,
        }
    }

    /// 是否缺失。
    #[must_use]
    pub fn is_absent(self) -> bool {
        matches!(self, Self::Absent(_))
    }
}

/// 一个源观测（源事实值对象）。
#[derive(Debug, Clone, PartialEq)]
pub struct UkCbObservation {
    /// 序列标识（原样保留）。
    pub series: UkCbSeriesId,
    /// 业务期间。
    pub period: Period,
    /// 取值（含具名缺失原因）。
    pub value: UkCbValue,
    /// 源侧单位。
    pub unit: UkCbUnit,
    /// 频率。
    pub frequency: Frequency,
    /// 修订标识；离线形态未提供时为 `None`（**不得**伪造）。
    pub revision: Option<String>,
}

impl UkCbObservation {
    /// 修订标识。离线形态未提供修订面时恒为 `None`。
    #[must_use]
    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }
}

/// 校验观测的完整性。
///
/// # Errors
///
/// 期间越界、取值为非有限数、频率与期间明显不一致（如日频配年期间）时返回
/// [`UkCbError::Invalid`]；修订标识为空串时返回 [`UkCbError::Missing`]。
pub fn validate_observation(observation: &UkCbObservation) -> UkCbResult<()> {
    observation.period.validate()?;
    if let UkCbValue::Present(value) = observation.value {
        if !value.is_finite() {
            return Err(UkCbError::Invalid(
                "取值为非有限数值（NaN / 无穷）".to_owned(),
            ));
        }
    }
    if let Some(revision) = observation.revision.as_deref() {
        if revision.trim().is_empty() {
            return Err(UkCbError::Missing("修订标识（给出了空串）".to_owned()));
        }
    }
    match (observation.frequency, observation.period) {
        (Frequency::Daily, Period::Day(_) | Period::Event { .. }) => Ok(()),
        (Frequency::Daily, _) => Err(UkCbError::Invalid(
            "日频观测的期间须为日粒度或事件日".to_owned(),
        )),
        (Frequency::Monthly, Period::Month { .. }) => Ok(()),
        (Frequency::Monthly, _) => Err(UkCbError::Invalid("月频观测的期间须为月粒度".to_owned())),
        (Frequency::Quarterly, Period::Quarter { .. }) => Ok(()),
        (Frequency::Quarterly, _) => Err(UkCbError::Invalid("季频观测的期间须为季粒度".to_owned())),
        (Frequency::Annual, Period::Year(_)) => Ok(()),
        (Frequency::Annual, _) => Err(UkCbError::Invalid("年频观测的期间须为年粒度".to_owned())),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_observation, UkCbAbsence, UkCbObservation, UkCbSeriesId, UkCbValue};
    use crate::error::UkCbErrorKind;
    use crate::value::{Date, Frequency, Period, UkCbIadbCode, UkCbUnit};

    fn observation() -> UkCbObservation {
        UkCbObservation {
            series: UkCbSeriesId::parse("IUDBEDR").expect("标识合法"),
            period: Period::Day(Date::parse("2026-09-18").expect("日期")),
            value: UkCbValue::Present(4.0),
            unit: UkCbUnit::Percent,
            frequency: Frequency::Daily,
            revision: None,
        }
    }

    #[test]
    fn series_id_keeps_the_source_text_and_is_bounded() {
        assert_eq!(
            UkCbSeriesId::parse("IUDSOIA").expect("合法").as_str(),
            "IUDSOIA"
        );
        assert_eq!(
            UkCbSeriesId::parse("IUDBEDR").expect("合法").iadb_code(),
            Some(UkCbIadbCode::Iudbedr)
        );
        assert_eq!(
            UkCbSeriesId::parse("ONS.CPI.YOY")
                .expect("合法")
                .iadb_code(),
            None
        );
        assert!(UkCbSeriesId::parse("a b").is_err());
        assert_eq!(
            UkCbSeriesId::parse("  ").expect_err("空标识").kind(),
            UkCbErrorKind::Missing
        );
    }

    #[test]
    fn absence_is_named_and_never_zero() {
        assert_eq!(
            UkCbValue::Absent(UkCbAbsence::BlankInSource).present(),
            None
        );
        assert!(UkCbValue::Absent(UkCbAbsence::ExplicitMarker).is_absent());
        assert!(!UkCbValue::Present(1.5).is_absent());
        assert_ne!(
            UkCbValue::Absent(UkCbAbsence::BlankInSource),
            UkCbValue::Absent(UkCbAbsence::ExplicitMarker)
        );
    }

    #[test]
    fn validation_rejects_non_finite_and_mismatched_frequency() {
        let mut sample = observation();
        sample.value = UkCbValue::Present(f64::NAN);
        assert_eq!(
            validate_observation(&sample).expect_err("NaN").kind(),
            UkCbErrorKind::Invalid
        );
        let mut mismatched = observation();
        mismatched.period = Period::Year(2026);
        assert_eq!(
            validate_observation(&mismatched)
                .expect_err("日频配年期间")
                .kind(),
            UkCbErrorKind::Invalid
        );
        assert!(validate_observation(&observation()).is_ok());
    }

    #[test]
    fn revision_is_absent_until_the_source_provides_one() {
        assert_eq!(observation().revision(), None);
        let mut revised = observation();
        revised.revision = Some("r1".to_owned());
        assert_eq!(revised.revision(), Some("r1"));
        revised.revision = Some("   ".to_owned());
        assert_eq!(
            validate_observation(&revised).expect_err("空修订").kind(),
            UkCbErrorKind::Missing
        );
    }
}
