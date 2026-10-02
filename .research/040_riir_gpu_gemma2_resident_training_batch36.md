# Research 040 — riir-gpu Gemma-2 RESIDENT Training Cluster (Batch 36)

**Date:** 2026-08-15
**Source:** `riir-ai/crates/riir-gpu/src/gemma2_cubecl_train/resident.rs` (1,639 LOC)
**Method:** Protocol A — 2 parallel mining subagents (forward L1-1081 / backward L1083-1639)
+ coordinator re-verification of every load-bearing quote + cross-file claims.
**Result:** **7 rules, corpus 275 → 282** (C 29→30, D 43→48, H 12→13).
Bug yield filed as riir-ai Issue 689 (15 hazards).
`gemma2_cubecl_train/` is now FULLY MINED (Batch 35: mod.rs + attention_gpu.rs;
Batch 36: resident.rs).

## 1. What the file is

The Plan 481/482 all-GPU LoRA training path — the third file of the Gemma-2
training cluster that Batch 35 deferred. Three public paths live here:

1. `training_forward_gpu` — the per-position GEMV forward (GPU-resident
   dispatches, activation handle saving, the Plan 480 fix for Issue 004).
2. `training_forward_batched` — the Plan 482 whole-sequence GEMM forward
   (~55K per-position dispatches → ~182 batched; CPU attention under Issue 430
   Option C, fused GPU attention under Option B gated by `PLAN423_GPU_ATTENTION`).
3. `backward_lora_grads_batched_input_with_aux` — the Plan 482 T9 batched
   backward (~35K per-position GEMV dispatches → ~183 batched GEMMs;
   the Issue 424 Plane-kernel accuracy fix rides `dispatch_gemm_backward_gpu`).

## 2. The 7 rules

| # | Rule | Cat | One-liner |
|---|------|-----|-----------|
| 1 | `constant-scale-folded-into-upload-copy` | C | A hyperparameter-derived CONSTANT scale (LoRA alpha/rank) is folded into the byte-level upload COPY so no kernel ever applies it; the backward keeps the unscaled original — fold-into-copy, never into the source operand. |
| 2 | `side-branch-operand-residency-matches-mutation-lifetime` | D | An aux computation riding a GPU hot path keeps ITS OWN operands device-resident, uploaded once per STEP at step entry (the middle rung between config-lifetime and call-local); the hybrid CPU-LoRA path paid ~70K sync round-trips and ran 4.8× slower than pure CPU. |
| 3 | `activation-checkpoint-as-handle-clone-deferred-batch-readback` | D | The checkpoint medium during compute is the device-handle clone (refcount bump, cannot stall), never an inline read; the tail `read_batched` collapses 14 handles/layer into ONE client.read — 364 syncs → 26. |
| 4 | `training-forward-shed-decode-loop-shape` | D | The per-position loop + incremental KV cache is inherited DECODE structure; free the position axis by making the coupling op (attention) whole-sequence — ~55K dispatches → ~182. |
| 5 | `sibling-dispatches-before-any-readback` | D | Queue ALL independent sibling dispatches BEFORE the FIRST readback; the dispatch-read interleave serializes completely. Applies with DIFFERENT input buffers (q/k/v) — fan-out's shared-input premise doesn't apply. |
| 6 | `independent-cpu-work-fills-dispatch-read-window` | D | CPU work independent of the readback (the LoRA param-grad half) belongs INSIDE the dispatch→read window — wall time becomes max(GPU, CPU) + join. Visible-but-unexploited in source (hazard-class rule). |
| 7 | `grad-terms-merge-before-shared-link-jacobian` | H | Multiple loss terms through the SAME nonlinear link (CE + aux through softcap) must SUM at the link INPUT with ONE Jacobian application; adding a term after the Jacobian is silently WRONG, not just slower. |

## 3. Seed verdicts (falsification 7-for-9 across the last 3 batches)

Four coordinator seeds FALSIFIED by subagent verification (the highest count
yet — the dedup-pressure prediction for this file materialized):

- **F4 `position-table-precompute-once-per-sequence` — FALSIFIED (premise
  wrong + already ruled).** The per-position path does NOT recompute the
  cos/sin table per (layer, Q/K): `dispatch_rope_gpu` (dispatch.rs L1063-1066)
  uses `RopeCosSinCache::get_or_compute`, whose doc says "Recomputes the table
  and re-uploads only when `pos` changes; otherwise clones the cached handle"
  (rope_geglu_cubecl.rs L433-437). And Batch 17's header (corpus.rs L112-114)
  explicitly dropped `rope_geglu_cubecl`: "pairing-generic + precomputed
  cos/sin table are composition of existing dispatch/precompute principles."
