# Research 167 — The RSI survey (B0–L5) consumed as a healer positioning audit

**Status:** DISTILLED — positioning record + [Issue 102](../.issues/102_corpus_lifecycle_library_drift.md) filed (corpus lifecycle / Library Drift adoption). No new primitive claimed; the taxonomy and the lifecycle mechanism are both published prior art we consume.

**Date:** 2026-09-13
**Source:** [arXiv:2609.11873](https://arxiv.org/abs/2609.11873) — "The Last AI Built by Humans: Toward Genuine Recursive Self-Improvement" (Duan, Liu, Tang et al., SJTU + Theseus Labs + Tsinghua + ByteDance + ModelBest + Xiaohongshu + Humanlaya + Agent-Native Research Lab + Shanghai AI Lab; 2026-09-10, 75pp; 491 papers + 72 companies surveyed)
**Classification:** Private (this note); source is public.

## 1. TL;DR

The survey organizes recursive self-improvement by **which improvement-loop decisions the AI controls** (not by algorithm or artifact): B0 in-task refinement → L1 execution of human-defined procedures → L2 strategy selection → L3 learner-conditioned experience acquisition → L4 deployment adaptation → L5 recursive inheritance (the improver/verifier/policy itself is revised and reused). Its value to us is not a mechanism — nearly every mechanism it surveys is another paper we distill on its own — but **an audit instrument**: held against the healer, it produces a file-level positioning table (§3) showing we already ship L1–L4 mechanisms plus *structural* L5, and it exposes exactly one actionable gap the ladder makes visible: **the corpus has admission (Issue 089) but no retirement/cap — the Library Drift failure mode** (unbounded accumulation degrades retrieval and stalls progress before task scores show it; arXiv:2605.19576, surveyed at L4). That gap is filed as Issue 102. Everything else the survey could flag is already covered by shipped discipline (§4).

## 2. The ladder (consumed vocabulary)

| Level | The AI controls... | The paper's canonical example |
|---|---|---|
| B0 | nothing persists — output refinement within one task | Self-Refine, Reflexion |
| L1 | executing a human-defined improvement procedure, artifacts persist | FineWeb-Edu labeling; Meta repair skills |
| L2 | *which* intervention to attempt (strategy search under fixed objective/evaluator) | GEPA, ADAS, AFlow, AutoKernel |
| L3 | *what to learn from next* (learner-conditioned experience acquisition) | SIMA 2 ASKA, SEAgent, R-Zero |
| L4 | *which consequences of deployment experience persist* (memory/skills/harness) | PANDO, Metis, ReasoningBank |
| L5 | the improvement mechanism itself (improver/evaluator/policy revised + inherited) | STOP, RQGM, A-Evolve-Training |

Three survey findings that shape the audit: (a) **structural vs effective L5** — showing a revised mechanism is inherited and invoked (structural) is weaker than showing it produces better successors under matched budgets (effective; almost nobody clears it, incl. AIDE 2); (b) **Library Drift** — the L4 silent failure mode of unbounded skill libraries, with the warning that *over-aggressive retirement performs worse than unguided*; (c) **autonomy ≠ quality** — higher level does not mean better improvement process; the two axes are assessed separately.

## 3. The positioning audit — where the healer sits, with evidence

| Level | Healer mechanism (file-level evidence) | Honest reading |
|---|---|---|
| **B0** | The heal loop itself: drafter→pruner→oracle per span, per run. The `--fix-compile` per-attempt ring (`.heal/fixseq/`, Issue 91 T1.5) is B0 *with in-run attempt memory* — discarded at run end by design. | Matches. Not a defect: B0 is the reference level, not an insult. |
| **L1** | The corpus IS the human-defined procedure library the healer executes at scale: 800+ entries across 6 domains, each batch landing through the human-gated distill discipline (`.distill/001_mining_queue_snapshot.md`, batch records in HISTORY.md). The L0→L2 pipeline executes them deterministically. | Solid L1 — the paper's FineWeb-Edu/Meta-CAPT shape, ours is deterministic (oracle-gated) rather than LLM-judged. |
| **L2** | Retrieval + routing decide *which candidate intervention to attempt* per span: `RuleIndex` fan-out + `RerankMode::Structural` (riir-rag, `latent_retrieval`), `KernelExpertRouter::pick_domain` (modelless argmax), domain auto-fan-out; the opt-in `nonergodic_driver` ordering policy (Issue 093 — mechanism shipped, gain honestly NULL/negative, closed). | L2 mechanism present but **bounded**: the search machinery (drafter, pruner, corpus) is fixed — exactly the paper's L2 boundary ("L2 autonomy concerns the search for a better system configuration; it does not imply authority to redefine what constitutes success"). |
| **L3** | **Learner-conditioned experience acquisition ships**: `--frontier-report` per-rule `r/f/n/p̂/LCB₅` + band-mass (Issue 089 T1/T2), `--misses` frontier-ordered authoring queues per domain (T3), the T4 admission band `0 < p̂ ≤ 0.5` gated on snapshot accumulation, provenance weighting (T5). The system measures its own learnability frontier and aims the next authoring/mining batch at it — the paper's SIMA-2/SEAgent pattern with exact (deterministic) `p̂` instead of stochastic rollouts (Research 165's core extraction). | Genuine L3, ahead of most of the field (survey: 13.0% of papers). The paper's L3 risks ("experience corruption", consistency-as-difficulty-proxy) mostly don't apply — our frontier feed is oracle-verified, not proxy-judged. |
| **L4** | Deployment adaptation ships: `self_evolve` (DEFAULT-ON) records outcomes on every applied fix (`EvolveRecorder` → Warm-tier trajectory store), `LatentFixMemory` + Elo/BetaPosterior selection evolve with use, watch mode (Plan 019 Phase 5) is literally deployment-loop healing. Score-bench `rule_outcomes` per-rule persist/broken/created rows (Issue 089 T8) are the retention-evidence stream. | Genuine L4 (survey: 5.7% of papers). The paper's L4 risk ("persistent update failure — a retained change harms later tasks invisibly") is our `broken` counter + G3 membership-floor discipline. |
| **L5** | **Structural**: the post-mining intake — the healer's own usage artifacts (misses, wrong suggestions, divergence classes) feed the healer's improvement queue (`.distill/001` §Post-mining intake; the `cargo-heal` skill's logging discipline); corpus generations (v1→v8) are authored from measured gaps — e.g. the gen-8 shaped-verdict ladder was born from the observed `manual_map` closure-birth mechanism. The revised mechanism (better rules, better proposers, better benches) demonstrably governs later rounds. **Effective**: partial and honestly uncontrolled — `score_history.jsonl` (8 rows / 7 generations) shows the aggregate trajectory, and gen-2's REFUSAL (healer outgrew the v2 ladder, admission gate fired) is direct evidence the inheritance loop bites — but no matched-budget "successors improve faster" claim exists, the same bar AIDE 2 failed. | Structural L5 yes; effective L5 unclaimed — exactly the paper's structural/effective split. Positioning statement for docs/selling points: *a deterministic, oracle-verified RSI loop measured at L3/L4 with structural L5* — rare company per the survey's own distribution (5.7% + 5.9%). |

## 4. What the survey exposes — gap analysis

1. **Corpus lifecycle (ACTIONABLE → Issue 102).** The paper's Library Drift finding (arXiv:2605.19576): unbounded skill accumulation degrades retrieval quality and stalls progress *before task scores show it*; the fix is outcome-driven lifecycle management (append-only evidence log → retire on contribution fade → cap the active set → meta-skill authoring guide). Our corpus has the evidence log (trajectory store + `rule_outcomes`) and report-only saturation candidates (Issue 089 T2 — `p̂=1 ∧ n≥50`, never deletes), but **no fade retirement, no active-set cap, no paired admission/retirement instrument** — Issue 089 Caution 2 explicitly defers deletion to "a coverage-pass decision" that does not exist. The drift cost is already measurable in our shape: cargo bloat measured ~70% of binary .text as corpus tables (the `dist` promotion's 11.9→4.5 MB), the mining-time dedup grep grows per batch, and the `latent_retrieval` fan-out dilutes as near-duplicate rules accumulate. The paper's counter-warning is encoded as a gate: over-aggressive retirement measured WORSE than unguided.
2. **Evaluator-epoch discipline (RQGM, L5) — already covered.** Frozen evaluators per epoch + independent anchor + discard scores tied to a replaced evaluator = our generation freeze gates (`generation_v8_freeze.rs`: compile/label/INNOCENCE fences, drift gates) + per-generation ladders that are never cross-compared. No action.
3. **L2 evaluator-exploitation hazards — structurally immune.** Random-seed cherry-picking / test-label extraction require an LLM-judge or adaptive benchmark access; our oracles are real `cargo clippy`/rustc/test/hadolint runs over committed fixtures, and the fixture-overfit hazard is covered by the withheld-pair OOD axis (Issue 062) + provenance strata (Issue 089 T6). No action.
4. **ARA — Agent-Native Research Artifacts (validation).** Their 93.7% vs 72.4% QA-accuracy result for executable+trace-preserving artifacts over papers is the survey-shaped argument for our `.plans`/`.benchmarks`/`.research` + freeze-gate + provenance-row discipline (and for the fixseq ring). Validates; no action.
5. **HCI (Headroom-Closed Index) — not adopted.** A benchmark-trajectory normalization for cross-domain model comparison; our league surfaces (score-bench generations, perf-rematch league) each have their own committed methodology and no cross-benchmark pooling need. Noted, declined.

