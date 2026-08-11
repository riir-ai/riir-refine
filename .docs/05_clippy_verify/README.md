# 05 — clippy_verify (Phase 3 lint resolution)

> **Purpose:** truthful lint resolution verification via real
> `cargo clippy` re-run. Opt-in because it's slow (~130ms/verify) and
> requires cargo+clippy on PATH. The fast L2 `syn` check stays always-on.

## Files

| File | Scope |
|---|---|
| [`clippy_verifier.md`](clippy_verifier.md) | `ClippyVerifier` + `ClippyVerifyResult` + temp-crate scaffolding |

## Source

- [`src/clippy_verifier.rs`](../../src/clippy_verifier.rs)

## Feature gate

`clippy_verify` (opt-in). Default-off because:

1. **Slow**: ~130ms/verify (temp dir creation + `cargo clippy` subprocess)
2. **External dependency**: requires `cargo` + `clippy` on PATH
3. **Temp I/O**: creates a temp crate dir per verification

The fast path (L2 `syn` check, ~5.5µs) is always-on and is the
load-bearing validation for the inner loop.

## See also

- [`../04_pruner/honesty_contract.md`](../04_pruner/honesty_contract.md) —
  why the fast path's `lint_resolved` is always `true`
