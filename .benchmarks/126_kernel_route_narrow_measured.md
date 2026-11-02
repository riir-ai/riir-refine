# Bench 126 — the route-narrowed kernel index: the G5 cost-leg repair MEASURED (Issue 196 R2+R3)

**Status:** CLOSED — verdict recorded; the narrowed design is MEASURED NEGATIVE on the cost axis and stays opt-in (never default); the G5-cost fence stays up, the next lever is named and it is NOT narrowing.

## The pre-registration (Issue 196 / Issue 191 T1)

- Ceilings (Issue 191 bar 5, UNCHANGED): added ON−OFF scan latency p50 ≤ 1.0 ms/file, p95 ≤ 5.0 ms/file.
- Utility floor (Bench 125): 15/15 category experts own ≥ 1 rank-1 kernel suggestion.
- R3's honest-verdict clause: "including the possibility that 15× narrowing alone does not reach 1.0 ms and the flag stays opt-in with the measured residual named."

## What landed (R1, commit 06217f8f)

`kernel_route_narrow` (opt-in, implies `kernel_opt` + `latent_retrieval`) — the kernel domain's matcher index as **15 per-category sub-indices + a stride-sampled global tail** (`ROUTE_TAIL_RULES=128`), every query (whole span + each significant line) retrieving its span's **top-6 routed slices + tail** (`ROUTE_TOP_K_EXPERTS=6`, the measured sweep point). Routing = TF-cosine lexical (`SpectralVocab` at category granularity, `Discriminative` order); line queries inherit the span's routed set; OFF posture byte-identical. Shared primitives (`retrieve_scored_into`, `merge_retrieved_best`, `finalize_ranked`, `significant_lines`) extracted in corpus_index.rs as pure motion — the flat path untouched.

### The two measured refutations inside R1 (both recorded in route_narrow.rs docs)

