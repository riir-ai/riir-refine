# Research 097 — Direct-Operable SIMD Bit-Slicing (arXiv:2608.26368) distill verdict

**Status:** DISTILLED — pending owner/batch execution (queued as Plan 044 / Batch 83)

## Source

"Direct-Operable SIMD Bit-Slicing: A Framework for Memory-Efficient Predicate
Evaluation" — Arunkumar Mathiyazhagan, arXiv:2608.26368 [cs.PF], submitted
2026-08-26, 25 pages. Read in full via the queue-11 arXiv listing walk (the
pooled 2608.26368 entry from the Batch 82 session). Found via `fetch` on the
cs.PF recent listing (MCP search quota exhausted until 2026-09-15; native
fetch is the standing fallback).

**Provenance posture:** single-author independent work (Amazon affiliation
disclosed, work declared independent); arXiv-only, NOT peer-reviewed; all
numbers are the author's JMH measurements on Apple M-series (NEON 128-bit) +
one Xeon 8375C (AVX-512) cross-check. Java 21 + Project Panama Vector API +
off-heap MemorySegment. Grade: B (mechanisms transfer; the JVM-specific
halves do not; honest in-paper negatives present and recorded below).

## What the paper does

Row-oriented integer columns are transposed into `b` bit-planes
(`P_j[i] = (x_i >> j) & 1`), one contiguous bit-vector per bit position,
with `b` chosen per block (N=1024) from the block max
(`b = 32 - clz(max)`). Predicates evaluate DIRECTLY on the compressed form
("zero-decompression"):

- **Equality** = one AND (or AND-NOT) per plane: `res &= cBit ? plane : ~plane`.
- **Greater-than** = MSB→LSB two-mask reduction: `gt |= eq & plane` when
  `cBit == 0`; `eq &= plane` when `cBit == 1`. Branchless per plane.
- **BETWEEN** = `¬gt(hi) ∧ gt(lo-1)` (2 passes; `lo == 0` skips one pass).
- **IN-list** = OR of k equality masks (k×b plane passes — scales linearly).
- Compound predicates = the masks AND/OR-compose in one pass.
- Signed → per-block bias (`x - Vmin`); doubles → IEEE 754
  order-preserving transform (`bits ^ ((bits>>63) | 1<<63)`); strings →
  dictionary codes.
- NULL → separate validity plane, one extra AND per block.

Measured (ARM NEON 128): predicate throughput 3.4–10.7× over scalar `int[]`
(b=26 → b=6); memory 1.2–5.3× over `int[]`; TPCDS-modeled 50M-row scans
2.4–10.8×; dictionary-code string equality 28–43×; double BETWEEN 9.4×;
timestamp range 1.5–2.1×. Cross-arch: AVX-512 relative speedups within 20%
of NEON (8.5× vs 10.7× on the same benchmark).

## Verdict: YES — B, queued as the next `rust_perf` batch (Plan 044 / Batch 83)

The substrate transfers cleanly to Rust: bit-plane words are `u64` and the
predicate kernels are pure AND/OR/NOT word loops that LLVM auto-vectorizes
(the paper's NEON 128-bit result is directly relevant to our M3 host; AVX-512
only widens). The JVM-specific value propositions (GC pressure, ArrayList
boxing, off-heap accounting) do NOT transfer and are excluded from any rule.

### Keep-candidates

