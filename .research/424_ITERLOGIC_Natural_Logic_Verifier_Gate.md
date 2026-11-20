# Research 424: ITERLOGIC — Natural-Logic Verifier Gates for Candidate Generation

> **Source:** "Iterative Natural Logic Verification for Multi-hop Question Answering" — Jihao Shi, Liang Yang, Hongfei Lin (Dalian University of Technology), Findings of EMNLP 2026, pp. 43407–43419, <https://aclanthology.org/2026.findings-emnlp.2039.pdf>
> **Date:** 2026-11-19
> **Status:** Done — Gain verdict, both tracks; POCs filed at `.issues/250` + `.issues/251`
> **Related Research:** riir-refine 033 (selection blend lineage), riir-refine 421-423 (recent rulewright distills)
> **Related Plans:** none filed (POC-class → issues per the ungoat discipline)
> **Classification:** Public

---

## TL;DR

ITERLOGIC generates candidate intermediate sentences for multi-hop QA, scores each against its source facts with a span-level natural-logic verifier (7-relation lattice + monotonicity projection + 2-state DFA over a DeBERTa span classifier), rejects candidates below a graded threshold, regenerates up to T=5 with varied decoding, and selects by entailment score. Ablations: iteration +0.32 acc; natural-logic verifier vs plain NLI +1.44 overall / +3.86 challenge (WorldTree). The paper's value TO US is not the QA numbers — it is three transplantable mechanisms: (a) a **graded candidate-vs-evidence gate ahead of an expensive verifier**, (b) a **multiplicative (log-sum) composition law** for multi-source evidence (soft-AND), (c) the **budgeted reject-regenerate ladder** with typed exhaustion. Filed as two POC issues targeting the fixer's compile-gate lane and the candidate-selection blend.

**Distilled (modelless, inference-time):** 8 of 10 components are closed-form. The relation algebra, monotonicity projection table, DFA fold, threshold schedule, and argmax are compile-time data + arithmetic — zero training. Only two surfaces are trained (span relation classifier, BCE entailment head), and both have modelless stand-ins in the stack (retrieval-similarity scoring, hashed-feature heads, trajectory Beta evidence).

---

## 1. Paper Core Findings

1. **Verifier-in-the-loop beats generate-then-score**: the natural-logic verifier actively gates generation (reject + regenerate), not just post-hoc scoring.
2. **Natural logic beats plain NLI as the gate**: +1.44 overall / +3.86 challenge on WorldTree (ablation Table 4) — span-level relation structure (7 relations × monotonicity context) carries signal a holistic NLI scorer loses.
3. **Verifiers substitute for generator fine-tuning**: no LLM fine-tuning at all — beats LoRA-finetuned LLaMA-3/Qwen/Mistral/DeepSeek-R1-distill by 12–15 pts on QASC/WorldTree (Table 5), and beats o1-preview by 3.15 (QASC) / DeepSeek-R1 by 8.5 (QASC) / 5.8 avg (WikiHop) (Table 6).
4. **Composition scoring = log p₁ + log p₂** across the two source facts (Eq. 5): "amplifies weak logical support" vs averaging — a product (soft-AND) law, punishing any weakly-supported leg.
5. **Regeneration is sampling, not diagnosis**: no feedback loop into the prompt; diversity from decoding randomness; threshold 0.5; T=5; argmax over attempts (Eq. 6).
6. Final selection: per-candidate BCE entailment scoring (TEAM-style, Eq. 8).

## 2. Distillation — Path 0 component inventory

Merged from the No-GD + model-based advocate briefs (both run this session); every "analog" row carries its signal-diff.

