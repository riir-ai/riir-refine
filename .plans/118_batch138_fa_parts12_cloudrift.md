# 118 — Batch 138: the CloudRift "Learning FlashAttention the Hard Way" Parts 1+2 distill

**Status:** LANDED 2026-09-09 — full Protocol A run in one session (the box window held: v8 absent, zero compute processes all cycle). 10 rules + 4 riders, corpus 591 → 601, commit `a68005e`; batch138 gates 3/3 FIRST-SHOT, retrieval floors byte-identical 7th consecutive, router UP 91→95/131 (no re-pin), score-bench #85 FLAT (8th). Research 158 (157 taken mid-session by the VeloxQuant sibling). Landing tail = this commit + `.raw/emmy138` rm.

## Source

- **Article (Part 1):** Dmitry Trifonov, "Learning FlashAttention the Hard Way —
  Part 1" (`https://www.cloudrift.ai/blog/learning-flashattention-the-hard-way-part-1`,
  2026-07-06). URL SHAPE: the queue's bare `/blog/learning-flashattention-the-hard-way`
  404s ("Article not found", verified live) — the real paths carry the `-part-1` /
  `-part-2` suffix (the B137 404 lesson, third strike). Full text fetched live
  2026-09-09 (this session); author/date verified.
- **Article (Part 2):** Dmitry Trifonov, "Learning FlashAttention the Hard Way —
  Part 2" (`.../learning-flashattention-the-hard-way-part-2`, 2026-07-08). Full
  text fetched live same session. THE SCOREBOARD: 25959µs scalar → 612 (tensor
  cores) → 511 (smem staging) → 315 (bank-conflict pad, 132M replays → ~0) →
  271 (C→A register repack) → 233 (Q hoist) → 222 (pipelining) → 219 (x4 drain
  pairing) → 211 (TMA) → **206 (register tiling = two independent softmax chains
  per warp) = FlashAttention-2 parity**, ~80% of the 5090 tensor roofline. FOUR
  REFUSED: warp-scoped barrier (~0µs), fast exp (~0µs, incl. the base-2 qk_scale
  fold), deeper rings (d3/d4), warp specialization (213 vs 206 — no wgmma on
  sm_120). FA-3 = Hopper (wgmma async MMA); FA-4 = B200 tcgen05+TMEM — neither
  silicon exists on consumer Blackwell.
- **Code surface:** `cloudrift-ai/emmy` — B137's clone @ `1c058c9d` already
  carried the fused attention kernel ("past FlashAttention-2"); tip HAS MOVED
  (ls-remote 2026-09-09: `bdd26c539933a54dd34af01d46ec43addaa49a03`) → fresh
  clone at the then-current tip per the pin discipline, license re-verify
  (B130/B137 recorded Apache-2.0), `.raw` discipline, rm -rf at landing. The
  `EMMY_KNOBS` codec (TILE/STAGE/REDUCE/PLACE) + `emmy compile -c "$SDPA" --
  target sm_120 --ir cuda` reproduce every listing.
- **Provenance:** blog cite-shape + repo tip dual-source (the B130/B137 shape);
  the blog carries the measured numbers (every rung) and the refusals; the repo
  carries the moveset. All fixtures original; quotes re-verified against the
  live article text already fetched in full + the clone at mine time.
- **One article per batch:** the matmul sibling was Batch 137; this plan
  consumes the PAIR (Parts 1+2) as ONE batch — they are one article series
  sharing one kernel (Part 1 = the algebra, Part 2 = the lowering), unlike the
  B137/B138 split which separated two independent articles.

## What it is (decision content — Part 1's algebra backing Part 2's measured ladder)

Part 1: **the twisted monoid**. Safe softmax looks non-associative (the max
depends on the whole vector); the online-softmax carrier `(m, d)` (Milakov &
Gimelshein 2018) merges associatively under the rescale combine
`m = max(m1,m2); d = d1·e^(m1−m) + d2·e^(m2−m)`, identity `(−∞, 0)`. Welford's
`(n, μ, M₂)` triple (Chan/Golub/LeVeque parallel form) is the same shape.
FlashAttention's `(m, d, o)` carrier is a **twisted monoid**: one coordinate
(m) acts on the addition of the others via `e^(Δm)` rescaling — transport of
structure ψ(m,D,O) = (m, D·e⁻ᵐ, O·e⁻ᵐ) over the plain direct-product monoid.
**3HT (Gibbons 1996):** if a loop computes both as foldl and foldr, an
associative combine EXISTS — the test for "when any loop is secretly
associative". Stability checked separately: exponents ≤ 0, d ∈ [1, N],
relative error ≤ (N−1)u/(1−(N−1)u); tiling never amplifies (each merge
rescales by e^Δm ≤ 1). Quantization is many-to-one → NOT a bijection →
transport does not apply; dequantization (affine) does. Base-2 twist:
σ = s·log₂e is affine → exp2 everywhere, free via qk_scale.
Part 2: the moveset lowers it (scoreboard above); the autotuner converges to
Dao's hand-picked constants (128 query rows, 4 warps, 2 m-atoms/warp; FA-2's
own comments document the manual sweep "line-for-line like the autotuner's
fork table"). TMA gives the LSU lead (3.31M vs 4.36M) but measures ~flat —
transport is not the bottleneck; the tensor pipe is (~80% roofline), which is
exactly why the fast-exp swap (an SFU-side move) measured flat and why FA-3/4
(asynchronous MMA + TMEM) stay locked behind datacenter silicon.

