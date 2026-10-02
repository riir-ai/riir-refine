# 117 — Distill verdict: antirez/ds4 (DwarfStar)

**Status:** RECORD (Batch 97 landed 2026-09-02 — see the addendum below)

- **Source:** https://github.com/antirez/ds4 (DwarfStar) — Salvatore Sanfilippo's native
  DeepSeek-V4-Flash / GLM-5.2 inference engine. C + Metal + CUDA + ROCm. Beta-quality,
  fast-changing per its own README. **Inspected 2026-09-02** (M3 box, via zread GitHub
  reads). This is the engine behind the 250.1 t/s @11709-token prefill / 26.68 t/s
  decode M3 Max q2 row already recorded in the league opponent-watch
  (`.distill/001_mining_queue_snapshot.md` streaming note).
- **Route:** `kernel_opt` (Metal kernels + GPU host orchestration). The tempting WRONG
  domain is `rust_perf` for the C host files (kvstore/ssd/hotlist) — those are storage
  concepts that route to riir-neuron-db thinking, not corpus rules; only the kernel +
  dispatch-adjacent host patterns mine. CUDA MMQ lane is secondary (the corpus's Marlin
  cluster B92 already covers W4A16 MMA plumbing broadly).
- **Plan:** `.plans/063_batch97_dwarfstar_ds4_metal_distill.md` — **EXECUTED same day**
  (all T1–T10; see the addendum).

## Batch 97 addendum (2026-09-02, LANDED)

- **Pin at mine time:** `110afdd8886586f18fc9b28bc5533152dd10e728` (2026-09-01 tip,
  "Use random completion response IDs"), clone at `../vendor/ds4`.
- **Final rule table (9 landed, corpus 426 → 435):**

| # | Rule | Cat | Disposition vs this verdict |
|---|---|---|---|
| 1 | `half-domain-exact-band-dequant-with-raw-bit-oracle` | N | candidate 1, GOAT lead, as predicted; T6 exhaustive CPU twin landed (`batch97_mxfp4_half_band_exhaustive_oracle`) and CAUGHT a real bug in its own first implementation (normal path stored the implicit mantissa bit, overlapping the exponent field — 0.5 → 0x3C00 = 1.0; the gate doing its job) |
| 2 | `tail-simdgroup-cull-barriers-preserved` | E | candidate 3 as predicted |
| 3 | `sibling-projections-one-kernel-shared-staged-tile` | C | candidate 4 as predicted; coordinator dedup found the nearest neighbor is B15 `fuse-gate-up-gemv-activation` (GEMV register-sharing twin) — landed with that distinction in-concept |
| 4 | `expert-major-work-map-token-scatter-clamped` | H | candidate 5 as predicted; B94 expert_major lead resolved as predicted (adjacent, distinct) |
| 5 | `proven-shape-static-sibling-identical-results-fallback` | M | candidate 6 as predicted |
| 6 | `barrier-after-async-mma-before-staged-tile-reuse` | A | candidate 2 (MPP direct-RHS), landed as the barrier-lifetime mechanism-doc rule with the honest Metal4-only scope note; upstream baseline contrast pinned (ggml-metal tensor mul_mm = TWO barriers/k-step single-tile — the ds4 delta is the double buffer) |
| 7 | `bit-exact-dispatch-fusion-boundary-emulation` | C | Lane B (dsv4_kv/dsv4_rope) — new beyond this verdict's list |
| 8 | `virtual-thread-folding-preserves-reduction-tree` | E | Lane B (dsv4_hc + flash packed8 + rope lane-identity) — new beyond this verdict's list |
| 9 | `mask-block-classification-skip-placement` | F | **Lane B candidate FALSIFIED as ds4-original**: upstream llama.cpp @ 9ca265a already ships `kernel_flash_attn_ext_blk` (3-class scan + Q>1 gate + consumer classes + inline vec skip); ds4 adopts verbatim. Landed with corrected origin (mechanism absent from OUR corpus; the popcorn precedent) — exactly the falsification the provenance check exists for |

