# Research 178: Schmidhuber "Recursive Self-Improvement" (IDSIA-9-26) — Three-Track Distill

> **Source:** J. Schmidhuber, "Recursive Self-Improvement (RSI) since 1987", Technical Note IDSIA-9-26, 17 Sept 2026 — https://people.idsia.ch/~juergen/recursive-self-improvement.html (essay/survey; CC BY-NC-SA)
> **Date:** 2026-09-17
> **Status:** DISTILLED — pending owner decision (issues 122 here + riir-train 559 filed; no plan opened)
> **Related Research:** 167 (RSI-survey autonomy-ladder positioning — the prior RSI distill in this repo), 174/175 (batch context), katgpt-rs 320 (Red Queen Gödel Machine → Plan 336 `best_belief.rs`), katgpt-rs 512 (Meta^n RSI, PASS), katgpt-rs 526 (fast weights), katgpt-rs 165 (healer curriculum)
> **Cross-ref (riir-train):** .issues/559 (quest-grammar RFT→GRPO loop)
> **Classification:** Private (healer-domain distill; home is riir-clippy per the MOAT table)

---

## TL;DR

Schmidhuber's RSI essay is a 39-year survey (Meta Evolution 1987 → self-modifying policies/SSA 1994 → fast-weight programmers/SRWM 1991-2022 → OOPS 2002 → Gödel Machine 2003 → adversarial curiosity → PowerPlay 2011 → DGM/HGM 2025-26). **Verdict: Gain (modelless track).** The workspace already ships modelless forms of seven of the eight extractable mechanisms — most strikingly the healer's `fix_verify` compile gate IS the Gödel-machine acceptance rule and the frontier band `0 < p̂ ≤ 0.5` IS PowerPlay's capability frontier, measured. The ONE uncovered, documented-gap-pulling extraction is **PowerPlay's generator half**: a modelless fixture generator that constructs the simplest code shapes the current fix corpus cannot repair (oracle-verified), which simultaneously feeds Issue 089's unwired frontier admission gate, creates the interacting-error corpus Bench 087/099 say cannot exist today, and unblocks the flip-dynamics gain axes (horizon report, necessity swap-donor pool). Filed as **issue 122**. Training-track top item (RFT→GRPO quest-grammar loop) filed as **riir-train issue 559**.

---

## 1. Source digest (what the essay claims)

| Sec | Mechanism | Acceptance/keeper rule (the load-bearing half) |
|---|---|---|
| 1 | Meta Evolution (1987) — GP applied to itself, recursively | meta level inherits the object level's selection |
| 2 | Self-modifying policies + EIRA/Success-Story (1994-97) | keep a self-modification ONLY when the life-long journal shows reward-per-time accelerated after it; single trial, no resets |
| 3 | Fast Weight Programmers (1991) + Self-Referential Weight Matrix (1992; modern 2022) | slow net writes fast net's weight deltas; self-referential net addresses its own O(n) connections via O(log n) outputs; unnormalized linear Transformer ≡ 1991 outer-product FWP; VS-ML: forward dynamics alone can implement learning algorithms |
| 4 | OOPS (2002) — ordered problem solver | Levin-budget search biased to programs REUSING earlier solution prefixes; nothing counts as solved unless the oracle says so within budget; asymptotically time-optimal curriculum |
| 5 | Gödel Machine (2003) | rewrite any of its own code only after a machine-checkable PROOF (axioms include the utility function) that expected utility increases; global optimality from the stop rule ("proved that continuing the search is useless") |
| 6 | Adversarial curiosity (1990/97) + compression progress (2006) | two policies BET on binary outcomes of self-generated experiments; surprise = reward; curiosity ∝ compression-progress of a pinned archive |
| 6 | PowerPlay (2011) | at each step, search for the SIMPLEST problem still unsolvable by the current solver; commit BOTH problem and solution; monotone generality counter; nothing forgotten by construction |
| 7-8 | DGM/HGM/RQGM (2025-26), ICL-as-meta-learning | LLM-agent self-modification against fixed benchmarks; recent RSI survey: arXiv:2607.13104 |

