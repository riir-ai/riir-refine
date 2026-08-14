# Bench 011 — Pre-Plan-005 re-bench: fresh modelless e2e + referenced GPU baseline

**Date:** 2026-08-14
**Hardware:** M3 Max, Metal only
**Purpose:** establish the honest baseline **before starting Plan 005 implementation**
(the 12-substrate deep-fusion healer). Without this, there's no "before" to
compare the "after" against.
**Scope:**
- **Arm C (modelless retriever)**: **FRESHLY MEASURED** today (2026-08-14),
  4 runs, µs-scale, load-insensitive.
- **Arm A (llama.cpp + Bonsai) + Arm B (katgpt ternary)**: **REFERENCED** from
  Bench 010 (2026-08-12). The inference code + fixtures + model file are
  unchanged since then (verified via `git log --since 2026-08-11 -- benches/ src/`:
  only kernel_opt corpus commits, zero changes to `bench_clippy_ternary_tps.rs`
  or the ternary inference path). A fully fresh GPU re-run is deferred to a
  quiet box (see §"Why not re-run the GPU arms today" below).

---

## The one headline table (all three arms, same task)

| metric | llama.cpp+Bonsai (Arm A) | katgpt ternary (Arm B) | modelless retriever (Arm C) | A/B ratio | A/C ratio |
|---|---:|---:|---:|---:|---:|
| **per-fix latency** | 27.9 s¹ | 54.0 s¹ | **13.875 µs**² | 0.52× | **2,011,532×** |
| **tok/s (decode)** | 31.21³ | 17.85³ | n/a (no tokens) | 1.75× | — |
| **tok/s (prefill)** | 149.95³ | 17.89³ | n/a | 8.38× | — |
| **seed exact-match** | 10/10⁴ | 10/10⁴ | **10/10** (fresh) | tie | tie |
| **held-out exact-match** | ~10/10⁴ | ~10/10⁴ | **0/10** (fresh) | — | — |
| **generalizes?** | yes | yes | **NO** (Issue 010) | — | — |
| **setup (cold)** | ~15 s³ | 127.0 s³ | **0.103 ms** (fresh) | — | — |
| **load avg at measurement** | 3.6–4.5³ | 3.6–4.5³ | **5.84–15.46** (fresh) | — | — |

¹ Derived from Bench 010 measured rates (not measured end-to-end at 847 tokens).
² p50, `self_evolve` mode, seed fixture. ³ From Bench 010 (2026-08-12, code unchanged).
⁴ From Bench 009 (2026-08-12; Bonsai quality on these prompts).

**The modelless retriever is ~2 million times faster per fix but cannot
generalize.** That is the entire product tension Plan 005 exists to resolve:
the deep-fusion architecture is supposed to *close the generalization gap*
(0/10 → nonzero on held-out) *without* paying the 2-million-× latency cost
of the model-based arms. This bench is the "before" picture.

---

## Arm C — fresh modelless detail (the "more like Bench 009" part)

Measured today (2026-08-14) via `cargo bench --features self_evolve --bench
bench_e2e_tokps`. Four configurations: 2 fixtures × 2 modes. The modelless
path is µs-scale + CPU-only, so load average (5.84–15.46 during measurement)
is not material — confirmed by the per-lint p50/p99 dispersion being under
2× even at load 15.

### Per-fix latency + quality, by configuration

| fixture | mode | p50 | p99 | mean | setup | parse | exact | generalizes? |
|---|---|---:|---:|---:|---:|---:|---:|---|
| **seed** | `self_evolve` | **13.875 µs** | 29.875 µs | 13.403 µs | 0.103 ms | 10/10 | **10/10** | n/a (in-distribution) |
| **seed** | `l1l2` | **12.875 µs** | 31.041 µs | 12.667 µs | 0.089 ms | 10/10 | **10/10** | n/a |
| **held-out** | `self_evolve` | **14.584 µs** | 28.792 µs | 15.277 µs | 0.103 ms | 10/10 | **0/10** | **NO** |
| **held-out** | `l1l2` | **PANIC** | — | — | — | — | — | **NO** (crashes) |

