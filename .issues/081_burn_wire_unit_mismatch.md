# Issue 081 — the burn wire carried micro-KAT book values in a whole-KAT field: the first settlement clamp-wiped the account's 100 KAT grant; the server watermark is now poisoned at 2,323,993

**Status:** OPEN — filed 2026-09-09 (found in the owner's live e2e recheck).
Client fix LANDED same day: `wire_kat_total` conversion at the sync boundary
(see the commit ref in the plan note); this issue tracks the SERVER-side
residue + the prose fixes. 2026-09-10: owner item 1's migration SURFACE is
LANDED in riir-dapps (`791fed5` route `POST /admin/burn-watermark/rescale`
— bearer + typed ack + CAS + whole-scale guard, ÷1,000,000 in one
CommitBatch via the /burn row path — + 7 route tests + ledger units green;
record `d577636` there; dapps mirror issue `081`); item 3 was already DONE.
REMAINS: the owner-gated EXECUTION (deploy from a clean sibling-placed
worktree AFTER the sibling's Plan 020 Phase 3 pushes — its worker half is
committed locally `0c6f803` but unpushed — then the one rescale curl + the
replay-is-the-read verification; full runbook in the dapps mirror issue
`.issues/081_burn_wire_unit_mismatch.md`), and item 2's per-account fleet
pass = the same curl per account once the deployment grows past the owner.
2026-09-09 later: the LESSON's cross-repo unit
golden LANDED both sides — riir-dapps `tests/kat_burn.rs` + riir-clippy
`src/kat_burn_client.rs`, both named `cross_repo_unit_golden_book_micro_to_wire_to_debit`,
pinning the same concrete table (book 999_999 / 1_000_000 / 2_380_972 /
3_000_000 micro → wire 0/1/2/3 whole KAT → increment × 1_000_000 micro
debit, replay = 200 no-op) with no cargo dep between the repos — the
symmetric-mock/asymmetric-unit gap that let the incident ship is now a
drift tripwire. Owner items 1–3 below REMAIN (watermark rescale / fleet
books / proposal prose).

## What happened (measured live, ai.gist.rs)

1. The charge book counts **micro-KAT** (`charged_kat` = 1 micro per token at
   the v1 rate; the same unit as `FREE_GRANT_KAT: MicroUnits = 100_000_000`
   = the 100 KAT grant — dapps `kat/mod.rs:262`).
2. The pinned dapps wire contract carries `burn_kat_total` in **whole KAT**
   (the service debits `increment × 1_000_000` micro).
3. The client sent the book value UNCONVERTED. The machine's first accepted
   burn report (2026-09-09, unix 1788911127, from the riir-clippy repo)
   reported **2,323,993 whole KAT** for a true spend of **2.32 KAT**.
4. Warn-and-pay clamped the debit at the balance: the account's ENTIRE
   100 KAT grant was consumed (`GET /balance` → `balance_micro: 0`; the
   leaderboard's "0 KAT in circulation" is this clamp, not thrift).

## What is already fixed (client, this repo)

- `wire_kat_total(charged_micro) = charged_micro / 1_000_000` (floor) at the
  single sync boundary; the skip comparison + the Stacked total converted
  with it. Sub-KAT remainders accumulate in the book and ride the next
  whole-KAT crossing (the server cannot debit less than a whole KAT).
- Tests: conversion floor table + the socket test asserting the wire carries
  whole KAT from a micro book + the sub-KAT skip + the crossing report.
- Prose: `kat_meter` module docs + `CHARGE_CHANGELOG` v1 entry + the live
  front-page/leaderboard strings now say "1 KAT per million code word
  tokens" (the code's actual, deployed economics; the proposal's "~100M
  tokens ≈ $300 of Claude-equivalent value" intent is EXACTLY preserved by
  1 KAT/M tokens against the 100 KAT grant).

## What remains — owner decisions (server-side)

1. **The poisoned server watermark.** The monotone watermark for account
   `c79c93b8…5345` now stores 2,323,993 (whole KAT, from the buggy report).
   The fixed client's converted totals (single-digit KAT) will be ≤ stored
   for a very long time, so every report is a 200 no-op — the account can
   never report burn again (harmless while the balance is 0, but it also
   stays dead after a refill: burns never settle, the ack never advances).
   Options:
   - **(a) Rescale the row** — one-time migration: `kat:burnwm:<acct>`
     value ÷ 1_000_000 (→ 2). Devnet, one account, one row; a small dapps
     maintenance task (or even a hand-edit of the devnet DO store).
   - **(b) Fresh account** — new key, new grant; the old row stays inert.
     Zero code; loses the account identity + the 168-row contribution
     attribution.
   Default call recorded here: (a) before the next real billing cycle; (b)
   acceptable while the fleet is one owner account.
2. **The rest of the fleet's local books.** Any machine that ran the buggy
   binary has a v2 ack mirroring the inflated server totals. The fixed
   client handles this correctly by accident of the same arithmetic (the
   converted totals skip under the inflated acks) — the same poisoned
   watermark applies to every account that ever settled a burn. If the
   deployment grows past the owner's own account, option (a) needs a
   per-account migration pass, not a hand edit.
3. ~~**The proposal prose**~~ — **DONE 2026-09-09 (owner un-gate: "proposals
   can be edit, just noted to design change with date and git hash"):**
   `.proposals/005`'s three economics sites (the login line, the §Unit
   spec, the §Free tier) now carry dated `[DESIGN CHANGE 2026-09-09]`
   annotations stating the deployed economics (1 KAT per 1M code word
   tokens, 100 KAT grant — the ≈ 100M-token ≈ $300 intent preserved), the
   reason (whole-KAT wire units, Issue 081's incident), and the fix hashes
   (`e9eca71`, golden `3dd14dd` + riir-dapps `54d6965`). Original text
   kept verbatim; annotations only.

## Lessons

- **A money unit must have ONE name per side.** `charged_kat` holding micro
  while the wire field says `burn_kat_total` (whole) and a third constant
  (`FREE_TIER_GRANT_KAT`) holding micro with KAT in its name is three
  unit claims that all read plausible. The type alias (`MicroUnits` on the
  dapps side) is the only thing that kept the server honest; the client had
  no type, just a name.
- **The first e2e settlement of a money path is a test, not a
  formality.** The unit gap survived dozens of golden/contract tests because
  both sides tested their OWN units with mock numbers (40, 500, 12345) —
  symmetric mocks, asymmetric units. Only a live cross-boundary run (or a
  cross-repo unit-pinning test, the dapps-contract-pins pattern in
  riir-dao) catches it. The kat contract pins now live in
  `tests/dapps_contract_pins.rs` (riir-dao) for the ledger rows; the burn
  wire deserves the same cross-repo golden.
