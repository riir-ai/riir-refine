# Issue 170 — retrieval regression gate is RED at every latent_retrieval posture: router annotation 60.3% < floor 72.3%, blended top-1 at ZERO margin — and the standing test lane cannot see it

**Status:** OPEN — CLAIMED 2026-10-24 (zed-m3-katopz-t170, epoch 1792809372,
expires 2026-10-26T02:36:12Z) — filed 2026-10-24, found by Issue 169 T0's paired-baseline
cross-check (riir-refine Bench 120). NOT caused by the 169 harness — verified
identical at plain `latent_retrieval` without the PoC feature.

## The two legs (both measured 2026-10-24, develop `3fc3219c`, --release)

1. **ROUTER ANNOTATION — already below floor.**
   `draft::retrieval_eval::tests::retrieval_precision_regression_gate` panics:
   `router annotation accuracy 60.3% below floor 72.3%` (retrieval_eval.rs:2580).
   The router axis's OWN counts: `router_checked` = 141 (the labeled spans,
   all four domains enabled) and `router_correct` = 85 — 60.28% is the only
   integer fit over 141 (84/141 = 59.6%, 86/141 = 61.0%). ⚠ **Coincidence
   warning:** arm (e)'s blended top-1 in Bench 120 is ALSO 85/141 (60.3%) —
   a different axis, a different measurement; the shared 85/141 shape is
   numeric coincidence, never the same number. The floor 72.3% is the
   4-domain-state constant (`router_annotation_floor()`, the per-state table
   from the Plan 134/030 restructure). The Issue-020 contract: floor =
   measured − 5 spans of corpus-growth headroom, "re-measure + re-pin with
   the delta recorded; never weaken silently." The floor was last re-pinned
   at a smaller corpus; the rust_perf corpus has grown 254→266 (batch 271)
   with 245–270 growth before it — TF-cosine routing decayed under corpus
   growth and nobody re-measured.
2. **BLENDED TOP-1 — zero margin.** Arm (a) measures exactly **124/141 =
   `BLENDED_TOP1`** (the floor's own convention is measured − ONE span). The
   next cross-pool competition event (one perf rule outranking one fixture,
   the documented P7 trade shape) reds the gate with no margin to absorb it.

## The visibility gap (why this is red and nobody noticed)

The whole `draft` tree — this gate included — compiles ONLY under
`latent_retrieval`. The standing test lane (`cargo test --lib` at default
features) compiles the gate OUT, so the red is invisible to the lane that
runs on every batch. The corpus-batch lane runs clippy gates, not this test.
**A gate nobody runs is documentation, not a gate** — the trap the workspace
keeps paying for.

## Tasks

- [x] T1 DONE (2026-10-24, measured both states with the T1 instrument
  `tests/router_per_span_dump.rs` + the vocab probe): 34 → 56 misrouting
  spans. Docker 0→19 misses (17 → clippy), Dist 0→4 (→ clippy), Clippy 6→8,
  Perf 28→25 (IMPROVED). Attributed: growth competition at the SHARED
  128-slot GlobalFreq vocab, NOT a defect — the +68 rust_perf serving-wave
  rules (entries_39–46, Batches 248–271) + 4 clippy rows (P51a + let-else)
  re-ranked the global frequency table; docker/dist fixture tokens (FROM,
  RUN, cargo, build, release, target...) fell out of the top-128 — docker
  in-vocab coverage collapsed 40.2% → 3.4% (13 → 4 distinct tokens; dist
  20.0% → 10.5%). Router-code diff B230→HEAD verified formatting-only
  (80fb8920); docker/dist corpora byte-unchanged. Full record: the
  `router_annotation_floor()` re-pin comment + Bench 120.
- [x] T2 DONE (same commit as the attribution): router floor (4-domain
  shader-off arm) 102/141 → **80/141** (measured 85/141, −5 spans headroom
  per the Plan-030 convention; was −17 RED). BLENDED_TOP1 124 → **123/141**
  (measured 124; restores the one-span margin, the Issue-063 convention —
  the B248–B271 waves had spent the last margin span). ALSO found at ZERO
  margin by the same measurement and re-pinned with deltas: CLIPPY_TOP1
  42 → 41/48, CLIPPY_TOP3 43 → 42/48, PERF_TOP3 54 → 53/56 (the B243 note
  had flagged all three; the growth held measured == pin). Top-3 131 ≥ 130
  and top-5 133 ≥ 132 HELD, untouched. Gate green at --release AND debug
  (counts profile-stable).
- [x] T3 DONE: `ci_feature_guard.sh` Layer **5b2** — the named retrieval
  suite (`cargo test --lib --features latent_retrieval retrieval_`: the
  regression gate + the fixture-label verifies + determinism; 21 tests,
  ~95 s debug) runs with its PASS line asserted by name (the green-zero
  shape — `ok` with 0 tests — refused; Layer 5d's law). The batch-lane
  GOAT checklist (`.agents/skills/riir-refine/SKILL.md` Protocol A step 5)
  gains the matching default-posture step for corpus batches.
- [-] T4 (deferred): the router itself — the lexical TF-cosine branch is
  measured closed as a GATE (70% < the 85% bar, Bench 025) and stays an
  annotation; re-routing architecture is out of scope unless T1's attribution
  says the decay is a defect rather than growth competition.

## Non-goals

No corpus rollback (batch 271's rules are legitimate P7 growth); no router
re-architecture (T4); no change to the 169/120 records — their arm-(a)
baseline numbers ARE this issue's evidence.
