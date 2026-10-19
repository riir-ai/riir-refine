# Research 287 — Batch 252: the #369 prism verification kernel + the upstream leads

**Status:** RECORD — landed 2026-10-19 (shikuwa/4090 idle loop, Plan 256). Batch 252: +2 own rules + 5 count-neutral rider/second-instance faces + 1 watch disclosure; corpus kernel_opt 901 → 903 (C 57→58, F 20→21).

## Sources (all read at local pins)

| Source | Pin | What |
|---|---|---|
| PrismML fork #369 | `1180918489` (origin/prism tip; clone `E:/git/vendor/prismml-llama.cpp`) | "cuda: add opt-in staged GDN verification kernel" — `gated_delta_net_l8.cuh` +219, `gated_delta_net.cu` +19, `licenses/yukon-cudafast` MIT notice. Adapted from the Yukon engine `36c9536` — B251's OWN source commit, second stack. |
| Upstream #30602 | `ff5a2337` (origin/master; clone `F:/llama.cpp`) | "cuda : fuse qwen4exp's SCALE -> SIGMOID -> SCALE -> hc_post chain" — port of the Vulkan HC_POST_GATE fusion #29520 (`03a667aa`). |
| Upstream #30572 | `5aeac37a` (same clone) | "fattn do not check oob if k_VKQ_max = (k_VKQ_max - nbatch_fa)" — `fattn-tile.cuh:958` one-character `<` → `<=`. |
| Upstream #28851 | `0878f7c3` (same clone) | non-contiguous SUM (CPU/CUDA/Vulkan/WebGPU) — **WATCH-ONLY**, pinned by a negative assert in `batch252_rules_landed`. |

## The conversion verdicts

### #369 (the fork's own line) — SECOND-STACK instances, not new rules

The kernel is the Yukon L8 arm re-integrated for llama.cpp's verify band (8–16 tokens vs the eager path's ≥17). Both B251 laws re-appear verbatim:

1. **`lane-permuted-column-reduce-same-butterfly-tree-bit-identity`** — the header comment IS the law: "Preserve the original xor-16,8,4,2,1 reduction tree across four old lanes per new lane." (l8.cuh:13); the fold `__fadd_rn(__fadd_rn(p[0], p[2]), __fadd_rn(p[1], p[3]))` + xor 4/2/1 in-group (:15-21). Product-coverage validation: same law, same source commit, independent integration — landed as a SECOND INSTANCE annotation.
2. **`host-dispatch-predicate-mirrors-kernel-compiled-arch`** — the dispatch comment re-states it: "Host and device must agree on columns/warp. Kernel uses `__CUDA_ARCH__`; host uses `ggml_cuda_highest_compiled_arch(cc)`." (gated_delta_net.cu:8-9). SECOND INSTANCE annotation.

Three NEW integration faces → count-neutral riders on host entries:

- **R1** on `packed-code-cpasync-stage-per-use-prmt-decode` (gpu_41): the 4-byte element-granular `cp.async.ca` stage with the computed per-element dst remap (`dst_i = (r*8 + n_)*4 + m`, :93) so the token loop reads `float4` contiguous (:151) — stage narrow, read wide, no transpose pass.
- **R2** on `verify-ubatch-full-prestage-lane-token-gate-broadcast` (gpu_105): the fixed-token-window face — SMEM sized by the band ceiling (`stage[GDN_L8V_MAX_TOKENS][TOK_SM]`, TOK_SM = S_v+16+4, :78-79), dispatch gates `n_tokens ∈ [8,16]`, pad lanes staged-skip (`continue; // padding`, :99).
- **R3** on `gate-activation-precompute-pass-token-threshold-gated` (gpu_105): the template-arm specialization face — same `<RAW, G_PRECOMPUTED>` vocabulary, `if constexpr (RAW)` in-kernel activation (:124-131), `g_lane = G_PRECOMPUTED ? g0 : expf(g0)` (:132).

### #30602 → own rule `sigmoid-gate-chain-folded-into-consumer-template-arm` (C)