- **F5 `env-var-ab-gate-runtime-kernel-promotion` — FALSIFIED as
  corpus-worthy.** The env vars exist (`PLAN423_GPU_ATTENTION` L834,
  `GPU_BATCHED_FORWARD` L1128-1129) but the KERNEL content of that A/B is
  already `kernelize-host-chain-after-sync-batching-floor` (B35 — its
  ref_shape quotes this exact file's Issue 430 paths). The gate itself is
  GOAT-gate process, not a kernel pattern.
- **B2 `ping-pong-swap-loop-carried-gradient-buffers` — FALSIFIED (dedup).**
  `needless-swap-temp-hoist-to-scratch` fix(2) already encodes it: "when
  aliasing is genuine, the swap temp is loop-carried state and belongs in the
  persistent scratch struct" + its opt shows the hoisted `swap_tmp`.
  resident.rs L1597 is the CORRECT production instance of that codified
  pattern, not new wisdom.
- **B4 `scratch-preallocation-outside-layer-loop` — FALSIFIED (dedup, as
  predicted).** The L1281-1302 hoist decomposes exactly into
  `superset-sized-scratch-across-variants` (lora_scratch sized to max in_dim,
  prefix-consumption verified in engine backward.rs L293) +
  `scoped-init-at-consume-site` (L1473-1478, L1233) + `needless-swap-temp`
  fix(2) (the hoisted loop-carried buffers).

Two premise CORRECTIONS that strengthened their rules:

- **F3 verified + overstatement confirmed:** the per-position path's
  "This is the ONLY sync point in the entire forward" (L461) and "1 sync point
  instead of ~20,000" (L332-333) are contradicted by the file's OWN cost model
  at L959-961 ("14 separate read_handle calls = 14 blocking GPU↔CPU syncs per
  layer"); the per-position readback loop issues 14 × 26 × 256 + 2×256 =
  **93,440 individual blocking reads**. Only the batched path delivers the
  claim. Rule 3 landed WITH the honest caveat; the doc-lie went to Issue 689.
- **B1 premise corrected:** q/k/v backward GEMMs consume three DIFFERENT
  buffers (g_q_all/g_k_all/g_v_all, L1517-1522), so
  `fan-out-shared-input-single-sync`'s premise ("create the input handle
  ONCE") genuinely does not cover this case — only the ordering discipline
  applies, and `first-readback-pays-queue-drain`'s naive form is "defensive
  syncs between reads," not the dispatch-read interleave. Rule 5 landed with
  the corrected framing.

One weak candidate DROPPED by coordinator decision: `phase-disjoint-buffer-
aliasing` (subagent 2's Rule 4 — g_normed_mlp reused as g_normed_attn at
L1529-1530, live ranges verified disjoint, but the local win is ONE 9 KB Vec;
the decision shape is partially covered by needless-swap-temp + inplace-output-
alias's hazard analysis).

## 4. Bug yield — riir-ai Issue 689 (15 hazards)

Quantified at Gemma-2-2B dims (26 layers, n_embd 2304, q_dim 2048,
head_dim 256, n_head 8, n_kv 4, kv_dim 1024, mlp 9216, vocab 256,128):

| ID | Sev | Hazard |
|----|-----|--------|
| H1 | P2 | Default-path CPU KV repopulate re-applies RoPE with **~10.2M transcendentals/forward** (26×256×512 pairs×3; `apply_rope` has no freq table) because `k_post_rope` is never saved — the value was already computed on GPU (L820-821). Cheapest high-yield fix: one more handle clone. |
| H2 | P2 | Per-position path readback = **93,440 unbatched reads**, contradicting its own "ONLY sync point" doc (L332-333, L461) and the file's own L959-961 cost model. |
| H3 | P2 | `g_logits_all`: fresh **262 MB alloc + dead zero-fill per backward step** (n_tgt×vocab×4B), outside the hoisted scratch set; fully overwritten before any read. The backward-side twin of Issue 688 H12. |
| H4 | P2 | `g_attn_out_vecs` comment-lie: claims "view"/"reusable buffer", code allocates a fresh `Vec<Vec<f32>>` per layer — **26×seq_len allocations/step** (~109 MB @ seq 511). The TODO names the fix. |
| H5 | P2 | Nested↔flat reshape round-trip per layer around the attention backward: **~327 MB/step of layout-conversion copies @ seq 511**. Root cause: `attention_backward_layer`'s `&[Vec<f32>]` API + `g_x_at_layer_out`'s nested layout (every consumer slices it flat-compatible). |
| H6 | P2 | `read_handle` returns a fresh Vec per stage per layer: **~1.2 GB/step download-side allocation churn @ seq 511** — the read-side twin of the ~900 MB upload-side hoist. Fix substrate already in-tree: `read_handle_into` (kimi_k3_gpu_forward.rs L817-821). |
| H7 | P3 | Missed CPU/GPU overlap: A2 dispatches the down-GEMM non-blocking + A3's read is the next statement; the LoRA-down param-grads (deps: stage input + saved activation only) run after the read instead of inside the window (Rule 6's unexploited opportunity). |
| H8 | P3 | `rmsnorm_backward_into`'s single scratch out-param forces a scratch→destination copy per call (~244 MB/step @ seq 511); sibling `gegelu_backward_into` already has the two-destination API shape. |
| H9 | P3 | Doc-lie cluster: the `GPU_BATCHED_FORWARD=1` doc (L1128-1129) states the OPPOSITE of the actual selector (mod.rs L985: `!= Ok("0")` — batched is the DEFAULT); the "4.8× slower" sentence is ambiguous (it was the hybrid that was slower). |
| H10 | P3 | `dispatch_add_gpu` per LoRA application: **182 add dispatches/batched forward** (~20% of total), 46,592 in the per-position path — epilogue-fusable into the delta GEMV (B29's epilogue-at-register-write-phase). |
| H11 | P4 | Two tail reads not batched (L1033-1034: x_before_final + logits — 2 syncs that could be 1). |
| H12 | P4 | Per-position embedding build: per-pos Vec alloc + upload (legacy path only; batched path fixed at L757-769). |
| H13 | P4 | Dual KV caches in the per-position path (GPU cache written incrementally L211-223 + CPU cache re-derived at the tail) — the B34 hazard family; genuinely consumed there (`dispatch_attention_gpu`), not dead, but the same duplication shape. |
| H14 | P4 | Env lookups inside the layer loop (L834: `PLAN423_GPU_ATTENTION` per layer per forward — 26 lookups; trivially hoistable). |
| H15 | P4 | `attn_scale = 1.0 / (hd as f32).sqrt()` recomputed per layer (L1479). |

Suggested fix order: (1) H1 (save `k_post_rope` — one clone, kills 10.2M
transcendentals); (2) H2 doc fixes + port `read_batched` to the legacy path;
(3) H3+H4+H5 (flat layout unification — the attention API's flat-slice variant
kills three hazards at once); (4) H6 (`read_handle_into` adoption — substrate
exists); (5) H7 (window hoists); (6) H8 (dest-param rmsnorm variant).

## 5. Second-production-instance annotations

The file is a second production instance (no new rule) for:
- `op-major-backward-loop-inversion-batch-sequence-axis` (B34 — "~35K
  per-position dispatches → ~183" is this file's instance; 7/layer × 26 + 1 = 183 ✓)
- `adapter-grad-join-mirrors-forward-delta-insertion-point` (B35 — all four
  batched join sites add the adapter input-grad at exactly the forward's
  delta insertion point)
- `tree-reduction-depth-over-sequential-accumulate` (B25 — the Issue 424 note
  at L1097-1101 is the same fix's second call site)
- `sigmoid-deriv-recovered-from-output` + `saturating-nonlinearity-recovery-
  breaks-near-cap` (B32b/B35 — the LM-head tanh recovery at L1243)
- `needless-swap-temp-hoist-to-scratch` fix(2) (B32b — the L1597 swap)
- `fan-out-shared-input-single-sync` (B25 — QKV/GEMM shared-input clones)
- `gqa-kv-head-sharing` + `softcap-tanh-attention-score` (B23 — CPU attention
  internals)

## 6. GOAT

- **G1** `batch36_pseudocode_parses` 7/7 + `batch36_rules_present` +
  `batch36_concepts_covered` — all inside the suite.
- **G2** **324 kernel_opt** (was 321) + **201 default** lib tests PASS.
- **G3** clippy 0 warnings in all 4 states (kernel_opt/default ×
  lib/all-targets). 7 doc_markdown backtick fixes pre-commit (`481/482`,
  `read_batched`, `param-grad`, `g_logits_all`, `LoRA`, `RoPE`, `all-GPU`'s
  neighbor).
- Python orphan-`rule_id` scan: 0/282.

## 7. Generator

One-shot Python generator, one anchor bug caught cleanly pre-write (the
entries-vec close anchor used `rindex` of the last rule id, which resolved to
the marker's occurrence in the batch-35 TEST, not the entry — assert-before-
write aborted with zero pollution; fixed by anchoring on the last entry's
distinctive opt_pattern text `dots[t] = d; // raw dot`, count==1 asserted).
First compile after fix. Deleted after run.