| # | Paper component | Form | Analog ships? (signal-diff) | Extraction without GD | Consumer surface |
|---|---|---|---|---|---|
| 1 | 7-relation lattice (≡, ⊑, ⊒, negation, alternation, cover, independent — MacCartney & Manning 2009) | Closed-form finite algebra | No (workspace greps `natural logic` = zero; `entailment` hits are NLI dataset suites, not a verifier primitive) | YES — static tables + join/meet; Lean-provable invariants | katgpt-rs-class substrate module (no consumer yet — NOT filed per ungoat discipline) |
| 2 | Monotonicity projection δ (7×2 upward/downward LUT, Table 2) | Closed-form LUT | No | YES — `const` table, O(1) per projection | Same substrate |
| 3 | 2-state DFA chain aggregation | Closed-form sequential fold | Partial (FSMs ship in game runtime; verify loops ship in the fixer — but no span-relation fold) | YES — pinned fold order (determinism law), zero-alloc | The chain verifier of (a) |
| 4 | log p₁ + log p₂ soft-AND (Eq. 5) | Derived composition law | **Partial — real delta**: `select_best_candidate` composes `W_EVO·evolution + W_RATE·reliability` ADDITIVELY (`self_evolve.rs:134-136`, `selection_tether.rs` pins the additive cold-start identity). Additive forgives a weak leg; product punishes it | YES — Σ log-sigmoid over leg logits (house: sigmoid, never softmax) | `selection_tether` challenger — Issue 251 CLOSED `8d43c3e0`: TIE → DEMOTED (Bench 164; 9 real-store groups, pick agreement 1.0, Δ 0, product 6.3× slower; feature `composition_product` kept opt-in as the negative artifact) |
| 5 | Threshold-0.5 reject → regenerate ≤T=5 → argmax | Schedule + ladder | **Partial, posture-corrected (verdict round 1)**: `fix_verify` verifies POST-apply with a BINARY compile gate over a per-package BATCH (`src/fix_verify.rs:316` — one baseline + revert rounds; no order within a batch), and `heal` selects ONE candidate per finding before verify (no next-candidate retry). Missing: a GRADED cheap gate DECLINING doomed candidates BEFORE the expensive gate (shadow-score → decline-guard) + budget cap + typed abstain on exhaustion; `steer_retry` is the retry-ladder precedent (hosted-transport, deadline-bounded) | YES — deterministic diversification (retrieval-slice/seeded jitter) replaces temperature; non-degeneracy census is the falsifier (ladder arm only) | Decline-guard POC — `.issues/250` (T0 measures the cost model first; DOA stop if wasted verifies are thin) |
| 6 | Span-level relation classifier (DeBERTa, softmax) | Trained head | No modelless analog proven (exemplar-KNN surrogate unproven; corpus-distance gate's signal differs: query-vs-corpus whole-decision distance vs candidate-vs-evidence span consistency) | Partial — v1 hashed-feature BCE head ≈0 GPU; v2 20–30M span-pair encoder 10–25 GPU-h (4090) | Same as (a), trained rung |
| 7 | Hypothesis-entailment BCE selection (Eq. 8) | Trained head | YES (drafter option scoring + Beta-LCB trajectory evidence cover the shape) | YES (surrogate) | Decision-lane (secondary) |
| 8 | Argmax over attempts, strict tie rule | Trivial | YES | YES | Both |
| 9 | Offline census/labeling (fix_verify as labeler) | Data pipeline | No | YES — perturb accepted fixes → compile gate labels ground truth by construction (**syntactic classes only** — inverted guards/swapped operands usually still compile and are excluded; verdict round 1) | Phase 0 of `.issues/250` |
| 10 | Attempt DAG / Pareto rank | Search structure | YES (L3 Pareto search primitive) | YES | Optional arm |

### Fusion subsection

Closest cousins considered and why none kills the claim:

- **`fix_verify` loop** (riir-refine): verify-reject-revert EXISTS, but binary + post-apply + expensive, and BATCHED per package with ONE selected candidate per finding (`src/fix_verify.rs:316`; coordinator-verified). The graded PRE-apply decline-gate is the delta — posture-corrected per verdict round 1: shadow-score → decline-guard, never "order compiles".
- **`seat_rerank`** (riir-refine/instinct): hosted pick-or-abstain with echo trust anchor — pick-among-given, not generate-verify-regenerate.
- **`CorpusDistanceGate`** (riir-reflex): fused abstention on query-vs-corpus distance for WHOLE decisions — different signal (no candidate, no evidence spans).
- **`selection_tether.rs`** (riir-refine): the additive blend is bit-pinned cold-start; the product law is a composition-law challenger, not a replacement of the evidence feeds.
- **`steer_retry`** (riir-refine): the retry-on-verifier-rejection ladder precedent — deadline-bounded, hosted-transport; the pre-compile gate differs in verifier (local, µs) and posture (ordering-first).
- **`descent_repair`** (riir-grams, quest_grammar Repair seam): corrupt document → single-element moves → repaired-or-typed-abstain; the graded gate could rank its moves — game-side secondary surface, noted not filed.
- **Published prior art (§4 searches)**: rejection sampling + verifier-guided generation are a dense CLASS (RLHF rejection sampling, LLM-VeriOpt 2026, SHARS hallucination-rejection 2026, CoT-verification surveys); **build-outcome prediction for CI** (Rangari 2025, 85.24% acc) is published prior art for the compile-prediction class. The lattice is MacCartney 2007/2009 textbook. **Verdict consequence: the class is not novel — the claim is scoped to the workspace transplant + composition law + posture, which is Gain-tier by construction.**

### Stationary-surface check (step 4a)

The gate itself is a static scorer (no mutated state) — its honest outputs are LEVEL quantities (per-candidate score, ordering, decline counts). The ladder's dynamics claim (regeneration helps) requires VARIED candidates: our drafter is deterministic given (span, corpus), so diversity must come from deterministic diversification, and the **non-degeneracy census** (attempts must produce distinct candidates) is the pre-registered falsifier before any "+iteration" claim.

### Game-context reframe (step 4)

Quest_grammar's Repair seam could consume the same graded-gate + budgeted-regeneration shape for quest-text repairs (rank single-element moves by consistency before accepting). Secondary; not filed — no current pain signal there.

### Consumer-context reframe (step 4, rulewright surfaces a–e)

(a) retrieval index — exemplar corpus for relation scoring; (b) trajectory memory — the Phase-0 labeled corpus (fix_verify as labeler, **syntactic classes only** — compile labels cannot see inverted guards/swapped operands, which usually still compile; semantic classes would need a separate behavior-labeled arm with test outcomes as the labeler); (c) score-bench — the derived G3 bar lands in the run summary; (d) drafter→pruner selection — the gate sits exactly at the pruner→verify seam, **posture-corrected per verdict round 1: the lane verifies ONE selected candidate per finding inside a per-package batch (`fix_verify::verify(&[FileEditSet])`), so the honest shape is shadow-score → decline-guard, not order-compiles**; (e) rustc_errors playbooks — the code-domain relation remap (Equiv/Widen/Narrow/Invert/Independent) is a v2-optional label space requiring behavior-level labels; the shipped heads make no semantic-class claim.

## 3. Verdict

**Pinned claim (§1.5 precondition form; posture-corrected per verdict round 1):** *Budgeted rejection-sampling of generated candidates through a graded span-level symbolic consistency gate, with multiplicative composition across source constraints, as (a) a shadow-scored decline-guard ahead of the fixer's per-package compile batch and (b) a composition-law challenger to the additive candidate blend — distinguished from shipped fix_verify (binary post-apply), CorpusDistanceGate (whole-decision distance), and seat_rerank (pick-among-given) by the graded candidate-vs-evidence threshold + product aggregation + budget/exhaustion semantics.*

**Per-track tiers (never pooled):**

- **Track A (modelless primitive transplant): GAIN** — the algebra + ladder are world-published (MacCartney 2009; rejection-sampling class dense), so Q1 fails at world level (⇒ not Super-GOAT); the workspace-level behaviors are new (Q2: graded pre-apply decline-gate is a behavior the fixer lacks; Q3: "skips compile work on candidates a measured floor rates doomed" is a finishable selling point, bar DERIVED from T0's census; Q4: connects fix_verify + trajectories + ternary + self_evolve). GOAT-eligible; the tier lands only if the `.issues/250` G3 gate passes (and T0's DOA stop may honestly kill it first).
- **Track C (trained verifier): GAIN** — v1 BCE head ≈ 0 GPU-hrs; v2 span-pair encoder 10–25 GPU-h on the 4090, escalated ONLY past the pre-registered stop (v1 ≈ similarity-floor within noise ⇒ modelless wins, stop). DeBERTa-large explicitly rejected (~40–80 GPU-hrs, buys nothing). Hosting: riir-train trains → ternary `.bits` head ships beside the L1 drafter (the `rule_embed_poc` precedent); laya explicitly WRONG base (parity-pinned, intent-classification checkpoints — not diffs).
- **Track B (self-adaptive runtime latent updates): none** — the paper carries no latent-state-update mechanism.

**MOAT gate (riir-refine, consumer-first):** in scope — the POC targets fixer cost/quality (heal-lane economics) with a measured floor comparison; the note is public-safe (distillation of a published paper + our own improvement ideas; no rethink-class content). katgpt-rs substrate module deliberately NOT filed (no consumer — ungoat discipline: no runnable proof path ⇒ issue-at-most, and here not even that until a consumer names itself).

**Verdict record (round 1 — REVISE → corrections applied; landed `a719b348e`):** the Claude reviewer's three bullets (compile-label scope; the missing cost model behind the compile-cut bar — batch posture verified by coordinator grep at `src/fix_verify.rs:316` + `self_evolve.rs` single-candidate selection; G1 byte-identity vs selection-driven kept sets) and one smaller point (Issue 251's input law + named cold-start pins) are all applied above; `claim_lease_check --lint` and `numbering_gate` re-run clean after the edits.

