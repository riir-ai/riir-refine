# Issue 116 — Corpus-truth delta surface (compiled fix-lane `lint_set` vs leased suggest-lane corpus)
**Status:** RESOLVED 2026-09-16 — T1-T3 landed (`src/corpus_delta.rs` + the suggest-lane summary line + `--corpus-delta`, 14 fixtures). Filed 2026-09-16 from the room-to-improve triage (session record: HISTORY.md same-day entry).

## Why

The two lanes read two corpus truths **by design**: the fix lane consumes the
compiled-in `lint_set` (which is why freshly-landed lints heal immediately),
while the suggest lane's leased corpus lags — a freshly-landed lint shows
`[no corpus rule]` in suggest while `--fix` already consumes it
(AGENTS.md: "the two lanes' corpus truths are different by design").

The design is right; the **discoverability** is the gap: users learn the
delta by switching modes, and `[no corpus rule]` reads as "not supported"
when it means "not leased yet". `[no corpus rule]` is documented as "a live
gap signal for the mining intake" — but nothing aggregates it.

## Shape of the fix

- Suggest-lane summary line: `N diagnostics whose lint IS in the compiled
  fix set but has no leased corpus rule yet` — the freshness delta, named,
  once per run (the provenance-strata line precedent: additive row,
  instrumentation not verdict).
- Optional `--corpus-delta` detail: the per-lint table behind that count
  (lint name, fix-lane rule id, leased-corpus absence), report-only.
- No behavior change to either lane's truth — this surfaces the seam, it
  does not close it.

## Honest caveats

- The delta is expected to be small and transient by design (lease cadence);
  the line must not become noise — print only when N > 0.
- Do NOT "fix" by making the lanes share one truth: the split is load-bearing
  (compiled lint_set is why the fix lane can consume lints the lease hasn't
  seen; the lease is why suggest ranks on live evidence).

## Tasks
- [x] T1 — Delta counter at the suggest seam (lint ∈ compiled fix set ∧
      absent from leased corpus), printed only when non-zero.
- [x] T2 — `--corpus-delta` per-lint table (report-only, exit unchanged).
- [x] T3 — Fixture test: a lint in `lint_set` but not the leased set counts;
      a lint in both does not; empty delta prints nothing.

## Related
- Issue 113 (`[no corpus rule]` semantics), Issue 103 (sync payload v3
  `domain` field — the same leased-corpus staleness family, server side).

## Landed (2026-09-16)

`src/corpus_delta.rs` (pure over its inputs, lib-tested at default features)
+ the run-summary line and `--corpus-delta` table in `bin/heal.rs`.

### The partition, and why it is a partition rather than a filter

The issue's shape asked for the freshness count. Building it made the
stronger framing obvious: **every `anchored_missing` lint lands in exactly
one of two buckets, and `[no corpus rule]` means the opposite thing in
each.**

| bucket | what the marker means | the action |
|---|---|---|
| in the compiled fix set | "not leased **yet**" | `cargo heal --fix` rewrites it today |
| in neither lane | "unsupported" | mining intake (Issue 119's territory) |

Reporting only the first would have been the flattering number and the less
useful one — a reader needs to know WHICH message they are being handed, and
the answer is per-lint. So the table reports both and the counts partition
(asserted: `delta_lints + unfixable == missing_lints`, same for spans).

### The tiers stay apart

`compiled_fix_set()` (Issue 119) unions FIXABLE ∪ VERIFY because a
coverage-debt figure must not move with the reader's feature flags. The
delta needs them **split**: a VERIFY row only applies under
`--fix --verify`, so collapsing them would send half the rows to an
invocation that will not touch them. `fix_lane_tiers()` is the split
sibling, and the summary line names the verify share (`(1 --verify tier)` /
`(all --verify tier)`). A lint in BOTH tables reports the LOOSER remedy —
`redundant_closure` is historically both, and sending a reader to a flag
they do not need is the same defect in the other direction.

### Never noise

`summary_line()` returns `None` on an empty delta, and the delta is EXPECTED
to be empty — it is transient by design, bounded by the lease cadence. It is
also silent when every missing lint is a genuine coverage gap, which is the
common case: a line firing on gaps would print on nearly every real run,
which is the cries-wolf instrument this repo's own docs warn gets ignored.

### ⚠ The freshness bucket is 0 BY CONSTRUCTION on a lease-less build

Measured by `--coverage-report` the same day: **fix-set-only = 0** — the
compiled fix set is a strict subset of the seed corpus, so with no lease
overlay there is nothing for the delta to find and **no end-to-end CLI run
can exercise that path**. That is not a defect; it is the seam being closed
whenever the lease is current.

It does mean every arm over hand-built input asserts only the abstraction.
`a_lagging_lease_produces_a_real_freshness_delta` drives the real producer
instead: a `DomainOverlay` that REPLACES clippy's seed with one unrelated
rule (exactly a lagging lease), a real `needless_return` anchor through
`suggest_file_anchored`, and the resulting `anchored_missing` fed to
`delta()` against the real `FIXABLE_LINTS`. It asserts the precondition
(`rule_book("needless_return").is_none()`) so it cannot pass vacuously if
the overlay semantics change.

### Verified end to end

A probe crate with `needless_return` (in the corpus) + `must_use_candidate`
(in neither lane) through a real `cargo clippy --message-format=json` feed:
the gap bucket reports the one uncovered lint, the freshness bucket is
correctly empty, and the summary line is correctly silent. `--corpus-delta`
without a feed REFUSES with the precondition named rather than printing an
empty table — a flag that silently does nothing is worse than one that says
why.

lib 1632/1632 (+14) · heal_usage_flag_parity 3/3 · clippy clean on the
write-set.

### Non-goal held

The lanes still read two truths. The compiled fix set is why a freshly-landed
lint heals at once; the lease is why suggest ranks on live evidence. This
names the seam and prints "DIFFERENT BY DESIGN" on its own report so the
next reader does not file it as a bug.
