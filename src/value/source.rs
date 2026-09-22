//! S01–S11 源登记、S07 曲线路由、IADB 关键码与规划调度。
//!
//! 本模块的**源事实**来自 `specs/adapter/uk_cb.md` §1.1 / §1.2 / §1.3 / §1.4。
//! 清单**未**固定的内容（端点、Series 全集、UA、限流、许可）一律不在此编造：
//! 「形式」列未点名格式的源（S02 / S06 / S08 / S09）在离线解析中返回
//! [`NotApplicable`](crate::UkCbErrorKind::NotApplicable)。

/// 本源标识。观测的源标识须等于本值。
pub const SOURCE_ID: &str = "uk_cb";

/// 规划调度**不是**访问合同。
///
/// `PLANNED_SCHEDULE` 的时刻取自清单 §1.4 的「规划 UTC」段，仅用于说明规划意图；
/// 本层无网络、无授权，故任何时刻都**不得**被当作访问时点、抓取窗口或 SLA 承诺。
pub const PLANNED_SCHEDULE_IS_NOT_ACCESS_CONTRACT: bool = true;

/// 清单声明的源侧形式。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UkCbFormat {
    /// CSV。
    Csv,
    /// XML。
    Xml,
    /// Excel（`.xls` 一类工作簿）。
    Excel,
    /// JSON。
    Json,
    /// xlsx 工作簿。
    Xlsx,
    /// ZIP 归档。
    Zip,
}

impl UkCbFormat {
    /// 稳定字符串形式。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Xml => "xml",
            Self::Excel => "excel",
            Self::Json => "json",
            Self::Xlsx => "xlsx",
            Self::Zip => "zip",
        }
    }

    /// 本层离线解析**已实现**的格式：仅 CSV 与 JSON。
    ///
    /// 其余格式（xlsx / ZIP / XML / Excel）的解析栈**未落地**，
    /// 调用方会得到 `NotApplicable` 而不是「假装已支持」的结果。
    #[must_use]
    pub fn is_offline_implemented(self) -> bool {
        matches!(self, Self::Csv | Self::Json)
    }
}

/// 规划源标识（S01–S11）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UkCbSourceId {
    /// S01 IADB（批量序列）。
    S01,
    /// S02 Bank Rate（`IUDBEDR` + 官网）。
    S02,
    /// S03 Bank Return（资产负债表）。
    S03,
    /// S04 APF（Gilt / 企债 / QT）。
    S04,
    /// S05 市场操作（STR / ILTR / 美元回购）。
    S05,
    /// S06 SONIA（`IUDSOIA` + Compounded）。
    S06,
    /// S07 收益率曲线（nominal / real / OIS）。
    S07,
    /// S08 官方汇率（`XUDL*`）。
    S08,
    /// S09 货币信贷。
    S09,
    /// S10 DMO Gilt（拍卖 / 发行）。
    S10,
    /// S11 ONS CPI / RPI（辅助）。
    S11,
}

impl UkCbSourceId {
    /// 源标识字符串。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::S01 => "S01",
            Self::S02 => "S02",
            Self::S03 => "S03",
            Self::S04 => "S04",
            Self::S05 => "S05",
            Self::S06 => "S06",
            Self::S07 => "S07",
            Self::S08 => "S08",
            Self::S09 => "S09",
            Self::S10 => "S10",
            Self::S11 => "S11",
        }
    }

    /// 是否为**收益率曲线**源（S07）。
    ///
    /// 曲线产品 MUST 派往 `yield_curve` 槽位，**不得**直写观测（清单 §1.3）。
    #[must_use]
    pub fn is_curve(self) -> bool {
        matches!(self, Self::S07)
    }

    /// 该源的登记条目。
    #[must_use]
    pub fn plan(self) -> UkCbSourcePlan {
        let index = match self {
            Self::S01 => 0,
            Self::S02 => 1,
            Self::S03 => 2,
            Self::S04 => 3,
            Self::S05 => 4,
            Self::S06 => 5,
            Self::S07 => 6,
            Self::S08 => 7,
            Self::S09 => 8,
            Self::S10 => 9,
            Self::S11 => 10,
        };
        SOURCE_PLANS[index]
    }
}

