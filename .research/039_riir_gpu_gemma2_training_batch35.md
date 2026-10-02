# Research 039 — riir-gpu Gemma-2 Training Cluster (Batch 35)

**Date:** 2026-08-15 · **Batch:** 35 · **Corpus:** 267 → 275 (A 49, B 45, C 29, D 43, E 17, H 12, L 9, N 16; +8)
**Source:** `riir-gpu/src/gemma2_cubecl_train/` — `mod.rs` (1,286) + `attention_gpu.rs` (349). `resident.rs` (1,639, the Plan 481 GPU-resident experiment) deferred to a later batch.
**Method:** 2 parallel subagents with explicit falsification instructions + coordinator re-verification of every load-bearing quote (all verified verbatim before landing).

## 1. The cluster

The Gemma-2 training counterpart of Batch 25's inference-side cluster:

- **`attention_gpu.rs`** — the Issue 430 Option B fused causal attention kernel: single dispatch computing QK^T + softcap + softmax + weighted-sum per (head, query_pos) workgroup, eliminating 52 GPU↔CPU syncs per forward. Carries a shipped-and-fixed bug record (Issue 430 G1 FAIL on 4090) that became Rule 4's evidence.
- **`mod.rs`** — hybrid LoRA training forward/backward: `build_backward_handles` (pre-transposed base weights, build-once) + `training_forward` (6 GPU GEMVs/layer, activations saved) + `backward_lora_grads` (7 transposed GEMVs/layer via handles; elementwise backward reuses validated CPU engine functions).

## 2. The 8 rules landed

