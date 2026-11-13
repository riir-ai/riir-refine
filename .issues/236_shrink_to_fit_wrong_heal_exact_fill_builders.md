# Issue 236: rulewright wrong-heal — `shrink-to-fit-after-build` fires on exact-capacity builders

**Date:** 2026-11-13
**Status:** OPEN — wrong-heal class observed live (the gate worked; the rule's admission needs an exact-fill guard)
**Claimed:** amo-idle-i236, 1794590181, expires 2026-11-15T17:16:21+00:00

## The observation

`cargo refine fix` on riir-infer's `int_ref` lane (Issue 1214 T1a, commit `7340300`) applied 6 edits of the
`shrink-to-fit-after-build` perf rule to functions whose output `Vec` is built
`Vec::with_capacity(n)` and then filled with EXACTLY n pushes on every path (no early exit, no conditional push):

- `ops/di_rmsnorm.rs:72` (`di_rmsnorm` output `y`)
- `ops/matmul.rs:53,87` (`int_gemm`, `int_gemm_u8i8` outputs)
- `ops/matmul.rs:134` (the test oracle `naive_i8` — a bench-gated oracle, doubly noise)
- `ops/rope.rs:99` (`apply_int_rope` output)
- `src/laya/riir/int_ref/tensor.rs:339` (`requant_i8_static` output)

Every one is a no-op (`shrink_to_fit` does nothing when `len == capacity`) and pure noise in a
reference lane; all 6 reverted by hand. Healed during the lawfully ordered run (dry run first, then
`fix --verify-args "--features int_ref"`); compile-gated verification passed, so nothing caught it.

## The wrong-heal class

The rule's matcher sees `Vec::with_capacity` + a builder loop and prescribes `shrink_to_fit()`
without asking whether the builder is EXACT-FILL by construction. When `len == capacity` holds on
every path (count-known builders: `with_capacity(m*n)` + an `m*n`-iteration push loop), the heal
is dead code the author then has to revert — noise that trains the wrong lesson.

## Suggested fix (R1)

Add an exact-fill admission to the proposer: only prescribe `shrink_to_fit()` when the builder
cannot be proven exact-fill from the immediate function body — e.g. skip when the `with_capacity`
argument is an arithmetic expression over the same loop bounds that drive the pushes (`m * n` vs
`for i in 0..m { for j in 0..n { push } }`), or when every push path is unconditional in an
exactly-`n`-iteration loop. Conservative default: if exact-fill cannot be DISPROVEN cheaply,
don't fire (the miss costs nothing; the wrong heal costs a revert + an issue).

## Secondary observation (R2)

The run summary reported `13% fixed · 86% unfixed (fixable — the fixer's gap)` for this lane —
the `needless_range_loop` findings (5 sites, all single-array-indexable after an `enumerate`/`zip`
rewrite) were in-corpus but not fixed. Manual rewrites landed in `7340300`. Not urgent: flagging
the gap rate on a fresh lane, since "fixable — the fixer's gap" is the self-evolve signal.

## Session

zed-41cd0608@amo, epoch 1794576200; the healed tree: riir-infer `7340300` (reverts included);
the rulewright run: `cargo refine fix --verify-args "--features int_ref" crates/riir-infer-laya/src/laya/riir/int_ref`.
