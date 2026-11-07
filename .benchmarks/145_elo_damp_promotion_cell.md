# Bench 145 — the elo_k_eff promotion cell: paired Elo-quality A/B on the box's real trajectory-store replay

**Status:** OPEN — protocol PRE-REGISTERED before measurement (the Bench-141 queue item A discipline); the instrument runs AFTER this registration is committed. Owner gate stays closed until the numbers land here.

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
