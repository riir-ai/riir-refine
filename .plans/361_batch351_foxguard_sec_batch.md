# Plan 361 — Batch 351: foxguard sec transcription batch

**Status:** IN FLIGHT — staged distill landed (Research 394, YES B−); the cargo half awaits a free amo window (Exclusive sr1197 job live at staging time; Plan-337 law held — this plan's build-dependent tasks start only in a quiet window).

## Source

`0sec-labs/foxguard` @ `fbeab16e1eb0dd38c6b6ddd451415495aca2b7d8` (v0.14.1, MIT OR Apache-2.0) — Rust SAST, 245 built-in rules, 15 languages. Candidates + dedup evidence + quotes: Research 394.

## Tasks

- [ ] T0: re-clone `.raw/foxguard` at the pin; byte-verify every quoted line in Research 394 against the clone (the five detector bodies: `rust_lang.rs` NoSqlInjection + PqVulnerableCrypto + NoCommandInjection, `go.rs` NetHttpNoTimeout, `javascript.rs` NoUnsafeRegex, `python.rs` NoOpenRedirect, `pq.rs` boundary matching); `py ../katgpt-rs/scripts/dual_allocation_gate.py` from riir-refine cwd.
- [ ] T1: dedup re-greps at landing HEAD (six vocabularies per Research 394's table: value-position sql, quantum/pq, redos/backtrack, read-timeout/slowloris, redirect, command-new-program) — zero unexpected owners.
- [ ] T2: authoring — R1 `sql-query-value-interpolation-unbound` (B+ lead; MANDATORY in-rule adjudication vs `sql-identifier-interpolation-unallowlisted`, both directions), R2 `quantum-vulnerable-primitive-selection` (the PQ family seed; skip-list + migration table + `crypto_token_in_path` boundary law), R3 `regex-backtracking-engine-untrusted-pattern` (the regex-crate-linear anti-regime IS the decision content), R4 `server-connection-without-read-timeout` (+ the client-side face; adjudication vs `untrusted-keyed-state-unbounded` in-rule), R5 `open-redirect-unvalidated-target`, R6 `command-new-untrusted-program-path` (face-or-drop gate vs `shell-string-command-interpolation`). Faces: weak-hash use-site-only rider + jwt decode-without-verify (face-or-drop).
- [ ] T3: fixtures — `b351_fixtures.rs` standalone-compiled AND executed BEFORE first corpus edit (the four-arm discipline; the R1 fixture must include the trusted-constant anti-regime arm and the identifier-vs-value disambiguation pair).
- [ ] T4: gates at the isolated `CARGO_TARGET_DIR=/tmp/b351_target` — default/sec/no-default/all-features lib + clippy -D on touched files ×4 postures + retrieval floors (`retrieval_precision_regression_gate` + the Layer-5b2 default-posture lane) + fmt on touched files.
- [ ] T5: `scripts/pins_manifest.py --write` WITH the corpus change same-commit; score-bench run appended; release-set binary NOT owed (sec ∉ RELEASE_FEATURES — verify build_stamp at landing).
- [ ] T6: Research 394 status line → landed; AGENTS.md sec rule-count + latest-batch row; `.docs/11_domains/` row; snapshot landing row (this file's Status → LANDED with the commit hash); home-firing evidence rows for R1/R2/R4 collected during T4.
- [ ] T7: ONE `feat:` commit + push; `rm -rf .raw/foxguard`.

## Defers

- `- [-]` none at staging.

## Notes

- The claim row lands in `.distill/001_mining_queue_snapshot.md` when the cargo half starts (the push is the arbiter; 48 h expiry).
- Zero GPU at every stage (sec fixtures are CPU-only) — but the GOAT-free sec batch still holds the Plan-337 build-discipline line: no cargo while a measurement saturates the box.