- **Falsification ledger:** candidate 7 (`row_blocks` hoisted index) DROPPED — LICM/CSE
  territory + a thinner B44 instance, no bounded decision. The mask-classification
  provenance falsification above. dense.metal proved NOT llama.cpp-lineage (DS4-native
  matvec family) — 5 leads recorded for the next batch instead of forced rules
  (quad-composition args-aliasing, sum6 route-fused down projection, the
  shared-read-not-merging pair boundary, tiny-matvec parallelism down-shift, the
  2..8-token matvec-ext bridge). Host `ds4_metal.m` single-entry pipeline memo + the
  M5 stable-sigmoid-vs-exp finding → P8 post-mining intake, not rules.
- **Yield vs estimate:** verdict estimated 4–6 after falsification; landed 9 — the
  estimate was conservative because Lane B (unverified files at verdict time) yielded
  3 clean rules and the provenance check rescued one candidate rather than killing the
  batch.
- **GOAT:** batch97 rules_present + concepts_covered + pseudocode_parses
  (cfg(not(kernel_verify))) + `batch97_mxfp4_half_band_exhaustive_oracle` + kernel_opt
  lib 1252/0 + default lib 1008/0 + retrieval floors 14/14 HELD (~304 s, no re-pins) +
  clippy lib kernel_opt/default/all-targets all 0 + score-bench 96.2/1.9/100/100 FLAT.
  Full CI guard not run (B95/B96 honest-skip shape — every affected state validated
  individually).
- **Count pins walked:** tests.rs corpus-count + expected-array A/C/E/F/H/M/N (the
  `kernel_categories_counts_match_plan_003_table` pin caught the category deltas) +
  sums x2 + grouped x2 + router.rs expert total + mod.rs doc + category headers;
  AGENTS.md kernel bullet 414→435 (was already stale from B96 — now true).

## Provenance chain

README acknowledges llama.cpp/GGML explicitly: "Some source-level pieces are retained
or adapted here under the MIT license: GGUF quant layouts and tables, CPU quant/dot
logic, and certain kernels" (GGML copyright retained in their LICENSE). The base
mul_mv/mul_mm quant kernels are llama.cpp-lineage. The candidates below are
**DS4-specific** (none exist in ggml-org llama.cpp's ggml-metal as of the league's
9ca265a pin — the B96 lesson applies: at mine time, verify the fork tip is where each
mechanism landed, and that upstream truly lacks it).

## Discipline evidence (why the grade is not C)

- Two purpose-built A/B harnesses: `speed-bench/metal_decode_schedule_bench.c`
  (alternates variant order AND variant-to-session assignment; **aborts unless every
  full-vocabulary logit row is bit-identical**) and `metal_prefill_variant_bench.c`
  (ABBA/BAAB blocks, poisoned host logit buffers, bit-identity abort). This is our own
  Bench-728 interleaved-discipline, independently arrived at.
- `kernel_test_mxfp4_down_half_lut` — a test-only GPU kernel covering all 4096
  exponent/code pairs of the exact-band table, raw-bit compared against the arithmetic
  fallback, including both sides of the band boundary and the 0xff row. The
  exhaustive-gate mandate from Batch 59/68, shipped as a kernel.
- Measured M3 Max rows committed as CSV/SVG (`speed-bench/`).

## Ranked candidate table (verified: `metal/moe.metal` read in full)