/// 一个规划源的登记条目。
///
/// `declared_form` / `declared_frequency` 是清单「形式」「频率」两列的**逐字文本**
/// （不解释、不改写），`formats` 只装清单**明确点名**的格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UkCbSourcePlan {
    /// 源标识。
    pub id: UkCbSourceId,
    /// 清单给的源名称。
    pub name: &'static str,
    /// 清单「形式」列的逐字文本。
    pub declared_form: &'static str,
    /// 清单明确点名的格式（可能为空，也可能两项）。
    pub formats: &'static [UkCbFormat],
    /// 清单「频率」列的逐字文本。
    pub declared_frequency: &'static str,
}

/// S01–S11 源登记表（清单 §1.1 的逐条落点）。
pub const SOURCE_PLANS: [UkCbSourcePlan; 11] = [
    UkCbSourcePlan {
        id: UkCbSourceId::S01,
        name: "IADB",
        declared_form: "CSV/XML",
        formats: &[UkCbFormat::Csv, UkCbFormat::Xml],
        declared_frequency: "日/月/季",
    },
    UkCbSourcePlan {
        id: UkCbSourceId::S02,
        name: "Bank Rate `IUDBEDR`",
        declared_form: "事件",
        formats: &[],
        declared_frequency: "事件",
    },
    UkCbSourcePlan {
        id: UkCbSourceId::S03,
        name: "Bank Return",
        declared_form: "CSV/Excel",
        formats: &[UkCbFormat::Csv, UkCbFormat::Excel],
        declared_frequency: "周",
    },
    UkCbSourcePlan {
        id: UkCbSourceId::S04,
        name: "APF",
        declared_form: "CSV",
        formats: &[UkCbFormat::Csv],
        declared_frequency: "周/操作日",
    },
    UkCbSourcePlan {
        id: UkCbSourceId::S05,
        name: "市场操作",
        declared_form: "CSV",
        formats: &[UkCbFormat::Csv],
        declared_frequency: "操作日",
    },
    UkCbSourcePlan {
        id: UkCbSourceId::S06,
        name: "SONIA `IUDSOIA`",
        declared_form: "日 T+1",
        formats: &[],
        declared_frequency: "日",
    },
    UkCbSourcePlan {
        id: UkCbSourceId::S07,
        name: "收益率曲线 ZIP",
        declared_form: "ZIP",
        formats: &[UkCbFormat::Zip, UkCbFormat::Xlsx],
        declared_frequency: "日",
    },
    UkCbSourcePlan {
        id: UkCbSourceId::S08,
        name: "汇率 XUDL*",
        declared_form: "日",
        formats: &[],
        declared_frequency: "日",
    },
    UkCbSourcePlan {
        id: UkCbSourceId::S09,
        name: "货币信贷",
        declared_form: "月",
        formats: &[],
        declared_frequency: "月",
    },
    UkCbSourcePlan {
        id: UkCbSourceId::S10,
        name: "DMO Gilt",
        declared_form: "CSV",
        formats: &[UkCbFormat::Csv],
        declared_frequency: "日/事件",
    },
    UkCbSourcePlan {
        id: UkCbSourceId::S11,
        name: "ONS CPI/RPI",
        declared_form: "JSON",
        formats: &[UkCbFormat::Json],
        declared_frequency: "月",
    },
];

/// 清单 §1.2 的 6 个关键 IADB 码（规划**白名单种子**）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UkCbIadbCode {
    /// Official Bank Rate。
    Iudbedr,
    /// SONIA。
    Iudsoia,
    /// 准备金余额（周）。
    Rpwb55a,
    /// M4 存量。
    Lpmauyn,
    /// GBP/USD。
    Xudluss,
    /// 英镑有效汇率。
    Xudleri,
}

