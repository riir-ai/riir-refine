# Plan 125 — Config-audit class C2: the self-referential config struct detector

**Status:** COMPLETE — 2026-09-12 (all tasks `[x]`; G1 23/23 fixtures, G2 507 files 1 TP / 0 FP, G3 1231 lib green + clippy `-D warnings` clean, G4 zero new deps)

**Intake:** `.distill/001_mining_queue_snapshot.md` §"Post-mining intake" class C
(2026-09-08, the riir-ai Issue-888 T18 escalation), sub-shape **C2**. Measured
once: riir-games `PlasmaConfig` — 10 `pub` fields, presets advertising
per-game tuning values that tuned nothing; only readers were its own
`validate()`, its own tests, and a GOAT-proof file's config-only tests;
deleted at riir-ai `dbb939b26`.

**Detector shape (the intake's own words):** "per-struct — collect all
`.field` reads of the type's fields crate-externally; a struct with zero
external reads is class C2 regardless of internal reads."

**C1 is NOT this plan** — orphaned-by-optimization needs git HISTORY (a
reader that vanished), a review-time heuristic on commits touching config
inits; stays in the intake for a future unit.

## What C2 adds over A+B (the gap it fills)

Classes A and B require **zero reads** of the field. The C2 shape HAS reads
— but they are confined to (a) the struct's own inherent impls
(`validate()`, helpers) and (b) test context (`#[cfg(test)]` mods, `#[test]`
fns, `tests/`+`benches/` files). An internally-read, externally-orphaned
config struct is invisible to A/B (their `reads > 0 → skip` guard) and to
`dead_code` (`pub` = assumed used). Also catches the never-plumbed literal
default (zero reads anywhere, literal init — A needs a const, B needs an
assert): nothing fires today.

## Firing condition (within the instrument's syn-scan limits)

For a struct T with a hand-written `impl Default` (≥1 field candidate):

- `external(f) = reads_total(f) − reads_tests(f) − reads_in_impl(T, f)` for
  every candidate field f of T
- C2 fires iff `external(f) == 0` for ALL of T's candidate fields.

"Reads" keep the instrument's name-matching semantics (untyped — over-
counting reads ⇒ false negatives only, never false positives; destructuring
binds count as reads; macro bodies token-scanned). Reads inside `impl U`
blocks where U ≠ T (e.g. an `Engine::tick` consuming the config) count as
EXTERNAL — correctly keeping T live. Under-reporting vs the crate-external
ideal (sibling/upstream consumers invisible) stays in the report footer.

## Tasks

- [x] T1 Extend `InteractionIndex` + `InteractionVisitor`: per-field
      `reads_tests` tally + `(field, impl-type)` read tally; test-context
      tracking (`#[cfg(test)]` mod, `#[test]`/`#[bench]` fn, `tests/` /
      `benches/` file paths); impl self-type context stack; thread the file
      name through pass 2. *(The probe caught the missing hook: impl methods
      are `syn::ImplItem::Fn` — `visit_item_fn` never fires inside impls.)*
- [x] T2 `classify()` struct-level aggregation + `SelfReferentialConfig`
      finding type + report/render section (summary line gains the C2
      count); module doc gains the class-C section (shape + honest limits +
      precedence note: C2 is per-struct, may co-fire with per-field A/B on
      an all-inert struct — the row aggregates).
- [x] T3 Fixture tests (the Plan-124 set + C2 arms): fires (validate+test
      reads only), never-plumbed literal (zero reads, no const/assert),
      external-read suppresses, `impl U` read is external, external
      destructuring counts, same-field-name masking (documents the
      false-negative direction), tests/-file path context — 8 C2 arms,
      15+8=23 green.
- [x] T4 Validate: 1231 lib green (isolated `/tmp/clippy-c2` target — the
      sibling's 093-T2 WIP compiled clean at default features, no worktree
      fallback needed) + real-file runs: riir-games 274 files → 1 verified
      TP (`PetAiConfig` — whole-struct orphan, consumer never materialized,
      `melee_reach` fully dead; 0 refs in riir-mmorpg-examples), self 233
      files → 0 findings; 1 TP / 0 FP over 507.
- [x] T5 Docs: AGENTS.md `--config-audit` bullet extended with class C;
      intake row class C → C2 DONE / C1 remains; HISTORY.md row.
- [x] T6 Commit + push (below); `.plans/.highwater` repaired 123 → 125
      (the Plan-124 session's missed write-back).

## Validation gates (report-first law — gates nothing, always exit 0)

G1: fixture tests green (new arms + the Plan-124 set unregressed).
G2: real-file run completes and its findings are hand-adjudicated (every
    row TRUE or explained by the documented masking).
G3: default `cargo test --lib` green (no feature regressions).
G4: no new deps; pure syn scan; the report stays suggest-only.
