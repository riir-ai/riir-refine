//! `clippy::unnecessary_min_or_max` — `min`/`max` between two INTEGER
//! literals folds to the winning literal: `100u32.min(97)` → `97`,
//! `97u32.min(100)` → `97u32`, `5u32.max(9)` → `9` (the riir-infer
//! specimen, P116: `100.min(97)` → `97`).
//!
//! Probe matrix (clippy 1.99.0, `/tmp/probe_cm`, plan 353):
//! - FIRES: receiver AND arg both int literals — decimal, hex/oct/bin
//!   (`0x64u32.min(0x61)` → `0x61`, the winner's own text), negatives
//!   (`(-5i32).min(-9)` → `-9`), paren receivers (`(100u32).min(97)` →
//!   `97`), TIES (`97u32.min(97)` → `97` — the ARG, clippy's own tie rule).
//! - SILENT: variable receivers, float literals (the NaN ordering class —
//!   mirrored), const paths (`100u32.min(HI)`).
//! - the message reads "`<recv>` is never smaller/greater than `<arg>`";
//!   the suggestion replaces the WHOLE call span with the winning literal's
//!   own text (suffix/prefix/parens ride verbatim from its span).
//!
//! Fix construction (plan 353): one edit, whole method-call span, the
//! winner's original span text. Value comparison parses the literal as
//! i128 after stripping suffix + underscores; a parse failure or i128
//! overflow refuses (u128 literals beyond i128 range are the honest
//! divergence — i128 covers every realistic comparison; clippy typeck
//! handles the rest via its own constant folding). Ties adopt the ARG
//! (the probe-pinned tie rule). Floats refuse upstream-side (NaN); the
//! `min`/`max` chain shape is `manual_clamp`'s territory, not here.

