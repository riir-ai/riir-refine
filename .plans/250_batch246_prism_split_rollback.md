# Batch 246 — the prism fork delta: split-alignment + pending-rollback fork laws

**Status:** IN_FLIGHT (claimed 2026-10-16, shikuwa/4090 lane, 48h expiry)

Source: PrismML-Eng/llama.cpp (MIT, the league opponent fork), branch `prism` delta
`e3de715222..5ae2e169e` (3 commits, polled fresh this session at the vendor clone
`E:/git/vendor/prismml-llama.cpp`; quotes verified verbatim at the pins):

- `71c1473207` #363 "ggml : support -sm tensor for Hadamard-folded qwen35 (Bonsai 2)"
  → ONE kernel_opt rule, category L (layout): the SHARD-ALIGNMENT law family.
- `5ae2e169ea` #354 "memory : keep a pending rollback across seq_cp, and default
  MTP draft depth to 2 on CUDA" → ONE rust_perf caching rule (the fork-carries-
  pending-rollback law) + ONE count-neutral rider on
  `cost-aware-verify-length-survival-cost-table` (entries_gpu_86, the platform-
  keyed verify-cost face: CUDA verify cost grows faster with depth than Metal;
  depth-2 default when draft-mtp + CUDA offload + no explicit flag).
- `b858f3bdaa` #362 (tests: portable popcount) = NULL, recorded only.

League intel (no bout owed): the fork now serves OUR league model Bonsai 2 27B
PQ2_0/Q2_0 under 2-GPU tensor split (2x H200, mean KLD 2.5e-4 vs single GPU,
top-1 99.3%); no new prism-b tag (ladder top 10777) and no ggml-cuda absorption
movement — the watch-971 bout trigger does NOT fire.

## Tasks

- [ ] T0: quotes re-verified at the pins at conversion (vendor clone at `71c1473207` + `5ae2e169ea`)
- [ ] T1: kernel_opt rule `entries_gpu_100.rs` (new file) + mod.rs wiring + L-category membership + counts (847→848, L 21→22)
- [ ] T2: rust_perf rule `entries_31.rs` (new file) + mod.rs wiring + counts (217→218, caching 15→16)
- [ ] T3: count-neutral rider on `cost-aware-verify-length-survival-cost-table` (entries_gpu_86) — the platform-keyed verify-cost face
- [ ] T4: batch246 needle + parse tests (tests.rs both domains) + count-pin walk
- [ ] T5: gates — default lib + kernel_opt lib, clippy × postures, retrieval floors (kernel_opt+latent_retrieval; expect HELD, re-pin only if moved)
- [ ] T6: score-bench appended + gen2/gen3 freeze regen (rust_perf corpus delta owed, plan 137 T7.5)
- [ ] T7: docs — AGENTS.md ×2 rows + .docs/11_domains/rust_perf.md + kernel_opt.md + README counts + Research 278 born-complete
- [ ] T8: landing row in .distill/001 + plan ticked COMPLETE + commit/push

## Numbering

- plan 250 (`.plans/.highwater` 249→250)
- research 278 (`.research/.highwater` was STALE at 275 with 276 on disk — repaired; 277 = the M3 Batch-245 claim, theirs; write-back 278)
- Batch 246 (Batch 245 = the M3 lane's live claim, plan 248)
- dual_allocation_gate: 0/0/0 at allocation (fetched clean)
