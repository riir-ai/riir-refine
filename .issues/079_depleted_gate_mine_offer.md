# Issue 079 — the depleted refusal offers no choice: prompt the mine-and-sync path (y/N) instead of a flat exit 1

**Status:** OPEN — owner-directed UX (2026-09-09, live session); fix lands in the same commit as this file

## The finding

The 075 soft gate refuses a depleted, never-contributed account with a
message that NAMES the contribute path but does not OFFER it — the user
must read the message, open a second terminal, run `--mine` + `sync`
themselves, and re-run. The owner's ask (live session, m3 at balance 0):

> it should ask user to run --mine and prompt y/n when out of KAT and try
> to run --fix

Secondary finding from the same session (operational, resolved here):
**the gate had never fired on m3 because the global binary predated
`de729c9`** — the last burn push (~01:15) wrote a v1 ack (no
`last_balance_micro`), and the accepted `sync` (stale `Ok` arm) wrote no
`contributed_once.json` marker. Server truth all along:
`GET /balance?acct=c79c…` → `{"balance_micro":0}` and the crank
`skipped_treasury:1` / `needs_alert:true` (no auto-refuel until the
treasury is funded — riir-dapps 025's owner decision still pending).
Lesson: a rebuilt-same-day binary is not the same thing as a binary that
contains the day's commits — the mtime lied by two hours.

## The fix (landed with this file)

`enforce_kat_soft_gate` now offers the contribute-now path BEFORE the flat
refusal:

1. On `Refuse`, ask once: `out of KAT — contribute mining data now to
   lift the gate? [y/N]`.
   - `RIIR_HEAL_KAT_PROMPT_YES=1|true|yes` — the automation hook (agents
     and CI run without a TTY; this workspace is mostly agents).
   - An explicit `--unmine` opt-out is never re-asked — it already
     answered.
   - No env and no TTY → no offer; the refusal is byte-identical to the
     075 shape (CI/pipe behavior unchanged).
2. Yes → run `run_kat_sync()` inline (the burn push + the batch push +
   the lease pull). One ACCEPTED push writes the machine-global
   `contributed_once.json` — the 075 sticky grace — so the re-evaluation
   (re-reading only what the sync can change: the marker + the v2 ack's
   server balance) lifts the gate and the run proceeds.
3. Still refused (nothing queued to push, or the push failed) → the
   refusal prints with one extra honest line pointing at the sync output;
   top-up remains the path. An empty outbox cannot mint a contribution —
   by design (the marker is keyed on an accepted batch).

## Deliberately NOT done here

- The 100M-KAT grant copy (`--mine` onboarding line 2, `kat_meter.rs`
  projection, front page) stays wrong until riir-dapps 025's owner picks
  an option — "do NOT close by picking silently".
- No auto-yes without consent: `Unset` consent + y-answered prompt IS the
  consent gesture (the documented "running sync IS the consent" rule,
  now with an explicit question first); `OptedOut` skips the offer.

## Tests

`tests/kat_soft_gate_cli.rs` grows two rows: the auto-yes path (offer
runs → closed port / no store → still refused, no marker written) and the
no-TTY negative (the offer must not fire without the env or a TTY).

## Related

- riir-clippy 075 (the gate), 076 (the burn push that carries the v2 ack).
- riir-dapps 025 (the 10⁶× grant mismatch; m3 refuel pending the owner
  decision; treasury funding gates every crank grant leg).
