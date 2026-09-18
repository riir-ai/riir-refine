# Research 190 — Batch 163: the frontier-lead mining pack IX (`manual_string_new` + `needless_late_init`)

**Status:** RECORD (batch landed 2026-09-19, Plan 154; the Batch 162 re-arm's
recorded spares consumed as rules, both families graduated, **the
recorded-spare queue is now EMPTY** — the next re-arm owes a fresh probe
round over clippy's live lint surface).

## Source

The plan152-verified spares, recorded at Batch 162 (Research 189). This
batch re-verified them at the live toolchain with its OWN probe
(`scripts/plan154_frontier_probe.py`, 2026-09-19) and ARMED them: rules,
proposers, corpus entries, frontier families, group pins.

## The probe's design-input matrix (the new measurements)

**`manual_string_new`** (`String::from("")` → `String::new()`, suggestion
`String::new()` rendered MachineApplicable):

- `String::from("x")` SILENT (empty-only); `String::from(s)` SILENT;
  even `let s = ""; String::from(s)` SILENT — clippy is LITERAL-ARG-ONLY.
- `<String as From<&str>>::from("")` FIRES (same suggestion) — the
  turbofish re-spells the type the path already names, so handling it is
  resolution-identical; `From::from("")` (inference could be Cow/Box) is
  refused by us.
- `"".to_string()` FIRES upstream — a different rewrite seam (method call,
  not a path call); REFUSED, the documented narrower-than-clippy
  divergence (same posture as string_lit_as_bytes's `\u{` refusal).
- **GROUP: PEDANTIC** (pedantic-only fires; `clippy::all`-only and
  nursery-only silent; default-groups arm EMPTY). The stale AGENTS.md
  "nursery" posture string_lit_as_bytes carried was the warning here —
  this batch pinned msn by three separate group arms BEFORE landing.
- `Some(String::from(""))` fires in place (nested-in-larger-expr, the
  batch-162 shape).

**`needless_late_init`** (`let x[: T]; x = v;` → `let x[: T] = v;`,
suggestion `let x[: T] = <rhs>` — annotation carried):

- `nli-annotated` fires, suggestion `let x: u32 = 5u32` — annotation
  rides.
- `nli-intervening` SILENT — anything between the pair silences clippy;
  the proposer mirrors (adjacent-in-block only).
- `nli-ifelse` fires needless_late_init with an EMPTY diagnostic; the
  actionable combined suggestion comes from `useless_let_if_seq` — a
  different seam; branch initializers REFUSE with that note.
- `nli-call-init` fires (`let x = mk(hits)`); `nli-mut` SILENT — `mut`
  bindings refuse (clippy's own scope; its fix would also drop the mut).
- `nli-two-assigns` returned `[]` but is a VOID arm: `let x; x = 1; x = 2;`
  is E0384 (does not compile) — the multi-assign guard is the `mut`
  refusal itself, not a clippy-silence measurement. Honest note; the arm
  should not be cited as a verdict.
- `nli-question` fires (`let x = parse(hits)?` accepted upstream); our
  emit accepts any rhs verbatim — the adjacency guard makes the move
  order-identical, so `?`/`.await`/blocks ride.
- **GROUP: STYLE, default-on** (default-groups arm FIRES).

## The batch's syn finding — statement attributes RESPAAN everything

The `attr_on_assign_refuses` fixture took three guard attempts:
`Expr::Assign.attrs` reads EMPTY (0) even with `#[allow(unused)]` on the
statement; the statement span starts at `#`; and — the decisive dump —
**the lhs path's own span is RESPANNED to start AT the attribute**
(`left=58..76` for a statement spanning `58..84`). syn folds statement
attributes into the statement span and the inner expression nodes ride
the respan. No structural span comparison can see the tokens.

**The guard is textual**: the statement's first non-whitespace text must
BE the lhs ident (`head.starts_with(ident)`); an attribute (or any other
token) before it refuses. Same class as the Batch-150 hoist guard (syn
respans statement-position spans — recorded then, re-learned here at one
layer down). Fixture: `let x; #[allow(unused)] x = 5u32; x + x` refuses.

## Verification walk (the batch gates)

- GOAT G1/G2: default lib **1991/0** (+25 tests: 10 msn + 15 nli... 25
  module tests across the two proposers); latent **2144/0**;
  latent,frontier_miner,score_bench **2308/0**.
- `pool_depth_rescue_guard` did NOT re-fire — the pack IX pair is not
  vec-shaped; RERANK_POOL 28 stands.
- Score-bench #139: **96.36% FLAT bit-identical** to #138 (created 1.8%
  unchanged; rule_outcomes show neither new rule fired on any v1
  fixture). Two PRE-EXISTING toolchain drift warnings on clean fixtures
  (doc_markdown now fires on katgpt-rs-quantile_balance_router-002/003-clean;
  fresh-wins, recorded not fought — the v1 fixture set's frozen labels vs
  clippy 1.98, present before this batch).
- Freeze regen v2 **90 labels / 11 exclusions**, v3 **89 / 16** — v3
  PURELY ADDITIVE (the two new k1 labels, composites byte-identical); v2
  exclusions 12→11 = the benign window shuffle. Bad-atom set UNCHANGED
  (6 v2 / 7 v3): neither new atom interferes — both fire exactly their own
  lint on every window they land in (checked across k1/k2/k3/k4 in the
  regen output). Both freeze e2es PASS 0 drift (40.6s + 43.1s).
- Frontier e2e **VERDICT ✓**: 40 seeds clean, 19 family loops, 2 fresh
  admissions, 17 committed rows 0 drift, 2 rows appended (`1a84d4d6993ee44e`
  msn, `0e0c4e17b4501b5d` nli — heldout, solving_rule set), band fold
  19/19. The store holds **19 solved rows**.
- Retrieval floors HELD (15/15, blended 94.7/96.2/96.9 vs the
  77.1/86.4/92.4 floors).
- Clippy `-D` clean ×3 states: default, `--all-features`
  (1 doc_markdown backtick healed in the new modules + the corpus doc
  comment's `= 84.` reflow — a line starting `+ the 2 Batch 163 rows` is
  a rustdoc LIST MARKER and the following lazy continuations warn
  doc_lazy_continuation; the `+` moved mid-line, the batch-162 shape),
  and the latent state's `let_and_return` backtick.
- The temp regen example (`examples/freeze_regen_tmp.rs`) carried
  `required-features = ["score_bench"]` while it lived — the default
  `--all-targets` lane caught it E0432 the moment it was created (the
  Issue-513 class, again; deleted with the file before landing).

## Next pool

The recorded-spare queue is EMPTY. The next re-arm (next mining window)
owes a fresh probe round: derive clippy's live lint surface
(`clippy-driver -W help`, the `--coverage-report` derivation), diff
against the covered 19 families, pick the next default-firing
multi-crate pair, probe, arm. Arxiv walk #10 opens Mon 09-21.

## Landing

Plan 154 (commit-pair with this note); corpus 82→84, FIXABLE 63→65,
atoms 63→65; the seventh consecutive all-FIXABLE pack.
