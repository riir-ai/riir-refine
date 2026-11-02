# Issue 196 — wire the category-expert router into the kernel_opt matcher index (the G5 cost-leg repair path)

**Status:** CLOSED MEASURED-NEGATIVE (Bench 126, 2026-11-02) — R1 landed + measured; the narrowed
index FAILS the pre-registered ceilings (and is SLOWER than flat: added p50 45.8 ms vs flat's
36.3 ms vs the 1.0 ms ceiling) and loses the 15/15 utility floor (14/15). The flag stays opt-in,
never default; the G5-cost fence stays up with the next lever named (substrate-level score-only
retrieval, riir-neuron-db's lane). Lease closed: zed-riir-g5 epoch=1793591179 (claimed
2026-11-02T16:26Z, expires 2026-11-04T16:26:19+00:00 — closed 2026-11-02 well before expiry).

## What happened

Bench 125 measured the G5 utility/cost leg (Issue 191 bar 5, the Bench-124 disclosed
deferral). Utility PASSes decisively — all 15 category experts own at least one span's
rank-1 kernel suggestion over 4,055 kernel-cited spans — but the cost leg fails by 39× at
the median: adding the kernel domain to the scan multiplies per-file latency from a
27.8 ms p50 baseline to 66.9 ms (added p50 38.8 ms, p95 177.6 ms; 463/565 files over the
1 ms ceiling).

The mechanism is not mysterious: the kernel domain's L0 retrieval scores every span chunk
against the FULL 1051-rule corpus — the largest rule mass of any domain (~439 rules in all
other default domains combined). The `KernelExpertRouter` (15 category experts, one per
`KernelCategory`, 15/15 reachable since Plan 004) exists precisely to narrow this — its own
module docs say "consult category-relevant rules first without scanning all" — and the
`kernel_opt_demo` example routes signatures through it. But the production scan path
(`LatentMatcher::suggest_all_domains`) indexes the kernel domain as ONE flat `RuleIndex`
(`index_domain!(kernel, ...)` over `KernelOptDomain::build_corpus()`) and fans every span
out to all 1051 rules. The router narrowing was never wired into the matcher.

## The repair shape

- Narrow the kernel domain's retrieval inside the matcher: route each span chunk to its
  expert (the activity vector comes from the span's code — the same constructor
  `build_activity` the demo uses; the exact feature extraction for code spans is the design
  work) and score only that expert's category rules (~70 rules/expert, a 15× cut of the
  scored mass), then merge with a small residual slice from the global corpus (the
  router's own "consult category-relevant rules first WITHOUT scanning all" phrasing allows
  a bounded global tail — pick a constant, disclose it, pin it).
- Behind a feature flag (`kernel_route_narrow`, opt-in), the match-census discipline:
  OFF posture byte-identical (the G3 disclosure boundary), ON posture changes retrieval
  composition — the rank-k candidates differ, so Bench 125's census re-runs at both
  postures and the RECORD discloses the recall delta (the utility leg must still pass:
  15/15 experts at rank-1 is the floor to hold).
- Success bar: re-run `scripts/g5_utility_cost.sh` at the SAME pre-registered ceilings
  (p50 ≤ 1.0 / p95 ≤ 5.0 ms added). A ~15× rule-mass cut plausibly lands p50 near 2.6 ms —
  still over 1.0 — so the flag likely ALSO needs the per-span embedding work deduplicated
  (the span embedding is computed once per span already; the 15× is the scoring fan-out)
  AND possibly a cheaper kernel embedder tier. Measure first at the plain narrowing before
  adding tiers; the ceiling re-registration is an OWNER act (Issue 191 T1), never assumed
  here.
- The activity-vector extraction from arbitrary Rust spans is the one genuinely new design
  surface: `build_activity` today consumes a kernel SIGNATURE (M/N/K/dtype/hw + op-class
  flags) provided by the consumer. Code spans don't carry signatures. Options: (a) a
  heuristic extractor over the span text (tensor-shaped identifiers, loop nesting, dtype
  names — disclosed, deterministic); (b) reuse the span EMBEDDING the matcher already
  computes (the tf-cosine/latent vector) and route on a per-expert projection of it —
  no new extraction, one matrix multiply. Option (b) is the substrate-first choice: the
  matcher computes the span embedding for scoring regardless; routing on it adds one
  dot-product per span.
