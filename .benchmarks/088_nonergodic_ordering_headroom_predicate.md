# Bench 088 — Issue 093 T2 follow-on: the G6/G7 ordering A/B at three corpus widths, and the HEADROOM predicate

**Status:** RECORD — **no measured gain for the nonergodic ordering on any surface available, and one measured COST REGRESSION in the shared-handle configuration.** [Bench 085](085_nonergodic_driver_wiring.md) landed the wiring and called the gain axis an honest NULL from a 2-scaffold e2e. This bench runs the lane Issue 093 actually designated — `examples/rustc_errors_floor.rs --only ab` — at **three corpus widths**, and separates two things Bench 085 could not: *the corpus cannot measure* (K=1) from *the corpus measured, and the answer is parity or worse* (MULTI-5). The verdict does not change (`nonergodic_driver` stays opt-in, out of the release set), but its **evidential standing** does: on the widest-menu lane the A/B has power — a control arm moves — and the policy still does not.

**Date:** 2026-09-12 · **Box:** Apple M3 Max (macOS 25.6.0), shared (load 9–21 across the runs) · **Toolchain:** rustc 1.98.1 (48a229cea 2026-09-01), pinned by `rust-toolchain.toml` · **Profile:** `--release`.

> **Wiring + mechanism gates:** [Bench 085](085_nonergodic_driver_wiring.md) · **PoC:** [Bench 084](084_nonergodic_driver_poc.md) · **The prior null on the K=1 corpus:** [Bench 045](045_fix_compile_goat_corpus212.md) §G6/G7(b) · **The depth lever this bench reuses:** [Bench 087](087_fixseq_flip_prevalence.md) §3 (Issue 91 T2) · **Issue:** [093](../.issues/093_nonergodic_per_run_strategy_posterior.md).

## What this bench adds (no new policy — it consumes the landed one)

1. **`DriverFixer::with_observer` / `with_revive` passthroughs** (`src/domains/rustc_errors/floor.rs`). Before this the G6/G7 harness could install an ordering but NOT its feedback channel, so any stateful policy run through it would have ordered from a belief that never learned and reported "no difference" for the wrong reason. That is an inert-arm trap, not a measurement.
2. **Two nonergodic arms**, both on the landed `nonergodic_order::NonergodicOrdering` (unchanged): **shared** (ONE handle across the corpus — learns across RUNS) and **per-run** (a fresh handle per case — the deployed shape: one `new()` per CLI invocation, one driver run). They are kept separate deliberately, and on the strided lane they **disagree**.
3. **`corpus_gen::compose_multi` + `compose_multi_strided`**, and a `--multi K [--strided]` flag on the A/B. `compose_multi` is the chunked composition Bench 087 §3 introduced inline in `examples/fixseq_prevalence.rs`, lifted into the library so more than one harness can use it. **`compose_multi_strided` is new and is the point of this bench** — see §"Why strided".
4. **Headroom counters on EVERY arm** — `attempts` (the `FixCompileReport::attempts` ring length = apply→recheck DECISIONS made) and `multi-attempt runs`. Bench 045 asserted saturation from `iterations`; `attempts` is the quantity an ordering acts on.
5. **A liveness sentinel** (observer ticks) and a **menu-width histogram** (`PolicySnapshot::menu_len` per case). Those two answer "could this arm have done anything?" independently of what it did.

## Lanes run

| lane | command | result |
|---|---|---|
| gated lint | `cargo clippy --features nonergodic_driver --all-targets` | exit 0; **0 findings in the three touched files** (`floor.rs`, `corpus_gen.rs`, `rustc_errors_floor.rs`) |
| default lint | `cargo clippy --all-targets` | exit 0 |
| the policy's own gates | `cargo test --features nonergodic_driver --test nonergodic_driver_ab` | **6 passed, 0 failed** — Bench 085's suite is unaffected by the passthroughs |
| touched lib surface | `cargo test --lib domains::rustc_errors::floor` | **8 passed, 0 failed** (default features) |

