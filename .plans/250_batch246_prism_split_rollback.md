# Batch 246 — the prism fork delta: split-alignment + pending-rollback fork laws

**Status:** COMPLETE 2026-10-16 (shikuwa/4090 lane — all tasks done; gates green; landing commit follows this plan tick)

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

- [x] T0: quotes re-verified at the pins at conversion (vendor clone at `71c1473207` + `5ae2e169ea`) — both diffs read in full this session, every needle verbatim
- [x] T1: kernel_opt rule `entries_gpu_100.rs` (new file) + mod.rs wiring + L-category membership + counts (847→848, L 21→22)
- [x] T2: rust_perf rule `entries_31.rs` (new file) + mod.rs wiring + counts (217→218, caching 15→16)
- [x] T3: count-neutral rider on `cost-aware-verify-length-survival-cost-table` (entries_gpu_86) — the platform-keyed verify-cost face
- [x] T4: batch246 needle + parse tests (tests.rs both domains) + count-pin walk (kernel_opt ×14 historical asserts + router.rs; rust_perf ×7 incl. the serde/drafter 163→164 pair)
- [x] T5: gates — default lib 2678/2678 · kernel_opt lib 3133/0/1i · corpus tests 154/154 + 405/405 + router 23/23 · clippy ×3 postures 0 errors (7 doc_markdown healed via cargo refine) · retrieval floors HELD no re-pin (blended 90.6/94.2/95.3; kernel self-variants 100.0%; 58.55 s)
- [x] T6: score-bench run #185 appended + gen2/gen3 freeze e2e --ignored both ok (57.84 s / 39.56 s)
- [x] T7: docs — AGENTS.md ×2 rows + README + .docs/11_domains/rust_perf.md + kernel_opt.md ledger rows + Research 278 born-complete
- [x] T8: landing row in .distill/001 + plan ticked COMPLETE + commit/push

## Numbering

- plan 250 (`.plans/.highwater` 249→250)
- research 278 (`.research/.highwater` was STALE at 275 with 276 on disk — repaired; 277 = the M3 Batch-245 claim, theirs; write-back 278)
- Batch 246 (Batch 245 = the M3 lane's live claim, plan 248)
- dual_allocation_gate: 0/0/0 at allocation (fetched clean)