| # | Pattern | Source anchor (moe.metal) | Grade | Notes |
|---|---|---|---|---|
| 1 | **Half-domain exact-band dequant table + exhaustive GPU raw-bit oracle** (GOAT lead) | `dequantize_mxfp4_half_scale` + `dequantize_mxfp4_half_lut` (generated 256×16 half LUT) + `kernel_test_mxfp4_down_half_lut` | A− | E8M0 is a pure power of two and every E2M1 magnitude is exact in binary16, so for E8M0 bytes in [103, 142] the half product is the once-rounded value of the float product; **outside the band (subnormal/overflow halves) keep f32 arithmetic**. 32-byte half table replaces the 64-byte f32 table + the per-element f32→f16 narrowing in the GEMV hot loop. The oracle kernel pins all 4096 pairs incl. both band boundaries and the 0xff NaN-canonicalization fallback row. |
| 2 | **Metal4 MPP direct-RHS double-buffered dequant staging** (mechanism-doc class) | `kernel_attn_out_low_mpp_direct_rhs` + `kernel_mul_mm_id_mpp` (`#ifdef DS4_METAL_HAS_TENSOR`) | B+ | next-k-step dequant overlaps the current `matmul2d` cooperative matmul → one threadgroup barrier per k-step instead of two; in-source contract: "the barrier after mm.run prevents the next K iteration from replacing staged tiles while the cooperative matmul still reads them". Full tiles skip all bounds work via a `full_tile` host-dispatch guarantee. NOT runnable on our M3 Max (Metal4 TensorOps is M5-class; the source itself says "pre-M5" elsewhere) — B70/B92 precedent: the rule encodes the mechanism, the numbers stay source-specific. |
| 3 | **Tail-SIMDgroup cull under preserved barriers** | `CULL_TAIL_SIMDGROUPS` template flag / `mma_active` in `kernel_mul_mm_id` + `kernel_mul_mm_id_pair_swiglu_f16_impl` | B | "Keep all threads in staging and at every threadgroup barrier, but let the second row-half skip MMA on short final expert tiles." Barrier-safety-PRESERVING work skip — the complement of our B57 barrier-adding rules and distinct from predicate-gated-skip (B30, host-level). Two commit-parameterized variants (64×32 and a 32×32 compact-tail kernel) — the A/B harness isolates each cull with its own rollback env. |
| 4 | **Gate+up pair GEMM sharing ONE staged B tile** | `kernel_mul_mm_id_pair_swiglu_f16_impl` (+ compact-tail twin) | B | Two A streams (gate, up) staged against ONE shared B (activation) tile per k-step: "two threadgroup barriers per k-step total instead of two per matrix"; epilogue matches the unfused `swiglu_weight_f16` exactly — "the fused result is bit-identical to the unfused path". The 5th member of our same-input family (B58/B63/B77/B94) but the PAIR/two-A-one-B shape is new to the corpus. |
| 5 | **Expert-major compact work map via atomic scatter + work trailer** | `kernel_mul_mm_id_map_scatter_work` (vs the token-scan `kernel_mul_mm_id_map0`) | B− | Turns token-major top-k route ids into expert-major 32-row matmul tiles in ONE kernel: threadgroup atomics build per-expert counts, `tile_counts` rides the same staging shmem, a work trailer publishes the exact non-empty-tile count ("Zero work must overwrite any stale prior value"). Duplicate clamp: "Production top-k selections are unique. Keep malformed or synthetic duplicates from exceeding the fixed expert slice" (`row < ne21` guard before the store). **Lead resolved:** in-corpus `expert_major` is B94's scratch-bank LIFETIME rule (oMLX pool.py) — different mechanism (bank eviction vs map construction); cite B94 as adjacent, not duplicate. |
| 6 | **Static-shape proven specialization with identical-results fallback** | `kernel_mul_mv_mxfp4_pair_swiglu_static_impl` + the in-kernel `static_shape` branch | B− | Host proves `expert_in_dim == 4096 && row_bytes == 2176` (and the down twin: ne00==2048, nei0==6, ne0==4096) before selecting the pipeline → compile-time trip counts + literal strides + unrolled K walk; a defensive dynamic fallback lives INSIDE the kernel so a mis-selected pipeline "produce[s] identical results instead of garbage". Adjacent to B23 `hardcoded-fast-path-jit-specialization`, but the in-kernel identical-results fallback arm is the new content. |
| 7 | `row_blocks = row_bytes / sizeof(block)` hoisted block-index addressing | `kernel_mul_mv_mxfp4_f32_impl` etc. | C/drop at mine time | Likely dedups against the B85/B44 row-major/layout family. |

**Drop rows (transparency):** the sum6/slots6/addr/group6/group8/group24/table variant
families (dispatch plumbing around one quant kernel — ambient structure); the generated
LUT/grid constant tables (data, not decision content); GLM per-value scalar decode
helpers (`ds4_glm_q*_K_value` — correctness, no new decision content beyond existing
dequant rules); `kernel_mul_mm_id_map0`'s shmem-staged token scan (superseded shape).

