# 115 — Batch 136: the Lattimore "Graph Algorithms in Rayon" distill

**Status:** COMPLETE (2026-09-09 — Batch 136 landed: rust_perf 117 → 120, three NotBenchable-annotated concurrency rules, all T0–T5 gates green; queue snapshot updated in the landing commit).

## Source

- **Source:** David Lattimore, "Graph Algorithms in Rayon"
  (davidlattimore.github.io/posts/2025/11/27/graph-algorithms-in-rayon.html, 2025-11-27).
- **Provenance:** blog post, NO repo clone — cite-shape only, ALL fixtures original (the Batch-135
  discipline; same author feed as the mined "Wild Performance Tricks" — this is the UNMINED
  sibling post, POST-dating it). Fetched full text at mine time (16.8 KB HTML, 20 rayon hits).
- **License/citation:** blog cite-shape; no code copied; every fixture authored fresh from the
  described shapes.

## What it is (decision content)

A qualitative approach comparison for graph traversal (work unknown in advance) on a rayon pool,
with NO measured numbers — every rule lands NotBenchable-annotated decision-content, and the
author's own closing verdict IS the ladder's decision content:

1. `spawn_broadcast` + own job control (channel + park/wake per thread) — complex, STARVES the
   pool: a nested `par_iter` from those threads has only themselves (the rest are parked in
   private job control, unavailable to rayon).
2. **Scoped spawning** (`rayon::scope`/`in_place_scope`) — the composition-safe default DESPITE
   the per-task heap boxing and rayon's own "more expensive" doc warning; pick by COMPOSITION
   requirements first, allocation second. The doc warning is the trap the ladder corrects.
3. **Channel + `par_bridge`** — each work item carries a sender clone; drop the ORIGINAL after
   seeding; the channel closes when the LAST item drops (the correct dynamic-queue shutdown
   shape). Alloc-light, no per-task boxing — but (a) nested `par_iter` inside bridge-fed work can
   steal, then block pulling another item from the SAME channel while the held sender clone keeps
   it open → DEADLOCK (rayon tasks are thread-TIED; the blocked stealer never releases its
   sender), and (b) it does NOT COMPOSE: an owned value created in one work item cannot be split
   into further borrowed work items (borrow checker), and two queue-based jobs that need borrows
   released in sequence cannot combine. "Feels like writing async code without async/await."
4. async/await speculation (tasks not thread-tied → steal-then-block would resume on another
   thread) — the author has NOT tried it; recorded as the future-work note, NOT a rule.
5. Author's verdict: RETURN to scoped spawning; "a channel works fine for simple tasks and avoids
   the heap allocations, but it really doesn't compose at all well." Unboxed-spawning rayon patch
   idea (closures ≤ 32 bytes) = upstream API change, not a user-code rule.

## Tasks

- [x] T0 — Source fetched full-text; date + authorship + cite-shape verified; no clone needed
      (nothing to pin beyond the URL; the `.raw` discipline does not apply to blog cite-shape).
- [x] T1 — Dedup re-verification at the CURRENT corpus (117 rust_perf rules): zero corpus hits
      under `par_bridge|spawn_broadcast|scoped.?spawn|work.?item|crossbeam`; nearest neighbours =
      `channel-send-batching-unpark` (B65; producer-side throughput batching — orthogonal axis,
      this is queue STRUCTURE) + `bounded-channel-backpressure` (async send backpressure — not
      dynamic work distribution) + B135 `partitioned-mut-slices-zero-sync` (KNOWN disjoint work —
      the ladder is UNKNOWN work; complementary). READ each hit before ruling (done).
- [x] T2 — Entry authoring (original minimal fixtures, fn-wrapped snippets — the B122/123
      parse-gate lesson; NotBenchable annotations per the walk verdict).
- [x] T3 — Rider check: rayon `try_for_each_init` per-WORK-ITEM init (the 0.6.0 update post) —
      folded as a detection-shape note inside the ladder rule, not a standalone rule (upstream-
      known behaviour, 1-liner at authoring time).
- [x] T4 — Gates: batch136 presence/parse gates; lib default count pin; kernel_heal +
      kernel_heal+latent floors (rust_perf rules ride both); retrieval re-run byte-identical;
      ROUTER annotation re-pin per the Issue 020 contract; clippy 0 ×3; score-bench FLAT.
      Isolated `CARGO_TARGET_DIR` per the shared-box rule; CPU-only (no GPU-adjacent claim).
- [x] T5 — Count pins walked: corpus.rs header + entries.len + concurrency category comment +
      benchable/not lists + annotated counts + proposers mod.rs + AGENTS ×2 + README + .docs
      `11_domains/rust_perf.md` + queue snapshot update in the same commit.
