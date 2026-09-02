# Plan 015 — Proposal 011 Phase 0 substrate: `kat_payreq` (T0.1 + T0.5 + T0.4)

**Status:** COMPLETE — riir-dapps `6df7097` (Phase 0 ledger substrate landed, all
validation green). riir-clippy Issue 061 updated; Issues 059/060 verified as
correctly-gated wait-state trackers.

Proposal: riir-clippy `.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md`
(§1 PayRequest object, §2 self-pay, §13/ask-14 the request-ledger rule, §12 ask-12
the signed registration intent + funding binding).

## Design (substrate-composed, no new deps beyond the ed25519 verify seam)

New module `src/kat/payreq.rs` behind feature `kat_payreq = ["kat_ledger", "dep:ed25519-dalek"]`
(ed25519-dalek is already optional in the manifest — the kat_solana/kat_governance
convention; verification only, the ledger never signs). Zero config-row / Meta-row
widening in this plan: T0.1 adds no knobs (the faucet daily cap, `daily_grant_budget`,
`link_ttl` are Phase 1 config), and audit views scan rows — so `kat:cfg:v1` (136 B
core) and `Meta` (65 B) stay byte-identical, the loud-`ConfigMismatch` fingerprint
untouched.

- [x] **T0.1a — the request row (ask-14: track · limit · chain mapping).**
      `kat:req:<request_id:16>` → fixed-width 96 B row:
      `{kind: u8, status: u8, account: [u8;16], amount: i64 (0 = any-amount, ask 3),
      amount_paid: i64, created: u64, updated: u64, chain_anchor: [u8;32]}`.
      Kind: `Payreq | FaucetRequest | RegistrationIntent | Resolve | Settle`.
      Status machine `Created → Resolved → Settled | Expired | Refused`, terminal
      states immutable (transition fns refuse, never silently rewrite). Refusals are
      rows (`refuse_request` writes status=Refused + reason byte) — the limits are
      auditable too. `chain_anchor` zeroed until epoch settlement composes it (the
      slot exists now; the fill is the settle-path's).
- [x] **T0.1b — lifecycle + limiter read paths on `KatLedger`.**
      `payreq_create` (write-ahead row, BLAKE3-canonical id), `payreq_resolve`,
      `payreq_refuse`, `payreq_settle` (ONE `CommitBatch`: payer debit + payee credit
      + request→Settled + `kat:inv:` invoice row — the 007 one-batch discipline;
      KAT-rail settle is a TRANSFER: no mint demand, no supply change), and the
      limiter's read path `requests_by_account` (scan + filter — the proposal's own
      "no second store" rule; cost trivial at request cadence).
- [x] **T0.1c — the invoice row.** `kat:inv:<id:16>` →
      `{payee, payer, token: u8, amount_paid: i64, settled_unix: u64, qr_png_blake3:
      [u8;32]}` (§1: the QR PNG is a BLAKE3-committed asset of the row; committed at
      settle time as zeroed until T1.4 renders it — honest placeholder, never a fake
      hash).
- [x] **T0.5 — registration-intent + funding-binding rows (the ask-12 substrate).**
      - `kat:uintnt:<reference:32>` → `{kat_account: [u8;16], min_amount: i64,
        expiry_unix: u64, signature: [u8;64]}` + `verify_intent_signature` (ed25519
        over the canonical intent bytes against the account's verifying key — the
        G-REG "forged intent ⇒ refused" seam; nobody binds an intent to an account
        whose key they do not hold).
      - `kat:fbind:<kat_account:16>` → `{solana_source: [u8;32], bound_unix: u64}` —
        FIRST-WINS (`bind_funding` returns the existing binding on conflict; a later
        DIFFERENT source is recorded as a flagged request row, never an overwrite).
- [x] **T0.4 — self-pay fixtures.** `tests/kat_payreq.rs`:
      (a) payreq A→A settles to a paid invoice, no net supply change (Σ balances
      conserved), exactly-once (double settle refuses);
      (b) self-pay via the settle seam with any-amount request (payer chooses);
      (c) lifecycle: illegal transitions refused, terminal immutability, refusal rows
      auditable, per-account read path sees every lifecycle row;
      (d) funding binding first-wins (second different source recorded + flagged,
      binding unchanged);
      (e) intent signature verify: happy path + forged (wrong key) refused + expired
      intent refused;
      (f) invoice row round-trip byte-exactness + id = BLAKE3(canonical)[..16] pin.
- [x] Chain-side self-pay fixture (T0.4's "chain" half): the A→A `TxDelta` shape is
      riir-chain-sdk/chain territory — NOT re-implemented here; the payreq/ledger
      halves are the substrate this repo owns. Recorded honestly below.
- [x] GOAT gates: `cargo clippy -p riir-dapps --features kat_payreq --all-targets`
      clean (only 2 PRE-EXISTING `kat_governance`-method dead-code warnings that fire
      under any non-governance feature set at HEAD — ledger.rs untouched);
      `cargo test --test kat_payreq` 10/10 green; default-features + `kat_ledger`
      compile clean; ledger suite 14/14 (the config/Meta-row goldens) untouched;
      the full KAT feature union green end-to-end (~380 tests incl. payreq).
      **En-route catch: the settle's insufficient-balance gate was checked_sub-only
      — i64 checked_sub does NOT fail on negative results; the fixtures caught it,
      fixed with an explicit sign check (balances never go negative, the ledger's
      own record_burn posture).**

## Deliberately NOT in this plan

- HTTP routes (T1.1), the web wallet (T1.2), QR rendering (T1.4) — service-plane,
  next plan. This module is the ledger DO's host-agnostic half only.
- Rate-limit ENFORCEMENT (T1.1's limiter-backed-by-rows + T1.7's crank): the row
  substrate + read paths land here; the caps (`faucet_daily_cap_kat`,
  `daily_grant_budget`) arrive as Phase 1 config knobs — which WILL widen
  `kat:cfg:v1` and follow the staking widening precedent, loudly.
- The USDC settle path (five-check capture) — `kat_solana` composes these rows at
  T1.6/T5.1; T0.5 lands the binding + intent seams it writes through.

## Task list

- [x] `kat_payreq` feature + module skeleton (Cargo.toml, `kat/mod.rs` re-export)
- [x] Request row type + key + status machine (+ refusal rows)
- [x] Invoice row type + key
- [x] Intent row type + signature verify
- [x] Funding-binding row + first-wins
- [x] `KatLedger` composition: create/resolve/refuse/settle + `requests_by_account`
- [x] `tests/kat_payreq.rs` (the T0.4 fixture set above)
- [x] Validation: clippy + tests across `default / kat_ledger / kat_payreq` states
- [x] Commit + push (riir-dapps develop) — `6df7097`
- [x] riir-clippy Issue 061 updated (T0.1/T0.5/T0.4- halves → landed, commit ref);
      Issues 059/060 re-verified as correctly-gated wait-states (no executable work —
      recorded, files stay as trackers)
