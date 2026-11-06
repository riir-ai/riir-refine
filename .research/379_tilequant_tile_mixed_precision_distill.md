# Research 379 — TileQuant distill verdict (arXiv 2611.07217 code release)

**Status:** DISTILLED — pending owner decision (nominal Batch 338, Plan 347 STAGED unclaimed)

## Source

- Repo: `HanzhiZhang-Ulrica/TileQuant` @ `6c66e0c60f80243bc9747c81319ceb7efdc4b5c0` (HEAD push 2026-10-12T03:42:50Z; commit "TileQuant: tile-level mixed-precision INT8/BF16 quantization").
- Paper: arXiv 2611.07217 "TileQuant" (walk-24 B lead; submitted 2026-11-04 — 23 days after the last push; the release precedes the paper, the honest order).
- License: **NONE on disk** (no LICENSE/LICENSE.md; GitHub API license: null). Recorded as a caveat — the corpus consumes DECISION CONTENT with ORIGINAL Rust fixtures (never source verbatim), and unlicensed-source distillations stay decision-content-only; any future outbound-PR posture for this family is FAIL-CLOSED absent a license. Precedents: GPL-3.0 (CVEngine) and undeclared (jeva, measurement-only) were both distilled with the posture recorded.
- Size: ~3.6k lines Python across src/ (Python/CUDA-12/A100 INT8 tensor cores, Triton kernels); no README-measured numbers table (the paper carries the 2.66x throughput claim; source-reported, provenance never bars).

## Verdict line

**YES B− corpus [2–3 own rules after ~50% falsification + 3–4 count-neutral faces] / lane-intel MEDIUM-HIGH / product-coverage: seeds the Triton-kernel family — the corpus's first PURE-Triton source** (vLLM's Triton files embed in the vllm family; this is a standalone Triton kernel repo).

## Mechanism map (what the repo does)

64x64 hardware-tile mixed INT8/BF16 weight quantization for decoder linears (Q/K/V/O, Gate/Up/Down): a calibrated importance map keeps a small tile fraction (keep 1–20%) in BF16, the rest INT8 with one scale per tile; activations quantize INT8 per 64-COLUMN tile with a batch-wide scale; a row-owned Triton GEMM (`gemm_rows_splitk_64x64.py`) computes exact INT32 tile dots for INT8 tiles (scaled in FP32, ascending column order) and FP32 tensor-core dots for BF16 tiles, with an in-kernel serial split-K reduction. Weights pack at checkpoint load ("no quantized checkpoint is stored" — the decode-blind absorption shape); a fixed-shape CUDA-graph decoder runs greedy decode.

## Ranked candidates

