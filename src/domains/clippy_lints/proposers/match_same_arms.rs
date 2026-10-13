//! `clippy::match_same_arms` — adjacent arms with identical bodies merge
//! their patterns: `p1 => B, p2 => B` → `p1 | p2 => B` (P51a, 2026-10-14;
//! pedantic group per the pinned 1.98.1 driver; probe-verified against the
//! driver's own merge suggestion, `MaybeIncorrect`).
//!
//! ## The conservative subset (v1)
//!
//! Real clippy computes pattern-overlap lattices to move arms across
//! interleaved arms and matches bodies with a typed `SpanlessEq`; this
//! proposer takes the SUBSET that is execution-equivalent by construction:
//!
//! - **ADJACENT arms only** — merging `A => B, B' => B` neighbours cannot
//!   reorder which arm a value selects (the `A | B'` or-pattern tries the
//!   alternatives in source order; identical bodies make the choice
//!   immaterial either way). Clippy's non-adjacent moves need overlap
//!   analysis we do not have — refused (silent direction).
//! - **Source-identical bodies** — the body TEXT of every arm in the run
//!   must be byte-identical (`src` on the span), strictly narrower than
//!   clippy's typed equality.
//! - **Binding-free patterns** — an arm whose pattern BINDS anything
//!   refuses the whole run: a body that names a binding could bind in one
//!   arm and shadow an outer name in the other (the merge would re-resolve
//!   it), and clippy's `bindings_eq` machinery is typed. Wildcards `_`
//!   bind nothing and are textual — but a wildcard inside a MERGE produces
//!   `p | _ => B`, valid yet exactly the shape upstream suggests DELETING
//!   instead; wildcard arms refuse (the delete-arm suggestion is a
//!   different rewrite, not taken).
//! - **No guards, no arm attrs, no match attrs** — same-guard merging is
//!   typed upstream and refused here.
//! - Runs of 2+ identical-bodied arms merge into ONE arm (`p1 | p2 | .. |
//!   pn => B`); the FIRST arm's body bytes are the surviving body, every
//!   arm's PATTERN bytes survive verbatim, everything else in the run is
//!   rebuilt scaffolding — the interior-comment guard refuses a run whose
//!   comments would drop.
//!
//! ## Why merging is safe
//!
//! A match executes exactly ONE arm per evaluation; the merged or-pattern
//! selects the same union of values with the same body — side effects,
//! divergence, and fall-through are untouched. Exhaustiveness and type
//! inference are unchanged (the same patterns exist, grouped).

use syn::visit::Visit;
use syn::{Arm, ExprMatch, File, Pat};

use super::helpers::{byte_range, drops_interior_comment, pattern_free};
use super::{src, SourceEdit};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { source, edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        syn::visit::visit_expr_match(self, m);
        if !m.attrs.is_empty() {
            return;
        }
        for run in identical_runs(self.source, m) {
            if run.len() < 2 {
                continue;
            }
            if let Some(se) = self.merge_run(m, &run) {
                self.edits.push(se);
            }
        }
    }
}

/// Maximal runs of CONSECUTIVE arms with byte-identical body text,
/// binding-free/attr-free/guard-free patterns.
fn identical_runs<'a>(source: &str, m: &'a ExprMatch) -> Vec<Vec<&'a Arm>> {
    let mut runs: Vec<Vec<&Arm>> = Vec::new();
    let mut cur: Vec<&Arm> = Vec::new();
    let mut cur_body: Option<String> = None;
    for arm in &m.arms {
        let body = src(source, &arm.body);
        let eligible = body.is_some()
            && arm.guard.is_none()
            && arm.attrs.is_empty()
            && !matches!(&arm.pat, Pat::Wild(_))
            && pattern_free(&arm.pat);
        let same = matches!((&cur_body, &body), (Some(a), Some(b)) if a == b);
        if eligible && same {
            cur.push(arm);
        } else {
            if cur.len() > 1 {
                runs.push(cur.clone());
            }
            cur = if eligible { vec![arm] } else { Vec::new() };
            cur_body = if eligible { body } else { None };
        }
    }
    if cur.len() > 1 {
        runs.push(cur);
    }
    runs
}