impl UkCbIadbCode {
    /// 清单使用的码字符串。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Iudbedr => "IUDBEDR",
            Self::Iudsoia => "IUDSOIA",
            Self::Rpwb55a => "RPWB55A",
            Self::Lpmauyn => "LPMAUYN",
            Self::Xudluss => "XUDLUSS",
            Self::Xudleri => "XUDLERI",
        }
    }

    /// 按清单 §1.2 的**6 个种子**严格选择关键码。
    ///
    /// 大小写敏感、不做近义推断；不在种子内者按语义拒绝 ——
    /// 清单未钉死的全集**不得**由本层自行扩充。
    ///
    /// # Errors
    ///
    /// 不在 6 个种子内时返回 [`UkCbError::SemanticallyRejected`](crate::UkCbError::SemanticallyRejected)。
    pub fn from_code(code: &str) -> crate::UkCbResult<Self> {
        for candidate in IADB_KEY_CODES {
            if candidate.as_str() == code {
                return Ok(candidate);
            }
        }
        Err(crate::UkCbError::SemanticallyRejected(format!(
            "IADB 码 {code:?} 不在清单 §1.2 的 6 个关键种子内；本层不得自行扩集"
        )))
    }
}

/// 6 个关键 IADB 码（与清单 §1.2 逐行对应）。
pub const IADB_KEY_CODES: [UkCbIadbCode; 6] = [
    UkCbIadbCode::Iudbedr,
    UkCbIadbCode::Iudsoia,
    UkCbIadbCode::Rpwb55a,
    UkCbIadbCode::Lpmauyn,
    UkCbIadbCode::Xudluss,
    UkCbIadbCode::Xudleri,
];

/// 关键 IADB 码选择守卫（[`UkCbIadbCode::from_code`] 的自由函数形式）。
///
/// # Errors
///
/// 不在 6 个种子内时返回 [`UkCbError::SemanticallyRejected`](crate::UkCbError::SemanticallyRejected)。
pub fn iadb_code(code: &str) -> crate::UkCbResult<UkCbIadbCode> {
    UkCbIadbCode::from_code(code)
}

/// 曲线路由目标（清单 §1.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurveRoute {
    /// 目标槽位。
    pub slot: &'static str,
    /// 该槽位下清单点名的 provider 标识。
    pub providers: [&'static str; 2],
}

/// 曲线路由登记：S07 收益率曲线 → `yield_curve`（#11 `boe_iadb` / #12 `uk_dmo`）。
pub const CURVE_ROUTE: CurveRoute = CurveRoute {
    slot: "yield_curve",
    providers: ["boe_iadb", "uk_dmo"],
};

/// 曲线路由守卫：曲线源 MUST 被拒绝并指向 `yield_curve`，**不得**直写观测。
///
/// # Errors
///
/// `source` 为 [`UkCbSourceId::S07`] 时返回
/// [`RoutedElsewhere`](crate::UkCbErrorKind::RoutedElsewhere)。
pub fn guard_curve(source: UkCbSourceId) -> crate::UkCbResult<()> {
    if source.is_curve() {
        return Err(crate::UkCbError::RoutedElsewhere(format!(
            "{} 收益率曲线归 {}（#11 {} / #12 {}），不得直写观测",
            source.as_str(),
            CURVE_ROUTE.slot,
            CURVE_ROUTE.providers[0],
            CURVE_ROUTE.providers[1]
        )));
    }
    Ok(())
}

/// 规划调度窗口（**UTC 规划时刻**，非访问合同）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UkCbScheduleWindow {
    /// 用途标签。
    pub label: &'static str,
    /// 规划 UTC 时刻（`HH:MM`）；清单只写「日历驱动」者无固定时刻。
    pub utc_time: Option<&'static str>,
    /// 清单 §1.4 调度段的逐字文本。
    pub declared: &'static str,
}

