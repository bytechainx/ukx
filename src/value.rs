//! ukx 的基础值对象：日期、期间、频率、源侧单位与校验入口。
//!
//! 本层只需要「日 / 月 / 季 / 年」的**身份**，不需要时区、夏令时、算术或格式化，
//! 因此**不引入** `chrono` / `time`（工作区契约 `source-library-contract` §2.2）。
//!
//! 子模块承载源登记与观测值对象（门面只做声明与重导出）：
//!
//! - [`source`]：S01–S11 源登记、S07 曲线路由、IADB 关键码与规划调度；
//! - [`observation`]：序列标识、观测值对象与缺失取值表达。

mod observation;
mod source;

pub use observation::{
    validate_observation, UkCbAbsence, UkCbObservation, UkCbSeriesId, UkCbValue,
};
pub use source::{
    guard_curve, iadb_code, CurveRoute, UkCbFormat, UkCbIadbCode, UkCbScheduleWindow, UkCbSourceId,
    UkCbSourcePlan, CURVE_ROUTE, IADB_KEY_CODES, PLANNED_SCHEDULE,
    PLANNED_SCHEDULE_IS_NOT_ACCESS_CONTRACT, SOURCE_ID, SOURCE_PLANS,
};

use crate::error::{UkCbError, UkCbResult};

/// 严格 ISO 日历日期 `YYYY-MM-DD`（月 / 日两位补零）。
///
/// **接受**：`2026-09-18`。
/// **拒绝**：`2026-9-18`（未补零）、`2026/09/18`（分隔符）、带时间部分者。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// 年（可负，按来源原样保留）。
    pub year: i16,
    /// 月，1–12。
    pub month: u8,
    /// 日，按月份与闰年取值。
    pub day: u8,
}

impl Date {
    /// 构造并校验一个日期。
    ///
    /// # Errors
    ///
    /// 月不在 1–12、或日超出该月（含闰年二月）天数时返回 [`UkCbError::Invalid`]。
    pub fn new(year: i16, month: u8, day: u8) -> UkCbResult<Self> {
        let date = Self { year, month, day };
        date.validate()?;
        Ok(date)
    }

    /// 严格解析 `YYYY-MM-DD`。
    ///
    /// # Examples
    ///
    /// ```
    /// use ukx::{Date, UkCbErrorKind};
    ///
    /// let date = Date::parse("2026-09-18")?;
    /// assert_eq!((date.year, date.month, date.day), (2026, 9, 18));
    /// assert_eq!(
    ///     Date::parse("2026-9-18").unwrap_err().kind(),
    ///     UkCbErrorKind::Invalid
    /// );
    /// # Ok::<(), ukx::UkCbError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// 非 `YYYY-MM-DD` 形态或字段越界时返回 [`UkCbError::Invalid`]。
    pub fn parse(input: &str) -> UkCbResult<Self> {
        let bytes = input.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(UkCbError::Invalid(format!(
                "日期须为严格 YYYY-MM-DD 形态，收到 {input:?}"
            )));
        }
        let year = parse_fixed_digits(&bytes[0..4], "年")?;
        let month = u8::try_from(parse_fixed_digits(&bytes[5..7], "月")?)
            .map_err(|_| UkCbError::Invalid("月超出取值域".to_owned()))?;
        let day = u8::try_from(parse_fixed_digits(&bytes[8..10], "日")?)
            .map_err(|_| UkCbError::Invalid("日超出取值域".to_owned()))?;
        Self::new(year, month, day)
    }

    /// 校验月范围、日范围与闰年。
    ///
    /// # Errors
    ///
    /// 月不在 1–12、日不在 1–该月天数（含闰年二月）时返回 [`UkCbError::Invalid`]。
    pub fn validate(&self) -> UkCbResult<()> {
        if self.month < 1 || self.month > 12 {
            return Err(UkCbError::Invalid(format!(
                "月须在 1–12，收到 {}",
                self.month
            )));
        }
        let max = days_in_month(self.year, self.month);
        if self.day < 1 || self.day > max {
            return Err(UkCbError::Invalid(format!(
                "日须在 1–{max}（{} 年 {} 月），收到 {}",
                self.year, self.month, self.day
            )));
        }
        Ok(())
    }

    /// 是否闰年（公历规则：4 年一闰，100 年不闰，400 年再闰）。
    #[must_use]
    pub fn is_leap_year(year: i16) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// 取两位 / 四位定长十进制数字（**要求补零**）。
