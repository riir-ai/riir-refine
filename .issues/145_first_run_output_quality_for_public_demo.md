# Issue 145 — first-run output quality: readable loop binding + no off-domain suggestions in a dry run
**Status:** OPEN — 2026-10-03 (found generating the refine.gist.rs demo capture, riir-dapps Issue 112 T1)

**Priority:** P1 — riir-dapps Issue 112 T1 (a GENERATED `cargo refine` dry run + `--fix` diff on
refine.gist.rs, the numbers law: never typed) is blocked on this: today any honest capture shows
both defects below to a first-time visitor.
**Audience:** every first-run user; the public demo capture.

## Repro (cargo-refine 0.2.0, logged out — HOME=$(mktemp -d), so nothing is reported or billed)

A fresh lib crate, one file:

```rust
/// Sums the even values.
pub fn even_sum(values: &Vec<u64>) -> u64 {
    let mut total = 0;
    for i in 0..values.len() {
        if values[i] % 2 == 0 {
            total += values[i];
        }
    }
    return total;
}
```

## Defect 1 — `needless_range_loop` fix emits `__elem`

`cargo refine --fix` (verified, compile-gated, 3 edits kept) produces:

```diff
-pub fn even_sum(values: &Vec<u64>) -> u64 {
+pub fn even_sum(values: &[u64]) -> u64 {
-    for i in 0..values.len() {
-        if values[i] % 2 == 0 {
-            total += values[i];
+    for __elem in values {
+        if *__elem % 2 == 0 {
+            total += *__elem;
```

Correct and compiling, but no Rust developer writes `__elem`; Clippy's own suggestion reads
`for <item> in values`. The fresh-binding choice is deliberate (collision-proof, see the
proposer's module doc) — the fix is to pick a READABLE fresh name with the same guarantee.

- [ ] T1 derive the binding from the collection (`values` → `value`, `items` → `item`, else
      `item`), fall back with a numeric suffix (`item2`) when the name appears anywhere in the loop
      body or enclosing fn; keep `__elem` only as the last resort
- [ ] T2 (optional) for a `Copy` element read only by value, `for &value in values` drops the
      `*` splices — only where param resolution proves `Copy`; otherwise keep `*value`
- [ ] T3 GOAT: the needless_range_loop arm set stays green (compile-gated), plus an arm pinning the
      readable name and the collision fallback

## Defect 2 — the dry run lists Docker / release-profile suggestions for a `.rs` span

The same dry run ("routed: perf") prints, under `./src/lib.rs:1-21`, three `[docker]` rows
(`musl-scratch-ca-certs`, `workdir-absolute-path`, `cache-mount-id-scope`) and three `[dist]`
rows (`large-api-dependency-mostly-unused-hint`, …) — "top-3 per domain" over every domain,
including ones that cannot apply to a Rust source span in a crate with no Dockerfile. Read cold,
it looks like the tool misread the file.

- [ ] T4 a span's suggestion list shows only domains whose file kind matches the span
      (`.rs` → clippy/perf/rustc; `Dockerfile*` → docker; `Cargo.toml` → dist), or labels the rest
      as "other domains (not this file)" behind a flag
- [ ] T5 arm: the repro above prints no `[docker]`/`[dist]` rows

## Also observed (not in scope here)

- `len_zero` (`l.len() > 0`) reports "NO CORPUS RULE (gap signal)" and `manual_is_multiple_of` is
  oracle-anchored but not fixed — expected coverage gaps, already the self-evolve queue's job.
