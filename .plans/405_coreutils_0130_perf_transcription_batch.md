# Plan 405 — uutils/coreutils 0.13.0 perf transcription batch (nominal Batch 406)

**Status:** STAGED — plan-first, zero corpus edits until the cargo window opens (verdict: Research 440, YES B− Claude-adjudicated round 3 AGREE, session f1c2a64e; rounds 1–2 REVISE; R2 B+ until its materialization-cost cell measures)
**Claimed:** zed-m3-coreutils-distill@m3, 1795417515, expires 2026-11-25T07:05Z

## Source

- uutils/coreutils tag `0.13.0` = `07b76df463390fd56856725201ae52df5df0a063` (2026-11-19), MIT — transcription GO, outbound-PR lane OPEN.
- Base `0.12.0` = `5ab52189443867d56e8beb8984f52efd18fd7682`. Range: 1324 squash-merged commits, 901 touching `src/`.
- Verdict record: `.research/440_coreutils_0130_perf_distill.md` (the ranked table, the dedup evidence, the drop rows, the caveats).

## Goal

Land the 0.13.0 perf work's new mechanisms — one composition refinement of an owned density law (R1), one unowned-adjacent regime (R2), four decision/alloc candidates — as `rust_perf` rules with ORIGINAL fixtures, never copied source. Target: 2–4 rules + 6–8 count-neutral faces from the 7 staged candidates (R1–R7) and 14 faces (F1–F14, tickable per owner) of Research 440, plus the T2b corpus-name fix.

## Pre-registered proof paths (decided BEFORE the first corpus edit)

| Candidate | Gate | Bars (pre-registered) | Falsifier |
|---|---|---|---|
| R1 `match-density-tiered-filter-sparse-run-copy-dense-compaction` | G1 + G2 + G4 on the perf-bench Pareto validator | G1 byte-identical output vs the naive per-byte table filter over all densities {0, 1/1024, 1/128, 1/64, 1/8, 1/2, 7/8} × seeds; G2 **three baselines + the tiered arm = four arms** — the naive per-byte table filter (the G1 oracle), memchr-run-copy ALONE, branchless compaction ALONE, and the tiered form — in one sentence: at every density and every size [1K, 16K, 256K] the tiered form is never slower than the best single arm, beats each single arm inside that arm's own regime, and lands ParetoDominant-or-better at ≥2 of the 3 sizes; all four arms alloc-free | any size below the best single arm × 0.95, or the mid-density regime showing no measurable branch effect, ⇒ the fixture proved SIMD not tiering — demote to a face on `prefilter-skip-density-demotion` in the landing commit (the ungoat law) and record the measured negative |
| R2 `sort-key-lazy-when-comparisons-per-item-low` | G1 + G2 | G1 order-identical to the cached-key arm (the key is order-preserving; ties first-index); G2 **primary axis = key MATERIALIZATION cost** (allocations + bytes of key material built vs bytes the comparisons inspect) on a dear-key/early-exit fixture, key-eval COUNT a secondary column only — arithmetic forbids a count bar (a lazy k-way heap merge at k≥2 does ~n·log₂k comparisons × 2 key reads ≥ n, so it fires for the wrong reason), k=1 exactly zero comparisons, wall-clock never slower than 0.95× at k = 1 | if the lazy compare cannot beat the cached arm's materialization cost at ANY k with a dear key, the regime narrows to k = 1 and the rule is re-worded before landing — never landed with a wider claim than measured |
| R4 `explicit-stack-dfs-ancestor-set-exit-marker` | G1 + alloc axis | G1 the same visit order and the same cycle verdicts; peak set size = depth, not N, on a chain-shaped tree of depth ≥ 1024 | — |
| R5 `in-loop-interior-remove-single-pass-fold` | G1 + G2 | G1 identical merged ranges incl. touching, nested, and disjoint inputs; G2 ParetoDominant on all-adjacent input at [1K, 16K, 256K] | a Tie on disjoint-heavy input is EXPECTED and recorded as the anti-regime, not a failure |
| R3 | decision-only | `NotBenchable`, `bench_spec: None` (the domain-unbenchable annotation class — kernel link resolution) | — |
| R6 | benchable-with-fragility | time the hot per-byte loop with the opaque call inlined vs out of line; record the rustc version beside the numbers | a win that does not reproduce across two rustc versions ⇒ face on `match-branch-cold-confirm` |
| R7 | benchable by counting metadata calls | one lstat vs two stats per path on a directory-walk fixture; the metadata-call count is the primary axis | — |

