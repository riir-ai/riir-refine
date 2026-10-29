# Plan 312 — Batch 307: the 10-29b staged kernel_opt pool (leads + riders)

**Status:** IN EXECUTION — claimed 2026-10-29T18:3xZ (shikuwa-4090-b307); the 10-29b lead
delta sweep's staged addendum (snapshot, 2026-10-29T12:1xZ) is the verdict record this
batch converts; T0 quote re-verification at the pins + T1 dedup at the slot done pre-claim.

## Scope

The kernel_opt pool of the 10-29b staged addendum (2 DISTINCT own + 4 rider faces), converted
under the B293 cross-domain precedent: kernel_opt-only — B300 (perfect-hash, to 10-30T09:34Z),
B301 (leviathan Phase 3, plan 306, to 10-30T13:41Z) and B306 (infinity tokio, plan 310, to
10-31T14:21Z) hold the rust_perf/sec heads; zero candidate overlap (grep-verified per atom,
below). The numerics row rides in-batch as a count-neutral rider on the B299 drain rule
(entries_gpu_134) — its Rust fixture home is kernel_opt's transform-placement face.

## Candidate slate (T0 grades from the staged addendum bind; honest drops allowed)

| # | Lead | Source | Grade | Disposition |
|---|---|---|---|---|
| A | #64419 `fa1cf1d` + #64345 `3c39dda` `write-coverage-proven-uninitialized-alloc` (TWO same-day instances of ONE atom) | vllm | STRONG DISTINCT B+ | OWN rule (2 source instances, one entry) |
| B | #4791 `9e2e8e2` `gate-rank-dispatch-function-constant-variant` (scalar vs per-channel g indexing) | mlx | DISTINCT B | OWN rule |
| C | #64482 `8126ebd` SM100 skinny-GEMM TP=1 rows (new table axes: static_k + TP-class keying + the hand-tuned wrinkle) | vllm | RIDER B | rider on `swept-shape-bucket-launch-tables` (entries_gpu_17) |
| D | #31075 `6d19453` MXFP4 fp16-bit-trick LUT-free nibble decode (no threadgroup memory) | llama.cpp | RIDER B- split | rider face (a) on `lut-free-bit-spread-vadd4-unpack` (entries_gpu_33) |
| E | #31075 `6d19453` contiguous unary/binary 1-thread-per-threadgroup -> <=256-thread geometry + GLU row split | llama.cpp | RIDER B- split | rider face (b) on the B304 singleton-thread-axis family (`concat-row-space-thread-split-not-singleton-outer-dim`, entries_gpu_137) |
| F | #1030 `54ce0e3` soft_cap applied ONCE at the split-K drain (tanh non-distributive over +) + the double f32 round-trip removal | uzu | B+ correctness-carrying | count-neutral rider face on `contraction-partials-sum-once-at-drain` (entries_gpu_134) — the non-linear placement complement |

Pre-adjudicated kinship (from the staged addendum, slot re-verified): the entries_gpu_26
island-permutation rule owns OVERWRITE-coverage dead-compute (a different coverage axis —
"is the packed compute dead?", not "skip the zero-fill because writes cover reads"); the FLA
`cuda-graph-static-buffer-zero-replay` is the OPPOSITE pole (replay demands physical zeros);
`identity-keyed-graph-retention-stable-arena` (entries_gpu_08) owns the zeroing-resize face
with no coverage contract. The mlx function-constant kin: `pipeline-cache-key-covers-every-
function-constant` (entries_gpu_35) owns the CACHE-KEY side; `adjacent-step-kernels-fuse-
mode-specialized` (entries_gpu_134) owns the FUSION side — the rank-variant INDEXING law is
unowned.

## Tasks

- [x] T0 quote re-verification at the pins: the five PRs fetched as .patch into `.raw/b307/`
      (vllm #64419/#64345/#64482, mlx #4791, llama.cpp #31075, uzu #1030 — org `trymirai`);
      every staged quote byte-verified against the diff text (18:2xZ, pre-claim)
- [x] T1 dedup greps at 3+ vocabularies vs the corpus at origin HEAD `854c0224`: zeros/uninit
      family 61 hits — zero coverage-contract owners; function-constant family — cache-key +
      fusion owners cited, rank-dispatch unowned; launch-table/minifloat/singleton families —
      rider homes located, no dup
- [ ] T2 authoring: entries_gpu_139.rs (2 own rules, each with pattern, bounded fix-space,
      category, rationale, source quote, distinct-from kinships, grep evidence) + 4 rider
      faces (C on gpu_17, D on gpu_33, E on gpu_137, F on gpu_134)
- [ ] T3 wiring: mod.rs slices + category comments; router expert-total re-pin (count moves
      +2)
- [ ] T4 count pins: the kernel_opt total + per-category sites via ONE shape-asserted pass
      (tests.rs sites + the Plan-003 per-category table + AGENTS.md + README rows)
- [ ] T5 gates: G1 batch307 needle+parse (`--features kernel_opt`); G2 kernel_opt lib + default
      lib; G3 clippy -D x3 postures; retrieval floors both postures (kernel_opt,latent_retrieval
      + default); freeze/score-bench NOT owed (zero clippy-corpus delta, P30); binary rebuild
      NOT owed (kernel_opt outside the release set)
- [ ] T6 records: Research 351 (the batch note — the staged addendum is the verdict record,
      the note carries the landing provenance); AGENTS.md kernel_opt count; .docs/11_domains/
      kernel_opt.md; snapshot landing row; this file's tasks ticked
- [ ] T7 commit `feat: batch 307: ...` + push; `.raw/b307/` removed at close

## Gates, criteria, risks

- Shared-tree hazard: this box's checkout (E:/git/riir-refine) is 60-behind with the sibling
  B300-lane WIP — ALL work happens in the detached worktree `E:/git/wt-b307` at origin
  `854c0224`, gated under an isolated CARGO_TARGET_DIR (`/tmp/plan312-target`); the local
  checkout is untouched.
- The embedded-quote defect class (B299's twice-fired lesson): never replace an old prefix
  WITH its leading quote when prepending rider notes.
- Fixtures carry no string literals (the B274 lesson); needles keyed to landed text AFTER the
  entries are written (the Batch-259 lesson).
- Count pins surface wider than the corpus grep: the lib-test failing list is the only
  reliable enumerator (the B274 lesson).
- Claim expiry 48 h per protocol (2026-10-31T18:3xZ); push is the arbiter; other lanes
  re-fetch before staging and yield to this row on the 10-29b kernel_opt pool.

Session: shikuwa-4090-b307, 1793347000
