# Plan 041 — Proposal 007 T2.1: the staking CLI surface (stake / stake --info / unstake)

Status: **complete** (landed 2026-08-28, commit `6ee000e`) · validation: default lib
712/0 · all-features lib 1409/0/13i · unit 19/0 · integration 9/0 · clippy 0 in touched files

Repo: riir-clippy · Proposal: [007 §CLI surface](../.proposals/007_kat_usdc_staking.md) L71-92 ·
Service contract: `riir-dapps/src/kat/staking.rs` (landed 7a18690) + `solana.rs` (the five-check
rail + the three-param transfer-request URL)

## Context

The Worker/DO HTTP shell is deferred (the T1.6 posture), so like the lease client the CLI
**composes requests + renders**, validated against a local std-TCP mock; the live-Worker flip is
a later one-line change. The service contract to mirror:

- transfer-request URL: `solana:<payee>?amount=<X>&spl-token=<USDC>&reference=<ref>` (the
  three load-bearing params, `payment_request_url` shape)
- **stake-custody payee is SEPARATE from the revenue payee** (refundable principal never
  commingles) — configurable via env, like the lease service URL
- tranche/account shapes for `--info`: `StakeAccountHead` (funding, restake, seq, accrued_kat),
  `TrancheRow` (kind/amount/staked_at/unlock_at), `StakingParams` (the authority triple + the
  class sub-shares + the exit constants)
- unstake = drain all unlocked tranches → one refund intent; refusals carry exact remaining
  ticks

### The one copy correction vs the mock (amendment (b))

The proposal mock's `Unlock:` line says *"(top-ups restart the 7-day clock)"* — written
**before** the same-day amendment (b) (per-tranche lockup rows; "a top-up never re-locks
earlier tranches"). The amendment supersedes the mock line; the render ships the amended
truth: *"(each deposit owns its own 7-day clock)"* + the per-tranche FIFO countdown the
mission asks for. Recorded here so the divergence from the mock text is deliberate, not drift.

## Tasks

- [x] T1 `src/kat_stake_client.rs` under `kat_service`: the mirrored base58
      (encode/decode/fixed-32, the dapps math-matrix goldens), `format_amount_base`,
      `parse_usd_to_base`, the transfer-URL composer, the fresh on-curve reference
      (ssh-key Ed25519 → 32B pubkey, the dapps `new_payment_request` entropy pattern), the
      stake-config env resolution (`RIIR_HEAL_KAT_STAKE_PAYEE` / `RIIR_HEAL_KAT_USDC_MINT`,
      base58-validated), the `StakeIntent`/`UnstakeIntent` local queue (`.heal/stake/`),
      and the three renders (stake copy / info / unstake disclosure).
- [x] T2 the `kat_transport` half: `StakeTransport` + `HttpStakeTransport` (ureq, the
      lease-transport pattern) — `POST /stake/request`, `GET /stake/info?acct=`, `POST
      /stake/unstake`; service-first with the honest not-live fallbacks.
- [x] T3 CLI wiring: `stake --usd <N>` / `stake --info` / `unstake` in `src/bin/heal.rs`
      (the T1.6 command pattern; QR via the existing `render_qr_ascii`).
- [x] T4 the caveat-#15 copy gate by test (the `no_dollar_sign_in_any_render` precedent,
      extended): no `$` on any KAT-accrual line; accruals labeled "service credit"; "APY"
      only inside the no-promise phrase; the escrow/vent/refund-to-source/accrual-stop
      disclosures present in the unstake render. (In-module + integration copies.)
- [x] T5 `tests/kat_stake_client_g2s.rs` — the std-TCP mock fixtures (the g10 pattern):
      the three wire shapes, the not-live honest paths, the copy gate, the G2-S render
      budget smoke. 9/9 green.
- [x] T6 CI Layer 5c pin (`--test kat_stake_client_g2s`) + `.highwater` 40→41.
- [x] T7 Validation: default build + `--lib` green 712/0 (zero new default deps —
      `kat_service`/`kat_transport` stay opt-in, `ssh-key`/`ureq`/`qrcode` all already in
      tree); all-features lib 1409/0/13i; g2s 9/0 + g10 13/0 + g9 6/0 re-run green; clippy
      0 new in touched files (healer first — 3 mechanical edits — then the semantic
      manual set: doc backticks, `# Panics`, `unused_self`→assoc fn, redundant closures,
      `write!` over `push_str(&format!)`, bounded-cast allows with justification); the
      `block v0.1.6` future-incompat + the 23 unresolved doc links are PRE-EXISTING at
      clean HEAD (stash-proven, same counts). Live CLI smoke: login → stake --usd 100
      (copy + URL + QR + queue) → stake --info (not-live fallback view) → unstake
      (queue + full disclosure), all against an unreachable service URL.

## Deferrals (honest)

- `stake --restake on|off` — the flag RENDERS in `--info` (the mock's restake row) but the
  flip is a service mutation enforced server-side at credit time (`set_restake`); queueing a
  flip the deferred Worker can never receive is theater. Lands with the Worker shell.
- The live-Worker flip (one-line: the service URL already resolves the same way).
- T2.2 (ops signer + daily batch drain) and T2.3 (chaos/soak on the full loop) — later waves.
