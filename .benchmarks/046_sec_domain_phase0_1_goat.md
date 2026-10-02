# Bench 046 — `sec` domain Phase 0+1 GOAT (Plan 018, Proposal 004)

**Date:** 2026-08-22
**Feature:** `sec` (opt-in, default-off — promotion needs measured consumer pull)
**Corpus:** 6 rules (5 of 11 candidates killed at mine-time falsification)

## T1.1 — the clippy-gap probe (measured, clippy 1.93.0, real `cargo clippy`)

Probe crate: 11 shapes in per-file modules, default run + restriction-group run +
explicit `-W improper_ctypes_definitions -W clippy::uninit_assumed_init`. The
evidence table that decided every candidate:

| # | Candidate | Measured on the buggy shape | Verdict |
|---|---|---|---|
| 1 | `ffi-extern-fn-catch-unwind` | nothing fires (style noise only) | **SHIP** (proposer) |
| 2 | `ptr-deref-null-check-first` | nothing fires | **SHIP** (proposer) |
| 3 | `drop-impl-no-panic` | `unwrap_used` fires, NO suggestion | **SHIP** (proposer) — additive boundary |
| 4 | `ffi-fnptr-type-unsafe-extern` | `improper_ctypes` fires WITH fix ("consider using an `extern fn(...)`") | KILL (dup) |
| 5 | `ffi-by-value-no-drop-type` | `improper_ctypes` fires WITH fix (`#[repr(C)]`) | KILL (dup) |
| 6 | `ffi-portable-c-alias` | nothing fires, BUT the alias map is platform-dependent (`c_long` ≠ `u64` on Win32) — not parse-only-provable | KILL (unbounded fix) |
| 7 | `extern-fn-reference-param-to-pointer` | **`improper_ctypes_definitions` measured SILENT** on `&u32` AND `&Vec<u32>` extern-fn definitions (explicitly enabled!) | **SHIP** (suggest-only) |
| 8 | `rc-cycle-interior-mutability` → narrowed `rc-backedge-field-prefer-weak` | nothing fires | **SHIP** (suggest-only) |
| 9 | `no-box-leak` | nothing fires, but fix is contextual | KILL (unbounded fix) |
| 10 | `maybe-uninit-requires-justification` | `uninit_assumed_init` fires ERROR-level on the DIRECT chain; let-bound form silent; fix contextual | KILL (unbounded fix) |
| 11 | `slice-from-raw-parts-len-guard` (ADDED — the `from_raw_parts` contract: ptr non-null even at len 0) | nothing fires | **SHIP** (proposer) |

**5 falsified / 6 shipped ≥ the ≥6 Phase-2 gate.** Phase 2 stays `[-]` (per
Plan 018's own defer; opens on the Phase-1 verdict — which is now met, so
Phase 2 is UNBLOCKED for a future batch).

Surprise findings worth keeping: (a) `improper_ctypes_definitions` does NOT
fire on extern-fn DEFINITIONS with reference params on 1.93 — the ANSSI
reference-param territory is genuinely open; (b) `uninit_assumed_init` is
error-level on direct chains but misses the let-bound form — clippy's own
coverage gap.

## T0.4 — the banned-list boundary gate (`tests/sec_boundary_gate.rs`)

Real clippy over all 6 fixtures with the banned list enabled:
- `no_banned_lint_fires_with_an_equivalent_fix` — **PASS**: no banned lint
  fires MachineApplicable anywhere.
- `unwrap_used_fires_without_fix_on_drop_fixture` — **PASS**: the pinned
  allowed overlap (fires, no machine fix — our fix is additive).
- A future clippy that grows an auto-fix for our territory FAILS the gate →
  that rule retires.

## T0.5 — the unsafe-count visitor (geiger taxonomy, zero deps)

`count_unsafe` + `UnsafeCounts` (unsafe fns/methods, attribute-unsafety
`#[no_mangle]`/`#[export_name]` on safe fns, unsafe impls/traits, unsafe
exprs) + `file_forbids_unsafe`. Feeds `SecVerdict.unsafe_count_delta`
(signal, never gate). Unit-pinned: 5-counter taxonomy test.

## T1.3 — the 4 proposers (conservative gates)

| Rule | Fires on | Refuses on |
|---|---|---|
| `ffi-extern-fn-catch-unwind` | extern-ABI fn, unit return, body w/o `catch_unwind` | non-unit return, already-contained |
| `ptr-deref-null-check-first` | unsafe fn, exactly 1 raw param, `-> Option`, textual `(*p)`, no `as_ref`/`is_null` | safe fn, non-Option, multi-param, guarded |
| `drop-impl-no-panic` | `.unwrap()`/`.expect()` inside `impl Drop` `fn drop` | anything outside drop glue |
| `slice-from-raw-parts-len-guard` | `from_raw_parts[_mut]`, len a bare path, no `len == 0` in file | literal len, existing guard |

## GOAT gates

| Gate | Result |
|---|---|
| **G1** fixtures fire + clean spans | 30/30 sec unit tests (`every_fixable_rule_fires_on_its_fixture`, `clean_spans_fire_no_lint`); **negative control: riir-viewbridge dry `--fix --domain sec` = 0 edits / 24 files** (the `riir_ffi!` macro surface + the catch_unwind guard); **positive control: 3 edits end-to-end** on a synthetic victim |
| **G2** latency | rides the shared index+scratch path (no new hot-path work); existing latency tests green in-suite |
| **G3** no-regression | default lib 657/0 (baseline preserved) · sec 687/0 · sec+kernel+latent 953/0/2i · clippy `--all-targets` 0 warnings in default + sec states · retrieval floors HELD (blended 93.1/96.0/97.0; kernel 100% in combo; sec self-variants 100% top-1) |
| **G4** alloc | scan path reuses `SuggestScratch` (stability test green in-suite); proposers allocate like all proposers (fix-path, not scan-path) |
| **G5-real** | viewbridge 0 edits ✓ + riir-chain wallet/catchup 0 edits (26 files, dry) ✓ |

## Router annotation re-pin (the Issue 020 contract, applied)

Adding the 4th domain mechanically lowers the lexical router's annotation
accuracy (the eval fixture set carries no sec-labeled spans, so every
sec-routing counts as a miss by construction; sec's 6 long full-fn patterns
form a common-token centroid):

| State | Measured | Floor pinned |
|---|---|---|
| 2-domain (baseline) | 70.3% | 66/101 |
| 3-domain (+kernel) | 64.6% | 84/130 |
| 4-domain (+sec) | **44.6%** (45/101) | 40/101 |
| 4-domain + kernel | **59.5%** (78/131) | 73/131 |

Annotation is display-only (Issue 020 verdict — gating killed at the 85%
bar); the RETRIEVAL floors all held. Also fixed en route: `SpectralVocab`'s
hardcoded `[u32; 3]` domain arrays → `Vec` (the 4-domain state PANICKED
before; pre-existing limitation exposed by the first 4-domain build).

## Feature-state count deltas

- default lib: 657 (unchanged — sec is opt-in)
- `--features sec`: 687 (+30: 26 sec unit + 2 boundary-gate-adjacent lib + 2
  retrieval-eval)
- `sec,kernel_opt,latent_retrieval`: 953

## Verdict

**Phase 0 + Phase 1 PASS.** The domain ships 6 rules / 4 proposers with the
boundary gate + negative control green. Phase 2 (unsafe + crypto hygiene)
is UNBLOCKED by the ≥6 verdict but stays deferred per Plan 018's own gates.
Promotion to default requires measured consumer pull (≥1 workspace repo
adopting sec in CI) — the `rust_perf` precedent.
