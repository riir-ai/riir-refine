# Research 079 — `sec` Domain Phase 0+1 (the fourth HealDomain ships)

**Date:** 2026-08-22
**Status:** RECORD (Phase 0+1 landed; Phase 2 unblocked-but-deferred)
**Refs:** Proposal 004 · Research 078 (seed) · Plan 018 · Bench 046

## What landed

`src/domains/sec/` — the fourth retrieval `HealDomain` (clippy/perf/kernel
precede it; `rustc_errors` is driver-based, not retrieval). 6 rules, 4
proposers, the geiger-taxonomy unsafe counter, the banned-list boundary
gate, `--domain sec` CLI wiring, DomainRouter 4th-domain fan-out.

## The mine-time falsification record (T1.1 — the batch's core discipline)

5 of 11 candidates KILLED before authoring, each with measured evidence
(real clippy 1.93.0 probes — the full table in Bench 046):

1. `ffi-fnptr-type-unsafe-extern` — `improper_ctypes` fires WITH a fix
   ("consider using an `extern fn(...)` instead"). Same detection territory,
   near-same fix. KILL.
2. `ffi-by-value-no-drop-type` — `improper_ctypes` fires WITH `#[repr(C)]`
   fix. KILL.
3. `ffi-portable-c-alias` — the alias map is platform-dependent (`c_long`
   is 32-bit on Win32) — a parse-only proposer cannot prove size-identity.
   KILL on bounded-fix grounds even though no lint fires.
4. `no-box-leak` — no lint, but the fix is contextual (arena? lifetime
   extension? ownership restructure?). KILL.
5. `maybe-uninit-requires-justification` — `uninit_assumed_init` is
   error-level on the DIRECT chain (measured!); the let-bound form is
   open territory, but the fix ("initialize properly") is contextual. KILL.

The 6 survivors: `ffi-extern-fn-catch-unwind` (the GOAT candidate — the
in-house `#[riir_ffi]` rule generalized), `ptr-deref-null-check-first`,
`drop-impl-no-panic`, `extern-fn-reference-param-to-pointer` (suggest-only),
`rc-backedge-field-prefer-weak` (suggest-only; renamed from the plan's
`rc-cycle-interior-mutability` to the parse-only-provable narrowed shape —
RefCell interior mutability is NOT what we detect; the direct `Rc<Self>`
backedge field is), `slice-from-raw-parts-len-guard` (ADDED beyond the
plan's list — the documented `from_raw_parts` "ptr non-null even at len 0"
UB class; the most mechanical bounded fix of the batch).

## The two measured surprises (worth remembering)

1. **`improper_ctypes_definitions` is SILENT on extern-fn definitions with
   reference params on 1.93** — both `&u32` and `&Vec<u32>`, even explicitly
   enabled. The ANSSI reference-param territory is genuinely open, not
   shadow-covered. (The DECLARATION side — extern blocks — is fully owned by
   `improper_ctypes`.)
2. **`uninit_assumed_init` misses the let-bound form** — fires error-level
   on `MaybeUninit::uninit().assume_init()` direct chains but not on
   `let mu = MaybeUninit::uninit(); mu.assume_init()`. Clippy's own
   coverage gap; recorded, not exploited (no bounded fix).

## The 4-domain router re-pin (the honest delta)

The lexical annotation router drops to 44.6% (2-domain: 70.3%) with sec
enabled — mechanical: the eval fixture set has no sec-labeled spans, so
every sec-routing counts as a miss; sec's 6 long full-fn patterns form a
common-token centroid that absorbs generic spans. The RETRIEVAL floors
(the load-bearing ones) all held. Floors re-pinned per the Issue 020
contract (both 4D states, recorded in Bench 046 + the floors block docs).

En-route fix: `SpectralVocab`'s hardcoded `[u32; 3]` domain arrays →
`Vec`-keyed by domain count — the first 4-domain build PANICKED on the
pre-existing limitation.

## Boundary discipline (T0.4 — the standing gate)

`tests/sec_boundary_gate.rs` runs REAL clippy over the 6 fixtures with the
banned list enabled: no banned lint may fire MachineApplicable (a future
clippy that auto-fixes one of our territories FAILS the gate → that rule
retires). The pinned allowed overlap: `unwrap_used` fires on the drop
fixture with NO suggestion — our Drop-scoped `.unwrap_or_default()` is
additive, not duplicative.

## Negative + positive controls (T1.4 G1)

- **Negative**: `cargo heal --fix --domain sec` (dry) over riir-viewbridge's
  24 files = **0 edits** (the `riir_ffi!` macro wraps its surface; the
  catch_unwind guard skips contained fns). A nonzero there would have
  falsified the proposer.
- **Positive**: a synthetic victim (unwrapped `#[no_mangle] extern "C" fn`
  + Drop-unwrap + Option ptr deref) = 3 edits end-to-end.
- riir-chain wallet/catchup (26 files, dry — sibling-active repo, no
  writes) = 0 edits.

## Phase 2 disposition

The ≥6-rule gate is MET (6/6). Phase 2 (unsafe-impl-send-sync, crypto
trio, RustCrypto second seed) stays `[-]` per Plan 018 — a future batch,
not this one. The 20-rule domain bar (Research 078's density estimate)
remains the honest ceiling concern: 6 + Phase 2's ~6-8 gets close.

## Corpus deltas

- sec corpus: 6 rules (`Ffi` ×2, `DropLeak` ×2, `UnsafeHygiene` ×2)
- rust_perf: unchanged (74) · clippy: unchanged (40) · kernel: unchanged (367)
- default lib tests 657 → 657 (opt-in feature, zero default-surface change)
