# Plan 033 — GreptimeDB Prom-Read Conversion Distill (Batch 75)

> **Source:** "Just One Function, 10x Faster? Reading a Rust Performance PR" —
> Dennis Zhuang, Greptime, 2026-07-31/08-06,
> https://greptime.com/blogs/2026-07-31-prom-read-conversion-optimization
> (walkthrough of greptimedb PR #8587, `src/servers/src/prom_store.rs`).
> Found via idle-loop online perf search (queue item 11), 2026-08-26.
> **Verdict: YES, grade A-** — production Rust, measured 4–16.5× (criterion,
> 100 samples, p<0.05), single-function bounded fix-space, honest regime axes
> (adjacent vs interleaved = the fast-path hit-rate axis), zero game/engine
> coupling.

## Corpus delta (rust_perf 80 → 83)

| # | Rule (working id) | Category | Mechanism | Source evidence |
|---|---|---|---|---|
| A | `encoded-layout-borrowed-view` (dictionary-expansion trap) | allocation | A generic materializing accessor (`iter_column_as_string`) silently expands a compact encoding (`Dictionary<u32,Utf8>` = 3 strings + 1000 indices) into R owned `String`s — slower than no encoding at all (dict 2.03M vs utf8 2.73M rows/s BEFORE). Fix: per-layout borrowed-view enum returning `&str`, zero copies until a new group is confirmed | Step 3 + Step 4; the 8.9% dictionary-materialization profile line |
| B | `group-keys-borrow-allocate-on-first-sight` | complexity | Group-by-composite-key building owned keys per row: R·L string copies + R allocs + R BTreeMap ordered inserts (log S full-key compares each). Fix: borrow for comparison, `HashMap<u64, Vec<usize>>` hash index into a `Vec<Group>`, allocate the owned key ONLY on first sight (S·L copies) + one `S·log S` sort at the end if ordering is observable | Step 2 + Step 5 + Step 6; R·L → S·L cost model |
| C | `sorted-input-previous-row-fast-path` | complexity | When input is (mostly) sorted by group key, compare the current row against the group the PREVIOUS row landed in BEFORE hashing — no hash on the fast path. Correctness never depends on it (hash fallback with collision candidate lists + full compare). Adjacent ordering: ~S hashes total vs R | Step 5; adjacent 10–16.5× vs interleaved 4–5.5× — the hit rate is the axis |

**Dedup adjudication (done pre-plan):**
- `avoid-clone` = signature-level `fn(Vec<T>)` → `fn(&[T])` borrow — A is NOT
  that (no visible `.clone()`; a generic accessor does the expansion).
- `btreemap-sorted-iteration` (954) = the OPPOSITE regime (HashMap + sort per
  query → BTreeMap maintained-sorted wins on REPEATED sorted reads). B is the
  complementary regime (build once + ONE final sort beats R ordered inserts).
  B carries a context_notes cross-ref so consumers see the sorted-read-frequency
  axis that reconciles them.
- No grouping/first-sight/previous-row rule exists in rust_perf (grep-verified:
  the only "adjacent" hits are cache-line false positives).
- kernel_opt `verbatim-upload-on-format-match` (B37) is the GPU twin of A —
  cited as provenance, not dedup (cross-domain twins are established corpus
  practice).

## Tasks

- [x] T1 Verify all quoted source claims against the fetched article text (2
      subagents, Protocol A step 3) + final dedup re-grep on their output
- [x] T2 Author 3 corpus entries (rule_id/name/category/buggy_code/fixed_code/
      context_notes/bench_spec) in `src/domains/rust_perf/corpus.rs`; A carries
      the dictionary-trap regime note, B the btreemap twin cross-ref, C the
      adjacent/interleaved hit-rate regime note
- [x] T3 Bench specs: A (dict materialization vs borrowed view @ S∈{100,1000},
      R∈{10K,100K,1M}), B (per-row owned keys + BTreeMap vs first-sight + hash
      Vec + final sort, R axis at fixed S), C (hash-every-row vs prev-row fast
      path, AXIS = adjacent vs interleaved @ S=1000 R=100K) — alloc axis
      mandatory (A: R vs S allocs is the headline)
- [x] T4 Standing gates: `batch75_pseudocode_parses` + rules-present +
      concepts-covered (the B51/B63 family)
- [x] T5 Count pins: header counts (80→83, benchable 70→73, annotated),
      `rules_for_expert_total_matches_corpus`, category table walk
- [x] T6 GOAT: G1 syn-parse all new rules; G2 lib tests both states; G3 clippy 0
      `--all-targets` both states + full `ci_feature_guard.sh`; retrieval
      floors HELD (`kernel_opt,latent_retrieval` sweep, no re-pins without
      record); Pareto verdicts honest (any Tie/Dominated → annotate, never
      promote); alloc-count bit-stable ×2
- [x] T7 Research note `.research/089_greptime_prom_read_distill_batch75.md`
      (born-complete, Status: RECORD) + `.research/.highwater` 88→89
- [x] T8 Docs: `.docs/11_domains/rust_perf.md` batch row + AGENTS.md Batch 75
      entry + `.distill/001_mining_queue_snapshot.md` queue update (same commit)
- [x] T9 Commit `feat: batch 75 ...` + push; `.plans/.highwater` 31→33 (032
      landed unbumped — fix the stale ledger in the same commit)

## Pool drops (recorded, not mined)

- nnethercote "How to speed up the Rust compiler in July 2026" — monthly
  rustc-internal patch list; perf-book content already corpus foundation;
  rustc-specific fixes don't generalize (the B61 oxc exception was a
  compiler-building methodology post, this is a changelog).
- Medium "Data-driven perf optimization with Rust and Miri" — Miri-as-profiler
  niche, 2.7× single-case, no bounded fix-space class.
- Reintech/Khimananda "guides" — listicle class (Research 085 precedent).

## Non-goals

- No proposer for any of the 3 rules this batch (structural rewrites,
  type-blind proposers can't do them safely — B73/B74 suggestion-only
  precedent).
- No kernel_opt entries (the B37 twin gets a provenance cite, not a new rule).
