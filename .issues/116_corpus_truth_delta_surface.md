# Issue 116 — Corpus-truth delta surface (compiled fix-lane `lint_set` vs leased suggest-lane corpus)
**Status:** OPEN — P2, unblocked. Filed 2026-09-16 from the room-to-improve triage (session record: HISTORY.md same-day entry).

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
- [ ] T1 — Delta counter at the suggest seam (lint ∈ compiled fix set ∧
      absent from leased corpus), printed only when non-zero.
- [ ] T2 — `--corpus-delta` per-lint table (report-only, exit unchanged).
- [ ] T3 — Fixture test: a lint in `lint_set` but not the leased set counts;
      a lint in both does not; empty delta prints nothing.

## Related
- Issue 113 (`[no corpus rule]` semantics), Issue 103 (sync payload v3
  `domain` field — the same leased-corpus staleness family, server side).
