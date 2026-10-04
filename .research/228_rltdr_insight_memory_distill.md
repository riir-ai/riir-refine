# RLTL;DR — self-generated insight internalization: distill verdict

**Status:** RECORD — Gain, dual-track. Modelless track: issue 140 filed (insight-conditioned fix retry + abstraction-gated insight store POC). Model-based track: riir-train Plan 432 filed (SFTL;DR on (span, insight) tuples + L4 revival). Game-side fusion lead (NPC lesson memory — prior-art claim 7 clean) filed as riir-ai `.issues/1022`. Novelty sweep record: `../katgpt-rs/.research/599_rltdr_novelty_check.md`. **Addendum 2026-10-04: the diversity tripwire is RESOLVED at the code level** — riir-refine `9cfd0c33` (the E0382/E0505 move ladders: clone-at-move reverts on non-`Clone` at equal count, the `use-before-move` swap rung keeps) manufactures the fail→resolve saddle on demand: 24→58 saddle tuples, E0502-only→{E0502,E0505,E0382}. The saddle-manufacturing law itself is a distillable: **a two-rung strategy ladder where rung-1 is type-gated to fail (equal-count code-swap → revert) and rung-2 recovers — the corpus-side twin of the paper's fail→insight→success shape.**

- Source: arXiv:2609.37633 "RLTL;DR: Self-improvement by Internalizing Self-generated Feedback" (Kirchhof et al., Apple, 2026-09-29). Qwen 3.5 9B Thinking policy, 8xB200, 1-4 days/run. License CC BY-NC-ND (paper text; the MECHANISMS and measured laws below are facts, not code — no license constraint on independent implementation).
- Read: full HTML (arxiv.org/html/2609.37633v1), 2026-10-01.

## TL;DR