- **THE DOMINANT COST IS THE LINE-QUERY LOOP, not the whole-span queries** (measured
  2026-11-01, this repo's src/: 342,876 lines, 248,223 significant (>8 chars) ≈ 72%):
  `suggest_deep_into` issues one full-corpus retrieval PER SIGNIFICANT LINE (the
  multi-scale merge — "a whole-function span whose interior line instantiates a rule still
  surfaces that rule"), so the query mass is ~20× the span count. The narrowing must
  cover the line queries (route LINE queries to their CONTAINING span's routed expert —
  the lines exist to refine the span's own scoring, so the span's expert is the right
  scope; a per-line re-route would re-embed 248k queries for nothing), or the first lever
  is cheaper line queries by construction. Retrieval internals: the retriever owns the
  packed chunk embeddings (`retrieve_for_scoring` over `by_zone_hash`'s corpus); a
  per-category SUB-RETRIEVER slice (15 + one global-tail) is the likely shape — the
  embeddings are built once at `build_with` and the split is index-structural, not
  per-call. G4 discipline: scratch capacities stabilize, no per-call allocation; the
  OFF-flag posture stays byte-identical (the G3 disclosure boundary).

## Tasks

- [x] R1 — the route-narrowed kernel index behind `kernel_route_narrow` (commit 06217f8f): the
      option-(b) embedding-dot routing was implemented FIRST and REFUTED by measurement (kernel
      top-1 100% → 16.7% ≈ random — the 8-D ModellessEmbedder is a hash-space signal, the Issue-020
      finding); pivoted to the TF-cosine lexical router (`SpectralVocab::build_from_patterns`,
      `Discriminative` order) at category granularity. The single-slice shape was ALSO refuted by
      the K-sweep diagnostic (13/30 at K=1 → 29/30 at K=6); K=6 + tail (ROUTE_TAIL_RULES=128) is
      the first point holding the retrieval-precision gate floors UNTOUCHED (no eval pin moved).
      Full disclosures in `src/domains/kernel_opt/route_narrow.rs` module docs.
- [x] R2 — the census re-run at both postures (Bench 125's harness + the rank1_rule_ids paired
      disclosure): utility floor LOST at the narrow posture — 14/15 distinct rank-1 experts (expert
      9 dropped out), 728/1051 cited (−22), 2,426/4,063 rank-1 flips, 0 spans lost. Recorded in
      Bench 126 §R2.
- [x] R3 — the cost re-run at the same ceilings, the honest verdict recorded (Bench 126): added
      p50 45.849 ms / p95 219.815 ms vs the 1.0/5.0 ms ceilings — FAIL, and 26% SLOWER than the
      flat reference at p50 (the 7-calls-per-query × per-call floor beats the ~2× mass cut; the
      mechanism section is in Bench 126). The 15× narrowing plausibility argument was wrong about
      the cost model, and quality demanded K≥6 (≈52% mass) anyway.
- [x] R4 — the promotion-path decision, applied per the pre-registration: the ceilings do NOT
      pass → the flag stays opt-in, promotion DENIED, no ceiling re-registration requested (the
      residual is 46× the ceiling and moves the wrong direction), the re-campaign stays fenced
      (Bench 124 §Next updated), and the next lever is named OUTSIDE this repo's shape:
      substrate-level score-only retrieval (riir-rag, riir-neuron-db's lane) or a multi-scale
      query-mass lever — both Bench-gated quality trades, never a refine-side index shape.

## Refs

- Bench 125 (the measurement this repairs; the verdict table + the mechanism section).
- Bench 124 §Next (the re-campaign fence this issue feeds).
- Issue 191 bar 5 (the pre-registered ceilings; T1's "the numbers are the contract").
- `src/domains/kernel_opt/router.rs` (the unwired narrowing; Plan 003/004 docs),
  `src/draft/latent_matcher.rs` `index_domain!(kernel, ...)` (the flat index), and
  `examples/kernel_opt_demo.rs` (the routing composition the matcher lacks).
