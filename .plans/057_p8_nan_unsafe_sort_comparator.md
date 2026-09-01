# Plan 057 — P8 intake: `nan-unsafe-sort-comparator` (sec domain)

**Status:** IN PROGRESS
**Source:** riir-clippy `.distill/001_mining_queue_snapshot.md` §Post-mining intake,
open class P8 (filed 2026-09-01 from riir-ai Issue 832 T5's lint verdict).
**Repo:** riir-clippy only. **Domain vehicle:** `sec` (the P8 verdict's
detect-only tier + the domain's panic-safety charter).

## The class

`v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))` — NaN-unsafe
float sort comparator. Two live defects: (a) `partial_cmp` returns `None` on
NaN, so the comparator orders NaN as equal to EVERYTHING — an inconsistent
comparator (not a total order); (b) since Rust 1.81 the std sorts document
that they MAY PANIC ("user-provided comparison function does not correctly
implement a total order") on exactly that input. ≈300 library sites
workspace-wide (katgpt-rs 164 / riir-train 77 / riir-clippy 21 / ndb 27).

## The probe (Protocol C step 2 — evidence first)

Temp crate `/tmp/p8_probe`, real clippy 1.97 with
`-W clippy::pedantic -W clippy::nursery -W clippy::perf -W clippy::suspicious
-W clippy::correctness -W clippy::style` over four fixture shapes (f32/f64
sort, sort_unstable_by + `.unwrap()`, select_nth_unstable_by, reversed args):

**ZERO clippy lints fire** on the pattern (only trivial
`missing_panics_doc` on the `.unwrap()` variant). The pattern is genuinely
unoccupied upstream → the sec boundary ("no equivalent clippy fix exists")
is satisfied with margin. The sec boundary gate iterates the corpus
automatically, so the new fixture is covered without gate edits.

## Why the P8 verdict's type-blindness concern dissolves

The intake argued the fix is not parse-only-rewritable because the
replacement needs operand DIRECTION and float TYPE (it assumed the
katgpt-core `float_order::{desc,asc,desc_f64,asc_f64}` twins). **std
`total_cmp` dissolves both:**

- Direction: `a.partial_cmp(b)` → `a.total_cmp(&b)` preserves the recv/arg
  order mechanically — the direction rides the same positions.
- Type: `total_cmp` exists name-identically on BOTH `f32` and `f64`.

Residual type-blind hazard (honest): a non-float receiver with
`partial_cmp` but no `total_cmp` (e.g. `Option<f32>`) fails E0599 — the
`--fix --verify` compile gate auto-reverts it (the constant-time-secret-
compare precedent). The proposer stays conservative (strict gates below)
and every applied edit rides the verify gate.

## Design

- **Rule:** `nan-unsafe-sort-comparator`, new family `SecFamily::PanicSafety`
  (label `panic-safety`) — std-sorts-panic-on-inconsistent-comparator is a
  panic-containment pattern, sibling of the Drop-glue and len-zero rules.
- **Proposer** (FIXABLE_SEC, strict-gated — never a blind fix):
  1. enclosing fn (any depth) contains a `sort_by` / `sort_unstable_by` /
     `select_nth_unstable_by` call (the intake's precision guard);
  2. the chain is EXACTLY `<recv>.partial_cmp(<1 arg>)` + one trailing
     `.unwrap_or(<path ending in Equal>)` or `.unwrap()` — deeper chains
     rewrite only the matched prefix (`.reverse()` etc. compose);
  3. comment guard: any comment inside the chain span refuses (the house
     `drops_interior_comment` discipline);
  4. macro args refuse; `unwrap_or` with a non-`Equal` default refuses
     (a different default is a semantic decision, never mechanical);
  5. arg emission: `&arg`, parenthesized for non-place exprs (Binary/Cast/
     Range/...); `Expr::Reference` args kept verbatim (`&&T` deref-coerces).
- **Boundary:** zero clippy coverage (probe above) — the gate's generic
  iteration covers the new fixture with no table edits.

## Tasks

- [ ] T1 — `SecFamily::PanicSafety` variant + `label()` arm (grep-verified:
      no other exhaustive matches on `SecFamily`).
- [ ] T2 — corpus entry `nan-unsafe-sort-comparator` (buggy = the
      `unwrap_or(Equal)` f32-sort shape; fixed = `total_cmp`) + corpus
      header-table row.
- [ ] T3 — proposer `sec/proposers/nan_safe_cmp.rs` + registry wiring +
      `FIXABLE_SEC` entry; module tests (fires / sort-guard refusal /
      non-Equal refusal / macro refusal / comment refusal / reversed-args /
      `.unwrap()` form / prefix-composition / self-variant round-trip).
- [ ] T4 — validation: default lib tests, `--features sec` lib tests,
      `sec_boundary_gate` (needs cargo; runs live), retrieval gate under
      `latent_retrieval` (floors HELD or honestly re-pinned + recorded),
      `--score-bench` run appended to the history, clippy 0 touched files.
- [ ] T5 — docs: `.docs/11_domains/sec.md` rule table + AGENTS.md sec
      section + the P8 intake checkbox marked DONE with the commit hash
      (+ the total_cmp refinement recorded in the intake's entry).
- [ ] T6 — commit `feat: sec rule nan-unsafe-sort-comparator (P8 intake)` + push.

## Non-goals

- No sweep of the ~300 sibling sites this cycle (the capability lands; the
  sweep is a later unit per repo, heal-driven with `--features sec`).
- No katgpt-core `float_order` emission in the rewrite (std `total_cmp`
  is dependency-free and type-agnostic; the workspace's own 832 fix used
  float_order for its in-crate consistency, not as a tool mandate).
- No `rust_perf` bench claim — this is a correctness/panic-safety rule;
  a perf framing would be dishonest (the compare cost is a wash).
