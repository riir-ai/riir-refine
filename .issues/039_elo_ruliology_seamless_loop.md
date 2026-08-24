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

- [x] **T1 — substrate decision + reservation**: file the katgpt-rs ELO→core
      promotion issue (or record the adapt-locally decision with citation);
      reserve `elo: f32` (+ `span_elo: f32`? decide) on the 038-T1 Pod row
      schema so 038 lands once.
      **DONE 2026-08-24.** Both halves:
      - **Substrate:** the promotion issue is filed — katgpt-rs `686`
      (`b5cfd29d`; ELO/PL → `katgpt-core`, the 3-consumer argument — arena +
      PL in-tree + this issue's T2). Until 686 lands, T2 proceeds
      adapt-locally with citation (the `ruliology_search.rs` precedent) and
      switches to the core primitive on close.
      - **Reservation:** `elo: f32` + `span_elo: f32` on `TrajRowDoc`
      (serde-defaulted at base 1200 via `default_elo()` — the arena/PL shared
      convention; pre-039 rows decode unrated, test-pinned). `span_elo`
      DECIDED IN: the match semantics rate both sides (span difficulty = the
      opponent rating); stored denormalized per-row until T2 decides whether
      spans get their own derivation.
- [x] **T2 — rating computation**: batch PL→Elo over trajectory rows on
      store load (038 backend); unit-gate vs hand-computed fixtures + the
      arena calculator on small cases (cross-check the two math paths).
      **DONE 2026-08-24** (adapt-locally per the T1 decision — standard
      Elo, K=32, base 1200/400; PL stays the katgpt-rs 686 upgrade).
      - **Math module `src/elo.rs`** (always compiled, pure f32):
      `expected_score`/`update_pair` (the arena `EloCalculator` formula,
      cited) + `EloTable` (string-id rating table). Unit gates: the exact
      ±16 hand fixture (E=0.5 at equal ratings — exactly representable),
      zero-sum, upset-sensitivity, determinism, the f64 arena-reference
      cross-check over a mixed 12-match sequence (<0.01 drift — the
      "two math paths" gate), + the PL-consistency fixed point (a pair at
      75% win rate equilibrates at gap 400·log10(3), the PL conversion's
      own curve — standard Elo and PL agree at equilibrium).
      - **Store pass `TrajStore::recompute_elo`** + `EloRecomputeSummary`:
      canonical-order batch replay (rows in key order — BTreeSet range;
      losses-then-wins within each row, the documented modeling choice —
      the store keeps aggregates, not event order), final ratings written
      back denormalized on EVERY row of each player (any row reports the
      rule's current rating — the T3/T4 consumer contract). Idempotent
      (bit-identity re-derivation writes 0 rows); the ONLY rating writer
      (record/record_applied never touch elo — the non-goal pin, G4).
      Apply-only rows stay at base 1200 (no matches).
      - **CLI `--recompute-elo`**: standalone write mode (latent_retrieval
      builds) — backfill on demand; prints the pass summary.
      - **Backfilled the three real stores**: riir-clippy (9 matches, 2
      rules, 8 rows), riir-dapps (3 matches, 1 rule, 2 rows), riir-ai
      (112 matches, 7 rules, 101 rows) — all idempotent on re-run. The
      emergent signal is real: `uninlined_format_args` 1625 (62 rows),
      `match_bool` 1361, `map_unwrap_or` 1337; the spans that BEAT rules
      (0W/2L `score_fn`/`state.first()` shapes) rate UP at ~1231-1241 —
      span difficulty emerging as the opponent rating, the ruliology
      payoff. Tests: 7 elo + 4 store (backfill/idempotent/durable, final-
      rating denormalization, pre-039 decode-unrated→backfill-rated, the
      batch-only non-goal pin) — 744 latent / 669 default lib, clippy 0
      both states + bin.
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