| # | Candidate | Grade | Category | Source | Notes |
|---|---|---|---|---|---|
| 1 | `ascending-tile-arena-address-is-loop-index` | B−/C+ | D storage-layout | pack.py:38-45 + gemm header L13-27 | Load-time sort of tile payloads into the consumer's traversal order ("the tiles of output-row tile r are contiguous and in ascending input-column order, so row-owned kernels address the payload directly from the loop position (no index indirection). The per-tile quantization does not depend on the storage order.") — the layout-choice-for-prefetch law: the hot loop's weight address IS the loop index, Triton prefetches the next tile while the current is in the tensor cores; the decision content (which tiles INT8) is order-free so the sort costs nothing semantically. MANDATORY in-rule kinship notes: `csr-index-via-prefix-sum` owns the CSR row_ptr/cumsum CONSTRUCTION mechanics; `repeat-axis-aliased-index-arena` (B336) owns arena ALIASING; `load-time-decode-blind-transform-absorption` (B336) owns load-time absorption. This owns the ADDRESS-SHAPE axis (storage order chosen so the kernel needs no indirection). Honest drop allowed at T0 if the T0 read collapses it into csr. |
| 2 | `batch-wide-column-tile-actmax-scale-pass` | B− | N quant | quantize_activation.py:29-44 + fused 77-112 | The activation scale is per-64-COLUMN-tile and shared across the WHOLE batch: a dedicated absmax sweep pass (row-loop inside one kernel, one scale per column tile), then a quantize-with-scales kernel for large N; FUSED single-kernel when N ≤ 64. This is the positive arm of the B335 GEMV-prologue ANTI-face ("every threadgroup recomputes the row statistics" — here the statistics are computed ONCE, never per-threadgroup). The scale is rounded to the NATIVE dtype (bf16/fp16) before the f32 use (gpu_99 rounding-points kin) with the never-zero guard (`tl.where(max_abs > 0.0, scale, 1.0)`). Kin: gpu_34/69/99 in-place per-tile scales (uzu T0 row), gpu_99 dtype-boundary face. |
| 3 | `compiled-handle-direct-launch-key-completeness` | C+/B− (face-leaning) | D host | launch.py (CachedLauncher, whole file) | Decode-step host tax: "A decode step launches a few hundred small kernels and Triton's per-call binder/specialization dominates their host cost" — the first call goes through the JIT path and RETURNS the compiled handle; later same-key calls invoke `kernel.run(...)` directly. The key-completeness contract is the decision content: the caller's key must cover "argument dtypes, 16-byte pointer alignment, the exact integer arguments (==1 / divisible by 16 / 32-bit range), all constexprs and launch options" + the correctness gate "Launch hooks (profilers) need Triton's launch metadata: use the JIT path". Adjacent-DISTINCT from `pipeline-cache-key-covers-every-function-constant` (molten, B293 face 4 — the Metal pipeline-cache key-completeness GENUS owner): same genus, different framework (Triton runtime binder vs Metal function constants) + the hook-refusal arm the molten rule lacks. In-rule adjudication note mandatory; may land as a face on the molten rule. |
| 4 | FACE: serial split-K fixed order + replay-safe self-resetting counter | face | E parallelism | gemm L159-183 + workspace L193-218 | "the last split to arrive adds the partials in fixed split order 0..SPLIT_K-1. Deterministic, and no second kernel launch" = POSITIVE INSTANCE of `splitk-reduce-one-launch-slice-order-pinned` (B199 + slice-1 rider, which owns "the ORDER + LAUNCH-COUNT contract of split-K reduces within one process") — cite, never re-own. The DISTINCT atom worth a face: the arrival counter SELF-RESETS by the last finisher (`tl.atomic_xchg(counter, 0, sem="relaxed")`) so "every kernel returns the counters it used to zero, so the buffers are reused without re-initialization (also inside CUDA-graph capture/replay)" — the replay-safety axis (a captured graph cannot re-zero between replays). Adjudicate at T0 as a face on splitk-reduce-one-launch or on the molten read-and-clear lifecycle face (B293). |
| 5 | FACE: graph-capture pool-pinning admission | face | D host | graph_decode.py:13-15 | `MAX_CAPTURED_PREFILL_TOKENS = 16384` — "a captured prefill would pin all of its activations in the graph pool": the GRAPH-POOL-RESIDENCY axis of capture admission, distinct from B336 `graph-fallback-coverage-ladder` (coverage) and B319 tiny-proj-aux-stream (capture-only latency). Face on either. |
| 6 | FACE: sibling-projection shared act-quant memo | face | — | mixed_linear.py:20-36, 54-112 | "Q/K/V and gate/up project the same activation independently. Keep one inference-only entry per CUDA stream so those sibling projections can reuse the exact same activation quantization" — weakref+version-signature keyed memo, evict-before-realloc. The MEMO arm vs `fuse-gate-up-gemv-activation`'s FUSION arm (B335 faces live there) — same goal, different mechanism. Face or drop at T0. |
| 7 | KILL | — | — | select_split_k | `splitk-tier-ladder-per-tier-ceiling-min-partition-inputs` + `splitk-atomic-free-contention-threshold` + `splitk-width-shape-need-gate` own the split-K selection ladder family (CTAs-per-SM target, pow2, min-tiles floor = the same three shapes). |
| 8 | KILL | — | — | sx*sw scale fold, IAA scale search | `spread-constrained-scale-fold-int8-admission` (B335) + gpu_33 chunk-rescale own the per-tile scale-product shape; the IAA 3-candidate {0.75,1.0,1.25} importance-weighted scale search is OFFLINE calibration (NotBenchable-by-regime, thin decision content). |
| 9 | KILL/cite | — | — | accumulation order | "FP32 accumulation in ascending column order... fixed split order... so results are deterministic" = EXTERNAL CORROBORATION of the Issue-940 determinism law (same class as gpu_142's third implementation) — cite in the note, never a rule. |
| 10 | KILL/lane-intel | — | — | concat_packs_along_rows | QKV row-stacking with exact column slices, payloads copied unchanged — kin to gpu_01 fused dispatch; the HOME value is the riir-infer GDN in_proj_qkvz/in_proj_ba pair (lane-intel row below), not a corpus rule. |

## Lane-intel (workspace)

- **4090 league axis:** per-tile W8A8 with a BF16-retention keep-map is the SAME Ada INT8-TC axis the B335 verdict named (INT8 ~2x FP16 on sm_89) — a dense-weight class, NOT transferable to PQ2_0 ternary (different format family); the 1–20% importance-retention shape is the outlier-channel retention pattern (the 2611.01864 "quantizing recurrent LMs is easier" corroboration stands — recorded there, nothing new filed). Same-box cells own any claim.
- **GDN projection pair:** the shared-input QKV/gate-up packing (concat_packs + the act-quant memo) maps onto riir-infer's `in_proj_qkvz`/`in_proj_ba` pair. The OWNING LANE files the sibling issue (the blazing-smtlib T5 precedent); never this verdict's act.
- **Decode-blind absorption (positive instance, cite):** "Target weights are packed while the checkpoint is loaded; no quantized checkpoint is stored" — the load-time-decode-blind-transform-absorption law (B336) re-derived independently.

## Product-coverage

First PURE-Triton source. The Triton-kernel family was 0 members (vLLM's Triton files count toward the vllm family). Follow-up pool (accumulation clause at batch time): flash-attention Triton reference kernels, liger-kernel, unsloth Triton kernels, torchao subclasses.

## Dedup evidence (greps this session, at corpus HEAD 1056)

- split-K family: `"splitk-reduce-one-launch-slice-order-pinned"`, `"target-tile-split-k-deterministic-two-dispatch"`, `"splitk-tier-ladder-per-tier-ceiling-min-partition-inputs"`, `"splitk-atomic-free-contention-threshold"`, `"splitk-width-shape-need-gate"`, `"row-count-invariant-bits-split-keyed-on-weight-shape"` — the split-K cluster is DENSELY owned (candidates 4/7 adjudicated against it).
- Arena/index family: `csr-index-via-prefix-sum` + `repeat-axis-aliased-index-arena` + `load-time-decode-blind-transform-absorption` present; zero owner of the ADDRESS-SHAPE/prefetch axis (candidate 1 survives to T0).
- Act-quant family: gpu_34/69/99 (per-tile scales), gpu_99 (rounding-points), the B335 GEMV-prologue faces — the BATCH-WIDE dedicated-pass arm unowned (candidate 2 survives).
- Launch family: `pipeline-cache-key-covers-every-function-constant` present; no Triton-binder bypass owner (candidate 3 face-or-own at T0).

## Caveats

- NO LICENSE (see Source) — decision-content-only posture recorded; the batch lands ORIGINAL fixtures.
- Single-author repo, 0 stars, created 2026-05-06 — maintenance irrelevant per the skill, recorded for provenance.
- Unread at verdict: calibrate.py (369 ln), decisions.py body (498), graph_decode.py body (479), quant_utils.py (164), models/rules.py (76), explore/ — re-read at mine time (T0); the heads read cover the verdict's claims.
- The paper's 2.66x/quality numbers are paper claims (no in-repo bench table) — cited as paper claims only.
- Fast-moving risk LOW (push 2026-10-12, pre-paper; the pin is the release state).

## Queue slot

Plan 347 (nominal Batch 338) STAGED unclaimed — claim-first at the next kernel_opt slot per the queue protocol. Research highwater 378 → 379. The LoopDuet lead (the other walk-24 staged scan) CLOSED no-source this session (snapshot row upstream).
