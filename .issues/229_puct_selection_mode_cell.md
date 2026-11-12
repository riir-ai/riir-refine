# Issue 229: SelectionMode::Puct — prior×evidence candidate ordering (the crossover cell)
**Status:** CLOSED MEASURED-NEGATIVE 2026-11-12 (amo idle lane, plan 368) — T1–T5 executed per the pre-registration (verdict AGREE ×2 + verdict round-1 REVISE, session a8204575); the round-1 decision rule fired NOT PROMOTED (ρ̂_upper 0.31 < ρ* 0.544) and §8 withdrew the mode — no zombie mode. Re-open conditions priced in T5. **Lease closed:** amo-idle-i229, 1794492600, was expires 2026-11-14T14:10:00+00:00 — landed in the window.

## Claim

A PUCT-shaped selection mode for `select_best_candidate`, consuming **Q** (the incumbent evidence score) and **P** (the modelless prior) with a visit-count exploration bonus: `score(cand) = Q(cand) + c_puct·P(cand)·√(ΣN_pool)/(1+N(cand))` where `Q(cand) = W_EVO·evo + W_RATE·reliability` is today's incumbent combined score (reliability per the incumbent mode) and `P(cand)` is the prior normalized to [0,1] within the pool. Distinguished from the incumbent modes (`BetaPosterior`/`LinearBlend`/`Tether` — exploitation-only; `Entropic` — pool tilt, no counts) by a bonus that is largest at thin evidence and decays per candidate as `N(cand)` grows, so selection shifts from prior-driven to record-driven per candidate — the per-candidate crossover. Class prior art acknowledged (AlphaZero PUCT; `PUCBSelector` paper defaults c=0.2/K=64): the claim is composition + framing, not mechanism novelty.

## Live-state verified this session (2026-11-12)

