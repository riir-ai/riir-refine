# The `HealDomain` trait

> The **one new abstraction** introduced by Plan 001 (Proposal 001). Everything
> else in the Phase 1 refactor was rename + reorganization. The domain-agnostic
> core (drafter + pruner + corpus) operates on the corpus entries this trait
> produces. The domain owns: (1) what counts as a fixable rule, (2) how to
> validate a proposed fix against ground truth.

## The trait

```rust
pub trait HealDomain: Sized {
    type Entry: Clone + Send + Sync + Serialize + DeserializeOwned + HealEntry + 'static;
    type Verdict: Clone + Send + Sync;

    fn build_corpus() -> Vec<Self::Entry>;
    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict;
}
```

- **`Entry`** — the corpus entry type. Each domain defines its own
  (`ClippyFixEntry`, `PerfRule`, `KernelRule`). Must implement `HealEntry`
  (field accessors) so the domain-agnostic core can read it.
- **`Verdict`** — the validator's verdict. Each domain picks its own shape
  (clippy: `ClippyVerifyResult` / `ClippyParseVerdict`; perf:
  `PerfVerifyResult` / `PerfParseVerdict`; kernel: `KernelVerifyResult` /
  `KernelVerdict`).
- **`build_corpus()`** — constructs the seed corpus for this domain
  (BLAKE3-committed via `Corpus::new`).
- **`validate(rule_id, proposed_fix)`** — the **Stage 2 tuning-phase
  validator** (NOT the hot-path guard). The tuning loop calls this OFF the hot
  path to build the Stage 2 adaptive drafter pool. clippy re-runs clippy
  (~130 ms), perf runs tests (seconds), kernel runs compile (multi-seconds to
  minutes when full).

## The `HealEntry` accessor trait

Type inference through associated types doesn't work in Rust (the original
Plan 001 draft proposed `draft_fix<D: HealDomain>(candidates: &[D::Entry])`,
but Rust cannot infer `D` from `D::Entry = ClippyFixEntry`). The solution is a
separate `HealEntry` trait with direct field accessors:

```rust
pub trait HealEntry {
    fn rule_id(&self) -> &str;
    fn buggy_code(&self) -> &str;
    fn fixed_code(&self) -> &str;
}
```

Methods are then generic over `E: HealEntry` with straightforward inference.

## Caveat (Proposal 001 caveat 1)

The signature is **provisional**. The three domains (clippy, perf, kernel) fit
this shape NOW, but a fourth domain might not. Ship exactly 3 impls; do not
speculatively add methods for hypothetical domains. Revisit if a fourth domain
forces a change.

## The three shipped impls

| Domain | Entry type | Verdict (parse-only) | Verdict (with `*_verify`) |
|---|---|---|---|
| `clippy_lints` | `ClippyFixEntry` | `ClippyParseVerdict` | `ClippyVerifyResult` (`clippy_verify`) |
| `rust_perf` | `PerfRule` | `PerfParseVerdict` | `PerfVerifyResult` (`rust_perf_verify`) |
| `kernel_opt` | `KernelRule` | `KernelVerdict` | `KernelVerifyResult` (`kernel_verify`) |

Each domain's `validate` has two impls (gated by `cfg(feature = ...)`): a
parse-only fast path (always available, uses the shared `RustAnalyzerPruner`)
and a richer verifier path (behind the domain's `*_verify` feature).

## Source

- [`src/domain.rs`](../../src/domain.rs) — the trait definitions
