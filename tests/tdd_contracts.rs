#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! TDD 行为契约（特性 005）。
//!
//! 入口集合 = 本 crate 全部公开入口（含 `validate*` / 判定函数 / 解析器 / 值对象方法）。
//! 入口列按 `specs/features/002-public-api-compliance-and-test-tiers/contracts/public-api-contract.md`
//! 的机器格式书写：只写 `类型::方法` 或裸标识符（自由函数名 / 公开类型名），
//! **不带**参数、返回值与泛型 —— 它同时是该契约的登记基准，检查器按**字符串精确比对**，
//! 且要求标识符为裸形式（`entryIsReal` 只认 `fn <名>` 与
//! `pub struct|enum|trait|type <名>`）。常量类事实（如 `SOURCE_PLANS` / `IADB_KEY_CODES`）
//! 由承载它的公开类型条目（`UkCbSourceId::plan` / `UkCbIadbCode::from_code`）覆盖；
//! 其余常量（如 `SOURCE_ID` / `PLANNED_SCHEDULE` / `CURVE_ROUTE`）由
//! `tests/sdd_spec.rs` 的对应断言覆盖。
//!
//! 下表每个入口先在 `/tmp` 变异副本上观测应红、再在本树观测绿；
//! 变异为一次性探测（未提交脚本），红/绿用例名均在本文件内真实存在。
//!
//! // TDD-PROBE: parse_uk_cb_observations | 变异：不检查「序列 + 期间」重复 | 红=parse_rejects_duplicate_identity | 绿=parse_rejects_duplicate_identity
//! // TDD-PROBE: guard_curve | 变异：is_curve 判定失效，曲线不再被路由 | 红=curve_source_is_routed | 绿=curve_source_is_routed
//! // TDD-PROBE: iadb_code | 变异：非种子码也返回第一个种子 | 红=iadb_code_rejects_non_seed | 绿=iadb_code_rejects_non_seed
//! // TDD-PROBE: UkCbIadbCode::from_code | 变异：大小写不敏感（接受小写） | 红=iadb_code_is_case_sensitive | 绿=iadb_code_is_case_sensitive
//! // TDD-PROBE: UkCbIadbCode::as_str | 变异：XUDLERI 返回 "XUDLERIX" | 红=iadb_key_codes_match_manifest | 绿=iadb_key_codes_match_manifest
//! // TDD-PROBE: UkCbSourceId::as_str | 变异：S11 返回 "S12" | 红=source_ids_are_s01_to_s11 | 绿=source_ids_are_s01_to_s11
//! // TDD-PROBE: UkCbSourceId::is_curve | 变异：改为判定 S08 | 红=only_s07_is_curve | 绿=only_s07_is_curve
//! // TDD-PROBE: UkCbSourceId::plan | 变异：S07 取到 S08 的登记条目 | 红=plan_lookup_is_consistent | 绿=plan_lookup_is_consistent
//! // TDD-PROBE: UkCbFormat::is_offline_implemented | 变异：恒返回 true | 红=only_csv_and_json_are_implemented | 绿=only_csv_and_json_are_implemented
//! // TDD-PROBE: UkCbFormat::as_str | 变异：Xlsx 返回 "xls" | 红=format_names_match_manifest | 绿=format_names_match_manifest
//! // TDD-PROBE: UkCbSeriesId::parse | 变异：不校验字符集 | 红=series_id_shape_is_enforced | 绿=series_id_shape_is_enforced
//! // TDD-PROBE: UkCbSeriesId::iadb_code | 变异：非命中恒返回第一个种子码 | 红=series_id_reports_key_code_only_on_hit | 绿=series_id_reports_key_code_only_on_hit
//! // TDD-PROBE: UkCbSeriesId::as_str | 变异：恒返回空串（标识丢失） | 红=series_id_as_str_round_trips | 绿=series_id_as_str_round_trips
//! // TDD-PROBE: UkCbValue::present | 变异：缺失返回 Some(0.0) | 红=absent_value_is_not_zero | 绿=absent_value_is_not_zero
//! // TDD-PROBE: UkCbValue::is_absent | 变异：恒返回 false | 红=absent_value_is_not_zero | 绿=absent_value_is_not_zero
//! // TDD-PROBE: validate_observation | 变异：不校验非有限数值 | 红=validate_rejects_non_finite_value | 绿=validate_rejects_non_finite_value
//! // TDD-PROBE: UkCbObservation::revision | 变异：未提供修订时返回 Some("v1") | 红=revision_is_absent_when_not_provided | 绿=revision_is_absent_when_not_provided
//! // TDD-PROBE: parse_period | 变异：年粒度分支失效 | 红=parse_period_forms | 绿=parse_period_forms
//! // TDD-PROBE: Date::parse | 变异：接受 2026/09/18 分隔符 | 红=date_parse_requires_iso_separator | 绿=date_parse_requires_iso_separator
//! // TDD-PROBE: Date::new | 变异：不校验日上限 | 红=date_new_rejects_out_of_range_day | 绿=date_new_rejects_out_of_range_day
//! // TDD-PROBE: Date::validate | 变异：闰年二月按 28 天 | 红=date_validate_handles_leap_february | 绿=date_validate_handles_leap_february
//! // TDD-PROBE: Date::is_leap_year | 变异：仅按 4 年一闰判断 | 红=leap_year_follows_gregorian_rule | 绿=leap_year_follows_gregorian_rule
//! // TDD-PROBE: Period::validate | 变异：季上限放宽到 9 | 红=period_validate_rejects_bad_quarter | 绿=period_validate_rejects_bad_quarter
//! // TDD-PROBE: Frequency::as_str | 变异：Irregular 返回 "irreg" | 红=frequency_round_trips | 绿=frequency_round_trips
//! // TDD-PROBE: Frequency::parse | 变异：接受 "irreg" 别名 | 红=frequency_parse_rejects_unknown | 绿=frequency_parse_rejects_unknown
//! // TDD-PROBE: UkCbUnit::as_str | 变异：Native 返回 "nat" | 红=unit_round_trips | 绿=unit_round_trips
//! // TDD-PROBE: UkCbUnit::parse | 变异：接受 "billions" | 红=unit_parse_rejects_unknown | 绿=unit_parse_rejects_unknown
//! // TDD-PROBE: authorize | 变异：过期检查失效 | 红=authorize_denies_expired_evidence | 绿=authorize_denies_expired_evidence
//! // TDD-PROBE: ensure_authorized | 变异：Denied 映射为 Invalid | 红=ensure_authorized_maps_to_authorization_denied | 绿=ensure_authorized_maps_to_authorization_denied
//! // TDD-PROBE: publication_semantics | 变异：时间精度返回 Instant | 红=publication_semantics_is_pinned | 绿=publication_semantics_is_pinned
//! // TDD-PROBE: is_formal_pit_eligible | 变异：恒返回 true | 红=formal_pit_is_never_eligible | 绿=formal_pit_is_never_eligible
//! // TDD-PROBE: UkCbError::kind | 变异：NotApplicable 映射为 Invalid | 红=error_kind_mapping_is_exact | 绿=error_kind_mapping_is_exact
//! // TDD-PROBE: UkCbError::is_retryable | 变异：可重试判定取反 | 红=only_invariant_is_retryable_at_the_boundary | 绿=only_invariant_is_retryable_at_the_boundary
//!
//! 入口列一律写成**可调用入口**（`类型::方法` 或裸函数名），与其余新仓一致；
//! 纯数据类型与常量表（如 `SOURCE_PLANS` / `IADB_KEY_CODES` / `PLANNED_SCHEDULE` /
//! `CURVE_ROUTE`）由对应入口（`UkCbSourceId::plan` / `UkCbIadbCode::as_str` 等）
//! 的断言覆盖，不单独立行。

