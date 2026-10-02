# Bench 086 — Rule-corpus Cross-OOO axis (Issue 098 T1–T4)

> **Status:** MEASURED + WIRED (2026-09-12) — T3 verdict: **PASS** (3 absorption candidates at cross < 0.6); axis wired into `frontier-report` (additive section) + `score_history.jsonl` (`cross_ooo` field, the `ood` additive law).
> **Source:** katgpt-rs Research 552 (bioRxiv 2026.09.05.748439 PoC, Bench 758 GOAT) + riir-ai Research 378; primitive `katgpt_core::ooo_audit` behind `direction_bank_audit`.
> **Driver:** `cargo run --example cross_ooo_axis --features "score_bench kernel_opt sec docker" --release`

## What was measured

The absorption/splitting audit (Research 552 §F2) aimed at THIS repo's own rule
bank: rules = units, each rule's oracle-labeled triggering spans = its MEI set
(ALL of them — a binary oracle has no graded activation, so the paper's top-K
cut does not apply; the bank is RAGGED and the free `ooo_score`/`cross_ooo`
kernels compose directly into a hand-built `BankAudit` for `greedy_curate`).

## Population scan (the K≥2 precondition) — the honest per-domain verdict

| domain | entries | rules | auditable (≥2 spans) | max spans/rule |
|---|---|---|---|---|
| clippy_lints (corpus) | 49 | 49 | **0** | 1 |
| rust_perf (corpus) | 126 | 126 | **0** | 1 |
| kernel_opt (corpus) | 607 | 607 | **0** | 1 |
| sec (corpus) | 15 | 15 | **0** | 1 |
| docker (corpus) | 31 | 31 | **0** | 1 |

**Every non-clippy corpus is exactly one entry per rule** — Cross-OOO
own-similarity is undefined at K=1, so the axis is UNMEASURABLE there. That is
a corpus-shape finding, not an instrument failure: the corpora were built as
one-fixture-per-rule distillations. The clippy domain is auditable only
through its oracle-labeled FIXTURE families (the corpus itself is 49/49):

| family | spans | labels |
|---|---|---|
| `v1_frozen` | 33 | frozen real-clippy (mined, provenance-recorded) |
| `gen6_real_repo` | 17 | freeze-e2e real-clippy (17/17 verified) |
| `retrieval_fixture` (`CLIPPY_SPANS`) | 50 | one-span-per-lint verified canon |
| **pool** | **100** | 42 rules, 17 auditable (≥2 spans), 25 unauditable (1 span) |

(v2–v5/v7/v8 generation ladders EXCLUDED — corpus-derived composites,
in-distribution by construction.)

## The metric evolution (the Bench 758 metric-granularity law, observed live)

**Control — plain TF-cosine over `RustOpBigramTokenizer` tokens: REJECTED.**
It produced 6 absorption candidates dominated by micro-span punctuation rules
(`deref_addrof↔double_parens 0.500`, `deref_addrof↔needless_return 0.500`,
`double_parens↔needless_bool 0.500`, …) with scores pinned at exactly 0.500 —
the tie signature. Root cause: plain TF lets the generic scaffold
(`fn f x u32 op:{`) swamp the rare op signature (`op:&*`, `op:((`), so
micro-span MEIs tie every own-vs-other comparison. The `op_tokenizer` module's
own design doc names the fix: "ubiquitous runs earn near-zero IDF and
contribute nothing; rare ones earn high IDF and do the separating" — the
tokenizer was DESIGNED for IDF scoring, not plain TF.

**Final — TF-IDF cosine** (`idf(t) = ln(N/df(t))` over the pool): generic
scaffold terms read exactly 0; the rare op runs do the separating.

## The measurement (TF-IDF, paper-default cutoffs 0.8/0.8)

`greedy_curate`: **unique_feature_count = 12 of 17** audited (3 pruned
redundant, 2 dropped uninterpretable: `double_parens` ooo 0.763,
`single_match_else` ooo 0.418).

### Absorption candidates (cross < 0.6, ascending)

