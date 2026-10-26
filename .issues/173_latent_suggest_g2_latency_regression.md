# 173 — `latent_suggest_goat` G2: per-chunk latency 1.6× over the macOS bar at its own feature set
**Status:** OPEN — T1 profiled + first fix landed (riir-neuron-db `ef8599a`, 1587 → 1062 µs/chunk); still 6% over the 1000 µs bar, T2 continues

## Measurement
Box: amo (M5 Ultra, AC power), quiet: no other cargo/rustc, load 3.9 on 30
cores. `--release`, `g2_latency_bar_per_chunk` alone, 480 chunks, two runs each.

| feature set | µs/chunk | bar (macOS) |
|---|---|---|
| target's own `latent_retrieval,clippy_lints,rust_perf` | **1592, 1586** | 1000 |
| `--all-features` (ci_feature_guard Layer 5c) | **4830, 4832** | 1000 |

Two separable effects:
1. **Own-set regression (~1.6×):** the `< 1 ms` bar is the M3 / Plan-005
   calibration; this box is faster than an M3, yet it misses by 59%. The
   suspect is retrieval cost scaling with mined-corpus growth (rust_perf
   198 → 200+ at batch 239, `4085aa24`, and the mining lane keeps adding rows).
   **Not verified:** the first step is a profile of `LatentMatcher::suggest_span`,
   not a guess.
2. **All-features posture (~3× on top):** `from_enabled_domains()` also
   indexes kernel_opt/sec/shader_perf, so the bar's population differs. The
   bar needs a posture-aware form (the Issue 143 dual-pin precedent), or a
   per-domain budget.

## Tasks
- [x] T1 — profiled with macOS `sample` on a line-tables release build (6 × 1 s windows over looped G2 runs). The hot path is riir-neuron-db, not riir-refine: `Bm25Index::search` (~150 samples) + **std SipHash on its per-posting score accumulator** (`Sip13Rounds::write` + `hash_one`, ~175 — more than the search body), then `index::wedge_l1_norm_8` (~75), `retrieve_diverse_two_stage`, and `rank_top_k`'s collect. (`Bm25Index::index` / `ShardIndex::insert` / papaya drop are matcher build, not per-query.)
- [ ] T2 — fix the hot terms; re-measure against the 1000 µs bar. **No bar re-pin to hide the regression.**
  - [x] BM25 accumulator → foldhash: riir-neuron-db `ef8599a` (G1 bit-identical — tie-break is a total order by key, per-doc sums run in query-term order; G2 1.45–1.48× per search at 9.6k postings; promoted under the `bm25` feature, which is what this repo enables — `default-features = false`, so a neuron-db `default` promotion would never have reached here). **End-to-end G2 here: 1587 → 1059/1065 µs/chunk** (built against the patched tree in a symlinked /tmp workspace).
  - [x] NEGATIVE — D=8 constant-index specialization of `wedge_l1_norm_8` (bit-identical, 200k random pairs): **1.00× on three runs** (5.29–5.31 ns/call both arms). LLVM already constant-folds the generic kernel's dim=8 + fixed shifts after inlining; the term's profile weight is its CALL COUNT (hull candidates × greedy steps in `retrieve_diverse*`), so the lever is algorithmic (fewer span evaluations), not the kernel. Not shipped.
  - [ ] Remaining per-query ranking after the fix (re-profiled): BM25 search body ≈ wedge call volume > foldhash > `rank_top_k` collect. Candidates: dense per-doc accumulation (a u32 doc ordinal in `Posting` → `Vec<f32>` scratch, no hashing at all — needs stable ordinals under concurrent index/remove); pruning `retrieve_diverse`'s span evaluations; query-term dedup/stop-terms for code chunks (very long postings for `fn`, `let`, punctuation).
- [ ] T3 — posture-aware G2 for the all-features build (dual pin with measured headroom, or per-enabled-domain budget), measured alone and with the box state recorded.
