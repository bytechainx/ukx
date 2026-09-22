//! ukx 的离线解析器：字符串 → 观测集合。
//!
//! **输入形态**由本库在 `docs/标准.md` 声明（CSV 与 JSON 两种**离线交换形态**，
//! 不是 BoE 的文件布局）。本模块**没有**网络参数：不接受 URL、HTTP 客户端、
//! 认证信息、环境变量或任何代理配置。
//!
//! 四条硬约束：
//!
//! 1. **曲线路由优先**：S07 收益率曲线在解析前即被拒绝并指向 `yield_curve`；
//! 2. **只接受显式标注的合成样本**：`_synthetic: true` 与非空 `_note` 缺一不可；
//! 3. **未知字段原子失败**：CSV 表头与 JSON 结构都不接受声明之外的字段；
//! 4. **重复身份拒绝**（**不**去重）：同一「序列 + 业务期间」出现两次即整体失败。
//!
//! 「形式」列未点名格式的源（S02 / S06 / S08 / S09）与未落地的格式
//! （xlsx / ZIP / XML / Excel）返回
//! [`NotApplicable`](crate::UkCbErrorKind::NotApplicable) —— **不假装已支持**。

use serde::Deserialize;

use crate::error::{UkCbError, UkCbResult};
use crate::value::{
    guard_curve, parse_period, Frequency, UkCbAbsence, UkCbFormat, UkCbObservation, UkCbSeriesId,
    UkCbSourceId, UkCbUnit, UkCbValue,
};

/// CSV 离线交换形态的表头（**逐字固定**）。
///
/// 列顺序与列名都是本层声明的一部分：多、少、改名或换序一律拒绝。
const CSV_HEADER: &str = "series,period,value,unit,frequency,revision";

/// 合成标注的 CSV 前导行前缀。
const CSV_PREAMBLE_SYNTHETIC: &str = "# _synthetic:";
/// 合成说明的 CSV 前导行前缀。
const CSV_PREAMBLE_NOTE: &str = "# _note:";

/// 解析某个声明源的离线样本。
///
/// # Errors
///
/// - `source` 为 [`UkCbSourceId::S07`] → [`UkCbError::RoutedElsewhere`]；
/// - 该源的格式未被清单点名，或点名的格式未落地 → [`UkCbError::NotApplicable`]；
/// - 缺合成标注 → [`UkCbError::SemanticallyRejected`]；
/// - 形态 / 结构非法、未知字段、缺必需字段 → [`UkCbError::Invalid`]；
/// - 序列标识或单位语义不被接受 → [`UkCbError::SemanticallyRejected`]；
/// - 重复身份 → [`UkCbError::SemanticallyRejected`]。
pub fn parse_uk_cb_observations(
    source: UkCbSourceId,
    input: &str,
) -> UkCbResult<Vec<UkCbObservation>> {
    guard_curve(source)?;
    let plan = source.plan();
    let format = plan
        .formats
        .iter()
        .copied()
        .find(|format| format.is_offline_implemented())
        .ok_or_else(|| unimplemented_format(plan.formats))?;

    let observations = match format {
        UkCbFormat::Csv => parse_csv(input)?,
        UkCbFormat::Json => parse_json(input)?,
        other => {
            return Err(UkCbError::NotApplicable(format!(
                "{} 格式的解析栈未落地，不得假装已支持",
                other.as_str()
            )));
        }
    };

    let mut accepted: Vec<UkCbObservation> = Vec::with_capacity(observations.len());
    for observation in observations {
        if accepted
            .iter()
            .any(|seen| seen.series == observation.series && seen.period == observation.period)
        {
            return Err(UkCbError::SemanticallyRejected(format!(
                "重复身份：序列 + 业务期间在样本内出现两次（序列 {}）",
                observation.series
            )));
        }
        accepted.push(observation);
    }
    Ok(accepted)
}

/// 未落地格式的具名错误（不回显任何输入内容）。
fn unimplemented_format(formats: &[UkCbFormat]) -> UkCbError {
    if formats.is_empty() {
        UkCbError::NotApplicable(
            "该源在清单「形式」列未点名格式，离线解析不实现（不得猜格式）".to_owned(),
        )
    } else {
        let names: Vec<&str> = formats.iter().map(|format| format.as_str()).collect();
        UkCbError::NotApplicable(format!(
            "清单点名的格式 {} 均未落地，不得假装已支持",
            names.join(" / ")
        ))
    }
}

