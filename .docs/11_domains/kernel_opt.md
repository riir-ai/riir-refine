# `kernel_opt` domain (Plan 001 Phase 3, absorbs Proposal 012)

> The third `HealDomain` impl. Heals GPU kernel optimization. Absorbs
> Proposal 012's design (thermal-tier + latent-MoE + numeric validator) into
> the multi-domain framework. **4090 LoRA deferred** across all tiers above
> Plasma.

## What ships vs deferred (Proposal 012 §Phase 0)

| Component | Ships now | Deferred |
|---|---|---|
| `KernelOptDomain: HealDomain` | ✅ | |
| Seed corpus (Batch 1-17: 135 rules; popcorn COMPLETE; riir-gpu opened Batch 14, mined Batch 15-17) | ✅ | Future source: riir-gpu (~196K lines, mostly unmined), new popcorn releases |
| `KernelExpertRouter` (`pick_domain`) | ✅ (8-dim activity, 15/15 reachable, Plan 004) | Mined centroids via `CommittedFieldBlend` |
| `KernelThermalRouter` (5-tier stub) | ✅ (returns Plasma) | Warm/Cold LoRA (4090, riir-train) |
| `KernelVerifier` (compile-only) | ✅ (behind `kernel_verify`) | Full numeric compare vs reference |
| L4 LoRA fallback | | ✅ Deferred across all domains |

## Corpus

