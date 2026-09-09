# Issue 086 — `tests/dist_gates.rs` is a load-bearing SILENT-NOW target: no `[[test]]` row, no guard invocation, never executed

**Status:** OPEN — three-line fix owed; found by the workstation
`cfg_gated_drift_sweep.py` red the moment katgpt-rs's stale `max_silent_now`
pin-drift was cleared (2026-09-10). Not fixed by the filer: this repo has
three live sessions and the 085-dist lane is one of them — the row belongs
in that lane's next commit.

## The finding

`tests/dist_gates.rs` (landed `79dcf9b`, Issue 085's dist domain) opens
`#![cfg(feature = "dist")]` as a whole-file gate and has **no `[[test]]`
row** in `Cargo.toml` and **no invocation** in `scripts/ci_feature_guard.sh`
or any workflow. A plain `cargo test --test dist_gates` compiles the file to
an **empty binary** and prints `ok. 0 passed`, exit 0 — byte-for-byte a real
pass (the green-zero rule, katgpt-rs AGENTS.md §"cfg-gated targets").

Measured by `scripts/cfg_gated_target_audit.py ../riir-clippy`:
`silent_now=1, load_bearing=1` — the filename token (`gates`) says its green
IS evidence, and nothing has ever executed it. This is the "gate nobody
runs" class this repo's own AGENTS.md §"Gate Coverage" documents as found
eleven separate times — this is the twelfth instance, and unlike issue 105's
class in riir-mmorpg-examples, no CI verdict was even being laundered:
nothing names this target at all.

## The fix (the audit prints it verbatim)

```toml
[[test]]
name = "dist_gates"
required-features = ["dist"]
```

Plus one of:

- a `ci_feature_guard.sh` row that RUNS it at `--features dist` with a
  passed-test count pin (the riir-mmorpg-examples issue 105 pattern:
  floors, not exact pins, per target), or
- an explicit note in the guard why it stays unexecuted (it should not stay
  unexecuted — these are the domain's own G-gates).

Verification discipline (issue 105's): prove the row SUFFICIENT at the
EXACT feature set — `cargo test --test dist_gates --no-run --features dist`
must name the executable (an artifact is evidence; silence is not), then run
it once for the passed-count. Never a superset build.

## Close condition

Row landed + guard row green (or the reasoned unexecuted note), the drift
sweep's riir-clippy row reports `silent_now=0`, this file removed per the
noise-reduction rule with the record at `.docs`/AGENTS.
