# Plan 357 — Batch 345: HerRei/local-upscale (LocalSR) distill batch

**Status:** OPEN — verdict Research 389, YES B− / lane NONE / tokio family +1. Batch targets ~4-6 entries across rust_perf (measurement ×2, concurrency ×1, io ×2) + sec (×1-2) + kernel_opt (×1), all NotBenchable decision-content.
**Claimed:** zed-m3-localsr-b345, epoch 1794201250, expires 2026-11-11T03:15:00Z

- [ ] Re-clone + re-pin the source (v0.3.0-beta moves fast) and record the fresh sha + license in the entries headers
- [ ] T0: re-verify every Research 389 quote at the fresh pin (model_adapter `_checkpoint_trust`, commands.rs:1614, benchmark_v2 docstring + constants, edit_guard `process_memory`, hardware `_system_memory`, live_preview docstring, worker.rs gate arms 31-145, downloads.rs flow, updates.rs 782-835, coreml_runtime docstring + constants)
- [ ] Read the UNREAD set before authoring: commands.rs, updates.rs full, benchmark_v2 remainder, video_pipeline/video_io/video_memory remainders, coreml_runtime remainder, hardware remainder, queue_timing remainder
- [ ] Dedup re-verification at 3+ vocabularies for R1-R8, with the two mandatory adjudication notes carried verbatim from Research 389 (R3 vs the 0sec deserialize-DATA drop; R5 vs entries_02 unbounded→bounded)
- [ ] Author rust_perf measurement entries: R1 `benchmark-spread-above-threshold-reports-unstable` (B+), R2 `macos-pressure-reads-footprint-not-rss` (B, incl. the vm_stat reclaimable face)
- [ ] Author the sec entry: R3 `execution-artifact-trust-ladder-before-deserialize` (two-sided host/worker gate; positive instance = our own wasm-validator store manifest)
- [ ] Author kernel_opt entry: R4 `accelerator-backend-first-tile-acceptance-gate` (adjudicated vs B337 + entries_19; Core ML table rows carried SOURCE-REPORTED, never bars)
- [ ] Author rust_perf concurrency entry: R5 `slow-consumer-latest-value-single-slot` (tokio family +1; `bench_spec: None`)
- [ ] Author rust_perf io entries: R6 `untrusted-download-verify-then-atomic-install` (+ the re-verify-at-use face or its split rule), R7 `ui-event-stream-sampled-gate-terminal-forward` (drop-or-keep gate at authoring)
- [ ] R8 spawn-hygiene candidate: merge-or-drop decision recorded in the entries header
- [ ] Fixtures ORIGINAL minimal reproductions, standalone-compiled AND executed BEFORE entry (the R3 fixture needs the execution-capable-runtime sink; the R1/R2 fixtures are measurement-honesty arms)
- [ ] No benchable GOAT exists (Research 389 §GOAT) — every entry lands `bench_spec: None` with the metric-mismatch reason in context_notes; no Pareto validator run owed
- [ ] Standing gates: clippy -D at all postures the batch touches, retrieval floors both lanes, pins_manifest --write only if the corpus count genuinely moved (same commit), score-bench + freeze-table regeneration per plan 137 T7.5
- [ ] Count pins: AGENTS.md + README rust_perf/sec/kernel_opt rows, health_views, the kernel-side witness row — same commit as the corpus change
- [ ] Update Research 389 status line at landing (DISTILLED → LANDED with the batch ref); queue snapshot row updated in the same commit
- [ ] Commit + push on develop, `feat:` prefix, Session trailer; `rm -rf .raw/local-upscale` at close if a fresh clone was made
