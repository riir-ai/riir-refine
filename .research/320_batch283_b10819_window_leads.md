# Research 320 — Batch 283: the prism b10819 window leads (look-ahead fusion + the certifying-uid host path)

**Status:** RECORD — landed with Batch 283 (Plan 287); corpus kernel_opt 969→970, rust_perf 270→271, +1 count-neutral rider.

## Source

PrismML llama.cpp fork, scored tag `prism-b10819-909f0ca` (`909f0cac9c`), the window's two
sm_89-relevant commits (both measured league-movers in riir-ai Bench 1000's opp-side +2.11%
decode move on the 4090):

- **#391 `e75baeeb24`** — "cuda: fuse RMS_NORM (+ residual ADD) with the signed Hadamard
  transform at both norms of every layer, bit-identical, +4.8-5.4% decode on H200".
- **#393 `63f16c408f`** — "ggml-backend, cuda : cut ~55-65 us of host time per decode step
  (batched sched input uploads, no node scans on CUDA graph replay; byte-identical)".

Read in full at the vendor clone (`E:/git/vendor/prismml-llama.cpp`, checked out AT the pin by
the Bout-1000 pre-stage — zero fetch). MIT lineage.

## What landed

### 1. `look-ahead-fusion-precomputed-skip-overlap-walk` (kernel_opt D, entries_gpu_128)

The graph-level LOOK-AHEAD FUSION: a fusion may compute a downstream node chain AHEAD of its
graph position and record the skipped nodes in a `precomputed` set the evaluation loop
consumes (`continue`), when:

- the early-consumed operands are graph LEAVES ("weight and signs are read now: they must be
  leaves (model tensors), not results of the nodes in between");
- unit use-counts on the producer norm and the consumer chain (the load-bearing one: the
  look-ahead launch never writes the norm's own output buffer — a second reader of the
  normalized row is only safe because the fusion writes it as a side output);
- THE MEMORY-OVERLAP WALK: the early-written output's byte range is checked against every
  intervening node's reads AND writes (a data-pointer overlap lambda), including each src of
  each intervening node — reads OF the fusion's own outputs are exempt;
- no stream concurrency active (`concurrent_events.empty()`).

Plus the INTER-FUSION PRIORITY YIELD: `ggml_cuda_hadamard_feeds_ptq1` scans an 8-node window
(through RESHAPE aliases) for a PTQ1_0 consumer; when found, the norm+transform fusion STANDS
ASIDE — the deeper transform+quant fusion (`ggml_cuda_try_fwht_q8`) wins the shared nodes.
Kill-switch `GGML_CUDA_NORM_FWHT_FUSION=0`; stands aside for `GGML_CUDA_FWHT_FUSION_PQ2`.
Bit-identity by arithmetic-order replay: the fused kernel executes the incumbent chain's
operations in the same order (the layout invariant `thread tid holds elements tid + i*1024`
for both the norm loop and the butterfly; the shared butterfly transcription — the B251
single-transcription law). Fallback composition: the #209 ADD+RMS_NORM+MUL fusion handles
what the wider shape declines.

Dedup: zero prior owner (grep `look.?ahead|precomputed|ahead of (its|their) position` — the
only hits are the MTPLX measurement-law mention and device-side submit-ahead rules).
Kinship: `rht-epilogue-fusion-contract` (WHERE a Hadamard fuses into a GEMM epilogue — this
owns the GRAPH-level bridging of non-adjacent nodes), `reduction-restructure-needs-source-
owned-comparator` (the oracle axis), `fuse-add-rms-norm` (entries_popcorn — the #209 fallback
this composes with, carrying the Batch-280 rider).

### 2. `revalidation-skipped-by-certifying-identity-token` (rust_perf caching, entries_49)

The certifying-token validation skip: a per-step re-validation over an unchanged input is
skippable when a token exists and matches, where the token is (1) written ONLY by the
validation's own success path, (2) guarded for ABSENCE (`uid != 0` — a never-stamped input
against a fresh slot reads 0 == 0 and would skip validating an input NO scan ever passed —
the trap the buggy fixture demonstrates), (3) minted by the BUILDER (monotonic per
re(derivation), so token equality implies the same instance), (4) backed by the full scan as
the sound fallback. The secondary content: the derived consumer map rebuilds only at the
update boundary ("only read while the nodes are evaluated or captured: not built when a
captured CUDA graph is replayed").

Honesty note in-rule: the 55-65 µs headline covers the WHOLE commit; the token site predates
the commit (this commit wired the token into the ADMISSION).

### 3. The count-neutral rider (chain-dispatches-single-sync-point += batched-async-upload)

#393's ggml-backend.cpp half: N blocking input copies → N async issues + ONE wait before the
split computes, with the LIFETIME LAW ("the host data of the uploads can be overwritten once
this function returns (inputs from the user) or once a later split computes (computed
inputs): wait for the copies, as the blocking copies did") and the refusal ladder (meta
backends, no async-upload iface, non-contiguous inputs, the events path, wrong buffer type).

## Gates (all green)

- Fixtures standalone-compiled + executed pre-landing: kernel ref 2/2 + opt 11/11; rust_perf
  buggy 2/2 + fixed 4/4 (rustc --edition 2021 --test -O).
- batch283 needle + parse gates first-run PASS (13 verbatim marks + the rider marks).
- kernel_opt lib 3232/0; default lib 2731/0; latent_retrieval lib 2917/0 (482 s).
- Retrieval floors HELD no re-pin (blended 124/141 ≥ 123, top-3 130 ≥ 130, top-5 133 ≥ 132).
- clippy −D ×3 postures clean (5 doc_markdown + 1 single-element-loop healed in-batch).
- Score-bench #200: heal 0.9636 / created 0.0182 / parse 1.0 FLAT (gen-1 SATURATED; frontier
  snapshot gen-7 appended).
- Maintained binary rebuilt at the release set + installed (`build: a560e65a…`, release set
  complete) — rust_perf is release-set surface, the §0 law.
- Count pins via shape-asserted python passes: tests.rs 25 multi-line + 10 single-line sites
  + the router expert-total 969→970 + the rust_perf 271/150/218 sites. EN-ROUTE REPAIR: the
  `batch249_slice3_rust_perf_riders` cross-domain pin (kernel_opt tests.rs watching the
  rust_perf corpus) was STALE at 267 — it missed Batch 282's +3 (the B266-missed-pin class
  its own message documents); repaired to 271 with the disclosure in-pin.
- En-route class note: a pin's VALUE line and its MESSAGE string are separate edit sites —
  the first pass updated messages and missed two value lines (annotated 217); the lib run
  caught both (left: 218, right: 217). The gate is the oracle.

## Queue after

walk #17 opens 2026-10-26T00:00Z (claimed by this lane, expiry 2026-10-27 ~04:3x +07;
close-out takes research ≥321); B277 = the sibling's live claim (Plan 281) — yield;
nnethercote ~11-01; the 971 watch re-arms on the next scored fork tag.

NOTHING INVENTED — every quote at the pin this session, every gate a run.
