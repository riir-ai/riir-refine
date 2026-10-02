# Plan 194 — Batch 195: the mlxfast-bonsai2 accept-wave distill (delta `0c61273..0c7c392`)

**Status:** COMPLETE 2026-10-02 (idle-loop unit; B194 landed 10-01 — the one-batch-per-day slot was this one). Source: `Layr-Labs/mlxfast-bonsai2-27b-engine`, the standing vendor clone `/Users/katopz/git/vendor/mlxfast-bonsai2`, NET delta `0c61273..origin/main` = **25 commits, 22 files, +9150/−777** (fetched 2026-10-02; tip `0c7c392`). MIT verified at the tip. LANDED: 9 rules (`entries_gpu_63.rs`) + 3 riders + the MTP-slice leads STAGED (Research 229); corpus 717→726 (B 77, C 51, D 162, E 68, N 56, O 7). Gates: batch195 parse (9) + contracts + riders green, kernel slice 396/0, lib 2423/0, clippy -D ×3, retrieval 21/21 HELD (234s).

## The delta's shape (coordinator read, pre-staging)

- **NEW MTP lane** (`ContinuousBatchingV2/MTP/`: `CBv2AcceptGlue`, `CBv2PromptLookupDraft`, `EngineLoopV2+MTPExecution`, `+MTPFinalize`, `+MTPTargetVerification` + `Qwen35MTP`/`Qwen35MTPTopTwo` + `CBv2TopTwo`) — prompt-lookup drafting + top-two target verification wired into the engine loop.
- **NEW `SequenceKV/ContiguousKVBackend.swift`** (356 lines) + `FullSequenceKV.swift` (+9) — a contiguous KV storage backend for the batching engine.
- **`Hadamard.swift` rework** (371 lines changed) + `PrismHadamardQwen35`/`PrismHadamardCheckpointValidation` — the checkpoint-validation + kernel shape may carry new numerics/layout rules.
- **`KVCache.swift`** (36 lines), `AttentionV1`/`EngineLoopV2` (the accept/evict half), `DFlash2Draft`/`Qwen35DFlash2Assistant` (the drafter seam), `Qwen35A3B*` (a new model variant — A3B target verify).
- NO tools/ reorg this wave (the B190 exclusion class measured ABSENT).

## Tasks

- [x] T0: pin the shas in this plan (base `0c61273`, tip `0c7c392` — recorded); verify MIT LICENSE at the tip; confirm the 25-commit log is accept/validate submissions only (no force-push rewrite between fetch and pin: re-read `rev-list --count 0c61273..origin/main` at authoring = 25).
- [x] T1: stage the net diff — 2 parallel subagents over disjoint slices: (A) MTP/draft/accept half, (B) KV/layout half. Quotes verified at the pin; **delta-freshness check added beyond the plan** (`git diff 0c61273..origin/main | grep "^+"` per candidate) — it KILLED the staging's GPUWake lead as a B193 duplicate and caught the reservation rule as present-at-base (provenance noted in-rule).
- [x] T2: dedup RE-RUN at the live corpus (rule-id level, both vocabularies): two near-collisions adjudicated — `exhaustive-domain-input-patch-bit-identity` (B115: enumerate-to-PATCH vs this batch's enumerate-to-ADMIT — distinct axes, both stated in-rule) and `hazard-tracked-aliasing-scratch-allocator` (B105: within-encoder tracker vs the cross-queue retention arm — distinct, composed in-rule); `seed-first-command-buffer-earliest-submission` (B193) absorbed the GPUWake lead outright.
- [x] T3: author entries — 9 rules in `entries_gpu_63.rs` (O pair, D trio + 1 more D, B, C, N, E); yield at the honest end of the plan's 4–8 estimate (9) because the MTP slice's detail pass truncated — its leads STAGED, not dropped (Research 229 §Staged).
- [x] T4: ORIGINAL minimal fixtures; decision content in-rule; source-reported provenance labeled (mechanism-only honesty flags on all nine — the wave's only numbers are the ~10-20 ms/GB wiring figure and B193's own wake figure, both quoted as provenance).
- [x] T5: standing gates — batch195 parse gate (9) + contracts gate + riders gate GREEN; kernel slice 396/0; lib default 2423/0; clippy `-D` clean ×3 postures (default / kernel_opt / rust_perf+kernel_opt); retrieval floors 21/21 HELD (234s, /tmp target dir).
- [x] T6: count pins walked — router total 717→726 + the four static totals + the per-category table (B 76→77 incl. the B193-comment-missed repair, C 50→51, D 159→162, E 67→68, N 55→56, O 5→7) + AGENTS.md kernel bullet + kernel_opt.md seed row + README ×2.
- [x] T7: score-bench + freeze regen — NOT owed (kernel_opt-only, zero clippy-corpus delta, P30).
- [x] T8: Research note `.research/229_mlxfast_wave2_batch195_conversion.md` (staging table + drops + lane intel; the MTP-slice truncation + staging recorded honestly).
- [x] T9: `.raw` cleanup N/A (the standing vendor clone stays); `.distill/001` queue row updated at landing.
- [x] T10: docs landed (AGENTS.md + kernel_opt.md + README); commit `feat: batch 195: ...` + push.

## Notes

- The lane intel axis (mandatory): does the wave move any scored league cell definition? The MTP/top-two lane rides DFlash2 spec-decode — riir-ai Issue 1011's decode-axis watch may tick again. Detection-only; no bench run from this repo (Plan 337 + the M3 light-work rule).
- Promisor-clone law: every quote via `git show <tip>:<path>`, never a working-tree grep.
- Yield honesty: a submission-wave this large can be mostly glue (the MTP lane is engine plumbing); drop classes recorded per the B190 precedent (12 below-rider-grade drops recorded there).
