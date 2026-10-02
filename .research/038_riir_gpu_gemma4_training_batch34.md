# Research 038 — riir-gpu Gemma-4 training cluster (Batch 34)

**Date:** 2026-08-15
**Batch:** 34 (kernel_opt domain)
**Sources:** `riir-gpu/src/gemma4_cubecl_train/{mod.rs,backward_gpu.rs}` (1,316 LOC) + verified secondary context (`gemma4_cubecl/{dispatch.rs,kv_cache.rs,mod.rs}`, `gemma2_cubecl/dispatch.rs`, `gemma2_cubecl_train/mod.rs`, `gemma4_q4k_train/mod.rs`, riir-engine `gemma4_train/{mod,backward}.rs`, `gemma4.rs`, katgpt-transformer `kv_cache.rs`)
**Corpus:** 258 → **267 rules** (A 46, B 44, C 28, D 43, E 16, F 10, G 19, H 11, I 4, J 5, K 4, L 9, M 9, N 15, O 4)

## 1. Why this cluster

Batch 25 (Research 026) mined the Gemma cluster's **inference** side
(`gemma2_cubecl/` + `gemma4_cubecl/`). This batch mines the **training**
counterpart: `gemma4_cubecl_train/` — a hybrid GPU training forward
(GEMVs/attention on GPU, elementwise + activation saving on CPU at 4
sync points per layer) + a Phase 2.5 GPU backward (7 per-layer
transposed GEMVs via pre-transposed handles; elementwise + weight-grad
outer products on CPU). The sibling modules it cross-references
(`gemma2_cubecl_train`, `gemma4_q4k_train`) were read for recurrence
evidence but are NOT yet mined — they remain future batches.

## 2. The 9 rules landed

| # | rule_id | cat | concept |
|---|---------|-----|---------|
| 1 | `exact-op-parity-for-analytic-backward` | N | the training forward must evaluate the EXACT op the analytic backward differentiates (standard tanh, not a fast approximation) — otherwise forward value and derivative come from two different functions and every gradient is systematically biased |
| 2 | `mirror-ring-store-sliding-window-contiguity` | A | a sliding-window KV ring allocated as 2·sw mirrored slots makes any ≤sw window contiguous by construction — readers take one slice, never per-element modulo or wrap-assembly; invariant: raw reads need seq_len ≤ 2·sw |
| 3 | `split-dispatch-expose-fused-intermediate` | D | a fused chain returning only its final output forces consumers needing the intermediate (backward activation saving, LoRA insertion) to re-launch the producer; ship a split entry returning both from one execution |
| 4 | `preallocated-activation-slots-inplace-save` | A | training activations written into pre-sized `[seq][dim]` slots via `copy_from_slice` at the moment of production — the forward loop is alloc-free regardless of how many tensors are saved; size asserted once |
| 5 | `hybrid-cpu-elementwise-gpu-throughput-sync-points` | D | route only throughput-bound kernels (GEMV/attention, batched where siblings share input) to GPU; elementwise runs CPU-side in the sync-point gaps — exactly 4 syncs per layer per position, every elementwise op launch-free |
| 6 | `pre-transposed-consumer-layout-handles-build-once` | L | when a frozen operand is consumed in the OPPOSITE layout (backward W^T), materialize the consumer-layout copy once at setup into resident handles (~2× memory) under the frozen-base contract — zero per-step transposes |
| 7 | `shape-matched-sibling-weight-grads-share-input-loop` | C | wk+wv / gate+up rank-1 weight-grad updates with identical row counts share ONE walk of the shared input operand — one load, two FMA streams (honest caveat: L1-resident CPU inputs make this loop-overhead halving; the traffic win is on device) |
| 8 | `op-major-backward-loop-inversion-batch-sequence-axis` | D | positions are independent inside each backward op stage (attention backward sits BETWEEN stages); invert (layer→position→op) to (layer→op-stage→all positions) so every per-position GEMV becomes one sequence-batched GEMM + one readback — 7/layer/step instead of 7×seq_len |
| 9 | `backward-helper-expose-shared-norm-statistic` | H | a backward helper that computes a norm statistic (inv_rms) the same call site's parameter-grad also needs must return it — hidden statistics force every site to recompute the full reduction (2-3× reduction waste) |

## 3. Seeds resolved by verification (the load-bearing calls)

