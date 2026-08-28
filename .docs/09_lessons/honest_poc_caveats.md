# Honest POC caveats — what the pipeline does NOT do

The clippy-healing pipeline is a POC. These are the documented
limitations, not bugs.

## 1. The fast path doesn't verify lint resolution

`RustAnalyzerPruner::validate()` always returns `lint_resolved = true`.
This is a **documented lie**: the fast path doesn't shell out to
`cargo clippy`, so it can't truthfully claim the lint is resolved.

The truthful path is `validate_with_clippy()` (Phase 3, opt-in
`clippy_verify` feature), which does shell out. ~130ms/verify vs ~5.5µs.

See [`../04_pruner/honesty_contract.md`](../04_pruner/honesty_contract.md).

## 2. The seed corpus is illustrative, not verified

The 10 seed corpus entries represent *patterns* the tool learns from.
The `buggy_code` and `fixed_code` fields have NOT been verified as
actual trigger/fix pairs via `cargo clippy` in the fast path. They are
hand-curated examples.

For truthful verification, enable `clippy_verify` and run
`validate_with_clippy` on each entry.

See [`../02_corpus/seed_corpus.md`](../02_corpus/seed_corpus.md).

## 3. The model-based fallback (L4) is a seam, not a serving path

The `l4_lora` feature ships the fallback **seam only**: `pub trait
L4Fallback` (`src/l4_fallback.rs:56`) + `HealPipeline::with_l4_fallback`
(`src/self_evolve.rs:445`) + `DaemonL4` (the transport client for
riir-train's `bonsai_clippy_l4_eval --serve`, `src/l4_daemon.rs:198`).
It is inert unless a fallback is installed — no fallback is installed in
production. `clippy_lora` is a DEAD placeholder kept for invocation
back-compat; enabling it compiles nothing.

No local model is armed — measured, not argued: Bonsai-27B scored
WIN 5 / REGRESSION 4 on the 9-case reachable set (`.benchmarks/039`),
failing the bar (WIN ≥ 1 AND REGRESSION == 0) — it does not beat
`decline` — after the RLVR G1 gate failed twice 0/60 (riir-train
Benches 465/467) at ~191 s/fix (Bench 463). The CF llama-3.3-70b arm
won the re-gate (`.benchmarks/049`) but stays behind the owner's
leak-posture sign-off. Full narrative + the serving-lane decision:
[`../10_self_evolve/l4_fallback.md`](../10_self_evolve/l4_fallback.md).

The modelless path (L0–L3) is primary, and `decline` is a correct
answer: L4 can only turn `None` into `Some`, never degrade a modelless
result.

## 4. No real clippy integration (no `cargo clippy` parsing in L1)

The L1 drafter takes `(lint_name, buggy_code)` as input. In production,
this would come from parsing `cargo clippy --message-format=json`
output. The pipeline does NOT include a clippy JSON parser — that's the
caller's responsibility.

The `clippy_verifier.rs` module (Phase 3) DOES parse clippy JSON output,
but only for the `verify()` path (checking whether a fix resolved a
lint), not for the `draft_fix()` input path.

## 5. No WASM rust-analyzer pruner

The `wasm_pruner` feature flag is a placeholder. The WASM
rust-analyzer variant (for sandboxed validation) is deferred on rubrc
(a WASM-buildable rustc/rust-analyzer) maturity.

## 6. Demo weights, not trained weights

The `heal_demo.rs` example uses a small deterministic ternary weight
pattern (`(i + j) % 3`), not trained weights. In production, weights
would be loaded from `.bits` files (frozen direction vectors trained on
a clippy lint corpus). The training pipeline lives in riir-train.

## 7. RustAssistant ceiling (75%)

Prior art: [RustAssistant](https://arxiv.org/abs/2412.16358) (Microsoft
Research, 2024) achieves ~75% lint resolution on their benchmark. This
repo's modelless path has not been benchmarked against that ceiling.
The GOAT gates measure correctness of the pipeline mechanics (parse,
draft, validate), not end-to-end lint resolution rate on real codebases.

## See also

- [`../04_pruner/honesty_contract.md`](../04_pruner/honesty_contract.md) — the L2 honesty contract
- [`modelless_first.md`](modelless_first.md) — why L4 is deferred