1. **The issue's option (b) — routing on the existing 8-D span embedding — REFUTED before it could ship:**
   kernel-domain top-1 fell **100% → 16.7%** (blended 90.1% → 75.4%) when routing rode
   `ModellessEmbedder`-direction·category-centroid dots. The 8-D embedder is BLAKE3+DFT+sigmoid — a
   hash-space signal, near-orthogonal across distinct texts (the Issue 020 record's own finding) — so
   15-way category routing on it is ~random: 1/15 slice hit + the 12% tail ≈ the observed 16.7%.
   Pivot: the TF-cosine lexical router (the Issue-020-proven primitive, `DomainRouter` machinery one
   granularity down; `SpectralVocab::build_from_patterns` generalized for the DomainId-free call).
2. **The single-expert slice — REFUTED by the K-sweep** (the R1 diagnostic on the retrieval-precision
   gate's own 30 renamed self-variant spans; `diagnostic_routing_accuracy_on_gate_fixture_spans` +
   `diagnostic_k_sweep`): end-to-end top-1 13/30 (K=1) → 20/30 (K=3) → 27/30 (K=5) → **29/30 (K=6)**.
   Router accuracy: top-1 9/30, top-3 18/30. K=6 is the FIRST point holding the repo's
   retrieval-precision gate floors **UNTOUCHED** — kernel top-1 96.7%, blended 89.5/93.0/94.7% vs
   floors 87.2/92.2/93.6% — the gate runs UNCHANGED at the narrow posture (no eval pin moved; the
   pins are the quality contract and the narrow posture MEETS them). K=5 missed blended top-3 by
   ONE span (157/171 vs 130/141) and was rejected on the convention, not re-pinned.

## R2 — the census re-run at both ON postures (the G5 harness + the extended three-posture runner)

Harness: `examples/kernel_goat_g5.rs` (unchanged walk; +cfg-derived `posture` label + per-span
`rank1_rule_ids` emission), runner `scripts/g5_utility_cost.sh` (now THREE postures + paired
recall-delta disclosure). Corpus: this repo's src/ (566 files, sorted walk), top_k=5, medians of
3 timed passes after 1 warmup pass. Same binary walk, three postures.

| census | flat (kernel_opt) | narrow (kernel_route_narrow) |
|---|---|---|
| kernel-cited spans (last pass) | 4,063 | 4,063 (0 lost) |
| distinct rank-1 experts | **15/15** | **14/15 (LOST one)** |
| rules cited ≥ once | 750/1051 | 728/1051 |
| zero-match rules | 301 | 323 |
| rank-1 flips vs flat | — | 2,426 / 4,063 spans (59.7%) |
| cited-set shrinkage | — | 100 rules cited flat, never narrowed |

The per-expert rank-1 tables (flat → narrow): expert 9 dropped out entirely (13 rank-1 spans flat →
0 narrowed); the full tables are in the run's stdout (jsons preserved in the verdict files below).

## R3 — the cost verdict at the SAME pre-registered ceilings

| posture | added p50 ms/file | added p95 ms/file | files over 1.0 ms | verdict |
|---|---|---|---|---|
| flat (kernel_opt) — the Bench-125 reference, re-measured | 36.346 | 171.152 | 463/566 | FAIL |
| **narrow (kernel_route_narrow, K=6+tail)** | **45.849** | **219.815** | 479/566 | **FAIL — 26% SLOWER than flat at p50** |

Box state: Windows (MINGW64_NT-10.0-26300), 24 logical cpus, 4090 workstation idle, AC power; the
walks are the harness's own 3-pass medians (medians absorb transient load). The flat reference
reproduces Bench 125's 38.795 ms p50 within box-state variance (36.3 vs 38.8, −6%).

## The mechanism (why narrowing LOST to flat)

Per-query the narrowed path issues **7 retrieve calls** (6 slices + tail) vs flat's **1 call over the
full corpus**, and each call retrieves up to latent_k=16 + bm25_k=16 candidates
(`DEFAULT_LATENT_K`/`DEFAULT_BM25_K`, riir-rag retriever.rs) — so the narrowed path returns ~7× the
candidate volume per query and pays the substrate's per-call fixed costs (query construction,
candidate assembly, per-call HashMap/Vec) 7 times. At this corpus size (~1051 rules, ~5k chunks) the
full-corpus N-term the narrowing cuts is ALREADY cheap relative to per-call overhead × call
multiplicity — narrowing by corpus mass loses to call-count multiplication. The issue's ~15×-rule-mass
plausibility argument assumed per-query cost scales with scored mass; the measured cost scales with
**call count × per-call floor**, and the K that quality demands (K≥6 to hold the gate's floors) drives
the call count up faster than the mass drops.

## Verdict (R4 — the promotion-path decision, applied)

- **kernel_route_narrow stays OPT-IN, never default** — the GOAT gate FAILS the cost leg (G2-class)
  at the pre-registered ceilings, and the utility floor (15/15) is lost at 14/15. Promotion denied by
  the pre-registered numbers; no ceiling re-registration is requested (the measured residual is not a
  near-miss: it is 46× the ceiling at p50 and moves the WRONG direction vs flat).
- **The G5-cost fence stays up** (Bench 124 §Next): the re-campaign remains fenced on the cost leg,
  and the fence note now names the two levers narrowing did NOT test — they live outside this repo's
  cut-by-mass shape:
  1. **Fewer line queries** — the multi-scale merge's ~20× query mass is the cost driver; any lever
     there (e.g. a per-line short-circuit when the span-level pool already carries the line's best
     rule) changes OUTPUT composition and is a Bench-gated quality trade, not a retrieval-index shape.
  2. **A substrate-level score-only retrieval mode** — one retrieve call over the full corpus WITHOUT
     the per-call content/packing floor (the two-stage prefilter alone, or a fused multi-zone query).
     That is riir-rag substrate work (riir-neuron-db's lane; the "composes riir-rag as-is, nothing
     here reimplements retrieval" boundary law) — the follow-up belongs THERE, filed as
     riir-neuron-db's candidate, not implemented behind a refine-side flag.
- The narrowed index's one measured WIN: **matcher build 4,457 ms (flat) → 1,589 ms (narrow)** — 2.8×
  faster index construction (16 small indices vs one full pass with multi-scale line hashing).
  Irrelevant to G5 (build is once per process), recorded for the ledger's honesty.
- The flag stays in the tree for replication (the `mattr_k_poc` measured-negative precedent), with
  its standing docs stating the negative verdict.

## What survives the negative (landed regardless — commit 06217f8f + this commit)

- The corpus_index shared-primitive extraction (pure motion; the flat path byte-identical — 2,799
  default-posture tests + 3,554 narrow-posture tests green, clippy clean at default/narrow/flat/combos).
- `SpectralVocab::build_from_patterns` (DomainId-free vocab construction; `build` delegates).
- The harness's `rank1_rule_ids` paired disclosure + the three-posture runner (standing G5 instruments).
- The two measured refutations themselves — the option-(b) hash-space routing result and the K-sweep —
  are standing evidence for any future routing/index work on this domain.
- P43 repair: `kernel_goat_g5` example gained its required-features row (the Bench-125 landing's miss
  — default-feature `clippy --all-targets` was red on the example since 41f2ae4f).

## Reproduction

```sh
sh scripts/g5_utility_cost.sh --root src --out-dir /tmp/g5_196
# OFF  = latent_retrieval            -> g5_off.json
# flat = latent_retrieval,kernel_opt -> g5_on.json
# narrow = kernel_route_narrow       -> g5_narrow.json
# the script adjudicates both ON−OFF diffs at the same ceilings + prints the R2 recall delta
cargo test --release --features kernel_route_narrow --lib retrieval_precision_regression_gate
cargo test --release --features kernel_route_narrow --lib diagnostic_routing diagnostic_k_sweep -- --nocapture
```

Run date 2026-11-02; develop at 06217f8f + this commit's records; jsons at /tmp/g5_196 (volatile).