RLVR/GRPO starves when Pass@128=0 (all-negative groups → zero advantage → no learning signal). RLTL;DR breaks the barrier with two moves: (1) **sequential insight-conditioned rollouts** — after each failed attempt, the policy (in a FRESH context, given the verifier's failed-unit-test outputs) writes a one-sentence TL;DR "insight" (≈17 tokens, e.g. "Remember to paginate search results"); the next attempt is conditioned on all accumulated insights; (2) **insight internalization** — an SFT loss on the in-context insight tokens conditioned on the task ALONE (π(f₁..f_I|g), implemented as a backprop-mask flip, λ=0.5). Eval Pass@1 without insights: GRPO 0-1% → RLTL;DR 12-13%. The reduction (**SFTL;DR**) is the headline: training ONLY on 4k deduplicated (task, insight) tuples — 4.6M forward / 68k backward tokens, never backpropping (or forwarding) any rollout — recovers ≈ full RLTL;DR ≈ SFT on 3.5k full rollouts (400M/6.6M). "On this sort of task, keep this sort of thing in mind" is a ~100× compression of the learning signal.

## The five measured laws (the transferable content — all runtime facts, none require training)

| # | Law | Measurement |
|---|-----|-------------|
| L1 | **Abstraction level gates transfer.** One-sentence GENERIC insights internalize + transfer to unseen tasks; detailed diagnostic paragraphs / summaries / corrected code help MORE in-context (+41.1% vs +36.0%) but transfer WORSE when trained on (13.8-15.7% vs 21.4% train Pass@1). Mechanism: episode-specific literals (IDs, tokens, paths) are unpredictable from the task alone → label noise. |
| L2 | **Verifier access is the critical signal.** Insight generator without failed-unit-test outputs: 3.4% vs 21.4%. Generator identity (student vs GLM 5.2 teacher, thinking on/off) matters little — the privileged information, not the generator, carries the value. |
| L3 | **Condition on distilled insights, never raw attempt histories.** The SGE baseline (prepending summaries of previous attempts, no failure reflection) FAILS — "contextual drag" misleads next rollouts. Insight generation in a FRESH context (not continuing the failed chat) for the same reason. |
| L4 | **Goldilocks consult gate.** Insights inserted only when the running success rate ≤50% — target the struggling regime, skip the solved one. Robust to the exact threshold. |
| L5 | **Insight quality is gaugable at runtime.** Insight advantage (Pass@1 with − without, must stay >0), too-easy ratio (<20% — insight makes the task trivial), too-hard ratio (<50% — insight never helps). Insight reliance (per-token log-lik delta with/without) stays mild (+0.034) — used without being leaned on. |

Plus the exploration result: sequential insight-conditioned sampling finds ≥1 solution for 57% of Pass@128=0 tasks vs 6% i.i.d. at k=16 (frozen model, no training — the loop alone).

## Path 0 decomposition (component → coverage → extraction)

| Component | Ships? | Analog in our stack | Delta |
|-----------|--------|---------------------|-------|
| Sequential feedback-conditioned retry loop | PARTIAL | rustc_errors `CompileDriver` retries with strategies; `.heal/fixseq/` rings record every attempt (seq, error code, strategy, outcome, errors before→after); `nonergodic_order.rs` orders the frozen strategy menu per-run | Order ≠ conditioning. No attempt is conditioned on DISTILLED lessons from prior failures in the run — the ring is write-only today |
| Verifier-in-the-loop | SHIPS | clippy/rustc oracle (`clippy_oracle.rs`, `--clippy-json` feed, `fix_verify` compile gate) — denser and cheaper than the paper's unit tests | — |
| (task→insight) retrieval store | NOT | Latent retrieval is span→RULE over a corpus-curated rulebook; `self_evolve`/`traj_store` store trajectories + Elo/Beta counters, not distilled lesson text; `context_notes` are corpus-authored annotations, not runtime-mined | The §7.4 lane: runtime-mined, verifier-anchored, abstraction-gated one-sentence lessons, retrieved by task similarity |
| Abstraction-level admission (L1) | NOT | traj_store evidence tiers (Certified/Heuristic/Withdrawn) certify OUTCOME, not GRANULARITY; neuron-db consolidation has no literal-density filter | `admit(f) ⇔ len ≤ S_max ∧ literal_density(f) < θ` — a closed-form predicate |
| Goldilocks consult gate (L4) | NOT | `demonstrability_gate` scales pool depth by domain constant (measured FLAT, Issue 134); nothing gates on running success rate | `consult ⇔ p̂_run ≤ 0.5` — one sigmoid (house law: sigmoid, never softmax) |
| Insight-advantage gauges (L5) | PARTIAL | `tier_guard.rs` Lemma-4.2 LR test certifies durable tiers by goal-side vs outcome-history agreement — the SAME bookkeeping shape | Extend from rule-rows to insight-rows (withdraw on LB95(advantage) ≤ 0) |
| Internalization without GD | PARTIAL | `self_evolve` updates Elo/Beta/ratings from OUTCOMES (rule-keyed) | Update from INSIGHTS, task-fingerprint-keyed (task-class direction vectors, routing-table boosts) — the legal latent-update class |
| Positive-ratio filtering | PARTIAL | `loss_grpo.rs` ships `mask_zero_advantage_groups` (post-hoc sibling) | Pre-advantage filter to 75% positive — prevents the all-negative group from forming (riir-train Plan 432 rider) |
| SFT on (task, insight) tuples | NOT | — | riir-train Plan 432 (the cheap headline: 68k backward tokens) |

## Dedup-grep evidence (workspace, both vocabularies)

- `self_evolve.rs` + `traj_store.rs`: outcome-tier certification, Elo/BetaPosterior selection, `EvolveRecorder` — numbers and ratings, no lesson text, no attempt-conditioning seam. The 213/reef note's dedup section states it exactly: "outcome gates on numbers".
- `nonergodic_order.rs` (Issue 093/099/100, closed-negative): per-run strategy POSTERIOR ordering. Signal-diff: ordering reweights a fixed menu by history; RLTL;DR conditioning injects NEW distilled information into the attempt. Different inputs (outcome counters vs verifier-output-derived text).
- `tier_guard.rs`: the LR-test is the insight-advantage monitor already armed for a different lane (rule rows).
- `frontier_miner`: manufactures the Pass@k=0 population (unsolved fixtures) — the EVAL SET this note's POC needs, for free.
- riir-neuron-db `consolidation.rs` (Raven/δ-Mem): sleep-cycle consolidation exists; no abstraction-level admission (grep clean), so L1 as a promotion filter is a real delta.
- riir-ai `cgsp_runtime`: curiosity/self-play ships; no lesson/skill text memory (grep clean).
- katgpt-rs `.research/599` (created this session; second-pass addendum appended): the 7-claim novelty sweep — claims 2/3/4/5 (insight DB+retrieval, feedback-conditioned retry, sequential insight sampling, context distillation) are rich prior art (ExpeL 2308.10144, Voyager 2305.16291, AWM 2409.07429, Self-Debugging 2304.05128, RustAssistant 2308.05177 — the exact rustc domain, HiLL, SDPO 2601.20802, RLTF 2602.02482, ECHO 2605.24517); claims 1/6 (the abstraction-level transfer ablation, the success-rate consult gate) came back NOT FOUND. Claim 7 (game-NPC lesson memory) was NOT FOUND at round 1 but RETRACTED by the second pass: Generative Agents (Park et al. 2304.03442) reflection IS the episode→generic consolidation + retrieval mechanism for simulated agents — riir-ai 1022 carries the narrowed engineering-envelope scope.

## Verdict: Gain, dual-track (per-track, never pooled)

**Not Super-GOAT** — Q1 fails: the insight-database+retrieval mechanism is published prior art (ExpeL 2023 onward; the paper's own §7.4 concedes and cites it). What the paper adds — and what we take — is the MEASURED DESIGN LAWS (L1-L5) and the ~100× SFTL;DR compression, both directly applicable to the healer (fusion priority #2) and cheaply trainable (riir-train, Path 0.5).

- **Track (a) modelless — GOAT-tier application.** The healer is the paper's setting wearing our names: fix attempts = rollouts, clippy/rustc = the verifier (denser than theirs), the fixseq rings = the sequential rollout log already collected (the paper's ~1B-token collection cost is SUNK for us), the frontier fixtures = the Pass@k=0 population. The insight-conditioned retry loop + the (span→insight) store + L1/L4 admission gates are all runtime objects with zero gradients. Actionable → issue 140.
- **Track (c) model-based — GOAT-tier plan.** SFTL;DR on (span, insight) tuples mined from the rings: teacher = Bonsai-2-27B over the paper's 5-part scaffold, student = 1-2B or `.bcl4` LoRA; 3-10 GPU-h for the core. The L4 revival (Plan 432 Phase 2) is the highest-leverage consumer — as a HYPOTHESIS, not a premise: Plan 336's own record (T2.5) attributes the 0/60 to training instability (eval ppl 5.37→569 by step 100) and module-copying, and the verifier-grounded mining fallback (Issue 456 / Bench 462) was tested and REFUTED — so "this recipe supplies the missing signal" is one competing explanation (signal) among at least two (capacity, instability), and Phase 2's T2.3 negative branch is the instrument that decides. Actionable → riir-train Plan 432.
- **Track (b) self-adaptive — folded into (a):** closed-form internalization (Elo/centroid/routing updates FROM admitted insights, task-fingerprint-keyed) is the legal latent-update class; ships as issue 140's T4.

MOAT check: riir-clippy moat row ("retrieval floors, score-bench heal rate, OOD generalization, fixer coverage") — this targets fixer coverage on the frontier class. In scope.

## Mandatory outputs

- **Issue (this repo, `.issues/140`)** — insight-conditioned fix retry + abstraction-gated insight store POC (the modelless lane).
- **Plan (riir-train `.plans/432`)** — SFTL;DR insight internalization + L4 revival (the training lane, Path 0.5: recipe + GPU-hours + GOAT gate vs modelless baseline).
- **Issue (riir-ai `.issues/1022`)** — NPC lesson-memory fusion lead (novelty TBD, mechanism-novelty RETRACTED vs Park et al. 2304.03442 at round 2 — any claim scoped to the engineering envelope: crowd scale, two-brain sync law, L1 admission filter, L4 consult gate).

## Fusion section (novelty TBD, not claimed)

- **L1 × traj_store evidence tiers**: a THIRD admission axis (granularity) beside outcome and review-class — detailed traces stay in the ephemeral ring for offline mining; only abstraction-passed one-liners reach the durable tier. Two-tier memory made structural.
- **L4 × demonstrability pool-depth family**: the consult gate is what Issue 134's flat pool-depth axis should have been — gated on measured running success, not a domain constant. Reuses the same measurement surface.
- **Paper × cgsp_runtime × NeuronShard (riir-ai 1022)**: NPC fails task → game predicate = verifier → one-sentence lesson → NeuronShard latent entry → sleep-cycle consolidation promotes episode→generic (L1 as the consolidation filter) → retrieved by task similarity only when competence is low (L4, one-way think-brain per the two-brain law). Measurable on the SCN solvability floor.
- **Insight store × replay/verifier lane**: insights condition CANDIDATE GENERATION, never the judge — the K=2 quorum stays blind, preserving measurement honesty.

## Caveats

- Leetcode overfit in the paper: domains with little shared rule structure don't transfer. Lint classes ARE named shared rules (the good regime), but the healer's hardest spans may be Leetcode-shaped — the POC must report per-family transfer, not pooled.
- The paper's "semantic smoothness" requires scale (9B Thinking). Our 1-2B students internalize a 17-token mapping, not a rollout — the rule_embed_poc precedent (94→98 top-1 at d=64) supports small-model capacity for exactly this shape; the 27B teacher carries the insight WRITING.
- NOT FOUND claims rest on a search sweep with timeouts (599's own caveat) — treat L1/L4 novelty as open, not proven; claim 7 was retracted at round 2 (Park et al.) and no mechanism claim should be made from it.
- The paper is CC BY-NC-ND: we take mechanisms and measured facts for independent implementation, not text.