- `SelectionMode` enum + `with_selection_mode` builder ship (`src/self_evolve.rs` L153+ / L388); `BetaPosterior` is `#[default]` (L180) — at `c_puct = 0` the new mode's score reduces EXACTLY to the incumbent combined score, so flag-off (and c=0) byte-identical is structural, not a promise.
- `W_EVO=0.6`/`W_RATE=0.4` consts (L132-138); `Entropic` = KL-budgeted Boltzmann tilt (L182-185, Issue 026); `Tether` = adaptive global blend (`selection_tether.rs`, Issue 033).
- **`select_best_candidate` is single-pick** (returns one `(usize, f32)`, `src/self_evolve.rs` ~L495); the caller validates that one pick and a miss goes to `resolve_miss` → L4. **No ranked retry-loop exists.** This cell does NOT add one — the L4/miss path is untouched, recorded as deliberate scope; any future retry loop is a separate task with its own cost axis.
- `BetaPosterior` carries a **documented crossover** (L172-179): regret advantage −23.8% / −14.4% / −6.8% / −1.9% at max attempts 1 / 3 / 5 / 10, reversing to +2.9% at 30 — a known large-N reversal a PUCT arm must be separated from (T3's LinearBlend arm).
- **Bench 028** (`.benchmarks/028_t93_selection_regret.md`) is the gate precedent: mean selection regret over ≥20,000 randomized trials, 1–5-attempt regime, promoted BetaPosterior at −8.4% mean / −23.8% single-attempt. T3 mirrors this harness shape.
- `traj_store::shape_conditional_score(store, code, strategy_id, shape)` (L2162) — the evidence axis has a shape-conditional read already.
- `katgpt-pruners::state_source::PUCBSelector` — generic `(id, score, visits)` selector, **zero riir-\* consumers** (grep this session; only riir-ai Plan 141's dependency graph names it) — the goat-audit stall this filing closes either by consumption or by recorded negative.
- Standing priors: rustc_errors nonergodic ordering lane **CLOSED NEGATIVE** (Issue 093, `nonergodic_order.rs` — rotation-style exploration ordering); `Entropic` shipped, never displaced the default.

## Tasks

- [x] T1: implement `SelectionMode::Puct` — **EXECUTED then WITHDRAWN per
  the demotion law (see T5)**. The mode existed through the full
  pre-registered cell (G1 byte-identical anchors at mirror AND pipeline
  seams; dose-response monotone; two-pass scoring over reused scratch;
  pool ≤ 1 loud degradation) and was removed by §8 after the round-1
  REVISE decision rule fired. `BetaPosterior` stays the default; the
  enum site carries the withdrawal note + the re-open conditions.
- [x] T2: dose-response arm — **EXECUTED**: c ∈ {0 (EQUAL — G1 anchor
  green over 80,000 replay episodes + the 360-answer pipeline seam), 0.2
  (PUCB default), 2.0}; exploration rate 0.0000 / 0.115 / 0.509 —
  monotone in c as pre-registered. **The dose finding: c=2.0 is
  catastrophic at every ρ (+84.7% worse than the incumbent even at the
  oracle prior) — any future re-open carries a hard dose ceiling, c=0.2
  the only measured-good dose.**
- [x] T3: paired cell on a **sequential replay harness** — **EXECUTED**
  (2,500 instances × 80,000 episodes: thin 20k + dense 60k; common random
  numbers; counts update per episode; P synthesized at known correlation
  `ρ·p + (1−ρ)·z`, swept). Thin-phase mean regret vs BetaPosterior
  (0.03412) at c=0.2: ρ=0.0 −60.8% · 0.3 −30.8% · 0.4 −18.3% · 0.5 +3.4%
  · 0.6 +18.3% · 0.7 +27.8% · 1.0 +39.6% — the +10% bar first clears at
  **ρ* ≈ 0.544** (linear interpolation). The replay passed its raw bars
  at ρ ≥ 0.7 and regressed below 0.4 — the forking-paths shape the
  verdict round-1 REVISE refused to read as a pass.
- [x] T4: falsifier arms — **EXECUTED, both have teeth**: (a) shuffled-
  prior control: honest ρ=0.7 regret 0.02465 vs shuffled 0.06915 (+103%
  vs beta) — the gain vanishes without prior information (the mechanism
  is the prior, not exploration noise); (b) Entropic comparison:
  separated (0.03415 vs best-PUCT 0.02061) — the crossover framing is
  not an Entropic re-skin.
- [x] T5: verdict + demotion-law execution — **NOT PROMOTED, §8
  WITHDRAWN (2026-11-12, verdict round-1 REVISE + the pre-registered
  decision rule)**. Chain: the raw G2 bar (+39.6% ≥ 10%) was a
  forking-paths read (the verdict rejected taking the best ρ cell);
  the round-1 folds — the real-stream ρ̂ instrument
  (`examples/rho_real_stream`, this box's durable store: **Spearman
  +0.24 / Pearson +0.31 at N≥3, an inflated UPPER bound** — only picked
  candidates get outcomes; the shipped binary's learning half is
  memory-only so the fresh-process prior is the neutral 0.5), the
  ρ ∈ {0.4, 0.5, 0.6} locator cells at the fixed dose, the
  pre-registered rule (upper < ρ* ⇒ withdraw; lower ≥ ρ* ⇒ pin;
  straddle ⇒ withdraw) — resolved to **ρ̂_upper 0.31 < ρ* 0.544 ⇒
  NOT PROMOTED**. `SelectionMode::Puct` withdrawn (variant, scoring
  path, builder, constants, both harness binaries removed; the ρ̂
  instrument kept as the re-open's first read). **Re-open conditions
  (Research 396 §8 as priced by this cell): a real prior ≥ ρ* ≈ 0.54
  on the deployment stream — the named candidate is the seat-rerank
  head (Bench 119) — with a NEW pre-registered cell at the hard dose
  ceiling (c=0.2), never a re-flag of this one.** The lane's durable
  contribution is the crossover-law pricing + the ρ̂ instrument. The
  goat-audit row closes RECORDED NEGATIVE (Research 396 §7's
  consumption-or-negative satisfied).

## Pre-registered GOAT gate

- **G1 correctness:** `c_puct = 0` produces byte-identical ordering to the incumbent mode (structural: the bonus vanishes and Q IS the incumbent score); selection deterministic given store state; default untouched; tests pin N=0 candidates ordered by Q + full bonus.
- **G2 perf (the pass bar):** ≥10% relative reduction in mean selection regret vs the `BetaPosterior` incumbent over the 1–5-attempt regime on the replay harness (≥20k trials, Bench 028 shape). Below bar ⇒ demote per §8. Report-only: the ≥30-attempt end, where the bonus is exhausted.
- **G3 no regression:** at the ≥30-attempt end, Puct stays within the incumbent's own documented crossover band (the LinearBlend-vs-BetaPosterior envelope, L172-179) — a PUCT "win" that is just the known large-N reversal fails here; LinearBlend arm adjudicates. Untouched-behavior sweep clean; L4/resolve_miss path untouched.
- **G4 alloc:** selection path alloc-free (bounded scratch; no per-candidate heap).

**Honest expectation, pre-registered:** pools are often small and the neighboring exploration lane (Issue 093) measured negative — "no measurable gain at small n" is a likely outcome and is the pre-registered demotion, not a failure of the filing.
