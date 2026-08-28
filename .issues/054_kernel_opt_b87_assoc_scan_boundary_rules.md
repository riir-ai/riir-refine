# Issue 054 — kernel_opt Batch 87: associative-scan boundary rules (arXiv:2602.21204 distill)

**Status:** OPEN — queued as Batch 87 (after Batch 85 cflow / Batch 86 Big Pineapple); plan opens at batch scheduling (plan-FIRST discipline)

## Provenance

- Source: arXiv:2602.21204 "Test-Time Training with KV Binding Is Secretly Linear Attention" (NVIDIA, ICML 2026) — Appendix I (the non-reducibility analysis) + §6 ablation trajectory.
- Corpus record: `../katgpt-rs/.research/516_TTT_KVB_Secretly_Linear_Attention.md` (katgpt-rs commit `513088cd`).
- Relation to corpus: the **boundary complement** of Batch 79's `first-order-affine-recurrence-associative-scan` — B79 covers the positive case (affine ⇒ scan, with its rel-error gate regime); this batch covers **what BREAKS the scan**. Paper-distilled precedent = B79 itself (the Trainium paper landed without an in-tree firing population).

## Tasks

- [ ] R0 — plan-first: open the batch plan at the next free `.plans/` number with the task set before any corpus edit.
- [ ] R1 — distill `state-normalization-between-updates-breaks-associative-scan`: normalization applied to the recurrence **state** between updates breaks chunk-parallel associativity (`Norm(A+B) ≠ Norm(A)+Norm(B)`) — nested normalization forces strict sequential dependency even though the mechanism remains linear attention; the LaCT weight-norm case is the live anti-shape; fix-space = normalize q/k instead (φ-side) or accept the sequential form.
- [ ] R2 — distill `dynamic-kernel-params-break-prefix-scan`: updating the kernel-function parameters Θ inside the recurrence makes φ history-dependent via nested-nonlinearity chains (silu′ nesting) — the state update is no longer expressible as a history sum; fix-space = freeze Θ (the paper's Variant 1+2 measured BEST-quality-or-near + the only parallelizable shape).
- [ ] R3 — B79 rider: the boundary nuance — **head-input / qk-side normalization is part of φ** (static per-token transform) and does NOT break the scan; negative-control class grep-verified against riir-gpu `expand_and_l2_normalize_heads_f32` (normalizes Q/K heads pre-recurrence → in-tree SAFE).
- [ ] R4 — standard batch gates: syn/pseudocode parse, category count pins, coverage + retrieval floors, verdict-stable; dedup pass must confirm R1/R2 read as B79's complement, not overlap.
- [ ] R5 — provenance discipline: source grade B (theory sound; the 4.0×/1.19× numbers are the authors' LaCT end-to-end runs, never cite isolated) — mark both rules theory-distilled (suggestion-only / NotBenchable-speed class as authoring decides); cite R516 + B79 in-rule.

## Deferred

- [-] The 4-probe paradox kit (steps↑→worse / gradient-ascent≈same / Q-K spectrum / Q←K≈no-op) — no in-tree consumer (no TTT-KVB layer in any served model); stays research-note-only (`../katgpt-rs/.research/516` §probe kit) until a consumer materializes.
- [-] Reduction-trajectory recipe items (V1 update-last-layer-only, weight-norm/per-token-LR/momentum redundancy) — training-recipe decision table; no TTT-layer pretraining pipeline exists in-workspace; recorded in R516 §trajectory.

## Non-goals

- No in-tree kernel changes: the rules have zero in-tree firing population (GDN normalizes Q/K, not state; the deltanet chunked kernels are the delta-rule class — non-affine for a different reason B79's regime notes already handle).
- No riir-train filings: the paper's recipes govern TTT-KVB fast-weight inner loops, not the workspace's outer-loop LoRA pipelines (loose analogy, transfer unproven — R516 §signal-diff).

## Close condition

Batch 87 lands both rules + the B79 rider through the standard gates, OR the batch dies at authoring-time dedup (if R1/R2 collapse into B79's existing regime notes as riders, land R3 alone and close).
