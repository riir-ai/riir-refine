# Research 241 — Batch 206: the 10-07 staged leads (llama.cpp #29442 + #28782)

**Status:** CONVERTED — 2026-10-07 (Plan 206; both members graded at the pins and landed as 2 kernel_opt rules; corpus 769 → 771: D +2).

**Box:** shikuwa/4090 (riir-refine idle loop, post-Batch-205).
**Staging provenance:** riir-ai `.benchmarks/rematch/2026-10-07_watch.md` (the 10-07 subject-read first grades — B− for #29442 "conversion/narrowing batch-admission family", C+ for #28782 "stream-assignment family"; "quote verification owed at any batch composition" — this batch paid that debt); the queue position came from `.distill/001_mining_queue_snapshot.md` (the BATCH-205 row: "the 10-07 staged leads front the next kernel slot").

## Sources (MIT; clone `.raw/b206-llamacpp` depth-150 master, removed at close)

| Sha | Repo | Subject | PR | Watch grade | Disposition |
|---|---|---|---|---|---|
| `ab09ea4c148868aa4919b36e86962bf6ff2f32a7` | ggml-org/llama.cpp | cuda: BF16/FP16 conversion to f32 chunking | #29442 | B− | CONVERTED → `widening-conversion-row-chunk-staging-budget` (D) — carries BOTH halves: the 512 MiB row-chunk staging bound AND the `ldc` stride-respect admission widening (one story, one rule) |
| `51ce9c11a6f2dfa895696c0048c4333e8953b728` | ggml-org/llama.cpp | ggml-cuda: use per-thread stream for buffer-init padding memset | #28782 | C+ | CONVERTED → `per-thread-stream-blocking-op-explicit-sync` (D) — the legacy-default-stream escape with the completion guarantee kept; the ROCm `-j` re-enable rides as hazard evidence |

Both commits merged 10-06 (direct squash commits on master, not merge commits — located via `git log --grep` at the window `5e03bdd..c479922a`).

## T0 adjudication (grade revisions at composition)

- **#29442 upgraded B− → B**: the watch pass read it as a conversion-chunking only; the full diff shows TWO coupled mechanisms — the staging budget AND the dst-admission widening (`ggml_is_contiguous(dst)` → `ggml_is_contiguous(dst) || (compute_type == GGML_TYPE_F32 && ggml_is_contiguous_1(dst))`, with `ldc = nb1 / sizeof(float)` replacing `ne0` at all FIVE cuBLAS call sites, strideC `ne1*ldc`). The rule keeps them as ONE story because the chunk arm is ILLEGAL without the stride half (chunk writes land at original dst offsets; the old ne0 lead dimension would scatter across padded rows — the in-source comment states it: "F32 chunks can have padding between rows to preserve the original destination stride"). Decision content: two-conjunct chunk admission (staging exceeds budget AND `rows_per_chunk < src0->ne[1]`), budget-derived row count, shallow-header row slices re-entering the same impl (guard in the outer wrapper → no recursion), byte-stride lead dimension.
- **#28782 C+ held**: 3 code lines + a CI carve-out removal, but the mechanism is crisp and the CI half is the HAZARD EVIDENCE (multi-threaded `buffer_init_tensor` was racing on the shared legacy default stream — that is why ROCm ran `test-backend-ops` serialized, `args_extra=""`). Context law verified at the parent: the file's OTHER ~10 memset entry points already ran `cudaMemsetAsync(cudaStreamPerThread) + cudaStreamSynchronize(cudaStreamPerThread)` — buffer-init's padding memset was the LAST blocking holdout, and precisely the one concurrent host threads hit.

## Quote verification (the Batch-31 discipline)

All quotes re-verified **verbatim** at the pins; line numbers read from `git show <sha>:<path>` at the POST-commit files, NOT hunk-derived:
- ggml-cuda.cu @ ab09ea4: the padding comment :1412, the widened assert :1413-1414, `ldc` :1418, `max_src0_convert_size` :1661, the admission conjuncts :1662-1666, `rows_per_chunk` :1668, the chunk guard :1670, the loop :1674-1679.
- ggml-cuda.cu @ 51ce9c1: `padded_size` :761, the guard :763, the memsetAsync pair :765-766; parent's blocking form at 51ce9c1^ :765; parent's pre-existing per-thread sites :775-776/:844-845.
- ci/run.sh @ 51ce9c1: the ROCm block REMOVED (parent had `# TODO: fix multi-threaded for ROCm` at :652 + the GG_BUILD_ROCM `args_extra=""` if-block); the new file carries only the MoltenVK/OpenVINO carve-outs.

## Grades

- `widening-conversion-row-chunk-staging-budget`: **B** (upstream-merged, a bounded-staging + admission-widening law with the stride coupling as the novel half; no published figures — NotBenchable annotated: the win class is peak-staging memory + padded-dst admission, not a kernel-internal throughput arm).
- `per-thread-stream-blocking-op-explicit-sync`: **C+** (thin diff, crisp law; the CI re-enable is behavioral evidence; NotBenchable annotated).

## Dedup greps (both vocabularies, zero prior owners)

- Conversion/chunk family: the corpus's chunk-vocab rule names are submission limits (`chunk-encoder-submits-below-driver-limit`), work-stealing floors (`chunk-downsize-work-stealing-microtile-floor`), recurrence chunks (B203's pair), and the A-category block-bounded PARTIALS rule (`row-independent-pass-block-bounded-scratch` — fixed block size for row-independent attention passes, NOT a budget-derived conversion slice). No widening-staging byte budget existed. Zero hits for `max_src0_convert`/`512 MiB` staging, and the `ldc` grep found only the shape-sentinel rule's `ldc == 4` mention (entries_gpu_46 — different mechanism entirely).
- Stream family: `captured-graph-init-unconditional-and-stream-ordered` owns the capture node-set completeness + stream-ordered VISIBILITY edge (read in full — different axis); `producer-store-evict-last-pins-cross-stream-consumer` is cache-eviction policy; the rust_perf B200 one-stream rules are population-keyed execution modes. No blocking-default-stream rule existed. Zero hits for `cudaStreamPerThread`/`per-thread stream`/`default stream`.
- Near-neighbor kinships cited in-rule (all verified present): `row-independent-pass-block-bounded-scratch`, `chunk-encoder-submits-below-driver-limit`, `stream-invariant-operand-skips-staging-pipeline`, `narrow-m-l1-panel-sizing`, `ids-path-rebinds-stride-meaning`, `captured-graph-init-unconditional-and-stream-ordered`, `producer-store-evict-last-pins-cross-stream-consumer`, `lone-stream-graph-replay-mode-switch` (rust_perf).
- The two same-batch rules cross-distinct (asserted by the batch206 contracts gate): staging-budget vs stream-choice — disjoint mechanisms, neither cites the other.

