# Plan 023 — KAT Phase 0 T0.5: the code-token meter

Executes [Proposal 005](../.proposals/005_kat_heal_service_network.md) **Phase 0 T0.5 only** —
the alloc-free, version-stamped, raw-span-byte word-token meter (the "KAT bill"). Opened
2026-08-26 after Issue 037's close cleared the Phase 0 blocker (A1 cadence, 3rd distinct heal
day).

## Scope discipline (why this plan is one task)

- **In scope:** T0.5 — the meter + its G9 fixtures. `kat_service` gets its feature flag
  (empty — no deps for the meter; `qrcode`/HTTP arrive with T0.1+ per the Issue-041 T4
  audit: third-party crates are not boundary-gated, no new riir dep).
- **Out of scope (later Phase 0 tasks, own commits/plans):** T0.1 account key +
  `login`/`account`, T0.2 redaction gate, T0.3 sync payload schema (what the meter's
  burn eventually reconciles against), T0.4 pending-record estimator, T0.6 the
  charge-model decision (consumes meter events — built after the producer exists).
- **Ordering rationale (session decision):** T0.6's charge model consumes meter events,
  so the meter (producer) lands first as a standalone unit with its own persistence.
- **Phase 3 (fixer node) untouched** — blocked on Issue 040's arms (owner CF creds / GPU
  window).

## Design (from Proposal 005 §KAT + G9, restated as the contract)

1. **Unit:** 1 KAT = 1 word token of code processed. A **word token is a maximal run of
   bytes that are not ASCII whitespace** (`u8::is_ascii_whitespace`: space, `\t`, `\n`,
   `\r`, `\f`). Counted over **raw bytes of the source the healer processes** — NOT an
   AST, NOT parse-dependent.
2. **Raw-bytes is a correctness requirement (G9c):** parse-dependent counting bills zero
   on unparseable files — exactly the L4 miss class routed to the most expensive path
   (Issue 030). Unparseable spans MUST bill non-zero.
3. **Host-independence (G9a):** line endings must not move the count — satisfied *by
   construction*: `\r` is ASCII whitespace, so CRLF/LF/CR checkouts of the same repo
   produce identical token counts. Path separators never appear in file content. The
   counter is a pure function of bytes.
4. **Version stamp (G9b):** `METER_VERSION` + `METER_CHANGELOG` (every version that
   changes the count carries an explicit delta entry — never a silent reprice). The
   version rides the persisted meter file.
5. **Accounting-only (G9d):** client-side counts never mint or spend; the meter is a
   local burn ledger to be reconciled at sync (T0.3+). No balance math here.
6. **Persistence:** repo-local `<root>/.heal/kat_meter.json` (same root family as the
   trajectory store — `RIIR_HEAL_TRAJ_ROOT` → first path arg's nearest `.git` ancestor →
   cwd). Missing file → fresh; corrupt/unknown-version → warn + fresh (the
   `EvolveRecorder` envelope precedent: never aborts a heal run).
7. **Billing surface (T0.5):** every `.rs` source read by the main heal loop (the fix +
   suggest passes share one read — one bill per read; fixpoint passes re-parse
   already-billed bytes internally) + every `.wgsl` source read by the non-syn scan.
   `--fix-compile` (the rustc_errors driver) reads files through its own driver —
   follow-up wiring when that path gets a meter (noted below).
8. **Opt-in + kill-switch:** feature `kat_service` (default-off — G3: default build
   bit-identical, no meter code compiled); env `RIIR_HEAL_KAT_METER=0|off|false|no`
   disables at runtime; `RIIR_HEAL_KAT_METER_FILE` overrides the file path (tests/UX).

## Tasks

- [x] T1 `kat_service` feature in `Cargo.toml` (empty feature — meter needs no deps;
  serde/serde_json already main deps)
- [x] T2 `src/kat_meter.rs`: `count_word_tokens` (alloc-free, ~30 LOC) + `METER_VERSION` +
  `METER_CHANGELOG` + `KatMeter` (open/record/save/session accounting) behind
  `kat_service`