use ukx::{
    authorize, ensure_authorized, guard_curve, iadb_code, is_formal_pit_eligible, parse_period,
    parse_uk_cb_observations, publication_semantics, validate_observation, AvailabilityEvidence,
    Date, Frequency, Period, PitEligibility, TimePrecision, UkCbAbsence, UkCbAuthorization,
    UkCbAuthorizationEvidence, UkCbError, UkCbErrorKind, UkCbFormat, UkCbIadbCode, UkCbObservation,
    UkCbSeriesId, UkCbSourceId, UkCbSourcePlan, UkCbUnit, UkCbValue, CURVE_ROUTE, IADB_KEY_CODES,
    PLANNED_SCHEDULE, SOURCE_ID, SOURCE_PLANS,
};

const CSV: &str = include_str!("fixtures/iadb_key_series.csv");
const JSON: &str = include_str!("fixtures/ons_cpi.json");

fn date(text: &str) -> Date {
    Date::parse(text).expect("测试日期合法")
}

fn csv_with(rows: &str) -> String {
    format!(
        "# _synthetic: true\n# _note: 合成样本\nseries,period,value,unit,frequency,revision\n{rows}"
    )
}

fn observation() -> UkCbObservation {
    UkCbObservation {
        series: UkCbSeriesId::parse("IUDBEDR").expect("标识合法"),
        period: Period::Day(date("2026-09-18")),
        value: UkCbValue::Present(4.0),
        unit: UkCbUnit::Percent,
        frequency: Frequency::Daily,
        revision: None,
    }
}