| pair | cross | read |
|---|---|---|
| `manual_map ↔ needless_return` | **0.462** | real entanglement — the gen-8 closure-birth mechanism (`manual_map`'s arm→closure splice produces needless-return-shaped code); both members interpretable (ooo 0.98 / 0.91), `manual_map` curation-pruned |
| `manual_map ↔ single_match_else` | **0.500** | same match-arm family cluster |
| `single_match ↔ single_match_else` | **0.500** | **the calibration anchor** — clippy's own split of one lint into two; genuinely indistinguishable at op-bigram granularity. The axis detecting THIS pair is the defend-wrong validation |

### Top-20 most-redundant pairs (threshold-agnostic, ascending)

| # | pair | cross |
|---|---|---|
| 1 | manual_map ↔ needless_return | 0.462 |
| 2 | manual_map ↔ single_match_else | 0.500 |
| 3 | single_match ↔ single_match_else | 0.500 |
| 4 | double_parens ↔ needless_range_loop | 0.600 |
| 5 | map_unwrap_or ↔ needless_return | 0.684 |
| 6 | double_parens ↔ identity_op | 0.700 |
| 7 | map_unwrap_or ↔ single_match_else | 0.700 |
| 8 | needless_range_loop ↔ semicolon_if_nothing_returned | 0.750 |
| 9 | needless_range_loop ↔ single_match_else | 0.750 |
| 10 | redundant_static_lifetimes ↔ semicolon_if_nothing_returned | 0.750 |
| 11 | redundant_static_lifetimes ↔ uninlined_format_args | 0.750 |
| 12 | single_match_else ↔ uninlined_format_args | 0.750 |
| 13 | needless_range_loop ↔ needless_return | 0.769 |
| 14 | needless_return ↔ single_match_else | 0.769 |
| 15 | match_bool ↔ needless_return | 0.789 |
| 16 | collapsible_if ↔ needless_return | 0.800 |
| 17 | comparison_to_empty ↔ double_parens | 0.800 |
| 18 | deref_addrof ↔ filter_next | 0.800 |
| 19 | deref_addrof ↔ manual_map | 0.800 |
| 20 | deref_addrof ↔ single_match | 0.800 |

### Per-rule rows (most-entangled first)

| rule | spans | ooo | min_cross | partner |
|---|---|---|---|---|
| manual_map | 2 | 0.980 | 0.462 | needless_return |
| needless_return | 11 | 0.910 | 0.462 | manual_map |
| single_match | 2 | 1.000 | 0.500 | single_match_else |
| single_match_else | 2 | 0.418 | 0.500 | manual_map |
| double_parens | 3 | 0.763 | 0.600 | needless_range_loop |
| needless_range_loop | 2 | 0.969 | 0.600 | double_parens |
| map_unwrap_or | 8 | 1.000 | 0.684 | needless_return |
| identity_op | 7 | 0.989 | 0.700 | double_parens |
| redundant_static_lifetimes | 2 | 0.990 | 0.750 | semicolon_if_nothing_returned |
| semicolon_if_nothing_returned | 2 | 0.898 | 0.750 | needless_range_loop |
| uninlined_format_args | 6 | 1.000 | 0.750 | redundant_static_lifetimes |
| match_bool | 8 | 0.989 | 0.789 | needless_return |
| collapsible_if | 4 | 0.948 | 0.800 | needless_return |
| comparison_to_empty | 2 | 1.000 | 0.800 | double_parens |
| deref_addrof | 3 | 0.928 | 0.800 | filter_next |
| filter_next | 2 | 0.990 | 0.800 | deref_addrof |
| needless_bool | 3 | 0.990 | 0.800 | single_match_else |

## Verdict (T3): PASS — the axis is wired onward

3 absorption candidates at cross < 0.6, headlined by the known-true
`single_match ↔ single_match_else` split (defend-wrong anchor, pinned by
`t2_single_match_split_pair_is_the_calibration_anchor`). Wiring landed:

- **`frontier-report`** gains the additive cross-OOO section (bin-side under
  `score_bench`, one line per audited frontier rule: `min_cross` + partner +
  ABSORPTION-CANDIDATE flag; ungated builds print exactly what they printed
  before — `frontier_report::render` is ungated and stays untouched).
- **`score_history.jsonl`** rows gain `cross_ooo` (the `ood`/`strata` additive
  law: `serde(default)` + skip-when-absent; the Warm-tier `ScoreRunRow` mirror
  drops it — the four-exception schema-identity contract, updated in
  `traj_store.rs`).
- **Console**: `--score-bench` prints the axis line (unique_feature_count,
  pool, candidate count) beside the rule-outcomes line, sharing one cached
  measurement with the row (`cross_ooo_axis()`, the `ood_axis` precedent).
- **Mining queue** (T4): the absorption candidates land in
  `.distill/001_mining_queue_snapshot.md` as merge/consolidation candidates —
  single_match family consolidation + the manual_map/needless_return born-lint
  cluster already documented in the gen-8 shaped-verdict work.

## Test gates

7 lib tests in `score_bench::cross_ooo`: pool determinism + family coverage;
sim symmetry/diagonal + IDF-zero law (ubiquitous-token contributions vanish;
identical text → 1.0 within f32 epsilon) + the 2-span identical-text corner
(every term df=N → zero vectors → 0.0); planted-duplicate detection (exact
clones → cross exactly 0.0; Fig-4a clone-pair semantics — ooo collapses to
0.5, both members dropped uninterpretable, ONE unique feature from the pair);
real-pool bit-determinism; the single_match calibration anchor.

## Honest caveats

- The audit measures rule DISTINGUISHABILITY at op-bigram granularity, not
  semantic identity: `manual_map ↔ needless_return` 0.462 says their trigger
  spans are mutually indistinguishable AT THIS METRIC — the gen-8 closure-birth
  record independently corroborates the entanglement.
- 25 of 42 rules are unauditable at K≥2 (single span) — reported in the axis
  header, never folded into the audited count.
- The non-clippy domains need multi-span corpora before the axis can say
  anything; that is a corpus-growth decision, not an instrument gap.
- f32 arithmetic: identical-text off-diagonal reads 0.9999999 (the diagonal is
  exactly 1.0 by construction) — tests pin epsilon, not bit-identity, for that
  case.
