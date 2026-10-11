# Research 060 — rust_perf Batch 54 pipeline: academic-paper survey for candidate rules

**Date:** 2026-08-17
**Status:** DISTILLED — pending owner decision (candidates staged only; no corpus/bench/proposer changes in this note — the batch landing is a separate unit gated on the bench window)
**Kind:** paper survey (queue item 12 lineage — the research-skill path feeding the rust_perf growth domain; Batch 48 was online perf-search, Batch 53 was web-search candidates, this is the ACADEMIC sweep those two missed)
**Target:** `rust_perf` corpus (58 rules today) — candidates for Batch 54+.

## 1. Method + honest headline

Swept: arxiv advanced search (`Rust performance optimization`, `performance bugs`, `Rust heap allocation`, `asynchronous Rust` — cs classification, all dates, abstracts shown) + web discovery queries (Rust perf-bug studies, memory-ordering overhead, quickselect/top-k/radix measurements). Fetched/verified the promising abstracts directly.

**Headline finding: the 2023-2026 academic literature does NOT contain a rich vein of measured Rust source-level performance idioms.** Recent Rust papers are overwhelmingly *correctness*-bug studies (rustc bugs, memory-safety sanitizers, fuzzers, LLM repair) or *system*-level work (GPU frameworks, databases, consensus). Measured idiom-level work that transfers to a bounded source-to-source rewrite exists mostly in (a) the classic algorithms-experimental literature (search layouts, selection, top-k, radix — 1990s-2010s, still the best measurements available), and (b) two genuinely recent on-point arxiv papers (Eytzinger-layout search is classic; meta-monomorphization 2602.12973 is 2026 and Rust-measured). Five candidates staged below — quality over volume, per the survey mandate.

