# Build commands

## Default (modelless path only)

```bash
cargo check
cargo clippy --all-targets
cargo test --lib
```

## Opt-in features

```bash
# Phase 3 lint resolution verification via real cargo clippy (Issue 003)
cargo check --features clippy_verify

# L3 ruliology Pareto-front candidate search (Issue 005)
cargo check --features ruliology_search

# Issue 004: latent-first self-evolution loop (staged, pending commit)
cargo check --features self_evolve

# Every feature at once — catches combo-only regressions
cargo check --all-features
```

## Full CI guard (5 layers)

```bash
./scripts/ci_feature_guard.sh
```

The 5-layer pattern mirrors sibling repos (`riir-chain`,
`riir-neuron-db`):

| Layer | What | Catches |
|---|---|---|
| 1 | `--no-default-features` baseline | Missing katgpt-core dep |
| 2 | Default features | Core path regressions |
| 3 | Per-feature `cargo hack --each-feature` sweep | Single-feature breaks |
| 4 | `--all-features` combo check | Feature-combo regressions (the `merkle_root`/`can_freeze` class) |
| 5 | Clippy (`-D warnings`) + lib tests | Lint + test regressions |

Requires `cargo-hack` for layer 3 (gracefully skips if absent). No Lean
proof gate (repo has no `.proofs/`), no wasm32 gate (`wasm_pruner`
deferred).

## GOAT benches

```bash
# L1 hot path (ternary SIMD matvec)
cargo bench --bench bench_001_l1_matvec

# L2 hot path (syn syntax parse check)
cargo bench --bench bench_002_l2_pruner_syn

# L3 opt-in refinement (Pareto-front candidate search)
cargo run --example ruliology_search_demo --features ruliology_search --release

# Phase 3 opt-in verification (requires cargo+clippy on PATH)
cargo run --example clippy_verify_demo --features clippy_verify --release
```

Results persisted in [`.benchmarks/001_goat_consolidated.md`](../../.benchmarks/001_goat_consolidated.md).

## See also

- [`../08_benchmarks/`](../08_benchmarks/README.md) — GOAT gate methodology
- [`../../scripts/ci_feature_guard.sh`](../../scripts/ci_feature_guard.sh) — the CI script
