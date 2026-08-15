//! `clippy::match_bool` — `match b { true => 1, false => 0 }` →
//! `if b { 1 } else { 0 }`.
//!
//! Pattern: a two-arm match, no attributes, no guards, whose arm patterns
//! are the boolean literals `true` and `false` (either order; bodies are
//! arbitrary).
//!
//! Span-preserving (Issue 016): the whole match's byte range is replaced by
//! `if <scrutinee> <true-body> else <false-body>` — scrutinee and bodies
//! keep their ORIGINAL source text; non-block bodies are braced. The if
//! condition takes any expression, so the scrutinee needs no parens.

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprMatch, File, Lit, Pat};

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
        if let Some(replacement) = self.try_match_bool(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_match_bool(&self, m: &ExprMatch) -> Option<String> {
        if m.arms.len() != 2 || !m.attrs.is_empty() {
            return None;
        }
        if m.arms.iter().any(|a| !a.attrs.is_empty() || a.guard.is_some()) {
            return None; // guards/arm-attrs would be lost by the splice
        }
        let scrutinee = src(self.source, &*m.expr)?;
        for (t, f) in [(&m.arms[0], &m.arms[1]), (&m.arms[1], &m.arms[0])] {
            if pat_is_bool(&t.pat, true) && pat_is_bool(&f.pat, false) {
                let t_body = braced_body(self.source, &t.body)?;
                let f_body = braced_body(self.source, &f.body)?;
                return Some(format!("if {scrutinee} {t_body} else {f_body}"));
            }
        }
        None
    }
}

/// Is this pattern the boolean literal `true`/`false`?
fn pat_is_bool(p: &Pat, want: bool) -> bool {
    matches!(p, Pat::Lit(pl) if matches!(&pl.lit, Lit::Bool(b) if b.value == want))
}

/// A match-arm body as an if/else body: block bodies keep their ORIGINAL
/// braces; expression bodies get fresh ones.
fn braced_body(source: &str, body: &Expr) -> Option<String> {
    let text = src(source, body)?;
    Some(match body {
        Expr::Block(_) => text,
        _ => format!("{{ {text} }}"),
    })
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn true_first_becomes_if_else() {
        let input = "fn f(b: bool) -> u8 { match b { true => 1, false => 0 } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(out.contains("if b { 1 } else { 0 }"), "not converted: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn false_first_also_rewrites() {
        let input = "fn f(b: bool) -> u8 { match b { false => 0, true => 1 } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(out.contains("if b { 1 } else { 0 }"), "arm order not normalized: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn block_bodies_keep_original_braces() {
        let input = "fn f(b: bool) { match b { true => { one(); }, false => { two(); } } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(out.contains("if b { one(); } else { two(); }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn three_arms_untouched() {
        let input = "fn f(b: bool) -> u8 { match b { true => 1, false => 0, _ => 2 } }";
        let out = propose_fix("match_bool", input);
        assert!(out.is_none(), "3-arm match must not fire: {out:?}");
    }

    #[test]
    fn non_bool_patterns_untouched() {
        let input = "fn f(x: u8) -> u8 { match x { 1 => 2, 3 => 4 } }";
        let out = propose_fix("match_bool", input);
        assert!(out.is_none(), "non-bool patterns must not fire: {out:?}");
    }

    #[test]
    fn guarded_arm_untouched() {
        let input = "fn f(b: bool) -> u8 { match b { true if b => 1, false => 0 } }";
        let out = propose_fix("match_bool", input);
        assert!(out.is_none(), "guarded arm must not fire: {out:?}");
    }
}