## Gates (all green)

- batch206 parse+contracts 2/2 FIRST SHOT (2 ids × parse both sides; 15 needle markers + NotBenchable + batch/plan stamps + the cross-distinct pair).
- kernel_opt+latent_retrieval lib **3072/0** (3 ignored = the documented freeze-regen e2es) — 1243-1246 s on this box (the suite is SLOW here: dozens of draft:: retrieval/headroom tests at 60 s+ each; budget ~21 min, never two full runs in one command).
- Count pins re-pinned 769 → 771 with honest labels: tests.rs ×4 (corpus_size, categories exhaustive, sum, grouped) + the batch116_rules_present pin + router.rs:472 + mod.rs D comment + AGENTS.md ×2 + README ×1. **Two en-route pin catches, both the pins doing their job**: (1) the per-category `expected` array still asserted `Dispatch == 171` (I had updated the D comment but initially missed the array — caught by the first full-suite run's single failure); (2) `batch116_rules_present` carried its own 769 pin with the Batch-205 label (caught by the same run; the B205 session's relabel list said "tests.rs ×4" but this pin is a fifth site — the count-walk miss class, repaired with the honest label).
- clippy `-D` ×3 postures clean (default / all-features / no-default; 4 doc_markdown mechanicals in the new entries_gpu_76.rs header fixed in-place — the B192 class, trivial backticks).
- retrieval 21/21 floors **HELD** (233.8 s, no decay, no re-pin).
- freeze/score-bench NOT owed (zero clippy-corpus delta, P30).

## Queue after

- nnethercote R134 rides the next rust_perf slot (none staged — the 10-06/10-07 upstream pool is kernel_opt-class and now fully converted).
- Walk #14 opens Tue 10-13.
- The Research-239 follow-up pool (same-file `q2_0_q8_0`, ggml-org arm quants, karusrus i8mm pin-before-reading, SYCL XMX `a14c7de9` watch-only) stays in intake.
- riir-ai Issue 1036 KB5121794 rerun + the other owner-gated items unchanged.
- Plan 204 T11 (the M3-Max GOAT for the B204 NEON rule) rides the M3 idle loop.
