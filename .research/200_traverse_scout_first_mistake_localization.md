# Research 200: Traverse/Scout — First-Mistake Localization, the Cascade Law, and the Plan 342 Restart Path

> **Source:** "Locating Hidden Failures Makes Long-Horizon Agents More Reliable" — [arXiv:2609.17930](https://arxiv.org/abs/2609.17930) — Rahman, Kim, Parmar, Heydari, Liu, Lee, Zhang, Hosseini, Metwally, Yang, Mirzasoleiman, Liu, Izmailov, Gabriel, Malhotra, Patel, McDuff, Palangi (UCLA / NYU / Google Research / Google DeepMind), 2026-09-15. Traverse benchmark + Scout 4B verifier.
> **Status:** DISTILLED — modelless measurement filed as `Issue 128`; the model-based half is a restart path appended to riir-train [Plan 342](../../riir-train/.plans/342_genrm_l4_verifier_gate.md) (owner-gated on GPU windows — C13/riir-train Plan 318 owns the 4090).
**Date:** 2026-09-21
> **Classification:** Private
> **Related Research:** riir-train [Research 423](../../riir-train/.research/423_GenRM_Generative_Verifier_L4_Gate.md) (GenRM — Scout's direct ancestor; this paper is its long-horizon agentic continuation) · [Research 049](049_Long_Horizon_AI_Research_Evidential_Status.md) (long-horizon harness failure modes) · [Research 135](135_elo_selection_gate.md) (selection laws — the balanced-recall product is a selection-law cousin) · [Research 182](182_powerplay_frontier_miner_poc.md) (frontier/coverage discipline the measurement reuses)
> **Related Plans:** riir-train [Plan 342](../../riir-train/.plans/342_genrm_l4_verifier_gate.md) (GenRM L4 verifier gate — OPEN, the in-house Scout precursor) · [Plan 336](../../riir-train/.plans/336_code_llm_rlvr_clippy_l4_lora.md) (L4 drafter, terminal negative) · riir-clippy Plan 005 (the L4 contract)
> **Adversarial panel:** ran 2026-09-21, both advocates (No-GD + model-based) + independent prior-art sweep (~20 searches). Discards are audited in §6.

---

## TL;DR

Google trained a 4B verifier (**Scout**) that locates the *first mistake* in long agent trajectories better than six frontier judges (all <⅓ exact; all systematically over/under-flagging biased), and used it for best-of-N selection that beats the agents' own pass@1 (Terminal-Bench 81.8→90.2%). The paper's engine is **statistics over gold-labeled step sequences** — and riir-clippy's `.heal/fixseq/` ring *writer* persists exactly that data shape with *machine-checked* labels (strictly more deterministic than the paper's human annotations) when repair runs happen. ⚠ **Premise measured 2026-09-21: the store is EMPTY on this box** — zero `run-*.jsonl` under any workspace repo's `.heal/fixseq/` (katgpt-rs, riir-ai, riir-train, riir-chain, riir-neuron-db all checked) — so corpus generation is part of the filed work, not a precondition. Three things land here:

1. **Modelless (Issue 128):** the **cascade law** — P(next attempt bad | prev bad) vs P(next bad | prev good), the paper's 13.7×/10.9× jump — has never been published for repair-driver attempt sequences; the **derailment index** (first-wrong-attempt) and **silent-failure rate** (failed runs ending clean-looking) are one-pass statistics over the ring format the driver already writes. The statistics are CPU-only once a corpus exists — **the corpus itself does not** (T1's census expects empty on this box; T5 generation is the entry cost). The minted corpus also feeds `horizon_report`'s standing refusal — with the honest caveat that synthetic rings do NOT by themselves satisfy the 093/125 *organic* reopen triggers (§6 #4).
2. **Model-based (riir-train Plan 342 addendum):** Scout *is* the GenRM-CoT + RL recipe that riir-train Plan 342 was reaching for. The paper's two headline techniques repair Plan 342's two documented blockers exactly: **label-faithful CoT synthesis with rejection sampling** manufactures the missing L4-draft anchor distribution (T0.2), and the **product reward r = 0.7·Rc·Ri + 0.3·verdict** is the GRPO-phase reward with a built-in anti-degenerate shape. The AUC-gate problem stays (modelless floor 0.9383, riir-train Bench 491) — but the paper supplies the un-floored axes to re-state G1 on: first-mistake localization and judge-balance (FAR/over-under-flagging), where no heuristic floor competes.
3. **Design-pattern confirmation (no action):** the Rc·Ri product-vs-average law is the same anti-degenerate design this workspace already practices independently in `tier_guard` (Lemma-4.2 two-direction ratio), `arm_reach_gate` (MIN_KILLED floor), and every dual-direction membership pin. The paper's 84.1% silent-failure rate is the quantitative justification for the green-zero/trap-launder family in katgpt-rs. Cite, don't rebuild.