/// 清单 §1.4 的规划调度（**规划语义**，见 [`PLANNED_SCHEDULE_IS_NOT_ACCESS_CONTRACT`]）。
pub const PLANNED_SCHEDULE: [UkCbScheduleWindow; 8] = [
    UkCbScheduleWindow {
        label: "SONIA 首次发布",
        utc_time: Some("08:05"),
        declared: "SONIA 08:05/08:35",
    },
    UkCbScheduleWindow {
        label: "SONIA 复核",
        utc_time: Some("08:35"),
        declared: "SONIA 08:05/08:35",
    },
    UkCbScheduleWindow {
        label: "IADB 日度",
        utc_time: Some("16:30"),
        declared: "IADB 日度 16:30",
    },
    UkCbScheduleWindow {
        label: "Bank Return 周四轮询",
        utc_time: Some("15:05"),
        declared: "Bank Return 周四 15:05 轮询",
    },
    UkCbScheduleWindow {
        label: "APF 操作日",
        utc_time: Some("14:35"),
        declared: "APF 操作日 14:35",
    },
    UkCbScheduleWindow {
        label: "收益率曲线",
        utc_time: Some("18:00"),
        declared: "曲线 18:00",
    },
    UkCbScheduleWindow {
        label: "月度日历驱动",
        utc_time: None,
        declared: "月度日历驱动",
    },
    UkCbScheduleWindow {
        label: "补采校验",
        utc_time: Some("02:00"),
        declared: "02:00 补采校验",
    },
];

#[cfg(test)]
mod tests {
    use super::{
        guard_curve, iadb_code, CurveRoute, UkCbFormat, UkCbIadbCode, UkCbSourceId, CURVE_ROUTE,
        IADB_KEY_CODES, PLANNED_SCHEDULE, SOURCE_ID, SOURCE_PLANS,
    };
    use crate::error::UkCbErrorKind;

    #[test]
    fn source_registry_matches_the_manifest() {
        assert_eq!(SOURCE_ID, "uk_cb");
        assert_eq!(SOURCE_PLANS.len(), 11);
        for plan in SOURCE_PLANS {
            assert_eq!(plan.id.plan(), plan);
        }
        assert_eq!(SOURCE_PLANS[0].declared_form, "CSV/XML");
        assert!(SOURCE_PLANS[1].formats.is_empty(), "S02 未点名格式");
        assert_eq!(SOURCE_PLANS[10].formats, &[UkCbFormat::Json]);
    }

    #[test]
    fn format_implementation_is_limited_to_csv_and_json() {
        assert!(UkCbFormat::Csv.is_offline_implemented());
        assert!(UkCbFormat::Json.is_offline_implemented());
        for format in [
            UkCbFormat::Xml,
            UkCbFormat::Excel,
            UkCbFormat::Xlsx,
            UkCbFormat::Zip,
        ] {
            assert!(!format.is_offline_implemented(), "{}", format.as_str());
        }
    }

    #[test]
    fn key_codes_are_exactly_the_six_seeds() {
        assert_eq!(IADB_KEY_CODES.len(), 6);
        assert_eq!(IADB_KEY_CODES[0].as_str(), "IUDBEDR");
        assert_eq!(IADB_KEY_CODES[5].as_str(), "XUDLERI");
        assert_eq!(
            iadb_code("IUDBEDX").expect_err("非种子码").kind(),
            UkCbErrorKind::SemanticallyRejected
        );
        assert_eq!(
            iadb_code("iudbedr").expect_err("大小写敏感").kind(),
            UkCbErrorKind::SemanticallyRejected
        );
        assert_eq!(UkCbIadbCode::Lpmauyn.as_str(), "LPMAUYN");
    }

    #[test]
    fn curve_is_routed_and_schedule_is_planned_only() {
        assert!(UkCbSourceId::S07.is_curve());
        assert_eq!(
            guard_curve(UkCbSourceId::S07)
                .expect_err("曲线须路由")
                .kind(),
            UkCbErrorKind::RoutedElsewhere
        );
        assert!(guard_curve(UkCbSourceId::S01).is_ok());
        assert_eq!(
            CURVE_ROUTE,
            CurveRoute {
                slot: "yield_curve",
                providers: ["boe_iadb", "uk_dmo"]
            }
        );
        assert_eq!(PLANNED_SCHEDULE.len(), 8);
        assert_eq!(PLANNED_SCHEDULE[6].utc_time, None, "日历驱动无固定时刻");
    }
}
