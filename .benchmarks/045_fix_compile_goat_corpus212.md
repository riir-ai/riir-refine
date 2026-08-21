# Bench 045 — Plan 019 Phase 6 GOAT: `--fix-compile` over the N=212 corpus

**Date:** 2026-08-22
**Plan:** `.plans/019_rustc_errors_fix_compile.md` (Phase 6: T6.1–T6.8, T6.10)
**Feature:** `rustc_errors` (opt-in, default-off; the ab section also needs `latent_retrieval`)
**Toolchain:** rustc 1.97.0 (repo pin), macOS M3, otherwise-idle box
**Repro:**
```bash
# the measurement session, chunked (each command self-contained):
cargo run --release --example rustc_errors_floor --features rustc_errors -- --only probe
cargo run --release --example rustc_errors_floor --features rustc_errors -- --only floor
cargo run --release --example rustc_errors_floor --features rustc_errors -- --only driver
cargo run --release --example rustc_errors_floor --features rustc_errors -- --only seeds
cargo run --release --example rustc_errors_floor --features "rustc_errors,latent_retrieval" -- --only ab

# the standing gates:
cargo test --features rustc_errors --test generated_corpus_spot          # family spot (fast)
RIIR_FULL_CORPUS_INTEGRITY=1 cargo test --features rustc_errors --test generated_corpus_spot full_corpus
cargo test --features "rustc_errors,latent_retrieval" --test fix_compile_gates
```

## The corpus (T6.1 — N≥200 met)

212 cases = 12 committed anchors (hand-authored `from`/`to` mutators) + 200
generated (parametric seed templates, deterministic table-driven generation —
pinned byte-stable by `generation_is_deterministic`). Every generated seed
carries a `#[cfg(test)]` ground-truth behavior test (the G1 semantics axis).
Stratified by territory (the anti-inflation rule):

| code | mutants | territory | generator families |
|---|---:|---|---|
| E0597 | 42 | uncontested | str-borrow ×10, vec-slice ×10, as_str ×5, format ×5, str-idx ×10 |
| E0499 | 26 | uncontested | flatten-scope ×25 (5 types × 5 push-pairs) + anchor |
| E0502 | 37 | uncontested | copy-index ×20, string-len ×15 + 2 anchors |
| E0505 | 21 | contested (our opening) | drop-clone-under-borrow ×20 + anchor |
| E0596 | 22 | contested (delegated) | drop-mut ×15 vec + ×5 String + 2 anchors |
| E0382 | 21 | contested (delegated) | drop-clone ×20 + anchor |
| E0507 | 16 | contested (delegated) | field-clone ×15 + anchor |
| E0373 | 27 | contested (delegated) | drop-move ×10 str + ×15 sum + 2 anchors |

**Integrity (the env-gated full gate): all 212 mutants provoke their declared
code; the null arm scores exactly 0.0%; all 212 seed ground-truth tests pass.**
Two corpus bugs were caught by these gates on the first run — a literal `!` in
a case name made an invalid Cargo manifest (zero diagnostics = false
integrity failure; fixed by package-name sanitization in the scaffold), and
**10 hand-counted string-length expectations were wrong** ("seven7" is 6
chars, not 7 — the seeds COMPILED but their own tests failed, which
check-time integrity cannot see and which only the seed-validity pass
caught). Both are now standing gates.

## G-floor at N=212 (the floor arm)

| code | mutants | matched | diverged | unfixed | fix rate | divergence |
|---|---:|---:|---:|---:|---:|---:|
| E0597 | 42 | 0 | 0 | 42 | **0%** | — |
| E0499 | 26 | 0 | 0 | 26 | **0%** | — |
| E0502 | 37 | 0 | 0 | 37 | **0%** | — |
| E0505 | 21 | 0 | **21** | 0 | 100% | **100%** |
| E0596 | 22 | 22 | 0 | 0 | 100% | 0% |
| E0382 | 21 | 21 | 0 | 0 | 100% | 0% |
| E0507 | 16 | 16 | 0 | 0 | 100% | 0% |
| E0373 | 27 | 27 | 0 | 0 | 100% | 0% |
| **total** | **212** | 86 | 21 | 105 | **50.5%** | 19.6% |

