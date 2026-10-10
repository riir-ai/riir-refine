# Issue 159: the net-pin conflict posture refuses every run at startup — the all-features e2e lane (auto_oracle_features_cli) has been red since the T9/T11 landing

**Status:** FIXED on develop — the conflict posture partitions local state under the default net (one advisory line) and keeps every dial fatal; the e2e + the build-stamp pin assertion are posture-aware. This file remains as the record (remove on the next hygiene pass).

Open — filed by the idle worker 2026-10-10 during the kat_ua (riir-kat 003 T15) landing; the failure is PRE-EXISTING, reproducible at `492381b2`..`9374ddc6` (before the UA commit `38e7164e`).

## The finding

`./scripts/ci_feature_guard.sh` Layer 5c (`cargo test --all-features --tests`) fails on
`tests/auto_oracle_features_cli.rs::auto_oracle_features_knob_anchors_gated_diagnostics`:

```
service url refused: this build enables BOTH net-mainnet and net-devnet — the allowed
host set is EMPTY by overlap semantics; rebuild with at most one pin
(net: conflict: net-mainnet+net-devnet (empty allowed set — every host refuses))
```

- The test spawns `CARGO_BIN_EXE_cargo-refine --suggest` on a scratch crate. Under
  `--all-features` the binary carries BOTH net pins → the `command_service_url()` seam
  (landed `492381b2`, 2026-10-09, "the ONE command_service_url() seam gating every dial
  site") refuses at run start — BEFORE any suggestion work, and before
  `RIIR_REFINE_KAT_METER=0`/`KAT_GATE=0` (which the test sets) could matter: the refusal
  is the URL RESOLUTION, not a dial.
- The spawned run's startup path includes the Issue-153 update check ("every real run
  checks at start"), which resolves the service URL → the conflict refusal → exit 2.
- The test's `cfg` gate only requires `latent_retrieval,rust_perf,clippy_lints,kat_service`
  — it cannot see the posture it runs under, so it reds only in all-features lanes.

## Shape of the fix (one of)

1. **The conflict posture must not refuse non-network commands at startup** — the pin
   contract says "refuses loud at the URL seam" for DIALS; a `--suggest` scan over a
   local crate dials nothing. Move the conflict refusal from URL RESOLUTION to
   dial time (the update check's cache-only-at-startup half already exists: `--info`
   "reads the CACHE only — a read never touches the network" — extend that posture
   awareness to the conflict case).
2. **The test bins its posture** — run A's spawn env carries
   `RIIR_REFINE_SERVICE_URL`/config pointing at a loopback URL... but `allow-local` is
   ALSO conflict-refused (both pins = empty set INCLUDING loopback). So option 2 alone
   is not available without a third env knob.
3. **The e2e runs at a narrower feature set** — the `[[test]]` row's required-features
   cannot EXCLUDE features, so this needs the guard's Layer 5c to run this one target
   at the release set instead of all-features (a posture pin, precedent: the
   wasm32-only rows).

Option 1 is the root-cause fix (the conflict posture's empty allowed set is a
compile-time fact; refusing it at STARTUP makes every all-features-built binary
dead on arrival for local-only work, which is exactly the posture this workspace's
own guard builds).

## Repro

```sh
cd /Users/katopz/git/riir-refine
cargo test --all-features --tests --test auto_oracle_features_cli   # fails
cargo test --features latent_retrieval,rust_perf,clippy_lints,kat_service,kat_transport \
  --test auto_oracle_features_cli                                   # passes (single pin posture)
```

Found during: the kat_ua landing's guard run (the refine half). The riir-kat half's
own guard is ALL LAYERS PASS (`b1eb3a8`); this red is refine-side and predates it.
