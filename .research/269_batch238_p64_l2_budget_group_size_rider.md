# Research 269 — Batch 238: the P64 conversion (llama.cpp #30215 L2-friendly workgroup scheduling)

**Status:** RECORD — landed 2026-10-15 as a count-neutral rider on `l2-grouped-tile-schedule` (B130, `entries_popcorn.rs`); corpus stands 813; Plan 240; commit = the Batch-238 landing commit on `develop`.

## Source + pin

- **ggml-org/llama.cpp PR #30215** "vulkan: Use L2-friendly workgroup scheduling for cm2 mulmat on DGX Spark" @ commit `3e390af18` (merged by 0cc4m 2026-10-15; commit author Ruben Ortlam <rortlam@redhat.com>; **MIT**). Single-file patch: `ggml/src/ggml-vulkan/vulkan-shaders/mul_mm_cm2.comp` (+26/−3).
- **Pin method:** the fetched `.patch` (byte-verified formulas this session) + the PR body (both tables + both review threads re-read at fetch). No clone needed — the patch is one shader file; the PR conversation carries the regime evidence. The `.raw` clone rule is satisfied by the fetched artifacts (removed nothing — nothing was cloned).
- **Pre-staging:** graded B+ as P64 by the 2026-10-15 4090 envelope pass (`.distill/001` envelope row at `29caa5a5`); quotes re-verified at conversion time per the P-row's own law — DONE, zero drift between the staged row and the pin.

## The law (three decision contents, all NEW vs the existing B130 faces)

1. **The budget-derived group size.** The base rule ships `group_size` as an unexplained parameter; the cloudrift rider (Batch 130) carries `const int SWIZ=8` FIXED. This rider owns the DERIVATION axis: `#define GROUP_A_BUDGET (8u * 1024u * 1024u)` then `group_m = clamp(GROUP_A_BUDGET / max(a_panel_bytes, 1u), 1u, min(blocks_m, 32u))`, with `a_panel_bytes` computed **per operand datatype** — four formulas verbatim at the pin: `#if defined(MULMAT_QUANT)` → `(BM * p.K / mm_quant_k()) * MmABlockBytes`; `DATA_A_F32` → `BM * p.K * 4`; `QUANT_K == 1` → `BM * p.K * 2`; else the q4-class → `(BM * p.K) / 2 + (BM * p.K) / 16`. The A-panel bytes one group must keep L2-resident DECIDE the group width — floored at 1, capped at 32 and at the M extent.
2. **The merged-axis linearization** — the THIRD concrete formula shape for the rule (popcorn's group-column-major walk = 1st; cloudrift's band-of-8 = 2nd). The old schedule read `ic = gl_WorkGroupID.y` + `ir = gl_WorkGroupID.x % blocks_m` (the removed `#else` lines at the pin); the new one linearizes BOTH axes: `const uint lin = gl_WorkGroupID.y * blocks_m + (gl_WorkGroupID.x % blocks_m);` then de-linearizes through `(group_id, first_m, gsize, in_group)` to `ir = first_m + in_group % gsize; ic = in_group / gsize;`. Grid SHAPE unchanged — a pure permutation of workgroup→tile mapping, bit-identical outputs by construction (the same correctness clause as the base rule and entries_gpu_39).
3. **The MUL_MAT_ID carve-out** — the applicability boundary: `#ifdef MUL_MAT_ID` deliberately keeps `const uint ir = gl_WorkGroupID.x % blocks_m;` because the uniform-A-panel bytes assumption breaks on the ragged expert path (per-expert A panels differ). Cross-refs `expert-major-grid-enumeration` (entries_gpu_39) which owns the grid-SHAPE axis for the MoE lane; this rule stays a swizzle WITHIN one grid.

## The regime law (the PR body, verbatim-verified)

