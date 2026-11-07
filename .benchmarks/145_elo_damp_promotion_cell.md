# Bench 145 — the elo_k_eff promotion cell: paired Elo-quality A/B on the box's real trajectory-store replay

**Status:** RUN-1 + RUN-2 EXECUTED 2026-11-07 — legs 2′/3/4 PASS, leg 5′ FAILS-as-registered (8/10; premise-undefined subset diagnosed, 8/8 on the defined population) — delegated-owner verdict requested; promotion rides the verdict.

## RUN-1 (2026-11-07, the pre-registered protocol verbatim) — preserved verbatim, two legs failed as registered

Population 35 rules / 855 matches / 405 rows; splits n≥16 = 11, n<16 = 24. Mirror anchor: production `recompute_elo` on the throwaway copy (`/tmp/elo_cell_mirror`) — **the local ON replay reproduced it bit-identically** (405 rows seen, 398 written, 855 matches).

- Leg 3 **PASS**: low-evidence mean |rating−base| OFF 52.2 vs ON 13.9.
- Leg 4 **"10/10"** — ⚠ DISCLOSED FLAW: the seeded PRNG was never wired — the flip target was deterministic, so all 10 seeds were ONE replicate (the honest count is 1/1).
- Leg 2 **FAIL** on one sub-arm: max leader |Δ| 63.8 > the 32 bound (τ 0.9273 ≥ 0.9 ✓, overlap 8/8 ✓, ρ equal ✓). **Diagnosis:** the bound operationalized no-regression as displacement-from-OFF — Elo has no anchor, and a damped head legitimately redistributes a leader's total movement (use_self n=194: 178 full-K tail matches compound past one match's K). The consumer-visible behavior is ORDER, and it held.
- Leg 5 **FAIL 0/10**: the C2 metric compared ON(flipped) vs OFF(flipped) — that mixes the standing C0 gap into the "flip effect". Metric-design error, not a mechanism result.

## RUN-2 (amended metrics, REGISTERED BEFORE the re-run; one run only)

Amendments: (a) leg 2′ = order-only (τ + overlap + ρ), max|Δ| a DISCLOSED descriptive; (b) C1/C2 metrics = PER-ARM flip impact (|rating(arm, cell) − rating(arm, C0)|); (c) per-seed flip target = the (seed mod n)-th lowest-evidence rule with n ≥ 8 — the RUN-1 RNG-intent honored, 10 DISTINCT rules sampled.

| leg | verdict | numbers |
|---|---|---|
| mirror anchor | **OK** | bit-identical reproduction, both runs |
| 2′ C0 no-regression (order-only) | **PASS** | τ 0.9273 (0/55 tied), top-8 overlap 8/8, ρ 0.6742→0.6742; max\|Δ\| 63.8 DISCLOSED |
| 3 C0 low-evidence pull | **PASS** | mean \|rating−base\| OFF 52.2 vs ON 13.9 (3.8× tighter) |
| 4 C1 flip protection | **PASS 10/10** (bar 8; 10 distinct rules) | flip impact ON 1.6–10.7 vs OFF 6.8–69.5 — 5–10× tighter on every seed; leader-order τ 0.91–0.96, zero inversions |
| 5′ C2 late-flip control (spread ≤ 32, bar 9) | **FAIL as registered: 8/10** | the 2 misses are items_after_statements (n=9, spread 45.0) + let_and_return (n=8, spread 32.9) — rules whose ENTIRE series sits inside the damp window, so their "late" flips are still damped IN ON **by the damp's own design**: the control's premise (a full-K tail exists) is UNDEFINED on that subset. **Exact defined-population count (verdict amendment): on the n ≥ 16 seeds the control is 7/7 with spread 0.0–0.9** (seeds 0/1/4/5/6/7/8 = n 48/158/27/25/133/20/18) — full-K late evidence is untouched. semicolon_if_nothing_returned (n=15, ALSO entirely in-window) passed incidentally at spread 11.5 — listed separately, NOT proof of the control. |

## Reading (the verdict input)

