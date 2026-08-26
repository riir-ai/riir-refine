# Plan 028 — KAT Phase 0 T0.4: the pending-record estimator

Executes [Proposal 005](../.proposals/005_kat_heal_service_network.md) **Phase 0 T0.4 only** —
the local, deterministic pending-record estimator: "count rows not yet covered by the
last-synced `batch_commitment` watermark (persisted alongside payloads), including rows
blocked by `RedactionGate` as a separate count", rendered under **caveat #15's
service-credit framing — no `$` beside an accrual**.

## Scope discipline

- **In scope:** T0.4 — `src/kat_pending_estimator.rs` (the pure estimator + the sync
  watermark type it compares against + the watermark file persistence helpers T1.6's
  `sync` command will call after a successful push), the provisional service-credit
  estimate with its own version discipline, the caveat-#15 framing renders + their
  adversarial no-`$` gate, unit tests, the G4-class integration binary, the CI Layer 5c
  row.
- **Out of scope:** the `cargo heal sync` subcommand + transport + the watermark WRITE
  call site (Phase 1 T1.6 — this module ships the read side + the constructors that
  push path will use); the account CLI display line (T1.6 owns the account surface
  extension; the framing functions land now so that surface consumes them verbatim);
  server-side proof status (Phase 2 — the estimator is explicitly LOCAL: it cannot know
  quorum outcomes, only sync coverage); any mint/reconcile logic (G9d holds —
  accounting-only, the estimate is a display projection, never a balance).
