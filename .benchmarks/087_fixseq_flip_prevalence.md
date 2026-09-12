# Bench 087 — Fix-loop flip prevalence over the T1.5 attempt ring (Issue 91 T2)

**Status:** MEASURED 2026-09-12 — **NEGATIVE, and structurally so.** 0 flips on
all three keys over 230 attempts / 252 runs, at real depth power (20 runs at the
`max_iters=5` ceiling), with **0 wasted attempts** (230/230 `Kept`). The verdict
is not a low prevalence estimate: the shipped `--fix-compile` loop carries a
well-founded measure that makes a within-run state revisit **impossible**. The
T2 detector is therefore **NOT built** — a detector with no phenomenon to detect
and no budget burn to reclaim. Reopen triggers in §6; the sharpest one
(**Issue 093 T2's revive hook**) landed *during* this measurement and is analysed
in §5.5.

> Runner: `examples/fixseq_prevalence.rs` (`required-features = ["rustc_errors"]`,
> default-on). Driver: the **unmodified production** `CompileDriver` via
> `floor::DriverFixer` + `FloorHarness::measure_driver`. `--release`.
> Box: M3 Max, macOS 25.6.0, 2026-09-12.
>
> **Replicated across two code bases.** Every number below was produced twice —
> once at `e9c64748` and again at `aef0f331` (which merged Issue 093 T2's
> observer/revive seams and Issue 098 into the driver). **Bit-identical results**:
> the 093 hooks default to `None`, so the default arm's dynamics are unchanged.

## What the corpus is — read this before the numbers

**SYNTHETIC.** `--fix-compile` fires only on *compile errors*, and every repo in
the workspace compiles clean: a walk of all 33 sibling repo dirs under `~/git`
on 2026-09-12 found `.heal/` in **17** of them and `.heal/fixseq/` in **ZERO**
(`find . -maxdepth 3 -type d -name fixseq` → no hits) — the organic corpus T1
asked for does not exist, and cannot be produced without injecting errors, which
is exactly what this harness does. Two things are nonetheless REAL and carry the
verdict:

1. **The driver is the shipped driver** — `CompileDriver::isolated()`, production
   `Playbooks`, `max_iters = 5`, unmodified. The *dynamics* observed are the
   dynamics that ship.
2. **The loop-invariant check** (§4) is a property of those dynamics, not of the
   corpus. It is what upgrades "0 observed" to "0 possible".

A synthetic prevalence number must never be quoted as the organic prevalence T1
asked for. It is not quoted as one here.

## 0. The instrument asserts itself non-inert

A zero from a recurrence classifier is worthless if the classifier cannot fire.
The runner's `self_check()` runs on **every** invocation, before any measurement,
over rings with known answers:

| ring | expected | asserted |
|---|---|---|
| `A@aa → B@aa → A@aa` | code=1, span=0, pair=1 | ✓ (span=0 is correct — one span hammered is repetition, not oscillation) |
| `A@aa → A@bb → A@aa` | code=0, span=1, pair=1 | ✓ |
| `A@aa → B@bb → C@cc` | 0, 0, 0 | ✓ |
| `A@aa → A@aa` (adjacent) | 0, 0, 0 | ✓ (no intervening attempt) |
| `Kept` with `errors (1 → 1)` | 1 invariant violation | ✓ |

Printed as `instrument canary: PASS …` at the head of every run.

## 1. Arms

| arm | construction | cases | why |
|---|---|---|---|
| **SINGLE** | the stock corpus: 12 committed anchors (`MUTANT_CASES`) + 200 `corpus_gen` cases, one mutation per scaffold | 212 | the corpus that exists |
| **MULTI-5** | 5 generated cases composed into ONE scaffold, each in its own `pub mod cN` → 5 simultaneous errors per run | 40 | **the power lever** — see §3 |

## 2. Results

| quantity | SINGLE | MULTI-5 |
|---|---|---|
| runs | 212 | 40 |
| runs with ≥1 attempt | 126 | 24 |
| attempts total | **126** | **104** |
| attempt-depth histogram | `0=86 1=126` | `0=16 1=4 5=20` |
| runs at depth ≥3 (**the power**) | **0** | **20** |
| flips, key = `code` | **0** (0 runs) | **0** (0 runs) |
| flips, key = `span_hash` | **0** (0 runs) | **0** (0 runs) |
| flips, key = `(code, span_hash)` | **0** (0 runs) | **0** (0 runs) |
| outcomes | 126 `Kept` / **0 reverted** | 104 `Kept` / **0 reverted** |
| loop invariant | HOLDS (126/126) | HOLDS (104/104) |
| total fix rate (context) | 59.4% | 50.0% |
| wall | 126.7 s | 28.8 s |

Rings dumped via the shipped `evolve::dump_fixseq` — 126 + 24 = **150
`run-<ts>.jsonl` files**, i.e. the T1.5 store validated end-to-end **at scale**,
not just on the one hand-run e2e case it shipped with.

Strategy mix over the 230 attempts (both arms pooled): `e0597-scope-hoist` 82 ·
`e0499-swap-second-decl` 51 · `e0505-clone-at-move` 41 · `e0502-clone-out` 31 ·
`e0502-deref-removal` 25. Four error codes reach an attempt at all
(E0597/E0502/E0499/E0505); E0373/E0382/E0507/E0596 ship no template (Bench 041)
and account for most of the 102 zero-attempt runs.

## 3. The power caveat — why SINGLE alone would have been worthless

Every SINGLE run is **≤1 attempt deep** (`0=86 1=126`, max 1). A flip needs ≥3
attempts by definition. **SINGLE has ZERO power**: its 0 measures the corpus's
single-mutation construction, not the driver. Reporting it alone would be the
canonical green zero — an instrument that could not have found the thing.

MULTI-5 is the repair: 20 of its 40 runs run **5 attempts deep**, saturating
`max_iters`. Those 20 runs are precisely the "burn-full-budget" population the
T2 detector was proposed to short-circuit. They still show 0 flips.

## 4. The two-sided half — the loop invariant (why the zero is structural)

Read off `driver.rs` (at `aef0f331`) and **checked on every one of the 230
records**:

- **Kept** iff `new_diags.len() < before_errors` (`driver.rs:610`) — the
  coded-error count **strictly decreases**, and `diags` advances.
- **Every non-keep path** (`:537`, `:571`, `:594`, `:659`) restores the file
  byte-exactly, leaves `diags` untouched, and does `tried.insert(tried_key(&best))`
  — permanently suppressed for the run (Research 166 §0: "suppression never
  revives"; §5.5 below is the 2026-09-12 amendment to that clause).
- One attempt per iteration; `max_iters = 5`; break when no untried candidate remains.

So the run's state admits the lexicographic measure **(coded-error count ↓,
|tried| ↑)** over a finite candidate set, bounded below by 0. To revisit a state
you would need either an error-count *increase* (never kept) or to re-try a
*suppressed* candidate (never allowed). **There is no cycle.** Saddle-circling —
the fusion's premise — is not rare in this driver; it is unreachable.

The runner checks the measurable half on live data: `Kept ⇒ strict decrease`,
`reverted ⇒ count unchanged`, and cross-attempt continuity (the count entering
attempt *n+1* equals the count leaving *n*). **HOLDS, 230/230.**

⚠ **Honest limit of the empirical half:** 230/230 attempts were `Kept`, so the
run exercised only the *strict-decrease* arm. The `tried`-suppression arm of the
measure is a **code read**, not a measurement — no revert occurred to exercise
it. Stated as a code read, not as a verification.

## 5. The other half of T2 — there is no budget burn to reclaim

T2 asked to "measure decline-early vs burn-full-budget". Measured first, as the
gate on whether the comparison is even well-posed:

**Wasted attempts: 0 / 230.** Every attempt the shipped playbooks proposed was
kept. The 20 depth-5 MULTI-5 runs are not thrash — they are 5 successive
successful repairs that end `clean`. An early-decline detector on this loop can
only *decline attempts that would have succeeded*; its best possible outcome is
a tie and its expected outcome is a loss. The A/B was therefore **not run** —
running it would have measured a detector against a baseline with nothing to
improve, and reporting a wash as a result would be a wash dressed as evidence.

### 5.5 The revive hook (Issue 093 T2) — landed mid-measurement, and it does NOT break the argument

`8993ec51` (merged into `aef0f331`) added `CompileDriver::with_revive(ReviveFn)`
— "a candidate already in the `tried` set; `true` re-admits it for one more
attempt" (`driver.rs:314-318`), consulted at the retain (`driver.rs:508`). This is
**reopen trigger #4 arriving while the bench was being written**, so it is
adjudicated here rather than deferred:

- **The default arm is untouched.** `revive: None` ⇒ the retain drops every tried
  candidate unconditionally; the 230-attempt result above was replicated
  bit-identically before and after the merge. The wiring is gated **twice**:
  `#[cfg(feature = "nonergodic_driver")]` (non-default) AND `armed_by_env()`
  (`RIIR_HEAL_NONERGODIC`, which defaults *on* — so the **feature** is the real gate).
- **The measure SURVIVES, in weakened form.** `NonergodicOrdering::revive` is
  **one-shot per `(span_hash, strategy_id)`**: it reads `revive_used`, returns
  `false` if set, and sets it on every successful revival
  (`nonergodic_order.rs:537-553`). So the suppression term becomes
  `|tried| + |revived-once|` — still a finite, monotone, well-founded quantity.
  Cycles are bounded to **at most one repeat per key**, not re-admitted without
  bound. The §4 conclusion weakens from "no revisit" to "at most one revisit per
  candidate key, under `max_iters = 5`" — which is still not circling.
- **This corpus cannot measure the revive arm, and the vacuity is provable.**
  The retain is `!tried.contains(c) || revive(c)` — short-circuiting, so `revive`
  is consulted **only** for candidates already in `tried`. `tried` gained **zero**
  members across all 230 attempts (every attempt was `Kept`). **`revive` is never
  called on this corpus**, so a revive-arm run here would return a guaranteed-
  vacuous 0. Measuring it was skipped for that reason, not overlooked.
- **The re-measure trigger is therefore sharp:** the revive arm becomes
  measurable exactly when the revert rate leaves 0 (trigger #1). Those are the
  same event, and Bench 085's own gate is the place to re-run this one.
- Not measured here and NOT claimed: any behaviour of the `nonergodic_driver`
  arm. This section is a **code read** of two landed functions, labelled as such.

## 6. Verdict + scope

**T2 NEGATIVE.** The two-way flip detector is **not built**. Scope, precisely:

- ✅ **Within one `--fix-compile` run, default features**: no flip is observed AND
  none is possible (§4). Solid.
- ✅ **With the `nonergodic_driver` revive arm**: at most one revisit per
  candidate key (§5.5) — a code read, not a measurement.
- ⚠ **Synthetic corpus**: the *rate* numbers describe injected borrowck mutants
  repaired by 5 template strategies over 4 error codes. Not an organic rate.
- ⛔ **Across runs: NOT closed.** `tried` is per-run and rings are per-run
  (`run-<ts>.jsonl`). Two `--fix-compile` invocations over the same tree with an
  intervening human/tool edit *can* revisit a state, and §4 says nothing about it.
  The store is already join-able across runs by timestamp + `(repo, span_hash)`.
- ⛔ **`max_iters = 5`** caps observable depth at 5; a flip needing >5 attempts is
  unobservable in principle (though §4 forbids it regardless).

### Reopen triggers (any one re-opens T2)

1. **The revert rate leaves ~0.** 0/230 today. Adding *speculative* playbook
   strategies (ones that can fail a recheck) creates a wasted-attempt population,
   and §5's "nothing to reclaim" argument dies with it. Re-run this bench.
   **A second route to the same trigger, filed 2026-09-12 as
   [Issue 099](../.issues/099_interacting_error_corpus.md): an INTERACTING-error
   corpus.** Every corpus this bench ran on is built from independent
   single-mutation cases — `compose_multi` concatenates members that cannot
   affect each other — so a fix can never re-break anything, which is a
   precondition for both a flip and a revert. 099 T3 re-runs this bench over a
   corpus where repairing A changes B's diagnostic set; 099 T1 is the
   admissibility gate that has to prove it does.
2. **A cross-run join finds recurrence.** Once real `.heal/fixseq/` corpora
   accumulate, join rings by `(repo, span_hash)` across `run-<ts>` files.
   Cross-run is the one path §4 does not close.
3. **The keep rule stops being monotone.** Any change to `new_diags.len() <
   before_errors` (e.g. a severity-weighted or semantic keep rule) removes the
   Lyapunov function and the structural argument with it.
4. **The revive path loses its one-shot bound.** Landed and analysed in §5.5;
   it holds *because* `revive_used` is set on every revival. A multi-shot or
   budget-based revive policy re-admits unbounded cycles by construction and
   MUST re-run this bench as part of its own gate.

## 7. What this bench does NOT claim

- Not an organic prevalence (§0). Not a claim about the `--fix` (clippy) loop,
  which has its own dynamics (T1: 1% fail rate, 23 stuck families — repeated
  failure, no pair structure). Not a quality verdict: the 100%-divergence rows on
  E0597/E0499 are the *floor harness's* long-standing finding (`MatchedInverse`
  vs `Diverged`), reproduced here as context only.
- The near-miss half of the paper's saddle picture **does** have a healer
  analogue, and it is not a trap: `keep iff count decreases` commits *divergent*
  repairs (E0597 42/42 diverged, E0499 26/26) while the count drops. That is a
  **repair-quality** problem the floor harness already measures — not a
  trap-detection problem, and not addressable by a flip counter. If Research 546's
  "commits a nearly-correct answer" concern has a real target in this repo, that
  is it.

## Reproduce

```bash
cargo run --release --example fixseq_prevalence --features rustc_errors -- --store /tmp/fixseq_single
cargo run --release --example fixseq_prevalence --features rustc_errors -- --multi 5 --store /tmp/fixseq_multi5
```