- [x] T3 `src/lib.rs`: module declaration + re-export
- [x] T4 `src/bin/heal.rs` wiring: extract `resolve_heal_root` (shared with
  `open_traj_store`), meter open (env kill-switch + path override), per-file record in the
  main `.rs` loop + the `.wgsl` loop, save + one summary line at exit
- [x] T5 G9 fixture tests (`tests/kat_meter_g9.rs`, self-gated `#![cfg(feature =
  "kat_service")]`):
  - G9a bit-identity across repeated counts
  - G9a line endings (CRLF/LF/CR) do not move the count (the host-independence pin)
  - G9c unparseable spans (binary garbage + invalid UTF-8 + broken Rust) bill non-zero,
    deterministically
  - G2 alloc-free via `CountingAllocator` (0 allocs on the count call)
  - persistence round-trip (cumulative + sessions survive reopen; version stamp rides the
    file)
  - definition pins: empty/whitespace-only → 0; non-ASCII bytes are token bytes (Unicode
    identifiers are single tokens)
- [x] T6 validation: `cargo check/clippy/test --features kat_service` green; default +
  `--all-features` builds unchanged; bin compiles with `kat_service` added
- [x] T7 update `.plans/.highwater` (22 → 23)

## Deferred `[-]` (recorded, not silently dropped)

- [-] Cross-host M3 ↔ Windows literal run: the property is *pinned locally* by the CRLF
  fixture (the only host-dependent byte difference for identical repo content is EOL,
  which the definition neutralizes by construction). The literal two-host run belongs to
  the first CI that gets a Windows runner (Proposal 003's release pipeline), where it
  becomes a real gate — not to this plan.
- [-] `--fix-compile` metering: the rustc_errors driver owns its own file reads; wire
  when the sync payload (T0.3) needs its burns (a driver without a sync target has
  nothing to reconcile).
- [-] T0.6 charge model (per-unit burn vs volume-tiered lease, Proposal 005 caveat #16):
  separate decision task — consumes meter events, lands after this producer.

## Validation record (2026-08-26, M3)

- `cargo test --features kat_service --lib` — **686/0** (default 677/0 baseline held; +9
  kat_meter unit tests)
- `cargo test --features kat_service --test kat_meter_g9` — **7/7** (G9a determinism,
  G9a line endings, G9a path separators, G9c unparseable×2, G9b version stamp,
  persistence round-trip, summary-no-currency)
- `cargo test --features kat_service --test kat_meter_g2_alloc` — **1/1** (0 allocs
  counting 1 MiB, CountingAllocator; own binary — no parallel-sibling pollution)
- `cargo clippy --features kat_service --all-targets` — **0 warnings** (10 pedantic
  findings fixed: 8× must_use, 1× missing_errors_doc, 1× uninlined_format_args —
  const-path arg in an escaped-quote template the healer conservatively skips)
- `cargo check --all-features --lib` green (combo safety); default build untouched
  (G3 — meter code not compiled)
- Bin wiring e2e (M3): `--suggest` on the sample fixture → `kat: 122 code word
  token(s) billed across 1 source(s) (meter v1, cumulative 122 — accounting only,
  reconciles at sync)`; second run accumulates (244, sessions 2);
  `RIIR_HEAL_KAT_METER=0` emits no kat line and leaves the ledger untouched
- En-route: `cargo heal --fix --write --verify` run over the new files (0 applicable
  edits — the pedantic doc/attribute class is outside the fixable corpus, correctly
  refused; trajectories recorded to the repo store)

## Honest notes

- The alloc fixture lives in its OWN test binary (`kat_meter_g2_alloc.rs`): the
  global CountingAllocator counts every thread, so parallel sibling tests would
  pollute the before/after bracket (measured: 58 phantom allocs from the tempdir+
  serde siblings before the split). The `t33` convention, hardened.
- `open_at` + `RIIR_HEAL_KAT_METER_FILE` exist for testability + user control; the
  default location is repo-local `<root>/.heal/kat_meter.json` beside the trajectory
  store (shared `resolve_heal_root`, now also serving the kat path).
- The healer itself (fix path) does NOT yet meter its runs — `cargo heal` (the global
  binary) is built with `fix_verify,clippy_verify` features, not `kat_service`.
  When the service goes live, the release build adds the feature (Proposal 003's
  release pipeline owns that flag set).