The bimodal floor at N=12 held exactly at N=212: **0% on all three
uncontested codes (105 mutants), 100% on all delegated codes (matched), and
100% fix at 100% divergence on E0505** (rustc's `&v.clone()` rewrites the
borrow site — aliasing silently changed — on every one of the 21).

## G-coverage + G1 (the driver arm — the product numbers)

| code | mutants | cargo fix | `--fix-compile` | verdict |
|---|---:|---|---|---|
| E0597 | 42 | **0%** | **100%** (42 diverged) | net-new |
| E0499 | 26 | **0%** | **100%** (26 diverged) | net-new (the swap reorders instead of re-scoping) |
| E0502 | 37 | **0%** | **100%** (21 **matched** + 16 diverged) | net-new (deref-removal restores Copy seeds byte-identically) |
| E0505 | 21 | 100% but **100% diverged** | **100% MATCHED** | **the contested win** |
| delegated ×4 | 86 | 100% matched | 0% (no template) | delegated by the floor gate |

- **UNCONTESTED stratum: 105/105 fixed (100%)** where `cargo fix` scores 0/105.
- **Total: 126/212 repaired; the G1 semantics pass ran `cargo test` on every
  one: 126/126 keep the seed's behavior — zero semantic breaks.** The
  "diverged" verdicts are semantically-equal-but-textually-different repairs
  (hoists, swaps, clone-outs), each NAMED in the diff by its T6.10 note.
- **Never-wrong: non-compiling edits surviving a run = 0** (the keep
  criterion — strict error-count decrease — plus byte-exact revert; pinned by
  `driver_reverts_no_progress_candidate` and the report-side gate).

## G2 latency (release, M3, incremental `cargo check` dominates)

| axis | n | p50 | p99 | max |
|---|---:|---:|---:|---:|
| per case (scaffold → before-check → fix → after-check) | 212 | 151 ms | 164 ms | 174 ms |
| per driver iteration (propose → apply → recheck) | 126 | 78 ms | 95 ms | 96 ms |

## G4 alloc (the propose/apply path)

~**1,504 allocations per proposal pass** (syn parse + candidate Strings),
**bounded and non-growing across repeated identical passes** (the gate; a
batch repair tool does not need zero allocs — it must not leak them). The
cargo-check subprocess is the oracle, out of scope by the plan's own wording.
Gate: `propose_path_allocations_are_bounded_not_growing` (counting
allocator, 10 passes, growth + spread bounds).

## G6/G7 self-evolve

- **(a) Store round-trip, both backends** — neuron-db:
  `traj_store::tests::record_then_reopen_then_retrieve`; JSON envelope:
  `json_backend_trajectory_survives_save_load_roundtrip` (save → fresh
  recorder → load → trajectory + identity preserved). Both pinned.
- **(b) Ordering A/B — HONEST NEGATIVE (the T4.2 clause fires):**
  learning pass (126 trajectories recorded into a warm store), then

  | arm | clean | iterations | reverts |
  |---|---|---:|---:|
  | static (min-delta) | 126/126 | 126 | **0** |
  | flat counters | 126/126 | 126 | **0** |
  | shape-conditional | 126/126 | 126 | **0** |

  **Static min-delta ordering already succeeds first-try on every
  corpus case — 126 iterations for 126 fixes, zero reverts anywhere — so
  ordering has no headroom on this corpus and the A/B cannot
  differentiate.** The T4.2 mechanism stays installed (the CLI seam + the
  flip test pin the property: reversed store history flips candidate
  order — something static ordering can never do), but no benefit is
  claimed. Same class as T9.4/T7.3: the corpus's shapes don't exercise
  the regime where learning helps (real-use misses with repeated
  shape-families would). **T4.6 consequence:** per-repo store scope stays
  the default; no `--global-traj` work.
- **(c) Miss capture** — `dump_writes_entries_and_readme_once` +
  `dump_on_no_misses_touches_nothing` (evolve fixtures) +
  `driver_records_miss_when_no_playbook_matches`; the delegated codes'
  86 unfixed cases are the standing authoring queue demonstration.

