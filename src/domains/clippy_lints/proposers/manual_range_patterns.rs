//! `clippy::manual_range_patterns` — `0 | 1 | 2` → `0..=2` (and the wider
//! family: scrambled, descending, duplicate, negative, and range-bearing
//! or-patterns).
//!
//! This is a FAITHFUL PORT of clippy's own algorithm, not an approximation:
//! the lint's source at rust-lang/rust@`1.98.1`
//! (`src/tools/clippy/clippy_lints/src/manual_range_patterns.rs`) was
//! transcribed statement-for-statement onto syn, and the transcribed
//! behavior was probed against the INSTALLED oracle on 25 cells across two
//! toolchains (0.1.93 stable and 0.1.98 — the workspace pin — identical on
//! every cell). The gates below are clippy's, including the non-obvious
//! ones a naive "ascending contiguous literals" rule would miss:
//!
//! - a bare int literal, OR a bounded range alternative (`a..=b` / `a..b`) —
//!   anything else (bindings, chars, unbounded/half-open ranges, nested
//!   sub-patterns) declines the WHOLE pattern;
//! - `cases >= 3`, OR `cases > 1` when at least one case is a range
//!   (`4..=6 | 9` fires at two alternatives);
//! - min tracks its candidate on `<=` (ties → the LATER literal wins),
//!   a plain-num max on `>=` (ties → later, and the range-end KIND resets
//!   to `..=`), a range-right max on strict `>` (the range's own end kind
//!   is kept — `0..2 | 1` suggests `0..=1`, value-decremented but
//!   text-verbatim right end with the kind coming from the winner);
//! - the coverage walk: every value in `min..=max` must be covered by a
//!   found number or a found range, gaps decline (`1..=3 | 5` is silent,
//!   `1..5 | 2 | 9` is silent);
//! - the suggestion is `{min_text}{..= or ..}{max_text}` — the boundary
//!   literals' source text VERBATIM (sign, digit separators, suffix:
//!   `1u32 | 2 | 3` → `1u32..=3`, `1_0 | 1_1 | 1_2` → `1_0..=1_2`,
//!   `-3 | -2 | -1` → `-3..=-1`), so the rewrite is set-identical AND
//!   inference-identical.
//!
//! Boundary notes (probed, not assumed):
//! - scrambled NEGATIVES fire too (`-1 | -3 | -2` → `-3..=-1`) — the
//!   order-free set semantics reach them through the negated-literal
//!   representation; probe-confirmed, and it is exactly the shape a
//!   pairwise-ascending port would wrongly refuse;
//! - u128 literals above `i128::MAX` decline (clippy's own documented
//!   `i128::try_from` limit — "intentionally not handling numbers greater
//!   than `i128::MAX` ... to support negative numbers").
//!
//! Divergences from the oracle, all conservative (we stay silent or
//! rewrite set-identically where clippy acts):
//! - code inside macro invocations is invisible to syn (crate-wide
//!   property of every proposer here; clippy sees post-expansion HIR);
//! - a paren-grouped case chain the outer pattern cannot fully collapse
//!   (`(0 | 1 | 2) | 5` — gap at 4) still rewrites its inner chain to
//!   `(0..=2) | 5`, one step clippy would not take (it stays silent);
//!   the full collapse `(0 | 1 | 2) | 3` → `0..=3` matches clippy;
//! - a coverage-jump landing exactly past `i128::MAX` declines instead of
//!   reproducing clippy's release-mode wrap-around (reachable only from a
//!   range ending at `i128::MAX`, where clippy terminates by wrap luck —
//!   the healer runs debug builds and must not panic).
//!
//! FIXABLE (not VERIFY): set-identical + inference-identical replacement —
//! if the input compiled, the output compiles. A comment INSIDE the
//! or-pattern span declines via the shared [`drops_interior_comment`] guard
//! (the emitter deletes dropped-gap bytes; clippy never displaces comments,
//! Issue 027/028/029 class).