Batch 1 (Research 002): 21 rules — 2 original + 19 popcorn-distilled across 5 categories.
Batch 2 (Research 003): 7 rules — flash-attention tiling patterns.
Batch 3 (Research 004): 6 rules — linear-attention recurrence patterns.
Batch 4 (Research 005): 9 rules — MoE dispatch + backward-pass patterns.
Batch 5 (Research 006): 4 rules — NSA block-sparse selection patterns.
Batch 6 (Research 007): 3 rules — varlen packing + chunked reductions.
Batch 7 (Research 008): 9 rules — burn-cubecl autotune dispatch + conv-as-GEMM + memory layout.
Batch 8 (Research 009): 9 rules — burn-cubecl kernel polymorphism + dtype/alignment specialization + cubecl-opt IR strength reduction.
Batch 9 (Research 010): 8 rules — popcorn fusion + dtype/quant + dispatch (add_rms_norm, group_norm_linear, jsd, int8_int2_matmul, _quant).
Batch 10 (Research 011): 12 rules — popcorn compute + chunked reduction + polymorphism (mean_pooling, solve_tril, hadamard_transform, comba, dyt, token_shift, sparsemax, selective_log_softmax, softmax, rms_norm, geglu, swiglu, rope, gelu).
Batch 11 (Research 012): 11 rules — popcorn linear-attention recurrence family (gla, simple_gla, rwkv7, delta_rule, iplr_delta_rule, gated_oja_rule) + parallelism (chunk_global_cumsum, retention, lightning_attn) + flash-attn tiling (deltaformer) + compute (based, rebased, linear_attn) + dispatch (lightning_attn). A 12th candidate (chunk-reset-scan) was dropped as a duplicate of chunk-reset-cumsum (Batch 6).
Batch 12 (Research 013): 14 rules — popcorn gated delta family (gated_delta_rule, gated_delta_product) + specialized recurrences (mesa_net, mesa_net_decode, titans_linear, ttt, rwkv4, rwkv6) + advanced attention (wall_attn, multi_token_attention) + MoE gate (momoe) + KDA gate (kda_gate, kda_gate_cumsum) + hierarchical mask (log_linear_attn). Three candidates (kda per-key-dim gate, forgetting_attn cumsum decay, abc two-state softmax) were dropped as duplicates.
Batch 13 (Research 014): 6 rules — popcorn final unmined pass. RL loss stability (grpo_offpolicy: expm1 for KL cancellation) + complex-free RoPE (llama4_rope: paired reals) + multimodal RoPE (qwen2vl_mrope: cyclic modality table) + RWKV-7 channel mixing (rwkv7_channel_mixing: stateful cat-prefix shift + squared ReLU from sqrelu) + RWKV-7 gate output (rwkv7_gate_output: fused reduce-then-broadcast). rwkv7_k_update + sigmoid + tvd yielded no new rule.
Batch 14 (Research 015): 4 rules — **first batch mining riir-gpu (the stack's own production kernels), not external libraries**. Norm-fold into matmul (gemv_q4k_batched_rmsnorm: fold inv_rms scalar into the GEMV dot) + reduction-split pre-pass (compute_inv_rms: scalar pre-pass + fold) + single-plane reduction (Issue 611 fix: one plane suffices vs N redundant planes) + CPU-bottleneck offload (gpu_transpose Issue 402: GPU compute kernel writes directly into pooled memory). Two riir-gpu files (lora_fused SGMV, rope_geglu section-view) assessed + dropped (SGMV overlaps MoE grouped; section-view is pointer arithmetic not an opt).
Batch 15 (Research 016): 4 rules — **second batch mining riir-gpu**. Block-contiguous weight layout (gemm_ternary_block_contiguous: pack 3 separate weight buffers into 1 to cut global-memory accesses 3×) + fused dual GEMV + GeGLU (gemv_geglu_f16: fuse 2 sibling GEMVs + act into 1 dispatch, saves 52 dispatches/decode at 26 layers) + orthogonal pre-rotation (spectralquant/attention.rs: pre-rotate query once vs each key, ~4096× fewer rotation FLOPs) + water-fill tail-dim shared codebook (spectralquant/dequant.rs: top dims per-dim codebooks, tail dims share one low-bit codebook, 2.3× codebook memory reduction). One riir-gpu file (forward_flashprefill) assessed + dropped (cascade structure overlaps NSA Category I; 4 staged dispatches is a non-opt).
Batch 16 (Research 017): 4 rules — **third batch mining riir-gpu**. cmma hardware unit over scalar ALU (gemm_ternary_simdgroup: use the cooperative matrix multiply-accumulate hardware unit instead of scalar ALU — 64 FMAs/cycle on the dedicated unit) + dequant-once-per-K-tile into shared memory (gemm_ternary_simdgroup: dequant a weight tile once, let the hardware matrix unit consume it across all output rows — m× amortization) + swap-large-axis-to-X (gemm_ternary_simdgroup: put the large GEMM axis on the X cube-count for workgroup distribution) + norm-decoupled low-bit scoring (forward_turboquant: store per-position L2 norm separately, quantize the unit-normalized key at 2-4 bits, reconstruct score as norm × dot). Four files assessed + dropped (lora_fused SGMV overlaps Category H; spectralquant/rotation boilerplate; spectralquant/qjl composition of water-fill with JL; spectralquant/calibration two-pass reduce overlaps split-reduction).
Batch 17 (Research 018): 4 rules — **fourth batch mining riir-gpu (ternary scalar-ALU hot loop)**. Skip-all-zero-packed-words-via-or-reduce (gemm_ternary_batched: OR-reduce all tile rows' pos+neg bit-words, skip the 32-iteration sign-extraction loop when the whole tile's word is zero — ternary sparsity exploit) + hoist-group-invariant-scale-out-of-word-loop (gemv_ternary_block_contiguous: cache the per-group quant scale in a register across the 4 words it covers, reload only at the group boundary — 4× fewer global loads) + split-sign-accumulators-defer-subtraction (both ternary files: keep separate pos/neg accumulators inside the bit loop, subtract once at the end — 32→1 subtracts/word + independent ILP chains) + clamp-tile-indices-to-sentinel-discard-at-write (gemm_ternary_batched: alias OOB tile rows to m-1, compute fully, discard at write time — keeps the loop body uniform without padding overhead). Two files assessed + dropped (rope_geglu_cubecl: pairing-generic + precomputed cos/sin table are composition of existing dispatch/precompute principles; forward_dflare: 17-binding layout + heterogeneous KV are architectural, not kernel optimization).
Total: 135 rules. Popcorn corpus COMPLETE; riir-gpu (~196K lines) remains open as the next distillation source (9 of ~200 files mined).

### Category A: Memory access patterns (10 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `matvec-coalesce` | coalesced memory access | Strided read (uncoalesced) → transposed coalesced read |
| `coalesced-row-read` | coalesced memory access | Strided load (H*D apart) → contiguous load (adjacent) |
| `vectorized-pack-load` | vectorized memory access | Scalar load → 4-element packed load (16-byte transactions) |
| `grid-stride-loop` | work distribution | Fixed one-thread-per-element → grid-stride for arbitrary n |
| `gemm-tiling` | shared-memory tiling | Naive GEMM (global mem) → tiled GEMM (shared mem reuse) |
| `masked-boundary-load` | boundary-safe memory access | Unchecked load → masked load with fallback |
| `offload-cpu-bottleneck-to-gpu-kernel` | CPU→GPU data-prep offload | CPU transpose (674ms/step bottleneck) → GPU compute kernel writing into pooled memory (riir-gpu Issue 402) |
| `block-contiguous-weight-layout` | multi-buffer → single-buffer packing | 3 separate SoA weight buffers (pos+neg+scale) → 1 block-contiguous AoS buffer (1 access per group, riir-gpu gemm_ternary_block_contiguous, 3× fewer global-memory accesses) |
| `dequant-once-per-k-tile-into-shared-memory` | amortize dequant across shared tile | Dequant each weight element inline as consumed → dequant a weight tile ONCE into shared memory, let the hardware matrix unit consume it across all output rows that share the tile (riir-gpu gemm_ternary_simdgroup, m× amortization) |
| `hoist-group-invariant-scale-out-of-word-loop` | register-cached group invariant | Reload scale from buffer every word iteration → cache the per-group quant scale in a register across the 4 words it covers, reload only at the group boundary (riir-gpu gemv_ternary_block_contiguous, 4× fewer global loads) |

### Category B: Compute patterns (23 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `autotune-warps-stages` | autotune dispatch | Hardcoded threads → sweep over (warps, stages) |
| `upcast-before-reduction` | numerical precision | fp16 reduction → fp32 upcast before mean/sum |
| `branchless-select` | branch divergence elimination | Conditional branch → select/if-expr |
| `unroll-fixed-inner` | loop unrolling for ILP | Dynamic loop → fixed-size unrolled |
| `forceinline-device-fn` | function inlining | Regular call → `#[inline(always)]` |
| `recursive-kronecker-cat` | structured matrix construction | Element-wise build → recursive cat (O(log d) steps) |
| `optional-bias-branch` | optional operand skip | Always-add bias → branch on `None` (skip when absent) |
| `shift-via-pad-subtract` | one-pass sequence shift | Two-pass shift+sub → fused pad+subtract view |
| `sorted-cumsum-threshold` | support-set selection | Iterative projection → sort+cumsum+threshold (single pass) |
| `polynomial-taylor-feature-map` | softmax kernel approximation | Full softmax → polynomial Taylor feature map (Based: 1+x+0.5x²) |
| `positive-feature-map-shift` | non-negative normalizer | Raw signed q/k → elu+1 shift (normalizer never crosses zero) |
| `hierarchical-log2-level-mask` | O(log seq) hierarchical mask | Dense N×N decay → O(N log N) sparse hierarchical (per-level scales) |
| `iterative-cg-solve-over-matvec` | conjugate-gradient solve | Materialize inverse → CG over matvec with Jacobi preconditioner |
| `chunked-test-time-gradient-descent` | mini-batch chunked GD | Sequential per-token GD → chunked GD with causal mask + last-step update |
| `expm1-for-near-zero-exp-minus-one` | numerical stability (cancellation) | `exp(x)−1` → `expm1(x)` (avoids catastrophic cancellation near 0) |
| `complex-mul-via-paired-reals` | complex-free representation | Complex dtype → interleaved `[real, imag]` pairs + 4 scalar muls |
| `stateful-shift-via-cat-prefix` | cross-batch stateful shift | Zero-pad shift → `cat(x_prev, x[:-1])` (carries real cross-batch state) |
| `squared-relu-activation` | sparse sharpening activation | `relu(x)` then square → fused `max(x,0)²` (zeros preserved, non-zeros sharpened) |
| `pre-rotate-once-exploit-orthogonality` | orthogonal pre-rotation identity | Rotate each key by R then dot with q → pre-rotate query once (`q'=R^T·q`) for orthogonal R, then dot `q'` with each unrotated key (riir-gpu spectralquant/attention, ~4096× fewer rotation FLOPs) |
| `cmma-hardware-unit-over-scalar-alu` | hardware matrix unit vs scalar ALU | Scalar dot in a plane (occupancy-bound, ~1.08×) → cooperative matrix multiply-accumulate hardware unit (tensor core / Metal simdgroup_matrix / cmma, 64 FMAs/cycle on the dedicated unit, riir-gpu gemm_ternary_simdgroup) |
| `skip-all-zero-packed-words-via-or-reduce` | ternary sparsity exploit | Always enter the 32-bit extraction loop → OR-reduce all tile rows' pos+neg bit-words, skip the loop when the whole tile's word is zero (riir-gpu gemm_ternary_batched) |
| `split-sign-accumulators-defer-subtraction` | deferred sign subtraction | Subtract pos−neg per bit inside the loop (32/word) → keep separate pos/neg accumulators, subtract once at the end (1/word + independent ILP chains, both ternary files) |
| `clamp-tile-indices-to-sentinel-discard-at-write` | divergence-free tile bounds | Per-lane bounds check inside the loop → alias OOB tile rows to m-1 sentinel, compute fully, discard redundant results at write time (riir-gpu gemm_ternary_batched) |

### Category C: Fusion patterns (12 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `fuse-norm-act-linear` | kernel fusion | 3-pass norm+act+linear → single fused pass |
| `fuse-quant-matmul` | quantization fusion | Separate quant+matmul → fused (BitNet b1.58) |
| `fuse-softmax-mask` | mask fusion in softmax | Separate mask+softmax → fused masked softmax |
| `fuse-xent-logsumexp` | cross-entropy fusion | Softmax+NLL → fused logsumexp (no prob matrix) |
| `fuse-add-rms-norm` | residual add + RMS norm fusion | 2-pass add+norm → fused (returns new residual too) |
| `fuse-group-norm-linear` | group norm + linear fusion | 2-pass norm+linear → fused (no intermediate tensor) |
| `fuse-jsd-logsumexp` | log-domain divergence fusion | Materialize prob matrices → logaddexp (no matrices) |
| `gather-selective-reduction` | selective reduction | Full softmax → gather selected tokens only (GRPO) |
| `fuse-gate-transform-chunk-cumsum` | gate + cumsum fusion | Separate gate + chunk cumsum → fused (no intermediate) |
| `rkv-receptance-correction` | fused reduce-then-broadcast | 4-pass triple-mul+reduce+broadcast-mul+add → fused `(r·k·ρ).sum(keepdim)·v + o` |
| `fold-norm-scalar-into-matmul` | norm-fold into matmul | Separate CPU norm pass + upload → fold inv_rms scalar into the GEMV dot product (riir-gpu gemv_q4k_rmsnorm, ~30s/step saved at 12B) |
| `fuse-gate-up-gemv-activation` | sibling-GEMV + activation fusion | 3 dispatches (gate GEMV + up GEMV + GeGLU) → 1 fused kernel (riir-gpu gemv_geglu_f16, saves 52 dispatches/decode at 26 layers) |

### Category D: Dispatch patterns (9 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `sm-count-block-size` | occupancy-aware block sizing | Fixed block → SM-aware `min(128, next_pow2)` |
| `predicate-gated-backend` | conditional kernel dispatch | Unconditional fast path → shape/dtype predicate gate |
| `pow2-block-alignment` | power-of-2 alignment | Arbitrary dim → `next_power_of_two` |
| `predicate-gate-div512` | shape-divisibility gate | Unconditional packed path → `inner_dim % 512 == 0` gate |
| `multi-condition-predicate` | compound shape+dtype gate | Single condition → multi-condition (rows%32 AND width%8) |
| `layer-depth-decay-schedule` | depth-aware parameter schedule | Constant decay slope → slope ∝ (1 − layer_idx/num_layers) |
| `dual-mode-gate-transform` | dual-mode gate dispatch | Hardcoded formula → unbounded softplus / bounded sigmoid by parameter |
| `cyclic-modality-table-selection` | multimodal RoPE dispatch | Single RoPE table → split head_dim into sections, cyclic gather from N modality tables |
| `swap-large-axis-to-x-for-workgroup-distribution` | dispatch axis swap for SM distribution | (M,P) cube count with small M on X under-utilizes SMs → (P,M) swap so the large axis gets X-tiles for even workgroup distribution (riir-gpu gemm_ternary_simdgroup, mirrors matmul_swap_ab) |

### Category E: Parallelism patterns (8 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `multi-dim-grid-launch` | multi-dimensional parallelism | 1D nested loop → 3D parallel grid (batch×head×time) |
| `block-count-clamp` | grid dimension clamping | Unbounded blocks → clamped to 65535 |
| `backward-as-conjugate` | backward kernel reuse | Separate backward → forward with conjugate flag |
| `reverse-scan-via-double-flip` | reverse-direction scan | Sequential reverse loop → flip+cumsum+flip (reuses forward scan) |
| `pairwise-distance-decay-matrix` | decay matrix materialization | Sequential per-step decay → N×N matrix from arange distance |
| `softmax-topk-renormalize-scatter` | `MoE` sparse gate | Dense gate → softmax+topk+renorm+scatter sparse gate matrix |
| `split-reduction-scalar-prepass` | reduction-split for fusion | CPU reduction + upload full tensor → scalar pre-pass (O(batch) output) + fold into main consumer (riir-gpu compute_inv_rms) |
| `single-plane-sufficient-reduction` | redundant-plane elimination | N planes all computing identical sum → one plane (8× wasted FLOPs, riir-gpu Issue 611 fix) |

### Category F: Flash-attention tiling (10 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `online-softmax-streaming` | online softmax | 2-pass max+exp → streaming m/acc single pass |
| `exp2-log2-base2` | base-2 numerics | exp/log → exp2/log2 (faster on hardware) |
| `block-pointer-2d` | 2D block pointer | Manual offsets → block_ptr 2D tile load |
| `heuristic-compile-dispatch` | heuristic dispatch | Runtime branch → compile-time autotune heuristic |
| `causal-diag-offdiag-split` | causal mask split | Diagonal+off-diagonal → separate kernels (no mask check) |
| `shared-mem-aware-tile` | shared-memory tile sizing | Fixed tile → occupancy-aware (load vs compute bound) |
| `kv-block-streaming` | KV block streaming | Load all KV → stream one block at a time |
| `strict-causal-mask-tril-offset` | strictly-causal mask | tril(0) → tril(-1) (exclude diagonal) + NaN guard |
| `block-local-reference-frame-for-overflow` | overflow-safe decay | Global prefix diff (overflows fp32) → per-block reference frame (bounded) |
| `two-stage-mask-softmax-then-conv` | two-stage mask | Mask after conv only → -inf before softmax + 0 after conv |

### Category G: Linear-attention recurrence (17 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `state-forget-gate` | gated recurrence | Ungated write → `state *= exp(g); state += k⊗v` |
| `householder-rank1-update` | Householder reflection | Naive write → project out stored component then write |
| `chunkwise-parallel-recurrence` | chunked parallelism | Sequential recurrence → chunk-level parallel + inter-chunk reduction |
| `recurrent-vs-chunk-dispatch` | backend dispatch | Single path → recurrent (decode) vs chunk (train) |
| `multi-write-per-step` | multi-write recurrence | Single write → N writes per step (DeltaNet) |
| `chained-two-pass-recurrence` | two-pass recurrence | Single pass → two chained gated passes with softmax bridge |
| `per-key-dim-diagonal-gate` | per-dim decay gate | Per-head scalar gate → per-key-dim vector gate |
| `erase-replace-low-rank-transition` | diagonal+low-rank transition | 2-phase forget+write → 4-phase erase+decay+replace+write (RWKV-7) |
| `oja-subtractive-normalization` | Oja normalization | Write k⊗v → project out stored component `k − Sv` first |
| `identity-plus-low-rank-delta` | I+ba^T rotation | Forget gate → `(I+ba^T)` additive-rotational update |
| `cumsum-gate-to-decay-matrix` | pairwise decay from cumsum | Per-step gate loop → cumsum + pairwise diff + exp |
| `gate-then-correct-delta-rule` | gated delta rule | Correction against un-decayed state → decay first then correct |
| `gate-then-multi-householder-write` | gated multi-Householder | Gate per inner write → one gate + N inner delta writes |
| `online-gram-ridge-solve` | online ridge regression | Batch solve → online gated Gram accumulators + per-step solve |
| `momentum-gated-memory-recurrence` | SGD momentum + EMA memory | Plain GD → coupled momentum + EMA memory recurrences |
| `current-token-bonus-read-before-update` | boosted read + plain write | Write-then-read → boosted read (transient bonus) then update |
| `wkv-logsumexp-three-channel-state` | 3-channel logsumexp WKV | 2-channel EMA (overflow) → 3-channel (num/den/max) |

The patterns are Rust-shaped pseudocode so the shared `RustAnalyzerPruner` can
validate them in the fast path. When the full validator lands (CubeCL compile +
numeric compare), the patterns will carry real kernel source.

## Router — `KernelExpertRouter`

Wraps `katgpt_core::variable_rank_domain_expert::pick_domain<15, 5>` (Plan 558):

```rust
pub const NUM_KERNEL_EXPERTS: usize = 15; // one per corpus category (A–O)
pub const ACTIVITY_DIM: usize = 8;        // M, N, K, dtype, hw, is_attention, is_fusion, is_ir_transform

pub fn route(&self, activity: &[f32; ACTIVITY_DIM]) -> usize {
    pick_domain::<NUM_KERNEL_EXPERTS, ACTIVITY_DIM>(activity, &self.directions)
}
```

**Plan 003 (2026-08-14):** router wired to the 135-rule corpus via 15 category
experts. Each expert owns its category's `rule_ids` via `KERNEL_CATEGORIES`.
- **Activity vector** = raw physical (kernel signature: `[M, N, K, dtype, hw,
  is_attention, is_fusion, is_ir_transform]` — 8-dim per Plan 004, normalized
  to `[0, 1]`). The `build_activity()` helper is the consumer-facing constructor.
- **Domain directions** = latent (hand-crafted category centroids — see
  `CATEGORY_KERNEL_DIRECTIONS` const; mining via `CommittedFieldBlend` deferred).
- **Expert index** = raw categorical (synced).
- Modelless argmax — zero allocation.

**Corpus wiring accessors:**
- `category_for_expert(idx) -> KernelCategory` — maps expert → category.
- `rules_for_expert(idx) -> &[&str]` — maps expert → category's rule_ids.
- `expert_for_rule(rule_id) -> Option<usize>` — reverse lookup.

**Plan 004 (2026-08-14):** extended the activity vector from 5 to 8 dims by
adding 3 operation-class one-hot axes (`is_attention`, `is_fusion`,
`is_ir_transform`). **All 15/15 categories are now reachable** (was 13/15 in
Plan 003 — categories C and O were unreachable due to 5-dim rank deficiency).
The `build_activity()` helper provides the consumer-facing constructor for
constructing activity vectors from shape + operation-class flags.

The `kernel_heal` feature enables `katgpt-core/variable_rank_domain_expert`
so `pick_domain` is reachable.

**Composition example** (`examples/kernel_heal_demo.rs`): the router is
composed with the drafter + corpus end-to-end. The demo routes 8 representative
kernel signatures, narrows each to its category's rules (~4–6 vs 68 flat — a
15× candidate reduction), then drafts + validates a fix. This is the reference
composition for future kernel_opt consumers. Run with:
`cargo run --example kernel_heal_demo --features kernel_heal`.

