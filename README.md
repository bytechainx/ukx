# ukx

`ukx` 是英国央行数据源（`source_id = uk_cb`）的**源事实层**：把清单声明的源登记、
关键 IADB 码、曲线路由禁则与规划调度落成可编译、可测、可判定的 Rust 库，
并提供**离线**解析与 fail-closed 授权判定。

- **零网络**：没有 HTTP 客户端、没有异步运行时、没有端点字面量、没有凭据读取。
- **fail-closed**：本源授权为 `unknown`（无 Owner 签核文件；Series / UA / 许可 UNKNOWN）。
- **类型化源事实**：S01–S11 源登记、6 个关键 IADB 码、7 种源侧形式、观测值对象。
- **可判定的禁则**：S07 收益率曲线路由 `yield_curve`，**不得**直写观测。
- **诚实边界**：只实现 CSV 与 JSON 两种离线形态；xlsx / ZIP / XML / Excel 与
  「形式」列未点名格式的源一律返回 `NotApplicable`，**不假装已支持**。

## 安装

本 crate **不发布到 crates.io**，仅以 GitHub 源码 / **git 依赖**形式引入：

```toml
[dependencies]
ukx = { git = "https://github.com/bytechainx/ukx" }
```

本地同仓开发也可用路径引入：

```toml
ukx = { path = "../ukx" }
```

## 用法

最小用法：解析一份**显式标注**的合成样本，读回结构化观测。

```rust
use ukx::{parse_uk_cb_observations, UkCbSourceId};

let input = "# _synthetic: true\n\
# _note: 合成样本，非真实源数据\n\
series,period,value,unit,frequency,revision\n\
IUDBEDR,2026-09-18,4.00,percent,daily,\n";

let observations = parse_uk_cb_observations(UkCbSourceId::S01, input)?;
assert_eq!(observations[0].series.as_str(), "IUDBEDR");
assert_eq!(observations[0].value.present(), Some(4.0));
# Ok::<(), ukx::UkCbError>(())
```

曲线路由与关键码守卫：

```rust
use ukx::{guard_curve, iadb_code, UkCbErrorKind, UkCbSourceId};

assert_eq!(
    guard_curve(UkCbSourceId::S07).unwrap_err().kind(),
    UkCbErrorKind::RoutedElsewhere
);
assert_eq!(iadb_code("IUDBEDR")?.as_str(), "IUDBEDR");
assert_eq!(
    iadb_code("IUDBEDX").unwrap_err().kind(),
    UkCbErrorKind::SemanticallyRejected
);
# Ok::<(), ukx::UkCbError>(())
```

## 能力矩阵

| 能力 | 生产入口 / 类型 | 边界 |
| --- | --- | --- |
| 源登记 | `UkCbSourceId` / `SOURCE_PLANS` | 11 个源；清单「形式」「频率」逐字保留 |
| 关键 IADB 码 | `UkCbIadbCode` / `IADB_KEY_CODES` / `iadb_code` | 严格 6 个种子；**不得扩集** |
| 曲线路由 | `guard_curve` / `CURVE_ROUTE` | S07 → `yield_curve`（#11 `boe_iadb` / #12 `uk_dmo`） |
| 规划调度 | `PLANNED_SCHEDULE` | **规划语义**，不是访问合同 |
| 离线解析 | `parse_uk_cb_observations` | 仅 CSV 与 JSON；未知字段 / 重复身份 / 未标注样本原子失败 |
| 观测校验 | `validate_observation` | 期间、取值有限性、修订、频率与期间一致性 |
| 缺失表达 | `UkCbValue` / `UkCbAbsence` | 具名缺失；**不**静默转 0 |
| 授权判定 | `authorize` / `ensure_authorized` | 六条拒绝路径 fail-closed |
| publication | `publication_semantics` | 恒 `Date` + `Inferred` + `NotEligible` |

## 解析与拒绝口径

| 情形 | 结果 |
| --- | --- |
| S07 收益率曲线 | `RoutedElsewhere`（**先于**格式判定） |
| S02 / S06 / S08 / S09（清单未点名格式） | `NotApplicable` |
| xlsx / ZIP / XML / Excel 格式 | `NotApplicable`（解析栈未落地） |
| `_synthetic` 缺失或为 `false`、`_note` 为空 | `SemanticallyRejected` |
| CSV 表头多列 / 少列 / 改名 / 换序 | `Invalid`（原子失败） |
| 字段含引号（本层 CSV 子集不支持） | `Invalid`（拒绝以防误解析） |
| JSON 未知字段 / 缺必需字段 | `Invalid` |
| 同一「序列 + 业务期间」出现两次 | `SemanticallyRejected`（**拒绝**，不去重） |
| 非种子 IADB 码、未知单位 | `SemanticallyRejected` |
| 期间形态非法、取值非有限 | `Invalid` |

错误消息只给形态与位置，不回显原始正文、取值片段或凭据。

## 缺失取值

`UkCbValue` 只有两种形态：`Present(f64)` 与 `Absent(UkCbAbsence)`。
缺失理由是具名的（`BlankInSource` / `ExplicitMarker`），
`present()` 对缺失返回 `None` —— **永远不会**返回 0。

## 非目标

不做联网采集与传输层；不猜端点、Header、限流数字或 Series 全集；
不实现 xlsx / ZIP / XML / Excel 解析栈；不做存储与再分发；不做单位换算；
不算派生指标；不宣称 package stable、SLA 或新鲜度保证。

## 门禁

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo package --no-verify
```

`production_decision = NO-GO`。清单 COMPLETE ≠ ship；authorization ≠ Production Ready。

## 许可

MIT OR Apache-2.0
