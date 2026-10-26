# 172 — 16 pre-existing `cargo test --release --all-features --tests` failures across 12 integration targets
**Status:** OPEN — 10 of 16 fixed 2026-10-26 (T1 done, see below); 6 drift/perf rows remain (T2/T3), 1 filed cross-repo (riir-dapps Issue 138)

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
- [x] T1b — **landed (10 of 16 green at `--release --all-features`, each target re-run alone):**
  - `unmine_consent_cli` ×3 — the TESTS drifted at the per-net T1 move (riir-kat Issue 003, `fd4be603`): they planted the flat `.refine/outbox` / `.refine/kat_sync_state.json`, which nothing reads any more. Repointed at `kat_state_stamp::net_state_dir_for_url` + `SyncWatermark::net_path` (the binary's own resolvers); the heal.rs comment that still promised a flat-watermark clear was corrected. 10/10 at minimal and all-features.
  - `build_stamp_cli` tuna ×3 + `kat_refill_cli` mainnet ×1 — the both-pins posture after all (the tuna tests are `admin`-gated, so T1's minimal-set run never compiled them). Precedent in the same file (issue 159: the conflict posture is a legal build): under both pins the tests now assert the strict-seam refusal (exit 2, `service url refused`, mock untouched) instead of the single-net verdict.
  - `micro_literal_gate` — the scanner elided only literal `#[cfg(test)]`, so `update_check.rs`'s `#[cfg(all(test, feature = "update_check"))] mod tests` was scanned (11 rows). `test_cfg_attr_len` now elides `all(test, …)` (test a REQUIRED conjunct) and keeps scanning `any(test, …)`, `not(test)` and look-alike idents — a fixture arm pins all four. `format_usdc` derives its 6-dec unit from `kat_stake_client::USDC_DECIMALS`; `DEFAULT_FUEL` got an allowlist row (a step budget, not money).
  - `secret_scan_cli` — the update check writes `.refine/update_check.json` into the cwd (= the scan root) before the walk; `CI` disables it on the runner, so this was workstation-only. The test env now pins `RIIR_REFINE_UPDATE_CHECK=0`.
  - `p12_suppress_reverted_e2e` — NOT hermetic: none of the KAT kill switches were set, so `--fix` reached a live-service dial (refused under both pins). One `heal_cmd` builder now carries the suite convention for all three runs (the kill-switch run had its own `Command` that would have drifted again).
- Observed, out of this issue's lanes (no lane builds them): at `net-mainnet` alone without `allow-local`, three `kat_refill_cli` loopback-mock tests refuse; at `+admin` the `tuna_airdrop_missing_key` verdict depends on whether the box has a real `~/.config/riir-auth` key (the test does not isolate `RIIR_AUTH_ACCOUNT_DIR`).
- [-] `kat_issue088_cross_repo_pins` — filed riir-dapps Issue 138 (`37e81a2`): floor 3 vs gate 5 since riir-kat `9d31648`/`73f2b46`; the fix moves signed goldens + needs a kat-service deploy.
- [ ] T2 — the drift rows: adjudicate each (fix the source, re-pick the fixture, or re-pin WITH a reason) — never a bare re-pin.
- [x] T3 — re-run ALONE (`--release --all-features --test latent_suggest_goat`, quiet box, 2026-10-26): **still FAILS — 5016 µs/chunk vs 1000 µs bar** (480 chunks; G1/G4 + calibrations pass). Not load-sensitive: a ~5× latency regression (or a bar mis-calibrated for this config). Next: bisect the target's history for the first commit where G2 reds.
