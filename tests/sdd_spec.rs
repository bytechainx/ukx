#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! SDD 规格对照（特性 005）：把 `docs/标准.md` 的每个 `##` 章节转成可执行断言。
//!
//! 章节与断言函数须与 `docs/标准.md` 的 `##` 章节 1:1（检查器按标题逐字比对）。
//!
//! // SPEC-MAP: S-1 | 1. 源事实范围 | assert_source_fact_scope
//! // SPEC-MAP: S-2 | 2. 源登记 S01–S11 | assert_source_registry
//! // SPEC-MAP: S-3 | 3. 关键 IADB 码与不得扩集 | assert_iadb_key_codes
//! // SPEC-MAP: S-4 | 4. 曲线路由边界 | assert_curve_routing_boundary
//! // SPEC-MAP: S-5 | 5. 规划调度口径 | assert_planned_schedule
//! // SPEC-MAP: S-6 | 6. 离线解析形态（CSV / JSON） | assert_offline_parse_forms
//! // SPEC-MAP: S-7 | 7. 原子失败与重复身份 | assert_atomic_failure_and_duplicates
//! // SPEC-MAP: S-8 | 8. 缺失取值的具名表达 | assert_named_absence
//! // SPEC-MAP: S-9 | 9. 合成夹具声明 | assert_synthetic_fixture_declaration
//! // SPEC-MAP: S-10 | 10. 授权判定（fail-closed） | assert_authorization_is_fail_closed
//! // SPEC-MAP: S-11 | 11. publication 与 PIT 资格 | assert_publication_semantics
//! // SPEC-MAP: S-12 | 12. 依赖与零网络边界 | assert_dependency_and_zero_network
//! // SPEC-MAP: S-13 | 13. 验收 | assert_acceptance

use ukx::{
    authorize, ensure_authorized, guard_curve, iadb_code, is_formal_pit_eligible, parse_period,
    parse_uk_cb_observations, publication_semantics, validate_observation, AvailabilityEvidence,
    Date, Frequency, Period, PitEligibility, TimePrecision, UkCbAbsence, UkCbAuthorizationEvidence,
    UkCbErrorKind, UkCbFormat, UkCbIadbCode, UkCbObservation, UkCbSeriesId, UkCbSourceId, UkCbUnit,
    UkCbValue, CURVE_ROUTE, IADB_KEY_CODES, PLANNED_SCHEDULE, SOURCE_PLANS,
};

const CSV: &str = include_str!("fixtures/iadb_key_series.csv");
const JSON: &str = include_str!("fixtures/ons_cpi.json");
const STANDARD: &str = include_str!("../docs/标准.md");
const CARGO_TOML: &str = include_str!("../Cargo.toml");

/// 运行期递归收集本仓 `src/` 下全部 `.rs`（**不**用手写清单，避免新增文件成为扫描盲区）。
fn source_files() -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("src 目录可读") {
            let path = entry.expect("目录项可读").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    walk(&root, &mut out);
    out
}

fn date(text: &str) -> Date {
    Date::parse(text).expect("测试日期合法")
}

fn csv_with(rows: &str) -> String {
    format!("# _synthetic: true\n# _note: 合成样本\nseries,period,value,unit,frequency,revision\n{rows}")
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

/// S-1：本源覆盖清单 §1 声明的范围，且不越界到网络 / 存储 / 派生。
#[test]
fn assert_source_fact_scope() {
    assert_eq!(ukx::SOURCE_ID, "uk_cb");
    assert_eq!(ukx::AUTHORIZATION_STATUS, "unknown");
    // 解析入口只需「源标识 + 字符串」：不需要任何网络 / 凭据上下文。
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, CSV)
            .expect("合成样本可解析")
            .len(),
        6
    );
    // 11 个源标识齐备。
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
    assert_eq!(ids.len(), 11);
    for (index, id) in ids.iter().enumerate() {
        assert_eq!(id.as_str(), format!("S{:02}", index + 1));
    }
}

