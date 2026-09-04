# Bench 041 — `rustc_errors` premise probe + `cargo fix --broken-code` G-floor

**Date:** 2026-08-21
**Issue:** `.issues/035_cargo_fix_floor_premise_probe.md` (T1, T2)
**Plan:** `.plans/019_rustc_errors_fix_compile.md` (T0.4, T6.9)
**Feature:** `rustc_errors` (default-ON since 2026-08-29 by owner call, after all GOAT gates — incl. this bench's floor — had passed; driver-shaped, inert until `--fix-compile` is invoked, so default-on costs compile time only. *Stale label `opt-in, default-off` corrected 2026-09-01 per katgpt-rs `.issues/702`.*)
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

## 3. The `--fix-compile` arm (measured 2026-08-22, Phase 3 landing)

The pending arm above landed with Phase 3 (`DriverFixer` — the production
driver + registry over the same corpus; E0502 deref-removal/clone-out, E0499
swap, E0505 clone-at-move; the E0597 family from Phase 2).

| code | mutants | cargo fix | `--fix-compile` | verdict |
|---|---|---|---|---|
| E0597 | 2 | **0%** unfixed | **100%** (2 diverged) | **net-new** — the uncontested head, covered |
| E0499 | 1 | **0%** unfixed | **100%** (1 diverged) | **net-new** — the swap reorders instead of re-scoping |
| E0502 | 2 | **0%** unfixed | **100%** (1 **matched** + 1 diverged) | **net-new** — deref-removal restores the `Copy` seed byte-identically |
| E0505 | 1 | 100% but **100% diverged** | **100% MATCHED** | **the contested win** — clone-at-move keeps `r` borrowing `v`; rustc's `&v.clone()` silently changes aliasing |
| E0596 | 2 | 100% matched | 0% (no template) | delegated to `cargo fix` by the floor gate |
| E0382 | 1 | 100% matched | 0% (no template) | delegated |
| E0507 | 1 | 100% matched | 0% (no template) | delegated |
| E0373 | 2 | 100% matched | 0% (no template) | delegated |

**Total: 6/12 fixed, 2 matched + 4 diverged.** The uncontested subset is
6-for-6 counting E0505: every mutant `cargo fix` cannot fix (or fixes
wrongly) is repaired, and the one where it diverges is repaired to the known
inverse. The 4 diverged verdicts are semantically-equal-but-not-byte-seed
reorders (E0597 hoist keeps an empty block; E0499 swaps instead of re-scoping;
E0502 clone-out clones the element instead of reading `.len()` through the
reference) — each named in the diff, per the plan's semantics-honesty rule.

**The head-to-head that ships the product claim:** on the 5 uncontested
mutants the floor scores 0/5 and `--fix-compile` scores 5/5; on E0505 the
floor's only fix silently changes aliasing and ours matches the inverse.
Nowhere does `--fix-compile` apply a fix the floor beats.

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

## 4. Toolchain re-verify — rustc 1.98.1 (2026-09-05)

The stable pin bump to **1.98.1** (repo-wide, owner-directed; riir-clippy
`4c96c64`) armed this bench's standing T4 reminder. Re-ran the full surface
under rustc **1.98.1** (`premise_probe` 6/6 + `floor_corpus_integrity` 3/3 +
`rustc_errors_floor` release):

- **§1 premise probe: UNCHANGED.** Same 8-row table — 0 suggestion spans on
  E0597/E0499/E0502, 1 machine-applicable span on each contested code with the
  same replacements (`.clone()` ×3, `mut `, `move `). The
  contested/uncontested split survived the bump: rustc 1.98 did not gain
  auto-fixes for the uncontested three, and still owns the five contested.
- **§2 `cargo fix` floor at N=212: bit-stable.** Identical per-code row to the
  Bench 045 record — E0373 27/27, E0382 21/21, E0499 0/26, E0502 0/37, E0505
  100% fix at 100% divergence (21), E0507 16/16, E0596 22/22, E0597 0/42.
  Total 50.5%, uncontested stratum 0/105.
- **§3 `--fix-compile` driver: bit-stable.** Identical to the Bench 045 driver
  arm — E0597 42 diverged, E0499 26 diverged, E0502 21 matched + 16 diverged,
  E0505 21 MATCHED, delegated 86 unfixed. 126/212 repaired (59.4%),
  uncontested 105/105, G1 semantics pass 126/126, no-op arm 0.0%, seed
  validity 212/212.
- **G2 latency:** p50 156 ms/case / 95 ms/iteration (vs the recorded 151 / 78
  at 1.97) — the box carried sibling waves at ~99% CPU during the run; not
  read as a toolchain signal.

Verdict: **the entire measured surface is bit-stable across 1.97 → 1.98.1.**
No territory moved; no playbook delegation invalidated; T4 discharged for
this bump.

Same-window re-gates recorded elsewhere: score-bench #36 flat
(96.2/1.9/100/100 — clippy 1.98 shifted nothing on the frozen fixtures); the
three toolchain-regime `rust_perf` rules re-gated via the perf-bench
validator with verdict classes unchanged (`mir-destprop-branch-site-merge`
ParetoDominant, `debug-assert-length-hoist` Mixed, `runtime-feature-detect-
per-chunk` Mixed); clippy 1.98's new `manual_midpoint` lint fired twice on
the libdivide G1 mirror (algorithm-mirroring code, `#[allow]`ed with reason —
the construction is pinned to the harness string by the substring canary).
