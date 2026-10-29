# Research 346: exact-match hint builder (modded-nanogpt PR #367) — distill verdict

**Status:** DISTILLED — pending owner decision

> **Source:** the `exact_match/` Rust pyo3 extension of hermabr's [PR KellerJordan/modded-nanogpt#367](https://github.com/KellerJordan/modded-nanogpt/pull/367) (fork `hermabr/modded-nanogpt-public`, MIT); **provenance pinned at PR head `e390cb25cf5c732ed4471c2ad861af28114e0a56`**. Author writeup: <https://brunborg.com/n-gram-hinted-lm> (2026-10-28).
> **Inspected:** 2026-10-29 — GitHub patch API at the pinned sha; full source read of `slot.rs` (846 L), `stream.rs` (705 L), `lib.rs` (265 L), `canon.rs` (220 L) + manifest (release profile `lto="thin"`, `codegen-units=1` — they benchmark; good signal, cited). No `.raw` clone this pass — patches fetched verbatim; clone + re-pin at mine time per the standing law.
> **Session:** zed-m3-exactmatch-rect, 1793238159
> **Related:** riir-train Research 482 (the training-record half of the same PR), katgpt-rs Research 278 (Engram — the design-space cousin)

## Verdict

**YES B− corpus / no league-lane mapping / product-coverage: generic scatter+hash infrastructure (fires at home: riir-neuron-db LocalKv WAL + Bm25Index, riir-infer quant tables, seal-editor dbs).** Domain routing: `rust_perf` — NOT kernel_opt (CPU-side data-structure work; the AVX-512 intrinsics are CPU SIMD and the rust_perf simd/caching/io taxonomy owns them; the tempting wrong domain is kernel_opt because the code smells like GPU-adjacent table building — it is a 48-core CPU build path).

## The source in one paragraph

A ~2.0k-line pyo3 cdylib that builds and queries exact-match n-gram retrieval indexes at speedrun scale (37B tokens indexed in ~6 s on 48 cores; O(1) two-bucket lookups over a 352-GiB-class table). Two index types: SlotIndex (validation; hash-partitioned u64 slot table) and StreamIndex (training; strictly causal per-step resolution over a tag→position index, radix-sorted per partition). The build is a two-stage scatter: stage 1 hashes n-grams (AVX-512 IFMA `vpmadd52luq` multiply-add-shift + `vpermb` byte permutes) and scatters u64 entries through per-partition 64-byte "burst lines" staged in L2; stage 2 places entries into two candidate buckets per key with branch-free saturated fill counts and streams completed lines with non-temporal stores.

## Ranked candidates

| # | Pattern | Source | Grade | Dedup evidence (both vocabularies greped) |
|---|---|---|---|---|
| 1 | `random-scatter-burst-line-staging` — per-key 64 B staging lines in L2 (one cache line per partition), ALWAYS-copy-conditional-advance flush queue (every burst is copied to the queue; the queue index advances only when the burst is full → zero data-dependent branches in the insert path), non-temporal drain every CHUNK inserts. Two-regime: random-key-heavy scatter (win) vs sequential writes (anti-regime — staging adds a copy for nothing) | `slot.rs` `hash_scatter` + `stream.rs` `index` | **B+ GOAT lead** | `write.?combin` zero hits; `burst` matched only a RwLock bench comment and bounded-buffer pooling (allocator axis, different mechanism); the branchless family (`branchless-conditional`, `sortedness-check-branchless-accumulate`) owns accumulate-shape loops, not scatter staging; `select_unpredictable` zero hits |
| 2 | `prefault-before-timed-region` — touch one byte per page of all build memory before the clock (`write_volatile` per 4096 B, parallel 16 MB pieces), `MADV_POPULATE_READ` file shards into page cache, huge-page-aligned mmap + `MADV_HUGEPAGE`, `MADV_DONTFORK`, VMA-split `MADV_DONTDUMP` (mmap-lock contention vs a /proc smaps reader), local NUMA binding | `lib.rs` `touch`/`prefault`/`mmap`/`madvise` | **B** | `prefault|madvise|huge.?page|numa` zero corpus hits; the measurement-honesty family warms the CLOCK (whole-round in-situ adoption trial; seed-warm-the-scored-seam), never memory pages — distinct axis (first-touch fault + zeroing cost inside the timed region) |
| 3 | `select-unpredictable-saturating-two-bucket-insert` — `select_unpredictable(in1, h1, h2)` cmov bucket pick; saturating fill `fill + (f < slots)` so the count stops at bucket size; a dropped entry lands in lane 0 of the full bucket's ALREADY-FLUSHED staging line (slots is a multiple of the burst width) so it provably never reaches the table | `slot.rs` `place()` | **B−** (small rule or rider on `branchless-conditional`) | parent rule exists (`branchless-conditional`); the new atoms are the cmov placement + the saturating-fill drop trick, not the branchless multiply itself |
| 4 | `nested-checksum-shared-slot` — suffix-nested key families (6-gram ⊂ 10-gram ⊂ 20-gram) share ONE 64-bit slot: bit budget split 44/12/6 by collision cost (a short-key collision corrupts the answer; a long-key collision only degrades trust), longest-match = the ordered fingerprint-agreement run | `slot.rs` entry layout | C+ suggestion-only | zero hits; niche — needs suffix-nested key families |
| 5 | Shared-counter false-sharing padding after a measured ⅓ slowdown (`#[repr(align(128))] Padded<T>`) | `lib.rs` | DROP | positive instance of `align-64-false-sharing` (Batch 65) — the corpus owns the rule; the instance's measurement ("the arena's shared block counter made the build a third slower when it shared a line with the arena pointer, in about half the runs") is rider-grade color at most |
| 6 | glibc mallopt keep-freed-memory (mmap/munmap avoidance under a /proc smaps reader holding the mmap lock) | `track_1_short/perf/host_heap.py` | DROP | Python host script (out of domain) + Slurm-specific trigger |

