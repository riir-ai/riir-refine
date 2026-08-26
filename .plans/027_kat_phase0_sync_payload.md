# Plan 027 — KAT Phase 0 T0.3: the sync payload schema

Executes [Proposal 005](../.proposals/005_kat_heal_service_network.md) **Phase 0 T0.3 only** —
the versioned sync payload schema over [`TrajStore`] rows with the BLAKE3 batch commitment,
**consuming `RedactionGate::check` so no row is serialized into a payload without passing the
gate** (the consumer T0.2's gate was built waiting for — `lib.rs`'s comment says exactly that:
"T0.3's sync payload consumes it the same way").

## Scope discipline

- **In scope:** T0.3 — the wire schema + builder (`src/kat_sync_payload.rs`), the NDJSON
  envelope (header + gated row lines), the per-field gate check, the BLAKE3 batch commitment
  + its verify path, the adversarial G4 integration fixtures, the CI Layer 5c row.
- **Out of scope (later tasks, own commits/plans):** T0.4 pending-record estimator; the
  `cargo heal sync` CLI subcommand + transport (Phase 1 T1.6 — the payload BUILDER is the
  substrate it will call); the server-side ingest (riir-dapps T1.2); any mint/reconcile
  logic (G9d holds — accounting-only until the server exists).
- **No egress happens here.** The builder produces a `String`. Nothing in this plan sends
  anything anywhere — the transport is Phase 1. This is the schema + its enforcement point,
  landed before the first egress path exists (the correct order: the gate shipped in T0.2,
  the payload shape ships now, `sync` later composes both).

## Decisions (recorded for the reviewer)

1. **NDJSON, per the session decision.** One JSON object per line: line 1 = header
   (metadata + commitment), lines 2.. = gated rows. Matches the `.heal/` jsonl store
   convention (`trajectories.kv` rows, `redaction_log.jsonl`); streamable; debuggable with
   `head`/`jq`. `payload_version` is the SCHEMA version; the header ALSO carries
   `redact_version` (= [`REDACT_VERSION`] at build time) so the receiver knows which gate
   definition produced the verdicts — the two versions move independently by design (a gate
   rule change is not a schema change), and the alignment rule is: the header's
   `redact_version` is stamped from the compiled constant, never trusted from input.
2. **Feature gate: `all(kat_service, latent_retrieval)`.** The module consumes
   `RedactionGate` (behind `kat_service`) AND `TrajRowDoc` (behind `latent_retrieval` — the
   store's riir-neuron-db dep). `kat_service` alone stays dep-light (meter/account/redact
   unchanged — making `kat_service` imply `latent_retrieval` would bloat every KAT build
   with the riir-rag tree for nothing). CI covers the combo standing: Layer 5b runs
   `--lib --all-features`, Layer 5c runs the integration binary under `--all-features`.
3. **Wire row is built FROM `TrajRowDoc`, not a mirror.** The payload row carries a gated
   subset of the store row's fields; `from_gated` is the only construction path inside the
   builder, so field drift between store and wire surface is a compile error, not a silent
   omission (the "pin every reimplemented derivation against its source" lesson).
   `shape_id` = the STORE's own `shape_hash` (made `pub` in this plan) — the payload row is
   addressable by the store's row identity, no second derivation.
4. **Per-field gating.** Every text field that crosses the wire is gated independently:
   `shape`, `fix`, `lint_key`, `last_run_id`. First `Blocked` on any field excludes the
   whole row (the row is atomic — a half-row on the server is worse than no row). With
   `allow_redacted: true`, each field redacts in place; the row's gate metadata records
   kind + field + fingerprint (never bytes, never byte ranges — ranges are meaningless
   across fields). Real run ids (`run-{hex13}-{pid}`) are entropy-rule-safe by length.
5. **Deterministic commitments.** Included rows sort by `(lint_key, shape_id)`; the
   commitment is `blake3` (full 64-hex) over the exact row-line bytes (each line + `\n`).
   Same rows + same policy + same `created_unix` → byte-identical payload. The header
   carries the commitment (self-reference forbids covering itself); end-to-end
   authentication is Phase 1 transport's job — this commitment makes the ROW set
   tamper-evident + rebuild-verifiable.
