# Research 431: ELITE — Evaluation-Free Recursive Self-Improvement (Explore-then-Integrate) — PASS

> **Source:** [arXiv:2611.21056](https://arxiv.org/abs/2611.21056) — "Evaluation-Free Recursive Self-Improvement for Coding Agents" — Ren, Rong, Chen, Nanbo, Piękos, Ostaszewski, Guo, Schmidhuber (Jilin University + KAUST), arXiv:2611.21056v1, 16 Nov 2026 (32 pp). Same group as HGM (ICLR 2026); ELITE's own ablations use SIFT (arXiv:2609.19526) and DGM/HGM as baselines.
> **Date:** 2026-11-21
> **Status:** Done — closed. **Verdict: PASS** (modelless track; every extractable mechanism already ships or is measured-exhausted — §2/§3). No plan, no guide, no open primitive. One in-flight correction this session: the Gain candidate filed first (a fix-pass-composition POC) was killed by the §0 source read of `fix_verify.rs` BEFORE commit — the composition it proposed already ships (§3.2). Deleted pre-commit; `.issues` highwater restored to 254; this note is the record.
> **Related Research:** 208 (SIFT — ELITE's own select-one ablation baseline; BT class measured 67.7% avg vs integration 76.1% — post-publish line added there), 178 (Schmidhuber RSI — `fix_verify` identified as "the Gödel-machine acceptance rule"; ELITE now measures both the rule AND its counterfactual removal, §2 C1), 203 (RRSI keeper discipline), 001 (the survey whose "Framework: agent edits own scaffold = N/A for riir-clippy" row — ELITE evolves the SCAFFOLD, riir-refine evolves the CORPUS; row stands), katgpt-rs 320 (Red Queen Gödel Machine), 512 (Meta^n fixed-Ω PASS — this note follows its PASS-note shape), 440 (AIDE²), 616 (MASS), riir-ai 401 (harness-evolution guide — DGM the Level-1 shape), riir-ai 399 (Agentic Scientist orchestration)

## TL;DR

ELITE removes **downstream candidate task evaluation** from the coding-agent self-improvement loop: instead of evaluate-and-select (rank children by task rollouts, keep the winner — DGM/HGM/SIFT/GEA/MGM), it explores K=7 focused modifications implemented independently from the SAME parent, classifies their mechanisms (complementary / redundant / conflicting / overly complex / incomplete), and REIMPLEMENTS a compatible subset as one coherent descendant — filtered only by deterministic structural/operational gates. SWE-bench Pro (DeepSeek, 3 runs): 60.0 → 85.6% with **zero** candidate task evals (DGM 86.7 @ 810 evals, HGM 88.3 @ 800), 8.8×/3.2× less wall-clock, and better transfer (73.3% Verified / 69.3% Polyglot vs the baselines' flat-to-negative). The paper's value HERE is evidence, not mechanism: **ELITE is an external, SOTA-scale measurement of the loop `fix_verify` already ships** (§3.2), and its ablations rank the selection modes the workspace uses (§2 C2/C3). Verdict PASS — the four novelty gates all read NO for the stack (§5), and the one candidate gap dissolved on the source read.

## 1. Source digest (what the paper claims)

| Component | Mechanism |
|---|---|
| **Evaluate-and-select** (the named incumbent) | downstream task outcomes decide which candidates/lineages carry forward; M×N rollout cost; benchmark-conditioned adaptation risk (Kapoor et al. 2024) |
| **Same-parent exploration** | K hypotheses (capability + concrete limitation + focused mechanism + acceptance conditions), implemented independently against ONE parent → directly comparable implementation-level alternatives |
| **Exploration gate `G_exp`** (deterministic) | nonempty patch · compiles · imports/tool-loading · permitted modification scope · dependency deployment rules. 41/48 passed |
| **Mechanism integration `I`** | LLM integrator reads all children's evidence, shortlists 1–3, reads full patches, classifies on five axes, REIMPLEMENTS the compatible subset from the ORIGINAL parent (never concatenates patches). Primary mechanisms carried: 25/41 eligible |
| **Acceptance gate `G_acc`** (stricter) | adds smoke tests of tool-call protocol + main flow. 5/7 passed; failure RETAINS the parent |
| **Headline numbers** | Pro 85.6% (0 evals) vs DGM 86.7 / HGM 88.3; transfer 73.3/69.3 (DGM 63.3 −0.7pp / HGM 65.3 +1.3 — evaluation-driven evolution did NOT transfer); wall-clock 4.9 h vs 43.15/15.66; cost $8 vs $45/$30 |
| **Key ablations** | w/o deterministic gates **76.1 → 52.0 (COLLAPSE)** · select-one-LLM 71.8 · SIFT-style BT select 67.7 · evaluation-based select Pro 91.7 but transfer 74.6 avg < 76.1 · single exploration K=1 69.9 · direct joint implementation 72.1 vs 76.1 · w/o hypothesis generation 71.0 · K 3/5/7/9 → 70.5/74.8/76.1/75.6 (plateau) |

Post-hoc: the integrated descendant beat 5/7 of its own children (81.7% vs mean 69.5%) without ever measuring them; two intermediate REGRESSIONS were carried (the gate enforces validity, not improvement — the paper is honest about this).

## 2. Three corroboration rows (the durable value of this distillation)

| # | ELITE finding | Standing fleet law it corroborates | Shipped cousin |
|---|---|---|---|
| C1 | **w/o deterministic gates → 52.0 (−24.1 pp collapse)** — the gates are the load-bearing component | `fix_verify` (baseline → apply → `cargo check --all-targets` → attribute errors → revert granularly) — R178 identified it as the Gödel-machine acceptance rule; ELITE measures the COUNTERFACTUAL of removing it, at agent-scaffold scale | `src/fix_verify.rs`; `tests/fix_verify_failure_matrix.rs` |
| C2 | **Integration (76.1) > LLM select-one (71.8) > SIFT-style BT select (67.7)** over the same K=7 pool | The pool decision-mode axis: rank-1 selection (shipped, `RERANK_POOL=37`, top-1 recall 46/48) and hosted seat_rerank (armed 2026-10-30, judge-ordered = SIFT's class — the class ELITE ranks LAST) are both SELECT modes; ELITE prices the third mode as the strongest without-eval alternative | `draft/instantiation_rerank.rs`, `escalation_dispatch.rs`; post-publish line on R208 |
| C3 | **Evaluation-based select wins on-target (91.7 Pro) but loses transfer (74.6 avg < 76.1)** | The measured benchmark-conditioning signature: rethink Bench 089 (Pro-armed rate axis did not transfer; banking77 −10.2pp paraphrase), R135 (Elo selection NEGATIVE), R208's cheap-oracle inversion. C3 does NOT soften the GOAT law — promotion claims stay benchmark-gated; it prices the trade ELITE made | score-bench + GOAT gates unchanged |

A fourth, smaller corroboration: **direct joint implementation (72.1) < separate executable children + integration (76.1)** supports the mining batches' independent-per-candidate transcription against a pinned source over a single jointly-authored proposal.

## 3. Coverage census (§3.6 signal-diffs — why nothing is filed)

### 3.1 Already shipped under other names (the ELITE vocabulary was already in use, unnamed)

- **Corpus-entry integration** — the mining batches' count-neutral rider faces fold onto host rules ("compose, never merge"), in-rule adjudications both directions, dedup kills recorded as gate-works evidence. Signal-diff vs ELITE's five axes: complementary ≈ "second instance arms a seam", redundant ≈ dedup kill, conflicting ≈ "adjudicated distinct both directions", incomplete ≈ "recorded as lane-intel/drop". Same classification SHAPE, corpus granularity; **no actionable delta** — and the mapping is an analogy, not an identity: the paper's "conflicting" is resolved by its integrator (one coherent reimplementation), while "adjudicated distinct" keeps BOTH rules, each scoped to the shapes only it may fire on — nothing is merged or dropped.
- **Same-parent exploration** — batches transcribe candidates independently against ONE pinned source sha. ELITE's joint-vs-separate ablation validates the convention; adds nothing.
- **Compact evolution history `m_t`** ≈ `.refine/trajectories.kv` + blueprint rows. Covered.

### 3.2 The Gain candidate that died (session record — the §0 lesson)

First pass read the paper's compose-compatible-edits + one-deterministic-gate shape as a fix-lane gap: "a pass over a file with N mechanical fixes pays N compile gates; compose the compatible subset into ONE gate". An issue was drafted (fix-pass composition POC) — then the source read falsified its premise: `src/bin/heal.rs` accumulates ALL of a file's edits into ONE `FileEditSet` (`fixed: current.clone()` — the file with every applied edit), and `FixVerifier::verify(&verify_batches)` runs **ONE baseline check + ONE `cargo check --all-targets` per owning package** over the whole composed state (`n_checks: baseline + rounds`; rounds only on failure, where errors are attributed to `(file, line)` and reverts are granular — per-line or whole-file — KEEPING survivors). That is ELITE's loop verbatim at package granularity: compose → one deterministic gate → targeted revert keeping the compatible subset. **There is no gap; the paper is an external SOTA-scale measurement of `fix_verify`'s design.** Issue deleted pre-commit; the number was never consumed (highwater restored). Lesson for the template: the §0 source read of the CONSUMER module belongs BEFORE drafting the issue, not after — the L2-syn-parse gate (`G_exp` analog) and the verify batching were both one grep away.

**Known delta (recorded so a later lane does not re-mine it):** the loop-equivalence above is structural, not total. ELITE's integrator RECONCILES conflicting mechanisms — it rewrites the conflicting parts into one coherent design — while `fix_verify` DROPS the offending edit and keeps the survivors verbatim. Three reasons this is not a gap today: (1) for single-span mechanical fixes, a conflict means one edit is wrong in the context its sibling edits created — dropping IS the correct reconciliation, and re-landing it would need semantic re-authoring, not arithmetic; (2) a reconciler of that kind is the LLM integrator the modelless hot path forbids by mandate (the escalation chain's contract is pick-or-abstain, not generate); (3) no measured demand exists — no heal-lane telemetry shows gate-reverted edits that were individually correct and later re-applied. **Re-open trigger:** if revert telemetry ever shows a measured revert-then-reapply rate on compile-gate drops (survivors correct in isolation, re-landed by a later pass), the reconcile question re-opens as an ESCALATION-seat design question (off the hot path), never as a hot-path LLM stage.

### 3.3 The axes with no room (measured, not assumed)

- **Per-finding pool composition**: candidates for ONE finding are alternative spellings (redundant class), and selection is near-oracle — plan 351 CLOSED-NEGATIVE (adaptive pool depth kill-gate tripped: d* constant at the grid floor, IQR 0.0), pool recall 46/48 at top-1 with `RERANK_POOL=37`. Composition has no room where selection already lands the oracle pick.
- **LLM integrator**: ELITE's integration operator is an LLM; the fixer's hot path is modelless by mandate, and the escalation chain's contract is pick-or-abstain rerank (options echoed exactly) — an integration seat would be a rethink/instinct contract change with no measured demand behind it. Recorded, not filed.
- **Scaffold self-modification**: riir-refine does not rewrite its own implementation (R001's Framework-N/A row); its self-improvement surface is corpus + fix lane. ELITE's subject is the scaffold — out of scope by boundary.

## 4. Published prior art (§4 searches, run BEFORE the verdict)

1. **Headline** ("evaluation-free self-improvement coding agents mechanism integration instead of selection"): ELITE is the class-leader (arXiv + OpenReview); remaining hits are generic self-improving-agent survey/blog content. No competing evaluation-free RSI system surfaced.
2. **Component (integration-as-selection)**: genetic-programming crossover/recombination is the 40-year ancestry — the paper's own §2 concedes it. The new part is evaluation-free gating + mechanism-level (not patch-level) reasoning at agent-scaffold scale.
3. **Component (candidate fusion)**: **PatchFusion** (arXiv:2607.01597, "A single patch is not enough: deterministic fusion of repair candidates") — cited in ELITE §2, NEVER distilled in the workspace. Flagged to the post-mining intake (`.distill/001_mining_queue_snapshot.md`) rather than distilled here (one verdict per paper); it is the closest published cousin if a fix-fusion consumer ever needs prior-art cover.
4. **Landscape**: Ren et al. 2026 survey (arXiv:2607.13104) + R001's survey cover the evaluate-and-select family exhaustively; neither names an evaluation-free member before ELITE.

Internal-first (pre-flight): workspace grep for `2611.21056|Evaluation-Free Recursive Self|Explore-then-Integrate` — ZERO prior distillations (the "elite" hits are Progressive MCGS's Elite sampler, unrelated).

## 5. Verdict

| Gate | Verdict |
|---|---|
| Track | Modelless extraction only (the paper's loop is LLM-driven with a fixed backbone — no optimizer/loss/RL/backprop anywhere; the three-track adversarial panel SKIPPED per §3.5's inference-side exemption, reason recorded) |
| Tier | **PASS** (§1.55: ships + no actionable improvement after the §3.2 falsification — every corroboration row records standing law, none changes config or code) |
| Novelty gate | Q1 NO (crossover ancestry + PatchFusion published) · Q2 NO (no new capability class for the stack — fix_verify already runs the loop) · Q3 NO (no selling point a competitor lacks) · Q4 connects C1–C3 to three pillars via this note only. **Not Super-GOAT, not GOAT.** |
| MOAT gate (riir-refine, consumer-first) | PASS on fit, zero code — the note's value is evidence for the rulewright's own design posture (gates-first, integration-shaped entry) |
| Reverse-grep for documented gaps the paper could fill | `.docs/`/`.benchmarks/`/TODO sweep mapped to §3.3's measured-exhausted axes; the one candidate (compile-gate amortization) dissolved on the `fix_verify` source read. "No, I checked" carries evidence: §3.2 + §3.3 |
| Ladder check | Priority #1 (self-evolve product stack) primary ✓ — consumed as evidence. Game family (priority #2): integration-over-selection maps to scenario/quest-grammar candidate composition; no filed artifact — no measured bottleneck named there. Priority #3+ (serving): the ESC ladder is selection by construction — no consumer. |
| Files | this note · PASS-Redirects lines on 208, 178, 001 · `.highwater` bumps ride the allocation commit (research 431; the issues bump REVERTED with the deleted draft — 254 restored) |

## Proof path (ungoat discipline)

PASS carries no fixture obligation; its checkable claims are the §4 searches (auditable above) and the §3.2 source read (`src/fix_verify.rs` `verify()` + `heal.rs` `verify_batches` — one grep reproduces both). The one drafted-and-killed artifact (issue 255) is recorded in §3.2 with its falsifying evidence so the composition idea does not get re-mined later without hitting this note first.