fn evidence() -> UkCbAuthorizationEvidence {
    UkCbAuthorizationEvidence {
        scope: "offline fixture parse".to_owned(),
        signer: "Owner".to_owned(),
        owner_signed: true,
        valid_from: date("2026-01-01"),
        valid_until: date("2026-12-31"),
    }
}

#[test]
fn parse_rejects_duplicate_identity() {
    let input =
        csv_with("IUDBEDR,2026-09-18,4.0,percent,daily,\nIUDBEDR,2026-09-18,4.1,percent,daily,\n");
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, &input)
            .expect_err("重复身份")
            .kind(),
        UkCbErrorKind::SemanticallyRejected
    );
}

#[test]
fn curve_source_is_routed() {
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S07, CSV)
            .expect_err("曲线须路由")
            .kind(),
        UkCbErrorKind::RoutedElsewhere
    );
}

#[test]
fn iadb_code_rejects_non_seed() {
    assert_eq!(
        iadb_code("IUDBEDX").expect_err("非种子码").kind(),
        UkCbErrorKind::SemanticallyRejected
    );
    assert_eq!(iadb_code("IUDBEDR").expect("种子码").as_str(), "IUDBEDR");
}

#[test]
fn iadb_code_is_case_sensitive() {
    assert_eq!(
        iadb_code("iudbedr").expect_err("小写").kind(),
        UkCbErrorKind::SemanticallyRejected
    );
}

#[test]
fn iadb_key_codes_match_manifest() {
    let expected = [
        "IUDBEDR", "IUDSOIA", "RPWB55A", "LPMAUYN", "XUDLUSS", "XUDLERI",
    ];
    assert_eq!(IADB_KEY_CODES.len(), 6);
    for (code, text) in IADB_KEY_CODES.iter().zip(expected) {
        assert_eq!(code.as_str(), text);
    }
}

#[test]
fn source_ids_are_s01_to_s11() {
    let ids = [
        UkCbSourceId::S01,
        UkCbSourceId::S02,
        UkCbSourceId::S03,
        UkCbSourceId::S04,
        UkCbSourceId::S05,
        UkCbSourceId::S06,
        UkCbSourceId::S07,
        UkCbSourceId::S08,
        UkCbSourceId::S09,
        UkCbSourceId::S10,
        UkCbSourceId::S11,
    ];
    for (index, id) in ids.iter().enumerate() {
        assert_eq!(id.as_str(), format!("S{:02}", index + 1));
    }
}

#[test]
fn only_s07_is_curve() {
    for id in [
        UkCbSourceId::S01,
        UkCbSourceId::S06,
        UkCbSourceId::S08,
        UkCbSourceId::S11,
    ] {
        assert!(!id.is_curve(), "{id:?} 不是曲线源");
    }
    assert!(UkCbSourceId::S07.is_curve());
}

#[test]
fn plan_lookup_is_consistent() {
    for id in [UkCbSourceId::S01, UkCbSourceId::S07, UkCbSourceId::S11] {
        assert_eq!(id.plan().id, id);
    }
    assert_eq!(UkCbSourceId::S07.plan().name, "收益率曲线 ZIP");
}

#[test]
fn only_csv_and_json_are_implemented() {
    assert!(UkCbFormat::Csv.is_offline_implemented());
    assert!(UkCbFormat::Json.is_offline_implemented());
    for format in [
        UkCbFormat::Xml,
        UkCbFormat::Excel,
        UkCbFormat::Xlsx,
        UkCbFormat::Zip,
    ] {
        assert!(
            !format.is_offline_implemented(),
            "{} 未落地",
            format.as_str()
        );
    }
}

#[test]
fn format_names_match_manifest() {
    assert_eq!(UkCbFormat::Csv.as_str(), "csv");
    assert_eq!(UkCbFormat::Xml.as_str(), "xml");
    assert_eq!(UkCbFormat::Json.as_str(), "json");
    assert_eq!(UkCbFormat::Xlsx.as_str(), "xlsx");
    assert_eq!(UkCbFormat::Zip.as_str(), "zip");
    assert_eq!(UkCbFormat::Excel.as_str(), "excel");
}

