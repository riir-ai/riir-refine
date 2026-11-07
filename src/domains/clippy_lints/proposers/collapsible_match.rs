//! `clippy::collapsible_match` — a match arm whose body is a plain `if` with
//! NO else collapses into a GUARD arm: `P => { if COND { B } }` →
//! `P if COND => B` (the rca specimen, cyclomatic.rs:120-127).
//!
//! Probe matrix (clippy 1.99.0, `/tmp/probe_cm` + the rust-code-analysis
//! capture `/tmp/rca_clippy.json`, plan 353):
//! - plain-if-no-else arm: FIRES, `MachineApplicable`, clippy's own splice is
//!   3-part (delete ` => {` on the arm line, `{` → `=> ` at the if-head's
//!   close, delete from the if-close through the arm's close brace). The
//!   whole-arm rewrite here produces the same guard form.
//! - if-let arm (the classic `Some(y) => { if let Some(z) = y {..} }`):
//!   FIRES upstream but is a PATTERN-MERGE rewrite needing binding-mode
//!   analysis — v1 diverges (the documented narrower-than-clippy class);
//!   the --clippy-json oracle lane covers those sites when fed.
//! - if-WITH-else arm: SILENT upstream — the no-else guard here is
//!   upstream-exact.
//! - the guard form does NOT re-fire (fixpoint clean, compile-verified).
//!
//! Semantics note (`context_note`): when the pattern matches and COND is
//! false, the guard form falls through to LATER arms where the body-if form
//! consumed the value (an arm-body `{}` of unit type is benign; anything
//! else is observable) — clippy ships the exact same shift; we mirror.
//! Guards cannot be fallible: a COND carrying the `?` token compiles in
//! body position but NOT as a guard (E0001-class) — refused. `.await` in
//! the cond is refused conservatively (guard-await legality is contextual;
//! the parse-only pass cannot arbitrate). The cond's evaluation ORDER is
//! unchanged (guards evaluate after the pattern matches, before the body —
//! the same position the body-if cond ran at). Guard borrows of the
//! scrutinee live through the arm body (E0506 class, check-visible — the
//! `fix_verify` gate's territory, disclosed not statically detected).

use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, File, ItemFn, Stmt};

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
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        // Nested items are skipped by construction: visit_item_fn re-entering
        // a nested fn is fine (arms inside nested fns are equally in scope),
        // matching the house shape.
        syn::visit::visit_item_fn(self, f);
    }

    fn visit_expr_match(&mut self, m: &'ast syn::ExprMatch) {
        syn::visit::visit_expr_match(self, m);
        for arm in &m.arms {
            if let Some(replacement) = self.try_guard_form(arm) {
                self.edits.push(edit(arm, replacement));
            }
        }
    }
}

