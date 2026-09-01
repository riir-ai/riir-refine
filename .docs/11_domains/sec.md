# `sec` — security-pattern healing (4th retrieval domain)

> Phase 0+1 shipped 2026-08-22 (Plan 018 / Proposal 004 / Research 078-079 /
> Bench 046). Opt-in feature `sec`, default-off — promotion needs measured
> consumer pull (the `rust_perf` precedent). **Phase 2 (Batch 88) landed
> 2026-08-29** — see below.

## What it is

Security-relevant **source-code patterns with bounded fix-spaces**: FFI
panic containment, Drop-glue panic safety, raw-pointer null hygiene, the
`from_raw_parts` len-zero contract, Rc backedge leaks — and since Phase 2:
unsound thread-safety impls, panicking FFI exports, weak-hash / password-
KDF hygiene, and constant-time secret comparison. NOT dependency auditing
(cargo-audit/cargo-deny), NOT a lint engine (dylint), NOT judgment-bearing
detection — every rule is parse-only provable with a bounded fix, or
suggest-only.

## The Phase-1 corpus (6 rules, 5 of 11 candidates killed)

| Rule | Family | `--fix` | The bounded fix |
|---|---|---|---|
| `ffi-extern-fn-catch-unwind` | `Ffi` | ✓ | wrap unit-return extern fn bodies in `std::panic::catch_unwind` (the in-house `#[riir_ffi]` rule generalized) |
| `ptr-deref-null-check-first` | `UnsafeHygiene` | ✓ | `let Some(p) = (unsafe { p.as_ref() }) else { return None };` prologue in Option-returning unsafe fns |
| `drop-impl-no-panic` | `DropLeak` | ✓ | `.unwrap()`/`.expect()` → `.unwrap_or_default()` inside drop glue (double-panic = abort) |
| `slice-from-raw-parts-len-guard` | `UnsafeHygiene` | ✓ | `if len == 0 { &[] } else { from_raw_parts(..) }` (ptr must be non-null even at len 0) |
| `extern-fn-reference-param-to-pointer` | `Ffi` | suggest-only | `&T` param → `*const T` + explicit deref (signature-changing — the C-side caller ripples) |
| `rc-backedge-field-prefer-weak` | `DropLeak` | suggest-only | direct `Rc<Self>` backedge field → `Weak<Self>` (constructor sites change too) |

## The Phase-2 corpus (Batch 88 — 5 rules, probe vs clippy 1.97)

All 5 candidates probed OPEN against real clippy 1.97.0 (pedantic +
restriction + nursery; zero equivalent-fix firings). The crypto trio is
**dep-gated** (the `fxhashmap` pattern: fires only when the file already
imports the replacement crate — the healer never edits `Cargo.toml`).

| Rule | Family | `--fix` | The bounded fix / gate |
|---|---|---|---|
| `unsafe-impl-send-sync-unsound-field` | `UnsafeHygiene` | suggest-only | `unsafe impl Sync/Send` on a same-file type with `Rc`/`RefCell`/`Cell` fields — the Rudra unsound-impl class clippy cannot see; fix = Arc/Mutex redesign (type-level, never mechanical) |
| `ffi-export-panicking-body` | `Ffi` | suggest-only | panicking body (unwrap/expect/panic/indexing/assert) in a `#[no_mangle] extern` fn — the VALUE-return complement of Phase-1's catch-unwind (unit-return); fix needs a caller-chosen sentinel |
| `weak-hash-to-blake3` | `CryptoHygiene` | suggest-only, dep-gated | SHA1/MD5 (broken) + SHA256 (house loser) digest calls → blake3; fires only when the file imports blake3 |
| `password-hash-argon2` | `CryptoHygiene` | suggest-only, dep-gated | fn named `*password*` calling a bare digest → argon2 (memory-hard KDF); name-based trigger — false-NEGATIVES honest and expected |
| `constant-time-secret-compare` | `CryptoHygiene` | yes (strict-gated) | `a == b` → `a.ct_eq(b).into()` via `subtle::ConstantTimeEq`; gates: file imports subtle + (enclosing-fn name OR operand name hits the secret vocabulary) + both operands plain idents + `#[cfg(test)]` mods skipped |