fn parse_fixed_digits(bytes: &[u8], label: &str) -> UkCbResult<i16> {
    let mut value: i16 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return Err(UkCbError::Invalid(format!("{label}须为定长补零十进制数字")));
        }
        value = value * 10 + i16::from(b - b'0');
    }
    Ok(value)
}

/// 某年某月的天数。
fn days_in_month(year: i16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if Date::is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// 业务期间。**不得**用 `String` 顶替本类型。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Period {
    /// 日粒度：`YYYY-MM-DD`。
    Day(Date),
    /// 月粒度。
    Month {
        /// 年。
        year: i16,
        /// 月，1–12。
        month: u8,
    },
    /// 季粒度。
    Quarter {
        /// 年。
        year: i16,
        /// 季，1–4。
        quarter: u8,
    },
    /// 年粒度。
    Year(i16),
    /// 事件期间：以事件发生日标识，**不**代表日频序列。
    Event {
        /// 事件发生日。
        date: Date,
    },
}

impl Period {
    /// 校验期间取值域（月 1–12、季 1–4、日按月份与闰年）。
    ///
    /// # Errors
    ///
    /// 任一字段越界时返回 [`UkCbError::Invalid`]。
    pub fn validate(&self) -> UkCbResult<()> {
        match *self {
            Self::Day(date) | Self::Event { date } => date.validate(),
            Self::Month { month, .. } if (1..=12).contains(&month) => Ok(()),
            Self::Month { month, .. } => {
                Err(UkCbError::Invalid(format!("月须在 1–12，收到 {month}")))
            }
            Self::Quarter { quarter, .. } if (1..=4).contains(&quarter) => Ok(()),
            Self::Quarter { quarter, .. } => {
                Err(UkCbError::Invalid(format!("季须在 1–4，收到 {quarter}")))
            }
            Self::Year(_) => Ok(()),
        }
    }
}

/// 解析本层声明的期间形态：`YYYY-MM-DD` / `YYYY-MM` / `YYYY-Qn` / `YYYY` / `event:YYYY-MM-DD`。
///
/// 这四种形态是**本层的离线交换形态**（见 `docs/标准.md`），不是 BoE 文件布局；
/// 形态之外一律拒绝，**不猜**。
///
/// # Errors
///
/// 形态不匹配或字段越界时返回 [`UkCbError::Invalid`]。
pub fn parse_period(input: &str) -> UkCbResult<Period> {
    let bytes = input.as_bytes();
    if let Some(rest) = input.strip_prefix("event:") {
        return Ok(Period::Event {
            date: Date::parse(rest)?,
        });
    }
    if bytes.len() == 10 {
        return Ok(Period::Day(Date::parse(input)?));
    }
    if bytes.len() == 7 {
        if bytes[4] == b'-' && bytes[5] == b'Q' {
            let quarter = u8::try_from(parse_fixed_digits(&bytes[6..7], "季")?)
                .map_err(|_| UkCbError::Invalid("季超出取值域".to_owned()))?;
            let year = parse_fixed_digits(&bytes[0..4], "年")?;
            let period = Period::Quarter { year, quarter };
            period.validate()?;
            return Ok(period);
        }
        if bytes[4] == b'-' {
            let year = parse_fixed_digits(&bytes[0..4], "年")?;
            let month = u8::try_from(parse_fixed_digits(&bytes[5..7], "月")?)
                .map_err(|_| UkCbError::Invalid("月超出取值域".to_owned()))?;
            let period = Period::Month { year, month };
            period.validate()?;
            return Ok(period);
        }
    }
    if bytes.len() == 4 {
        return Ok(Period::Year(parse_fixed_digits(&bytes[0..4], "年")?));
    }
    Err(UkCbError::Invalid(format!(
        "期间形态须为 YYYY-MM-DD / YYYY-MM / YYYY-Qn / YYYY / event:YYYY-MM-DD，收到 {input:?}"
    )))
}

/// 频率。取值为契约固定的七个。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frequency {
    /// 日频。
    Daily,
    /// 周频。
    Weekly,
    /// 月频。
    Monthly,
    /// 季频。
    Quarterly,
    /// 年频。
    Annual,
    /// 事件驱动。
    Event,
    /// 不规则 / 混合。
    Irregular,
}

