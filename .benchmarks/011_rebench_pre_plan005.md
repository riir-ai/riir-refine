# Bench 011 — Pre-Plan-005 re-bench: fresh modelless e2e + referenced GPU baseline

**Date:** 2026-08-14
**Hardware:** M3 Max, Metal only
**Purpose:** establish the honest baseline **before starting Plan 005 implementation**
(the 12-substrate deep-fusion healer). Without this, there's no "before" to
compare the "after" against.
**Scope:**
- **Arm C (modelless retriever)**: **FRESHLY MEASURED** today (2026-08-14),
  4 runs, µs-scale, load-insensitive.
- **Arm A (llama.cpp + Bonsai) + Arm B (katgpt ternary)**: **RETRACTED — referenced
  from Bench 010 (2026-08-12) but that reference is STALE.** The upstream inference
  stack (`riir-ai/crates/riir-gpu`) has 20+ commits since 2026-08-11 including:
  - `28317ac8b` — zero-copy RAW-Metal matmul2d prefill (Issue 663 T5, **1.21× measured**, landed 2026-08-14 09:55)
  - `8512cd711` — speculative decoding substrate for Metal decode (riir-ai Issue 665 T1-T6, **1.21× at 88% acceptance**, landed 2026-08-14 13:06 — hours ago)
  - `56595bbaf`, `5a67e96f1`, `a4b5785e1` — bind group cache + dispatch profiling + decode breakdown
  - `46a2eca60` — cuda_graphs_forward promoted to DEFAULT-ON
  - `b12a692be`, `5cce1807a` — chunked conv1d into prefill
  **Bench 010's numbers (17.85 tok/s decode, 17.89 prefill) predate ALL of these.**
  Additionally, the bench harness (`bench_clippy_ternary_tps.rs`) only enables
  `ternary_gemv` (the minimal GEMV path) — it does NOT exercise `ternary_gemm_batched`,
  `metal_tensor_gemm`, or `speculative_decode`. A fresh GPU re-bench on "full recent
  features" requires BOTH a re-run AND harness feature updates. See §"GPU arms
  re-bench plan" below.

---

## The one headline table (all three arms, same task)

| metric | llama.cpp+Bonsai (Arm A) | katgpt ternary (Arm B) | modelless retriever (Arm C) | A/B ratio | A/C ratio |
|---|---:|---:|---:|---:|---:|
| **per-fix latency** | ~~27.9 s~~¹ ⁵ | ~~54.0 s~~¹ ⁵ | **13.875 µs**² | ~~0.52×~~⁵ | **2,011,532×** |
| **tok/s (decode)** | ~~31.21~~³ ⁵ | ~~17.85~~³ ⁵ | n/a (no tokens) | ~~1.75×~~⁵ | — |
| **tok/s (prefill)** | ~~149.95~~³ ⁵ | ~~17.89~~³ ⁵ | n/a | ~~8.38×~~⁵ | — |
| **seed exact-match** | 10/10⁴ | 10/10⁴ | **10/10** (fresh) | tie | tie |
| **held-out exact-match** | ~10/10⁴ | ~10/10⁴ | **0/10** (fresh) | — | — |
| **generalizes?** | yes | yes | **NO** (Issue 010) | — | — |
| **setup (cold)** | ~~~15 s~~³ ⁵ | ~~127.0 s~~³ ⁵ | **0.103 ms** (fresh) | — | — |
| **load avg at measurement** | ~~3.6–4.5~~³ ⁵ | ~~3.6–4.5~~³ ⁵ | **5.84–15.46** (fresh) | — | — |

¹ Derived from Bench 010 measured rates (not measured end-to-end at 847 tokens).
² p50, `self_evolve` mode, seed fixture. ³ From Bench 010 (2026-08-12). 
⁴ From Bench 009 (2026-08-12; Bonsai quality on these prompts).
⁵ **STALE — see retraction above.** Bench 010's GPU numbers predate the batched
   Metal prefill (Issue 663, +1.21×) + speculative decode (riir-ai Issue 665, +1.21× at
   88% acceptance) + multiple GPU perf commits. The Arm A/C latency ratio is
   directionally correct (modelless is millions of × faster) but the exact
   number is not trustworthy. A fresh GPU re-bench is REQUIRED before any
   Plan 005 phase claims a GPU-side improvement.

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

## UPDATE (2026-08-14, session 2): FRESH GPU numbers — batched prefill measured

After the user correctly called out that I was blindly referencing stale Bench 010
numbers, I re-measured Arm B (katgpt ternary) with the harness updated to use
the **batched simdgroup cmma GEMM prefill path** (the production DEFAULT-ON
feature in riir-gpu that the old harness was NOT exercising).

**Load at measurement: 5.54, 0 sibling cargo/llama processes.**

| metric | Bench 010 (stale, GEMV loop) | **FRESH (simdgroup batched)** | improvement |
|---|---:|---:|---:|
| **prefill tok/s** | 17.89 | **25.493** | **+42.5% (1.42×)** |
| **decode tok/s** | 17.85 | 17.610 | ~same (decode path unchanged — speculative not yet wired) |
| **total tok/s** | 17.88 | **21.998** | **+23.0% (1.23×)** |
| prefill gap to llama.cpp | 8.38× | **5.88×** | closed by 2.5× |