6. **Blocked rows appear only as a count.** `rows_blocked` in the header + per-input-row
   outcomes on the builder result (for caller-side `RedactionLog` recording — the builder
   stays pure; logging policy belongs to the caller, the fixer_regate precedent). Blocked
   row DETAILS never cross the wire — the local log already has them, and the server has no
   use for rows it cannot accept.
7. **The builder is the sanctioned egress path.** Wire types are `pub` for the consumer
   side (parse/verify — the server parses these as data per the proposal) and for tests;
   producing a VALID payload (header + commitment over gated rows) is only possible via
   `SyncPayloadBuilder`. Nothing stops a hostile local process from hand-writing NDJSON —
   that threat is not ours to defend (the gate's threat model is accidental leakage through
   OUR surfaces, T0.2's contract).
8. **G4 meta-gate scope:** re-scanning a built payload with `scan_secrets` asserts over the
   ROW LINES (skip the header line) — the header's own 64-hex `batch_commitment` is our
   cryptographic metadata, exactly the documented fail-closed fixture-hash class; the G4
   property is about USER CONTENT. For redacted payloads the documented terminal-state rule
   applies (re-scan may re-flag a placeholder site; assert bytes-gone, not zero findings).

## Honest scope (documented in the module, not papered over)

- Inherited from the gate (T0.2's list, unchanged): no layer-stripping; line-based scanner
  (cross-line split secrets inside ONE field evade env/literal rules); committed hash
  literals in shapes/fixtures trip the entropy rule fail-closed.
- Cross-field reassembly is not scanned: a secret split across `shape`/`fix` cannot
  reassemble in the serialized form (JSON structure interleaves), but a secret split across
  two LINES within one field inherits the scanner's line-based limit.
- No transport, no retry, no resume cursor — Phase 1. The payload is a complete snapshot
  batch, not a delta stream (a delta/cursor design needs the server's idempotency contract
  first — `shape_id` is the upsert key it will use).

## Numbering note (owner call flagged)

`.plans/` carries a live **026 dual-allocation**: `026_dirty_encoding_write_elision_batch72.md`
(commit `e42df0f`, 10:37, landed on origin first) vs `026_kat_phase0_redaction.md` (commit
`44ec9c1`, 11:36). Both committed + referenced (the mining snapshot + Research 084 vs
Proposal 005 T0.2), neither baked into a test filename — the push-wins tie-breaker favors
batch-72, but renumbering touches a sibling session's committed docs in the shared checkout.
**Flagged as owner call** (the same-day `.research/` 083 dual-allocation precedent — the
sibling flagged rather than unilaterally renumbered). This plan takes 027 (verified free at
write time); `.highwater` bumped to 27.

## Tasks

- [x] T1 `src/traj_store.rs`: `shape_hash` → `pub` (the payload's `shape_id` derives from
  the store's own identity — no second derivation)
- [x] T2 `src/kat_sync_payload.rs` behind `all(kat_service, latent_retrieval)`: 
  `PAYLOAD_VERSION`/`PAYLOAD_CHANGELOG` (G9b), `PayloadHeader` + `PayloadRow` +
  `RowGateMeta`/`FindingMeta` (serde both ways), `SyncPayloadBuilder`
  (`new`/`build`/`build_at` — per-field gating, deterministic sort, commitment), 
  `SyncPayload` + `RowOutcome`, `verify_payload` + `parse_rows`, unit tests
- [x] T3 `src/lib.rs`: module declaration + doc comment (the dual-gate reason)
- [x] T4 `tests/kat_sync_payload_g4.rs`: adversarial fixtures — planted AWS key in
  `shape`, GitHub token in `fix`, secret in `lint_key` + `last_run_id`, mixed batch
  (clean rows survive byte-identical, dirty row absent + counted), `allow_redacted` path
  (placeholders + fingerprints in meta, finding bytes gone), no-policy deny-all,
  external-origin deny, tamper detection (commitment), determinism (same ts → identical
  bytes; input order shuffled → identical), aggregate zero-secrets meta-gate over row
  lines
- [x] T5 `scripts/ci_feature_guard.sh` Layer 5c: add `--test kat_sync_payload_g4`
- [x] T6 validation: `cargo test --features kat_service,latent_retrieval --lib` (module
  unit tests) + the G4 binary + full `ci_feature_guard.sh` all layers + clippy clean at
  both feature extremes
- [x] T7 Proposal 005 T0.3 checkbox → `[x]` with this plan's record
