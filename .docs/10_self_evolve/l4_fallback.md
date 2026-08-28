# L4 generative fallback — the seam, the local model, and the lane decision

> **Purpose:** the L4 fallback seam (`l4_lora` feature) — the trait, the
> daemon transport, the local model behind it, the measured verdicts, and
> the delegated perf/sec serving-lane decision (2026-08-28). **The seam is
> real; the fallback it would carry is measured-UNARMED.** Declining is a
> correct answer.

## 1. The seam (shipped, inert by default)

The `l4_lora` feature ships **composition surface only** — trait + pipeline
wiring, no model, no GPU, no heavy deps. It is inert unless a fallback is
installed via `HealPipeline::with_l4_fallback`:

| Piece | Where | What |
|---|---|---|
| `pub trait L4Fallback` | `src/l4_fallback.rs:56` | `generate_fix(rule_id, buggy_code) -> Option<String>` + `name()`; `None` = decline (always safe) |
| `NullL4` | `src/l4_fallback.rs:88` | always-declines impl — "installed but disabled" marker + the composition-bench control arm |
| `HealPipeline::with_l4_fallback` / `has_l4_fallback` | `src/self_evolve.rs:445` / `:453` | install / query |
| `DaemonL4` | `src/l4_daemon.rs:54`, `impl` at `:198` | transport client for the riir-train eval daemon (`--serve`); std-only, owns the child process |
| Contract gate | `tests/g4_l4_composition.rs` (13 tests) | pins the reachability contract below |

Absent a fallback the pipeline is byte-identical to L0–L3 — the whole
stage costs one `Option::is_some` check on the miss path.

`clippy_lora` is a **dead placeholder** (declared in `Cargo.toml`, zero
code references) kept only so a stray `--features clippy_lora` doesn't
break. All L4 work lives under `l4_lora`.

## 2. The reachability contract (why L4 sees only misses)

L4 is consulted *only* when the modelless path has nothing usable
(`src/l4_fallback.rs` module docs, enforced by `g4_l4_composition.rs`):

1. **Never consulted on a modelless hit** — a parsing L0–L3 fix means L4
   is *not called at all* (not called-and-discarded). At ~48 s/invocation
   against ~16 µs for the retriever, one spurious call is a
   3-million-fold regression.
2. **Consulted only on a genuine miss** — no candidates, or the
   top-scoring candidate fails the L2 pruner.
3. **Never degrades a modelless answer** — a non-parsing L4 fix is
   discarded; L4 can only turn `None` into `Some`, or a non-parsing
   `Some` into a parsing one.
4. **Absent by default** — no fallback installed ⇒ byte-identical
   pipeline.

On top of the contract, the **relevance gate** (Issue 030, commit
`d8440f6`) fixed the third miss class: the serve/decline decision was
`parses` alone — a syntactic test used as a relevance proxy — so
off-distribution spans got valid-but-unrelated Rust and L4 was
suppressed entirely (measured miss rate **0/29**, Bench 037). With the
gate: out-of-corpus 9/9 decline, in-distribution unchanged, addressable
gap **9/29** (Bench 037). Lesson generalized: *a syntactic check is not
a relevance check.*

The **input-scope gate** (Bench 040 / Plan 016, `ScopeModel`) sits one
level up at the heal seam: out-of-scope (non-Rust) input declines the
whole heal before any L4 call — the ~48 s GPU call is never spent on
garbage.

## 3. The local model behind the seam

The reference implementation is **Bonsai-27B**
(`riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf`) with a clippy-fix LoRA
trained in riir-train (Plan 336). The model has a ~135 s load, so the
only viable transport is load-once/serve-many:

```
riir-train:  cargo run -p riir-train-engine --example bonsai_clippy_l4_eval -- --serve
this repo:   DaemonL4 spawns it, speaks the line protocol
             (<<<READY>>> / <<<FIX n>>> … <<<END n>>> / <<<QUIT>>>)
```

Measured ~193 s for the first fix, ~24 s for each after (vs ~193 s
*every* time for subprocess-per-fix — which is worse than the CPU
baseline that already failed G2). Two response-contract constraints are
easy to get wrong and are documented in `src/l4_daemon.rs`:

- **The prompt must not mention the lint** — all 1092 rows of the
  v2edit training corpus share one lint-agnostic instruction line
  (`TRAINED_INSTRUCTION`, reproduced verbatim; also shared by the CF
  re-gate harness since Issue 040). `rule_id` is for logging only.
- **Edit-only output is often a fragment** — the L2 pruner's
  `fn _wrap() { … }` fallback rescues statement-shaped fragments but not
  unbalanced delimiters, so even a G1-passing adapter loses a fraction
  of answers to invariant 3. That fraction is a property of the response
  contract, not a client bug.

## 4. Measured verdicts

