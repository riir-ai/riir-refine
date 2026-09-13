# Issue 103 — sync payload v3: `PayloadRow` gains `domain` (the corpus-view Phase-2 axis; deprecates the server-side seed mapping)

**Status:** OPEN — P2 pull-gated on the corpus-view Phase-1 deploying + filling (riir-dapps Plan 026 T1–T5 landed 2026-09-13; the devnet deploy is owner-eyes-gated on the ~805-compiled → network-observed magnitude). Not a blocker for anything live today.

## Why

The corpus view (riir-dapps Plan 026) buckets distinct pattern classes per
domain. Phase 1 maps rule name → domain SERVER-SIDE from the committed
`seed_corpus.json` — the same staleness class the plan kills, in milder form:
a new domain's rules degrade to `other` until the printer's export is re-run
and the worker redeployed, and every unmapped name lands in `other` forever.

The healer KNOWS each rule's domain at sync time (its compiled rule→domain
table). Carrying it in the payload makes the view axis exact with zero server
state.

## The change

1. `src/kat_sync_payload.rs`: `PayloadRow` gains `domain: String` (the
   emitting domain's id — `clippy`, `rust_perf`, `kernel_opt`, `sec`,
   `docker`, `rustc_errors`, `dist`; empty string = unknown, which the
   receiver treats as `other`).
2. `payload_version` 2 → 3 (`SYNC_PAYLOAD_VERSION`). The Ed25519 signature
   covers `payload_version`, so a bump re-pins BOTH sides' golden payloads
   in the SAME commit window (the REDACT_VERSION precedent, kat_mining.rs's
   header comment).
3. Dual-pinned goldens re-pinned together:
   - this repo: `kat_sync_payload::tests` (`golden_payload_bytes_are_pinned`
     + the builder fixtures);
   - riir-dapps: `tests/kat_mining.rs` `GOLDEN_PAYLOAD` (+ `GOLDEN_COMMITMENT`
     if row-line bytes change — they do, the field is in the row line) and
     `tests/kat_corpus_view.rs`'s duplicated builder.
4. Receiver (riir-dapps `ingest_payload`): accept BOTH v2 (domain = None →
   seed mapping, the Phase-1 path, kept for the fleet's transition window)
   and v3 (domain = Some → exact). Never a hard cutover — mixed fleets are
   the normal state; the seed mapping retires when the fleet's v2 share is
   zero.
5. `QueueRow` gains the optional domain (wire-additive codec field — a v2
   row decodes with None; the corpus-view pass prefers it over the seed
   map).
6. The corpus-view registry becomes append-stable across the transition:
   v3 domains the registry has never seen APPEND (the registry-anchored slot
   law, corpus_view.rs); `other` keeps shrinking as the fleet updates.

## Verification hooks

- The dual-pinned golden is the drift tripwire: a v3 bump that changes only
  ONE side's golden fails the other side's test — coordinated window.
- riir-dapps `tests/kat_corpus_view.rs`: a v3-ingested row lands in its
  exact domain slot with NO seed mapping supplied.
- The seed-table fallback still serves v2 clients (a mixed-fleet test: one
  v2 row + one v3 row, both bucketed correctly).

## Non-goals

- No domain VALIDATION receiver-side beyond the registry's 8-slot cap (the
  healer's own table is the authority; a lying client mislabels only its
  own rows' view bucketing — the prove plane is unaffected).
- No backfill: pre-v3 rows keep their seed-mapped/`other` classification
  (the arrivals view is approximate by declaration; the registry-anchored
  never-re-label law forbids rewriting history).
