# Research 089: GreptimeDB Prom-Read Conversion — Borrowed Views, First-Sight Grouping, Previous-Row Fast Path (Batch 75)

> **Source:** "Just One Function, 10x Faster? Reading a Rust Performance PR" —
> Dennis Zhuang (Greptime founder/CEO), 2026-07-31 (pub 2026-08-06),
> https://greptime.com/blogs/2026-07-31-prom-read-conversion-optimization —
> a walkthrough of greptimedb PR #8587 (by @lyang24), rewriting
> `recordbatches_to_timeseries` in `src/servers/src/prom_store.rs` (the
> Prometheus remote-read tail: columnar Arrow RecordBatch → row-oriented
> TimeSeries). Found via idle-loop online perf search (queue item 11).
> **Date:** 2026-08-26 · **Status:** RECORD (Batch 75 landed — corpus 80→83)
> **Plan:** [033](../.plans/033_greptime_prom_read_distill_batch75.md)

## Verdict & distill grade

**YES — grade A-** (external article, measured, single-function bounded
fix-space, honest regime axes, zero game/engine coupling). The strongest
single-function Rust perf article of the 2026-08 search pool: criterion
bench (100 samples, every case p=0.00, M4 Max), a complete cost model
(R·L/S·L), and the author's own honest decomposition of WHY the win exists
("two plain things: do not copy what you can borrow, and do not repeat per
row what you can do once").

**Premise correction (Lane B):** the coordinator's brief said "NO Arrow
dependency anywhere in-tree" — FALSE. `riir-train` declares `arrow = "55"` +
`parquet = "55"` (optional workspace deps, the `strandset_data` feature,
Plan 318 Phase D2). Optional + example-gated + does not do dictionary-label
grouping — consumer pull for an Arrow-shaped rule remains ~nil, and the
landed entries are std-only regardless. Recorded so no future note repeats
the false premise.

## The three rules (rust_perf 80 → 83; allocation 18→20, complexity 15→16)

| # | Rule | Verdict (ours, isolated) | Source evidence (END-TO-END) |
|---|---|---|---|
| A | `encoded-layout-borrowed-view` (allocation) | **ParetoDominant 3/0/0 — 41.98×/39.38×/29.95×** at R=10K/100K/1M, allocs 4,000,004→0, bytes 156 MB→0 @ R=1M (S=100, L=4) | The dictionary trap: `iter_column_as_string` expanded `Dictionary<UInt32,Utf8>` (few distinct + u32/row) into R owned Strings per column — "the 3 strings become 1000". Fix = the `LabelValues<'a>` borrowed-view enum, `value(row) -> Option<&str>`, zero copy |
| B | `group-keys-borrow-allocate-on-first-sight` (allocation) | **ParetoDominant 3/0/0 — 1.92×/2.27×/2.37×** at R=10K/100K/300K (S=1000, L=3), allocs 70,133/700,133/2,100,133 → **8,019 FLAT** (the 7R+2S→8S+30 model, exact), bytes 55.9 MB→435 KB @300K | "R·L copies become S·L... allocation moves from every iteration to the iteration that finds something new" + BTreeMap→`HashMap<u64, Vec<usize>>` candidate lists + one final `S·log(S)` sort |
| C | `sorted-input-previous-row-fast-path` (complexity) | **Mixed 1/0/1 — adjacent (run=100) 12.92× FASTER, interleaved (run=1) 0.99× tie** (R=100K, S=1000; allocs 19/19 equal by design) | "Comparing the current row against the series the previous row landed on is enough... correctness does not depend on it". DERIVED (not stated in source): the fixed arm's own adjacent-vs-interleaved ratio ~2.1–2.8× is the honest isolated bound |

All three **suggestion-only** (type/semantics-blind to a parse-only
proposer: encoded layout, key repetition, and input ordering are all
invisible to syntax alone).

## Lane B audit findings (all numbers verified against the fetched article)

- **All quoted numbers verified** (4.1–16.5× across 16 cases; 1.8M–3.2M →
  9.6M–34.8M rows/s; the 35% dictionary anomaly is a TIME ratio, not a
  throughput drop; R·L→S·L verbatim). **Provenance flag:** the 36.2%-CPU /
  8.9%-dictionary profile lines are the PR AUTHOR's secondhand numbers —
  the blogger explicitly did not reproduce that environment ("The profile
  came from the author, and I did not reproduce that environment").
- **The `entry()` API trap is the real decision content for B**: std
  `HashMap::entry`/`BTreeMap::entry` require an OWNED key, so the natural
  map APIs FORCE per-row key materialization — the manual hash index
  (borrow → hash → candidate list → full compare → materialize on first
  sight) is the escape. Without this sentence the rule reads as "use a
  HashMap", which is not the mechanism.
- **Three correctness contracts encoded** (context_notes + 2 unit tests):
  (1) the map value is a candidate LIST — `HashMap<u64, usize>` merges
  distinct groups on hash collision ("Merging two different series for the
  sake of speed is a mistake this function cannot make"); (2) output order
  flips sorted→first-appearance unless the final sort ships; (3) with
  optional fields skipped one key can be a PREFIX of another — exhaustive
  compare required (`labels.next().is_none()` in the source; our
  `key_matches` length check).
- **The error-path provenance correction**: the PR author called the old
  behavior a "silent skip"; the blogger DISPUTES this — the old code
  collected an empty Vec then indexed `column_values[row]`, which PANICS
  out of bounds for any non-empty batch ("Nothing silent about it"). The
  landed `Err`-with-column-name beats both.
- **Family tension resolved in C's notes**: `sortedness-check-branchless-
  accumulate`/`filter-count-to-any` REMOVE per-element early exits inside
  vectorizable scans (the branch guards ns of work); C ADDS a cheap
  pre-check before expensive per-row work (hash+probe, µs of work). Both
  are correct in their regimes — the annotations say so explicitly.

## The rider (not a 4th rule)

`btreemap-sorted-iteration` (Bench 013, ParetoDominant 3.73–6.91×) gained
the completing half of its regime table (it had `context_notes: None`):
**0 sorted reads → plain Vec/HashMap; ONE final sort after a single
grouping pass → build unordered + sort once (rule B); MANY repeated sorted
reads over a long-lived map → maintain the BTreeMap (this rule).** The two
rules are mirror regimes on the sorted-read-frequency axis — a third
standalone rule would triple-count one mechanism.

## Pool drops (recorded, not mined)

- **nnethercote "How to speed up the Rust compiler in July 2026"** —
  monthly rustc-internal patch changelog; the perf-book content is already
  corpus foundation; rustc-specific fixes don't generalize (the B61 oxc
  exception was a compiler-building METHODOLOGY post; this is a list).
- **Medium "Data-driven perf optimization with Rust and Miri"** —
  Miri-as-profiler niche, single 2.7× case, no bounded fix-space class.
- **Reintech / Khimananda "guides"** — listicle class (Research 085
  precedent).

## Landing record (GOAT ALL PASS)

- G1: `batch75_pseudocode_parses` + both bench harnesses compile+run.
- G2: default lib **696/0**; bench-state lib **714/0**.
- G3: clippy **0** `--all-targets` in default AND `rust_perf_bench`
  states; full `ci_feature_guard.sh` **ALL LAYERS**; retrieval floors
  **14/14 HELD** (perf@1 96.2→97.9 band — corpus growth IMPROVED perf
  retrieval; no re-pins).
- G5/G-det: verdicts stable ×2 invocations; alloc-det stable ×3.
- Count pins walked (the 18-site discipline + 2 found by failing runs):
  header 80→83/70→73, `seed_corpus_has_eighty_three…`, allocation 20,
  complexity 16, `seventy_three_rules_have_bench_specs`, annotated 22→**26**
  (3 new + the btreemap rider), unannotated 58→57, serde round-trip pin,
  drafter-propagation pin + its **verdict-token contract**
  (ParetoDominant/Dominated/Mixed/NotBenchable — the B62 lesson class;
  caught the missing-prefix state before landing).
- **Landing bugs caught by the gates** (recorded for the generator
  lessons): (1) the part-1 anchor consumed the previous entry's closing
  `},` without re-emitting it (unclosed-delimiter at compile); (2) the
  count-comment insertion dropped a `// ` prefix on one continuation line
  (parse error at the number `48`... actually at the bare rule id); (3)
  Lane A's buggy snippet had a TYPE error (`Vec<(K,u64,u64)>` vs
  `map.into_iter()` yielding `(K,(u64,u64))`) that syn-parse cannot see —
  caught only by the bench validator's compile; the minimal fix threaded
  through the harness fold (`for (key, (count, sum))`). Lesson: **a
  pseudocode_parses gate is necessary but not sufficient — the bench
  compile is the only type-check new snippets get.**
- Bench sizing: B's top cell 300K not 1M (the buggy arm at 1M is ~7M
  allocs + 1M ordered BTreeMap inserts per call — minutes-scale for one
  cell; the R/S-ratio story is unchanged; the B73 sizing lesson).

## Queue state after this batch

rust_perf 83 rules (58 benchable→ no wait, 73 benchable / 10 NotBenchable
/ 26 annotated). Next options unchanged in kind: sec Phase 2 (owner
call) / rustc_errors promotion (owner call) / B70-fusion reopen triggers /
the next online-search pool (this batch re-validated the
recent-articles path: one strong engineering-deep article > many
listicles).