use super::helpers::{byte_range, drops_interior_comment};
use super::{edit, src, SourceEdit};
use std::collections::HashSet;
use syn::visit::Visit;
use syn::{Expr, File, Lit, Pat, PatOr, RangeLimits, UnOp};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_pat_or(&mut self, o: &'ast PatOr) {
        // Decide BEFORE descending: when the whole or-pattern rewrites, its
        // span is replaced wholesale and any nested or-pattern inside dies
        // with it — descending would emit an overlapping second edit.
        if let Some(replacement) = rewrite(self.source, o) {
            if drops_interior_comment(self.source, byte_range(o), &[]) {
                return;
            }
            self.edits.push(edit(o, replacement));
            return;
        }
        syn::visit::visit_pat_or(self, o);
    }
}

/// HIR hands clippy a FLAT, paren-free or-chain (`(0 | 1 | 2) | 3` lowers
/// to one `PatKind::Or` with four alternatives); mirror that before
/// classifying, or every paren-grouped chain would wrongly decline.
fn flatten<'a>(pat: &'a Pat, out: &mut Vec<&'a Pat>) {
    match pat {
        Pat::Or(o) => {
            for case in &o.cases {
                flatten(case, out);
            }
        }
        Pat::Paren(p) => flatten(&p.pat, out),
        _ => out.push(pat),
    }
}

/// The `<min>..=<max>` replacement text when the or-pattern is the oracle's
/// firing shape, else `None`.
fn rewrite(source: &str, o: &PatOr) -> Option<String> {
    let mut cases = Vec::new();
    for case in &o.cases {
        flatten(case, &mut cases);
    }
    let any_range = cases.iter().any(|p| matches!(p, Pat::Range(_)));
    if !(cases.len() >= 3 || (cases.len() > 1 && any_range)) {
        return None;
    }

    // Clippy's `Num` sentinels (i128::MAX / i128::MIN with dummy spans) are
    // `None` here — the suggestion only ever reads a real boundary.
    let mut min: Option<(i128, String)> = None;
    let mut max: Option<(i128, String)> = None;
    let mut kind_included = true;
    let mut numbers_found = HashSet::new();
    let mut ranges_found = Vec::with_capacity(cases.len());

    for case in &cases {
        if let Pat::Lit(lit) = case {
            let (val, text) = int_num(&lit.lit)?;
            numbers_found.insert(val);
            // `Num::min` keeps its candidate on strict less — ties hand the
            // span to the LATER literal.
            if min.as_ref().is_none_or(|(m, _)| val <= *m) {
                min = Some((val, text.clone()));
            }
            if max.as_ref().is_none_or(|(m, _)| val >= *m) {
                max = Some((val, text));
                kind_included = true;
            }
        } else if let Pat::Range(r) = case {
            // Half-open range alternatives (`..=5`, `1..`) decline the whole
            // pattern — clippy matches `PatKind::Range(Some, Some, _)`.
            let lo = r.start.as_deref()?;
            let hi = r.end.as_deref()?;
            let (lo_val, lo_text) = expr_num(source, lo)?;
            let (hi_val, hi_text) = expr_num(source, hi)?;
            let included = matches!(r.limits, RangeLimits::Closed(_));
            // `0..2` matches 0..=1: the decrement is VALUE-only — the right
            // end's text stays verbatim and the end KIND carries the `..`.
            let hi_eff = if included {
                hi_val
            } else {
                hi_val.checked_sub(1)?
            };
            if min.as_ref().is_none_or(|(m, _)| lo_val <= *m) {
                min = Some((lo_val, lo_text));
            }
            // The range-right max update is STRICT — a later equal num keeps
            // its own text and resets the kind (probe: `0..2 | 1` → `0..=1`).
            if max.as_ref().is_none_or(|(m, _)| hi_eff > *m) {
                max = Some((hi_eff, hi_text));
                kind_included = included;
            }
            ranges_found.push((lo_val, hi_eff));
        } else {
            return None;
        }
    }

    // The coverage walk (clippy verbatim): every value under the span must
    // be claimed by a found number or the widest containing found range.
    let (min_val, min_text) = min?;
    let (max_val, max_text) = max?;
    let mut num = min_val;
    while num <= max_val {
        if numbers_found.contains(&num) {
            // Clippy's plain `num + 1`: only reachable here for a range
            // ending at i128::MAX, where clippy survives by release-mode
            // wrap-around — decline instead of overflowing a debug build.
            num = num.checked_add(1)?;
        } else if let Some((_, hi)) = ranges_found
            .iter()
            .filter(|(lo, hi)| *lo <= num && num <= *hi)
            .max_by_key(|(_, hi)| *hi)
        {
            num = hi.checked_add(1)?;
        } else {
            return None;
        }
    }

    Some(format!(
        "{min_text}{}{max_text}",
        if kind_included { "..=" } else { ".." }
    ))
}

