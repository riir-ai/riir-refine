# Plan 127 — P015 Phase 2: the `cargo heal replay` client

**Status:** COMPLETE (2026-09-15) — T2.3's armed-lane leg owner-gated by design (see the task note)

The client half of the replay-endorsement lane
([Proposal 015](../.proposals/015_replay_endorsement_reward_class.md) Phase 2;
the server half landed as riir-dapps Plan 029 — ledger surface `00b6583`,
worker routes `861d5f5`, wire composers in the riir-kat leaf `c80bd43`).

The lane: login-gated lease → local `fix_verify` + clippy-oracle replay
(scratch project) → signed verdict submit; toolchain fingerprint check; kill
switch `RIIR_HEAL_REPLAY=0`.

## Tasks

- [x] T2.1a riir-kat `kat_replay_client.rs` — the client half (pure: signed
      lease/verdict body builders + ack parsers behind `client`; transport:
      `ReplayTransport` trait + ureq `HttpReplayTransport` +
      `request_lease`/`submit_verdict` behind `kat_transport`) + lib.rs row
- [x] T2.1b riir-clippy `src/kat_replay.rs` — the replay engine
      (clippy_verify-gated): `toolchain_fingerprint` (`rustc -vV` BLAKE3),
      the PINNED single-item rate rule, `judge_item` (step-0 hash check →
      tamper; oracle batch `[shape, fix]` → vector) + lib.rs row
- [x] T2.1c `src/bin/heal.rs` wiring — `KatCommand::Replay`, the `replay`
      parse arm, the feature-gated dispatch (refuses loud without
      kat_service+kat_transport+clippy_verify), `run_kat_replay()` (kill
      switch → login gate → consent opt-out refusal → fingerprint → lease →
      judge → submit loop → summary), usage row
- [x] T2.2 VersionSkew handling — local refusal (skip + report, never submit
      a verdict the lease pin makes unscoreable); mismatch wording per P015
      caveat 2
- [-] T2.3 e2e — the lane is 503-inert on every deployment until Phase 3
      arms it (`KAT_REPLAY_*` knobs unset), so the devnet leg is owner-gated
      BY DESIGN. Phase-2-scoped substitute: riir-kat mock-transport contract
      tests pinning the client against the route contract shapes (the
      refill/burn CLI-test precedent). The wrangler-dev armed leg is
      documented as the Phase-3 activation gate's own smoke.
- [x] T2.4 docs — `.docs/01_orientation/replay_lane.md` (the lane page),
      node_tiers.md table cell fix (the stale "class NOT YET DESIGNED" —
      P015 designed it; the client ships, settlement stays Phase 3), P015
      status line, HISTORY entries
- [x] Gates — clippy + tests green on both repos (isolated
      CARGO_TARGET_DIR), commit + push

## The pinned client rate rule (determinism law)

Two honest verifiers must agree on the FULL 18-byte vector (the rates ride
the commitment). The single-item replay pins:

- `compile_ok` = the FIX compiles (the oracle's `compiles` on the fix arm)
- `oracle_resolved` = the shape compiles AND the fix compiles AND
  `lints(shape) \ lints(fix) ≠ ∅` — the fix STOPPED at least one lint the
  pre-fix span fired. The wire carries no `lint_key` (P015's
  stripped-vector design), so the REMOVED set is the observable evidence
  of a heal; ambient pedantic/nursery lints firing on both arms cancel in
  the diff by construction (measured: the first intersection-based rule
  died on the real-oracle test — `must_use_candidate` fires on most pub
  fns, so an empty-intersection rule reads nearly every honest heal as
  unresolved). A clean shape + clean fix reads UNRESOLVED (no evidence,
  never a vacuous pass). The v2-lease-carries-lint_key refinement is the
  Phase-3 reopen.
- `heal_rate_milli` = 1000 if `oracle_resolved` else 0
- `created_rate_milli` = 1000 if `lints(fix) \ lints(shape) ≠ ∅` else 0
  (born-closure lints — the gen-8 shaped half-win prices healed+created)

Step 0 (before any compile): `blake3(fix) == fix_hash` else the item is a
canary → `Tamper` verdict. Unmeasurable (cargo missing / oracle did not
run) → SKIP the item, never submit (a verdict is a claim).

## Landing record

- **riir-kat**: `kat_replay_client.rs` NEW (569 lines: pure body builders +
  ack parsers + `accept_line` behind `client`; `ReplayTransport` trait +
  ureq `HttpReplayTransport` + `request_lease`/`submit_verdict` behind
  `kat_transport`; 8 tests incl. the two-sided signature-verification pins
  and the mock-transport contract suite) — 236 lib tests green, clippy
  clean at `client,kat_transport`.
- **riir-clippy**: `src/kat_replay.rs` NEW (the engine: `rustc -vV`
  BLAKE3 fingerprint, the pinned rate rule — REMOVED-set evidence after
  the real-oracle test killed the intersection rule on ambient pedantic
  lints, `judge_item` with step-0 canary check, 12 tests incl. the
  real-oracle heal); `lib.rs` module + re-export rows; `heal.rs`:
  `KatCommand::Replay` + parse/dispatch/usage + `run_kat_replay()` (kill
  switch → login → consent → fingerprint → lease → judge → submit →
  summary). 1640 lib tests green at `clippy_verify,latent_retrieval`;
  default build compiles (module cfg'd out); clippy clean (the healer
  took 9 mechanical edits in the same files — its own dogfood).
- **Live smoke (M3, devnet)**: kill switch exit 0; the full gate chain
  runs and the lease dial surfaces the DEPLOYED worker's own refusal
  (`route not found` — the worker predates the Plan-029 routes; arming is
  Phase-3/owner). Exactly the honest Phase-2 posture: the client is
  correct against the wire contract (pinned by the shared leaf + the
  mock suite), and the lane is inert everywhere until Phase 3.
- **Deferred (owner-gated)**: the armed-lane e2e + any `replay_cut_bp ≠ 0`
  question — Phase 3.