## Thermal — `KernelThermalRouter` + `ThermalTier`

The FlashAR/QGF thermal-tier pattern: Plasma→Hot→Warm→Cold→Freeze escalation.

| Tier | What runs | Where | Latency | Status |
|---|---|---|---|---|
| Plasma | Modelless cache hit (Stage 1 drafter) | M3 Metal (local) | µs | ✅ shipped |
| Hot | Stage 2 adaptive drafter (Issue 010) | M3 Metal (local) | µs | ✅ shipped |
| Warm | LoRA draft + validate | 4090 CUDA | ms | ⏳ deferred (riir-train) |
| Cold | Full validate vs reference | 4090 CUDA | s | ⏳ deferred |
| Freeze | Archive snapshot | cold storage | offline | ⏳ deferred |

**Scaffold behavior:** returns `ThermalTier::Plasma` for ALL calls — the
modelless path is the primary tier per the modelless-first mandate. The
escalation logic (Plasma → Hot on cache miss, Hot → Warm on Stage 2 pool miss,
Warm → Cold on LoRA uncertainty, Cold → Freeze on archive trigger) lands when
the 4090 path is wired (riir-train).

`ThermalTier` is `#[repr(u8)]` (1 byte, sync-efficient):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ThermalTier {
    #[default]
    Plasma = 0,  // modelless cache
    Hot = 1,     // Stage 2 adaptive
    Warm = 2,    // LoRA draft — DEFERRED
    Cold = 3,    // full validate — DEFERRED
    Freeze = 4,  // archive — DEFERRED
}
```

## Verifier — `KernelVerifier` (compile-only stub)

Behind `kernel_verify`. **STUB** — delegates to the shared `RustAnalyzerPruner`
for syntax validation. Full CubeCL compile + numeric compare vs reference is
deferred (Proposal 012 §Phase 0). `is_full_validator_available()` returns
`false` in the stub so callers can detect the deferred state.

## Domain classification (per AGENTS.md)

- Kernel **shapes** (M, N, K, dtype, hw) = raw physical (deterministic,
  replayable — needed for quorum sync of kernel selection decisions).
- Expert **directions** = latent (dot-projection onto learned centroids,
  projected to a scalar expert index via `pick_domain`). NOT synced — only
  the resulting expert index crosses the sync boundary.
- Thermal tier = raw categorical (1 byte, `#[repr(u8)]`).

