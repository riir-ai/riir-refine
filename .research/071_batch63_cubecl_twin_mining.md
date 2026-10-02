# Research 071: Batch 63 — the CubeCL twin (`ternary_deltanet_gpu_forward.rs`) two-lane mining record

**Status:** RECORD — Batch 63 landed 2026-08-19 (corpus 364 → 367; 3 rules, all coordinator-verified distinct).
**Source:** `crates/riir-gpu/src/ternary_deltanet_gpu_forward.rs` @ origin/develop `42cc13e0d` (5,432 lines, committed-clean; extracted byte-exact to `target/b63/cubecl_twin.rs`). The 721-closeout unblock condition fired 2026-08-19 (queue snapshot `3094cc4`).
**Method:** Plan 009 (plan-first), two parallel mining lanes (spawn agents), coordinator verification of every surviving candidate's distinctness claim against the actual corpus rule texts.

## Lane verdicts

**Lane B (kernel/math): 0 candidates — structurally empty, honestly.** The file is the host-side orchestration twin: exactly ONE inline kernel (`dequant_wte_row_f32`, a port of the B22-mined bit-plane layout); the chunked-recurrence kernels live in `deltanet_chunked_cubecl.rs` (mined B28), the tree-verify kernels in `ternary_tree_verify_driver`, the decode-recurrence kernels in `deltanet_cubecl.rs`/`qwen_attention_cubecl.rs` (B28/B23). Zero host-side numerical math (verified by grep — no exp/sigmoid/sqrt/tanh/powf on host). 24 candidate mechanisms examined → all duplicates (mapped in the lane report).

**Lane A (dispatch/orchestration): 3 surviving (2 A-grade, 1 B-grade), 11 dedup drops, 7 C-drops.** The file is ~40% Issue 640/642 diagnostic apparatus (atomic stage/sub-stage masks, A/B knobs) — measurement methodology, correctly killed per the B36-F5 precedent (an env-A/B-gate rule was falsified on exactly these grounds).

## The 3 rules (coordinator-verified)

1. **`same-input-gemv-weight-concat-at-upload`** (D 70→71; Issues 642 F3/F2 + 648 F9). N same-input GEMVs merged by baking the weight CONCAT into the upload (`from_weights_concat`/`from_two_weights`) — the STOCK kernel dispatches it. The substrate-constrained third branch of the same-input-merge family: B58's `same-input-gemv-segments-single-launch` needs a multi-segment launcher API (the CUDA twin's route — the twins CONVERGED on the same problem from opposite constraints); B15's `fuse-gate-up-gemv-activation` needs a bespoke kernel; concat needs only frozen weights. Offset-consumers (K@0/V@kvd, gate@0/up@mlp) pay zero extra dispatches; separate-buffer consumers pay ONE split kernel (4 GEMVs → 2). Constraint: re-concat on weight change (hot-swap substrates keep separate handles).
2. **`deferred-residual-fused-into-next-layer-norm`** (C 33→34; Issue 645 T3). The post-FFN residual add is NOT dispatched at its layer's end — deferred across the loop boundary into the NEXT iteration's already-fused add+norm kernel; first layer (no predecessor) + tail (no successor) special cases; −63 dispatches/token at n_layer=64 with ZERO new kernels. The composition ladder is in-source: producer-side fusion (B19's route, the down-GEMV writing `self.x`) beats the deferral when available and disables it. New axis: op-deferral across loop iterations to reach a fusion site (the scheduling analog of `defer-readback-until-after-submit`'s timing deferral). Distinct from B9 `fuse-add-rms-norm` (kernel fusion at one site — this needs no new kernel) and from B19 (producer-side in-place).
3. **`reset-state-in-place-fill-never-realloc`** (A 86→87; Plan 528 T0.2). Sequence reset via in-place zero-fill kernels over process-lifetime buffer identity — the prior shape (128 `create_from_slice` per reset) fragmented the dynamic pool to SIGKILL under the Go arena. Cursor-bounded caches (KV read only `[0..=pos]`) skip even the fill — `pos = 0` subsumes the content reset. The pool-churn class recurs 3× in this file alone.

## Lane convergence (cross-validation)

