# Research 316 — Batch 277: the Humming + Chord quantized-GEMM pair (kernel_opt corpus, Plan 281)

**Status:** RECORD — landed 2026-10-25 (shikuwa/4090, riir-refine idle loop; the claim
`aa52ff31b` discharged by the landing row).

## Sources (both staged 2026-10-25, converted same day)

1. **vllm-project/humming @ `db930afa`** (Apache-2.0; announced 2026-10-21) — the unified
   quantized-GEMM kernel library, JIT-compiled. Licence chain verified at T0b: root LICENSE
   blob `57bc88a15a0e` == the staging read exactly.
2. **novitalabs/chord @ `6199d44a`** (Apache-2.0; Novita Labs, W4A16 MoE CUDA operators for
   the Kimi K2.5 family). Licence chain verified at T0b: root LICENSE blob `261eeb9e9f8b`
   size 11357 == staging; vendored DeepGEMM at
   `chord_kernels/operator/include/deep_gemm/LICENSE` = MIT (DeepSeek 2025); CUTLASS
   submodule @ `59b61c60` read via raw.githubusercontent = BSD-3-Clause (SPDX line verbatim).
   CHAIN CLEAN.

Pins resolved at T0b (plan 281): chord main tip `6199d44a863a` == the staged tree sha EXACTLY
(zero drift); humming pinned at main tip `db930afa` (4 commits past the benchmark pin
`7ec54ed` — heuristic/CI/dtype-cache drift only; chord's own performance.md discloses the
b4f3fc6..tip window "within 1.6%"/GPU-unchanged).

## Landing: corpus 955 → 962 (entries_gpu_125; D +1, E +1, K +2, L +2, N +1)

SEVEN own rules (all NotBenchable-here — the measured cells are H200/B300, ours SM89; the
fixtures ARE the CPU-testable oracles):

| # | rule | category | one-line law |
|---|---|---|---|
| 1 | `zero-exponent-float-mantissa-is-the-format-pivot` | N | an E0 float's mantissa IS a sign-magnitude integer — int→float routes through the integer path, float→int and fp zero-points REQUIRE an E0 source, E0 sources get +1 mantissa of coverage at BOTH layers, the branchless fold IS the exact dequant |
| 2 | `undocumented-fp-dtype-cubin-patch-per-sm-verified` | K | the PTX type table is NOT the hardware capability boundary — substitute-and-patch reach, per-SM hardware-verified allowlist RECORDING its refusal (SM107 illegal instruction), two-key gate, width rule, the INT8-UMMA `(100,110)` set partitions the same family differently |
| 3 | `swap-ab-decode-operand-roles-tile-legality` | L | weight on mma-M, tokens on mma-N — a few-token block is ONE m16n8k16, block-M 8 legal; dequant-once-reuse; the config validator is where the legality lives |
| 4 | `one-pack-serves-all-schedules-nibble-order-contract` | L | pack ONCE in the consumer-canonical fragment order — "Only the shift immediates differ, so both orders cost the same instructions"; one packed copy serves WGMMA prefill + swap-AB MMA decode + grouped operators |
| 5 | `stream-k-tail-sequenced-deterministic-lockword-reuse` | E | a deterministic stream-K tail is SEQUENCED, not atomized — serial chain ≤3 vs counter, k=0 adds in CTA order and rounds ONCE, FP32 partials buy cross-launch determinism at a measured cost, both protocols leave the lock word at 0 (the GPU face of the katgpt-rs Issue-940 law, named in-rule) |
| 6 | `semi-static-token-tiles-populated-count-guard` | D | guard only the possibly-empty tiles — j=0 is unconditional (the predicate-free anchor); the populated count from a sentinel row-index; correctness rides the epilogue re-mask |
| 7 | `expected-load-headroom-kdepth-split-spill-asymmetry` | K | expected-load headroom with the K-depth split — loose 1.3× on deep-K (a spill re-reads the whole K), tight ceil(1.25×, 8) on small-K (the redundant pass is cheap) |

WATCH ROW (negative-assert pinned in the batch gate): the humming support-matrix platform
rows (FP4 e2m1 activation SM120+, README:30; INT8 SM75+, README:31).

## The two staging upgrades this session found over plan 281's T3-prepass

1. The hardware-intel WATCH was upgraded to the FULL rule 2: `mma.py:42-47` carries the
   cubin-patch mechanism IN SOURCE (the staging knew only the blog-level "absent from the
   PTX type system"), including the per-SM verified allowlist and the SM107 refusal record.
2. The expected-load headroom rider became the FULL rule 7: `heuristics.py:58-73` carries a
   K-DEPTH SPLIT of the headroom via the spill-cost asymmetry — a decision axis the
   design.md summary did not surface.

## Method + gates

- 3 parallel authoring subagents over disjoint slices (humming pair / chord swap-AB+one-pack
  / chord stream-K+semi-static+headroom); every quote verbatim at the pins (the T2 15/15
  pass + the coordinator's 28-needle pass at landing); the DEDUP greps per candidate at
  authoring (zero owners each; nearest-kin named + distinguished in-rule).
- Fixtures: ORIGINAL minimal Rust, no string literals (the B276 law), compiled standalone
  (rustc --emit=metadata ×14) AND test-RUN pre-assembly — running caught 3 real fixture bugs
  an E0614 deref, a tuple destructure, a vacuous cost-model comparison) + 1 quote-accuracy
  fix (the FP32-vs-BF16 Δ is −2.3..+2.4%, three down-stage rows measured FP32 faster — not
  the "+0.1..+2.4%" the brief carried).
- Gates ALL GREEN first-run: batch277 needles + parse 2/2; kernel_opt lib 3223/0; default
  lib 2729/0. Count pins re-pinned 955→962 (tests.rs sites + router.rs + the Plan-003
  category table + category headers + AGENTS ×2 + README head + `.docs/11_domains`).
  Retrieval floors NOT owed (no retrieval-affecting surface moved beyond the corpus add —
  the floors are pinned to the retrieval channel, re-measure owed only if moved; the
  kernel_opt-only delta rides the P30/B250 precedent for freeze/score-bench; binary rebuild
  NOT owed — kernel_opt outside the release set).
- Riders: the T3-prepass six (rounding-order family, gpu_09 ×2, gpu_17, gpu_21, gpu_43)
  DEFERRED to a rider commit under the same claim (plan 281 T4 `- [-]` disclosure).

## Notes

- Research numbering: RENUMBERED 315 -> 316 at the B278 merge (2026-10-25, zed-m3-b278): the M3's Locals staging (7fa36a9b7) allocated 315 FIRST by push, and this landing's own anticipation ('renumber at their landing') is hereby discharged — first allocation keeps, the loser renumbers. The original line: the M3's Batch-278 staging row (7fa36a9b7) anticipated "Research 315"
  for B278 in prose; the highwater at this landing read 314, so 315 allocates HERE per the
  numbering law (value+1; prose is not an allocation — the B267-consumed-302 precedent).
  The B278 lane renumbers to 316 at its own landing (a one-token prose fix in its staging
  row).
- `.raw/b277/` clones removed at close per the plan-281 T7 law.
