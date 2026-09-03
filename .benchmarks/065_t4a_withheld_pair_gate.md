# Bench 065 — Issue 062 T4a: the withheld-pair gate that decides route (b)

**Status:** RUN 2026-09-03, M3 Max. Verdict **FAIL at the decisive row** →
**route (b) is REFUTED and Issue 062 T4 closes REFUTED.** No widen-and-retry:
that is the preregistration's own instruction for this branch, and the
evidence below is stronger than the branch anticipated.

**Harness:** `src/draft/tpr_t4a_withheld.rs` + `tests/t062_t4a_withheld_clippy.rs`,
feature `tpr_structure`. **Preregistered at `19beece`, before the harness had
produced a number** — thresholds, verdict rule and the FAIL-needs-a-readable-
instrument asymmetry are all in that commit.

**Determinism:** bit-stable over 3 separate processes (every cell, every row).

```bash
CARGO_TARGET_DIR=/tmp/riir_clippy_t4a cargo test --release \
  --features tpr_structure --test t062_t4a_withheld_clippy -- --nocapture
```

---

## 1. What was asked, and what decides it

Issue 062 T4's owner call (`324c9a6`) routed (b) — fuse the TPR structural
evidence channel into the reranker — against `clippy_lints` only, `rustc_errors`
excluded, **gated on this test**. The gate withholds one observed
`(role, filler)` pair at a time, refits, and asks whether TPR can identify a
state whose pair combination it has never seen, against an atomic-dictionary
null that cannot by construction.

Preregistered bar: TPR beats the null by **≥ 20.0 pp** (Issue 707 G8's
`G8_MARGIN_PP`, verbatim — this gate is deliberately not a softer version of
the upstream one), with every trial readable, the null informative on its own
training set, and a one-flip cushion.

## 2. The arm

`clippy_lints / tokenbag256 [coalesced T=8]` — the arm Bench 063 §12.7 measured
at **13 / 45 testable fillers**, chosen over `rustc_errors` because its 18.8%
pair coverage is *structural* (17.7 states/pair: the pairs that occur are
saturated) while clippy's is *thin observation* (1.8 states/pair, 62 of 270
pairs), the failure more corpus actually fixes.

```
corpus: n_states=113 m(roles)=6 n_fillers=45 D=256
composition: roles/filler max=4 mean=1.378 over 45 fillers, 13 testable;
             distinct pairs 62
populations: validated 30 pairs / 45 scored states
           | fitted-but-unvalidated 32 (all single-role-filler)
           | outside the fit 208 of the 270-pair space
```

The three populations are **reported apart and asserted to partition the pair
space**. Pooled, the 208 unobserved pairs would let a prediction read as a
measurement.

The **shared** pool (every distinct observed pair) is forced, not stylistic: a
per-trial pool of `(r, ·)` at one fixed role makes every filler role-determined
inside the scored universe, so `WithheldPairReport::verdict` would be `None` on
every trial — the Issue 711 refusal, fired by the harness's own pool
construction rather than by the corpus.

## 3. The preregistered run, and why it did not decide

At BIC's `d = 1`: TPR **0.0%** OOD, null **0.0%**, chance 1.6% (pool 62),
margin **0.0 pp**. On the preregistration as committed that is a FAIL.

