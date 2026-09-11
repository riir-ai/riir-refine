# Bench 083 — Issue 092 T1: cross-domain rule-corpus alignment PoC (GW vs name/keyword vs no-alignment)

**Status:** RECORD — T1 defend-wrong verdict measured: **GW correspondence map LOSES at this embedding** (precision = base rate). T2/T3 do not fire. Report-first per R135/Bench-047 doctrine; no wiring, no heal-path change.

**Date:** 2026-09-12 · **Commits:** katgpt-rs `b09f1368` (the `gw_coupling` accessor — the correspondence surface the PoC reads) · this repo: `examples/gw_corpus_poc.rs` + feature `gw_corpus` (opt-in, measurement-only).

## Setup

- **Corpora:** clippy_lints (49 rules) × rust_perf (126 rules) = 6,174 candidate pairs. Universe fixed by `build_corpus()`.
- **Arm (a) GW correspondence map:** each rule embedded from `buggy_code`+`fixed_code` ONLY — **names withheld from this arm** (sharing lexical signal with the baseline would make the comparison circular). Trigram profiles (lowercased, whitespace-collapsed) → cosine distance → per-domain matrix normalized by mean off-diagonal distance (scale-invariant shape). rust_perf split deterministically even/odd into two 63-rule halves (`GW_MAX=64 < 126`); two `gw_coupling` solves vs the 49 clippy rules; couplings unioned → full 49×126 mass map; pairs ranked by mass. Half-solve losses 0.0662 / 0.0753 (score ≈ 0.43) — measurable shared structure, not isometry.
- **Arm (b) name/keyword baseline** (today's dedup, as a ranking): token Jaccard over identifiers (clippy `lint_name` tokens vs perf `rule_id`+`name`+`category` tokens), ties broken lexicographically (deterministic).
- **Arm (c) no-alignment:** base rate from a deterministic stride sample of 60 pairs.
- **Ground truth:** ALL top-60 of each arm + 60 stride pairs = **180 pair reads, hand-labeled** against criteria fixed BEFORE labeling: TRUE = same trigger family AND same fix-transform family, at a specificity where only a handful of rules in either corpus belong (same THEME — "allocation", "use idiomatic API" — is explicitly NOT enough: the vacuous-family trap). Full buggy/fixed text read for both rules of every pair. The sheet + labels live in this record's appendix; the harness emits them deterministically (`--sheet` / `--score`).
- **Known negatives:** the planned class — rust_perf `context_notes` "Distinct from <clippy-lint>" cross-refs — turned out **EMPTY**: no perf rule's notes name any of the 49 clippy lints (all "Distinct from" refs are within-domain perf ids). The extractor ran; 0 violations reported is a vacuous 0, honestly noted.

## Results (budget 60 per arm, matched)

| arm | trues in top-60 | precision@60 | recall est. (p̂=0.017) |
|---|---|---|---|
| (a) GW coupling | **1** | **0.017 — exactly the base rate: no signal** | 0.010 |
| (b) name/keyword | **5** | **0.083 — 5× base rate: real, weak signal** | 0.049 |
| (c) no-alignment | — | base rate 0.017 | ≈1.0 expected finds |

- Base-rate sample: 1/60 stride pairs TRUE → p̂ = 0.017; total-true estimate ≈ 103 (Wilson 95% ≈ 20–550 — the recall denominator is weak; **precision is the load-bearing read**).
- GW-exclusive TRUEs (baseline score 0): exactly **1** — `manual_slice_fill` × `retain-inplace` (gw=0.00252, ranked #29). The one capability data point — at base-rate precision.
- The GW arm's top masses (0.01587 = 1/63, full-column assignments, ranks #0–#8) are all semantically unrelated pairs (`collapsible_else_if`×`powi-vs-powf`, `double_parens`×`parallel-disjoint-pread`, …): the coupling's strongest one-to-one assignments are structural artifacts.
- **Robustness:** flipping ANY single borderline label cannot rescue the GW arm — if the one GW TRUE is judged FALSE, GW precision is 0/60 (below base rate); striking the most contestable baseline TRUE (`unnecessary_lazy_evaluations`×`ok-or-else`, the inverse-direction pair) still leaves baseline at 4/60 = 0.067, 4× GW.

## Mechanism (why the GW arm lost)

1. **The embedding carries surface-token distribution, not fix-shape.** Trigram-cosine over short snippets is dominated by snippet length + Rust boilerplate (`fn`, `for`, `let`). The resulting distance matrices are smooth; GW aligns their coarse structure and spreads mass, with a few saturated columns landing on snippet-shape coincidences.
2. **True correspondences are name-visible BY CONSTRUCTION in this test bed.** Both corpora name rules after the Rust API they fix (`chunks_exact`, `sort_by`, `filter`, `unwrap_or`, `fill`/`copy_from_slice`) — the two domains share one API vocabulary, which is exactly what the baseline consumes. The vendor-confirmed prior-art gap (SonarQube "no similar-rule mapping", eslint↔sonar unanswered) is about analyzers with DISJOINT vocabularies; this corpus pair is not that regime.
3. Structure-only alignment can in principle see fix-transform similarity an identifier cannot — but only once the embedding encodes fix-shape (AST/transform fingerprints, not character n-grams).

## Honest limitations

- The labeler is the same agent that built the harness (arm membership visible in sheet headers). Mitigation: criteria pre-registered + the verdict is robust to every single-label flip (above); still, an independent re-label of the 180 pairs is the cheapest replication.
- One corpus pair, one embedding, one budget. The verdict is "GW loses AT THIS EMBEDDING on THIS test bed", not "GW alignment cannot work for rule corpora".
- Recall estimates inherit the 1/60 base-rate CI; read precision.

## The 7 labeled TRUEs

| pair | arm that surfaced it | family |
|---|---|---|
| `chunks_exact_to_as_chunks` × `chunks-exact` | baseline (#0) | slice chunking API selection |
| `manual_slice_fill` × `copy-from-slice` | baseline (#1) | element-wise loop → bulk slice method |
| `unnecessary_sort_by` × `sort-by-key` | baseline (#4) | comparator closure → specialized sort |
| `filter_next` × `filter-count-to-any` | baseline (#21) | filter + wrong terminal → direct query method |
| `unnecessary_lazy_evaluations` × `ok-or-else` | baseline (#40) | or/or_else laziness-cost axis (inverse direction, same concern) |
| `unnecessary_lazy_evaluations` × `unwrap-or-else-lazy` | random sample | same axis, the perf-side half |
| `manual_slice_fill` × `retain-inplace` | **GW (#29)** | element-wise → bulk in-place slice method |

## Verdict + reopen trigger

**T1: GW correspondence map LOSES to the name/keyword baseline at matched review budget (0.017 vs 0.083 precision; GW = base rate).** T2 (dedup actions + ingest screening) and T3 (evidence-transfer experiment) are gated on a T1 win and do NOT fire. T4 moot.

**Reopen trigger** (recorded, not scheduled): a semantic fix-shape embedding — AST- or transform-fingerprint similarity rather than character trigrams — or a corpus pair with genuinely disjoint vocabularies (e.g. clippy × docker, where names share nothing). Until then the standing dedup surface remains name/convention-level, now with a measured negative justifying it.

## Reproduction

```sh
cargo run --features gw_corpus --example gw_corpus_poc -- --sheet > sheet.md   # deterministic
cargo run --features gw_corpus --example gw_corpus_poc -- --score labels.txt
```

## Appendix — ground-truth labels (180 pairs; full file also at `.heal/gw_poc/labels.txt`, uncommitted runtime dir)

TRUEs listed above; every other labeled pair FALSE with a one-line family-diff note. Format `<clippy_lint> | <perf_rule> | VERDICT | note`:

```
collapsible_else_if | powi-vs-powf | FALSE | if-merge vs float method choice
comparison_to_empty | compaction-tail-boundary-prefers-complete-turn | FALSE | empty-check vs truncation boundary
double_parens | parallel-disjoint-pread-hides-cold-page-latency | FALSE | paren removal vs IO parallelism
iter_next_loop | newton-refinement-vs-hardware-transcendental | FALSE | iterator misuse vs fast-math
manual_unwrap_or | dispatch-handler-pair-hoist | FALSE | match-to-combinator vs loop-invariant hoist
needless_bool_assign | async-yield-point | FALSE | bool assign vs cooperative yield
byte_char_slices | repr-u8-enum | FALSE | byte literal syntax vs enum layout attr
manual_checked_ops | mem-take | FALSE | div guard vs move idiom
while_let_on_iterator | channel-send-batching-unpark | FALSE | iterator idiom vs channel batching
uninlined_format_args | bool-to-bitpack | FALSE | format arg inline vs bitpacking
redundant_field_names | keyword-map-to-match | FALSE | field shorthand vs dispatch structure
needless_lifetimes | keyword-map-to-match | FALSE | lifetime elision vs dispatch structure
filter_next | bool-to-bitpack | FALSE | find vs bitpacking
redundant_closure | weight-tile-consumption-order | FALSE | closure-to-fn vs layout repack
question_mark | division-by-invariant-multiplication | FALSE | propagation vs magic division
single_match | division-by-invariant-multiplication | FALSE | if-let vs magic division
unnecessary_unwrap | weight-tile-consumption-order | FALSE | unwrap fusion vs layout repack
deref_addrof | weight-tile-consumption-order | FALSE | deref removal vs layout repack
redundant_pattern_matching | division-by-invariant-multiplication | FALSE | is_ok vs magic division
needless_match | runtime-feature-detect-per-chunk | FALSE | identity match vs detection hoist
map_unwrap_or | software-prefetch-storage-bound | FALSE | map_or fusion vs prefetch removal
manual_let_else | runtime-feature-detect-per-chunk | FALSE | let-else vs detection hoist
unnecessary_lazy_evaluations | runtime-feature-detect-per-chunk | FALSE | eager default vs detection hoist
match_bool | runtime-feature-detect-per-chunk | FALSE | if-for-match vs detection hoist
wildcard_in_or_patterns | software-prefetch-storage-bound | FALSE | pattern simplify vs prefetch removal
collapsible_if | filter-count-to-any | FALSE | condition fusion vs existence short-circuit
identity_op | software-prefetch-storage-bound | FALSE | identity removal vs prefetch removal
semicolon_if_nothing_returned | retain-inplace | FALSE | semicolon vs in-place filter
manual_slice_fill | retain-inplace | TRUE | element-wise slice work -> bulk in-place method (fill/retain family)
redundant_static_lifetimes | retain-inplace | FALSE | lifetime elision vs in-place filter
chunks_exact_to_as_chunks | retain-inplace | FALSE | chunked decode vs in-place filter
unused_unit | retain-inplace | FALSE | unit return vs in-place filter
needless_return | software-prefetch-storage-bound | FALSE | return removal vs prefetch removal
unnecessary_sort_by | retain-inplace | FALSE | sort specialization vs in-place filter
items_after_statements | runtime-feature-detect-per-chunk | FALSE | item order vs detection hoist
needless_bool | software-prefetch-storage-bound | FALSE | bool identity vs prefetch removal
suboptimal_flops | retain-inplace | FALSE | float method choice vs in-place filter
useless_format | filter-count-to-any | FALSE | const format vs existence short-circuit
use_self | filter-count-to-any | FALSE | Self path vs existence short-circuit
single_char_pattern | filter-count-to-any | FALSE | str pattern type vs iterator existence
unnecessary_mut_passed | filter-count-to-any | FALSE | mut borrow vs existence short-circuit
manual_is_multiple_of | filter-count-to-any | FALSE | divisibility method vs existence check
range_plus_one | filter-count-to-any | FALSE | inclusive range vs existence check
needless_range_loop | filter-count-to-any | FALSE | index loop vs existence check
iter_nth_zero | filter-count-to-any | FALSE | nth(0) misuse vs existence check
needless_borrow | filter-count-to-any | FALSE | borrow removal vs existence check
manual_map | filter-count-to-any | FALSE | map combinator vs existence check
useless_conversion | filter-count-to-any | FALSE | no-op conversion vs existence check
collapsible_if | collect-avoidance | FALSE | condition fusion vs chained iterator
byte_char_slices | layout-reorder | FALSE | byte literal vs field reorder
byte_char_slices | bevy-reactive-loop | FALSE | byte literal vs event loop mode
redundant_static_lifetimes | layout-reorder | FALSE | lifetime elision vs field reorder
unnecessary_lazy_evaluations | hot-byval-move-over-memcpy-threshold | FALSE | eager default vs move shrinking
redundant_static_lifetimes | bevy-reactive-loop | FALSE | lifetime elision vs event loop mode
needless_match | hot-byval-move-over-memcpy-threshold | FALSE | identity match vs move shrinking
map_unwrap_or | hot-byval-move-over-memcpy-threshold | FALSE | map_or fusion vs move shrinking
needless_bool | collect-avoidance | FALSE | bool identity vs chained iterator
useless_conversion | transposed-weight-register-gemv | FALSE | no-op conversion vs GEMV layout
chunks_exact_to_as_chunks | chunks-exact | TRUE | slice chunking API selection family (chunks/chunks_exact/as_chunks)
manual_slice_fill | copy-from-slice | TRUE | manual element-wise slice loop -> bulk slice method (fill/copy_from_slice family)
manual_unwrap_or | unwrap-or-else-lazy | FALSE | match-to-combinator vs eager-to-lazy; different trigger and transform
map_unwrap_or | unwrap-or-else-lazy | FALSE | map_or fusion vs laziness; different axes
unnecessary_sort_by | sort-by-key | TRUE | comparator closure sort -> specialized sort call family (sort()/sort_by_key)
unnecessary_unwrap | collect-avoidance | FALSE | unwrap fusion vs chained iterator
comparison_to_empty | dyn-to-enum | FALSE | empty-check vs devirtualization
iter_next_loop | bevy-reactive-loop | FALSE | iterator misuse vs event loop mode
iter_next_loop | vec-with-capacity | FALSE | iterator misuse vs preallocation
needless_range_loop | bevy-reactive-loop | FALSE | index loop vs event loop mode
needless_range_loop | vec-with-capacity | FALSE | index loop vs preallocation; iterate-directly != preallocate
redundant_static_lifetimes | static-str | FALSE | elide lifetime vs change return type; different rules
unnecessary_lazy_evaluations | collect-avoidance | FALSE | eager default vs chained iterator
unnecessary_mut_passed | collect-avoidance | FALSE | borrow fix vs chained iterator
unnecessary_sort_by | collect-avoidance | FALSE | sort specialization vs chained iterator
useless_format | format-concat-loop | FALSE | no-arg format vs loop concat capacity
chunks_exact_to_as_chunks | dyn-to-enum | FALSE | chunking vs devirtualization
collapsible_else_if | ok-or-else | FALSE | if-merge vs lazy error construction
comparison_to_empty | vec-remove-front-to-vecdeque | FALSE | empty-check vs queue data structure
deref_addrof | iter-copied-deref-map | FALSE | no-op deref vs adapter specialization
double_parens | hashset-insert-bool-return | FALSE | paren removal vs double-lookup merge
filter_next | filter-count-to-any | TRUE | filter-adapter + wrong terminal op -> direct query method (find()/any())
filter_next | filter-map-fuse | FALSE | terminal op replacement vs adapter fusion
iter_next_loop | format-concat-loop | FALSE | iterator misuse vs string capacity
manual_let_else | ok-or-else | FALSE | let-else vs lazy error construction
manual_map | copy-from-slice | FALSE | map combinator vs bulk slice copy
manual_map | filter-map-fuse | FALSE | Option match-to-map vs iterator adapter fusion
manual_map | iter-copied-deref-map | FALSE | Option match-to-map vs deref adapter
manual_map | papaya-lockfree | FALSE | map combinator vs lock-free map structure
manual_slice_fill | chunks-exact | FALSE | loop-to-fill vs chunking method; chunks-exact is not a loop-to-bulk rule
manual_unwrap_or | ok-or-else | FALSE | match-to-combinator vs lazy error construction
map_unwrap_or | ok-or-else | FALSE | map_or fusion vs lazy error construction
match_bool | hashset-insert-bool-return | FALSE | if-for-match vs double-lookup merge
needless_bool | hashset-insert-bool-return | FALSE | bool identity vs double-lookup merge
needless_borrow | avoid-clone | FALSE | redundant borrow vs own-to-borrow conversion
needless_range_loop | format-concat-loop | FALSE | index loop vs string capacity
needless_return | hashset-insert-bool-return | FALSE | return removal vs double-lookup merge
redundant_closure | unwrap-or-else-lazy | FALSE | closure-to-fn vs laziness
single_match_else | ok-or-else | FALSE | if-let-else vs lazy error construction
uninlined_format_args | format-concat-loop | FALSE | inline args vs string capacity
unnecessary_lazy_evaluations | ok-or-else | TRUE | or/or_else laziness-cost axis family (case-dependent direction)
unnecessary_unwrap | unwrap-or-else-lazy | FALSE | unwrap fusion vs laziness
while_let_on_iterator | collect-avoidance | FALSE | for-loop idiom vs chained iterator
chunks_exact_to_as_chunks | vec-remove-front-to-vecdeque | FALSE | chunking vs queue data structure
collapsible_else_if | unwrap-or-else-lazy | FALSE | if-merge vs laziness
comparison_to_empty | collect-append-to-extend | FALSE | empty-check vs extend fusion
comparison_to_empty | filter-count-to-any | FALSE | empty-check vs existence short-circuit
comparison_to_empty | mutex-to-atomic | FALSE | empty-check vs lock structure
filter_next | retain-inplace | FALSE | existence query (find) vs in-place mutation (retain); different transform family
iter_next_loop | copy-from-slice | FALSE | iterator misuse vs bulk slice copy
iter_next_loop | dedupe-hashset-vs-vec | FALSE | iterator misuse vs dedup structure
iter_next_loop | hashmap-with-capacity | FALSE | iterator misuse vs preallocation
iter_next_loop | iter-copied-deref-map | FALSE | iterator protocol vs adapter specialization
iter_next_loop | vec-insert-front-deque | FALSE | iterator misuse vs queue data structure
iter_nth_zero | iter-copied-deref-map | FALSE | first-element extraction vs deref adapter
manual_checked_ops | copy-from-slice | FALSE | checked arith vs bulk slice copy
manual_let_else | copy-from-slice | FALSE | let-else vs bulk slice copy
manual_let_else | unwrap-or-else-lazy | FALSE | let-else vs laziness
manual_slice_fill | bsearch-sorted-slice | FALSE | loop-to-fill vs search structure
manual_unwrap_or | copy-from-slice | FALSE | match-to-combinator vs bulk slice copy
match_bool | collect-avoidance | FALSE | if-for-match vs chained iterator
needless_bool | hashmap-with-capacity | FALSE | bool identity vs preallocation
needless_borrow | streaming-prefix-holdback-bounded-buffer | FALSE | borrow removal vs stream matcher
unnecessary_lazy_evaluations | control-reproduces-on-baseline-trims-overclaim | FALSE | eager default vs bench method
chunks_exact_to_as_chunks | hashmap-with-capacity | FALSE | chunking vs preallocation
manual_is_multiple_of | lock-guard-narrow-field-accessor | FALSE | divisibility method vs lock narrowing
range_plus_one | hashmap-with-capacity | FALSE | inclusive range vs preallocation
items_after_statements | partitioned-mut-slices-zero-sync | FALSE | item order vs ownership partition
suboptimal_flops | sort-key-cache-precompute-out-of-lock | FALSE | float method vs comparator caching
identity_op | bitplane-transpose-repeated-predicate-scan | FALSE | identity removal vs column encoding
single_match | loop-fusion | FALSE | if-let vs pass fusion
deref_addrof | unset-control-arm-silent-default-parity | FALSE | deref removal vs bench method
needless_match | unset-control-arm-silent-default-parity | FALSE | identity match vs bench method
needless_lifetimes | copy-from-slice | FALSE | lifetime elision vs bulk slice copy
manual_slice_fill | unset-control-arm-silent-default-parity | FALSE | loop-to-fill vs bench method
redundant_static_lifetimes | chunks-exact | FALSE | lifetime elision vs chunking
chunks_exact_to_as_chunks | prefix-sum-2d | FALSE | chunking vs precomputation
match_bool | ab-isolate-one-component-by-baseline-byte-copy | FALSE | if-for-match vs bench isolation
filter_next | bevy-reactive-loop | FALSE | existence query vs event loop mode
needless_lifetimes | prefix-sum-2d | FALSE | lifetime elision vs precomputation
needless_bool_assign | bounded-channel-backpressure | FALSE | bool assign vs backpressure
redundant_pattern_matching | cache-line-block-partitioned-lock | FALSE | is_ok vs lock granularity
chunks_exact_to_as_chunks | transposed-weight-register-gemv | FALSE | chunking vs GEMV layout
unnecessary_sort_by | sorted-input-previous-row-fast-path | FALSE | sort specialization vs grouping fast path
useless_conversion | cache-line-block-partitioned-lock | FALSE | no-op conversion vs lock granularity
useless_format | vec-macro-zero-fill | FALSE | const format vs zeroed alloc
while_let_on_iterator | vec-macro-zero-fill | FALSE | for-loop idiom vs zeroed alloc
needless_lifetimes | hashset-insert-bool-return | FALSE | lifetime elision vs double-lookup merge
redundant_closure | bufwriter-batched | FALSE | closure-to-fn vs buffered IO
uninlined_format_args | avoid-clone | FALSE | inline args vs borrow-not-clone
range_plus_one | filter-before-expensive-transform | FALSE | inclusive range vs order-of-operations
manual_is_multiple_of | copy-from-slice | FALSE | divisibility method vs bulk slice copy
manual_let_else | bufwriter-batched | FALSE | let-else vs buffered IO
needless_range_loop | matched-control-disentangles-policy-from-model | FALSE | index loop vs bench method
collapsible_else_if | diagnostics-not-just-exit-code | FALSE | if-merge vs bench method
filter_next | serialize-once-byte-buffer-store | FALSE | existence query vs serialization layout
question_mark | encoded-layout-borrowed-view | FALSE | propagation vs column layout
uninlined_format_args | debug-level-gated-computation | FALSE | inline args vs level gating
redundant_closure | papaya-lockfree | FALSE | closure-to-fn vs lock-free map
semicolon_if_nothing_returned | debug-assert-length-hoist | FALSE | semicolon vs assert level
question_mark | entry-vs-contains-insert | FALSE | propagation vs entry merge
question_mark | hashmap-with-capacity | FALSE | propagation vs preallocation
needless_match | entry-vs-contains-insert | FALSE | identity match vs entry merge
unnecessary_lazy_evaluations | unwrap-or-else-lazy | TRUE | or/or_else laziness-cost axis family (same concern as ok-or-else pair)
unnecessary_lazy_evaluations | byte-find-position-to-memchr | FALSE | eager default vs SIMD scan
filter_next | sender-per-work-item-terminates-channel | FALSE | existence query vs channel termination
manual_unwrap_or | distance-sq-no-sqrt | FALSE | match-to-combinator vs sqrt elimination
manual_checked_ops | format-concat-loop | FALSE | checked arith vs string capacity
collapsible_else_if | vec-insert-front-deque | FALSE | if-merge vs queue data structure
needless_bool_assign | macro-generated-code-optimize-at-generation-point | FALSE | bool assign vs macro emission
iter_next_loop | vec-remove-front-to-vecdeque | FALSE | iterator misuse vs queue structure
comparison_to_empty | dispatch-handler-pair-hoist | FALSE | empty-check vs loop-invariant hoist
iter_next_loop | parallel-disjoint-pread-hides-cold-page-latency | FALSE | iterator misuse vs IO parallelism
manual_slice_fill | weight-tile-consumption-order | FALSE | loop-to-fill vs layout repack
needless_lifetimes | division-by-invariant-multiplication | FALSE | lifetime elision vs magic division
redundant_closure | powi-vs-powf | FALSE | closure-to-fn vs float method
single_match_else | bool-to-bitpack | FALSE | if-let-else vs bitpacking
unnecessary_unwrap | keyword-map-to-match | FALSE | unwrap fusion vs dispatch structure
wildcard_in_or_patterns | retain-inplace | FALSE | pattern simplify vs in-place filter
comparison_to_empty | sort-by-key | FALSE | empty-check vs comparator specialization
collapsible_if | collect-avoidance | FALSE | condition fusion vs chained iterator
manual_let_else | runtime-feature-detect-per-chunk | FALSE | let-else vs detection hoist
```
