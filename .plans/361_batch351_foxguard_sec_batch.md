# Plan 361 — Batch 351: foxguard sec transcription batch

**Status:** LANDED 2026-11-12 — T0–T7 all DONE in the amo-idle-i361-cargo lane (the lease folds on landing; the landing commit `15cbe3a5` carries the record). Yield arc: the sibling amo-zed-b351 lane claimed 9074fb0a first, yielded unpushed to the later-pushed claim (e4289cff, the push-is-the-arbiter law) and recorded the correction 26ba5a93+2a1a51de.
**Lease released:** amo-zed-b351@amo yielded unpushed-code within minutes of its 9074fb0a claim — the later-pushed claim (amo-idle-i361-cargo, e4289cff) is the arbiter and holds the batch (the push-is-the-arbiter law; provenance in prose, no claim-shaped residue).

## Source

`0sec-labs/foxguard` @ `fbeab16e1eb0dd38c6b6ddd451415495aca2b7d8` (v0.14.1, MIT OR Apache-2.0) — Rust SAST, 245 built-in rules, 15 languages. Candidates + dedup evidence + quotes: Research 394.

## Tasks

- [x] T0: re-clone `.raw/foxguard` at the pin; byte-verify every quoted line in Research 394 against the clone (the five detector bodies: `rust_lang.rs` NoSqlInjection + PqVulnerableCrypto + NoCommandInjection, `go.rs` NetHttpNoTimeout, `javascript.rs` NoUnsafeRegex, `python.rs` NoOpenRedirect, `pq.rs` boundary matching); `py ../katgpt-rs/scripts/dual_allocation_gate.py` from riir-refine cwd.
  - DONE 2026-11-12 (amo, the T0/T1 read-only lane — amo Exclusive sr1197 held; zero cargo): clone at `fbeab16` (v0.14.1 tag) verified; every quoted detector fact byte-lands — `format_has_interpolation` rust_lang.rs:104 + NoSqlInjection :287 + the FP-refinement comment :315-316 + `crypto_token_in_path` :127 + the boundary comment :360 + "Only match direct calls (scoped_identifier" :407 + the weak-hash use-site-only + complete-`::`-segment law :356-361 + go.rs slowloris :645/:655/:661 + js `js_redos_re` :32 + NoUnsafeRegex :1598 + py url_for/reverse safe-half :1646-1650 + js/jwt-decode-without-verify :2225-2257 + PQ skip list ml_kem pq.rs:100 / ml_dsa :116 / slh_dsa :132 / fn_dsa :151 / hqc :169 + CNSA 2.0 :9/:286 + X25519MLKEM768 :26/:59 + FN-DSA/Falcon :24/:142-149; dual_allocation 0/0/0.
- [x] T1: dedup re-greps at landing HEAD (six vocabularies per Research 394's table: value-position sql, quantum/pq, redos/backtrack, read-timeout/slowloris, redirect, command-new-program) — zero unexpected owners.
  - DONE 2026-11-12 same lane: PQ 0, ReDoS 0, open-redirect 0, decode-without-verify 0; timeout = the 2 reuseaddr probe-dial hits (as recorded); command::new = the 7 shell-layer/argv fixture hits (as recorded); SQL value-position = 3 hits ALL inside B350's identifier rule (concept + buggy + fixed fixture strings — the research recorded 2 at verdict time, the third is the same rule's fixed_code, same axis, still clear).
- [x] T2: authoring — R1 `sql-query-value-interpolation-unbound` (B+ lead; MANDATORY in-rule adjudication vs `sql-identifier-interpolation-unallowlisted`, both directions), R2 `quantum-vulnerable-primitive-selection` (the PQ family seed; skip-list + migration table + `crypto_token_in_path` boundary law), R3 `regex-backtracking-engine-untrusted-pattern` (the regex-crate-linear anti-regime IS the decision content), R4 `server-connection-without-read-timeout` (+ the client-side face; adjudication vs `untrusted-keyed-state-unbounded` in-rule), R5 `open-redirect-unvalidated-target`, R6 `command-new-untrusted-program-path` (face-or-drop gate vs `shell-string-command-interpolation`). Faces: weak-hash use-site-only rider + jwt decode-without-verify (face-or-drop).
- [x] T3: fixtures — `b351_fixtures.rs` standalone-compiled AND executed BEFORE first corpus edit (the four-arm discipline; the R1 fixture must include the trusted-constant anti-regime arm and the identifier-vs-value disambiguation pair).
- [x] T4: gates at the isolated `CARGO_TARGET_DIR=/tmp/b351_target` — default/sec/no-default/all-features lib + clippy -D on touched files ×4 postures + retrieval floors (`retrieval_precision_regression_gate` + the Layer-5b2 default-posture lane) + fmt on touched files.
- [x] T5: `scripts/pins_manifest.py --write` WITH the corpus change same-commit; score-bench run appended; release-set binary NOT owed (sec ∉ RELEASE_FEATURES — verify build_stamp at landing).
- [x] T6: Research 394 status line → landed; AGENTS.md sec rule-count + latest-batch row; `.docs/11_domains/` row; snapshot landing row (this file's Status → LANDED with the commit hash); home-firing evidence rows for R1/R2/R4 collected during T4.
- [x] T7: ONE `feat:` commit + push; `rm -rf .raw/foxguard`.

## Defers

- `- [-]` none at staging.

## Notes

- The claim row lands in `.distill/001_mining_queue_snapshot.md` when the cargo half starts (the push is the arbiter; 48 h expiry).
- Zero GPU at every stage (sec fixtures are CPU-only) — but the GOAT-free sec batch still holds the Plan-337 build-discipline line: no cargo while a measurement saturates the box.
