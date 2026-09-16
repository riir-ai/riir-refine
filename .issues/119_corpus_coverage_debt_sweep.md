# Issue 119 — Corpus coverage-debt sweep (report-first: clippy's live lint surface vs the clippy_lints corpus)
**Status:** OPEN — P1, unblocked. Filed 2026-09-16 from the room-to-improve triage (session record: HISTORY.md same-day entry).

## Why

`[no corpus rule]` (Issue 113) is a live gap signal, but it only fires on
diagnostics a real run happens to hit — nobody has quantified the corpus's
coverage of clippy's **full** lint surface. The clippy_lints domain carries
54 corpus rules; the installed clippy ships several hundred lints
(style/complexity/perf/pedantic/nursery/correctness groups). The delta is
the corpus's coverage debt, and it is currently invisible until a user hits
it.

The repo's own history says frequency-ranked intake wins: the Issue 016
expansion was "frequency-ranked from kornel's 100K-crate clippy run + Li et
al.'s 8.09M-warning crates.io study ∩ clippy's `MachineApplicable` axis ∩
parse-only provability" (Bench 021). A live sweep re-derives that ranking
against the CURRENT toolchain instead of a 2026 study snapshot — the
`cargo_comment_audit`/bench_doc_audit precedent: derived population, never
hand-typed.

Secondary output: the same report sizes the thin-domain question (`sec` = 15
rules, `dist` = 6) the same way, and feeds staged-charter rows into
`.distill/001` §post-mining intake (the Batch-146 accumulation precedent:
stage until a second input arrives).

## Shape of the fix

- Report-only CLI (`--coverage-report` or a `scripts/` sweep, either fits the
  frontier-report law): derive the live lint list from the INSTALLED
  toolchain (`clippy-driver -W help` output), diff against corpus `rule_id`s
  + the fix lane's compiled `lint_set`, print the uncovered table grouped by
  lint group with the MachineApplicable/restriction axes called out.
- Floors, never pins: min corpus size, min derived lint count — a green zero
  over a blind walk is the failure the frontier-report law forbids
  (`refuses exit 1 on empty` posture).
- NO auto-authoring: output is staged-charter rows for the mining queue,
  adjudicated by the existing batch discipline (dual-evidence, dedup grep).
- Per the repo's dedup law: the sweep MUST dedup against the FULL corpus
  (clippy_lints + rust_perf + kernel_opt) before staging — the Batch-146
  `dispatch-pass-fusion` near-miss precedent.

## Honest caveats

- The derived list is toolchain-version-dependent by design — that is the
  point (coverage debt moves as clippy ships lints); print the toolchain
  version on the report header.
- Restriction/legacy lints will dominate the raw count; the report's value
  is the MachineApplicable ∩ high-frequency intersection, so the frequency
  axis needs either the historical studies (cite as fixed evidence) or a
  local corpus-run histogram (the Bench-061 fired-histogram precedent) —
  state which one each row uses.
- This gates nothing and changes no verdicts — report-first (R135/Bench 047).

## Tasks
- [ ] T1 — Derivation: live lint list from the installed clippy driver +
      toolchain version stamp; population floors; refuses-green-zero.
- [ ] T2 — Diff vs clippy_lints corpus `rule_id`s + compiled `lint_set`;
      grouped table with applicability axes; dedup against the full
      multi-domain corpus.
- [ ] T3 — Frequency axis: join the fired-histogram evidence (Bench 061
      class) where it exists; mark the rest study-cited or uncited, never
      invented.
- [ ] T4 — Stage the top rows into `.distill/001` §post-mining intake as
      charter-input candidates (accumulation rule: never batch-justifying
      alone).

## Related
- Issue 113 (`[no corpus rule]` semantics), Issue 116 (the leased-vs-compiled
  freshness seam — different axis: freshness, not coverage).
- `.benchmarks/021` (the frequency-ranked intake precedent), Bench 061 (the
  fired histogram), the frontier-report law (R135/Bench 047).
