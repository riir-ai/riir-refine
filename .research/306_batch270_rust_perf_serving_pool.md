# Research 306 — Batch 270: the splash-wave rust_perf serving pool (Plan 274)

**Status:** RECORD — landed 2026-10-21 (shikuwa/4090, riir-refine idle loop; the claim
`83f9acc7c` discharged by this landing). Source: incoai/splash @ the staged window
`be3b083c..origin/main` (205 non-merge commits, Apache-2.0) — the B268 Slice-B staged pool,
converted. Clone `.raw/b270/splash` (tip `1482f6b`, tag 1.4.0 — past the staged window;
disclosed, not mined). Zero-GPU throughout.

## What landed

+12 own rust_perf rules, corpus 242→254 (entries_39/40/41/42 — four authoring slices,
≤3 rules each per the B268 stream-idle lesson), ALL ANNOTATED NotBenchable (bench_spec
None) + 4 count-neutral rider faces:

| # | rule | cat | commits |
|---|---|---|---|
| 1 | restore-point-chain-copies-keyed-on-disk-state-count | caching | a20f344+65f9713 |
| 2 | persistent-cache-teardown-needs-second-witness | caching | 6e21cc9 |
| 3 | hot-slot-recycler-flat-tables-zero-alloc-claims | caching | a7e5347+f3eb1a0 |
| 4 | cache-share-capped-at-measured-distinct-working-set | caching | 18f9c69 |
| 5 | cache-reuse-assertion-gated-on-eviction-counter | caching | 2712e67 |
| 6 | publication-takes-over-condemned-cache-entry | caching | 716760d |
| 7 | reclaim-progress-keyed-on-refused-resource-returned | allocation | 9ca9768+f6957e8+32cf878 |
| 8 | refusal-wait-keyed-on-shortfall-returning-transfer | concurrency | fdc8f01+c27aaf7+e90b43c+a7406bb+41e8c82 |
| 9 | watchdog-deadline-moves-on-command-progress | dispatch | 8df9e95 |
| 10 | page-miss-blocks-one-ranged-read-scatter | io | c7a42cc+c14ba73 |
| 11 | tier-write-defers-to-urgent-reads-except-a-read-behind-it | io | fc6fa54+be30994+47325f9 |
| 12 | constrained-unconstrained-lanes-one-decode-command | dispatch | f52d1ad (the staged intel row — LAND verdict) |

Riders: 945635d (runway-refilled-by-consuming-op → host rule 7), ea1683d
(retry-hint-from-refill-ledger → host rule 8), 41e8c82 (form-keyed reusable counts → host
rule 8; adjudicated a FACE not a fold — its law is count-keying/underflow, not wait-keying),
d26ff04 (release-unit-matches-demand-unit → host `discretionary-cache-growth-keys-on-spare-not-available`,
entries_26, the Batch-237 governor's RELEASE-side sibling).

Slice D's f52d1ad grade LANDED (not dropped): real decision content (the scheduler's
constraint-mode partition deleted, per-lane mask scoping at Runtime.cpp:2356/2312-16/2369-73),
fresh dedup survived, honest anti-regimes priced by the source (streamed experts −1.6%
within noise). Source-reported −23.7%/−34.0% time/token on 35B/27B mixed-lane traffic.

## Slice-reported drift disclosures (all landed in-rule)

- Rule 1: single-chain disk-vs-RAM predicates are IDENTICAL (every block ancestors the
  deepest disk state) — the fixture uses two independent chains (the minimal honest fork).
- Rule 3: the shrink-ghosts test needed an ARC-bound-aware promotion (T1 = 2 at the new
  size, bound 3 ≤ 4); alloc law measured via a THREAD-LOCAL counting GlobalAlloc (a
  process-global counter was contaminated by parallel test threads — 15 phantom allocs
  root-caused). Grade A- (shape proven locally: buggy >100 allocs, fixed == 0 over 3,000 ops).
- Rule 6: the staged "rather than allocating fresh while the condemned one drains" does not
  match the pin — the pre-fix failure was a REFUSAL (std::logic_error), never a duplicate
  allocation; landed as the source states.
- Rule 7: the staged "not on optimistic estimates" wording does not exist in the source;
  the diff-faithful law is retry keyed on the RETURNED RESOURCE CLASS vs per-eviction
  "progress". f6957e8's reuse-path half folded as the rebuildable-cache clause.
