# Issue 043 — GLM-4.7-flash measured re-designation gate (Proposal 005 model pivot)

Status: **RESOLVED-NEGATIVE 2026-08-26 — gate run complete; GLM-4.7-flash FAILS the Bench 049 bar in BOTH modes; qwen3.8-27b designation stands (no pivot)**

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

## T1 — knob matrix (rerun with the `.result` unwrap, 2026-08-26 afternoon session)

| probe | finish | content | reasoning | neurons | verdict |
|---|---|---|---|---|---|
| P1 budget 512 default | stop | "OK" | 449 chars | 4.52 | budgeted reasoning lands |
| **P2 `enable_thinking:false`** | **stop** | **"OK"** | **0 chars** | **0.155** | **ELIMINATES reasoning (~29× cheaper)** |
| P3 `reasoning_effort:low` | stop | "OK" | 449 chars | 4.52 | IGNORED (identical to default) |
| P4 `reasoning_effort:minimal` | — | — | — | — | error envelope (invalid enum) |
| P5 `thinking:{type:disabled}` | stop | "OK" | 520 chars | 5.36 | IGNORED |
| P6 budget 2048 default | stop | "OK" | 449 chars | 4.52 | reasoning length is prompt-fixed, not budget-scaled |

**`chat_template_kwargs.enable_thinking:false` is the ONE switch CF's GLM stack honors** — the
exact inversion of qwen (kwarg ignored, `/no_think` suffix works there). Wires into `CfClient`
as the default for glm models (`RIIR_CF_GLM_THINKING=1` opts back into budgeted reasoning).

## T2 — real per-fix sizing (measured from the arm runs)

| mode | arm | avg tokens/fix | GLM-rate neurons/fix | $/fix (list) |
|---|---|---|---|---|
| thinking-off @1024 | A raw | 61 in + 31 out | **1.46 n** | $0.000016 |
| thinking-off @1024 | B prepass | 318 in + 31 out | **2.86 n** | $0.000031 |
| thinking-on @2048 | A raw | ~40 in + 550–1083 out | **16.6 n** | $0.000183 |

Free-tier capacity (10k n/day, GLM rates): raw ~6,800 fixes/day, prepass ~3,500/day,
reasoning-on ~600/day — capacity was never the constraint; quality is (below).

**Display-bug note:** the harness's `neuron cross-check` line converts tokens at hardcoded
**qwen3.8** rates (40,909/290,909 per M), overstating GLM burn ~8×. The ledger `usd_est` is
correct (uses the passed `--price-in/--price-out`). Fix is a one-liner if GLM ever returns.

## T3 — CfClient patch (commit `71c129c`)

- `build_request_body` extracted (pure): glm models get `chat_template_kwargs.enable_thinking:false`
  by default; `RIIR_CF_GLM_THINKING=1` opts back in. qwen3 `/no_think` unchanged.
- `parse_chat_result` extracted (pure): reads `reasoning_content` (GLM's DeepSeek-style twin,
  prefers the longer field); null-content + `finish_reason=length` → distinct
  `REASONING_OVERFLOW` error prefix instead of the misleading unknown-schema error.
- 9 unit tests; module 24/24, clippy 0. **Live-validated in the think arm**: 3 generations hit
  the overflow class at 2048 and were recorded unscoreable with the correct diagnosis.

## T4 — the arms vs the Bench 049 bar

Runs: worktree harness @ `71c129c` (rides the sibling's landed T0.2 redaction wiring — 9 allowed,
0 blocked every run), reachable set n=9, temperature 0, thinking-off @1024 + one reasoning-on
@2048 arm.

| arm | model/mode | keep_oracle | **keep_strict** | bar | never-wrong (keeps) | $/fix |
|---|---|---|---|---|---|---|
| bar A | llama-3.3-70b (Bench 049) | 6/9 | **5/9** | — | held | $0.00025 |
| bar B | llama-3.3-70b (Bench 049) | 7/9 | **6/9** | — | held | — |
| A | GLM off | 5/9 | **3/9** | 5/9 ❌ | **held** | $0.000016 |
| B | GLM off | 4/9 | **3/9** | 6/9 ❌ | **held** | $0.000031 |
| A-think | GLM reasoning @2048 | 4/9 | **3/9** | 5/9 ❌ | **held** | $0.000183 |

