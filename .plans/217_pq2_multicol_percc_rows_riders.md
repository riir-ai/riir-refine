# Plan 217 — Batch 217: the staged Leads 12+14 converted as TWO count-neutral kernel_opt riders

**Status:** COMPLETE — all gates green (3105/0 kernel_opt+latent_retrieval lib, 2488/0 default, clippy ×2 clean, batch217 gate first shot); two count-neutral riders landed; commit at close.

Staging rows: `.distill/001_mining_queue_snapshot.md` lines 1-3 (the 10-10 ~19:2x perf-search row: Leads 13/14/15 staged; Lead 12 staged at the 10-09 23:4x row). Quotes verified VERBATIM this session from the FULL .patch of each commit (`/tmp/b217/lead12.patch` + `/tmp/b217/lead14.patch`, fetched at the pins `2a42998c5` / `668256445`).

⛔ **STAGING-DRIFT CORRECTION (the mandatory composition-time dedup re-grep paid for itself):** the Lead-12 staging row named three "unowned atoms". Re-grep at composition: the **activation-sum digit-bias correction** (`sum(q*(c-1)) = sum(q*c) - sum(q)`, the int16-in-half-slot qsum transport) is **OWNED by `deferred-bias-ternary-dp4a-exact-isum`** (entries_gpu_49.rs, Batch 180 — same identity, same bit-transport law, verbatim text); the **multi-column activation-layout axis keyed per (type, columns)** is **OWNED by `planar-transposed-activation-multi-column-quant-matvec`** (same file, the layout-picker-enum rule). Genuinely new: the **encode-time intra-group qs permutation that eliminates per-weight decode** + the **per-CC (Ada exact-cc) dedicated-kernel admission**. → RIDER, not the staged own-rule option. Disclosed in-rule.

## Tasks

- [x] Protocol H (M3 sync all repos; riir-train ff'd 258d2da9→4f72f6e8; zero zombies; no prunable worktrees; disk 259Gi/283Gi healthy; 4090 gitsync fired via Tailscale, LAN refused per documented drift)
- [x] 4090 mining-partition check (riir-refine CLEAN at d0eb681a, zero corpus WIP — CLEAR)
- [x] Fetch + read BOTH full .patches at the pins; every staging-row quote verified verbatim (incl. the measure-then-remove second commit, the -ub 4 pplx drift, the sm_86 law)
- [x] Dedup re-greps at composition (dp4a/permuted/planar/qsum/layout vocabularies) — the correction above; `planar-transposed-activation-multi-column-quant-matvec` + `fewrow-tensor-tile-decode-into-rhs-operand` named hosts; B186 stage-time-repack distinct-from written
- [x] Allocate Plan 217 (highwater 216→217, dual_allocation_gate clean)
- [x] Rider 1: entries_gpu_49.rs `planar-transposed-activation-multi-column-quant-matvec` += PrismML #306 (the layout family's FOURTH member GGML_CUDA_Q8_1_PQ2: encode-time decode-eliminating permutation + per-CC Ada admission + quantizer-kernel agreement + qsum instance-cite + clamped-rows instance + measure-them-remove + pplx drift disclosure + lane intel)
- [x] Rider 2: entries_gpu_66.rs `fewrow-tensor-tile-decode-into-rhs-operand` += PrismML #340 (the width-validation axis's PER-CC member: exact-cc==860 ROWS_1 at the planar single-column shape; guard+launcher shared selector; smem per rows; case-1 instantiation split)
- [x] Gate: `batch217_riders_landed_on_their_hosts` (2 hosts × verbatim needles) in tests.rs
- [x] Count pins verify UNMOVED (778 / D 179 — riders are count-neutral; no edits to pins, the existing asserts must stay green)
- [x] Gates: kernel_opt+latent_retrieval lib FULL run (isolated /tmp/b217tgt) + default lib + clippy `-D` (default all-targets + kernel_opt lib) + retrieval floors HELD
- [x] Freeze/score-bench NOT owed (zero clippy-corpus delta, P30); .docs/11_domains NOT owed (count-neutral riders, no new rules)
- [x] Snapshot landing row (prepend `.distill/001_mining_queue_snapshot.md`); Lead 15 (MXFP4 SYCL, full .patch UNREAD) explicitly left for the next cycle
- [x] Commit (named files ONLY — the sibling's Cargo.toml + examples/p157* WIP is NOT mine) + push

## Queue after (for the snapshot row)

Leads 9/10/11 + k=8-MMA (Research 250) at the rider tier; Lead 15 at the head (full .patch owed at ITS conversion); Lead 13 (own-rule-or-rider, full .patch + the A10 number owed); Plans 215/216 staged; the lithos batch (Research 249, plan-first at conversion, fresh-session context); walk #14 Tue 10-13 (decode-razor re-poll); mlxfast floor ~10-17; nnethercote ~11-01; dapps Issue 122 stays the sibling's lane.
