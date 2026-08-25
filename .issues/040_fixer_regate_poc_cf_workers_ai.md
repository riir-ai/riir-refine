# Issue 040 — Fixer re-gate POC: partial-credit metrics + modelless pre-pass + CF Workers AI

Refs: [Proposal 005](../.proposals/005_kat_heal_service_network.md) §caveat #2 + §"The remote flow" · [Plan 336](../../riir-train/.plans/336_*) G1 record (0/60 EM, Benches 465/467) · Bench 045 (the `rustc_errors` 105/105-vs-0 precedent)
Filed: 2026-08-25 (owner's call: "quality is maybe not that bad — only LoRA is bad? correct me" + the CF-LLA pipeline proposal)

## Why this POC exists (the quality record, corrected)

The fixer blocker in Proposal 005 cites Plan 336 G1 = 0/60 twice. Three facts make that number weaker evidence than it looks:

1. **"Only the LoRA is bad" is false** — G1's 0/60 was **tied with the frozen-backbone control**: base Bonsai also scored zero. What is genuinely strong is Bonsai *throughput/serving* (riir-ai Issue 742: 1.24× vs vLLM, TTFT 1.84×, tokenizer parity 17/17) — different axis.
2. **The metric is brutal** — minimal-diff exact match. Nobody has measured partial-credit quality (compiles? oracle-verified heal? fewer lint findings?), which is what the product actually needs.
3. **The eval corpus was rebuilt twice mid-campaign** (v2 → v2edit) under suspicion of corpus-side defects — a confound never fully discharged.

Meanwhile the owner's proposed pipeline (modelless pre-pass → LLM → verify strict-keep → trainer) is ~80% shipped substrate (Issue 030 reachability, `fix_verify` + oracle, the rustc_errors 105/105 never-wrong precedent). The one unmeasured interaction: does **pre-pass + verify-filtering** turn "0/60 EM" models into an acceptable fixer?

## The falsifiable question

**Does verify-filtered acceptance beat `decline` (no fix) on the reachable-miss set, under partial-credit metrics, on any arm?** If yes → the Proposal 005 fixer quality blocker lifts for that arm (leak posture still separate). If no arm beats decline → the blocker stands and Proposal 005 stays trainer-only.

## Arms

| Arm | Model | Pre-pass | Cost |
|---|---|---|---|
| A | CF `@cf/meta/llama-3.3-70b-instruct-fp8-fast` (REST) | none (raw) | ~$0.0017/fix |
| B | CF llama-3.3-70b | **modelless pre-pass**: span + modelless candidate diff + retrieved corpus few-shots as context | same + prompt tokens |
| C | local Ternary-Bonsai-27B base (`bonsai_clippy_l4_eval` batch) | same pre-pass | GPU time (M3/4090) |
| D | local Bonsai + Plan 336 LoRA checkpoint | same pre-pass | GPU time — optional, only if C shows signal |

## Metrics (per arm)

EM (compat with the old gate) · compile-pass rate · **oracle-verified heal rate** (clippy findings strictly fewer — the never-wrong keep rule) · **verify-filtered acceptance rate** (the bar) · tokens & $/fix · latency/fix.

Eval set: the clippy-L4 v2edit test chunks + the L4 three-miss classes (no-candidate / no-parse / off-corpus) as the reachable-miss set.

## Tasks

- [ ] T1 Partial-credit eval harness (extends `bonsai_clippy_l4_eval` scoring: compile-pass / oracle-verified / strict-keep columns; CF client behind an env token)
- [ ] T2 Pre-pass prompt shaping (span + modelless candidate + corpus few-shots via the existing retrieval ranker)
- [ ] T3 Run arms A–C (D optional); record per-arm table in a `.benchmarks/` note
- [ ] T4 Verdict vs the bar (verify-filtered acceptance > decline on reachable-miss set); update Proposal 005 caveat #2 + Phase 3 accordingly
- [ ] T5 CF data-usage policy verification for the leak-posture half (no-training default confirmed in writing)

## Notes

- Not gated on Issue 037 A1 — this is evidence-gathering, not a release.
- CF budget: dollars, not hundreds (116-sample eval × ~$0.002).
- Redaction: eval corpora are in-house fixtures — no consumer data in this POC.
