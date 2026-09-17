# Issue 122: PowerPlay frontier miner — modelless generator for the healer's capability frontier

**Status:** OPEN — fusion idea distilled from riir-clippy/.research/178 (Schmidhuber RSI essay); novelty survives at the pinned conjunction only (class-level prior art: POET/MCC, Absolute Zero, Self-Play SWE-RL); PoC-gated.

---

## Pinned claim (novelty-bearing form)

A **modelless generator** (non-learning, zero LLM, deterministic given a seed) that constructs **the simplest code fixtures the current fix-rule corpus cannot yet repair**, verifies candidates with the **compile/clippy oracle only** (no LLM in the verification loop), and grows the corpus at the capability frontier — PowerPlay's acceptance invariant (unsolved-before / solved-after / all-prior-still-green) applied to the healer's own rule corpus.

## Why now — three documented gaps this closes

1. **Issue 089 T4's frontier admission gate is built but unwired** — `frontier_report.rs:1-4` ("evidence volume is the bottleneck, so nothing here GATES anything yet") and `heal.rs:7226-7228` (wiring "waits on the report proving band-mass is predictive"). A generator that densifies the band `0 < p̂ ≤ 0.5` on demand is the evidence factory the gate has been waiting for.
2. **No interacting-problem generator exists** — `.benchmarks/087_fixseq_flip_prevalence.md:196-206`: 0/230 reverts; "Every corpus this bench ran on is built from independent single-mutation cases — `compose_multi` concatenates members that cannot affect each other — so a fix can never re-break anything." Issue 099 T3 explicitly needs a corpus where repairing A changes B's diagnostic set. Dependent-mutation composition is the missing instrument (also unblocks the horizon-report gain axis, "honestly NULL — no real surface with revert dynamics exists", and gives the necessity-bench swap arm a real donor pool).
3. **Mining intake is user-driven only** — the auto-oracle (Issue 113) anchors spans from real diagnostics, but nothing SELF-generates at the frontier. PowerPlay's loop is the self-play twin of `--mine`.

## Admissibility invariant (per generated row)

A candidate fixture `c` is committed iff ALL of:
1. Oracle declares the current corpus FAILS to heal `c` within budget (verified via `clippy_oracle`, default features);
2. `K(c)` is minimal among admissible candidates (simplest-first: fewest mutation ops / smallest span count — MDL tie-break pinned);
3. After a solution `s` is found by the existing lanes, the full regression re-run keeps every prior corpus row green (a "solution" that breaks an old row is REFUSED);
4. Exactly-once by content address (duplicate fixture rejected — BLAKE3 of the fixture source).

Committed rows carry `(fixture, solution, verification)` and append to `frontier_snapshots.jsonl` so band-mass evidence accumulates for the Issue-089 gate.

## Constraints (house laws this must respect)

- **Corpus freeze-table law (plan 137 T7.5 / Batch-151 lesson):** any corpus-touching landing regenerates the gen2/gen3 freeze tables in the SAME batch — new rows move the deterministic stride windows and the freeze-status gates red on develop by design until labels/exclusions are regenerated as data.
- **bench_guard posture:** the generator writes corpus/eval fixtures, never bench-number artifacts; perf rules stay declined on bench targets.
- **Determinism:** same seed → byte-identical fixture set (property test); zero network, zero LLM calls in the loop.
- **Suggest/fix lane split respected:** generated fixtures are corpus INPUTS; the two lanes' corpus truths (lease vs compiled) are not conflated (Issue 116 shape).

## GOAT gate (PoC → promote path)

- **G1** admissibility per row as above, incl. the adversarial case (solution that re-breaks an old row must refuse) — negative test fires.
- **G2** band-mass increase measured per K generated rows (frontier snapshots delta) — the Issue-089 predictive evidence.
- **G3** no shipped-lane regression: necessity-bench two-sided canaries green; score-bench heal rate at-or-above at equal corpus size.
- **G4** generator is deterministic + zero-LLM (property test), fixture build reproducible across machines (BLAKE3 of derived corpus identical).
- Feature flag `frontier_miner` (opt-in); promote only on G1-G4 AND the band-predictivity reading the Issue-089 gate needs.

## Non-goals

- Not a training loop (the training-track cousin — RFT/GRPO quest-grammar and the drafter DPO lane — lives in riir-train .issues/559).
- Not agent self-modification: the generator grows the CORPUS, never healer source (the deliberate Gödel-machine boundary this repo already enforces: verified corpus/data evolution, source-level self-rewrite forbidden).
- Not a benchmark-construction substitute for real-repo drift fixtures (Bench 024's real-clippy drift corpus stays the drift oracle).

## Prior-art deltas to carry in any write-up

(i) generator has no LLM and does not learn (vs AZR/SSR/Voyager) · (ii) capability substrate grown is a modelless rule corpus, not model weights or agent code (vs SSR/AZR/DGM) · (iii) frontier defined by the corpus's own repair failures, not a learning-progress reward or difficulty ramp (vs AZR) · (iv) PowerPlay→code-corpus lineage gap stands 2015-2026.
