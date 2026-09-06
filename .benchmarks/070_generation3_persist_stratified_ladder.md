# Bench 070 — generation 3: the persist-stratified ladder (Bench 069 reopen #1)

**Status:** RECORD — generation 3 frozen (72 verified specs), stratification gate PASS,
**ADMITTED** (gen-3 87.5% < v1 96.2%); the bench's second live series.

**Box:** 4090 / Windows, CPU-only (the oracle spawns cargo+clippy; no GPU anywhere).
**Freeze command:**
`cargo test --release --features score_bench --test generation_v3_freeze -- --ignored --test-threads=1 --nocapture`
**Bench command:**
`cargo run --release --features score_bench --bin cargo-heal -- --score-bench --generation 3`

## The design (pre-registered before the first freeze run)

Bench 069's ranked reopen list, candidate #1: build the difficulty axis from
the v1 persist map's actual failure shapes — the documented parse-only
divergence classes — instead of corpus atoms. The corpus-atom ladder
(generation 2) measured 100% at every k because shapes the corpus teaches
are shapes the proposers solve; generation 3 stratifies the atom table with
four AUTHORED persist-shape atoms where a vocabulary lint fires and the heal
lane DELIBERATELY stays silent.

Two gates were pre-registered at authoring time (in the source and in the
e2e, before any oracle ran):

