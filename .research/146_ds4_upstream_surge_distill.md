# Research 146 — Batch 123: the ds4 upstream-surge distill (GLM/Metal SSD-expert caching + tooling)

**Status:** RECORD — Batch 123 landed 2026-09-07 (M3 box, CPU-only; one sibling `cargo test -p riir-chain` lane light on CPU, not a timing gate).

## Source + provenance

antirez/ds4 (MIT) main `9ab705347c1775e7599ede7eb81a6255ec7dccb5` → `c0a6119f363ef82125877142f13fb3fe491cba14` — 48 commits, 41 files, +6118/−1100. Part B of the Plan-102 double trigger (the PR-vision half landed as Batch 122). Clone `.raw/ds4` staged + pin re-verified via ls-remote (07:08 +07); quotes re-verified at the pin by both mining subagents AND at entry-write time (the Batch-31 discipline); `rm -rf` at landing.

## Lanes + selection (9 proposed → 5 landed)

Two parallel subagents (Protocol A step 3) read the lanes and proposed 9 candidates; all key quotes re-verified by the coordinator before authoring.

**kernel_opt +3 (corpus 552 → 555; A 116→117, H 24→26):**

1. **`batch-pool-read-makes-serial-readahead-redundant`** (d6580a5, the lane GOAT) — an advisory read-ahead hint for ranges whose next consumer is a queued parallel read pool is duplication, not prefetch: the hint's I/O serializes ahead of the pool's own parallel reads. Dispatch boundary: "is there a queued parallel reader of these identical bytes behind this point?" — not "is read-ahead useful?". Hints stay on pool-less sibling paths (decode-side single loads). M5 Max: prefill 73/77 → 93/97 tok/s (~+27%), frontier logits byte-identical.
2. **`cold-large-prefill-whole-layer-lookahead-over-partial-cache`** (6ccc1e7 + 437dbcb) — read-pattern dispatch keyed on (position OR generic-layout ∧ chunk-size ≥256 ∧ cache temperature), with exactly ONE next-layer read overlapping current-layer compute. The named COMPLEMENT of `row-addressable-operand-index-not-full-tensor-map`: the regime where "the rows you need" ≈ all rows and index-granular random access loses to sequential. 8K/4K append 63/62 → 91/84 tok/s; 512-token 14.6 → 26.0; 154,880 logits byte-identical.
3. **`static-seed-priority-bounded-to-observed-scale`** (91dda0b) — static preload lists seeding a shared eviction-priority namespace must enter at magnitude comparable to the observation counters (band 1..=31), else no evidence can ever evict a seed — a permanent cold-start dictatorship. The MAGNITUDE contract, complementary to `decayed-frequency-eviction-recency-tiebreak-route-protection`'s FIELD-separation hazard. Fixed-code SSD reads 94.65 → 78.18 GiB (−17.4%).

**rust_perf io +2 (corpus 108 → 110; both NotBenchable):**

4. **`full-vector-bit-identity-over-argmax`** (85a4f0e/5ad935d, the lane-C lead) — a MEASURED failure class: the source's tooling detected full-vector drift at 4/4 frontiers where argmax IDs matched, and the old comparator silently accepted missing/duplicate/NaN rows. Contract: full float32 bit identity (signed zero preserved) + artifact pre-validation (manifest coverage, canonical spelling, internal ratio consistency) + three statuses (identical/drift/invalid). The comparison-contract twin of our own bit-identity gates.
5. **`renderer-error-is-not-capacity-overflow`** (e1711c2) — a bool try-fit validator folding render errors into "doesn't fit" routed every failure to the destructive compaction path; the fix is a tri-state (Error/Fits/Overflow) with the cause propagated and only a true capacity overflow allowed to compact. Source comment pinned: *"-1 is a rendering/embedding error, not a request to compact the context."*

## Considered, NOT landed (honesty ledger)

- `vendor-blas-tuned-algo-latched-by-version` (1ef9bba/e83b106) — strong harness (version+revision identity gate, tri-state latch, support probe, total fallback; 262.64→301.30 tok/s warm 16K) but the decision content is hipBLASLt-build-specific constants; a future batch can pair it with cuBLASLt/CubeCL analogues.
- `k-loop-tail-mask-seed-once-prefix-reload` (64e490b + 49ec955's no-unroll-on-runtime-trip-count corollary) — real smem-staging shape; K-loop-interior; follows if a second MMQ pass mines more.
- `memory-stream-terminator-needs-own-byte` (c0a6119, the tip) — fmemopen NUL clobbering the last snapshot byte; C/FFI-specific (Rust writer APIs make the shape rare); excellent test-rider though ("every byte of the final float is nonzero").
- `small-append-cache-resident-expert-dispatch` (d7ade9f) — the ladder's cached arm; close kin of landed rules 1–2, cited as related; lands if a follow-up pass needs it.
- Dropped by the subagents with reasons: d077fa6, 759dd7c, 6eac69e, 930ab73, 2803820/45f54e9, 394865c, 4814f44 (API trap), fb2abb2 (alignment trap), e965352 (feature guard), 715ed7a (repairs rule 3's decay machinery), edcafe4 (13-line handoff, thin), 660e1d4 (resource-lifetime, plausible later).

## Gates (all pass; parse-gate repair = the B122 bare-statement lesson re-applied)

- rust_perf `batch123_rules_present/pseudocode_parses/concepts_covered` 3/3 (the parse gate caught the caller-side top-level `if`/`match` — fn-wrapped; second consecutive batch where the gate earned its keep).
- kernel_opt: entries registered in `KERNEL_CATEGORIES` (A +1, H +2) — the exhaustive-coverage gate verifies category membership, count pins walked: 552→**555** (tests ×3 sites, router ×1, mod doc comment), A 116→117, H 24→26.
- Lib default **1165/0**; kernel_heal **1478/0/1i**; kernel_heal+latent **1595/0/2i**; retrieval re-run **kernel 100/100/100, blended 94.7/96.9/97.7** (byte-identical to B118/119/121/122); score-bench **FLAT 96.2/1.9**; clippy 0 (default, kernel_heal, rust_perf).

## Walk #8 riders (same session)

- arXiv:2609.02027 (multi-turn LRU hit-ratio) — NOTE, no batch; recorded as the analytic comparator in riir-ai Issue 882's Related (`1a4244c1e`).
- arXiv:2609.04040 (confidence-gated admission) — **QUEUED as Batch 124** (rust_perf evaluation-design: matched controls disentangle predictor from admission gate; proxy-vs-endpoint metric split; gate-closed null-arm identity).
- arXiv:2608.30226 (LaMoC) — SKIP (Fisher-blended training-time compression; riir-train-adjacent, routing-restricted).