The graph-fusion law: a SCALE→SIGMOID→SCALE chain building gate weights (`2*sigmoid(inject/hc)` — gate_in = 1/hc, gate_out = 2.0f, verified at the Vulkan origin's test builder) folds INTO the consumer as a `gated` template arm computing `gate_out * sigmoid(gate_in * p)` from the RAW weights; the host rewires the source (`post = gate_scale_in->src[0]`), scalars ride op params; admission is a graph-pattern match (`hc_post_gate_ops = { SCALE, UNARY, SCALE, DSV4_HC_POST }` + edge checks + the zero-bias guard "a bias on either scale is not handled" + same-shape). Four instantiations from two flags. Bit-identity by construction (same op order, same scalars). Distinct from B220 (MoE multiply-weight fold into REDUCE — no chain, no rewiring) and B251's gate-activation rule (WHICH pass activates — orthogonal to eliding materialization).

### #30572 → own rule `oob-free-loop-inclusive-of-last-full-tile` (F)

The one-character class: the `oob_check = false` fast arm's loop bound must be INCLUSIVE of the last fully-covered tile (`k_VKQ_0 <= k_VKQ_max - nbatch_fa`); the strict `<` silently demoted one fully-covered tile per row-block to the checked path — a silent perf tax invisible to correctness tests (both bounds compute identical answers). The oracle is the tile-coverage partition (fast + ≤1 checked == exact cover; fast count +1 on the step-aligned face) — pinned in the opt_pattern test. Kinship: B26 `dense-fast-path-bounded-by-selector-coverage` (admission vs in-regime loop bound), `pad-tail-to-fixed-tile` (pad-into-regime vs bound-into-regime), entries_gpu_34's extent-clamped views (correctness of the checked path vs routing away from it).

### #28851 — WATCH (no corpus entry)

Non-contiguous SUM support across 4 backends is op-generality (a `nb00 != sizeof(float)` strided fallback beside the contiguous fast path in `ggml_compute_forward_sum_f32`). No perf mechanism, no bounded fix-space — the corpus's decision-content bar excludes it. The negative assert in the gate pins the disposition so a future session doesn't re-stage it as a rule.

## Gates (all green, 2026-10-19, /tmp target dir)

- `batch252_rules_landed` + `batch252_riders_landed`: FIRST-RUN PASS (2/2)
- kernel_opt lib 3163/0 (was 3161 + the 2 new gates) · default lib 2696/0
- clippy −D warnings ×3 postures (default / kernel_opt / all-features) clean
- retrieval floors HELD: kernel self-variants 100%, blended 90.6% ≥ floor — no re-pin
- freeze/score-bench NOT owed (kernel_opt-only corpus — the P30/B250 precedent)

## Corrections during verification (the house disclosure rule)

- fattn cite tightened to `:958-959` (the hunk header said 955; the `while` is :958, `constexpr bool oob_check = false;` :959 in the post-image).
- l8.cuh line spans corrected after first-pass misquotes: MIN/MAX tokens :9-10 → :10-11; the reduce fold :11-19 → :13-22; the device arch gate :37 → :53; TOK_SM/stage :72-73 → :78-79; padding :93 → :99; the RAW arm :126-134 → :124-132, `expf(g0)` :137 → :132. All re-verified against `/tmp/gdn_l8_369.cuh` (extracted from the pin).
- The R1 asm quote initially broke the Rust string literal (raw `"` inside `"..."`) — landed verbatim with escaped quotes; the gate needle matches the VALUE.
- The "gate_in carries 1/hc, gate_out carries 2" claim was verified at the Vulkan origin #29520's test builder (`post = ggml_scale(ctx, ggml_sigmoid(ctx, ggml_scale(ctx, post, 1.0f / (float) hc)), 2.0f);`) — not assumed from the CUDA diff alone.

## League side-notes (coordination, no action)

- The fork tip `1180918489` (#369) is UNCHANGED since the 10-19 ~08:1x intake check; newest tag still `prism-b10777-e3de715` (docs-only) — the decode re-pin trigger (next SCORED tag carrying #369, or L8V default promotion) NOT fired. The fork's `origin/master` is its upstream-sync branch (10,616 commits past the prism lineage; tip `5ea87ddad2`) — upstream-material only.
- Their H200 validation numbers (0.990–1.022× model-level, 0.864×/0.875× regression at 48 heads/2-seq, default-off by their own doc) are already recorded in the intake row — the decode threat to our 1.0597× hold is real but not imminent; their port deliberately does NOT adopt the Yukon prefill kernel (our plan-621 L8 + Issue-1066 chunked prefill lane has no mirrored opponent move).
