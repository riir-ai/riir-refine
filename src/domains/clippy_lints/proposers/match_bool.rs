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
//! own). Only this exact polarity layout is converted.
//!
//! P6 extensions (oracle-probed vs clippy 1.97, ALL MachineApplicable):
//! - wildcard-second two-arm: `[P => A, _ => B]` (either arm order, either
//!   polarity) → `if s { A } else { B }` / `if !s { A } else { B }`
//! - guard-on-the-literal-arm: `[P if g => A, _ => B]` → the FLAT
//!   conjunction `if s && g { A } else { B }` / `if !s && g { .. }` —
//!   clippy emits the flat form, NOT a nested if (the old "would need a
//!   nested if" note was wrong; short-circuit `&&` preserves the guard's
//!   evaluate-only-when-P semantics). A bool-LITERAL second arm with a
//!   guarded first arm is E0004 (guards don't count toward
//!   exhaustiveness) — impossible in compiling code, not handled.
//!
//! Span-preserving (Issue 016): the whole match's byte range is replaced by
//! `if <scrutinee> <true-body> else <false-body>` — scrutinee and bodies
//! keep their ORIGINAL source text; non-block bodies are braced. The if
//! condition takes any expression, so the scrutinee needs no parens.

use super::helpers::is_postfix_safe;
use super::uninlined_format_args::all_expr_args;
use super::{SourceEdit, edit, src};
use proc_macro2::TokenStream;
use syn::visit::Visit;
use syn::{Expr, ExprMacro, ExprMatch, File, Lit, Macro, Pat, StmtMacro};

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
    /// no guards), extended (P6, oracle MA) to the wildcard-second form
    /// `[P => A, _ => B]` and the guarded-literal form `[P if g => A,
    /// _ => B]` (flat `&&` conjunction).
    fn try_two_arm(&self, m: &ExprMatch) -> Option<String> {
        if m.arms.iter().any(|a| !a.attrs.is_empty()) {
            return None; // arm-attrs would be lost by the splice
        }
        let scrutinee = src(self.source, &*m.expr)?;
        let scrut_postfix = is_postfix_safe(&m.expr);
        // `!s && g` / `if !s` — unary `!` binds tighter than `&&`, so a
        // postfix scrutinee takes no parens; binary/macro forms need them
        // (`!a == b` parses as `(!a) == b` — a silent wrong-condition).
        let neg_cond = if scrut_postfix {
            format!("!{scrutinee}")
        } else {
            format!("!({scrutinee})")
        };
        // Guarded-literal form: the literal arm carries the guard, the
        // other arm is the wildcard (clippy MA: flat `s && g`).
        let (lit, other) = (&m.arms[0], &m.arms[1]);
        let (lit, other) = if other.guard.is_some() {
            (other, lit)
        } else {
            (lit, other)
        };
        if let Some(guard) = &lit.guard {
            let negated = match (&lit.pat, &other.pat) {
                (p, Pat::Wild(_)) if pat_is_bool(p, true) => false,
                (p, Pat::Wild(_)) if pat_is_bool(p, false) => true,
                _ => return None,
            };
            let guard_text = src(self.source, &*guard.1)?;
            let lit_body = braced_body(self.source, &lit.body)?;
            let other_body = braced_body(self.source, &other.body)?;
            let cond = if negated {
                format!("if {neg_cond} && {guard_text}")
            } else {
                format!("if {scrutinee} && {guard_text}")
            };
            return Some(format!("{cond} {lit_body} else {other_body}"));
        }
        for (t, f) in [(&m.arms[0], &m.arms[1]), (&m.arms[1], &m.arms[0])] {
            let (t_lit, t_neg) = if pat_is_bool(&t.pat, true) {
                (true, false)
            } else if pat_is_bool(&t.pat, false) {
                (false, true)
            } else {
                continue;
            };
            // classic: both arms bool literals; P6: wildcard second arm
            let other_matches = pat_is_bool(&f.pat, !t_lit) || matches!(f.pat, Pat::Wild(_));
            if !other_matches {
                continue;
            }
            let t_body = braced_body(self.source, &t.body)?;
            let f_body = braced_body(self.source, &f.body)?;
            let cond = if t_neg {
                format!("if {neg_cond}")
            } else {
                format!("if {scrutinee}")
            };
            return Some(format!("{cond} {t_body} else {f_body}"));
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
        assert!(
            out.contains("if b { 1 } else { 0 }"),
            "not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn false_first_also_rewrites() {
        let input = "fn f(b: bool) -> u8 { match b { false => 0, true => 1 } }";
        let out = propose_fix("match_bool", input).expect("matched");
        // P6 oracle re-probe: clippy is ARM-ORDER-PRESERVING — false-first
        // emits `if !b { 0 } else { 1 }`, not the normalized true-first form
        // (probe vs clippy 1.97: sugg = `if !flag { 0 } else { 1 }`, MA).
        assert!(
            out.contains("if !b { 0 } else { 1 }"),
            "not converted: {out}"
        );
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
        let input =
            "fn f(b: bool, g: bool) { match b { true => x(), false if g => y(), false => {} } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("if b { x() } else if g { y() }"),
            "guarded fallback not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn guarded_fallback_false_first_rewrites() {
        let input =
            "fn f(b: bool, g: bool) { match b { false => x(), true if g => y(), true => {} } }";
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
        // 3-arm guarded-first is unprobed territory (the P6 oracle covered
        // the 2-arm form only); the guarded-FALLBACK shape requires the
        // guard on the MIDDLE arm.
        assert!(out.is_none(), "3-arm first-guard must not fire: {out:?}");
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
        // oracle fact (P6): literal + guard + BOOL-LITERAL second arm is
        // E0004 (guards don't count toward exhaustiveness) — impossible
        // code, stays silent either way.
        assert!(out.is_none(), "guarded arm must not fire: {out:?}");
    }

    #[test]
    fn wildcard_second_true_first_rewrites() {
        // P6 oracle (clippy 1.97, MA): `match flag { true => A, _ => B }`
        // fires match_bool with `if flag { A } else { B }`.
        let input = "fn f(b: bool) -> u8 { match b { true => 1, _ => 0 } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(out.contains("if b { 1 } else { 0 }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn wildcard_second_false_first_rewrites_negated() {
        // P6 oracle: `false => A, _ => B` → `if !flag { A } else { B }` —
        // postfix scrutinee takes NO parens (byte-parity with clippy).
        let input = "fn f(b: bool) -> u8 { match b { false => 0, _ => 1 } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(out.contains("if !b { 0 } else { 1 }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn wildcard_first_rewrites() {
        // P6 oracle: wild-first order also fires; the if normalizes to the
        // literal arm as the then-branch.
        let input = "fn f(b: bool) { match b { _ => stop(), true => walk() } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(out.contains("if b { walk() } else { stop() }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn guarded_literal_flat_conjunction() {
        // P6 oracle: `true if g => A, _ => B` → `if b && g { A } else { B }`
        // — the FLAT conjunction, not a nested if; short-circuit `&&`
        // preserves the guard's evaluate-only-when-P semantics.
        let input = "fn f(b: bool, g: bool) { match b { true if g => stop(), _ => walk() } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("if b && g { stop() } else { walk() }"),
            "{out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn guarded_literal_false_polarity_negated() {
        // P6 oracle: `false if g => A, _ => B` → `if !flag && g { .. }`.
        let input = "fn f(b: bool, g: bool) { match b { false if g => stop(), _ => walk() } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("if !b && g { stop() } else { walk() }"),
            "{out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn guarded_literal_non_postfix_scrutinee_gets_parens() {
        // `!a == b && g` parses as `(!a) == b` — a binary scrutinee must be
        // parenthesized in the negated form.
        let input = "fn f(a: bool, b: bool, g: bool) { match a == b { false if g => stop(), _ => walk() } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("if !(a == b) && g { stop() } else { walk() }"),
            "{out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn guarded_literal_wild_first_order_also_fires() {
        // P6 oracle: m2 probe — wild-first with guarded literal second also
        // fires; normalized to `if b && g { .. }`.
        let input = "fn f(b: bool, g: bool) { match b { _ => stop(), true if g => walk() } }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("if b && g { walk() } else { stop() }"),
            "{out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn macro_arg_post_template_rewrites() {
        // The ndb `ed_freeze_gate_poc` production shape — real clippy 1.97
        // fires match_bool on the match inside the println! arg (probed).
        let input = "fn f(b: bool, p: f64) { println!(\"sign={} p={p:.3}\", match b { true => \"OK\", false => \"NO\" }); }";
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
        let input = "fn f(w: &mut String, c: bool) { writeln!(w, \"{}\", match c { true => 1, false => 0 }).unwrap(); }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("if c { 1 } else { 0 }"),
            "stmt-macro arg not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn macro_arg_assert_operand_rewrites() {
        // Probe-verified: real clippy fires match_bool on the assert!
        // OPERAND position too (pre-template arg).
        let input =
            "fn f(b: bool, a: bool, c: bool) { assert!(match b { true => a, false => c }); }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("assert!(if b { a } else { c })"),
            "operand not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn macro_arg_nested_in_call_rewrites() {
        // A match inside a CALL inside a macro arg — the recursive
        // visit_expr descent reaches it at depth 2.
        let input = "fn f(b: bool) { println!(\"{}\", str_len(match b { true => \"A\", false => \"BB\" })); }";
        let out = propose_fix("match_bool", input).expect("matched");
        assert!(
            out.contains("str_len(if b { \"A\" } else { \"BB\" })"),
            "nested not converted: {out}"
        );
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
