# Research 046 — riir-gpu gemma2_d2f Batch 42: D2F block-causal decode + Self-Conditioning

**Date:** 2026-08-15
**Batch:** 42 (corpus 316 → 320; A 70, B 46, D 56)
**Targets:** `crates/riir-gpu/src/gemma2_d2f/mod.rs` (1,579 LOC — Gemma 2 D2F
block-causal decode, CubeCL GPU path, Plan 108) + `crates/riir-gpu/src/gemma2_d2f_sc.rs`
(800 LOC — Self-Conditioning support, Plan 250 T1-T4).
**Bug yield:** riir-ai Issue 695 (25 hazards: 2 CRITICAL, 4 HIGH, 3 MAJOR, 16 MINOR).
**Status:** the gemma2_d2f cluster is now FULLY MINED.

## What the cluster is

`GpuGemmaCubeCLD2F` is a **CPU-orchestrated hybrid** D2F (Diffusion-to-Forecast)
denoising decode for Gemma 2: GPU GEMV/attention dispatches sandwiched between CPU
elementwise (RMSNorm, RoPE, GeGLU, residuals), every dispatch result read back to CPU,
every input re-uploaded per call, KV in a CPU `CpuKVCache`. Block-causal attention:
bidirectional within the generation block, causal across blocks — two-phase per layer
(Phase A fills K/V for all positions, Phase B runs attention+MLP per position). The SC
file adds the arXiv:2604.02028 self-conditioning mechanism: the previous step's x̂₀
prediction concatenated with x_t and projected back through W_sc (identity-padded init).

The in-tree AR decode path (`gemma2_cubecl/gpu_forward.rs::forward_layer_gpu`) is
already device-resident (`GpuKVCache` + `KvStoreCubeCL` + GPU RoPE + fused
norm-residual). d2f cloned the **legacy CPU-hybrid** dispatch shape instead of
consuming that substrate — the defining architectural fact of the cluster.

## The 4 rules

### 1. `refine-loop-invariant-prefix-elision` (D — Dispatch)

