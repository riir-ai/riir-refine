# Issue 034 — DFlash2 selection-headroom POC (drafter mechanism × ruliology, no target model)

**Filed:** 2026-08-20 · **Repo:** riir-clippy
**Refs:** [Research 077](../.research/077_dflash2_draft_only_distillation.md) · [Plan 017](../.plans/017_dflash2_selection_headroom_poc.md) · owner ask (DFlash2-as-drafter for `cargo heal`, draft-quality + ruliology coverage, NO full model this POC)

## Context

Owner wants the DFlash2 drafter proven against our stack without the Qwen3.8-27B target. Research 077
established (source receipts): the released GGUF cannot generate standalone — no `tok_embd`, no
`lm_head`, encoder consumes 5×5120 target-feature floats/position; `llama-context.cpp:157` refuses
standalone contexts outright. What transfers modellessly is the blog's selection-headroom mechanism
(Recall@1 85.4% vs Recall@16 99.5%; +2.0M-param pairwise path selector) — the same phenomenon our
Bench 024 measured as retrieval pool crowding. This issue tracks the POC; the artifact-based L4 arm
(POC-B: llama.cpp dflash2 build @ `5ecbe1a` + drafter Q4_K_M already staged, needs ~17 GB target) is
deferred to the next POC per owner call — local weights kept, untouched.

## Tasks

- [ ] T1 headroom probe: recall@1 vs recall@{8,12,16} on the Issue 017 fixture set + clippy-bench
      composite set; classify misses as selection-misses (correct candidate in pool, not chosen)
      vs true-retrieval-misses
- [ ] T1-stop-rule: recall@16 − recall@1 < 2pp on both surfaces → record negative, close, no selector
- [ ] T2 `dflash_selector` feature: keep-K + bilinear pair coherence over ModellessEmbedder
      directions (A/B codebooks, BLAKE3-seeded, sigmoid gate) + greedy path trace; composes after
      rerank, before fix application; default-off
- [ ] T3 gates G1-G5 (≥50% headroom recovery on real ground truth; created_rate no-regress;
      <100 µs/span; alloc-free; feature-off byte-identical)
- [ ] T4 bench doc `.benchmarks/041_*` + Research 077 PoC-addendum backfill + promotion decision

## Close condition

Either the measured negative (T1 stop rule) or gates G1-G5 recorded with raw tables — then remove
this file per the noise-reduction rule, keeping the bench doc as the record.
