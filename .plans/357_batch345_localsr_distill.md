# Plan 357 — Batch 345: HerRei/local-upscale (LocalSR) distill batch

**Status:** LANDED — Batch 345 (commit: see git log "batch 345"), rust_perf 307→312, sec 50→51, kernel_opt 1075→1076; verdict Research 389. All gates green (full-feature 3997/0 + default 3632/0 [1 pre-existing sibling kat_ua flake disclosed] + rust_perf_bench 3663/0 + retrieval floors HELD no re-pin + dual_allocation 0/0/0).

- [x] Re-clone + re-pin the source (v0.3.0-beta moves fast) and record the fresh sha + license in the entries headers — PIN UNCHANGED at re-clone (cf6f8b55, tip 2026-11-07): T0 by pin identity
- [x] T0: re-verify every Research 389 quote at the fresh pin (spot-grep battery: 16/16 green)
- [-] Read the UNREAD set before authoring — DEFERRED: the entries' quoted spans were all verified at-pin; the full-body reads (commands.rs 2715 lines, updates.rs remainder, video_* remainders) stay for the next LocalSR-surface verdict; no entry quoted from unread text
- [x] Dedup re-verification at 3+ vocabularies for R1-R8, with the two mandatory adjudication notes carried verbatim from Research 389 (R3 vs the 0sec deserialize-DATA drop; R5 vs entries_02 unbounded→bounded)
- [x] Author rust_perf measurement entries: R1 `benchmark-spread-above-threshold-reports-unstable` (B+), R2 `macos-pressure-reads-footprint-not-rss` (B, incl. the vm_stat reclaimable face)
- [x] Author the sec entry: R3 `execution-artifact-trust-ladder-before-deserialize` (two-sided host/worker gate; positive instance = our own wasm-validator store manifest)
- [x] Author kernel_opt entry: R4 `accelerator-backend-first-tile-acceptance-gate` (adjudicated vs B337 + entries_19; Core ML table rows carried SOURCE-REPORTED, never bars)
- [x] Author rust_perf concurrency entry: R5 `slow-consumer-latest-value-single-slot` (tokio family +1; `bench_spec: None`)
- [x] Author rust_perf io entries: R6 `untrusted-download-verify-then-atomic-install` (+ the re-verify-at-use face folded in-rule + the run_worker_handshake install canary), R7 `ui-event-stream-sampled-gate-terminal-forward` (KEPT — per-class sampling with terminal-always/structural-never/last-force arms; test-pinned in-source)
- [x] R8 spawn-hygiene candidate: merge-or-drop decision recorded — DROPPED at authoring (thin standalone; the sec spawn family's adjacent axis)
- [x] Fixtures ORIGINAL minimal reproductions, standalone-compiled AND executed BEFORE entry (rust_perf 18/18 + sec 5/5 + kernel 4/4)
- [x] No benchable GOAT exists (Research 389 §GOAT) — every entry lands `bench_spec: None` with the metric-mismatch reason in context_notes; no Pareto validator run owed
- [x] Standing gates: full-feature 3997/0 + default 3632/0 + rust_perf_bench 3663/0; clippy -D all-features MY-FILES-CLEAN (the residual heal.rs/katplane/headroom_probe errors are the sibling 221 lane's in-flight WIP, disclosed); retrieval floors HELD (retrieval_precision_regression_gate ok, no re-pin); freeze/score-bench NOT owed (zero clippy-corpus delta — the B342/B343 posture); pins_manifest --write (rust_perf 312/notbenchable 184, sec 51, kernel 1076) same commit; dual_allocation 0/0/0
- [x] Count pins: AGENTS.md + README rust_perf/sec/kernel_opt rows, health_views (perf 312/sec 51/kernel 1076/total 1601), kernel router total 1076 + dispatch 256, kernel-side rust_perf witness 312, goat_g1 census 1..=154 + names 155, the rust_perf pins (312×5 sites, 255→260×3 sites, without_spec manifest-derived), the batch345 needles tests (rust_perf + sec)
- [x] Update Research 389 status line at landing (DISTILLED → LANDED with the batch ref); queue snapshot row updated in the same commit
- [x] Commit + push on develop, `feat:` prefix, Session trailer; `rm -rf .raw/local-upscale` at close
