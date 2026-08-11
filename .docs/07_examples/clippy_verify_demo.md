# clippy_verify_demo — Phase 3 GOAT gate

**Source:** [`examples/clippy_verify_demo.rs`](../../examples/clippy_verify_demo.rs)
**Feature:** `clippy_verify` (opt-in — requires cargo+clippy on PATH)
**Run:** `cargo run --example clippy_verify_demo --features clippy_verify --release`

## What it demonstrates

The Phase 3 GOAT gate for truthful lint resolution via real `cargo clippy`:

- **G1 (correctness)**: buggy code → `lint_resolved=false`; fixed code →
  `lint_resolved=true`. Uses `needless_return` as the test case.
- **G2 (latency)**: documents cold vs warm run times honestly (seconds,
  not µs — I/O bound).
- **G3 (no-regression)**: L2 syntax check (fast path) still works;
  `validate_with_clippy` (slow path) composes correctly.
- **G4 (alloc-free)**: N/A — documented honestly (process spawn + temp
  I/O, not a hot-path concern).

## Why this is a separate example (not a bench)

This GOAT gate is **slow** (~130ms/verify). It runs real `cargo clippy`
multiple times. Putting it in `benches/` would make `cargo bench` slow
and would require `clippy_verify` to be a default feature. Instead, it's
an opt-in example that runs on-demand.

## The honesty gap it closes

Before Phase 3, the L2 `validate()` method always returned
`lint_resolved = true` — a documented lie. The fast path doesn't shell
out to `cargo clippy`, so it can't truthfully claim the lint is resolved.

Phase 3 adds `validate_with_clippy()` which DOES shell out, making
`lint_resolved` truthful. This example proves the truthful path works.

## See also

- [`../05_clippy_verify/clippy_verifier.md`](../05_clippy_verify/clippy_verifier.md) — the verifier API
- [`../04_pruner/honesty_contract.md`](../04_pruner/honesty_contract.md) — the honesty contract