The No-GD advocate's structural observation (load-bearing for this distill): **in every original mechanism the KEEPER half — the part that constitutes "learning" — is never gradient descent.** SSA is a trajectory comparison; OOPS is arithmetic budget allocation; the Gödel machine is a certificate check; PowerPlay is an MDL argmin; curiosity is bet-settlement arithmetic. A dumb generator plus a sharp keeper is a complete self-improvement engine, and every sharp keeper below is closed-form and deterministic given its journal. This is exactly the stack's three-track thesis restated from 1987-2011 primary sources.

## 2. Pre-flight (what ran)

1. Product-set scope boundaries: AGENTS.md context for all roots (loaded); riir-train `.docs/02_pipelines/training_data_pipeline.md` read (corpus→LoRA pipeline + Plan 501-505 trajectory-collection chain, closed-negative on dual-LEO — the measured-negative precedent pattern).
2. `.research/` set derived: 9 repos carry one (katgpt-rs, riir-ai, riir-auth, riir-chain, riir-clippy, riir-dao, riir-neuron-db, riir-train, riir-viewbridge).
3. Prior RSI-family distills found and READ before verdicts: **167** (this repo — RSI-survey positioning, healer at "L3/L4 measured, structural L5"), katgpt-rs **320** (RQGM → `best_belief.rs` ε-quantile), **512** (Meta^n RSI, PASS), **526** (fast weights, fusion declined on swarm compute budget), **165** (healer curriculum).
4. Workspace grep + published prior-art search + three-track adversarial panel run in parallel (2 advocates + 2 search/grep agents, one round). Panel briefs described consumers by what ships (file/type names), not conclusions.

## 3. Path-0 inventory — modelless rows (coverage + auditable dispositions)

