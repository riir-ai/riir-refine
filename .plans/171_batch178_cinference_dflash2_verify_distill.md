# Plan 171 — Batch 178: cinference DFlash2 verify-kernel distill (kernel_opt)

**Status:** QUEUED (nominal Batch 178, behind B176 FWHT rider + B177 EXL3; one-batch-per-day cadence)
**Source verdict:** [Research 209](../.research/209_cinference_dflash2_verify_distill_verdict.md) — YES B+ / LANE INTEL HIGH
**Pin:** satellitedown/cinference @ `37814b1e1843315e456b60883a8c8ba08cc2aef0` (Apache-2.0; pass shas README-pinned `b4e8ed4`→`4994a0c`)

## Leads (5) + riders (4-6)

- LEAD-1 `producer-store-evict-last-pins-cross-stream-consumer` (memory.cuh:94-101) — GOAT lead, bit-identical hint
- LEAD-2 `graph-exec-update-gated-by-captured-topology-signature` (decode_graph.{h,cpp} + graphs.cpp:362-382,73) — strongest on-our-box benchable (4090 CUDA graphs, Issue 965)
- LEAD-3 `dead-line-l2-discard-after-last-consumer` (memory.cuh:103-107) — the dead-line safety contract is the content
- LEAD-4 `exact-widening-enables-fp16-pv-accumulation` (small_t_k8v4.cuh:8-11) — tolerance strategy: source greedy-oracle + Issue-750-T3 retention-walk precedent
- LEAD-5 `wide-verify-warp-specialized-scores-in-registers-pv` (small_t_k8v4.cuh:13-17) — narrow/wide two-regime + loader/compute warp split
- RIDER-A per-tile evict-first by CTA-read-count → `weight-stream-non-temporal-loads`
- RIDER-B single-resident-wave double-buffer regime → `cpasync-multistage-tile-pipeline`
- RIDER-C GDN replay-record staging → `gdn-transition-replay-log-commit-many` (⚠ may re-route to the 971-gated composed GDN batch — owner call at slot time)
- RIDER-D prepare-once-copy-per-split (verify query tile) → `stream-invariant-operand-skips-staging-pipeline` (adjacent; adjudicate)
- MAYBE row-slice-expensive-epilogue-across-ctas (measured 1/2/5/10-slice sweep) + warp-bitonic-topk-merge-shuffle — coordinator-graded at slot

## Tasks

- [ ] T0 re-clone at the pin (FULL clone this time if per-pass diffs are needed for annotations; else shallow); re-verify every quote at mine time; `.raw` cleanup at close
- [ ] T1 dedup re-verification: re-run both-vocabulary greps from Research 209 §dedup against HEAD corpus (the queue moves; B176/B177 may have landed adjacent rules)
- [ ] T2 author LEAD-1..5 corpus entries (mechanism-level, never NVFP4/SM120 format-level; original minimal fixtures only — license hygiene)
- [ ] T3 author RIDER-A/B/C/D annotations (+ adjudicate MAYBEs; drop `mask-free softmax` if it collapses into the mask-sentinel/elision family)
- [ ] T4 corpus registration + count pin (kernel_opt 627 → expected ~632; router.rs:472 pin if it asserts total)
- [ ] T5 standing gates: kernel_heal feature corpus tests green, kernel retrieval floors re-measured
- [ ] T6 freeze tables: VERIFY NOT OWED (kernel_opt additions do not touch `build_seed_corpus()` — the B56-rider precedent; assert zero clippy-lane delta)
- [ ] T7 score-bench append + frontier snapshot + gate-calibration (the standing post-landing rule)
- [ ] T8 Research 209 status → RECORD; HISTORY.md batch row; AGENTS.md kernel_opt count line
- [ ] T9 queue snapshot NEXT POOL update + commit+push (`feat:` batch commit; `docs:` if only records)
- [ ] T10 lane-intel follow-through: offer (not make) the Issue 846/989 addendum quoting this pin — sibling repos are not edited by a distill batch

## Honest scope notes

- Pass-1 attribution is composite — annotations carry the pass split (bit-exact passes 2-4 with per-pass deltas; pass 1 unsplit).
- The 846 verify-chunk DRAM floor (C ≥ ~1.44 on 4090/W4A16) is NOT reopened by this source — different silicon/quant operating point; recorded in Research 209 §lane.
- bench/ harness unread at verdict time — T0 reads it for the measured-GOAT leg if annotations quote per-kernel numbers.
