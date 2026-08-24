# Issue 039 — ELO-rated rules + evidence-grounded ruliology over the unified trajectory store

**Repo:** riir-clippy
**Filed:** 2026-08-24 (owner prompt: "it should be ruliology and elo too? so it will be seamless process around that")
**Status:** OPEN — **depends on [Issue 038](038_self_evolve_memory_neuron_db_backend.md)** (the store) + **[Issue 037](037_e2e_self_evolve_adoption_proof.md) T2** (verdicts; without outcomes ELO has zero signal). Feeds 037's E3/E4 learn gates.
**Branch:** develop

## The vision (seamless loop)

One substrate, no parsing between stages:

```
record (038 store) → verdict/tier (037-T2) → ELO ratings + ruliology front
   ↑                                                      │
   └────────── selection (BetaPosterior; ELO as measured challenger) ←─┘
```

Rules get ratings from REAL usage; the ruliology Pareto front gets grounded
in measured evidence instead of static structure alone; the audit prints an
ELO leaderboard (human-readable THERE, not in storage — per the 038 rule).

## Substrate-first grep verdict

1. **ELO substrate exists, twice, in katgpt-rs** — itself a DRY signal:
   - `katgpt-pruners/src/arena/types.rs` L84–142: `Ranking { wins, losses,
     draws, elo: f64 }`, `Leaderboard` (ELO sort + markdown render),
     `EloCalculator { k, base }`.
   - `katgpt-pruners/src/proof/plackett_luce.rs`: Plackett-Luce rating via
     Gibbs sampling **expressly for Elo conversion** (Plan 128 T4) — an
     ORDER-FREE batch rating, the right fit for replaying a trajectory store
     (classic ELO is order-dependent; PL over the full match set is not).
   - `katgpt-core/induced_cwm/tournament.rs` L110: chess-ELO score
     conventions (win=1/loss=0/draw=0.5) already the house standard.
2. **RuliologySearch is static today** — `src/ruliology_search.rs`: axes are
   `lint_satisfied` (parses, or clippy_verify when built) + `ast_delta_size`
   (structural). Zero connection to the 8,236-bucket trajectory store — the
   front re-derives from corpus structure what real usage already measured.

## Design verdict

| Axis | Verdict |
|---|---|
| **Match semantics** | Rule-vs-SHAPE ratings (both sides rated): every heal attempt = the applied rule playing the span; win = kept+verified, loss = reverted/created-warning, draw = review-only. Span difficulty emerges as the opponent rating — the ruliology payoff: *which fix wins where*. NO counterfactual rule-vs-rule (alternatives' outcomes are unobservable — shadow-eval is a non-goal). |
| **Rating math** | Adapt `plackett_luce.rs`'s batch PL→Elo (order-free, replayable) + `arena`'s EloCalculator scale (base 1200/400). Reuse per the `ruliology_search.rs` precedent (adapt-with-citation, no new dep) — **or** promote ELO/PL to `katgpt-core` (2 in-stack consumers already + this = 3): file the katgpt-rs promotion issue at T1 time, pick by owner call. |
| **Store residency (038 seam)** | Ratings are DERIVED views over trajectory rows — recompute on load (PL batch over ≤~10⁴ rows is cheap) or maintain incrementally in the Pod row (`elo: f32` on the 038-T1 row schema — reserve the field NOW so the schema does not churn later). No separate rating store, no JSON. |
| **Ruliology grounding** | `ScoredCandidate` gains an evidence axis (tier-weighted success rate / ELO from the store): the front becomes (lint_satisfied, ast_delta_size, evidence). Payoff stays primary; evidence reorders among parse-equal candidates — where today's ties fall to arbitrary sort order. |
| **Selection** | **BetaPosterior stays DEFAULT** — Issue 026's measured precedent: starved pools (0–3 observations) punish every fancier selection mode (entropic lost 4 variants). ELO is FIRST a reported axis (audit leaderboard + ruliology report); it may challenge BetaPosterior for selection ONLY after beating it on the score bench with ≥N-observed pools (the same gate that killed entropic). |

## Tasks

- [ ] **T1 — substrate decision + reservation**: file the katgpt-rs ELO→core
      promotion issue (or record the adapt-locally decision with citation);
      reserve `elo: f32` (+ `span_elo: f32`? decide) on the 038-T1 Pod row
      schema so 038 lands once.
- [ ] **T2 — rating computation**: batch PL→Elo over trajectory rows on
      store load (038 backend); unit-gate vs hand-computed fixtures + the
      arena calculator on small cases (cross-check the two math paths).
- [ ] **T3 — ruliology evidence axis**: extend `ScoredCandidate` + front
      ranking; fixtures pin (a) evidence breaks parse-equal ties, (b) payoff
      still dominates (a parse-failing high-ELO candidate NEVER outranks a
      parse-passing one — correctness before rating).
- [ ] **T4 — audit/report surface**: `self_evolve_audit.sh` section 1b —
      ELO leaderboard (top rules, draws on review-only, Withdrawn excluded)
      via `--dump-trajectories`; this is the human-readable rendering.
- [ ] **T5 — selection challenge gate (measured)**: score-bench A/B,
      BetaPosterior vs ELO-weighted selection, at real pool-sparsity; promote
      ONLY on a win; record as a score_history delta + research note.
      A loss keeps ELO report-only (still valuable) — honestly recorded.

## Non-goals

- No shadow/counterfactual evaluation of unchosen rules.
- No online ELO updates inside the hot fix loop (batch-on-load only — the
  heal path stays allocation-stable, G4).
- Not blocked-on-by Proposals 003/004 (that chain runs through 037); but
  039's T5 result is the natural E4 evidence if it wins.

## Relationship map

- 038 (store) → this issue reads its rows; 038-T1 reserves the elo fields.
- 037-T2 (verdict wiring) → the win/loss feed; without it every rating sits
  at base 1200 (the honest pre-wiring state — the audit will show exactly
  that until 037-T2 lands).
- Issue 026 (entropic, negative) → the selection-gate precedent this issue
  inherits instead of re-learning.
