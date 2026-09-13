# Bench 089 — Issue 099: the INTERACTING-error corpus — admissibility gate 34/34, the first non-zero revert rate, and a measured ordering REGRESSION (learned arms clean 22→10)

**Status:** RECORD — the corpus EXISTS and does what it was designed for: it is the first corpus where the driver makes **no-progress reverts** (12/34 runs) and where the **resolved rate moves between orderings** — and it moved DOWNWARD for every learned arm. The per-attempt Beta counters (flat / shape-conditional / nonergodic) all regress clean 22/34 → 10/34 vs static min-delta on this corpus, because a KEEP that strands the run on an unfixable E0614 scores as success and poisons the counter. Issue 093 T4's owed surface ("a corpus where the resolved rate can move") is delivered — with the honest negative answer for the learned orderings on it.

**Date:** 2026-09-13 · **Box:** 4090 RTX box "shikuwa" (Windows 11, i7-13700K), cargo quiet (no sibling compute) · **Toolchain:** rustc 1.98.1 (48a229cea 2026-09-01) · **Profile:** `--release`.

> **Issue:** `099` · **Predecessors:** [Bench 087](087_fixseq_flip_prevalence.md) (the 0/230 flip/revert zero this corpus exists to unstick) · [Bench 088](088_nonergodic_ordering_headroom_predicate.md) (the null-gain ordering A/B) · **Gate instrument:** `examples/interacting_gate.rs`.

## What landed (T1 + T2 + T5)

- **`src/domains/rustc_errors/corpus_interacting.rs`** — 34 interacting pairs in 3 families, every case a structural 2×2 (seed / broken / a_only / b_only) composed from SHARED segment tables so the variants cannot drift (the design probe caught a hand-copied variant set reading as a fake interaction — variable-name drift across variants, not compiler behavior):
  - **F1** `ix_f1_e0502_e0499_copy` ×12 — head-first Copy stratum. Cold: E0502@{a,b}, the member-B E0499 **masked**; deref-removal unmasks it; swap finishes. Clean 2-keep path.
  - **F2** `ix_f2_e0502_e0499_string` ×10 — String stratum. clone-out is the correct E0502 repair; swap finishes. Clean 2-keep path (clone-out diverges textually from the seed, semantically equal — the ground-truth test passes).
  - **F3** `ix_f3_e0502_e0499_mid` ×12 — mid-read stratum (head read BETWEEN the two mutable decls). The **engineered revert**: deref-removal turns `&v[0]` into a by-value `v[0]` that is itself an instant immutable borrow against the still-live first mutable borrow → E0502 re-fires on the same line → **count 2→2, no-progress REVERT**; clone-out then KEEP(2→1) into a stall (see §Mechanisms).
- **`examples/interacting_gate.rs`** — the T1 admissibility gate. SET tier: pair multiset ≠ sum of singles, keyed `(code, primary-line-TEXT)`; LADDER tier: the production playbooks' `(strategy_id, replacement, trigger-text)` menu for member B compared across A-broken/A-fixed — Issue 099's mechanism-2 claim measured at the exact seam the driver consumes. Validity floor: seed clean / a_only declares code_a / b_only declares code_b. **First run: 34/34 ADMITTED in 14.2s.** Exit 1 on any committed non-interacting case — the rustc-drift tripwire.
- **`FloorHarness::check_source`** — the gate's compile primitive, sharing the scaffold/check/dedupe path with every measurement arm (gate and arms can never see different states).
- **`--interacting`** on `fixseq_prevalence` (T3) and `rustc_errors_floor --only ab` (T4); **T5**: the 34 pairs joined `floor_case_list()` + `case_count()` (N: 212 → **246**) + the spot test's family coverage (`ix_f1_/f2_/f3_` rows) + the full-corpus integrity gate.
- **`--runs`** on `fixseq_prevalence` — per-run attempt rings (code:strategy=outcome(before→after)), the structure the depth histogram hides.

## Lanes run

| lane | command | result |
|---|---|---|
| cheap structural tests | `cargo test --features rustc_errors --lib corpus_interacting` | **7/7** (determinism, unique names, brace balance, gt-in-every-variant, pairwise-distinct, shared-segment composition, as_generated) |
| admissibility gate | `cargo run --release --example interacting_gate --features rustc_errors` | **34/34 ADMITTED**, 14.2s |
| full-corpus integrity (T5) | `RIIR_FULL_CORPUS_INTEGRITY=1 cargo test --test generated_corpus_spot full_corpus` | **PASS over 246** — all mutants declare their codes, null arm fixes nothing, all 246 seeds' ground-truth tests pass (160s) |
| T3 flip prevalence | `fixseq_prevalence -- --interacting [--runs]` | below |
| T4 ordering A/B | `rustc_errors_floor --features rustc_errors,latent_retrieval,nonergodic_driver -- --only ab --interacting` | below |
| floor re-pin (T5) | `rustc_errors_floor -- --only floor,driver` | G-floor 43.5% (246); driver 60.2%, E0502 row 71 mutants 33/26/12; G1 semantics **148/148**; G2 p50 128ms/case |
| lint | `cargo clippy --features rustc_errors --all-targets` | 0 findings in touched files (healer fixed 2 `useless_format`; pre-existing repo warnings untouched) |

## T3 — flip prevalence over the interacting corpus (n=34)

