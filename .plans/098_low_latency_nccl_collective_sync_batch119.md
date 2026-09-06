# Plan 098 — Batch 119: low-latency-nccl collective sync distill (sentinel + LL flag-in-data)

**Status:** IN PROGRESS — verdict R143 (2026-09-06); batch queued behind arxiv walk #8 + Batch 118.

Source: arXiv:2607.16100 ("Every Microsecond Matters", ETH+NVIDIA) + repo `ss16118/low-latency-nccl` (clone at T1). Verdict: YES B corpus / no lane mapping (single-GPU workspace) — see `.research/143_low_latency_nccl_collective_sync_distill.md`.

## Phase 1 — provenance + anchors

- [ ] T1 clone `ss16118/low-latency-nccl` into `.raw/low-latency-nccl` (shallow; `.raw` discipline), pin `@ <sha>` + license in R143 BEFORE any entry cites file:line; locate anchors: `ncclLLBuffer` ctor + `advanceEpoch`, `send`/`recv`/`recvUnrolled`/`recvReduce`/`bcast`/`reset`, `ncclCalcScratchBufferSize`, `ncclLLBufferInitSentinel`, the LL128-atomic kernel; re-verify every quote at the pin (the R107 coordinator spot-check convention).

## Phase 2 — dedup + entry authoring

- [ ] T2 dedup re-verification at mine time under BOTH vocabularies + the third angle (`semaphore|spin|acquire|poll`): confirm zero flag-IN-data / sentinel-init coverage; confirm distinctness text vs `stripe-sms-l2-semaphore-reduce` (B92 — flag AFTER data, acquire+fence), `barrier-free-per-dependency-wait-from-loop-counters` (host-side), and the readback-timing family.
- [ ] T3 author entries in the next `entries_gpu_NN.rs` (find the highest at write time; likely 29):
  - `sentinel-readiness-over-separate-flag` (lead, B): dtype-sentinel init + poll `val != sentinel`; decision text carries the four constraints (never-equal-sentinel incl. FP32 sign-bit NaN payloads; reset-before-reuse; peer-store-visibility precondition; single-writer slots ⇒ determinism preserved).
  - `ll-flag-in-data-packed-poll` (regime twin, B−): 16B atomic `{flag, data}` poll flag==epoch; two-regime split vs sentinel by message size (LL = latency-bound tiny; sentinel = bandwidth-bound moderate).
  - Both entries embed the LL128-atomic determinism caveat (flag-via-atomic-ADD is non-deterministic FP, γ_{N−1} bound ≈3.8e−6 @N=64 FP32, addition-only, NVLink cache-line atomicity — forbidden for bit-identity/replay-gated consumers).
- [ ] T3b rider → B58 `output-buffer-rotation-pool-multi-forward`: append the bidirectional-credit phrasing ("each receive is implicit permission for the next send") + arXiv:2607.16100 citation to the concept text.
- [ ] T4 standing gates: batch parse gates 3/3; `kernel_heal` count floor held; default lib floor held; combined floor held; clippy 0 in the batch states; corpus count + count-pin sites walked (incl. the batch's own test pin — the B116/B117 double-surprise lesson).

## Phase 3 — records + cleanup

- [ ] T5 finalize R143 (Status → RECORD with landed numbers); queue-snapshot Last-landed line; AGENTS.md Batch-119 entry — NOTE: AGENTS.md may carry sibling WIP at batch time; coordinate or defer the entry line rather than sweeping hunks.
- [ ] T6 `rm -rf .raw/low-latency-nccl`; commit + push on `develop` (pathspec-scoped; staged-set audit if the tree is shared).
