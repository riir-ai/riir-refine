# clippy-bench — the frozen real-code healing bench + score curve

> **Plan 014.** The bench that can falsify "we're getting better": a FROZEN
> fixture set healed by the production `--fix` path, scored against real
> `cargo clippy` before/after, with an append-only score history rendered
> as an SVG curve. The denominator never grows with the corpus — the score
> can only move by healing better (or worse, which is the point).

## Why (the gap it closes)

| prior number | why it couldn't answer the question |
|---|---|
| retrieval precision (Bench 022→032) | authored fixtures coupled to the corpus; kernel rows are self-variant (circular) |
| corpus rule count | coverage metric — grows by construction, cannot fall |
| Bench 009's 0/10 held-out | measures the L1 verbatim cache tier, not the proposer path |
| the sweep campaign | real evidence, but episodic — no unified score, no time series |

## The fixture set (frozen v1, 34 fixtures)

| provenance kind | count | note |
|---|---|---|
| `real:` mined from pre-sweep sibling commits | 7 | std-only fns that compile standalone; the bare-stub filter drops crate-internal spans by construction |
| `authored-composite:` T3.5.5 spans | 27 | realistic MULTI-lint spans (2+ co-applying lints), oracle-labeled at freeze — NOT the degenerate minimal set |
| `real:` clean (decline axis) | included in the 7 | correct behavior = propose NOTHING |

