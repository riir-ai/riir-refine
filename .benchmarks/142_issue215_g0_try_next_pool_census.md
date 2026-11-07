# Bench 142 — Issue 215 G0: the try-next pool census

**Status: G0 density PASS, mechanism-as-specified REFUTED — do NOT proceed to G1 as pre-registered; re-scope (see Verdict).** 2026-11-07, m3, Session: issue215-g0-subagent.

`PROVENANCE: commit 8d2755d9985c-dirty (dirty = the new example + its Cargo row only) · AC Power · load {9.90 7.53 7.05} · powermode 2 (high) · stride 1 (full population, no sample) · fresh ClippyVerifier work-root /tmp/i215_g0/verify · K 4 · wall 174 s`
Box was shared (sibling agents live; load ~8-10) — the per-compile latency is a shared-box figure.

## Instrument

`examples/try_next_pool_census.rs` (REPORT-ONLY, exit 0; `--features cf_workers_ai,score_bench`).
Pools are the REAL `HealPipeline::heal_with_pool` pools via `RegateHarness::classify_outcome`
(the arm-H / HARVEST birth, no second pipeline). Per candidate: `apply_edit(module, cand, false)`
(the harvest merge) then `ClippyVerifier::verify` → `compiled` (the `score_case` oracle).
A non-merging candidate = compile FAIL at zero measured cost (production `FixVerifier` would
splice + compile it — cost column under-counts, disclosed). `ran=false` = UNSEEN (0 occurred).
Modules whose BASELINE does not compile are excluded (3). Pipeline NOT memory-warmed between
cases (fresh-install shape).

```
cargo run --release --features cf_workers_ai,score_bench --example try_next_pool_census -- \
    --work-root /tmp/i215_g0/verify
```

## Populations

| population | cases | floor | note |
|---|---|---|---|
| v2edit labelled test chunks (`load_v2edit` 0..6, target = row `lints[0]`) | 408 (405 pooled) | 300 | full, no sample |
| score-bench v1 fixtures, healable (`expect_lints[0]`) | 29 | 20 | full |
| miss queue `.refine/misses/` (+ every `*/.refine/misses` in the workspace) | **0** | — | EMPTY on this box — UNSEEN, not zero-signal; also the per-CODE queue is the rustc-errors domain, not the clippy `heal_with_pool` path |
| cross-check: frozen Bench 114 `cases.jsonl` (1500 v2edit_all, memory-WARMED, 2026-10-11) | 1500 | — | read-only re-analysis of its per-candidate `compiled` flags |

## The three G0 numbers

| metric | v2edit (405) | score-bench (29) | Bench 114 frozen (1500) |
|---|---|---|---|
| (a) pool ≥ 2 | **405/405 = 1.000** (sizes 5:64, 6:341) | **29/29 = 1.000** | 1446/1500 = 0.964 (sizes mostly 2–4) |
| (b) compile outcomes DIFFER | **342/405 = 0.844** | **28/29 = 0.966** | 1413/1446 = 0.977 |
| (c) per compile (scaffold clippy) | mean 209 ms · p50 208 · p90 212 (n 357, tail support 37) | mean 202 · p50 202 · p90 206 (n 28, support 4) | — (not timed) |
| (c) pool try-all | mean 184 ms · p50 208 · p90 213 | mean 195 · p50 201 | — |
| (c) pool try-next unbounded | mean 177 ms · p50 207 · p90 212 | mean 195 · p50 201 | — |
| (c) pool try-next K=4 | mean 6 ms · p50 0 | mean 0 | — |

(c) is the scaffold-crate cost; production `FixVerifier` re-checks the real crate (seconds+), so
the absolute cost transfers only as a LOWER bound.

## What the contrast IS (decomposition — the load-bearing finding)

| | v2edit | score-bench |
|---|---|---|
| candidates = merge-fail + compile-fail + compiles | 2366 = **2009** + 3 + 354 | 173 = **145** + 0 + 28 |
| first (proposal-order) candidate fails | 397/405 | 29/29 |
| recoverable by try-next, unbounded | 334 | 28 |
| recoverable by try-next **within K = 4** | **2** | **0** |
| proposer member is LAST in pool | 341/405 | 28/29 |
| differing pools where ONLY the proposer member compiles | **329/342** | **28/28** |
| pick-success: oracle-any / **static proposer-first (1 compile)** | 342 / **341** | 28 / **28** |
| modelless served answer fails compile | 333/341 | 29/29 |

The signal is dense but STRUCTURAL, not a ranking-uncertainty signal: the drafter/memory members
are corpus-shaped snippets that do not merge into real modules (the Bench 113 98.3% merge-fail
finding, unchanged), and the Issue-161 proposer member — appended LAST by `collect_pool_members`
— is the one that compiles. Consequences:

1. **The pre-registered mechanism (try-next in proposal order, K ≤ 4) recovers 2/405 (0.5%)** —
   pools are now 5–6 wide, so the proposer member sits at index 4–5, past the budget.
2. **The unbounded control arm (b) recovers 334/405** but pays the full walk every time.
3. **A deterministic static reorder — proposer member first — reaches 341 of the 342
   oracle-reachable pools at exactly ONE compile**, dominating both. It needs no outcome harvest
   and no budget gate; it is a pool-ORDER fix, not a critic.
4. Contrast labels harvested from these pools would teach "proposer member compiles, snippets
   don't" — a fact already known by construction; the BPO-style harvest has ~no information here.

## Verdict

