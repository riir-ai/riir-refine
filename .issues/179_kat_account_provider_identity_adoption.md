# Issue 179 — `kat_account` adopts the provider-identity substrate (numeric-id GitHub binding + the signing session)

**Status:** OPEN — filed 2026-10-27 (amo, auth-plane follow-through
session). **NOW half LANDED 2026-10-27 (`95bc8c18`)**: items 1–3 done —
the render + the byte-preserve tests (`tests/kat_account_g9.rs` g9f) +
the survey table below; the claim record `179_claim_now_half.md` removed
per the noise rule. The AFTER half ((4)–(6)) stays blocked on the dapps
139/140 route hosts; item 6's client half is claimed (`amo-idle-totp`).
Scoped consumer-adopts ticket for
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

**Claimed:** zed-amo-179-residue, 1793055600, expires 2026-10-29T22:40:00Z — scope: item (6) RESIDUE ONLY (the `kat_consent_client.rs` env affordance landing + the riir-kat stale-draft tree reconciliation); items (1)–(5) untouched; 175's operational lease unaffected (no migration ops here).

Context: [issue 175](175_refine_mainnet_account_migration.md)
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

   ✅ **LANDED (survey table, folded from the `95bc8c18` session record):**

   | refine op | class | session-gate fit | note |
   |---|---|---|---|
   | `account migrate --github` (dual-sign filing) | WALLET (a key-identity act on the ledger) | STRONGEST — the exact `OpClass::Wallet` the T2 session refuses to seed | issue 175's T-MIGRATE; would require the operator to re-biometric every filing attempt within a 15-min window |
   | `--fix` compile-gated writes | local code mutation | NO — never a seed door; the fixer runs unattended (cron/idle) and a TTL gate would break every schedule-bearing lane | disclosed, deliberately out of scope |
   | `--mine`/sync push tails | network write, non-money | WEAK — the burn ack is accounting, not authority | a session here adds friction without an authority win |
   | `stake`/`pay`/`buy`/`topup` money verbs | WALLET-class on the wire | STRONG — the seed door exists precisely for these | but they are interactive verbs today; the session's ABSOLUTE TTL means a fresh biometric per 15-min window |

   Verdict: the only ops the session class fits WITHOUT breaking the
   unattended lanes are the interactive wallet verbs (`migrate` filing
   included). Arming any of them is an owner call (a behavior change to
   every schedule-bearing workflow is NOT implied); nothing here arms.

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
   `kat:totpenroll:v1:`, disarm + current-step code). **UNBLOCKED
   2026-10-27: dapps 139 LANDED (`eae0f05`, plan 053 T8) — the route
   host + the wire contract exist.**

   **Lease closed:** amo-idle-totp (claimed 2026-10-27, epoch
   1793043273) — item (6) LANDED 2026-10-27 03:36+07, before expiry;
   reshaped zed-amo-idle epoch 1793052760 (the push stays the arbiter).
   - Scope: riir-kat `kat_protocol_totp.rs` (feature-free wire: the
     enroll/disarm message composers v1+v2, the body composers, the
     base32 seed decode from the one-time URI, the response parsers —
     byte-symmetric with the landed
     `cloudflare/kat-service/src/totp_routes.rs` composers) +
     `kat_totp_client.rs` (the `client`+`kat_transport` gate, the
     `kat_consent_client` shape: enroll mint/confirm legs, disarm,
     `current_code` via the ungated `riir_auth::totp_sha256`, the
     `X-Kat-Totp` header const) + the two consumption sites (refine
     `kat_migrate_client` header attach — the OLD account's armed
     factor, `--totp-code`/`RIIR_KAT_TOTP_CODE`; the consent heartbeat
     env affordance, unset = byte-path unchanged) + the
     `cargo refine totp --enroll|--disarm` CLI verbs (interactive; the
     operator's authenticator app holds the seed — refine stores NO
     seed, the headless-lane custody non-goal holds).
   - Plan-337 posture: authored against the elephant window-3
     saturation (no cargo in this window — the R1 watcher lane holds
     the box). GATE RECIPE for the landing window: `cargo clippy -p
     kat-* --all-targets -- -D warnings` × (default / --features
     kat_transport / --no-default-features) → `cargo test -p
     kat-* --lib totp` + the refine lib tests → `cargo fmt` on the
     touched files → the release-set binary rebuild + `--version`
     stamp (§0 bin-age — the client surface IS release-set) → THEN
     the landing commit + push; the wip checkpoint stays LOCAL until
     the gates pass.
   - AUTHORED 2026-10-27 (this session, checkpoint `6684cca` LOCAL in
     riir-kat + the refine tree): riir-kat `kat_protocol_totp.rs` (374
     lines, 7 tests) + `kat_totp_client.rs` (189 lines, 2 tests) + the
     lib rows; refine `kat_migrate_client.rs` (`totp_code` on
     `FileParams`, `post_json_with_totp`, the `totp_required` remedy
     line, `fetch_ledger_id` pub) + `kat_consent_client.rs` (the env
     affordance) + `src/bin/heal.rs` (`totp --enroll|--disarm --code`,
     `--totp-code`, usage) + the lib re-export rows. The sibling's
     dirty `kat_protocol_migrate_tally.rs` untouched (staged_set_audit
     verified the split). NOTHING COMPILED YET — the recipe above is
     the landing gate.
   - **LANDED 2026-10-27 03:06/03:36+07** (supersedes the
     NOTHING-COMPILED-YET note above): riir-kat `45cb2d3`
     (`kat_protocol_totp.rs` 477 lines + `kat_totp_client.rs` 435 lines,
     both grown through the gate recipe, + the lib rows + issue 007
     filed) → refine `c9923d2f` (`src/bin/heal.rs` +281 — the
     `kat totp enroll|disarm` verbs + `--totp-code`;
     `src/kat_migrate_client.rs` +121 — `totp_code` on `FileParams`,
     `post_json_with_totp`, the `totp_required` remedy line,
     `fetch_ledger_id` pub; the AGENTS.md CLI row). dapps recorded the
     pair (`93cd35c`). **RESIDUE (not landed, verified on origin
     2026-10-27): the consent heartbeat env affordance — riir-kat
     `kat_consent_client.rs` carries the authored edit
     (`RIIR_KAT_TOTP_CODE` → `X-Kat-Totp` on the consent route for
     ARMED accounts, unset = byte-path unchanged) ONLY in its dirty
     worktree; origin has ZERO TOTP mentions in that file. The env form
     is the residue; the `--totp-code` flag form landed. Finish = land
     that one-file edit when the auth lane's tree clears (or descope
     with the reason here). The dirty tree is the auth lane's WIP — not
     this lane's to commit.**

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
