# `sec` — security-pattern healing (4th retrieval domain)

> Phase 0+1 shipped 2026-08-22 (Plan 018 / Proposal 004 / Research 078-079 /
> Bench 046). Opt-in feature `sec`, default-off — promotion needs measured
> consumer pull (the `rust_perf` precedent).

## What it is

Security-relevant **source-code patterns with bounded fix-spaces**: FFI
panic containment, Drop-glue panic safety, raw-pointer null hygiene, the
`from_raw_parts` len-zero contract, Rc backedge leaks. NOT dependency
auditing (cargo-audit/cargo-deny), NOT a lint engine (dylint), NOT
judgment-bearing detection — every rule is parse-only provable with a
bounded fix, or suggest-only.

## The Phase-1 corpus (6 rules, 5 of 11 candidates killed)

| Rule | Family | `--fix` | The bounded fix |
|---|---|---|---|
| `ffi-extern-fn-catch-unwind` | `Ffi` | ✓ | wrap unit-return extern fn bodies in `std::panic::catch_unwind` (the in-house `#[riir_ffi]` rule generalized) |
| `ptr-deref-null-check-first` | `UnsafeHygiene` | ✓ | `let Some(p) = (unsafe { p.as_ref() }) else { return None };` prologue in Option-returning unsafe fns |
| `drop-impl-no-panic` | `DropLeak` | ✓ | `.unwrap()`/`.expect()` → `.unwrap_or_default()` inside drop glue (double-panic = abort) |
| `slice-from-raw-parts-len-guard` | `UnsafeHygiene` | ✓ | `if len == 0 { &[] } else { from_raw_parts(..) }` (ptr must be non-null even at len 0) |
| `extern-fn-reference-param-to-pointer` | `Ffi` | suggest-only | `&T` param → `*const T` + explicit deref (signature-changing — the C-side caller ripples) |
| `rc-backedge-field-prefer-weak` | `DropLeak` | suggest-only | direct `Rc<Self>` backedge field → `Weak<Self>` (constructor sites change too) |

## The boundary discipline (non-negotiable)

`tests/sec_boundary_gate.rs` runs REAL clippy over the corpus fixtures with
the banned list enabled (`mem_forget`, `indexing_slicing`,
`arithmetic_side_effects`, `unwrap_used`/`expect_used`, `improper_ctypes*`,
`partialeq_ne_impl`, `undocumented_unsafe_blocks`, `disallowed_methods`):
no banned lint may fire MachineApplicable on a sec fixture — a future
clippy that auto-fixes one of our territories FAILS the gate and that rule
retires. The one pinned allowed overlap: `unwrap_used` fires on the drop
fixture with NO suggestion (our fix is additive).

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
