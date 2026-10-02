# Research 229 — mlxfast-bonsai2 wave-2 accept-wave distill (Batch 195 conversion)

**Status:** RECORD (Batch 195 landed 2026-10-02; corpus 717→726, commit `feat: batch 195`).

## Source + method

- Source: `Layr-Labs/mlxfast-bonsai2-27b-engine` @ `0c7c392` (MIT; the standing vendor clone
  `/Users/katopz/git/vendor/mlxfast-bonsai2`). NET delta `0c61273..0c7c392` — 25 commits, 22 files,
  +9150/−777 (the 09-30..10-02 accept wave; NO tools/ reorg this time — the B190 exclusion class
  measured absent).
- Method: plan-first (Plan 194, Protocol A step 0); 2 staging subagents over disjoint slices
  (MTP/draft/accept vs KV/layout/numerics); **delta-freshness check on every candidate** — quotes
  verified against `git diff 0c61273..origin/main | grep "^+"` before authoring, not just the tip.
- The freshness check EARNED its keep immediately: the staging's `gpu-wake-scalar-touch` lead was
  KILLED as a **B193 duplicate** — `seed-first-command-buffer-earliest-submission` (entries_gpu_60)
  already cites `ContiguousKVBackend.swift:27-44` (CBv2GPUWake) verbatim; the lead rode the delta's
  diff CONTEXT, not its + lines. The same check confirmed `lazy-admission-reservation-atomic-register`
  is **present at base `0c61273`** (3 `bytesReserved` hits at both pins) — landed with the honest
  "present at base, first surfaced this wave" provenance note instead of a delta claim.

## Landed (9 rules + 3 riders; `entries_gpu_63.rs`)

| # | rule | cat | one-line |
|---|---|---|---|
| 1 | `io32-count-pinned-text-narrowing` | O | 64→32-bit kernel-source-text narrowing, count-pinned + launch bounds; stock-text fallback on drift |
| 2 | `strided-view-offset-fit32-launch-proof` | O | the family's general member: prove arbitrary strided views fit 2^31 by the span sum, then dispatch |
| 3 | `write-only-touch-side-queue` | D | provably write-only touches on a second queue + serial side thread + owner-thread availability + bounded drain |
| 4 | `last-query-rect-projection` | B | q|gate projection on the consumed rows only via rect-partitioned packed matmul; bit-checked |
| 5 | `roundfuse-packed-embedding-gather` | C | codes/scales/biases gathers → one launch, pure-copy contract, per-geometry verdict cache |
| 6 | `enumerable-domain-bit-identity-admission-gate` | N | enumerate-to-ADMIT (2^16 bitwise) vs B115's enumerate-to-PATCH; load-time binary gate |
| 7 | `cross-queue-output-retention-until-completion` | E | allocator hazard tracking does not span unordered queues — retain until the CB event signals |
| 8 | `atexit-registration-order-shutdown` | D | teardown by registration order; shared-lock handoff refusal; bounded final wait (NEW law) |
| 9 | `lazy-admission-reservation-atomic-register` | D | max(actual, estimate) reservations + atomic check+register (the TOCTOU overshoot fix) |

Riders: `kv-first-append-unzeroed-alloc-offset-gated-tail` += the adopt/install contract + `ready = enabled && verified` gate (B179);
`dtype-native-source-read-with-shared-admission-predicate` += the Metal BF16 head-rotation pad-in-read instance (B176);
`prompt-continuation-draft-override-measured-min-match` (rust_perf) += the draft-vocab-cutoff honesty law
("The cutoff can change draft acceptance" — the record's own correction of an acceptance-neutral claim).

## Staged (the MTP-slice leads — next kernel slot, pending their own quote re-verification)

The MTP/draft/accept staging slice returned ~49 candidates at SUMMARY level (its detail pass was
truncated by the subagent's output budget); per the standing discipline (staged = quoted in the
snapshot, never landed unverified), the strongest are staged for the next kernel slot:
`exact-form-device-trial` (device-ranked re-timing of ported constants, bit-identical alternatives),
`pack-worthiness-shape-admission` (packed-format admission by weight shape, M5 Max measured table),
`lossless-11bit-repack-trial` (lossless re-encode, per-shape-class adoption),
`draft-tape-behavior-gated-trial` (timing trial requires proposal-identity vs a recorded tape),
`walk-parallel-edge-precompute` (dependency-free edge scoring, 80µs→13µs measured),
`first-layer-ahead-submission` (+69µs measured; SLICE_LAYERS recorded negative),
`deferred-block-release`, `alias-scratch-reuse`, `accept-packet-one-launch` + the MLX metalKernel
signature gotcha (E-class), `per-layer-verification-coverage`, the producer-fold family (7 fusions
with the standard fixture spread), `single-pass-continuation-scan`, the io32 local twins, the pack11/
pack12 arithmetic leads. Plus E-class hazards: the env-polarity migration hazard (opt-in→opt-out
flips default behavior fleet-wide), the gather wrap-semantics hazard, the `x.size <= Int32.max`
admission the delta added to BoundaryBlocks.

## Drops (recorded per the standing ledger discipline)

- `gpu-wake-scalar-touch` — **B193 duplicate** (the freshness check's catch).
- `prompt-swizzle-supertile` — measured **NEGATIVE +1.45%** in-source (the tile-order cache-locality
  experiment lost; keep the anti-lead beside B181's negative ledger).
- `verify-slice-third-boundary-experiment` — an OPEN experiment with a prior-negative sibling
  ("every extra command buffer at verify width has a cost"); mine only after its own verdict.
- The bank-pad removal in `PrismHadamardQwen35.swift:75` (`red[4*16*33]` → `red[4*16*32]`) —
  HAZARD-WATCH, not evidence: the +1 bank-pad word was removed with no comment and no measurement;
  correctness unaffected (warp-consecutive indexing + a barrier), perf direction unrecorded. If the
  corpus ever wants a bank-padding rule, this delta is the counter-example to interview.

## Lane intel

The wave's MTP/top-two lane rides DFlash2 spec-decode — riir-ai Issue 1011's decode-axis watch may
tick again on the next scored build. Detection-only from this repo (no bench run; Plan 337 + the
M3 light-work rule).

## Gates (all green)

batch195 parse gate (9) + contracts gate + riders gate · kernel slice 396/0 · lib default 2423/0 ·
clippy `-D` clean ×3 postures (default / kernel_heal / rust_perf+kernel_heal) · retrieval 21/21
floors HELD (234s) · score-bench + freeze NOT owed (kernel_opt-only, zero clippy-corpus delta, P30).
