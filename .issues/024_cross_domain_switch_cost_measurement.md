# Issue 024 — Cross-domain switch-cost measurement (the healer's skill-switching gap)

**Source:** [katgpt-rs Research 484](../../katgpt-rs/.research/484_Skill_Entropy_Directed_Switch_Cost.md) + its 2026-08-16 addendum — He et al. [arXiv:2608.05139 "Toward Skill-Native LLMs: Skill Entropy for Benchmarking and Training Long-Horizon Reasoning"]
**Filed:** 2026-08-16
**Kind:** measurement/POC (no production code changes unless a gap is measured)

## Problem

The multi-domain framework (Plans 001 + 005) is a two-level mixture: inner `pick_domain` latent-MoE (gated — exact kernel signatures) + outer `DomainRouter` (deliberately dense fan-out — Issue 020 killed gating at the ~70% lexical ceiling). What is **unmeasured** is the paper's headline failure class in our own terms: **does an anti-pattern heal worse when a different domain's anti-pattern is co-present (or its fix landed first) than in isolation?**

The paper measured −4..−13% cross-skill vs solo accuracy on LLMs, with failure concentrating at hard switches. Our crowding evidence (Issue 017: `identity_op` topped 5 of 14 misses — tiny-token patterns crowd the 8-D space) suggests a retrieval-level analog exists. A fix-level analog may also exist (span overlap / edit interference between domains' proposers — currently caught only reactively by `fix_verify`'s auto-revert).

This is the falsifiable gap that decides whether the dense fan-out is switching-cost-free (close, no action) or whether fix-ordering by measured switch cost becomes the second `SwitchCostTable` consumer (katgpt-rs Issue 663) — the proactive complement to `fix_verify`'s reactive revert.

## Tasks

- [ ] **T1 — Cross-domain fixture batch.** Author ~20–30 spans each co-presenting a clippy lint + a rust_perf anti-pattern (both drawn from the existing corpora so gold rule-ids are known). Verification at authoring mirrors the standing harness: clippy lints fire under real `cargo clippy` (the T3.5.5 uncontaminated oracle); perf patterns fire their own proposer and zero cross-domain proposers (the `clippy_fixture_labels_verify_via_proposers` pattern extended). New file `src/draft/eval_fixtures_cross_domain.rs`; balance ordered pairs (clippy-first ≈ perf-first).
- [ ] **T2 — Retrieval-axis measurement.** Extend `retrieval_eval.rs`: for each co-present anti-pattern, hit@k (top-1/3/5) vs the same rule's solo hit@k from the existing 130-span baseline. Report a **directed** delta table per ordered domain pair (clippy→perf vs perf→clippy) — directionality is load-bearing (Research 484 §6.3; collapsing to symmetric is the easiest mistake). Re-pin the Issue 017 regression floors per its contract if the new batch shifts any baseline (Issue 022 precedent: truthfulness/re-pinning wins over floor stability).
- [ ] **T3 — Fix-axis measurement** (only if T2 shows no gap, or to complete the picture): run the `--fix --verify` path (Bench 031 harness) over the cross-domain fixtures; measure auto-revert rate + final kept-fix rate for cross-domain fix sequences vs same-domain sequences at matched edit counts.
- [ ] **T4 — Verdict + record.** `.benchmarks/0NN_cross_domain_switch_cost.md` with both axes' tables. Signal criterion: any ordered pair with solo−co-present gap ≥ 5pp (beyond tie-order noise) on either axis. Gap → file the fix-ordering consumer issue (consume katgpt-core `switch_cost` once Issue 663 lands; order fixes by measured switch cost ahead of the fixpoint loop). No gap → record dense fan-out vindicated as switching-cost-free, close this issue.
- [ ] **T5 — Research 484 back-reference.** Append the measured table (or the null result) to Research 484's riir-clippy consumer row so the P2 gating resolves either way.

## Non-goals

- Implementing `SwitchCostTable` here — that is katgpt-rs Issue 663 (substrate-first: consume, don't duplicate).
- Implementing fix-ordering before the measurement justifies it.
- kernel_opt inclusion — self-variant probes (Issue 017) have no cross-domain span semantics; clippy×perf is the real pair today.

## Honest caveats

- The 5pp criterion is a judgment call, not derived — the paper's −4..−13% is on LLM skill-switching, not retrieval; our crowding baseline (identity_op) suggests single-digit pp is the plausible effect size. State the raw numbers either way.
- Fixture authoring is the load-bearing risk (Issue 022 lesson: 5/40 corpus entries didn't fire under the real toolchain at authoring time). The T1 verification gates exist to prevent a vacuous measurement.
