# Issue 154 — boundary C3: the reflexer-vessel allowlist row is not machine-parseable (missing backticks)

**Status:** OPEN — found by the 170th boundary-guard full-workspace run (2026-10-08, this box): `✗ VIOLATION riir-refine → reflexer-vessel is NOT in riir-refine/BOUNDARY.md 'May depend on'`. The row EXISTS (landed with Plan 202 R4, 2026-10-08) — this is the 145th class: a contract-SHAPE error, not an undeclared edge.

## Root cause

`ci_boundary_contract.sh`'s `allowlist()` recognises a column-1 crate name two ways only:

1. the workspace family convention — `^(riir|katgpt|neuron)[a-z0-9-]*$`;
2. **backticked** — the documented form for non-family names ("the backtick form is how a non-family dep declares itself … without letting prose in this cell silently widen the allowlist").

The Plan-202-R4 row spells column 1 bare (`| reflexer-vessel |`), which matches neither → the measured edge `riir-refine → reflexer-vessel` reads as undeclared and the workspace gate exits 1.

## The fix (one cell)

Backtick the cell: `` | `reflexer-vessel` | `` — no prose change, the allowlist extraction then prints `reflexer-vessel` and C3 goes green. Filed before the fix per the boundary protocol (issue-before-fix, even trivially fixable).
