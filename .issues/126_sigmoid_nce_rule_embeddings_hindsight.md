# Issue 126 — Sigmoid-NCE rule embeddings from fix-trajectory hindsight + Lemma-4.2 tier guard

**Status:** OPEN — not started
**Filed:** 2026-09-20 · Research: [riir-ai Research 386](../../riir-ai/.research/386_Contrastive_RL_Goal_Conditioned_Value.md) · arXiv:2206.07568 (CRL) — consumer-surface distillation (fusion priority #2)

## Lane (a) — trained span↔rule embeddings, hindsight-labeled

**The reframe:** rule selection IS goal-conditioned value — `φ(span)ᵀψ(rule) ≈ P(rule fixes span)`, the CRL critic with rule-as-goal. The labels already exist and are **machine-checked**:

- **Positives (hindsight):** `(span_embed, rule_that_actually_fixed_it)` from the `.heal/fixseq/` rings (the `span_embed` column stamped per attempt, Issue 107 T2) + `traj_store` outcomes — the paper's "hindsight relabeling is a standalone RL algorithm" applied to repair: every kept fix is a reached goal.
- **Hard negatives:** rules that fired and **reverted** (compile-gate rejects) — stronger than the paper's sampled marginals.

Today `span_embed.rs` is a hashed bag (untrained); `retrieval_eval.rs` Issue 017 scoped out "no embedder re-training" — **this issue supersedes that scope for this lane only** (the eval fixtures + top-1/recall@k axes stay the authority).

- [ ] Trainer (riir-train side, ≤0.5 GPU-h on 4090): project the hashed span bag + per-rule corpus snippets into d=64 (paper Table 2 pins 64; matches `span_embed` width class); loss = **sigmoid-NCE (NCE-binary)** — `log σ(φᵀψ⁺) + log(1−σ(φᵀψ⁻))`, in-batch negatives. NOT softmax-InfoNCE: house rule (sigmoid never softmax) AND katgpt-rs Bench 048 (sigmoid reaches positive margin at d≈log n; InfoNCE needs Θ(n^⅓)).
- [ ] Ship as **frozen i8 tables** through the freeze/thaw envelope (BLAKE3 sidecar) — the runtime stays modelless (one dot product + structural rerank unchanged).
- [ ] Eval: new posture in `retrieval_eval.rs` (baseline = today's hashed + `RerankMode::Structural`, composite rig + withheld-pair OOD set). **G2 gate: `hits_at_1` must beat the baseline** — the axis is committed and has moved before (+55.6pp on the Issue-121 canonical-key fix), so movement is measurable. G3: clean-span false-positive profile no worse. G1: oracle-anchored (`[oracle]`) spans stay top-1.
- [ ] Prior-art honesty: ContraCode / DPR / CodeContrast / ContrastRepair publish contrastive code embeddings — this is an APPLICATION on machine-checked repair outcomes (ground truth no external corpus has), not a new class. Frame the bench doc accordingly.

## Lane (b) — Lemma-4.2 exclusion filter as the evidence-tier guard

`traj_store`'s Certified/Heuristic arbitration is hand-thresholded today. The paper's Lemma 4.2 prices the alternative: exclude trajectory credit when `|π(τ|s_g)/π(τ|s_j) − 1| > ε` (commanded-goal vs reached-goal likelihood ratio), with closed-form slack `V^{π'} ≥ V^π − 2γε/(1−γ)`.

- [ ] Add the ratio test as the tier-arbitration guard (probabilities the store already logs; ε and γ become the ONLY knobs, with the bound as the documented cost).
- [ ] G1 adversarial arm: a poisoned trajectory (wrong-goal credit) MUST be excluded; a clean one MUST pass. G3: selection monotone on replay.

## Order

Lane (a) first (cheapest, most instrumented); lane (b) is a small follow-on touching `self_evolve`/`traj_store` — read the Le-Critique/Issue-033 coefficient-shape history before touching the blend constants there.
