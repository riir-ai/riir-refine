# Proposal 008 — Adaptive tokenomics: an autonomous, bounded governance agent for the KAT economy (new repo `riir-dao`)

Status: **draft** (filed 2026-08-27). **EXECUTED 2026-08-27 (subagent campaign): Phase 0 COMPLETE (T0.1 dao `1de05ba` + registered in the workspace boundary contract; T0.2/T0.3 dapps `80a9aef`) + Phase 1 COMPLETE with the honest **G5 VERDICT: FAIL (wash)** ([Bench 001](../../riir-dao/.benchmarks/001_g5_simulator.md)) — the agent is ADVISORY-ONLY, the defend-wrong recorded outcome (adversarial half PASSED; the v1 candidates are inert against the T2.3 decay schedule — re-open needs a NEW plan) + Phase 2 T2.1/T2.2 COMPLETE (dao `1ae80d8`: GLM advisory transport + decision artifacts + audit CLI + the dapps contract pins, zero drift); **T2.4 voting COMPLETE 2026-08-28 (dapps `4cae5f2` + the clippy CLI half — [Bench 005](../../riir-dapps/.benchmarks/005_008_t24_shallow_dao_voting_g9_gov.md), G9-GOV PASS)**; T2.3 soak async; Phases 3/4 BLOCKED by the G5 gate exactly as designed.** **Cross-ref 2026-08-28 — 005 T2.5 landed** (riir-dapps [Plan 011](../../riir-dapps/.plans/011_t25_g7_econ_sim_and_007_t23_staking_chaos.md) / [Bench 004](../../riir-dapps/.benchmarks/004_g7_econ_sim_and_staking_chaos.md), commit `20bebb6`): the G7 sim is a MIXED PASS whose axis-E FAIL lands squarely on this proposal's decay lever — no value in the declared 0.90–0.99 range keeps mining alive at a 1000-epoch horizon, and the one healthy set measured (`decay 9 980` + `cap 5e11`) is OUTSIDE that bound. Two consequences recorded rather than smoothed over: (1) the §levers table's decay row and its "mint ≤ 90% of burn" phrasing are both corrected below; (2) with G5 already FAIL (advisory-only) **and** the shipped lever ranges now measured unable to reach the only healthy operating point, the owner's "auto-live tokenomics" ask is **not deliverable by this proposal as specified** — the gap is a cap/initial supply decision, which is a NEW plan and an owner act, never an agent move. The parameter freeze itself stays unratified by owner decision (one instrument is not enough validation) — riir-dapps [Issue 006](../../riir-dapps/.issues/006_freeze_ratification_gate_and_decay_bound_conflict.md). **RESOLVED 2026-08-28 in this proposal's favour, and now APPLIED** ([riir-dao Bench 002](../../riir-dao/.benchmarks/002_longevity_cross_bench.md), `67c18d7`; ratified in riir-dapps `3577986`, mirrored in riir-dao `b5d5052`, Issue 006 CLOSED): the cross-bench instrument confirmed Bench 004, and the set taken is the **in-bounds minimum-change set** (`decay 9_500 → 9_900`, cap unchanged) — so **`LeverBound::DECAY_FACTOR` is NOT widened**, this proposal's never-lever contract holds unamended, and the safety inversion stays intact. **The applied result strengthens the inversion rather than merely preserving it, and this is the sharpest statement of it available:** the ratified genesis now sits EXACTLY at `DECAY_FACTOR.max_bp`, and since higher bp = slower decay = MORE cumulative emission, `max_bp` is this lever's **inflationary extreme**. So the shipped deployment **saturates the decay lever at its safe end**: the agent has zero upward headroom, every in-bounds move it can make (Δ ≤ 100 bp toward `min_bp` 9_000) makes the schedule HARSHER, and **the agent cannot inflate long-run mining emission through `decay_factor` at all** — one-directional by construction, with no special case in the guard and no reliance on the agent behaving well. Adopting `9_980` would have inverted exactly this: a never-lever bound widening AND 80 bp of fresh inflationary headroom for the agent. Pinned on both sides of the contract (`riir-dapps tests/kat_governance.rs::genesis_decay_saturates_lever_safe_end`, `riir-dao tests/dapps_contract_pins.rs::genesis_reward_schedule_mirrors_the_dapps_defaults`), and both pins were **verified to fire**, not merely to pass. The healthy-longevity set stays permanently agent-unreachable by design, asserted mechanically by riir-dao's `genesis_out_of_guard_bounds` test. Two measured facts made that the right trade: a cap raise does nothing for 9_900 (identical death @800 across 1e11→1e12 — it is schedule-bound, so no supply decision is needed), and `9_980 + 5e11` would have cost a never-lever bound widening plus a 5× supply-cap raise to buy roughly four more years of schedule. The honest limit stated above still stands and is now sharper: **no decay this agent can reach keeps mining alive to a 1000-epoch horizon at launch scale under any cap** — the levers are bounded, correctly, below what the economy would need, and closing that gap remains an owner act, not an agent move. The owner's ask: "a strategy that adjusts by our shallow AI self-heal to make tokenomics sustain and auto-live (think auto-DAO) — a tokenomics agent that gets the data from ruliology and reasons about it, inflate/deflate/burn/dilute, auto-prevent flash loan, circuit break, ask Cloudflare GLM to reason about the decision — think like game theory to keep miner dopamine and tokenomics stable." This proposal answers that ask with one load-bearing safety inversion: **the agent never holds the money levers — the ledger does. The agent proposes parameter sets inside hard bounds; `KatLedger` enforces the bounds AT APPLY TIME.** Amends [Proposal 005](005_kat_heal_service_network.md) T2.3's "parameters frozen at T2.3" → *initial values* frozen at T2.3, then versioned/bounded/agent-movable after the Phase 4 autonomy ramp. Creates the repo [`riir-dao`](../../riir-dao/) (name matches the sibling pattern; alternatives `riir-tokenomics`/`riir-econ` rejected — long/obscure). **Noted 2026-08-27 (007 amendment):** the staking slice splits `usdc`/`kat-reward` sub-shares — one new bounded sub-lever added to the dilute row. **Amended 2026-08-27 (2nd owner review):** ban/unban **voting** added as the first governance primitive — the shallow-DAO surface (agent proposes from anomaly telemetry, stakers vote with snapshot weights, authority executes; GLM reasoning slots into the existing advisory seam later) — with new gate **G9-GOV**; exit mechanics (007's escrow/venting/caps) added to the never-lever list.
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: Proposal 005 §Economics (the economy governed) × the riir-clippy self-heal selection math (`SelectionMode::BetaPosterior` + Elo — now public substrate in `katgpt_core::rating` + `beta_lcb_order_into`, Bench 675) × riir-dapps epoch-settle/anchor (`KatLedger`, the exact-once ring, Merkle anchors — the apply+audit seam) × riir-dapps `FairRoll` (commit-reveal variance — the dopamine lever) × `src/llm_spend.rs` USD-cap discipline (the cost breaker precedent) × RAI's reflexive PI controller (prior art — §References, our differentiators named in §Honest caveats #8)
Related: [Proposal 005](005_kat_heal_service_network.md), [Proposal 007](007_kat_usdc_staking.md) (the staking actuator), [riir-dapps Proposal 001](../../riir-dapps/.proposals/001_agreement_boundary_and_tiered_durability.md) (which lists tokenomics as out-of-scope for dapps — the boundary argument for this repo)