impl Frequency {
    /// 稳定字符串形式（用于离线交换形态与跨库对齐）。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Quarterly => "quarterly",
            Self::Annual => "annual",
            Self::Event => "event",
            Self::Irregular => "irregular",
        }
    }

    /// 解析稳定字符串形式。
    ///
    /// # Errors
    ///
    /// 不在七个取值内时返回 [`UkCbError::NotApplicable`]（**不**做近义推断）。
    pub fn parse(input: &str) -> UkCbResult<Self> {
        match input {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "monthly" => Ok(Self::Monthly),
            "quarterly" => Ok(Self::Quarterly),
            "annual" => Ok(Self::Annual),
            "event" => Ok(Self::Event),
            "irregular" => Ok(Self::Irregular),
            _ => Err(UkCbError::NotApplicable(format!(
                "频率 {input:?} 不在契约的七个取值内"
            ))),
        }
    }
}

/// 源侧单位。
///
/// 清单**未**逐序列固定单位 ⇒ 本层只做保留与最小具名表达，
/// **不推断**、**不换算**（换算归下游 Normalize）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UkCbUnit {
    /// 源未声明单位（本层不推断）。
    Unspecified,
    /// 指标原生单位，值随序列而异（本层原样保留）。
    Native,
    /// 百分比：仅由调用方在离线形态中**显式声明**时使用，本层不据序列名推断。
    Percent,
}

impl UkCbUnit {
    /// 稳定字符串形式。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::Native => "native",
            Self::Percent => "percent",
        }
    }

    /// 解析稳定字符串形式。
    ///
    /// # Errors
    ///
    /// 不在三个取值内时返回 [`UkCbError::SemanticallyRejected`]。
    pub fn parse(input: &str) -> UkCbResult<Self> {
        match input {
            "unspecified" => Ok(Self::Unspecified),
            "native" => Ok(Self::Native),
            "percent" => Ok(Self::Percent),
            _ => Err(UkCbError::SemanticallyRejected(format!(
                "单位 {input:?} 不在本层声明的三个取值内"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_period, Date, Frequency, Period, UkCbUnit};
    use crate::error::UkCbErrorKind;

    #[test]
    fn date_parse_is_strict_and_checks_the_calendar() {
        assert_eq!(Date::parse("2026-09-18").expect("合法").month, 9);
        for bad in ["2026-9-18", "2026/09/18", "2026-02-30"] {
            assert_eq!(
                Date::parse(bad).expect_err(bad).kind(),
                UkCbErrorKind::Invalid
            );
        }
        assert!(Date::is_leap_year(2000) && !Date::is_leap_year(1900));
    }

    #[test]
    fn period_forms_are_declared_and_bounded() {
        assert_eq!(
            parse_period("2026-09-18").expect("日"),
            Period::Day(Date::parse("2026-09-18").expect("日期"))
        );
        assert_eq!(parse_period("2026").expect("年"), Period::Year(2026));
        assert_eq!(
            parse_period("event:2026-09-18").expect("事件"),
            Period::Event {
                date: Date::parse("2026-09-18").expect("日期")
            }
        );
        for bad in ["2026-Q5", "2026/09/18", "26-09"] {
            assert_eq!(
                parse_period(bad).expect_err(bad).kind(),
                UkCbErrorKind::Invalid
            );
        }
    }

    #[test]
    fn period_validate_rejects_out_of_range_fields() {
        assert!(Period::Month {
            year: 2026,
            month: 12
        }
        .validate()
        .is_ok());
        assert!(Period::Quarter {
            year: 2026,
            quarter: 5
        }
        .validate()
        .is_err());
    }

    #[test]
    fn frequency_and_unit_round_trip() {
        for frequency in [Frequency::Daily, Frequency::Event, Frequency::Irregular] {
            assert_eq!(
                Frequency::parse(frequency.as_str()).expect("可回读"),
                frequency
            );
        }
        for unit in [UkCbUnit::Unspecified, UkCbUnit::Native, UkCbUnit::Percent] {
            assert_eq!(UkCbUnit::parse(unit.as_str()).expect("可回读"), unit);
        }
        assert_eq!(
            UkCbUnit::parse("billions").expect_err("未知单位").kind(),
            UkCbErrorKind::SemanticallyRejected
        );
    }
}
