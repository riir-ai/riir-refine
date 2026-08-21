# Bench 041 — `rustc_errors` premise probe + `cargo fix --broken-code` G-floor

**Date:** 2026-08-21
**Issue:** `.issues/035_cargo_fix_floor_premise_probe.md` (T1, T2)
**Plan:** `.plans/019_rustc_errors_fix_compile.md` (T0.4, T6.9)
**Feature:** `rustc_errors` (opt-in, default-off)
**Toolchains:** rustc **1.97.0** (repo pin, `rust-toolchain.toml`) — and rustc
**1.93.0** (box default) for the original ad-hoc probe. Both agree; the split
below is not a single-toolchain artifact.
**Repro:**
```bash
cargo test --features rustc_errors --test premise_probe
cargo test --features rustc_errors --test floor_corpus_integrity
cargo run --release --example rustc_errors_floor --features rustc_errors
```

## Why this bench exists

Plan 019 was filed on the claim that rustc emits NO machine-applicable
suggestions for the borrowck family and that `cargo fix` is "structurally
blind" there. It is false on 5 of the 8 codes probed. The plan's own founding
lesson — *when reasoning and the compiler-oracle disagree, the oracle wins* —
is now enforced by tests rather than asserted in prose.

## 1. Premise probe (T1) — does rustc already fix this code?

One isolated temp crate per case; `cargo check --message-format=json` for
suggestion spans + applicability, then `cargo fix --broken-code` on a pristine
copy to see what rustc repairs unprompted.

| case | code | sugg spans | machine-applicable | `cargo fix` edited | territory | replacement |
|---|---|---|---|---|---|---|
| `e0597_local_lifetime` | E0597 | 0 | 0 | no | **UNCONTESTED** | — |
| `e0499_double_mut` | E0499 | 0 | 0 | no | **UNCONTESTED** | — |
| `e0502_borrow_then_mut` | E0502 | 0 | 0 | no | **UNCONTESTED** | — |
| `e0382_use_after_move` | E0382 | 1 | 1 | yes | contested | `.clone()` |
| `e0505_move_out_of_borrowed` | E0505 | 1 | 1 | yes | contested | `.clone()` |
| `e0507_move_out_of_ref` | E0507 | 1 | 1 | yes | contested | `.clone()` |
| `e0596_borrow_as_mut` | E0596 | 1 | 1 | yes | contested | `mut ` |
| `e0373_closure_outlives` | E0373 | 1 | 1 | yes | contested | `move ` |

**The addressable head is 3 codes, not 7.**

**Plan 019's flagship template is redundant.** Playbook template 1 (`move`
closure) targets the `Box<dyn Fn>`-captures-a-local shape. rustc diagnoses
that as **E0373**, not E0597, and `cargo fix` applies `move ` itself
(`closure_outlives_shape_is_e0373_and_rustc_fixes_it` pins both halves).

## 2. G-floor (T2) — `cargo fix --broken-code` over the mutation corpus

12 known-inverse mutants: a compiling seed plus a declared source rewrite that
breaks it, so the ground-truth fix is the seed — no hand labeling. Every repair
is scored against that inverse, because fix rate alone cannot distinguish
"restored the meaning" from "compiles now, means something else".

| code | mutants | matched | diverged | unfixed | fix rate | divergence |
|---|---|---|---|---|---|---|
| E0597 | 2 | 0 | 0 | 2 | **0%** | — |
| E0499 | 1 | 0 | 0 | 1 | **0%** | — |
| E0502 | 2 | 0 | 0 | 2 | **0%** | — |
| E0373 | 2 | 2 | 0 | 0 | 100% | 0% |
| E0382 | 1 | 1 | 0 | 0 | 100% | 0% |
| E0507 | 1 | 1 | 0 | 0 | 100% | 0% |
| E0596 | 2 | 2 | 0 | 0 | 100% | 0% |
| E0505 | 1 | 0 | **1** | 0 | 100% | **100%** |
| **total** | **12** | 7 | 1 | 5 | **58.3%** | 14.3% |

