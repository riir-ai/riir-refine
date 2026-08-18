# Bench 039 — does L4 beat DECLINE on the reachable set? (Plan 336 T5.1)

**Verdict:** **NO — L4 does not beat decline. T5.1 stays BLOCKED (measured, not argued).**
**Harness:** `riir-clippy/tests/t51_l4_vs_decline.rs` (3 arms; arm 0 fast, arms 1–2 `#[ignore]`d)
**Checkpoint:** riir-train Plan 336 **T2.9 step-50 best** — rank 4 / alpha 8.0 / 48 DeltaNet layers,
LORC SHA256 `2c4bd5d14fca6c31123fe39044e3aa4a16f052282dbb6a78e887ece3372c233a`
(verified byte-identical to `E:\tmp\p336_t29\v2edit_rank4_lorc.bin.best.bin` on the 4090), converted LORC→BCL4.
**Date:** 2026-08-17 · **Machine:** M3 Max, CPU decode, `--max-new 96`, single process, `--test-threads=1`
**Plan:** [riir-train Plan 336](../../riir-train/.plans/336_code_llm_rlvr_clippy_l4_lora.md) T5.1
**Predecessor:** [Bench 037](037_modelless_miss_rate_l4_addressable_gap.md) (measured the 9/29 reachable gap)

## Why this benchmark is not another G1

All four prior gates (riir-train Bench 459/460/465/467) scored the adapter on
`clippy_l4_v2edit_test_chunk_*` — the *training* distribution's held-out split —
and all four returned 0/60. **The shipped pipeline never sends L4 anything from
that distribution.** Since the Issue 030 gate fix (`d8440f6`), L4 is consulted
only on the out-of-corpus arm: a measured **9/29** of cases (Bench 037), where
`corpus.lookup` returns nothing and the only surviving candidate is an
irrelevant `Memory` trajectory. Those 9 cases are L4's actual deployment
distribution, and nobody had measured them.

The bar is **DECLINE**, not zero. Since `d8440f6` the pipeline returns `None` on
all 9, and `None` is a *correct* answer — it lets the caller escalate honestly
instead of paying a verifier to discover a plausible-looking fix is unrelated.

**Pre-registered in the harness header before the run:**

> **L4 beats decline iff `WIN >= 1` AND `REGRESSION == 0`.**

`REGRESSION == 0` is a hard assertion; `WIN >= 1` is reported, not asserted, so
the harness stays runnable as a probe while G1 is still 0.

## Results

### Arm 0 — the DECLINE baseline (fast, not ignored)

**9/9 decline.** Measured rather than inherited from Bench 037, and it doubles as
a regression guard: if this ever serves, the Issue 030 relevance gate has broken
and every number below is measured against the wrong baseline.

### Arm 1 — L4 on the reachable set

| lint | outcome | s | L4 output |
|---|---|---|---|
| `or_fun_call` | *unscoreable* | 28.6 | `let x = opt.unwrap_or_else(expensive);` |
| `large_enum_variant` | **REGRESSION** | 40.1 | `enum E { A(u8), B([u8; 4096]) }` — **no-op, input echoed** |
| `too_many_arguments` | WIN\* | 68.6 | `fn f(a,b,c,d,e,g,h)` — **parameter `f` deleted** |
| `redundant_clone` | *unscoreable* | 34.9 | `let s2 = s.clone(); drop(s2);` (no-op) |
| `vec_init_then_push` | *unscoreable* | 37.5 | `let mut v = vec![1, 2];` |
| `manual_strip` | *unscoreable* | 43.9 | `if s.starts_with("ab") { let t = &s[2..]; }` (no-op) |
| `box_collection` | **WIN** | 33.7 | `struct S { v: Vec<u8> }` |
| `option_if_let_else` | *unscoreable* | 42.8 | `if let Some(x) = o { x + 1 } else { 0 }` (no-op) |
| `ptr_arg` | **WIN** | 40.9 | `fn f(v: &[u8]) -> usize { v.len() }` |

```
scoreable  4/9   (5 excluded — see "the first run was wrong")
WIN        3/4
NEUTRAL    0/4
REGRESSION 1/4
VERDICT: L4 does NOT beat decline  (WIN>=1 AND REGRESSION==0)
```

