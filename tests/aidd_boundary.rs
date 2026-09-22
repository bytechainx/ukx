#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! AIDD 对抗 / 边界用例（特性 005）。
//!
//! 候选由 AI 生成，逐条人工复核后仅保留「结论=保留」项；丢弃项登记于 PR 描述。
//!
//! // AIDD: 期间 `2026-Q5` 季越界 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §6 期间形态 | 结论=保留
//! // AIDD: 空 value 与显式非数值标记的区分 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §8 缺失具名表达 | 结论=保留
//! // AIDD: 日频观测配年粒度期间 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §8 频率与期间一致 | 结论=保留
//! // AIDD: 序列标识含空格与分号 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 标识原样保留 | 结论=保留
//! // AIDD: 非种子 IADB 码的大小写变体 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 不得扩集 | 结论=保留
//! // AIDD: 十万字符的序列标识 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §7 解析不得 panic | 结论=保留
//! // AIDD: JSON `value` 为布尔或数组 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §8 具名缺失 | 结论=保留
//! // AIDD: 前导 UTF-8 BOM | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §6 CSV 子集 | 结论=保留
//! // AIDD: 同序列不同期间不构成重复身份 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §7 重复身份 | 结论=保留
//! // AIDD: 修订列为全空白 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §8 修订不得伪造 | 结论=保留

use ukx::{
    parse_period, parse_uk_cb_observations, validate_observation, Date, Frequency, Period,
    UkCbAbsence, UkCbErrorKind, UkCbObservation, UkCbSeriesId, UkCbSourceId, UkCbUnit, UkCbValue,
};

const JSON: &str = include_str!("fixtures/ons_cpi.json");

fn csv_with(rows: &str) -> String {
    format!(
        "# _synthetic: true\n# _note: 合成样本\nseries,period,value,unit,frequency,revision\n{rows}"
    )
}

/// 边界：季粒度只允许 1–4，`2026-Q5` 必须被拒。
#[test]
fn quarter_out_of_range_is_rejected() {
    for bad in ["2026-Q0", "2026-Q5", "2026-Q9"] {
        assert_eq!(
            parse_period(bad).expect_err(bad).kind(),
            UkCbErrorKind::Invalid
        );
    }
    assert_eq!(
        parse_period("2026-Q1").expect("合法季"),
        Period::Quarter {
            year: 2026,
            quarter: 1
        }
    );
}

/// 边界：空值与非数值标记是**两种**具名缺失，且都不是 0。
#[test]
fn blank_and_marker_are_distinct_absences() {
    let blank = csv_with("IUDBEDR,2026-09-18,,percent,daily,\n");
    let parsed = parse_uk_cb_observations(UkCbSourceId::S01, &blank).expect("空值");
    assert_eq!(
        parsed[0].value,
        UkCbValue::Absent(UkCbAbsence::BlankInSource)
    );

    let marker = csv_with("IUDBEDR,2026-09-18,n/a,percent,daily,\n");
    let marked = parse_uk_cb_observations(UkCbSourceId::S01, &marker).expect("标记");
    assert_eq!(
        marked[0].value,
        UkCbValue::Absent(UkCbAbsence::ExplicitMarker)
    );
    assert_ne!(marked[0].value, parsed[0].value, "两类缺失不得混同");
}

/// 边界：频率与期间不一致必须被拒（日频配年期间）。
#[test]
fn frequency_period_mismatch_is_rejected() {
    let input = csv_with("IUDBEDR,2026,4.0,percent,daily,\n");
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, &input)
            .expect_err("日频配年期间")
            .kind(),
        UkCbErrorKind::Invalid
    );
}

/// 边界：序列标识的字符集约束生效，不因「原样保留」而放行任意文本。
#[test]
fn series_id_character_set_is_enforced() {
    for bad in ["a b", "IUDBEDR;", "IUDBEDR/GBP", "序列"] {
        let kind = UkCbSeriesId::parse(bad).expect_err(bad).kind();
        assert!(
            matches!(kind, UkCbErrorKind::Invalid | UkCbErrorKind::Missing),
            "{bad} 应被拒绝"
        );
    }
    assert!(UkCbSeriesId::parse("XUDLUSS").is_ok());
}

/// 边界：非种子 IADB 码的大小写变体一律拒绝，不得近义放行。
#[test]
fn iadb_code_variants_are_rejected() {
    for bad in ["iudbedr", "Iudbedr", "iuDBEDR", "IUDBEDR "] {
        assert_eq!(
            ukx::iadb_code(bad).expect_err(bad).kind(),
            UkCbErrorKind::SemanticallyRejected
        );
    }
}

/// 边界：超长序列标识须被正常解析或拒绝，不得 panic、不得截断。
#[test]
fn huge_series_id_does_not_panic() {
    let huge = format!("S{}", "x".repeat(100_000));
    let input = csv_with(&format!("{huge},2026-09-18,1.0,native,daily,\n"));
    let parsed = parse_uk_cb_observations(UkCbSourceId::S01, &input).expect("超长标识仍可解析");
    assert_eq!(parsed[0].series.as_str().len(), 100_001);
}

/// 边界：JSON `value` 为布尔 / 数组 / 对象时按具名缺失处理，绝不 panic。
#[test]
fn json_non_numeric_value_becomes_named_absence() {
    for raw in ["true", "[1,2]", r#"{"a":1}"#, r#""""#] {
        let input = JSON.replacen("2.5", raw, 1);
        let parsed = parse_uk_cb_observations(UkCbSourceId::S11, &input).expect("非数值按缺失");
        assert!(parsed[0].value.is_absent(), "{raw} 应表达为具名缺失");
    }
}

/// 边界：前导 BOM 是编码产物，容忍一次；不影响其余校验。
#[test]
fn leading_bom_is_tolerated_once() {
    let input = format!(
        "\u{feff}{}",
        csv_with("IUDBEDR,2026-09-18,4.0,percent,daily,\n")
    );
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, &input)
            .expect("BOM 容忍")
            .len(),
        1
    );
    let doubled = format!(
        "\u{feff}\u{feff}{}",
        csv_with("IUDBEDR,2026-09-18,4.0,percent,daily,\n")
    );
    assert!(parse_uk_cb_observations(UkCbSourceId::S01, &doubled).is_err());
}

/// 边界：同序列不同期间不是重复身份，必须都能通过。
#[test]
fn same_series_different_period_is_not_duplicate() {
    let input =
        csv_with("IUDBEDR,2026-09-18,4.0,percent,daily,\nIUDBEDR,2026-09-19,4.0,percent,daily,\n");
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, &input)
            .expect("不同期间")
            .len(),
        2
    );
}

/// 边界：修订列为全空白时视为未提供，不得伪造修订标识。
#[test]
fn blank_revision_is_none() {
    let input = csv_with("IUDBEDR,2026-09-18,4.0,percent,daily,   \n");
    let parsed = parse_uk_cb_observations(UkCbSourceId::S01, &input).expect("空白修订列");
    assert_eq!(parsed[0].revision(), None);

    let observation = UkCbObservation {
        series: UkCbSeriesId::parse("IUDBEDR").expect("标识"),
        period: Period::Day(Date::parse("2026-09-18").expect("日期")),
        value: UkCbValue::Present(4.0),
        unit: UkCbUnit::Percent,
        frequency: Frequency::Daily,
        revision: Some("  ".to_owned()),
    };
    assert_eq!(
        validate_observation(&observation)
            .expect_err("空修订")
            .kind(),
        UkCbErrorKind::Missing
    );
}