### Category F: Flash-attention tiling (Batch 2, 8 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `online-softmax-streaming` | flash-attention online softmax | Full matrix materialization → streaming running max+sum+rescale |
| `exp2-log2-base2` | base-2 math | Natural exp/ln → base-2 exp2/log2 (dedicated GPU instruction) |
| `block-pointer-2d` | structured 2D block pointer loads | Manual offset arithmetic → structured block ptr with boundary check |
| `heuristic-compile-dispatch` | compile-time heuristic dispatch | Runtime branch → compile-time specialization (dead branch elimination) |
| `causal-diag-offdiag-split` | causal matrix diagonal/off-diagonal splitting | Uniform precision → diagonal fp32 + off-diagonal bf16-safe |
| `shared-mem-aware-tile` | shared-memory-aware tile sizing | Fixed tile → SM-aware (Hopper larger, Ampere smaller) |
| `kv-block-streaming` | KV block streaming through shared memory | Full KV load → block streaming (O(block) shared mem) |
| `strict-causal-mask-tril-offset` | strictly-causal mask via tril(-1) | Standard causal tril(0) → tril(-1) (exclude diagonal) + NaN guard |

### Category G: Linear-attention recurrence (Batch 3, 11 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `state-forget-gate` | exponential decay gate on recurrent state | Unbounded accumulation → `state *= gate.exp()` |
| `householder-rank1-update` | rank-1 Householder reflection | General update → error-correcting delta rule |
| `chunkwise-parallel-recurrence` | chunkwise parallelism for state-carrying loops | Fully sequential → chunk-parallel (intra-chunk matmul) |
| `recurrent-vs-chunk-dispatch` | sequence-length-aware backend dispatch | Always chunk → recurrent for short, chunk for long |
| `multi-write-per-step` | multiple state writes per timestep | Single write → N Householder writes under one gate (DeltaProduct) |
| `chained-two-pass-recurrence` | chained sequential recurrences with softmax bridge | Single pass → K-state then V-state with softmax bridge (GSA) |
| `per-key-dim-diagonal-gate` | per-dimension diagonal decay gate | Scalar decay → per-key-dim vector gate (gla) |
| `erase-replace-low-rank-transition` | diagonal+low-rank state transition | 2-phase update → 4-phase erase/decay/replace/write (rwkv7) |
| `oja-subtractive-normalization` | Oja subtractive normalization | Raw write → project-out-then-write `β(k − Sv)` (gated_oja_rule) |
| `identity-plus-low-rank-delta` | (I+ba^T) state rotation | Decay-gated update → additive-rotational (iplr_delta_rule) |
| `cumsum-gate-to-decay-matrix` | pairwise decay from per-step gate | Per-step gate → cumsum+diff pairwise matrix (simple_gla) |