Per-sample: GLM-A keeps `redundant_clone` (llama FAILS it), `option_if_let_else`, `ptr_arg`;
GLM-B keeps `vec_init_then_push`, `box_collection`, `ptr_arg`; GLM-think uniquely keeps
`large_enum_variant` (llama's arm-B-only fix) but starves `box_collection`/`option_if_let_else`/
`ptr_arg` at 2048 (reasoning-overflow). Union across GLM modes = 6 distinct — equals llama's
union — but a union is an ensemble argument, not a single-model designation. Complementary
failure modes are noted for any future ensemble/verdict-quorum design; they do not rescue this gate.

**Never-wrong held on every kept fix in all three arms** — all Divergent/contract-break
generations (A: `or_fun_call` contract + `manual_strip` divergent; B: `option_if_let_else`
divergent; think: `manual_strip` divergent) were correctly vetoed/not-kept. The filters work.

## Verdict

**FAIL — GLM-4.7-flash is NOT re-designated.** Both modes beat decline (3/9 vs 0/9, so the
Issue 040 question stays answered YES in general) but both sit **below the llama bar on the
pre-registered strict-keep metric** (3/9 vs 5/9 raw / 6/9 prepass), and reasoning-on costs
~11× more per fix while starving 3/9 generations at 2048. The ~8× per-token cost advantage
cannot buy back a 40-50% quality gap at the fixed free-tier volume where quality-per-fix is
the product. Proposal 005's designated model stands unchanged (no pivot-of-record amendment —
the T5 condition "if T4 clears the bar" is false; the proposal carries no GLM references to
make stale).

## Measured facts (so far)

- **~5 n per 128-token generation** (128 out × 36,400 n/M ≈ 4.7 n + ~60 in × 5,500 n/M) — matches
  list rates; cheaper than any prior candidate per token.
- GLM emits reasoning in BOTH `reasoning` and `reasoning_content` (DeepSeek-style) fields.
- **Client parser gap**: `CfClient` looks for qwen's `message.reasoning` only; on GLM null-content
  it reports "neither chat nor legacy response shape" — verdict correct (nothing usable) but it
  cannot distinguish *reasoning overflow* from *unknown schema*. `CfGen.reasoning_chars` reads 0
  for GLM.

## Remaining tasks

- [x] T1 Rerun the knob matrix with `.result` unwrapping — **done** (table above; `enable_thinking:false`
      eliminates reasoning; `reasoning_effort` + `thinking` ignored).
- [x] T2 Size the real per-fix cost — **done** (table above; 1.46 n/fix raw, 2.86 prepass, 16.6
      reasoning-on; capacity 3.5k-6.8k fixes/day — never the constraint).
- [x] T3 Patch `CfClient` — **done** (commit `71c129c`; live-validated in the think arm).
- [x] T4 Arms A/B at sized budget — **done** (3 arms run; FAIL the bar in both modes;
      never-wrong held everywhere).
- [x] T5 Proposal 005 amendment — **discharged, no amendment** (the condition "if T4 clears the
      bar" is false; proposal carries no GLM references to make stale; designation unchanged).
- [x] T6 Ledger-row hygiene — **done** (3 new rows appended: armA/armB/armA-think;
      `.benchmarks/llm_cost_ledger.jsonl` now 14 rows; GLM session total $0.00207 list ≈ 189 n
      at GLM rates — see the display-bug note for why the in-run cross-check line reads higher).

## Environment / coordination

- Worktree `/Users/katopz/git/riir-clippy-glm` @ `71c129c` (rides the sibling's landed T0.2
  redaction gate): built harness (`target/release/fixer_regate`, feature `cf_workers_ai`),
  `glm_gate.sh` (smoke/armA/armB @1024 thinking-off) + `glm_probe.sh` (envelope-fixed knob
  matrix) + `glm_probe2.sh`. Result JSONLs in `target/gate/` (gitignored — the ledger + this
  issue are the durable record).
- Wrangler OAuth refreshed again for the arms (2026-08-26 ~05:1xZ); scopes verified working
  for inference (the runs above are the proof).
- Account day burn 2026-08-26 (this model): smoke ~45 n + T1 probes ~19 n + armA 13 n + armB 26 n
  + armA-think 150 n ≈ **~253 n at GLM rates** ($0.00207 list) of 10,000 — noise. (The in-run
  "neuron cross-check" lines overstate ~8× — qwen-rate display bug, noted under T2.)
- Sibling session: T0.2 LANDED (`44ec9c1`) — these arms rode its redaction wiring; sibling
  mid-T0.3 (sync payload) in the shared checkout at close (their WIP untouched by this session).
