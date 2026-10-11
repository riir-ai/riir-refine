# Plan 062 — Proposal 011 T1.1: the `/payreq` routes on the kat-service worker

**Status:** T1.1 COMPLETE — riir-dapps `f43c014` (routes + limiter + contract
suite, all validation green); the create-message golden pin followed in
riir-dapps `0a4d37a` with the T1.3 client landing as riir-clippy `9a5086a`.
Issue 061 updated.

Proposal: riir-clippy `.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md`
(§1 the request IS a shareable link, §13/ask-14 the request-ledger rule,
caveat #7 — signature-verified, idempotent, rate-limited, no unauthenticated
state writes; caveat #12 — the link is a capability URL, view-only leaks the
request and nothing else).

## Design

All routes live on the LEDGER DO (the `ledger_route` fallthrough already
serves them; no worker-router change). Account identity is DERIVED, never
trusted from the wire: `account = blake3(pubkey)[..16]` — the sync plane's
`submitter_for` derivation, re-homed as `payreq::account_for_pubkey` (kept
local because `kat_mining` is an independent feature; equality pinned by a
golden test under the worker's feature union). Every state-writing route
verifies an Ed25519 signature by the account key over a domain-separated
canonical message (the `verify` convention of the account-key lane — mining
v2 + gov votes; `verify_strict` stays the ops-lane convention).

Canonical messages (substrate-owned, `riir-dapps/src/kat/payreq.rs`):

- `create_message(payee_pubkey, amount, label)` = `kat:payreq:create:v1:` ‖
  payee_pubkey ‖ amount_le ‖ label — signed by the PAYEE (only the key
  holder can open requests on their account).
- `resolve_message(request_id, payer_pubkey)` = `kat:payreq:resolve:v1:` ‖
  request_id ‖ payer_pubkey — signed by the PAYER (the intent-to-pay
  bookkeeping step).
- `settle_message(request_id, payer_pubkey, amount)` = `kat:payreq:settle:v1:` ‖
  request_id ‖ payer_pubkey ‖ amount_le — signed by the PAYER; the amount is
  INSIDE the signature, so a captured settle intent cannot be replayed at a
  different amount.

Replay/idempotence is the row machine, not a nonce cache: a replayed settle
hits a terminal row and is refused (409); a replayed create re-derives the
same BLAKE3 id and is answered idempotently; a same-id different-shape create
is a 409 collision.

The limiter reads the rows (no second store): per-account OPEN
(`Created|Resolved`) request cap from `KAT_PAYREQ_MAX_OPEN` (service-plane
DoS bound, `ServiceEnv` — NOT `kat:cfg:v1`; the tokenomics caps
`faucet_daily_cap_kat` / `daily_grant_budget` stay T1.7/T1.8 config-widening
territory). Over cap → the created row is transitioned to `Refused` (the
refusal is itself a row, G-REQ) + 429. Expiry (`link_ttl`) is T1.8 — the row
has no expiry slot by design this phase; documented honestly in the view
response.

### Routes (wire)

| Route | Auth | Behavior |
|---|---|---|
| `POST /payreq/create` | payee sig | write-ahead row → limiter → 200 `{request_id, created, amount_micro}`; 409 collision; 429 over-cap + refusal row |
| `GET /payreq/{id}` | none (capability URL) | view: status/payee/amount; 404 unknown |
| `POST /payreq/{id}/resolve` | payer sig | Created→Resolved; idempotent on Resolved; honest `settled` view; 409 expired/refused |
| `POST /payreq/{id}/settle` | payer sig | `payreq_settle` ONE batch (debit+credit+Settled+invoice) → drain → receipt; 409 terminal; 400 refusals (insufficient, pinned-amount mismatch, non-Payreq kind); 404 unknown |
| `GET /payreq/{id}/invoice` | none | the settled invoice row; 404 until settled |

## Tasks

- [x] Substrate: `payreq::account_for_pubkey` + the three canonical message
      composers (pub, tested) + `mod.rs` re-export + the cross-pin test vs
      `mining::submitter_for` (runs under the worker's feature union) — 12/12
      with the union, 11/11 payreq-only
- [x] kat-service: `kat_payreq` feature + `ServiceEnv.payreq_max_open_per_account`
      (manual Default = 16; `KAT_PAYREQ_MAX_OPEN` env; `do_host::service_env_of`)
- [x] Routes: create / view / resolve / settle / invoice + the route-table doc
- [x] Contract tests (contract.rs): full lifecycle (create→resolve→settle→invoice,
      balances, drained), replayed settle 409, forged signatures 401
      (create/resolve/settle), pinned-amount mismatch 400, any-amount settle,
      underfunded 400, self-pay conservation, idempotent create, limiter 429 +
      refusal row, unknown-id 404s, invoice-before-settle 404 — 16/16
- [x] Validation: `cargo test` native contract suite (kat-service), `cargo
      clippy` clean (kat-service fresh-target + `riir-dapps --features
      kat_payreq --all-targets`), the substrate's `tests/kat_payreq.rs` 10/10
      still green (13/13 after the goldens)
- [x] Commit + push (riir-dapps develop) — `f43c014` (+ golden `0a4d37a`)
- [x] riir-clippy Issue 061: T1.1 → landed with commit ref (plus the T1.3
      client half, `9a5086a`, landed the same session)

En-route fixes the compile/test gates caught (recorded honestly): the
`VerifyingKey::verify` trait-import gap, `PayreqError` Debug-format in the 500
arms, `id_str.as_deref()` needless-Option borrow, the limiter test's off-by-one
(14 fills not 15 — cap 16 means the 17th refuses), and the clippy drive-bys in
routes.rs (manual_unwrap_or_default, manual_is_multiple_of — both pre-existing,
mechanical, in a file already touched).

## Addendum — T1.3 (the client half, same session)

T1.3 landed with this plan's service half instead of waiting: Plan 062's scope
stayed T1.1 (the routes), and the client module + CLI (`pay --request`) landed
as the paired unit — the golden pin is only load-bearing when BOTH sides exist.
Details in Issue 061's T1.3 row (riir-clippy `9a5086a`).

## Deliberately NOT in this plan

- The web wallet page (T1.2), QR rendering (T1.4), faucet preset + `link_ttl`
  (T1.8), the grant crank (T1.7), USDC settle (T1.6/T5.1). The invoice's
  `qr_png_blake3` stays zeroed until T1.4 renders — the honest placeholder
  Plan 015 shipped.
- No `kat:cfg:v1` widening: the per-account open cap is a service-plane
  ServiceEnv knob. The row shapes are untouched (Plan 015 goldens hold).