/// S-2：S01–S11 登记与清单「形式」「频率」逐字文本。
#[test]
fn assert_source_registry() {
    assert_eq!(SOURCE_PLANS.len(), 11);
    assert_eq!(SOURCE_PLANS[0].name, "IADB");
    assert_eq!(SOURCE_PLANS[0].declared_form, "CSV/XML");
    assert_eq!(SOURCE_PLANS[0].formats, &[UkCbFormat::Csv, UkCbFormat::Xml]);
    assert_eq!(SOURCE_PLANS[0].declared_frequency, "日/月/季");
    assert_eq!(SOURCE_PLANS[6].declared_form, "ZIP");
    assert_eq!(SOURCE_PLANS[10].formats, &[UkCbFormat::Json]);
    assert_eq!(SOURCE_PLANS[10].id.plan().name, "ONS CPI/RPI");
    // 「形式」列给的是节奏而非格式者，formats 为空。
    for index in [1, 5, 7, 8] {
        assert!(
            SOURCE_PLANS[index].formats.is_empty(),
            "{} 未点名格式",
            SOURCE_PLANS[index].id.as_str()
        );
    }
}

/// S-3：6 个关键码与「不得扩集」守卫。
#[test]
fn assert_iadb_key_codes() {
    assert_eq!(IADB_KEY_CODES.len(), 6);
    let expected = [
        "IUDBEDR", "IUDSOIA", "RPWB55A", "LPMAUYN", "XUDLUSS", "XUDLERI",
    ];
    for (code, text) in IADB_KEY_CODES.iter().zip(expected) {
        assert_eq!(code.as_str(), text);
        assert_eq!(iadb_code(text).expect("种子码可选中"), *code);
    }
    for bad in ["IUDBEDX", "iudbedr", "XUDL", ""] {
        assert_eq!(
            iadb_code(bad).expect_err(bad).kind(),
            UkCbErrorKind::SemanticallyRejected
        );
    }
    // 观测标识原样保留，命中种子才返回关键码。
    assert_eq!(
        UkCbSeriesId::parse("IUDSOIA").expect("合法").iadb_code(),
        Some(UkCbIadbCode::Iudsoia)
    );
    assert_eq!(
        UkCbSeriesId::parse("ONS.CPI.YOY")
            .expect("合法")
            .iadb_code(),
        None
    );
}

/// S-4：曲线路由优先于格式判定，且不得直写观测。
#[test]
fn assert_curve_routing_boundary() {
    assert!(UkCbSourceId::S07.is_curve());
    assert_eq!(
        guard_curve(UkCbSourceId::S07)
            .expect_err("曲线须路由")
            .kind(),
        UkCbErrorKind::RoutedElsewhere
    );
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S07, CSV)
            .expect_err("曲线须先路由")
            .kind(),
        UkCbErrorKind::RoutedElsewhere
    );
    assert!(guard_curve(UkCbSourceId::S01).is_ok());
    assert_eq!(CURVE_ROUTE.slot, "yield_curve");
    assert_eq!(CURVE_ROUTE.providers, ["boe_iadb", "uk_dmo"]);
}

/// S-5：规划调度常量与其「非访问合同」口径。
#[test]
fn assert_planned_schedule() {
    assert_eq!(PLANNED_SCHEDULE.len(), 8);
    assert_eq!(PLANNED_SCHEDULE[0].utc_time, Some("08:05"));
    assert_eq!(PLANNED_SCHEDULE[1].utc_time, Some("08:35"));
    assert_eq!(PLANNED_SCHEDULE[2].utc_time, Some("16:30"));
    assert_eq!(PLANNED_SCHEDULE[3].utc_time, Some("15:05"));
    assert_eq!(PLANNED_SCHEDULE[4].utc_time, Some("14:35"));
    assert_eq!(PLANNED_SCHEDULE[5].utc_time, Some("18:00"));
    assert_eq!(PLANNED_SCHEDULE[6].utc_time, None, "日历驱动无固定时刻");
    assert_eq!(PLANNED_SCHEDULE[7].utc_time, Some("02:00"));
    assert!(STANDARD.contains("调度是规划语义，不是访问合同"));
}

/// S-6：只实现 CSV 与 JSON；其余格式与未点名格式的源一律未实现。
#[test]
fn assert_offline_parse_forms() {
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
    // CSV 形态：本层声明的表头与期间形态。
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
        parse_period("2026-Q3").expect("季"),
        Period::Quarter {
            year: 2026,
            quarter: 3
        }
    );
    assert_eq!(parse_period("2026").expect("年"), Period::Year(2026));
    assert_eq!(
        parse_period("event:2026-09-18").expect("事件"),
        Period::Event {
            date: date("2026-09-18")
        }
    );
    for bad in ["2026-9-18", "2026/09/18", "2026-Q5", "26-09"] {
        assert_eq!(
            parse_period(bad).expect_err(bad).kind(),
            UkCbErrorKind::Invalid
        );
    }
    // JSON 形态。
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S11, JSON)
            .expect("JSON 合成样本")
            .len(),
        3
    );
    // 未点名格式的源 → NotApplicable。
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
}