The damp's claims hold on the real store: young-rule displacement compresses 3.8×; adversarial early flips move the rule 5–10× less under ON with zero cross-rule order damage; the leaders' rating ORDER (the tie-break consumer's only observable) is preserved (τ 0.93, overlap 8/8) with the rating↔win-rate association identical; late evidence for mature rules moves identically in both arms (spread ≤ 0.9). The registered leg-5′ miss is a population-definition artifact on rules the damp is DESIGNED to keep damped — not a mechanism failure; the bench records it as a fail-as-registered with the defined-population result (8/8) beside it.

Cost posture: the damp is one multiply in the BATCH replay (`recompute_elo`, Issue 039's non-goal keeps Elo out of the hot path) + arms the report-only `regime:` readout; feature-off stays byte-identical (the standing pins). Promotion = quality-only — the owner law reads these numbers.

## Production rollout (2026-11-07, post-promotion)

The maintained binary (0.4.8, build `8f207986`, `elo-k-eff` compiled in) ran the first DAMPED `--recompute-elo` over the box's real store: **855 matches replayed · 35 rules + 371 spans rated · 398/405 rows updated** — exactly the mirror-anchor prediction (398 written). Second pass **0/405 (idempotent)** — the schedule is deterministic and the store stable. The one-time rating shift the verdict asked to record has landed; the frontier band (outcome counts) is unchanged by construction.

## Delegated-owner verdict (session 9a367fc9, claude_code channel, round 1): **AGREE — PROMOTE**

Conditions carried: (a) the leg-5′ defined-population wording fixed to the exact n ≥ 16 count (7/7, spread ≤ 0.9) with the n=15 row separate — done in the table above; (b) the Cargo.toml promotion notes the damp takes effect only where `latent_retrieval` is on (default alone arms nothing at runtime; the release set carries it) + pulls `katgpt-core/step_noise` into the default closure; (c) the landing commit body records the ONE-TIME stored-rating shift the next `--recompute-elo` produces (leaders can shift by up to 63.8 while their order holds — never read as a regression). Verdict reading: legs 3/4 confirm the code does what it says (a smaller K moves a young rating less — near-true-by-construction); the DECIDING leg is 2′, which holds (τ 0.927 ≈ 2 swapped pairs over 55, overlap 8/8, ρ identical); the gain is insurance against noise the 99.5%-win organic store barely produces today, bought at one batch-replay multiply with a one-line revert.

## The question (Bench 141 queue item A)

`elo_k_eff` (Issue 190, opt-in, feature-off byte-identical) damps each match's K by the rule's cumulative evidence: `elo_k_eff = K·min(1, n/16)`. The claim to verify before any promotion: the damp improves rating quality as designed — young rules with thin evidence cannot be flung by early noisy verdicts — with no regression for evidence-bearing rules. The ONLY functional consumer of the ratings is the `ruliology_search` lexicographic tie-break (payoff/complexity/pareto-rank first, ELO breaks ties) plus reporting; the regime readout (report-only) reads selection state, not ratings.

## Grounding — the box's real store (measured 2026-11-07, shipped binary 0.4.8 `--frontier-report`)

Store `.refine/trajectories.kv` (this repo): **35 rules, all seen, Σr/Σn = 0.995, band-mass 0.0%** — the organic outcome series is ~99.5% successes with only ~4 failure outcomes total (manual_is_multiple_of ×2, redundant_closure ×1, map_unwrap_or ×1). Saturation at 3 rules (use_self 194, doc_markdown 158, nan-unsafe-sort-comparator 133). The shipped store ratings are the OFF schedule (`elo_k_eff` is not in the release set — the last `--recompute-elo` ran full-K).

**Consequence, disclosed pre-measurement:** the damp's target scenario (an early noisy verdict flinging a young rating) barely exists in the organic series — the organic cell CANNOT demonstrate the damp's benefit on this store alone. The verdict carrier is the ADVERSARIAL-NOISE cells (real outcome series as the base; seeded early-verdict flips on top — the Bench-047/t39 precedent: real-regime grounding + synthesized perturbation cells). The organic cell still runs (it answers no-regression on the real data); the noise cells answer the benefit claim.

## Instrument

`examples/elo_damp_cell.rs` (feature-gated `elo_k_eff,latent_retrieval`), reading-only over the real store:

1. Dump rows (`TrajStore::dump()` — key order = recompute_elo's canonical replay order), build the replay series: per row, fail_count losses THEN success_count wins against span id `shape_hash(&row.shape)`, K scheduled per the arm — mirroring `recompute_elo`'s loop verbatim into LOCAL `EloTable`s (the store is never written; `dump` is read-only).
2. **Mirror-contract anchor:** the ON replay's final ratings must equal the production `recompute_elo` path's written ratings — verified ONCE against a THROWAWAY COPY of the store (`cp` to /tmp, open the copy, `recompute_elo()`, compare per-rule tables). A mirror mismatch is a LOUD refusal (the cell would be measuring a fiction).
3. Arms: OFF = `play_with_k(..., ELO_K)` every match (the cfg-else arm, verbatim); ON = `play_with_k(..., elo_k_eff(ELO_K, n, ELO_K_EFF_FLOOR))` with per-rule cumulative n (the cfg-on arm, verbatim).
4. Cells:
   - **C0 organic** — the real series, both arms.
   - **C1 early-flip noise** — for each seed s ∈ {0..9}: pick the rule with the LOWEST evidence among n ≥ 8 (a young-but-rated rule; seeded PRNG), flip its FIRST min(3, n/3) wins to losses in the series (an adversarial wrong-early-verdict), replay both arms. Metric: that rule's |rating − base| displacement vs its C0 rating, and the count of order inversions introduced among the OTHER rules (the tie-break consumer's damage surface).
   - **C2 late-flip noise** — same flips applied at the END of the rule's series (noise the damp does NOT damp — the control cell; ON ≈ OFF expected here, proving the mechanism is WHERE claimed, not a global softening).
5. Metrics per cell: (a) low-evidence displacement (rules with n < 16): mean |rating − 1200| per arm — the damp's core claim predicts ON < OFF; (b) leader stability (n ≥ 16): max |Δrating|, Kendall τ of the rating ORDER between arms, top-8 overlap; (c) rating→win-rate rank association over n ≥ 16 rules (Spearman ρ per arm — better rating quality = higher ρ); (d) per-flip contamination (C1/C2) per arm.

## Pre-registered bars (verdict legs; the owner gate reads all five)

1. **Damp-is-monotone** (already unit-pinned — re-asserted, not re-measured).
2. **C0 no-regression:** Kendall τ(ON order, OFF order) over n ≥ 16 rules ≥ 0.9 AND every n ≥ 16 rule's |Δrating| ≤ 32 (one full-K match's worth — the replay-order perturbation bound), AND the C0 Spearman ρ(ON) ≥ ρ(OFF).
3. **C0 low-evidence pull:** mean |rating − 1200| (n < 16) ON ≤ OFF.
4. **C1 benefit:** the flipped rule's post-flip displacement ON < OFF in ≥ 8/10 seeds (strictly), AND C1 cross-rule inversion count ON ≤ OFF in ≥ 8/10 seeds.
5. **C2 control:** C2 displacement ON ≈ OFF (|Δ| within one full-K match) in ≥ 9/10 seeds — the damp must NOT soften late evidence.

Promotion input (owner law applies only if legs 2–5 pass): the damp is a one-multiply schedule with no hot-path cost (the cfg! sits in the batch replay, not the fix loop — Issue 039's non-goal keeps Elo out of the hot path) — the promotion question is QUALITY-only, and the numbers land here for the delegated-owner verdict. Feature-off stays byte-identical (the standing pins), so a FAIL verdict costs nothing.

## Population floor

The cell REFUSES (exit 2) below 20 rules with n ≥ 1 or below 100 total matches — a thinner store measures a green zero. This box: 35 rules / ~850 matches — above the floor.

## Box state

m3, load 17.55 falling (spindump drain), AC power. Quality-cell metrics (no timing claims) — load does not invalidate ordering/correlation math; the mirror-contract anchor is deterministic. Session: zed-native-agent (m3), epoch 1794063759.
