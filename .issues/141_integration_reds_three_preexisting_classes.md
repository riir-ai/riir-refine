# Issue 141 — integration tests red at HEAD: three pre-existing non-rename classes

**Status:** OPEN — **Class B FIXED 2026-10-01** (the PERF_SPANS `avoid-clone` fixture cast `b as u64` → `u64::from(b)` — u8→u64 is lossless, so real clippy co-fired `cast_lossless` beside the labeled `manual_map` on the merged span; the lossless-explicit spelling keeps the span's vocabulary exactly `{manual_map}`. Verified: `cross_domain_spans_oracle_truth` 1/1 green + lib 2423/0; the fixture comment carries the class record). Classes A + C remain red — each needs its owning session's one honest read (the fleet-critical warning below stands). Found during the 192 Phase-2 rename pass; the rename-class reds (24 tests, the `.refine` state-dir fixture stragglers) are FIXED at `926fd4d2`. What remains red is three older classes the plan-192 gate line (`--lib` only) never saw. Repro: `cargo test --tests --features "kat_transport,latent_retrieval,kat_service,fix_verify,clippy_verify" --no-fail-fast`.

## Class A — `kat_wetrun_gwet` 4/11: the payload redact floor vs v2 fixtures

`a_sync_that_cannot_confirm_the_record_is_not_a_pass`, `the_full_wet_run_passes_on_devnet`, `the_refund_lands_exactly_once_and_is_faucet_funded`, `the_supply_gate_can_actually_fail`.

The client refuses: `the payload's redact version 2 is below the floor 3 — refusing to push (§7 step 4: redact ≥ floor)` → every run dies at `step 4 signed sync push` (`NotReady`). The test fixtures build v2-shaped payloads; the shipped floor is 3 (the dapps Issue-103 v3-domain era). The fixtures need the v3 shape (or the builder path the real client uses), NOT a floor change — the floor is the honest client posture.

## Class B — `cross_domain_oracle_truth` 1/1: clippy oracle drift on the merged-span fixture — FIXED 2026-10-01

`manual_map+avoid-clone/cf: corpus lints on the merged span must be exactly the labeled one — left {cast_lossless, manual_map} vs right {manual_map}`. Real clippy now co-fires `cast_lossless` on the fixture body (oracle drift, the same class the probe arms exist for). Fixed per this issue's own remedy: the fixture's cast is now the lossless-explicit form (`u64::from(b)`) so the exact-set assertion pins `manual_map` alone again (fixture comment records the class; lib 2423/0 + the oracle test green).

## Class C — `kat_stake_client_g2s` 1/9: the mock exhausts its script on the v2 seq re-fetch

`unstake_not_live_queues_and_still_discloses` — `unstake: the service stopped answering the seq re-fetch` (deterministic, 2/2 runs). The unstake v2 client (riir-kat `37de79a`, the `op_seq+1` retry-once shape) re-fetches the seq once more than the mock's response script serves. Either the script needs the extra 409/400 arm or the client re-fetch count changed — read riir-kat's `unstake_message_v2` flow against the mock before touching either side.

## Why filed, not fixed

The owning lanes are active (the unstake v2 client is the A10 fleet-critical path; the sync payload floor is the fleet v0.2.0 surface). Silent fixture "repairs" on those two would launder real behavior questions; each class needs its owning session's one honest read. The rename pass leaves them exactly as found.
