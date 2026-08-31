# Plan 054 — Batch 93: the ktransformers residue disposition (zero-corpus-delta batch)

**Status:** DONE (2026-08-31)
**Batch:** 93 — Protocol A closure batch for Research 107's remaining candidates
**Predecessor:** Batch 92 / Plan 053 (`66f1c8d`)

## Why a zero-yield batch

Research 107 queued 14 kernel_opt candidates + 8 rust_perf/host candidates.
Candidates 1-12 landed as Batch 92. The snapshot's "REMAINING QUEUED (13-14)"
line drove this batch — and the mine-time verification COLLAPSED it:

- Candidate 14's content was already inside Batch 92's
  `quant-group-scale-fetch-gating` entry (the cu:464-472 stripe-alignment
  precondition is quoted verbatim in the landed gpu_concept + the source
  comment block). The snapshot line was stale the day it was written.
- Candidate 13 has NO source anchor (grep-verified below). The R107 note
  graded the host/CPU lane C+ with "mostly overlap instincts our corpus
  already encodes" — the honest verdict is a disposition record, not
  forced entries. The distill discipline ("a rule without probe evidence
  does not land", Batch 33's rejected-seed precedent) applies with full
  force when the candidate is a one-liner from an ISA-specific lane with
  no measured numbers anywhere in the source.

A zero-corpus-delta batch is a landed precedent (Batch 50: the ZERO-RULE
file-mining batch — value = the hazard yield + the disposition record).

## Tasks

- [x] T1 Source re-verification (M3 box, clone at `.raw/ktransformers` @
      21095b9): (a) candidate 13 — grep `L2|l2` across
      `archive/csrc/custom_marlin/` + `kt-kernel/`: only the Marlin
      L2-resident serial reduction (cu:1077-1080 — ALREADY LANDED as B92
      `stripe-sms-l2-semaphore-reduce`) + an ISA-specific output-tiling
      comment in `kt-kernel/operators/amx/test/test_lora_fused_add.cpp:2850`
      (test code, O_TILE=256, not K-block, not ½-L2). **The ½-L2 formula
      does not exist in the repo.** (b) candidate 14 — `gptq_marlin.cu:465-471`
      verbatim anchor CONFIRMED ("Ensure that the number of tiles in each
      stripe is a multiple of the groupsize; this avoids an annoying
      special case where a stripe starts in the middle of group") and
      CONFIRMED ALREADY LANDED inside `quant-group-scale-fetch-gating`
      (entries_gpu_09.rs L121-123 + the gpu_concept sentence).
- [x] T2 Host-candidate site verification (kill reasons grounded, not
      note-transcribed): `shared_mem_buffer.h` (MemoryRequest {funcs,
      sizes} + append_pointer rebind — the B32a split-scratch/arena
      family); `task_queue.h:29` + `task_queue.cpp:53-61` (`sync(size_t
      allow_n_pending)` — real anchor, killed on the bar, not on absence);
      `hasher.hpp:9-10` + `cache_entry.hh:63` (`hash_seed`/`check_hash_seed`
      /`hash_check` — the dual-seed design is DEAD SCAFFOLDING: the check
      field/seed are declared and never consumed anywhere in the tree);
      `cache_entry.cpp:167` (the evict-count check is commented out).
- [x] T3 Dedup grep (both corpora) per candidate — verdicts in the table
      below; external-validation cites recorded on B77 (intensity-gated
      twins) and the B58/B63 same-input family (fuse-independent-gemms).
- [x] T4 Corpus delta: **NONE** (0 entries, 0 riders, 0 count-pin moves).
      Standing gates untouched; retrieval floors untouched by
      construction (no corpus edit).
- [x] T5 Snapshot updated: the ktransformers intake section marked
      DISPOSITIONED (lane CLOSED); the stale "REMAINING QUEUED (13-14)"
      line corrected; the 8 rust_perf/host candidates dispositioned with
      reasons. Also recorded: the snapshot file is ~1980 lines vs its own
      ~400-line budget — the history-fold (into a `.research/` note) is
      the next hygiene unit.
- [x] T6 `.plans/.highwater` repaired: said 52 while plan 053 existed
      (the previous session's missed bump — the recorded collision class,
      now at its 8th+ occurrence); bumped to 54 with this plan.
- [x] T7 AGENTS.md Batch 93 entry (zero-yield disposition record,
      B50 precedent) + R107 status line → DISPOSITIONED.
- [x] T8 Commit + push.

## The disposition table (the batch's product)

| Candidate | Verdict | Grounded reason |
|---|---|---|
| 13 `k-block-half-of-l2` | KILL | No anchor — the ½-L2 formula is absent from the repo (T1a); the only L2 content already landed as B92 `stripe-sms-l2-semaphore-reduce` |
| 14 `k-block-aligned-to-quant-group` | COVERED | Landed in B92: `quant-group-scale-fetch-gating` carries the cu:464-472 stripe-alignment precondition verbatim (T1b) |
| `grow-only-arena-pointer-rebind` | KILL | Verified site = the B32a caller-owned-split-scratch + arena-reuse family; the C++ `std::function`-rebind idiom has no Rust gap (Rust carves with `split_at_mut`) |
| `intensity-gated-mat-vec-twins` | KILL | Covered by B77 `batch-width-routed-dual-tile-twins` — same decision content (batch-size-routed kernel selection); cite as external validation |
| `fuse-independent-gemms-one-task-space` | KILL | The same-input family already has 3 members (B58 launch-merge / B63 weight-concat / B77 quantize-memo); gate+up concat is B63's exact shape |
| `allow-n-pending-sync` | KILL | Real anchor (`task_queue.h:29`, `cpp:53-61`) but killed on the BAR, not absence: parameter-not-rewrite (no bounded mechanical shape), the win is workload-dependent (the metric-mismatch class B78/B65 documented), zero measured evidence — every landed concurrency rule carried a measured verdict; this would land below that bar |
| `host-callback-on-gpu-stream` | ROUTE | Serving-arch intel (R107 already records it for Plan 548: "the only pattern found for capturing host-completion into a replayable graph") — not a corpus rule |
| `dual-seed-page-hash` | KILL | Dead scaffolding in source (T2: `hash_check` + `check_hash_seed` declared, never consumed); our prefix caches are BLAKE3-committed — 64-bit collision detection via a second hash is moot at 256-bit commitments |
| `batched-evict-count` | KILL | Commented-out config check in source (`cache_entry.cpp:167`); a positive instance of the count-fused-into-mutation family (B30) at best |
| `recompute-ratio-guard` | KILL | No anchor found; one-liner, no isolable decision content |
| `decode-prefill-kernel-split` | KILL | Superseded by landed rules: B92 device-M makes the kernel mode-FREE (the better half of the idea); B77's twins own mode-routing where a split is kept |

## Honest caveats

- The kill verdicts rest on dedup + bar arguments, not on measurements —
  the same epistemic state as every dedup kill since Batch 30. The
  difference here: for the first time the ENTIRE residue of a source is
  dispositioned in one batch, so the lane closes instead of staying
  "queued" indefinitely (stale queue lines are how 14 dual-landed).
- If a future measured-GOAT from our own kernels needs a K-block sizing
  rule, it should be distilled from THAT measurement (the B49 path), not
  from this folklore one-liner.

## Status

DONE — T1-T8 all `[x]`. Commit: see git log (feat: batch 93 disposition).