Rule ids use the corpus kebab-case convention (matches `vec-with-capacity`, Batch 53's `powi-vs-powf`).

## 2. Sources

| Source | What it is | Verified how | Use here |
|---|---|---|---|
| arXiv:1509.05053 — Khuong & Morin, "Array Layouts for Comparison-Based Searching" (cs.DS; v2 2017, ACM JEA-reviewed) | Extensive multi-hardware experimental study of sorted/binary vs Eytzinger(BFS) vs implicit B-tree vs vEB layouts for repeated predecessor queries in RAM | Abstract fetched | Candidate 1 evidence |
| arXiv:2602.12973 — Bruzzone & Cazzola, "Meta-Monomorphizing Specializations" (cs.PL, Feb 2026, 36pp) | Zero-cost specialization via compile-time metaprogramming; surveys public Rust codebases for TypeId-dispatch workarounds; 16 Rust micro-benchmarks: compile-time specialization matches/outperforms runtime `TypeId` dispatch | Abstract fetched | Candidate 5 evidence |
| Musser, "Introspective Sorting and Selection" (Software: Practice & Experience, 1997) + Hoare's quickselect (1961) | Classic introselect selection-vs-sorting complexity + practice | Classic literature (not fetched) | Candidate 2 evidence |
| Thiemicke, "Implementation of a vectorized Quicksort using AVX-512" (DLR thesis, 2021, elib.dlr.de/145402) | Academic thesis embedding `select_nth_unstable` usage; compares comparison vs non-comparison (radix) classes | Search snippet (not fetched in full) | Candidates 2 + 4 corroboration |
| rustbites.com #75 "select_nth_unstable — Find the Kth Element Without Sorting" + hampuswessman.se order-statistics post (2022) | Rust-web measured/practitioner corroboration for selection vs sort | Search snippets | Candidate 2 corroboration |
| Fagin PODS'96 top-k aggregation; Ilyas et al. CSUR 2008 top-k survey; Frederickson 1993 heap selection | Classic top-k literature (don't materialize a total order for a k-subset) | Classic literature (not fetched) | Candidate 3 evidence |
| nilmamano.com "Top-K Problems: Sorting vs Heaps vs Quickselect" | Comparative practitioner writeup | Search snippet | Candidate 3 corroboration |
| radsort crate (crates.io) | Rust radix sort with benchmark claims vs std sort | Known crate (README not fetched — flag) | Candidate 4, weak web leg |
| arXiv:2607.20353 — MoST (LLM code-optimization framework; 50 Rust tasks) | Strategy-guided optimization from historical commits | Abstract fetched | REJECTED as source (see §5) |
| arXiv:2608.09530 — "The Unseen Delta" (ISSTA'26, GCC vs Clang optimization landscape) | Top-down differential compiler-performance analysis | Abstract fetched | REJECTED (see §5) |
| arXiv:2606.31238 — LLVM -O3 per-pass empirical study (84,750 measurements) | Marginal pass impact; "IR instruction count is an unreliable predictor of runtime" | Abstract fetched | §5 methodology note only |
| arXiv:2505.21323 — ROS 2 in async Rust (ECRTS'25) | Real-time structure for R2R apps | Abstract fetched | REJECTED (see §5) |
| Innsbruck thesis "Performance Impact of Memory Ordering in Concurrent C++" (ulb-dok.uibk.ac.at) | Measured ordering overheads | Search result only (not fetched) | §5 rejected candidate evidence |
| arXiv:2208.08918 — data-access performance anti-patterns (data-intensive systems) | N+1 / over-fetch anti-pattern catalog (data layer) | Abstract skim via search | §5 rejected (wrong layer) |

## 3. Staged candidates (5)

### 3.1 `select-nth-unstable` (complexity) — RECOMMENDED FIRST

- **Anti-pattern shape:**
```rust
fn median(v: &mut Vec<u32>) -> u32 {
    v.sort();
    v[v.len() / 2]
}
```
- **Bounded fix:** `v.select_nth_unstable(v.len() / 2); v[v.len() / 2]` — introselect, O(n) expected vs O(n log n). std-only, one call swap.
- **Evidence:** Musser 1997 introselect (classic); Thiemicke 2021 thesis (academic Rust-context usage); rustbites #75 + hampuswessman 2022 (Rust-web corroboration). No 2023-2026 arxiv study specific to this — honestly classic+web grounded, but the complexity gap is the largest of any candidate.
- **Dedup verdict:** closest existing = `sort-unstable` (chooses unstable vs stable *sort* — still fully sorts) and `sort-by-key` (key extraction). Neither covers *selection instead of sorting*. Distinct.
- **Guard (must carry into the proposer spec):** only valid when the post-sort code consumes just the k-th element (or a k-prefix via the `_by` variants); if any later code relies on total order, the rewrite is wrong. Detection: slice uses after the statement are `[k]`-indexed only.
- **Benchability:** **Benchable** — expect ParetoDominant at large N; watch small-N ties (LICM anti-hoist idiom per Research 054 §3 applies — the sort is pure compute).

### 3.2 `topk-bounded-heap` (complexity)

- **Anti-pattern shape:**
```rust
fn top_k(v: &[u32], k: usize) -> Vec<u32> {
    let mut s = v.to_vec();
    s.sort();
    s[s.len() - k..].to_vec()
}
```
- **Bounded fix:** min-`BinaryHeap` capped at k (push; if `len() > k` pop root) → O(n log k) + O(k) space; final drain gives the top-k. std-only.
- **Evidence:** classic top-k literature (Fagin PODS'96; Ilyas et al. CSUR 2008 survey — the "don't compute the total order for a k-subset" principle; Frederickson 1993 heap-selection optimality); nilmamano comparative writeup. Web+classic grounded.
- **Dedup verdict:** closest = `select-nth-unstable` (§3.1 — single k-th element) and `sort-*`. The k-element top-k with bounded memory is a distinct consumer shape (recommendation feeds, leaderboards). Distinct from both.
- **Benchability:** **Benchable, Regime-dependent** — wins when k ≪ n (log k ≪ log n); loses when k→n (heap constant factors + no SIMD partitioning). Expect a `context_notes` regime caveat like `dedupe-hashset-vs-vec`.

### 3.3 `bsearch-eytzinger-layout` (complexity/layout)

- **Anti-pattern shape:**
```rust
// many queries against one large resident sorted array
fn lookup_all(table: &[u64], queries: &[u64]) -> Vec<usize> {
    queries.iter().map(|q| table.binary_search(q).unwrap_or(N)).collect()
}
```
- **Bounded fix:** build the Eytzinger (BFS/heap-order) permutation once at construction; branchless search with `#[inline]` prefetch of the next two candidate nodes. Membership/predecessor semantics preserved; `Result` mapping reconstructed from the final probe node.
- **Evidence:** arXiv:1509.05053 (Khuong & Morin) — the canonical measured study; for large n Eytzinger + cmov + prefetch is fastest across modern hardware, beating sorted/binary, B-tree, and vEB layouts (explicitly overturning Brodal-Fagerberg-Jacob SODA'03). Date caveat: 2015/2017, outside the 2023-2026 focus — staged anyway because nothing newer supersedes it and it is *directly* on-point.
- **Dedup verdict:** closest = `bsearch-sorted-slice` (O(n) `.any()` → `binary_search` on a sorted slice). This is the *next rung*: repeated lookups on a large already-sorted array → layout change. Different anti-pattern shape (the buggy form is already the *fixed* form of `bsearch-sorted-slice`). Distinct.
- **Benchability:** **Benchable, Regime-dependent** — the paper itself says small n favors sorted order; expect the crossover to appear in the Pareto bench (context_notes likely). Harness needs an anti-LICM shape (same input re-queried).

### 3.4 `radix-sort-integers` (complexity/arithmetic)

- **Anti-pattern shape:** `keys.sort_unstable()` (or `sort_by_key`) on `u32`/`u64` keys at large N.
- **Bounded fix:** LSD radix sort — `radsort` crate, or ~40-line in-crate impl if the dep is unwanted. Order of equal keys: radsort is stable, so `sort` ↔ radix is semantics-preserving; `sort_unstable` → radix also fine (stability is a strengthening).
- **Evidence:** classic non-comparison-sort literature (Knuth Vol. 3 lineage); Thiemicke 2021 thesis (radix as the non-comparison baseline class); radsort crate benchmarks (web leg, README not re-fetched — the weakest leg of this candidate). No peer-reviewed Rust-specific radix-vs-std-sort measurement was verified in this sweep — honest gap.
- **Dedup verdict:** closest = `sort-unstable` / `sort-by-key` (both comparison-based family). Non-comparison integer sort is a distinct axis. Distinct.
- **Benchability:** **Benchable, Regime-dependent** — classic crossover at small N (radix passes + histogram overhead); wins 2-4× at large N on integer keys. External-crate precedent exists (`fxhashmap`→rustc-hash, `smallvec-stack`→smallvec).

### 3.5 `type-id-branch-specialization` (dispatch) — bench-first, flagged

- **Anti-pattern shape:**
```rust
use std::any::TypeId;
fn emit<T: 'static>(v: &T, out: &mut String) {
    if TypeId::of::<T>() == TypeId::of::<String>() { /* string fast path */ }
    else if TypeId::of::<T>() == TypeId::of::<u32>() { /* int path */ }
    else { /* generic fallback */ }
}
```
- **Bounded fix:** per-type trait impls (a sealed trait with an impl per special case, generic fn dispatches statically) — the manual-specialization workaround inverted into real static dispatch, enabling per-type inlining that the runtime branch blocks.
- **Evidence:** arXiv:2602.12973 (Bruzzone & Cazzola 2026) — measured on 16 Rust micro-benchmarks: compile-time specialization matches or outperforms runtime `TypeId`-based dispatch; and a comprehensive public-Rust-codebase study found "numerous workarounds" of this exact shape. This is the one 2026-paper-measured-in-Rust candidate.
- **Dedup verdict:** closest = `dyn-to-enum` (`Box<dyn Trait>` vtable → enum match). Same de-virtualization family, different detection shape: generic fn with a `TypeId::of` comparison chain (no trait object in sight). Distinct.
- **Benchability:** **Benchable** — but flagged: the fix introduces a trait + bound that ripples to call sites (precedent: `dyn-to-enum` also changes the parameter type; still, this is the least mechanical of the five). Recommend landing as corpus+bench first, proposer honestly deferred (the Issues 012/013/014/054 precedent), or benching before deciding to stage at all.

## 4. Dedup table (corpus collisions checked against all 58 + Batch 53 staged 4)

| Candidate | Closest existing/staged rule | Why distinct |
|---|---|---|
| `select-nth-unstable` | `sort-unstable` | selection vs sorting — never fully orders the slice |
| `topk-bounded-heap` | `select-nth-unstable` (this batch), `sort-by-key` | k-element subset + bounded memory vs single element / key extraction |
| `bsearch-eytzinger-layout` | `bsearch-sorted-slice` | layout change of the *already-optimal* form; targets repeated hot lookups at large n |
| `radix-sort-integers` | `sort-unstable` | non-comparison sort axis; integer-key-specific |
| `type-id-branch-specialization` | `dyn-to-enum` | generic-fn `TypeId` chain (no trait object) vs vtable receiver |
| Batch 53 overlap check | `powi-vs-powf`, `distance-sq-no-sqrt`, `hashmap-with-capacity`, `soa-hot-field` | zero overlap — none of the five touches pow/sqrt/map-capacity/SoA |

No collisions with Batch 48 (`collect-append-to-extend`, `vec-insert-front-deque`, `vec-macro-zero-fill`, `unwrap-or-else-lazy`, `hashset-insert-bool-return`, `filter-map-fuse`, `iter-copied-deref-map`, `filter-count-to-any`) either — all allocation/iterator-shaped, not selection/search/sort-axis/dispatch.

## 5. Rejected veins (with reasons — the negative results are the survey's other deliverable)

| Vein / source | Rejection reason |
|---|---|
| `atomic-ordering-downgrade` (SeqCst→Relaxed on plain counters; Innsbruck C++ thesis measured) | **Semantic hazard, not mechanically dischargeable**: weaker ordering changes allowed observable interleavings unless the counter is provably never used for synchronization — a proof a parse-level proposer cannot make. Also x86-64 reality: only SeqCst *stores* pay (locked exchange); RMWs are the same instruction — the win is narrow and platform-specific. Closest existing: `mutex-to-atomic`. Regime+platform-dependent → reject at the staging gate. |
| False-sharing padding (`#[repr(align(64))]`) | Requires knowing which fields are written by which threads (profiling/concurrency context) + fix changes a *shared* struct's layout — not locally provable from one span. |
| "The Unseen Delta" (ISSTA'26, GCC vs Clang perf deltas) | Compiler-level root causes (codegen sequences), not source idioms; the "fix" is a compiler patch or binary transplant. |
| MoST (arXiv:2607.20353) as a *rule source* | Meta-framework; its Rust strategies are summaries of historical optimization commits — expected to map onto rules the corpus already has (allocation hoisting, data-structure swaps). No per-strategy measured claims usable as rule evidence. |
| LLVM -O3 per-pass study (arXiv:2606.31238) | Methodology relevance only: "IR instruction count is an unreliable predictor of runtime" independently validates this repo's choice of *timing + alloc-count* (not proxy metrics) as the bench axes. Not a rule source. |
| ROS 2 async Rust (ECRTS'25) | Architecture-level (thread prioritization, callback-to-thread mapping). |
| Data-access anti-patterns (arXiv:2208.08918) | Data-layer patterns (N+1, over-fetch) — the Rust-side equivalents already shipped via Research 032 (lock-across-await, channel depth). |
| `small-map-linear-scan` (HashMap → Vec scan at small N) | Needs size knowledge at the call site (profiling/context); the regime boundary is already documented on `dedupe-hashset-vs-vec`. Would be a context_notes-annotated rule at best; not staged. |
| String interning / SVO beyond `smallvec-stack` | No measured paper found in this sweep (folklore/library docs only). |
| Async-runtime-overhead idioms | No academic source-level measurements found (arxiv async-Rust results are scheduling/session-types/security). Existing corpus async rules (`async-future-size`, `async-yield-point`, `tokio-guard-across-await`) came from Research 032's engineering source — nothing new to add from academia. |

## 6. Recommendation (bench order)

1. **`select-nth-unstable`** — std-only, one-call swap, largest complexity gap, cleanest proposer guard story. Bench first.
2. **`topk-bounded-heap`** — std-only; pairs naturally with 3.1 in the same bench window (shared harness shape); expect Regime-dependent annotation.
3. **`bsearch-eytzinger-layout`** — std-only but the layout build must live outside the timed query loop (construction amortization is the paper's setting); design the harness around repeated queries on one resident table.
4. **`radix-sort-integers`** — external dep + weakest evidence leg; land only if 1-3 leave bench-window capacity.
5. **`type-id-branch-specialization`** — bench-first decision: measure the paper's claim on our Pareto harness before committing to corpus+proposer scope; honest risk it lands as corpus-only-with-context-notes.

Combined with Batch 53's four staged candidates, a Batch 54 landing of the top 3-4 here would grow rust_perf 58 → ~65 with two genuinely academic-grounded axes (search layout, specialization) the web searches never surfaced.

## 7. What this does NOT cover (honesty section)

- **No 2023-2026 empirical "Rust performance idioms" paper appears to exist.** The field's Rust publications target correctness bugs, memory-safety tooling, and systems applications. This survey's academic yield therefore leans on the classic experimental-algorithms literature (1996-2017) plus two recent on-point papers. If a dedicated venue study exists (e.g., in an SE conference), it eluded both arxiv keyword search and web discovery with the six query families used.
- **Unverified recollection, deliberately not cited:** a paper on Rust heap-allocation predictability for real-time (crates.io-scale allocation-site study) was half-remembered and could not be confirmed by any search combination — treated as nonexistent rather than cited from memory. If it exists, it would ground the *existing* allocation rules (prior-art confirmation), not new ones.
- **Energy-efficiency literature skipped** (Pereira-lineage language rankings): language-vs-language, not idiom-level — nothing expressible as a rewrite.
- **GPU/kernel veins out of scope** — kernel_opt corpus is complete (Batch 51); nothing here mines GPU papers for kernel rules.
- **radsort benchmark numbers not re-fetched** — candidate 3.4's web leg rests on pre-survey knowledge of the crate; the bench window will produce the authoritative numbers anyway.
- Classic citations (Musser 1997, Fagin 1996, Ilyas 2008, Frederickson 1993, Knuth) are from model knowledge, not fetched during this session — standard, stable references, but flagged per the no-fabrication rule.
