# GOAT gate methodology

Every new primitive ships behind a GOAT gate that must pass before
promotion to default-on. The gate has four levels:

## G1 — Correctness / determinism

What it checks: the primitive produces the correct result. For
deterministic primitives (L1 matvec, L2 parse check), this is
bit-identical output on repeated runs. For search primitives (L3
Pareto-front), this is the correct filtering + selection.

How it's measured: two consecutive runs, bit-identical comparison (for
deterministic primitives) or property-based assertions (for search
primitives).

## G2 — Performance

What it checks: the primitive meets its latency target.

| Layer | Target | Measured | Margin |
|---|---|---|---|
| L1 matvec | p50 < 10µs | 2.250µs | 4.4× under |
| L2 syn parse | p50 < 100µs | 5.459µs | 18.3× under |
| L3 search (10 candidates) | < 10ms | 392.96µs | 25.5× under |
| Phase 3 clippy verify | documented (~130ms) | ~130ms | not hot-path |

How it's measured: `cargo bench` (L1, L2) or `cargo run --release`
(L3, Phase 3), 10,000 iterations, p50/p99/mean.

## G3 — No-regression

What it checks: the primitive doesn't break existing behavior. For
feature-gated primitives, this means default features still pass all
tests. For always-on primitives, this means the full test suite passes.

How it's measured: `cargo test --lib` before and after the primitive
ships.

## G4 — Alloc-free (or bounded)

What it checks: the primitive doesn't allocate on the hot path. For
hot-path primitives (L1, L2), this is zero steady-state allocation
(scratch buffer capacity stable across 1000 calls). For non-hot-path
primitives (L3, Phase 3), this is documented N/A.

How it's measured: scratch buffer capacity comparison before/after 1000
iterations, or thread-local allocation counters.

## Promotion rule

Per the global AGENTS.md: promotion to default-on requires a
**modelless** gain. A perf gain on a biased/incorrect answer is NOT a
modelless gain — it's a speedup of a wrong result. The quality gate
(G1) must pass modellessly for the GOAT to hold.

All four layers (L1, L2, Phase 3, L3) are modelless — no training, no
backprop. They pass the promotion rule.

## See also

- [`measured_results.md`](measured_results.md) — the actual numbers
- [`../../.benchmarks/001_goat_consolidated.md`](../../.benchmarks/001_goat_consolidated.md) — persisted record
