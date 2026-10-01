# The KAT billing + TUNA trial-credit surfaces

What the footer charges, what the tanks are, what happens offline, why
there is no debt, and what a depleted account can still do. The client
half of the KAT network (riir-dapps Proposal 005) and the TUNA trial
credit (riir-dapps Proposal 006). The client-side record is Issue 123;
this page is the current-shape reference.

## The two tanks, one meter

Healing is billed per million code word tokens at a fixed µ rate
(`CHARGE_MICRO_KAT_PER_MILLION_TOKENS`). The meter is plane-agnostic µ —
the SERVER decides which tank pays, always in the same order (B3):

1. **TUNA** — the trial credit. Heals only, non-transferable, expires.
   Spendable = positive, unexpired tranches; expired-but-unswept credit
   is inert and is never reported.
2. **KAT** — the network currency. Everything else: mining earnings,
   top-ups, legacy remainders.

The user never has to do anything about the handoff — when TUNA runs
out mid-report, the rest of that report draws KAT automatically, and the
burn-sync tail names the exact split
(`drew X TUNA + Y KAT — trial credit exhausted mid-report; billing KAT now`).

## The run footer (the current shape)

```text
     fixed     8,791 tokens → 0.0088 TUNA   (1 TUNA or KAT / 1M tokens)
     tuna      ⣿…⣄⣿⣿⣿  47% (expires in 12d)
     kat       84.92 KAT · server balance at the last sync
     sync      ✓ current — ledger matches · mint settles hourly · leaderboard ↗ (you: c79c…5345)
```

- **Run line** — neutral `fixed` label (the rate is fixed in either
  currency), the token count, the metered µ, and the AMOUNT unit naming
  the tank this run draws from (TUNA while spendable trial credit is
  left, KAT once it is gone — or the ack carries no TUNA truth). The
  static `(N TUNA or KAT / 1M tokens)` annotation tells the whole
  fallback story; the reader never has to wonder which tank pays next.
- **TUNA tank** — a fuel gauge, not a ledger readout: `{pct}%
  ({expiry})`. The percentage display-clamps at 100 (a multi-tranche
  account can hold more than the gauge's claim scale; the bar never
  overfills), `expires today` under a day (never `in 0d`), and
  `(expired — the sweep reclaims the rest)` after expiry. Renders only
  when the burn ack carries server-reported TUNA truth; absent = no
  line, never an invented zero. Exact µ numbers live in `cargo refine
  --info` / `account`.
- **KAT line** — the server-reported balance at the last accepted push.
  A PLAIN balance (grant + mining + legacy remainder are
  indistinguishable by then) — never "X of grant burned".
- **Sync line** — state-aware: current / the stacked whole-KAT gap +
  queued mining batches / logged out / no-transport.

## Offline: nothing is debited anywhere

The TUNA/KAT draw is SERVER-side, at the burn report. Offline:

- The run completes normally; the exit code is unaffected.
- The burn is a LOCAL COMMIT — one stacked line
  (`burn sync: offline — N KAT stacked, reports with the next run`), the
  ack watermark untouched. The wire is whole-KAT granular, so a sub-KAT
  sliver is by design unacked until the next crossing (not a stuck
  delta).
- The meter keeps its local projection; nothing waits for a connection.

Back online, the next run (or `cargo refine sync`) pushes the FULL
cumulative total in one report — the server decides the plane THEN,
against the balances at that moment (TUNA may have expired while
offline; KAT may have been spent by another machine). The server's
monotone watermark makes replays no-ops, and the ack advances only on a
200-accepted response.

## No debt exists

The server's `apply_burn` is **balance-clamped warn-and-pay**: it debits
what exists, advances the watermark (monotone max), and records the
event row at the heal's full size whatever paid for it. The unbilled
remainder is the clamp, by design — the client meter reconciles.
Nothing ever goes negative, and nothing waits for a refill to
reconcile.

## The depleted posture (the 075 soft gate)

A logged-in account whose LAST SYNCED server balance is ≤ 0 **and** that
has never contributed-and-synced gets scanning runs REFUSED at start
(exit 1). Mode-wide — every scanning run burns fuel, so gating only
`--fix --write` would be a sieve. The sticky grace is server truth only
(the burn ack's `contributed == Some(true)`, written from the
authenticated push response); logged-out / offline / never-synced
machines are never gated.

The refusal names exactly TWO refuel paths (the faucet/drip tier is
retired — riir-dapps Issue 094 D4: *"in the long run they have to pay or
this tokenomics will die"*):

1. **Contribute** — `cargo refine --mine` (what mining is) → `cargo refine
   sync`. One accepted push records the server's `contributed` truth and
   lifts the gate for good.
2. **Top up** — the payment page.

The refusal also OFFERS the contribute-now path (one y/N on a TTY,
`RIIR_HEAL_KAT_PROMPT_YES=1` / `--yes` for agents) and SURVEYS what the
run would have done, count-only:

```text
What this run would have done (count-only survey — nothing was fixed, nothing was billed):
  * survey: 3 fixable finding(s) found — NOT applied (account depleted; the fixes stay gated)
  * mining: 2 batch(es) queued — `cargo refine sync` pushes them (payouts land at the 7-day epoch settle)
  * payouts land within 7 days of a push — exact dates: <service>/leaderboard
```

The count is the advertisement; the fix content stays paid (no
suggestion text, no scan summary, no charge ledger — the 075 contract
holds). Metric note: the count is SPANS WITH RETRIEVAL, the run
summary's own "N with suggestions" — the retrieval-first suggester
returns nearest-corpus candidates for any span, so a coded file can read
≥1 even when nothing is truly fixable, and a file measures 0 only with
no spans at all (zero bytes). It is an advertisement in the suggest
lane's own precision, nothing more.

**Payout clock:** mining contributions are credited at the 7-day epoch
settle — within 7 days of an accepted push; the leaderboard names the
exact dates. Contributed accounts are never re-gated while depleted,
because the settle pays them.

**Kill switches:** `RIIR_HEAL_KAT_GATE=0` (the gate) and
`RIIR_HEAL_KAT_METER=0` (the shared billing plane).

## Where the truth lives

| Surface | Source |
|---|---|
| Footer tanks | the burn ack v4 (`tuna_balance_micro`/`tuna_expires_unix`, `last_balance_micro`) — server truth, defensive parse, absent = `None` renders nothing |
| Per-report plane split | the burn response's `charged_tuna_micro`/`charged_kat_micro` — named by the tail, deliberately NOT persisted into the ack (a per-report fact, not position) |
| Depleted predicate | riir-kat `src/kat_soft_gate.rs` (pure; the CLI consumes it) |
| Exact position | `cargo refine --info` / `account` / the leaderboard |

## Tests

| Suite | Covers |
|---|---|
| `tests/kat_burn_stack.rs` (4) | offline stack = local commit; next run pushes the full stacked total; the split names the trial-credit draw; no-new-burn sends nothing |
| `tests/kat_soft_gate_cli.rs` (17) | the refusal matrix + the contribute-now offer + every survey arm (NOT-applied, queued-outbox count, no-sources no-fabrication, zero) + the kill switches |
| `src/kat_footer.rs` tests (21) | the run line, the tank gauge (clamps, expiry, absence), the kat line, sync states |

## Refs

- Issue 123 — the client-side record (ack v4, gate planes, footer, survey)
- riir-dapps Proposal 006 — TUNA: trial credit + token conditions
- riir-dapps Issue 094 D4 — the faucet retirement + the two-door doctrine
- Issues 075 / 079 / 080 — the gate, its contribute-now offer, server-truth grace