/// S-7：未知字段 / 缺字段 / 重复身份 / 未标注样本的原子失败面。
#[test]
fn assert_atomic_failure_and_duplicates() {
    let unknown_column =
        "# _synthetic: true\n# _note: 合成\nseries,period,value,unit,frequency,revision,extra\n";
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, unknown_column)
            .expect_err("未知列")
            .kind(),
        UkCbErrorKind::Invalid
    );
    let reordered =
        "# _synthetic: true\n# _note: 合成\nperiod,series,value,unit,frequency,revision\n";
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, reordered)
            .expect_err("换序")
            .kind(),
        UkCbErrorKind::Invalid
    );
    let missing_columns = "# _synthetic: true\n# _note: 合成\nseries,period,value\n";
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, missing_columns)
            .expect_err("缺列")
            .kind(),
        UkCbErrorKind::Invalid
    );
    let short_row = csv_with("IUDBEDR,2026-09-18,4.0,percent\n");
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, &short_row)
            .expect_err("行缺列")
            .kind(),
        UkCbErrorKind::Invalid
    );
    let duplicated =
        csv_with("IUDBEDR,2026-09-18,4.0,percent,daily,\nIUDBEDR,2026-09-18,4.1,percent,daily,\n");
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, &duplicated)
            .expect_err("重复身份")
            .kind(),
        UkCbErrorKind::SemanticallyRejected
    );
    let unmarked =
        "series,period,value,unit,frequency,revision\nIUDBEDR,2026-09-18,4.0,percent,daily,\n";
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, unmarked)
            .expect_err("缺标注")
            .kind(),
        UkCbErrorKind::SemanticallyRejected
    );
    let json_extra = JSON.replace(
        r#""series": "ONS.CPI.YOY","#,
        r#""series": "ONS.CPI.YOY", "extra": 1,"#,
    );
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S11, &json_extra)
            .expect_err("未知字段")
            .kind(),
        UkCbErrorKind::Invalid
    );
    let quoted = csv_with("\"IUDBEDR,2026-09-18\",4.0,percent,daily,\n");
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, &quoted)
            .expect_err("引号")
            .kind(),
        UkCbErrorKind::Invalid
    );
}