## Dedup-gate evidence (both vocabularies, `src/domains/kernel_opt/` + `rust_perf/`)

- `cooperative_tensor|matmul2d|metal.?4|mpp_direct` → **0 hits** (novel).
- `tail_cull|cull|skip_mma` → **0 hits** (novel).
- `pair_swiglu|shared_b_tile|two_a_streams` → **0 hits** (novel; same-input family
  ships under launch-merge/weight-concat/quantize-memo names — none is the pair
  staging shape).
- `half_scale|exact_band|once_rounded|mxfp4` → **0 hits** (novel).
- `static_shape|fixed_shape|shape_prove` → **0 hits** (novel; B23 is JIT-specialization
  vocabulary, different concept).
- `expert_major` → LEAD, **read**: B94 scratch-bank rule — distinct mechanism, cite as
  adjacent.
- `stage-tile-pipeline` → LEAD, **read**: B92 cp.async multistage ring — adjacent
  (CUDA async-copy pipeline vs Metal4 cooperative-matmul barrier amortization); cite
  as adjacent, different substrate.
- `row_blocks|row_major` → hits in B85/B44 territory → candidate 7 likely dup, kept at
  C with a drop-at-mine-time prediction.

## GOAT lead test (candidate 1, against the four criteria)

1. **Benchable:** yes — our Q2_0 GEMV decode lane already carries f16-scale work
   (`group_scale_f16`, Bench 768 / Issue 764 T2); a half-table exact-band variant is a
   measured-GOAT shape on M3.
2. **Exact semantics achievable:** yes — the band argument is a once-rounding identity
   over pure power-of-two exponents; and the ported-numeric mandate applies: the
   exhaustive equivalence gate is 256×16 pairs (+ boundary rows) vs the
   f32-multiply-then-narrow reference — the source ships exactly this oracle shape.
3. **Magnitude story:** table bytes halve (64→32) AND the per-element f32→f16
   narrowing instruction leaves the GEMV inner loop — mechanism-level, not "probably
   faster".
4. **Deterministic:** table lookup + one band branch; same bits every call.

## Yield estimate

Lane 1 (moe.metal, verified): 3–4 rules after ~50% coordinator falsification
(candidates 1, 2, 3, 4; 5/6 as B− backup). Lane 2 (unverified files below): 1–2 more.
Batch 97 realistic total: **4–6 rules**.

## Honest caveats

- **Unverified files (re-read at mine time):** `metal/flash_attn.metal`,
  `metal/dsv4_hc.metal`, `metal/dsv4_kv.metal`, `metal/dsv4_rope.metal`,
  `metal/dsv4_misc.metal`, `metal/set_rows.metal`, `ds4_metal.m` (the host — large),
  `ds4_kvstore.c`, `ds4_ssd.c`, `ds4_layer_pack.c`, `ds4_streaming_hotlist*.inc`,
  `cuda/mmq/*`. This verdict stands on moe.metal full-read + structure + READMEs; the
  compressed-KV/flash lane could raise the grade.
- **Beta-quality, fast-moving:** pin the commit at mine time, quote verbatim at mine
  time (the WIP-snapshot rule).
- **Provenance split:** quant mul_mv/mul_mm kernels are llama.cpp-adapted (MIT, GGML
  copyright retained); candidates 1–6 appear DS4-original — verify absence in upstream
  at mine time before citing ds4 as the origin.
- **Non-Rust source:** fixtures stay original minimal Rust/CubeCL/WGSL reproductions
  (license hygiene + the binary-only rule; B92 ktransformers / B96 PrismML precedent
  for non-Rust GPU sources).
- **Candidate 2 is Metal4-only** → lands with an honest scope note (mechanism
  documentation; not runnable on M3 Max silicon), never cited as an applicable lever.
- The engine's streaming/SSD-expert-cache machinery (`ds4_ssd.c`,
  `ds4_streaming_hotlist*.inc`) is architecturally interesting for riir-neuron-db but
  is NOT a corpus lane — out of scope here; noted for the league/owner watch instead.
