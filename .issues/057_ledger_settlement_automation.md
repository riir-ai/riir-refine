# 057 — ledger↔analytics settlement is a hand-append process; every new account day drifts "100% unreconciled"

**Status:** RESOLVED — landed with the filing commit (T1–T5 all DONE same day, 2026-08-29).

Found 2026-08-29 by reading the live dashboard: the reconciliation card showed
`2026-08-28 · ledger 0.00 · analytics 132.84 · DRIFT · 100.0% unreconciled`
while 08-26 and 08-27 read RECONCILED.

## Root cause (measured, not guessed)

The Issue 044 T12 reconciliation card compares ledger-per-day neurons against
CF GraphQL billing truth — but **nothing ever writes the settlement rows**.
They were appended BY HAND (`7bdb710`, `83d853c`). Four measured facts make
the drift structural, not incidental:

1. **No automation settles a completed day.** `gen_dashboard.py` renders the
   card and prints "append a settlement row" — the action item is aimed at a
   human, forever.
2. **The launchd nightly was never bootstrapped** (`launchctl print
   gui/$(id -u)/com.riir.ops-nightly` → not found; AGENTS.md listed it
   owner-gated-remaining) — and **even if it were, `ops_nightly.sh` has no
   settlement step at all.** Installing the cron alone would NOT have fixed
   the 08-28 row.
3. **`.benchmarks/llm_cost_ledger.jsonl` is not in the nightly's ARTIFACTS
   commit list** — an automation that appended a settlement row would leave
   the ledger permanently dirty (the exact class the ARTIFACTS comment
   documents for `corpus_history.jsonl`), and a later `git checkout` would
   silently drop settled rows, to be re-appended as duplicates.
4. **Analytics retention is ~1–3 days on this account** (044 Notes) — an
   unsettled day's server truth expires, converting recoverable drift into a
   permanent `ledger only · outside analytics retention` row.

Direction check that classified the finding: ledger 0.00 vs analytics 132.84
means the settlement row is ABSENT (the card's own named remedy), not the
Issue 044 T12 mis-bucketing class (wrong-window rows) — that defect is fixed
(`eff_ts` / `covers_unix`) and its guard held.

## Fix

- **T1** `gen_dashboard.py --settle` — for every COMPLETE UTC day (strictly
  before today; the current day is still growing) whose ledger attribution
  disagrees with GraphQL beyond the card's own tolerance (0.5%, floor 1 n),
  append ONE settlement row: `covers_unix` = the settled day's UTC midnight
  (the T12 rule — never bucket by append time), `arm: "settlement"`,
  `server.neurons` = analytics − ledger-attributed (sign carried honestly;
  a negative row is a legitimate correction), `model:
  account/unattributed-aggregate` (aiInferenceAdaptiveGroups has NO model
  dimension — probed `7bdb710`; per-model attribution is not derivable).
  Runs BEFORE `build_data` so the regenerated dashboard shows RECONCILED.
- **T2** Idempotency: settled-day detection reads the LEDGER FILE ITSELF
  (`arm ∈ {settlement, reconciliation}` by `eff_ts` day), so an
  appended-but-uncommitted row still blocks a duplicate on the next run; the
  nightly's commit only makes it durable. Days already settled (the two T12
  hand rows) are skipped.
- **T3** `ops_nightly.sh`: gen invocation gains `--settle`; the ledger joins
  ARTIFACTS (explicit-path commit only — never `git add -A`); commit message
  names the settlement.
- **T4** The launchd nightly is BOOTSTRAPPED on the M3 (the two commands the
  plist header documents, previously owner-gated-remaining) — settlement now
  runs daily without a session. `launchctl print` verified post-install.
- **T5** The standing 2026-08-28 drift settled by an immediate manual run
  (`--settle`): the 132.84 n day reconciles in the same session instead of
  waiting for the first scheduled run.

## Honest scope

- Settlement granularity is the account DAY. The two T12 hand rows used
  hour-bucket attribution because a human knew the session window; the
  automated row cannot (no model/hour dimension derivable beyond the day
  total the card itself consumes). The per-model server cards still read 0
  until a per-model analytics source exists — the same 7bdb710 caveat,
  unchanged.
- No CF token → settlement logs and skips; the card keeps reporting drift
  honestly rather than papering over it.
- The CI workflow (inert, T7) keeps plain `--sweep` — settlement is
  local-first by design (the ledger is committed from the M3, the T7
  rationale).

## Verification

- Live run 2026-08-29: settlement appended for 2026-08-28; regenerated card
  reads RECONCILED; idempotent second run appends nothing.
- `verify_dom` PASS on the regenerated HTML.
