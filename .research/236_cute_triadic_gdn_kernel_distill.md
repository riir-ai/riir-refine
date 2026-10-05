# 236 — cute-triadic-gdn: cuTe chunked-GDN kernel source (distill verdict)

**Status:** DISTILLED — pending owner decision (batch queued behind Batch 201; plan-first Protocol A step 0 at conversion — no `.plans/` number allocated now, the PRISMML-intake/hotpath precedent wording).

Date: 2026-10-05 · user-directed `@distill` (paired with the paper distill in `../riir-infer/.research/007_Triadic_GDN_Tensor_State.md`, which owns the paper math; this note owns the mining verdict).

## Source

`OliverSieberling/cute-triadic-gdn` **@ `2caa4098073da92c2ba0d573df6453ceb7dbce45`** (tip = the Blackwell PR-1 merge, "Merge pull request #1 from OliverSieberling/blackwell"), **MIT**. Inspection date 2026-10-05, clone `.raw/cute-triadic-gdn` deleted at close, every quote below re-verified at the pin. Companion repo `TriadicLinearAttention` @ `d69108ef` (MIT) — training loop, routed to riir-train Plan 442, not a corpus source.

Completeness signals: README with wire-level tensor table + shapes + env knobs; a real test (`tests/test_joint.py`, kernels vs torch reference, fwd + ALL 7 gradients); FP64 verification at the 1.3B shape (rel ℓ₂ < 1%, paper App. A); `torch.library` custom-op wrapping (torch.compile-safe); provenance chain cited in NOTICE (chunkwise schedule follows **FlashQLA** — QwenLM's Hopper GDN kernels; conv kernel adapts **flash-linear-attention** — both MIT). No `todo!`-class stubs found in the kernels tree. Module-grade: complete.

## Domain routing

**`kernel_opt`** — CUDA/CuTe kernel patterns around a chunked recurrent (Gated DeltaNet) pipeline. The tempting WRONG domain: `rust_perf` (no CPU SIMD content whatsoever). Secondary framing refused: this is NOT a `measured-GOAT` distillation — see NotBenchable below.

**NotBenchable (domain reason, per the 2026-09-17 dial9 correction — an annotation, never a demotion):** the kernels are cuTe DSL for **sm90 + sm100/sm103** (README: "Hopper (H100, sm90) and Blackwell (sm100/sm103, tested on B300)"). The validator has no host for either triple — our CUDA lane is the 4090 (**sm89**), our M3 lane is Metal/CubeCL. The win metrics (fwd+bwd wall on H100, mask-recompute memory on 128k-token batches) live in a runtime no validator arm can host. Every candidate below carries `bench_spec: None`; the batch grades on decision content × novelty × dedup clearance, the Batch-136 precedent (NotBenchable rules land as first-class decision-content entries).

## Dedup-gate evidence (both vocabularies, greps over `src/domains/kernel_opt/` 2026-10-05)

- **Tiling / state residency:** corpus `row-owned-register-blocked-recurrence` (Batch 28 deltanet cluster) keeps ONE d×d state slice register-resident per (head,row) — the fits-on-chip regime. Candidate R1 is the **doesn't-fit regime**: state d×E×d exceeds on-chip memory → tile the NON-interacting axis. Zero hits for "tile the non-interacting/value axis"; "value axis" colocation hits are GEMV tiling, different decision. NOT a positive instance.
- **Decay stability:** corpus `sign-branch-stable-sigmoid` owns sigmoid-eval overflow; `beta-decay` backward cluster (Batch 24) owns decay GRADIENTS, not decay-FACTOR evaluation. Zero hits for log-gate-diff decay forms or an exp-ratio anti-lever. R2 clears.
- **Recurrence-axis checkpointing:** corpus `activation-recompute-budget` is the per-layer activation cache-vs-recompute axis; zero hits for chunk-boundary recurrent-state stores or bitwise-reproducible backward contracts. R3 clears (composes with, does not duplicate).
- **Warp specialization / TMA:** corpus `autotune-warps-stages` is a config sweep, not the producer/consumer structure; no warp-specialization or TMA rule exists. Candidate considered and DROPPED anyway (below).
- **Varlen packing:** zero `cu_seqlens`/varlen hits; the packed-variant candidate folded into R3's rider (below) rather than standing alone.

## Ranked candidates

| # | pattern | source file | grade | notes |
|---|---|---|---|---|
| R1 | `state-slice-resident-tiling-noninteracting-axis` — recurrent state too big for on-chip memory → split along the axis whose slices never interact (value axis, 32-col blocks/thread block), keep ALL interacting-axis slices (E) of the block register-resident for the whole sequence; slices interact only through the output sum. Regime: state bytes > RF/SMEM budget, slices along the chosen axis carry no cross-slice dataflow. | README + paper §2.5 + `sm90_joint_fwd.py` (32-col blocks; paper: 512 KiB/head at E=8 vs 128 KiB/block) | **B** | the axis-choice decision is the content; distinct-from `row-owned-register-blocked-recurrence` (fits-regime) + `dynamic-shared-memory-runtime-sized` (sizing, not axis choice) |
| R2 | `log-gate-diff-decay-no-clamp-stability` — per-slice scalar decay factors evaluated as exp(diff of accumulated log gates): the difference is non-positive by construction → **no clamp, ever**; the cheaper mid-chunk-reference refactor (2CE exponentials) creates positive exponents that overflow FP32 at e^88.7 — **measured refusal** (FP64 emulation of just that clamp: rel err 0.05 @ e⁻²⁰⁰, 1.5 @ e⁻⁶⁰⁰). Two-regime: safe-form vs cheaper-but-overflowing. | paper App. A "Numerical stability" (mirrored in kernel log-gate accumulation) | **B+** | the measured anti-optimization is the GOAT-shaped content (B189 measured-negative-ledger class); distinct-from `sign-branch-stable-sigmoid` (sigmoid eval) |
| R3 | `chunk-boundary-state-checkpoint-bitwise-backward` — forward stores chunk-start recurrent states in BF16 so backward skips the recurrence recompute; backward = short reverse recurrence (state grads) + parallel per-chunk grads; fixed reduction order, **no FP atomics → bitwise-reproducible gradients**; rider: `GJ_SAVE_MASKS=0` recompute-vs-store env (bitwise identical, 768 MiB @128k). | `sm90_joint_fwd.py:1` docstring + `ops/gdn_joint.py:170-173` + paper App. A | **B−** | the recurrence-axis store-vs-recompute knob + determinism contract; distinct-from `activation-recompute-budget` (per-layer activations) — the varlen-packed docs + occupancy fallback (one block per doc/head/value-block, 16-col blocks when too few docs) ride as the rule's second face |
| D1 | warp-specialized 4-warpgroup producer/consumer + TMA double-buffer + per-role `warpgroup_reg_alloc/dealloc` | `sm90_joint_fwd.py` | **DROP** | real structure, Hopper/Blackwell-specific machinery (TMA, WGMMA, reg realloc) with no transferable decision content to our CubeCL/Metal + sm89 lanes — the sm90 warp-spec rung on the M3 was already measured-closed (riir-ai Plan 597/Bench 933); re-open only if a cuTe-family lane ever exists |

## Yield estimate

2–3 rules after coordinator falsification (R1, R2 near-certain; R3's mask/env rider may fold into R2/R1 if the coordinator reads them as one store-vs-recompute rule). Corpus delta: +2–3 kernel_opt, all annotated (`without_spec` 80→82-83 class).

## Product-coverage axis

The cuTe DSL (CUTLASS 4.x Python) is an **emerging kernel-authoring family with zero corpus entries** — this verdict seeds the family's follow-up pool: CUTLASS examples tree, `fla-org/flash-linear-attention`, `QwenLM/FlashQLA` (the schedule's own upstream). If a later batch needs a cuTe-fluent source, those three are the canonical pool; accumulation clause applies (pool sources read before the batch window fold into the same batch).

## League/lane-intel axis — HIGH

- Maps to **riir-ai Issue 971** (4090 prefill hold defense — chunked-GDN prefill, Plan 533 Phase 2-4 revival, trigger NOT fired as of the 10-03 watch): this repo is a **second complete reference implementation** of the chunkwise GDN family (FlashQLA-following schedule, fwd + full bwd, packed varlen, E=1 ≡ GDN) beside llama.cpp PR #26001 and the gdn-fused-v2 thread. Dated interim entry appended to the issue this date.
- Successor-arch watch (recorded, not armed): a triadic-class opponent checkpoint moves long-context quality cells at fixed params; decode reads are E× the state traffic and the paper measures TRAINING only — neither half is measured on league shapes. Full sizing: riir-infer Research 007 §League.
- NOT transferable to the league rows today: no triadic checkpoint exists; the kernels don't run on the 4090 (sm89); the chunkwise revival itself stays trigger-gated per Issue 971's own contract.

## Honest caveats

- All rules NotBenchable (no sm90/sm100 host) — decision-content batch only.
- The upcycling recipe is **paper-only** (the training repo ships no surgery script) — provenance split recorded in riir-train Plan 442.
- Single-author young repo, 6 days old, one PR merged; patterns are timeless but re-pin at mine time if the tree moved.
- The paper's kernel description (App. A) and the repo agree at every point checked (warp roles, BF16 chunk-state store, mask knob, FP64 verification) — no staging drift found.

## Queue slot

BEHIND Batch 201 (claimed, in flight on shikuwa/4090 — the yield discipline blocks any corpus landing until it lands). Next kernel slot after B201; **plan-first Protocol A step 0 AT CONVERSION** (no plan number allocated now). If the Issue 971 revival window fires first, the batch MAY fold into that work session (same files, fresher context) — owner's call at conversion.
