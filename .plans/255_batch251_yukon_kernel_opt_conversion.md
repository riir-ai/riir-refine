# Plan 255 — Batch 251: the Yukon kernel_opt conversion (Research 284)

**Status:** LANDED 2026-10-19 — 16 own rules + 21 rider faces, corpus 885→901; gates all green (batch251 needles 2/2 first-run, kernel_opt lib 3161/0, default lib 2696/0, clippy −D ×3 clean, retrieval floors HELD 100%/90.6); freeze/score-bench NOT owed (kernel_opt-only, P30). Claimed by zed-m3-idle/M3, 2026-10-19 ~07:33 +07 (the push is the arbiter).

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

- [x] T0 claim row + this plan committed & pushed BEFORE authoring (the push is the arbiter)
- [x] T1 Slice A: candidates 1–8 → `entries_gpu_105.rs`, every quote re-verified at the pin (agent A; 2 research-note corrections folded in-rule: NOFINAL flag vs SLOT0 mark constants, the FNV naming; multi-line literals normalized to the house single-line convention at wiring)
- [x] T2 Slice B: candidates 23–30 → `entries_gpu_106.rs`, every quote re-verified at the pin (agent B; correction `mmq-ptq1-j16.cuh:10-12`→:10-11; committed `dbbfa518`)
- [x] T3 Rider slice C1: 8 faces on entries_gpu_01/07/20/27/39 (the brief under-allocated — faces 21/32/31 moved to the coordinator's own pass on gpu_01)
- [x] T4 Rider slice C2: 10 faces on entries_gpu_02/09/18/22/23/36/53/99 (39/39 quotes script-verified)
- [x] T4b Coordinator's own rider pass: faces 21+32 (fuse-gate-up) + 31 (fuse-residual) on entries_gpu_01 — quotes hand-verified at the pin
- [x] T5 Coordinator spot-checks: 12 load-bearing quotes re-verified char-for-char (lane-permute fold, magic anchor, lazy publish, funnelshift, SGN remap, sticky set, broadcast shuffles, k-way merge, decode-back atomicOr, host/device arm)
- [x] T6 mod.rs wiring: mod decls 105+106 + entries() extends + KERNEL_CATEGORIES additions (7 slices + count comments; the L comment's stale 22 repaired to the table's 23→26)
- [x] T7 Count pins: tests.rs total 885→901 (4 multi-line sites + 4 single-line batch-history sites) + 7 per-category pins + router.rs experts-cover pin + AGENTS.md kernel_opt bullet + .docs/11_domains/kernel_opt.md record
- [x] T8 Batch gates: `batch251_rules_landed` (16 rule needle sets + the drop NEGATIVE assert) + `batch251_riders_landed` (17 host faces) — FIRST-RUN PASS after 5 needle-text corrections (the NotBenchable assumption wrong for the 4 CPU-exact rules: magic-anchor, ptq1, sgn, funnelshift)
- [x] T9 Gates: retrieval floors re-measure (/tmp target) kernel 100% / blended 90.6 HELD no re-pin + kernel_opt lib 3161/0 + default lib 2696/0 + clippy −D ×3 postures clean; freeze/score-bench NOT owed (kernel_opt-only, P30)
- [x] T10 Docs: AGENTS.md kernel_opt Role bullet + queue-snapshot landing row + this plan + Research-284 addendum
- [x] T11 Commit `feat: batch 251: ...` + push; `.raw/yukon-engine` removed at close (the ephemeral-clone law)
