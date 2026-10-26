# Plan 299 — Meta-Blueprint Phase A: instrument + report (proposal 024 / issue 178)

**Status:** In progress — claimed 2026-10-27 (zed-amo-178-phaseA, lease `804a75fe`); scope = Phase A ONLY (A1–A5). Phase B/C are gated on this phase's volume read + owner go — NOT this plan.

Implements [Proposal 024](../.proposals/024_healer_meta_blueprint.md) Phase A: the always-on-with-demote
blueprint-history instrument + `--blueprint-report`. No actuator change; the history row append is the
sole declared side-effect (G1). No feature gate — plain fs + serde_json + blake3 (all default deps).

## Design pins (from the proposal's verdict + reviewer rounds)

- Lane: ON by default, demote-only env `RIIR_REFINE_META_BLUEPRINT=0` (the exact literal; env wins).
- Campaign = a `--fix` run (the heal campaign). `--fix-compile` and suggest-only runs do NOT append (v1;
  recorded in the schema docs — different outcome vocabulary, Phase B may add the row mode).
- Row: v1 schema — `v, ts, digest, mode, arm("incumbent"), posture{verify, verify_args, scoped_recheck,
  repo_posture, fmt, domain, top_k}, findings{total, by_domain, fixable_clippy}, outcomes{files_written,
  edits_written, verify_ran, kept_files, kept_edits, reverted, checks, skipped_checks, declined},
  strata{rule → {written, reverted, declined}} capped at 32 entries by count` — one bounded line.
- Digest: BLAKE3 over (mode, posture, sorted relative paths), truncated 16 bytes hex — groups re-runs of
  the same campaign content; NOT a dedup key (re-runs are separate campaigns).
- `--blueprint-report` exists regardless of any feature, only READS; missing history = exit 1 (never a
  green zero); empty history = honest `runs: 0`, exit 0.
- Scoring is arm-invariant by construction: Phase A has ONE posture (the incumbent); the report's
  posture↔verdict audit split exists so a future arm can never win by blinding the judge — the audit
  makes verify-posture-vs-verdict visible on every report run.
- Volume close (pre-registered): 8 weeks after the FIRST history row, `total < 16` OR no lever class
  with ≥ 6 campaigns in each of ≥ 2 arms ⇒ `META-VOLUME-CLOSE` (lane closes; report + suggestions
  survive). Past the window the line (or its explicit not-tripped `META-VOLUME-OPEN` form) prints on
  EVERY report run — cannot-silently-not-fire. Before the window: no line.
- Demotion-suggestion join: necessity history (`.benchmarks/necessity_history.jsonl`) rows with verdict
  `DECORATIVE`/`NO-EVIDENCE` (disabled arm) → report-only `demote-suggest:` lines naming the exact
  `--active-set-demote <domain>:<rule>` command; rules already in the active set are skipped; the join
  never auto-executes (owner-gated surface).

## Tasks

- [x] Claim the issue-178 Phase A lease + push (`804a75fe`)
- [x] A1 `src/blueprint.rs`: row schema + `campaign_digest` + `append_row`/`maybe_append_row` (env gate) + run-end append wired in `src/bin/heal.rs` (reverted-per-rule hoisted from the verify block)
- [x] A2 `--blueprint-report`: flag + standalone-mode row + dispatch + render (run count/span, posture↔verdict audit split, lever splits, strata table)
- [x] A3 necessity-bench → demotion-suggestion join (report-only) + the volume-close line (8 weeks / 16 campaigns / ≥6-per-arm × ≥2-arm thresholds as consts)
- [x] A4 G1 tests: append round-trip + bounded row; demoted env ⇒ zero writes (file never created); report determinism; malformed rows skipped-and-counted; missing history exit 1; write-failure is loud-on-stderr only (happy path prints nothing — stdout byte-identity by construction, pinned by the print-free `maybe_append_row` shape + the e2e)
- [x] A5 volume-close liveness tests: synthetic history below both thresholds at week 8 ⇒ CLOSE fires; sufficient volume ⇒ OPEN form; before the window ⇒ no line; zero rows ⇒ no line; past-window renders ALWAYS carry a `META-VOLUME` line
- [x] E2E (beyond A4/A5 units): scratch-crate campaign — missing history exit 1; a real `--fix` campaign appends exactly one bounded row (posture/findings/strata correct); demoted run appends nothing; a `--verify` campaign records `verify_ran=true checks=2 kept=1`; the necessity join renders the DECORATIVE suggestion, excludes LOAD-BEARING, counts the malformed line loud
- [x] Docs: AGENTS.md CLI-contract bullet + proposal 024 Phase A checkboxes + issue 178 lease closed
- [ ] Rebuild the maintained binary at the release feature set + `cargo refine --version` build hash == HEAD (the Issue-153 law — healer source touched)
- [x] Commit + push; reshape the issue-178 lease line to non-claim form; mark plan tasks complete in the same commit

## Notes

- Sibling-collision handling: the riir-auth 175/179 lane landed heal.rs hunks (the `run_kat_account` provider-binding block + the kat_info module) mid-flight; the landing commits **this session's blob only** (HEAD + blueprint hunks, the `git hash-object`/`update-index` path) — the sibling's working-tree hunks stay unstaged and untouched.
- Bin-test posture note: the bin test harness needs `kat_transport` in the feature set (the tuna `E0425`s at a non-release posture are pre-existing, not this lane's).
- 15 blueprint lib tests at default + 16 at `score_bench` (the wire-mirror drift guard); full lib suite 2749 passed / 0 failed; `cargo clippy --all-targets -- -D warnings` green; the standalone-mode matrix pins `--blueprint-report`.
