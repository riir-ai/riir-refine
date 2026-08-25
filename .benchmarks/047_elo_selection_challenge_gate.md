# Bench 047 — Issue 039 T5: the ELO selection challenge gate (measured)

**Date:** 2026-08-25
**Verdict:** **ReportOnly — ELO does NOT replace BetaPosterior for selection.**
No variant achieves the strict win the issue's design verdict demands; ELO
stays exactly where T2–T4 put it (audit §1b leaderboard + ruliology
tie-break; zero selection wiring). The result is NOT a clean kill like
entropic (Bench 035) — it is a no-strict-win with three load-bearing
findings, recorded below.

**Harness:** `tests/t39_elo_selection_ab.rs` (8 tests, deterministic,
self-verifying — the live paired cells re-derive the ReportOnly verdict on
every run).

```bash
cargo test --release --features latent_retrieval --test t39_elo_selection_ab -- --nocapture --test-threads=1
```

## The gate question

Issue 039's design verdict: *"BetaPosterior stays DEFAULT … ELO is FIRST a
reported axis; it may challenge BetaPosterior for selection ONLY after
beating it on the score bench with ≥N-observed pools (the same gate that
killed entropic)."* T5 is that challenge.

**Promotion rule (encoded as `promotion_gate`, branch-tested):** promote only
on a strict win — discovery strictly greater in BOTH evo-noise cells AND
paired McNemar `z > +1.96` in both AND no regret regression.

## Grounding — the REAL evidence regime (not t26's assumed one)

t26/t33 drew pre-observations uniform `0..=3` — ~40× denser than reality. T5
draws from the measured store distribution (2026-08-25, 8228 rows across
riir-ai 101 / riir-game-sdk 8117 / riir-clippy 8 / riir-dapps 2):

| S+F per (rule, fix) row | 0 | 1 | 2 | 3 | 4 |
|---|---|---|---|---|---|
| rows | 8106 | 110 | 9 | 2 | 1 |
| share | **98.5%** | 1.3% | 0.1% | — | — |

The rule-level aggregation asymmetry is measured too (ELO keys by rule and
aggregates across spans; the Beta counts key per (rule, fixed_code) and stay
thin): real per-rule match counts `[86, 17, 15, 6, 2, 2, 2, 1]` —
`uninlined_format_args` 86W/0L → 1625.3 at rule level while its per-fix
candidates carry 0-1 observations.

