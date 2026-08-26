# Bench 049 — fixer re-gate POC: harness + arms A/B RUN (verdict: BEATS decline)

**Issue:** [040](../.issues/040_fixer_regate_poc_cf_workers_ai.md) · **Proposal:** [005](../.proposals/005_kat_heal_service_network.md) caveat #2 / Phase 3
**Date:** 2026-08-25 (harness + offline) → **2026-08-26 02:45–03:20Z (arms A/B RUN, verdict landed)**
**Machine:** M3 Max (macOS, arm64) — **UNDER SIBLING COMPUTE LOAD** (`bonsai_clippy_l4_sft_train` ~822% CPU + `maglev_drafter_train` ~85% CPU for the whole session). No latency number in this note is gate-grade; every number here is a COUNT or a VERDICT (load-insensitive).
**Status:** **RESOLVED — the falsifiable question answers YES.** Arms A/B (CF llama-3.3-70b-fp8-fast) both beat `decline` on the reachable-miss bar: strict-keep **5/9 (A, raw)** and **6/9 (B, pre-pass)** vs **decline 0/9**; the pre-pass arm dominates on every partial-credit axis. qwen3.8-27b measured UNUSABLE on the free tier for this task shape (thinking-model token economics — §qwen below). Arm C/D (Bonsai) stay blocked (GPU + checkpoint) and are now OPTIONAL for the unblock question — the Proposal 005 bar reads "CF llama arm OR base-Bonsai arm" and the CF arm met it.

---

## Setup