### Category H: MoE dispatch + backward pass (Batch 4, 9 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `grouped-launch-cumsum-offsets` | grouped GEMM with prefix-sum offsets | N separate launches → single grid with cumsum-derived group offsets |
| `l2-grouped-tile-schedule` | swizzled L2-cache-aware tile mapping | Row-major block map → grouped swizzle for L2 reuse |
| `fused-matmul-activation` | pointwise activation in matmul epilogue | Separate matmul+activation passes → fused epilogue |
| `fused-backward-chain` | two backward ops fused into one kernel | Two backward kernels with global handoff → one fused kernel |
| `activation-recompute-budget` | partial activation save with in-kernel recompute | Save all activations → save fraction + recompute rest (checkpointing) |
| `atomic-scatter-reduction` | lock-free atomic scatter for parallel writes | Locked scatter → relaxed atomic add |
| `weight-grad-batched-outer-product` | weight gradients as batched outer product GEMM | Per-token rank-1 updates → batched tiled GEMM |
| `csr-index-via-prefix-sum` | GPU-side CSR sparse layout via prefix sum | Host nonzero+sort → in-kernel prefix sum + atomic counter |
| `autotune-aggregate-workload-key` | autotune keyed on aggregate workload size | Fixed config → autotune on total work (Msum) for irregular batches |