**REJECTED — the softcap "recompute" hypothesis (mine).** I expected the
backward's `logits[v] / logit_cap` re-tanh to be a recompute-from-input
hazard. Subagent 2 falsified it: the forward saves **post-cap** logits,
so `logits[v]/cap` is *algebraic recovery of tanh from the cached
output* — the tanh variant of `sigmoid-deriv-recovered-from-output`
(Batch 32a). The code is already optimal there. Mining it would have
duplicated an existing rule on a false premise.

**REJECTED — the dual-KV-cache "pattern" (my S4 seed).** K/V stored into
BOTH the GPU module cache AND the MultiLayerKVCache looks like
per-consumer materialized views, but both are CPU-side `Vec<f32>` and
the CPU training forward proves the MultiLayerKVCache alone suffices for
attention reads — the duplication exists only to reuse the inference
dispatch surface unchanged. Filed as hazard H9 (2-3× KV memory +
double store), NOT as wisdom.

**CONFIRMED-WITH-CORRECTION — exact-op parity (my S1 seed).** The rule
stands on the code + the engine-side backward (which differentiates
standard tanh at `gemma4_train/backward.rs` L268-272). But the module
header's supporting contrast — "the inference forward (Phase 1) uses
fast_tanh" — is **FALSE for the GPU path**: `gemma4_cubecl`'s `gelu_tanh`
+ `softcap` both call `.tanh()`; `fast_tanh` has zero occurrences in
riir-gpu (grep-verified). The real fast_tanh consumer is the CPU
inference softcap (riir-engine `gemma4.rs` L548-550). Doc bug → H13.

**FOUND BY VERIFICATION — the mirrored-ring OOB boundary (subagent 1).**
The mirror layout's raw-index read contract (`t·kvd` with no remap, in
the CPU backward) holds only while every attended `t < 2·sw`. Index
algebra verified: for `t ∈ [sw, 2sw)`, `t%sw = t−sw` so the mirror slot
IS the raw slot — correct AND in-bounds below the boundary; **OOB panic
beyond it** (sliding_window=1024 → seq_len > 2048 panics; no assertion
anywhere). Filed as Issue 687 H1, and the invariant ships inside the
rule text.

**CONFIRMED — pre-transposed handles triple recurrence.** The pattern
ships in `gemma2_cubecl_train` (build-once comment at its training
loop), `gemma4_q4k_train` (memoized `is_none()` variant), and this
cluster — three independent models, one pattern.

## 4. Bug yield — riir-ai Issue 687 (13 hazards)

