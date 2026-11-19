# Issue 249 — ci_feature_guard Layer 5c is red at HEAD: a by-construction feature conflict + real content rot

**Status:** OPEN — filed 2026-11-19 from a full Layer 5c run at `73379e6d` (amo job `01a1f346-cdd4`, `cargo test --all-features --tests --no-fail-fast`, 132 test binaries, **28 failing tests in 14 binaries**). Unclaimed. Layers 1–5b2 are GREEN at the same commit (single-feature combos, all-features compile, clippy at both extremes, lib tests 3933/0, retrieval gate 30/0).
**Claimed:** katgpt-rs-96@amo, 1795087130, expires 2026-11-20T11:18:50Z

## A. Design defect — `--all-features` enables two mutually exclusive net pins

Every networked CLI test refuses before doing anything:
`service url refused: this build enables BOTH net-mainnet and net-devnet — the allowed host set is EMPTY by overlap semantics; rebuild with at most one pin`.
Layer 5c (`scripts/ci_feature_guard.sh`, `cargo test --all-features --tests`) therefore cannot pass **by construction**; per the script's own comment it had never completed before the timeout was added. Hit (at least): `kat_join_cli` (join/config/env/tty set), `build_stamp_cli` (`tuna_airdrop_*`), `frame_time_report_cli` (5), `export_writes_a_verifiable_batch_without_an_account`.

- [ ] **A1** Run Layer 5c on an explicit feature set = all features minus one net pin (derive it from `cargo metadata`, never a hand list), or add a second pass per pin. Pinned-by-membership exemption if a third exclusive pair exists.

## B. Real content rot (independent of A — each needs its owning lane)

- [ ] **B1** `micro_literal_gate::no_anonymous_million_literal_in_non_test_source`: `1e6` literals in `src/domains/kernel_opt/corpus/fixtures/b362_fixtures.rs:351-358` (convert or allowlist with a reason).
- [ ] **B2** `heal` `corpus_stats_needle_tests::source_walk_agrees_with_runtime_count_for_compiled_domains`: kernel_opt source-walk needle 1109 vs runtime corpus 1093 (Issue 066 class).
- [ ] **B3** `cross_domain_oracle_truth::cross_domain_spans_oracle_truth`: `question_mark+btreemap-sorted-iteration/cf` merged span lints `{missing_panics_doc, question_mark}`, expected only the labeled one.
- [ ] **B4** `scope_gate_adoption::g3_in_distribution_healing_is_bit_identical`: `evolution_score` 0.32000002 vs 0.31999964 for `redundant_static_lifetimes` (haircut not saturating to exactly 1.0).
- [ ] **B5** `agent_skill_md_fresh::binary_help_surface_is_covered`: the binary's help prints `cargo refine account`; the skill does not render it.
- [ ] **B6** Triage after A1 lands (may be A-caused or real): `kappa_theta_separation_pins`, `t32_measured_evasion_set_is_pinned`, `the_redact_floor_matches_the_service_gate_version`, `unwrap_used_fires_without_fix_on_drop_fixture`, `replay_is_deterministic`, `l4_savings::without_the_gate_the_same_input_burns_the_l4_call`, `suggest_run_discloses_findings_quiet_when_clean`.

Full log at filing: `/tmp/rr-l5c.log` on amo (ephemeral). Not caused by `2c1ce5e0` / `87986f84` / `73379e6d` (lint + example-row + `ane` mirror fixes; none of the failing files touched).