## TL;DR

The KAT economy's parameters (burn split, emission multiplier, mining decay, staking share, lease price, reward-variance profile) are frozen constants that nobody can tune without a release. Give them to an **autonomous agent in a new repo `riir-dao`**: every epoch it reads committed telemetry (ledger flows + heal-network health aggregates, including the ruliology-derived metrics), **selects a bounded parameter set modellessly** (BetaPosterior/Elo over strategy candidates — the same selection math that already ships in the healer's self-evolve loop, consumed from the public `katgpt-core` primitives), consults **Cloudflare GLM for reasoning at guard boundaries** (advisory-only, never executes — its proposals pass the same deterministic guard check), and commits a **BLAKE3-anchored decision artifact** (telemetry hash + strategy scores + guard trace + advisory transcript + chosen params) that epoch settle applies. **Flash-loan defense and circuit breakers are structural**: parameters move ≤ bounded deltas per epoch, decisions read epoch-aggregate (never spot) telemetry, staking weights snapshot a lockup-age before payout (007), and — decisive in v1 — **KAT is non-transferable: there is no KAT market to flash-loan from**. Miner retention is engineered as **variable-ratio reward variance via `FairRoll` commit-reveal** (EV-capped jackpots + streaks — the strongest known reinforcement schedule, honestly labeled as behavioral design). Everything is proven in a **deterministic simulator (defend-wrong PoC) BEFORE any autonomy**: agent vs frozen-constants baseline vs adversarial best-responder. If the agent can't beat frozen constants on the Pareto, it stays advisory-only — that verdict is a gate, not a hope.

## The problem this solves

Proposal 005's economics are three hardcoded constants (`POOL_NUM/POOL_DEN` 70/20/10, `InflationSchedule` params, `KatPack` prices) plus a T2.3 note to "freeze" tuned values. That has three failure modes:

1. **No response to shocks.** A demand spike starves miners (pool thin relative to activity); a demand dip over-pays them; an adversarial farming wave eats the pool for a quarter before a human notices. Every fix is a code release.
2. **Owner toil forever.** "Tune the tokenomics" becomes a recurring human task — exactly the class of work this stack keeps automating (self-evolve, sweepers, housekeeping).
3. **No telemetry loop exists at all.** The ledger has the numbers (burn, mint, pool, TVL after 007) and the heal network has the health metrics (heal rates, miss backlog, score-bench deltas, rule-Elo drift — the ruliology surface), but nothing computes epoch summaries or closes the loop from them to parameters.

What goes wrong without this proposal: the economy launches with constants that were right for a simulation and wrong for the world, and stays wrong.

## The proposed design

### The control loop (5 layers, one epoch cadence)

```mermaid
graph TD
    T["1 SIGNAL — committed epoch-summary rows:<br/>burn · mint · pool · TVL · miners-active · heal_rate ·<br/>miss_backlog · score-bench Δ · rule-Elo drift · USD costs"]
    S["2 STRATEGY — parameterized policy candidates<br/>(hold · deflate · boost-mining · shift-staking · tighten-variance …)<br/>selected via katgpt_core BetaPosterior/Elo over observed outcomes"]
    G["3 GUARD — per-lever delta bounds · global breaker ·<br/>finite/anomaly screens (llm_spend + finite.rs discipline)"]
    A["4 ADVISORY — CF GLM reasoning at boundaries/novel regimes<br/>(structured numbers in · strict-JSON proposal out · NEVER executes;<br/>same guard check · full transcript committed)"]
    D["5 COMMIT — GovernanceDecision artifact (BLAKE3)<br/>→ anchor → epoch settle applies EpochParams"]
    L["KatLedger — HARD BOUNDS AT APPLY TIME<br/>(refuses any out-of-bounds param set, whoever proposed it)"]
    T --> S --> G --> D
    G <-.->|boundary / novel regime| A
    D --> L
    L -.->|epoch summary rows| T
```