/// 解析 CSV 离线交换形态。
fn parse_csv(input: &str) -> UkCbResult<Vec<UkCbObservation>> {
    // UTF-8 BOM 是编码产物、不是数据：容忍一次（常见导出工具会写入）。
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut synthetic: Option<bool> = None;
    let mut note_present = false;
    let mut header_seen = false;
    let mut observations: Vec<UkCbObservation> = Vec::new();

    for (index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim_end_matches('\r');
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix(CSV_PREAMBLE_SYNTHETIC) {
            synthetic = Some(rest.trim() == "true");
            continue;
        }
        if let Some(rest) = line.strip_prefix(CSV_PREAMBLE_NOTE) {
            note_present = !rest.trim().is_empty();
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if !header_seen {
            if line != CSV_HEADER {
                return Err(UkCbError::Invalid(format!(
                    "CSV 表头须逐字为 {CSV_HEADER:?}（第 {} 行）",
                    index + 1
                )));
            }
            header_seen = true;
            continue;
        }
        observations.push(parse_csv_row(line, index + 1)?);
    }

    if !header_seen {
        return Err(UkCbError::Missing("CSV 表头".to_owned()));
    }
    require_synthetic_marker(synthetic, note_present)?;
    for observation in &observations {
        crate::value::validate_observation(observation)?;
    }
    Ok(observations)
}

/// 解析一行 CSV 数据。
fn parse_csv_row(line: &str, line_no: usize) -> UkCbResult<UkCbObservation> {
    if line.contains('"') {
        return Err(UkCbError::Invalid(format!(
            "第 {line_no} 行含引号：本层 CSV 子集不支持引号转义，拒绝以防误解析"
        )));
    }
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() != 6 {
        return Err(UkCbError::Invalid(format!(
            "第 {line_no} 行的列数须为 6，收到 {}",
            fields.len()
        )));
    }
    Ok(UkCbObservation {
        series: UkCbSeriesId::parse(fields[0])?,
        period: parse_period(fields[1])?,
        value: parse_value_token(fields[2]),
        unit: UkCbUnit::parse(fields[3].trim())?,
        frequency: Frequency::parse(fields[4].trim())?,
        revision: optional_token(fields[5]),
    })
}

/// CSV 取值列：空 → 具名缺失；可解析为有限数 → 有值；其余 → 具名缺失（标记不解释）。
fn parse_value_token(token: &str) -> UkCbValue {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return UkCbValue::Absent(UkCbAbsence::BlankInSource);
    }
    match trimmed.parse::<f64>() {
        Ok(value) if value.is_finite() => UkCbValue::Present(value),
        _ => UkCbValue::Absent(UkCbAbsence::ExplicitMarker),
    }
}

/// 可选文本列：空 / 全空白 → `None`。
fn optional_token(token: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// 校验「显式合成标注」。
fn require_synthetic_marker(synthetic: Option<bool>, note_present: bool) -> UkCbResult<()> {
    match synthetic {
        Some(true) => {}
        Some(false) => {
            return Err(UkCbError::SemanticallyRejected(
                "样本声明 `_synthetic: false`：本层只接受显式标注的合成样本".to_owned(),
            ));
        }
        None => {
            return Err(UkCbError::SemanticallyRejected(
                "样本缺少 `_synthetic: true` 标注：本层只接受显式标注的合成样本".to_owned(),
            ));
        }
    }
    if !note_present {
        return Err(UkCbError::SemanticallyRejected(
            "样本缺少非空 `_note` 说明：合成样本须自带可直接阅读的说明".to_owned(),
        ));
    }
    Ok(())
}

/// 解析 JSON 离线交换形态（S11 ONS 面）。
fn parse_json(input: &str) -> UkCbResult<Vec<UkCbObservation>> {
    let raw: RawObservationFile =
        serde_json::from_str(input).map_err(|error| json_error(&error))?;
    require_synthetic_marker(raw.synthetic, has_text(raw.note.as_deref()))?;

    let mut observations: Vec<UkCbObservation> = Vec::with_capacity(raw.observations.len());
    for item in raw.observations {
        let observation = UkCbObservation {
            series: UkCbSeriesId::parse(&item.series)?,
            period: parse_period(&item.period)?,
            value: json_value(item.value)?,
            unit: UkCbUnit::parse(item.unit.trim())?,
            frequency: Frequency::parse(item.frequency.trim())?,
            revision: item.revision.and_then(|text| optional_token(&text)),
        };
        crate::value::validate_observation(&observation)?;
        observations.push(observation);
    }
    Ok(observations)
}

/// JSON 取值：`null` → 空；有限数 → 有值；字符串 → 空或标记；其余 → 拒绝。
fn json_value(value: serde_json::Value) -> UkCbResult<UkCbValue> {
    match value {
        serde_json::Value::Number(number) => match number.as_f64() {
            Some(finite) if finite.is_finite() => Ok(UkCbValue::Present(finite)),
            _ => Err(UkCbError::Invalid("JSON 取值须为有限数值".to_owned())),
        },
        serde_json::Value::Null => Ok(UkCbValue::Absent(UkCbAbsence::BlankInSource)),
        serde_json::Value::String(text) => Ok(UkCbValue::Absent(if text.trim().is_empty() {
            UkCbAbsence::BlankInSource
        } else {
            UkCbAbsence::ExplicitMarker
        })),
        _ => Err(UkCbError::Invalid(
            "JSON 取值只接受数值、null 或字符串标记".to_owned(),
        )),
    }
}

/// 是否有可读文本。
fn has_text(text: Option<&str>) -> bool {
    text.is_some_and(|value| !value.trim().is_empty())
}

/// 内容无关的 JSON 诊断：只给行列位置，**不**回显正文或取值片段。
fn json_error(error: &serde_json::Error) -> UkCbError {
    UkCbError::Invalid(format!(
        "JSON 形态非法（第 {} 行第 {} 列）",
        error.line(),
        error.column()
    ))
}

/// JSON 根对象。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawObservationFile {
    #[serde(rename = "_synthetic", default)]
    synthetic: Option<bool>,
    #[serde(rename = "_note", default)]
    note: Option<String>,
    observations: Vec<RawObservation>,
}