## GOAT candidate (the strongest benchable)

`random-scatter-burst-line-staging`, against the four criteria: (1) **benchable** — the Pareto validator hosts a synthetic random-key scatter into a preallocated partition array: direct-store arm vs burst-staged arm, both zero-alloc, deterministic input; (2) **exact semantics** — integer slots, byte-identical final table by construction (the source proves order preservation: "the finish places a stolen run right after the pieces before it, so the table is the same as without stealing"); (3) **magnitude story** — a per-insert random store pays RFO + TLB pressure; burst lines pay one 64 B load + one 64 B store into L2-resident lines and amortize the random access into CHUNK-batched drains; the branch-free queue eliminates ~1 mispredict per BURST inserts; (4) **deterministic** — same input, same table, no timing-dependent verdicts. Passes all four.

## Yield estimate

2–3 rules + 2–3 riders after the standard ~50% coordinator falsification (candidates 1–2 as rules; 3 as rule-or-rider; 4 as suggestion-only if it survives; 5 as a rider line on the existing false-sharing rule).

## Honest caveats

1. **PR is OPEN and unmerged — RE-PIN IS A BATCH REQUIREMENT, not optional.** Everything here is pinned to head `e390cb25`; the batch's FIRST task re-clones + re-pins, and if the PR head has moved, re-diffs the four files and re-grades any candidate whose source hunks changed BEFORE any corpus edit.
2. **AVX-512-mandatory build path** (`assert!(*AVX512)`; record hosts are Sapphire Rapids / Zen 4). Rule fixtures must be baseline-ISA — the AVX-512 hash is color, not the corpus's rule shape (our simd rules are portable/auto-vectorize shaped).
3. **Scale sensitivity**: the headline wins (500× vs naive Python, 11× vs an AI-researched version) are END-TO-END speedrun deltas; the blog's stage ladder (hash-map 21 s → slot 18.3 s → shared-slot 11.5 s → sort/branch-free+L2 6.7 s) mixes mechanisms per stage — never cite a stage delta as an isolated mechanism number; our own bench owns that.
4. **Not a spec-decode/league source.** The hint LOOKUP is an eval-forward feature, not speculative drafting; the spec-decode lanes (closed-negative per riir-ai AGENTS.md) are unchanged by this source — hence "no league-lane mapping."
5. `canon.rs` (BPE-replay canonical token mask) read, no candidate — tokenizer-specific, single-use. `stream.rs`'s radix-sort-by-tag resolve path shares stage-1 machinery; no separate rule beyond candidate 1.

## Queue slot

Behind the live staged queue; the batch opens plan-first on owner decision (the standing law — no corpus edit before its `.plans/` plan).