Daemon load 128.5 s (one-time). Per-fix decode: **mean 43.7 s, median 37.4 s,
min 28.6, max 68.6** (first run, same 9 inputs: mean 43.7, median 37.4, max 96.2).

## The first run was wrong, and the correction is the main finding

The first execution reported **8/9 WIN**. It was a measurement artifact.

`ClippyVerifier` writes the code verbatim into `src/lib.rs`. Most fixtures here
are *statement* fragments referencing undefined names (`opt`, `s`, `o`), so the
crate does not compile → clippy emits compile errors and **no lints** →
`present_lints` is empty → `lint_resolved` returns `true` → scored WIN. Measured
directly:

```
redundant_clone  "let s2 = s.clone(); drop(s2);"   -> could not compile
manual_strip     "if s.starts_with(..) { .. }"     -> could not compile
box_collection   "struct S { v: Vec<u8> }"         -> compiles, lint runs
```

**Three of the eight "wins" had L4 output byte-identical to the input.**

I had validated the instrument before the run — and the validation passed while
proving nothing. It used `ptr_arg`, an *item*, which is exactly the case that
works. **Validating a silent-failure verifier on one positive example shows it
can detect the lint; it does not show that a negative result means the lint is
absent.** The control has to be per-case, not per-instrument.

The fix is `control_fires`: a case is scoreable only if the **buggy** input
demonstrably trips the lint in the same scaffold. Everything else is
`Unscoreable`, excluded from the denominator and printed. A no-op
(`output == input`) additionally can never be a WIN.

Excluding beats "wrap the fragment in `fn f() { … }`": these fragments reference
undefined names, so a wrapper still would not compile — it would relocate the
false negative somewhere less visible. **5 of 9 reachable cases are currently
unscoreable by this verifier**, which is itself a finding: the fixture and the
verifier were built for different shapes.

## Two routes to a false WIN, and only one is fixed

1. **Compile-failure route** — the crate does not build, no lint fires,
   "resolved". **Fixed** by `control_fires`.
2. **Destructive-edit route** — the crate builds, the lint genuinely clears, and
   the edit is still wrong. **NOT fixed, and not fixable by this harness.**

`too_many_arguments` is route 2, live: `fn f(a,b,c,d,e,f,g,h)` →
`fn f(a,b,c,d,e,g,h)`. It **deleted parameter `f`**. Seven arguments is under the
threshold, so the lint clears and the verifier honestly reports WIN. But
`RustAnalyzerPruner` only checks that it parses and `ClippyVerifier` only checks
the lint cleared — **neither checks that behaviour is preserved**, so a fallback
that resolves lints by deleting code passes every gate currently in the tree.

This asymmetry is why it matters more than the REGRESSION: a regression is
*caught* (the lint still fires, the caller escalates), a destructive edit is
*silent*. Counting it, the admissible reading of arm 1 is **2 clean wins
(`ptr_arg`, `box_collection`), 1 destructive win, 1 regression, on 4 scoreable
cases** — not 3/4.

The two clean wins are real fixes: `&Vec<u8>` → `&[u8]` and `Box<Vec<u8>>` →
`Vec<u8>`. This adapter is not inert.

## What this refutes

**My own hypothesis going in — that the edit-only response contract would
collide with the seam's parse gate and everything would be discarded — is
REFUTED.** `NEUTRAL == 0` in both runs: all 9 generations parsed and were
surfaced. **0 declined, 0 unparseable.** The parse gate is suppressing nothing,
so `p_win` is limited by fix quality, not by contract misalignment.

### Arm 2 — the mechanism, derived independently

Arm 2 re-derives the split by a different route: it calls `generate_fix`
directly, bypassing the pipeline, then applies `RustAnalyzerPruner` and
`ClippyVerifier` by hand — so it does not depend on `consult_l4` discarding the
evidence.

```
declined (no output)          0/9
unparseable (discarded)       0/9
parses but lint unresolved    1/9      <- large_enum_variant
parses and lint resolved      8/9
```

