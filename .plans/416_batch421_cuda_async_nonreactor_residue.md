# Plan 416 — Batch 421: the cuda-async non-reactor residue (the B419 T0b staged lane)

**Status:** open — the B419 T0b verdict lane's own unit; 7 staged candidates + 6 marginal faces/riders, quotes re-derived at T0 (the B419 reader report rode the T8 teardown — the batch lane re-reads at its own T0 by design).
**Claimed:** amo-idle-i260@amo, 1795639776, expires 2026-11-27T20:49Z

Source: NVIDIA/cuda-rust cuda-async @ `f35d6ccc2e` (the Batch-417/419 T0 pin; tip-unmoved at B419 — re-check via ls-remote at T0). Apache-2.0. The staged candidates (queue snapshot, B419 landing block):

- **C1** `predicate.rs` — frame-confusion-unrepresentable-distinct-atom-variants (the frame-confusion incident + "unrepresentable rather than merely avoided")
- **C2** `device_context.rs` — thread-local-take-restored-on-every-exit-path (the replace(None).is_none() map-destroying check, two pinning tests)
- **C3** `cuda_graph.rs` — sticky-driver-mode-ends-before-panic-resumes (end_capture on every path incl. between catch and resume)
- **C4** `device_operation.rs` — memoized-device-value-reorders-consumer-stream-on-handoff (cuStreamWaitEvent before the value hands out, same-stream skip)
- **C5** `cuda_graph.rs` — graph-capture-bans-allocating-ops-replay-readdresses (the three named hazards + the compile_fail doctests)
- **C6** `launch.rs` — arg-arena-interior-pointers-materialized-at-consume-only (MaybeUninit until written, freed post-copy-out)
- **C7** `once_executor` — once-executor-unwind-settles-failed-waiters-released (drop-guard settles Failed + notify_all; the ThreadId re-entry refusal)

Marginal faces/riders staged: leak.rs report_leak choke-point (F2's orbit); adopt_capture dedup face; GraphExecHandle Arc face (arc-refcount-owned); FunctionKey display_hash face (key-completeness-adjacent); predicate overflow face; the pending-update drain face.

## Tasks

- [ ] **T0** — shallow clone at the pin, tip-unmoved check (`git ls-remote` vs `f35d6ccc2e`; a moved tip = re-pin + re-verify per the B414 discipline); reader verification of all 7 candidates: exact quotes + line anchors + mechanism statements at the pin, drift corrections carried
- [ ] **T1** — dedup re-verified at the landing HEAD (3+ vocabularies per candidate, every hit read); kinship adjudications in-rule both directions; the 6 marginal faces routed to their owners or dropped
- [ ] **T2** — author the verdict's survivors in `entries_gpu_171.rs` (the 4-6-rule size discipline; every fold-to-face option re-adjudicated at authoring and recorded in-rule); category placement + the category count pins
- [ ] **T3** — fixtures `b421_fixtures` standalone-compiled AND executed pre-entry (debug AND -O, zero warnings); the gate catches arm defects before landing
- [ ] **T4** — gates: lib both postures (default + rust_perf_bench), clippy `-D` ×3, fmt gate, pins_manifest
- [ ] **T5** — retrieval floors: Layer 5b2 suite + the priced synth floor (`--ignored`); re-pin only on decay, never silent
- [ ] **T6** — the Issue-260 landing law: `kernel_goat_g1 --full --emit-status` rides THIS corpus-touching landing in the same commit (289+ green / 0 fail expected; the new rules' rows join the store)
- [ ] **T7** — count web one-landing (AGENTS kernel_opt ledger + README + .docs/11_domains/kernel_opt.md + health_views tree total + category slices + kernel-side rust_perf witness + router total pin); score-bench/freeze NOT owed (zero clippy_lints delta — the standing convention, asserted)
- [ ] **T8** — teardown: `.raw/cuda-rust` + `/tmp/plan416` removed at close; disk delta checked
