# Issue 073: Anchor-primacy + entity-perturbation eval — identity fields must cross compression boundaries as discrete anchors

**Status:** Open — T1–T3 actionable (modelless), T4 trigger-gated
**Source:** katgpt-rs Research 535 — [arXiv:2608.11676 "XBridge: Entity-Grounded Latent Bridge for Heterogeneous LLM Communication"](https://arxiv.org/abs/2608.11676)
**Filed:** 2026-09-06

## Problem

XBridge names a failure mode we have measured twice without a unifying explanation: **rare-token compression collapse** — continuous compression preserves contextual semantics but loses discrete identity (names, ids, codes, numbers) first. Our own evidence:

- **riir-ai Bench 868 / Issue 852**: the Bonsai dense gist measured 0.93×/0.95× vs plain prose (2.0× bar) and carried FEWER source inferences — closed-negative, reopen trigger "a materially different compressor". The dense gist is a continuous bottleneck; the identity loss is the mechanism.
- **riir-clippy Bench 068**: same-rule anchor channels in fixer prompts PASSed the pre-registered gate (6/10 vs 5/10 strict-keep) — anchor channels work, but nothing pins WHY or guards the general rule.

XBridge's role-separation finding: **the discrete anchor channel determines *which* entities appear in output; the continuous channel determines *how* the receiver reasons about them.** Applied to the healer: identity-critical fields (lint id, rustc error code, symbol names, span ranges) must cross prompts / TraceQuery / gists as **verbatim discrete anchors**, never only inside a continuous summary or paraphrase.

Grep-confirmed gap: no entity/identity perturbation test exists anywhere in the workspace (all `perturb` hits are index-chaos / FD-sweep / routing-independence senses).

## Tasks

- [ ] **T1 — Anchor-primacy rule (pinned)**: prompt builders (`cf_workers_ai.rs` growing-input path, `prompt_growing_same_rule`), TraceQuery result rendering, and any gist path carry identity fields (lint id, error code, symbols, span range) verbatim in a dedicated anchor position. Pin with a test: compose a prompt from a fixture, assert the raw error code and lint id appear byte-exactly; a refactor that paraphrases/summarizes them fails.
- [ ] **T2 — Entity-perturbation fixture class (the falsifiable gate)**: for N frozen score-bench fixtures, produce a perturbed twin where the error code / symbol in the context is swapped for a same-kind alternative; assert the healed fix follows the swapped entity (XBridge §4.4 intervention shape). This catches prompt builders that "reason from the gist" instead of the anchor. Non-vacuity: at least one fixture pair must distinguish a naive summarizing builder from the anchor-primacy builder (canary both directions).
- [ ] **T3 — Bt8: prose + verbatim anchor ledger** (fires Issue 852's documented reopen trigger — "a materially different compressor"): A/B on the pinned 12-case corpus (Bench 868 protocol, GPU backend for affordability): plain prose baseline vs prose + a discrete anchor block (rule ids, error codes, symbol names as structured fields, no polarity marks). Prediction from XBridge: the anchor ledger restores identity transport prose alone sometimes loses, without the dense-gist continuous bottleneck. GOAT bar: G2 ≥ 2.0× (the 868 bar) or a documented honest loss closing the reopen permanently.
- [-] **T4 — LAM cross-tokenizer mapper** (defer, trigger-gated): deterministic cross-vocabulary token mapping is trivially modelless (`katgpt-tokenizer` + GGUF `BpeTokenizer` are the substrate) but has zero consumers — every drafter/verifier pair in the stack shares a vocabulary. Trigger: any cross-family drafter/verifier pairing (mismatched tokenizers) or a cross-model agent handoff. Recorded in katgpt-rs Research 535 §2 row 1.

## References

- katgpt-rs `.research/535_XBridge_Entity_Grounded_Latent_Bridge.md` (full Path 0 decomposition + signal-diffs)
- riir-ai `.benchmarks/868_issue852_gist_grammar_ab_and_gpu_backend.md`, `.benchmarks/864` (gist negative + first datum); Issue 852 (reopen trigger)
- riir-clippy `.benchmarks/068_issue067_arm_g2_trace_as_state.md` (same-rule channels PASS)
- Landscape: LatentMAS arXiv:2511.20639; arXiv:2606.13594 (continuous-only surface this refutes); arXiv:2608.30963 (Research 121 row 12, C not-mineable — reconciled in Research 535 §3)