Both lanes independently dropped the same mechanisms as B58 duplicates (rotation pool, GPU checkpoint/rollback, batched reads, tok_slice, conv1d double-buffer, dead-slot carry, GEMV-residual, add+rmsnorm fusion) — the two-lane method's built-in reproducibility. The ONE disagreement — Lane B counted the weight-concat family as covered by B58's segments rule, Lane A proposed it as new — was adjudicated by the coordinator against the actual rule texts: B58's fix is a launch-time segment table over a multi-segment launcher API; the concat is an upload-time weight-layout transform using the stock kernel. Different substrate requirements, different consumer shapes → genuinely distinct branches of one decision family. Both twins solving the same problem under opposite constraints (CUDA has the segment launcher; CubeCL doesn't) is itself the evidence that the family needs both branches encoded.

## Hazard yield (filed as riir-ai issue, worktree dance)

1. **Dead `attn_wk`/`attn_wv` device-resident weight upload** (L266-273 + L5280-5289, `#[allow(dead_code)]`; the comment claims "upload source for attn_wkv" — false: the CPU weights are the source at L1527-1540; no debug tap reads them — the tap reads `attn_kv` at L2945-2950). The B56 `superseded-format-constructor-live-fields-only` class, path-superseded.
2. **Dual residency of input-proj weights**: decode dispatches `in_proj_concat` only, prefill dispatches the four separate handles (L4188-4195) — both sets uploaded + permanently live (~2× input-proj bytes); audit gate/up vs gate_up too.
3. **3× Metal weight residency** when `metal_tensor_gemm` on: all three path caches (metal-rs L1470-1545, wgpu L1551-1569, raw-Metal L1577-1595) eagerly initialized; only one dispatches.
4. **`reset_state` doc-lie** (L4719-4728 vs L4764-4767): doc says it zeros KV caches "the safe, defensive choice"; the implementation deliberately does not (bounded reads — the code is right).
5. **`ResidualAdd`-as-copy doc-lie** (L4664-4679): "there is no dedicated device-to-device copy kernel in this crate" — `CopyCubeCL` is imported (L59) and used (L5142+); the prefill tail could be 1 copy instead of FillZeros + ResidualAdd (2 dispatches + a needless zero-fill).
6. **B58 rotation pool never back-ported** (L4940-4943): `forward_speculative_verify` allocates K fresh vocab-sized logits buffers per call — the exact pool-churn class `reset_state`'s own comment warns caused SIGKILL; the cudarc twin pre-allocates.
7. **CPU rollback write asymmetry** (L5056-5085): 96 batched reads but 96 separate per-layer writes.
8. **Training forward: 5 unbatched `read_one` per layer** (L1911-1930) — the anti-shape this file fixes elsewhere.
9. **Suspected aliasing UB in the SHIPPING prefill residual** (L663-681): `PREFILL_PINGPONG_RESIDUAL` fix ships default-OFF; the file itself documents "aliasing UB in WGSL/Metal, and `launch_unchecked` does not validate it". Interacts with B19's in-place advocacy — flagged for the owner.
10. Dead placeholder `conv_dim` (L5242-5243) + stale 30-line design-monologue (L3246-3276) + superseded single-token-slicing doc (L3634-3636 vs the batched variants at L4300-4324) + dead reserved buffers `residual`/`hidden_copy` (L1093-1101).

## Process lessons

- **The 18th count-pin site**: `router.rs::rules_for_expert_total_matches_corpus` ("experts cover all 364") — the router's expert coverage derives from KERNEL_CATEGORIES, so the pin broke the moment the table grew. Found by the full kernel_opt suite (not the targeted run).
- **The B51 case-sensitivity lesson struck a 5th time**: concepts-covered needle "next loop iteration" vs the concept's "NEXT loop iteration".
- **The B37 line-initial `+` lesson did NOT recur** (the generator discipline held this time — all doc text was generator-written except the header line, which had 3 backtick misses instead: bare `CubeCL` ×2 + `create_from_slice`).
- **Template placeholder discipline**: my generator's `ref_shape: CODE1_REF,` template lacked `.to_string()` — 6 E0308s; the fix-by-line-scan caught exactly the 6 (existing entries already carried it).

## GOAT

G1: all 3 rules' ref_shape/opt_pattern syn-parse (batch63_pseudocode_parses) + concepts carry the verified quotes. G3: kernel_opt 811/0/1i + default 607/0 + retrieval floors HELD + clippy 0 all-targets (both feature states). G4: no new deps. The kernel domain has no timing benches (standing discipline — corpus patterns, not Pareto candidates).