## G-semantics (T6.10) — the exclusion class, enforced by test

- `box_leak_is_never_emitted` (Phase 2, unchanged) — the canonical
  wrong-but-compiling fix is structurally refused.
- **8 templates carry semantic notes** (surfaced on every applied/reverted/
  planned record + printed in the CLI diff — live-verified:
  `⚠ semantic change: clones the element — the binding owns a copy instead
  of borrowing (aliasing not preserved)`): e0597-anon-lifetime ("loosens
  the lifetime annotation"), e0597-move ("ownership leaves the enclosing
  scope"), e0597-move-clone ("closure owns a copy"), e0597-drop-the-box
  ("no longer heap-allocated"), e0597-shared-ownership ("shared, not
  exclusive"), e0597-scope-hoist ("drop order changes"), e0502-clone-out
  ("a copy instead of borrowing"), e0505-clone-at-move ("destination owns a
  copy; source borrow preserved").
- 2 templates are unannotated BY DESIGN: e0502-deref-removal (Copy-typed
  reads are byte-identical) + e0499-swap-second-decl (a pure reorder).
  Gate: `every_annotated_strategy_actually_carries_its_note`.

## G3 no-regression

default lib **657** ✓ · rustc_errors+latent lib **787** ✓ · all-features lib
**1082** ✓ · clippy **0** on every touched surface (the 4 all-features
warnings are the sibling riir-gpu crate's pre-existing state) · e2e suites
fix_compile 6 + fix_compile_borrowck 6 + fix_compile_e0597 4 +
floor_corpus_integrity 3 + premise_probe 6 ✓ · retrieval precision floor ✓ ·
clippy-bench score history unaffected (no proposer touched).

## G1 correctness (T6.2)

By construction (oracle-gated keep) + asserted: every shipped fix compiles;
**126/126 repaired cases pass their seeds' behavior tests post-fix**; the
T2.3 already-compiles no-op gate unchanged
(`driver_reports_nothing_on_clean_crate` +
`already_compiling_incident_tree_is_untouched`).

## Verdict — the plan's gates

| gate | verdict |
|---|---|
| T6.1 corpus N≥200 | **PASS** — 212 cases, integrity + seed-validity + null-arm all green |
| T6.2 G1 correctness | **PASS** — 126/126 post-fix behavior-green; no-op gate green |
| T6.3 G2 latency | **PASS** — measured p50/p99 per case + per iteration (release) |
| T6.4 G3 no-regression | **PASS** — 3 lib suites + clippy 0 + e2e + retrieval floors |
| T6.5 G4 alloc | **PASS** — bounded, non-growing (~1.5k allocs/pass, published) |
| T6.6 G-coverage | **PASS** — per-family tables published (above + README); never-wrong pinned |
| T6.7 G6/G7 | (a) **PASS** both backends; (b) **HONEST NEGATIVE** recorded (no headroom — static is already first-try-perfect on this corpus; T4.2 mechanism retained, T4.6 scope stays per-repo); (c) **PASS** |
| T6.8 record | this bench + README section + AGENTS.md feature row + mining-queue pointer |
| T6.9 G-floor | measured at N=212 (the table above; the N=12 record stands in Bench 041) |
| T6.10 G-semantics | **PASS** — notes enforced by test; Box::leak never-emitted |

## Honest caveats

- The corpus is still mutation-class-bounded: every case's ground truth is
  "restore the seed". Real-world broken code has no seed; the 86 delegated
  misses + real-use `.heal/misses/` are where the next templates come from.
- The ordering A/B's zero-headroom result is corpus-specific — it does NOT
  refute shape-conditional ordering in general (the flip test proves the
  mechanism); it says THIS corpus cannot measure a benefit. Real use with
  repeated shape families is the reopen condition.
- G4 is boundedness, not zero: the propose path allocates (syn); the plan's
  own scope note excludes the oracle subprocess, and a batch tool's alloc
  profile is not a hot-path budget.
- Single-toolchain measurement (1.97.0); the premise probe is the standing
  tripwire for toolchain drift.
