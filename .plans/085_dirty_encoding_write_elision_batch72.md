# Plan 085 — Batch 72: the frguthmann/NVIDIA compute-shader external distill (1 rule + 1 rider)

> **Renumbered 026 -> 085 on 2026-09-05 (Issue 069 citation-weight arbitration).**
> `026` was allocated twice: this plan and `026_kat_phase0_redaction.md` (the
> KAT redaction gate — 8 inbound citations vs 3 here); the KAT plan keeps 026.

**Filed:** 2026-08-26 (idle-loop housekeeping unit; queue item 11 online-search pool → Protocol A).
**Source pool:** two external GPU-compute articles fetched + read in full this session:
- frguthmann.github.io "Exploring ways to optimize compute shaders — Part 1" (F. Guthmann, 2020; GLSL/Vulkan Game-of-Life, Nsight-driven, honest failed-experiments)
- developer.nvidia.com "Optimizing Compute Shaders for L2 Locality using Thread-Group ID Swizzling" (L. Bavoil, NVIDIA, 2020; Battlefield V DXR denoiser measurement)

**Verdict going in:** 1 NEW rule + 1 spec-enhancement rider. The NVIDIA
thread-group-tiling candidate was KILLED at dedup as a positive instance of
the existing `l2-grouped-tile-schedule` (popcorn corpus — its fixture IS the
grouped-tiling swizzle shape); the B66 simd-chunk rider precedent applies.

## Tasks

- [x] T1 Dedup greps (dirty/write-elision/conditional-store; swizzle/morton/locality) — recorded in Research 084
- [x] T2 Author `skip-unchanged-writes-via-dirty-encoding` (A) into `entries_gpu_07.rs` + header count refresh
- [x] T3 Rider: extend `l2-grouped-tile-schedule` doc comment (applicability conditions + N≈16 + Morton-negative + 47%/L2-hit evidence; the pinned concept phrase `swizzled L2-cache-aware tile mapping` kept EXACT — batch4's concepts test is an exact-match `.contains(&str)`, the rider first landed in gpu_concept and was moved to the doc comment after that test failed)
- [x] T4 `KERNEL_CATEGORIES` A-list +1; count pins 372→373 / A 88→89 (mod.rs doc + ledger, tests.rs plan-003 table + 4 comments + the seed-count pin `seed_corpus_has_expected_entry_count` — the 19th count-pin site, found by test failure; router.rs comment + assert)
- [x] T5 Standing gates: `batch72_rules_present` + `batch72_concepts_covered` + `batch72_pseudocode_parses` (with the B70 `#[cfg(not(feature = "kernel_verify"))]` guard — re-hit once, fixed in-WIP)
- [x] T6 GOAT: G1 syn-parse ✓, G2 lib 677/0 default + 890/0/1i kernel_opt + 1221/0/13i all-features, G3 clippy 0 both states + **ci_feature_guard ALL LAYERS PASS** + retrieval floors HELD zero decay (blend@1 94.7, clippy@1 89.6, perf@1 96.2)
- [x] T7 Research note `.research/084_*`; AGENTS.md corpus count 372→373 + batch line; `.docs/11_domains/kernel_opt.md` count + batch row
- [x] T8 Snapshot update (`.distill/001_mining_queue_snapshot.md`) — same commit
- [x] T9 Commit `feat: batch 72 ...` + push

## Honest notes

- The `.research/` 083 dual-allocation collision (083_elo_selection_gate.md +
  083_omlx_external_distill_batch71.md) pre-exists; this batch uses 084 and
  flags the collision in the research note — renumbering either 083 breaks
  live AGENTS.md references (owner call, not fixed here).
- Regime honesty is load-bearing for the new rule: sparse ~20% win / dense
  measured LOSS (13.5ms vs 11.5ms) encoded in-concept (kernel_opt rules have
  no `context_notes` field — the B49 in-concept precedent).
