# Issue 035 — `cargo fix --broken-code` is the floor for `rustc_errors`; premise probe + repro

**Date:** 2026-08-21
**Status:** OPEN — blocks Plan 019 Phase 3 (contested codes) and the product claim
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

- [ ] T1 Port the probe to `tests/premise_probe.rs`: assert the per-code
  suggestion counts on the pinned toolchain. A rustc bump that moves a code
  from uncontested → contested must FAIL, not silently void the product claim.
- [ ] T2 G-floor harness (Plan 019 T6.9): head-to-head vs
  `cargo fix --broken-code` on the mutation corpus, reported per error code.
- [ ] T3 Driver delegation: for any contested code where the floor is not
  beaten, `--fix-compile` delegates to `cargo fix` instead of shipping a
  template. No template authored ahead of a measured win.
- [ ] T4 Re-measure on toolchain bumps. The contested set only grows.

## Close condition

T1+T2 landed, and every contested code either has a measured win (template
ships) or a recorded loss (delegation ships). Remove the drift row + this file
per the noise-reduction rule when closed.