- **G0 (b) is NOT negligible → the pre-registered NEGATIVE close does not fire by the letter.**
- **But G0 refutes the mechanism as specified**: on the measured pools the gated K ≤ 4
  try-next is ~inert (0.5%) and a strictly cheaper deterministic competitor (proposer-first
  order, 1 compile) captures ≥ 99.7% of the oracle ceiling. G1 as pre-registered would compare
  three arms that all lose to an arm it does not contain.
- **Recommendation (needs owner/issue-author decision, NOT taken here):** re-scope Issue 215 —
  add arm (d) static proposer-first to G1 (or replace the lane with that ordering change under
  `heal_with_pool`'s bit-identical-`result` contract), and re-run this census on the pool the
  production seat lane actually uses before any try-next work.

## Disclosed gaps (low-confidence edges)

- **Production seat pool ≠ `heal_with_pool` pool.** `bin/heal.rs`'s seat lane (the
  `StoodByModelless` demote point the issue names) builds its pool from `matcher.rule_book`
  `fixed_code` across matched rules — no proposer member. That population was NOT measured here;
  its contrast density is UNSEEN.
- Miss queue population is empty on this box (UNSEEN).
- Pool width 5–6 here vs 2–4 in Bench 114 — pool composition moved since 2026-10-11 (corpus /
  memory seeding); not root-caused. The structural conclusion holds on BOTH populations.
- Merge-fail candidates carry 0 ms cost; per-compile figures are on a shared, loaded box.

## Follow-up: the PRODUCTION seat-lane pool (the gap above, now measured)

`PROVENANCE: commit 3698ff524bd7-dirty (dirty = this follow-up's factor + example edit only) · AC Power · load {10.38 10.16 8.61} · powermode 2 (high) · stride 1 · fresh work-root /tmp/i215_g0b/verify · K 4 · wall 669 s` (shared box, sibling agents live).

**Factor (behaviour unchanged):** the seat lane's two inline helpers in `bin/heal.rs` moved to the
lib so the census builds the IDENTICAL pool — `LatentMatcher::seat_rule_book_pool(&SpanSuggestion)`
(every matched rule's non-blank corpus `fixed_code`, `by_domain` × rank order, deduped first-seen)
and `draft::latent_matcher::line_span_bytes`. heal.rs now calls both; bodies moved verbatim.

**Census construction (mirrors heal.rs T5.2):** `matcher.suggest_file(module, _, top_k = 3)` →
the FIRST span whose pool ≥ `DEFAULT_MIN_POOL` (2) with a valid byte span (heal.rs consults ONE
span per file) → each candidate spliced over the span bytes (`replace_range`) → `ClippyVerifier`
compile. A splice equal to the module is a no-op FAIL (FixVerifier keeps 0 on an untouched file).
Same 405 + 29 cases (same baseline-compile exclusion).

| metric | seat pool × v2edit (405) | seat pool × score-bench (29) |
|---|---|---|
| (a) pool ≥ 2 | **405/405 = 1.000** (sizes 5:1, 6:404) | **29/29 = 1.000** (6:29) |
| (b) compile outcomes DIFFER | **95/405 = 0.235** | **5/29 = 0.172** |
| candidates: compile-fail / compiles | 1094 / 1335 (no merge step) | 41 / 133 |
| first candidate fails | 166/405 — of which **164 have NO compiling candidate** | 6/29 — all 6 have none |
| try-next recovery, unbounded | **2/405 (0.5%)** | **0/29** |
| try-next recovery, K = 4 | **2/405 (0.5%)** | **0/29** |
| first-compiling position (0-based) | 0: 239 · 1: 2 · none: 164 | 0: 23 · none: 6 |
| (c) per compile | mean 198 ms · p50 197 · p90 216 (n 2429, support 244) | mean 149 · p50 166 · p90 188 |
| (c) pool try-all | mean 1189 ms · p50 1178 · p90 1292 | mean 893 · p50 869 |
| (c) pool try-next unbounded | mean 609 ms · p50 207 · p90 1282 | mean 344 · p50 164 |
| (c) pool try-next K=4 | mean 442 ms · p50 207 · p90 852 | mean 266 · p50 164 |

The 95 differing pools are almost all "candidate 0 compiles, some later one doesn't" (93/95) —
contrast the try-next mechanism never consumes. Where the first pick fails, the pool is
all-fail 164/166 times: the next candidate does not rescue it. **On the production pool the
mechanism's ceiling is 2/405 recoverable picks**, at a K=4 cost of ~0.44 s/pool mean (scaffold —
the real-crate `FixVerifier` cost is larger).

### Follow-up verdict

**NEGATIVE on the production seat-lane pool** (the issue's own demote point): signal density is
moderate (0.17–0.24) but the recoverable fraction — the only part try-next converts — is ≤ 0.5%,
and the harvest's contrast is dominated by "first compiles, later fails" pairs. Combined with the
`heal_with_pool` result above (dense but structural, dominated by a static proposer-first order),
**neither measured pool supports the budget-gated try-next lane**. Close decision is the
coordinator's.

### Disclosed limits of the follow-up

- **"Compiles" ≠ correct.** Splicing a DIFFERENT rule's corpus `fixed_code` over a span compiles
  55% of the time (1335/2429) — many are destructive replacements the compile gate admits. This
  census measures the compile oracle the issue names, not fix quality; it biases (b) toward
  "compiles", i.e. toward FAVOURING try-next, and the verdict is negative anyway.
- **Firing condition not mirrored:** heal.rs consults the seat only on a modelless MISS (no
  surviving edit). The census measures every module; the miss-conditioned subpopulation was not
  isolated.
- Matcher posture: `LatentMatcher::from_enabled_domains()` (RegateHarness), no lease overlay, no
  active-set — the default posture, not every production posture. Span path is a fixed
  `census.rs` placeholder.