#[test]
fn series_id_shape_is_enforced() {
    for bad in ["a b", "IUDBEDR;", "IUDBEDR/x", ""] {
        let kind = UkCbSeriesId::parse(bad).expect_err(bad).kind();
        assert!(
            matches!(kind, UkCbErrorKind::Invalid | UkCbErrorKind::Missing),
            "{bad} 应被拒绝"
        );
    }
    assert!(UkCbSeriesId::parse("ONS.CPI.YOY").is_ok());
}

#[test]
fn series_id_reports_key_code_only_on_hit() {
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
}

#[test]
fn series_id_as_str_round_trips() {
    let id = UkCbSeriesId::parse("IUDBEDR").expect("合法");
    assert_eq!(id.as_str(), "IUDBEDR");
    assert_eq!(id.to_string(), "IUDBEDR");
}

#[test]
fn absent_value_is_not_zero() {
    let input = csv_with("IUDBEDR,2026-09-18,,percent,daily,\n");
    let observations = parse_uk_cb_observations(UkCbSourceId::S01, &input).expect("空值合法");
    assert_eq!(
        observations[0].value,
        UkCbValue::Absent(UkCbAbsence::BlankInSource)
    );
    assert_eq!(observations[0].value.present(), None, "缺失不是 0");
    assert!(observations[0].value.is_absent());
    assert_eq!(UkCbValue::Present(1.5).present(), Some(1.5));
    assert!(!UkCbValue::Present(1.5).is_absent());
}

#[test]
fn validate_rejects_non_finite_value() {
    let mut sample = observation();
    sample.value = UkCbValue::Present(f64::NAN);
    assert_eq!(
        validate_observation(&sample).expect_err("NaN").kind(),
        UkCbErrorKind::Invalid
    );
    sample.value = UkCbValue::Present(f64::INFINITY);
    assert_eq!(
        validate_observation(&sample).expect_err("INF").kind(),
        UkCbErrorKind::Invalid
    );
    assert!(validate_observation(&observation()).is_ok());
}

#[test]
fn revision_is_absent_when_not_provided() {
    assert_eq!(observation().revision(), None);
    let input = csv_with("IUDBEDR,2026-09-18,4.0,percent,daily,r1\n");
    let observations = parse_uk_cb_observations(UkCbSourceId::S01, &input).expect("修订列合法");
    assert_eq!(observations[0].revision(), Some("r1"));
}

#[test]
fn parse_period_forms() {
    assert_eq!(
        parse_period("2026-09-18").expect("日"),
        Period::Day(date("2026-09-18"))
    );
    assert_eq!(
        parse_period("2026-08").expect("月"),
        Period::Month {
            year: 2026,
            month: 8
        }
    );
    assert_eq!(
        parse_period("2026-Q4").expect("季"),
        Period::Quarter {
            year: 2026,
            quarter: 4
        }
    );
    assert_eq!(parse_period("2026").expect("年"), Period::Year(2026));
    assert_eq!(
        parse_period("event:2026-09-18").expect("事件"),
        Period::Event {
            date: date("2026-09-18")
        }
    );
}

#[test]
fn date_parse_requires_iso_separator() {
    assert_eq!(date("2026-09-18").to_string(), "2026-09-18");
    for bad in ["2026/09/18", "2026-9-18", "2026-09-18T00:00:00"] {
        assert_eq!(
            Date::parse(bad).expect_err(bad).kind(),
            UkCbErrorKind::Invalid
        );
    }
}

#[test]
fn date_new_rejects_out_of_range_day() {
    assert_eq!(
        Date::new(2026, 4, 31).expect_err("四月无 31 日").kind(),
        UkCbErrorKind::Invalid
    );
    assert!(Date::new(2026, 4, 30).is_ok());
}

#[test]
fn date_validate_handles_leap_february() {
    assert!(Date::new(2024, 2, 29).is_ok(), "闰年二月有 29 日");
    assert!(Date::new(2023, 2, 29).is_err(), "平年二月没有 29 日");
}

#[test]
fn leap_year_follows_gregorian_rule() {
    assert!(Date::is_leap_year(2024));
    assert!(!Date::is_leap_year(1900), "百年不闰");
    assert!(Date::is_leap_year(2000), "四百年再闰");
}

