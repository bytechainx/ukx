# Changelog — ukx

本文件记录 `ukx` 的用户可见变更，遵循 [Keep a Changelog](https://keepachangelog.com/)
与 [Semantic Versioning](https://semver.org/)。

本 crate 为特性 005（数据模块独立 API 库）新建，版本从 `0.1.0` 起算。

## [0.1.1] - 2026-09-23

### 修正

- JSON 解析拒绝布尔、数组与对象取值；授权拒绝非法日期和倒置有效期。
- 按版本规则「实现向契约靠拢」推进 PATCH，不新增公共 API。

## [Unreleased]

## [0.1.0] - 2026-09-22

### 新增

- 从 `specs/adapter/uk_cb.md` 的源事实新建独立 crate，`source_id = uk_cb`，
  零内部依赖、零网络、零凭据。
- `src/error.rs`：`UkCbErrorKind`（8 个语义分类）、`UkCbError` 与 `UkCbResult<T>`；
  `is_retryable` 除 `Invariant` 外恒为 `false`（本层无网络）。
- `src/value.rs`：严格 `YYYY-MM-DD` 的 `Date`（月 / 日 / 闰年校验）、`Period`、
  `parse_period`（`YYYY-MM-DD` / `YYYY-MM` / `YYYY-Qn` / `YYYY` / `event:…`）、
  `Frequency`、源侧单位 `UkCbUnit`。
- `src/value/source.rs`：S01–S11 源登记 `SOURCE_PLANS`（标识 + 名称 + 清单「形式」逐字文本 +
  点名格式 + 「频率」逐字文本）、按标识取登记条目的 `UkCbSourceId::plan`、`UkCbFormat`、
  6 个关键 IADB 码 `IADB_KEY_CODES` 与严格选择守卫 `iadb_code`、曲线路由
  `CURVE_ROUTE` / `guard_curve`、规划调度 `PLANNED_SCHEDULE`。
- `src/value/observation.rs`：`UkCbSeriesId`（原样保留 + 字符集校验 + 关键码命中判定）、
  `UkCbValue` / `UkCbAbsence`（缺失取值的具名表达，不静默转 0）、
  `UkCbObservation` 与 `validate_observation`。
- `src/parse.rs`：离线解析 `parse_uk_cb_observations(source, input)`；
  曲线路由优先、未知字段原子失败、重复身份拒绝、只接受显式标注的合成样本、
  仅实现 CSV 与 JSON 两种形态。
- `src/authz.rs`：fail-closed 授权判定 `authorize` / `ensure_authorized`。
- `src/pit.rs`：publication 语义三元组，恒 `Date` + `Inferred` + `NotEligible`。
- `tests/{tdd_contracts,sdd_spec,aidd_boundary}.rs` 与合成夹具
  `tests/fixtures/iadb_key_series.csv`、`tests/fixtures/ons_cpi.json`；
  `benches/hot_path.rs` 微基准。
- `src/` 每个模块（除门面 `lib.rs`）均带**与源码同文件**的内联单元测试
  （组织 P0：`testing.md` §1.1）。

### 变更

- **TDD-PROBE 入口列格式归一（仅测试面，公开 API 一字未改）**：按
  `specs/features/002-public-api-compliance-and-test-tiers/contracts/public-api-contract.md` 的
  机器解析格式，把 `tests/tdd_contracts.rs` 的入口列由「带签名」改写为
  `类型::方法` 或裸函数名（例：`parse_uk_cb_observations`、`UkCbIadbCode::from_code`）。
  该格式是 C2 精确集合比对与 `entryIsReal` 判定的共同基准；变异列 / 红列 / 绿列未改动。
- 入口表 33 行与 33 条语义变异一一对应，`/tmp` 变异副本实跑 **33/33 被捕获**
  （先跑未变异对照为绿，再逐条施加变异并断言红）。

### 说明

- `production_decision = NO-GO`：本源无 Owner 签核文件，`authorization = unknown`
  （Series / UA / 许可 UNKNOWN），故不实现采集、不引入 HTTP 客户端、不猜端点。
- 清单「形式」列未点名格式的源（S02 / S06 / S08 / S09）与未落地的格式
  （xlsx / ZIP / XML / Excel）返回 `NotApplicable`，**不假装已支持**。
- S07 收益率曲线路由 `yield_curve`，**不得**直写观测。
- 夹具全部为**合成样本**，不是真实源数据，不构成任何证据。