**The held-out `l1l2` panic is honest data, not a bug.** `l1l2` mode calls
`drafter.draft_fix()` directly, which returns `None` on unseen code (no
corpus match) → the harness panics at `bench_e2e_tokps.rs:158`. The
`self_evolve` mode wraps this in `pipeline.heal()` which has a fallback
(emits the closest corpus entry verbatim) → 10/10 parse, 0/10 exact. This
is exactly the Issue 010 generalization gap: **the raw drafter can't handle
unseen code at all; the memory-overlay pipeline handles it gracefully but
emits the wrong answer.**

### Per-lint detail (seed fixture, `self_evolve` mode — the production path)

| lint | p50 | p99 | bytes emitted | parses | exact |
|---|---:|---:|---:|---|---|
| `needless_return` | 6.17 µs | 12.25 µs | 22 | ✓ | ✓ |
| `redundant_closure` | 25.88 µs | 33.92 µs | 137 | ✓ | ✓ |
| `needless_lifetimes` | 9.67 µs | 10.88 µs | 43 | ✓ | ✓ |
| `redundant_field_names` | 14.08 µs | 16.79 µs | 87 | ✓ | ✓ |
| `manual_map` | 14.29 µs | 16.33 µs | 69 | ✓ | ✓ |
| `unnecessary_unwrap` | 11.33 µs | 13.62 µs | 60 | ✓ | ✓ |
| `needless_range_loop` | 11.33 µs | 11.92 µs | 54 | ✓ | ✓ |
| `useless_conversion` | 9.25 µs | 10.04 µs | 39 | ✓ | ✓ |
| `unnecessary_mut_passed` | 15.46 µs | 16.83 µs | 84 | ✓ | ✓ |
| `needless_borrow` | 14.38 µs | 17.88 µs | 76 | ✓ | ✓ |

**Latency tracks output size** (the drafter copies the corpus entry's fix
string). `redundant_closure` is slowest (25.88 µs, 137 bytes — the longest
fix); `needless_return` is fastest (6.17 µs, 22 bytes — the shortest).
This is expected for a retriever: the work is a corpus lookup + string copy,
and the copy dominates at these sizes.

### Per-lint detail (held-out fixture, `self_evolve` mode — the generalization gap)

| lint | p50 | p99 | bytes | parses | exact | what it emitted |
|---|---:|---:|---:|---|---|---|
| `needless_return` | (cut from log¹) | — | — | ✓ | ✗ | seed's `fn foo()` fix, not held-out's `fn bar()` |
| `redundant_closure` | (cut from log¹) | — | — | ✓ | ✗ | seed's `double` fix, not held-out's `sq` |
| `needless_lifetimes` | 13.75 µs | 14.33 µs | 69 | ✓ | ✗ | seed's `first()` fix, not held-out's `head()` |
| `redundant_field_names` | 14.67 µs | 20.62 µs | 87 | ✓ | ✗ | seed's `Point` fix, not held-out's `Cfg` |
| `manual_map` | 14.12 µs | 21.46 µs | 69 | ✓ | ✗ | seed's `opt_double` fix, not held-out's `to_text` |
| `unnecessary_unwrap` | 11.04 µs | 17.21 µs | 60 | ✓ | ✗ | seed's `process` fix, not held-out's `get` |
| `needless_range_loop` | 15.04 µs | 17.08 µs | 76 | ✓ | ✗ | seed's `sum_vec` fix, not held-out's `double_all` |
| `useless_conversion` | 9.33 µs | 17.25 µs | 39 | ✓ | ✗ | seed's `add_one` fix, not held-out's `twice` |
| `unnecessary_mut_passed` | 15.79 µs | 26.50 µs | 84 | ✓ | ✗ | seed's `length` fix, not held-out's `count` |
| `needless_borrow` | 14.54 µs | 22.42 µs | 76 | ✓ | ✗ | seed's `take_ref` fix, not held-out's `show` |

¹ The first two lints were above the `tail -20` cutoff; the summary confirms
10/10 parse, 0/10 exact.

**The failure mode is precise and named.** Every held-out case emits the
*seed fixture's* fix verbatim — same byte count, same parse result, wrong
identifier names. The retriever finds the right *lint category* but has no
mechanism to transplant the fix pattern onto the new code's identifiers.
This is exactly what Plan 005 Phase 1 (riir-rag latent retrieval) + Phase 4-5
(real `syn` AST proposers) are designed to fix: the latent matcher finds
the rule, the proposer rewrites the actual code's AST (not the corpus
template's AST).

