//! `clippy::match_bool` — `match b { true => 1, false => 0 }` →
//! `if b { 1 } else { 0 }`.
//!
//! Pattern: a two-arm match, no attributes, no guards, whose arm patterns
//! are the boolean literals `true` and `false` (either order; bodies are
//! arbitrary).
//!
//! Extended shape (Issue-029 miss class, found live in riir-neuron-db
//! `bench_478`): a three-arm match `[P => A, !P if g => B, !P => {}]` — the
//! guarded middle arm plus an EMPTY same-polarity fallback — rewrites to
//! `if s { A } else if g { B }` (the empty fallback is the implicit else;
//! real clippy fires `match_bool` on this shape with no autofix of its
//! own). Only this exact polarity layout is converted: a guard on the
//! FIRST arm would need a nested `if`, which is a different rewrite.
//!
//! Span-preserving (Issue 016): the whole match's byte range is replaced by
//! `if <scrutinee> <true-body> else <false-body>` — scrutinee and bodies
//! keep their ORIGINAL source text; non-block bodies are braced. The if
//! condition takes any expression, so the scrutinee needs no parens.

use super::uninlined_format_args::all_expr_args;
use super::{edit, src, SourceEdit};
use proc_macro2::TokenStream;
use syn::visit::Visit;
use syn::{Expr, ExprMacro, ExprMatch, File, Lit, Macro, Pat, StmtMacro};

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

    /// Match expressions inside format-family macro args are invisible to
    /// syn's default visitor (macro token streams are opaque), but real
    /// clippy 1.97 DOES fire `match_bool` there — probed on every position:
    /// `println!("{}", match b { true => .., false => .. })` (post-template
    /// arg), `assert!(match b { .. })` (assert operand), a match nested in
    /// a CALL inside a macro arg. Parse the macro's expression args and
    /// visit them with `self` — spans stay file-relative through the
    /// token round-trip, and the recursive visit covers arbitrary nesting
    /// depth. Only KNOWN-family macros are opened (an arbitrary macro's
    /// args are not safely parseable as expressions).
    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        syn::visit::visit_expr_macro(self, m);
        self.scan_macro_args(&m.mac);
    }

    /// Statement-position macros (`println!(…);`) parse as `Stmt::Macro`,
    /// not `Expr::Macro` — the print family lives there, so both shapes
    /// are visited.
    fn visit_stmt_macro(&mut self, m: &'ast StmtMacro) {
        syn::visit::visit_stmt_macro(self, m);
        self.scan_macro_args(&m.mac);
    }
}

