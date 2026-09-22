# ukx 公开 API

本文对应 `ukx 0.1.0` 的公开消费面（`source_id = uk_cb`）。
本层是**离线源事实层**：无网络、无凭据、无存储。

## 公开消费面

| 能力 | API | 一行语义 |
|------|-----|----------|
| 离线解析 | `parse_uk_cb_observations(UkCbSourceId, &str)` | 源标识 + 字符串 → 观测集合；曲线路由优先、未知字段 / 重复身份 / 未标注样本原子失败 |
| 曲线路由守卫 | `guard_curve(UkCbSourceId)` | S07 → `RoutedElsewhere`，不得直写观测 |
| 关键码选择 | `iadb_code(&str)` / `UkCbIadbCode::from_code(&str)` | 严格 6 个种子；非种子码 → `SemanticallyRejected` |
| 观测校验 | `validate_observation(&UkCbObservation)` | 期间、取值有限性、修订、频率与期间一致性 |
| 期间解析 | `parse_period(&str)` | `YYYY-MM-DD` / `YYYY-MM` / `YYYY-Qn` / `YYYY` / `event:YYYY-MM-DD` |
| 授权判定 | `authorize(Option<&UkCbAuthorizationEvidence>, Date)` | fail-closed；六条拒绝路径 |
| 授权判定（`?` 友好） | `ensure_authorized(Option<&UkCbAuthorizationEvidence>, Date)` | 拒绝时返回 `AuthorizationDenied` |
| publication 语义 | `publication_semantics()` | 恒 `(Date, Inferred, NotEligible)` |
| PIT 资格 | `is_formal_pit_eligible()` | 恒 `false` |
| 错误模型 | `UkCbError` / `UkCbErrorKind` / `UkCbResult<T>` | 8 个语义分类 + `kind()` / `is_retryable()` |

## 登记与常量

| 类型 / 常量 | 语义 |
|------|------|
| `UkCbSourceId` | S01–S11；`as_str` / `is_curve` / `plan` |
| `UkCbSourcePlan` / `SOURCE_PLANS` | 每源的标识、名称、清单「形式」逐字文本、点名格式、「频率」逐字文本 |
| `UkCbFormat` | `Csv` / `Xml` / `Excel` / `Json` / `Xlsx` / `Zip`；`is_offline_implemented` 仅 CSV 与 JSON 为真 |
| `UkCbIadbCode` / `IADB_KEY_CODES` | 清单 §1.2 的 6 个关键码（规划白名单种子）；**不得扩集** |
| `CurveRoute` / `CURVE_ROUTE` | `yield_curve` 槽位与 #11 `boe_iadb` / #12 `uk_dmo` |
| `UkCbScheduleWindow` / `PLANNED_SCHEDULE` | 8 条**规划**调度窗口（UTC）；不是访问合同 |
| `SOURCE_ID` / `AUTHORIZATION_STATUS` / `AUTHORIZATION_EVIDENCE` | 名册登记值的只读引用 |

## 值对象

| 类型 | 语义 |
|------|------|
| `Date` / `Period` | 严格 `YYYY-MM-DD` 日期与业务期间（日 / 月 / 季 / 年 / 事件）；`Date::new` / `Date::parse` / `Date::validate` / `Date::is_leap_year` |
| `Frequency` | 七档频率（daily … irregular），`as_str` / `parse` 可回读 |
| `UkCbUnit` | 源侧单位（`Unspecified` / `Native` / `Percent`）；本层不换算、不据序列名推断 |
| `UkCbSeriesId` | 序列标识 newtype：原样保留 + 字符集校验；`as_str()` 取文本、`iadb_code()` 判是否命中 6 个关键码 |
| `UkCbValue` / `UkCbAbsence` | 取值或**具名缺失**（空值 / 显式标记）；`present()` 缺失返回 `None`、`is_absent()` 判缺失，**不**静默转 0 |
| `UkCbObservation` | 观测：序列 + 期间 + 值 + 单位 + 频率 + 修订；`revision()` 在无 vintage 面时为 `None` |

## 选择规则

1. 解析只接受**显式标注**的合成样本（`_synthetic: true` + 非空 `_note`）。
   本层无采集授权，不接受未标注或自称真实的样本。
2. 曲线源（S07）在任何其他判定之前被路由；「先路由、后格式」不可调换。
3. 只有 CSV 与 JSON 被实现；「形式」列未点名格式的源（S02 / S06 / S08 / S09）
   会得到 `NotApplicable` —— 这是**设计**，不是缺口待补。
4. 重复身份选择**拒绝**而不是去重（口径见 `docs/标准.md` §7）。
5. 关键 IADB 码只用 `iadb_code` 判定；全集未钉死，**不得**自行扩集或近义替换。
6. 缺失取值请用 `UkCbValue::present()`：缺失返回 `None`，永远不会返回 0。
7. 判定授权用 `authorize` / `ensure_authorized`；本源当前无签核文件，正常结果是 `Denied`。

## 明确不做

- 联网采集、传输层、重试与限流（无授权，`FR-037`）。
- xlsx / ZIP / XML / Excel 解析栈（未落地；调用方会得到 `NotApplicable`）。
- 端点的 URL / Header / 限流数字 / Series 全集（清单禁止猜）。
- 派生指标与单位换算；存储与再分发。