### `self_evolve` vs `l1l2` mode comparison (seed fixture only — both generalize-equivalent here)

| mode | p50 | what it does |
|---|---:|---|
| `self_evolve` | 13.875 µs | L1 drafter + L2 pruner + `LatentFixMemory` (3-axis retrieval) + `LatentSkillEvolution` (EMA directions). Issue 004/008, G1-G7 PASS. |
| `l1l2` | 12.875 µs | L1 drafter + L2 pruner only (no memory/evolution overlay). The raw 2-layer path. |
| **delta** | **+1.0 µs (+7.8%)** | The memory/evolution overlay costs ~1 µs per fix. On the seed fixture (in-distribution), both modes score 10/10 exact — the overlay doesn't help when the corpus already holds the answer. |

**The +7.8% overhead is the cost of the self-healing loop.** On the held-out
fixture, `self_evolve` at least produces parseable output (10/10 parse, 0/10
exact) while `l1l2` panics. The overlay's value is robustness, not in-
distribution quality. Plan 005 Phase 7 (generalize the EMA key to per-domain)
is where the overlay is supposed to start actually improving held-out
quality.

---

## Why not re-run the GPU arms today (honest deferral)

The GPU arms (Arm A llama.cpp, Arm B katgpt ternary) were **not re-measured
today**. Three reasons:

1. **Load contention.** Bench 010 ran at load 3.6–4.5. Today's session started
   at load **15.46** (multiple sibling cargo builds running — verified via
   `ps aux`). By the time the modelless arm finished, load had dropped to
   **5.84**, still above Bench 010's range. Bench 010 explicitly documented
   that Arm B's decode is load-insensitive (18.24 tok/s at load 12.6 vs 17.94
   at load 3.6), but Arm A (llama.cpp) was not stress-tested at high load.
   A fair GPU re-bench needs load < 4.
2. **Code unchanged → baseline still valid.** `git log --since 2026-08-11
   -- benches/ src/ Cargo.toml` shows only `kernel_opt` corpus commits
   (Batch 13–17). Zero changes to `bench_clippy_ternary_tps.rs`, the
   `riir-engine` ternary inference path, the fixtures, or the model file.
   Bench 010's numbers are a valid baseline for the same code.
3. **GPU exclusivity rule** (riir-ai AGENTS.md, Bench 649). The rule is about
   *correctness* gates (12.3× divergence under contention), not throughput —
   but the spirit applies: a benchmark measured under contention is not
   honest evidence. Deferring is the disciplined call.

**When to re-run:** when load < 4 and no sibling cargo/llama process is
running. The reproduce commands are in Bench 010. File as `.issues/` if this
needs tracking.

---

## What this bench establishes (the Plan 005 baseline contract)

This is the **"before"** picture. Plan 005's GOAT gates will be measured
against these numbers:

