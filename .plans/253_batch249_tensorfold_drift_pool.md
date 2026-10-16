# Plan 253 — Batch 249: the TensorFold drift-pool mega-intake (f1aef9c3..c7137bb31, 1023 commits)

**Status:** IN FLIGHT — claimed 2026-10-16 00:2x UTC (4090 idle loop; claim row in `.distill/001`, expiry 2026-10-18T00:20Z). The standing named lead, re-measured at T0: the window is 1023 commits (NOT B219's 72 — the pool grew through the 1.0.x release wave: v1.0.0-beta.6, v1.0.11, the Python→native engine transition + app-side churn). Source: standing clone `E:/git/vendor/tensorfold` @ origin/main `c7137bb31a7dad924fb541a7068422127b9a94b2`, Apache-2.0 (since 0.6.0, the B196 provenance note).

## Method (the B196 mega-intake shape)

2 disjoint read subagents over commit slices (A: kernels/quant/attention/serving-exactness;
B: host scheduling/memory/graph lifecycle/app-adjacent-sift) + coordinator dedup
(3+ vocabularies per candidate vs B188/B196/B215/B219 landed sets) + coordinator
quote verification at the pin (verbatim at `c7137bb31`). Count-neutral riders preferred
over new hosts; a graded-too-thin honest close is valid (corpus stands 848).

## Tasks

- [ ] T1. Slice the drift pool: kernel/serving-relevant commits vs app-side churn
      (path-filtered commit list into `.raw/b249/`).
- [ ] T2. Two parallel read subagents over disjoint slices; each returns candidates
      (pattern, bounded fix-space, category, rationale, SOURCE QUOTE + file:line at the pin).
- [ ] T3. Coordinator dedup: 3+ vocabularies per candidate against the landed corpus
      (B188/B196/B215/B219 sets incl. `entries_gpu_*`, `entries_popcorn`, rust_perf);
      false positives adjudicated; drops recorded with reasons.
- [ ] T4. Quote re-verification at `c7137bb31` (verbatim, file:line) for every
      candidate that survives dedup.
- [ ] T5. Author: corpus entries (+ the batch249 standing syn-parse/needle gate;
      count-neutral riders where the host exists).
- [ ] T6. GOAT gates: batch249 needle+parse gate first-run; kernel_opt lib tests;
      default lib standing count; clippy ×2 postures (default + kernel_opt,latent_retrieval);
      retrieval floors re-measure at the kernel_opt,latent_retrieval posture (21/21 HELD
      or re-pinned with attribution).
- [ ] T7. Docs: Research 281 born-complete; AGENTS ×2 corpus-count lines (only on a
      count delta — count-neutral riders leave them standing); `.docs/11_domains/kernel_opt.md`
      Batch-249 row; queue-snapshot landing row; plan ticked COMPLETE.
- [ ] T8. Commit `feat: batch 249 ...` + push (freeze/score-bench owed ONLY on a
      clippy-corpus delta, P30).

## Non-goals

- No consumer-repo edits (this is a corpus batch).
- No promotion of kernel_opt to default (its standing GOAT gate remains pending).
- The M3's Batch 248 files (rust_perf entries + pins for simdutf/unicode-normalization)
  are theirs — zero edits there.
