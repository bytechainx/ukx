#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! ukx 热路径微基准：CSV 离线解析 + 曲线路由守卫 + 关键码选择。
//!
//! 离线、无网络、无凭据；输入为**合成样本**（见 `tests/fixtures/`）。
//! 本基准只用于观察本层纯函数开销，**不构成**任何能力或 SLA 声明。

use std::hint::black_box;
use std::time::Instant;

use ukx::{guard_curve, iadb_code, parse_uk_cb_observations, UkCbSourceId};

const ITERATIONS: u32 = 2_000;

const INPUT: &str = "# _synthetic: true\n\
# _note: 微基准内联合成样本，不是真实源数据。\n\
series,period,value,unit,frequency,revision\n\
IUDBEDR,2026-09-18,4.00,percent,daily,\n\
IUDSOIA,2026-09-18,3.95,percent,daily,\n\
RPWB55A,2026-09-18,1050000,native,weekly,\n";

fn main() {
    let start = Instant::now();
    let mut parsed = 0usize;
    for _ in 0..ITERATIONS {
        let observations = parse_uk_cb_observations(black_box(UkCbSourceId::S01), black_box(INPUT))
            .expect("合成样本应可解析");
        parsed += observations.len();
    }
    let parse_elapsed = start.elapsed();

    let start = Instant::now();
    let mut guarded = 0usize;
    for _ in 0..ITERATIONS {
        if guard_curve(black_box(UkCbSourceId::S07)).is_err() {
            guarded += 1;
        }
    }
    let guard_elapsed = start.elapsed();

    let start = Instant::now();
    let mut selected = 0usize;
    for _ in 0..ITERATIONS {
        for code in [
            "IUDBEDR", "IUDSOIA", "RPWB55A", "LPMAUYN", "XUDLUSS", "XUDLERI",
        ] {
            if iadb_code(black_box(code)).is_ok() {
                selected += 1;
            }
        }
    }
    let select_elapsed = start.elapsed();

    println!(
        "bench_ukx_parse_csv: iters={ITERATIONS} observations={parsed} total={parse_elapsed:?} per_iter={:?}",
        parse_elapsed / ITERATIONS
    );
    println!(
        "bench_ukx_curve_guard: iters={ITERATIONS} routed={guarded} total={guard_elapsed:?} per_iter={:?}",
        guard_elapsed / ITERATIONS
    );
    println!(
        "bench_ukx_iadb_select: iters={ITERATIONS} selected={selected} total={select_elapsed:?} per_period={:?}",
        select_elapsed / ITERATIONS
    );
}