impl Collector<'_> {
    /// `P => { if COND { B } }` → `P if COND => B`.
    fn try_guard_form(&self, arm: &syn::Arm) -> Option<String> {
        if !arm.attrs.is_empty() {
            return None; // attrs would be lost by the whole-arm rewrite
        }
        if arm.guard.is_some() {
            return None; // already a guard arm
        }
        let Expr::Block(body_block) = arm.body.as_ref() else {
            // The arm body must be a block to hold the if; `P => if C {..}`
            // (no outer braces) is the same collapse but a different shape —
            // conservative refuse.
            return None;
        };
        if body_block.block.stmts.len() != 1 {
            return None;
        }
        let Stmt::Expr(Expr::If(if_expr), _) = &body_block.block.stmts[0] else {
            return None;
        };
        // Upstream-exact: the if must have NO else (if-with-else is silent).
        if if_expr.else_branch.is_some() {
            return None;
        }
        // The then-branch becomes the guard arm's body verbatim — it must be
        // a block so the arm still consumes the value in the COND-true case.
        if if_expr.then_branch.stmts.is_empty() {
            return None; // empty then-block: `P => { if C {} }` — degenerate, refuse
        }
        // Guard-syntax legality: the cond may not carry `?` (guards cannot be
        // fallible — the original compiled in body position, the guard form
        // would not) nor `.await` (conservative — contextual legality).
        let cond_text = src(self.source, if_expr.cond.as_ref())?;
        if cond_text.contains('?') || cond_text.contains(".await") {
            return None;
        }
        // Pat/guard/cond/body texts verbatim; the replacement re-emits the
        // arm in the guard form. Then-branch block text rides from the
        // original span (inner layout preserved byte-identically).
        let pat_text = src(self.source, &arm.pat)?;
        let then_text = src(self.source, &if_expr.then_branch)?;
        let mut out = format!("{pat_text} if {cond_text} => {then_text}");
        if arm.comma.is_some() {
            out.push(',');
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn plain_if_arm_becomes_guard_arm() {
        // The rca specimen shape (multi-line cond with closures — the real
        // upstream diagnostic, rust-code-analysis cyclomatic.rs:120-127).
        let input = concat!(
            "enum K { ElseClause, Other, A, B }\n",
            "fn f(n: K, has: fn(K) -> bool) -> f32 {\n",
            "    let mut stats = 0.0;\n",
            "    match n {\n",
            "        K::A => { stats += 1.0; }\n",
            "        K::ElseClause => {\n",
            "            if has(n) {\n",
            "                stats += 1.0;\n",
            "            }\n",
            "        }\n",
            "        _ => {}\n",
            "    }\n",
            "    stats\n",
            "}\n",
        );
        let out = propose_fix("collapsible_match", input).expect("matched");
        assert!(
            out.contains("K::ElseClause if has(n) => {"),
            "guard form not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn multi_line_cond_with_closures_transfers() {
        // The exact rca shape: a multi-line cond containing closures.
        let input = concat!(
            "fn f(n: u8, w: fn(u8) -> bool) -> u32 {\n",
            "    match n {\n",
            "        1 => 0,\n",
            "        2 => {\n",
            "            if w(n) && w(n + 1) {\n",
            "                1\n",
            "            }\n",
            "            else {\n",
            "                0\n",
            "            }\n",
            "        }\n",
            "        _ => 2,\n",
            "    }\n",
            "}\n",
        );
        // if-with-else is SILENT upstream — must not fire.
        let out = propose_fix("collapsible_match", input);
        assert!(out.is_none(), "if-with-else must not fire: {out:?}");
    }

    #[test]
    fn rca_exact_shape_produces_the_guard_form() {
        // The real rca arm (cyclomatic.rs:120-127): no else, block body with
        // one statement, closures in the cond.
        let input = concat!(
            "fn f(n: u8, w: fn(u8) -> bool, v: fn(u8) -> bool) -> f32 {\n",
            "    let mut stats = 0.0;\n",
            "    match n {\n",
            "        1 => { stats += 1.0; }\n",
            "        2 => {\n",
            "            if w(n) && v(n) {\n",
            "                stats += 1.0;\n",
            "            }\n",
            "        }\n",
            "        _ => {}\n",
            "    }\n",
            "    stats\n",
            "}\n",
        );
        let out = propose_fix("collapsible_match", input).expect("matched");
        assert!(
            out.contains("2 if w(n) && v(n) => {\n"),
            "guard form not produced: {out}"
        );
        // The old arm shape must be gone.
        assert!(!out.contains("2 => {"), "old arm survives: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn cond_with_question_mark_refused() {
        // Guards cannot be fallible — `?` in the cond would break the guard
        // form (the parse-visible refusal class).
        let input = concat!(
            "fn f(o: Option<u8>) -> u8 {\n",
            "    match o {\n",
            "        Some(v) => {\n",
            "            if ok(v)? > 1 {\n",
            "                1\n",
            "            }\n",
            "        }\n",
            "        _ => 0,\n",
            "    }\n",
            "}\n",
        );
        let out = propose_fix("collapsible_match", input);
        assert!(out.is_none(), "fallible cond must not fire: {out:?}");
    }

    #[test]
    fn already_guard_arm_untouched() {
        let input = "fn f(n: u8, w: fn(u8) -> bool) -> u8 { match n { 1 if w(n) => 1, _ => 0 } }";
        let out = propose_fix("collapsible_match", input);
        assert!(out.is_none(), "guard arm touched: {out:?}");
    }

    #[test]
    fn multi_statement_arm_untouched() {
        // Upstream fires on multi-stmt bodies too?? — v1 is conservative:
        // single-stmt arms only (the rca shape), documented divergence.
        let input = concat!(
            "fn f(n: u8, w: fn(u8) -> bool) -> u8 {\n",
            "    match n {\n",
            "        2 => {\n",
            "            log(n);\n",
            "            if w(n) {\n",
            "                1\n",
            "            } else {\n",
            "                0\n",
            "            }\n",
            "        }\n",
            "        _ => 2,\n",
            "    }\n",
            "}\n",
        );
        let out = propose_fix("collapsible_match", input);
        assert!(out.is_none(), "multi-stmt arm fired: {out:?}");
    }

    #[test]
    fn trailing_comma_reemitted() {
        let input = concat!(
            "fn f(n: u8, w: fn(u8) -> bool) -> u8 {\n",
            "    match n {\n",
            "        2 => {\n",
            "            if w(n) {\n",
            "                1\n",
            "            }\n",
            "        },\n",
            "        _ => 0,\n",
            "    }\n",
            "}\n",
        );
        let out = propose_fix("collapsible_match", input).expect("matched");
        assert!(out.contains("=> {"), "shape: {out}");
        assert!(
            out.contains("2 if w(n) => {\n                1\n            },"),
            "comma lost: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }
}