1. **Stratification gate** (replaces v2's M4 monotone-k-decay): every
   persist atom resolves 0 on its own k=1 rung. The v2 decay gate does not
   carry: gen 3's difficulty axis is per-atom strata, not k-composition, and
   the prefix-window geometry makes strict monotonicity a sampling artifact.
   The per-k table is REPORTED, not asserted.
2. **The M5 admission gate** (CLI, generalized): a NEW generation's first
   row must measure strictly below v1's latest, else refused with nothing
   appended.

## The first freeze run FALSIFIED two of the five authored atoms

The stratification gate's first catch, same-day:

| authored atom | freeze verdict | cause | disposition |
|---|---|---|---|
| `persist_named_arg_rename` | **RESOLVED** (gate FAIL) | the P3 rename rewrite (`n = v` → template rewrite, probe-verified clippy 1.97) closed the named-arg-with-rename class AFTER the bench_337 record was written — the cited persist record was stale | replaced |
| `persist_nested_format` | bad atom (labels ≠ gold) | clippy 1.98 fires the NEW `format_in_format_args` on the nested-format shape instead of `uninlined_format_args` — the class moved OFF-vocabulary (labels move between toolchains, the oracle's own caveat) | dropped |
| `persist_comment_arm` (first shape) | bad atom | clippy fires `manual_unwrap_or_default` (a more specific vocabulary lint) on the Some/None-default shape and the healer SOLVES it (`o.unwrap_or(0)`); single_match_else never fires there | reshaped to the corpus's own Some/block-fallback shape + interior comment |
| `persist_array_default` | verified persist | — | kept |
| `persist_useless_format` | verified persist | — | kept |

Replacement atom: `persist_comment_hoist` — items_after_statements with an
interior comment the hoist would displace (the Batch-90 leading-gap guard).

**The lesson is the loop:** a persist atom is a CLAIM that a divergence
class is still open, and the freeze oracle adjudicates claims, not prose.
The gen-3 record would have pinned two closed/moved classes as permanent
difficulty if the gate had been authored to trust the table.

## The landed atom set (4 persist + 38 corpus)

| persist atom | lint fired | why the heal stays silent |
|---|---|---|
| `persist_array_default` | `map_unwrap_or` | the array-literal guard: real clippy's own MachineApplicable rewrite E0308s in a plain-let position; the slice_pinned relaxation needs a `&[T]` annotation this shape lacks |
| `persist_comment_arm` | `single_match_else` | the interior comment would be displaced by the if-let rewrite; `drops_interior_comment` refuses (the healer is SAFER than clippy) |
| `persist_comment_hoist` | `items_after_statements` | the leading-gap comment guard refuses the hoist |
| `persist_useless_format` | `useless_format` | the String-typed named-capture terminal needs the arg's TYPE (clone vs to_string); the positional-only proposer form stays silent by design |

## The freeze measurement

| Measure | Value |
|---|---|
| Atoms | 42 (4 persist + 38 corpus; 2 bad corpus double-fires: needless_match, uninlined_format_args — known from gen 2) |
| Specs | 78 (42-atom table: 42 k=1 + 16 k=2 + 12 k=3 + 8 k=4) |
| Verified | **72**, oracle labels ≡ union-of-atom gold 72/72 (real clippy) |
| Excluded | 6 (2 bad atoms + 4 cascade composites; zero independent interference) |
| Stratification gate | **PASS** (all 4 persist atoms resolve 0) |
| Re-freeze drift gate | PASS (identical re-run, zero label drift) |

Per-k solve rates (reported, not asserted — pre-registered):

| k | resolved/before | rate |
|---|---|---|
| 1 | 36/40 | **90.0%** |
| 2 | 26/30 | **86.7%** |
| 3 | 26/30 | **86.7%** |
| 4 | 24/28 | **85.7%** |

Near-monotone decay EMERGED even without the gate — the persist atoms at the
list head give wider rungs slightly more persist-weight. The axis is the
strata, not k; the table is recorded for visibility.

## The M5 admission verdict (exercised live)

```
generation v3 ADMITTED: heal_rate 0.8750 < v1 0.9623 — the generation is harder, floors begin
```

Row appended: run #47, `generation: 3`, heal_rate 0.875, created_rate 0.109
(14 created non-vocabulary lints — the documented per-atom fix cascades,
amplified by composites: `manual_map` → `single_option_map`-class nursery
output, the v1 cascade classes at composite width), parse_safety 100%,
decline 100%. `score_history_v3.svg` rendered. The v2 refusal path still
reproduces under the generalized gate (v2 1.0000 ≥ v1 0.9623, exit 1,
nothing appended — JSONL length verified unchanged after the refusal run).

## En-route find: the CLI generation/limit arms were corrupted at HEAD

The `--score-bench --generation` flag **did not parse at HEAD** (`3d43c90d`):
the collision-salvage merge had left the generation-parsing block inside the
`"--limit"` arm (so `--generation 2` fell through to the unknown-flag path
and `--misses --limit N` errored on `args.misses_limit` never being set —
the field existed, was documented in `--help`, and was set by NO flag).
`--score-report`/`--memory-stats` arms were duplicated (unreachable-arm
warnings). Fixed in this unit: both arms restored (`--limit` →
`misses_limit`, `--generation` → the 1|2|3 parse), duplicates removed;
all four CLI surfaces re-verified live (`--generation` validation errors,
v2 refusal, v3 admission, `--misses --limit 3` exit 0).

The Bench-069 record's CLI quotes were produced BEFORE the corruption
landed in the same merge window; the surface was broken by the landing
commit itself. Lesson for the collision ledger: the merge that keeps
"their queue listing + my detector" must re-derive the MATCH ARMS, not
just the functions — arm-level splice conflicts are invisible to a
function-level conflict review.

## What remains (the next difficulty axes)

- **The v3 rate is a FLOOR, not a frontier** — all four persist classes are
  permanent DESIGN refusals; no planned work moves gen-3's rate. A
  solvable-tomorrow axis needs either:
  - **the rustc-errors-side ladder** (Bench 069 candidate #4, the salvaged
    design): compose `corpus_gen` cases at k and measure the `--fix-compile`
    driver per k — a different oracle (per-sub-lint reduction, exact by
    construction) and a different repair domain; **EXECUTED 2026-09-06 —
    generation 4, ADMITTED at 54.3%:
    [`.benchmarks/071_generation4_rustc_error_ladder.md`](071_generation4_rustc_error_ladder.md)**; or
  - **vocabulary extension to rust_perf rules** (candidate #1b): the
    rust_perf divergence classes (array-literal vec-with-capacity, dep-gated
    proposers) are open frontier but changing `vocabulary()` re-baselines
    v1 — its own plan. **EXECUTED 2026-09-06 — generation 5, ADMITTED at
    69.2% (addressable 100% / persist 0%), as a SEPARATE generation via the
    gen-4 pattern (v1 NOT re-baselined):
    [`.benchmarks/072_generation5_rust_perf_ladder.md`](072_generation5_rust_perf_ladder.md)**; or
- **Real-G8 conversion A/B**: still data-gated (`.heal/misses/` empty).
- Saturation telemetry on gen-3 starts reporting once 6 gen-3 rows exist.
- **EXECUTED 2026-09-06 (4090): candidate #2 (real-repo interference
  spans) — premise FALSIFIED by mining, the real-shape salvage landed as
  generation 6 and was ADMITTED (95.2% < v1):
  [`.benchmarks/073_generation6_real_repo_shape_ladder.md`](073_generation6_real_repo_shape_ladder.md)**.

## Validation

- lib default 1121/0; lib 4-feature union (score_bench, latent_retrieval,
  rustc_errors + perf default) 1301/0/8i; `generation_v3_freeze` always-on
  1/0/1i + the freeze e2e green twice (deterministic, zero drift).
- clippy on the union state: zero findings on every touched line (the two
  remaining warnings in touched files are the sibling's pre-existing
  `era_line`/evolve rows at HEAD, not this unit's hunks).
- The `generation_v3_freeze` [[test]] row carries `required-features =
  ["score_bench"]` (the 713-T3 reader axis; the `#![cfg]` protects the
  count).
- Files: `src/score_bench/generations.rs` (persist table + v3 machinery +
  generalized spec/freeze/fixture builders + the landed freeze record),
  `src/score_bench/report.rs` (HISTORY_SVG_V3), `src/bin/heal.rs`
  (generalized run_score_bench + the CLI arm repair),
  `tests/generation_v3_freeze.rs` (the freeze e2e), `Cargo.toml` (the
  [[test]] row), `.benchmarks/score_history.jsonl` (+1 gen-3 row),
  `.benchmarks/score_history_v3.svg` (new).
