# Issue 061 — Proposal 011 rollout: the KAT product loop (pay-request, genesis ceremony, deploy UX, wet-run, onboarding)

Status: **OPEN (2026-09-01)** — Phase 0 **T0.2 (the riir-clippy slice) landed this
session** (see the Landed section + [Plan 059](../.plans/059_kat011_t02_and_corpus_runtime_counts.md));
every other task lives in riir-dapps / riir-deployer / riir-chain-sdk and awaits their
respective sessions. The proposal is draft and all five phases were open at filing; this
issue is the cross-repo tracker so the phased scope survives the noise-reduction rule.
Branch: `develop` (per global rule — no feature branches)
Proposal: [`.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md`](../.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md)
Related: [Issue 059](059_proposal_007_remainder_ops_signer_and_counsel_gate.md) (the
shared counsel envelope), [Issue 060](060_proposal_008_remainder_new_plan_and_gated_levers.md)
(the crank's grant machinery context), riir-deployer
[Proposal 001](../../riir-deployer/.proposals/001_riir_control_cf_worker_do.md), riir-chain
Proposal 008 (MemoShard).

## Phase 0 — ledger + SDK substrate

- [-] T0.1 `payreq` request-ledger lifecycle rows + invoice rows + faucet credit row
      type (riir-dapps, `kat_payreq`). **The request-ledger rule (ask 14) is
      load-bearing: every request is a row FIRST — the limiter reads these rows,
      refusals are rows too, and `chain_anchor` is the chain-mapping slot.**
- [x] T0.2 clippy CLI: `login` default-SSH adoption + `RIIR_HEAL_KAT_KEY` override;
      `account` welcome + `--history` (local view; the copy rule test-enforced) —
      **LANDED this session** ([Plan 059](../.plans/059_kat011_t02_and_corpus_runtime_counts.md)).
      The service half (live `/balance`, the all-synced + nodes-healthy welcome gate,
      server-side history) is the `[-]` below.
- [-] T0.2b — the service half of account: welcome gate evaluated from real sync/node
      health + `/balance` history consumption (riir-dapps route shape lands with
      T1.1). Blocked on T0.1/T1.1.
- [-] T0.3 riir-chain-sdk `pay_request` module (KAT compose + resolve/transfer; USDC
      arm stubbed behind the mock).
- [-] T0.4 self-pay fixtures (chain + ledger + payreq) — the wet-run loop IS self-pay
      (ask 2).
- [-] T0.5 registration-intent + funding-binding row types (the ask-12 substrate).
      **The binding verdict is decided: the REFERENCE key is the mandatory machine
      link; the memo is mandatory-in-URL audit copy — a memo-mandatory gate would
      strand real payments.**

## Phase 1 — the KAT rail + web wallet (DEFAULT, ships first — owner call 2026-09-01)

- [-] T1.1 `/payreq` routes (create/resolve/settle/invoice) + write-ahead rows +
      limiter-backed-by-rows + signed resolve (riir-dapps; caveat #7 — signature-
      verified, idempotent, rate-limited, no unauthenticated state writes).
- [-] T1.2 the web wallet on the kat-service host: GitHub OAuth → `.keys` fetch →
      key-match → session → press-and-hold SEND (**server-enforced min-hold** — the
      client's 3 s is cosmetic until the worker enforces `held_ms ≥ 3000`) → settle →
      invoice row. **The root/genesis float stays CLI-only.**
- [-] T1.3 clippy `pay --request [--amount]` (KAT default; blank = any amount) +
      link-first (`--qr` fallback) + poll-on-credit (riir-clippy half — after T1.1).
- [-] T1.4 invoice display + bit-exact recheck (golden-pinned QR; caveat #3 — the
      re-encode pin rides the `qrcode` crate version + render params).
- [-] T1.5 G1-S/G-WEB money + auth + adversarial suite (replay, session theft, amount
      manipulation, self-pay, hold-bypass, root-float refusal).
- [-] T1.6 Stripe/USDC rails as MOCK tests only (five-check machinery against
      fixtures; no live rail this phase).
- [-] T1.7 the daily grant crank (alarm pass, `(account, utc_date)` exactly-once,
      activity gate, budget cap) + G-CRANK. **A GRANT crank via M2 treasury-first —
      never a mint faucet; the amount joins the 008 never-lever free-grant family.**
- [-] T1.8 `faucet --request` preset + `faucet_daily_cap_kat` enforcement + `link_ttl`
      expiry (expired = settle-refused).

## Phase 2 — genesis ceremony (bar)

- [-] T2.1 params yaml + validation against the pinned tokenomics constants (drift
      FAILS — no silent clamp; 008/010 ratified defaults).
- [-] T2.2 ceremony: keys → chainspec authorship → registry genesis → SSH provision
      (caveat #6 — dry-run-by-default; wrong root key = locked treasury, double mint =
      supply bug; the ceremony is the ONLY sanctioned author of testnet/mainnet.toml —
      closes the 040 D4 gap).
- [-] T2.3 testnet ceremony executed once; `testnet.toml` lands.

## Phase 3 — deploy UX + health

- [-] T3.1 TTY selector + config/version acks (flags-only non-TTY; mainnet hardening —
      caveat #5: a prompt is not a security boundary).
- [-] T3.2 `node-sync` probe + 3-node verify gate (reachability + height + sync%).
- [-] T3.3 G-DEPLOY on a local 3-node cluster.

## Phase 4 — wet-run + foo onboarding

- [-] T4.1 `wetrun` harness (steps 1–6) + G-WET. **Caveat #2 is load-bearing: the
      refund is FAUCET-funded, never mint/burn-pool — the G-WET assertion (mint/burn
      counters bit-unchanged) is the money-printer guard.**
- [-] T4.2 mainnet refusal (unit + live, NO override by design) + `--dry-run`
      read-only proof.
- [-] T4.3 install.sh copy alignment (gist-rs/cargo-heal — the 003 surface, NOT
      riir-clippy releases) + foo end-to-end doc.
- [-] T4.4 testnet wet-run executed; mesh-ready verdict recorded.

## Phase 5 — live USDC pay-and-register (real users; counsel + demand gated)

- [-] T5.1 live Solana-Pay settle path + G-REG matrix run live (the ten §12 cases;
      fixtures first; the `#[ignore]`d live arm un-ignored).
- [-] T5.2 ops reconcile runbook for expired-intent stuck funds (until outbound
      refunds exist — Issue 059 T2.2 territory).
- [-] T5.3 testnet-then-mainnet launch decision (the counsel envelope shared with the
      KAT rail — Issue 059 T3.1).

## Deferred — gated (from the proposal; tracked, not scheduled)

- [-] Chain-NFT invoice mint (after the Warm-tier invoice proves out; opt-in,
      Glacial cadence — a per-payment NFT would fail the rate test).
- [-] MemoShard v4 signing-message adoption (riir-chain 008 decision).
- [-] LIVE Stripe + Solana-Pay rails (mock-tested; only the launch decision waits).
- [-] KAT-rail launch beyond operator-owned accounts (the counsel gate — Issue 059).
- [-] `riir-wallet-wasm` invoice display (browser wallet; CLI view first).
- [-] Discord faucet bot (link-first made it pure automation; reopen on volume,
      gated on a per-account daily cap + identity proof no weaker than GitHub-SSH).
- [-] Baz fixer tier — its OWN plan, gated per 011 §9: (a) a fixer arm beats `decline`
      on the reachable set, (b) leak-posture sign-off, (c) reward economics with
      G5-class evidence (blocked by 060's advisory-only verdict). Opening it before
      clippy-tier onboarding lands would put the least-proven earning lane in front of
      the newest users.

## Summary

The proposal's fourteen owner asks are all verdicted SHIP (two corrected: 8.1 →
gist-rs/cargo-heal per 003; 12 → reference-key binding, memo as audit copy) or
explicitly deferred. Phase 0 T0.2 is the only task native to this repo and it LANDED
this session; everything else needs riir-dapps (rows + web wallet + crank), the
riir-deployer genesis/deploy work, riir-chain-sdk, and the two counsel-gated launches.
Next action per the proposal: the riir-dapps Phase 0 plan, and Phase 2's ceremony
early — bar's dry-run genesis is what puts the mint in the wallet every foo gets paid
from.
