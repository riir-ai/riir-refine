# Issue 137 — CRB-style compile-clean / test-failing corruption eval (new score-bench generation + DLM remask arming check)

**Status:** Active — T1–T6 LANDED (generation v9 corpus: 48 execution-verified
fixtures; localization metrics + arming gate + panel; per-class rows, loop-harm
detector, seeded CI; T5 healer-ranking POC measured OUTCOME (c)
infrastructure-ahead-of-consumer: item-level relative gap +0.191, hit@1 0.438
vs chance 0.323 — the ranking separates CRB corruption but no semantic-bug
consumer exists). T7 (bench doc + research-note status + AGENTS line) remains.
Control arm: WIRED but empty by construction for the v1 type-preserving engine
(disclosed in the pin test + panel — no compile-failing candidate arises from a
type-preserving swap table on these seeds); the bucket + verify arm stay live
for future engines.
**Date:** 2026-09-27
**Source:** arXiv:2512.15596 "Corrective Diffusion Language Models" (CRB instrument),
code `zhangshuibai/CDLM` @ `12fe555aa815959fe1323a85f3d8a9656c480149` (MIT)
**Priority:** modelless PRIMARY track (the training half is
`../riir-train/.plans/424_cdlm_correction_aware_dlm_posttrain.md`, SECONDARY)

## Problem

CDLM's instrument: type-preserving, tokenizer-consistent corruption + execution
validation + known-position localization metrics. Three facts shape the Rust port:

1. **Compile-clean / test-failing is the only real bucket.** Where code fails
   `cargo check`, rustc/clippy already mark the span (the healer reads spans FROM the
   oracle in `--fix-compile`) — hit@K there measures rustc span accuracy, near-
   trivial. Where code compiles but fails tests, NO oracle marks the error: that is
   where localization is a genuine open question. In Rust the type-preserving
   compile-clean swap classes are literal / operator / identifier-from-same-scope —
   cargo-mutants' territory (cited prior art; Pydra/BugsInPy/mutation-testing also
   cited in the note).
2. **The k-ladder exists — this is a new generation, not a harness.**
   `src/score_bench/generations.rs` ships `generation_v2()` (deterministic k-ladder,
   issue 072 T5) and `gen4-k*` (rustc error-code ladder, Plan 093). "Difficulty = |E|
   replaced tokens" IS what that k already is. Build the corruption generation as
   the next free `generation_v*()` row (v8 already exists as of this filing — pick
   v9+ after re-reading `generations.rs` at implementation time) under the house
   append-refusal law: not vacuous, harder than the prior generation.
3. **The consumer split is honest.** PRIMARY consumer: the DLM remask confidence
   gate arming check (katgpt-forward d2f / diffusion_sampler; riir-train Plan 414
   sampler) — CDLM's causal finding ("uninformative confidence makes confidence-
   gated loops actively harmful") becomes: measure the gate's selection signal over
   these fixtures BEFORE arming the remask loop; refuse to arm below floor. NOT a
   consumer: `reheat_fix_pass` (`src/bin/heal.rs:11838`) — verified fixed
   `MAX_PASSES = 4`, no confidence gate anywhere; withdrawn as a target.

## Tasks

- [x] **T1 — corruption generator** (`score_bench` fixture construction lane or a
      `scripts/` sibling): syn-AST slot classes (operator table `+↔-`, `<↔<=`,
      `&&↔||`; identifier-from-same-scope; literal-same-type), token-count preserving,
      seeded. Two-tier filter: `cargo check` must stay CLEAN, then `cargo test` must
      FAIL (keep only genuinely-broken programs); emit per-program error-span
      manifest `E`. `CARGO_TARGET_DIR=/tmp/...` isolation per house rules. Gates:
      BLAKE3-pinned corpus; zero-corruption draw never kept as "corrupted"; planted
      no-op corruption REJECTED by the filter (proves it discriminates); per-|E|
      count floors. **LANDED: `src/score_bench/corruption.rs` (engine: 8 seed
      programs — the seed gate caught two real authoring bugs pre-run — op table
      `+↔- *↔/ <↔<= ==↔!= &&↔||`, int-literal XOR-1, same-type param-swap plans
      with shadow guards; `while`-condition sites excluded by the hang-safety law;
      spans recomputed in CORRUPTED-code coordinates) +
      `tests/corruption_corpus_freeze.rs` (two-tier filter, pid-scoped probe crate,
      30s wall cap, signal/timeout rejection, text-dedup, write/verify modes) +
      `src/score_bench/corruption_corpus_v9.rs` (48 fixtures = 16 per |E| rung,
      blake3 digest pin; floors 10/10/10 fire downward only).**