| # | sev | hazard |
|---|-----|--------|
| H1 | P1 latent | mirrored-ring backward OOB panic for seq_len > 2·sw — no assertion; correct (index algebra) below the boundary |
| H2 | P1 | backward-handle staleness under `replace_weights` (no invalidation) + self-contradictory LoRA doc: forward runs MERGED weights but the doc claims base-only transposes suffice — base-only handles drop the ΔW^T·g input-grad term (found independently by both subagents) |
| H3 | P1 perf | Phase 2 processes zero-gradient positions — only Phase 1 skips empty `d_logits`; loops A/C run 7 GEMVs + CPU ops on zeros for the (n_in−1)/seq_len fraction (mathematically safe to skip: zero grad propagates to zero through every op in the chain) |
| H4 | P1 perf | ~2.06M allocations + ~27 GB alloc/free per training step — 6 layer-scope `vec![vec![0.0; dim]; seq_len]` per LAYER + per-position n-vecs + engine-internal per-head allocs |
| H5 | P1 perf | 345,088 blocking GPU round-trips per step (7 GEMVs × 48 layers × 1024 + 1024 lm_head); each `dispatch_gemv` is upload+launch+readback; wte_t GEMV re-reads 4 GB per position (~4 TB/step) — Rule 8 is the fix |
| H6 | P2 perf | the CPU weight-grad accumulation, not the GEMVs, is the per-step bottleneck — wte grad 1.006G FMA/position (17× the largest layer grad); all-layer rank-1 ≈ 44 TB RMW/step at seq 1024; the header's "transposed GEMVs are the bottleneck" claim is per-op true, per-step misleading |
| H7 | P2 | 24-line byte-identical CE-backward block duplicated in `forward_backward_gpu` + `_v2` (mechanical diff empty) + 2 MiB/position softmax+dl vocab alloc churn |
| H8 | P2 | `dispatch_attention_only` re-run — double KV-window build + extra upload/sync per layer per position (Rule 3's fix surface) |
| H9 | P3 | dual CPU-side KV caches: 2-3× KV memory + double store — avoidable; unify at the split-dispatch refactor |
| H10 | P3 | RoPE freq table derived twice (forward: precomputed tables; backward: inline formula per layer per step) — currently bit-identical (verified), but divergence = silent gradient corruption; also 5,632 powf/step waste |
| H11 | P3 | norm-statistic recomputation 2-3× at 7 sites (~393K redundant n-element reduction passes/step) — Rule 9's fix surface |
| H12 | P3 | `transpose_f32` naive scalar strided transpose at 12B scale (47.8 GB, ~1 MB stride) — minute-class one-time; instance of `offload-cpu-bottleneck-to-gpu-kernel` unapplied |
| H13 | P4 docs | header misattributes fast_tanh to GPU inference (false); `reserve(stride)` reserves one position not sw; dead `let _ = stride;` / `let _ = layer_weights;`; dead `weights` param threading + silent stale-weight trap (B6) |

## 5. Candidates dropped as duplicates/thin (12)

- softcap tanh "recompute" — falsified (already `sigmoid-deriv-recovered-from-output`)
- `d_logits` empty-Vec skip — instance of `predicate-gated-skip-empty-workset-step`
- dual-KV-cache — rejected as wisdom (hazard H9)
- RefCell scratch reuse — thin vs `caller-owned-split-scratch` + `reuse-allocation-clear` (subagent self-assessed)
- embed-scale folding — `loop-fusion`/rust_perf territory
- `all_tokens` concatenation — idiomatic, no kernel wisdom
- test re-exports — API ergonomics, not kernel opt
- KV-reset loop — buggy-noisy (H13), disqualified as wisdom
- freq-table rebuild — issue material (H10), concept covered by `deterministic-tiling-projection-tables`
- reverse-layer loop + ownership move — standard backward ordering
- per-layer-type dims — correctness plumbing
- Phase-3 embedding scatter — instance of `active-index-grad-scatter-subset-backward`

## 6. Method notes

- **The Batch 33 verification protocol paid off again**: both subagents
  independently verified against the DEPENDENCY side (engine backward,
  dispatch surface, substrate cache) rather than trusting the module's
  own comments — which is exactly how the fast_tanh doc-lie (H13), the
  OOB boundary (H1), and the softcap falsification were found. The
  mod.rs comments describe intent; three of them describe it WRONG.
- **Coordinator seeds went 4-for-6 with two corrections** — the falsified
  softcap seed and the rejected dual-cache seed were both my own; the
  subagent falsification step is now 2-for-2 batches in preventing
  would-be wisdom bugs.
- **Quantification at real scale**: the subagents pulled the real 12B
  config dims (48 layers, n=3840, mlp=15360, vocab=262144) from the
  config-match tests — the issue's arithmetic (1.006G FMA/position,
  345,088 round-trips, 44 TB RMW) is grounded, not hand-waved.
- Generator landing: the one-shot Python script hit TWO anchor bugs
  before write (a `%`-formatting collision with literal `%` in concept
  text — fixed by plain concatenation; a double-update of the D-header
  comment) — both aborted cleanly with zero repo pollution, then
  landed first-compile after fix (1 doc-warning case-fix in the
  concepts test aside).

## 7. GOAT

- **G1:** `batch34_pseudocode_parses` 9/9 (all ref_shape + opt_pattern
  syn-parse) + `batch34_rules_present` + `batch34_concepts_covered` (3
  new tests).
- **G2:** 318 kernel_opt (was 315) + 201 default lib tests PASS.
- **G3:** clippy 0 warnings in all 4 states (kernel_opt/default ×
  lib/all-targets); 8 doc warnings fixed pre-commit (backticks + a
  leading `+` that markdown parsed as a list item — new lint variant).
- Orphan-`rule_id` scan: 0/267.

## 8. Progress

riir-gpu: **~74 of ~200 files mined** (the Gemma-4 training cluster
done; gemma2_cubecl_train + gemma4_q4k_train remain). Corpus: **267
rules**. Next candidates: `gemma2_cubecl_train/` (mod.rs 1,286 +
resident.rs 1,639 + attention_gpu.rs 349 — the Gemma-2 training
residuals, natural continuation), `gemma4_q4k_train/` (4 files),
`gemma2_d2f*`, `kimi_k3_gpu_*` (4 files), `weaver_gpu*` (4 files),
`kvarn/`, `hypernet/`, `maxsim/`. `ternary_deltanet_gpu_forward.rs` +
`_cudarc.rs` still BLOCKED on sibling WIP.
