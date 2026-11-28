# Issue 262: cargo refine fix healed Rust-2024 let-chains into an edition-2021 crate (collapsible_if class)

**Status:** Open — the mis-heal is hand-fixed at the consumer (riir-infer `i1254-p0a-p0b` commit `3992a74`, 2026-11-28); this issue owns the PROPOSER-side defect so the class cannot re-fire. Unclaimed — the repair belongs in the rulewright's collapsible_if proposer + its compile-gate posture (the `fix_verify` lane that failed to catch it).
**Claimed:** zed-r1262@amo, 1795867211, expires 2026-11-30T12:00:11+00:00

## The incident (2026-11-28, riir-infer lane 1254, session zed-3591ad54@4090)

`cargo refine fix --verify-args "--features deltanet_ternary_inference" src/bin/i1254_capture.rs`
run against the riir-infer worktree `E:/git/wt-i1254` (riir-infer-core is **edition 2021**,
rustfmt-default style, toolchain 1.99.0) reported:

```
fix src/bin/i1254_capture.rs (4 edit(s) verified):
  [collapsible_if          ] line 217
  [manual_is_multiple_of   ] line 297
  [collapsible_if          ] line 410
  [manual_is_multiple_of   ] line 508
verify: 1 file(s) / 4 edit(s) — baseline + compile-gated apply
verify: 2 cargo check(s) — kept 1 file(s) / 4 edit(s), reverted 0 edit(s)
```

The two `collapsible_if` heals produced **Rust-2024 let-chains** in the edition-2021
crate:

```rust
// emitted (does NOT compile at edition 2021 — E0658-class, "let chains are
// only allowed in Rust 2024 or later"):
if let Some(l) = limit
    && attempted >= l
{
    break;
}
```

The next clippy run at the consumer failed to compile (`let chains are only
allowed in Rust 2024 or later`, two sites) — the mis-heal was hand-repaired with
`limit.is_some_and(|l| attempted >= l)` in riir-infer commit `3992a74`.

## The two defects this issue owns

1. **The proposer emits edition-incompatible syntax.** The collapsible_if
   fixer's collapsed form must respect the TARGET CRATE's edition: at edition
   ≤ 2021 the legal collapse is `if limit.is_some_and(|l| attempted >= l)` (or
   a nested-if keep), never a let-chain. The proposer needs the edition from
   the crate's manifest (or the target's `--edition`), and a guarded template
   per edition class.
2. **The compile gate read a posture that compiled the mis-heal.** `verify`
   reported "kept, reverted 0" over "2 cargo check(s)" — yet the emitted file
   does not compile at the crate's own edition. The verify lane's check must
   have run against a context where the let-chain parsed (a 2024-edition
   scratch, a different crate, or a check that never re-read the written
   file) — find which, and make the gate refuse LOUD instead of KEEP on any
   non-compiling write. The `unused_parens` twin from the same run (reported
   at line 434, not applied) is the second oracle-visibility miss in the same
   session: oracle findings listed but not applied should surface in the run
   summary as named-unapplied, never folded into a green.

## Repro

riir-infer at `3992a74^` (the file state before the consumer's hand-fix):
`src/bin/i1254_capture.rs` lines ~217 and ~410 held the nested
`if let Some(l) = limit { if attempted >= l { break; } }` shape. Re-run
`cargo refine fix` with `--verify-args "--features deltanet_ternary_inference"`
against an edition-2021 crate to reproduce.

## Acceptance

- the proposer emits edition-appropriate collapse forms (fixture: an
  edition-2021 crate fixture + an edition-2024 one, both asserted);
- `verify` cannot report KEPT for a written file that fails `cargo check` at
  the crate's own edition (the P104 unverified-kept audit is the precedent
  shape);
- the riir-refine score-bench lane runs green after the repair.
