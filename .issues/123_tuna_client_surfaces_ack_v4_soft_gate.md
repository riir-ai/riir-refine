# Issue 123 — TUNA trial-credit client surfaces: ack v4, the 075 soft gate's second plane, footer + `--info` readouts

**Status:** OPEN — client half LANDED 2026-09-18 (ack v4 + gate + footer + `--info`, all additive with byte-identical degradation pinned by tests); **server wire half LANDED 2026-09-18** (riir-dapps `90cfb0e` — Issue 097, resolved + removed under the noise-reduction rule; the record lives in the dapps HISTORY "Proposal 006 LANDED" section + [Proposal 006](../riir-dapps/.proposals/006_tuna_trial_credit_and_token_conditions.md) B2): `/burn` + `/history` carry the zipped `tuna_balance_micro`/`tuna_expires_unix` pair (spendable-only) + the `charged_tuna_micro`/`charged_kat_micro` split; the worker manifest now pins `kat_tuna`); **THE CLAIM FLIP LANDED 2026-09-18** (riir-dapps `c6d2e9c` + the coherence follow-up `789d18f`: `/account/claim` draws TUNA — D1 executed; `/tuna/genesis` ops route; `/balance` + `/history` `granted` report the unified claim predicate `grant_claimed`, so `ensure_grant_flow` + the welcome gate converge for trial accounts; **devnet has the reservoir — genesis run 2026-09-18T05:07:40Z, live claim/replay/balance/stats verified** `5e32d72`; apex + mainnet.ai.gist.rs code-only, claims 503 fail-closed until the owner runs `/tuna/genesis` there). **DEVNET IS LIT**: the fleet's machines point at devnet, so `/burn` acks now carry TUNA truth — the footer TUNA line + the gate's second plane render for any account holding trial credit, byte-identical everywhere else. **PLANE SPLIT LANDED 2026-09-18** (the last client-reachable item): `BurnReportAck` parses `charged_tuna_micro`/`charged_kat_micro` (defensive; absent = pre-B2) and composes `plane_split_note()` in the protocol leaf — `burn sync: drew 70.000000 TUNA of trial credit — KAT balance untouched` / `… TUNA + … KAT — trial credit exhausted mid-report; billing KAT now` (the both-positive wording IS the mid-report TUNA→KAT handoff, Proposal 006 Q1); the burn tail (`print_burn_sync`, so both the post-run auto-sync AND `cargo heal sync`'s drain) renders it when `Some` — a 0/0 replay, a pre-B2 server, and a KAT-only report all render nothing, keeping the one-line contract. **Deliberate surface decision:** the FOOTER run line stays the local session projection — the wire is whole-KAT cumulative, so the server's split names a REPORT (which may cover stacked deltas), not the session; the tail is the honest surface. The split is deliberately NOT persisted into the ack (a per-report fact, not position). Still open (owner-gated only): **light-up on apex/mainnet** (owner deploy + one genesis curl each — the TUNA bar-scale item below is RESOLVED by the fuel-gauge clamp: multi-tranche display caps at 100%, exact re-aim deferred until the server reports the real claim size on the wire); post-D1 grant-bar honesty RESOLVED BY REMOVAL (below), the run-line unit honesty RESOLVED (the run line bills the active tank — below), the tank-line brevity RESOLVED (fuel-gauge caption — below), and the depleted-run survey RESOLVED (count-only — below). **Fleet binary refresh + live verification (2026-09-18 evening, the owner's "no TUNA in --info / perfect 100 KAT" report):** the installed global binary predated the whole client half (built 03:03; these surfaces landed 10:19–16:09) — rebuilt at HEAD `af5e5e87` (release set complete, `--version` stamp) and installed; live `--info` on devnet then verified: no TUNA line for the fleet's KAT-grandfathered account (`c79c…5345`) is BY DESIGN (the claim flip refuses KAT-granted accounts, once-per-life across both planes → no spendable tranches → `/history` omits the zipped pair → `None` renders nothing); the line lights up for a fresh trial account (the `5e32d72` e2e). The same report surfaced the µ-sliver display floor (arm above) and two stale-truth reads worth knowing: the footer bar reads the REPO-LOCAL charge ledger, and the soft-gate line's `last synced balance` can carry the pre-D1-correction ack (99,999,998) until the next burn push or explicit `sync` records server truth.

## Why

Proposal 006 (riir-dapps, ACCEPTED 2026-09-18) ships TUNA — the trial
credit: heals only, non-transferable, expires. The owner asked how the
CLI surfaces it, and the review found one real bug in the plan of record:

**The 075 soft gate would refuse a trial user who is NOT depleted.** The
gate's predicate is KAT-only (`last_balance_micro ≤ 0` + never
contributed). Post-B3, `apply_burn` draws TUNA first — a trial user
holding 47 TUNA with a 0 KAT balance can still heal, but the gate reads
the KAT leg alone and refuses. That is the Issue-094 shape (a property
asserted where nobody checks it) caught before the plane exists.

## Landed (client half, additive, forward-compatible)

1. **Burn ack v4** (`riir-kat/src/kat_protocol_burn.rs` +
   `kat_burn_client.rs`): `BurnReportAck` + `KatBurnAck` gain
   `tuna_balance_micro: Option<i64>` and `tuna_expires_unix: Option<u64>`,
   parsed/written DEFENSIVELY (absent or wrong-typed → `None` — a
   pre-TUNA server keeps parsing). `BURN_ACK_VERSION` 3 → 4; the loader's
   upgrade arm widened to EVERY older version (all migrations since v1→v2
   were additive-field only; upgrading in place keeps the burn total and
   the gate's synced balance where the old reset dropped both). Pinned by
   the v4-parse arm + the real-v1-file-upgrades-silently arm.
2. **Soft gate second plane** (`riir-kat/src/kat_soft_gate.rs`):
   `SoftGateInput` gains `tuna_balance_micro`; a server-reported positive
   TUNA balance LIFTS the refusal. `None` is "no trial-credit truth" —
   never an invented zero — so pre-v4 acks keep the KAT-only posture byte
   for byte. The refusal prose names the exhausted trial credit when the
   server said so (`Some ≤ 0` is a fact, not an absence).
3. **Footer TUNA line** (`src/kat_footer.rs`): `TunaLine { balance,
   grant scale, expires, now }` renders a 4th line between grant and sync
   — `tuna  ⣿…  47.2 of 100 TUNA left · expires 2026-10-18 (12d) · trial
   credit — heals only` — only when the ack carries TUNA truth; `None`
   renders nothing (the 3-line footer, byte-identical, pinned by the
   existing shape tests). Shared date helper `ymd_from_unix`
   (civil-from-days, no chrono dep).
4. **`--info`** (`src/bin/kat_info/mod.rs`): the wallet view gains the
   `tuna:` position line (from `/history`, additive parse) and the
   soft-gate ALLOW line names the trial credit. Both degrade to nothing
   on a pre-TUNA deployment.

## Still open

- [x] **Light-up** — RESOLVED server-side (riir-dapps Issue 097, landed + live on devnet 2026-09-18): a B2 server sends `tuna_balance_micro` + `tuna_expires_unix` on `/burn` and `/history`, and this client parses them (ack v4). Apex + mainnet.ai.gist.rs light up with the owner deploy + genesis curl.
- [x] **Plane-split run line** — LANDED 2026-09-18 as the **burn-tail note** (`plane_split_note` in riir-kat's protocol leaf + the `print_burn_sync` render here, e2e-pinned by `tests/kat_burn_stack.rs::a_split_report_names_the_trial_credit_draw` against a B2 mock). Surface decision on record: the footer's run line stays the local session projection — the wire is whole-KAT cumulative, so the server split names a REPORT (possibly stacked), not the session; and KAT-only reports stay silent (the report line already names the KAT charge).
- [x] **Post-D1 grant-bar honesty** — **RESOLVED BY REMOVAL (owner call, 2026-09-18 evening, the "it's vice versa" report)**: the grant bar was a LOCAL meter projection draining KAT per token regardless of the server's actual debit (it showed a KAT burn during a replay no-op, while the server draws TUNA-first — B3 — so the display inverted the real order), and post-D1 the grant concept IS TUNA, so a KAT-grant bar was obsolete info bound to mislead new users. The footer now renders the tanks in SERVER DRAW ORDER from SERVER truth (the burn ack): the TUNA tank first, then the KAT balance as a PLAIN line (`122.27 KAT · server balance at the last sync` — a mixed account's balance is grant + mining + legacy remainder, so "X of grant burned" would lie in both directions). No grant bar, no burn-vs-grant caption, anywhere. A repo with no per-repo ack renders meter + sync only (honest absence). The µ-sliver qualifier arm (landed earlier the same day) is superseded by the removal — tombstone test `the_local_grant_projection_never_returns` pins it. The claim-kind signal (server→client) is no longer needed for the footer; it only matters if a future surface wants to NAME the grant kind.
- [x] **TUNA bar scale** — RESOLVED for fresh claims by B2 itself: the claim draws `cfg.free_grant`, the SAME value the footer gauges against, so a fresh claim's bar is exact. Residual caveat: multi-tranche accounts (server-side `tuna_credit` top-ups) can exceed the bar's scale — re-aim when the server reports the real claim size on the wire.

- [x] **Run-line unit honesty** — LANDED 2026-09-18 (riir-clippy `826455d5`; the owner's "still say KAT unit when burn TUNA" follow-up): the footer's run line now bills the tank the server actually draws — `tuna this run  N tok → X TUNA   (1 TUNA / 1M tokens · …)` while spendable trial credit is left (ack TUNA truth present + positive + unexpired), `kat`/KAT once it is exhausted, expired, or the ack carries none. The meter stays plane-agnostic µ and the label is a render-time projection — a mid-run handoff stays the burn-tail split's job (the surface decision above is unchanged). **AMENDED same day (owner format call) to the NEUTRAL form** (`88419d3a` → the `fixed` label, `N tokens` spelled out, the static `(1 TUNA or KAT / 1M tokens)` annotation replacing the meter/charge version stamps — the amount unit still names the active tank; the "or KAT" wording tells the fallback story so the reader never has to wonder which tank pays next; the version stamps left the footer). Current shape + the whole condition/debt/offline story documented at `.docs/01_orientation/kat_billing.md`. Pinned by `the_run_line_bills_the_tank_the_server_draws` + `an_exhausted_or_expired_tuna_tank_bills_kat_again` (21/21 footer tests, clippy clean).
- [x] **Tank-line brevity** — RESOLVED 2026-09-18 (owner call: the `X of Y TUNA left · expires DATE (Nd) · trial credit — heals only` line is too long): the tank is a FUEL GAUGE now — `{bar}  {pct}% (expires in Nd)` — with the percentage display-clamped at 100 (multi-tranche accounts can hold more than the claim scale; the bar never overfills), `expires today` under a day (never `in 0d`), and `(expired — the sweep reclaims the rest)` after expiry. The exact µ numbers live in `--info` / `account`; the `trial credit — heals only` tail moved there too (docs carry it). Pinned by `the_tank_gauge_clamps_and_names_today` + the re-pinned caption arms (21/21 footer tests).
- [x] **Depleted-run guidance (the owner's "how many fixable but out of KAT so NOT FIX / how many mining / how long to wait")** — LANDED 2026-09-18: the 075 refusal now SURVEYS what the run would have done, COUNT-ONLY — `{N} fixable finding(s) found — NOT applied (account depleted; the fixes stay gated)` (the same span-level metric the run summary uses, over the compiled-in corpus — the gate runs before the lease context), the sync outbox's queued mining batches (`cargo heal sync` pushes them / `cargo heal --mine` contributes), and the payout clock (within 7 days of a push + the leaderboard for exact dates). The 075 contract HOLDS: no meter, no charge ledger, no scan summary, no fix content — the count is the advertisement, the fixes stay paid (D4). Pinned by `the_refusal_surveys_what_it_would_have_fixed` (a fixable fixture measures ABOVE zero, `return x;` must not leak, `kat_charge.json` must not exist) + the re-pinned refusal arms (17/17 soft-gate tests: the queued-outbox count, the no-sources no-fabrication arm, and the Some(0) zero arm each have their own test). ⚠ Metric note, measured en route: the count is SPANS WITH RETRIEVAL (the run summary's own "N with suggestions") and the retrieval-first suggester returns nearest-corpus candidates for ANY span — even a comment-only file reads "1 fixable" (the suggestions are corpus examples, not observed defects); a file measures 0 only with no spans at all (zero bytes). The survey's count is an advertisement in the suggest lane's own precision, nothing more.

## Verdict on the owner's Q1/Q2 (recorded)

- Q1 `--fix`: yes — plane-aware tail, landed as far as the client can
  reach without server truth (balance + expiry now; run-split open).
- Q2 `--info`: yes — wallet TUNA line + gate view, landed; the two-door
  conversion counters (healed vs expired) ride B4's sweep and should
  surface in the same `/history` read when they exist.

## The offline arc (recorded 2026-09-18, the owner's "offline `--fix`" question)

- **Offline, nothing is debited anywhere.** The TUNA draw is SERVER-side
  (B3, at the burn report); the client meter is display-only. A failed
  push is a LOCAL COMMIT — one stacked line, no ack, exit code unaffected
  (`tests/kat_burn_stack.rs::a_failed_push_is_a_local_commit`).
- **TUNA and KAT are deducted LATER, at report time.** The next online
  run (or `cargo heal sync`) pushes the FULL cumulative total in one
  report; the server decides the plane THEN — TUNA-first against the
  balances at that moment, KAT remainder, the split named by the tail
  (`the_next_run_pushes_the_full_stacked_total_and_advances_the_ack` +
  `a_split_report_names_the_trial_credit_draw`).
- **No debt exists.** The server's `apply_burn` is balance-clamped
  warn-and-pay (riir-dapps `burn_report_increment_clamp_and_monotone_max`):
  it debits what exists, advances the monotone watermark, records the
  event row at the heal's full size. Nothing goes negative; nothing waits
  for a refill to reconcile — the unbilled remainder is the clamp, by
  design ("the client meter reconciles").
- **The depleted posture is gated at START, not chased.** A depleted
  account that never contributed is REFUSED at the next run start
  (`depleted_never_contributed_refuses_at_run_start`), offered
  contribute-or-pay; contributed accounts are not re-gated until the
  7-day epoch settle pays out. Offline burn while depleted lands clamped
  whenever the report finally syncs.

## Refs

- riir-dapps Proposal 006 (`.proposals/006_tuna_trial_credit_and_token_conditions.md`)
- riir-dapps Issue 097 — the server wire half
- riir-dapps Issue 075 / 080 — the gate + its server-truth rule (the v3 precedent this v4 follows)
