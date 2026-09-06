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

- [x] **T1 — Anchor-primacy rule (pinned)**: prompt builders (`cf_workers_ai.rs` growing-input path, `prompt_growing_same_rule`), TraceQuery result rendering, and any gist path carry identity fields (lint id, error code, symbols, span range) verbatim in a dedicated anchor position. Pin with a test: compose a prompt from a fixture, assert the raw error code and lint id appear byte-exactly; a refactor that paraphrases/summarizes them fails. **DONE 2026-09-06 (`085a5e8`)** — all four builders pinned (`prompt_raw`/`prompt_prepass`/`prompt_growing`/`prompt_growing_same_rule`): module byte-verbatim, few-shot + channel rule anchors in their dedicated header positions, candidate lines verbatim (indented). riir-clippy has no gist path (that surface is riir-ai `llm_gist_author`, closed-negative Bench 868) and the rustc_errors domain is template-driven (no LLM prompt) — the fixer builder family IS the pinable surface.
- [x] **T2 — Entity-perturbation fixture class (the falsifiable gate)**: for N frozen score-bench fixtures, produce a perturbed twin where the error code / symbol in the context is swapped for a same-kind alternative; assert the healed fix follows the swapped entity (XBridge §4.4 intervention shape). This catches prompt builders that "reason from the gist" instead of the anchor. Non-vacuity: at least one fixture pair must distinguish a naive summarizing builder from the anchor-primacy builder (canary both directions). **DONE 2026-09-06 (`085a5e8`), builder level** — perturbation = same-kind fn rename (`greedy_path`→`greedy_route`) + rule-id swap on the frozen fixture `katgpt-rs-bench_ptrm_width_scaling-002` (inlined verbatim + `issue073_fixture_matches_frozen_set` drift tripwire under `score_bench`); assert: the twin prompt flips EXACTLY the anchor (inverting the swap on the twin prompt reproduces the original byte-for-byte — the builder paraphrases nothing around it), while a naive shape-summary builder (the XBridge failure mode: semantics kept, identity dropped) is rename-INVARIANT on the same twins — the pair discriminates in both directions. Scope note: "the healed fix follows the swapped entity" is the LLM-level property — T3's A/B measures it; T2 pins the necessary builder-level condition (the prompt's anchor follows the swap), which is the half a refactor can silently break. **Mutation-verified**: a temporary paraphrasing mutation of `push_prepass_body` reds all 4 tests (the canary fires in the failing direction); removed after proof.
- [-] **T3 — Bt8: prose + verbatim anchor ledger** — OPEN, GPU A/B: plain prose baseline vs prose + a discrete anchor block on the pinned 12-case corpus (Bench 868 protocol). Held 2026-09-06: the 851 league v6 benchmark waits for box quiet on this M3 and the A/B wants the GPU backend — run when the box clears. The anchor-block prompt builder should be added next to `prompt_growing_same_rule` and pinned by the T1 tests when it lands.
- [-] **T4 — LAM cross-tokenizer mapper** (defer, trigger-gated): deterministic cross-vocabulary token mapping is trivially modelless (`katgpt-tokenizer` + GGUF `BpeTokenizer` are the substrate) but has zero consumers — every drafter/verifier pair in the stack shares a vocabulary. Trigger: any cross-family drafter/verifier pairing (mismatched tokenizers) or a cross-model agent handoff. Recorded in katgpt-rs Research 535 §2 row 1.

## References

- katgpt-rs `.research/535_XBridge_Entity_Grounded_Latent_Bridge.md` (full Path 0 decomposition + signal-diffs)
- riir-ai `.benchmarks/868_issue852_gist_grammar_ab_and_gpu_backend.md`, `.benchmarks/864` (gist negative + first datum); Issue 852 (reopen trigger)
- riir-clippy `.benchmarks/068_issue067_arm_g2_trace_as_state.md` (same-rule channels PASS)
- Landscape: LatentMAS arXiv:2511.20639; arXiv:2606.13594 (continuous-only surface this refutes); arXiv:2608.30963 (Research 121 row 12, C not-mineable — reconciled in Research 535 §3)
