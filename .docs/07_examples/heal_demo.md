# heal_demo — end-to-end pipeline demo

**Source:** [`examples/heal_demo.rs`](../../examples/heal_demo.rs)
**Feature:** default (no feature flag needed)
**Run:** `cargo run --example heal_demo`

## What it demonstrates

The full L0→L1→L2 pipeline on the seed corpus:

1. **L0**: Builds a `ClippyCorpus` from `build_seed_corpus()` (40 lints —
   10 seed + 30 Issue 016 structural-expansion).
2. **L1**: Drafts fixes using `ClippyTernaryDrafter::with_weights()` with
   a small deterministic demo weight matrix (8 rows × 16 cols).
3. **L2**: Validates each drafted fix with `RustAnalyzerPruner::validate()`.
4. **Summary**: Prints pass rate + per-lint details (confidence, parse
   status, diff preview).
5. **Fallback demo**: Also runs the L0 template path (no weights) to show
   the `CorpusTemplate` source.

## Demo weights

The demo uses a small deterministic ternary weight pattern:

```rust
match (i + j) % 3 {
    0 => 1.0,
    1 => -1.0,
    _ => 0.0,
}
```

In production, these would be loaded from `.bits` files (frozen direction
vectors trained on a clippy lint corpus). The demo pattern exercises the
SIMD matvec path without requiring trained weights.

## Output shape

```text
╔══════════════════════════════════════════════════════════════╗
║     riir-refine — End-to-End Clippy Healing Pipeline Demo   ║
╚══════════════════════════════════════════════════════════════╝

📦 L0 Corpus: 10 lint entries (BLAKE3 verified: ✅)
   Lints: manual_map, needless_borrow, needless_lifetimes, ...

⚡ L1 Drafter: ternary SIMD matvec (weights: 8 rows × 16 cols)

🔍 L2 Pruner: syn-based syntax validation

─── Pipeline Run ─────────────────────────────────────────────────

Lint: needless_return [TernaryMatvec]
  Confidence: 0.6225
  Parse:      true  Lint resolved: true  ✅ VALID
  Buggy:  fn foo() -> i32 { return 42; }
  Fixed:  fn foo() -> i32 { 42 }

─── Summary ────────────────────────────────────────────────────
  Total lints:    10
  Fixes drafted:  10 (L1 ternary matvec)
  Fixes valid:    10 (L2 syntax pruner)
  Pass rate:      100.0%
```

## See also

- [`../02_corpus/seed_corpus.md`](../02_corpus/seed_corpus.md) — the 40-entry corpus (10 seed + 30 Issue 016)
- [`../03_drafter/clippy_ternary_drafter.md`](../03_drafter/clippy_ternary_drafter.md) — the drafter
