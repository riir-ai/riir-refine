# Plan 040 — KAT T1.6: the corpus-lease CLIENT half

**Status:** DONE — landed `0a7cfd8` on develop (2026-08-28); validation
numbers in §Gates

Proposal 005 Phase 1 T1.6 — the client-side corpus lease consumer, per
§"Corpus sync-back lifecycle" (normative, proposal lines 105–147) + the
update-first phase + delta-scoped boundary reheat + QR/out-of-KAT warn UX +
`sync`/`update`/`claim` commands. The service side (T1.5) landed in
riir-dapps `src/kat/lease.rs` as a LIBRARY; the Worker/DO HTTP shell is
deferred — the client is built against the specified HTTP contract with a
local std-TCP mock server for tests (the fixer_regate / dapps kat_stripe
mock precedent).

## Boundary rule (load-bearing)

riir-clippy may NOT depend on riir-dapps. The lease wire format is a
**versioned serialization contract**: this module MIRRORS dapps'
`EpochManifest` serialization byte-for-byte (canonical_body / to_bytes /
decode, the domain separator `b"kat-lease-manifest-v1"`, commitment =
BLAKE3(domain ‖ canonical body)) with **pinned golden vectors derived from
the dapps side** (captured 2026-08-27 by running dapps'
`EpochManifest::build` on the dapps test shapes via a temp dump test —
provenance comments in the golden module). A drift on either side breaks
the pinned vectors — that IS the drift detector (the C# WireProtocol
golden-fixture precedent, riir-unity Issue 003).

## Feature gating

- `kat_service` (existing, dep-light: ssh-key only) gains the lease module
  with ZERO new deps (wire mirror = blake3, already core; store = std fs).
- NEW `kat_transport = ["dep:ureq", "dep:qrcode", "kat_service"]` — the
  HTTP transport (reuses the existing optional ureq dep) + the QR render
  (qrcode crate, third-party, NOT boundary-gated — the kat_service
  precedent). kat_service stays dep-light.

## Deliverables

- [x] D1 `src/kat_lease_client.rs` — the client library:
  - [x] §1 wire mirror (`LeaseManifest`/`LeaseSegmentDesc`/`LeaseError`/
    `LeaseVerdict`/`LeaseStage` — mirror of dapps `EpochManifest`/
    `SegmentDesc`/`KatLeaseError`/`LeaseVerdict`/`StagedSwap`)
  - [x] §2 golden vectors (cross-repo drift detector) — 5 pinned shapes
    captured from dapps @ 84dd049
  - [x] §3 corpus pack (the segment payload semantic contract, versioned)
  - [x] §4 `LeaseStore` — `.heal/corpus-lease/` (content-addressed segments +
    `current.json` pointer + `charge.json` lease-charge projection);
    ATOMIC install = write segments → rename pointer (swap-or-nothing)
  - [x] §5 transport trait + ureq impl (kat_transport) + `run_update_phase`
    (bounded; 304 fast path; 200 → diff → fetch → stage → verify → swap;
    tamper → reject + fall back local)
  - [x] §6 reheat planner (pure, deterministic: chunk-key → candidate-set
    compare; delta-scoped token billing) + `ReheatReport::itemized_line`
  - [x] §7 QR render + payment URL + claim queue + sync egress compose
- [x] D2 `LatentMatcher::from_enabled_domains_with_overlay` — additive
  constructor in `src/draft/latent_matcher.rs` (latent_retrieval): a
  leased epoch's packs REPLACE the compiled-in corpus per covered domain
  (epochs are full-corpus states per covered domain; feature-off domains
  ignore overlay entries) — 2 unit tests pin replace + no-statement
- [x] D3 CLI (src/bin/heal.rs): `update` / `sync` / `claim --tx <sig>`
  commands; `--update-timeout <s>` (default 4, 0=skip), `--no-update`,
  `--no-reheat`, `--corpus-epoch <E>` (pin, implies skip-check); the
  update-first phase at run START; the run-end reheat pass; the QR/warn
  UX at ≤10M / 0 remaining — e2e-verified offline (login/claim/sync)
- [x] D4 tests: 23 in-module golden/wire/store/reheat units +
  `tests/kat_lease_client_g10.rs` (kat_transport) — 13 std-TCP mock
  server fixtures (G10 a–e client halves + sync/claim wire + G3 silence
  + out-of-KAT + compat)
- [x] D5 CI: `kat_lease_client_g10` pinned in Layer 5c
- [x] D6 plan + Cargo.toml + docs (this file + feature comments)

## Env / kill-switches

- `RIIR_HEAL_KAT_SERVICE_URL` — base URL (default
  `https://kat.heal.gist.rs`; the QR pay URL rides the same host)
- `RIIR_HEAL_KAT_LEASE=0|off|false|no` — kills update + reheat (CI
  belt-and-braces; no test invokes the real network by construction)
- `RIIR_HEAL_KAT_LEASE_DIR` — override the store dir (tests)

## Gates (before commit)

- [x] G-A default build + `cargo test --lib` green, ZERO new default deps
  (712 passed; qrcode/ureq absent from the default tree)
- [x] G-B `cargo test --features kat_transport` green — 13/13 mock-server
  + G10 fixtures; all-features lib 1390 passed / 0 failed
- [x] G-C clippy `--all-targets` zero new warnings in touched files
  (healer-first discipline applied — 4 verified edits; remaining tree
  warnings are a sibling's untracked gp_acquisition WIP, verified not
  mine by path scoping)
- [x] G-D offline bit-identity VERIFIED e2e: kat features ON + offline +
  fresh store → healing output byte-identical to features-off (the only
  delta: the two pre-existing KAT accounting summary lines; stderr
  identical; the update phase silent by construction + pinned by the
  g3_offline_failure_is_silent_and_local_stands fixture)
- [x] G-E `scripts/ci_feature_guard.sh`: Layer 5c pins the new binary;
  the earlier all-features RED (3 pre-existing `compile_oracle`
  signature errors at clean HEAD e128100, proven by stash) was unblocked
  by a 1-line closure-coercion drive-by at the call site
- [x] G-F G10 client halves: (a) 304 fast path — no body, no charge,
  stalled server → healing proceeds within budget; (b) atomicity — fault
  injection before the pointer swap → next run sees the old epoch, never
  partial; (c) tamper — 200 body ≠ own commitment → reject + fallback;
  (d) reheat delta-billing — no-changed-candidates straddle ≈0 KAT; N
  changed spans → exactly those spans' tokens, deterministically;
  (e) itemization separated — all 13/13 green

## Honest scope / deferrals

- The live Worker wiring: when the shell lands, `sync`'s POST +
  `claim`'s submit flip from local-queue to real calls (the transport +
  wire formats are already pinned by the mock tests).
- Reheat under `--verify`: pass-2 edits are REVIEW-ONLY (never written
  without verification — never-wrong holds; they land on the next run,
  deterministically, since the epoch is pinned). Pass-1 verify-batch
  composition is untouched.
- The reheat proposer pass re-enters at FILE granularity (the proposer
  machinery is file-scoped) while BILLING is span-granular (only changed
  spans' chunk tokens re-bill — the load-bearing contract).
- `--corpus-epoch <E>` loads an installed epoch without any network.
