# Issue 125 — Specialist option-scorer PoC: healer rule selection as one-pass choice classification (CUA-S1 recipe)

**Status:** OPEN — research-derived PoC task, no code yet. Source: [katgpt-rs Research 573](../katgpt-rs/.research/573_CUA_S1_Open_Jev_Recipe_Specialist_Option_Scorer.md) (trycua CUA-S1 @ `83f142c4`, MIT — the open Jev replication; Research 562's reopen trigger). Modelless default is NOT challenged — this is a challenger arm, promote only on measured GOAT gain.

## Why

The suggest lane is a modelless option scorer: `AstChunker → ModellessEmbedder → RuleIndex (KNN+BM25) → RerankMode::Structural → canonical merge`, composite top-1 ≈ **85.2%** (Bench 096 control arm). CUA-S1 demonstrates the trained-specialist class on an **isomorphic task** — context (UI element bytes) + N options (`fill` pointers + `check`/`click`/`skip`) → one probability per option, single forward pass — and prices the gap: **99.7% for a 706k-param specialist vs 83.6% for the general hosted model**, with the general model's losses concentrated on look-alike confusers and un-trained conventions. The recipe is fully open and **supervised-only** (no RL): cross-entropy, AdamW, 6 epochs over ~150k synthetic rows with forced confuser pairs and signature-disjoint splits.

Serving envelope (TTPO rule): the heal path is not the 20 Hz tick — spans are already chunked/embedded; a ~700k byte-level scorer costs ~ms/span beside the embedder. The game tick keeps `pick_domain` (ns-tier) untouched.

## The PoC (defend-wrong shape; three competitors)

1. **Data** — render choice-classification rows from shipped labeled corpora (no new labeling):
   - `context` = span text bytes (≤224, CUA-S1's truncation), `options` = the canonical-merge candidate rules for that span (N≈7–33) + `skip`, `label` = the oracle-anchored correct rule or decline.
   - Sources: `retrieval_eval` fixtures, fixseq rings (oracle-anchored spans), `--clippy-json` oracle rows.
2. **Confuser augmentation** (the CUA-S1 lesson): force look-alike rule pairs into option sets (the corpus's known confusable families — `redundant_closure` vs `redundant_closure_for_method_calls` class) so the scorer must read the span, not keyword-match.
3. **Competitors**: (a) shipped modelless structural rerank (the baseline), (b) trained specialist — tinyx clone (byte embed 257 → 2-layer width-128 encoder over context, 1-layer over options, `AttentionHead`: `logit_n = q_n·softmax(q_nKᵀ/√r)V/√r`), (c) frozen/no-adaptation control (untrained head) per §3.6.
4. **GOAT gates** (Report-the-Floor bound):
   - **G1**: composite top-1 ≥ structural baseline on the SAME eval fixtures.
   - **G1-OOD**: withheld-pair (rule, span-family) OOD — gap ≤10pp where structural holds (Bench-062 P4 bar); a Bench-065 memorization signature = REFUTED, no widen-and-retry.
   - **G2**: ≤ ~1ms/span batched.
   - **Control**: shuffled-context arm ≈ chance (a structured verdict with a non-degrading shuffle is a harness defect — Bench-063 P5 discipline).
   - **Calibration**: report decision-level ECE/Brier vs the sigmoid floor — CUA-S1 ships uncalibrated by its own admission; do not inherit that (ties katgpt-rs Issue 810).
5. **If GOAT**: freeze/thaw the 2.8 MB checkpoint (BLAKE3 envelope), wire as a rerank arm behind a feature flag, promote on measured gain, demote the loser per house rule. **If REFUTED**: record numbers in Research 573, keep modelless default, reopen only on a corpus-order-of-magnitude change.

## Metric-taxonomy adoption (small, independent of the PoC's outcome)

Adopt CUA-S1 `evals/metrics.py` rows into `retrieval_eval` reporting (provider-neutral, no inference):
- `wrong_rule` vs `wrong_span` decomposition (their `wrong_action`/`wrong_target`) — mis-ranked rule vs mis-targeted span have different fixes.
- `acceptable` alternatives in gold — multiple correct rules for a span stop being scored as errors.
- `coverage` / `abstention_rate` / `selective_accuracy` split — decline behavior becomes a measured axis (score-bench's `persist/broken/created` already covers the `unsafe_action` analog).

## Reference implementation (2026-09-20 addendum — Research 574)

vLLM PR #57250 (unmerged, `ready`) + its `examples/features/diffusion_reads/structured_server.py` interposer = the first **self-hostable** Jev-shaped decision server on open weights (DiffusionGemma 26B-A4B). Relevant here, no scope change: (a) the interposer's question types (yes/no, scale, multiple-choice) are the Noul/Choice/Score contract Research 562 distilled — a live oracle for decision-contract shapes if this PoC is ever validated against a hosted endpoint; (b) its `samples=auto` policy (label-entropy H1 > 0.1 → 4 reads, answer as mean ± bars) implements the metric-taxonomy rows below (`abstention_rate`/`selective_accuracy`) as **agreement-across-reads** — a self-hosted confidence proxy needing no ground truth; (c) measured datapoints + our-silicon validation plan (4090, NVFP4-marlin float16 workaround): katgpt-rs Research 574 + Issue 859.

## Notes

- Softmax-over-options is the categorical-choice exception to the house sigmoid rule (mutually exclusive rule choice per span); all gating/projection consumers stay sigmoid.
- Training is GPU-minutes on the 4090 (706k params × ~200k rows × 6 epochs); CPU-trainable fallback.
- Out of scope: replacing the modelless default, `rustc_errors` domain (narrow option space; KNN holds), any game surface.

## Tasks

- [ ] Render choice rows from the three labeled sources (deterministic, seeded)
- [ ] Confuser-pair augmentation + shuffled-context control arm
- [ ] tinyx scorer + training loop (isolated `CARGO_TARGET_DIR`, /tmp)
- [ ] G1 / G1-OOD / G2 / control / calibration gates, numbers recorded
- [ ] Metric-taxonomy rows in `retrieval_eval` (independent of PoC verdict)
- [ ] Verdict recorded in Research 573; issue closed under the noise-reduction rule with the commit hash