(Arm 2 is a raw diagnostic and deliberately does **not** apply `control_fires`,
so its "8/9 resolved" still contains the 5 unscoreable cases. Its job is the
mechanism split, not the verdict.)

**Confirmed: 0 declined, 0 unparseable, by two independent paths.** The
contract-mismatch hypothesis is dead.

**And arm 2 exposes why the 5 cases are unscoreable — the two gates disagree
about what "valid Rust" means.** `RustAnalyzerPruner::validate` accepted *every*
output, including bare statement fragments like `let s2 = s.clone(); drop(s2);`,
while `cargo` cannot compile those as a lib crate at all. So the seam's parse
gate is strictly more permissive than the verifier's compile step. That gap is
the mechanism behind both of this bench's problems at once: it is why nothing is
discarded (`NEUTRAL == 0`) *and* why 5 cases cannot be scored. It is a property
of the two instruments, not of the adapter.

It also refutes the flat reading of the four 0/60 gates. An adapter at 0/60 on
the training-distribution split produced 2 clean fixes out of 4 scoreable
deployment cases. **Held-out EM on the training split does not predict behaviour
on the deployment distribution** — consistent with T4.1's finding that held-out
ppl does not predict EM, one level further out.

## Verdict and consequences

**T5.1 stays BLOCKED — recorded as a measured blocker, not `[x]`.** The
pre-registered rule fails on the harm half: `REGRESSION == 1`. T5.1 is "export
the adapter for the L4 fallback", and exporting one that echoes its input while
the lint still fires re-introduces exactly the false-confidence failure Issue 030
was filed to remove. The blocker is no longer "no checkpoint scores anything" —
it is now **specific and addressable**: one regression and one destructive edit
in four scoreable cases.

Note the export remains partly moot as written: `DaemonL4::spawn` consumes a
checkpoint *path*, so no separate export artifact is required.

**For T5.3** (owned by a concurrent session, riir-train Bench 471): this run
supplies `L` = 43.7 s mean / 37.4 s median and refutes latency as the binding
constraint, but it cannot supply an admissible `p_win`. Under that bench's own
clause — a WIN count is not admissible until decomposed into semantics-preserving
vs destructive — arm 1 yields `p_win = 2/4` clean, `p_dest = 1/4`, `p_reg = 1/4`
on n=4. **T5.3 stays unchecked**, now for a sharper reason: `p_reg > 0` and
`p_dest > 0` both flip the sign of the promotion arithmetic independently of
`p_win`.

## Update 2026-08-18 — both next levers BUILT (CPU-only); re-run pending

Both levers below were implemented and their instruments validated. The re-run
that would convert them into new counts is blocked on an unrelated external
condition (below), so **the verdict in this document is unchanged**.

**Lever 1 — a reachable-set fixture that actually compiles.** All 9 fixtures were
replaced with item-level equivalents for the *same 9 lints* and each was verified
to trip its lint in the verifier's scaffold:

```
or_fun_call fires=True     large_enum_variant fires=True   too_many_arguments fires=True
redundant_clone fires=True vec_init_then_push fires=True   manual_strip fires=True
box_collection fires=True  option_if_let_else fires=True   ptr_arg fires=True
```

So the scoreable denominator becomes **9/9 instead of 4/9**. Two needed care:
`redundant_clone` only fires when the clone is genuinely dead
(`let s = String::from("hi"); let s2 = s.clone(); drop(s2);`), and
**`box_collection` only fires on a PRIVATE, constructed field** — clippy skips
`pub` fields because they are public API. The lint names are unchanged, so the
out-of-corpus premise still holds and arm 0 still measures **9/9 decline**.

**Lever 2 — the semantic-preservation check, which closes route 2.** Implemented
as `contract_preserved` over `syn`: it compares *contract* identifiers — function
names, function parameter names, struct/enum names, field names, variant names —
and fails when the fix drops one.

It deliberately **excludes local `let` bindings**, and that exclusion is what
makes it usable: deleting a local binding is the *correct* fix for
`redundant_clone`, so the naive "no identifier may disappear" rule would reject
correct fixes. Parameters and fields are the item's interface; locals are not.

