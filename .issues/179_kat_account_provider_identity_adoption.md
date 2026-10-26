# Issue 179 — `kat_account` adopts the provider-identity substrate (numeric-id GitHub binding + the signing session)

**Status:** OPEN — filed 2026-10-27 (amo, auth-plane follow-through
session). Scoped consumer-adopts ticket for
[`riir-auth/.plans/001_provider_bound_biometric_identity.md`](../../riir-auth/.plans/001_provider_bound_biometric_identity.md)
T9's by-design split ("consumers (riir-refine `kat_account`,
seal-editor, riir-rethink) adopt via their own follow-ups after the
substrate lands" — the substrate IS landed: T2–T5, `87f8221`..`77259fe`).

**Dependency + sequencing:** the SERVER route hosts land first — dapps
[139](../../riir-dapps/.issues/139_totp_sensitive_route_enforcement.md)
(leased: zed-m3-queue-cycle, plan 053) + dapps
[140](../../riir-dapps/.issues/140_recovery_authority_class.md) (the
recovery authority class; carries the provider bind/unbind route host
per plan 053 T0.5 item 4). Until a bind route exists, a client-side
binding has nowhere to be honored — the adoption below is deliberately
split into a NOW half (local-only, no wire) and an AFTER half (wire).

Context: [issue 175](../../.issues/175_refine_mainnet_account_migration.md)
(the live mainnet migration) is leased through 2026-10-28T13:30Z for its
T-PREFLIGHT scope with "no code changes in this lease" — this issue does
NOT touch 175's operational tasks (T-E2E/T-MIGRATE/T-POST/T-KEYS are
owner-adjacent wire ops); it files the CODE adoption so the two never
collide on one lease.

## The substrate (landed in riir-auth — consume, never copy)

- `AccountState` v3 with additive `providers: Vec<ProviderBinding>`
  rows (`{ provider, numeric_id, login_display, bound_at_unix }`) —
  the record-integrity law: the rows are UNGATED fields so a
  feature-off rewrite cannot silently destroy them; only the gated
  flow CREATES them.
- `provider_login` (feature `provider_login`): the GitHub RFC 8628
  device flow over an injectable transport; `ProviderIdentity {
  numeric_id, login }`; `bind_provider` = idempotent refresh /
  server-refusal posture on conflict.
- `signing_session` (T2): the 15-min ABSOLUTE-TTL in-memory session —
  wallet-class ops (`OpClass::Wallet`) are STRUCTURALLY refused at the
  seed door; warm check ~46 ns (riir-auth Bench 006).
- `rebind` (T4 client half): `RebindClaim` legacy→numeric two-step,
  SSHSIG namespace `riir-auth:rebind:v1`, shared `REBIND_REQUIRED`
  refusal text.
- `totp` (T5): `totp_sha256`/`verify_totp` — RFC 6238 SHA-256/8-digit/
  30 s, replay-refusing typed verdicts, wasm32-clean.

## NOW half (local-only, no server dependency — claimable today)

1. **`cargo refine login`'s record gains the binding columns** — after
   a successful login, IF the record carries `providers` rows (written
   by a provider_login flow), `account`/`--info` renders the bindings
   (numeric id + display login + bound_at) through the ONE formatter
   (`account_line` law — computed labels, never hand-padded). Read-only
   adoption: refine never writes binding rows itself until the wire
   half lands. This is the kat_account slice of "every binary that can
   read the record can preserve the rows" — refine ALREADY preserves
   them by owning the v3-compatible struct; making them VISIBLE closes
   the silent-data class.
2. **The byte-preserve test** — a record fixture WITH provider rows
   round-trips through refine's account load/save UNCHANGED (the
   feature-off-rewrite hazard, asserted for the refine consumer
   specifically).
3. **The signing-session seam, ADOPT-where-beneficial survey** — the
   mine/push/burn tails run unattended today (cron/idle-loop); a
   session TTL gate on THOSE is a behavior change to schedule-bearing
   lanes and is OUT of scope here. The survey records which refine ops
   WOULD be session-gated if the owner arms it (the account migrate
   lane: issue 175's T-WIRE CLI is the first candidate — the dual-sign
   request is the wallet-class op the session class exists for).
   Report-only; arming is owner-gated.

## AFTER half (blocked on the dapps route hosts — one lease each)

4. **`provider_login` wiring** — a `cargo refine login --provider
   github` arm (device flow) that CREATES the binding rows (numeric id,
   display login). Blocked on: the bind/unbind route host (dapps 140).
5. **The re-bind lane surface** — when a caller hits a name-only
   refusal, quote `REBIND_REQUIRED` VERBATIM (byte-equal; the shared-
   string law — a drift breaks the client remedy) and offer the re-bind
   two-step. Blocked on: the recovery route host (dapps 140) + a
   server-issued nonce seam.
6. **TOTP on the migrate/consent routes' client half** — the
   `kat_totp_client` companion (plan 053's honest-limits split names
   refine/riir-kat as its home). Blocked on: plan 053's wire contract
   landing (shapes pinned in its T0.4: enroll signed over
   `kat:totpenroll:v1:`, disarm + current-step code).

## Gates

- (1)/(2): the fixture round-trip is byte-equal both feature postures;
  the render adds zero columns when `providers` is empty (byte-identical
  `account` output — the additive-law test).
- (3): survey lands as a table in this issue, not code.
- (4)–(6): each carries its own claim line + the blocking issue's
  number; none re-derives substrate (the consume-never-copy law).

## Non-goals

- No wire ops (175 owns the live migration; T-MIGRATE is owner-adjacent).
- No keychain/biometric custody adoption in refine — the machine key is
  the refine posture by design (`cargo refine login` adopts in place);
  biometric custody is a local-device surface (auth issues 007/008,
  owner-gated) that refine's headless/idle lanes explicitly must NOT
  take (a prompt-on-boot lane would break the idle loop).