It was not recorded as one, because **0.0% on both arms of all 30 trials is the
exact shape where "the corpus does not generalize" and "the readout cannot
answer" are indistinguishable** — and the preregistration checked the *null's*
vacuity (Bench 062's lesson) while never checking TPR's.

### Amendment A1 — the symmetric vacuity check

`tpr_informative()`: the same artifact, the same shared pool, scored on the
states the fit **saw**. Same `> 0.5` bar Issue 707 G8 applies to the null's ID
top-1.

**Measured: TPR ID top-1 21.1% at `d = 1`** — 13× the chance floor, so the
readout is not dead; far under the bar, so a 20 pp OOD margin was never
*reachable*. A readout cannot generalize better than it interpolates, so the
ID number is the **ceiling on the achievable margin**. The FAIL the raw number
implied would have been decided by the readout's weakness, which is the
"gate verdicts decided by orthogonal quantities" failure.

A1 is **one-directional and safe to make after seeing the number**: it can only
convert a FAIL into an INCONCLUSIVE. It cannot produce a PASS and cannot raise
a margin. An amendment that could strengthen the conclusion it is applied to
would not be legitimate here; one that can only weaken it is.

At `d = 1` the gate therefore returns **INCONCLUSIVE**.

## 4. Amendment A2 — the d sweep: is the zero the corpus or the geometry?

BIC picks `d` by fit parsimony. The withheld-pair readout's **discriminability**
is a different quantity, and at `d = 1` a filler vector is a *scalar* — two
candidates sharing a role differ only in scale, and an L2 argmin over a 256-D
state can barely separate them. That is a handicap on the instrument with no
bearing on whether the corpus is structured.

Sweeping is **not** the widen-and-retry T4a forbids: no corpus is added, and
every geometry swept is one Bench 063 P13 already declared admissible on this
arm. It is also gated on the verdict in-harness — on a PASS or a FAIL the sweep
is skipped, because there the gate has decided and a sweep *would* be fishing.

| `d` | TPR ID | TPR OOD | null OOD | margin | verdict | inadmissible refits |
|---|---|---|---|---|---|---|
| 1 | 21.1% | 0.0% | 0.0% | 0.0 pp | INCONCLUSIVE | 0 |
| 2 | 46.8% | 2.2% | 0.0% | 2.2 pp | INCONCLUSIVE | 0 |
| **4** | **63.3%** | **0.0%** | **0.0%** | **0.0 pp** | **FAIL** | **0** |
| 8 | 81.1% | 0.0% | 0.0% | 0.0 pp | INCONCLUSIVE | **4** |

The single non-zero OOD cell (`d = 2`, 2.2%) is **one state of 45** — at the
1.6% chance floor, not above it.

### Amendment A3 — the refit's admissibility, not the corpus's

Bench 063 P13 measured admissibility at **113** states. Every trial fits a
corpus 1–5 states smaller, and the per-role starvation bound (`> d` states per
role) is exactly the one a withhold can break: at `d = 8` a role holding 9
states that loses 2 to the withhold is starved.

A3 checks it per trial, on the corpus the refit actually saw, with the parent's
role vocabulary held **fixed** (rebuilding would re-derive the vocabulary and
could shrink `m` — a different geometry than the one the gate declares).

**It fired.** `d = 8` has **4 of 30 refits inadmissible**, so its 81.1% ID is
partly read off vacuous fits and the row is correctly INCONCLUSIVE rather than
the sweep's strongest evidence. Without A3 that row would have been read as the
best geometry available, and the whole §5 selection would have landed on it.

## 5. The decisive row, by a rule stated before it was applied

Reading a conclusion off a `d` chosen after seeing the sweep is cherry-picking
**only when the choice favours the conclusion**. The rule applied here favours
the opposite one:

> Among the rows where every instrument condition holds — readable (including
> A3's admissible refit), null informative, TPR informative — take the one with
> the **highest in-distribution top-1**, i.e. the geometry where TPR is
> strongest and its achievable OOD margin is largest.

If the hypothesis fails at its own best readable geometry it fails everywhere
below it too. The selection is adversarial to the conclusion being drawn, and
it is computed in-harness (`max_by` over `tpr_id_top1`) rather than eyeballed.

**Decisive row: `d = 4`.** ID 63.3%, 0 inadmissible refits, null informative,
every trial readable at coverage 100.0%. There TPR scores **0.0% OOD against a
1.6% chance floor and a 0.0% null → margin 0.0 pp against a 20.0 pp bar →
T4a FAIL.**

## 6. The finding: memorization, measured

**The ID column is not flat — it climbs 21.1% → 46.8% → 63.3% → 81.1% while OOD
stays at chance throughout.**

That is the whole result. The fit gets steadily, substantially better at
identifying pairs it *trained on* as `d` grows, and gains **nothing** that
transfers to an unseen `(role, filler)` combination. It is the atomic-
memorization failure mode Research 527 predicts as the default, measured on our
own corpus — and it is the opposite of the systematicity claim route (b) would
have been fused on.

Note what this rules out. A weak instrument and a memorizing one both print
`OOD 0.0%`; A1 and A3 are what tell them apart, and here they say the
instrument is fine (63.3% ID on an admissible fit) and the *generalization* is
absent.

It also sharpens Bench 062 Phase 1. There the clippy atomic null was **vacuous**
(0.0% ID — no two healer-corpus bindings share a normalized token structure, so
the memorizer could not fit its own training set), which left T1's OOD failure
readable only as filler-dominated similarity. Here the memorization reading is
direct on the **TPR** arm: it fits its own training set at 63.3% and still
generalizes at chance.

## 7. Consequences

1. **Issue 062 T4 closes REFUTED.** Route (b) is dead on this corpus.
   The preregistration's "do not widen and retry" governs: this is not the
   INCONCLUSIVE branch, where widening was the authorized response.
2. **Route (a) is unaffected and unattractive.** Bench 063 §11.6 already
   *measured* the 8-D consumer space flat (both probes below 1.0, `r_bow`
   beating `r_full`, permuting roles improving the fit), so widening
   `LATENT_DIM` would be a cross-repo contract change in pursuit of a signal
   that is absent at both widths measured.
3. **katgpt-core's `tpr` feature stays opt-in.** Its T-Promote was blocked by
   the no-default-consumer rule, and T4 was the candidate consumer. There is
   now no landed consumer and no route to one on this corpus. This is a
   consumer-side refutation, **not** a defect in `tpr` — every upstream gate
   (G1–G4, G8) passes on a planted corpus, and G8's own record notes it is a
   positive control rather than evidence about real corpora. Bench 065 is that
   evidence, and it is negative.
4. **T6 (counterfactual surgery) inherits this and should be read against it.**
   `surgery_delta_into` is bit-additive on any artifact, so it answers a
   counterfactual about an unseen pair cleanly, confidently and wrongly. The
   208-pair outside-the-fit population above is the size of that hazard on this
   corpus: **77% of the pair space**. T6 must report `ObservedPairs::contains`
   per query and keep the populations apart — which was already its plan, and
   is now quantified.

## 8. What would change this verdict

Not more clippy corpus — that is the forbidden retry, and §6 says why it would
not help: the failure is not thin observation, it is that composition is not
being learned at any capacity. What *would* be evidence:

- a corpus where roles and fillers are genuinely crossed **by construction**
  (planted, as G8's is) yet drawn from real spans — i.e. the collinearity
  Bench 062 caveat 2 identified (32/45 clippy roles occur under exactly one
  shape-class) removed at the source rather than sampled around;
- a role scheme other than shape-class. This bench tests one binding of the
  axes, decided in Bench 063 T3 and forced by `n_bind_slots == arity`.

Both are new issues, not retries of this one.

---

## Appendix — verbatim run

```
=== Issue 062 T4a: withheld-(role, filler)-pair gate (route (b) kill-switch) ===
preregistered: PASS = every trial readable + null informative + margin >= 20.0 pp
with a one-flip cushion; FAIL = bar not cleared -> T4 REFUTED, no widen-and-retry;
INCONCLUSIVE = cleared but one flip crosses, or an instrument was unreadable

── Issue 062 T4a: withheld-pair gate ──  clippy_lints / tokenbag256 [coalesced T=8]
  corpus: n_states=113 m(roles)=6 n_fillers=45 D=256 d=1 (BIC, admissible)
  composition: roles/filler max=4 mean=1.378 over 45 filler(s), 13 testable; distinct pairs 62
  populations: validated 30 pair(s) / 45 scored state(s) | fitted-but-unvalidated 32 | outside the fit 208 of 270 pair space
  unvalidated by reason: single-role-filler 32 | would-empty-role 0 | fit-error 0

  role                         filler                              held  train     tpr    null    cov
  iter-chain                   map_unwrap_or                          3    110    0.0%   24.5%    0.0% 100.0%
  iter-chain                   needless_return                        1    112    0.0%   20.5%    0.0% 100.0%
  iter-chain                   redundant_closure                      1    112    0.0%   23.2%    0.0% 100.0%
  iter-chain                   unnecessary_sort_by                    2    111    0.0%   25.2%    0.0% 100.0%
  macro                        manual_map                             1    112    0.0%   21.4%    0.0% 100.0%
  macro                        map_unwrap_or                          1    112    0.0%   23.2%    0.0% 100.0%
  macro                        match_bool                             5    108    0.0%   26.9%    0.0% 100.0%
  macro                        semicolon_if_nothing_returned          3    110    0.0%   19.1%    0.0% 100.0%
  macro                        unnecessary_sort_by                    1    112    0.0%   23.2%    0.0% 100.0%
  multi-item                   collapsible_if                         2    111    0.0%   15.3%    0.0% 100.0%
  multi-item                   redundant_pattern_matching             1    112    0.0%   19.6%    0.0% 100.0%
  multi-item                   redundant_static_lifetimes             1    112    0.0%   15.2%    0.0% 100.0%
  multi-item                   semicolon_if_nothing_returned          1    112    0.0%   16.1%    0.0% 100.0%
  multi-item                   single_match                           2    111    0.0%   19.8%    0.0% 100.0%
  multi-item                   single_match_else                      2    111    0.0%   24.3%    0.0% 100.0%
  plain-fn                     collapsible_if                         1    112    0.0%   21.4%    0.0% 100.0%
  plain-fn                     manual_map                             1    112    0.0%   21.4%    0.0% 100.0%
  plain-fn                     match_bool                             2    111    0.0%   18.9%    0.0% 100.0%
  plain-fn                     needless_return                        2    111    0.0%   23.4%    0.0% 100.0%
  plain-fn                     redundant_pattern_matching             1    112    0.0%   19.6%    0.0% 100.0%
  plain-fn                     redundant_static_lifetimes             1    112    0.0%   18.8%    0.0% 100.0%
  plain-fn                     semicolon_if_nothing_returned          1    112    0.0%   30.4%    0.0% 100.0%
  plain-fn                     single_match                           1    112    0.0%   20.5%    0.0% 100.0%
  plain-fn                     single_match_else                      1    112    0.0%   22.3%    0.0% 100.0%
  plain-fn                     unused_unit                            1    112    0.0%   22.3%    0.0% 100.0%
  other                        match_bool                             1    112    0.0%   17.0%    0.0% 100.0%
  other                        redundant_closure                      1    112    0.0%   20.5%    0.0% 100.0%
  other                        semicolon_if_nothing_returned          1    112    0.0%   21.4%    0.0% 100.0%
  other                        single_match_else                      2    111    0.0%   18.0%    0.0% 100.0%
  other                        unused_unit                            1    112    0.0%   18.8%    0.0% 100.0%

  null: informative on every trial = true (ID coverage > 99.9% each)
  TPR (A1): in-distribution top-1 21.1% over the same pool -> informative = false
  readable: every trial verdict Some + coverage 1.0 + admissible refit = true (0 inadmissible refit(s))
  OOD top-1 (state-weighted): TPR 0.0%  vs  null 0.0%  (chance 1.6%, pool 62)
  margin 0.0 pp (bar 20.0 pp); one flip is worth 2.2 pp over 45 scored state(s)
  → T4a INCONCLUSIVE

T4a verdict: INCONCLUSIVE — margin 0.0 pp over 45 scored state(s), one flip worth 2.2 pp

── A2: is the zero the corpus or the geometry? (d sweep, admissible grid) ──
  d    tpr-ID   tpr-OOD  null-OOD    margin  verdict / refit admissibility
  1     21.1%      0.0%      0.0%      0.0pp  INCONCLUSIVE  0 inadm refit(s)
  2     46.8%      2.2%      0.0%      2.2pp  INCONCLUSIVE  0 inadm refit(s)
  4     63.3%      0.0%      0.0%      0.0pp  FAIL          0 inadm refit(s)
  8     81.1%      0.0%      0.0%      0.0pp  INCONCLUSIVE  4 inadm refit(s)

Decisive row: d=4 — TPR's strongest READABLE geometry (ID 63.3%, 
0 inadmissible refit(s), null informative). Rule stated above the selection: 
take the row most favourable to the hypothesis.
  There TPR scores 0.0% OOD against a 1.6% chance floor and a 0.0% null 
-> margin 0.0 pp against a 20.0 pp bar -> T4a FAIL.

  The ID column is NOT flat: 21.1% -> 81.1% as d grows, so the readout 
gets steadily better at identifying pairs it TRAINED on while OOD stays at 
chance throughout. That is the memorization signature, not a weak instrument 
— the two are what A1/A3 exist to tell apart.
test t062_t4a_withheld_pair_gate ... ok
```
