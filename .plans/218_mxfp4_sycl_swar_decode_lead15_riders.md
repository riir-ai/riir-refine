# Plan 218 — Batch 218: Lead 15 (llama.cpp #29809 MXFP4 SYCL) as two count-neutral kernel_opt riders

**Status:** LANDED 2026-10-10 at riir-refine `49bfc500` (gates ALL GREEN — full suite 3109/0, default 2488/0 == pin, clippy -D ×2 clean, retrieval floors held, count pins unmoved)

## Source (the staging row, now FULLY read)

llama.cpp `d890cc8959019be0e3527638f66a5a46118bb8b9` — PR #29809 "sycl: accelerate
MXFP4 MoE inference with arithmetic decoding and weight reordering" (MIT; Jon
Staples / Captain-Tripps; Assisted-by Claude Opus 5.5). The Batch-217 session
staged this lead from the PR BODY only; the full 427-line single-commit .patch
was fetched this session (`/tmp/b218/lead15.patch`) and every quote below is
verified verbatim at it — the owed verification closes here.

- Patch shape: 6 files, +263/−4 — convert.cpp (reorder dispatch),
  dequantize.hpp (reordered-layout decode), ggml-sycl.cpp (the chunked
  reorder + alignment admission), mmvq.cpp (reorder-aware GEMV, ncols 1..8),
  quants.hpp (the `block_q_t<MXFP4>` [qs][e] layout traits), vecdotq.hpp
  (the SWAR arithmetic decode + the reorder vec-dot).
- PR-body numbers: gpt-oss-20b 55.84→106.64 tok/s (1.91×) decode, 120b 2.02×
  on two Arc Pro B70 cards, ppl 14.5148→14.5129 within noise — source-reported
  (Arc B70 hardware this workspace does not own; NotBenchable, the B136 class).

## The two riders (count-neutral — 783 stays, D stays 179)

