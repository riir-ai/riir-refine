# The silent-now gate — a `#![cfg]` target nobody names

> A whole-file `#![cfg(feature = "x")]` integration test with no
> `[[test]]` row and no invocation compiles to an **empty binary** and
> prints `ok. 0 passed`, exit 0 — byte-for-byte a real pass. The repo's
> own guard Layer 5d now fails on any test binary that ran zero tests,
> and Layer 5c executes every target at `--all-features` — but the row
> still matters: `required-features` is what makes the **exact-set**
> invocation (`cargo test --features x`, no superset) build the target,
> and the union a `--all-features` build supplies can mask an import the
> exact set forgot (the `required_features_build_audit` lesson, katgpt-rs
> Issue 513).

## The precedent (the twelfth instance of the class)

`tests/dist_gates.rs` landed 2026-09-10 with the dist domain (Issue 085)
as a whole-file `#![cfg(feature = "dist")]` target with **no row and no
invocation** — found by the workstation `cfg_gated_target_audit.py` sweep
(`silent_now=1, load_bearing=1`: the filename token `gates` says its green
IS evidence, and nothing had ever executed it). Closed same day: the
`[[test]]` row with `required-features = ["dist"]`, the exact-set proof
(`--no-run` names the executable, then `7 passed; 0 failed` at exactly
`--features dist`), and the audit re-run reporting SILENT-NOW 0. Issue
file, fix, and removal in one commit per the noise-reduction rule.

## The discipline

1. A `#![cfg]`-gated test file gets a `[[test]]` row **in the same
   commit** as the file.
2. Prove the row sufficient at the EXACT feature set: `cargo test --test
   <name> --no-run --features <set>` must NAME the executable (an
   artifact is evidence; silence is not), then run it once for the
   passed-count. Never a superset build.
3. The audit (`scripts/cfg_gated_target_audit.py`, katgpt-rs home) is the
   standing verdict: SILENT-NOW must be 0.