use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, Lit, UnOp};

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
    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) = try_min_or_max(self.source, m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

/// An int literal's value: decimal/hex/octal/binary, underscores stripped,
/// optional int suffix stripped, unary-minus wrapper folded. `None` on
/// floats/chars/anything unparseable (refuse = the safe direction).
fn int_literal_value(expr: &Expr) -> Option<i128> {
    // Parenthesized forms (`(-5i32)`, `(100u32)`) unwrap — clippy fires
    // through them (probe-pinned); nested parens recurse.
    if let Expr::Paren(p) = expr {
        return int_literal_value(p.expr.as_ref());
    }
    let negated = matches!(expr, Expr::Unary(u) if matches!(u.op, UnOp::Neg(_)));
    let lit_expr: &Expr = if negated {
        match expr {
            Expr::Unary(u) => u.expr.as_ref(),
            _ => unreachable!("negated guard"),
        }
    } else {
        expr
    };
    let Expr::Lit(lit) = lit_expr else {
        return None;
    };
    let Lit::Int(int_lit) = &lit.lit else {
        return None;
    };
    let digits = int_lit
        .token()
        .to_string()
        .replace('_', "")
        .to_ascii_lowercase();
    let digits = digits
        .split_once(|c: char| !c.is_ascii_digit() && c != 'x' && c != 'o' && c != 'b')
        .map_or(digits.as_str(), |(head, _)| head);
    let value: i128 = if let Some(rest) = digits.strip_prefix("0x") {
        i128::from_str_radix(rest, 16).ok()?
    } else if let Some(rest) = digits.strip_prefix("0o") {
        i128::from_str_radix(rest, 8).ok()?
    } else if let Some(rest) = digits.strip_prefix("0b") {
        i128::from_str_radix(rest, 2).ok()?
    } else {
        digits.parse::<i128>().ok()?
    };
    let sign: i128 = if negated { -1 } else { 1 };
    sign.checked_mul(value)
}

fn try_min_or_max(source: &str, m: &ExprMethodCall) -> Option<String> {
    if m.method != "min" && m.method != "max" {
        return None;
    }
    if !m.attrs.is_empty() || m.turbofish.is_some() {
        return None;
    }
    if m.args.len() != 1 {
        return None;
    }
    // Both operands must be int literals (floats/idents/const paths are
    // upstream-silent or unprovable — mirrored).
    let recv_val = int_literal_value(m.receiver.as_ref())?;
    let arg_val = int_literal_value(&m.args[0])?;
    // The winner: min → the smaller (ties → the ARG, the probe-pinned rule);
    // max → the larger (ties → the ARG likewise — clippy suggested `97` for
    // `97.min(97)`, i.e. the arg text).
    let keep_arg = match m.method.to_string().as_str() {
        "min" => arg_val <= recv_val,
        "max" => arg_val >= recv_val,
        _ => return None,
    };
    // Float-typed literals must refuse — an `i128` parse accepts `1.0`'s
    // digits? No: `Lit::Float` is not `Lit::Int`, so floats never reach here.
    // BUT a float SUFFIX on an int literal (`5u32` vs `5f32`) is impossible —
    // `Lit::Int` guarantees the int family. The float refusal is structural.
    let winner_text = if keep_arg {
        src(source, &m.args[0])?
    } else {
        src(source, m.receiver.as_ref())?
    };
    Some(winner_text)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn receiver_larger_min_folds_to_arg() {
        // The P116 specimen.
        let out = propose_fix("unnecessary_min_or_max", "fn f() -> u32 { 100u32.min(97) }")
            .expect("matched");
        assert!(out.contains("{ 97 }"), "arg not kept: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn receiver_smaller_min_keeps_receiver() {
        let out = propose_fix("unnecessary_min_or_max", "fn f() -> u32 { 97u32.min(100) }")
            .expect("matched");
        assert!(out.contains("{ 97u32 }"), "receiver suffix lost: {out}");
    }

    #[test]
    fn max_smaller_receiver_folds_to_arg() {
        let out = propose_fix("unnecessary_min_or_max", "fn f() -> u32 { 5u32.max(9) }")
            .expect("matched");
        assert!(out.contains("{ 9 }"), "arg not kept: {out}");
    }

    #[test]
    fn max_larger_receiver_keeps_receiver() {
        let out = propose_fix("unnecessary_min_or_max", "fn f() -> u32 { 9u32.max(5) }")
            .expect("matched");
        assert!(out.contains("{ 9u32 }"), "receiver suffix lost: {out}");
    }

    #[test]
    fn tie_keeps_the_arg() {
        // The probe-pinned tie rule: `97u32.min(97)` → `97` (the arg).
        let out = propose_fix("unnecessary_min_or_max", "fn f() -> u32 { 97u32.min(97) }")
            .expect("matched");
        assert!(out.contains("{ 97 }"), "tie must keep the arg text: {out}");
    }

    #[test]
    fn negatives_fired() {
        let out = propose_fix(
            "unnecessary_min_or_max",
            "fn f() -> i32 { (-5i32).min(-9) }",
        )
        .expect("matched");
        assert!(out.contains("{ -9 }"), "negative arg not kept: {out}");
    }

    #[test]
    fn hex_winner_text_rides_verbatim() {
        let out = propose_fix(
            "unnecessary_min_or_max",
            "fn f() -> u32 { 0x64u32.min(0x61) }",
        )
        .expect("matched");
        assert!(out.contains("{ 0x61 }"), "hex text not preserved: {out}");
    }

    #[test]
    fn paren_receiver_winner_rewrites_whole_call() {
        let out = propose_fix(
            "unnecessary_min_or_max",
            "fn f() -> u32 { (100u32).min(97) }",
        )
        .expect("matched");
        assert!(
            out.contains("{ 97 }"),
            "paren receiver call not folded: {out}"
        );
    }

    #[test]
    fn variable_refused() {
        let out = propose_fix(
            "unnecessary_min_or_max",
            "fn f(a: u32) -> u32 { a.min(97) }",
        );
        assert!(out.is_none(), "variable receiver fired: {out:?}");
    }

    #[test]
    fn float_literals_refused() {
        let out = propose_fix(
            "unnecessary_min_or_max",
            "fn f() -> f32 { 1.0f32.min(2.0) }",
        );
        assert!(out.is_none(), "float fired: {out:?}");
    }

    #[test]
    fn const_path_arg_refused() {
        let input = "const HI: u32 = 97;\nfn f() -> u32 { 100u32.min(HI) }";
        let out = propose_fix("unnecessary_min_or_max", input);
        assert!(out.is_none(), "const path fired: {out:?}");
    }

    #[test]
    fn clamp_chain_not_this_lint() {
        // `x.max(3).min(5)` is manual_clamp's shape — the receiver is not a
        // literal so this proposer refuses (no double-claim).
        let out = propose_fix(
            "unnecessary_min_or_max",
            "fn f(x: u32) -> u32 { x.max(3).min(5) }",
        );
        assert!(out.is_none(), "chain fired: {out:?}");
    }
}
