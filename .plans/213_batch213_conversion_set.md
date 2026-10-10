# Plan 213 — Batch 213: the census-unblocked conversion set

**Status:** COMPLETE (2026-10-10, M3 idle-loop continuation; all four riders landed count-neutral, gates green first shot on the needle gate; commit + hash recorded in the envelope row)

## Source set (all staged in `.distill/001` envelope rows — quotes re-verified at the pins THIS batch)

| # | Lead | Pin | Domain | Disposition (intake lean, confirmed/overturned at T2 family read) |
|---|---|---|---|---|
| 1 | #29608 sycl pinned-ring upload staging (`000bee54a`) | ggml-org/llama.cpp | kernel_opt | rider-or-rule; intake lean = the gpu_61 ring-overwrite gating shape applied to upload staging |
| 2 | #30087 GDN four state columns per warp (`24e41838e`) | ggml-org/llama.cpp | kernel_opt | rider-on-gpu_09 (family EXTENDED, B205/B208 precedent) or own rule |
| 3 | P45 mlxfast Hadamard cached-zero-pad middle rung (`6e19fe1` submission) | vendor/mlxfast-bonsai2 | kernel_opt | ANNOTATION-GRADE RIDER (pre-adjudicated) on the B195 pad facet host |
| 4 | #28713 CUDA top-k algorithm selection (`71ad0590f`) | ggml-org/llama.cpp | kernel_opt | rider-on-B196-R2 topk family (GPU shape-keyed face) or own rule |

Behind (NOT this batch): Leads 4–8, 9/10/11 (rider tier), Lead 12 (kernel_opt staging head), k=8-MMA (Research 250, rider tier), the lithos batch (Research 249, plan-first step 0).

## Tasks

- [x] T1: Protocol H light — git sync M3, 4090 gitsync fired, mining-partition check (4090 riir-refine tree CLEAN at `9374ddc6`, no corpus WIP in flight; the box's cargo processes are other lanes — partition CLEAR), disk 211Gi/283Gi healthy, sibling cargo test running on M3 (their lane, disjoint write set: src/bin/heal.rs + .benchmarks — named-path staging holds)
- [x] T2: dual_allocation_gate clean (0/0/0) + plan allocation 212→213
- [x] T3: source verification at the pins — DONE (verification subagent, all quotes VERBATIM; corrections carried: the 2 GiB row is pre-existing upstream (never attribute to #29608), P45's LeadingRowsSlot entered at 09b44c5 not the submission (pin split f616f65 + 0c7c392), the kill-switch renamed MLXFAST_NARROW_ZERO_PAD → MLXFAST_RIDER_NARROW_ZERO_PAD at HEAD, no `slots` field on the staging struct)
- [x] T4: dedup greps + family reads — DONE: gpu_09 (row-packing; the #292 Metal GDN precedent makes #30087 the FOURTH backend), gpu_61 (ring-overwrite; the gating shape owned → upload-direction rider), the topk rule in entries_gpu_64.rs (kernel_opt D — the register-cliff axis composes with #28713's shape axis), the pad facet host = the B176 rule in entries_gpu_51.rs (the B195 pad-in-read rider already lives there → middle-rung rider beside it)
- [x] T5: author the four riders — DONE (disjoint files: entries_gpu_09/51/61/64.rs)
- [x] T6: batch213 gates — DONE FIRST SHOT (batch213_riders_landed_on_their_hosts, 4 hosts × needles); batch* family 453/0; count pins HOLD at 777 (count-neutral as planned)
- [x] T7: full gates — kernel_opt+latent_retrieval lib 3103/0 (3 ignored = documented freeze-regen e2es, 1492 s, ONE run at the isolated /tmp/b213tgt); default lib 2488/0; clippy `-D` ×3 postures clean (kernel_opt / default / no-default); retrieval floors HELD (20 green + the documented non-gate T0′ measurement ignored; 41 s, no decay, no re-pin)
- [x] T8: envelope row in `.distill/001` + commit + push

## Constraints

- Count pins: riders are count-neutral (kernel_opt stays 777, D stays 178, B stays 86) — any drift is a defect; own-rules re-pin with honest labels in the same commit.
- Freeze/score-bench NOT owed (zero clippy-corpus delta, P30) unless an own-rule lands in a non-kernel domain.
- NotBenchable annotation posture: Lead 1/2 + P45 carry no in-source perf numbers; Lead 3 carries measured numbers but the rider lands the DECISION content (source-reported, the B208 fewrow precedent).
- The sibling's WIP (src/bin/heal.rs, .benchmarks/*, cloudflare/dashboard) is NOT ours — named-path staging only.
- Isolated target dir for the gate runs (sibling cargo holds the default target dir).