1. **`bitplane-transpose-repeated-predicate-scan`** (data-structure or simd;
   B+ GOAT lead) — for integer columns filtered REPEATEDLY (queries ≫ 1 over
   the same encoded data), store the column bit-plane transposed at b = bits
   of the block max and evaluate filters as bitwise plane reductions (eq =
   AND/AND-NOT per plane; gt = the two-mask MSB→LSB recurrence; BETWEEN =
   compose). Mechanism: (a) bytes traversed shrink 32/b (b=6 → 5.3×), which
   converts an L2/DRAM-bound scalar scan into an L1-resident word loop;
   (b) each word op covers 64 values (lane parallelism on top). Regime gates
   (all in-rule, from the paper's own negatives):
   - **Amortization**: encoding is O(N·b) once; break-even measured at
     6.7–25.4 scans (b=6 → b=26). Write-once/read-once (ETL, log forwarding)
     = LOSS — the encoding cost is never recovered.
   - **Bit-width**: win scales inversely with b; b≈32 → 1× compression, no
     win. Paper guideline: b ≤ 16.
   - **Residency crossover**: at 1M rows Q1/Q2 measured 0.5–0.6× (scalar
     wins while BOTH arms fit L1/L2 — branch prediction + OoO beat the
     transposed form on cache-resident data). The win needs working set
     > cache OR very small b. Our bench MUST size cells past the L2 boundary
     (the B75 sizing lesson applies in reverse: top cell large enough for
     the regime to exist).
   - **Point lookups**: bit-plane layout is scan-shaped; random access pays
     gather-from-b-planes. Out of regime.
   - **IN-list**: k·b passes — for k ≳ 8 the hash-on-materialized path wins;
     document the ceiling.
   - **Wide-type equality loses**: timestamp `==` measured 0.5–0.7× (scalar
     `==` is one instruction vs b plane ANDs). Range/comparison predicates
     are the win; equality only pays when b is tiny (dictionary codes).
   - Bench shape: encode once outside the timed loop (ingest cost), then
     time N scans of a gt/eq/BETWEEN predicate: scalar `v > c` count/mask
     vs bit-plane mask reduction. G1: mask bit-identity vs the scalar
     reference (exact, no tolerance — integer ops). Expected verdict
     ParetoDominant at b≤16 past-cache cells with the amortization axis
     (scans-per-encode) visible; annotate the cache-resident + high-b cells
     honestly.
   - Differentiation to pin IN-rule: vs `soa-hot-field` (field-level
     columnar layout for strided reads — no compression, no predicate
     algebra; bit-plane is the per-BIT extreme with a different consumer
     shape); vs `simd-chunk` (the word loops are trivially chunked — that is
     the vehicle, not the mechanism); vs `bool-to-bitpack` (flag DENSITY for
     counting, not value predicates); vs `sparse-set-bit-iteration-tz-blsr`
     (iteration WITHIN a word, not layout across values).

2. **`float-order-preserving-bits`** (arithmetic; C+/B− optional second) —
   the IEEE 754 order-preserving transform
   (`bits ^ ((bits >> 63) | 1<<63)`; i.e. flip-all for negatives, set sign
   bit for positives) maps the f64 line to u64 with identical order,
   unlocking integer-keyed machinery: radix sort, bit-plane filters (the
   paper's §5.10.2 double BETWEEN = 9.4× rides exactly this transform),
   integer bucketing. Rust note: `f64::total_cmp` implements the same
   transform internally — the rule is about feeding INTEGER consumers
   (radix/bitmasks), not about comparator choice. Bench: comparison sort of
   f64 vs LSD radix sort over transformed bits. Honest risk: the win belongs
   mostly to radix-sort-vs-comparison-sort (a ~20-line bounded but
   non-trivial fix shape); if the isolated-transform bench cannot separate
   the transform's own contribution, drop to a context rider on the lead
   rule (the paper's double lane needs it) and record why.

3. **Rider: `branchless-conditional` gains the selectivity-constant context**
   (doc-comment rider, the B72 precedent) — the paper's scalar baseline
   swung 5.4K → 1.2K ops/s across selectivities (0.1% → 90%, a ~4×
   branch-prediction swing) while the branchless bit-plane form stayed
   constant (16.2–17.1K, <6%). That is external measured evidence for the
   existing rule's regime (data-dependent branch throughput instability at
   medium selectivity) — one sentence + the citation in the rule's doc, no
   corpus-count change beyond nothing.

### Drops (with reasons)

- **String/dictionary lane (28–43×)** — Java-baseline-inflated: the scalar
  arm is per-element `String.equals()` (pointer deref + char compare). Rust
  `str == str` is already vectorized memcmp; the interning half is the
  dropped-as-architectural string-interning class (B61 oxc verdict). The
  general "intern to integer codes" idea survives only as context inside
  the lead rule's dictionary-code mention.
- **JVM memory-economics section (§6.2), off-heap MemorySegment, GC
  results** — JVM-specific; no Rust decision content.
- **`lo == 0` skip pass** — Rust clippy already flags unsigned `>= 0`
  comparisons (`absurd_extreme_comparisons`); not a corpus rule.
- **Per-block bias/FOR encoding as a standalone rule** — real technique
  (timestamps b≈17 after bias vs 41 raw) but the bounded fix shape in Rust
  collapses into "narrow fields" (`usize-index-to-u32` /
  `narrow-counter-overflow-promotion` territory); keep as a regime sentence
  inside the lead rule (adaptive per-block b is part of its encoding spec).
- **Multi-column composition cost model (§6.7)** — arithmetic derivation,
  correct but ambient (masks AND-compose; no decision content beyond the
  lead rule's own design).

## Cross-arch + host notes

- NEON 128-bit (our M3) is the paper's PRIMARY platform; AVX-512 relative
  speedups within 20% — the bench verdicts should port.
- The paper's own caveat: JDK C2 may avoid AVX-512 EVEX (frequency
  throttling) — irrelevant to Rust but a reminder that wide-register gains
  are host-dependent; our verdict is aarch64-scoped with the same
  cross-arch re-gate discipline as B77's `runtime-int-exponent-powi`.

## Queue state after this note

Plan 044 (Batch 83) holds the batch tasks: corpus entries (lead + optional
float-transform + branchless rider), bench specs, G1 bit-identity gates,
count pins, GOAT (G1/G2/G3 + retrieval floors + verdict-stable), docs +
AGENTS + snapshot, commit+push. GOAT benches need a quiet box (load was
~8.3 with sibling builds at verdict time — the Plan 337 discipline applies;
the verdict/plan landed first as the zero-contention unit).