**Weakest point, named by me:** the modelless relation surrogate's quality on FINE relation distinctions is unproven — if it only separates {entails, contradicts, neutral} cleanly, the honest landing is a disclosed 3-relation coarsening (lossy-surface law, per-family retention gate). The paper's +3.86 challenge delta rides DeBERTa span probabilities we do not have; our G3 bar may be reachable by the similarity floor alone, which would make the trained rung unnecessary — that is a valid GOAT outcome, not a failure. And the sharpest structural risk: the lane may not waste enough compiles for ANY gate to pay — T0's census exists to find that out for the cost of one script, and its DOA stop is the cheapest branch of the whole POC.

## 4. Routing — files created this session

- `riir-refine/.issues/250_verifier_gate_pre_compile_poc.md` — the fixer pre-compile gate POC (Phase 0 census → v1 head vs similarity floor → pre-registered v2 escalation), bars inline.
- ~~`riir-refine/.issues/251_soft_and_composition_challenger.md`~~ — CLOSED `8d43c3e0` (issue file removed): implemented as `SelectionMode::ProductComposition` behind `composition_product`. Paired replay over the real store gave a TIE (no weak-leg candidates in a success-saturated store), so the challenger is DEMOTED, the additive blend stands, and the T3 sweep was not triggered. Re-open trigger and the cost note (`p₁·p₂` instead of `ln`) are in Bench 164.
- This note. No plan files (POC-class). No katgpt-rs substrate filing (consumer-less).
