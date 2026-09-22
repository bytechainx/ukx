#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]

//! ukx —— 英国央行数据源（`source_id = uk_cb`）的**源事实层**：
//! 源登记与 IADB 关键码类型化、离线解析、曲线路由与 fail-closed 授权判定。
//!
//! ## 能力
//!
//! | 能力 | 类型 / 入口 | 状态 |
//! |------|-------------|------|
//! | 源登记 | [`SOURCE_PLANS`]（S01–S11，含清单「形式」「频率」逐字文本） | 已落地 |
//! | 关键 IADB 码 | [`UkCbIadbCode`] / [`IADB_KEY_CODES`] / [`iadb_code`] | 已落地（严格 6 个种子） |
//! | 曲线路由 | [`guard_curve`] / [`CURVE_ROUTE`] | 已落地（S07 → `yield_curve`） |
//! | 规划调度 | [`PLANNED_SCHEDULE`] | 已落地（**规划语义**，非访问合同） |
//! | 观测值对象 | [`UkCbObservation`] / [`UkCbValue`] / [`UkCbSeriesId`] | 已落地 |
//! | 离线解析 | [`parse_uk_cb_observations`] | 仅 CSV 与 JSON 两种形态 |
//! | 授权判定 | [`authorize`] / [`ensure_authorized`] | fail-closed |
//! | publication 语义 | [`publication_semantics`] | 恒 `Date + Inferred + NotEligible` |
//! | xlsx / ZIP / XML / Excel 解析 | — | **未落地**（返回 `NotApplicable`） |
//! | 联网采集 | — | **未实现**（无授权，`FR-037`） |
//!
//! ## 责任边界
//!
//! 本库做：把清单声明的源、码、路由禁则与调度落成**可编译、可测、可判定**的代码；
//! 对离线字符串做严格解析；对「该产品是否属于本域」给出可判定的结论。
//!
//! 本库**不做**：联网采集、代理 / 会话管理、凭据注入、存储与再分发、派生指标。
//!
//! ## 非目标
//!
//! 不猜端点、Header、限流数字或 Series 全集；不实现 xlsx / ZIP / XML / Excel 解析栈；
//! 不把 S07 收益率曲线洗白为普通观测；不实现任何采集调度。
//!
//! ## 诚实边界
//!
//! `production_decision = NO-GO`；清单 COMPLETE ≠ ship；authorization ≠ Production Ready。
//! 本源授权为 `unknown`（无 Owner 签核文件），因此：
//!
//! - 授权判定默认拒绝（[`authorize`] 的 fail-closed 落点）；
//! - 解析器**只接受**显式标注的合成样本（`_synthetic: true` + 非空 `_note`）；
//! - 清单未点名格式的源（S02 / S06 / S08 / S09）与未落地的格式一律 `NotApplicable`，
//!   **不假装已支持**。
//!
//! # 最小示例
//!
//! ```
//! use ukx::{parse_uk_cb_observations, UkCbSourceId};
//!
//! let input = "# _synthetic: true\n# _note: 合成样本，非真实源数据\n\
//!              series,period,value,unit,frequency,revision\n\
//!              IUDBEDR,2026-09-18,4.00,percent,daily,\n";
//! let observations = parse_uk_cb_observations(UkCbSourceId::S01, input)?;
//! assert_eq!(observations[0].series.as_str(), "IUDBEDR");
//! assert_eq!(observations[0].value.present(), Some(4.0));
//! # Ok::<(), ukx::UkCbError>(())
//! ```

pub mod authz;
pub mod error;
pub mod parse;
pub mod pit;
pub mod value;

pub use authz::{
    authorize, ensure_authorized, UkCbAuthorization, UkCbAuthorizationEvidence,
    AUTHORIZATION_EVIDENCE, AUTHORIZATION_STATUS,
};
pub use error::{UkCbError, UkCbErrorKind, UkCbResult};
pub use parse::parse_uk_cb_observations;
pub use pit::{
    is_formal_pit_eligible, publication_semantics, AvailabilityEvidence, PitEligibility,
    TimePrecision,
};
pub use value::{
    guard_curve, iadb_code, parse_period, validate_observation, CurveRoute, Date, Frequency,
    Period, UkCbAbsence, UkCbFormat, UkCbIadbCode, UkCbObservation, UkCbScheduleWindow,
    UkCbSeriesId, UkCbSourceId, UkCbSourcePlan, UkCbUnit, UkCbValue, CURVE_ROUTE, IADB_KEY_CODES,
    PLANNED_SCHEDULE, PLANNED_SCHEDULE_IS_NOT_ACCESS_CONTRACT, SOURCE_ID, SOURCE_PLANS,
};