- Rule 11: the staged "same tier ordering" is precisely the file's OWN worker queue
  (queuedReads — submit fetch_add :758-760, worker fetch_sub :787-89); "that read waits"
  realized as the write proceeds so the read behind waits ONE write, not every future deferral.

## THE CONFLATION CATCH (the batch's own lesson — count pins)

This lane initially read the landing rows' "corpus 926→937" as a CROSS-DOMAIN total and
bumped the kernel_opt tests.rs + router.rs pins 937→949. The combo posture run caught it:
`build_kernel_seed_corpus()` MEASURES 937 — the kernel corpus is ~937 rules BY ITSELF
(AGENTS.md: "923 rules standing at Batch 261" → 937 at B268); the landing rows' "corpus"
count is the KERNEL domain count, never the workspace total. Reverted via
`git checkout HEAD -- src/domains/kernel_opt/` and re-pinned only the ONE legitimate
cross-domain witness (kernel tests.rs `batch249_slice3_rust_perf_riders`, which pins the
RUST_PERF count 242→254). LESSON: a count pin's domain is its corpus BUILDER's domain —
read the builder, not the prose. The rust_perf-side pins (242→254, annotated 189→201,
without_spec 121→133) are domain-local and correct.

## Gates (all run this session)

- batch270 needle gate `batch270_rules_landed` FIRST-RUN PASS (39 needles × 12 rules +
  4 rider faces + the annotated-all assert; 2 en-route generator fixes: `rule.name`
  binding, unused `notes`/`haystack` binds removed, `#[allow(too_many_lines)]` on the
  exhaustive gate — the batch-244 precedent).
- default lib **2727/0**; rust_perf_bench lib **2758/0** (the `fourteen_rules_are_not_benchable`
  pin 121→133 — the pin-hiding class surfacing again, this test only compiles under
  rust_perf_bench); kernel_opt+latent_retrieval lib **3395/0** (3 ignored documented;
  2224 s); retrieval floors HELD **22/22 no re-pin** (blended 90.6/94.2/95.3; 381 s).
- clippy −D ×3 postures clean (default / no-default / rust_perf_bench); en-route the
  cargo-refine healer healed the slice headers' doc_markdown findings (7 edits,
  compile-gated).
- freeze gen2 + gen3 e2es PASS (--ignored, real clippy; 67.7 s gen2; no table movement —
  fixture labels stable under the corpus delta). 2 score-bench DRIFT rows
  (katgpt-rs quantile_balance_router-002/003-clean: frozen [] vs fresh [doc_markdown],
  fresh wins) — pre-existing fixture class, disclosed.
- score-bench #197 appended: heal **0.9636 FLAT** / parse **1.0** (gen-1 SATURATED,
  marginal +0.0000; floors held no re-pin).
- release-set binary rebuilt (build `83f9acc7c` era, release set complete) — installed
  to `~/.cargo/bin` at landing (the Windows cp, never a silent symlink assumption).

## Count pins moved (ONE shape-asserted script pass + 3 adjudicated stragglers)

rust_perf/corpus/tests.rs ×7 (total, with_spec msg, annotated ×2, unannotated msg,
witness) + mod.rs header ×2; kernel_opt/corpus/tests.rs ×1 (the batch249_slice3
rust_perf-count witness 242→254); AGENTS.md; README; .docs/11_domains/rust_perf.md.
The kernel-side 937 pins are NOT batch-270 surface (domain-local).

## Staged for Batch 271+ (the second tier, graded B/B− at the B268 read, NOT converted)

`253506f`, `88d0e57`, `91d4ddb`, `4d17995`, `fa63d97`+`593f2a6`, `68b9e70`, `81ae003`,
`ed1092f`+`2f5fe8d`, `dab8841`+`6fcd9b1`+`25502fd`+`7e51fcb`, `5ea4043`+`82b93c5`,
`b7844dc`, `4fc7465`, `e2c0af2` — scope control: the batch was already at 12 rules
(the protocol's 8-12 band, exceeded by the f52d1ad LAND); the second tier keeps the
B-slice grades and stays staged. Splash post-window commits (`1482f6b`..) are the next
standing-watch's concern.