## Dedup pre-pass (T1, DONE 2026-09-09 — zero-build ERE grep over kernel_opt + rust_perf)

ERE discipline held (`grep -E` throughout — the B137 BRE false-empty lesson).

| concept | corpus state | nearest rule_ids (READ at mine time before ruling) |
|---|---|---|
| online softmax (m,d) merge | EXISTS | `online-softmax-rescale-carry` family (B79-era), `softmax-denominator-cancels-through-scale-invariant-norm` (B110), `batch-invariance-locks-reduction-geometry` (B110) — Part 1's algebra is the PROOF BACKING, not a new rule; expect the (m,d,o) split-KV combine to be coverage |
| Welford / variance | none needed | no GPU kernel in the workspace computes streaming variance — the Part-1 Welford section is explicitly a "side quest" (author's own words); note-level only |
| 3HT / secretly-associative loop | NO direct rule | zero hits for `secretly.?associative|homomorphism|foldl|foldr`; nearest = `interleave-independent-accumulators-only` (B125, order axis) — CANDIDATE (decision-content, NotBenchable): the loop-associativity TEST as compiler-recognizability |
| C→A register repack (cvt.rn.f16x2) | NO direct rule | `mma-fragment-layout-m16n8k16` (layout ATLAS), `ldmatrix-mma-layout-loads` — both cover FRAGMENT LAYOUTS, not the PRODUCER-C-to-CONSUMER-A in-register repack via lane-map alignment; zero hits for `repack|cvt\.rn|f16x2`; CANDIDATE (the 271µs rung, 1.16×) |
| loop-invariant operand hoist (Q) | partial | `same-input-folded-transform-memo-per-build` (B116, graph-build CSE), B137 register-tiling rule (register tier) — the Q hoist's decision content is the INDEX-AXIS test (operand index carries no stream axis) + FA-2's disabled-by-default counter-fact; CANDIDATE (233µs, 1.16×) |
| ldmatrix x4 drain pairing | COVERED (B137) | the B137 ldmatrix rule landed the x2→x4 fusion as its full rule — rider/confirmation only |
| smem slab pad vs hardware swizzle | EXISTS + composable pair | `vectorized-smem-write-no-bank-padding`, `xor-swizzle-smem-static-index-tables`, B130 `lds-cluster-cross-warp-phase-not-static-distance` — the FA pair (pad-when-hand-drained 1.62×, swizzle-when-TMA no-pad) is the SAME composable pair B137 recorded for the matmul article → rider |
| TMA / LSU relief | COVERED | `tma-single-issuer-bulk-tile` (B130), `cpasync-multistage-tile-pipeline` (B130 rider) — the FA facet (TMA-as-LSU-relief measuring ~flat when tensor-bound) → rider at best |
| warp specialization refusal | EXISTS + instance | B137's WS situational-payoff rule — the FA refusal sharpens it (WS needs ASYNC MMA/wgmma; sync mma.sync + WS = pessimization, 213 vs 206) → rider with the hardware-gate facet |
| deeper rings refusal (d3/d4) | COVERED | B130 `autotune-warps-stages` (triple-buffer same-IPC ceiling rider) — the FA negative (occupancy loss beats overlap at every geometry) → confirmation rider |
| fast exp refusal (~0) | partial | B130 `fast-math-contraction-behind-feature-flag` (fmad exact half), `hardcoded-fast-path-jit-specialization` — the FA negative's decision content is the LIMITER-DIAGNOSIS (SFU-side swap pays only when SFU-bound; tensor-bound → flat; FA-4's FMA-exp answers a datacenter-Blackwell imbalance) → CANDIDATE (NotBenchable, the measurement-refused class the snapshot names) |
| gmem-constant-for-graph-stability | NO direct rule | `warm-reachable-dispatch-signatures-before-timed-window` (B115, JIT signatures), `pipeline-cache-key-covers-every-function-constant` (B129, cache keys) — the FA-1 kernel comment ("all constants are passed via gmem to avoid CUDA-graph recompilation; the worst offender is seq_len") is a DISTINCT axis: per-input scalar args force graph re-capture → gmem indirection; CANDIDATE (note-shaped, M-category) |
| split-KV / Flash-Decoding (m,d,o) partials | EXISTS | `split-k-two-pass-attention-decode-long-context`, `target-tile-split-k-deterministic-two-dispatch` — the REDUCE=g4k deferred-combine (global row-max rescale, atomics-vs-kernel fork) is exactly this family; rider/confirmation only |
| two independent chains per warp | partial | `independent-accumulator-chains-hide-load-latency-at-capped-occupancy`, B137 register-tiling — the FA variant fans ONE streamed B-fragment across TWO independent (m,l,O) carriers (amortize the drain over chains, not loads); adjudicate rule-vs-rider at mine time after reading both neighbors |
| out-of-bounds prefetch clamp | weak | `extent-clamped-operand-views-partial-k-tile` (B128) adjacency; note-level |

