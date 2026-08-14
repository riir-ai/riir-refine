# Bench 019 — Plan 005 Phase 6 T6.1: span-preserving `--fix` GOAT (G1-G4)

**Date:** 2026-08-14
**Scope:** the `cargo heal --fix` wiring — `SourceEdit` splice emitter +
CLI `--fix [--write] [--quiet]` + the 5 rust_perf proposers refactored from
AST-mutation to edit-collecting.
**G5 (real fix in a sibling repo):** **BLOCKED** — the riir-ai working tree
is mid-rebase with a sibling session's in-flight 672-revert work; landing a
real fix there must wait for their rebase to complete. Recorded as T6.2.

## The emitter

`proc-macro2` `span-locations` (1.0.107 has `Span::byte_range()`; verified:
`syn::parse_file` in a normal lib populates real byte offsets — pinned by
the `span_text_round_trips` test). Each proposer walks the parsed file
**immutably** and emits `SourceEdit { start, end, replacement }` where the
replacement text is built from the **original source text of the matched
nodes** (`src(source, node)`) plus short literals — the edited regions keep
the caller's own formatting for every sub-expression they contained.

`apply_edits`: selection (sort asc-by-start, outer-first on ties; drop any
edit starting before the running end — overlaps/containments), then apply
**back-to-front** so earlier byte offsets stay valid as replacements change
lengths. (The one-pass asc-order version corrupted offsets — caught by the
fxhashmap corpus fixture; the two-phase fix is load-bearing.)

No `prettyplease`, no whole-file token re-emission, no reformat.

## Gates

| Gate | Target | Measured | Verdict |
|---|---|---|---|
| **G1 correctness** | fixture round-trips: detect → rewrite → output parses + carries the fixed-form marker | 147 lib tests (all 5 proposers' fixtures + the corpus end-to-end case; L2 pruner parse-check on every output) | **PASS** |
| **G1' span preservation** | everything outside matched spans byte-identical | `output_preserves_untouched_bytes`: comments, blank structure, untouched stmts survive verbatim; exactly 1 line differs on the vec fixture. Live: `--fix --write` on a real 700-line file (`src/adaptive_drafter.rs`) changed exactly 1 line (`Vec::new()` → `Vec::with_capacity(4)` at L197) | **PASS** |
| **G2 perf** | proposer pass cheap per file | parse + 5 proposer scans over 46 repo files, ~0.4s total incl. index build + suggest machinery (unchanged from Bench 018 regime) | **PASS** |
| **G3 no-regression** | existing suite green | 147 lib tests (141 pre-existing + 6 new emitter/splice tests), clippy --all-targets clean, `--features rust_perf_verify` check clean | **PASS** |
| **G4 alloc-free hot path** | — | N/A at this layer: the proposers allocate only for the `Vec<SourceEdit>` outputs (bounded by matches) — the suggest hot path is unchanged (Bench 018 §G4 stands). Honest note, not a claimed gate | n/a |

## Real-world smoke (review mode, no writes)

`cargo heal --fix --quiet src/` on this repo: 4 files / 12 candidate edits.
Inspected: `adaptive_drafter.rs:197` (`Vec::new()` + `0..4` push loop) —
**true positive**, textbook `with_capacity(4)`. The `fxhashmap` hits on
`draft/corpus_index.rs` etc. are syntactically true but **require the
`rustc-hash` dep** — exactly the documented caveat (parse-only validation
cannot check deps; review mode is the safety net).

## Honest limits

- `--fix` covers the 5 rust_perf proposers only (T6.1 scope). The 10
  clippy_lints proposers remain mutation-based (consumed by `self_evolve`);
  migrating them to the edit emitter is follow-up work.
- Kernel: no proposers (Phase 5 deferred — concept-grade corpus, see Plan
  005 §Phase 5).
- `--write` trusts the proposers' visitor gates; signature-visible rewrites
  (`static-str`, `bool-to-bitpack`) ripple to callers — `rust_perf_verify`
  (cargo test) is the semantic gate, run by the consumer.

## Commands

```bash
cargo test -p riir-clippy --lib                    # 147 pass
cargo clippy --all-targets                         # clean
cargo run --features latent_retrieval --bin cargo-heal -- --fix --quiet src/   # review
cargo run --features latent_retrieval --bin cargo-heal -- --fix --quiet --write src/
```
