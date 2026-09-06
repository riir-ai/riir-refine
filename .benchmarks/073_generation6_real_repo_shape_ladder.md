# Bench 073 — generation 6: the real-repo shape ladder (Bench 069 §What remains candidate #2, SALVAGED)

**Status:** RECORD — candidate #2's premise REFUTED by mining, the real-shape
salvage ADMITTED (gen-6 95.2% < v1 96.2%, deterministic ×3 + a 4th
byte-stability run).

**Box:** 4090 / Windows, CPU-only (the oracle spawns cargo+clippy; no GPU).
**Freeze command:**
`cargo test --release --features score_bench --test generation_v6_freeze -- --ignored --test-threads=1 --nocapture`
**Bench command:**
`cargo run --release --features score_bench --bin cargo-heal -- --score-bench --generation 6`

## The premise is refuted — the mining falsification (the headline finding)

Candidate #2 (Bench 069): *"mine multi-lint spans from sibling repos at
controlled k, where lint interactions are organic."* The pre-registered
design (Plan 095 §1) assumed the population exists. It does not, at any
mechanical extraction granularity:

| extraction | sources | candidates | compiled | organic multi (k≥2) |
|---|---|---|---|---|
| lone fns (the v1 mine path) | 4 sweeps | 1,247 fns | ~570 | **0** |
| adjacency runs (greedy over passing items) | katgpt-rs examples | 2,096 items → 1 run | — | **1** (same-class pair) |
| file-header carves (Plan 095 §1a) | 8 sweeps/trees | ~1,300 carves | ~120 | **4** |

The binding constraint is the BARE-STUB FILTER (v1's documented scope
limit): ~90% of real fn spans reference crate-internal or external-crate
types and never compile standalone, and the compilable crust is
predominantly lint-free (ndb: 80 compiled, 0 lints; game-sdk HEAD: 17
compiled, 0 lints). The multi-lint population lives exactly where the
filter cannot see. The four repos probed at HEAD post-heal are clean —
the pre-heal state exists only at pre-sweep shas.

**The salvage** — the v1 record already carried the answer in its own
provenance strings: *"trait scaffolding added, lint spans verbatim."* The
file-header carve mechanizes that: each candidate ships with its FILE'S OWN
header context (use + type + const items, then macros + impls, then sibling
fns as tier 3), so crate-internal lint spans compile verbatim — no
synthesized stubs, no edited spans. At the carve tier the organic multi
population is 4/17 (measured, never padded), and the SINGLE-lint real-shape
rung becomes the generation's population.

## The landed generation

17 spans (13 single + 4 organic multi), 6 vocabulary lint classes, 4 source
repos, mined at the parent of each repo's first heal sweep
(provenance-by-construction: the sweep diff IS the lint evidence):

| repo @ mining sha | sweep | spans |
|---|---|---|
| katgpt-rs @ 5b2d4297~1 | 5b2d4297 (root examples, 147 files) | 7 |
| riir-train @ a1197e06~1 | a1197e06 (engine examples, 49 files) | 4 |
| riir-train @ e6084429~1 | e6084429 (engine src, 30 files) | 2 |
| riir-ai @ 1e050442a~1 | 1e050442a (engine src, 174 files) | 4 |
| riir-chain @ 7421245a~1 | 7421245a (root lib src, 47 files) | 2 |
| riir-auth @ d7a78dd~1 | d7a78dd (src heal, 35 edits) | 0 kept (1 harvested, cap) |

## The freeze

| Measure | Value |
|---|---|
| Spans | 17/17 verified at the BENCH instrument (single-snippet `lints_for`) |
| Exclusions | **0** — batch (mining) and single (bench) instruments agree on every vocabulary label |
| Relabels | 0 |
| Territory | multi 4 / single 13 (k-truth asserted) |
| Freeze cost | 3.35 s (17 single-snippet oracle runs, release) |
| Re-freeze drift | PASS (identical, zero drift) |

The G2 label fence's cross-checks all read EMPTY: every mining-time
batch-oracle claim fired at freeze time, and every additional bench firing
was out-of-vocabulary pedantic/nursery noise (`cast_precision_loss`,
`must_use_candidate`, …) — correctly outside the fence.

## The bench: ADMITTED

| Measure | Value |
|---|---|
| heal_rate | **95.2%** (0.95238) — v1 anchor 96.2% → **ADMITTED** |
| created_rate | 0.0% (zero new lints) |
| parse_safety | 100% |
| decline_correctness | 100% |
| Determinism | ×3 identical (rows #53-#55) + a 4th run after a delimiter-only re-land: byte-identical (row #56) — the score is content-determined |

**Territory solve rates (the interaction signal, pre-registered):**

| territory | solve | rate |
|---|---|---|
| single (k=1, real shapes) | 13/13 | **100.0%** |
| multi (k≥2, organic) | 7/8 | **87.5%** |

The single rung at 100% is itself a result: on REAL single-lint shapes
(spelling variance the corpus never taught), the proposers solve everything
— generalization measured, not assumed. The multi rung's single miss is
`single_match_else ×1` in `gen6-transposition-impl1-000` — root-caused by
manual probe: the `Some(_) =>` arm carries an interior comment the rewrite
would displace — the **Issue-028 comment-guard divergence class**,
re-found organically. That is a DESIGN refusal (the healer is SAFER than
clippy's autofix), not a proposer bug; the bench re-discovered a known
divergence class in real code, which is exactly what a difficulty axis is
for.

The prediction audit (Plan 095 §1): multi < single **held** (87.5% < 100%);
"single < v1" **refuted** — real single-lint shapes solve at parity. The
overall admission still cleared (v1's 96.2% anchor sits below 100% because
v1's own composites mix in the persist atoms).

## What remains

- **The multi rung is anecdote-sized (n=4 spans, 8 lint instances)** — the
  organic-interference claim stays MEASURED-BUT-THIN; it graduates only if
  the population grows. The standing lever is v1's deferred Phase 3
  (per-span stub manifests): the carve tier already halves that problem —
  the residual is fns whose context is EXTERNAL-crate items, where a
  per-fixture path-dep manifest (compile the sibling at the mining sha) is
  the next rung. That is a plan of its own, owner-gated on the score-bench
  series justifying it.
- Real-G8 conversion A/B: still data-gated (`.heal/misses/` empty).
- Saturation telemetry on gen-6 activates at 6 rows (currently 4).
- **The last unexecuted difficulty axis (#3, overlap-edit chains) is now
  EXECUTED 2026-09-06: ADMITTED as generation 7 (78.3% < v1) —
  [`.benchmarks/074_generation7_overlap_chain_ladder.md`](074_generation7_overlap_chain_ladder.md).**

## Validation

- lib default 1129/0 (gen-1 baseline unchanged); lib score_bench union
  1324/0/8i (+4 gen-6 always-on gates: floors, parse/territory k-truth,
  id-disjointness incl. v1, freeze-hole loudness).
- Freeze e2e green ×2 (drift gate, zero warnings); `--generation 6` CLI
  live (admission + territory table + `score_history_v6.svg`); parse range
  1..=6; help text updated.
- clippy on the union state: ZERO findings on touched lines (the 9
  `needless_raw_string_hashes` on the landed literals were healed by
  re-normalizing delimiters to minimal hashes — with the corrected
  predicate: plain `r"` only when the body contains NO `"` at all; the two
  remaining warnings are the pre-existing composite.rs `if_not_else` and
  the gen-5 doc-comment at generations.rs:823, both outside this unit's
  hunks).
