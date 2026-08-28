# Bench 057 — glm-5.3-flash fixer-quality re-gate: both arms BEAT the Bench 049 llama bar

**Status:** RECORD — re-gate PASS; the `5a5caed` owner-pivot default (`@cf/zai-org/glm-5.3-flash`) is now
quality-gate-measured, not just owner-mandated. Date: 2026-08-28.

## Why

`5a5caed` (2026-08-28) swapped the fixer default from glm-4.7-flash → glm-5.3-flash on the owner's rule
("if it failed, switch to 5.3"), because Issue 043's re-designation gate had measured 4.7 at strict-keep
**3/9** on both arms vs the Bench 049 llama-3.3-70b bar (**5/9** raw / **6/9** pre-pass). The pivot left an
honest gap: 5.3's own fixer-quality arms had never run. This bench closes it — same instrument, same
reachable-9 set, same protocol (Bench 049 §Arms A/B).

## The one-call smoke (Proposal 005 rule — never gated-use without a cheap probe)

Raw REST via the wrangler OAuth bridge (per-run export, the dao Plan 003 pattern — creds are not persisted
on this box), account `7e11…d66eb`:

| max_tokens | reasoning_effort | finish_reason | content | completion_tokens |
|---|---|---|---|---|
| 32 | *(unset)* | length | **"" (null)** | 32 — ALL reasoning_content |
| 32 | low | (overflow class) | null | reasoning starvation |
| 64 | low | stop (marginal, 1-in-N) | "OK" | 14 (45 chars of reasoning) |
| 96 | low | stop | "OK" | 3 |
| **128 (the fixer default)** | low | **stop** | **"OK"** | **3** |

**Finding (pinned as a doc note on `parse_chat_result`):** GLM-5.3 can spend the ENTIRE `max_tokens`
budget on `reasoning_content` even with `reasoning_effort:"low"` — below ~64 the qwen starvation class
reproduces; **≥ 128 is clean**. The harness `max_new = 128` sits exactly on the floor. This is the GLM-5
twin of the qwen `enable_thinking` budget trap; the `REASONING_OVERFLOW` classification already catches
it loudly if a caller sizes below the floor.

## Arms A/B — strict-keep on the reachable-9 set (`--source reachable`, pricing v3)

| arm | model | strict-keep | vs llama (049) | vs glm-4.7 (043) | keep_oracle | parses | merge | vetoed | divergent | regressions | $/fix | latency p50/max |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A (raw) | glm-5.3-flash | **7/9** | 5/9 | 3/9 | 8/9 | 8 | 8 | 1 | 0 | **0** | $0.00003 | ~3.2s / 12.7s |
| B (pre-pass) | glm-5.3-flash | **7/9** | 6/9 | 3/9 | 9/9 | 9 | 9 | 2 | 0 | **0** | $0.00007 | ~3.9s / 9.2s |

**Verdict: PASS on both arms — the first CF model to beat the llama-3.3-70b bar on the fixer-quality
axis**, at ~8-10× lower cost (llama's measured $0.00025/fix, Bench 049) and without the leak-posture
question (llama routes through the same CF account; the standing owner sign-off item is unchanged).
Never-wrong held everywhere: the veto path caught the unsafe generations, zero divergent, zero
regressions — the strict gate is doing its job on a model that wants to be helpful.

Miss-class labels read `OffCorpus` on all 9 — consistent with 043/049 (the modelless path declines all
9 reachable fixtures; that is what makes them "reachable misses"). Instrument unchanged.

Cost track: two ledger rows appended (`.benchmarks/llm_cost_ledger.jsonl`, pricing v3, model
`@cf/zai-org/glm-5.3-flash`): arm A 614+438 tok ≈ $0.00031, arm B 2922+427 tok ≈ $0.00067. The server
GraphQL meter reported 0 neurons for the model at run time — analytics lag (the recon card's known
class); client meter is the cost basis here. Honest extra: ~6 unledgered smoke probes ≈ 1.7 n each
(≈ 10 n ≈ $0.0001 total) — raw curls outside the harness, noted here rather than back-filled.

## What this does NOT claim

- n=9, one account day, one seed of a nondeterministic model — the margin over llama is 2/9 on arm A
  and 1/9 on arm B; treat as "meets-and-exceeds the bar on the standard instrument", not a stable
  effect size.
- The 40-case full-corpus EM arm (0/40 for every model so far) was NOT re-run — EM is capped by the
  96.6%-unmergeable v2edit references (Bench 049 §metric critique) and says nothing model-differentiating.
- Local Bonsai (arms C/D) stay demote-loser (Issue 040 §serving-lane decision).

## Ops notes

- Creds path that made this runnable without provisioned secrets: wrangler OAuth bridge → per-run
  `CF_API_TOKEN`/`CF_ACCOUNT_ID` export (wrapper scripts under `target/`, gitignored, token never
  printed or persisted).
- Dashboard: the nightly runner (Issue 044 T14, `7d984d8`) was audited + hardened this same evening;
  this bench's ledger rows land in the next regen + deploy.

## Files

- `.benchmarks/llm_cost_ledger.jsonl` — +2 rows (arms A/B, pricing v3)
- `src/cf_workers_ai.rs` — budget-floor note on `parse_chat_result` (the max_tokens ≥ 128 rule)
- Run artifacts (gitignored): `target/regate_53_arm{A,B}.jsonl`