impl Collector<'_> {
    /// Visit every expression argument of a known format-family macro.
    /// ALL args are walked (pre-template operands included — the
    /// `assert!(match ..)` probe position fires real clippy), recursing
    /// through `self.visit_expr` so nested family macros and matches
    /// inside calls are reached at any depth.
    fn scan_macro_args(&mut self, mac: &Macro) {
        let Some(args) = all_expr_args(mac) else {
            return;
        };
        for arg in args {
            let stream: TokenStream = arg.toks.iter().cloned().collect();
            if let Ok(expr) = syn::parse2::<Expr>(stream) {
                self.visit_expr(&expr);
            }
        }
    }

    fn try_match_bool(&self, m: &ExprMatch) -> Option<String> {
        if !m.attrs.is_empty() {
            return None;
        }
        if m.arms.len() == 2 {
            return self.try_two_arm(m);
        }
        if m.arms.len() == 3 {
            return self.try_guarded_fallback(m);
        }
        None
    }

    /// The classic two-arm form: `[true => A, false => B]` (either order,
    /// no guards).
    fn try_two_arm(&self, m: &ExprMatch) -> Option<String> {
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

    /// The guarded-fallback form (the `bench_478` shape): `[P => A, !P if g => B,
    /// !P => {}]` → `if s { A } else if g { B }` (or `if !s { A } else if
    /// g { B }` when the first arm is the `false` arm). The empty last arm
    /// is the implicit else; the guard becomes the `else if` condition.
    fn try_guarded_fallback(&self, m: &ExprMatch) -> Option<String> {
        let (first, mid, last) = (&m.arms[0], &m.arms[1], &m.arms[2]);
        if m.arms.iter().any(|a| !a.attrs.is_empty()) {
            return None;
        }
        if first.guard.is_some() || last.guard.is_some() || mid.guard.is_none() {
            return None; // only the middle arm carries the guard
        }
        let first_is_true = pat_is_bool(&first.pat, true);
        let mid_lit = if pat_is_bool(&mid.pat, true) {
            true
        } else if pat_is_bool(&mid.pat, false) {
            false
        } else {
            return None;
        };
        // The middle + last arms must share the polarity complementary to
        // the first arm, and the last arm must be the EMPTY fallback.
        if mid_lit == first_is_true {
            return None;
        }
        if !pat_is_bool(&last.pat, mid_lit) {
            return None;
        }
        if !body_is_empty(&last.body) {
            return None;
        }
        let scrutinee = src(self.source, &*m.expr)?;
        let guard = src(self.source, &*mid.guard.as_ref().unwrap().1)?;
        let first_body = braced_body(self.source, &first.body)?;
        let mid_body = braced_body(self.source, &mid.body)?;
        let cond = if first_is_true {
            format!("if {scrutinee}")
        } else {
            format!("if !{scrutinee}")
        };
        Some(format!("{cond} {first_body} else if {guard} {mid_body}"))
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

/// Is this arm body the empty fallback (`{}` block or unit `()`)?
fn body_is_empty(body: &Expr) -> bool {
    match body {
        Expr::Block(b) => b.block.stmts.is_empty(),
        Expr::Tuple(t) => t.elems.is_empty(),
        _ => false,
    }
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
    fn guarded_fallback_true_first_rewrites() {
        // The bench_478 production shape: guarded middle arm + empty
        // same-polarity fallback becomes an if / else-if chain.
        let input = "fn f(b: bool, g: bool) { match b { true => x(), false if g => y(), false => {} } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("if b { x() } else if g { y() }"),
            "guarded fallback not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn guarded_fallback_false_first_rewrites() {
        let input = "fn f(b: bool, g: bool) { match b { false => x(), true if g => y(), true => {} } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("if !b { x() } else if g { y() }"),
            "mirror polarity not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn guarded_fallback_nonempty_last_untouched() {
        let input =
            "fn f(b: bool, g: bool) { match b { true => x(), false if g => y(), false => z() } }";
        let out = propose_fix("match_bool", input);
        assert!(out.is_none(), "non-empty fallback must not fire: {out:?}");
    }

    #[test]
    fn guarded_fallback_same_polarity_untouched() {
        let input =
            "fn f(b: bool, g: bool) { match b { true => x(), true if g => y(), false => {} } }";
        let out = propose_fix("match_bool", input);
        assert!(out.is_none(), "same-polarity guard must not fire: {out:?}");
    }

    #[test]
    fn guarded_fallback_first_guard_untouched() {
        let input =
            "fn f(b: bool, g: bool) { match b { true if g => x(), false => y(), false => {} } }";
        let out = propose_fix("match_bool", input);
        assert!(out.is_none(), "guard on the first arm needs a nested rewrite: {out:?}");
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

    #[test]
    fn macro_arg_post_template_rewrites() {
        // The ndb `ed_freeze_gate_poc` production shape — real clippy 1.97
        // fires match_bool on the match inside the println! arg (probed).
        let input =
            "fn f(b: bool, p: f64) { println!(\"sign={} p={p:.3}\", match b { true => \"OK\", false => \"NO\" }); }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("println!(\"sign={} p={p:.3}\", if b { \"OK\" } else { \"NO\" })"),
            "macro-arg match not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn macro_arg_stmt_macro_rewrites() {
        // writeln! in statement position parses as Stmt::Macro — covered
        // by the second visitor.
        let input =
            "fn f(w: &mut String, c: bool) { writeln!(w, \"{}\", match c { true => 1, false => 0 }).unwrap(); }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(out.contains("if c { 1 } else { 0 }"), "stmt-macro arg not converted: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn macro_arg_assert_operand_rewrites() {
        // Probe-verified: real clippy fires match_bool on the assert!
        // OPERAND position too (pre-template arg).
        let input =
            "fn f(b: bool, a: bool, c: bool) { assert!(match b { true => a, false => c }); }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(out.contains("assert!(if b { a } else { c })"), "operand not converted: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn macro_arg_nested_in_call_rewrites() {
        // A match inside a CALL inside a macro arg — the recursive
        // visit_expr descent reaches it at depth 2.
        let input =
            "fn f(b: bool) { println!(\"{}\", str_len(match b { true => \"A\", false => \"BB\" })); }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(out.contains("str_len(if b { \"A\" } else { \"BB\" })"), "nested not converted: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn macro_arg_non_family_macro_untouched() {
        // An arbitrary macro's args are not opened (not safely parseable
        // as expressions) — the match stays invisible, like the real
        // oracle's conservative silence on unparseable positions.
        let input = "fn f(b: bool) { my_macro!(match b { true => 1, false => 0 }); }";
        let out = propose_fix("match_bool", input);
        assert!(out.is_none(), "non-family macro arg must not fire: {out:?}");
    }
}