### Category I: NSA block-sparse selection (Batch 5, 4 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `block-sparse-gather` | block-sparse indexed gather via selection array | Dense iteration over all N → index-array block gather |
| `precompute-scalar-reduction` | hoist invariant row reduction out of per-block loop | Recompute delta per block → precompute once in separate pass |
| `block-compression-before-routing` | pool to coarse blocks, route on compressed reps | Score all fine positions → pool to blocks, score compressed, attend top-k |
| `sliding-window-plus-sparse` | decompose attention into local + global sparse, gated sum | Single attention pass → local window + global sparse with gates |

### Category J: Varlen packing + chunked reductions (5 rules — 3 Batch 6 + 2 Batch 10)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `varlen-packing` | pack variable-length sequences via cu_seqlens, no padding | Padded batch (waste on padding) → packed 1D buffer with cumsum offsets |
| `chunk-reset-cumsum` | cumsum that resets at chunk boundaries | Full cumsum → cumsum reset every chunk_size elements |
| `cumsum-additive-bias` | additive bias via cumulative sum of a gate | O(N²) bias matrix → O(N) cumsum difference |
| `chunk-split-mean-pool` | chunked reduction via split + per-chunk mean | Running sum accumulator → split(C).mean() exposes fixed chunk size |
| `ragged-chunk-pad` | pad ragged tail to block_size | Branch on last-chunk → pad tail, one uniform path |

