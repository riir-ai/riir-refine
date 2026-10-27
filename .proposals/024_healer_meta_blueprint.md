# Proposal 024 — Healer Meta-Blueprint: campaign-shape lever tuning above the Level-1 loop

Status: **draft** (executes the first step of `issue 178`)
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: [riir-ai Research 401 §7](../../riir-ai/.research/401_Harness_Evolution_Game_Autoplay_Guide.md) × [katgpt-rs Plan 631](../../katgpt-rs/.plans/631_harness_evolution_loop.md) / [Research 637](../../katgpt-rs/.research/637_Harness_Evolution_Loop_Primitive.md) × the healer's shipped Level-1 stack (evolve/elo/traj_store/score_bench/active_set/escalation_dispatch)
Related: [riir-game-sdk Plan 009](../../riir-game-sdk/.plans/009_arena_harness_evolution_generations.md) (consumer #1) · [riir-ai Plan 642 Phase 2](../../riir-ai/.plans/642_harness_evolution_autoplay.md) (the game-side sibling) · [riir-refine Research 178](../.research/178_Schmidhuber_RSI_Essay_ThreeTrack_Distill.md) (meta-evolution lineage)

## TL;DR

**GO, staged, report-only first — with the acceptance oracle pinned arm-invariant.** The healer's Level-1 loop (propose → `fix_verify` compile-gate → `self_evolve` trajectories) is rich, but every *campaign-shape* knob — lint-class fan-out priorities, verify depth, escalation-lane thresholds — is static config set once by hand, and the loop between the measurement reports (`--necessity-bench`, `--gate-calibration`, `--misses`) and the one manual actuator (`--active-set-demote`) is closed by humans. This proposal stages a **meta level**: a bounded blueprint Λ over campaign-shape levers, scored per campaign with the arena's two-tier comparator (verbatim, from katgpt-core) under a JUDGING POSTURE THAT NEVER VARIES ACROSS ARMS, selected by `hint_regret` LCBs, intensity-gated by the `escalation_guard` latch — Phase A report-only instrumentation (default-on row append, demote-only env), Phase B shadow arms, Phase C actuated A/B behind an opt-in feature with held-out judging. Phase A is the deliverable that survives every verdict; Phases B/C carry the honestly-slow selection machinery behind a pre-registered, computable volume close. The staged design may also make the healer the **earliest second committed consumer** for Plan 631's conditional primitives (`BestKeeper`/`MutationArms`) — only if Plan 642 stays unstarted (see the corrected consumer arithmetic below); the re-open is not this lane's deliverable. The gain claim is honestly priced as SLOW: dev-time campaign volume is sparse, so the design carries a pre-registered insufficient-volume close, and the PoC graveyard in this repo (eight tuning-lane refutations) is weighted as prior evidence against any eager promotion.

## The problem this solves

The healer measures everything and tunes nothing above the per-fix level. What exists today, all Level-1 (per-candidate / per-lint):

- `src/evolve.rs` — `LatentSkillEvolution`, per-lint EMA direction overlays (`EvolveConfig { alpha: 0.1, beta: 0.05, temperature: 1.0 }`) — static hyperparameters, never re-tuned.
- `src/elo.rs` — rule-vs-shape Elo over the trajectory store (Issue 039, delegating to `katgpt_core::rating`).
- `src/adaptive_drafter.rs`, `src/selection_entropic.rs`, `src/selection_tether.rs` — candidate-selection machinery, each with its own fixed mixing weights.
- `src/tier_guard.rs` — Lemma-4.2 certification over the durable tier; `src/traj_store.rs` — durable learn rows; `.benchmarks/score_history.jsonl` — corpus-generation history.

And what stays static at the campaign level, none of it consumed by anything adaptive:

- **Lint-class priorities** — the fan-out/ranking posture across provenance strata and domains is fixed per build; the only lever is the *manual* `--active-set-demote/readmit/cap` (`src/active_set.rs`).
- **Verify budget** — `fix_verify`'s round/scope posture (baseline → apply → re-check, `scoped_recheck`) is config-once.
- **Escalation thresholds** — `escalation_dispatch`'s `modelless_cap` worthiness and `max_rate` sticky latch are manifest/config rows set by verdict, never revisited by evidence.

The measurement reports exist (`--necessity-bench` prices a rule retirement; `--gate-calibration` reads admission-gate strata; `--misses` ranks authoring queues) but the path from report to actuator is a human reading one and typing the other. Without a meta level, every campaign re-runs whatever posture the last human verdict left behind, and cross-campaign evidence (which verify depth actually pays off on which repo family? do escalation thresholds latch too early?) accumulates in `memory.json`/`traj_store` and is never consumed by a decision.

## The proposed design

**Campaign** = one `cargo refine --fix` run over a target. **Blueprint Λ** = the campaign-shape config the run executes under. **Outcome** = the two-tier comparator verbatim: verdict class first (a campaign REGRESSES if any kept edit broke compile under verify; otherwise IMPROVED/NEUTRAL by improved-count vs regression), then fired fraction (applied fixes / findings) — `katgpt_core::harness_evolution::two_tier_score` (landed at `dae2da52`, Bench 957), consumed as-is, no healer-local re-derivation.

**The judging posture is arm-invariant (non-negotiable — this is what makes the selection measure anything).** Λ's `verify_depth` tunes `fix_verify`'s operational depth, and the campaign verdict class is *read from that same verification* — so if arms were graded under their own verify posture, the arm that verifies least would look best precisely when verification is weakest (a scoped-closure miss or a posture gap would blind the oracle and read as fewer regressions; Issue-171's scoped-closure parity is an empirical GOAT result, not a law of nature). The rule: **verdict class and fired fraction are always judged under the incumbent/default verify posture** (full-depth detection, run after the arm's operational pass); an arm's own depth affects its cost and retention only — recorded per row, never fed to the scorer. The Phase A history row therefore carries the verify posture beside the verdict, and `--blueprint-report` prints the posture↔verdict split so any oracle asymmetry is auditable.

Λ v1 — three lever classes, bounded:

```text
Λ = (
  lint_class_priority: { stratum → fan-out weight },   // bounded ladder per class, ~6 classes
  verify_depth:        { rounds: 1..=3, scoped_recheck: bool },  // floor = 1 compile-gated round, NEVER 0
  escalation_thresholds: { modelless_cap, max_rate },  // bounded around the manifest's current rows
)
```

**Deliberately excluded from Λ:** corpus lease cadence and anything KAT-spend-adjacent (issue 178 sketched it; the guard law wins — spend is a never-lever), consent states, the bench-guard posture, the compile gate itself, the never-lever list and the lever bounds (the dao pattern — the frame is never inside the picture).

**Mechanism, per phase:**

- **Phase A — instrument + report (no behavior change to fix outcomes).** Every fix run appends one row to `.refine/blueprint_history.jsonl` (local, append-only, the `score_history.jsonl` shape): blueprint digest, verify posture beside the verdict, per-domain findings, improved/regressed counts, fired fraction, regression flag. Posture decision (fixed here, not deferred to plan time): the append is **always-on with a demote-only env** (`RIIR_REFINE_META_BLUEPRINT=0`, the escalation_guard env grammar) — the `self_evolve` DEFAULT-ON/opt-out pattern, chosen because campaign volume is the lane's scarcest resource and opt-in collection would starve it. New `--blueprint-report` command: per-stratum outcome rates, the posture↔verdict audit split, the necessity-bench → demotion-suggestion join (rules with priced retirements + chronic failure — a *suggestion list*, never an auto-demote), the per-lever outcome splits, and the **volume-close line** (below). This closes the reports→actuator gap at the suggestion level with zero risk.
- **Pre-registered volume close (computable, fires itself).** Arm population pinned NOW: the Phase B grammar is bounded at **≤ 9 arms** (3 lever classes × 3 ladder rungs). The close metric is computed by `--blueprint-report` on every run: 8 weeks after Phase A's first history row, if **total campaigns < 16** (fewer than 2/month) **OR no lever class holds ≥ 6 campaigns in each of ≥ 2 arms**, the report prints a loud `META-VOLUME-CLOSE: insufficient volume (<detail>)` line — the assertable trigger. On the line tripping, issue 178's Phases B/C close with the recorded reason and Phase A's report + demotion suggestions survive as the landing. The line is computed arithmetic, not prose; it can fire and be observed, and it cannot silently not-fire (a report that stops printing it past week 8 is itself wrong).
- **Phase B — shadow arms.** An arm = a candidate blueprint from the bounded ladder+clamp mutation grammar per lever class (the `MutationArms` pattern, seeded, deterministic lowest-index tie-break; ≤ 9 arms total). A shadow selector accumulates per-arm `(successes, failures)` across real campaigns and orders arms with `hint_regret::beta_lcb_order_into` — recorded, never applied. An `escalation_guard::RollingRateLatch` (cost-ceiling shape) observes per-campaign regressions and latches the shadow lane off on a streak (demotion = back to reporting only). Zero behavior change; the output is the arm table + its LCBs in the report. **Phase B also pre-registers the held-out family partition**: the repo-family set (grouped by the healer's swept-repo families) with the minimum per-family campaign count any Phase C acceptance requires — named in the plan, before any arm data exists, so the later bar cannot be gamed.
- **Phase C — actuated A/B (opt-in `meta_blueprint`, DEFAULT-OFF).** Blueprint candidates apply to real runs under seeded deterministic assignment with the **randomization unit = the campaign (per run invocation)**, stratified within repo-family where the family is known — never per-repo-path (a per-repo seed would draw the same arm for the same repo every time, aliasing family effects into arm effects and invalidating held-out judging by construction). The incumbent blueprint is replaced only on a Wilson-LB strict-superiority margin on **held-out repo families** (never the families that tuned it) **plus a per-stratum non-regression floor**: hard-class fix rate on held-out families must not regress — two-tier score alone is maximizable by shopping easy lint classes while hard classes rot. Regression-streak latching demotes to the static default; the champion chain persists as a digest-chained ring (the `BestKeeper` shape) for generation chaining. Every Phase C arm row discloses **realized spend** (escalation arms bounded within the manifest envelope still move real money at the top of `max_rate`). GOAT gate below; promotion to default is a separate owner call.

**The conditional-primitive connection (corrected to Plan 631's own arithmetic; not this lane's deliverable).** Plan 631 T1.2/T1.3 name their ≥2-consumer condition as "(arena generations via sdk Plan 009 + riir-ai Plan 642; *optionally* riir-refine `EvolveRecorder`)" — Plan 642 is the source's named second consumer, and the healer is its optional third. This proposal does not reorder that: the healer's Phase B commitment **may become the earliest second committed consumer only if Plan 642 remains unstarted** (its Phase 0 prerequisite has not begun), and the T1.2/T1.3 re-open lands in **katgpt-rs** by that repo's lane — never here. If the healer's G2 fails or Plan 642 lands first, the primitives stay consumer-local and nothing here is lost.

## Honest caveats — READ BEFORE IMPLEMENTING

1. **Volume is the whole ballgame, and it is sparse.** Campaigns are dev-time events; at plausible idle-lane + dev-use rates, LCB separation between arms takes months, possibly quarters. Pre-registered close condition (the design section's computable form): 8 weeks after Phase A's first history row, **total campaigns < 16 OR no lever class with ≥ 6 campaigns in each of ≥ 2 arms** → the `META-VOLUME-CLOSE` line fires and the lane closes with `insufficient volume`; Phase A's report + demotion suggestions survive as the landing. No arm-selection machinery ships into that world.
2. **The PoC graveyard prior is negative.** This repo's tuning lanes have measured out REFUTED/NEGATIVE repeatedly — eight of them (`set_rerank`, `choice_scorer_poc`, `mattr_k_poc`, `differential_anchor`, `noise_weighted_evidence`, `llm_gist_author`, `gw_corpus`, `nonergodic_poc`). The honest prior that a blueprint arm beats the incumbent is LOW. That is exactly why Phase A is the deliverable and Phase C is default-OFF: the meta level must earn every posture it takes, and the most likely true outcome is a well-instrumented null.
3. **Synthetic campaigns are not stand-ins.** The score-bench's synthetic fixtures cannot substitute for real campaigns (corpus-generation drift; the Issue-030 relevance lesson — a syntactic pass is not a relevance signal). Only real `--fix` campaigns count toward arm statistics, which caps volume permanently (see caveat 1).
4. **Correlation is not causation in shadow mode.** Phase B arms differ in more than Λ (repo mix drifts week to week), so shadow LCBs are screening evidence only; Phase C's seeded randomized assignment is what licenses a causal read. Shipping promotion off shadow data alone is forbidden.
5. **Goodhart surface — and the grading-oracle trap (the sharpest one).** The compile oracle is the ground truth and a never-lever — a blueprint may tune *how much* verification runs (depth ≥ 1, scoped-recheck posture) but never *whether* it runs, and NEVER what the verdict is judged under: **scoring is always arm-invariant** (the incumbent/default verify posture detects regressions for every arm; see the design section). Without that pin, the shallow-verify arm wins by blinding the judge — the failure mode this design exists to prevent. Held-out family judging prevents overfitting to recently-touched repos; the Phase B pre-registered family partition keeps the bar ungameable. The demotion-suggestion join must never auto-execute: auto-demoting corpus rules touches mining economics and stays a report until the owner gates it.
6. **Inventor's-regret risk on Λ's axes.** The three v1 lever classes were chosen because actuators already exist (fan-out, fix_verify, escalation_dispatch). The most plausible miss: the highest-value lever is actually the corpus-lease rule-mix (which overlays to lease), which is spend-adjacent and therefore fenced off here. If Phase A's report shows lease-mix dominating outcome variance, that is a finding to hand the owner, not a lever to seize.
7. **Honest reachability admission.** The volume close AND the Phase C acceptance bar may both turn out unreachable at real campaign rates — that is an acceptable outcome, not a lane failure: **Phase A is the deliverable** (the instrument, the report, the demotion suggestions, the volume evidence). The selection machinery is the part that must earn its existence.

## Fusion lineage

1. **[riir-ai Research 401](../../riir-ai/.research/401_Harness_Evolution_Game_Autoplay_Guide.md) §7 + [Plan 642 Phase 2](../../riir-ai/.plans/642_harness_evolution_autoplay.md)** — the meta level's shape (blueprint Λ, ESC-gated intensity, never-lever guards, held-out judging) is transplanted from the game-runtime fusion at the same level; the healer is the priority-#2 consumer reframe recorded there. What the combination produces that neither has alone: the first consumer whose campaign outcomes are compiler-checked verdicts (stronger ground truth than scenario gates).
2. **[katgpt-rs Plan 631](../../katgpt-rs/.plans/631_harness_evolution_loop.md) / [Research 637](../../katgpt-rs/.research/637_Harness_Evolution_Loop_Primitive.md)** — the extracted comparator (`two_tier_score`, landed) + the conditional primitives this proposal un-blocks; `hint_regret::beta_lcb_order_into` + `wilson_score_ci` consumed as-is (no new LCB, the round-2 law).
3. **The healer's own Level-1 stack** (`evolve.rs` + `elo.rs` + `traj_store.rs` + `score_bench` + `active_set.rs` + `escalation_dispatch.rs`) — the meta level sits one level up and consumes their outputs; it replaces none of them.
4. **[riir-refine Research 178](../.research/178_Schmidhuber_RSI_Essay_ThreeTrack_Distill.md)** — the meta-evolution lineage (Schmidhuber 1987: the meta level inherits the object level's selection; the keeper half is closed-form arithmetic) — the reason the selector here is `beta_lcb` arithmetic and not a learned model.

## GOAT gate

- **G1 (default-posture contract, posture decided in the design):** Phase A's append is always-on with demote-only `RIIR_REFINE_META_BLUEPRINT=0`. G1 therefore asserts: **stdout + fix outcomes are byte-identical** with the lane active; **the history row is the sole declared side-effect**; with the env demoted, zero writes occur (file absent or untouched). `--blueprint-report` is Phase A surface — it exists regardless of the feature and only reads the history file. Without the `meta_blueprint` feature (Phases B/C), no arm machinery, no selection behavior, and nothing beyond the Phase A row + the report command exists.
- **G2 (gain at equal budget, Phase C):** held-out repo families under the Phase B pre-registered partition with the pre-registered per-family minimums; two-tier pooled campaign score with a Wilson-LB strict-superiority margin over the incumbent blueprint; **per-stratum non-regression floor** (hard-class fix rate on held-out families must not regress — easy-class shopping cannot promote); regression rate non-inferior; aggregate volume never decides (the serving law); **scoring posture arm-invariant** (the incumbent verify posture judges every arm). Box-state provenance line recorded beside the numbers (the latency-claim law).
- **G3 (no-regression + guard integrity):** default posture byte-identical (G1's contract); ESC latch demotes the lane on a regression streak; the verify floor (≥ 1 compile-gated operational round) is compile-asserted; **the judging posture (incumbent/default) is a never-lever** — compile-asserted, negative-tested; the compile oracle, consent, spend bounds, and bench-guard are compile-asserted never-levers (negative tests per lever); **every Phase C arm row discloses realized spend** (escalation arms at the top of the `max_rate` envelope spend real money during A/B).
- **G4 (cost):** the meta level runs at run start/end, never in the heal hot path — zero-alloc is NOT required and none is claimed (honest deviation from the usual G4; the hot path is untouched). The history row append is one bounded write per run.
- **Floor rule:** no UQ is published. If a future convergence-speed readout is ever published, it benchmarks against the conformal-naive floor (Report-the-Floor) or stays an internal selection rule.

## What ships now (riir-refine) vs deferred (katgpt-rs / owner)

### Ships now — Phase A scope (a plan would own the tasks)
- `.refine/blueprint_history.jsonl` append (bounded row, digest-keyed; always-on with demote-only `RIIR_REFINE_META_BLUEPRINT=0` — the sole declared side-effect) + `--blueprint-report` (strata outcomes, posture↔verdict audit, demotion suggestions, lever splits, the volume-close line). No actuator change.

### Deferred — validation-gated
- Phase B shadow arms + Phase C actuated A/B behind `meta_blueprint` (opt-in, DEFAULT-OFF), each gated on the previous phase's volume/verdict.
- katgpt-core `BestKeeper`/`MutationArms` re-open — lands in **katgpt-rs** by that repo's lane once this repo's Phase B commitment + the arena consumer are on record.

### Explicitly NOT shipped by this proposal
- Any auto-execution of demotion suggestions (owner-gated surface).
- Any tuning of: corpus lease cadence, KAT pricing, spend ceilings' bounds, consent states, the compile gate, the bench-guard, the never-lever list itself.
- Any LLM-in-the-loop selection (the modelless law — the selector is closed-form arithmetic), any trained component, any new dep.

## Phased rollout (sketch — a plan would expand this)

### Phase A — instrument + report (LANDED 2026-10-27 — plan 299, issue 178)
- [x] A1 blueprint-history row schema + append at run end (always-on, demote-only env; G1's sole-declared-side-effect contract)
- [x] A2 `--blueprint-report`: strata outcomes + posture↔verdict audit split + lever splits
- [x] A3 necessity-bench → demotion-suggestion join (report-only) + the volume-close line (the ≤9-arm grammar and the 16-campaign / ≥6-per-arm thresholds)
- [x] A4 G1 tests: stdout+outcomes byte-identity with the lane active; env demoted → zero writes; negative tests
- [x] A5 volume-close liveness tests: the line fires on a synthetic history below both thresholds at week 8; it does NOT fire above them; and past week 8 the line (or its explicit not-tripped form) prints on EVERY report run — the cannot-silently-not-fire assertion

### Phase B — shadow arms (gated on A's volume read)
- [ ] B1 arm grammar over the three lever classes (ladder+clamp, seeded; ≤ 9 arms)
- [ ] B2 shadow selector (`beta_lcb_order_into`) + arm table in the report
- [ ] B3 escalation_guard latch on regression streaks (shadow lane only)
- [ ] B4 pre-register the held-out family partition + per-family minimums (before any arm data)
- [ ] B5 the katgpt-rs consumer note (their lane; conditional on Plan 642's continued absence)

### Phase C — actuated A/B (gated on B's LCB screening + owner go)
- [ ] C1 `meta_blueprint` feature (DEFAULT-OFF); seeded assignment, randomization unit = the campaign, stratified within repo-family
- [ ] C2 held-out family judging (pre-registered partition) + Wilson-LB acceptance + per-stratum non-regression floor + champion ring
- [ ] C3 realized-spend disclosure per arm row; GOAT G2 bench + verdict in `.benchmarks/`; demote-or-promote by verdict

## Risks

1. **Volume risk (correctness of the lane itself):** campaigns too sparse for any verdict — mitigated by the pre-registered close (caveat 1); the lane's cost is bounded by Phase A's one-row-per-run footprint.
2. **Governance risk:** auto-demote/auto-apply overreach — mitigated by report-only suggestions + DEFAULT-OFF actuation + compile-asserted never-levers; the escalation lanes' spend surfaces are untouched.
3. **Architectural risk:** none cross-repo at Phase A (local files + one command). The Phase B consumer note creates a katgpt-rs landing dependency — deliberately pointed at that repo's lane, conditional on Plan 642's continued absence, and never this repo's deliverable.
4. **Perf risk:** negligible — run-bound, not hot-path; the report command is read-only over a local jsonl.

## Out of scope

The corpus mining pipeline itself, the score-bench corpus generations, the escalation manifest's row vocabulary, service-side (worker) stats aggregation, and any change to `fix_verify`'s compile-gate semantics. This proposal tunes the posture around those surfaces, never their contracts.

## References

1. [arXiv:2604.21003](https://arxiv.org/abs/2604.21003) — "The Last Harness You'll Ever Build" (two-level harness/meta-evolution). **Distilled** at Research 401 (private, full) + Research 637 (public stub); this proposal reuses those distillations, no re-fetch.
2. [arXiv:2601.00728](https://arxiv.org/abs/2601.00728) — "Precision autotuning for linear solvers via contextual bandit-based RL" (fetched 2026-10-26). **Cited**: the bandit-over-bounded-config-vectors pattern with verification cost in the reward — the closest autotuning-domain shape; different domain (numeric precision), no mechanism taken.
3. [arXiv:2105.07202](https://arxiv.org/abs/2105.07202) — "FOGA: Flag Optimization with Genetic Algorithm" (GCC flag autotuning; cites OpenTuner, Ansel et al. CGO 2014, as the bandit-based state of the art). **Cited-only**: the compiler-heuristic-autotuning lineage the healer's lane belongs to.
4. Schmidhuber, "Recursive Self-Improvement since 1987" — **distilled** at [riir-refine Research 178](../.research/178_Schmidhuber_RSI_Essay_ThreeTrack_Distill.md) (meta-evolution lineage; keeper-is-arithmetic).
5. The LLM-harness class (DGM arXiv:2505.22954, HarnessCompass arXiv:2608.01918, JIT-Agent arXiv:2608.25593, ScholarEvolve arXiv:2609.40169) — **cited-only** via Research 401 §4's landscape; none taken (LLM-in-the-loop is the forbidden slot).

## Closing verdict

GO, staged: Phase A (always-on-with-demote instrumentation + the reports→actuator suggestion join + the self-firing volume-close line) is cheap, useful even under a null verdict, and starts the earliest-second-consumer clock; Phase B/C carry the honest-slow selection machinery behind the pre-registered volume gate, with the judging posture pinned arm-invariant so the evidence, if it ever accumulates, is real. Next action: open the Phase A plan in this repo when a session picks the lane; the katgpt-rs T1.2/T1.3 re-open waits for a Phase B commitment note AND Plan 642's continued absence — not this proposal.
