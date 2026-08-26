# Issue 043 — GLM-4.7-flash measured re-designation gate (Proposal 005 model pivot)

Status: **OPEN — smoke + probes run 2026-08-26; knob matrix MISREAD (envelope bug, rerun pending); budgeted-reasoning CONFIRMED WORKING**

Repo: riir-clippy
Proposal: [005 §The daily trainer window](../.proposals/005_kat_heal_service_network.md) (commit `b53f526` designated qwen3.8-27b + encoded the re-designation gate this issue executes)
Precedent: Issue 040 / [Bench 049](../.benchmarks/049_fixer_regate_poc.md) — the bar to beat: arm A 5/9, arm B 6/9 (pre-pass), decline 0/9; never-wrong held.

## Why

Owner candidate: `@cf/zai-org/glm-4.7-flash` — verified from CF docs (2026-08-26):
$0.06/M in + $0.40/M out, **5,500 / 36,400 neurons per M** (~8× cheaper than qwen3.8-27b's
$0.450/$3.20, 40,909/290,909), 131,072 ctx, chat.completion schema, Cloudflare-hosted,
**Reasoning: Yes** (with `reasoning_effort` + `chat_template_kwargs` knobs exposed on the
OpenAI-compat endpoint). If usable, free-tier capacity ≈ 120–330 fix-pairs/day vs qwen's 25–40 —
the daily trainer window's engine.

## What ran (2026-08-26, M3, worktree `../riir-clippy-glm` @ b53f526)

1. **Smoke** (`glm_gate.sh smoke` → harness `--arm A --source reachable --model @cf/zai-org/glm-4.7-flash`):
   **the exact qwen failure class, live** — every generation hit `finish_reason: "length"` with
   `content: null` at the harness default `--max-new 128`. GLM-4.7-flash **reasons by default**;
   reasoning consumed the whole 128-token budget. 9/9 unscoreable, 0/9 strict-keep (does not beat
   decline — as expected for a budget-starved run). ~45 n spent.
   - Harness gotcha: `--limit` caps **v2edit loading only** — the reachable set always runs all 9.
2. **Knob probes** (raw curl): P2 `chat_template_kwargs.enable_thinking:false`, P3/P4
   `reasoning_effort` low/minimal, P5 `thinking:{type:disabled}`, P6 budget 2048 — **all MISREAD**:
   the REST response is wrapped in the CF envelope `{"result": {...}}` and the probe parsed the
   envelope, not `.result`. The probes ran (billed ~30–60 n) but produced no signal. Lesson: unwrap.
3. **Probe2 (correct read)**: minimal POST at `max_completion_tokens: 256` → **`content: "OK"`
   arrived WITH reasoning present** — **budgeted reasoning WORKS**. The smoke failure is budget
   sizing, not hard unusability. This is precisely the gate Proposal 005 encoded ("completion
   budget sized for reasoning + visible output").

## Measured facts (so far)

- **~5 n per 128-token generation** (128 out × 36,400 n/M ≈ 4.7 n + ~60 in × 5,500 n/M) — matches
  list rates; cheaper than any prior candidate per token.
- GLM emits reasoning in BOTH `reasoning` and `reasoning_content` (DeepSeek-style) fields.
- **Client parser gap**: `CfClient` looks for qwen's `message.reasoning` only; on GLM null-content
  it reports "neither chat nor legacy response shape" — verdict correct (nothing usable) but it
  cannot distinguish *reasoning overflow* from *unknown schema*. `CfGen.reasoning_chars` reads 0
  for GLM.

## Remaining tasks

- [ ] T1 Rerun the knob matrix with `.result` unwrapping — does any switch **eliminate** reasoning
      (cheapest per fix), or is budgeted reasoning the mode? (`glm_probe.sh`, fix the python to
      `r.get("result", r)`; ~5–20 n per probe)
- [ ] T2 Size the real per-fix cost: one real fix-prompt at `--max-new 1024` and `2048` —
      reasoning tokens + visible fix tokens + neurons → the trainer-window capacity number.
- [ ] T3 Patch `CfClient` (small, gated): parse `reasoning_content`; classify null-content +
      `finish_reason=length` as **reasoning-overflow decline** (distinct error), per the
      Proposal 005 gate.
- [ ] T4 Arms A/B at sized budget (`--max-new 1024+`, `--price-in 0.06 --price-out 0.40`):
      strict-keep vs the Bench 049 bar (A 5/9, B 6/9, decline 0/9) + never-wrong + $/fix.
- [ ] T5 If T4 clears the bar → amend Proposal 005 pivot-of-record (capacity numbers, gates,
      caveat #18 refresh, references row) in the same measured-re-designation commit.
- [ ] T6 Copy any new ledger rows from the worktree gate dir into `.benchmarks/llm_cost_ledger.jsonl`
      (the smoke's 1 row is being committed with this issue; probes were raw curl — no rows).

## Environment / coordination

- Worktree `/Users/katopz/git/riir-clippy-glm` @ `b53f526` (clean; sibling's T0.2 WIP untouched):
  built harness (`target/release/fixer_regate`, feature `cf_workers_ai`), `glm_gate.sh`
  (smoke/armA/armB — extracts wrangler OAuth token without printing), `glm_probe.sh` (fix before rerun).
- Wrangler OAuth: refreshed 2026-08-26 04:32Z → expires 05:32Z; scopes include `ai:write` +
  `account:read` (+ `containers:write` — the T2.0 scheduled-container surface). Re-run
  `npx wrangler whoami` to refresh before the next gate session.
- Account day burn so far (this model, 2026-08-26): smoke ~45 n + probes ~90 n ≈ **~135 n of 10,000** —
  noise; ledger rows carry the authoritative record.
- Sibling session is mid-T0.2 (redaction gate, Plan 026, uncommitted `kat_redact.rs` + harness wiring).
  These runs used HEAD (pre-redaction) — samples are authored fixtures + pre-vetted v2edit, no user
  code, the same posture as Bench 049. Future gate runs should ride their landed wiring.