Pre-existing warnings the repo already carries (`frontier_pairs`, `frontier_report`, `evolve`, `orphan_report`, `lib.rs`) are untouched and not counted. One NEW `clippy::format_collect` on the added `compose_multi` was fixed by hand — `cargo heal --fix --write --verify --verify-args "--features nonergodic_driver"` scanned the file and fixed **0 edits**, so `format_collect` is a healer MISS (noted for the usage-artifact intake queue).

**Canary:** an undefined call planted inside the `#[cfg(feature = "nonergodic_driver")]` arm block produced `error[E0425] … could not compile riir-clippy (example "rustc_errors_floor")`, exit 101, then restored. The lane does compile the arms.

## Results

### Lane 1 — K=1, the stock template-covered stratum (n = 126)

| arm | clean | iterations | reverts | attempts | multi-attempt runs |
|---|---:|---:|---:|---:|---:|
| static (min-delta) | 126/126 | 126 | 0 | 126 | **0** |
| flat counters | 126/126 | 126 | 0 | 126 | **0** |
| shape-conditional (shipped) | 126/126 | 126 | 0 | 126 | **0** |
| nonergodic shared | 126/126 | 126 | 0 | 126 | **0** |
| nonergodic per-run | 126/126 | 126 | 0 | 126 | **0** |

`observer ticks 126` · `menu width histogram {1: 105, 2: 21}` · `t_observed 126` · all discordants **0/0** · `HEADROOM PREDICATE = 0`.

