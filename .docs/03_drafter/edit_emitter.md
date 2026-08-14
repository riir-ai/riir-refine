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
  [`../07_examples/cargo_heal.md`](../07_examples/cargo_heal.md)).

## See also

- [`../07_examples/cargo_heal.md`](../07_examples/cargo_heal.md) — the CLI wiring
- [`../11_domains/rust_perf.md`](../11_domains/rust_perf.md) §Proposers
- [Bench 019](../../.benchmarks/019_plan005_phase6_fix_span_preserving_goat.md)