#[test]
fn period_validate_rejects_bad_quarter() {
    for bad in [
        Period::Quarter {
            year: 2026,
            quarter: 5,
        },
        Period::Month {
            year: 2026,
            month: 13,
        },
    ] {
        assert_eq!(
            bad.validate().expect_err("越界").kind(),
            UkCbErrorKind::Invalid
        );
    }
    assert!(Period::Year(2026).validate().is_ok());
    assert!(Period::Day(date("2026-09-18")).validate().is_ok());
}

#[test]
fn frequency_round_trips() {
    for frequency in [
        Frequency::Daily,
        Frequency::Weekly,
        Frequency::Monthly,
        Frequency::Quarterly,
        Frequency::Annual,
        Frequency::Event,
        Frequency::Irregular,
    ] {
        assert_eq!(
            Frequency::parse(frequency.as_str()).expect("可回读"),
            frequency
        );
    }
}

#[test]
fn frequency_parse_rejects_unknown() {
    for bad in ["irreg", "fortnightly"] {
        assert_eq!(
            Frequency::parse(bad).expect_err(bad).kind(),
            UkCbErrorKind::NotApplicable
        );
    }
}

#[test]
fn unit_round_trips() {
    for unit in [UkCbUnit::Unspecified, UkCbUnit::Native, UkCbUnit::Percent] {
        assert_eq!(UkCbUnit::parse(unit.as_str()).expect("可回读"), unit);
    }
}

#[test]
fn unit_parse_rejects_unknown() {
    assert_eq!(
        UkCbUnit::parse("billions").expect_err("未知单位").kind(),
        UkCbErrorKind::SemanticallyRejected
    );
}

#[test]
fn authorize_denies_expired_evidence() {
    let mut expired = evidence();
    expired.valid_until = date("2026-09-01");
    assert!(matches!(
        authorize(Some(&expired), date("2026-09-22")),
        UkCbAuthorization::Denied { .. }
    ));
    let mut not_yet = evidence();
    not_yet.valid_from = date("2026-10-01");
    assert!(matches!(
        authorize(Some(&not_yet), date("2026-09-22")),
        UkCbAuthorization::Denied { .. }
    ));
}

#[test]
fn ensure_authorized_maps_to_authorization_denied() {
    assert_eq!(
        ensure_authorized(None, date("2026-09-22"))
            .expect_err("证据缺失")
            .kind(),
        UkCbErrorKind::AuthorizationDenied
    );
    assert!(ensure_authorized(Some(&evidence()), date("2026-09-22")).is_ok());
}

#[test]
fn authorize_denials_carry_a_readable_reason() {
    let as_of = date("2026-09-22");
    let mut candidate = evidence();

    // 六条拒绝路径都必须给出**可读理由**，否则调用方只能记录「失败」。
    let mut denials = vec![authorize(None, as_of)];

    candidate.scope = "   ".to_owned();
    denials.push(authorize(Some(&candidate), as_of));

    candidate.scope = "offline fixture parse".to_owned();
    candidate.signer = "  ".to_owned();
    denials.push(authorize(Some(&candidate), as_of));

    candidate.signer = "Owner".to_owned();
    candidate.owner_signed = false;
    denials.push(authorize(Some(&candidate), as_of));

    candidate.owner_signed = true;
    denials.push(authorize(Some(&candidate), date("2025-12-31")));
    denials.push(authorize(Some(&candidate), date("2027-01-01")));

    assert_eq!(denials.len(), 6);
    for verdict in denials {
        assert!(
            matches!(verdict, UkCbAuthorization::Denied { .. }),
            "该路径必须拒绝"
        );
        if let UkCbAuthorization::Denied { reason } = verdict {
            assert!(!reason.trim().is_empty(), "Denied 必须携带可读理由");
        }
    }
}

#[test]
fn publication_semantics_is_pinned() {
    assert_eq!(
        publication_semantics(),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        )
    );
}

#[test]
fn formal_pit_is_never_eligible() {
    assert!(!is_formal_pit_eligible());
}