Preliminary batch shape (rule-level dedup at mine time decides): 3-5 new rules
(c-to-a-repack; q-hoist-by-index-axis; fast-exp-limiter-diagnosis;
secretly-associative-loop-test; gmem-constant-graph-stability) + 5-8
riders/facets (pad-vs-swizzle pair; x4 pairing confirmation; WS-needs-async-MMA
hardware gate; deeper-rings confirmation; TMA-LSU-relief; split-KV combine
confirmation; two-chains fan adjudication). Route: **kernel_opt primary**
(entries_gpu_38.rs next), rust_perf io secondary only if a host-side candidate
survives dedup (expected: none — this is all device-side).

**Lane-intel axis: TO ASSESS at mine time — NOT auto-does-not-fire.** The
attention prefill kernel is the 4090 lane's own surface (Bench 896/897:
`att_pf_mq8*` attention family). The Q-hoist / C→A repack / two-chains moves
and the WS-needs-async-MMA gate are candidate intel for the 4090 prefill arm;
the FA-3/4 silicon-availability analysis is league intel (the league's Bonsai
lane runs on M3/4090 — both consumer). Record the verdict explicitly either way.

## Tasks

- [x] T0 — Plan-first (this file); both articles fetched FULL-TEXT live
      (2026-09-09), URL shapes corrected (-part-1/-part-2 suffixes), author/date
      verified. Repo pin: T2 (clone executes this cycle — window open, no box
      gate).
- [x] T1 — Dedup pre-pass over the CURRENT corpus (591 kernel_opt + 120
      rust_perf): table above; rule-level re-verification (READ each named
      neighbor) required at mine time BEFORE authoring — the pre-pass is
      file/grep-level, the B30 filter is load-bearing at rule granularity.
- [x] T2a — Fresh clone `cloudrift-ai/emmy` → `.raw/emmy` @ then-HEAD
      (`bdd26c53` at plan time; re-verify == ls-remote at clone), license
      re-verify; two parallel subagent read-lanes (A: Part-2 moveset surface +
      knobs codec; B: the refused-optimizations + FA-3/4 analysis + Part-1
      algebra candidates); proposals + adjudications into the Appendix;
      quotes re-verified at the pin BEFORE authoring.
- [x] T3 — Author entries (`entries_gpu_38.rs`, expected 3-6 rules) + batch138
      standing gates (`batch138_rules_present` / `batch138_concepts_covered` /
      `batch138_pseudocode_parses`) + count pins (entries.len ×3, category
      table, KERNEL_CATEGORIES slices + slice comments, mod.rs history + doc).
      rule_id-anchored edits throughout; python pin-phrase check + rustfmt-parse
      pre-validation before any compile claim.
- [x] T4 — GOAT: G1 batch138 gates by name; G2 lib default + kernel_heal +
      kernel_heal+latent counts; G3 clippy 0 ×3 all-targets + retrieval floors
      (kernel 100/100/100, blended 94.7/96.9/97.7) + router annotation
      re-measure per the Issue 020 contract (re-pin ONLY on measured decay,
      recorded never silent) + `--corpus-stats` + `--score-bench` #84.
- [x] T5 — rust_perf bench-feature Pareto/alloc validator for any rust_perf
      candidate (expected none); NotBenchable annotations + context_notes for
      decision-content rules (expected: the fast-exp limiter rule + the 3HT
      test rule — both measurement-shape, not M3-executable).
- [x] T6 — Count pins walk: corpus.rs header totals + category table + mod.rs
      lists + AGENTS ×2 + README ×2 + `.docs/11_domains/kernel_opt.md` (status
      row + Batch 138 ledger paragraph) + `.docs/11_domains/README.md` index;
      grep-verify zero stale prose counts remain.
- [x] T7 — Research note `.research/157_batch138_fa_parts12_distill.md`
      born-complete: both URL shapes, the scoreboard table, the four refusals
      as recorded negatives, lane-intel verdict with the per-rule split,
      blog-vs-repo honesty, the full gate row; `.highwater` 156 → 157.
- [x] T8 — Landing sequence: commit per Protocol A (T3 authoring → T4 gates →
      T6 docs → plan+snapshot+Research 157 → score-bench → plan close +
      snapshot LANDED + `.raw/emmy` rm). Queue snapshot updated in the same
      commit as each landing step.

## Appendix — T2a subagent proposals

(filled at T2a)
