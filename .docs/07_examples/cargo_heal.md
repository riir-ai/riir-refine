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

`--fix` mode (Phase 6 T6.1 + T6.4 + Issue 016): runs the compiled-in
proposers' visitors on each file + applies their **span-preserving** edits
— the 5 rust_perf rules PLUS the 33 structural clippy lints (T6.4
migrated the clippy proposers from AST-mutation + whole-file re-emission
to the same edit-splice emitter; the T6.2 G5 real-file run later demoted
`redundant_closure` to the type-blind set after it caught an E0631
fn-path coercion break — see
[Bench 019 §G5 addendum](../../.benchmarks/019_plan005_phase6_fix_span_preserving_goat.md); Issue 016
expanded the set 6→33 + grew the `--verify` tier 1→4 — see
[Bench 021](../../.benchmarks/021_clippy_structural_expansion_goat.md)). Everything outside a matched span stays
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

The 4 type-blind clippy proposers (`needless_borrow`,
`useless_conversion`, `unnecessary_mut_passed`, `redundant_closure`) are
NOT `--fix` eligible — their detection is only sound with type knowledge
(available in the `self_evolve` loop's cargo-clippy-verified path, not
parse-only validation). They stay suggest-only at the CLI.

```text
cargo heal --suggest [--domain <auto|clippy|perf|kernel>] [--top-k <N>] <path>...
cargo heal --fix [--write] [--quiet] [--verify [--verify-args <s>]] <path>...

--suggest    Suggest mode: print matching rules per code span (no edits).
--fix        Fix mode: apply the compiled-in proposers' span-preserving edits
             (perf: vec-with-capacity, entry-vs-contains-insert, static-str,
             bool-to-bitpack, fxhashmap; clippy: needless_return,
             redundant_field_names, manual_map,
             unnecessary_unwrap, needless_range_loop, needless_lifetimes).
--write      With --fix: write the spliced file in place (default: review only).
             Output preserves surrounding text verbatim outside edited spans —
             run rustfmt on written files afterwards for final formatting
             (span-preserving edits keep pre-existing indentation where rustfmt
             would reindent, e.g. match_bool mixed block/expr arms; Issue 027 T3).
--quiet      With --fix: skip the --suggest pass.
--verify     With --fix (needs --features fix_verify): write, cargo-check,
             auto-revert ONLY the offending edits (rustc JSON errors mapped
             to file/line), re-check, report. Implies --write. Loosens the
             vec-with-capacity usize gate (plain-path bounds attempted;
             E0308 backs the revert) + adds the 4 VERIFY_LINTS
             (redundant_closure, comparison_to_empty, while_let_on_iterator,
             single_char_pattern).
--verify-args <s>  Extra cargo args for --verify (whitespace-split, e.g.
             "--features a,b,c" — MUST cover every edited module; a
             default-features check is vacuous when `default = []` gates
             the edited modules, the Bench 019 G5 lesson).
--domain     Filter suggest output to one domain (default: auto — all enabled).
--top-k <N>  Rules per span per domain (default 3).
<path>...    Files or directories to scan (.rs files only).
```

The `fxhashmap` fix requires the healed crate to have the `rustc-hash`
dependency — parse-only validation cannot check this (documented per-rule
caveat; review mode is the safety net). Under `--fix --verify` the dep gate
is automatic: the owning crate's `Cargo.toml` is text-scanned for
`rustc-hash` and the rule hard-skips when absent (no cargo run needed).

## Verify mode (Issue 015)

`--fix --verify` is the compile gate as tooling — the
`rust_perf_verify`/`clippy_verify` pattern applied to the edit path
instead of the draft path. Algorithm: baseline check on the unmodified
tree (a broken tree cannot host a verification) → write every fixed file
→ check → revert only the edits whose lines carry rustc errors
(line-matched span-level; whole-file fallback when attribution is
impossible) → re-check → repeat until clean (bounded by edit count + 2).
The `#[ignore]`d e2e fixture test (`fix_verify_e2e_bad_edits_reverted_good_survive`,
spawns real cargo) is the standing acceptance proof; the real-repo GOAT
(katgpt-pruners, the Bench 019 G5 crate: 14 edits / 12 kept / 2
auto-reverted — the u8-capacity E0308 + the fn-path coercion E0631, the
exact G5 run-1 failure classes — every named skipped site landed
compile-confirmed usize, 1245/1245 tests) is
[Bench 031](../../.benchmarks/031_fix_verify_mode_goat.md) (landed as
katgpt-rs `212ed7942`).

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