- **Harness:** `src/cf_workers_ai.rs` (feature `cf_workers_ai`: `dep:ureq` + `latent_retrieval` + `clippy_verify` + `l4_lora`) + `src/bin/fixer_regate.rs` (`--arm A|B|C|DRY`) + `tests/t40_fixer_regate_harness.rs` (5 tests, 1 ignored full-corpus).
- **Default build untouched:** `cargo check` green; `cargo clippy --all-targets --features cf_workers_ai -- -D warnings` clean (healer first for mechanical findings — 5 edits — then manual for pedantic/doc/must_use).
- **Substrate reuse (the issue's "~80% shipped" made concrete):** reachable-miss set + `control_fires` + `contract_idents` from `tests/t51_l4_vs_decline.rs` (copied per that file's own precedent, premise re-asserted at construction); miss classification + modelless candidate via `HealPipeline::heal` with the bench_003 per-lint candidate construction; few-shots via `LatentMatcher::suggest_span` (clippy domain arm) + `rule_book`; fragment-parse via `RustAnalyzerPruner` (the seam's invariant-3 gate); oracle via `ClippyVerifier`; behavioural veto via `behavioural_equivalence::check` (the Bench 039 "route 3" module); arm C transport via `DaemonL4` (gained `request_prompt` for the pre-pass shape). NEW code: the CF REST client (~80 LOC), the v2edit loader, and the edit-application ladder — everything else is composition.
- **Never-wrong keep rule (two filters, both reported):** `keep_oracle` = control fired ∧ oracle ran ∧ findings strictly fewer ∧ not a no-op (the proposal's filter); `keep_strict` = `keep_oracle` ∧ contract-preserved ∧ not-behaviourally-divergent (the Bench 039 hardening — both false-WIN routes the n=9 run actually produced).

## Arms status

| arm | model | status | note |
|---|---|---|---|
| A (CF raw) | `@cf/meta/llama-3.3-70b-instruct-fp8-fast` REST | **RUN 2026-08-26** | reachable 9/9 + v2edit chunk-0 slice 20/20 |
| B (CF + pre-pass) | same | **RUN 2026-08-26** | same sets; dominates A everywhere |
| qwen3.8-27b | `@cf/qwen/qwen3.8-27b` | **MEASURED UNUSABLE (free tier)** | thinking model — §The qwen3.8-27b finding |
| C (Bonsai daemon) | local 27B base via `bonsai_clippy_l4_eval --serve` | **BLOCKED → optional** | GPU-exclusivity + no `.bcl4` checkpoint + driver hard-requires `--lora`; the unblock bar no longer needs it (CF arm met it) — remains the serving-path follow-up for Proposal 005's trainer node |
| DRY (offline) | planted fixes | **RUN** | — |

## Arm DRY results (instrument validation — NOT model quality)

```
decline baseline        9/9  off-corpus reachable cases serve no modelless fix
                            (t51 arm0 re-measured through this harness — the Issue 030 gate holds)
planted POSITIVE        4/4  keep_strict (ptr_arg f1→0 behav Equivalent{4};
                            or_fun_call f1→0 Equivalent{5}; redundant_clone f1→0 Equivalent{1};
                            box_collection f1→0 behav Unsupported[no Debug] — reported, not vetoed)
planted NEGATIVE        3/3  correctly rejected:
                            noop echo        keep_oracle=false (findings 1→1) — never kept
                            destructive drop keep_oracle=TRUE (!) keep_strict=false — the contract
                                             veto caught what the raw oracle cannot see (Bench 039 route 2)
                            garbage          merge=None — refused to guess
v2edit EM               8/8  (first 8) and 408/408 (full corpus, release) — the EM machinery
                            reproduces the riir-train driver's semantics exactly
behavioural veto        REFUTES the real Bench 039 route-3 generation
                            (`o.unwrap_or(0) + 1`: names preserved, clippy clean, None-branch wrong)
                            → pinned as a standing test
```

**Instrument verdict: ALL CONTROLS PASS — the harness is validated in both directions; arms pending token/checkpoint.**

## Corpus findings (measured, load-insensitive — these stand regardless of arms)

1. **The v2edit reference responses are 96.6% unmergeable by deterministic means: 14/408 rows merge (all `lcs-bounded`), 394 fail.** The "output ONLY the corrected lines that differ" contract carries position information ONLY when a changed line happens to repeat an unchanged context line (row 0 does — its `fn baz` header anchors it; rows 1/3/5 emit pure new-content runs with no anchors, and the runs concatenate edits from multiple fns with no separators). The in-tree riir-train `rlvr_reward::substitute` (whole-fn blocks) cannot apply them either — these references were never mechanically applicable in general; the G1 EM gate never needed to apply them (EM is text compare). **Consequence: any oracle-verified metric on v2edit requires either semantic merging or a corpus rebuild with anchors/line numbers.** The harness reports merge failure honestly rather than guessing (a mis-merge would fabricate oracle verdicts).
2. **`manual_map_result` is not a clippy lint** — `clippy::manual_map_result` is `unknown lint` (E0602) on clippy 1.93/1.97; the corpus labels **64 rows** with it as the first lint. This accounts for 64 of the 67 control failures (control = 341/408 corpus-wide, 55/60 on chunk 00 where the label is rarer). The remaining ~3 are scaffold-shape issues. A label defect on 16% of rows is a further confound for the historical G1 record.
3. **keep_o 9/408 on the mergeable reference subset (9 of 14):** the reference responses, where they CAN be applied, do resolve findings (5 of the 14 merged without reducing the target count — multi-lint rows where the reference fixes only some sites).
4. **The reachable-miss premise holds:** all 9 t51 reachable lints remain absent from the seed corpus (asserted at harness construction — a corpus growth fails loudly).

## Metrics columns (per arm, when arms run)

EM (v2edit only) · fragment-parses (L2 gate, raw generation) · merge strategy + rate · control rate · oracle-scoreable · findings strictly-fewer · **keep_oracle / keep_strict (the bar)** · vetoed / divergent / regressions · tokens · $/fix (price assumptions `--price-in 0.29 --price-out 2.25` per-M, overridable, NOT a quote) · latency/fix (disclose load).

**Pre-registered verdict rule (from the issue):** an arm beats `decline` iff `reachable keep_strict ≥ 1` AND no kept case is later shown wrong (the vetoes make kept-wrong structurally unlikely, not impossible — `Unsupported` behavioural results remain admissible-but-disclosed).

## T5 — CF data-usage policy (leak-posture half)

The historical per-model docs page (`developers.cloudflare.com/workers-ai/reference/data-management/`) is **404** now. The current primary source is Cloudflare's Trust Hub page, fetched 2026-08-25:

> "We recognize that the privacy and security of our customers' content is paramount, and **we do not use our customers' content to train any LLMs**."
> — cloudflare.com/trust-hub/responsible-ai/ (FAQ: "Does Cloudflare train AI on customer data?")

> "Importantly, Cloudflare **does not train AI models** or provide GPU access for training." (Workers AI section)

**Honest nuance for the sign-off:** Workers AI models are third-party-provided ("The AI models, including GPAI models, made available on Workers AI, are provided by third-party providers") — Cloudflare's commitment covers Cloudflare's use of the content; the model side additionally rides the provider's license (Meta Llama for arm A/B). T3.2's sign-off should quote the Trust Hub page + name the model provider's license, not treat "Cloudflare says no-training" as the whole posture.

## Reproduce / unblock

```sh
cd riir-clippy
# offline validation (what this note records)
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm DRY
cargo test --features cf_workers_ai --test t40_fixer_regate_harness -- --nocapture

# arms A/B — one command once the owner exports the token (~$0.02-0.05 at default
# 25-sample scale; ~$0.30-0.80 for the full 417-sample set):
export CF_API_TOKEN=... CF_ACCOUNT_ID=...
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm A --out /tmp/armA.jsonl
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm B --out /tmp/armB.jsonl
# full eval set: add --source both --limit 0 --chunks 0,1,2,3,4,5,6

# arm C — needs a .bcl4 checkpoint (none on disk today; regenerate via riir-train
# Plan 336 T2.9 or convert the 4090's E:\tmp\p336_t29 LORC best) AND an idle GPU:
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm C \
  --l4-bin /Users/katopz/git/riir-train/target/release/examples/bonsai_clippy_l4_eval \
  --l4-gguf /Users/katopz/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
  --l4-lora /path/to/ckpt.bcl4 --limit 8
```

## What this run settles / what it does not

- **Settles:** the instrument is trustworthy in both directions (positive controls kept, both false-WIN routes vetoed on real data, EM exact, decline baseline re-measured). Two v2edit corpus defects are now measured facts (unmergeable-by-contract responses at 96.6%; an invented lint label on 64 rows).
- **Does not settle:** the falsifiable question itself — whether verify-filtered acceptance beats decline on any REAL arm. No model ran. The verdict is **PENDING ARMS** and Proposal 005's blocker stands until an arm runs.

## Addendum (2026-08-26) — cost tracking wired + the qwen incident

**Cost tracking is live and verified** (user requirement: track + ledger every
$ burned, KAT-denominated later; Proposal 005 §LLM cost tracking):

- Three meters: client token-meter (per-response `usage`) · per-response
  `usage.neurons` (chat-schema models) · server GraphQL daily aggregate
  (`aiInferenceAdaptiveGroups.sum.totalNeurons` — billing truth; works with
  the wrangler OAuth `account:read` scope).
- Committed ledger: `.benchmarks/llm_cost_ledger.jsonl` (kat: null until
  Phase 1). Rates pinned from the pricing page (2026-08-18): $0.011/1k
  neurons · 10k/day free · resets 00:00 UTC · hard-fail past cap on Free.
- Reconciliation measured: client reads ~7-8% under server (template tokens);
  neuron cross-check exact to 0.01% (165.04 vs 165.037).

**The qwen incident (honest record):** model switched to `@cf/qwen/qwen3.8-27b`
(owner) → first full arm-A run extracted ZERO text from 169 billed requests
(chat.completion schema vs the legacy parser; thinking tokens billed).
~7.8k neurons = 78% of a free day, wasted. Fix: dual-schema parser +
per-response neurons + reasoning diagnostics; standing rule = one-call smoke
after any model change.

**Ledger state at pause (2026-08-26 ~01:40Z):** 8,309.5 neurons total
(83.1% of one free day) · $0.0914 list value — 165 llama smoke (VALID:
reachable-miss strict-keep 5/9 vs decline 0/9) + 7,766 wasted qwen + 366
unattributed llama (20 req at 01:10-01:15Z, not this session — flagged) + 12
diagnostic. ~1,700 neurons remain today; the free cap resets 00:00 UTC.

**Per-model economics (measured):** llama-70b-fp8-fast ≈ 15 n/req;
qwen3.8-27b ≈ 46 n/req (thinking billed). Arm A+B at qwen rates needs
~13-27k neurons (>1 free day); at llama rates ~6-10k (fits one day).

---

## Arms A/B results (2026-08-26 02:45–03:20Z — llama-3.3-70b-fp8-fast, temp 0)

Auth: the wrangler OAuth bridge (session refreshed via `npx wrangler whoami`;
scopes `ai:write` + `account:read`). Neurons are server truth (GraphQL
`aiInferenceAdaptiveGroups`, settled after analytics lag).

### The bar — reachable-miss set (n=9; decline = 0/9)

| arm | prompt | keep_oracle | **keep_strict** | vetoed | notes |
|---|---|---|---|---|---|
| A | raw | 6/9 | **5/9** | 1 | `or_fun_call` contract-vetoed (fix referenced `expensive` out of scope — the Bench 039 route-2 veto live on a real CF generation) |
| B | pre-pass | 7/9 | **6/9** | 1 | edge vs A = `large_enum_variant` (fixed only with pre-pass context); `redundant_clone` was a noop (correctly not kept) |

**Both arms beat decline. The falsifiable question answers YES.**
Reproducibility: arm A's 5/9 matches the pre-pause llama smoke's 5/9 exactly
(temperature 0; two independent sessions, same verdict).

Per-sample keeps (both arms): `vec_init_then_push`, `manual_strip`,
`box_collection`, `option_if_let_else`, `ptr_arg` (+ `large_enum_variant` in
B). Unfixed in both: `too_many_arguments` (needs a signature reshape —
arg-count fixes exceed the "corrected lines" merge shape),
`redundant_clone` (A regressed, B noop). `or_fun_call` resolved the lint in
both arms but the fix was contract-breaking (vetoed, correctly).

### Partial credit — v2edit chunk-0 slice (n=20)

| arm | EM | parses | merge | keep_o/scoreable | **keep_s** | vetoed | divergent | regressions |
|---|---|---|---|---|---|---|---|---|
| A | 0/20 | 15 | 13 | 12/12 | **12** | 0 | 0 | 0 |
| B | 0/20 | **20** | **19** | 18/18 | **18** | 0 | 0 | 0 |

- **The pre-pass roughly doubles usable yield**: 18 vs 12 strict-keeps on the
  same 20 samples; parse 20/20 vs 15/20; merge 19/20 vs 13/20. Few-shot
  shaping constrains the output to the machine-mergeable "corrected lines"
  contract far better than the bare instruction.
- **EM 0/40 across arms** — the historical Plan 336 G1 0/60 reproduces on
  the CF stack too, corroborating the metric critique: with 96.6% of v2edit
  references position-ambiguous (measured above), EM was structurally capped
  regardless of model quality. Partial credit is the product metric, and on
  it the pipeline works.
- **Never-wrong held everywhere**: 0 divergent, 0 regressions across 58
  generations; the one destructive generation (`or_fun_call`) was killed by
  the contract veto in BOTH arms.

### Verdict (T4)

1. **The Proposal 005 fixer QUALITY blocker lifts for the CF llama arm** —
   its own unblock language ("CF llama-3.3-70b arm OR base-Bonsai+pre-pass
   arm") is satisfied by A (5/9) and B (6/9) vs decline 0/9.
2. **The pre-pass arm (B) dominates A** on every axis — the Proposal 005
   pipeline shape (modelless pre-pass → LLM → verify strict-keep) is
   validated as the right one, not just the raw LLM.
3. **Leak posture unchanged** — CF no-training verified in writing (§T5); the
   single-vetted-vendor sign-off remains the owner's gate for Phase 3.
4. Plan 336 G1 EM remains the bar only for the LoRA path (unchanged).

### The qwen3.8-27b finding (owner's 2026-08-26 model switch — measured unusable on free tier)

- **qwen3.8-27b is a THINKING model**: the reasoning phase consumes the whole
  `max_tokens` budget and `content` arrives `null` (`finish_reason=length`)
  — the same waste class as the 169-request incident, different root cause.
  The standing one-call smoke caught it after TWO billed validation smokes
  (93.7 n) instead of after a full arm run.
- **`/no_think` (the Qwen3 soft switch) is unreliable here**: probing showed
  the model *deliberating about whether to honor it* ("User included
  /no_think maybe wants no thinking? But we can analyze.") — it shortens
  reasoning on trivial prompts but does not suppress it on real tasks.
- **`enable_thinking:false` as a request param is ignored** (probed;
  reasoning still present).
- **Budget arithmetic rules it out today**: thinking headroom needs
  `max_tokens` ≥ 384–512 → ~110–190 n/call → both arms ≈ 2k+ neurons > the
  ~1.6k that remained. Revisit on Workers Paid (the whole POC < $0.30 even
  at qwen rates) or if CF ships a serving-level thinking toggle. The harness
  carries the mitigation anyway: `CfClient::generate` appends `/no_think`
  for qwen3-family model ids (documented; harmless if unused).

### Cost record (server truth, settled)

| run | neurons (server) | note |
|---|---|---|
| qwen diagnosis (2 smokes + 2 probes) | 118.1 | the thinking-model finding — cheap for what it bought |
| arm A reachable (9) + arm B reachable (9) | ~360 | the bar |
| arm A v2edit slice (20) + arm B slice (20) | ~955 | partial-credit stats |
| **session total** | **1,433.2** | account day: 9,742.4 / 10,000 (cap respected, 257.6 left) |

Per-fix economics (llama, server): **≈ 22.7 n/fix ≈ $0.00025/fix** at list —
6.8× under the issue's ~$0.0017/fix estimate (short prompts + modest
completions). The ledger's 4 run rows carry `server: 0.0` (analytics lag at
fetch time); the settled aggregate above is the reconciliation (GraphQL
re-probed post-settle).