**Arms:** `LinearBlend` (raw-rate baseline) · `BetaPosterior` (incumbent) ·
`EloStatic` (ratings replayed from the SAME pre-observations, frozen during
the trial — the exact `TrajStore::elo_table()` batch-loaded form) ·
`EloOnline` (the same + per-attempt match updates). Dense cells add the
production-form asymmetry: ELO sees dense rule history (replayed at the
rule's long-run rate, popularity from the real distribution), Beta keeps
thin per-fix counts, current-span truth deviates by `span_dev`.

## Measured — headline cells (real sparsity, equal evidence)

TRIALS=20,000/cell, BUDGET=3, t26 pool shapes, seed `0x5EED_0039`:

| evo_noise | arm | discovery | mean-attempts | first-pick regret |
|---|---|---|---|---|
| 0.6 | LinearBlend | 0.9013 | 1.598 | 0.0300 |
| 0.6 | **BetaPosterior** | **0.9185** | 1.558 | 0.0292 |
| 0.6 | Elo-static | 0.9160 | 1.562 | 0.0291 |
| 0.6 | Elo-online | 0.9191 | 1.557 | 0.0291 |
| 0.2 | LinearBlend | 0.9190 | 1.527 | 0.0039 |
| 0.2 | **BetaPosterior** | **0.9399** | 1.482 | 0.0028 |
| 0.2 | Elo-static | 0.9399 | 1.482 | 0.0027 |
| 0.2 | Elo-online | 0.9398 | 1.482 | 0.0027 |

## Measured — paired McNemar (200,000 trials, seed `0x9A17_0039`)

| evo_noise | challenger | rate vs incumbent | b/c discordants | z | reading |
|---|---|---|---|---|---|
| 0.6 | Elo-static | 0.9189 vs 0.9203 | 375/663 | **−8.94** | significantly WORSE |
| 0.6 | Elo-online | 0.9207 vs 0.9203 | 233/161 | **+3.63** | better, +0.0004 absolute |
| 0.2 | Elo-static | 0.9423 vs 0.9422 | 209/202 | +0.35 | NS |
| 0.2 | Elo-online | 0.9423 vs 0.9422 | 77/75 | +0.16 | NS |

**Gate application:** `EloOnline` fails (z=+0.16 at 0.2 — no significant
win); `EloStatic` fails (z=−8.94 — significantly worse). → **ReportOnly.**

## Measured — dense rule evidence (the production form)

TRIALS=20,000, seed `0x0D3E_0039`:

| span_dev | evo_noise | Beta | Elo-dense | Elo-dense-online | z (dense vs beta) |
|---|---|---|---|---|---|
| 0.2 | 0.6 | 0.9173 | 0.9207 | 0.9216 | **+4.09** |
| 0.2 | 0.2 | 0.9396 | 0.9389 | 0.9393 | −1.52 (NS) |
| 0.6 | 0.6 | 0.9065 | 0.9103 | 0.9110 | **+4.06** |
| 0.6 | 0.2 | 0.9273 | 0.9264 | 0.9259 | −1.14 (NS) | |

The dense form wins only in the harsh-evo regime (where the evo axis is
unreliable and dense rule history substitutes for it) and is NS-to-slightly-
negative in the mild regime — **regime-dependent, not robust → no
promotion** on this evidence either.

## The three findings (the honest record)

1. **At real sparsity, the selection axis barely matters.** 98.5% of
   candidates are unseen → all pools are decided by the evo axis identically
   (pinned by `t39_unseen_pool_all_arms_pick_identically`); the discordant
   mass is 0.2–0.5% of 200k trials and effect sizes are ≤0.04pp absolute.
   The Issue 026 starved-pool doctrine is CONFIRMED with store-measured
   numbers: the bottleneck is EVIDENCE VOLUME, not selection math.

2. **The store's exact batch form is the significantly-WORSE arm (z=−8.94).**
   Mechanism: at real sparsity the pre-observations carry almost nothing, so
   the arms differ almost entirely in WITHIN-TRIAL learning — Beta updates
   counts per attempt; frozen ratings cannot. (Corollary: IF ELO selection
   is ever revisited, it must be the online form — never a frozen table.)

3. **No variant achieves the strict win.** `EloOnline` is the best
   challenger (a real but microscopic +0.04pp at noise 0.6, z=+3.63; NS at
   0.2); the dense production form is regime-dependent (+4.09 harsh-evo,
   NS negative mild-evo). BetaPosterior's T9.3 promotion survives the
   real-sparsity re-test (margin over LinearBlend holds: 0.9185 vs 0.9013 /
   0.9399 vs 0.9190).

## What stays / what is void

- ELO stays **report-only**: audit §1b leaderboard + the ruliology evidence
  tie-break (T3/T4 surfaces unchanged); no `SelectionMode` variant, no
  pipeline wiring.
- The issue's "record as a score_history delta" half is **void on a loss**
  (and would be void on a win too, honestly): the clippy score-bench's
  `heal_fixpoint` does not route through `select_best_candidate` at all, so
  no selection change can move the score-bench curve. This gate's metric is
  the selection-bench harness itself.
- The harness is kept as the reproducible negative-result artifact (the
  Issue 026 / `gemma2_directions` precedent); recorded constants +
  `t39_recorded_verdict_is_report_only` pin the decision; the live gate
  assertion re-derives it every run.

## Reopen condition

The natural trigger: **store densification**. The gate's regime is set by
`REAL_ROW_COUNTS` — when per-candidate evidence grows (say the median
candidate carries ≥2 observations, i.e. verdict-bearing rows dominate the
store), the selection axis starts to matter and this gate should re-run with
the then-current distribution. Second trigger: the evo axis degrading (if
evolved directions stop tracking span quality, the dense-ELO form's
harsh-evo win becomes the relevant regime).

## Validation at close

- 8/8 harness tests (release, latent_retrieval, single-threaded recorded
  run + parallel re-run).
- Live-store grounding report confirms the frozen constants against the
  current stores (108 rated players in riir-ai; standings match T4's audit
  §1b: 1625.3 / 1360.7 / 1336.6 …).
- Full three-state validation (lib tests / clippy `--all-targets
  -D warnings` / bin tests) at commit.
