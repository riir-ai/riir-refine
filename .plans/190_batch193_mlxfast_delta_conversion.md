# Plan 190 — Batch 193: the mlxfast-bonsai2 delta conversion (post-reorg accept wave)

**Status:** COMPLETE — landed 2026-09-30 (+14 rules, 689→703; +2 riders on B178/B180; +1 rust_perf family-host amend; 4 drops + 5 folds adjudicated; all gates GREEN).

## Context

- Source: `Layr-Labs/mlxfast-bonsai2-27b-engine` (MIT), standing clone
  `/Users/katopz/git/vendor/mlxfast-bonsai2`, NET delta `831fae7..0c61273`
  (46 commits, 41 accepts; the arena reorg 2026-09-28 reset the tree + added
  Seatbelt sandboxing — tools/ infra EXCLUDED by the staging verdict; the
  B181/B182 content re-landed identically and vanishes from the net diff).
- Staged pack: 8 A-family + 14 B-family candidates → **8-12 rules + 2-3
  amendments** estimated; anti-lead ledger +6-8 (research-note material).
- Batch 192 = Plan 189 / entries_gpu_59.rs. This batch authors
  **entries_gpu_60.rs**.
- Score-bench + freeze regen NOT owed (kernel_opt-only, zero clippy-corpus
  delta — P30).

## Conversion duties (from the staging verdict)

- [x] T0 Source pins verified (`0c61273f2` reachable in the standing clone after fetch) — DONE at plan open.
- [x] T1 Slice A authored (submission/scheduling/KV family — 7 rules from 11 candidates; quotes verified VERBATIM at the pin; scratch `/tmp/b193_slice_A.rs` consumed into entries_gpu_60.rs).
- [x] T2 Slice B authored (kernel/numerics/layout — 6 rules from 10 candidates; quotes verified VERBATIM at the pin; scratch `/tmp/b193_slice_B.rs` consumed into entries_gpu_60.rs).
- [x] T3 COIN-FLIP 1: `plane-form-activation-reuse-topology` → STANDALONE (a THIRD axis — consumer-sharing topology SGN/CT/AT; distinct-from B180 byte-layout + B186 L1-formula + B21 staged-tile written in-rule).
- [x] T4 COIN-FLIP 2: `drafter-head-rides-verify-int8-via-exact-widening` → FOLDED as the RIDER on B178 (the exact rider sentence landed on the host; 'never an emitted token' carries the lossy-surface guard).
- [x] T5 INSTANCE-OR-EXTENSION 1: PrismML `5244ceade` = INSTANCE of B180 (same mmvq-ptq1_0.cuh lineage — 9 planes, block_q8_1 stride law); rider landed with the TWO new atoms (the +0x7F7F7F7F^0x80808080 decode spelling + the nwarps-pinned-4 summation-order-stability clause).
- [x] T6 INSTANCE-OR-EXTENSION 2: llama.cpp `18bbc46` = the CONSUMED B176 rider boundary (3-line diff: TG_MIN_N 1024→512, both dtypes) — nothing beyond, no landing; recorded in Research 225.
- [x] T7 ADOPTION FAMILY: new rule `adoption-trial-rotate-operands-per-launch-dram-streaming` + the AMEND on the REAL host (rust_perf entries_10.rs — slice A's correction: NOT entries_gpu_50.rs): adoptMargin 0.005→0.015 amended in-place (never a competing threshold), the sampled-self-test premise RETIRED (7/43), the rotation cross-reference.
- [x] T8 entries_gpu_60.rs assembled (14 entries), mod.rs wired (`mod entries_gpu_60;` + extend + KERNEL_CATEGORIES ×14).
- [x] T9 tests.rs: `batch193_pseudocode_parses` + the contracts gate + the riders/amend gate (incl. the rust_perf host markers) — 3/3 GREEN.
- [x] T10 router.rs expert-coverage pin 689 → 703 (honest label) + the per-category count pins + the two corpus-count pins (seed_corpus_has_expected_entry_count + batch116_rules_present).
- [x] T11 Gates ALL GREEN: kernel slice 389/0; lib default 2398/0; clippy -D ×3 postures clean (1 doc_markdown backtick en route); retrieval floors 21/21 HELD (kernel 100% ×3, 235.41s).
- [x] T12 Research note `.research/225_mlxfast_delta_batch193_conversion.md` (STATUS: RECORD; anti-lead ledger; coin-flip + instance/extension reasons; 4 staging-claim corrections).
- [x] T13 AGENTS.md kernel_opt count bumped 689 → 703 + the Batch 193 narrative + the kernel_heal feature line 684 → 703.
- [x] T14 `.distill/001` queue row updated (Batch 193 CONVERTED + rider re-armed unchanged).
- [x] T15 Commit + push.

## Non-goals

- No clippy-corpus changes → no freeze-table regen, no score-bench (P30).
- No riir-train/routing-restriction touches; no 4090 work (box CPU-busy with
  a sibling lane — riir-infer 030 stays queued).
- Anti-leads are RESEARCH-NOTE material, never corpus rows (the staging
  verdict's negative ledger).
