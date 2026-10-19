# Plan 255 — Batch 251: the Yukon kernel_opt conversion (Research 284)

**Status:** IN PROGRESS — claimed by zed-m3-idle/M3, 2026-10-19 ~07:33 +07, expires 2026-10-21 ~07:33 +07 (the push is the arbiter; 48 h expiry per protocol).

Convert the staged Yukon enumeration (Research 284 — Layr-Labs/cudafast-bonsai2-9b-engine @ `36c9536`, MIT, GumbiiDigital; the riir-train fork's `licenses/yukon-cudafast` carries the notice) into Batch 251: **16 own rules** (17 staged minus the thin B− M candidate 15 `restrict-local-alias-not-template-formal-cudafe-host-stub`, DROPPED at conversion adjudication — no perf axis, toolchain-hazard annotation class, two in-tree instances, the research note's own drop-or-rider clause) + **~21 count-neutral rider faces** on 17 host entries. Target: corpus 885 → 901.

Clone: `.raw/yukon-engine/` re-cloned this session, detached HEAD verified `36c95362b23b83724503fef4e88e61d748f9355f` EXACT (the ephemeral clone had been cleaned by the ephemeral-clone law; re-cloned for this conversion, removed at close per the same law).

## Category ledger (the conversion lands measured truth; the note's spread line A×4 vs its own rule list A×5 is a note typo — per-candidate assignments govern)

- Slice A (Group 1, candidates 1–8): 1 A `lane-permuted-column-reduce-same-butterfly-tree-bit-identity` · 2 G `lazy-rollback-journaled-window-replay-replaces-eager-state-snapshots` · 3 C `dead-destination-scale-elided-consumer-applies-on-load` · 4 B `gate-activation-precompute-pass-token-threshold-gated` · 5 D `topk-chunk-partial-lists-head-only-kway-merge` · 6 A `per-request-dirty-range-clear-graph-capture-sticky-marks` · 7 A `verify-ubatch-full-prestage-lane-token-gate-broadcast` · 8 L `host-dispatch-predicate-mirrors-kernel-compiled-arch`
- Slice B (Group 3, candidates 23–30): 23 B `magic-anchor-mma-exact-int-float-ffma-epilogue` · 24 A `j16-warp-fragment-direct-no-smem-tile` · 25 L `decode-layout-weight-permute-copy-registry` · 26 L `ptq1-value-exact-transcode-roundtrip-fallback` · 27 B `signed-trit-remap-single-mma-bias-fold` · 28 A `mbarrier-staged-in-structural-padding` · 29 N `halfword-payload-funnelshift-aligned-word-read` · 30 C `stacked-gate-up-single-launch-split-store`
- Totals: A +5 (178→183) · B +3 (100→103) · C +2 (55→57) · D +1 (211→212) · G +1 (38→39) · L +3 (23→26) · N +1 (80→81) · M unchanged 21 · sum 885→901

## Rider file partition (file-disjoint — two hosts share files, so the split is BY FILE, never by rule)

- **C1 (11 faces)**: `entries_gpu_01.rs` (14 warp-shuffle multi-value single-schedule) · `entries_gpu_07.rs` (9+34+36 weight-stream ×3) · `entries_gpu_20.rs` (11 radix boundary ×2 faces in one) · `entries_gpu_27.rs` (18 fwht-unconditional-width, 38 swar direct-ladder) · `entries_gpu_39.rs` (10 tile-max in-pass threshold)
- **C2 (10 faces)**: `entries_gpu_02.rs` (20 device-argmax dependent-chain) · `entries_gpu_09.rs` (13 cpasync consumer-permuted, 22 reduction-restructure shared-helper) · `entries_gpu_18.rs` (19 rht fold-site-per-producer) · `entries_gpu_22.rs` (17 batch-invariance epilogue-arm) · `entries_gpu_23.rs` (12+35 grid-dep non-restrict fence + prologue-prefetch) · `entries_gpu_36.rs` (37 tma mixed-operand) · `entries_gpu_53.rs` (33 stage-operand scale-metadata) · `entries_gpu_99.rs` (16 bit-equal nvcc `_rn` intrinsic)

## Tasks

- [ ] T0 claim row + this plan committed & pushed BEFORE authoring (the push is the arbiter)
- [ ] T1 Slice A: candidates 1–8 → `entries_gpu_105.rs`, every quote re-verified at the pin (agent A)
- [ ] T2 Slice B: candidates 23–30 → `entries_gpu_106.rs`, every quote re-verified at the pin (agent B)
- [ ] T3 Rider slice C1: 11 faces on entries_gpu_01/07/20/27/39, quotes re-verified (agent C1)
- [ ] T4 Rider slice C2: 10 faces on entries_gpu_02/09/18/22/23/36/53/99, quotes re-verified (agent C2)
- [ ] T5 Coordinator spot-checks: ~8 load-bearing quotes re-verified char-for-char by the coordinator (the B223 law)
- [ ] T6 mod.rs wiring: mod decls 105+106 + entries() extends + KERNEL_CATEGORIES additions
- [ ] T7 Count pins: tests.rs total 885→901 (×4 sites: L114, L5505, L5516, L9624) + per-category pins (A/B/C/D/G/L/N) + mod.rs doc count + AGENTS.md kernel_opt bullet + .docs/11_domains/kernel_opt.md
- [ ] T8 Batch gates: `batch251_rules_landed` (16 rule needle sets) + `batch251_riders_landed` (17 host faces) — needles verbatim at the pin
- [ ] T9 Gates: kernel_opt+latent_retrieval retrieval floors re-measure (/tmp target) + full kernel_opt lib + default lib + clippy −D ×3 postures; freeze/score-bench NOT owed (kernel_opt-only, the P30/B250 precedent)
- [ ] T10 Docs: AGENTS.md kernel_opt Role bullet (B251 row) + queue-snapshot landing row + plan [x] + research-note addendum (284 §Conversion: landed counts + the candidate-15 drop + the note's spread-typo correction)
- [ ] T11 Commit `feat: batch 251: ...` + push; `.raw/yukon-engine` removed at close (the ephemeral-clone law)
