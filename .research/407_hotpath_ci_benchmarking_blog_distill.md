# 407 — hotpath.rs CI-benchmarking blog distill (the FOURTH hotpath-family pass)

**Status:** DISTILLED — pending owner decision · queue slot BEHIND Batch 362 (landing pending) → nominal Batch 363 · plan 374 filed · verdict HEAD `2a1a7875b`

## Source

- **URL:** <https://hotpath.rs/blog/rust-performance-benchmarking> — "Rust Benchmarking: How to Detect Performance Regressions in CI"
- **Author:** Paweł Urbanek (@pawurb, the hotpath-rs profiler author) · **Published:** 2026-11-10 · ~21 min
- **Inspected:** 2026-11-14 (this box, M3) · full-text fetch; no clone needed (blog source — the two code-shaped rows cite hotpath's OWN landed PRs #720/#801, upstream-present by construction; repo license MIT per R234's pin `ac4096ec8f`)
- **Trigger:** user-directed re-distill ("new blog to re-distill")

## Lineage — the fourth pass on the hotpath family

This article postdates every prior snapshot; the CI-regression mechanisms are the unmined residue:

| Pass | Note | What it produced |
|---|---|---|
| R032 (2026-08-14) | `.research/032` — the hotpath.rs *guide* blog | `regex-hoist-lazylock`, `arc-str-payload`, `tokio-guard-across-await` + **the Issue-014 alloc axis** (riir-refine's deterministic alloc bench axis descends from this pass) |
| R081 / B65 (2026-08-22) | `.research/081` — the profiler-contention blog | `align-64-false-sharing`, `channel-send-batching-unpark`; rust_perf 74→76 |
| R234 (2026-10-04) | `.research/234` — the repo `pawurb/hotpath-rs` @ `ac4096ec8f` MIT | `timing-hot-path-direct-counter-read`, `sample-durations-random-draw-not-stride`, `per-thread-spsc-chunk-drain-single-sweeper`, `thread-local-attribution-not-across-await` (conditional); verdict YES B, lane-intel NONE |
| R235 (2026-10-05) | `.research/235` — product fusion | the healer's `--hotpath-json` lane (Issue 146 / Plan 134) |
| **This pass** | `.research/407` — the **CI blog** (2026-11-10) | the CI-regression / deterministic-metric residue, below |

## Verdict

**YES B− corpus (rust_perf: measurement-honesty family extension + 1 benchable layout/async lead) / lane-intel: CONFIRMATION-ONLY / product-coverage: seeds the CI-benchmark-gating corner of the measurement-honesty family (6 canonical rules today: 3 `measurement-honesty` + 3 `measurement`).**

The STATUS drives recording (B-band YES → research note + plan + queue row, one commit); the letter is informational — the discount from B is the mostly-suggestion-only yield plus the vendor-promo caveat, not the decision content, which is genuinely quantified and dedup-clean.

## The source's core decision content — the determinism ladder

100 identical GitHub Actions runs (empty PR, empty commits, same source + config, the cross-thread `drain` benchmark, 1M signals):

| Metric | x86 variance | ARM variance | Determinism class |
|---|---|---|---|
| Wall-clock | 35–98 ms (~**3×**), 6 CPU models; same-model (EPYC 7763, n=41) still 1.28× | 28.79–57.37 ms (~**2×**), 99/100 Neoverse-N2 — same-model still 1.98× | noise |
| Allocations | **2,425,213 B in all 100 runs**; per-function `EventProducer::push` exactly 1,216,768 B | identical bytes | 100% deterministic |
| Callgrind icount (Ir) | ~**0.13%** | ~**0.065%** | near-deterministic, CPU-model-independent |

The inversion witnesses that bound every metric's role:

- **Allocations mislead:** naive `contains` dedup → HashSet dedup: allocations **+112.54%**, thread allocations +97.64%, timing **−98%** (deliberately exaggerated, but the proxy-inversion shape is real).
- **icount misleads:** `cache::cache_sequential` vs `cache::cache_random` — identical icount (20,971,545 avg, 0% delta), wall-clock 5.06 ms vs 133.04 ms (**+2529%**, ~26×). Regime: whenever performance depends on cache locality (data layout, pointer chasing, hash maps vs arrays). Remedy axis named: `--cache-sim=yes` (not yet exposed by hotpath).
- Criterion's own FAQ warns against cloud-CI timing; statistically significant timing deltas can be environment, not regression.

## Ranked candidate table

| # | Candidate | Route | Grade | Notes |
|---|---|---|---|---|
| 1 | `instruction-count-cache-blindness` | rust_perf / measurement-honesty (suggestion-only, NotBenchable-by-domain) | **B+ (batch lead)** | A deterministic instruction-count metric carries the cache-locality blindness as its ANTI-REGIME: identical icount with 26× wall-clock divergence; regime = cache-locality-dependent work; remedy axis = cache simulation; Valgrind thread serialization rides as a second regime condition (multithreaded benchmarks behave differently under Callgrind). UPSTREAM: n/a (measurement law; blog-measured witness). Dedup: MISS — the kernel_opt `dry-pass-icache-warm-before-real` / `launch-time-code-as-data-l2-warm` family owns the icache-WARMING axis, not the measurement-blindness critique. |
| 2 | `async-wrapper-future-size-exponential-nesting` | rust_perf / layout (async family) | **B+ (GOAT lead, BENCHABLE)** | A measured-async-fn wrapper stored the wrapped future 2–3× per nesting level → exponential future growth; hand-written `MeasuredFuture` keeps size linear; −98% allocations / −97% icount at 8 nesting levels (PR #801). Generic law: audit generated-future size per composition level; a wrapper that multiplies inner-future storage per level composes exponentially. UPSTREAM: **present** — hotpath's own landed PR #801 (decision content only, never a PR). Dedup: adjacent HIT `async-future-size` (entries_02) owns LARGE-LOCALS shrink (scoped blocks) — different fix-space (hand-written Future impl vs scoped blocks); MANDATORY in-rule adjudication both directions at authoring. |
| 3 | `wall-clock-ungated-shared-ci-noise-floor` | rust_perf / measurement-honesty (suggestion-only) | B | On shared/unpinned runners wall-clock is EXCLUDED from regression judgment (judged=false) and retained as report context only; gate thresholds are derived from a MEASURED no-code-change noise floor (the empty-PR methodology: 100 rebuilds, per-arch, per-CPU-model); thread-scheduling regime — cross-thread benchmarks get looser thresholds, single-threaded stricter. Adjacent: `benchmark-spread-above-threshold-reports-unstable` (entries_70) owns the UNSTABLE *verdict*; this owns the judgment-ROLE exclusion — adjudicate both directions. UPSTREAM: n/a. Home positive instance: the fleet's no-CI-wall-clock-gate posture (audited file+line this pass). |
| 4 | `alloc-proxy-inversion-work-ratio-caveat` | rust_perf / measurement-honesty | **B− face-or-rule gate** | An alloc-axis win carries the work-ratio caveat — the dedup inversion (alloc +112%, time −98%) proves the deterministic proxy inverts when the algorithm changes work per allocation. Adjacent: the corpus's own alloc axis + the ParetoDominated-to-suggestion-only demotion posture (entries_06 `stack-read-fast-path…`, entries_03 `narrow-counter-overflow-promotion`) already encode "allocs alone never win" — this row may be a FACE on that posture, not a new rule. Decided at authoring. |
| 5 | `ci-gate-policy-shape` (composite) | rust_perf / measurement-honesty | **B− composite-or-fold** | The gating construct: absolute budgets (exact call counts, alloc caps) + relative thresholds (alloc 2% judged, icount 3% reported) + `min_percent_total = 0.5` sub-share noise floor + timing-reported-never-judged + Criterion-shared-CI unreliability (fixed-iteration deterministic mode as the alternative). The generic LAW lands; the vendor TOML specifics stay out. May fold into #3 as regime/policy text if the authoring pass finds it indivisible. |
| 6 | `cpu-model-per-run-metadata` | rust_perf / measurement-honesty | C+ | GitHub assigns different CPU models per run (6 models / 100 runs); same-model does not cure noise; a benchmark report records the CPU model per run. Home instance: the fleet's box-state PROVENANCE discipline. Thin — DROP-OR-FACE gate at authoring. |
| — | DROP: ring-buffer chunk reuse (PR #720, −95% alloc churn) | — | DROP | `bounded-batch-buffer-pool` (entries_06) owns the bounded-recycle axis; R234's C3 lane already declined hotpath's drain internals as source-internals; PR #720 is their landed decision. Citation posture only. |
| — | DROP: same-runner baseline+PR pairing | — | DROP (watch) | Author-flagged unproven ("early results look encouraging… challenges around CI execution time and securely handling forked pull requests"); matched-control family adjacent. Watch trigger: the promised bare-metal-runners follow-up post. |
| — | DROP: hw-counter-fallback / cross-arch-comparison | — | DROP (fold) | Environment-conditional laws with no bounded fix-space; fold as regime context into #3/#5. |

## GOAT candidate — #2, all four criteria pass

1. **Benchable:** the rust_perf alloc axis (exact counting allocator) + `size_of` composition assertions host this directly.
2. **Exact semantics:** future-size and alloc counts are exact integers — G1 trivially bit-exact.
3. **Magnitude story:** exponential (2–3× per level) vs linear; −98% allocs measured at 8 levels.
4. **Deterministic:** same composition, same size, same allocs every run.

**Pre-registered caveat (the B361 lesson):** if the validator's `&Input` harness cannot express a composed N-level future held across a poll, spec stays None and a standalone preflight is the instrument of record (the `map-inplace-same-size-transmute` precedent, entries_77). The measurement-honesty rows fail criterion 1 for a DOMAIN reason (the validator hosts fix-vs-input measurement, not gate-policy) → suggestion-only NotBenchable-by-domain annotations, first-class (Batch 136/146/68 precedent); the domain limitation never demotes the source.

## Dedup evidence (both vocabularies, live at HEAD `2a1a7875b`)

- MISS: icount/callgrind/valgrind/instruction-count/pointer-chasing (zero hits anywhere; GPU icache family = different axis); judged/wall-clock-exclusion; min_percent/noise-floor-share; budget+threshold CI policy; hw-counter/pmu; cpu-model metadata; criterion-fixed-iteration.
- PARTIAL/HIT with adjudications carried above: alloc-proxy (the corpus's alloc axis + demotion posture — inverse counter-example unowned); async-future-size (large-locals ≠ wrapper-composition); bounded-batch-buffer-pool (owns recycle → #7 DROP); matched-control family (adjacent to same-runner pairing → watch row).

## Lane-intel axis — CONFIRMATION-ONLY (evidence-audited this pass)

- League scoreboard: 8 failing/open-short rows (4090 decode 0.9697× sub-parity era; M3 prefill 0.572×; m5ultra decode 0.888×/0.863×/0.842×; qwen3.8 chat 0.36×) — all GPU-kernel lanes with named levers; a CI-metrics blog moves none.
- riir-ai `.issues/` + `.plans/` + `.docs/09_performance/`: **zero hits** for callgrind / valgrind / icount / runner-variance / deterministic-metric / Criterion-noise. No sources-shelf row points at hotpath or benchmark methodology (the shelf is ANE-domain only).
- **CI wall-clock audit: NO workspace CI lane judges wall-clock on GitHub-hosted runners** — confirmed file+line (katgpt-rs workflows carry only billing-minute comments and doc-label floors; `x86_64_execution_matrix.sh` is workstation-only; `budget_gate.sh` is pinned workstation-only at `rust.yml` + its own header + `budget_lane.sh` "no GitHub Actions"; reflex/refine hosted lanes carry wedge-timeouts and slow-test warnings, both self-declared non-perf-gates). The blog's (e) policy is the fleet's standing posture, independently derived.
- The blog's (d) icount-blindness finding is an argument **against** icount as a league metric (the league's variance answer — paired same-window A/B + verified-idle + PROVENANCE + bit-identity pins — is stricter than the blog's), not for one.

## Product-coverage axis — the follow-up pool

The measurement-honesty family is small (6 canonical rules) and the ecosystem's CI-benchmarking toolset is unmined as a family. This blog seeds its CI-gating corner. Pool for the next passes (accumulation clause: pool sources read before the batch window fold into the same batch):

- Criterion FAQ (the cloud-CI warning) + the Criterion book (benchmark-authoring chapter)
- gungraun (formerly iai-callgrind) docs — instruction-count benchmarking harness
- CodSpeed docs — instruction-count CI product
- Bencher docs — continuous regression tracking
- The promised follow-up post on bare-metal GitHub runners (watch trigger — the author's spoiler says it moves stability)
- A FIFTH hotpath pass when their Callgrind integration matures (they promise `--cache-sim` exposure + more simulation metrics — that lands exactly the #1 remedy axis)

## Honest caveats

- **Vendor-affiliated:** the article promotes hotpath-cloud (the policies section is product documentation); the ~2k-report dataset is self-reported through the author's own tool — not independently verifiable.
- The 26× cache demonstration is synthetic and AI-assisted ("with the help of my AI friends") — a constructed boundary demonstration, not a production incident.
- **TOML-vs-prose tension:** the sample policy sets `[functions.icount] judged = true` while the prose says icount changes "are reported but don't fail with this configuration" — transcribe the prose as operative; re-check against hotpath's live docs at mine time.
- The ARM Callgrind misattribution caveat rides a "highly experimental" Valgrind fork — provenance thin, unverified.
- Freya cross-validation is second-hand (their dashboard), raw fixed-iteration timings (not Criterion statistical estimates).
- GitHub-runner environment ≠ the fleet's gate environment: every law lands as decision content, never as an adoption recommendation — the confirmation-only lane audit above is the boundary.
- The determinism-ladder magnitudes are regime-relative (the `drain` benchmark is cross-thread, 1M signals; synthetic single-threaded benchmarks run cleaner but represent less).

## Queue

Verdict row appended to `.distill/001_mining_queue_snapshot.md` (same commit). Plan: `.plans/374_hotpath_ci_blog_mining.md` — batch slot behind Batch 362 (ThreeRT, claimed live), nominal Batch 363. Plan-first before any corpus edit.