```
runs 34 · attempts 68 · depth hist {2: 34} · runs ≥3 attempts: 0 (flip power still 0)
flips: code 0 · span 0 · pair 0 (over 34 runs)
loop invariant: HOLDS over all 68 attempts (kept ⇒ strict decrease; reverted ⇒ unchanged; no discontinuity)
repair quality: 64.7% total — E0502 34 mutants: 12 matched / 10 diverged / 12 unfixed
```

**The first non-zero revert rate on the stock driver** (12 reverts — every F3 run's deref-removal attempt). Bench 087 §6's reopen trigger 1 ("the revert rate leaves ~0") now has its instrument: the zero was a property of INDEPENDENT corpora, exactly as Issue 99's premise argued. The flip rate stays 0 **with the power honestly printed**: all runs are ≤2 attempts deep (the clean paths are 2 keeps; the stall paths end at an E0614 miss), and a flip needs ≥3 — the invariant half (Bench 087 §4's Lyapunov argument) extends over these 68 attempts unchanged, so the 0 remains a consequence, not a sample.

## Mechanisms (measured, not derived — the state-probe record)

| state | diagnostics | consequence |
|---|---|---|
| F1 cold | `E0502@a` + `E0502@b` (member B's E0499 **masked**) | |
| F1 + deref-removal | `E0499@b` **unmasked** only | swap → clean (2 keeps) |
| F1 + clone-out | `E0614@*head` only — **the E0499 vanishes** | stall (E0614 has no playbook) |
| F3 cold | `E0502@head` + `E0499@b` | |
| F3 + deref-removal | `E0502@head` **re-fires** (by-value `v[0]` = instant borrow vs live `a`) + `E0499@b` | **2→2 REVERT** |
| F3 + clone-out | `E0614` only | stall |

Two interaction classes the corpus exposed that the issue's design did not name:

1. **Typeck-stage masking** — E0614 (deref of a non-pointer) is a TYPE error; it blocks borrowck entirely, so the still-present E0499 is not even reported. Masking is not only borrow-internal.
2. **The keep-that-strands** — clone-out on the Copy strata is a KEEP (2→1, strict decrease, invariant satisfied) that leaves the run at an unfixable E0614. Per-attempt success counters cannot see the difference between a keep that enables the next fix and a keep that ends the run.

## T4 — the ordering A/B over the interacting corpus (n=34)

| arm | clean | iterations | reverts | attempts | multi-attempt runs |
|---|---:|---:|---:|---:|---:|
| static (min-delta) | **22/34** | 68 | 12 | 68 | 34 |
| flat counters | 10/34 | 44 | 0 | 44 | 10 |
| shape-conditional | 10/34 | 44 | 0 | 44 | 10 |
| nonergodic shared | 10/34 | 44 | 0 | 44 | 10 |
| nonergodic per-run | 10/34 | 44 | 0 | 44 | 10 |

`discord per-run vs static: 0 wins / 12 losses` · `HEADROOM PREDICATE = 64 ≠ 0` (the gain axis is measurable) · verdict (fixed this bench, see below): **WORSE — record honest negative; keep static.**

**Why every learned arm loses the same 12 cases:** the learning pass (static-ordered) records per-attempt outcomes — deref-removal: 12 keep (F1) + 12 revert (F3); clone-out: 22 keep, 0 revert (F2's 10 correct + F3's 12 strand-keeps). Every counter-driven ordering therefore prefers clone-out; on F1 (Copy), clone-out produces the E0614 strand (clean 22→10). **The per-attempt keep signal is not a run-outcome signal on interacting corpora** — the exact failure mode Issue 099's "second error's candidate ladder changes rung" predicted, arriving one rung up: not as a different candidate, but as a poisoned counter.

**Instrument fix landed with this bench:** the A/B's auto-verdict line compared REVERTS ONLY and printed "IMPROVEMENT" on this corpus while clean dropped 22→10. It now requires clean ≥ static AND reverts ≤ static; the re-run prints `verdict: … clean 10 vs 22, reverts 0 vs 12 — WORSE (record honest negative; keep static)`.

## Verdicts (Issue 099's pre-written decision criteria)

- T1 **passes** (34/34) ⇒ the family ships (T5 landed, N=246 re-pinned).
- Reverts **non-zero** (12) ⇒ Bench 087/088's re-run lane is armed — and the ordering claim finally has a surface with something to win or lose. The measured answer: **static wins; all learned orderings regress.** Issue 093 T4's question is answered on evidence, not on a missing instrument.
- The loop invariant still HOLDS — the reverts are no-progress reverts, not oscillation; the driver's monotone keep rule remains stronger than the corpus.
- **Mechanism 2 (E0505×E0597 repair-class collision) measured NON-interacting** on rustc 1.93/1.98 (diagnostics exactly additive; the clone-at-move repair moves neither the E0597's spans, labels, nor suggestions) — dropped per the issue's own criteria, recorded in the module docs and the probe transcript.

## Honest limits

- All numbers are SYNTHETIC-corpus numbers (the harness's standing label); the organic `.heal/fixseq/` surface still has zero rings across 33 repos.
- Flip power remains 0 (no run reaches 3 attempts): the corpus gives REVERT dynamics and ORDERING sensitivity, not yet oscillation dynamics. The reopen trigger for flips stays armed on whatever surface first produces ≥3-attempt runs.
- The E0614 stall is a playbook-coverage gap, not a corpus flaw (real code strands on uncovered codes too); an E0614 deref-removal-at-use playbook is a healer-corpus lead (post-mining intake), deliberately out of this issue's scope ("NOT a new driver, ordering policy, or detector").
