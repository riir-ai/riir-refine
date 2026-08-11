# 08 — Benchmarks

> **Purpose:** GOAT gate methodology, measured results, and re-run
> instructions. Every primitive ships behind a GOAT gate that must pass
> before promotion.

## Files

| File | Scope |
|---|---|
| [`goat_methodology.md`](goat_methodology.md) | The 4-gate pattern (G1 correctness, G2 perf, G3 no-regression, G4 alloc-free) |
| [`measured_results.md`](measured_results.md) | Consolidated results (M3 Max, commit `5b46556`) |

## Persisted results

- [`.benchmarks/001_goat_consolidated.md`](../../.benchmarks/001_goat_consolidated.md) — L1 + L2 + Phase 3 + L3

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
```

## See also

- [`../01_orientation/build_commands.md`](../01_orientation/build_commands.md) — full build commands
- [`../../scripts/ci_feature_guard.sh`](../../scripts/ci_feature_guard.sh) — CI gate