#[test]
fn error_kind_mapping_is_exact() {
    let cases: [(UkCbError, UkCbErrorKind); 8] = [
        (UkCbError::Invalid("i".to_owned()), UkCbErrorKind::Invalid),
        (UkCbError::Missing("m".to_owned()), UkCbErrorKind::Missing),
        (
            UkCbError::AuthorizationDenied("a".to_owned()),
            UkCbErrorKind::AuthorizationDenied,
        ),
        (
            UkCbError::RoutedElsewhere("r".to_owned()),
            UkCbErrorKind::RoutedElsewhere,
        ),
        (
            UkCbError::WriteAuthorityDenied("w".to_owned()),
            UkCbErrorKind::WriteAuthorityDenied,
        ),
        (
            UkCbError::SemanticallyRejected("s".to_owned()),
            UkCbErrorKind::SemanticallyRejected,
        ),
        (
            UkCbError::NotApplicable("n".to_owned()),
            UkCbErrorKind::NotApplicable,
        ),
        (
            UkCbError::Invariant("v".to_owned()),
            UkCbErrorKind::Invariant,
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.kind(), expected);
    }
}

#[test]
fn only_invariant_is_retryable_at_the_boundary() {
    assert!(!UkCbError::AuthorizationDenied("a".to_owned()).is_retryable());
    assert!(!UkCbError::NotApplicable("n".to_owned()).is_retryable());
    assert!(UkCbError::Invariant("v".to_owned()).is_retryable());
}

#[test]
fn source_identity_constants_are_pinned() {
    assert_eq!(SOURCE_ID, "uk_cb");
    assert_eq!(ukx::AUTHORIZATION_STATUS, "unknown");
}

#[test]
fn source_plans_match_manifest() {
    assert_eq!(SOURCE_PLANS.len(), 11);
    let expected: [(UkCbSourceId, &str, &str, &str); 11] = [
        (UkCbSourceId::S01, "IADB", "CSV/XML", "日/月/季"),
        (UkCbSourceId::S02, "Bank Rate `IUDBEDR`", "事件", "事件"),
        (UkCbSourceId::S03, "Bank Return", "CSV/Excel", "周"),
        (UkCbSourceId::S04, "APF", "CSV", "周/操作日"),
        (UkCbSourceId::S05, "市场操作", "CSV", "操作日"),
        (UkCbSourceId::S06, "SONIA `IUDSOIA`", "日 T+1", "日"),
        (UkCbSourceId::S07, "收益率曲线 ZIP", "ZIP", "日"),
        (UkCbSourceId::S08, "汇率 XUDL*", "日", "日"),
        (UkCbSourceId::S09, "货币信贷", "月", "月"),
        (UkCbSourceId::S10, "DMO Gilt", "CSV", "日/事件"),
        (UkCbSourceId::S11, "ONS CPI/RPI", "JSON", "月"),
    ];
    for (plan, (id, name, form, frequency)) in SOURCE_PLANS.iter().zip(expected) {
        let _: &UkCbSourcePlan = plan;
        assert_eq!(plan.id, id);
        assert_eq!(plan.name, name);
        assert_eq!(plan.declared_form, form);
        assert_eq!(plan.declared_frequency, frequency);
    }
}

#[test]
fn planned_schedule_matches_manifest() {
    assert_eq!(PLANNED_SCHEDULE.len(), 8);
    let slots = [
        Some("08:05"),
        Some("08:35"),
        Some("16:30"),
        Some("15:05"),
        Some("14:35"),
        Some("18:00"),
        None,
        Some("02:00"),
    ];
    for (window, slot) in PLANNED_SCHEDULE.iter().zip(slots) {
        assert_eq!(window.utc_time, slot, "{}", window.label);
    }
}

#[test]
fn curve_route_target_is_pinned() {
    assert_eq!(CURVE_ROUTE.slot, "yield_curve");
    assert_eq!(CURVE_ROUTE.providers, ["boe_iadb", "uk_dmo"]);
}

#[test]
fn json_form_parses_and_rejects_unknown_fields() {
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S11, JSON)
            .expect("JSON 合成样本")
            .len(),
        3
    );
    let extra = JSON.replace(
        r#""series": "ONS.CPI.YOY","#,
        r#""series": "ONS.CPI.YOY", "extra": 1,"#,
    );
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S11, &extra)
            .expect_err("未知字段")
            .kind(),
        UkCbErrorKind::Invalid
    );
}

#[test]
fn undeclared_format_sources_are_not_applicable() {
    for source in [
        UkCbSourceId::S02,
        UkCbSourceId::S06,
        UkCbSourceId::S08,
        UkCbSourceId::S09,
    ] {
        assert_eq!(
            parse_uk_cb_observations(source, CSV)
                .expect_err(source.as_str())
                .kind(),
            UkCbErrorKind::NotApplicable
        );
    }
    assert!(guard_curve(UkCbSourceId::S01).is_ok());
}
