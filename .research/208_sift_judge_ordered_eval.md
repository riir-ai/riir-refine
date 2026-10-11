# Research 208 — SIFT: judge-ordered evaluation for recursive self-improvement (the cost-shape paper for Issue 130)

**Status:** DISTILLED — record + input to open `Issue 130`'s OOD-rig axis (T2 deferred by measurement 2026-09-23: δ = 0 on the deterministic oracle axis; the issue's live unblock path is the OOD rigs); no plan opened. No class-level novelty claimed.

**Date:** 2026-09-24
**Source:** [arXiv:2609.19526](https://arxiv.org/abs/2609.19526) — "Self Improvement via Fast Tree-search" (Fu, Kulanthaivelu, Yamada; MIT + Sakana AI; 2026-09-17, 24 pp)
**Related Research:** 203 (RRSI — the admission-gate issue this feeds), 135 (Elo selection NEGATIVE — the regime boundary), 167 (RSI survey), 178 (Schmidhuber keeper-half), 182 (frontier miner), katgpt-rs 040 (OpenDeepThink — the shipped `bt_rank` lineage), katgpt-rs 320 (Red Queen Gödel Machine — cited in the paper's own related work)
**Classification:** Private (healer-domain distill; home is riir-clippy per the MOAT table)

## TL;DR

SIFT replaces the expensive per-candidate benchmark evaluation that dominates recursive self-improvement (DGM/HGM lineage: ~$22k SWE-Bench evals, thousands of CPU-hours) with cheap pairwise LLM-as-judge comparisons aggregated by a **regularized Bradley-Terry model**; the **ranks** (never raw scores) drive an exponential rank-based parent sampler with a visit-count exploration penalty, P(i) ∝ exp(−α·r_b(i) − β·r_a(i) − η·log(1+v_i)), inside a disaggregated (async expansion ∥ evaluation) tree search. Polyglot 31.1–35.1% at ~1/10 DGM's CPU-hours; judge Spearman ρ=0.68 vs realized accuracy on full-file inputs (0.40 on diff chains); judge rank recovered failure modes invisible to benchmark scores (a gated-off verifier, a stateless "persistent" shell).

**Verdict: Gain (modelless track — record + input to Issue 130, no plan, no new issue).** Every mechanism class already ships: BT pairwise aggregation is `bt_rank` (katgpt-pruners, katgpt-rs Bench 011 GOAT-passed, λ-regularized, `BtScores::rank()` — even the ranks-not-scores interface), position-bias debiasing is `committee_boost/debiased_compare`, the k-way generalization is `proof/plackett_luce`, visit-count exploration is PUCT/bandit. The paper's residual value to us is (a) the **cost-shape argument** for Issue 130's admission-gate OOD-rig axis — cheap-signal-ordered expensive verification is a published pattern, and an ordering layer over a zero-spread axis orders nothing, so it attaches only to the stochastic axis the issue's 09-23 narrowing names as live; and (b) the **inversion finding**: our oracle/judge cost structure is the OPPOSITE of SIFT's premise, so the direct port (an LLM judge) is a non-starter — the transferable piece is the orchestration ordering only.

## 1. Source digest

| # | Mechanism | What it does |
|---|---|---|
| 1 | Pairwise LLM-judge + regularized BT | New candidate vs ≤K=10 strong incumbents; win matrix W; BT refit per round with pairwise pseudo-count λ (MM fixed-point, Σ-normalized); λ makes the objective strictly concave on the simplex (their Thm 1: unique maximizer — no degenerate scores for undefeated/sparse nodes) |
| 2 | Rank-based parent sampling | exp(−αr_b − βr_a − ηlog(1+v)) over the archive; ranks of BT strength + subset accuracy + visit decay; Prop 1 = geometric rank amplification; Prop 2 = no permanent starvation at any η≥0 |
| 3 | Disaggregated pipeline | Expansion and full evaluation run async; eval priority queue ordered by the same rank mixture; unevaluated nodes inherit parent accuracy for sampling |
| 4 | Full-files > diffs | Judge sees the final runtime files (byte-identical files omitted), not the patch chain: ρ 0.68 vs 0.40; swap-ordering adds ~nothing at 2× cost |
| 5 | Tiered judges | Weaker judge keeps global ρ (0.71 vs 0.72) but top-5 ordering agreement collapses (0.50 vs 1.00): cheap judge for bulk comparisons, strong judge for the frontier |
| 6 | Fixed eval subset | Random task subsets inflate nodes by luck-of-the-draw → fixed subset for comparability (our test-gate-floors posture, externally confirmed) |

Cost anatomy (their Table 1): Polyglot-50 full eval $6.00 + 2.6 CPU-h vs judge pair $0.044 + 0.0042 CPU-h — the ~136× signal-to-cost gap is the whole paper.

## 2. Pre-flight (what ran)

1. Scope: AGENTS.md context loaded for the product set + the healer. Not a training paper (no backprop anywhere — search + preference aggregation over harness code); no adversarial-panel trigger, no riir-train route.
2. `.research/` derived: katgpt-rs (to 585) + riir-clippy (to 207) read; closest cousins READ before verdict: **203, 135** (full), 320 (outline), katgpt-rs **040** (full).
3. Src trees via vocabulary grep across ALL workspace roots: `bradley|BradleyTerry|bt_rank`, `exp(−rank)|rank_sample|parent_sampl|visit_count` — every hit lives in katgpt-rs (katgpt-pruners `bt_rank.rs` READ; puct/bandit/configurator_bandit surface confirmed); riir-clippy's Elo surface (`elo.rs`, `tier_guard` `expected_score`) confirmed via AGENTS.md + Bench 047.
4. Prior-art searches run (§6) BEFORE the verdict.
5. Three-track grep: self-adaptive ships (`self_evolve`, `traj_store`, `elo`, `tier_guard`, `frontier_miner/report`, `active_set`); model-based lives elsewhere (TernaryDraftModel, quest_grammar); no LLM-judge-guided search loop anywhere.

## 3. Mapping table (SIFT mechanism → shipped cousin, signal-diffed)

| # | SIFT mechanism | Shipped cousin (read-verified) | Signal-diff → disposition |
|---|---|---|---|
| 1 | Regularized BT fit (MM fixed-point; pairwise pseudo-count λ) | `bt_rank::bt_fit` — gradient ascent, L2 λ=0.01, 100 iters, `BtScores::rank()/top_k()`, `fast_sigmoid` (katgpt-rs Bench 011 GOAT: BT > pointwise +10.6pp; > win rate τ +0.0157) | Same regularized-objective class; the delta is regularizer FORM: pair pseudo-counts give strict concavity on the simplex (their Thm 1) where L2-on-scores only bounds. **Discard (coverage)** — micro-delta recorded: if `bt_fit` ever sees degenerate undefeated-candidate fits on sparse graphs, the pseudo-count form is the drop-in repair. |
| 2 | Rank-based Boltzmann parent sampler (ranks of TWO signals + visit decay) | PUCT (`katgpt-moka-wasm/puct.rs`), bandit UCB (`bandit/mod.rs`), `best_belief_score` quantile select | Cousins consume VALUES + visits; SIFT consumes RANKS — deliberately scale-free because BT score magnitude varies with comparison topology while its ordering does not. **Discard (coverage at our scale)** — no archive we search samples parents (batch packs are 2 rules; the frontier store is a ledger, not a search frontier). Recorded as an extraction row with no consumer. |
| 3 | Swap-order position-bias debiasing | `committee_boost/debiased_compare.rs` (BtComparison/BtOutcome both orders) | Exact shape ships; SIFT's own ablation (Table 6: 0.68→0.67 at 2× cost) independently confirms debiasing's cost/benefit is context-dependent. **Discard (coverage + their ablation).** |
| 4 | Multi-candidate ranking judge (their prompt asks to rank N) | `proof/plackett_luce.rs` — "pairwise BT is the special case k=2" | **Discard (coverage)** — the k-way math ships. |
| 5 | LLM-as-judge transport | riir-llm ladder (groq→infron→CF Workers AI; `LlmRouter` policy) | Transport ships; consumer is the ops plane, never a heal/search path. See §4 — the INVERSION. |
| 6 | Disaggregated cheap→expensive ordering | coverage-report derivation → probe → pack → e2e → freeze-regen | Structurally the same staged cheap-to-expensive discipline, per batch instead of per node. **Discard (coverage at our cadence).** |
| 7 | Full-files > diffs judge input (ρ 0.68 vs 0.40) | `LatentMatcher` span chunks over `AstChunker` spans | The analog claim for OUR retrieval lane: judge/compare candidates on full post-fix file context, not fragments. **Record only** — a hypothesis for the retrieval axis, untested, and the oracle lane already sees full files (no gap today). |
| 8 | Theory: well-posedness / top-K recovery / no-starvation | `tier_guard` Lemma-4.2 two-sided ratio test; Bench 047 starvation law | Their Thm 2 (top-K recovery under sparse comparisons) and Prop 2 (visit-count no-starvation) are published support for the design space `tier_guard`/`bt_rank` already occupy. Validates design; not actionable (§1.55). |

## 4. The inversion finding (why the direct port is a non-starter)

SIFT's premise: ground-truth evaluation ($6 + 2.6 CPU-h) ≫ judge call ($0.044). The healer's premise is the **mirror image**: our ground truth — a clippy probe on a fixture — is seconds, free, deterministic; ANY LLM call (even the free-tier ladder) is slower, nondeterministic, and unverifiable. Replacing our cheap ground truth with an expensive noisy judge to save evaluation cost inverts the paper's economics. What actually transfers is only the **orchestration shape**: a cheap aggregation layer deciding WHICH candidates pay the expensive evidence tier — and in our stack the expensive tier is per-candidate benches (score-bench deltas, the `--necessity-bench --necessity-rule` pair), probe-verified e2es, and freeze-regen, not the oracle.

## 5. The regime map (where judge-ordered evaluation would and would not pay)

| Regime | Pool state | Verdict | Evidence |
|---|---|---|---|
| Span→rule selection at heal time | 98.5% of candidates unseen | **Never** — selection sophistication ≤0.04pp; evidence volume is the bottleneck | Bench 047 (R135) — the Elo-vs-BetaPosterior kill |
| Mining batch lead ordering | 2 rules/pack, handful of leads | Archive too small for BT aggregation to pay | batch discipline (Batches 155–171 cadence) |
| Issue 130 admission gate, OOD-rig axis | T2 DEFERRED BY MEASUREMENT (2026-09-23): repo stratum deep enough (21 findings) but δ = 0 — the deterministic oracle axis has no spread BY CONSTRUCTION (score-bench repeats REFUTED, do not run); live unblock = the OOD rigs, the only stochastic-class surface | **The one fit, on that axis only**: BT-rank accumulated modelless evidence (strata heal rates, `rule_outcomes`, probe outcomes), reserve the expensive OOD-rig evidence for the frontier — an ordering over a zero-spread axis orders nothing | this note + R203 + the issue's own path-(a)-LIVE narrowing |
| Rule demotion pricing (Issue 102) | dense per-rule counts in traj_store | Possible later; nothing demotes automatically by owner rule — out of scope while that stands | AGENTS.md `--active-set` law |

The tiered-judge finding (#5 in §1) is separately recorded for any future ladder consumer: bulk comparisons on the cheap tier, frontier ordering on the strong tier — the ladder (`Ladder::route`) can express this today if a consumer ever appears.

## 6. Prior art (§4 searches, run BEFORE the verdict)

Pinned claim (written before searching): *"Cheap pairwise-preference (BT-aggregated) intermediate signal ordering expensive evaluations in a recursive self-improvement archive, applied to a modelless code-healing corpus."*

- **Class level: KILLED three times.** (a) SIFT itself — the class instantiation. (b) Surrogate-assisted evolutionary computation with pairwise ranking (Harada et al., ELDR, 2022/23; Wauthier et al., ICML 2013 efficient ranking from pairwise comparisons; PICEA-globals 2013) — cheap surrogate ranks ordering expensive evaluations is a decade-old field. (c) BT aggregation for LLM judging (Zheng et al. MT-Bench 2023; Chatbot-Arena Elo lineage) — pervasive.
- **Named in the paper's own related work:** DGM, HGM, SICA, Red Queen Gödel Machine (katgpt-rs R320), Group Evolving-Agents, ADAS, AFlow.
- **Conjunction:** no published hit applies the ordering pattern where the ground-truth oracle is the CHEAP tier and the judge would be the EXPENSIVE one (§4's inversion) — but that conjunction is an argument against porting, not a moat. Same standing as R203's conjunction: fusion-grade, not class-novel.

## 7. Verdict

| Axis | Call |
|---|---|
| Tier | **Gain** (modelless track — the loop and every cousin runs in-process; serving-envelope native) |
| Novelty gate | Q1 NO (bt_rank ships + SIFT/surrogate-EA class) · Q2 NO (no new capability class — no per-candidate expensive-eval bottleneck exists at our cadence) · Q3 NO · Q4 connects pillars via this note only. **Not Super-GOAT, not GOAT.** |
| MOAT gate (riir-clippy, consumer-first) | PASS on fit — feeds the in-scope OOD-generalization axis (Issue 130) with zero new code |
| Ladder check | Priority #2 (healer) primary. Priority #1 (game): skill-library evolution with expensive rollout verification ordered by cheap preference is the analog; shipped cousins MUSE (katgpt-rs R172) + cgsp cover the class, and no dense expensive-eval skill archive ships — **no game filing** (same disposition as R203 §4). |
| Files | This note · redirect line on 203 · cross-ref on katgpt-rs 040 |

### Fusion (skill §1 step 6)

SIFT × RRSI (R203) × `frontier_report`: Issue 130 frames the admission gate, and its 2026-09-23 narrowing names the axis — the OOD rigs, the only stochastic-class surface (score-bench repeats are refuted by construction as δ sources). SIFT prices the ordering layer on exactly that axis: BT-rank candidate rules over accumulated modelless evidence (rules "compete" as pseudo-pairs on shared fixture families: i wins over j where i's fixed_code heals a span j's does not), reserve the expensive OOD-rig evidence for the BT frontier. The gate-vs-report question stays open behind T1's readout, and an ordering over a zero-spread axis orders nothing — the axis must move first. Bench 047's law still applies: the ordering pays only where per-rule evidence volume is dense enough to rank, which `rule_outcomes`/strata rows are and span pools are not. Second-order (recorded, not filed): SIFT's tiered-judge config × the riir-llm ladder for the ops advisory lane; SIFT's fixed-eval-subset argument as external support for test-gate floors.

> **PASS-Redirects (synthesis):** Fu, Kulanthaivelu, Yamada [arXiv:2609.19526 "Self Improvement via Fast Tree-search"] — the cost-shape companion to Issue 130's admission-gate axis: cheap pairwise BT-aggregated judge ordering expensive per-candidate evaluation (Polyglot 31–35% at ~1/10 DGM CPU-h; judge ρ=0.68 full-file vs 0.40 diff); direct port inverted by our cheap-oracle economics (§4), the orchestration ordering attaches to the OOD-rig axis only (§5).