/// S-8：缺失取值的具名表达，且绝不静默转 0。
#[test]
fn assert_named_absence() {
    let blank = csv_with("IUDBEDR,2026-09-18,,percent,daily,\n");
    let observations = parse_uk_cb_observations(UkCbSourceId::S01, &blank).expect("空值合法");
    assert_eq!(
        observations[0].value,
        UkCbValue::Absent(UkCbAbsence::BlankInSource)
    );
    assert_eq!(observations[0].value.present(), None, "缺失不是 0");
    assert!(observations[0].value.is_absent());

    let marker = csv_with("IUDBEDR,2026-09-18,provisional,percent,daily,\n");
    let marked = parse_uk_cb_observations(UkCbSourceId::S01, &marker).expect("标记合法");
    assert_eq!(
        marked[0].value,
        UkCbValue::Absent(UkCbAbsence::ExplicitMarker)
    );

    // 非有限数不是合法取值。
    let mut sample = observation();
    sample.value = UkCbValue::Present(f64::NAN);
    assert_eq!(
        validate_observation(&sample).expect_err("NaN").kind(),
        UkCbErrorKind::Invalid
    );
    // 修订标识未提供时为 None。
    assert_eq!(observation().revision(), None);
    // 频率与期间须一致。
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

/// S-9：夹具为合成样本的声明与自查。
#[test]
fn assert_synthetic_fixture_declaration() {
    assert!(CSV.contains("# _synthetic: true"));
    assert!(CSV.contains("# _note:"));
    assert!(JSON.contains(r#""_synthetic": true"#));
    assert!(JSON.contains("不是真实源数据"));
    assert!(STANDARD.contains("合成样本"));
    assert!(STANDARD.contains("不构成任何证据"));
    let claimed_real = CSV.replace("# _synthetic: true", "# _synthetic: false");
    assert_eq!(
        parse_uk_cb_observations(UkCbSourceId::S01, &claimed_real)
            .expect_err("自称真实")
            .kind(),
        UkCbErrorKind::SemanticallyRejected
    );
}

/// S-10：授权 fail-closed 的六条拒绝路径与一条放行路径。
#[test]
fn assert_authorization_is_fail_closed() {
    let as_of = date("2026-09-22");
    assert!(matches!(
        authorize(None, as_of),
        ukx::UkCbAuthorization::Denied { .. }
    ));

    let mut evidence = UkCbAuthorizationEvidence {
        scope: "offline fixture parse".to_owned(),
        signer: "Owner".to_owned(),
        owner_signed: true,
        valid_from: date("2026-01-01"),
        valid_until: date("2026-12-31"),
    };
    evidence.scope = String::new();
    assert!(matches!(
        authorize(Some(&evidence), as_of),
        ukx::UkCbAuthorization::Denied { .. }
    ));
    evidence.scope = "offline fixture parse".to_owned();
    evidence.signer = "   ".to_owned();
    assert!(matches!(
        authorize(Some(&evidence), as_of),
        ukx::UkCbAuthorization::Denied { .. }
    ));
    evidence.signer = "Owner".to_owned();
    evidence.owner_signed = false;
    assert!(matches!(
        authorize(Some(&evidence), as_of),
        ukx::UkCbAuthorization::Denied { .. }
    ));
    evidence.owner_signed = true;
    assert!(matches!(
        authorize(Some(&evidence), date("2027-01-01")),
        ukx::UkCbAuthorization::Denied { .. }
    ));
    assert!(matches!(
        authorize(Some(&evidence), date("2025-12-31")),
        ukx::UkCbAuthorization::Denied { .. }
    ));
    assert_eq!(
        authorize(Some(&evidence), as_of),
        ukx::UkCbAuthorization::Authorized {
            scope: "offline fixture parse".to_owned()
        }
    );
    assert_eq!(
        ensure_authorized(None, as_of).expect_err("证据缺失").kind(),
        UkCbErrorKind::AuthorizationDenied
    );
}

/// S-11：publication 三元组与 PIT 资格被钉死。
#[test]
fn assert_publication_semantics() {
    assert_eq!(
        publication_semantics(),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        )
    );
    assert!(!is_formal_pit_eligible());
}

/// S-12：零网络、零凭据、零跨仓依赖。
#[test]
fn assert_dependency_and_zero_network() {
    // 依赖面须**恰为**允许集：多一个即违规（无需枚举被禁项，避免把禁项名写进源码）。
    let mut dependencies: Vec<&str> = Vec::new();
    let mut in_dependencies = false;
    for line in CARGO_TOML.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]";
            continue;
        }
        if !in_dependencies || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            dependencies.push(name.trim());
        }
    }
    dependencies.sort_unstable();
    assert_eq!(
        dependencies,
        ["serde", "serde_json", "thiserror"],
        "依赖面须恰为允许集"
    );
    assert!(!CARGO_TOML.contains("path = \"../"));
    // 该断言保证的是：**本仓 `src/` 下每一个 `.rs` 文件**（运行期递归枚举，
    // 而非手写文件清单）都不含 `http://` / `https://` 端点字面量，也不读环境变量。
    let files = source_files();
    assert!(!files.is_empty(), "src 下应有源码文件");
    for file in &files {
        let text = std::fs::read_to_string(file).expect("源码可读");
        assert!(
            !text.contains("https://") && !text.contains("http://"),
            "{} 不得含端点字面量",
            file.display()
        );
        assert!(
            !text.contains("env::var"),
            "{} 不得读环境变量",
            file.display()
        );
        assert!(
            !text.contains("from_env"),
            "{} 不得读凭据环境",
            file.display()
        );
    }
}

/// S-13：验收命令被文档化，且本测试面可一次性执行。
#[test]
fn assert_acceptance() {
    for command in [
        "cargo fmt --all -- --check",
        "cargo clippy --all-targets --all-features -- -D warnings",
        "cargo test --all-features",
        "cargo package --no-verify",
    ] {
        assert!(STANDARD.contains(command), "标准文档须列出验收命令");
    }
    let _ = std::env::current_dir().expect("可取得当前目录");
}