The floor is bimodal, exactly along the territory line: **0% on all three
uncontested codes, 100% on all five contested ones.** No partial credit
anywhere. `cargo fix` either owns a code completely or cannot touch it.

### The one divergence — and what it costs the plan

`e0505_drop_clone_under_borrow`: seed `let r = &v; let w = v.clone();`, mutated
to `let w = v;`. rustc repairs the *other* line:

```rust
let r = &v.clone();   // rustc's fix: r now borrows a temporary
let w = v;
```

It compiles. `r` no longer aliases `v` at all — it borrows an unnamed
temporary. Aliasing changed, silently, under a `MachineApplicable` label.

That single case is the entire measured basis for the plan's
"ranked alternatives" pitch on contested codes — and it cuts the other way
too: on E0596, E0382, E0507 and E0373 (7 of 8 contested mutants) `cargo fix`
did not merely compile, it **reproduced the known inverse exactly**. The
differentiated-value claim on contested territory is not "rustc is crude";
it is "rustc is right 6 times out of 7 and silently wrong once."

## Verdict

- **T1 PASS** — premise probe is a standing test on 2 toolchains. A rustc bump
  that starts auto-fixing E0597/E0499/E0502 now fails CI instead of quietly
  deleting the product's head.
- **T2 PASS** — floor measured per code. `--fix-compile` templates for E0596,
  E0382, E0507, E0373 must clear a **100% fix rate at 0% divergence**; on this
  corpus there is no headroom, so those templates should NOT be authored.
- **The plan's uncontested head is confirmed real:** 0% floor on E0597, E0499,
  E0502 across 5 mutants. Any coverage there is net-new.
- **Contested territory is measured as near-solved by the toolchain.** The only
  opening is the E0505 aliasing class.
- **Not yet measured:** the `--fix-compile` arm (Plan 019 Phase 1 unimplemented).
  The `CompileFixer` seam and the report shape are final, so landing the driver
  is a one-line arm swap. `NullFixer` scores 0.0% — the harness is measuring
  the fixer, not the mutation.

## Instrument credibility (the checks that fired)

Three real bugs, all caught by the harness's own gates rather than by review:

1. **Parallel-test work-root collision.** Both runners keyed their temp root
   by pid; `cargo test` runs tests as threads of ONE process, so the
   clean-slate `remove_dir_all` deleted a sibling's crate mid-run. It surfaced
   as *"E0505 has 0 suggestion spans"* — a false product claim in the
   flattering direction. Fixed with a per-instance atomic counter. Had the
   table been "updated to match the measurement", the corrupted reading would
   have become the record.
2. **A mutant that still compiled.** `e0597_ref_escapes_block` narrowed a scope
   around a `usize` length — `usize` is `Copy`, nothing was borrowed. The
   no-op arm scored 8.3% instead of 0%, which is what exposed it. A mutant
   that compiles is a free win for every arm.
3. **A mutant failing with the wrong code.** `e0502_borrow_across_mutation`
   borrowed but kept an `as usize` cast → E0606 (bad cast), not E0502. Caught
   by the declared-code integrity check.

Both corpus bugs inflated the floor (66.7% → 58.3% after fixing). Bugs 2 and 3
are now permanent gates in `tests/floor_corpus_integrity.rs`.

## Corpus caveats (bounding the claim)

- **12 mutants, 6 mutator kinds, hand-authored seeds.** The rewrites are
  declared `from`/`to` pairs, not random AST fuzz. Numbers computed here are
  about these mutation classes, not about all Rust. Plan 019 T6.1's N≥200
  target is NOT met and is not claimed.
- **Mutation-corpus bias.** Every case's ground truth is "restore the seed",
  which favors fixers that undo edits. Real-world broken code has no seed to
  return to; a 100% matched-inverse rate here does not mean `cargo fix` is
  right 100% of the time in the wild.
- **Whitespace-insensitive matching.** `matched-inverse` compares
  whitespace-normalized source, so a reflow counts as a match. Semantics are
  not otherwise verified — the corpus carries no behavioral tests, so
  "matched" means textually equal to the seed, not proven equivalent.
- **Single-file crates, no deps, no features.** The feature-set blindness
  recorded for `fix_verify` applies here unchanged.
