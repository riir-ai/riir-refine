# Plan 226 — Batch 224: the conversion tail + the cycle-37 leads (MoE mul_mat_id compact-prepass)

**Status:** IN PROGRESS (claimed 2026-10-12 19:4x +07, shikuwa/4090, riir-refine idle loop cycle 38; claim row in `.distill/001_mining_queue_snapshot.md`, pushed before authoring)

## Scope (and what is NOT in this batch)

- The walk-#14 "conversion set" re-verified at claim time: **#29435 LANDED** (Batch 205, `entries_gpu_75` `work-splitting-requires-work-shape-homogeneity`) and **the B208 F16 rider LANDED** (the F16-scores watch face inside `entries_gpu_78`, "revert reason unstated — never adopt"). Both legs STALE — replaced by the cycle-37 leads.
- **#1806 stays live** (no corpus hit) → rider-or-rule on `channel-send-batching-unpark` (`entries_02.rs:488`).
- The **FLA batch (Research 253, prep-complete) stays UNCLAIMED** for walk #14 — the M3 loop's designated slot.
- Lead 2 (vulkan switch→ternary, #30164): WATCH (GLSL→SPIR-V origin; transferability to WGSL/naga unverified — the intake row's own gate).
- Lead 4 (F16C proxy-detection, #30270): WATCH (our exposure low — compile-time arms; already graded B− watch).

## Tasks

- [ ] T0 claim row posted + pushed (this commit)
- [ ] T1 quotes verified at pins via API: llama.cpp `073b74034` (#30373 metal) + `966baae76` (#29998 vulkan) + `ccdbe2905` (#30299 tests-link) + cubecl `6bba5fdf0` (#1806, tracel-ai) — every quoted line re-read at the pin (Batch-31 law)
- [ ] T2 dedup greps re-run at authoring: prepass/tile-despatch/compact-id family vs `entries_gpu_20/18/59` (indirect-device-dispatch-tile-map-moe) + B220 router-fold (`entries_gpu_*`) + B210 expert-cache; park/sleep/channel vs entries_02/06
- [ ] T3 own rule authored `entries_gpu_88.rs`: `moe-duplicate-id-compact-prepass-static-bound-dispatch` (D-class; the correctness half: duplicate expert ids overflow fixed ne21-capacity assumption; the perf half: compact per-expert (token,slot) two-pass prepass + one-unconditional-store-per-token + (expert, first-entry, valid-rows) tile descriptors + static upper-bound launch + tail early-exit; grid 8-16x shrink, 94% launch-to-exit tiles eliminated, pp +2% Qwen3.6-35B-A3B; riders in-rule: divergent-conditional-store→unconditional-store-with-padding + the function-constant ne02 honest-null)
- [ ] T4 riders: #1806 park-timeout-as-watchdog + fixed-sleep-remainder-tax face → `entries_02.rs` channel rule context_notes extension; #30299 C++ link-set twin → `src/domains/dist/rules.rs` bench-targets rule
- [ ] T5 syn-parse gate extended (kernel_opt corpus tests) + counts pinned
- [ ] T6 GOAT: G1 parse/syn (new rule); G2 `cargo test --lib` default + `--features kernel_opt`; G3 clippy `-D warnings` both feature states; retrieval re-measure `--features kernel_opt,latent_retrieval --lib retrieval` (floors: kernel self-variants 100%, blended 77.1/86.4/92.4); score-bench row + frontier snapshot
- [ ] T7 Research note `.research/254_*.md` (batch record) + AGENTS.md counts (kernel_opt 795→796, Batch 224 row) + `.docs/11_domains/` index line
- [ ] T8 `.distill/001_mining_queue_snapshot.md` batch row + intake rows marked DONE; commit `feat: batch 224: ...` + push

## Numbering

- Plan: 226 (highwater 225→226). Batch: 224 (last = 223 lithos-metal). Research: 254 (highwater 253→254). Corpus file: `entries_gpu_88.rs` (max 87).
- Dual-allocation gate at claim time: 0 twin / 0 independent / 0 counter-only.
