# 08 — Benchmarks

> **Purpose:** GOAT gate methodology, measured results, and re-run
> instructions. Every primitive ships behind a GOAT gate that must pass
> before promotion.

## Files

| File | Scope |
|---|---|
| [`goat_methodology.md`](goat_methodology.md) | The 4-gate pattern (G1 correctness, G2 perf, G3 no-regression, G4 alloc-free) |
| [`measured_results.md`](measured_results.md) | Consolidated results (M3 Max, commit `5b46556`) |
| [`clippy_bench.md`](clippy_bench.md) | The frozen real-code healing bench + SVG score curve + the generation ratchet (Plan 014 + M5 gens 2–8, Issue 089 T7's shaped-verdict gen) |
| [`entropy_gap_ood_axis.md`](entropy_gap_ood_axis.md) | The entropy-gap OOD axis on the bench (issue 077 T1/T4 — consumes `katgpt_core::regime_probe`; set choice, sign convention, first measured row) |
| [`placebo_arm.md`](placebo_arm.md) | The three-arm attribution shape for gate-shaped features — judgment vs revision-opportunity (issue 195, the JevAct Table-3 lesson); `--arm PLACEBO` + `FROZEN --placebo` |

## Persisted results

- [`.benchmarks/001_goat_consolidated.md`](../../.benchmarks/001_goat_consolidated.md) — L1 + L2 + Phase 3 + L3
- [`.benchmarks/008_real_world_validation.md`](../../.benchmarks/008_real_world_validation.md) — self_evolve G6/G7 (real cargo clippy)
- [`.benchmarks/013_rust_perf_bench_pareto_goat.md`](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) — rust_perf Pareto validator promotion gate (historical; pre-expansion corpus snapshots)
- [`.benchmarks/015_rust_perf_expansion_phase4_goat.md`](../../.benchmarks/015_rust_perf_expansion_phase4_goat.md) — rust_perf Phase 4 finalization on the expanded 39-rule corpus (29/31 ParetoDominant)
- [`.benchmarks/018_plan005_latent_suggest_goat.md`](../../.benchmarks/018_plan005_latent_suggest_goat.md) — Plan 005 latent `--suggest` GOAT (G1–G4 ALL PASS: 100% precision, 349 µs/chunk)
- [`.benchmarks/019_plan005_phase6_fix_span_preserving_goat.md`](../../.benchmarks/019_plan005_phase6_fix_span_preserving_goat.md) — Plan 005 Phase 6 span-preserving `--fix` GOAT (G1–G4 PASS; G5 PASS via the addendum — real fixes landed in katgpt-rs `katgpt-pruners` after the compile gate caught 3 proposer bug classes, all fixed at root cause)

## Re-run commands

```sh
# L1 (hot path)
cargo bench --bench bench_001_l1_matvec

# L2 (hot path)
cargo bench --bench bench_002_l2_pruner_syn

# L3 (opt-in refinement)
cargo run --example ruliology_search_demo --features ruliology_search --release

# Phase 3 (opt-in verification — requires cargo+clippy on PATH)
cargo run --example clippy_verify_demo --features clippy_verify --release

# self_evolve real-world validation (opt-in — requires cargo+clippy on PATH, ~30s)
cargo run --example self_evolve_real_world_demo --features "self_evolve clippy_verify" --release

# clippy-bench — the frozen healing score (Plan 014; run + append + render SVG)
cargo run --features score_bench --bin cargo-refine -- --score-bench

# Plan 005 latent suggest GOAT (opt-in; G2 gate is release-only)
cargo test --features latent_retrieval --test latent_suggest_goat -- --nocapture
cargo test --release --features latent_retrieval --test latent_suggest_goat -- --nocapture
```

## See also

- [`../01_orientation/build_commands.md`](../01_orientation/build_commands.md) — full build commands
- [`../../scripts/ci_feature_guard.sh`](../../scripts/ci_feature_guard.sh) — CI gate
