# Plan 124 — Config-hygiene intake consumption: classes A + B, `--config-audit` report lane

**Status:** COMPLETE (2026-09-11 — all tasks [x]; landed as the `--config-audit`
report lane + 15 fixture tests; validation record in HISTORY.md 2026-09-11)

## Why

The post-mining intake carries 3 open classes mined from a real consumer session
(riir-ai Issue 888 follow-ups), each with measured instances. Class A is the
top-ranked: **`config-default-is-a-named-constant-and-the-consumer-reads-the-constant`**
— a `pub` config field whose `Default` init is a named/associated const, where
consuming code reads the CONST (not `self.field`). Silent-wrong configuration: the
caller sets the knob, gets the constant, any A/B over that dimension measures
nothing. Invisible to `cargo clippy` at every feature state (a `pub` field is
assumed used-by-API in a lib crate).

Class B rides the same index: **`assert-on-default-config-value`** — a test whose
only interaction with a config field is an equality/boolean assertion against the
Default literal; an inert knob then reads as covered.

Class C (`config-path-orphaned-by-refactor`) is explicitly **review/sweep-time, not
per-compile** (needs git history) — stays an intake row, not code here.

## Architecture decision (why a report lane, not a corpus rule)

The healer's corpus rules are per-span retrieval entries (buggy/fixed code pairs).
Classes A/B are **cross-function detectors**: field default in one site, ~35 const
reads across 9 files. The right shape is a **scanner + report lane** — the
`--frontier-report` precedent (issue 089 T1+T2, landed 2026-09-11): top-level
ungated module, pure analysis over already-parsed input, report-first gates
nothing. The `dist` domain (suggest-citations-only scanner) is the second
precedent.

- Module: `src/config_audit.rs`, **ungated** (zero new deps — `syn` is already a
  core dep; store-free; lib tests cover it at default features).
- CLI: `cargo heal --config-audit [PATHS...]` (default `.`) — read-only report,
  exit 0 always (a report that reds is a gate nobody runs).
- Two-pass directory scan:
  - **Pass 1** (collect): per `.rs` file, find `Default` impls + constructor fns;
    for each struct field whose init RHS is a bare path expression resolving to a
    named const (`AUDIT_RING_K`) or associated const (`Type::COUNT`), record
    `(struct, field, const_path, file:line)`.
  - **Pass 2** (count reads across the whole scanned set): field reads via
    `.field` access AND **destructuring** (`let Cfg { field, .. } = c`,
    `Cfg { field: alias, .. }` — the documented blind spot that put LIVE fields
    in the consumer-script's most-confident bucket; counting it is our precision
    edge) + const-path reads. Interaction kinds: `read`, `assert_only` (class B),
    `array_length` (structurally un-honorable — fires with a note).
- **Fires** when: const-path default ∧ zero `read` interactions ∧ ≥1 const read.
- Suggest-only output: file:line, field, const path, interaction counts, the
  behavioral-fix suggestion (wire to `self.field` / write a behavioral test).

## Scope notes (honest)

- The strict shape catches 2 of the 3 measured class-A rows (`ring_capacity`,
  `belief_decay_rate`). The `target_modules` row (default `LoraTarget::all().to_vec()`,
  consumer reads `LoraTarget::COUNT` — const is NOT the default RHS) is a V2
  variant; NOT in this plan.
- Upstream-crate limitation from the intake (7 of 22 rows were live in a sibling
  repo) applies in reverse: the detector only sees the scanned set. Report footer
  states the scope.
- Exit code is ALWAYS 0 (report lane; the `--frontier-report` law: report-first,
  gate-later — no admission gate until the report proves predictive).

## Tasks

- [x] T1 — Probe evidence (Batch-33 law: no rule without probe): temp crate with
  the A-shape (lib + bin variants + the destructuring live-field case), clippy
  1.98 JSON output — confirm zero existing lint fires on the inert shape (the
  boundary claim is measured, not assumed). — DONE: 0 findings in BOTH crate
  forms (/tmp/probe124).
- [x] T2 — `src/config_audit.rs`: the two-pass scanner + report formatter
  (`audit_sources`/`audit_paths` → `ConfigAuditReport`), pure, zero new deps.
- [x] T3 — CLI wiring: `--config-audit` in `bin/heal.rs` (help, struct, parse,
  dispatch) + `pub mod config_audit;` in `lib.rs`.
- [x] T4 — Fixture tests: 15 tests — inert fires; live via `.field`/
  destructuring (blind-spot guard)/non-assert macro silent; array-length
  fires w/ note; assert-only fires; A-over-B precedence; non-const silent;
  condition-3 self-read suppression; cross-file index; cross-type `::COUNT`
  collision guard; struct-update silent; + the T5-forced precision set
  (behavioral-assert-is-a-read, `None`-comparator, negated-bool tautology).
- [x] T5 — Real-world validation: riir-chain 177 files/41 candidates → 1 B
  (`GuardVerdict.redirect`, hand-verified accurate — only
  `assert_eq!(v.redirect, None)` vs its `None` default, zero reads, no
  chaind consumer); riir-neuron-db 102/76 → 0; self 232/34 → 0. ZERO false
  positives. The hand-check REFINED class B to the intake's own spec
  (against-Default asserts only; behavioral asserts are reads).
- [x] T6 — Docs: intake A+B marked DONE; AGENTS.md CLI contract line;
  HISTORY.md section; this plan.
- [x] T7 — GOAT: G1 15/15 fixtures + hand-verified real findings; G3 full lib
  suite green (1202 tests) + clippy clean at default features; G2/G4 N/A
  report-only; score-bench N/A — no proposer change.
- [x] T8 — Commit + push (`feat: plan 124 …`), status → COMPLETE.

## GOAT gate

G1 correctness = fixture matrix (T4) + hand-verified real-repo findings (T5).
G3 no-regression = repo's own clippy/tests green. G2/G4 not claimed (report-only;
no hot path, no alloc gate — the scanner is offline tooling). Promotion: nothing
to promote (no default-off feature; ungated-by-design per frontier precedent).