A destructive edit is counted as **harm, not as a win**. That does not amend the
pre-registered rule — WIN was always defined as "strictly better than decline",
and a silent behaviour change never qualified. **Only the detector changed.**

**The detector has its own guard test, on real data** — six cases drawn from the
actual 2026-08-17 generations, not invented strings: the real destructive edit
(deleted parameter `f`) must be flagged; the two real fixes (`&Vec<u8>`→`&[u8]`,
`Box<Vec<u8>>`→`Vec<u8>`) must not; a correct `redundant_clone` fix that drops a
local must not; dropping a whole function or a struct field must be. This is
deliberate: the previous instrument was validated on a single positive example
and silently mis-scored an entire class, so a detector added to catch that gets
positive *and* negative cases or it repeats the mistake.

`cargo test` (fast arms): **2 passed** — arm 0 at 9/9 decline, detector guard green.

**Why the re-run has not happened yet.** The 27B eval binary
(`bonsai_clippy_l4_eval`) must be rebuilt, and the katgpt-rs workspace currently
does not compile: `crates/katgpt-forward/src/step.rs:196` has an uncommitted
`E0308` (`map_or(&[], |m| m.as_slice())` inferring `&[_; 0]`) belonging to a
concurrent session. Per the repo's no-touching-sibling-WIP rule it was left
alone. It is a build-environment blocker, not a result: no GPU is owed, and the
re-run is a ~10 minute CPU job once the workspace compiles.

(The staged checkpoint and prior binary were also reaped from `/tmp` mid-session;
the checkpoint has been re-fetched to `~/.cache/t51/` — SHA256 `2c4bd5d1…`,
identical to the original — so only the binary rebuild is outstanding.)

## Corrections from the n=9 re-run (2026-08-18)

Two claims in the n=4 sections above need qualifying. Both were found by the
re-run and are recorded here rather than by editing the original text, so the
sequence of what was believed when stays legible.

### 1. The destructive edit was FIXTURE-DEPENDENT, not a stable adapter property

The n=4 write-up presents `too_many_arguments` deleting parameter `f` as a live
example of route 2. On the compiling fixture the same lint, same checkpoint, same
decoder produced a **different failure**: it echoed the input back
(`pub fn f(a: u8, …, h: u8) { let _ = (a, b, c, d, e, f, g, h); }`), the lint
still fires, and it scores REGRESSION rather than a destructive win.

So the destructive edit **did not reproduce once the input changed shape**. The
route-2 hazard is real and the harness must keep detecting it — but this run
provides no evidence that this adapter reliably produces destructive edits, and
the n=4 reading ("1 destructive in 4") should not be quoted as a rate. **A single
observation of a failure mode on a single fixture is an existence proof, not a
frequency.**

### 2. The contract-preservation detector is validated on a real generation

`redundant_clone` is the case that decides whether the detector's central design
choice was right. The adapter emitted

```rust
pub fn f() { let s = String::from("hi"); drop(s); }
```

which is the **correct** fix — and it works by *deleting the local binding `s2`*.
A naive "no identifier from the input may disappear" rule would have flagged this
as destructive and scored a correct fix as harm. The shipped rule excludes local
`let` bindings and counts only contract identifiers (fn names, parameter names,
type names, field names, variant names), so it passed the case as a **WIN**.

That is the design decision justified against real data rather than against my own
reasoning, and it is why the exclusion is not a loophole: parameters and fields
are the item's interface, locals are implementation detail, and several clippy
lints are *defined* as "remove this local".

## Next lever (not run here)

The scoreable denominator is 4, which is too small to carry a promotion
decision either way. The cheapest improvement is **not** another training run —
it is a reachable-set fixture whose cases actually compile as a lib crate, so
all 9 (or more) are scoreable. That is a fixture change, costs no GPU, and would
raise `n` before anyone spends another 4 GPU-hours on rank 3. Arm 2 localises
exactly what to fix: the fixtures that `RustAnalyzerPruner` accepts but `cargo`
cannot build.

Second lever, required before any `p_win` is admissible anywhere: a
**semantic-preservation check**. Even a crude one — "the output must not delete a
parameter, field, or statement present in the input" — separates route 2 from a
genuine fix. Without it, every WIN in this repo is an upper bound.
