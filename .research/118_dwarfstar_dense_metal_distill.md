# Research 118: Batch 98 — antirez/ds4 dense.metal/moe.metal distill

**Status:** RECORD — 10 rules landed, corpus 435 → 445 (A 99→100, B 55→57, C 38→39, D 94→95,
E 28→30, H 15→16, L 12→13, M 12→13); GOAT all green; 4 riders on existing rules.

Batch 98 / Plan 065. Source: antirez/ds4 (`DwarfStar`) @ `110afdd8886586f18fc9b28bc5533152dd10e728`
(the same pin Batch 97 verified; clone `../vendor/ds4`). This batch consumes the leads Batch 97
explicitly queued in the mining queue snapshot ("NEXT BATCH LEADS ... dense.metal — DS4-native
matvec family") plus the two P8-intake candidates, plus three additional candidates found while
reading. Every quote re-verified byte-accurate at mine time (T3 coordinator pass) against the
pinned commit; anchors carry file + line. dense.metal is DS4-native (not llama.cpp-lineage) per
the Batch 97 provenance work.

## The 10 rules

| # | Rule | Cat | Mechanism (one line) | Source anchor |
|---|------|-----|----------------------|---------------|
| 1 | `grid-range-split-verbatim-impl-composition` | C | K same-impl-family sibling dispatches fuse into ONE kernel via a grid split at the first range's own boundary (tgs0 = ceil(width0/NR0)), re-based group ids, a BY-VALUE patched args copy, and a device-scope barrier before any epilogue reload | dense.metal L1097-1105, L1130-1139, L1141-1151, L1155+L1167-1169, L1197+L1341-1400 (5-way) |
| 2 | `share-activation-stream-keep-projection-reductions` | A | Same-activation matvecs share the activation WALK but keep per-stream accumulators/scales/reduction shapes verbatim — the non-merge boundary (SwiGLU same-row epilogue co-location) | dense.metal L325-330, L201-204, L379-392, L448-449 |
| 3 | `tiny-rows-scalar-thread-per-row-downshift` | E | K < warp width → one thread per output row, serial K, zero smem/barriers; host gate `in_dim < 32` | dense.metal L1440-1441, L1410-1416+L1431-1437; ds4_metal.m L4988-4994 |
| 4 | `small-batch-matvec-gemm-bridge-rung` | M | The GEMV↔GEMM middle rung: 2D in-simdgroup lanes (K lanes × row lanes), dequant-once dot-all-register-tokens, partial-lane shuffle reduce, no smem | dense.metal L1596-1597+L1696-1697, L1609-1617, L1652-1657; ds4_metal.m L19618, L18042-18044, L18477-18486 |
| 5 | `moe-routed-down-sum-single-accumulate-pass` | H | Routed top-k fan-out whose outputs are SUMMED: k dispatches + combine → ONE kernel, per-slot register accumulation, ONE final cross-lane reduction per row; tolerance-gated (NOT bit-identical) | moe.metal L6378-6391, L6393-6403, L552-573 (combine eliminated), L2626-2628 (TP addend) |
| 6 | `fused-vector-site-transcendental-form-nonportable` | B | Hardened scalar transcendental forms do NOT port to fused vector sites: measured — tanh-identity sigmoid produced non-finite logits at fused float4 sites on M5 Max while direct exp stayed finite; inline the validated form AT fused sites | dsv4_hc.metal L86-108, L142, L316+L326 |
| 7 | `monomorphic-hot-caller-last-hit-memo` | D | Hot cache-getter with per-layer-identical args: a single-slot last-hit memo in FRONT of the generic cache (miss falls through + back-fills) replaces key formatting + dict probe with two scalar compares | ds4_metal.m L3076-3081, L3082-3090, L3213-3223, L3091-3094; four sites |
| 8 | `per-row-scale-folded-into-rhs-staging-tile` | B | Per-row runtime scale folds into the existing F32→narrow RHS staging boundary (one fused multiply before the narrow store); NOT bit-identical to scale-then-matmul — the lossy-surface rule | dense.metal L2242-2246, L2299, L2356 |
| 9 | `dequant-byte-pairs-as-aligned-u16-loads` | L | Byte-pair dequant loads as aligned u16 (8 loads replace 16) when alignment is PROVEN from struct layout; a load-width twin, bit-identical per element | dense.metal L1542-1547, L1548-1561 |
| 10 | `cross-simd-bitonic-alternating-bank-exchange` | E | Cross-simdgroup bitonic exchange through parity-selected double-buffered TG banks, ONE publish barrier per cross stage, no read barrier — the stage-distance safety proof | dsv4_misc.metal L4988-4993, L5026-5043 |

## Mine-time corrections (the falsification work)

1. **"Args aliasing" corrected** (Lead 1): the queue-snapshot framing said "per-range aliased
   args struct" implying pointer-casting — it is a **by-value copy** of the shared args struct
   with two scalar fields patched (`nr0` pinned to the template constant, `ne01` set to the
   active range's width). The pair impl takes args by value, which is what lets the identical
   body serve both the standalone constant-args caller and the fused local copy. Encoded in-rule.
2. **matvec-ext doc drift** (Lead 4): the dense.metal comments say "2..8"/"2..5" tokens, but the
   live host gate is env-tunable (`DS4_METAL_Q8_MV_EXT_MAX_TOKENS`, default 16, range 2..128)
   with `r1ptg=4` token batching above 8, and Q4 instantiates `r1_1` (n_tok==1 uses ext for Q4).
   Encoded as the "never hardcode the comment's range as the regime" clause.
3. **M5 sigmoid hazard is INVERTED** (candidate 7): the P8-intake framing ("stable-sigmoid-vs-exp
   annotation") read as "the naive form needs hardening" — the measured contract is the opposite:
   the tanh-identity (the "hardened" form) breaks at fused float4 sites on M5 Max while the
   "naive" direct exp form stays finite and is the throughput baseline. Also located precisely:
   the tanh form exists ONLY in dsv4_hc.metal L86-108 (grep-verified absent from ds4_metal.m,
   dense.metal, dsv4_misc.metal, moe.metal). Encoded as "encode the observed behavior, not an
   assumed mechanism".
4. **Pipeline memo corrected** (candidate 6): the memo does NOT memoize pipeline CONSTRUCTION
   (the generic `g_pipeline_cache` already does that) — it memoizes the LOOKUP (NSString key
   materialization + dictionary probe). Four sites, not one (L3073/3132/3191/3285). This is what
   makes it cleanly distinguishable from `tiny-param-buffer-rebuild-over-cache` (B39): the
   payload is an expensive GPU object and the avoided cost is the lookup — the complementary end
   of B39's cost model, neither subsumes the other.

## Riders (4, on existing rules)

- **B97 `bit-exact-dispatch-fusion-boundary-emulation`** — the SCOPE-EXCLUSION half: a fused
  stage whose output is denominator/order-sensitive and feeds L ≥ ~10 stacked consumers must be
  excluded from mechanically-free fusion unless the prior dispatch's rounding boundary is pinned
  (the 43-layer MoE router: dsv4_misc.metal L4874-4881 exclusion, L5065-5069 + L5160-5163 pinned
  weight/transform rungs). Two more external instances of B97's own mechanism ride along.
- **B35 `mask-sentinel-matches-reduction-identity`** — the POINTER-STATE complement: ragged
  paired kernels point out-of-range lanes at the matrix BASE as a safe sentinel so one shared
  loop body serves valid+invalid lanes; reductions gated by active flags incl. skipping the dead
  arm's reduction (dense.metal L233-239 + L288-289).
- **B32a `sign-branch-stable-sigmoid`** — the LINEAR-TAIL member: softplus's overflow domain
  removed branchlessly via `select(log(1+exp(x)), x, x > 20)` (dsv4_misc.metal L5178-5183);
  threshold = accuracy+domain joint bound (x~20 tail vs x~88 overflow), re-derived per function.
- **B97 `proven-shape-static-sibling-identical-results-fallback`** — EXTERNAL CONFIRMATION: the
  sum6 static sibling re-proves the rule in a second engine (moe.metal L6498-6503: host-proven
  literal shape, byte-identical walk order, identical-results defensive fallback).

## Dropped with reasons

- Lane A's virtual-simdgroup cohort packing (dense.metal L559-561 `cohort = sgitg >> 2`) —
  the time-slice/pack-two-legacy-groups form already covered by B97
  `virtual-thread-folding-preserves-reduction-tree`. DROP-duplicate.
- Host-side shape-keyed `nr0=4` override (ds4_metal.m L19590-19593) — a
  `hardcoded-fast-path-jit-specialization` (B23) instance. DROP-duplicate.
- Lane A Additional 3 (sentinel-pointer ragged pair participation) — rider-scale, landed as the
  B35 rider above instead of a standalone rule.
- Lane B A3 (softplus select guard) — rider-scale, landed as the B32a rider above.

## GOAT (all green)

- **G1**: `batch98_rules_present` + `batch98_concepts_covered` + `batch98_pseudocode_parses`
  (cfg(not(kernel_verify)) per B70) — 3/3. All 10 ref/opt shapes parse.
- **G2**: kernel_heal lib **1255/0/1i** (Batch 97 baseline 1252 + 3 new gates; ALL count pins
  green — corpus-count, expected-array, sums, grouped, router expert-total); default lib
  **1008/0** (unchanged — no default-visible tests added).
- **G3**: clippy `-D warnings` clean in BOTH states (default + kernel_heal); retrieval floors
  **14/14 HELD** (no re-pins, ~307 s); score-bench run #29 **96.2/1.9/100/100 FLAT** (the batch
  is kernel_opt-only; the clippy --fix path untouched).
- **Gate-catch**: one case-mismatched needle in `batch98_concepts_covered`
  ("the threshold derives..." vs the concept's sentence-initial "The threshold derives...") —
  the B51 case-sensitivity lesson, 11th strike. Fixed the needle (gate doing its job).

## Count pins walked

tests.rs corpus-count 435→445 (+ message text + the Batch 98 history line) · expected-array
A/B/C/D/E/H/L/M (8 categories — the pin's per-category deltas) · sums x2 + grouped total x2 ·
router.rs expert total 435→445 · mod.rs "435-rule corpus" doc line + 8 category-header counts +
`mod entries_gpu_13;` + the extend call.

## Scope notes

- Lead 3 (tiny-rows downshift) carries a bench-note caveat: the source ships no in-file perf
  numbers for the short kernel — the win argument is structural; bench-pin before any promotion.
- No CI full-guard run this cycle (the B95 honest-skip shape): the three affected lib states +
  floors + pins + score-bench all validated individually.
- Fixture purity: fixtures are original minimal Rust reproductions (parse-only), per B77's
  landing lesson that the bench compile is the only type-check new snippets get — all 10 shapes
  passed the syn gate on first run after the needle fix.