1. **Rider 1 → `minifloat-normal-code-shift-bias-diff-decode`**
   (`entries_gpu_77.rs`, the B207 decode-mechanism family host): the family's
   FOURTH backend face — SWAR arithmetic replacing the TABLE gather exactly
   where the backend EMULATES the byte-permute primitive the gather compiles
   to. The admission predicate is this rule's `conversion_is_emulated` shape
   one level down: on SYCL/Intel, `dpct::byte_level_permute` is emulated with
   64-bit shifts ("eight per int, which made the MXFP4 GEMV compute-bound on
   Intel GPUs"), so the table decode that is free on hardware-permute backends
   becomes compute-bound. The decode law, verbatim: "Magnitudes
   0,1,2,3,4,6,8,12 = m + [m>=5] + [m>=6] + 3*[m>=7]; each byte stays below
   256, so the byte-wise adds never carry. -0 (code 8) is left as 0 so the
   two's-complement +1 cannot carry either." Bit-identical by construction
   ("Replaces table-based MXFP4 decoding with bit-identical 32-bit arithmetic,
   avoiding costly permute emulation").

2. **Rider 2 → `widening-conversion-row-chunk-staging-budget`**
   (`entries_gpu_76.rs`, the B206 staging-budget host — same source repo,
   same D dispatch family): the LAZY REORDER member — a pack-time layout
   transform ([qs][e] split: "The 17-byte AoS block leaves qs unaligned;
   split out, every 16-byte nibble block is aligned") staged per-expert
   against a 32-MiB cap, with the load-bearing NEW hazard: allocation
   success ≠ fit — "a whole-tensor temporary (hundreds of MB) can exceed the
   VRAM left on a nearly full card, and on Windows the driver then pages
   device memory out to host RAM instead of failing" (oversubscription
   silently pages, a worse failure than refusing). Plus: degrade-not-fail
   (alloc fail → warn + skip reorder, generic path stays), the alignment
   admission + fall-through ("The MXFP4 reorder kernels use 8-byte vector
   loads, so every expert slice must stay aligned."), the budget-derived
   chunk with floor 1 (`chunk_experts = max(1, min(n_expert,
   max_chunk_bytes / expert_bytes))` — the B206 `rows_per_chunk` mirror),
   per-expert self-containment as the chunking license ("Experts are
   self-contained, so the tensor is reordered a few experts at a time"),
   in-order-queue chunk pipelining ("the queue is in-order: each chunk's
   copy into tmp_buf waits for the previous chunk's kernel"), and the lazy
   trigger's prefill face ("reorder MXFP4 here as well, so prompt processing
   does not depend on a single-token decode having run first").

## Dedup (re-grepped at composition)

- B212 `lut-free-bit-spread-vadd4-unpack` — owns the CPU LUT/ALU DENSITY face
  (anti-LUT verdict is `__constant__`-divergent-lane-specific); THIS rider is
  the GPU BACKEND-EMULATION admission (the permute primitive emulated ⇒
  arithmetic), a different axis, cited not merged.
- B194 gather-LUT member + the family's field-arithmetic (Batch-199 rider) —
  cited as members one/two; bit-placement (B207) member three; THIS is member
  four (backend-arithmetic-over-emulated-permute).
- `swar-lane-widened-base3-dp4a-dot` (B116) — SWAR for MULTI-DIGIT base3
  lane-widening in a dot product, not a code→VALUE decode; distinct axis
  (digit packing vs decode arithmetic), named Distinct-from.
- B206 host itself — widening staging for a library GEMM, budget-derived ROW
  count, one-shot; THIS is a pack-time LAYOUT transform, budget-derived
  EXPERT chunk, in-place writeback, lazy-triggered, with the driver-paging
  hazard and degrade-not-fail. Compose, never merge.
- `a14c7de9` SYCL XMX PQ2_0 — watch-only in Research 239/247 (DPAS, different
  ISA class); the snapshot's "joins the a14c7de XMX pre-arranged family"
  names a RESEARCH pool, not a landed rule — no corpus host exists there, so
  the reorder atom hosts on B206 (the staging-budget shape), not on XMX.

## Tasks

- [x] T0 pre-flight: dual_allocation gate clean (0/0/0), `.plans/.highwater`
      217 re-read, partition check (riir-refine clean at `e80f0008`, no
      sibling mid-flight; dapps Issue 122 untouched — sibling lane)
- [x] T1 full-patch verification: `/tmp/b218/lead15.patch` fetched (427
      lines, single commit `d890cc895`), every rider quote verbatim at it
- [x] T2 dedup re-grep at composition (minifloat family, swar-base3, B212
      LUT, B206/B176, a14c7de watch-only adjudicated → no host there)
- [x] T3 Rider 1 landed on `minifloat-normal-code-shift-bias-diff-decode`
      (entries_gpu_77.rs, RIDER block appended to gpu_concept)
- [x] T4 Rider 2 landed on `widening-conversion-row-chunk-staging-budget`
      (entries_gpu_76.rs, RIDER block appended to gpu_concept)
- [x] T5 gate `batch218_riders_landed_on_their_hosts` (needle assertions on
      both hosts, the batch217 pattern) — riders are concept-only, no new
      pseudocode, no parse gate owed — FIRST SHOT GREEN
- [x] T6 GOAT: kernel_opt+latent_retrieval lib FULL run 3109/0 (3 ignored =
      the documented freeze-regen e2es; 1190.43 s, /tmp/b218tgt, nice-5) +
      default lib 2488/0 == standing pin + clippy `-D` ×2 postures clean
      (kernel_opt,latent_retrieval lib; default all-targets) + retrieval
      floors held inside the suite
- [x] T7 count pins UNMOVED asserted in-suite (riders concept-only; no pin
      file touched; freeze/score-bench NOT owed — zero clippy-corpus delta,
      P30)
- [x] T8 snapshot row (Batch 218, queue-after line) + commit + push

## Non-goals

- No AGENTS.md / `.docs/11_domains` edits (count-neutral riders — the
  Batch-217 precedent), no corpus-count bumps, no rust_perf/kernel rule adds.
- Lead 13 (MoE weighted-reduce fusion) stays NEXT in the queue — one unit per
  cycle; this plan converts Lead 15 only.