## 5. Fusion — Research 165 × Library Drift × this survey

None of the three owns the full loop alone: **165/Frog** owns *admission* (band `0 < p̂ ≤ 0.5`, LCB protection — landed as Issue 089); **Library Drift** owns *retirement/cap* (contribution-fade, active-set competition, meta-skill); **this survey** supplies the frame that shows both halves are one lifecycle (its L3/L4/L5 loop-closure questions: what enters, what persists, what governs). The fused loop — frontier-admitted entry → measured contribution → fade/saturation demotion out of fan-out (never deleted) → capped active set → meta-rule-guided authoring → the paired A/B instrument for causal retire/admit calls — is Issue 102's scope, deliberately split from 089's admission arm to keep both issues single-owning.

## 6. Per-track verdicts (one per track, not one per paper)

| Track | Verdict | File |
|---|---|---|
| Healer / modelless (this repo) | **Gain** — positioning audit (§3) + the lifecycle gap | this note + Issue 102 |
| Training (riir-train) | **PASS** — the survey is an index of other papers' recipes (A-Evolve-Training, ForgeTrain, SFT/GRPO/distillation rows), each distilled on its own merits; no recipe lands from the survey itself. Reverse-grep checked: no riir-train documented gap is filled by a survey-of-recipes. | none (this row is the record) |
| Game runtime (riir-ai) | **PASS** — the S2 embodied section (curricula, skill libraries, world-model co-evolution) maps onto shipped substrate (`cgsp_runtime`, `engram_runtime`, `sleep_time`, `npc_memory`) without exposing a gap the healer issue doesn't already own; the §3 positioning row is available as a selling-point line at the next doc-sync. | none |