- **DGX Spark GB10 iGPU, op-level** (test-backend-ops perf, us/run): q8_0[5120,12288] **+32.5%**, q8_0[5120,10240] **+30.1%**, q6_K[5120,6144] +23.6%, q6_K[5120,248320] +26.1%, q6_K[17408,5120] +20.9%, q6_K[6144,5120] +18.0%; several shapes reach remap/CUDA 0.99–1.01 (parity); the small GEMM is noise (q8_0[5120,1024] −0.6%).
- **Model-level** (llama-bench, −fa 1 −n 0 −b 2048 −ub 2048 −r 3): Qwen3.8-27B Q6_K pp2048 **+21.3%** (602.8→731.5 t/s; CUDA gap **1.31x→1.08x**), pp512 +14.7%; Llama-3.1-8B Q6_K pp2048 +16.0% (1.20x→1.03x); Gemma-31B Q5_K_M pp2048 +17.0% (1.28x→1.09x); **the small-K quant class gains least**: Llama-3.1-8B Q3_K_S pp2048 only +4.0% (gap 1.23x→1.18x — narrow A panels already fit the budget; the derivation explains the flat gain: `a_panel_bytes` small ⇒ `group_m` already capped ⇒ remap ≈ no-op).
- **Anti-regime (the fat-bandwidth law):** the author: "I also tested this on RTX 3090, but it's perf-neutral there." Reviewer jeffbolznv: "I had done something similar recently in vk_cooperative_matrix_perf, and in that test I found that it started to really matter once I get up to 8kx8k. I'm not seeing a gain on my 5090, but I expect this should benefit smaller GPUs." Fat-bandwidth discrete GPUs are already DRAM-fed at these shapes; the low-bandwidth iGPU class pays — and GB10 is in the occupancy-keyed value-split ladder's benchmark-gated CC list (B234), so the lane admission already knows the host.
- **HONEST NULL (the declined fast path):** jeffbolznv suggested skipping the remap's integer divisions for smaller ops; the author prototyped the `group_m >= blocks_m` early-out and: "I can do something like this, but I can't measure any difference from it" — the remap's integer divisions are free against a GEMM's work; landed as-is ("I'm fine with is as-is"). Same honest-null class as B224's ne02 function-constant fold.

## Dedup (grep evidence, this session)

- Vocabulary grep `GROUP_A_BUDGET|a_panel_bytes|L2-friendly|workgroup scheduling` over `src/domains/` → **zero hits** outside the new rider (the pre-edit corpus did not carry the derivation axis).
- vs B130's existing faces: the cloudrift rider = the SECOND formula shape with SWIZ fixed (this rider derives it); the base `opt_pattern` already parameterizes `group_size` (this rider owns the derivation OF that parameter — pattern untouched, concept extended).
- vs `expert-major-grid-enumeration` (entries_gpu_39): that changes the grid SHAPE for the MoE lane; this swizzles within a fixed grid and CARVES OUT the expert path — the cross-ref is bidirectional in-rule.
- vs `entries_gpu_48` (evict-last store policy) / `entries_gpu_09` (L2-resident output-buffer reduce): those decide LINE LIFETIME / WRITE staging; this decides TILE ORDER. The L2-policy trio composes with scheduling, never substitutes.

## Gates (all green, this session, default target dir — no sibling cargo)

- **G1** `batch238_rider_parse_and_needles_landed`: 18/18 needles verbatim + both B130 pseudocode halves still parse + `entries.len() == 813`.
- **G2** `cargo test --features kernel_opt --lib`: **3117 passed / 0 failed** (1 pre-existing ignored).
- **G3** clippy `--all-targets -D warnings` at BOTH postures (kernel_opt + default): **exit 0 / exit 0** (the two katgpt-* path-dep warnings are the sibling WIP tree's rustc check noise, not lint failures).
- **Retrieval** `--features kernel_opt,latent_retrieval --lib retrieval`: **21/21 PASSED** (228.65 s) — kernel self-variants 100%, blended floors HELD at the pinned 91.2/94.7/95.3; no re-pin.
- **Freeze/score-bench NOT owed** (P30): zero clippy-corpus delta (count-neutral kernel rider — the Batch-235/236 precedent).

## Dispositions

- NotBenchable at our pins (vulkan cm2 path; our hosts CUDA/Metal/CubeCL) — annotated in-rule.
- Lane intel: none of our shipped lanes read the vulkan backend; the DERIVATION axis is the transferable content (any L2-grouped schedule we ever ship — CubeCL/Metal tile walkers — should derive its group width from the panel-bytes budget, not tune it).
- Queue: P64 ticked in `.distill/001`; Batch 237 (M3, plan 239) slices A/C/D unaffected (disjoint files).