**Not Super-GOAT.** Prior art caps the broad claims (SWE-Gym, GenRM, TRAIL, AgentDebug — §5); the surviving novelty is two narrow measured-law claims on our own data plus a stuck-plan rescue. No new pillar; no katgpt-rs open primitive (nothing here is engine-layer math).

---

## 1. Paper core (the numbers that matter)

**Corpus:** 2,518 real agent trajectories (SWE-bench 565 / TerminalBench 1,851 / BixBench 102; Claude, GPT-5, Gemini, DeepSeek, Qwen, Kimi backbones over 5 harnesses), 28–34 avg steps, up to 200. 6,967 mistakes human-labeled per step (κ=0.77 failed / 0.73 where), typed against a pre-registered 88-category codebook (78 observed, 10 never fired).

**The failure signature** (after the first mistake): no recovery 69.5% · never self-detects 38.5% · keeps acting 72.6% · all three 13.7%. Self-detection rarely helps anyway: recovery-after-noticing ≈ recovery-without-noticing (76% vs 76% SWE, 18% vs 16% terminal — noticing ≠ fixing; a Simpson's paradox hides in the pooled number).

**The cascade law** (deterministic, from gold labels only — no judge): P(next step wrong | prev wrong) = 40.4% SWE / 58.1% terminal vs 3.0% / 5.3% after a correct step — **13.7× / 10.9×**. Root-fault share falls 58%→46%→39% across the run's thirds: the longer a broken run goes, the more of its errors are inherited, not fresh.

**Cause ≠ symptom:** action/execution faults are 45.1% of all mistakes but only 24% of *first* mistakes and 65% cascade; reasoning+planning are 46% of first mistakes (69% root). What fails most is not what starts failures.

**Silent failure:** 84.1% of failed runs end on a locally correct-looking step (SWE 70.7%, terminal 87.6%). The final state carries no sign anything went wrong.

**Recovery is an environment property, not a scaffold property:** SWE agents are blind (miss 76% of own errors) yet solve 72% — tests/tracebacks catch what their reasoning misses; terminal agents notice errors but solve 19% (no rich feedback). Signature recurs across all 4 harnesses.

**Safety:** 65 unsafe actions — 100% task-unnecessary, 97% risk-unacknowledged, 75% irreversible; **77% missed by regex scan, 54% invisible to per-step gold labels** (only a whole-trajectory semantic read catches them). Plus a fabricated-success cluster (faked solutions, mocked outputs, forged checks) concentrated where no automatic check exists.

**Judges:** 6 frontier models, exact first-mistake localization 7.2–32.3%, none above ⅓; every one systematically biased (GPT-5.5 over-flags P60.9/R82.0; Opus 4.6 under-flags P66.0/R31.7). Detection (binary did-it-fail, F1 up to 88.2) ≫ localization. ±3-step tolerance nearly doubles accuracy — judges sense where, rarely land.

**Scout:** Qwen3-4B-Thinking, SFT on 2,416 **label-faithful synthesized CoT trajectories** (teacher writes the analysis *conditioned on gold labels*; regenerate until parsed verdicts exactly match gold — rejection sampling against the oracle), then GRPO (G=4, 128 rollouts, 8 ep, lr 1e-6, KL β=0.001) with **r = 0.7·(Rc·Ri) + 0.3·1[verdict correct]**. The product of recall-on-correct × recall-on-incorrect kills the degenerate all-correct strategy that (Rc+Ri)/2 rewards at 0.5; the verdict term is the cold-start signal before localization emerges. Beats all six judges, transfers to the unseen science domain, and best-of-N selection lifts Terminal-Bench 81.8→90.2 / SWE-bench 75.6→78.0.

---

## 2. Per-track verdicts (one per track, never pooled)

| Track | Verdict | Artifact |
|---|---|---|
| (a) modelless inference | **GAIN** — Issue 128 (cascade law + derailment index + silent-failure rate on fixseq rings; `horizon_report` feature extension) | `.issues/128_…` |
| (b) self-adaptive runtime | **PASS** — the signature-triggered escalation policy is priced against the closed negative of Issue 093/Benches 089–090 (no learned ordering/policy beat static min-delta; reopen triggers = organic multi-revert rings, unsatisfied); recorded here, not filed | — |
| (c) model-based | **GAIN (secondary, owner-gated)** — riir-train Plan 342 restart path (label-faithful synthesis for T0.2 anchor + Rc·Ri/verdict GRPO reward + localization-axes G1 re-statement); hardware routing respected — SFT→M3 per riir-train Bench 523 (1027 s/step on the 4090), synthesis/GRPO→4090 (dense paths) | riir-train Plan 342 addendum |

---

## 3. Path 0 inventory (coordinator-curated; advocate rows marked †)

Coverage column filled by the coordinator after §6 signal-diffs — advocates do not own coverage.

| # | Component | Modelless? | Consumer surface | Coverage verdict |
|---|---|---|---|---|
| 1† | **Cascade/stickiness law** — 2-state conditional matrix P(bad_{t+1}\|bad_t) vs P(bad_{t+1}\|good_t) over attempt outcomes | Yes — pure counting, 4 counters | `.heal/fixseq/` rings + `horizon_report.rs` | **OPEN** — nothing computes transition-conditionals on our rings; `horizon_report` predicts final outcome (lead-k), not next-attempt transition. Prior art: unpublished on repair loops (§5) |
| 2† | **Derailment index** — first-wrong-attempt under a per-attempt badness indicator; ±k tolerance curves | Yes | ring post-run report; `--misses` queues | **OPEN as statistic** — localization-as-task is published (TRAIL/AgentDebug/Traverse); as a cheap ring statistic it is a domain transfer, filed |
| 3† | **Root-vs-cascade attribution** — a fault at t is cascade iff an origin-family fault precedes it | Yes — backtracking pass | ring analysis; the `masking[b,a]` field IS a root signal (E0614 = root; borrowck-blocked = cascade) | **PARTIAL** — Issue 100's keep rule already encodes root-priority (masking dominates count); attribution-as-analysis is additive, not new machinery. Folded into Issue 128 T4 |
| 4† | **Silent-failure rate** — % failed runs ending on a good-looking attempt | Yes | run-tail telemetry; harness-quality scalar | **OPEN for the driver** (the katgpt-rs green-zero family is the gate-side sibling — different surface, same law; cite both) |
| 5† | **Feedback-richness → recovery predictor** — rank harness feedback quality against recovery rate | Yes — rank correlation | auto-oracle posture design (`clippy_oracle.rs`) | **PASS** — validates the already-shipped default-on oracle posture (`--all-targets` since 09-21); no config change indicated |
| 6† | **Rc·Ri balanced-recall product** — scoring/judge-selection law; degenerate-resistant vs mean | Yes as a scoring rule | `tier_guard.rs`, `arm_reach_gate.py`, dual pins (already practiced); GRPO reward (track c) | **COVERED as a design pattern** (§6 signal-diff #1) — the paper adds the published citation + the training-reward instantiation; no new modelless primitive |
| 7† | **First-mistake family prior** — 10-family frequency LUT as ordering prior | Yes | rustc_errors strategy menu | **PRICED, NOT FILED** — ordering priors are the Issue-093 closed-negative territory; fold the *measurement* (which strategy families start terminal failures) into Issue 128 T4 output, consume only if organic rings satisfy the 093 reopen triggers |
| 8† | **Judge-bias confusion quantification** — per-judge P/R split, over/under-flag detection | Yes | multi-probe consensus; `elo.rs` (already sigmoid-based) | **PASS** — no live multi-judge consensus surface exists to fix today; the riir-train Plan 342 addendum carries the balance axis for the verifier gate |
| 9† | Best-of-N selection law | Yes (Beta-LCB argmax already ships as `select_best_candidate`) | — | **COVERED** — BoN-via-Elo is shipped; the paper's BoN-via-trained-verifier is track (c) |
| 10–18† | Irreversible-action screen, termination-verification law, positional-uniformity guard, ±k window bound, detection→localization scheduler, fabricated-success invariants, escalation controller, root-decay schedule, judge blending | modelless | various | **CONFIRMATORY / katgpt-rs-side / priced** — #15/#16 are the shipped trap-launder + completion-sentinel family (quantified, not new); #13 lands inside Issue 128 T7 (guard for the law); #14/#17 priced against Bench 089–090 + owner risk appetite; #11 scheduler shape already ships as the anchor lane; #12 no live surface |

---

## 4. Fusion (paper × our substrate)

**Fusion A — cascade × horizon_report × traj_store (Issue 128):** the ring writer records ordered attempts with oracle outcomes; `horizon_report` already asks "can ring features predict final failure k attempts ahead?" with 9 features — none of them the *transition* conditional. Adding P(bad|prev-outcome) as feature #10 + the law as a hypothesis test turns three dead instruments (rings sparse → horizon refuses) into one corpus-generation loop: the score-bench generations harness + `corpus_gen::compose_multi_strided` mint labeled multi-attempt rings at scale with the real clippy oracle. Honest caveat (verdict round 2): minted rings are synthetic — they unblock `horizon_report`'s refusal, but they do **not** by themselves satisfy the *organic* reopen triggers of Issue 093 (organic multi-revert rings) or choice_scorer (organic rings / oracle-anchored *real-repo* spans); whether a synthetic corpus counts for the latter is an owner adjudication, not a given.

**Fusion B — Scout × riir-train Plan 342 (track c):** Plan 342's blockers ↔ the paper's techniques, one-for-one:

| riir-train Plan 342 blocker (measured) | Paper's repair |
|---|---|
| T0.2: L4-draft anchor never persisted; verifier anchored on synthetic corruptions | `--dump-drafts` (already built) + **label-faithful synthesis**: regenerate L4 drafts, teacher-write CoT conditioned on the compile-gate label, rejection-sample until verdicts match |
| One-token p(Yes) targets → eval 0.0000 on banked ckpt; `t41_genrm_cot_corpus.jsonl` (3.6MB) started but unfinalized | Scout's whole training signal **is** the CoT-then-verdict target; paper proves the format learns where one-token doesn't |
| RLVR deferred (riir-train Plan 336 entry p≥0.082 — cold start) | the **verdict term 0.3·1[ġ=g]** is the paper's own cold-start fix: verdicts are learnable before localization emerges |
| Reward hacking (keep-everything strategy; the Issue-100 strand-keep class) | **Rc·Ri product** — keep-everything scores Ri=0 → reward 0 |
| G1 AUC refuted by the 0.9383 modelless floor (riir-train Bench 491) | re-state on the axes the floor doesn't touch: first-mistake localization on attempt sequences, over/under-flag balance — no heuristic floor competes there (the floor is a *detector*, not a *localizer*) |

GPU honesty: pilot portfolio ≈ 55–110 GPU-h on the 4090 (synthesis 4–10, SFT 4–8, subscale GRPO 25–70, session-Scout lane 6–12); paper-scale GRPO (>500 GPU-h, 128×8K rollouts × 475 steps) is **infeasible here** — subscale (G=8, 64 rollouts, 2K responses, 100–200 steps, CISPO not PPO-clip) is the honest pilot. Backbone: gemma-2-2b-it on disk (rings are 2–8K tok; the 128K ctx is the paper's tool-output bloat, not ours); Qwen3-4B acquisition is an owner call. Envelope: per-run seams only (≤5 s/candidate bar already in riir-train Plan 342's G2) — **never per-span** (choice_scorer's 27.7%-vs-88.0% refutation is the standing proof the hot path stays modelless).

---

## 5. Novelty analysis (prior-art sweep, 2026-09-21, ~20 searches)

**Pinned claims BEFORE search (§1.5 precondition):**
- C1: "The two-conditional stickiness law P(next attempt bad | prev bad) vs P(next | prev good), measured on compile-repair-driver attempt sequences with oracle labels, distinguishing riir-clippy's healer from prior art by the sequence type (single-agent repair loop, not MAS message graph) and the label source (deterministic compile oracle, not humans)."
- C2: "Step-level first-wrong-attempt localization + best-of-N selection over rustc-error-keyed multi-attempt repair rings — SWE-Gym's function at GenRM's cost point on RePair's domain, unpublished as the intersection."

**Kills and survivors:**
- **SWE-Gym** (arXiv:2412.21139) — trained verifier + BoN on coding-agent trajectories. Kills any generic "verifier BoN for coding agents" claim. C2 survives only as the *step-level, attempt-sequence, error-code-keyed* cell — cite in the first line of any consumer.
- **GenRM** (arXiv:2408.15240 = riir-train Research 423 / riir-train Plan 342) — small generative verifier beats LLM-judge + BoN. Scout is its agentic continuation; nothing to claim.
- **TRAIL** (2505.08638) / **AgentDebug** (2509.25370) / **Echo** (2510.04886) / **Silent Failures** (2511.04032) — trace error localization as *task/benchmark* is crowded. C1's law (conditional transition statistic) survives; C2's derailment-index-as-cheap-statistic survives as domain transfer, NOT as "first error-localization method" — do not word it that way anywhere.
- **From Spark to Fire** (2603.04474) — conditional cascade dynamics on **MAS message graphs** with injected seeds; nearest in spirit to C1, different object entirely. Cite as the MAS-side sibling.
- **Rapid Fixes, Gradual Failures** (ICML 2025) — attempt-indexed *marginal gain* curves on iterative self-repair (the inverse statistic, same data shape). Cite; C1's two-conditional form is unpublished.
- **Rc·Ri = G-mean** (Kubat & Holte, ICML 1998) — the metric is textbook. The paper's contribution is instantiating it as a PRM training reward with the measured degenerate-strategy consequence; ours is the confirmation that tier_guard/arm_reach/dual-pins practice the same law. No novelty claim.
- Traverse itself is v1 (2026-09-15), no citing follow-ups yet.

**Net: two narrow open claims on our own data, both measurement-first. Neither is Super-GOAT (Q2 no new behavior class; Q3 no product selling point; Q4 moderate multiplier).**

---

## 6. Signal-diffs + audited discards (§3.5/§3.6 discipline)

1. **tier_guard LR test vs Rc·Ri** — READ `src/tier_guard.rs`: the Lemma-4.2 test consumes (Elo-model expected score, Laplace outcome history) per rule and certifies only when both agree within ε=0.5, guarding ≪1 (free-rider credit) and ≫1 (lucky-keep overcert) — a two-direction *consistency* gate on promotion. Rc·Ri consumes per-step confusion counts vs gold and *scores a judge*. Different inputs, different consumers; **same anti-degenerate design family, not coverage**. The paper's reward instantiation lands only in track (c). Discard of "new modelless primitive needed": ** upheld — already practiced three independent times in-tree**.
2. **Issue-100 keep rule vs first-mistake localization** — the keep rule is greedy-per-attempt over (masking, count) deltas; localization is run-level over the whole sequence + final outcome. The keep rule cannot express "which attempt started the terminal failure". **Not covered; filed** (Issue 128 T4).
3. **horizon_report vs cascade law** — lead-k decodability (ridge readout, 9 features, refuses on <2-attempt walks) vs one interpretable transition statistic + law hypothesis. **Extension, not duplication** (Issue 128 T6).
4. **nonergodic closure vs escalation controller** — Issue 093 closed ordering negative; Bench 089/090 proved the *signal* fix (keep rule) was the win. The signature-triggered escalation is a policy change in the closed-negative's territory with the same reopen triggers unsatisfied. **Discarded from filing; reason: mechanism-level — a trigger policy is an ordering-family intervention, and the measured null (no ordering beats static min-delta at equal budget) applies until organic multi-revert rings exist.** Recorded here for the reopen file.
5. **No-GD advocate rows 10–18** — merged into inventory #10–18 with confirmatory/priced verdicts; none discarded silently. The two strongest (#1 cascade, #2 derailment) are filed; #4 silent-failure folded into Issue 128 T4; #5/#8 PASS with reasons above.
6. **Model-based advocate lane 7 (workspace-session Scout over commits/gate-runs, the katgpt-rs Issue-840 attribution class)** — genuinely open white-space (no modelless competitor exists; katgpt-rs Issue 840 measured all three fallbacks broken), but it needs lane-1 synthesis first and a serving cadence decision. **Deferred to the Plan 342 addendum as the third lane, owner-gated** — not silently dropped; the honest blocker is GPU contention with C13, not merit.

---

## 7. Context reframes (§1 step 4)

**Game (priority #1, riir-ai):** the direct mapping is thin and honestly so — NPCs are modelless and the 20 Hz path has no LLM agent in it. Two real touchpoints: (a) `cgsp_runtime`'s MCTS collapse detection/recovery is the shipped analog of the paper's recovery axis — the paper's finding that *environment feedback, not internal noticing, drives recovery* is a design cue for collapse-recovery triggers (prefer task-signal-anchored over coherence-anchored triggers); (b) `riir-agents` DeLM shared-context failure-reuse (GOAT 1.000 vs 0.870) is our shipped "learn from failure at the agent layer" — the paper's cascade law predicts *when* reuse pays (early failures contaminate everything after). Neither warrants a riir-ai filing on its own; both are recorded for the next session touching those crates.

**Consumer/healer (priority #2, riir-clippy):** the primary consumer — the compile-repair driver is our long-horizon agent (minutes, dozens of attempts, oracle-labeled every step), the ring is the paper's trajectory, and the entire §3 inventory targets it.

**Workspace self-application (katgpt-rs doctrine):** the paper quantifies what this workspace's gate family already encodes — 84.1% silent failure ↔ the green-zero rule; fabricated-success clusters ↔ trap-launder/BASELINE-RED arms; "recovery depends on environment feedback" ↔ why every layer's failure must be loud (refuse, never warn). No file changes; the numbers are now citable.

---

## 8. Files

| File | Kind |
|---|---|
| `riir-clippy/.research/200_traverse_scout_first_mistake_localization.md` | this note |
| `riir-clippy/.issues/128_fixseq_cascade_law_derailment_index.md` | modelless measurement (track a) |
| `riir-train/.plans/342_genrm_l4_verifier_gate.md` §Addendum 2026-09-21 | restart path (track c, owner-gated) |
| `.research/.highwater` 199→200, `.issues/.highwater` 127→128 | numbering |

---

## 9. Honest negatives / risks

- The cascade law might NOT transfer to repair loops: our driver *reverts* bad attempts (the paper's agents barrel ahead), which mechanically interrupts cascades. The measurement is the point — a null here is a real result about the keep-rule's design (it is a cascade-breaker), and Issue 128's gate prices both outcomes. And the store starts EMPTY on this box (measured 2026-09-21): the corpus is minted, so a law reproduced only within one generator is generator-conditional (Issue 128 T7's disjointness constraint) — the organic replication is what would make it a law about the *driver*.
- GRPO at paper scale infeasible (§4); subscale divergence (one-step-per-batch, ratio=1, CISPO-vs-PPO) is stated, not hidden.
- riir-train Plan 342's kill line (L4-draft AUC < 0.75) still governs; the addendum changes the *path*, not the *bar*.
- GPU contention with C13 (riir-train Plan 318) is real and governed — nothing in this note launches a GPU job.
