# Research 050 — riir-gpu hypernet cluster (first half): meta_lora + train_meta_lora — Batch 45

**Date:** 2026-08-15
**Repo:** `riir-clippy` (kernel_opt domain) ← mined from `riir-ai/crates/riir-gpu/src/hypernet/`
**Corpus:** 334 → **337** rules (A 77, B 50)
**Note on numbering:** this note is **050**, not the snapshot-anticipated 049 — `.research/049_Long_Horizon_AI_Research_Evidential_Status.md` was written by a concurrent session before this batch landed (the highwater was stale at 49 while 049 existed; caught by the folder re-scan at write time — the standing rule's second observed save).

## Target

The SHINE hypernetwork meta-LoRA cluster, first half:

- `hypernet/meta_lora.rs` (732 LOC; non-test code L1-500) — the CPU reference bidirectional transformer forward for memory extraction (`forward_extract_layers`, Plan 098/104 T1): context tokens + learnable memory embeddings → frozen LLM with Meta-LoRA injection at every layer → per-layer hidden states at memory positions → `[L, M, H]` memory grid. The Meta-LoRA weights are frozen at inference (trained during pretraining).
- `hypernet/train_meta_lora.rs` (601 LOC; non-test code L1-464) — the CPU finite-difference Meta-LoRA trainer (Plan 104 T3): 2-point central-difference gradients via full forward evals + AdamW; documented as the GOAT-proof validation path (production defers to `GpuDllmTrainer`).

The hypernet dir totals ~3,493 LOC across 8 files; this batch covers the meta-LoRA pair. Remaining for a future batch: `m2p_transformer.rs` (611) + `gpu_m2p.rs` (568) + `shine.rs` (402) + `routing.rs` (268) + `types.rs` (269).

## Method

Two parallel subagents (Lane A forward path, Lane B training path) with coordinator seeds, full-quote re-verification, and dedup greps against the 334-rule corpus BEFORE proposal. Coordinator verdicts on every seed.

## Rules landed (3)

1. **`scratch-depth-matches-shape-invariance`** (A) — a multi-level loop nest (layer × position × head) allocates each scratch at the deepest convenient point even when the buffer's shape is bound by quantities fixed for the whole call: shapes derived only from `(seq_len, dim)` are layer-invariant, position-invariant, and head-invariant, so each scratch belongs at the SHALLOWEST nest whose bounds determine its shape — allocation depth must match shape invariance.
   - Source: `forward_extract_layers` — `lora_buf` + `q`/`k`/`v`/`attn_residuals` per LAYER (L246, L249-252), `attn_out` per POSITION (L322), `scores` per HEAD per POSITION (L330), `mlp_residual`/`mlp_hidden` per position (L388/L395). The `scores` buffer alone is `num_layers × total_seq × n_head` allocations of `total_seq` floats.
   - The tell: two same-shaped residuals at different depths (`attn_residuals` per-layer vs `mlp_residual` per-position) while `hidden`/`grid` already sit at fn scope; `dllm.rs` `forward_bidirectional` runs the same bidirectional prefill on context-owned scratch (the in-tree positive contrast).
   - Dedup: distinct from `caller-owned-split-scratch` (ownership protocol), `superset-sized-scratch-across-variants` (sizing across variants), `scoped-init-at-consume-site` (zeroing scope — the complementary clear discipline), `needless-swap-temp-hoist-to-scratch` (false-premise temps; this is the general placement criterion of which that rule's clause 2 is one instance).

2. **`softmax-denominator-hoist-out-of-weighted-v`** (B) — the softmax-weighted value accumulation divides by the denominator inside the innermost (d × t) nest: `val += scores[t] / sum_exp * v[...]` recomputes the same division `head_dim` times per key position. Hoist `inv_sum = 1.0 / sum_exp` once per (position, head) so the PV loop multiplies, never divides (f32 divide ≈ an order of magnitude costlier than multiply; `sum_exp ≥ 1` by construction since the max subtracts out, so the reciprocal is always finite). The same restructure swaps to t-outer/d-inner making V reads contiguous.
   - Source: `meta_lora.rs` L345-358. The in-tree optimized twin is `riir-engine` `attention.rs` `attention_head` pass 3 (L63-76, incl. its own locality comment) — the file re-implemented it naively one crate away.
   - Dedup: distinct from `fold-norm-scalar-into-matmul` (folds an external scalar INTO a dot — this hoists a redundant division OUT), `hoist-group-invariant-scale-out-of-word-loop` (dequant scale load-hoisting), `online-softmax-streaming` (streaming tiling — the naive div pattern appears there only as ambient ref text). The memory-stride half of the fix is ambient `matvec-coalesce`; the division hoist is the new decision content.

3. **`frozen-cooperand-hoist-out-of-perturbation-loop`** (A) — a finite-difference/perturbation sweep whose eval API takes structured operands rebuilds the FROZEN co-operand per candidate, and the callee re-derives its own sweep-invariants per eval. Materialization cadence must match mutation cadence: build the frozen co-operand + callee-derived invariants ONCE at sweep entry into a prelude (adding a callee seam — a `loss_with_prelude` entry — when the invariant is derived inside the eval), so only the perturbed operand is re-materialized per candidate.
   - Source: the meta-grad loop rebuilds `MemoryEmbeddings { data: mem_flat.clone(), .. }` at L290-294/L310-314 for an operand mutated once per optimizer step (frozenness proven by mutation-site inventory: `mem_flat` mutates only at L328/L346/L365 + L370); the mem-grad loop mirror-images it with `unflatten_meta_lora` of the frozen meta operand (L331-336/L349-354). The callee re-derives: `compute_loss` recomputes the context-embedding `target` per eval (L170-182); `extract_memory_states` re-converts the full `LoraAdapter` set per call (meta_lora.rs L485-490). The mem-loop half is 100% pure waste — hoistable with zero semantic change.
   - Dedup: COMPLEMENTS `make-unmake-over-clone-per-candidate` (that rule fixes the VARYING operand's per-candidate clone via in-place perturb + rollback; this fixes the frozen one — no delta, no rollback, just hoist). Host-materialization twin of `side-branch-operand-residency-matches-mutation-lifetime` (device upload cadence) and `immutable-operand-upload-at-set-creation` (config-lifetime constructor binding).

## Falsified / deduped (highlights)

- **Representation-conversion-clone-per-invocation** (adapter conversion clones A/B per extract call) — DUPLICATE of the `immutable-operand-upload-at-set-creation` family (decision content identical: a frozen operand's converted representation is derived once, never per call; only the mechanism differs — device upload vs heap clone, and mechanism is not decision content). The thin stateless-fn nuance (the conversion site has no constructor to hold a cache — the fix is an API-shape change) recorded as an instance note, not a rule.
- **FD unflatten-per-perturbation** — INSTANCE of `make-unmake-over-clone-per-candidate` (the FD loop already performs make/unmake on the flat vector: `saved` capture + restore; the residual cost is the structured re-materialization boundary).
- **Loop-order swap alone** — ambient `matvec-coalesce`. **Per-position GEMV structure** — `op-major-backward-loop-inversion-batch-sequence-axis` + deliberate CPU-reference shape. **Online softmax** — ambient. **`1/sqrt(hd)` fold** — `constant-scale-folded` family, thin. **`attn_out[..] += val` on fresh zero** — folded into rule 1's opt_pattern comment (the enabler of clear-free hoisting). **`kv_group` recompute** — compiler strength-reduces.
- **AdamW `powi`** — already hoisted outside the per-param loop (L131-132). **FD O(P) structure as a rule** — documented trade-off (header L7-8 defers production to `GpuDllmTrainer`). **Per-step `meta_grads`/`mem_grads` vecs** — rust_perf `reuse-allocation-clear` domain instance. **B3 target-recompute standalone** — folded into rule 3's callee-seam half (the cheap component alone would be thin: O(ctx×n) ≈ 128 float-adds vs a full L-layer forward per eval).

Total: 3 rules, 9+ falsifications/dedups — consistent with the predicted high dedup pressure after 44 batches.

## Bug yield — riir-ai Issue 699 (filed)

**Lane A (meta_lora.rs):**
- **H1 (HIGH, conditional)**: `lora_buf` sized `model_config.lora_rank.max(1)` (L246) but `lora_apply` uses the buffer at the ADAPTER rank (`= meta_lora.rank = ShineConfig.meta_lora_rank`, default 128) and writes via `get_unchecked_mut` — **silent heap corruption** in release AND debug when `model_config.lora_rank < meta_lora_rank`. Real-model configs set `lora_rank: 0` → `.max(1)` = a 1-float buffer vs a 128-rank adapter → 127 floats OOB per `lora_apply`, × 6 sites × positions × layers. Tests pass by config coincidence (`Config::micro()` lora_rank = 4 == `small_shine_config` meta_lora_rank = 4; nothing asserts equality). Exposed via `ShineHypernet::with_weights` (the documented production path) + `route_context_to_expert`, both taking an external `model_config`. `model_config_from_shine` masks it by setting `lora_rank: shine.meta_lora_rank` — itself a conflation smell.
- **H2 (HIGH, conditional)**: `meta_layer_to_lora_adapter` sets `out_dim: hidden_dim` (L171) but the K/V calls pass `kvd`-sized slices (L293, L310) — under GQA (`n_kv_head < n_head` → `kvd < n`) `lora_apply` writes past the slice end via `get_unchecked_mut`. Masked in-tree (`micro()` has `n_kv_head == n_head`; `model_config_from_shine` forces equality). Related engine-inherited quirk: the same square adapter applied to `mlp1`/`mlp2` gives partial/truncated MLP LoRA coverage silently.
- **H3 (MEDIUM)**: `apply_meta_lora` is dead in production (no non-test callers; re-exported as public API) AND diverges from the forward path three ways: doc says `alpha/rank` but code applies `1/rank` (L149) while the forward path scales `alpha/rank = 16/rank` (L169); AND the two paths interpret the **same A/B bytes transposed** (`apply_meta_lora` reads `a[h*rank+r]`/`b[r*hidden+h]` per the `MetaLoRALayer` docs; `LoraAdapter`/`lora_apply` reads `[rank × in_dim]`/`[out_dim × rank]`; the adapter conversion clones the buffers verbatim). Tests are self-consistent per-path, so invisible to CI.
- **H4 (MEDIUM-LOW)**: vocab bounds checked only by `debug_assert` (L222-225) before an unchecked `wte` gather (L228) — release-mode OOB read; inconsistent with the release-checked `assert!` at L206-210 in the same fn.
- **H5 (LOW)**: stale cross-file comment "matches forward_base L2224-2226" (L260) — the pattern is real but lives at `transformer/mod.rs` L685-688 (doc rot only; the double-norm compat claim itself verified TRUE).

**Lane B (train_meta_lora.rs):**
- **H1 (MEDIUM, quantified)**: FD clone/materialization storm — 640 evals/step × (unflatten P + adapter conversion P + mem clone M floats) ≈ **368,640 floats ≈ 1.4 MB avoidable memcpy/step at test scale** (≈721 MB over a 500-step run); the mem-loop half is 100% pure waste. At documented scale (rank 128, hidden 1024, L=24 → P≈6.29M) the avoidable churn ≈ **~636 GB/step** — though at that scale the FD method itself is the blocker (the header's honest deferral to `GpuDllmTrainer`).
- **H2 (MEDIUM, currently benign)**: `compute_baseline_loss` hardcodes ctx_len 8 (L453) while training uses `train_config.context_length` — GOAT baseline measures a different distribution when `context_length != 8` (default is 8, so the shipped gate is unaffected).
- **H3 (LOW-MEDIUM)**: GOAT "trained < 50% of baseline" compares a single-sample `final_loss` (fresh random context, L412-424) against an n-sampled averaged baseline — statistically weak near threshold.
- **H4 (LOW)**: `generate_batch` claims "Map normal distribution to uniform token index" (L214) but `|N(0,1)|·V mod V` is a wrapped half-normal (max/min density ≈1.65×, low ids favored). Train/baseline/final all share the generator so no relative bias — intent mismatch only.
- **H5 (LOW, DX)**: no upfront `shine_config.hidden_dim == model_config.n_embd` check in `train_meta_lora` — misconfig panics deep inside the first FD eval (`extract_memory_states` assert) instead of fail-fast.
- **H7 (LOW, latent)**: the structured `meta_lora` is value-stale after L252 but retained for shape metadata through the loop (a future mid-training read would silently get pre-training weights).

## GOAT

- **G1** (parse): `batch45_pseudocode_parses` — all 3 rules' ref_shape + opt_pattern syn-parse (L2 pruner verdict). 3/3 PASS by name.
- **G2** (tests): 351 kernel_heal + 201 default lib tests, 0 failed (351 = 348 + the 3 new gate tests, exactly as predicted).
- **G3** (no-regression): clippy 0 warnings in both feature states (default + `kernel_heal`). `--all-features` remains broken by SIBLING WIP (`rust_perf/bench.rs:335 alloc_axis`) — pre-existing, not this batch's.
- 10 `doc_markdown` warnings fixed pre-commit (the standing gotcha family: bare `seq_len`/`sum_exp`/`head_dim`/`attention_head`/`lora_buf`/`lora_rank`/`get_unchecked_mut`/`kvd`/`out_dim`/`apply_meta_lora` in `//!` prose need backticks — the sixth occurrence of this class).

## Landing notes (generator discipline)

- One-shot Python generator with 20 anchors (16 corpus.rs + 4 router.rs), dry-run verified all unique before apply. First compile clean — the streak continues.
- Count-pin sites updated: entries assert (337), KERNEL_CATEGORIES A/B rows, per-category count comments (77/50), expected-counts table, exhaustiveness doc + test comments, sum + grouped asserts, router.rs doc header ×2 + test comment + assert, per-batch comment line, header `//!` batch block.
- The Research note number is **050** (049 was dual-allocated by a concurrent session — `.research/.highwater` was stale; folder re-scan at write time caught it, per the standing rule. Highwater fixed to 50).

## Next targets (mining queue update)

- `hypernet/` second half: `m2p_transformer.rs` (611) + `gpu_m2p.rs` (568) — the memory-to-parameter transformer pair; plus `shine.rs` (402) + `routing.rs` (268) + `types.rs` (269) if capacity remains.
- `maxsim/` (spectralquant.rs 581 + dispatch.rs 449 — distinct from the mined `spectralquant/` dir).
- `kimi_k3_gpu_backward_batched.rs` (1,394 — LOW value: shares Issue 693's H1/H2 + the broken LCG with the module file).
- STILL BLOCKED: `ternary_deltanet_gpu_forward.rs` + `_cudarc.rs` (sibling WIP).
