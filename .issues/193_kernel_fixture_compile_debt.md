# Issue 193 — repair the 65 kernel_opt fixture rules the G1 gate caught (parse-valid, compile-invalid or assert-failing)

**Status:** CLOSED 2026-11-01 — all 65 defective fixtures repaired (zero demotions, zero
asserts weakened); full G1 248 green / 0 FAIL / 803 parse-only in 100.8 s; lease closed
(see R6 for the commit roster). **Claimed:** zed-4090-193, 1793554051,
expires 2026-11-03T17:27:38Z. **Lease closed:** zed-4090-193, 1793554051, claimed
2026-11-01T17:27Z, closed 2026-11-01 with commits 3e143a7d9 (claim) through a82a5b29bd5
(the store regen).

## What happened

The corpus entry discipline asserted "standalone-compiled AND executed before entry" (the
AGENTS.md §Role law, the file-header provenance lines) — but the standing validator that
would enforce it never existed. Issue 191 T2 built it. First full run (2026-11-01, rustc
1.99.0, this box): **248 fixture-bearing rules → 183 green / 65 FAIL / 803 parse-only
(disclosed), 209.9 s**. The per-entry gate that ran at batch landing was `KernelOptDomain::
validate` = PARSE-only; compile and execution were never enforced standing, and the debt is
real: three of the RUN-failure rules were re-verified OUTSIDE the harness (standalone rustc on
the extracted opt arm) and reproduce — fixture bugs, not harness artifacts.

## The classes (from the full run's JSON report)

