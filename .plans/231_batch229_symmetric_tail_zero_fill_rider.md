# Plan 231 — Batch 229: the #29533 symmetric tail zero-fill rider (count-neutral)

**Status:** COMPLETE — all tasks green this session (claim `d7a8b27e`; the
quote verification at the pin preceded the plan). Landed as Batch 229
(corpus 803 stands; retrieval floors HELD; freeze/score-bench NOT owed).

Source: ggml-org/llama.cpp @ `411d548515017ca75c2a12c39659b6e7e1551dd7`
(#29533, MIT, landed 2026-10-13 11:03Z) — "vulkan : split matmul dispatch when
it would exceed maxComputeWorkGroupCount". Quotes verified at the pin via the
.patch endpoint THIS session (subject, the zero-fill sub-commit message, the
`block_a_to_shmem` `is_in_bounds` else-branch, the VK_WHOLE_SIZE assert, the
per-buffer-kind `b_col_bytes` pair, `padded_n - n_base` push-constant shift).

Count-neutral: corpus stays 803 (rider-appends on the host's `gpu_concept`);
freeze/score-bench NOT owed (zero clippy-corpus delta, P30).

## The law (from the verified quotes)

1. **SYMMETRIC TAIL ZERO-FILL**: in a tail-truncated K loop, EVERY operand
   stager must write the FULL staged tile — the consumer (mma) reads all
   BK_STEP steps regardless of the logical bound. The bug: "The A side skipped
   the shared memory write for the k steps past end_k, while the B side was
   already told the bound and zero filled. The A registers are read for all
   BK_STEP steps, so the stale scale went into the dot product and produced
   NaN at k = 32. Pass the bound to block_a_to_shmem and write zeros when it
   is false, for every DATA_A type, mirroring the B side." The repair shape:
   the guarded CALL (`if (in_bounds) block_a_to_shmem(...)`) becomes an
   UNCONDITIONAL call with the bound as a parameter — the else-branch writes
   zeros INCLUDING the dequant multiplier (`dm = 0.0f`) — the stale `.dm`
   scale was the NaN carrier, not just the payload bits.
2. **THE SUBBUFFER WINDOW LAW** (the dispatch-split half, same commit): a
   chunked dispatch shifts the B/D **subbuffer column windows**
   (`offset += n_base * col_bytes; size -= ...`), which REQUIRES
   `GGML_ASSERT(b.size != VK_WHOLE_SIZE && d.size != VK_WHOLE_SIZE)` — you
   cannot offset a WHOLE_SIZE subbuffer — and the col-stride must know the
   buffer's KIND (a raw src1 tensor keeps its own `nb[1]`; a staging buffer
   is tightly packed — two different `b_col_bytes` formulas). And every
   derived bound shifts with the chunk: the push constants carry
   `padded_n - n_base`, never the global `padded_n`.

## Tasks

- [x] T1 Dedup re-grep at authoring time (zero staging-side-mirror hits; the
      nearest hits are inverse/adjacent faces — TMA HW OOB fill, L1 rewind,
      offset-gated elision — none owns the symmetry law).
- [x] T2 Rider text on `ragged-chunk-pad`'s `gpu_concept` (the pad/tail family
      gains the staging-side member + the symmetry law + the subbuffer-window
      half).
- [x] T3 Needle test `batch229_rider_landed_on_ragged_chunk_pad_host` in
      `corpus/tests.rs` (13 verbatim needles; 1/1 green).
- [x] T4 Gates: kernel_opt lib 2982/0/1i; clippy ×4 postures clean; retrieval
      floors HELD (kernel 100.0%, blended 91.2/94.7/95.3+ — 21/21 gate pass,
      no re-pin).
- [x] T5 Docs: AGENTS.md Batch-229 rider note (803 stands) + the stale line-127
      count 795→803 repaired; `.docs/11_domains/kernel_opt.md` B229 row;
      Research 260 born-complete; the snapshot landing row.
- [x] T6 Commit `feat: batch 229: ...` + push; plan ticks in the same commit.