**The harness was measuring the WRONG path.** The old loop
`for tok in prompt_tokens { forward_token(tok) }` was the one-token-at-a-time
GEMV path — the slow path. The batched `fwd.prefill(prompt_tokens)` API
(processes all tokens via `prefill_project` using the simdgroup cmma GEMM
kernel) is the production DEFAULT-ON path. Switching the harness to it gave
**+42.5% prefill speedup for free** — zero code change to the inference engine,
just using the API that was already there.

### What's NOT yet measured (honest follow-ups)

1. **`metal_tensor_gemm` zero-copy overlay (Issue 663) — REGRESSION in this harness, NOT shipped.**
   When enabled, the zerocopy path:
   - **decode 17.2 → 0.37 tok/s (47× slower)** — likely GPU memory pressure
     from the zerocopy weight cache at 27B model scale (the cache is uploaded
     at init even when the runtime flag is off, just from having the feature
     compiled).
   - **prefill 25.0 → 13.4 tok/s (regression)** — the OPPOSITE of Issue 663's
     claimed +1.21×.
   - **G1 FAILS** on `needless_return` (emits `" T, p+1-1-1-1..."` garbage).
   Issue 663's +1.21× was measured on an ISOLATED prefill bench
   (`bench_663_zero_copy_interop.rs`), not the full 27B forward. **The gain
   does NOT reproduce at production scale.** This is an Issue 663 follow-up:
   the zerocopy path needs to be revalidated against the full forward, not
   just the isolated kernel. ZEROCOPY=1 env var forces it on for A/B
   re-investigation. Filed in riir-ai as a follow-up to Issue 663.
2. **`speculative_decode` (riir-ai Issue 665)** — the feature is compiled in but the
   harness still uses the sequential decode loop. Wiring
   `forward_speculative_verify` is the next harness update. The riir-ai Issue 665
   benchmark measured +1.21× at α=0.88 (thermal-throttled lower bound).
   **UPDATE (2026-08-14, session 3 — WIRED, honest NEGATIVE):** commit
   `c5660dc` added the `SPECULATIVE=1` arm (n-gram, K=2) to this harness.
   Result on clippy prompts: **α = 0.19–0.22** (natural-language `<think>`
   reasoning) — far below the α≥0.58 breakeven; decode **3.3–4.4× SLOWER**
   than sequential. riir-ai Issue 665's +1.21× (repetitive text) does NOT
   reproduce on this workload — **NET-NEGATIVE for clippy fix
   generation**. G1 (bit-identical emitted text vs sequential) PASSES
   after fixing the bench_665 off-by-one verify bug (argmax index vs
   draft index — filed as riir-ai Issue 667); `51d3586` then dropped the
   `current_logits` threading (K+1 position-aligned logits). Throughput
   measured under GPU contention — provisional per the Bench 649 rule;
   exclusive-GPU re-run pending. Mirrors Bench 012's DSpark finding:
   both speculative arms lose on clippy-class prompts.
3. **The reproducible win is simdgroup batched prefill: +42.5% (1.42×).** This
   is the production DEFAULT-ON path. The gap to llama.cpp on prefill closed
   from 8.38× to 5.88×.

### The lesson

**Refusing +21% because it isn't +50% is gate malpractice when you're 6× behind.**
The 1.5× GOAT threshold is a quality bar for DEFAULT promotion, not a deployment
bar for benchmarking. When measuring against a competitor (llama.cpp), you
ship every honest G1-passing gain you have, stack them, and report the compound.
The old harness was measuring the slow path AND I was refusing to enable the
fast features — double penalty.

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

1. **~~The GPU arms are referenced, not fresh.~~** **RETRACTED.** Bench 010's
   GPU numbers are STALE — the upstream inference stack (`riir-gpu` in riir-ai)
   has 20+ commits since 2026-08-11 including batched Metal prefill (Issue 663,
   +1.21×) + speculative decode (riir-ai Issue 665, +1.21× at 88% acceptance). The
   GPU arms MUST be re-bench with updated harness features before any Plan 005
   phase claims a GPU-side improvement. See §"GPU arms re-bench plan" above.
2. **The modelless arm is fresh but under load.** Load 5.84–15.46 during
   measurement. The arm is µs-scale + CPU-only, so load is not material
   (per-lint dispersion under 2×). But this is noted for honesty.
3. **The `l1l2` held-out panic is a harness artifact, not a production bug.**
   The production healer uses `pipeline.heal()` (the `self_evolve` path),
   which handles unseen code gracefully (10/10 parse, 0/10 exact). The `l1l2`
   mode is the raw drafter without the fallback — useful for measuring the
   raw drafter's latency but not representative of production behavior on
   unseen code.
4. **The 27.9 s / 54.0 s per-fix numbers for the GPU arms are STALE** — they
   extrapolate Bench 010's now-invalid rates. Do not use them.
5. **The 2,011,532× ratio (Arm A / Arm C) is directionally correct but the
   exact number is not trustworthy** because the numerator (Arm A latency)
   is stale. The modelless arm is millions of × faster — that direction
   holds. The exact ratio will change after the GPU re-bench. The product
   tension (fast-but-no-generalize vs slow-but-generalize) is unchanged.