Mining: `examples/score_bench_mine.rs` (`git show <sha>:<path>`, top-level
fns, the oracle's `compiles` flag is the std-only filter). Freeze labels at
mine time; every run re-derives + warns loudly on toolchain drift.

## Axes

| axis | definition |
|---|---|
| `heal_rate` | Σ resolved vocabulary lints / Σ vocabulary lints before |
| `created_rate` | Σ newly-created clippy lints (ANY) / Σ vocab-before — the false-positive axis |
| `parse_safety` | healed output still compiles / edited fixtures |
| `decline_correctness` | clean fixtures untouched (declining is a correct answer) |
| `shaped_rate` (gen 8+, Issue 089 T7) | the half-win axis: resolved findings credited at `verdict × resolved` — a fixture whose heal clears its target AND leaves new diagnostics behind earns 0.5 per finding (`shaped_rate ≤ heal_rate` always) |

Plus the **miss map**: which lints persist where, most-frequent first —
the diagnostic for WHERE healing still fails. Instrumentation axes riding
every row (the additive serde law): the **OOD entropy-gap axis** (issue 077),
the **provenance strata** (Issue 089 T6 — heal rate split by repo/authored/
construction class), the **shaped record** (Issue 089 T7 — n_shaped,
shaped_rate, per-rule half_counts), and the **per-rule outcome rows**
(Issue 089 T8 — fixtures/resolved/persist/broken/created/edits/shaped per
rule, single-target attribution only, the T7 law; `persist` vs `broken` is
T4's triage fork, `edits` is the log-length proxy the R(n) tie-break
consumes). Companion file (T4's evidence clock): every
`--frontier-report` run appends a state-change-deduped band-mass snapshot
to `.benchmarks/frontier_snapshots.jsonl` (generation = the frontier, max
not latest lane; ≥3 generations of rows are T4's GO evidence).

## Score history (the curve so far)

| run | sha | heal | created | safety | decline | note |
|---|---|---|---|---|---|---|
| #1 2026-08-19 | `17e3d8b` | 84.6% | 3 (5.8%) | 92.9% | 100% | baseline — caught a REAL proposer bug on its first execution (Issue 032) |
| #2 2026-08-19 | `86ba7f1` | **90.4%** | 3 (5.8%) | **100%** | 100% | Issue 032 fixed (atomic whole-loop edit) — the first step the curve shows: +5.8pp heal, safety back to 100 |
| #3 2026-08-19 | `c8d0d34` | **98.1%** | **1 (1.9%)** | 100% | 100% | the run-#2 miss map triaged to root causes — 5 proposer extensions (all probe-verified vs clippy 1.97 first) + a static-str trait-impl soundness fix |

Run #1's two parse-safety failures were exactly the Issue 032 fixtures
(`needless_range_loop` × `identity_op`/`double_parens` same-pass composition
half-applied the rewrite → dangling loop variable). The remaining miss map
(needless_range_loop, double_parens, needless_return,
redundant_pattern_matching, useless_format — one each) is the honest open
heal surface; `created_rate` 3 is the separate false-positive class.

### Run #3 — what the miss-map triage found (2026-08-19)

Every run-#2 miss + created was probed against real clippy BEFORE any code
changed (the per-fixture detail ships as the `#[ignore]`d
`per_fixture_diagnostics` test — the standing triage workflow):

| miss / created | root cause | outcome |
|---|---|---|
| `needless_return` persist | proposer skipped bare `return;` — clippy fires (probe: suggestion deletes the stmt) | **fixed** (deleted via `delete_stmt_edit`) |
| `double_parens` persist | `push((v + 1))` single-wrap call arg — clippy fires; the old "tuple break" note confused token shape with AST shape (`(a,b)` is `Expr::Tuple`, never `Paren`) | **fixed** (arg wrap stripped) |
| `redundant_pattern_matching` persist | proposer lacked the 2-arm bool-literal match shape (`Some(_) => true, None => false`) | **fixed** (→ `e.is_some()`, either arm order + wildcard opposite) |
| `needless_range_loop` persist | `names[i]` inside `format!` args = opaque token stream → scan saw zero uses → refused | **fixed** (macro-arg descent: format-family args parse as exprs with file-relative spans; `{i}` named captures refuse; other macros mentioning `i` refuse; `__elem` collision check walks macro tokens — a pre-existing shadowing hole) |
| `missing_const_for_fn` created ×2 | our `static-str` perf rewrite (beyond clippy's own fixes) made literal-returning fns const-able | **fixed** (`static-str` emits `const fn` — additive, probe-verified clean; also fixed a latent soundness hole: trait-impl fn signatures must match the trait, the old walker fired where the rewrite can never compile) |
| `single_option_map` created | NURSERY design-opinion lint ("fn that only maps over argument") — fires on `manual_map`'s OUTPUT, which is exactly what clippy itself suggests | **documented benign** — `cargo clippy --fix` produces the same cascade; not fixable without refusing correct fixes |
| `useless_format` persist (remains) | the String-typed inlined-named form `format!("{elem}")` still fires (probe) — clippy's own fix (`elem.to_string()`) then fires `implicit_clone` on `&String` (probe) — the true terminal `elem.clone()` needs the arg's TYPE | **documented boundary** — firing would trade the persist for an `implicit_clone` created (bench-symmetric +1.9pp each way); `created` is the trust axis, so the persist stays. The parse-only frontier: clippy's own `--fix` converges only because `implicit_clone` is ALSO a clippy fix |

## Generations (the M5 ratchet — `--generation <N>`)

A new generation is admitted only if it measures HARDER than v1 (gen 8:
only if the run measures ≥1 shaped verdict — its targets all clear by
construction). Records: `HISTORY.md` per generation.

| gen | ladder | admission |
|---|---|---|
| 1 | the frozen v1 real-code set (default) | the baseline |
| 2 | construction k-ladder over corpus atoms (issue 072 T5) | REFUSED first-freeze (100% ≥ v1 — not harder); the record stays as the drift gate |
| 3 | persist-stratified ladder (Bench 069 reopen #1) | admitted |
| 4 | rustc-errors k-ladder vs rustc ground truth (Plan 093) | admitted (cross-domain floor) |
| 5 | `rust_perf` vocabulary ladder, mechanical shape oracle (Plan 094) | admitted |
| 6 | real-repo shape ladder, spans carved from pre-heal sibling states (Plan 095) | admitted |
| 7 | overlap-edit-chain ladder, manufactured drop-and-refire overlap, depth rungs 1–5 (Plan 096) | admitted |
| 8 | **shaped-verdict ladder** (Issue 089 T7): atoms whose heal clears the target AND leaves new diagnostics — the closure-birth mechanism (`manual_map`'s arm→closure splice fires born-closure lints OUTSIDE FIXABLE: `redundant_closure`, `map_clone`, `map_identity`+`single_option_map`), so the fixpoint cannot dissolve them. 4 admitted / 3 excluded-on-record (dissolved). First run: shaped_rate 50.0% vs heal_rate 100.0%, 4/4 shaped, half_counts `manual_map`×3 (the "Beta (half-count)" feed; the Elo twin is the tie) | shape non-vacuity (n_shaped ≥ 1) |

## Run

```sh
cargo run --features score_bench --bin cargo-heal -- --score-bench   # run + append + render (gen 1)
cargo run --features score_bench --bin cargo-heal -- --score-bench --generation 8   # the shaped-verdict ladder
cargo run --features score_bench --bin cargo-heal -- --score-report  # re-render SVG only
```

History: `.benchmarks/score_history.jsonl` (append-only) · curve:
`.benchmarks/score_history.svg` (embedded in the repo README).

## Honest scope + Phase 3

Expect a STEP curve (root-cause fixes), not gradient descent — the
self-learning mechanisms measured negative and were killed (Plan 005
Phase 7 / Issue 026); improvement is engineering-iterative. Deferred:
perf/format axes (Phase 2), external crates.io source + per-fixture stub
manifests + cold/warm memory lines + statement-wrap mining for
crate-internal spans (Phase 3 — grows the `real:` fraction).

## See also

- [Plan 014](../../.plans/014_clippy_bench.md)
- Issue 032 (resolved + removed 2026-08-19, fix `86ba7f1`) — the partial-application composition bug baseline run #1 caught; the canonical record now lives in the score-history table above
- [`clippy_oracle.rs`](../../src/draft/clippy_oracle.rs) — the T3.5.5 real-clippy oracle
