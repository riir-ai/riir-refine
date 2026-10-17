# Plan 253 — Batch 249: the TensorFold drift-pool mega-intake (f1aef9c3..c7137bb31, 1023 commits)

**Status:** IN FLIGHT — claimed 2026-10-16 00:2x UTC (4090 idle loop; claim row in `.distill/001`, expiry 2026-10-18T00:20Z). The standing named lead, re-measured at T0: the window is 1023 commits (NOT B219's 72 — the pool grew through the 1.0.x release wave: v1.0.0-beta.6, v1.0.11, the Python→native engine transition + app-side churn). Source: standing clone `E:/git/vendor/tensorfold` @ origin/main `c7137bb31a7dad924fb541a7068422127b9a94b2`, Apache-2.0 (since 0.6.0, the B196 provenance note).
**SLICE 1 LANDED 2026-10-18 ~03:5x +07 (zed-4090-1018):** T6 re-measure at the pool-growth condition — **+770 mid-claim** (`c7137bb31..origin/main`, window 1023 → 1793; non-merge 596; 344 kernel/src-touching); the T1/T2 read-agent reports (`.raw/b249/agentA/B_report.md`) adjudicated; **the rider set landed count-neutral** (A-10 → masked-in-place 2-bit face, A-15 → splitk train-acceptance face, B-5 → prompt-grid any-row era face; `batch249_slice1_riders_landed` needles + host-parse guards; kernel_opt lib 3145 ✓; retrieval kernel 100% / blended 90.6 ≥ floor — **floors HELD, no re-pin**; clippy clean). Record: `.research/281_batch249_slice1_rider_trio.md`. Work commit: see git log `batch 249 slice 1`. **REMAINING `- [ ]` rows (T3 dedup-adjudication of the own-rule set, T4/T5 authoring, T7/T8) carry to slice 2** — the strongest-8 lists are in the agent reports; the +770 delta pool re-enumeration rides slice 2's T0 re-fetch (staging pins are stale by the 1.0.12 wave).

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
- [x] T6. GOAT gates: batch249 needle+parse gate first-run; kernel_opt lib tests;
      default lib standing count; clippy ×2 postures (default + kernel_opt,latent_retrieval);
      retrieval floors re-measure at the kernel_opt,latent_retrieval posture (21/21 HELD
      or re-pinned with attribution). **SLICE-1 DISCHARGED 2026-10-18**: needle gate first-run PASS,
      lib 3145 ✓, retrieval kernel 100% / blended 90.6/94.2/95.3 ≥ floor — floors HELD no re-pin
      (riders are gpu_concept-only; the fixture set + centroids unmoved), clippy clean both postures.
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