In an iterative refinement loop whose mutation set is a proper subset S (only the
generation block's tokens are rewritten), the complement's forward artifacts are
PROVABLY invariant when the attention mask closes it (block-causal `t_n`: prompt
positions attend only to `0..prompt_len`) — yet each step resets the KV cache and
re-forwards the ENTIRE sequence (the design doc admits it at mod.rs L499-502). The
rule: forward the prefix ONCE, persist its KV + hidden state, run each iteration over
S only. `O(steps × seq)` → `O(seq + steps × |S|)` in FLOPs, dispatches, and bus
traffic. Precondition: the refine set must be closed — SC conditioning that feeds
prompt embeddings breaks the invariance (restrict SC to S).

- Distinct from `append-cache-rewind-overwrite-in-place` (B39: extent management,
  still recomputes — this ELIDES the recompute).
- The per-position consumer-set sibling of `lm-head-loss-positions-only` (B37): same
  dispatch-elision class, cross-iteration producer-invariance complement.
- Generalizes `training-forward-shed-decode-loop-shape` across LOOP ITERATIONS
  (B36 batches the position axis within ONE forward).

### 2. `softmax-entropy-via-shift-identity` (B — Compute)

`ln(e_t) = logit_t − max` EXACTLY (the shift is exp's argument, so its log is the
shift itself) — an entropy-style Σ p·ln(p) over a softmax needs ONE transcendental
(`ln(S)`), not N. The file's `SamplerFeatures::from_logits` is the POSITIVE instance:
one fused sweep computes `sum_exp`, `sum_e_ln_e` via `e * logit_shifted`, and a top-3
cascade, all off one `e_t = exp(shift)` per token; entropy recovered as
`ln(S) − (1/S)·Σ e·ln(e)`. NaN guard on the underflow tail (0·−inf).

- Distinct from `fuse-xent-logsumexp`/`fuse-jsd-logsumexp` (pass-count fusion — this
  is per-element transcendental COUNT via algebraic identity) and `exp2-log2-base2`
  (base conversion — this ELIMINATES the ln calls).

### 3. `runtime-identity-gate-cached-verdict-elides-side-projection` (D — Dispatch)

A learnable side component with algebraically-neutral init (W_sc = [I|0]) ships a
runtime content-classification: the still-neutral verdict computed ONCE per WRITE (at
the sole setter boundary — re-derived from CONTENTS so externally-supplied buffers
are re-classified honestly), cached as a bool, and BOTH the side-input PRODUCTION and
the projection CONSUMPTION branch on it to ELIDE work. The hot loop never re-scans;
the O(n²) classification amortizes to one pass per write. Turns
"enabled-but-untrained = free" from a construction-time hope into a runtime invariant
that survives arbitrary buffer writes.

- The completeness requirement is load-bearing: the file's own hazard — `update_from_logits`
  gated only by `sc_enabled` while its only consumer is identity-gated OFF — computes
  2.36 GB/position/step of host work the loop provably discards (Issue 695 H16).
- Distinct from `zero-init-side-branch-frozen-base-path` (B30: VALUE contract — the
  inert math still RUNS) and `min-rank-inert-adapter-zero-alpha` (B31: ALLOCATION
  contract — scale-annihilated, no gate). This is the ELISION contract.
- Composes with `derive-shape-from-payload-not-cached-field` (write-through cache).

### 4. `expected-embedding-transposed-gemv-resident-head` (A — MemoryAccess)

The soft-token expectation `x̂ = Σ_v softmax_v · wte[v,:]` factors as the TRANSPOSED
GEMV of the tied lm_head the forward just dispatched (`logits = wte @ h ⇒ x̂ = wteᵀ @ p`).
One transposed-GEMV dispatch against the ALREADY-RESIDENT table (V floats up, n floats
back) replaces the entire host-mirror walk (2.36 GB + 590M MAC per position per step
at Gemma-2-2B). The vocabulary translation is the load-bearing insight:
"expected embedding over softmax" reads as a gather idiom but factors as a GEMV
against a weight the device already holds.

- Distinct from `offload-cpu-bottleneck-to-gpu-kernel` (B14: data-prep transforms, no
  resident-operand reuse, no orientation flip), `device-side-argmax-minimal-download`
  (B25: shrinking a device-array download — this DELETES a host mirror walk), and
  `transposed-consumption-accumulate-not-gather` (B40: host loop ORDER — this moves
  the op across the bus).

## Falsifications (18 total — the file is a museum of already-mined anti-patterns)

**Lane A (mod.rs, 13):**
1. Per-position Phase A/B over frozen operands → `training-forward-shed-decode-loop-shape`
   + `fold-query-tokens-for-occupancy` (the two-phase split already did the enabling
   refactor without harvesting the batching).
2. `get_combined_kv` fresh Vec + re-upload per Phase B position →
   `kv-window-device-resident-ring-append` (1.7 GB/block decode; the device-resident
   substrate exists in-tree, unused).
3. Dead prompt lm_head → `lm-head-loss-positions-only` (151 GFLOPs + 128 MB/block).
4. Full-vocab logits drain per position → `device-side-argmax-minimal-download` (256 MB/block).
5. CPU elementwise between GPU dispatches → `hybrid-cpu-elementwise-gpu-throughput-sync-points`.
6. W_sc identity-init skip → B30/B31 instances (the INIT; the runtime GATE mechanism is rule 3).
7. `from_logits` single-pass top-3/entropy/sum fanout → `one-sweep-spatial-feature-fanout`.
8. 15-way format triplication (5 families × 3 formats) → `op-family-type-dispatch`.
9. `read_handle` value-return + double copy → `readback-destination-passing-free-fn`.
10. Hardcoded Gemma-2-2B kernel constants → `hardcoded-fast-path-jit-specialization`
    (the UNASSERTED softcap/scale fields are the bug — Issue 695 H6).
11. `DiffusionSampler` learned accept gate → NOT proposed (ML design outside the 15
    categories; the early-exit `all unmasked → break` is the
    `predicate-gated-skip-empty-workset-step` family).
12. Per-launch 12-byte params rebuild → `tiny-param-buffer-rebuild-over-cache` (NOT a hazard).
13. "3 launches, 1 sync" comments → NOT doc-lies (consistent with
    `first-readback-pays-queue-drain` accounting).

**Lane B (sc.rs + SC integration, 5):**
1. `project_sc`/`_into` destination-passing pair → `readback-destination-passing-free-fn`
   instance (production uses `_into`; thin on CPU side).
2. Batched GEMM over the block for SC projection → the B34/B36 seq-batching family.
3. Flat vs nested estimate state → ~70% `flat-activation-slots` + generic reuse — kept as hazard.
4. Two-pass exp recompute in `compute_x0_estimate` → generic softmax-pass lore; the real
   fix is rule 4 (move the whole op).
5. `sc_scratch` + `mem::swap` ping-pong → `caller-owned-split-scratch` mechanics.

Plus 4 Lane-B self-falsifications during verification (the identity check DOES skip;
`is_identity_padded` is NOT in the hot loop; mask exclusion is BEFORE softmax —
correct restricted softmax; temperature application consistent).

## Bug yield — riir-ai Issue 695 (25 hazards)

**CRITICAL:**
- **H1:** combined KV rebuilt + re-uploaded per Phase B position — **1.7 GB/block
  decode** (80% of all uploads) for a 256 KB operand frozen after Phase A; GPU-born
  K/V round-trip through the CPU cache while the device-resident substrate sits unused.
- **H2:** invariant prompt prefix re-forwarded every denoise step — **~227 GMACs**
  wasted at defaults (7/8 of prompt work; the rule-1 fix).

**HIGH:** H3 per-position Phase B (46,592 blocking `read_one`/block decode), H4 dead
prompt lm_head (151 GFLOPs + 128 MB), H5 full-vocab drain (256 MB/block), H6 kernel
hardcodes softcap=50.0/scale=0.0625 with host fields dead — silent divergence for any
non-Gemma2-2B config, no CPU-parity test exists.

**MAJOR:** H7 `tokens.truncate(config.block_size)` truncates the wrong end (reachable
panic + silent block shrink), H8 `read_handle` double alloc ×46.6K reads, H16 the
ungated SC producer (2.36 GB/position/step never read; the G2a zero-overhead gate
measures only the consumer side), H17 the trained-SC path unreachable in-tree
(`ScTrainingForwardResult` never constructed; `set_w_sc` zero production callers;
riir-train's SC is a separate synthetic impl — enabling SC buys exactly H16's overhead
and nothing else), H18 trained-W_sc projection CPU-only (42.5 MB/position stream; no
device handle).

**MINOR:** nested-Vec activations (H9), 15-way dispatch DRY (H10), lib.rs doc-lie
(H11), alloc-claim overclaim (H12), doc-lies (H19), softmax max over wrong support
incl. mask token — silent zero estimate under underflow (H20), hot-loop bounds check
(H21), dead `detach_x0` config (H22), fresh nested estimate per step (H23), silent
shape-mismatch fallback (H24), 42.5 MB dead W_sc buffer (H25), dead defensive
memsets/fields (H14), two O(vocab) passes per masked position (H15), SC prompt-position
estimates (H13).

## Positive instances (done right)

- The entropy-shift identity (rule 2's source) — the file's best moment.
- The identity-gate design (rule 3's source): cached verdict, write-through
  re-derivation at `set_w_sc`, O(n²) scan off the hot loop.
- QKV/gate-up fan-out with shared input handles (`fan-out-shared-input-single-sync`).
- `wte_cpu` host cache for embedding gathers; `probs_scratch` reuse.

## Method notes

- **The 2-lane split worked well for a coupled pair of files:** Lane A (decode
  structure) + Lane B (SC substrate + integration) with overlapping quote
  responsibilities resolved cleanly — the only overlap (identity-skip as B30/B31
  instance vs rule 3) was resolved by scoping: the INIT is the instance, the runtime
  GATE mechanism is the rule.
- **Coordinator re-verification passed 4/4 rules** — all quoted lines exact (one
  anchor needed re-reading: L525-529 spans the comment + `CpuKVCache::new` call).
- **Generator lesson (new, third anchor-class):** the entries-vec tail anchor mixes
  literal 2-char `\n` escapes (INSIDE the Rust string literal) with REAL file
  newlines (after `.to_string(),`) — a uniform treatment of either kind returns
  count=0. The dry-run caught it before any write (2 aborted attempts, zero pollution).
- **Doc-markdown gotcha variant:** `ln(e_t)` / `logit_t` / `lm_head's` in `//!` header
  prose need backticks (3 fixes pre-commit — the underscore-identifier family again).

## GOAT

- **G1:** 342 kernel_opt lib tests PASS (339 + 3 new: `batch42_rules_present`,
  `batch42_concepts_covered`, `batch42_pseudocode_parses`).
- **G2:** 201 default lib tests PASS.
- **G3:** clippy 0 warnings in all 4 states for my files (default, kernel_opt,
  rust_perf, latent_retrieval — the 1 latent_retrieval warning is the sibling's
  pre-existing WIP `heal.rs:415`, untouched).
- All count-pin sites in sync at 320 (A 70, B 46, D 56; B 45, C 32, E 18, F 10, G 19,
  H 14, I 4, J 5, K 5, L 10, M 9, N 18, O 4), enforced by passing tests.

## Next mining targets

- `weaver_gpu*` (dflash 576 + corrector 1,449; the `weaver_gpu/` dir mod.rs ~1,700 +
  tests 57K remain separate), `kvarn/` (~1,838), `hypernet/` (~3,300), `maxsim/`
  (~1,049).
- `kimi_k3_gpu_backward_batched.rs` (1,394) still LOW value (shares Issue 693's
  H1/H2 + the broken LCG).
- `ternary_deltanet_gpu_forward.rs` + `_cudarc.rs` still BLOCKED on sibling WIP
  (unchanged).
- Progress: riir-gpu ~87 of ~200 files mined. Corpus: 320 rules.
