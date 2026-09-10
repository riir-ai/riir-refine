# Issue 087 — `kat_burn_stack` e2e: the burn auto-report returns `Skipped` at clean HEAD (3 tests red) — suspected account-plane regression from the auto-login/dir-move era

**Status:** OPEN — filed 2026-09-10 by the Issue 080/081/085 fix session (found during Issue 080 validation; NOT caused by that diff).

## The finding

`cargo test --test kat_burn_stack` fails **3 tests (0 passed)** at clean HEAD
`fcc478c`. Failure shape: the run bills and exits 0, but the burn
auto-report never fires — no `burn sync:` line, the mock server receives
zero requests (`run_burn_sync` returns `Skipped`, i.e. the account/key load
fails or a skip leg trips before any dial).

Reproduction evidence (Issue 080 validation, 2026-09-10): identical 3
failures reproduced at clean HEAD `fcc478c` in an isolated worktree
(sibling repos intact; worktree + `/tmp` target dir removed after), so the
regression is in COMMITTED state, not in any working-tree WIP. Suspected
root cause sits in the recently committed account-plane changes (Issue-081
account-dir move / the auto-login era, `--mine` auto-login + auto-sync
contract from the 2026-09-08/09/10 owner mandates).

## Why it matters

`kat_burn_stack` is the only e2e that exercises the burn auto-report stack
(warn-and-pay → cumulative book → `wire_kat_total` conversion → signed
push). Issue 081's incident class (symmetric mocks, asymmetric units) is
exactly what this target exists to catch — with it red, the burn wire has
no executable cross-boundary check until repaired. Note the sibling target
`kat_soft_gate_cli` (same binary+env harness) is fully green (13/13), which
localizes the break to the burn-report leg, not the harness.

## Suggested next step

Reproduce, then bisect the account-plane commits (the account-dir move and
`ensure_logged_in` inline chain are the prime suspects) for the load
failure that makes `run_burn_sync` skip; fix at the root, not by relaxing
the test.