| # | Mechanism | Closest shipped cousin (read-verified) | Disposition |
|---|---|---|---|
| M1 | Meta Evolution — corpus mined as if user code | `score_bench/generations.rs` strict-harder admission (gen-2 REFUSED when outgrown) + `necessity.rs` two-sided canaries | **Discard (coverage).** Corpus edits already undergo oracle-gated acceptance with freeze-table regeneration discipline (plan 137 T7.5 law); "mine the corpus dir as spans" adds no capability the fixture benches don't already enforce. |
| M2 | Success-Story keeper | `self_evolve.rs` + `elo.rs` (applied rule plays the span; difficulty emerges as opponent rating) + Bench 090 lex-decrease keep rule `(masking, count)` + `g_zero_player.rs` `DeltaGatedAbsorbCompress` (promote heuristic→constraint after ≥min_observations with mean reward-δ ≥ threshold) | **Discard (coverage, signal-diff'd).** SSA's statistic is reward-velocity all-time-high; ours are pair-win Elo / lex-decrease — same function class (evidence-gated keep over a journal), measured, and Bench 090 showed the SIGNAL (masking-aware keep) was the win, not the acceptance-ordering machinery. |
| M3 | Fast-weight writer/executor split | `katgpt-types/src/lora.rs` `LoraPair{reader,writer}` (Plan 025) + riir-ai `LoRAWeightVersion` ArcSwap (Lean-proved torn-read-free) + `delta_mem` outer-product writes `S += v⊗k` + `SenseHotSwap`/`InducedCwmSlot` lock-free slots + NeuronShard `style_weights[64]` self-written by its own sleep cycle | **Discard (coverage).** Note 526 already distilled the fast-weight paper family and declined the swarm fusion on the ~25 ns/NPC compute budget — re-opening needs a new consumer, not a new citation. |
| M4 | OOPS prefix-reuse ordering | retrieval ranking by verified-fix similarity + `min_delta` ordering + Bench 088/089/092 (learned orderings REGRESSED clean 22/34→10/34 until the keep-rule signal fix; no learned arm beats static min-delta on any corpus) | **Discard (measured-negative prior).** The ordering question is answered NEGATIVE on evidence; R5's trained OOPS policy carries the same bar. Re-open trigger stands in Bench 085 §6 (organic multi-attempt rings). |
| M5 | Certificate-gated self-rewrite | `fix_verify.rs` (baseline-check axioms → write → check → span-attributed auto-revert) + riir-chain `proof_gate.sh` (fail-on-sorry + axiom budget + `EXPECTED_THEOREMS` pin + negative test) + GOAT promotion + BLAKE3 `MerkleFrozenEnvelope` | **Discard (coverage).** The Gödel-machine acceptance rule ships as CI. Micro-delta (branch-and-bound stop rule on remaining budget) is the horizon-report's lead-k decodability axis — already instrumented; no new issue warranted. |
| M6 | Adversarial-bet curiosity + compression progress | `cgsp_runtime/` Solver/Conjecturer/Guide (DEFAULT-ON, Plan 299 GOAT) + `temporal_deriv` canonical prediction-error bus (surprise-kernel hook into `evolve_belief_additive` stubbed, waiting) + `epiplexity_scoring` (prequential coding = compression progress verbatim, Plan 130) + `jury_disagreement` (matcher-disagreement module already exists in `src/draft/`) | **Discard (coverage) with one handoff.** The stubbed surprise-kernel hook is pre-existing riir-ai work, not an RSI-extraction gap. |
| M7 | PowerPlay MDL frontier curriculum — **generator half** | `frontier_report.rs` measures the band (`0 < p̂ ≤ 0.5` = capability frontier) but nothing GENERATES at it; `compose_multi_strided` composes only independent single-mutation cases (Bench 087: 0/230 reverts — "a fix can never re-break anything") | **NOT COVERED → Gain → issue 122.** |
| M8 | Episode-conditioned adaptation (ICL) | retrieval + overlays + freeze/thaw + `best_belief.rs` (Plan 336) | **Discard (coverage).** |

## 4. Path-0 inventory — training rows (riir-train pipeline context read)

| # | Recipe item | Pipeline fit | Disposition |
|---|---|---|---|
| R1+R6 | SSA/Gödel acceptance as training-schedule multipliers (keep lineages only when quality-per-GPU-hour accelerates; accept artifacts only when count-floor harness passes) | all | **Fold as guidance** into any training plan; zero standalone GPU cost. |
| R2 | Recipe evolution (data-mix/layer-set/temperature/rank genome, league fitness) | (i) Bonsai distill | **Recorded** — 15-50 GPU-h; medium-high; wait for a distill-quality owner priority window. |
| R3 | Hypernet adapter generation (train a generator OF adapters, freeze outputs) | (ii)/(iii) | **Recorded** — 30-60 GPU-h; runtime-dynamic SRWM explicitly OUT (clashes with frozen ternary serving path + perf league). |
| R4 | VS-ML schedule head | (i) | **Discard** — learned-schedule-vs-tuned-fixed is honestly hard to win at this scale; med-low rank from the advocate itself. |
| R5 | Trained OOPS ordering policy | (iii) | **Discard** — same measured-negative prior as M4; data-gated on organic rings. |
| R7(a) | Adversarial hard-example mining on the distill corpus | (i) | **Recorded** — pairs with R8(a); fixed-token G1 gate. |
| R8(a)+(b) | PowerPlay frontier curriculum for training data | (i)+(iii) | **Folded into issue 122** (the modelless generator produces the curriculum; the training consumer is optional downstream) + riir-train 559 (drafter lane note). |
| **R9(ii)** | **RFT→GRPO self-play on quest-grammar** (sample drafts → keep SCN-solvable → GRPO via `loss_grpo.rs` group advantage + `rlvr_reward`; DPO pairs via `delta_filter.rs`) | (ii) quest LoRA | **Filed: riir-train .issues/559.** Verifier (SCN harness) + losses (GRPO family, grep-confirmed) + tiny corpus (~404 pairs) = highest leverage per GPU-h (est. 15-40 on the 4090). |
| R9(iii) | DPO on TrajStore confirmed/reverted pairs | (iii) healing drafter | **Folded into 559's non-goals note** — data-gated (organic rings scarce); measured-negative ordering prior applies until the keep-rule class evidence accumulates. |
| R10 | ICL multi-task continue-train | (i)/(iii) | **Discard** — quality-per-token dilution risk on the perf-league path; med-low. |

Training-track discipline note: the essay's headline training systems (SRWM self-referential weights, MetaGenRL meta-learned losses, VS-ML learned optimizers) are **out of scope with explicit justification** — no owned pipeline trains meta-learned RL algorithms or runtime-self-modifying weight matrices; the stack's model-based objectives are ternary distillation quality, quest-grammar LoRA, and drafter weights, and runtime SRWM dynamics directly violate the freeze/thaw-over-fine-tuning constraint (constraint #3). This is a scope statement, not a silent drop.

## 5. Published prior art (§4 searches, all run BEFORE the verdict)

Pinned claim (written before searching): *"PowerPlay-style self-generated curriculum for a code-healing rule corpus: automatically generate the simplest code fixtures the current fix-rule corpus cannot yet repair, verify candidates with a deterministic compile/clippy oracle (no LLM in the verification loop), grow the corpus at the capability frontier. Modelless: the generator does not learn."*

- **Class level: KILLED.** POET/MCC (arXiv:1901.01753, 2017-19) = modelless mutation generator + minimal-criterion frontier verifier in RL environments. **Absolute Zero** (arXiv:2505.03335, NeurIPS 2025) = the same frontier-plus-executor shape IN a code domain, but LLM-proposed and weight-training-targeted. **Self-Play SWE-RL** (arXiv:2512.18552, ICML 2026) = same domain (self-injects and repairs bugs of increasing complexity, test-patch verifier), but the generator is the RL-trained LLM itself and the growth target is model weights. Voyager (2305.16291) = LLM-generated curriculum + skill library. SWE-smith = scripted fault injection, uniform, no frontier selection. AlphaEvolve/DGM/HGM = solutions/agent-code evolve against FIXED external benchmarks — no problem generation.
- **The pinned conjunction: UNCOVERED.** No found work combines (i) a non-learning, non-LLM generator + (ii) a compile/clippy oracle with no LLM in the verification loop + (iii) growth of a fix-rule corpus selected by what that corpus itself cannot repair. Task-1 lineage gap stands: no published PowerPlay→code-corpus application 2015-2026.
- The Schmidhuber group's own 2026 SE move is CovQValue test generation (arXiv:2604.05159) — curiosity-driven branch exploration for test SUITES, not repair-corpus growth. The 2026 survey (arXiv:2607.13104) taxonomizes self-improvement by update target × driving signal; self-grown verifiable corpora are at most an implicit instance (abstract-level read; 97-pp full text unscanned — stated limitation).

**Consequence:** novelty survives only at the conjunction → fusion-grade, not class-novel → **not Super-GOAT** (Q1 fails at class level). Q2 (new capability class for THIS system: self-growing corpus with flip dynamics), Q3 (selling point: "the healer authors its own hardest curriculum"), Q4 (force multiplier: feeds frontier gate + bench corpora + mining intake = ≥2 surfaces) all hold — GOAT-tier pending the PoC.

## 6. Verdict

| Axis | Call |
|---|---|
| Tier | **Gain** (modelless track, serving-envelope primary — the loop runs in-process, zero GPU; training lane secondary via riir-train 559) |
| MOAT gate (riir-clippy, consumer-first) | **PASS on fit** — measured healer-quality gains via corpus growth + unblocks three documented gaps (below). Generic-math residue (none here) would go to katgpt-core; none does. |
| Ladder check | "What does this do for the healer?" — it IS the healer surface (priority #2 investment). Game-runtime residue: CGSP already covers curiosity; no new game filing warranted. |
| Files | This note · riir-clippy `.issues/122_powerplay_frontier_miner.md` · riir-train `.issues/559_quest_grammar_rft_grpo_loop.md` |

### The fusion (why this lands at all)

PowerPlay's generator × the healer's measured frontier band × `compose_multi_strided`'s interacting-mutation gap → **an MDL frontier miner that manufactures the corpus the flip-dynamics benches cannot currently exist on** (Bench 087: every corpus today is built from independent single-mutation cases; horizon-report gain axis "honestly NULL — no real surface with revert dynamics exists"; Issue 099 T3 asks for exactly a corpus where repairing A changes B's diagnostic set). One generator unblocks: (a) Issue 089 T4's unwired admission gate (band-mass evidence), (b) the interacting corpus + revert/flip reward signal, (c) a self-play twin for the user-driven mining intake.

## 7. Doc drift fixed in passing

`katgpt-rs/.agents/skills/research/SKILL.md` factory table pointed at `katgpt-rs/crates/katgpt-core/src/sense/` — that directory does not exist; the belief kernels live in `katgpt-rs/crates/katgpt-sense/src/` (`reconstruction.rs`). Fixed in the same commit as this note.
