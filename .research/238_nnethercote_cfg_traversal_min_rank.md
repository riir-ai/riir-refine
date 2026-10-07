# Research 238 — the nnethercote min-rank worklist rule (Batch 202, Plan 201)

**Status:** RECORD — distilled and landed as `dataflow-fixpoint-convergence-traversal-order` (rust_perf/complexity, `entries_16.rs`, NotBenchable annotated). One rule; the rust_perf slot freed by Batch 201's landing.

## Source

- **PR**: rust-lang/rust `#160193` "Improve CFG traversal" (author nnethercote). **Merged 2026-08-01T04:45:44Z, milestone 1.99.0, r=cjgillot; merge commit `cb9d1b0640549f1b041aae430dc413ce93f8c204`; head `4aadc68a8a49`.**
- **Blog**: nnethercote, "How to speed up the Rust compiler in September 2026" (2026-09-30), the Dataflow analysis section.
- Read live 2026-10-07 (the 4090 batch session): the blog HTML in full + the merged `compiler/rustc_data_structures`/`rustc_mir_dataflow` sources at the merge commit via raw.githubusercontent. Ephemeral fetch files removed at close.

## The mechanism (verbatim, `framework/mod.rs` @ `cb9d1b06`, :297-312)

> Every reachable basic block has a *rank*: its position within `rpo`. Rank order is dataflow order: for every edge A -> B that is not a back edge, rank(A) < rank(B). This is independent of basic block numbering (which depends on the vagaries of CFG construction).
>
> The CFG traversal uses a "min-rank" algorithm. First, all reachable basic blocks are marked as dirty. The loop-head invariant is that `curr_rank` always points to the minimum-rank dirty block in `rpo`. Before processing that block we mark it as clean. If the processing dirties a block with a rank lower than or equal to `curr_rank` (via a back edge, which could be an edge-to-self) then `curr_rank` is set to that lower-or-equal rank. After the block is processed, if `curr_rank` doesn't point to a dirty block it is moved to the next dirty block, and we iterate again.
>
> This algorithm ensures each basic block is processed only after all its dirty predecessors (ignoring back edges). When a back edge dirties an earlier block we return to that earlier block immediately, which avoids processing later blocks with possibly soon-to-be-stale information. Loop-free code is processed in a single pass.

Implementation anchors: the rewind `dirty.insert(target_rank); curr_rank = curr_rank.min(target_rank);` (:361-362); the advance `match dirty.first_set_at_or_after(curr_rank)` (:367-369); the rank assignment `let rank = if Self::Direction::IS_FORWARD { i } else { last - i };` (:331 — backward analyses fake a postorder by flipping the rank mapping, :316-317).

## The numbers (blog, verbatim)

> The old algorithm required 1.5 million calls to apply_effects_in_block to reach a fixpoint for the EverInitializedPlaces analysis used by the borrow checker; the new algorithm requires 90,000. This gave an enormous ~30% wall-time reduction for a check build of this crate.

The crate: cranelift-codegen; the function: "over 18,000 basic blocks". "For most code the new algorithm makes no difference" (blog) — the regime clause.

## ⛔ Drift corrections carried (vs the 10-02 staging row in `.distill/001`)

1. **The 17% figure is `#160033`'s, NOT `#160193`'s.** The blog attributes "reduced instruction counts on the match-stress benchmark by 17%" to #160033 (EverInitializedPlaces projection-data pruning) — a DIFFERENT PR in the same post. The 10-02 row's "17% reduction in instruction counts for a Check Full build" (attributed to commit 4aadc68's message) misattributed it; the entry carries the correction and the batch-202 gate asserts the needle.
2. **The 1,508,264 / 89,809 precise pair was NOT re-found** in the PR issue body/comments/review-comments this pass (greps empty) — NOT carried. The blog's rounded 1.5 million / 90,000 + the merged implementation comment carry the rule. (The 10-02 session read "the PR #160193 thread" — the pair may live in a thread view this pass's API reads did not reproduce; if a future session re-finds it, adding it back is a one-line entry edit.)
3. Mechanism precision (the 10-02 correction, HELD and strengthened by the merged source): the old FIFO worklist was already SEEDED in dataflow order — the defect is the RE-DIRTY policy (push-back). The rule is "re-process in dataflow order (min-rank)", never "seed in RPO".

## Grade

**B** — NotBenchable decision content (the win is a fixpoint's iteration count on the workload's own CFG; no standalone microbench axis; the B182/186 corpus-only precedent). Suggestion-only in spirit: an admission shape for any fixpoint in healed code, never a mechanical rewrite.

## Gating record (this batch)

- `batch202_rules_present_and_parse` ✓ (presence + category + syn-parse + notes needles incl. the 160033 drift correction)
- `seed_corpus_total_and_category_count_pinned` ✓ (179 → 180)
- `cargo clippy --all-targets` — zero findings from the batch (one doc_markdown healed en route; the residual `hard linking` incremental-cache line is the known Windows box artifact, plus the pre-existing `riir-kat` build-script warning)
- retrieval floors ✓ 20/20 (`--features latent_retrieval --lib retrieval`, 32 s — no decay; no re-pin)
- freeze + score-bench NOT owed (zero clippy-corpus delta, P30)
- dedup grep: fixpoint/worklist/traversal-order/dataflow over rust_perf + kernel_opt — zero collisions (the pre-plan pass; recorded in the entry's NOVEL clause)
