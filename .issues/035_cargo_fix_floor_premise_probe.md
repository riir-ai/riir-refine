# Issue 035 — `cargo fix --broken-code` is the floor for `rustc_errors`; premise probe + repro

**Date:** 2026-08-21
**Status:** OPEN — T1 + T2 DONE (Bench 041, 2026-08-21); T3 + T4 open. Still
blocks Plan 019 Phase 3 (contested codes).
**Refs:** `.plans/019_rustc_errors_fix_compile.md` (thesis revised in the same commit)

## Why

Plan 019 opened with the claim that rustc emits NO machine-applicable
suggestions for the borrowck family (E0597/E0596/E0502/E0499/E0505/E0507/E0382)
and that `cargo fix` is "structurally blind" there. That claim is FALSE on 4 of
the 7 codes. It was refuted by the oracle on the plan's own filing day — the
same failure mode the plan was written to prevent (a confident narrative with no
oracle in the loop).

This issue holds the repro, makes the probe a standing test, and pins the
comparison gate that the correction forces.

## Measured (2026-08-21, rustc 1.93.0 stable, `cargo fix --broken-code`)

| Code | suggestion spans | applicability | `cargo fix` edit | territory |
|---|---|---|---|---|
| E0597 (true local lifetime) | 0 | — | none | uncontested |
| E0499 (two `&mut`) | 0 | — | none | uncontested |
| E0502 (borrow + `&mut`) | 0 | — | none | uncontested |
| E0382 | 1 | MachineApplicable | `let w = v.clone();` | contested |
| E0505 | 1 | MachineApplicable | `let r = &v.clone();` | contested |
| E0507 | 1 | MachineApplicable | `s.v.clone()` | contested |
| E0596 | 1 | MachineApplicable | `let mut x = v;` | contested |
| E0373 (closure outlives) | 1 | MachineApplicable | `Box::new(move \|\| ...)` | contested |

Two details that matter more than the counts:

1. The **`Box<dyn Fn>`-captures-a-local shape** — the one Plan 019 lists as
   E0597 template 1 (`move` closure) — is diagnosed as **E0373**, not E0597,
   and `cargo fix` applies `move ` automatically. That template is demoted.
2. rustc's own suggestions are **semantics-changing**: `&v.clone()` for E0505
   produces a temporary whose borrow no longer aliases `v`. So the
   "never wrong by construction" pitch cannot be claimed on the contested set
   by refusing `.clone()` — only by ranking alternatives and naming the change.

## Repro

```bash
mkdir -p /tmp/e0_probe/src && cd /tmp/e0_probe
printf '[package]\nname="e0_probe"\nversion="0.0.0"\nedition="2021"\n[workspace]\n' > Cargo.toml
# src/lib.rs: one minimal reproduction per code —
#   E0382 let w = v; v.len()+w.len()
#   E0596 let x = v; x.push(1)
#   E0507 fn(s: &S) -> Vec<u8> { s.v }
#   E0502 let first = &v[0]; v.push(1)
#   E0499 let a = &mut *v; let b = &mut *v;
#   E0505 let r = &v; let w = v;
#   E0597 let r; { let x = 5; r = &x; }
#   E0373 Box::new(|| s.len()) returned as Box<dyn Fn>
cargo check --message-format=json    # count spans with suggested_replacement + applicability
cargo fix --broken-code --allow-no-vcs --allow-dirty && git diff
```

`--broken-code` is required (plain `cargo fix` refuses a non-compiling tree) and
is available on stable — no nightly, no `-Z`.

## Tasks

- [x] T1 Port the probe to `tests/premise_probe.rs`: assert the per-code
  suggestion counts on the pinned toolchain. A rustc bump that moves a code
  from uncontested → contested must FAIL, not silently void the product claim.
  **DONE** — 6 tests, all green on the repo pin (1.97.0) as well as the box
  default (1.93.0), so the split is not a one-toolchain artifact. Ships
  `src/domains/rustc_errors/{diag,probe}.rs` behind the retrieval-free
  `rustc_errors` feature (T0.3 satisfied at birth). Includes the
  fixture-mislabel gate that pins the E0597-vs-E0373 finding.
- [x] T2 G-floor harness (Plan 019 T6.9): head-to-head vs
  `cargo fix --broken-code` on the mutation corpus, reported per error code.
  **DONE** — `src/domains/rustc_errors/floor.rs` (`CompileFixer` seam,
  `CargoFixBaseline`, `NullFixer`, known-inverse mutation corpus) +
  `examples/rustc_errors_floor.rs` + `tests/floor_corpus_integrity.rs`.
  **Measured (Bench 041): floor = 58.3% over 12 mutants, bimodal along the
  territory line — 0% on all 3 uncontested codes, 100% on all 5 contested
  ones.** Of 8 contested repairs, 7 reproduced the known inverse EXACTLY;
  the lone divergence is E0505's `&v.clone()` (aliasing silently changed
  under a MachineApplicable label). The `--fix-compile` arm is not measured
  because Plan 019 Phase 1 is unimplemented — the seam is final, so landing
  the driver is an arm swap.
- [ ] T3 Driver delegation: for any contested code where the floor is not
  beaten, `--fix-compile` delegates to `cargo fix` instead of shipping a
  template. No template authored ahead of a measured win. **Blocked on Plan
  019 Phase 1 (no driver yet). Bench 041 already decides the policy: E0596,
  E0382, E0507, E0373 have a 100%-at-0%-divergence floor and get NO
  template — delegate. E0505 is the one contested code with an opening.**
- [ ] T4 Re-measure on toolchain bumps. The contested set only grows.
  Standing, not schedulable: `tests/premise_probe.rs` fails on the bump, which
  IS the reminder. `rust-toolchain.toml` pins 1.97.0 across the stack, so a
  bump is already a deliberate reviewed commit — re-run Bench 041 in it.

## Close condition

T1+T2 landed, and every contested code either has a measured win (template
ships) or a recorded loss (delegation ships). Remove the drift row + this file
per the noise-reduction rule when closed.