### Category K: Autotune dispatch (Batch 7, 4 rules — burn-cubecl)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `panic-safe-autotune-candidate` | panic-resilient autotune candidate | Direct call (panics on bad shape) → catch_unwind → Err (autotune skips) |
| `multi-priority-tune-groups` | shape-keyed priority-ranked autotune groups | Flat candidate list → priority-ranked groups (MAX/HIGH/MIN/NEVER) keyed on shape |
| `shape-kind-dispatch` | matmul-kind-keyed kernel family dispatch | Single general GEMM for all shapes → GEMV for matvec, GEMM for general |
| `vector-width-pow2-sweep` | vectorization width as an autotune dimension | Fixed vector width → sweep C ∈ {1,2,4,8,16,32,64} as autotune candidates |

### Category L: Conv-as-GEMM + memory layout (Batch 7, 5 rules — burn-cubecl)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `conv-1x1-as-matmul` | 1×1 convolution as direct matmul (skip im2col) | im2col expansion + GEMM → reshape to 2D + matmul (no column expansion) |
| `contiguity-guard-before-reshape` | stride-contiguity guard skips unnecessary relayout copy | Always-copy-into-contiguous → check stride contiguity, skip copy if already compatible |
| `cube-count-bounded-batch-pack` | grid-limit-aware batch packing | One batch per launch → pack max batches ≤ cube-count limit (find largest divisor) |
| `stride-align-vectorization-key` | stride-byte-alignment determines vectorization width | Fixed vectorization → trailing-zeros of (stride × bytes) picks vector width |
| `anchor-shape-bucketing` | shape bucketing for autotune cache reuse | Exact-shape autotune key → bucketed key (nearest N) so similar shapes share cache |

### Category M: Kernel parametric polymorphism (5 rules — 3 Batch 8 + 2 Batch 10)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `op-family-type-dispatch` | parametric kernel polymorphism over op trait | N copy-pasted kernels (one char differs) → one kernel body + `OpFamily` trait with N associated-type impls |
| `inplace-output-alias` | output buffer reuse via mutability check (skip allocation) | Fresh output alloc every call → `can_mut` check aliases output to disposable input (zero alloc) |
| `layout-permute-for-coalescing` | memory layout permutation for coalesced channel access | NCHW strided channel access → permute to NHWC (channel innermost, coalesced), permute back |
| `reshape-rank-for-backend` | rank adapter — reshape to backend-required rank | Leak rank contract to caller → reshape (1,-1,h), run kernel, reshape back |
| `detached-broadcast-insert` | non-differentiable broadcast via detach + view | Materialize full broadcast → detach() + insert dim (view, no alloc) |

