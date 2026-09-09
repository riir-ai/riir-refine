# 09 — Lessons

> **Purpose:** canonical patterns, design decisions, and honest POC
> caveats recorded for future contributors. Each lesson is grounded in a
> concrete precedent.

## Files

| File | Scope |
|---|---|
| [`modelless_first.md`](modelless_first.md) | The modelless-first mandate — why L0–L3 are modelless |
| [`borrow_without_dep.md`](borrow_without_dep.md) | Borrow the pattern, not the code-level dependency |
| [`freeze_thaw_pattern.md`](freeze_thaw_pattern.md) | BLAKE3-committed freeze/thaw for the corpus |
| [`honest_poc_caveats.md`](honest_poc_caveats.md) | What the pipeline does NOT do (documented limitations) |
| [`sigmoid_not_softmax.md`](sigmoid_not_softmax.md) | Why sigmoid, never softmax |
| [`silent_now_gates.md`](silent_now_gates.md) | A `#![cfg]`-gated test nobody names is a green zero — the row + exact-set proof discipline |

## See also

- [`../01_orientation/boundary_rationale.md`](../01_orientation/boundary_rationale.md) — why this is a separate repo
- [`../04_pruner/honesty_contract.md`](../04_pruner/honesty_contract.md) — the L2 honesty contract
