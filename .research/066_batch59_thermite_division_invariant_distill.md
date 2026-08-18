# Batch 59 — Thermite External-Source Distill: Division by Invariant Multiplication

**Status:** RECORD (batch landed same session, 2026-08-18)
**Source:** [raygon-renderer/thermite](https://github.com/raygon-renderer/thermite) (`crates/thermite/src/divider.rs`, derived from libdivide by ridiculous_fish / Kim Walisch)
**Verdict recap (via the `distill` skill):** YES, grade B+ — route `rust_perf` (NOT kernel_opt: CPU SIMD ≠ GPU kernels). Ranked candidates: divider A (GOAT), capability-gated-approx-op B, double-pump B−, input-domain gamma selection C, branch-hint-cold-fn C; drops MM_SHUFFLE macros + packed-struct-unaligned-read. Yield estimate 1-3 rules; this batch landed 1 (the GOAT) — the B/B−/C candidates remain optional follow-ups, all weaker (approx-op needs `math/policy.rs` verified at mine time — two read timeouts; DoublePump is architectural with a narrow consumer base; likely/unlikely is PGO-class, weak measurability).

## The rule

`division-by-invariant-multiplication` (arithmetic, corpus 62 → 63, 53rd benchable):

- **Buggy:** loop of `x / d` where `d` is a RUNTIME value invariant across the loop (function parameter) — hardware integer division per element (20-90+ cycles, not vectorizable).
- **Fixed:** construct a `FastDiv{U32,U64}` once (magic multiplier + shift via Granlund–Montgomery / libdivide construction), then mulhi + shifts per element (~4 cycles, pipelined). Setup pays one hardware div — amortized after ~3 applications.
- **Two-regime guard (thermite's own split, load-bearing):** CONSTANT divisors need nothing — LLVM already strength-reduces them to multiplication at compile time. The win is exclusively the runtime-invariant case. A bench harness without `black_box` on the divisor measures NOTHING (the baseline gets const-folded into the same magic multiply).

## The two correctness bugs the equivalence gate caught (the batch's real lesson)

The G1 gate for this batch is an exhaustive equivalence test (`division_magic_matches_hardware_div_exhaustive_low_d`: all u32 d in 1..=4096 + all u64 d in 1..=257 + edge divisors incl. `u32::MAX`/`u64::MAX`/primes near 2^31/2^32/2^63/2^64, boundary numerators + LCG probes, asserting `fd.div(x) == x / d`). Both bugs fired BEFORE landing — the exact defend-wrong discipline:

1. **Floor vs ceil magic (`d=3 x=3`):** thermite's construction applies `proposed_m.wrapping_add(1)` on BOTH branches; I dropped the `+1` on the plain (non-add) branch, storing the FLOOR magic `0xAAAAAAAA`. The canonical u32 ÷ 3 magic is `0xAAAAAAAB` (ceil). Floor magic under-approximates — fails at exact multiples of `d`.
2. **u32 wrap carry loss (`d=7 x=u32::MAX`):** the ADD-branch decomposition `q = (mulhi_32(x, m') + x) >> (l+1)` reconstructs `floor(x·M/2^p)` where `M ∈ (2^32, 2^33)` — the true sum reaches ~2^33, so computing `hi.wrapping_add(x)` in u32 DROPS the carry bit (error 2^29 after the shift). The theorem holds in wide math; the fix is a widening add (`(u64::from(hi) + u64::from(x)) >> 1 >> shift`). This is very likely why libdivide's exact algorithm shape differs from the naive decomposition — a u32-only branchfree version needs a different magic to avoid the carry; our fixture takes the honest widening-add route (still mul + add + shifts, carry-safe). **Distillation lesson: a ported numeric algorithm without an exhaustive equivalence gate ships subtly wrong — the two failure cases (d=3, d=7) are invisible at casual glance and at small random tests.**

The corpus fixture, bench harness, and test mirror are pinned together by substring canaries (4 needles) so the three copies cannot drift.

## Bench (M3 Max, aarch64, release, n=1K/100K/1M)

| n | buggy | fixed | speedup |
|---|---|---|---|
| 1,000 | 1109 ns | 546 ns | 2.03× |
| 100,000 | 110437 ns | 51900 ns | 2.13× |
| 1,000,000 | 1111166 ns | 537959 ns | 2.07× |

**ParetoDominant 3/0/0**; allocs 0/0 both sides (G-det stable ×3). `context_notes` stays `None` (clean unconditional win; regime documented in the rule's comment block — the distance-sq precedent). Platform caveat: aarch64 UDIV is comparatively cheap; x86-64 (DIV 30-90+ cycles) should see a wider gap.

## Gates

- G1: parse + the exhaustive equivalence test (both bugs above caught + fixed pre-landing).
- G2: 572 default / 590 `rust_perf_bench` / 624 `latent_retrieval` lib tests, 0 failed.
- G3: clippy 0 in default + `rust_perf_bench` states; retrieval gate green — floors re-pinned (Batch 59: `PERF_SPANS` 52→53, blended denominator 100→101; measured blended 94/98/98 of 101, perf 51/52/52 of 53, router 71/101 — the new span retrieves its rule top-1).
- G5: ParetoDominant 3/0/0 (above).
- Proposer: DEFERRED (bench-verdict-first precedent — the fix is an architectural rewrite constructing a Divider type; proposer candidate for a follow-up, `FIXABLE_RULES` unchanged).

## Drops (with reasons)

- `capability-gated-approx-op` — source file `math/policy.rs` unread at verdict time (2 timeouts); not landed without verification.
- `simd-width-extension-register-pairing` (DoublePump) — architectural, narrow consumer base; suggestion-only grade.
- `branch-hint-cold-fn` (likely/unlikely via `#[cold]`) — PGO-class layout effect, weak measurability.
- `algorithm-selection-by-input-domain` (thermite-special tgamma) — the crate is mostly `todo!()`; module incomplete.
- MM_SHUFFLE const macros (ambient), packed-struct unaligned read (minor layout family).

## Distill-skill first flight notes

First batch executed end-to-end under the new `distill` skill: the verdict's ranked table + GOAT pick + drop list mapped 1:1 onto what landed; the recording rule (queue line + research note, same commit) executed here. The equivalence-gate-is-mandatory-for-ported-numeric-algorithms lesson is a candidate addition to the skill's constraints (a "ported algorithm ⇒ exhaustive equivalence gate" rule).

## Addendum (2026-08-18, same day) — policy.rs verified, remainder verdict MARGINAL C+

Re-invoked the `distill` skill on thermite after Batch 59. `math/policy.rs` (previously unread, 2 timeouts) now read: it is a `Policy` trait with `const POLICY: PolicyParameters` (5 axes — `check_overflow`, `unroll_loops`, `precision` (Worst→Reference 5-level enum), `avoid_branching`, `max_series_iterations`) + type-level adapters (`ExtraPrecision<P>`, `CheckOverflow<P, const bool>`, ...) + 6 named policies. **The original B-grade guess DOWNGRADES on read to C+**: it is an architectural API-design pattern (same class as DoublePump — unreachable by a proposer, narrow consumer base), not a localized fix. The one extractable nugget — per-element overflow/NaN domain checks hoisted behind a caller-declared precondition — is corpus-novel (dedup grep clean under `check_overflow|is_finite|policy|precondition|hoist.*check`) but Mixed-magnitude at best (`is_finite` = 1-2 well-predicted instructions; LLVM sometimes hoists). **Remainder verdict: MARGINAL, C+ — no further thermite batch.** Reopen trigger: a measured-GOAT shape appears (a checked-per-element loop in our own production hot path), not more library reading. Source closed as fully harvested.