- **No wall-clock, no network, no store mutation.** `estimate_pending` is a pure
  function of `(rows, gate, origin, watermark)` — identical inputs → identical output
  (the Decision's constraint; the `build_at` determinism precedent). The loader
  (`SyncWatermark::load`) is IO, and the estimator never touches it.

## Decisions (recorded for the reviewer)

1. **"Pending" means NOT COVERED BY THE LAST SYNC, partitioned by what can still be
   proven.** Proof status is server-side (Phase 2 quorum) — locally unknowable. The
   honest local partition of every store row is exactly three classes:
   - `blocked` — the gate would exclude it from the next payload → it can NEVER be
     proven until the policy changes. Counted separately, excluded from the accrual
     estimate (a blocked row's "value" is money that cannot exist — counting it would
     violate caveat #15 harder than the mock it fixes).
   - `pending_unsynced` — gate-passed, current projection NOT covered by the watermark
     (new row, or changed since last sync: counts/tier/text moved).
   - `synced_awaiting_proof` — gate-passed, current projection already covered (the
     server holds this exact row; quorum outcome unknown locally).
   `records_pending_proof = pending_unsynced + synced_awaiting_proof` — BOTH classes
   are unproven from the local seat; the accrual estimate covers that sum (it must NOT
   drop when a sync moves rows between the two classes — value only leaves the accrual
   when the server proves or rejects, Phase 2).
2. **Coverage = per-row digest of the PAYLOAD PROJECTION, keyed by the store's
   identity.** Watermark map: `"{lint_key}:{shape_id}" → blake3_12(serde JSON of the
   PayloadRow)` — the payload row (not the raw `TrajRowDoc`) is what the server holds,
   so it is what "already sent" must be measured against: a policy change that alters
   redaction re-pends every affected row (honest — the server's copy is stale), and a
   verdict-count change re-pends the row (new evidence the server lacks). serde
   JSON of a struct is field-order-stable; `gate_row`'s findings order is fixed
   (field order × fixed rule list) — deterministic within a build, and the file's
   `sync_state_version` stamps the scheme (a scheme change re-pends everything, the
   fail-safe direction).
3. **The estimator runs THE BUILDER'S OWN `gate_row`** (made `pub(crate)` for this) —
   same code path as the next real payload, so estimator/payload agreement is by
   construction, not by discipline (no second projection to drift; the
   one-derivation lesson from T0.3's `shape_id`).
4. **The watermark file: `<root>/.heal/kat_sync_state.json`, load-never-aborts.**
   Missing → `empty()` silently (bootstrap is the normal pre-T1.6 state — every row
   pending). Corrupt / version mismatch → `empty()` + surfaced warning (the
   `LoadedPolicy` pattern — the estimator stays pure; printing belongs to the CLI).
   Recovery-to-empty is deliberately NOT salvage: unlike `charged_kat` (frozen history
   still valid under its own model), row digests from another scheme are
   uninterpretable, and the failure direction is over-sync (re-send everything) —
   cheap and safe — never under-sync (silent data loss). `SyncWatermark::load`
   returns `(Self, Option<String>)` so tests pin the warning without capturing stderr.
5. **`SyncWatermark::from_payload` is the T1.6 constructor** (post-push watermark from
   the built payload's parsed rows + its commitment) — shipped now so the sync path
   that lands in Phase 1 composes THIS derivation, not a fresh one (one derivation,
   both sides).
6. **The provisional rate is a version-stamped placeholder, honestly labeled.** The
   real per-record value is epoch-pool + novelty-class dependent and is frozen at T2.3
   (reward schedule) — Phase 0 cannot know it. `ESTIMATE_VERSION` +
   `ESTIMATE_CHANGELOG` + `PROVISIONAL_KAT_PER_PENDING_RECORD = 35` (the §Account
   mock's own implied ratio: ~42,000 KAT / 1,204 records ≈ 34.9), documented as
   provisional-until-T2.3. Independent of `SYNC_STATE_VERSION` on purpose (the
   meter/charge two-axis precedent: a rate change must not invalidate the watermark,
   and a watermark scheme change must not read as a reprice). The estimate saturates
   rather than overflows.
7. **Framing renders are load-bearing (caveat #15), gated by their own adversarial
   test.** `pending_line()` = `Records pending proof: N (est. X KAT service credit if
   proven — provisional rate)` — carries "service credit" + "if proven" + the
   provisional marker, and contains NO `$` anywhere. `blocked_line()` = count + policy
   hint, and carries NO KAT figure (blocked never earns). No render in this module
   ever emits `$` (the `$`-saved line belongs to consumed healing — the charge/meter
   surface, not an accrual); pinned by `no_dollar_sign_in_any_render`.
8. **Feature gate: `all(kat_service, latent_retrieval)`** — identical surface to the
   payload module (consumes `RedactionGate` + `TrajRowDoc` + the payload's
   `PayloadRow`/builder). CI coverage standing: Layer 5b `--lib --all-features` +
   Layer 5c's integration binary under `--all-features`.

## Honest scope (documented in the module, not papered over)

- The accrual estimate is a **projection, not a promise**: novelty weighting, epoch
  pool decay, the ×3 first-discoverer bonus, and the documented-miss-class floor all
  live in Phase 2's frozen schedule; the provisional rate ignores all of them and will
  be WRONG by design until T2.3. The render says "provisional" on the same line as the
  number — the disclaimer travels with the figure, never in a footnote.
- `synced_awaiting_proof` can overcount post-proof rows (a row proven + settled on the
  server still reads "awaiting" locally until settlement data flows back — Phase 2's
  settlement channel). Locally undecidable; documented at the field.
- The digest scheme is sensitive to serde float formatting acrossserde releases in
  principle (ryu shortest-form has been stable); the version stamp is the migration
  path if that ever moves (re-pend everything, over-sync once).

## Numbering

Plan 028 (verified free at write time — `.plans/.highwater` was 27, bumped to 28 in
this commit). The live **026 dual-allocation** flagged by Plan 027 remains an owner
call; this plan does not touch it.

## Tasks

- [x] T1 `src/kat_sync_payload.rs`: `SyncPayloadBuilder::gate_row` → `pub(crate)` (the
  estimator projects through the builder's own path — no second derivation)
- [x] T2 `src/kat_pending_estimator.rs` behind `all(kat_service, latent_retrieval)`:
  `SYNC_STATE_VERSION`/`SYNC_STATE_CHANGELOG` + `SyncWatermark` (`empty`/`load`/`save`/
  `from_payload`/`covers`), `ESTIMATE_VERSION`/`ESTIMATE_CHANGELOG` +
  `PROVISIONAL_KAT_PER_PENDING_RECORD`, `PendingEstimate` +
  `estimate_pending` (pure) + the caveat-#15 renders (`pending_line`/`blocked_line`,
  thousands-separated), unit tests (pure-function determinism, digest stability,
  recovery paths, framing)
- [x] T3 `src/lib.rs`: module declaration + doc comment (the dual-gate reason, the
  T1.6 consumer)
- [x] T4 `tests/kat_pending_estimator_g4.rs`: adversarial fixtures — empty store;
  all-blocked (planted secrets + deny-all; accrual stays 0, blocked never earns KAT);
  no watermark (bootstrap: everything pending_unsynced); fully-covered watermark
  (estimate holds — accrual does not drop on sync); changed-row re-pend (verdict
  bump); new-row pend; policy-change re-pend (redaction difference moves the digest);
  corrupt watermark → recover-to-empty + warning; version-mismatch watermark →
  recover-to-empty; save→load round-trip; `from_payload` covers exactly the payload's
  rows; `no_dollar_sign_in_any_render` + accrual-line language gates; determinism
  (same inputs → identical estimate + lines, twice)
- [x] T5 `scripts/ci_feature_guard.sh` Layer 5c: add `--test kat_pending_estimator_g4`
- [x] T6 validation: `cargo test --features kat_service,latent_retrieval --lib` (module
  unit tests) + the G4 binary + full `ci_feature_guard.sh` all layers + clippy clean
  at both feature extremes
- [x] T7 Proposal 005 T0.4 checkbox → `[x]` with this plan's record