impl Collector<'_> {
    /// `p1 => B, p2 => B, ..` → `p1 | p2 | .. => B` — one edit over the
    /// whole run, the first arm's body as the survivor.
    fn merge_run(&self, m: &ExprMatch, run: &[&Arm]) -> Option<SourceEdit> {
        let first = run[0];
        let last = run[run.len() - 1];
        // Whole-run span: from the first arm's pattern to the last arm's
        // body end (the match's own span would swallow the scrutinee).
        let start = byte_range(&first.pat).start;
        let end = byte_range(&last.body).end;
        let mut kept: Vec<std::ops::Range<usize>> = run.iter().map(|a| byte_range(&a.pat)).collect();
        kept.push(byte_range(&first.body));
        if drops_interior_comment(self.source, start..end, &kept) {
            return None;
        }
        let mut pats = Vec::with_capacity(run.len());
        for arm in run {
            pats.push(src(self.source, &arm.pat)?);
        }
        let body = src(self.source, &first.body)?;
        let _ = m; // span is derived from the arms; the match node only gates attrs
        Some(SourceEdit {
            start,
            end,
            replacement: format!("{} => {}", pats.join(" | "), body),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn two_identical_arms_merge() {
        // The P51a production specimen shape (proposers/mod.rs:1291,
        // cycle 71): two dispatch arms with the same body.
        let input = "fn f(x: &str) -> u8 { match x { \"a\" => b(source, file), \"b\" => b(source, file), _ => 3 } }";
        let out = propose_fix("match_same_arms", input).expect("matched");
        assert!(
            out.contains("\"a\" | \"b\" => b(source, file)"),
            "patterns merge with a single body: {out}"
        );
        assert!(
            out.matches("=> b(source, file)").count() == 1,
            "the duplicate body is gone: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn three_arm_run_merges_all() {
        let input = "fn f(x: u8) -> u8 { match x { 0 => 9, 1 => 9, 2 => 9, _ => 0 } }";
        let out = propose_fix("match_same_arms", input).expect("matched");
        assert!(
            out.contains("0 | 1 | 2 => 9"),
            "a maximal run merges into one arm: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_adjacent_identical_arms_refused() {
        // Clippy merges these (overlap analysis); the v1 subset refuses.
        let input = "fn f(x: u8) -> u8 { match x { 0 => 9, _other => 8, 1 => 9 } }";
        let out = propose_fix("match_same_arms", input);
        assert!(out.is_none(), "non-adjacent merge needs overlap analysis: {out:?}");
    }

    #[test]
    fn binding_pattern_refuses_the_run() {
        let input =
            "fn f(x: Option<u8>) -> u8 { match x { Some(v) => 9, None => 9 } }";
        let out = propose_fix("match_same_arms", input);
        assert!(
            out.is_none(),
            "a run whose patterns bind must refuse (Some binds v): {out:?}"
        );
    }

    #[test]
    fn wildcard_arm_refuses() {
        let input = "fn f(x: u8) -> u8 { match x { 0 => 9, _ => 9 } }";
        let out = propose_fix("match_same_arms", input);
        assert!(
            out.is_none(),
            "wildcard-in-merge is upstream's DELETE suggestion, not a merge: {out:?}"
        );
    }

    #[test]
    fn guarded_arms_refuse() {
        let input = "fn f(x: u8) -> u8 { match x { 0 if x > 1 => 9, 1 => 9 } }";
        let out = propose_fix("match_same_arms", input);
        assert!(out.is_none(), "guards refuse the run: {out:?}");
    }

    #[test]
    fn different_bodies_untouched() {
        let input = "fn f(x: u8) -> u8 { match x { 0 => 9, 1 => 8 } }";
        let out = propose_fix("match_same_arms", input);
        assert!(out.is_none(), "different bodies never fire: {out:?}");
    }

    #[test]
    fn comment_between_arms_refused() {
        let input = "fn f(x: u8) -> u8 { match x { 0 => 9, // why\n 1 => 9 } }";
        let out = propose_fix("match_same_arms", input);
        assert!(out.is_none(), "a comment the splice would drop refuses: {out:?}");
    }
}
