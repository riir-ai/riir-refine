# Issue 170 — retrieval regression gate is RED at every latent_retrieval posture: router annotation 60.3% < floor 72.3%, blended top-1 at ZERO margin — and the standing test lane cannot see it

**Status:** OPEN — filed 2026-10-24, found by Issue 169 T0's paired-baseline
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

- [ ] T1: attribute the router decay — which spans newly misroute vs the last
  pinned state, and which corpus batches moved them (growth competition vs a
  real regression). The 169 T0 harness (`--features multivector_poc --example
  multivector_poc`) prints the arm-(a) table in seconds if a per-domain
  breakdown is needed; the router axis itself is `run_eval`'s
  `router_checked`/`router_correct`.
- [ ] T2: re-pin BOTH floors with the delta recorded (the Issue-020/063
  conventions: measured − 5 spans router headroom, measured − ONE span
  blended) — in the SAME commit as the attribution note, never silently.
- [ ] T3: close the visibility gap — add a `latent_retrieval`-posture row to
  the standing test lane (test_gate-style suite or the batch lane's landing
  checklist) so this gate executes where corpus batches land. Without this,
  T2 re-pins another invisible floor.
- [-] T4 (deferred): the router itself — the lexical TF-cosine branch is
  measured closed as a GATE (70% < the 85% bar, Bench 025) and stays an
  annotation; re-routing architecture is out of scope unless T1's attribution
  says the decay is a defect rather than growth competition.

## Non-goals

No corpus rollback (batch 271's rules are legitimate P7 growth); no router
re-architecture (T4); no change to the 169/120 records — their arm-(a)
baseline numbers ARE this issue's evidence.