**Verdict: UNMEASURABLE.** Exactly 1.00 decisions per run (the zero in `multi-attempt runs` says it is 1 in *every* run, not a mean). The arm is LIVE — all 126 ticks fire — but every tick is its run's LAST event, so none precedes an ordering call. Independently, the menu is width-1 in 105 of 126 runs, which makes the policy's score constant and hands the decision to the incumbent min-delta tiebreak identically. Correctness is at its ceiling (126/126) and cost at its floor (0 reverts): no headroom in either direction. The 0/0 discordants are **UNDEFINED** (McNemar's `b + c = 0`), not "not significant". This reproduces [Bench 045](045_fix_compile_goat_corpus212.md) §G6/G7(b) exactly, and matches Bench 087 §3's SINGLE arm (`0=86 1=126`, max depth 1).

### Lane 2 — MULTI-5 chunked (n = 40; 5 simultaneous errors per scaffold)

| arm | clean | iterations | reverts | attempts | multi-attempt runs |
|---|---:|---:|---:|---:|---:|
| static (min-delta) | 20/40 | 104 | 0 | 104 | 20 |
| **flat counters** | 20/40 | **120** | **20** | 120 | 24 |
| shape-conditional (shipped) | 20/40 | 104 | 0 | 104 | 20 |
| nonergodic shared | 20/40 | 104 | 0 | 104 | 20 |
| nonergodic per-run | 20/40 | 104 | 0 | 104 | 20 |

`observer ticks 104` · `menu width histogram {0: 16, 1: 20, 2: 4}` · `t_observed 104` · all discordants **0/0** · `HEADROOM PREDICATE = 80`.

**Verdict: parity, on an instrument that is now ADMISSIBLE.** The predicate fires non-zero (20 depth-5 runs per arm), so the feedback loop does engage. And critically, **the flat-counters control MOVES**: +16 iterations and +20 reverts against every other arm. A bad ordering is visible on this corpus — so the nonergodic arms' 104/0 is a *measured* tie with min-delta and the shipped shape-conditional ordering, not an instrument that cannot see. What still blocks a *gain* is menu WIDTH, not decision count: 20 of the 24 ordering runs freeze a width-1 menu, because `compose_multi` chunks CONSECUTIVE generated cases and `generate_corpus()` emits families contiguously — five chunked cases are usually five variants of ONE template, hence one `strategy_id`.

### Lane 3 — MULTI-5 STRIDED (n = 40; 5 DIFFERENT families per scaffold)

| arm | clean | iterations | reverts | attempts | multi-attempt runs |
|---|---:|---:|---:|---:|---:|
| static (min-delta) | 0/40 | 120 | 0 | 120 | 40 |
| flat counters | 0/40 | 120 | 0 | 120 | 40 |
| shape-conditional (shipped) | 0/40 | 120 | 0 | 120 | 40 |
| **nonergodic shared** | 0/40 | **135** | **15** | 135 | 40 |
| nonergodic per-run | 0/40 | 120 | 0 | 120 | 40 |

`observer ticks 120` · `menu width histogram {3: 20, 4: 20}` · `t_observed 120` · shared snapshot `menu_len 4, t_observed 115, committed Some((2, 0.485)), store_stale false` · all discordants **0/0** · `HEADROOM PREDICATE = 160`.

**Verdict: the only lane where the ordering seam has a real choice — and the policy does not win it.**

- **Menus are finally wide** (3 or 4 distinct strategies in every run) and **every** run is multi-attempt. This is the configuration the whole mechanism was designed for.
- **Resolved rate is FLOORED at 0/40 for every arm** — a 5-family scaffold is beyond `max_iters = 5`, so the correctness axis cannot discriminate here either (a floor instead of Lane 1's ceiling). Honest consequence: this lane cannot award a resolved-rate WIN to anything.
- **The cost axis does discriminate, and it goes AGAINST the policy's shared-handle configuration:** `nonergodic shared` burns **135 iterations and 15 reverts** where every other arm (including plain min-delta) burns 120 and 0 — **+12.5% iterations, +15 wasted attempts, zero correctness benefit.** The per-run arm — the deployed shape — is at exact parity with min-delta.
- The shared/per-run split is therefore not cosmetic: it is the difference between a regression and a tie. The policy's own docs already say the handle is per-run; **this is measured evidence for that constraint**, and worth flagging to its owner as a caveat (a long-lived handle freezes its menu on the first run it ever sees — `menu_len 4` here — and then carries that menu, and its accumulated belief, into every later run).

## Why strided (the finding behind `compose_multi_strided`)

Lane 2's menu histogram is the reason this bench has a third lane. `generate_corpus()` emits its families contiguously, so `compose_multi`'s contiguous chunking puts **five variants of one template** in a scaffold — one `strategy_id`, a width-1 menu, and a policy that is provably the identity no matter how deep the run goes. `compose_multi_strided` draws the K members by stride instead (`cases[i * groups + g]`), so each scaffold spans K families. Measured effect: menu widths go from `{0:16, 1:20, 2:4}` to `{3:20, 4:20}`.

**The generalizable lesson: a depth lever is not a width lever.** Bench 087's MULTI-5 arm gave the attempt DEPTH that an oscillation detector needs (and correctly found none). The same corpus does NOT give the menu WIDTH an ordering A/B needs. Two different within-run properties, two different corpus requirements — and the chunked composition satisfies only the first.

## The reopen predicate (Issue 093 T4)

The harness prints, on every run:

```
HEADROOM PREDICATE (Issue 093 T4 reopen trigger): multi-attempt runs across all arms = N
```

- `N == 0` ⇒ the corpus cannot adjudicate ANY stateful ordering. Do not run an A/B over it; do not report its null as evidence about a mechanism.
- `N > 0` ⇒ the gain axis is measurable.

It is necessary, not sufficient — Lane 2 shows `N = 80` with menus still width-1. Read it together with the **menu-width histogram** (is there a choice?) and a **moving control arm** (can the instrument see a bad ordering?). Lane 3 is the first configuration in this issue's history where all three hold.

## Honest scope caveats

1. **Every corpus here is AUTHORED/SYNTHETIC.** Each case is a compiling seed plus a declared `from`/`to` rewrite; `floor.rs`'s header bounds it: *"this corpus measures the mutation classes we chose … not about all Rust."* Issue 093 T2's gate names *"the real 1%-fail surface"* — **none of these is that surface**, and nothing here may be quoted as a real-surface GOAT. The organic ring does not exist yet: Bench 087 measured `.heal/fixseq/` present in **ZERO** of 33 sibling repos.
2. **Lanes 2 and 3 are COMPOSED corpora**, not naturally multi-error code. Five independent errors in five sibling modules is not the same dependency structure as five interacting errors in one function — which is exactly where ordering is expected to matter most, and is not represented anywhere here.
3. **Neither MULTI lane can award a resolved-rate win**: Lane 2 is at 20/40 for every arm, Lane 3 floored at 0/40. The only axis that discriminated anywhere was cost (iterations/reverts), and only in Lane 2's control and Lane 3's shared arm.
4. **n = 40 in both MULTI lanes.** Small. The 15-revert regression is a clean deterministic difference, not a statistical estimate — it replicates byte-identically — but it is 40 scaffolds of one composition recipe.
5. **No perf claim.** Wall time is `cargo check`-subprocess-dominated on a shared box; only integer counts are asserted.
6. **This bench does not re-test the policy's mechanism.** That is `tests/nonergodic_driver_ab.rs` (6/6, re-run green here). This bench tests whether a CORPUS can adjudicate the gain axis, and what it says when it can.

## Replication

| lane | runs | result |
|---|---:|---|
| K=1 (n=126) | 3 | **byte-identical** (`diff` clean on all pairs) |
| MULTI-5 chunked (n=40) | 3 | **byte-identical** |
| MULTI-5 strided (n=40) | 2 | **byte-identical** |

Two further executions on earlier, less-instrumented binaries reproduced the K=1 static/flat/shape rows exactly (126/126 · 126 · 0). Integer counts over a deterministic corpus with a deterministic oracle — not timing- or order-sensitive.

## Verdict

**GOAT gain axis: NO MEASURED GAIN, on any of three corpus widths — and one measured REGRESSION.**

| gate | verdict |
|---|---|
| G1 correctness / resolved rate at equal budget | **PARITY everywhere** — 126/126 (Lane 1, ceiling), 20/40 (Lane 2), 0/40 (Lane 3, floor); all discordants 0/0 |
| G2 perf | **NOT MEASURED** — no latency claim; the oracle subprocess dominates |
| G3 no-regression | **PASS for the deployed (per-run) shape** — exact parity with min-delta on every lane. **FAIL for the shared-handle shape on Lane 3** (135 iterations / 15 reverts vs 120 / 0) |
| G4 alloc | **NOT MEASURED** |
| instrument admissibility | **PASS on Lane 2** (the flat-counters control moves: +16 iterations, +20 reverts), **PASS on Lane 3** (menus width 3–4, all runs multi-attempt) |

**Recommendation — owner decides promotion, per repo precedent: keep `nonergodic_driver` OPT-IN and out of the release set.** Bench 085's own verdict is unchanged; what changes is that "NULL" is no longer only "no surface". On the one lane where the seam genuinely has a choice, the policy ties min-delta at best and loses 12.5% of the iteration budget at worst. Nothing here refutes the T1 PoC's mechanism — the PoC's world had latent regimes that its components modelled by construction, and the rank-rotation port is the deliberately weaker family — but nothing here supports promotion either.

**Owed, in priority order:**

1. **Do not ship a long-lived `NonergodicOrdering` handle.** Lane 3's shared arm is the measured cost of one. The CLI already builds per-invocation; this is a constraint to keep, and the policy's docs should carry the measurement.
2. **A corpus where the resolved rate can actually move** — the composed lanes are ceiling-bound (Lane 2) or budget-bound (Lane 3). Candidates: `--multi 3 --strided` (wide menu inside `max_iters = 5`), or a raised `max_iters` on Lane 3.
3. **Interacting errors, not independent ones** — five errors in one function, where fixing A changes B's candidate set. That is the regime the PoC's cascade modelled and no corpus here reproduces.
4. **The organic surface**, when `.heal/fixseq/` accumulates one (zero rings across 33 repos as of 2026-09-12).

## Reproduction

```sh
cargo clippy --features nonergodic_driver --all-targets
cargo test  --features nonergodic_driver --test nonergodic_driver_ab
cargo build --release --features nonergodic_driver --example rustc_errors_floor

./target/release/examples/rustc_errors_floor --only ab                        # Lane 1 (K=1, n=126)
./target/release/examples/rustc_errors_floor --only ab --multi 5              # Lane 2 (chunked, n=40)
./target/release/examples/rustc_errors_floor --only ab --multi 5 --strided    # Lane 3 (strided, n=40)
```
