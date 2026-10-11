# Research 203 — RRSI: regularized recursive self-improvement (the keeper discipline, empirically pinned)

**Status:** DISTILLED — pending owner decision (`Issue 130` filed; no plan opened). No class-level novelty claimed — the paper itself plus the GP parsimony lineage are the prior art; this note records the dense shipped-cousin mapping and exactly one conjunction gap.

**Date:** 2026-09-22
**Source:** [arXiv:2609.24972](https://arxiv.org/abs/2609.24972) — "RRSI: Regularized Recursive Self-Improvement of Agent Harnesses" (Xia, Han, Wang, Chen, Zhang, Lee, Huang, Yu, CuiZhu, Ming, Yao, Gokturk, Pfister, Lee; Google Cloud AI Research + UNC-Chapel Hill + Stanford + WashU; 2026-09-21, 24 pp; code: github.com/google-research/rrsi)
**Related Research:** 167 (RSI survey ladder — the positioning audit), 178 (Schmidhuber RSI essay — the keeper-half distill, issue 122), 182 (frontier miner PoC), katgpt-rs 012 (TRT — internal RSI-overfitting risk note, 2025-06), katgpt-rs 512 (Meta^n RSI, PASS)
**Classification:** Private (healer-domain distill; home is riir-clippy per the MOAT table)

## TL;DR

RRSI treats LLM-agent harness evolution (prompts, control flow, tools, memory, context management around a frozen backbone) as recursive self-improvement and shows the unregularized loop overfits: evolve-set gains (up to +14.1) shrink or invert out-of-distribution — two of four prior baselines finish BELOW the unevolved harness. Seven regularizers flip the trade: annealed edit budgets (L0 analogy), evidence-aware credit assignment, stall-triggered exploration, leakage screening, a noise-adjusted acceptance floor (δ calibrated from repeated baseline evals), gain-proportional cost acceptance (ΔC ≤ β₀+β₁ΔS), and report-only structural pruning (L1). Result: the smallest evolve-split gain of any arm, the only OOD average clearing H₀ (43.6 vs 39.7 baseline avg; up to +4.7), on ~30% fewer policy tokens than unregularized evolution.

**Verdict: Gain (modelless track).** The healer IS our RSI system, and all but one of the seven regularizers already ship as cousins — four as discards (A budget: our batches already sit at the floor; B credit: `tier_guard`'s two-sided ratio test, strictly stronger on the credit-attribution axis — a different falsified object than RRSI's rejected-edits, architectural reasoning not measured; C stall: the coverage-report re-arm, manual where theirs auto-detects; G pruning: `frontier_report`'s contribution-fade — the ONE exact-shape cousin, same report-only design, landed the SAME DAY the paper hit arXiv, 2026-09-21, zero-day delta, independent by read-date 09-22). The ONE conjunction gap is **admission-time generalization gating** — a δ-calibrated floor on the repo-mined heal stratum plus within-band novelty/cost credit — filed as Issue 130 as a DESIGN DECISION (the gate's evidence costs a per-candidate bench delta the current batch discipline does not pay). Not Super-GOAT: Q1 dies at class level twice. The ablation table is published, peer-shaped support for the workspace's own floors/ratchets/report-first discipline.

## 1. Source digest

| # | Mechanism | Side | What it does |
|---|---|---|---|
| A | Annealed update sparsity (L0 analogy) | proposal | b_t = b_min + (b_max−b_min)·½(1+cos πt/T) — cosine-annealed cap on atomic edits bundled per candidate; early rounds broad, late rounds sparse/attributable |
| B | Evidence-aware credit assignment | proposal | every evaluated edit recorded (component, hypothesis, diff, ΔS, ΔC, accepted); rejected mechanisms are negative evidence the proposer conditions on |
| C | Structured exploration | proposal | stall = progress within noise band δ over window w → reserve m_draft proposal slots for never-exercised components U_t = K∖T_t |
| D | Leakage screening | selection | critic rejects benchmark/task-specific edits (task names, entity names, answers) BEFORE evaluation — a leaking candidate never gets the inflated score that would attract later rounds |
| E | Noise-adjusted floor | selection | Ŝ(H′) ≥ S★ − δ, δ calibrated from repeated evaluations of the unchanged base harness; blocks walking downhill through noise-sized regressions |
| F | Complexity-aware acceptance (L2 analogy) | selection | ΔC ≤ β₀ + β₁ΔS — extra inference cost must be paid for by measured gain (β₁ ≈ 25–45% token allowance per point in their settings) |
| G | Structural pruning (L1 analogy) | selection | components with no strictly positive measured gain in a pruning window become deletion targets REPORTED to the proposer |
| — | Within-band rule (Eq 17) | selection | when ΔS ≤ δ: w_s·ΔS − w_c·ΔC + w_n·ν > 0, where ν counts never-accepted structural component types — within the noise band only cost reduction or structural novelty can admit |

Key results: (i) ablation — removing acceptance regularizers raises evolve score (90.5→91.5) while OOD falls (43.6→41.0) and tokens rise ~50%; removing BOTH gives the highest evolve score (92.8) with OOD ≈ H₀ (40.3) at 3.80M tokens/trial vs 2.42M — the canonical evolve-set-overfit signature; (ii) cross-model transfer — a harness evolved with Gemini 3.5 Flash helps Gemini 3.1 Flash Lite (+30.4% relative) it never saw: mechanisms, not policy artifacts; (iii) all gains hold under deterministic simulators (EngDesign/Frontier-Eng), killing the judge-gaming explanation.

## 2. Pre-flight (what ran)

1. Product-set scope: AGENTS.md context loaded for all seven product-set roots + the healer (riir-clippy). Not a training paper — frozen backbone throughout; the "regularization" is search-process constraints (the paper itself stresses L0/L1/L2 are role analogies, not penalized objectives) → no adversarial panel trigger, no riir-train route.
2. `.research/` set derived on this box (11 dirs incl. two box-local); `.plans/` set derived (16).
3. Closest cousins READ before verdict: **167** (RSI survey → issue 102), **178** (Schmidhuber keeper-half → issue 122), **182** (frontier miner), katgpt-rs **012** (TRT — the workspace's own RSI-overfitting warning, 2025-06: "Successes overfit to specific solution steps").
4. Prior-art searches run (§6): RSI-harness landscape + GP parsimony lineage.
5. Three-track grep: self-adaptive ships (`self_evolve`, `frontier_miner`, `traj_store`, `elo`, `tier_guard`, `collapse_monitor`); model-based lives elsewhere (TernaryDraftModel, quest_grammar); no RRSI-shaped acceptance gate found anywhere.

## 3. The mapping table (paper mechanism → shipped cousin, signal-diffed)

| # | RRSI mechanism | Shipped cousin (read-verified) | Signal-diff → disposition |
|---|---|---|---|
| A | Annealed edit budget | Batch packs constant at 2 rules/batch (already ≈ b_min by the paper's own scale: their b_max is 3–4); frontier miner per-family K-minimality | **Discard (coverage at our scale).** Annealing a budget already at the floor is a no-op; vocabulary recorded. |
| B | Evidence-aware credit assignment | `traj_store` + `elo` + **`tier_guard`** (Lemma-4.2: `((goal/outcome)−1).abs() > ε`, `TIER_GUARD_EPSILON = 0.5`, γ=0.9 priced slack `2γε/(1−γ)`) + Bench 090 lex-decrease keep rule | **Discard (coverage, stronger cousin — on the credit-attribution axis).** Ours is a two-sided statistical ratio test between the goal model and outcome history — i.e. it falsifies the CREDIT ATTRIBUTION itself in both directions (free-rider credit and over-certification); RRSI's negative evidence falsifies rejected EDITS, a different object. Same function class (evidence-gated keep over a journal, both retaining negative evidence — `traj_store`'s `fail_count`/`consecutive_fails`/withdrawal); the delta is which object gets falsified. "Stronger" is architectural reasoning, not measured. |
| C | Stall-triggered structured exploration | coverage-report **re-arm** (the standing pipeline since Batch 155: spare-queue-empty → fresh uncovered-lint derivation, Batches 155/158/164/167/171) + frontier HeldOut/Frontier modes | **Discard (coverage; manual↔automatic delta).** Ours is operator-triggered, theirs auto-detects the stall (σ_t = 1[progress ≤ δ over w]). The automatic predicate is ~10 lines over `frontier_report`'s own series — recorded in Issue 130 as an optional rider, not standalone. |
| D | Leakage screening (pre-evaluation) | probe discipline (upstream-silence mirroring; the Batch 167 u32/u64 fixture-spelling lesson) + **provenance strata** (measures authored 98.0%/51 findings vs repo 50.0%/2) + boundary-guard (the workspace harness's own leakage screen) | **Partial — the one real gap.** We MEASURE the evolve-set overfit (strata) and PROBE upstream fidelity, but nothing at admission time requires generalization evidence beyond authored fixtures. → Issue 130. |
| E | Noise-adjusted floor Ŝ ≥ S★−δ | test_gate floors + drift ratchets (downward-only) + **`best_of_us`/`ab_median_ratio`** (Issues 723/831/833 — baseline-noise calibration for perf bars; ±21.7% sequential drift measured) | **Gap at the conjunction.** The noise-calibration discipline ships as BENCH discipline, never as an ADMISSION rule on corpus content. Honest scoping: our compile/clippy oracle is deterministic per pinned toolchain — the stochastic surface where δ bites is the retrieval/heal-rate benches and OOD rigs, not the oracle. → Issue 130. |
| F | Gain-proportional cost acceptance ΔC ≤ β₀+β₁ΔS | GOAT gates (gain proof) + `binary_size_gate` (6.5 MiB ceiling) — gain and cost checked SEPARATELY, never coupled parametrically | **Small gap, folded into Issue 130 as a design option.** ~70% of .text is corpus tables (Issue 102's own cargo-bloat measurement); a β-line would couple per-rule table cost to per-rule measured gain at admission. |
| G | Structural pruning (targets REPORTED to proposer) | **`frontier_report` contribution-fade** (FADED ≠ NEVER-CONTRIBUTED; report-only; T2–T4 predictive clock started 2026-09-21, commits `56f0b0fd`/`d0457204`) | **Discard (coverage — exact shape).** Same design decision (report-only, gate later on predictive evidence), landed the SAME DAY the paper appeared (2026-09-21 — zero-day delta; independent by read-date, not by distance). |
| — | Within-band novelty bonus (w_n·ν) | UCB bonuses in g_zero template proposer (proposal-side); coverage-report uncovered-lint ranking (proposal-side novelty) | **Small gap.** No acceptance-side novelty credit anywhere. Folded into Issue 130. |

## 4. Game-context reframe (priority #1 check)

NPC runtime habit adoption is a self-improvement loop over finite experience; the overfit analog is maladaptive habits from small samples. Shipped cousin: **`DeltaGatedAbsorbCompress`** (katgpt-pruners g_zero — absorb gated on accumulated per-arm δ evidence, "no δ evidence = don't promote", cached threshold flag for the hot path) + cgsp curiosity. Signal-diff: the cousin consumes prediction-error δ against FIXED thresholds; RRSI calibrates δ from repeated baseline variance and adds novelty/cost credit inside the band. The delta is small and 178 already placed CGSP as covering the curiosity surface — **no new game filing warranted**.

## 5. The meta observation (record; explicitly NOT actionable)

The workspace's own dev harness (AGENTS.md / skills / gates) is an RRSI instance run by agents: `skill_size_gate` = the L2 ceiling; numbering floors/ratchets = noise floors; boundary-guard = leakage screen; doc-sync prune-to-15 + contribution-fade = L1 pruning; freeze tables = evidence-aware state; test_gate floors = downward-only acceptance bars. RRSI's both-regularizers-removed ablation (highest evolve score, OOD ≈ H₀, +57% tokens) is published support for this discipline's shape — and the paper's δ-from-repeated-baseline recipe is the same recipe the x86_64 matrix / sequential-AB lineage already enforces for perf numbers. Per skill §1.55, "validates our design" is not actionable.

## 6. Prior art (§4 searches, run BEFORE the verdict)

Pinned fusion claim (written before searching): *"Acceptance-side regularizers — a noise-calibrated admission floor on the real-repo heal stratum (δ from repeated baseline benches), within-band structural-novelty/cost credit, and a gain-proportional cost line — applied to rule admission in a modelless code-healing corpus whose evolve set is authored fixtures and whose OOD is real-repo spans."*

- **Class level: KILLED twice.** (a) The paper itself is the class prior art for regularized RSI. (b) GP bloat control / parsimony pressure — Soule 1998; Luke & Panait 2006 "Parsimony pressure made easy"; Kronberger 2011 (adaptive covariant parsimony); Naik 2013 (GP generalization taxonomy); Bomarito 2022 (Bayesian model selection vs bloat) — complexity-regularized evolution of rules/programs since the 1990s.
- **Concurrent (named in the paper's own related work):** HarnessCompass (Zhang et al., generalization as explicit search objective), gated semantic quality-diversity archives (Luo et al.), EvoBench (Huang et al.), delta-attribution (Ding et al.).
- **Landscape:** RSI survey arXiv:2607.13104 (= Research 167's source); Schmidhuber essay (= 178's source); AIDE² (Weco, 2026-07) names the same failure: "optimizing the score the agent can see may lead to overfitting on the task set".
- **Conjunction:** no published hit applies these acceptance rules to a non-LLM, oracle-verified rule corpus — the same standing as 178's PowerPlay→code-corpus conjunction (fusion-grade, not class-novel).

## 7. Verdict

| Axis | Call |
|---|---|
| Tier | **Gain** (modelless track — the loop and every cousin runs in-process, zero GPU; serving-envelope native) |
| MOAT gate (riir-clippy, consumer-first) | PASS on fit — targets the in-scope OOD-generalization axis. No katgpt-core residue worth filing: the δ-calibration helper only earns a crate if Issue 130 is taken. |
| Ladder check | Priority #2 (healer) primary; priority #1 (game) coverage found (DeltaGatedAbsorbCompress + cgsp), no filing. |
| Files | This note · `Issue 130` · redirect line on 178 |

### Fusion (skill §1 step 6)

RRSI's acceptance rules × our provenance strata × score-bench `rule_outcomes`: the strata already splits authored vs repo heal per run; `rule_outcomes` already carries per-rule resolved/persist per sweep. Issue 130's gate composes them. The honest cost framing: admit-on-delta requires per-candidate evidence the current batch discipline does not pay for (probe + e2e + freeze-regen only) — that cost/benefit call is the owner decision the issue frames, and Issue 102's report-first precedent (gate only on proven-predictive evidence) is the conservative default if the owner declines. Second-order fusion recorded, not filed: RRSI's stall predicate (mechanism C) over `frontier_report`'s own series would automate the coverage-report re-arm trigger.

> **PASS-Redirects (synthesis):** Xia, Han, Wang et al. [arXiv:2609.24972 "RRSI: Regularized Recursive Self-Improvement of Agent Harnesses"] — the 2026 empirical keeper-discipline paper for the RSI lineage this repo already distills (167/178); four of seven regularizers ship as exact-shape cousins, one conjunction gap filed as Issue 130.
>
> **PASS-Redirects (synthesis):** Fu, Kulanthaivelu, Yamada [arXiv:2609.19526 "Self Improvement via Fast Tree-search"] — the cost-shape companion to Issue 130's admission-gate axis (the OOD rigs the 09-23 narrowing leaves live): cheap pairwise BT-aggregated judge ordering expensive per-candidate evaluations (Polyglot 31–35% at ~1/10 DGM CPU-h); our oracle economics invert the direct port, the orchestration ordering only transfers — distilled in [208](208_sift_judge_ordered_eval.md).