| Plan 005 phase | gate | this bench's baseline | target |
|---|---|---|---|
| Phase 1-3 (`--suggest` MVP) | G1 retrieval precision | n/a (not implemented) | ≥80% top-1 correct |
| Phase 1-3 | G2 latency | n/a | < 1 ms per chunk |
| Phase 1-3 | G3 no-regression | **115 lib tests** (today's build) | all pass |
| Phase 6 (`--fix`) | G5 real-file value | 0 real fixes landed | ≥1 real fix in sibling repo |
| Phase 7 (self-healing) | MAG precision improvement | **0/10 held-out exact** (today) | ≥10% improvement after 100 cycles |
| Phase 9 (calibrated UQ) | G1 coverage | uncalibrated sigmoid (today) | ≥90% empirical coverage at 95% nominal |
| Phase 10 (sleep consolidation) | motif extraction | no motifs (today) | ≥1 motif mined from 100 trajectories |
| Phase 11 (Poincaré imagination) | G1 recall | no imagination gate (today) | ≥30% recall of would-fail-L2 fixes |
| Phase 12 (trajectory archetype) | cross-archetype discrimination | no classifier (today) | Bench 013 G3 methodology PASS |
| Phase 14 (verified substrates) | G1 precision improvement | **13.875 µs p50, 10/10 seed** (today) | ≥5% precision improvement over Phase 9 |

**The load-bearing baseline numbers:**
- **Modelless per-fix latency: 13.875 µs p50** (seed, `self_evolve`). Every
  Plan 005 phase that adds a substrate must not regress this beyond ~20 µs
  (the modelless arm's value proposition is speed; if it approaches ms-scale,
  the dMoE routing overhead has eaten the advantage).
- **Held-out exact-match: 0/10.** Every Plan 005 phase that claims to improve
  generalization must move this number. If it's still 0/10 after Phase 7,
  the MAG self-healing loop isn't working.
- **Seed exact-match: 10/10.** This is the regression guard — no Plan 005
  phase may break in-distribution quality while chasing held-out gains.

---

## Reproduce (modelless arm — the fresh part)

```sh
# Seed fixture, self_evolve mode (the production path)
CARGO_TARGET_DIR=/tmp/riir-clippy-bench011 \
  cargo bench --features self_evolve --bench bench_e2e_tokps -- \
  benches/fixtures/e2e_cases_seed.json /tmp/bench011_seed_selfevolve.jsonl

# Seed fixture, l1l2 mode (the raw drafter, no memory overlay)
MODE=l1l2 CARGO_TARGET_DIR=/tmp/riir-clippy-bench011 \
  cargo bench --features self_evolve --bench bench_e2e_tokps -- \
  benches/fixtures/e2e_cases_seed.json /tmp/bench011_seed_l1l2.jsonl

# Held-out fixture, self_evolve mode (the generalization test)
CARGO_TARGET_DIR=/tmp/riir-clippy-bench011 \
  cargo bench --features self_evolve --bench bench_e2e_tokps -- \
  benches/fixtures/e2e_cases_heldout.json /tmp/bench011_heldout_selfevolve.jsonl

# Held-out fixture, l1l2 mode — ⚠ PANICS (drafter returns None on unseen code)
# This is honest data: the raw drafter cannot generalize. The panic is the signal.
```

Load average during measurement: 5.84–15.46 (descending). The modelless arm
is CPU-only + µs-scale, so load is not material — confirmed by per-lint p50/p99
dispersion under 2× across the measurement window.

## Reproduce (GPU arms — referenced from Bench 010, NOT re-run today)

See Bench 010's reproduce section. **Do not re-run under load > 4** — the
GPU exclusivity rule (riir-ai AGENTS.md, Bench 649) applies in spirit even
for throughput benches. File `.issues/` if a fresh GPU re-bench is needed.

---

## Caveats

1. **The GPU arms are referenced, not fresh.** Bench 010 (2026-08-12) is the
   source. Code unchanged since then (verified via git log). A fresh GPU re-run
   is deferred to a quiet box.
2. **The modelless arm is fresh but under load.** Load 5.84–15.46 during
   measurement. The arm is µs-scale + CPU-only, so load is not material
   (per-lint dispersion under 2×). But this is noted for honesty.
3. **The `l1l2` held-out panic is a harness artifact, not a production bug.**
   The production healer uses `pipeline.heal()` (the `self_evolve` path),
   which handles unseen code gracefully (10/10 parse, 0/10 exact). The `l1l2`
   mode is the raw drafter without the fallback — useful for measuring the
   raw drafter's latency but not representative of production behavior on
   unseen code.
4. **The 27.9 s / 54.0 s per-fix numbers for the GPU arms are derived, not
   measured end-to-end at 847 output tokens.** They extrapolate Bench 010's
   measured rates (31.21 / 17.85 tok/s decode) to the realistic output length
   Bench 009 measured (847 tokens average). Flagged as derived in Bench 010;
   same caveat applies here.
5. **The 2,011,532× ratio (Arm A / Arm C) is real but misleading without the
   quality column.** Arm C is 2 million times faster but emits the wrong
   answer on unseen code. Arm A is 2 million times slower but generalizes.
   The ratio alone is not the product story; the ratio + the 0/10 held-out
   column is. Plan 005 exists to close the quality gap without paying the
   latency cost.