| Bench | Question | Verdict |
|---|---|---|
| [`.benchmarks/037`](../../.benchmarks/037_modelless_miss_rate_l4_addressable_gap.md) | How much can L4 even address? | 9/29 reachable misses (after the Issue 030 relevance gate; 0/29 before) |
| [`.benchmarks/039`](../../.benchmarks/039_l4_vs_decline_reachable_set.md) | Does Bonsai-L4 beat `decline` on the reachable set? | **NO** — n=9: WIN 5 / NEUTRAL 0 / REGRESSION 4, and 2 of the 5 wins are *silently wrong*. Bar is `WIN ≥ 1 AND REGRESSION == 0`. |
| [`.benchmarks/040`](../../.benchmarks/040_contrastive_scope_adoption.md) | Does the scope gate save the L4 call? | YES — OOS 8/8 declined, 529 ns/input; the ~48 s call never fires on garbage |
| riir-train [Bench 465](../../../riir-train/.benchmarks/465_plan336_t41_g1_gate_step50.md) + [467](../../../riir-train/.benchmarks/467_plan336_t27_rank2_regate.md) | Does the trained adapter pass RLVR G1? | **FAIL 0/60** (tied with the frozen-backbone control), re-gate **0/60 degenerate** |
| riir-train [Bench 463](../../../riir-train/.benchmarks/463_plan336_t42_l4_inference_latency.md) | Latency acceptable? | **FAIL ~191 s/fix** |
| [`.benchmarks/049`](../../.benchmarks/049_fixer_regate_poc.md) | Re-gate on any arm | **CF llama-3.3-70b WINS** — strict-keep 5/9 (arm A) / 6/9 (arm B pre-pass) vs decline 0/9; ~$0.00025/fix; qwen3.8-27b unusable (thinking model eats the budget); the arm-C Bonsai daemon stayed blocked→optional |

## 5. The serving-lane decision (delegated perf/sec call, 2026-08-28)

Owner-delegated under the perf/sec mandate. The fixer lane's production
serving order, by measured quality-per-second:

1. **Modelless L0–L3 + decline — the default, unchanged.** Decline on
   the 9-case reachable-miss set is a *correct* answer (0 regressions);
   a wrong-but-parsing L4 fix is worse than the `None` the pipeline
   returns.
2. **CF llama-3.3-70b — the measured winner, NOT promoted here.** Bench
   049 lifts Proposal 005's fixer QUALITY blocker, but the arm sends
   code to a third party: the leak-posture sign-off is the owner's
   separate gate (and the kat-redact content-scanning lane is actively
   measured against exactly this question — see the `kat_redact` work).
   This decision deliberately does NOT touch that lane.
3. **Local Bonsai-27B L4 — stays opt-in and UNARMED. Demote-loser rule
   applied.** It does not beat decline (Bench 039), failed RLVR G1 twice
   (Benches 465/467), and costs ~191 s/fix (Bench 463) — a
   latency-negative, quality-negative path. **No further
   training/inference investment** until the reopen bar below is met.

Net effect: **no code or feature-flag change** — the default build
already behaves exactly as decided (seam present, nothing installed).
This section records the decision so the next "should we arm the local
model?" session starts from the measured verdict instead of re-litigating.

## 6. Reopen bar (the only path back)

The local L4 lane reopens when **all three** hold:

1. A real `.bcl4` checkpoint exists (Bench 049 arm C was blocked partly
   on this — the riir-train driver hard-requires `--lora`).
2. Its adapter scores `WIN ≥ 1 AND REGRESSION == 0` on the Bench 039
   9-case reachable set (the standing gate: `t51_l4_vs_decline.rs`).
3. The CF-vs-local tradeoff is re-measured — with the CF arm's quality
   bar now on record, the local lane must at minimum match the
   leak-free advantage it is supposed to offer, not merely exist.

## Files

- `src/l4_fallback.rs` — `L4Fallback` trait + `NullL4` + the 4 invariants
- `src/l4_daemon.rs` — `DaemonL4` transport + protocol + `TRAINED_INSTRUCTION`
- `src/self_evolve.rs:445` — `with_l4_fallback` / `has_l4_fallback`
- `src/cf_workers_ai.rs` — the CF Workers-AI client (Bench 049 winner arm)
- `tests/g4_l4_composition.rs` · `tests/t44_end_to_end_l4.rs` · `tests/t51_l4_vs_decline.rs`
- riir-train: `crates/riir-train-engine/examples/bonsai_clippy_l4_eval.rs` (+ `_sft_train`), Plan 336

## See also

- [`../09_lessons/modelless_first.md`](../09_lessons/modelless_first.md) — why L0–L3 are modelless
- [`../09_lessons/honest_poc_caveats.md`](../09_lessons/honest_poc_caveats.md) §3 — the POC-caveats framing of this page
- [`heal_pipeline.md`](heal_pipeline.md) — the pipeline the seam plugs into
- [`scope_model.md`](scope_model.md) — the input-scope gate one level up
- [AGENTS.md §Feature Flag Discipline → `l4_lora`](../../AGENTS.md) — the canonical feature entry
