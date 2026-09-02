# Issue 62: TPR × healer corpus fusion — structured retrieval axis + corpus-generalization bench (Research 527 F5)

**Status:** Open — Phase 1 immediately actionable (pure eval protocol, zero dependency); Phase 2 gated on katgpt-core `tpr` primitives (katgpt-rs `.issues/707`).

**Research:** [katgpt-rs/.research/527_TPR_Emergent_Symbolic_Structure_Binding_Algebra.md](../katgpt-rs/.research/527_TPR_Emergent_Symbolic_Structure_Binding_Algebra.md) (arXiv:2608.29530, McCoy/Soulos/Linzen/Smolensky 2026)
**Consumer rationale:** the healer corpus is the workspace's richest (role, filler) dataset — every entry IS a binding: role = rule/lint identity, filler = code shape. The retrieval stack's own history is the paper's thesis in miniature: the Issue 017/T3.5 misses were spans sharing the filler (generic fn shape) while differing in the ROLE (operator/punctuation pattern) — the RustOpTokenizer fix injected role-ish signal into tokens; TPR makes the role axis structural. And the one axis nothing measures is generalization: **every retrieval floor (14/14 gate) and every fixture set is in-distribution by construction.** The paper predicts atomic memorization is the default failure mode on unseen (rule, shape) combinations; this issue converts that prediction into OUR measurement before building anything.

## Phase 1 — the unmeasured axis (no dependency, no GPU)

- [ ] **T1** **Withheld-pair OOD generalization bench** for the healer corpus. Define shape-classes over the fixture population (`src/draft/retrieval_eval.rs` 130 spans + `src/draft/eval_fixtures_composite.rs` 28 composite, oracle-labeled via `clippy_oracle.rs`); withhold (rule, shape-class) pairs from the INDEX (not just the eval set); report in-distribution vs withheld-pair top-1/top-k side by side. **Atomic null:** verbatim/pattern-exact lookup on the same split — must FAIL OOD by construction; retrieval+rerank beating it is the systematicity certificate. Preregister the prediction before running (paper: memorization is the default outcome). ~1 day, no GPU. Either result is the record: OOD at ceiling → the Phase 2 TPR axis is honestly demoted for the healer; OOD near the atomic floor → Phase 2 is justified by our own measurement, not the paper's.
- [ ] **T2** **rustc_errors extension:** the same withheld-(error-code, pattern) protocol over the `corpus_gen` 200-case stratified corpus + 12 anchors. The domain already keys repair playbooks by error code — error code is the role; this measures whether playbook selection generalizes to unseen (code, pattern) combos.

## Phase 2 — structured retrieval axis (gated on katgpt-core `tpr`, issue 707 T1–T5)

- [ ] **T3** **ALS fit of the healer corpus** into role-filler form: filler = code-shape embedding (existing AstChunker/ModellessEmbedder path), role = rule identity (and separately domain); ridge-ALS closed form, monotone certificate, frozen BLAKE3 artifact. Substrate tie: `self_evolve`'s EMA domain direction vectors are already runtime-updated rank-1 role embeddings — the TPR role table should thaw from the same freeze/thaw cycle, not fork it.
- [ ] **T4** **TPR-structured retrieval axis** behind a `tpr_retrieval` feature (off by default): query-side structural denoise (score against the TPR projection, not the raw 8-D embedding — paper finding 7: decoders read better from the idealization) + role-conditioned match in the reranker. **Honest claim discipline: in-distribution gains are NOT the claim** (top-1 already 89.6–100 on the floors); the claim axes are (a) the OOD delta over T1's baseline by a preregistered margin, and (b) kernel_opt long-tail crowding (426 rules, the biggest corpus). Floors MUST hold (the 14/14 retrieval gate + score-bench flat unless OOD wins land).
- [ ] **T5** **BoW-null router per domain:** fit the m=1 shared-role ALS per corpus; residual ratio `r_bow/r_full < 1+ε` ⇒ that domain carries no binding structure ⇒ structured machinery skipped there. Records a per-domain structure verdict (prediction to test: clippy_lints strongly structured; kernel_opt categories possibly weaker than rule-level).
- [ ] **T6** **Counterfactual surgery on LatentFixMemory trajectories:** for score-bench persist-miss spans, surgery-swap the rule/domain filler and ask closed-form "would domain X / rule Y have healed this span?" — complements Bench 032's re-run-based switch-cost measurement with a zero-recompute counterfactual; feeds the post-mining intake (`.distill/001_mining_queue_snapshot.md`) with attributed misses.

## GOAT gate

- [ ] **G1** ALS determinism: monotone-certificate assert + double-run bit-identical artifact.
- [ ] **G3** Floors hold with the feature off AND on (14/14 retrieval gate; score-bench run recorded before/after).
- [ ] **G4** Zero-alloc hot path for the projection/unbind inside retrieval (tracking allocator, exact counts).
- [ ] **G8** OOD delta vs the T1 baseline at the preregistered margin, OR an honest no-gain record + demote-loser (flat cosine + Structural rerank keeps the slot at OOD parity). Per-domain BoW verdict recorded.
- [ ] **Promote** Only after gates pass AND T1 shows the OOD gap the axis claims to close; opt-in until then.

## Cross-refs

- katgpt-rs: R527 (§3 Fusion F5 = this issue), `.issues/707` (the `tpr` primitive; its G8 "real downstream consumer" battery is satisfied by T1+T4 here — the healer corpus is a better validation dataset than a synthetic micro-model).
- Existing substrate consumed, not duplicated: riir-rag `RuleIndex` (already consumes the R299 Clifford wedge KNN — the wedge is pairwise interaction; TPR adds role composition), `RerankMode::Structural`, `clippy_oracle.rs` ground truth.

**Consumer note for katgpt-rs 707:** the primitive's first real-world validation consumer is THIS issue's T1/T4 — the healer corpus (580+ labeled (rule, shape) bindings, oracle ground truth, measurable downstream accuracy) replaces a synthetic micro-model for the G8 intervention battery. T1 runs without the primitive and produces the OOD baseline that gates Phase 2.