| # | rule_id | cat | one-line |
|---|---|---|---|
| 1 | `mask-sentinel-matches-reduction-identity` | E | masked lanes PARTICIPATE in a block reduction by writing the identity element (−inf for max), and the sentinel is REWRITTEN between reductions with different identities (−inf → 0.0 between max-pass and sum-pass); non-reduction consumers bound at the valid extent |
| 2 | `smem-staging-gated-by-cross-thread-reuse` | A | stage an operand into smem iff within-workgroup reuse > 1; single-use operands (K row j read only by thread j) stream from global in the same dot loop |
| 3 | `kernelize-host-chain-after-sync-batching-floor` | C | a host-serviced segment floors at 2 syncs (down+up) after all batching; kernelize when intermediates are chain-local + no host work in the gap + per-iteration repetition — the 52 = 26 layers × 2 proof |
| 4 | `block-reduction-separate-scratch-runtime-strides` | B | multi-pass reduction consumers (softmax) reduce into SEPARATE scratch (the source feeds a later pass) + strides from the runtime extent — never in-place, never a hardcoded unroll (both failed G1 on 4090) |
| 5 | `saturating-nonlinearity-recovery-breaks-near-cap` | N | recovering a derivative from the cached OUTPUT yields exactly 0 once it rounds to 1.0 in f32 (tanh arg ≳ 9); recompute the RAW argument when saturation is reachable — the codebase makes opposite choices at its two sites |
| 6 | `adapter-grad-join-mirrors-forward-delta-insertion-point` | H | additive CPU-side LoRA deltas on GEMV outputs ⇒ the backward joins the adapter grad at the same point and the base transposes are IMMORTAL by construction; the merged convention silently drops ΔW^T·g (Issue 687 H2's class) |
| 7 | `side-consumer-readback-alias-live-handle-for-dependent-dispatch` | A | read an intermediate for side consumers but hand the next dispatch the still-live handle — the re-upload burns ~53 MB/step at seq 256 |
| 8 | `quadratic-attention-intermediates-recompute-from-linear-saves` | A | save Θ(N·d) tensors (Q post-RoPE + K/V via KV cache), recompute the Θ(N²) chain in hoisted scratch — the recompute also yields the raw pre-softcap argument the saturating backward needs |

Category deltas: A 46→49, B 44→45, C 28→29, E 16→17, H 11→12, N 15→16.

## 3. Seeds — one killed, one corrected, one survives as sharpened (falsification 3-for-3)

- **Seed "thread role rotation" — KILLED.** The kernel genuinely re-indexes threads per phase (L124 `d = tid`, L136 `j = tid`, L171 `tid < seq_len`, L231 `d3 = tid`) — verified — but the rotation carries no decision content beyond "each phase uses its natural parallel axis with a barrier between," which is ambient in EVERY block-cooperative fused kernel and covered piecewise by existing rules. Third consecutive batch where subagent verification killed a coordinator premise before it became corpus wisdom.
- **Seed "sync-count-driven fusion" — CORRECTED premise, then landed (Rule 3).** The coordinator's framing ("Option C = per-op dispatch with 52 syncs") was wrong: Option C had ALREADY applied download batching (resident.rs L844-851 reads QKV in ONE batched sync). The real insight is stronger: **batching has a floor of 2 per host-serviced segment** — that's what makes kernelization the only path to 0. The 52 decomposes cleanly (26 × 2, both factors verified in source).
- **Seed "stage shared operand once" — survives only sharpened (Rule 2).** The staging mechanics are `cooperative-block-wide-quantize-staging` (B20). The distinct content is the SELECTION GATE including the negative half — one dot loop legitimately reads Q from smem (multi-reader) and K from global (single-reader) in the same breath (L141).

## 4. Dedup verdicts (the predicted pressure materialized)

12 candidates dropped as duplicates/thin:
- attention grid decode → `derive-params-from-global-invocation-id` (B26)
- GQA mapping → `gqa-kv-head-sharing-bandwidth-reduction` (B23)
- softcap → `softcap-tanh-attention-score` (B23)
- phase-1/3 thread-private sweeps → `grid-over-output-thread-private-sweep` (B33)
- scalar params buffer → `pack-scalar-params-into-combined-buffer` (B29; the f32 VALUE-encoding nuance vs bitcast is a footnote, not a rule)
- unsafe launch contract → `launch-unchecked-caller-owned-size-contract` (B29)
- h-recompute at backward → `backward-helper-expose-shared-norm-statistic` (B34) in spirit — filed as hazard H8 instead
- per-layer buffer churn → `reuse-allocation-clear` / `superset-sized-scratch-across-variants` family
- residual clones ×2 → same reuse family
- save-before-destructive-norm → THIN (ordering, subsumed by activation-slots)
- pre-update loss logging → THIN (training economics, not GPU wisdom)
- CPU transpose in build handles → THIN (one-time setup; economics already covered)

Subagent 2's seed #2 (activation-save granularity) also resolved DUPLICATE (consumer-shaped saves covered by `preallocated-activation-slots` + `derive-shape-from-payload`), with the dead-slots delta filed as hazard.

## 5. Bug yield → riir-ai Issue 688 (15 hazards)

**From attention_gpu.rs:**
- H1 doc-lie: 3-way sync-count inconsistency (header 52 vs dispatch.rs "4/layer" vs resident "2/layer") + the module doc still describing the pre-fix "8-step unrolled" reduction the code replaced
- H2 the 9-scalar params buffer re-allocated per launch (26 device allocs/step for never-changing content)
- H3 the 256 constant triplicated (kernel/smem×3/launcher) with no assert tying them
- H4 q_dim/kv_dim derivation invariants un-asserted (silent OOB under launch_unchecked)
- H5 uninitialized smem lanes read-range-safe only by convention
- H6 GQA divisibility unstated

**From mod.rs (+ engine):**
- H7 dead `_into` imports + allocating hot path (~85K allocs, ~1.1 GB churn/step) + the engine doc-lie claiming the GPU path uses the `_into` variants
- H8 the h = A@x recompute at backward (~9.7 G MACs/step discardable for ~12 MB of Θ(rank) saves)
- H9 dead activation slots (q_pre_rope/k_pre_rope/v_proj — ~82 MB dead writes + 82 MB dead readback/step)
- H10 undocumented KV-cache liveness contract (intervening forward corrupts gradients)
- H11 per-layer gradient-buffer allocations (~258 MB churn/step) + half-built ownership swap
- H12 `EngineActivations::new` per step (the logits slot alone is 262 MB at seq 256)
- H13 "~4 GB" doc claim understates handle memory 2.6× (actual ~10.4 GB)
- H14 f32 sequential grad-norm (log-only)
- H15 sliding-window note: the training path implements pure causal attention — no Gemma-2 local/global alternation; long-sequence training would diverge from inference semantics

## 6. Landing method notes

- The one-shot Python generator hit ONE anchor bug before write (category-array anchors missing the string-literal closing quote) — aborted cleanly, zero pollution, second run first-compile. The `assert count==1`-before-write design is now 3-for-3 at catching anchor drift cheaply.
- A **16th count-pin site** surfaced via test failure (not grep): `kernel_categories_counts_match_plan_003_table` — the per-category expected-counts table inside the tests. Add to the skill's count-pin list.
- 4 doc_markdown warnings fixed pre-commit (backticks on `mod.rs`/`LoRA`/`q_dim`/`kv_dim` in the header block).

## 7. GOAT

- **G1:** `batch35_pseudocode_parses` 8/8 + `batch35_rules_present` + `batch35_concepts_covered` (3 new tests).
- **G2:** 321 kernel_opt (was 318) + 201 default lib tests PASS.
- **G3:** clippy 0 warnings in all 4 states (kernel_opt/default × lib/all-targets).
- Orphan-`rule_id` scan: 0/275.

## 8. Progress

riir-gpu: **~76 of ~200 files mined** (the Gemma-2 training cluster's mod.rs + attention_gpu.rs done; resident.rs 1,639 deferred). Corpus: **275 rules**. Next candidates: `gemma2_cubecl_train/resident.rs` (1,639 — the Plan 481 all-GPU LoRA path; expect dedup pressure from B26/B29/B34), `gemma4_q4k_train/` (4 files), `gemma2_d2f*`, `kimi_k3_gpu_*` (4 files), `weaver_gpu*` (4 files), `kvarn/`, `hypernet/`, `maxsim/`. `ternary_deltanet_gpu_forward.rs` + `_cudarc.rs` still BLOCKED on sibling WIP.