/// Clippy's `expr_as_i128` twin — the literal's value PLUS its verbatim
/// source text. Pattern literals arrive with an optional sign FUSED into
/// the token (`-3`, `1_0`, `900_u64`); magnitudes above `i128::MAX` decline
/// (clippy's own documented limit).
fn int_num(lit: &Lit) -> Option<(i128, String)> {
    let Lit::Int(li) = lit else {
        return None;
    };
    let val = li.base10_parse::<i128>().ok()?;
    Some((val, li.token().to_string()))
}

/// Range ends are EXPRESSIONS: syn hands `-3` over as `Unary(Neg)` there
/// (the sign is only token-fused for bare pattern literals), and the text
/// must come off the source span so the minus survives into the suggestion.
fn expr_num(source: &str, e: &Expr) -> Option<(i128, String)> {
    match e {
        Expr::Lit(l) => int_num(&l.lit),
        Expr::Unary(u) if matches!(u.op, UnOp::Neg(_)) => {
            let Expr::Lit(l) = &*u.expr else {
                return None;
            };
            let (mag, _) = int_num(&l.lit)?;
            Some((-mag, src(source, e)?))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    fn healed(code: &str) -> String {
        propose_fix("manual_range_patterns", code).expect("pattern matched")
    }

    fn refuses(code: &str) {
        assert!(
            propose_fix("manual_range_patterns", code).is_none(),
            "must stay silent: {code}"
        );
    }

    #[test]
    fn plain_ascending_rewrites() {
        let out = healed("fn f(n: u32) -> u8 { match n { 0 | 1 | 2 => 9, _ => 0 } }");
        assert!(out.contains("0..=2 => 9"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn descending_rewrites_to_value_min_max() {
        let out = healed("fn f(n: u32) -> u8 { match n { 12 | 11 | 10 => 9, _ => 0 } }");
        assert!(out.contains("10..=12 => 9"), "{out}");
    }

    #[test]
    fn scrambled_rewrites_sorted() {
        let out = healed("fn f(n: u32) -> u8 { match n { 9 | 7 | 8 => 9, _ => 0 } }");
        assert!(out.contains("7..=9 => 9"), "{out}");
    }

    #[test]
    fn duplicates_absorb() {
        let out = healed("fn f(n: u32) -> u8 { match n { 1 | 2 | 2 => 9, _ => 0 } }");
        assert!(out.contains("1..=2 => 9"), "{out}");
        let out = healed("fn f(n: u32) -> u8 { match n { 1 | 2 | 3 | 2 => 9, _ => 0 } }");
        assert!(out.contains("1..=3 => 9"), "{out}");
    }

    #[test]
    fn two_cases_stay_silent() {
        refuses("fn f(n: u32) -> u8 { match n { 5 | 6 => 9, _ => 0 } }");
        refuses("fn f(n: u32) -> u8 { match n { 12 | 11 => 9, _ => 0 } }");
    }

    #[test]
    fn gaps_stay_silent() {
        refuses("fn f(n: u32) -> u8 { match n { 1 | 3 | 5 => 9, _ => 0 } }");
        refuses("fn f(n: u32) -> u8 { match n { 0 | 1 | 2 | 4 => 9, _ => 0 } }");
    }

    #[test]
    fn bound_cases_stay_silent() {
        refuses("fn f(n: u32) -> u8 { match n { w @ 0 | w @ 1 | w @ 2 => 9, _ => 0 } }");
    }

    #[test]
    fn char_cases_stay_silent() {
        refuses("fn f(c: char) -> u8 { match c { 'a' | 'b' | 'c' => 9, _ => 0 } }");
    }

    #[test]
    fn negative_ascending_rewrites() {
        let out = healed("fn f(n: i32) -> u8 { match n { -3 | -2 | -1 => 9, _ => 0 } }");
        assert!(out.contains("-3..=-1 => 9"), "{out}");
    }

    #[test]
    fn negative_scrambled_rewrites() {
        // Probe-confirmed against 0.1.93 AND 0.1.98: the set semantics reach
        // negated literals regardless of source order.
        let out = healed("fn f(n: i32) -> u8 { match n { -1 | -3 | -2 => 9, _ => 0 } }");
        assert!(out.contains("-3..=-1 => 9"), "{out}");
    }

    #[test]
    fn mixed_sign_rewrites() {
        let out = healed("fn f(n: i32) -> u8 { match n { -1 | 0 | 1 => 9, _ => 0 } }");
        assert!(out.contains("-1..=1 => 9"), "{out}");
    }

    #[test]
    fn two_alternatives_with_range_fire() {
        // Two alternatives fire ONLY when a range case joins the or and the
        // span is fully covered — `4..=6 | 9` stays silent (7-8 uncovered,
        // probe-confirmed); `4..=6 | 7` collapses.
        let out = healed("fn f(n: u32) -> u8 { match n { 4..=6 | 7 => 9, _ => 0 } }");
        assert!(out.contains("4..=7 => 9"), "{out}");
        let out = healed("fn f(n: u32) -> u8 { match n { 1..=3 | 2 => 9, _ => 0 } }");
        assert!(out.contains("1..=3 => 9"), "{out}");
        refuses("fn f(n: u32) -> u8 { match n { 4..=6 | 9 => 9, _ => 0 } }");
    }

    #[test]
    fn excluded_range_keeps_verbatim_end_text() {
        // Probe-confirmed (`1..5 | 2 | 3` → `1..5`): the max boundary is the
        // range's right-end literal, its TEXT stays verbatim (`5`) even
        // though the matched value is decremented to 4 — the open `..` kind
        // carries the exclusion, so the suggestion is byte-faithful to what
        // clippy emits.
        let out = healed("fn f(n: u32) -> u8 { match n { 1..5 | 2 | 3 => 9, _ => 0 } }");
        assert!(out.contains("1..5 => 9"), "{out}");
        let out = healed("fn f(n: u32) -> u8 { match n { 1..5 | 2 => 9, _ => 0 } }");
        assert!(out.contains("1..5 => 9"), "{out}");
    }

    #[test]
    fn excluded_kind_resets_when_a_num_takes_max() {
        // Probe-confirmed (`0..2 | 1` → `0..=1`): the num's `>=`-tie steals
        // max (later text `1`) and resets the kind to `..=`.
        let out = healed("fn f(n: u32) -> u8 { match n { 0..2 | 1 => 9, _ => 0 } }");
        assert!(out.contains("0..=1 => 9"), "{out}");
    }

    #[test]
    fn uncovered_gaps_with_ranges_stay_silent() {
        refuses("fn f(n: u32) -> u8 { match n { 1..=3 | 5 => 9, _ => 0 } }");
        refuses("fn f(n: u32) -> u8 { match n { 1..5 | 2 | 9 => 9, _ => 0 } }");
    }

    #[test]
    fn negative_range_alternative_fires() {
        // Range ends arrive as negated EXPRESSIONS in syn (not fused tokens).
        let out = healed("fn f(n: i32) -> u8 { match n { -3..=-1 | -2 => 9, _ => 0 } }");
        assert!(out.contains("-3..=-1 => 9"), "{out}");
    }

    #[test]
    fn boundary_texts_survive_verbatim() {
        let out = healed("fn f(n: u8) -> u8 { match n { 0u8 | 1u8 | 2u8 => 9, _ => 0 } }");
        assert!(out.contains("0u8..=2u8 => 9"), "{out}");
        let out = healed("fn f(n: u32) -> u8 { match n { 1u32 | 2 | 3 => 9, _ => 0 } }");
        assert!(out.contains("1u32..=3 => 9"), "{out}");
        let out = healed("fn f(n: u32) -> u8 { match n { 1_0 | 1_1 | 1_2 => 9, _ => 0 } }");
        assert!(out.contains("1_0..=1_2 => 9"), "{out}");
        let out =
            healed("fn f(n: u64) -> u8 { match n { 900_u64 | 901_u64 | 902_u64 => 9, _ => 0 } }");
        assert!(out.contains("900_u64..=902_u64 => 9"), "{out}");
    }

    #[test]
    fn u128_above_i128_max_stays_silent() {
        // Clippy's own documented limit: i128::try_from fails on the
        // magnitude, the whole pattern declines.
        refuses(
            "fn f(n: u128) -> u8 { match n { 340282366920938463463374607431768211454 | \
             340282366920938463463374607431768211455 | 1 => 9, _ => 0 } }",
        );
    }

    #[test]
    fn paren_grouped_chain_collapses_like_hir() {
        // HIR flattens `(0 | 1 | 2) | 3` into one four-alternative or.
        let out = healed("fn f(n: u32) -> u8 { match n { (0 | 1 | 2) | 3 => 9, _ => 0 } }");
        assert!(out.contains("0..=3 => 9"), "{out}");
    }

    #[test]
    fn uncollapsible_outer_still_rewrites_inner() {
        // Gap at 4 declines the whole; the inner chain still collapses —
        // set-identical, one step clippy does not take (documented
        // divergence, conservative direction is rewrite-not-break).
        let out = healed("fn f(n: u32) -> u8 { match n { (0 | 1 | 2) | 5 => 9, _ => 0 } }");
        assert!(out.contains("(0..=2) | 5 => 9"), "{out}");
    }

    #[test]
    fn nested_in_ctor_pattern_fires_at_the_inner_span() {
        let out = healed(
            "fn f(o: Option<u32>) -> u8 { match o { Some(0 | 1 | 2) => 9, None => 0, _ => 7 } }",
        );
        assert!(out.contains("Some(0..=2) => 9"), "{out}");
    }

    #[test]
    fn comment_inside_the_span_declines() {
        // The emitter deletes dropped-gap bytes; a comment inside the
        // or-pattern would be deleted with them (Issue 027/028/029 class).
        refuses("fn f(n: u32) -> u8 { match n { 0 | 1 | /* mid */ 2 => 9, _ => 0 } }");
    }

    #[test]
    fn already_range_has_no_or_pattern() {
        refuses("fn f(n: u32) -> u8 { match n { 4..=6 => 9, _ => 0 } }");
    }

    #[test]
    fn untouched_bytes_survive() {
        let src = "// header\nfn f(n: u32) -> u8 {\n    // keep me\n    match n {\n        0 | 1 | 2 => 9,\n        _ => 0,\n    }\n}\n";
        let out = healed(src);
        assert!(out.contains("// header\n"), "{out}");
        assert!(out.contains("    // keep me\n"), "{out}");
        assert!(out.contains("0..=2 => 9"), "{out}");
    }
}