- [x] **T2 — localization metrics** over any span scorer: confidence gap
      (mean clean − mean error), hit@K, FPR, MRR. Loud-zero laws: constant-confidence
      scorer → gap must read 0 and the gate REFUSES (never "perfectly calibrated");
      shuffled-confidence control → hit@K must fall to chance (proves the metric
      discriminates); partial rankings never fold into a pass (two floors that break
      separately: fixture walk + full-ranking emission). **LANDED:
      `src/score_bench/localization.rs` (confidence-polarity contract — high =
      likely correct, the d2f/Plan-414 native polarity; chance_hit_at_k hypergeometric;
      emission floor + fixture-walk floor; unit-tested per law).**
- [x] **T3 — the new generation row** (`generation_v9` — v8 was the highest;
      re-read `generations.rs` at implementation time): compile-clean
      / test-failing fixtures at |E| ∈ {1,2,4} (the gen2/gen4 k convention), oracle-
      anchorable spans EXCLUDED from the headline bucket (they form the control arm:
      compile-failing corruption, reported separately, expected near-trivial).
      Append-refusal law asserted in the freeze test. **LANDED: `generation_v9` /
      `v9_freeze_status` in corruption.rs (digest self-check = the append-refusal;
      always-on `corpus_v9_pins_hold`); dispatch `--generation 9` → its own runner
      (the gen-4/5 shape, no clippy oracle, never appended to the heal score
      history). Control arm: wired, empty by construction for v1 (disclosed).**
- [x] **T4 — DLM remask arming check (primary consumer):** a small instrument that
      runs the arm's confidence source (d2f remask confidence, or Plan 414's trained
      gap once it exists) over the T3 fixtures and reports gap/hit@K/FPR; arming gate:
      below floor → refuse to enable the remask loop. Negative control: permuted
      confidence vector must be refused. Byte-identical when the instrument is not
      invoked (never in any default path). **LANDED: the `SpanScorer` seam +
      `arming_verdict` (pre-registered `DEFAULT_ARMING_FLOORS`, fixed check order,
      named refusals) + live controls (oracle arms; constant/shuffled/inverse
      refused with named reasons — re-proven every panel run). The PRODUCTION
      confidence source is the pending half: it lands by implementing the trait
      when the d2f/Plan-414 arm exists — the gate is consumer-ready.**
- [x] **T5 — healer span-ranking POC (secondary, negative-result pre-registered
      BOTH ways):** run the healer's existing span ranking over T3 fixtures; record
      whether scores separate corrupted from clean spans. Pre-registered outcomes:
      (a) separation AND a consumer → promote; (b) no separation → recorded finding;
      (c) separation but no consumer (the healer has no semantic-bug fixer today) →
      recorded as infrastructure-ahead-of-consumer. All three are legitimate
      closers; only silence is a failure. **MEASURED 2026-09-28 OUTCOME (c):
      `run_healer_ranking_poc` (item granularity, max fused rule-similarity per
      top-level item, test modules excluded): clean items mean 87.92 vs corrupted
      71.14 — relative gap +0.191, hit@1 0.438 vs chance 0.323. The ranking
      SEPARATES CRB corruption (a real contrast with the Bench-018 lint-shaped
      caveat: semantic corruption is corpus-ATYPICAL, so similarity drops) — but
      the healer has no semantic-bug fixer to consume a span ranking, so the
      recorded finding is infrastructure-ahead-of-consumer; the instrument rides
      the generation-9 panel for the DLM remask gate.**
- [x] **T6 — panel rows:** loop-harm A/B detector (paired non-inferiority on planted
      errors: the loop must fix them without breaking previously-correct positions —
      the paper's measured failure mode); oracle-anchored-vs-heuristic gap audit
      reported per class, never pooled (pooled-equals-min arm must fail); Bernoulli
      re-draws with CI-width disclosure, seeded (born compliant with the
      unseeded-global-RNG gate). **LANDED: `loop_harm_verdict` +
      `paired_non_inferior` (unit-tested both directions); per-class rows never
      pooled + `class_floor_verdict` min-class binding (pooled-equals-min arm fails
      by test); seeded Wilson + bootstrap CI width disclosed in the panel. The
      oracle-anchored-vs-heuristic GAP audit completes with T5's heuristic rows —
      the control arm is empty for v1, so the oracle-anchored half reports its
      empty bucket honestly.**
- [ ] **T7 — close-out:** bench doc in `.benchmarks/` with box state; update the
      research note's Status; cite cargo-mutants + Pydra in the doc's prior-art
      paragraph; remove scratch target dirs.

## References

- Research note: `../riir-clippy/.research/215_cdlm_corrective_diffusion_distill_verdict.md`
- Training half: `../riir-train/.plans/424_cdlm_correction_aware_dlm_posttrain.md`
- Prior generations: issue 072 T5 (gen2 k-ladder), Plan 093 (gen4 rustc ladder)
- Remask-gate consumer: `katgpt-forward` d2f remask / `diffusion_sampler.rs`
