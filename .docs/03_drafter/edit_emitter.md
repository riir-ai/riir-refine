# edit_emitter — the shared span-preserving `--fix` substrate

**Source:** [`src/edit_emitter.rs`](../../src/edit_emitter.rs)
**Feature:** none (always compiled — depends only on `syn` + `proc-macro2`)
**Landed:** Plan 005 Phase 6 T6.4 (extracted from `rust_perf::proposers`)

## What it is

The ONE implementation of the edit-splice protocol every `--fix` proposer
uses, regardless of domain:

- [`SourceEdit`] — a byte range + replacement text against the exact
  original source the file was parsed from.
- [`apply_edits`] — the two-phase splice: selection sorts ascending by
  start (outer/larger spans first on ties; an edit starting inside an
  already-selected region is dropped), then application runs
  **back-to-front** so earlier byte offsets stay valid as replacement
  lengths change.
- [`apply_edits_traced`] — `apply_edits` that also returns the splices it
  actually applied (in INPUT coordinates). Anything needing "what really
  applied" must use this — `apply_edits` silently drops overlapping edits,
  and a caller that ignores that double-counts re-applied pass-2 edits.
- [`map_through_splices`] — remaps a position recorded against a pass's
  INPUT through that pass's applied splices into OUTPUT coordinates. All
  comparisons use the ORIGINAL offset (a walking position compared against
  unshifted splice starts mis-orders deletion-then-insertion pairs); a
  position inside a replaced region clamps to the region's output start.
  The `--fix` review line numbers compose one remap per pass so they
  reference the FINAL written source even when earlier fixes changed line
  counts (`manual_map` collapses matches, `fxhashmap` inserts a `use`).
- helpers — `src` (node → its ORIGINAL source text via `proc-macro2`
  span-locations), `edit` (node → edit), `for_each_fn` (free fns + impl
  methods + nested inline mods), `swallow_following_space` (`&'a u8` →
  `&u8`, not `& u8`), `line_indent`, `span_range`.

## Why feature-neutral

`clippy_lints` and `rust_perf` are separate Cargo features; a shared
substrate living in either one would break the other under
`--no-default-features` single-domain builds. `edit_emitter` has no
feature gate — both domains' proposer modules re-export it
(`rust_perf::proposers` keeps its pre-T6.4 import path for API stability).

## Consumers

- `domains/rust_perf/proposers/` — the 5 perf rules (Phase 4 / T6.1).
- `domains/clippy_lints/proposers/` — the 10 clippy lints (T6.4
  migration from AST-mutation + whole-file `ToTokens` re-emission, which
  destroyed the caller's formatting).
- `bin/heal.rs` — composes both domains' edits per file, then iterates to
  a bounded fixpoint (see
  [`../07_examples/cargo_heal.md`](../07_examples/cargo_heal.md)). Uses
  `apply_edits_traced` + `map_through_splices` to report per-edit lines
  against the final written source.

## See also

- [`../07_examples/cargo_heal.md`](../07_examples/cargo_heal.md) — the CLI wiring
- [`../11_domains/rust_perf.md`](../11_domains/rust_perf.md) §Proposers
- [Bench 019](../../.benchmarks/019_plan005_phase6_fix_span_preserving_goat.md)
