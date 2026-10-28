# Plan 308 — Batch 303: splash return-visit delta (kernel_opt residue + ANE host family)

**Status:** STAGED — verdict landed (Research 344); batch execution claims per the mining queue; staged behind plan 307's live claim (Batch 302 pool, expires 2026-10-30T16:2xZ)

- [ ] T0 — re-clone incoai/splash at pin `cabbad2732b027540f5a141511544e0bb62224b7` into `.raw/b303/` (shallow; removed at close); re-verify every graded quote at the pin (the research note's caveat list names the unread files: Scheduler.cpp, Engine.cpp, KV-pool bodies, DFlashDraft.cpp, Runtime.cpp VerifyBudget consume-site, Linear.cpp takesPlaneViews — read before grading anything that cites them)
- [ ] T1 — dedup re-verification (3+ vocabularies per candidate) against kernel_opt 994 + rust_perf 274 + the freshly-landed Batch 302 rows (plan 307 may land first — its TF/cubecl/llama.cpp rules could kin-collide with the sgmatrix role-split and the sampler family): golden-ratio spread vs the staging/prefetch family; fp32-pipe role-split vs `mma-lane-map-whole-run-vector-loads-split-accumulator-chains` + `one-pack-serves-all-schedules-nibble-order-contract` + `integer-mma-span-epilogue-dequant`; GDN tape + 11-28% handoff vs `whole-step-gdn-decode-window-one-launch` (rider or own-rule adjudication); determinism pair vs the parity family (5 entries) + the Issue-940 law's corpus faces; conditional PV rescale vs `per-block-int32-chunk-rescale-fp32-accum`; ANE breaker/calibration/one-program vs `bounded-peer-handoff-wait` + `launch-heuristic-pinned-to-calibration-arch` + B219 + the B270/271 serving pool
- [ ] T2 — author via 2 parallel subagents over disjoint slices (lane A: kernel_opt candidates 1-10 + second tier from Research 344; lane B: ANE/host candidates — routing adjudicated at T1: rust_perf serving family per B270/271 precedent vs kernel_opt host-orchestration clause; every quote byte-verified at the pin; fixtures OUR OWN original Rust/MSL-shaped reproductions, never source verbatim; llama.cpp-lineage tables quoted only as provenance)
- [ ] T3 — coordinator wiring: mod.rs categories + count pins (tests.rs + router.rs + Plan-003 category table — the B299 lesson: ~37 asserts, not 5) + batch303 needle gates keyed to LANDED text
- [ ] T4 — gates: batch303 needle+parse (run at `--features kernel_opt` — the green-zero class); kernel_opt lib; default lib; clippy -D ×3 postures; retrieval floors RE-MEASURE at both postures (corpus delta). If any rust_perf rows land: freeze regen + score-bench + release-set binary rebuild owed (P30/B293); if kernel_opt-only: NOT owed
- [ ] T5 — Research note update (344 Status → RECORD at landing) or the landing row in a new note per numbering; AGENTS.md kernel_opt/rust_perf count bumps; .docs/11_domains delta; queue snapshot landing row (same commit)
- [ ] T6 — commit `feat: batch 303: splash return-visit delta ...` + push; `.raw/b303/` removed at close
- [-] Riders deferred to re-pin lanes: the 12 rider-magnitude findings (Research 344 §riders) feed existing rules at their next re-pin, not this batch — record each in the touched rule's next touch instead

## Candidate inventory (from Research 344 — re-graded at T0/T1, honest drops allowed)

| # | Candidate | Domain at staging | Grade |
|---|---|---|---|
| 1 | golden-ratio-decode-tile-phase-spread | kernel_opt | **B+ GOAT lead** |
| 2 | fp32-pipe-mma-role-split-biased-bf16-operands | kernel_opt | B+/B |
| 3 | gdn-decode-tape-deferred-retained-row-fold | kernel_opt | B |
| 4 | gdn-head-threadgroup-pipeline-tg-handoff (11-28% rider or own-rule) | kernel_opt rider | B− |
| 5 | metal-fma-carries-fast-math-reassociate-off-cannot-pin | kernel_opt (determinism) | B− |
| 6 | gdn-cross-phase-gate-bitwise-parity (1-in-10^5) | kernel_opt (determinism) | B− |
| 7 | conditional-pv-accumulator-rescale | kernel_opt | B− |
| 8 | kv-stage-reuse-overlapped-softmax | kernel_opt | C+ |
| 9 | apple9-coop-tensor-destination-init-lost-runtime-format | kernel_opt (correctness) | C+ |
| 10 | quad-lane-softmax-two-shuffle-16b-store | kernel_opt | C+ |
| 11-15 | second tier: unsigned-window-mask / lane-binding-macro / e4m3-shift-half / residency-end-kick / thread-local-byte-index | kernel_opt micro | C, pick 0-2 |
| 16 | one-program-many-functions-accelerator-service-memory | rust_perf (or kernel_opt host) | B+ |
| 17 | ane-split-calibration-fit-bandwidth-interference-term | rust_perf (or kernel_opt host) | B |
| 18 | accelerator-lost-breaker-forgets-calibration | rust_perf (or kernel_opt host) | B− |
| 19 | writeclass-share-partition-inflight-writes | rust_perf | C+ |
| 20 | spin-keep-awake-checked-capability-degrade-loud | rust_perf | C+ |

Yield estimate after falsification: 5-8 own rules + 12 riders (Research 344 §yield).
