# Issue 119 — Corpus coverage-debt sweep (report-first: clippy's live lint surface vs the clippy_lints corpus)
**Status:** RESOLVED 2026-09-16 — T1-T4 all landed (`cargo heal --coverage-report`, `src/corpus_coverage.rs`, 33 fixtures). Filed 2026-09-16 from the room-to-improve triage (session record: HISTORY.md same-day entry).

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
- [x] T1 — Derivation: live lint list from the installed clippy driver +
      toolchain version stamp; population floors; refuses-green-zero.
- [x] T2 — Diff vs clippy_lints corpus `rule_id`s + compiled `lint_set`;
      grouped table with applicability axes; dedup against the full
      multi-domain corpus.
- [x] T3 — Frequency axis: join the fired-histogram evidence (Bench 061
      class) where it exists; mark the rest study-cited or uncited, never
      invented.
- [x] T4 — Stage the top rows into `.distill/001` §post-mining intake as
      charter-input candidates (accumulation rule: never batch-justifying
      alone).

## Related
- Issue 113 (`[no corpus rule]` semantics), Issue 116 (the leased-vs-compiled
  freshness seam — different axis: freshness, not coverage).
- `.benchmarks/021` (the frequency-ranked intake precedent), Bench 061 (the
  fired histogram), the frontier-report law (R135/Bench 047).

## Landed (2026-09-16)

`cargo heal --coverage-report` — no paths, because the population is the
INSTALLED TOOLCHAIN. Arithmetic in `src/corpus_coverage.rs` (pure over its
inputs, lib-tested at default features, 33 fixtures); the driver spawn +
manifest read in `bin/heal.rs::run_coverage_report`.

**First measurement, clippy 0.1.98 (48a229ceae 2026-09-01):**

| | |
|---|---|
| derived lints | **822** in 9 groups |
| corpus-covered | 54 |
| fix-set-only | **0** |
| UNCOVERED | **768** |
| …of which fire by default | **449** |
| cross-domain collisions | **0** (over 173 other-domain rule ids) |

Per-group debt: style 136/158 · pedantic 132/143 · restriction 132/132 ·
complexity 127/143 · suspicious 83/83 · correctness 66/67 · nursery 50/53 ·
perf 37/38 · cargo 5/5.

### Two cross-checks nothing had measured

- **fix-set-only = 0** — every one of the 51 `FIXABLE_LINTS ∪ VERIFY_LINTS`
  names has a corpus rule. The corpus-lockstep law ("no corpus entry without
  a proposer") was only ever asserted in the other direction; this is the
  first measurement of the converse, and it holds.
- **0 name collisions** with the other five compiled domains (the Batch-146
  `dispatch-pass-fusion` near-miss law). Expected — those are kebab-case
  concept ids — but expected-and-unmeasured is what that law exists for.

### ⛔ The finding that outranks the candidate list

**0 of the 449 default-firing uncovered lints carry ANY frequency evidence.**
Every evidenced row in the ranked head is `allow`-level. The mechanism is the
evidence's own provenance: the Bench 061 histogram was captured with
pedantic+nursery ENABLED over repos this stack had already clippy-swept, so
it ranks the OPT-IN tail and is structurally blind to the default-firing
debt. Staging off the ranked head alone therefore stages opt-in lints,
deliberately or not. The report prints the cross-tab and says `⚠ DISJOINT`
rather than leaving a reader to compute it.

**Successor lead (T4's real output): a default-level fired histogram** — a
`cargo clippy` run with no pedantic/nursery over the sibling repos. It would
be the first evidence any of those 449 has ever had. Filed in `.distill/001`
§post-mining intake as a lead, NOT staged as a rule.

### T3 — the frequency axis, and what it refuses

Two sources, joined, never estimated:

- **local** — the Bench 061 fired histogram, with its capture date, source-
  repo count and dataset digest printed, plus a loud warning that it was
  captured under cargo 1.97 while the sweep runs 1.98.
- **study** — kornel's published 100K-crate top-10 (`.benchmarks/021`),
  transcribed WITH counts and restricted to the **3 rows in the `clippy::`
  namespace**. The other 7 are rustc lints; attributing their frequency to a
  clippy corpus gap is the transcription error the arm pins against.

A row carrying both is the dual-evidence tier the batch discipline stages
first, and **tier dominates count** — a 2-source row at 90 firings outranks a
1-source row at 1011, because the two are different populations (6 repos on
one day vs 100K crates) and summing them would invent a third number. A lint
neither source observed renders **`uncited`, not zero**.

Li et al. (ICSE 2024) is deliberately NOT transcribed: `.benchmarks/021`
cites it for the SHAPE of the distribution (top-5 > 80% of all warnings),
not a per-lint table, and inventing per-lint counts from a shape claim is
exactly what this axis exists to refuse.

### What is NOT derived, and says so

`MachineApplicable` is a property of an EMITTED diagnostic, not of a lint
declaration — no static sweep can report it. The report declares the axis
underivable and points at the `--clippy-json` oracle lane, rather than
printing a column it would have to invent.

### Floors

Three, failing differently, because a ceiling is green over whatever it can
SEE: `MIN_DERIVED_LINTS` (the `-W help` parse — a format change must RED, not
report a flattering small debt), `MIN_CORPUS_RULES` and `MIN_FIX_SET` (the
two things it is diffed AGAINST — an uncompiled `clippy_lints` would make the
debt read as total). Every one is armed by its own fixture.

### Collision note

A concurrent session implemented this issue in parallel as
`src/coverage_report.rs` (480 lines + CLI wiring) and deleted it unlanded
while this work was in flight. Its frequency axis asserted that no histogram
could exist for an uncovered lint ("nothing fired them through us") — that
premise is false and is the measured reason this implementation joins Bench
061: the histogram measures what REAL CLIPPY fires on our repos, and 26 of
its 30 rows are uncovered. The 4 that are covered are exactly the rows past
batches mined OUT of it (`use_self` + `suboptimal_flops` Batch 89,
`items_after_statements` + `manual_let_else` Batch 90) — i.e. it is already
the proven intake instrument for uncovered lints.
