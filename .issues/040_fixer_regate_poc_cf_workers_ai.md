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

- [x] T1 Partial-credit eval harness (`src/cf_workers_ai.rs` + `src/bin/fixer_regate.rs`, feature `cf_workers_ai`; scoring: EM / fragment-parse / merge-strategy / oracle-verified / strict-keep with BOTH Bench 039 vetoes; CF client behind env token, token never logged)
- [x] T2 Pre-pass prompt shaping (`prompt_prepass`: span + modelless candidate via `HealPipeline::heal` + corpus few-shots via `LatentMatcher::suggest_span` clippy arm)
- [-] T3 Run arms A–C — **DRY arm only** (offline validation: planted controls ALL PASS, decline baseline 9/9, EM 408/408 self-consistent). A/B blocked on `CF_API_TOKEN`/`CF_ACCOUNT_ID` (absent — env + `~/.wrangler` + `~/.cloudflare` checked, presence only). C blocked on BOTH GPUs busy (M3 sibling Plan-349 training 822% CPU; 4090 `bonsai_clippy_l4_sft_train.exe` resident → GPU-exclusivity rule) AND no `.bcl4` checkpoint on disk (t51's is gone; the riir-train driver hard-requires `--lora`). Record: [Bench 049](../.benchmarks/049_fixer_regate_poc.md).
- [-] T4 Verdict vs the bar — **PENDING ARMS** (nothing fabricated; Proposal 005 caveat #2 + Phase 3 updated to point at the harness + pending state). Bonus measured findings recorded in Bench 049: v2edit reference responses 96.6% unmergeable by deterministic means (14/408) + `manual_map_result` is an invented lint label (E0602 on clippy 1.93) on 64 rows — two further confounds for the historical G1 record.
- [x] T5 CF data-usage policy verified in writing — Trust Hub quote captured (the old per-model docs page is 404; Cloudflare: "we do not use our customers' content to train any LLMs" + the third-party-model nuance) — [Bench 049](../.benchmarks/049_fixer_regate_poc.md) §T5.

## Notes

- Not gated on Issue 037 A1 — this is evidence-gathering, not a release.
- CF budget: dollars, not hundreds (116-sample eval × ~$0.002).
- Redaction: eval corpora are in-house fixtures — no consumer data in this POC.

## Status (2026-08-26, cost-tracked)

**Arms A/B auth UNBLOCKED — no owner token needed** (the wrangler OAuth bridge:
`npx wrangler whoami` refresh → lift `oauth_token` from
`~/Library/Preferences/.wrangler/config/default.toml` → export as
`CF_API_TOKEN` + `CF_ACCOUNT_ID`; the session carries `ai:write` +
`account:read`, both needed. The T3 audit had checked `~/.wrangler` — the
wrong macOS path — and missed it).

**Costs now tracked three ways** (Proposal 005 §LLM cost tracking — verified
live): client token-meter · per-response `usage.neurons` (chat-schema
models) · server GraphQL daily aggregate (`aiInferenceAdaptiveGroups` →
`totalNeurons`, billing truth). Committed ledger:
`.benchmarks/llm_cost_ledger.jsonl` (KAT denomination at Phase 1).

**Model switched to `@cf/qwen/qwen3.8-27b` (owner, 2026-08-26).**

**Incident (the 169-request waste):** the first qwen arm-A run extracted zero
generation text from every request — Workers AI's chat-schema models
(qwen3.8) return `result.choices[0].message.content` (+ `message.reasoning`)
while the harness read only the legacy `result.response`. ~7.8k neurons
billed for nothing (thinking tokens ARE billed). Fixed (dual-schema parser +
per-response neurons + reasoning diagnostics); standing rule: one-call smoke
after ANY model change, before any billed run.

**Neuron budget state (2026-08-26 ~01:40Z): 8,309 / 10,000 burned (83%)** —
165 llama smoke (valid, 5/9 vs decline 0/9) + 7,766 qwen wasted + ~366 n of
llama usage at 01:10-01:15Z NOT from this session (20 req — flagged, another
session/process used the account) + 12 n diagnostic. **~1,700 neurons
remain today** (~37 qwen samples / ~110 llama samples). The full arm A
(~280 samples) + arm B needs ~13-27k neurons at qwen rates / ~6-10k at llama
rates — **cannot complete today**; the free cap resets 00:00 UTC.

**Resume options (owner's call):**
1. qwen3.8-27b, spread over 2 days (quality arm first)
2. llama-3.3-70b-fp8-fast for A/B (3× cheaper; both arms fit one free day)
3. proceed past the cap on Workers Paid overage (~$0.011/1k neurons — the
   whole POC < $0.30 even at qwen rates; only if the account is on Paid)

The harness is otherwise READY: dual-schema parser in, cost meters + ledger
in, free-allocation guardrails print per run.

Done:
- Harness + pre-pass + CF client + arm-C transport, all behind `cf_workers_ai` (default build untouched; `cargo clippy --all-targets --features cf_workers_ai -- -D warnings` clean).
- Instrument validated in BOTH directions (`--arm DRY` + `tests/t40_fixer_regate_harness.rs`, 4 fast + 1 ignored full-corpus test): positive planted fixes keep_strict; noop never kept; the destructive drop passes the raw oracle filter and is killed by the contract veto (Bench 039 route 2, live); the silent-logic-change generation is refuted by the behavioural veto (route 3, live); EM 408/408 vs the v2edit references; decline baseline 9/9 re-measured.
- Two corpus defects measured (Bench 049): 394/408 v2edit references are position-ambiguous (unmergeable deterministically — `rlvr_reward::substitute` cannot apply them either); `manual_map_result` (64 rows' first label) is not a real clippy lint.

Blocked (exact unblock):

```sh
# Arms A/B (one command each, once the owner provides the token):
export CF_API_TOKEN=... CF_ACCOUNT_ID=...
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm A --out /tmp/armA.jsonl
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm B --out /tmp/armB.jsonl
# full set: append --source both --limit 0 --chunks 0,1,2,3,4,5,6  (~$0.30-0.80 total)

# Arm C needs BOTH:
#  1. a .bcl4 checkpoint (none on disk; regenerate via riir-train Plan 336 T2.9, or
#     convert the 4090's E:\tmp\p336_t29\v2edit_rank4_lorc.bin.best.bin)
#  2. an idle GPU (M3: sibling Plan-349 training; 4090: bonsai_clippy_l4_sft_train.exe resident)
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm C \
  --l4-bin /Users/katopz/git/riir-train/target/release/examples/bonsai_clippy_l4_eval \
  --l4-gguf /Users/katopz/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
  --l4-lora /path/to/ckpt.bcl4 --limit 8
```

NOTE (arm C shape): the riir-train driver hard-requires `--lora`, so "arm C base model" as
written in the Arms table is not reachable without either a checkpoint or a riir-train driver
change (sibling WIP — deliberately not touched). Running arm C with the T2.9 LoRA checkpoint
is arm-D-shaped; record which one ran.