Every fixture is a four-arm shape: buggy / fixed / all-safe / tampered (the standing fixture discipline), standalone-compiled AND executed BEFORE the corpus entry.

## Tasks

- [ ] T0 — re-clone at the pin (`git clone --filter=blob:none --depth 1 --branch 0.13.0`), verify HEAD == `07b76df463…`, re-verify every quoted line of Research 440 by `grep -n`; record any drift. Read the remaining unverified bodies (`Line::create`, `map_err_context` — `SYMLINK_FOLLOW_LIMIT` = 40 already verified at `fs.rs:45`) and confirm or strike the `od` per-field `String` residue.
- [ ] T1 — dedup re-verification at the live corpus (the 16-row table of Research 440, re-run under both vocabularies); write the adjudications into each rule: R1 as a COMPOSITION vs `prefilter-skip-density-demotion` (its feedback latch owns the density CHOICE — this rule owns the exact-count tier, the per-block survivor-rate switch, the zero-hit borrow) + `branchless-conditional` (cite, not re-own — its own header already records the selectivity effect; R1 adds the per-block switch and the carried guess, never the regime) + `byteset-cardinality-dispatch` / `byte-flag-table-masked-run-scan`; R2 vs `sort_by_cached_key` (zero corpus owners — the genuine pro-precompute counterpart; do NOT adjudicate against `sort-key-cache-precompute-out-of-lock`, which owns lock+clone in a comparator, or the misnamed `sort-by-key`); R4 vs `recursive-input-walk-explicit-stack-indices-not-copies`; R5 vs `vec-remove-front-to-vecdeque` / `swap-remove`; R6 vs `match-branch-cold-confirm` (opposite regime: always-taken); R7 standalone (zero owners, verified).
- [ ] T2 — author entries in a new `src/domains/rust_perf/corpus/entries_97.rs` (slot re-derived at write time): R1 (simd or complexity — decide by where the matcher fires) and R2 (complexity or caching) as the leads; R3/R4/R5/R6/R7 per their pre-registered gates.
- [ ] T2b — fix the corpus's `sort-by-key` rule (entries_02.rs) in the same landing: its name and comment claim std's `sort_by_key` "extracts each key exactly once (N calls) then sorts the cached keys" — false for std (the key fn runs per comparison; caching is `sort_by_cached_key`'s job). Re-word to the true mechanism (a clearer key extraction vs a closure comparator — the rule's own Bench-016 notes already measured 4/4 Tie), or re-target the rule at `sort_by` → `sort_by_cached_key` for dear keys; count pins move accordingly. Found incidentally by this verdict (Research 440); this plan is the owning lane.
- [ ] T3 — fixtures: `src/domains/rust_perf/corpus/fixtures/b406_fixtures.rs` + the shape arms in `corpus/tests.rs`; every candidate dropped at authoring is recorded in the entries-file header with its reason.
- [ ] T4 — the faces F1–F14 as count-neutral notes on their owning rules (compose, never merge); each face cites its commit sha.
- [ ] T5 — standing gates at an isolated `CARGO_TARGET_DIR=/tmp/b406`: default `--lib`, `rust_perf_bench` posture, clippy `-D warnings` at default / no-default / all-features, the retrieval floor gate (both lanes, 30/0 — no re-pin), `scripts/fmt_gate.py`, the batch-406 needles.
- [ ] T6 — count pins one-landing: `scripts/pins_manifest.py --write`, the count web (AGENTS §Role, README, `.docs/11_domains/rust_perf.md`, `health_views`, the kernel-side rust_perf witness, the `corpus/mod.rs` head). Freeze/score-bench owed only if `clippy_lints` moves (asserted zero here).
- [ ] T7 — teardown: `.raw/coreutils*` and `/tmp/b406` removed in the same session; `df -h /` before/after in the landing row.
- [ ] T8 — landing: the queue-snapshot BATCH LANDED row, Research 440's status line → `RECORD`-landed, this status line, commit + push on `develop`, the maintained binary rebuild (rust_perf is in the shipped release set).
- [-] The post-release delta (45 commits on `main` after the tag) — deferred to a return visit; not this batch's scope.

## Why now / why not lower

The leads carry upstream-verbatim reasoning and verified corpus gaps (R2's counterpart `sort_by_cached_key` has zero owners); the source is MIT (the outbound lane stays open for the two `UPSTREAM: absent` residues); and the batch needs no GPU and no sibling checkout beyond `katgpt-rs`. It slots behind any in-flight kernel_opt batch — zero cargo contention by construction.