### Category N: dtype + alignment specialization (10 rules — 2 Batch 8 + 4 Batch 9 + 2 Batch 10 + 1 Batch 15 + 1 Batch 16)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `dtype-specialized-float-kind` | dtype-keyed intrinsic dispatch for low-precision floats | Generic powf (absent/imprecise for f16/bf16) → match on `FloatKind`, cast to concrete dtype, call its intrinsic |
| `alignment-gated-zero-copy-view` | alignment-gated zero-copy slice view via handle offset | Always-copy slice → if byte offsets are alignment-aligned, return a VIEW (offset-only metadata, zero copy) |
| `int2-pack-weights` | pack 4 ternary weights per byte | 1 weight/byte → 4 weights/byte (2-bit encoding, 4× bandwidth) |
| `per-token-activation-quant` | per-token symmetric int8 fake quant | Per-element quant → per-token (straight-through gradient) |
| `ternary-weight-quant` | per-tensor ternary {-1,0,+1} fake quant | Per-element quant → per-tensor ternary (BitNet b1.58) |
| `upcast-before-loss` | upcast to f32 before log/exp in loss | fp16 loss (log underflows) → f32 upcast before log/exp |
| `batched-upcast-map` | batched type promotion via map over tuple | Per-tensor upcast calls → `map(upcast, tuple)` (signals independence) |
| `dtype-restricted-backend` | backend restricted to precision-stable dtypes | All-dtype backend → dispatch on dtype (fast path only for stable) |
| `water-fill-tail-dim-shared-codebook` | non-uniform bit allocation + tail-shared codebook | Uniform per-dim codebooks at max_bits → top d_eff dims get per-dim codebooks, tail dims share one b_low-bit codebook (riir-gpu spectralquant/dequant, 2.3× codebook memory reduction) |
| `norm-decoupled-low-bit-scoring` | norm-decoupled low-bit key quantization | Store full-magnitude key, quantize directly (needs 8 bits) → store per-position L2 norm separately as a scalar, quantize the unit-normalized key at 2-4 bits, reconstruct score as norm × dot(unit_q, unit_k_quantized) (riir-gpu forward_turboquant, 4× less key memory) |

### Category O: IR strength reduction (Batch 8, 4 rules — cubecl-opt)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `strength-reduce-mul-pow2` | IR strength reduction: multiply by power of 2 → left shift | `x * 16` (mul unit) → `x << 4` (shift unit, frees throughput-limited mul) |
| `strength-reduce-mul-pow2-plus-minus-one` | IR strength reduction: multiply by (2^k ± 1) → shift ± self | `x * 15` (mul) → `(x << 4) - x` (shift + sub, 2 instructions, no mul unit) |
| `strength-reduce-div-pow2` | IR strength reduction: divide by power of 2 → right shift | `x / 8` (slow div unit, tens of cycles) → `x >> 3` (1 cycle, exact for unsigned) |
| `strength-reduce-mod-pow2` | IR strength reduction: modulo by power of 2 → bitwise and mask | `x % 64` (div/mod unit) → `x & 63` (bitwise-and unit, exact for unsigned) |

## Source

- [`src/domains/kernel_opt/mod.rs`](../../src/domains/kernel_opt/mod.rs) — `KernelRule` + `KernelOptDomain`
- [`src/domains/kernel_opt/corpus.rs`](../../src/domains/kernel_opt/corpus.rs) — `build_kernel_seed_corpus()`
- [`src/domains/kernel_opt/router.rs`](../../src/domains/kernel_opt/router.rs) — `KernelExpertRouter` + `pick_domain`
- [`src/domains/kernel_opt/thermal.rs`](../../src/domains/kernel_opt/thermal.rs) — `KernelThermalRouter` + `ThermalTier`
- [`src/domains/kernel_opt/verify.rs`](../../src/domains/kernel_opt/verify.rs) — `KernelVerifier` (stub)

## See also

- [`heal_domain_trait.md`](heal_domain_trait.md) — the trait this implements
- [Research 002](../../.research/002_popcorn_kernel_opt_distillation_batch1.md) — Batch 1 popcorn distillation methodology
- [Research 003](../../.research/003_popcorn_kernel_opt_distillation_batch2.md) — Batch 2 flash-attention tiling
- [Research 004](../../.research/004_popcorn_kernel_opt_distillation_batch3.md) — Batch 3 linear-attention recurrence
- [Research 005](../../.research/005_popcorn_kernel_opt_distillation_batch4.md) — Batch 4 MoE dispatch + backward pass
- [Research 006](../../.research/006_popcorn_kernel_opt_distillation_batch5.md) — Batch 5 NSA block-sparse selection
- [Research 007](../../.research/007_popcorn_kernel_opt_distillation_batch6.md) — Batch 6 varlen + chunked reductions
- [Research 008](../../.research/008_burn_cubecl_kernel_opt_distillation_batch7.md) — Batch 7 burn-cubecl autotune dispatch + conv-as-GEMM + layout
- [Research 009](../../.research/009_burn_cubecl_cubecl_opt_kernel_opt_distillation_batch8.md) — Batch 8 burn-cubecl kernel polymorphism + dtype/alignment + cubecl-opt IR strength reduction
- [Research 010](../../.research/010_popcorn_kernel_opt_distillation_batch9.md) — Batch 9 popcorn fusion + dtype/quant + dispatch
- [Research 011](../../.research/011_popcorn_kernel_opt_distillation_batch10.md) — Batch 10 popcorn compute + chunked reduction + polymorphism
- [Proposal 012](../../.proposals/) — the absorbed GPU kernel optimizer design
- [Plan 558](../../../katgpt-rs/.plans/) — `pick_domain` substrate (katgpt-core)
- [Research 453](../../../katgpt-rs/.research/) — LatentMoE variable-rank principle