**The safety inversion (load-bearing, restated):** the agent is a *proposer*. The enforcement point is the ledger — `KatLedger` gains a versioned `kat:params:<epoch>` row written only through a signed governance intent, and **apply-time bounds checking**: an out-of-bounds parameter set is refused by the ledger regardless of who signed it. A compromised agent, a jailbroken advisor, or a bug can move the economy at most `Δmax` per lever per epoch. The agent's governance key is ops-held and rotatable (the mint-authority discipline, 005 T4.3).

### The levers (what moves, and what never does)

| Lever (user's word) | Parameter | Hard bounds (per epoch) | Mechanism |
|---|---|---|---|
| **inflate** | `emission_mult` r on `mint = r × Σ credited outcomes` | 0.5–2.0 ×, Δ ≤ 10% | more KAT per credited outcome (still burn-backed). **Corrected 2026-08-28 (riir-dapps Issue 006):** the earlier phrasing "mint ≤ 90% of burn" implied an independent cap; there is none. The split is a **partition** — shares sum to exactly 10 000 bp, enforced at `apply_params` step 1 — so `mint_share_bp ≡ 10 000 − true_burn_share_bp`, i.e. **85–95%** across the true-burn lever's own 500–1 500 bp range, and exactly 90% only at the DEFAULT 1 000 bp. Moving true-burn moves the mint ceiling one-for-one; the two rows are one lever seen from both sides, not two bounds that could bind against each other |
| **deflate** | same r, downward | same | |
| **burn** | `true_burn_share` | 5–15%, Δ ≤ 2 pp | net supply cut vs re-mint |
| **dilute** | re-mint allocation: `mining_share`/`staking_share` of the 70% pool; `treasury_share`; the staking slice's `usdc`/`kat-reward` sub-split (007 amendment) | mining 50–90%, staking 5–30%, treasury 15–25%, sub-split 20–80%, Δ ≤ 5 pp each | redistribution of the burn re-mint — **there is NO unbacked-mint lever**: post-registration supply discipline (005) means dilution only happens through the reward pool, and the agent structurally cannot print |
| (retention) | `reward_variance` table — jackpot (p, multiplier) pairs via `FairRoll` | EV ≤ base + ε (hard), max multiplier ≤ 10× | the dopamine actuator, §Miner dopamine |
| (pricing) | corpus-lease price (KAT) | bounds per 005 §KAT | the one demand-side price inside the loop |
| (decay) | mining `decay_factor` | 0.90–0.99, Δ ≤ 0.01 | early-vs-late miner balance. **CONFLICT RESOLVED — BOUND UNCHANGED, and genesis now SATURATES this lever at its safe end (2026-08-28, riir-dapps [Issue 006](../../riir-dapps/.issues/006_freeze_ratification_gate_and_decay_bound_conflict.md) CLOSED, `3577986`).** Shipped genesis decay is `9_900` = this row's `max_bp` exactly. Higher bp = slower decay = more cumulative emission, so `max_bp` is the inflationary extreme of the range — the agent therefore holds **zero upward headroom** on this lever and every legal move it can make tightens the schedule. It cannot inflate long-run emission through decay at all. The original conflict, for the record:  005 T2.5's G7 sim ([Bench 004](../../riir-dapps/.benchmarks/004_g7_econ_sim_and_staking_chaos.md)) measured that **no decay in this range keeps mining alive** at a 1000-epoch horizon under `initial 1e9`/`cap 1e11` — and non-monotonically, slower decay dies *faster* (emission-cap death). The one healthy set it found, `decay 9 980` + `cap 5e11`, is **outside this bound**, so the agent structurally cannot reach it: per the never-lever list below, widening a bound is an owner-reviewed ledger upgrade, never an agent action. This is the honest limit of the shipped lever set, recorded rather than papered over — the owner's ask for auto-live tokenomics is **not currently deliverable by these levers at these genesis constants**, and closing that gap is a NEW plan (a cap/initial ratio change is a supply decision, not a lever move) |

**Never levers (frozen, by contract):** `max_supply` (chain-registered); the free grant; KAT↔USD pack prices (USD-side, operator-owned; the agent may *annotate* decisions with pricing observations, never set them); any USDC-out path; KAT transferability; the legal posture; the bounds themselves (changing a bound is an owner-reviewed ledger upgrade, not an agent action). Also frozen: **exit mechanics** (007's escrow window, venting thresholds, drain caps — an agent that could lengthen exits could trap funds) and **ban semantics** (membership only — a vote or agent action NEVER moves funds).

### Shallow-DAO voting: ban/unban (amended 2026-08-27, 2nd owner review)

The owner's call: add votable ban/unban so the proposal+vote **mechanism exists to test the SDK** — shallow is fine, LLM reasoning comes later. Design:

- **Proposal kinds v1:** `Ban { account, reason_code, evidence_hash }` / `Unban { account, reason_code }`. Proposal source v1: **the agent only** — its anomaly screens emit candidates with BLAKE3-pinned evidence hashes (005 §Enforcement posture's "anomaly flags" finally get an escalation path with a staker check). Staker-initiated proposals deferred (need deposit + threshold anti-spam — named, not designed).
- **Vote weight:** staked tranches under the SAME lockup-age rule as incentives (a tranche votes only if it would earn — flash-voting is excluded by the same snapshot+age defense as flash-farming; no new mechanism).
- **Lifecycle:** `open` (window W, ~3 days) → close → pass iff **quorum** (≥ Q% of eligible weight, e.g. 20%) AND majority of cast → **execution timelock** (T_exec ~48 h — the same discipline as 007's escrow: a manipulated vote can be halted inside the window, never executed blind) → the **authority executes** the membership flip. `expired`/`rejected` otherwise. One live proposal per account; the evidence hash is pinned at open so the debate cannot be rewritten mid-vote.
- **Ban semantics (constitutional):** ban revokes **network membership only** — corpus lease denied, sync rejected, mining settlement frozen, new stakes refused. The banned account's existing principal unstakes normally (escrow included). **A vote NEVER moves funds** — no confiscation, no balance claw, no exit-mechanics change.
- **Execution honesty (caveat #5):** v1 is agent-proposes + stakers-decide + **authority-executes** (our key flips the bit after a passed vote). Shallow by design — the owner's call — and the mechanism is the point.
- **GLM later, for free:** each proposal carries an optional advisory-transcript slot fed by the existing §Advisory layer (GLM reasoning over the evidence hash); the lifecycle runs without it, the seam ships day one.
- **The SDK test surface (the load-bearing reason):** the vote lifecycle exercises every KAT SDK seam in one e2e cycle — signed requests, snapshot weights, tally, timelock, batched execution, CLI rendering (`cargo heal vote --proposal <id> --yes|--no` shows weight + evidence). It becomes the SDK's standing e2e gate (G9-GOV).
- **Rows** (riir-dapps `kat_governance`): `kat:govprop:<id>` (kind, account, evidence_hash, open/close ticks, state, tally) + `kat:govvote:<prop>:<account>` (snapshot weight, choice) + Meta counters. The chain's `VoteProgram` is the semantic precedent (cite — per-op chain writes stay forbidden; voting is user-cadence, ledger-side, epoch-batched).

### 1 — Signal layer: ruliology telemetry, honestly sourced

New construction (named): the service plane computes **committed epoch-summary rows** (`kat:telemetry:<epoch>`, one durable write per epoch, BLAKE3-chained): ledger flows (burn, mint, pool paid/available, TVL, unstake queue) + heal-network health aggregates computed **server-side from synced data** — records synced, proven/rejected, miss-backlog by class, score-bench delta from accepted rules, rule-Elo movement (the ruliology surface: `TrajStore` Elo + `RuliologySearch` Pareto classes, aggregated). riir-dao consumes these rows as **data** — no cargo dep on riir-clippy (the 005 Issue-041 one-way rule, inherited). Decisions read **epoch aggregates only, never spot values** — TWAP-by-construction, the flash-loan oracle lesson without an oracle.

### 2 — Strategy layer: the shallow-AI self-heal, transplanted

Strategy candidates are small parameter-set generators ("hold", "shift 5pp pool → staking", "raise variance, cut base", …). Each epoch the agent picks via **`beta_lcb_order` / Elo `update_scored` over (strategy, observed-outcome) history** — the exact public primitives (`katgpt_core::rating`, Bench 675) the healer's `SelectionMode::BetaPosterior` uses. This is the owner's "adjust by our shallow AI self-heal", literally: the same selection math, rewarded on economy health instead of fix outcomes. **It is a selection mechanism, not an optimizer** — no convergence claim, and the simulator (G5) is the only evidence surface for whether it works.

### 3 — Guard layer (agent-side half)

- **Per-lever delta bounds** (the EIP-7265 doctrine: bound the change, not just the halt).
- **Global breaker:** any non-finite or implausible telemetry value → **freeze to last-good committed params**, zero movement, loud row (the `finite.rs` screen + `llm_spend` cap discipline: checked before the move, terminal for the epoch).
- **Novelty gate:** telemetry vector far (cosine distance) from the historical epoch cloud → advisory path + reduced delta budget (half bounds) — unknown regime, cautious steps.

### 4 — Advisory layer: GLM on Workers AI, reasoning without authority

- Designated model: **GLM** (owner directive 2026-08-27; the Workers-AI GLM catalog id — e.g. `@cf/zai-org/glm-4.7`-class — is config, **verified by the standing one-call smoke at first use**; the 005 model-pivot discipline applies: a model change without a re-smoke is the 7.8k-neuron incident class).
- Triggered on: guard-boundary proposals (delta at a bound), novel regimes (signal-layer gate), and a configurable every-Nth-epoch "explain yourself" audit mode.
- Contract: **structured numbers in, strict-JSON parameter proposal out** — no free text crosses into the prompt except committed enum labels; the output re-enters the SAME guard check; the modelless selector still owns the decision (GLM ranks/annotates; the deterministic layer decides). **The advisor can veto nothing and approve nothing — it can only propose within bounds, and the transcript is committed for audit.**
- Cost: per-neuron pricing inside a `llm_spend`-shaped USD daily cap, reimplemented repo-locally under the same discipline (reserve-then-reconcile). Advisory unavailability is the **fast path** — decisions proceed modellessly, never block on the LLM.

### 5 — Commit: proof-of-AI-work

Each decision artifact = `{telemetry_root, strategy_scores, guard_trace, advisory_transcript?, chosen_params, prev_params_root}` → BLAKE3 → anchored at epoch settle (the existing anchor machinery). **The trail IS the proof-of-AI-work**: deterministic replay (same telemetry + strategy state → bit-identical artifact — G1) lets any staker/auditor verify the governance work the agent claims its stipend for. The agent's operating budget: a capped, decayed governance stipend from the treasury share — payment for auditable governance work, not profit participation.

### Miner dopamine (the game-theory half)

- **Variable-ratio jackpots at claim time** via `FairRoll` commit-reveal (the shipped dapps service — unmanipulable randomness, neither miner nor operator steers the roll): e.g. `{90% ×1 · 9% ×2 · 1% ×10}` with **EV pinned ≤ base + ε by construction** — the agent may reshape *variance*, never expected emission.
- **Streak bonuses:** consecutive-epoch activity multiplier, decaying (loss-aversion framing, bounded, capped).
- **First-discoverer ×3** (005, unchanged) — the "come early" schedule.
- The behavioral design is **on the label**: retention mechanics are named as such in the docs, variance is capped (no unbounded jackpots), and G7 reports expected AND realized emission so variance never hides supply movement.

### The simulator (defend-wrong PoC — autonomy's precondition)

Deterministic sandbox in riir-dao (`sim/`, seeded — `FairRng::from_combined_seed` as the actor RNG, zero new deps): synthetic economy of healers/miners/stakers/demand-shocks + **adversarial best-responders** (sybil farms, flash-stake churn, grinding loops). Three arms head-to-head, ≥ 10⁴ epochs, seeded grid:

1. **adaptive agent** (full loop),
2. **frozen constants** (005 T2.3's params — the baseline),
3. **adversary vs adaptive** (breaker/damage accounting).

Metrics: burn/mint convergence, pool-overdraw events, miner retention (returning miners/epoch), TVL stability, incentive leakage to adversaries, USD invariant (005 G7, unchanged and still pass/fail). **Pass = Pareto-better than frozen on ≥1 axis while worse on none, AND adversarial damage ≤ bound with breaker trips.** Fail → the agent stays advisory-only (Phase 2 terminal state) — the defend-wrong discipline: the PoC defends OR refutes, and a refutation is a recorded outcome, not a re-roll.

### The autonomy ramp (never skip a step)

1. **Advisory mode** — artifacts + alerts only; operator applies params by hand.
2. **Guard-bounded auto** — in-bounds moves apply at epoch settle automatically; owner gets a review window (revert path) each N epochs.
3. **Steady state** — review windows lengthen with clean history. **There is no step 4**: out-of-bounds autonomy does not exist in this design.

### Repo: `riir-dao` (created with this proposal)

| | |
|---|---|
| **Domain test** | Is this autonomous tokenomics governance (signal → bounded strategy selection → guard → advisory → commit) over the KAT economy? Not settlement composition (dapps), not a client tool (clippy), not agreement (chain). |
| **Allowlist** | `riir-dapps` (kat ledger surface: telemetry rows + governance-intent seam), `katgpt-core` (public `rating`/beta primitives). **NO riir-clippy dep** (telemetry crosses as committed data), no game crates. |
| **Direction** | `riir-dao → riir-dapps → riir-chain` — the one-way graph extended by one node; `direction_gate`-class check ships with the scaffold. |
| **Deployment** | A CF Worker cron per epoch decision (the `riir-kat-housekeeping` precedent — milliseconds, $0); the simulator runs in CI, not prod. |

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **Goodhart is undefeated.** Every metric in the objective is gameable; the guards bound the damage per epoch, they do not eliminate gaming. The simulator's adversary arm exists because adversaries WILL probe the levers; the honest defense is bounded blast radius + committed trails, not a clean metric.
2. **Reflexivity — the controller sits inside the economy it controls.** Second-order effects (stakers front-running parameter moves, miners gaming retention streaks) are real. RAI's surviving lesson is *slow, bounded, committed moves* — hence per-epoch deltas, epoch-aggregate signals, and the weekly-effective decision cadence. RAI's dead predecessors (unbounded feedback AMMs) are the cautionary family; we are not a stablecoin and not pegging anything, but the loop-hazard class is the same.
3. **Prompt-injection surface is real but narrow.** Telemetry derives from user data; an adversary shaping aggregate telemetry to steer the agent is a novel attack class. Mitigations: numbers-only prompts (no free text crosses except committed enum labels), strict-JSON output schema, guard re-check on everything, committed transcripts. Residual: telemetry *shaping* (inflating sync counts to look healthy) — bounded by delta caps and named as a standing measurement item.
4. **The simulator is not the economy.** Parameters that win in sim may fail live (overfit). The ramp exists for exactly this; Phase 2's review windows are not ceremony.
5. **"Auto-DAO" honesty — v1 is an autonomous agent plus ONE shallow vote surface, not decentralized governance.** The agent is our Worker; keys are ours; stakers vote on exactly one thing (ban/unban membership — the 2nd-review amendment) while every economic lever stays bounded-agent, and execution of passed votes is authority-signed. User-facing copy says "autonomous bounded policy engine with staker membership votes", not "decentralized". Deeper governance (parameter votes, stater-initiated proposals, trustless execution) remains its own future proposal — the mechanism built here is deliberately both the upgrade path and the SDK test surface.
6. **The GLM model id is unverified** until the first one-call smoke; a wrong id is a loud failure, a silent schema mismatch is the 7.8k-neuron class. The 005 smoke rule is inherited verbatim.
7. **Agent bugs move money parameters.** The ledger-side apply-time bounds are the last line of defense, and the breaker's safe state is *frozen at last-good* — but a persistent subtle bias (not a crash) is only caught by the audit trail. The every-Nth-epoch advisory "explain yourself" mode doubles as the drift detector.
8. **Prior-art honesty (the invention claim, precisely stated).** Control-theoretic tokenomics is **not ours**: Reflexer's RAI runs a PI controller on its redemption rate (2020–), and arXiv:2210.12881 models infrastructure token economies as control systems. AI-agents-in-DAOs is an active literature. What THIS design claims as its own: (a) the controller operates over a **service-burn utility economy** (demand = real healing work, not a peg), (b) strategy selection is **modelless self-heal machinery** (BetaPosterior/Elo over policies) with the LLM strictly advisory-under-guards — the inverse of the LLM-agent-runs-the-DAO pattern in the literature, and the exact fixer pre-pass→LLM→verify shape applied to economics, (c) **ledger-side apply-time bounds** as the enforcement point (agent-proposes/ledger-enforces), and (d) commit-reveal **EV-capped variance shaping** as a governed retention lever. If any of these exists published, this proposal downgrades on discovery — the §4 search duty of the research skill applies at plan time.

## Fusion lineage

1. **[Proposal 005](005_kat_heal_service_network.md) §Economics** — the governed object; its G7 USD invariant and mint anchor are inherited unchanged (the agent moves parameters, never the invariants).
2. **riir-clippy self-evolve selection math → `katgpt_core::rating`/`beta_lcb_order_into`** — the strategy selector, consumed from the public crate (Bench 675's promotion made this fusion possible without a client-tool dep).
3. **riir-dapps `KatLedger` epoch settle + anchors + `FairRoll`** — apply seam, audit trail, and the variance lever, all shipped.
4. **`src/llm_spend.rs` (riir-clippy Issue 046)** — the reserve-then-reconcile cost breaker the advisory layer copies repo-locally.
5. **[Proposal 007](007_kat_usdc_staking.md)** — the staking actuator + the snapshot/lockup flash-loan defense the agent relies on.

## GOAT gate

- **G1 (determinism):** same telemetry rows + same strategy state → **bit-identical decision artifact** across 3 runs (the G9 meter discipline applied to governance). Replay-from-anchored-history reconstructs every past parameter set exactly.
- **G2 (cost):** modelless core < 1 s per epoch decision, alloc-bounded; advisory within the USD daily cap (reserve-then-reconcile ledger rows); Worker cron shape (no always-on compute).
- **G3 (no-regression):** with `kat_governance` off, `KatLedger` is bit-identical to 005 behavior (defaults = frozen constants); riir-clippy default binary untouched.
- **G4 (alloc/typing):** all money params integer-bp/u64; no float in any parameter path (render-only f32 for display).
- **G5 (simulator — the load-bearing gate):** adaptive vs frozen vs adversary per §The simulator, ≥10⁴ epochs, seeded grid, results committed to `.benchmarks/` with the sensitivity table. **Fail ⇒ no autonomy (advisory-only terminal).**
- **G6 (bounds at apply):** negative tests — every lever at bound+1 (and every bound-exceeding advisory proposal) **rejected by `KatLedger` itself**, whoever signs; a forged/stale governance intent refused.
- **G7 (breaker):** injected non-finite/implausible telemetry → frozen-at-last-good, zero movement, alert row; advisory outage → modelless fast path, no blocked epoch.
- **G8 (adversarial):** simulator adversaries (sybil farm, flash-stake churn, streak-grinding) cause damage ≤ the documented bound; every attempted exploit trips either ledger bounds or the breaker; the 007 G8-S fixtures hold under agent-moved parameters.
- **G9-GOV (governance lifecycle e2e — the SDK test surface):** one scripted cycle — seeded agent ban-proposal (pinned evidence hash) → snapshot weights → simulated staker votes → quorum → execution timelock → authority executes → ban enforced (lease/sync/staking-refused fixtures) → unban proposal → restore — plus adversarial fixtures: fresh-tranche (flash) votes carry zero weight; sub-quorum rejects; double-vote rejects; tampered evidence hash detected; a halted breaker freezes execution during the timelock. Runs in CI against `InMemoryWal` (the ledger-test discipline).

## What ships now vs deferred

### Ships now (per phase below)
- **riir-dao** scaffold (this proposal creates it): AGENTS/BOUNDARY/numbering, `direction_gate`-class check, CI skeleton.
- riir-dapps: `EpochParams` row + apply-time bounds + signed governance-intent path (feature `kat_governance`); epoch-summary telemetry rows (feature `kat_telemetry`); ban/unban voting rows + lifecycle (same `kat_governance` feature).
- riir-dao: signals/strategies/guards/decision core; simulator; advisory transport; audit CLI; ban-proposal composition from anomaly screens.

### Deferred — gated
- Autonomy ramp steps 2–3 (each gated on the prior step's clean history + G5).
- Deeper staker governance (parameter votes, stater-initiated proposals, trustless execution) — ban/unban voting is now v1 scope (§Shallow-DAO voting); the rest stays out.
- Any KAT-market-dependent levers (no market exists v1 — and that is itself a defense).

### Explicitly NOT shipped by this proposal
- LLM execution authority of ANY kind (advisory under guards, permanently).
- Unbacked mint / supply-cap changes / transferability / USDC-out (never-lever list is contract).
- Game-economy governance (this is the KAT service economy only; game economies stay on their own substrates).

## Phased rollout (sketch — a plan would expand this)

> **Numbering note (2026-08-28):** these are **008** phases. Proposal 005 has its own, unrelated Phase 3 (the fixer node, blocked on leak-posture sign-off) and Phase 4 (prod hardening). Always cite the proposal number with the phase — a bare "Phase 3" in a downstream plan is ambiguous.

### Phase 0 — the seam (no agent yet)
- [x] T0.1 riir-dao scaffold (repo, boundary gate, numbering, CI skeleton) — **DONE 2026-08-27, `1de05ba`** ([Plan 001](../../riir-dao/.plans/001_dao_t0_scaffold.md)): zero-dep workspace + the 6-module type skeleton (signal/strategy/guard/advisory/commit/params), `direction_gate.sh` (3 detectors, no-cargo, bare-clone proven), CI (push-to-main, no sibling checkout), numbering live. The proposal-scaffold drift (module renames) fixed; boundary ledger dispositions cleaned (`4f24d8d`) + the repo REGISTERED in riir-ai's canonical matrix (riir-ai `e51040213`)
- [x] T0.2 riir-dapps `EpochParams` versioned row + apply-time bounds + governance-intent signature path (`kat_governance`, default-off; G6 negative tests) — **DONE 2026-08-27, riir-dapps `80a9aef`** ([Plan 009](../../riir-dapps/.plans/009_kat_governance_bounds_and_telemetry.md)): 7 bp-lever bounds + 2 variance caps mirroring dao guard.rs (cross-repo golden pin), check order partition → **hard bounds BEFORE + regardless of signature** (the safety inversion) → Δ caps → Ed25519 intent binding → write-once; `[[u8;32];4]` rotatable governance keys (fail-closed); 21 G6 tests (every lever at bound±1, forged/tampered/stale/replayed intents). Honest scope: emission/sub-split/decay/variance bounds mirrored+pinned but live-enforced at the `:v2` row widening
- [x] T0.3 Epoch-summary telemetry rows (`kat_telemetry`) — ledger flows + heal-health aggregates, BLAKE3-chained — **DONE (same commit `80a9aef`)**: 168B `kat:telemetry:<epoch>` rows = dao's 16 fields in dao order + live_mask + prev_row_hash, chain root exposed (the anchor pattern), ledger flows live (a between-settles payout cursor), staking TVL under kat_staking, heal stats honest sentinel-0-with-clear-bit (fed via `note_heal_stats` when the sync plane lands); golden hex pinned BOTH sides (dao's consumer pins the same bytes — zero drift, proven `1ae80d8`)

### Phase 1 — modelless agent + simulator (defend-wrong BEFORE autonomy)
- [x] T1.1 Signal layer over telemetry rows (finite/anomaly screens) — **DONE 2026-08-27, `165cdc3`** ([Plan 002](../../riir-dao/.plans/002_008_phase1_modelless_agent_g5_simulator.md)): magnitude sanity on every field, mint ≤ 95% burn, cumulative pool overdraw, jump bounds, re-baseline-after-13-freezes, the committed `economy_health_bp` reward fn
- [x] T1.2 Strategy candidates + `katgpt_core` BetaPosterior/Elo selection — **DONE**: six pure partition-preserving candidates + the Beta-LCB/Elo selector on `katgpt_core::rating`/`hint_regret` (feature-minimal), per-candidate de-trended outcomes, weakest-slice exploration; params ALIGNED to the canonical dapps row (golden-pinned; the scaffold had drifted — dapps is canonical)
- [x] T1.3 Guard layer (delta bounds, breaker, novelty gate) — **DONE**: clamps (HOLD = exact identity), partition renorm with trace rows, integer-cosine novelty gate (half budget), breaker with zero clamps on freeze, lease pinned (D2), variance EV/multiplier caps
- [x] T1.4 The simulator + G5 campaign — **DONE, VERDICT: FAIL (wash)** ([Bench 001](../../riir-dao/.benchmarks/001_g5_simulator.md)): 3 arms × 5 seeds × 10⁴ epochs. Adaptive ≈ frozen (retention −48bp within ±100 tol; TVL +1bp; gap 0) — the v1 candidate set is economically inert against the T2.3 schedule (`1e9×0.95^e` strangles mining within ~600 epochs). **The adversarial half PASSED** (leakage 1.10% ≤ 8%, 782 breaker trips, flash-stake earned ZERO under the snapshot rule). **THE AGENT STAYS ADVISORY-ONLY** — the defend-wrong recorded outcome, not a re-roll; regression-pinned both directions. The campaign fixed 5 real defects en route (stale-reference breaker lock, jump deadband, weather-tripping burn bound, the health fn's kill-mining perverse incentive, selector starvation). Re-open levers (v2 candidates incl. decay_factor/Inflate, or a ledger budget-rule change) require a NEW plan

### Phase 2 — advisory + proof-of-AI-work
- [x] T2.1 GLM advisory transport (structured in / strict-JSON out / guard re-check / transcript commit; one-call smoke; USD cap) — **DONE 2026-08-27, `1ae80d8`** ([Plan 003](../../riir-dao/.plans/003_008_phase2_advisory_artifacts_audit.md)): closed committed vocabulary (nothing free-text enters the prompt), strict-JSON 7-lever schema (lease/variance structurally unproposable), integer micro-USD reserve-then-reconcile cap (llm_spend discipline repo-local), both CF response shapes + GLM thinking-off, `Box<dyn AdvisoryHttp>` seam (mock in default build; ureq behind `advisory_transport`), the guard re-check PROVEN by test (advisory params re-enter the SAME guard engine; out-of-bounds clamped identically; freeze ignores the advisor). **The live GLM smoke is an ops step** (creds; the mode + standing rule shipped — a model change without a re-smoke is the 7.8k-neuron incident class)
- [x] T2.2 Decision artifacts + anchoring + determinism gate (G1) + audit CLI — **DONE (same commit)**: `DecisionRecord` envelope + verify() (hash re-derivation + chain link + transcript commitment), `dao-audit render|verify|sample` (the 9-lever delta table with bounds annotations, every clamp, transcript, artifact hash); G1 bit-identity ×3 across separate process invocations. Anchoring at dapps epoch settle consumes the artifact as data (later wiring)
- [ ] T2.3 Advisory-mode soak (operator applies by hand; N epochs clean) — **ASYNC operator work** (the mechanism landed; the soak accumulates when live telemetry flows)
- [x] T2.4 Shallow-DAO voting surface: ban/unban lifecycle rows + tally + execution timelock + `cargo heal vote` CLI + the G9-GOV e2e gate (the SDK test surface) — **DONE 2026-08-28, riir-dapps `4cae5f2`** ([Plan 012](../../riir-dapps/.plans/012_008_t24_shallow_dao_voting.md), [Bench 005](../../riir-dapps/.benchmarks/005_008_t24_shallow_dao_voting_g9_gov.md)): **G9-GOV PASS** — the full cycle (agent-signed open with signature-PINNED evidence → snapshot weights → signed staker votes → quorum+majority → 48h timelock → gov-key halt/resume breaker → authority-signed execution → the membership flip visible at every enforcement seam: stake refused / lease denied / mining settlement frozen / **unstake still works**) + every negative (double-vote, window, flash weight 0, forged/mismatched keys, tampered evidence, quorum/majority rejections, halt freeze, mandate expiry) + the constitutional pin (**a passed ban never moved a balance byte**). Vote weight = USDC tranches under the EXACT `pay_due_staking` age rule (flash-voting excluded by construction); `kat_governance` widens to imply `kat_staking`. The CLI half: `cargo heal vote --proposal <id> --yes|--no` (weight+evidence render, login-key signing over the cross-repo-pinned `vote_message` golden, service-first + the signed-intent local queue). Honest deferrals: staker-initiated proposals, sync-rejection (service-plane), mining-enqueue refusal, the GLM transcript feed (the 32 B row slot ships)

### Phase 3 — dopamine levers (gated on G5 evidence)
- [ ] T3.1 `FairRoll` jackpot table as a governed lever (EV-capped by construction — tested)
- [ ] T3.2 Streak bonuses (bounded, capped; retention metric wired into signals)

### Phase 4 — autonomy ramp
- [ ] T4.1 Guard-bounded auto at epoch settle (owner review windows + revert path)
- [ ] T4.2 Steady-state cadence + drift detector (every-Nth-epoch advisory audit)

## Risks

1. **Goodhart/reflexivity (highest, permanent):** bounded, slow, committed, adversarially tested — never "solved"; the never-lever contract and ledger bounds cap the blast radius.
2. **Regulatory:** the agent moving burn/staking parameters adjacent to Proposal 007's counsel surface — the 007 counsel gate covers parameter *ranges* too; bounds are set inside the counseled envelope.
3. **Agent-key compromise:** bounded by ledger-side apply-time bounds + breaker + rotation; worst case is bounded mispricing for ≤ the review window, not fund loss.
4. **LLM cost drift:** capped daily, reserve-then-reconciled, advisory-optional (fast path is modelless).
5. **Architectural:** a new repo whose failure mode is dep drift toward riir-clippy/game crates — `direction_gate`-class check in CI from day one; telemetry crosses as data (the Issue-041 rule generalized).

## Out of scope

- Stablecoin/peg logic of any kind (RAI is cited prior art, not a template).
- KAT market making, DEX, oracles (no KAT market exists v1; when transferability is ever counseled, that is a new proposal with new defenses).
- Multi-token governance (other `riir-*` services adopt the pattern later, own proposals).
- The game-plane economy (FAME/karma stay on their substrates).

## References

### In-house (substrates this composes — grep-verified 2026-08-27)
1. [`riir-dapps/src/kat/ledger.rs`](../../riir-dapps/src/kat/ledger.rs) + `compose.rs` — epoch settle, the `POOL_NUM`/`TREASURY_NUM` split constants (the first levers), anchors, exact-once queue.
2. `katgpt_core::rating` (Elo `expected`/`update` + `update_scored`, Bench 675) + `beta_lcb_order_into` (Plan 576) — the strategy-selection primitives, public.
3. [riir-clippy `self_evolve` / `SelectionMode::BetaPosterior`](../src/self_evolve.rs) — the selection *pattern* this transplants (code-level dep: none, by design).
4. [riir-dapps `service::fair_roll`](../../riir-dapps/src/service/fair_roll.rs) — commit-reveal variance (the jackpot lever).
5. [riir-clippy `src/llm_spend.rs`](../src/llm_spend.rs) (Issue 046) — reserve-then-reconcile USD cap discipline.
6. `riir-clippy/src/ruliology_search.rs` + `TrajStore` Elo — the ruliology surface whose server-side aggregates feed signals.
7. [Proposal 007](007_kat_usdc_staking.md) — staking actuator, snapshot/lockup defense, G8-S fixtures.

### External (cited-only)
8. **Reflexer Labs / RAI** — the canonical control-theoretic tokenomics: a PI controller adjusts the redemption rate from the market/redemption-price error (Reflexer "Summoning the Money God"; Dankrad Feist's RAI write-up). Prior art for closed-loop token economics; our differentiators in caveat #8.
9. **arXiv:2210.12881** — *A Control Theoretic Approach to Infrastructure-Centric Blockchain Token Economies*: infrastructure token economies as dynamical systems with continual supplier incentives — the closest academic framing of what KAT is.
10. **Beanstalk governance attack post-mortems** (Veridise et al., 2023) — snapshot-before-payout as the flash-loan defense; generalized here by epoch-aggregate signals + 007's lockup-age rule.
11. **EIP-7265 (Circuit Breaker Standard)** — bounded-outflow doctrine; our per-lever delta bounds + per-day caps follow it.
12. **DAO-AI (arXiv:2510.21117)** + the autonomous-treasury-agent literature — agentic AI producing interpretable, auditable collective decisions; our inversion: the agent proposes, deterministic guards dispose.

## TL;DR

**Ship it phased, simulator-first — the agent earns autonomy by beating frozen constants in the sandbox, never by default.** The architecture is one sentence: *modelless strategy selection (the shipped self-heal math) proposes bounded parameter sets from committed telemetry; GLM reasons at the boundaries without authority; the ledger enforces the bounds at apply time; every decision is a BLAKE3-anchored artifact that replays bit-identically — that trail is the proof-of-AI-work.* Next action: scaffold `riir-dao` (done with this proposal) → open the Phase 0 plan (T0.1–T0.3) once 005 T1.x is stable.
