# Plan 226 — Batch 224: the conversion tail + the cycle-37 leads (MoE mul_mat_id compact-prepass)

**Status:** COMPLETE (landed 2026-10-12, shikuwa/4090, cycle 38 — claim `1df9bcf9`; all gates green; see T8 for the landing commit)

## Scope (and what is NOT in this batch)

- The walk-#14 "conversion set" re-verified at claim time: **#29435 LANDED** (Batch 205, `entries_gpu_75` `work-splitting-requires-work-shape-homogeneity`) and **the B208 F16 rider LANDED** (the F16-scores watch face inside `entries_gpu_78`, "revert reason unstated — never adopt"). Both legs STALE — replaced by the cycle-37 leads.
- **#1806 stays live** (no corpus hit) → rider-or-rule on `channel-send-batching-unpark` (`entries_02.rs:488`).
- The **FLA batch (Research 253, prep-complete) stays UNCLAIMED** for walk #14 — the M3 loop's designated slot.
- Lead 2 (vulkan switch→ternary, #30164): WATCH (GLSL→SPIR-V origin; transferability to WGSL/naga unverified — the intake row's own gate).
- Lead 4 (F16C proxy-detection, #30270): WATCH (our exposure low — compile-time arms; already graded B− watch).

## Tasks

- [x] T0 claim row posted + pushed (this commit) — `1df9bcf9`
- [x] T1 quotes verified at pins via API: llama.cpp `073b74034` (#30373 metal) + `966baae76` (#29998 vulkan) + `ccdbe2905` (#30299 tests-link) + cubecl `6bba5fdf0` (#1806, tracel-ai) — every quoted line re-read at the pin (2 parallel subagents; patch completeness reconciled vs the API stats — none truncated; cross-backend honest-mapping corrections surfaced + carried in-rule)
- [x] T2 dedup greps re-run at authoring: core atoms UNOWNED (tile-descriptor/launch-to-exit/static upper bound/unconditional store/divergent lanes/entry tile/compact list = zero corpus hits; rust_perf park_timeout/fixed-sleep/idle-server/PARK_STEP = zero); neighbors adjudicated in-rule (degenerate-selection B110 = selection-side vs THIS consumer-side; workspace/tile-picker/indirect-tile-map/router-fold/prepass×2)
- [x] T3 own rule authored `entries_gpu_88.rs`: `moe-duplicate-id-compact-prepass-static-bound-tile-dispatch` (D-class; the correctness half + the two-pass prepass + the unconditional-store law + 3-word/2-word descriptors + the static bound closed form both backends + tail early-exit + the measured figures + the ne02 honest-null + the cross-backend honesty) — wired into KERNEL_CATEGORIES D slice
- [x] T4 riders: #1806 park-timeout-as-watchdog + fixed-sleep-remainder-tax face → `entries_02.rs` channel rule name field (+ the rust_perf batch224 rider gate test); #30299 C++ link-set twin → `dist/corpus.rs` notes (+ needles in the boundary test)
- [x] T5 syn-parse gate extended (batch224_rule_parse_and_needles_landed, 14 needles + both halves parse) + counts pinned (tests.rs ×5 sites + category D 185→186 + the router.rs expert-total pin 795→796 + ledger comments + KERNEL_CATEGORIES D count comment)
- [x] T6 GOAT: G1 parse/syn ✓; G2 lib tests default 2504/0 + kernel_opt 2944/0 ✓; G3 clippy clean BOTH feature states (2 en-route heals: the `MoE` backtick + the pre-existing too_many_lines on batch223's test fn, annotated-allow) ✓; retrieval re-measure 21/21 — kernel self-variants 100.0%, blended 91.2/94.7/95.3, floors HELD, no re-pin ✓; freeze/score-bench NOT owed (zero clippy-corpus delta, the B223/P30 precedent)
- [x] T7 Research note `.research/254_batch224_moe_mul_mat_id_compact_prepass.md` + AGENTS.md counts (kernel_opt 796 ×1 + rust_perf rider note ×1) + `.docs/11_domains/` (kernel_opt.md seed line + B224 paragraph + dist.md rider note)
- [ ] T8 `.distill/001_mining_queue_snapshot.md` batch row + intake rows marked DONE; commit `feat: batch 224: ...` + push

## Numbering

- Plan: 226 (highwater 225→226). Batch: 224 (last = 223 lithos-metal). Research: 254 (highwater 253→254). Corpus file: `entries_gpu_88.rs` (max 87).
- Dual-allocation gate at claim time: 0 twin / 0 independent / 0 counter-only.
