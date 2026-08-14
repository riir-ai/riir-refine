# cargo-heal — the unified healing CLI (Plan 005 Phase 2 + Phase 6 T6.1/T6.4)

**Source:** [`src/bin/heal.rs`](../../src/bin/heal.rs)
**Feature:** `latent_retrieval` + `rust_perf` + `clippy_lints` (required —
the matcher/indexes live behind the first, the `--fix` proposers behind the
latter two)
**Run (in-repo):**

```sh
cargo run --features latent_retrieval --bin cargo-heal -- --suggest src/
cargo run --features latent_retrieval --bin cargo-heal -- --fix --quiet src/          # review
cargo run --features latent_retrieval --bin cargo-heal -- --fix --quiet --write src/  # apply
```

Installed via `cargo install --path . --features latent_retrieval`, cargo's
subcommand convention makes it `cargo heal --suggest src/`.

## What it does

`--suggest` mode (the MVP): walks real `.rs` files (std-only recursive
walker — no new dep; skips hidden/`target`/`vendor`), chunks them at
fn/impl/struct boundaries, retrieves matching corpus rules via the latent
substrate, and prints top-K rules per span with scores + the
fixed/optimized pattern snippet head for manual review.

`--fix` mode (Phase 6 T6.1 + T6.4): runs the compiled-in proposers'
visitors on each file + applies their **span-preserving** edits — the 5
rust_perf rules PLUS the 7 structural clippy lints (T6.4 migrated the
clippy proposers from AST-mutation + whole-file re-emission to the same
edit-splice emitter). Everything outside a matched span stays
byte-identical (no reformat; see
[Bench 019](../../.benchmarks/019_plan005_phase6_fix_span_preserving_goat.md)).
The proposer's visitor is the gate: latent suggestions never auto-apply.
Review mode (default) prints per-edit rule + final-source line; `--write`
splices in place. `--quiet` skips the suggest pass when fixing.

Fix application iterates to a **fixpoint** (bounded, 4 passes): an outer
edit's replacement keeps the inner nodes' ORIGINAL text, so fixes nested
inside a replaced region (`return Cfg { x: x };` — the field-shorthand
edits live inside the return-strip's span) are dropped by overlap
selection on pass 1 + picked up on pass 2 after re-parse. The
unchanged-output guard stops oscillation. Per-edit review lines reference
the **final written source**: each pass's edits are recorded in input
coordinates, then remapped through every pass's applied splices
(`apply_edits_traced` + `map_through_splices`) before line-counting — so
an edit before a line-count-changing fix (manual_map's collapse,
fxhashmap's inserted `use`) still points at the right line. Only edits
that were actually applied are reported (pass-1 overlap-dropped edits
re-applied on pass 2 are not double-counted).

The 3 type-blind clippy proposers (`needless_borrow`,
`useless_conversion`, `unnecessary_mut_passed`) are NOT `--fix` eligible —
their detection is only sound with type knowledge (available in the
`self_evolve` loop's cargo-clippy-verified path, not parse-only
validation). They stay suggest-only at the CLI.

```text
cargo heal --suggest [--domain <auto|clippy|perf|kernel>] [--top-k <N>] <path>...
cargo heal --fix [--write] [--quiet] <path>...

--suggest    Suggest mode: print matching rules per code span (no edits).
--fix        Fix mode: apply the compiled-in proposers' span-preserving edits
             (perf: vec-with-capacity, entry-vs-contains-insert, static-str,
             bool-to-bitpack, fxhashmap; clippy: needless_return,
             redundant_closure, redundant_field_names, manual_map,
             unnecessary_unwrap, needless_range_loop, needless_lifetimes).
--write      With --fix: write the spliced file in place (default: review only).
--quiet      With --fix: skip the --suggest pass.
--domain     Filter suggest output to one domain (default: auto — all enabled).
--top-k <N>  Rules per span per domain (default 3).
<path>...    Files or directories to scan (.rs files only).
```

The `fxhashmap` fix requires the healed crate to have the `rustc-hash`
dependency — parse-only validation cannot check this (documented per-rule
caveat; review mode is the safety net).

## Domain gating (T2.4)

Domains are feature-gated: clippy + perf are default-on; kernel needs
`--features kernel_heal`. Requesting a domain whose feature is off is a
**hard error** listing the enabled set (verified on `--domain kernel`
without `kernel_heal`).

## Smoke + GOAT

- Smoke-verified on this repo's own `src/` — 19 files, ~0.4 s total
  including index build: real spans + scores + suggestions (Plan 005
  Phase 2 exit criterion).
- The retrieval layer behind the CLI passed its GOAT gate — G1 100%
  (12/12), G2 349 µs/chunk, G3 104/104, G4 scratch stable — see
  [Bench 018](../../.benchmarks/018_plan005_latent_suggest_goat.md) and
  [`../03_drafter/latent_suggest.md`](../03_drafter/latent_suggest.md).

## Caveat

No minimum-score threshold — and none is possible (measured, Bench 018
addendum): clean adversarial code scores as high as buggy code under the
text-similarity scorer. Treat output as a ranked review queue, not a
verdict; `--fix`'s proposers are the instantiation gate.

## See also

- [`../03_drafter/latent_suggest.md`](../03_drafter/latent_suggest.md) — the retrieval layer this CLI wraps
- [Plan 005](../../.plans/005_real_file_span_healing.md) Phase 2 — the CLI tasks
