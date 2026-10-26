# 172 — 16 pre-existing `cargo test --release --all-features --tests` failures across 12 integration targets
**Status:** OPEN — observed 2026-10-26, classified by first panic line only (no per-test root-cause read yet)

Surfaced while verifying `8d731fd5` (the four-file inline-test move). **Attribution
control:** the 12 failing targets were re-run at `8d731fd5^` (`ea494f79`) in a
symlinked `/tmp` workspace, same flags — **the identical 16 fail there too**
(`comm` of the two FAILED sets is empty both ways). So none is caused by the
move; all predate it. Totals for the whole `--tests` run at `8d731fd5`: 115
binaries, 4724 passed, **16 failed**, 39 ignored. Box: amo (M5 Ultra, AC), the
run itself was the only heavy load. `ci_feature_guard.sh` names an
all-features layer, so this is either a guard that is red or a configuration
the guard does not actually execute — T0 answers which.

| class (hypothesis) | target | test | first panic |
|---|---|---|---|
| net-pin OVERLAP (`--all-features` = mainnet+devnet → empty HOSTED set, Issue 163) | `build_stamp_cli` | `tuna_airdrop_{below_floor_refuses_locally_at_exit_2, missing_key_refuses_pre_network, dry_run_is_fully_local_and_shows_the_hash}` | output mismatch at :291/:315/:335 |
| net-pin overlap? | `kat_refill_cli` | `refill_refuses_on_mainnet_posture_and_names_both_paths` | "mainnet refill must refuse" :417 |
| net-pin overlap? | `unmine_consent_cli` | `unmine_{records_the_opt_out…, deletes_no_queued_batch, clears_the_sync_watermark_sync_writes}` | output mismatch :113/:144/:164 |
| drift — pin | `kat_issue088_cross_repo_pins` | `the_redact_floor_matches_the_service_gate_version` | floor ≠ current gate version :26 |
| drift — literal gate | `micro_literal_gate` | `no_anonymous_million_literal_in_non_test_source` | 13 sites: `src/bin/heal.rs:3433-3434`, `src/update_check.rs:931-982` (test-mod values?), `src/validators/interp.rs:76 DEFAULT_FUEL` |
| drift — FP growth | `rule_embed_gate` | `rule_embed_posture_holds_the_pinned_gates` | clean fixable top-1 38 > pinned 31 |
| drift — corpus grew | `t51_l4_vs_decline` | `arm0_baseline_declines_on_every_reachable_case` | `ptr_arg` now has 1 candidate — fixture must be re-picked |
| drift — bit-identity | `scope_gate_adoption` | `g3_in_distribution_healing_is_bit_identical` | `redundant_static_lifetimes` 0.32000002 vs 0.31997228 |
| drift — floor | `secret_scan_cli` | `empty_walk_refuses_instead_of_passing` | 1 file scanned, 0 findings reads as a pass |
| drift — negative control | `shader_perf_gates` | `negative_control_zero_findings_on_vgpu_port_corpus` | negative control FAILED |
| drift? | `p12_suppress_reverted_e2e` | `p12_suppresses_a_twice_reverted_pair_across_runs` | "run 1 must heal the match shape" :89 |
| load-sensitive BAR? | `latent_suggest_goat` | `g2_latency_bar_per_chunk` | 5010 µs/chunk ≥ 1000 µs (measured inside the full-suite run — re-run ALONE before adjudicating) |

## Tasks
- [x] T0 — YES: `ci_feature_guard.sh` Layer 5c runs EVERY test target at `--all-features` (no allowlist; `#[ignore]` is the only exclusion). **The guard is RED today** on these 16 — this is a red CI gate, not a dormant configuration.
- [ ] T1 — re-run each class at DEFAULT features (and the net-pin rows at a single pin): a failure that only exists at the overlap posture is a configuration artifact → give those targets the right `required-features`/`cfg(not(all(net-mainnet, net-devnet)))` guard rather than a pin change.
  - **Measured 2026-10-26 (release, each target at its own required-features set):** the net-pin hypothesis is only PARTLY right — reclassify:
    - `build_stamp_cli` (req `latent_retrieval,rust_perf,clippy_lints,kat_service`): **6/6 pass** at the minimal set AND with `+net-mainnet,net-devnet` → the 3 `tuna_airdrop_*` reds come from some OTHER all-features member, not the pin overlap. Bisect the feature set.
    - `kat_refill_cli` (req `kat_transport,latent_retrieval,rust_perf,clippy_lints`): **5/5 pass** minimal; **4 fail** with `+net-mainnet,net-devnet` (vs 1 at full all-features) → a real overlap interaction, but non-monotone in the feature set.
    - `unmine_consent_cli` (same req): **3 FAIL at the MINIMAL set, no net pins** → NOT a configuration artifact: copy drift. `--unmine` now prints the opt-out record path + STOPPED/UNCHANGED/ACCOUNT blocks but **no queued-batch count and no outbox path**, which the three tests assert (`:113` "must name the outbox path", `:144` "must count what is queued", `:164` watermark). Fix site: `src/bin/heal.rs` `--unmine` branch (or the tests, if the copy change was intended — read the commit that changed it).
- [ ] T2 — the drift rows: adjudicate each (fix the source, re-pick the fixture, or re-pin WITH a reason) — never a bare re-pin.
- [x] T3 — re-run ALONE (`--release --all-features --test latent_suggest_goat`, quiet box, 2026-10-26): **still FAILS — 5016 µs/chunk vs 1000 µs bar** (480 chunks; G1/G4 + calibrations pass). Not load-sensitive: a ~5× latency regression (or a bar mis-calibrated for this config). Next: bisect the target's history for the first commit where G2 reds.