1. **RUN failures (~23)** — the fixture compiles and its OWN assert fails. Includes (verified
   standalone): `segmented-checkpoint-cheap-replay-backward` (gpu_65; `[5.0;4] vs [4.0;4]` —
   the replay test's two paths disagree), `componentwise-version-guard-rollover-silences-fast-
   path` (gpu_80; the "naive must disagree" assert fails), `head-dim-smem-tile-cap` (gpu_133;
   ref passes standalone, opt fails). Also: mma-accumulate-into-resident-accumulator-c-operand
   (gpu_75), verified-logits-derived-outputs-path-independence (gpu_82), mmvq-low-bit-vdr-two-
   rows-share-q8-loads (gpu_104, index OOB), attention-row-mask-bitmap-per-row-not-per-block
   (gpu_104), loop-back-edge-scratch-war-trailing-warp-barrier (gpu_108), autotune-sample-read-
   cold-not-cache-fill + cost-model-counts-work-the-kernel-skips (gpu_129),
   mma-window-admission-output-width-occupancy-floor + extent-one-dim-skips-every-layout-check-
   symmetrically + compile-run-rejection-persists-patience-compiles-on-reach +
   stage-fill-shared-physical-axis-reads-through-projection (gpu_134), write-coverage-proven-
   uninitialized-alloc + gate-rank-dispatch-function-constant-variant (gpu_139),
   hip-affine-contract-off-row-invariance (gpu_83), hadamard-transform-runtime-specialize-
   bounded-max-layout (gpu_120), trellis-reversed-lsb-tape-aligned-word-state-decode (gpu_120),
   constraint-row-recompute-on-device-not-table + ragged-batch-hybrid-feed-two-contracts +
   overlapped-branch-speculation-commit-or-discard (gpu_126), gdn-conv-chain-fusion-gather-
   relocation-done-set (gpu_131), vector-width-admission-checks-every-outer-stride (gpu_132),
   stream-table-consolidates-short-kernel-storms (gpu_134).
2. **Unresolved-symbol fixtures (~10)** — the test calls a helper named in NO arm:
   `real_tokens`, `run_once` (fusion-scope-by-barrier-cost-not-dispatch-count /
   length-keyed-dispatch-cut-invariance), `Gate` (kernel-contract-planted-gate-controls),
   `text_following_tokens` (calibrate-verify-window-on-text-not-proposer-chain), `find`
   (parser-backtrack-bounded-by-monotone-cursor), `process_tile` + `STEP`
   (oob-free-loop-inclusive-of-last-full-tile), `fold_one_slot_at_a_time`
   (ring-scan-grouped-load-ahead-wrap-step-same-fold-order), `Device`/`Family`/`Finish`
   (offline-device-tuned-table-dispatch, tuned-config-resource-limit-fallback,
   kv-reuse-prefix-ends-at-last-evaluated-token).
3. **Fragment arms (2)** — mid-function snippets that never were items:
   per-block-int32-chunk-rescale-fp32-accum, moe-broadcast-quantize-once-scatter (both gpu_33,
   "expected item, found keyword let").
4. **Timeout (1)** — relaxed-spin-acquire-confirm-metadata-flag-wait (gpu_118, >120 s run).
5. **Misc compile errors (~29)** — E0308 (attention-partition-two-knob-shared-partial-
   workspace, kquant-dequant-work-item-geometry-wide-stores), E0277 (whole-step-gdn-decode-
   window-one-launch: `f32: From<i32>`), E0599 (visibility-bounded-scan-shape-stable-
   allocations: `usize::bit_length`; stream-through-load-no-l1-allocate-once-read: Debug),
   E0614 (graph-replay-skips-host-node-scans), lifetime (barrier-on-reader-open-group-
   conflict-swap-overlap), `<`-as-generics parse (swa-evicted-prefix-tile-analytic-skip),
   match-non-exhaustive (fusion-unfusable-head-settles-prefix-to-next-search), Debug-derive
   class (sibling-kernel-admission-gap-absorbed-by-generalist, attention-route-retirement-by-
   measurement, coherence-scope-by-write-set-ordinary-loads-on-immutable), and the
   gpu_26/32/33/34 cluster (exhaustive-domain-input-patch-bit-identity, new-syntax-family-
   enumerates-every-keyed-surface, lut-free-bit-spread-vadd4-unpack, one-time-dense-repack-
   keyed-sync-publish, sm90a-window-shape-gated-wgmma-dispatch, streamk-starved-tile-grid-
   dispatch, kv-dequant-materialization-batch-threshold-gate, quantize-scale-math-wide-
   precision-denorm-safe, extent-clamped-operand-views-partial-k-tile, gqa-zero-stride-batch-
   broadcast, threadgroup-scope-reduction-single-writer, shard-granularity-lcm-never-cut-
   rotation-block, fa-fast-path-validated-exact-shape-admission-v-as-strided-view,
   ipc-peer-mapping-collective-teardown-pin-on-failure, autotune-round-bounded-by-roofline-
   ceiling).

## The repair law

- **Repair the FIXTURE, never weaken the GATE.** A fixture whose mechanism is sound gets a
  compiling, std-only, deterministic unit (the corpus's own test-design laws: mod-13
  symmetry-breaking values, must-fail twins, comparators, bit-identity over closed domains).
  A fixture whose assert is genuinely wrong gets the assert CORRECTED with the mechanism
  re-derived from the rule's cited source — never deleted silently.
- A rule whose fixture cannot be repaired without misrepresenting the mechanism gets its
  `#[test]` arms DEMOTED to parse-only prose (the fixture content folded into `gpu_concept`),
  with the demotion recorded here — the census floors then see the loss and Issue 191's G1
  floor posture absorbs it explicitly. Demotion is the last resort, never the batch norm.
- **One corpus file per commit-batch, G1 `--filter` for the touched rules + the smoke gate
  green before each batch lands; the FULL G1 run is the closing acceptance** (the campaign
  lane, exclusive rig not required — rustc compiles are CPU-bound and short).
- The census floors (248/72) do NOT move for repairs that keep fixture rules fixture-bearing;
  a demotion that drops a fixture rule requires the floor row re-derivation recorded here in
  the same commit (the T1 amendment discipline: pre-registered, never after).

## Tasks

- [-] R1 — the gpu_33/34 cluster (14 rules; the oldest batches — likely systematic shapes).
  LANDED 2026-11-01 (zed-4090-193): all 14 repaired to standalone compiling units — the
  oldest-batch hypothesis held: every failure was a pseudo-code body calling helpers named
  in NO arm (the entries predating any compile enforcement), plus 2 mid-function fragment
  arms. Repairs keep the decision content (per-arm `#![allow(dead_code)]` units, defined
  types, deterministic xorshift/mod-13 data, ref-vs-opt equality or exactness asserts,
  must-fail twins, work-counter pins); per-file `#[test]` token counts preserved exactly
  (3/2/6/5 — the census reconciliation stays green). Commits: gpu_26 `183e241dd2f`,
  gpu_32 `ad848608168`, gpu_33 `91063e74124`, gpu_34 `ba17e3077bf` (rebased to `65ac1845`+
  after a sibling push); G1 `--filter` 14/14 green, smoke gate 3/3.
- [-] R2 — the RUN-failure class (~23; each needs its assert re-derived from the cited source).
  LANDED 2026-11-01 (zed-4090-193): 25 rules across 15 files (the issue's class-1 list + the
  gpu_120/126 opt-COMPILE stragglers), three parallel repair lanes; every expectation
  re-derived from the rule's gpu_concept law — no assert weakened, no #[test] removed, no
  demotion. Two green-zero classes found en route: cfg-gated asserts the harness never
  compiles (gpu_80), and a NaN in a golden compared by a NaN-rejecting comparator (gpu_83).
  Lane A gpu_65/75/80/82/83: 2bd4c2c4^5..; lane B gpu_104/108/120/126/129; lane C
  gpu_131/132/133/134/139; the R1 one-time-dense regression the post-R2 full G1 caught
  (Key Hash over-match) is `2bd4c2c4`. Combined validation: full G1 222 green / 26 FAIL
  (215.3 s), smoke gate 3/3. One transient rustc STATUS_ACCESS_VIOLATION on rebuild
  (retry clean; the first "full" run after it was a STALE-BINARY read — rebuild exit
  status must be checked across pipes, the pipefail-discard class live).
- [-] R3 — the unresolved-symbol class (~10; the helper was probably lost in a corpus edit —
      check the git history of the entry before re-inventing it).
  LANDED 2026-11-01: the pickaxe half of this instruction is UNSAFE on this repo — a
  `git log -S` over full history hung 30 min (auto-gc fallout) and is now forbidden for
  this lane; the evidence settled it anyway (the fixtures predate compile enforcement —
  the helpers were never there). All helpers re-invented from their gpu_concept laws:
  run_once/run_unfused_control (fusion-scope + length-keyed), Gate/Contract
  (kernel-contract), text_following_tokens/DepthTable (calibrate-verify), find/find_tag_end
  (parser-backtrack), process_tile+STEP (oob-free-loop), fold_one_slot_at_a_time (ring-scan).
  Waves D+E+F commits `50ba6983fb8`/`90c68b7f74e`/`84e4432dfe9`.
- [-] R4 — the misc-compile class (~29, the gpu_85/86/100/101/103/104/118/120/126/129/131/132/
      133/134/139/142/147/26/32 long tail).
  LANDED 2026-11-01 (same waves): every class repaired at the mechanism level —
  `usize::bit_length` → `next_power_of_two()` (bit-identical on all asserted boundaries);
  Debug-derive class given working std derives that keep the asserts meaningful; E0308/E0277
  fixed at the use site; `<`-as-generics disambiguated; match-non-exhaustive made exhaustive;
  E0614 stray deref dropped; lifetime restructured to safe elision. Two LAW-level fixture
  bugs surfaced once the compile errors stopped masking them: barrier-on-reader let
  unknown-write ops JOIN the open group (the law says never) and attention-route's ref assert
  sat inside the D128 short-kv exception; both corrected per the repair law, not weakened.
- [-] R5 — the timeout rule (gpu_118) — bound the fixture's work or mark the spin-wait
      fixture release-only-bounded.
  LANDED 2026-11-01: ROOT-CAUSED, not just bounded — the `now_ns()` stub was pinned at 0,
  so the ReadyTimeout deadline could never fire and the spin was genuinely unbounded. The
  repair bounds the wait itself: a 512-miss escalation cap reporting the SAME ReadyTimeout
  fault (deterministic, no clock), fault-poll every 64 misses, the 4-way classification
  kept, and the ref arm bounded with its boolean-collapse defect pinned. Runs in ms.
- [-] R6 — close: FULL G1 run green (or the demotions recorded + floors re-derived), note the
      commit hashes here, fold into Issue 191 T5's campaign record.
  CLOSED 2026-11-01 (zed-4090-193): FULL G1 **248 green / 0 FAIL / 803 parse-only in 100.8 s**
  (rustc 1.99.0, this box, default+kernel_opt) — the whole fixture population compiles AND
  passes; smoke gate 3/3 (census reconciliation + floors + one e2e); kernel lib tests
  3350/0; floors held at 248/72 (zero demotions — the census floors never moved);
  `g1_status.tsv` regenerated in the same run (65 rows fixture-defect → fixture-green).
  Reviewer nit from the Issue-191 ratification: the exact per-class counts are now
  historically pinned by the R1–R5 rows above (R1 14, R2 25, R3 7 re-invented helpers,
  R4 ~18, R5 1 timeout — 65 total, all repaired, none demoted). Closing commits:
  R1 `183e241dd2f`+`ad848608168`+`91063e74124`+`ba17e3077bf` (+ the Key-Hash follow-up),
  R2 `5924f70f`..`2bd4c2c4` lane-per-file, R3/4/5 `50ba6983fb8`/`90c68b7f74e`/`84e4432dfe9`,
  store regen `a82a5b29bd5`. Folded into the Issue-191 T5 campaign record (Bench 124):
  the G1 axis of the REFUSED promotion now has a repair path complete — a re-campaign can
  run G1 against a fully green corpus; the owner-gated G2-convention decision (Bench 124
  §Next) remains the only open gate.

## Outcome (the campaign-facing summary)

**65/65 defective fixtures repaired, zero demotions, zero asserts weakened.** The G1 gate
(Issue 191 T2) caught real debt — the corpus's "standalone-compiled AND executed" entry law
had never had a standing validator, and the batches that landed under parse-only gates
accumulated pseudo-code bodies, wrong arithmetic, mis-derived expectations, one law
inversion, and one unbounded spin. Every repair re-derives the expectation from the rule's
cited gpu_concept law; the must-fail twins and work-counters now PIN the mechanisms instead
of narrating them. The corpus annotation surface (`context_notes`) flips 65 rules from
`fixture-defect` to `fixture-green` via the digest-keyed store.

## Refs

- Issue 191 (the gate that caught this; the T2 record carries the first-run numbers).
- The full-run JSON report shape: `G1Report` (`cargo run --release
  --example kernel_goat_g1 --features kernel_opt -- --full --out <path>`).
