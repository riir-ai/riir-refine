# Issue 032 — needless_range_loop × identity_op/double_parens composition breaks the loop variable

**Found by:** clippy-bench run #1 (Plan 014, 2026-08-19) — the bench's first
run caught this; `parse_safety` 92.9% (2/28 edited fixtures produced
non-compiling output).

**Status:** OPEN

## Repro (frozen bench fixtures `composite-range_loop_plus_identity_op` + `composite-range_loop_plus_double_parens`)

```rust
pub fn total_damage(hits: &[u32]) -> u32 {
    let mut total = 0;
    for i in 0..hits.len() {
        total += hits[i] * 1;          // identity_op fires here too
    }
    total
}
```

`--fix` output (broken — does not compile):

```rust
    for __elem in hits {
        total += hits[i];              // `i` no longer exists
    }
```

## Root cause (first pass)

The `needless_range_loop` proposer rewrites `for i in 0..v.len()` into
`for __elem in v` and rewrites the body indexing `v[i]` → uses of `__elem`.
In the composition, `identity_op` (or `double_parens`) edits the SAME body
expression first (`hits[i] * 1` → `hits[i]`); the range-loop proposer's
body rewrite then misses its expected shape, renames the loop variable, and
leaves the still-indexing body verbatim → `cannot find value i`.

Same-pass overlapping edits normally get dropped by `apply_edits_traced`'s
overlap selection — the breakage needs the fixpoint interaction (pass 1
identity_op, pass 2 range-loop) where the second pass's body-rewrite
expectation was invalidated by the first pass's edit.

## Fix direction

The range-loop proposer must not assume the pristine shape of the body
expressions it rewrites: after renaming the loop variable it must rewrite
EVERY `v[i]` occurrence in the body (or refuse when the body references
`i` in any way it did not itself produce — the map_unwrap_or
array-literal-guard precedent: refuse what you cannot see).

## Regression gate

The two frozen fixtures ARE the gate — clippy-bench `parse_safety` must
return to 100% when this is fixed (the SVG curve will show the step).
