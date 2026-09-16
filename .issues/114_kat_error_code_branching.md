# Issue 114 — the kat lane branches on error PROSE; a stable `code` now ships beside it

**Status:** OPEN — P2, non-urgent by construction. The wire change is ADDITIVE
and already deployed-ready: nothing in this repo breaks if this issue is never
done. Filed by the riir-dapps Plan 030 session (Agave mining Phase 1 T1.4.c),
which owns the server half and deliberately did NOT edit this repo — the
intake lane is actively written here.

## Why

riir-dapps Plan 030 T1.4 (Proposal 005 D6, mined from Agave's
`TransactionError` discriminants) landed machine-readable error codes on every
kat-service money route. Before it, a client that wanted to distinguish
"re-sign this" from "fix a field" from "you already won, stop retrying" had
only the `error` string — so **rewording a message is a docs edit that
silently changes client behaviour**, and two genuinely different failures that
happen to read alike are indistinguishable.

## The wire, as it ships today

Additive. An error body was, and still is:

```json
{"error": "<prose>"}
```

and money routes now add one field:

```json
{"error": "burn: signature verification failed", "code": "bad_signature"}
```

A client parsing only `error` is unaffected — that is pinned by a contract
test on the riir-dapps side, as is the fact that a route with **no** code
still emits the legacy two-field shape.

## The table

One code per terminal failure CLASS, never per message and never per route.
The rule: *if the client would take the same action, it is the same code; if
it would take a different action, it must be a different code even when the
prose is similar.*

| code | meaning | what the client should do |
|---|---|---|
| `bad_json` | the body was not the JSON the route expects | fix the request |
| `bad_field` | a field is missing, or the wrong shape/width | fix the request |
| `bad_signature` | an Ed25519 signature did not verify over the canonical message | **re-sign** |
| `unauthorized` | the ops bearer token was absent or wrong | get a token |
| `unconfigured` | the deployment never armed this surface (503, fail-closed) | **not** a retryable outage — nothing changes until an operator acts |
| `not_found` | no row at that id/epoch/binding | stop |
| `already_settled` | a replay of something already terminal | **never retry** — the first one won |
| `terminal_state` | the row's state machine forbids this transition | stop |
| `insufficient_balance` | the balance cannot cover the debit | get funds |
| `account_banned` | governance banned the account | stop |
| `rate_limited` | a rate limit / daily cap / cooldown | **retry later** — the one refusal where waiting helps |
| `ledger_refused` | a domain refusal that is none of the above | surface it |
| `host_unavailable` | the host or store failed | the only class meaning *nothing is wrong with your request* — retry |

Codes are **stable forever** once shipped. Renaming one is a breaking change;
adding one is not.

Source of truth: `riir-dapps/cloudflare/kat-service/src/error_code.rs`
(the module doc carries the same rules and the reasoning). Covered routes:
`/sync`, `/burn`, `/claim`, `/gov/vote`, `/stake/*`, `/payreq/*`, the
treasury/replay/internal money legs, and the whole `/art/*` family.

## The change here

1. Find the kat lane's refusal handling (the `kat_service` / `kat_transport`
   feature paths in `src/bin/heal.rs` and friends) and branch on `code` where
   it branches on prose today.
2. Where no `code` is present — an older deployment, or an unassigned route —
   keep the current prose behaviour. The field's ABSENCE is a supported
   answer, not an error.
3. Two branches are worth having even if nothing else changes, because they
   are the ones where prose-matching gets the retry policy backwards:
   `rate_limited` (retry later) vs `already_settled` (never retry), and
   `host_unavailable` (retry — your request was fine) vs every refusal.

## Known coarseness on the server side (read before over-fitting)

`DappError::LedgerRefused { reason: &'static str }` is ONE variant for every
store failure, corrupt row, ban and balance refusal. The server recovers the
three host-failure reasons by exact match on riir-dapps' own literals;
everything else collapses to `ledger_refused`. So **`account_banned` and
`insufficient_balance` are emitted only by routes that detect those
conditions themselves** — do not assume a ban always arrives as
`account_banned`. Splitting that properly is a `DappError` variant split in
riir-dapps with its own blast radius, deliberately not smuggled into T1.4.

## One defect this fixed, worth knowing about as a client

`RouteResp::error` interpolated its reason RAW into `{"error":"…"}`, so the
four routes whose prose contains a quote — `rail must be "solana" or
"stripe"`, `choice must be "yes" or "no"`, `kind must be "ban" or "unban"`,
and the burn-watermark ack — answered with **invalid JSON** on every refusal.
A client's `JSON.parse` threw, so an actionable 400 arrived as a parse
failure. Both constructors escape now. If this lane has a workaround for
unparseable 400 bodies, it can go.

## Gates

- Branch on `code` where prose is matched today; keep the prose fallback.
- A test per branch that the ABSENT-code path still behaves as it does today
  (an older deployment must not regress).
