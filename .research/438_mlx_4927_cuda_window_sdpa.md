# 438 — Distill verdict: ml-explore/mlx PR #4927 "[CUDA] faster window attention"

**Status:** DISTILLED — LANDED as Batch 404 (kernel_opt 1109→1113, Plan 403; see the queue-snapshot LANDING line + Plan 403's receipt)

## Source

- **PR:** https://github.com/ml-explore/mlx/pull/4927 — pierreablin, "[CUDA] faster window attention", 2 commits (`318a72d` "window attention in cuda" + `5a7769e` "fix sink + deps"), **merged 2026-11-22 into ml-explore/mlx main at `58d91ee`** (zcbenz approved). License MIT.
- **Inspected:** 2026-11-23 (zed@m3), from the merged `.diff` fetched over the wire — **every quote below is hunk-derived from the PR diff, NOT byte-verified at a clone**; T0 of the batch clones `.raw/mlx` at the merge sha and re-verifies each quote site in `mlx/backend/cuda/scaled_dot_product_attention.cpp`, `mlx/fast.cpp`, `mlx/fast_primitives.h`, `python/tests/test_fast_sdpa.py` (the ephemeral-clone law, pin before verdict).
- **Provenance chain:** ml-explore/mlx is a THREE-TIME prior source — Batch 197 (2026-10-02, the Metal SDPA-VJP pack @ `83b976ea`/`92951975`, 6 rules + 1 rider incl. the A− `sdpa-backward-as-tiled-matmul-bounded-materialization`), Batch 205 (2026-10-07, #4516+#4596, 8 rules), the 10-24 verdict (#4712+#4682 qmv/NAX, staged Plan 277). This PR is the **CUDA sibling of B197's Metal SDPA work — same file family** (`scaled_dot_product_attention.cpp`), and it cites the same upstream shape family. Read it as a family return visit, not a new source.

## What the PR does (mechanism, from the diff)

Before: CUDA window attention hit `has_fused_kernel`'s NYI gate — `return {false, "window attention support for CUDA is NYI"}` — so **every** windowed call ran the unfused fallback = **full attention**: O(L²) time and O(L²) memory. After:

1. **`set_window`**: the window encodes EXACTLY into cuDNN's fe::graph SDPA attributes — `set_diagonal_alignment` (TOP_LEFT when T_q == T_kv — "since cuDNN <= 9.9 on Hopper needs it" — else BOTTOM_RIGHT) + `set_diagonal_band_left_bound(window_size)` + `set_diagonal_band_right_bound(do_causal ? 0 : window_size - 1)`. No new kernel; the parameter rides the existing graph API.
2. **`supports_sdpa_cudnn` window admission** (the new refusal classes, each named):
   - `cudnnGetVersion() < 91501 → false` — "Older cuDNN does not support sinks (before 9.13) and **gives wrong gradients when T_kv % 128 != 0 (before 9.15.1)**" — a version-gated KNOWN-WRONG window, refuse loud.
   - `has_arr_mask || q.shape(2) > k.shape(2) → false` — "cuDNN does not support a window with an array mask, or when T_q > T_kv."
   - `q.shape(2) == 1 → false` — "The vector kernel reads only the keys in the window" (decode admits the vector kernel instead, after slicing).
3. **`clip_window`**: "Remove the window when it keeps all the keys" — `window_size < max(qL, kL) ? window_size : -1`; a band covering the whole domain is a vacuous constraint — pass the plain path so it takes its fastest form.
4. **`slice_last_kv`** (the decode fallback): "With one query, the window keeps the last window_size keys" — slice `kv[..., -W:, :]` on GPU and hand the EXISTING `sdpa_vector` the reduced operand; the kernel stays untouched, reads drop O(L)→O(W).
5. **Stride-0 decode refusal**: `use_cudnn_for_decoding` now refuses when `kv.strides(2) == 0` — a broadcast row is not a contiguous KV cache; the stride-derived contiguity check would misread it.
6. **VJP**: the old fused backward **threw** `invalid_argument` on gradients w.r.t. mask/sinks; now "the fused backward computes only the gradients of q, k and v" and any `arg >= 3` routes to the `Custom::vjp` composition fallback — which computes them correctly. Latent bug fixed in the same stroke: the sinks index moved from `inputs.back()` to `inputs[primals_size - 1]` (the inputs list gained output + logsumexp + cotangent, so `back()` had silently become the cotangent). `ScaledDotProductAttentionVJP` gains `window_size_` in `state()`/`is_equivalent` — primitive identity must carry every semantic parameter.

**Headline (PR tables, H100 + B200, bf16, 16Q/8KV, dim 128 — provenance, never adoption bars):** L=32768, window=512: fwd 133.49→0.43 ms (~310×), fwd+bwd OOM→1.99 ms, peak 41.3→1.35 GB. L=16384, W=512: 33.61→0.27 ms, 28.5→0.67 GB. Zero-window rows byte-flat (0.24 vs 0.25 ms — no regression on the unaffected path). Exhaustive correctness battery shipped: window × causal × {fp16,bf16} × shapes, backward parity incl. sinks and nested grad-of-grad, against a float32 CPU reference.

## Verdict

**YES B corpus (rider-batch staging) / LEAGUE INTEL MODERATE / MLX family continuation — no new family seed.** Live counts at verdict (grep-derived, `rule_id: "` over `src/domains/*/corpus*`): kernel_opt 1114 · rust_perf 392 · sec 85 (AGENTS head 1109/392/84 @ Batch 402, landed today — the delta is the post-head landing trail, resolved by pins_manifest at batch time).

## Domain routing

**`kernel_opt`**, category D (dispatch) for the lead; the admission/decoding faces ride D/N kin. The tempting WRONG domain: `rust_perf` — the fixtures are CPU Rust but the decision content is GPU host-orchestration (the routing table's "host-orchestration around GPU" row); the corpus's own MLX precedent (B197/B205) landed exactly this shape in kernel_opt. NOT file-mining: the riir-gpu frontier is closed; this is external-PR intake (precedent B197/B205/B398).

## Dedup gate — grep evidence (2026-11-23, both vocabularies)

| candidate | greps (source vocab + corpus operator vocab) | result |
|---|---|---|
| NYI→full-work dispatch | `window|sliding|sdpa|diagonal` over kernel corpus → **zero window-attention rules**; `NYI|falls.back|fallback` → `sibling-kernel-admission-gap-absorbed-by-generalist` (B205) is the INVERSE polarity (generalist absorbs a specialist's refusals; this refuses less at the gate) + `fa-fast-path-validated-exact-shape-admission` (admission-by-shape, not admission-by-encoding) | novel, adjudicate in-rule |
| slice-to-window decode | `slice.*window|window.*slice|last.n.keys` → zero owners. **Already practiced at home**: `riir-infer-gpu/src/gemma4_cubecl/mod.rs:36-38` — "the sliding-window mask is handled by slicing the KV cache window before upload" via `get_combined_kv_window` (decode + prefill) | novel as a RULE; cite our own production usage as the second instance |
| version-gated wrong-grad refusal | `cudnn|version.gate|wrong.grad` → zero. Kin: `compiled-vs-eager-per-op-exactness-admission` (per-op-CLASS axis — this is the dependency-VERSION axis, distinct), `mxu-op-exactness-scoping` (B398, width-scoped exactness — cousin face candidate) | novel |
| vjp fallback-not-throw | `throw.*grad|invalid_argument|vjp` → zero owners | novel |
| stride-0 broadcast refusal | positive-polarity owner exists: `gqa-zero-stride-batch-broadcast` (B197 — stride 0 as a broadcast optimization); this PR is the REFUSAL polarity (stride 0 disqualifies the contiguous-cache fast path) | **FOLD as face** on the B197 rule |
| primitive identity carries window | key-completeness family owns the law (`compiled-handle-direct-launch-key-completeness`, `pipeline-cache-key-covers-every-function-constant`) | **FOLD as face** on the family |
| vacuous-window clip | `vacuous|constraint.*covers` → zero rule owners; it is R1's complement arm on the same dispatch surface | **FOLD as face** on R1 |
| backward memory | `quadratic-attention-intermediates-recompute-from-linear-saves` (gpu_04) owns the save-linear/recompute-quadratic BACKWARD axis — different decision (memory layout at backward, not forward dispatch admission) | cite, both directions |

## Ranked candidates

| # | pattern (proposed rule_id) | source site (diff hunks) | grade | notes |
|---|---|---|---|---|
| 1 | **`windowed-parameter-nyi-gate-forces-full-work-fallback`** — a capability gate answering "not implemented" for a parameter the fast path's API can encode EXACTLY forces every such call onto the generic path that ignores the parameter's structure; the cost is not "no fusion," it is the quadratic work + memory the parameter exists to avoid. Fix = wire the parameter into the fast path (band bounds), refuse ONLY the genuinely-unencodable classes (array mask + window, T_q > T_kv), each refusal named | removed NYI gate + `set_window` + new `supports_sdpa_cudnn` arms | **A− (GOAT lead)** | category D. Two-regime: parameter expressible vs not. Magnitude: measured 310× / 41.3→1.35 GB. The DECISION is NotBenchable-by-domain (cuDNN latency — the entries_gpu_65 annotation class, never a demotion); the benchable arm is the work-quantity fixture: full L×L mask materialization vs O(L·W) band iteration (exact, deterministic, huge) — pre-registered as the validator arm. UPSTREAM: present at `58d91ee` (merged) — decision content only, never a PR. Faces: `clip_window` (the vacuous-constraint complement — a band covering the domain is not a specialization, pass the no-op so the plain path takes its fastest form) + the `window_size_`-in-`is_equivalent` identity face folded to the key-completeness family |
| 2 | **`slice-operand-to-window-before-kernel`** — when a query-invariant reduction of an operand is derivable from a parameter (decode: window ⇒ last W keys), reduce the OPERAND before the call and keep the kernel untouched; never teach the kernel the parameter | `slice_last_kv` + `eval_gpu` fallback arm ("With one query, the window keeps the last window_size keys") | **B** | O(L)→O(W) reads; the kernel unchanged is the point. Home-validated (gemma4_cubecl). Rust fixture: slice the last W rows vs scan all L with a per-element predicate — benchable. UPSTREAM: present at `58d91ee` |
| 3 | **`dependency-version-known-wrong-window-refusal`** — a fast path gated not on capability but on a KNOWN-WRONG dependency version window, refusing loud with the bug class named ("gives wrong gradients when T_kv % 128 != 0 (before 9.15.1)") — plus the alignment pin (TOP_LEFT when T_q == T_kv, "cuDNN <= 9.9 on Hopper needs it") as the same admission predicate's second clause | `supports_sdpa_cudnn` version arm + `set_window` comment | **B** | version axis distinct from per-op-class exactness admission; the wrong-gradient modulus (T_kv % 128) is the regime condition. UPSTREAM: present at `58d91ee` |
| 4 | **`fused-vjp-ungeneric-direction-falls-back-not-throw`** — a fused autodiff path that cannot express a gradient direction falls back to the composition (which computes it exactly), never throws; the throw was reachable and silently disabled correct autodiff for sink/mask grads | `fast.cpp` vjp: throw removed, `other_grads` → `Custom::vjp` | **B−** | faces: the `inputs.back()` → `inputs[primals_size-1]` latent bug (adding an input moved the sinks off the back); the primitive-identity fold (see #1). UPSTREAM: present at `58d91ee` |
| 5 | stride-0-broadcast decode refusal | `kv.strides(2) == 0 → false` | FOLD | face on `gqa-zero-stride-batch-broadcast` (B197) — the refusal polarity of the same stride-0 observation |

## Yield estimate

3–4 rules + 3 folds/faces after ~50% coordinator falsification (R1 is the lock; R2/R3 likely; R4 coin-flip on the authoring read).

## Lane intel (MODERATE)

1. **Home ownership VALIDATED — gemma4_cubecl (riir-infer-gpu):** the laya/clef gemma-arch lane already slices the KV window before upload (decode AND prefill; `get_combined_kv_window`, sliding = `[pos-sw+1, pos]`). The PR independently validates the same choice on cuDNN/CUDA. Nothing to file; the rule cites our usage as the second instance.
2. **League CUDA lane (4090): NO window lever.** The league GGUFs carry **no sliding-window attention keys** — `Ternary-Bonsai-2-27B-PQ2_0.gguf` (qwen35: 64L, GQA 24/4, head_dim 256, ctx 262144) and `Qwen3.8-27B-DFlash2-Q4_K_M.gguf` (dflash head) both probe clean of `sliding_window`/`layer_types` window keys (`scripts/plan402_gguf_probe.py`, 2026-11-23). The PR's prefill scenario (L=32k, W=4096: 133→1.85 ms class) has no target in our league models. Caveat: Bonsai's DeltaNet/attention layer split lives riir-train-side, not in the GGUF header — the attention layers carry no window key per the probe; an exhaustive per-layer audit is not owed for a no-hit.
3. **cuDNN fe::graph diagonal-band API (9.13 sinks / 9.15.1 gradient fix):** we use no cuDNN SDPA (Metal MSL + CubeCL/cudarc custom kernels) — the API-existence evidence transfers as the ENCODING axis argument, not as a lever. Not transferable: every absolute number (H100/B200 cuDNN bf16, 16Q/8KV/128dim ≠ our shapes/stacks).

## Product coverage

MLX family continuation (B197/B205/10-24) — no new family. The dispatch-admission class maps onto any ML-runtime dispatch surface cargo-refine consumers own (candle/burn-shaped code); minor note, no follow-up pool owed.

## Caveats

- **Quotes are hunk-derived from the merged diff**, not byte-verified at a clone — T0 re-verifies at the merge sha before any corpus edit (the pin discipline).
- The PR is 2 days old, merged by a maintainer with an approval — stable upstream state; the `is_equivalent`/cache-key surface moved (`SDPACacheKey` gained `window_size`) — if upstream revises the admission gates, re-pin at T0.
- Test-only file (`test_fast_sdpa.py`) read but not mined — it is the correctness battery evidence for R1/R2's exactness claims, cited not transcribed.
- Yield estimate assumes the authoring read holds; R1's fixture arm (mask materialize vs band iterate) must survive the standalone preflight before entry.

## Queue slot

Staged **Plan 403** (nominal Batch 404), NOT claimed — the next claimant opens it per the queue. Artifacts: this note + `.plans/403_mlx_4927_cuda_window_sdpa_batch.md` + the queue-snapshot line, same commit.