Honest scope notes: the R5 proposer is shape-based, so a `String`-typed
secret (subtle has no `ConstantTimeEq` for `String`) FIRES and then the
`--fix --verify` compile gate reverts it — the type backstop, recorded as
a fixture. The `!=` form stays out of Phase-2 scope.

## P8 intake (2026-09-01) — the 12th rule, new `PanicSafety` family

| Rule | Family | `--fix` | The bounded fix / gate |
|---|---|---|---|
| `nan-unsafe-sort-comparator` | `PanicSafety` | yes (strict-gated) | `partial_cmp(..).unwrap_or(Equal)` (or `.unwrap()`) inside a sort comparator → `total_cmp(..)` — NaN becomes deterministically ordered instead of equal-to-everything (an inconsistent order the std sorts MAY PANIC on since 1.81). Gates: enclosing fn contains a `sort_by`/`sort_unstable_by`/`select_nth_unstable_by` call + exact chain shape + `Equal`-only default + comment guard + macro args refuse. The arg carries over VERBATIM (`PartialOrd::partial_cmp` pins `(&self, other: &Self)` — the same shape `total_cmp` takes), so the output is the idiomatic human form; non-float receivers (`Option<f32>`, other Ord primitives) fail E0599 → the `--verify` gate auto-reverts. Probe 2026-09-01: zero clippy lints fire on the shape (real clippy 1.97, pedantic+nursery+perf+suspicious+correctness+style) — the boundary holds with margin. Source: riir-ai Issue 832 T5's P8 intake (≈300 sibling library sites); the intake's type-blindness concern dissolved by `total_cmp` (direction-preserving recv/arg order + name-identical on f32/f64) — recorded in the intake close-out. Honest recall limit: a comparator defined in a helper fn and passed to `sort_by(helper)` by name is not detected — the sort call and the chain must share the enclosing fn.

## The boundary discipline (non-negotiable)

`tests/sec_boundary_gate.rs` runs REAL clippy over the corpus fixtures with
the banned list enabled (`mem_forget`, `indexing_slicing`,
`arithmetic_side_effects`, `unwrap_used`/`expect_used`, `improper_ctypes*`,
`partialeq_ne_impl`, `undocumented_unsafe_blocks`, `disallowed_methods`):
no banned lint may fire MachineApplicable on a sec fixture — a future
clippy that auto-fixes one of our territories FAILS the gate and that rule
retires. The pinned allowed overlaps (all fire with NO suggestion — our
fixes are additive): `unwrap_used` on the drop fixture, `indexing_slicing`
on the panicking-FFI fixture, `undocumented_unsafe_blocks` on the
unsound-impl fixture.

## The unsafe counter (geiger taxonomy, zero deps)

`count_unsafe` / `UnsafeCounts` / `file_forbids_unsafe`: unsafe fns +
methods, attribute-unsafety (`#[no_mangle]`/`#[export_name]` on otherwise
safe fns), unsafe impls/traits, unsafe exprs. Feeds
`SecVerdict.unsafe_count_delta` — a review SIGNAL, never a gate.

## Negative control

riir-viewbridge IS the negative control: its whole FFI surface is
`riir_ffi!`-wrapped (catch_unwind inside), and the dry `--fix --domain sec`
run over its 24 files proposes 0 edits. A nonzero there falsifies the
proposer.

## Status

- Phase 0 (scaffold + wiring + boundary gate + unsafe counter): DONE
- Phase 1 (corpus + 4 proposers + GOAT): DONE (Bench 046)
- Phase 2 (unsafe + crypto hygiene): UNBLOCKED (≥6-rule gate met) but
  deferred per Plan 018 — a future batch
- Promotion to default: needs ≥1 workspace repo adopting sec in CI

## See also

- [Proposal 004](../../.proposals/004_sec_domain_fourth_heal_domain.md) — the design argument
- [Plan 018](../../.plans/018_sec_domain_phase0_batch1.md) — execution plan
- [Research 078](../../.research/078_sec_domain_seed_verdict_rust_secure_code_index.md) — the seed survey
- [Research 079](../../.research/079_sec_domain_phase0_1.md) — the Phase-0+1 record
- [Bench 046](../../.benchmarks/046_sec_domain_phase0_1_goat.md) — the GOAT numbers