## 7. Prior art (§4 record — searches run 2026-09-13)

- **Taxonomy space is crowded; we consume, not claim:** the survey's own §1.7 names four prior surveys; the L1–L5 grading closely tracks Alibaba's "The Path to Recursive Self-Improving Agents" (preprints 202608.0051 — the survey's ref [30]) and Weco's 4-levels blog. No novelty claimed over any of it.
- **Lifecycle mechanism is published prior art:** Library Drift (arXiv:2605.19576, Amazon Science), SkillOps (arXiv:2605.13716), "Skill Drift Is Contract Violation" (arXiv:2605.10990), a 124-paper "Dynamic Agent Skills" lifecycle survey (OpenReview fNXXsp9iub). Issue 102 adopts with attribution; its novelty is only the application surface (deterministic oracle-verified rule corpus + trajectory-store evidence + report-first law).
- **No prior art found on lint/heal-rule corpus lifecycle** (searched "self-improving linter clippy rule corpus mining lifecycle retirement") — the application is unclaimed, though this is adoption-value, not invention-value.

## 8. Family cross-refs

- katgpt-rs [Research 440](../../katgpt-rs/.research/440_AIDE2_Recursive_Self_Improvement_PASS.md) — AIDE², the canonical code-level-RSI PASS precedent (referenced by 512).
- katgpt-rs [Research 512](../../katgpt-rs/.research/512_Meta_n_Fixed_Omega_Growing_Input_PASS.md) — Meta^n; its PASS-Redirects row is how this family keeps citing itself.
- katgpt-rs [Research 289](../../katgpt-rs/.research/289_RecursiveMAS_Pass_Already_Shipped.md) — RecursiveMAS PASS.
- katgpt-rs [Research 320](../../katgpt-rs/.research/320_Red_Queen_Godel_Machine_Selective_Erasure_Best_Belief.md) — RQGM (the survey's L5 evaluator-evolution entry).
- Local [Research 165](165_Frog_Family_Healer_Curriculum_Distill.md) + [Issue 089](../.issues/089_frontier_band_healer_curriculum.md) — the L3 admission half this note completes.

**PASS-Redirects (synthesis):** Duan et al. [arXiv:2609.11873 "The Last AI Built by Humans: Toward Genuine Recursive Self-Improvement"] — consumed as the B0–L5 positioning audit; the one actionable extraction (corpus lifecycle) is Issue 102.
