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

## 3. L4 LoRA fallback is deferred

The L4 layer (adaptive LoRA learning on residual lints) is deferred to
riir-train. It does not exist in this repo. The `clippy_lora` feature
flag is a placeholder; enabling it compiles nothing.

The modelless path (L0–L3) is primary. LoRA is the conservative last
resort.

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