/// JSON 观测项。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawObservation {
    series: String,
    period: String,
    value: serde_json::Value,
    unit: String,
    frequency: String,
    #[serde(default)]
    revision: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::parse_uk_cb_observations;
    use crate::error::UkCbErrorKind;
    use crate::value::{
        Frequency, Period, UkCbAbsence, UkCbSourceId, UkCbUnit, UkCbValue, IADB_KEY_CODES,
    };

    const CSV: &str = include_str!("../tests/fixtures/iadb_key_series.csv");
    const JSON: &str = include_str!("../tests/fixtures/ons_cpi.json");

    fn csv_with(rows: &str) -> String {
        format!(
            "# _synthetic: true\n# _note: 合成样本\nseries,period,value,unit,frequency,revision\n{rows}"
        )
    }

    #[test]
    fn parses_the_csv_fixture() {
        let observations =
            parse_uk_cb_observations(UkCbSourceId::S01, CSV).expect("合成夹具应可解析");
        assert_eq!(observations.len(), 6);
        assert_eq!(observations[0].series.as_str(), "IUDBEDR");
        assert_eq!(observations[0].value.present(), Some(4.0));
        assert_eq!(observations[0].unit, UkCbUnit::Percent);
        assert_eq!(observations[0].frequency, Frequency::Daily);
        assert_eq!(
            observations[0].period,
            Period::Day(crate::Date::parse("2026-09-18").expect("日期"))
        );
    }

    #[test]
    fn parses_the_json_fixture() {
        let observations =
            parse_uk_cb_observations(UkCbSourceId::S11, JSON).expect("合成夹具应可解析");
        assert_eq!(observations.len(), 3);
        assert_eq!(observations[0].frequency, Frequency::Monthly);
        assert_eq!(
            observations[2].value,
            UkCbValue::Absent(UkCbAbsence::BlankInSource)
        );
    }

    #[test]
    fn curve_source_is_routed_before_anything_else() {
        assert_eq!(
            parse_uk_cb_observations(UkCbSourceId::S07, CSV)
                .expect_err("曲线必须路由")
                .kind(),
            UkCbErrorKind::RoutedElsewhere
        );
    }

    #[test]
    fn sources_without_declared_format_are_not_applicable() {
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

    #[test]
    fn unknown_header_column_is_rejected() {
        let input = "# _synthetic: true\n# _note: 合成\nseries,period,value,unit,frequency,revision,extra\n";
        assert_eq!(
            parse_uk_cb_observations(UkCbSourceId::S01, input)
                .expect_err("未知列")
                .kind(),
            UkCbErrorKind::Invalid
        );
    }

    #[test]
    fn missing_required_column_is_rejected() {
        let input = "# _synthetic: true\n# _note: 合成\nseries,period,value,unit,frequency\n";
        assert_eq!(
            parse_uk_cb_observations(UkCbSourceId::S01, input)
                .expect_err("缺列")
                .kind(),
            UkCbErrorKind::Invalid
        );
    }

    #[test]
    fn illegal_period_is_rejected() {
        let input = csv_with("IUDBEDR,2026-9-18,4.0,percent,daily,\n");
        assert_eq!(
            parse_uk_cb_observations(UkCbSourceId::S01, &input)
                .expect_err("非法期间")
                .kind(),
            UkCbErrorKind::Invalid
        );
    }

    #[test]
    fn duplicate_identity_is_rejected() {
        let input = csv_with(
            "IUDBEDR,2026-09-18,4.0,percent,daily,\nIUDBEDR,2026-09-18,4.0,percent,daily,\n",
        );
        assert_eq!(
            parse_uk_cb_observations(UkCbSourceId::S01, &input)
                .expect_err("重复身份")
                .kind(),
            UkCbErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn unmarked_sample_is_rejected() {
        let input =
            "series,period,value,unit,frequency,revision\nIUDBEDR,2026-09-18,4.0,percent,daily,\n";
        assert_eq!(
            parse_uk_cb_observations(UkCbSourceId::S01, input)
                .expect_err("缺标注")
                .kind(),
            UkCbErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn quoted_field_is_rejected_instead_of_misparsed() {
        let input = csv_with("\"IUDBEDR,2026-09-18\",4.0,percent,daily,\n");
        assert_eq!(
            parse_uk_cb_observations(UkCbSourceId::S01, &input)
                .expect_err("引号")
                .kind(),
            UkCbErrorKind::Invalid
        );
    }

    #[test]
    fn blank_value_is_named_absence_not_zero() {
        let input = csv_with("IUDBEDR,2026-09-18,,percent,daily,\n");
        let observations = parse_uk_cb_observations(UkCbSourceId::S01, &input).expect("空值合法");
        assert_eq!(
            observations[0].value,
            UkCbValue::Absent(UkCbAbsence::BlankInSource)
        );
        assert_eq!(observations[0].value.present(), None);
    }

    #[test]
    fn unknown_unit_token_is_semantically_rejected() {
        let input = csv_with("IUDBEDR,2026-09-18,4.0,billions,daily,\n");
        assert_eq!(
            parse_uk_cb_observations(UkCbSourceId::S01, &input)
                .expect_err("未知单位")
                .kind(),
            UkCbErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn json_unknown_field_is_rejected() {
        let input = JSON.replace(
            r#""series": "ONS.CPI.YOY","#,
            r#""series": "ONS.CPI.YOY", "extra": 1,"#,
        );
        assert_eq!(
            parse_uk_cb_observations(UkCbSourceId::S11, &input)
                .expect_err("未知字段")
                .kind(),
            UkCbErrorKind::Invalid
        );
    }

    #[test]
    fn json_duplicate_identity_is_rejected() {
        let input = JSON.replace(r#""series": "ONS.RPI.YOY""#, r#""series": "ONS.CPI.YOY""#);
        assert_eq!(
            parse_uk_cb_observations(UkCbSourceId::S11, &input)
                .expect_err("重复身份")
                .kind(),
            UkCbErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn error_messages_do_not_echo_values() {
        let input = csv_with("IUDBEDR,2026-09-18,SECRET-TOKEN-LOOKING,percent,daily,\n");
        let observations =
            parse_uk_cb_observations(UkCbSourceId::S01, &input).expect("非数值标记合法");
        assert_eq!(
            observations[0].value,
            UkCbValue::Absent(UkCbAbsence::ExplicitMarker)
        );
        let text = parse_uk_cb_observations(UkCbSourceId::S01, "IUDBEDR,2026-9-18,SECRET,x,y,\n")
            .expect_err("非法期间")
            .to_string();
        assert!(!text.contains("SECRET"));
    }

    #[test]
    fn leading_bom_is_tolerated() {
        let input = csv_with("IUDBEDR,2026-09-18,4.0,percent,daily,\n");
        let with_bom = format!("\u{feff}{input}");
        let observations =
            parse_uk_cb_observations(UkCbSourceId::S01, &with_bom).expect("BOM 是编码产物");
        assert_eq!(observations.len(), 1);
    }

    #[test]
    fn key_code_helper_is_strict() {
        assert_eq!(IADB_KEY_CODES.len(), 6);
        assert!(crate::value::iadb_code("IUDBEDR").is_ok());
        assert_eq!(
            crate::value::iadb_code("IUDBEDX")
                .expect_err("非种子码")
                .kind(),
            UkCbErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn json_structural_values_are_rejected() {
        for value in ["true", "[]", "{\"unknown\":123}"] {
            let input = format!(
                r#"{{"_synthetic":true,"_note":"合成回归","observations":[{{"series":"ONS.CPI.YOY","period":"2026-09","value":{value},"unit":"percent","frequency":"monthly"}}]}}"#
            );
            assert!(parse_uk_cb_observations(UkCbSourceId::S11, &input).is_err());
        }
    }
}
