//! `clippy::unreadable_literal` — `1234567` → `1_234_567`, `123456.789012`
//! → `123_456.789_012` (Batch 150; the only parse-only-PROVABLE pedantic
//! row in the staged head — a literal's digit grouping needs no type
//! knowledge, no scope resolution and no borrow reasoning, so the rewrite
//! is value-preserving by construction).
//!
//! ## Probe matrix (real clippy 0.1.98, `-W clippy::pedantic`)
//!
//! FIRES — ungrouped decimal literals with ≥6 digits: integers
//! (`999999`, `1000000`, `100000u64` — the suffix is preserved,
//! only the digits regroup) and floats (`123456.789`). SILENT — ≤5
//! digits (`100`, `1023`, `1024`, `10000` — the boundary is digit COUNT,
//! not value: `10000` is silent while `999999` fires), any existing
//! underscore (even a bad grouping — that is `large_digit_groups`'
//! territory), and hex/octal/binary notation. Note the `_`-separated
//! suffix form (`1234567_i64`) REFUSES — the `_` guard runs first; the
//! fixture uses the no-underscore suffix form only.
//!
//! Clippy's own suggestion is `MaybeIncorrect`; it does not matter here —
//! regrouping decimal digits cannot change a literal's value, so this
//! proposer is FIXABLE-tier (parse-only provable), not oracle-dependent.
//!
//! Guards (refuse): any `_` (already grouped), hex/octal/binary prefix
//! (`0x`/`0o`/`0b`), float exponent (`1e6`, `1.5E3` — regrouping across
//! the exponent boundary changes nothing but the digits text is not a
//! plain decimal), and non-digit/non-dot characters anywhere. The
//! fraction part is regrouped left-to-right only when it has >3 digits
//! (the `unreadable_literal_lint_fractions` default).
//!
//! ## The `excessive_precision` cross-fire guard (2026-09-21 intake)
//!
//! The separator regroup never changes digits — so on a float literal
//! whose digits EXCEED its width's shortest round-trip the edit is a
//! masking no-op: clippy's `excessive_precision` still fires after it
//! (the live case: `1.0009765625` in an `[f32; N]` test fixture became
//! `1.000_976_562_5`, warning standing). The guard consumes
//! `excessive_precision`'s OWN decision fn — this lint skips any literal
//! that rule would shorten. Width resolution: a suffix decides; a direct
//! `let _: f32/f64` (or `[f32; N]`) annotation decides; a `let x = [lit, …]`
//! array whose first SUFFIXED float element decides (unification); an
//! unconstrained unsuffixed float defaults to f64 (Rust's own inference
//! default — the probed firing fixtures `123456.789` / `3.14159265358979`
//! are f64-shortest and stay firing). Residual: floats constrained to f32
//! through shapes the visitor cannot see (fn-arg position, arithmetic with
//! suffixed operands) fall back to the f64 posture and may still regroup —
//! narrower than the live class, and the anchored oracle lane owns the
//! span when a feed is present.

use super::excessive_precision::shortest_round_trip;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprLit, File, Lit};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        let_width: None,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    /// Float width resolved for the current let-init: `Some(true)` = f32,
    /// `Some(false)` = f64, `None` = unresolved (defaults to the f64
    /// posture at the guard).
    let_width: Option<bool>,
    edits: Vec<SourceEdit>,
}

/// Resolve a type annotation to a float width: `f32`/`f64` directly, or
/// an array/slice OF one (the `let edges: [f32; 12] = …` shape).
fn float_width_of(ty: &syn::Type) -> Option<bool> {
    match ty {
        syn::Type::Path(tp) => match tp.path.segments.last() {
            Some(s) if s.ident == "f32" => Some(true),
            Some(s) if s.ident == "f64" => Some(false),
            _ => None,
        },
        syn::Type::Array(a) => float_width_of(&a.elem),
        syn::Type::Slice(s) => float_width_of(&s.elem),
        syn::Type::Group(g) => float_width_of(&g.elem),
        syn::Type::Paren(p) => float_width_of(&p.elem),
        _ => None,
    }
}

/// Array unification: the first SUFFIXED float element of an array
/// literal fixes the width of every unsuffixed float in it (Rust infers
/// the element type from the suffixed element; mixed suffixes would not
/// compile).
fn array_suffix_width(expr: &Expr) -> Option<bool> {
    let Expr::Array(arr) = expr else {
        return None;
    };
    for e in &arr.elems {
        if let Expr::Lit(ExprLit {
            lit: Lit::Float(fl), ..
        }) = e
        {
            let s = fl.suffix();
            return match s {
                "f32" => Some(true),
                "f64" => Some(false),
                _ => continue,
            };
        }
    }
    None
}

/// Does the `excessive_precision` decision fire for this literal at the
/// resolved width? (The separator fix is a masking no-op when it does.)
fn excessive_precision_owns(digits: &str, suffix: &str, let_width: Option<bool>) -> bool {
    let suffix = suffix.trim_start_matches('_');
    let width = match suffix {
        "f32" => Some(true),
        "f64" => Some(false),
        // Unsuffixed: annotation/array-unification if resolved, else the
        // f64 inference default.
        _ => Some(let_width.unwrap_or(false)),
    };
    let width = width.unwrap_or(false);
    let ep_suffix = if width { "f32" } else { "f64" };
    shortest_round_trip(digits, ep_suffix).is_some()
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_local(&mut self, l: &'ast syn::Local) {
        let saved = self.let_width;
        // A `let` annotation lives on the PATTERN (`let a: f32 = …` parses
        // as `Pat::Type`), not on the Local itself.
        if let syn::Pat::Type(pt) = &l.pat {
            self.let_width = float_width_of(&pt.ty).or(self.let_width);
        }
        if let Some(init) = &l.init {
            if let Some(w) = array_suffix_width(&init.expr) {
                self.let_width = Some(w);
            }
        }
        syn::visit::visit_local(self, l);
        self.let_width = saved;
    }

    fn visit_expr_lit(&mut self, e: &'ast ExprLit) {
        syn::visit::visit_expr_lit(self, e);
        if !matches!(e.lit, Lit::Int(_) | Lit::Float(_)) {
            return;
        }
        let Some(text) = src(self.source, e) else {
            return;
        };
        if matches!(e.lit, Lit::Float(_)) {
            let (digits, suffix) = split_suffix(&text);
            if !digits.is_empty() && excessive_precision_owns(digits, suffix, self.let_width) {
                // The digit-reducing lint owns this literal; the separator
                // regroup would mask its warning instead of fixing it.
                return;
            }
        }
        let Some(regrouped) = regroup(&text) else {
            return;
        };
        if regrouped != text {
            self.edits.push(edit(e, regrouped));
        }
    }
}

/// Split the literal token text into (digits, suffix). The suffix is the
/// trailing alphanumeric run's alphabetic-origin tail (`i64`, `u128`,
/// `f32` — suffixes CONTAIN digits, so a plain alphabetic scan stops too
/// early; the run boundary is the first non-alphanumeric char from the
/// end, and the suffix starts at the run's first alphabetic character).
/// A `_` separator directly before the suffix (`1234567890123_i64`)
/// belongs to the suffix text.
fn split_suffix(text: &str) -> (&str, &str) {
    let bytes = text.as_bytes();
    let mut end = text.len();
    while end > 0 && bytes[end - 1].is_ascii_alphanumeric() {
        end -= 1;
    }
    let run = &text[end..];
    let Some(offset) = run.bytes().position(|b| b.is_ascii_alphabetic()) else {
        return text.split_at(text.len()); // no suffix
    };
    let suf_start = end + offset;
    if suf_start > 0 && bytes[suf_start - 1] == b'_' {
        text.split_at(suf_start - 1)
    } else {
        text.split_at(suf_start)
    }
}

/// Regroup a plain decimal literal. `None` = refuse (not a plain
/// ungrouped decimal, or already grouped / below the digit threshold).
fn regroup(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    let (digits, suffix) = split_suffix(text);
    // Any underscore WITHIN the digits refuses (already grouped — even a
    // bad grouping is `large_digit_groups`' territory). The suffix
    // separator (`1234567890123_i64`) is not a digit underscore.
    if digits.is_empty() || digits.contains('_') {
        return None;
    }
    // Notation guards: radix prefixes and float exponents are out.
    let lower = digits.to_ascii_lowercase();
    if lower.starts_with("0x") || lower.starts_with("0o") || lower.starts_with("0b") {
        return None;
    }
    if digits.contains(['e', 'E']) {
        return None;
    }
    let (int_part, frac_part) = match digits.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (digits, None),
    };
    if !int_part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if let Some(f) = frac_part {
        if !f.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    // Digit-count threshold: ≥6 digits in the integer part, or (floats)
    // >3 fraction digits (the fractions default is linted).
    let frac_long = frac_part.is_some_and(|f| f.len() > 3);
    if int_part.len() < 6 && !frac_long {
        return None;
    }
    let mut out = group_right(int_part);
    if let Some(f) = frac_part {
        out.push('.');
        if f.len() > 3 {
            out.push_str(&group_left(f));
        } else {
            out.push_str(f);
        }
    }
    out.push_str(suffix);
    Some(out)
}

/// `1234567` → `1_234_567` (right-to-left groups of 3).
fn group_right(digits: &str) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let first = digits.len() % 3;
    let mut idx = 0usize;
    while idx < digits.len() {
        if idx > 0 {
            out.push('_');
        }
        let end = if idx == 0 && first > 0 { first } else { 3 };
        out.push_str(&digits[idx..idx + end]);
        idx += end;
    }
    if out.is_empty() {
        out.push_str(digits);
    }
    out
}

/// `789012` → `789_012` (left-to-right groups of 3 — fraction digits).
fn group_left(digits: &str) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let bytes = digits.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('_');
        }
        out.push(*b as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{group_left, group_right, split_suffix};

    fn fix(code: &str) -> Option<String> {
        super::super::propose_fix("unreadable_literal", code)
    }

    #[test]
    fn fires_on_every_probe_firing_shape() {
        // NOTE: fixtures are fn-wrapped — a bare top-level `let` is not a
        // valid Rust FILE and `syn::parse_file` would refuse it before the
        // proposer ever ran.
        for (buggy, fixed) in [
            ("fn f() { let a = 999999; }", "fn f() { let a = 999_999; }"),
            ("fn f() { let a = 1000000; }", "fn f() { let a = 1_000_000; }"),
            ("fn f() { let a = 1234567; }", "fn f() { let a = 1_234_567; }"),
            (
                "fn f() { let a = 1234567890123_i64; }",
                "fn f() { let a = 1_234_567_890_123_i64; }",
            ),
            ("fn f() { let a = 123456.789; }", "fn f() { let a = 123_456.789; }"),
            ("fn f() { let a = 100000u64; }", "fn f() { let a = 100_000u64; }"),
            (
                "fn f() { let a = 123456.789012; }",
                "fn f() { let a = 123_456.789_012; }",
            ),
            (
                "fn f() -> f64 { let a: f64 = 3.14159265358979; a }",
                "fn f() -> f64 { let a: f64 = 3.141_592_653_589_79; a }",
            ),
        ] {
            assert_eq!(fix(buggy).as_deref(), Some(fixed), "case: {buggy}");
        }
    }

    #[test]
    fn silent_on_every_probe_silent_shape() {
        for code in [
            "fn f() { let a = 100; }",
            "fn f() { let a = 1023; }",
            "fn f() { let a = 1024; }",
            "fn f() { let a = 10000; }", // 5 digits — the measured boundary
            "fn f() { let a = 1_000_000; }", // already grouped
            "fn f() { let a = 6186491_8973511; }", // any underscore → other lint's territory
            "fn f() { let a = 0xFFAA; }", // hex
            "fn f() { let a = 0b101010; }", // binary
            "fn f() { let a = 1e6; }", // exponent
            "fn f() { let a = 1.5e3; }", // float exponent
            "fn f() { let a = 12345; }", // 5 digits
            "fn f() { let a = \"123456\"; }", // string, not numeric
        ] {
            assert_eq!(fix(code), None, "must stay silent: {code}");
        }
    }

    #[test]
    fn grouping_arithmetic() {
        assert_eq!(group_right("1234567"), "1_234_567");
        assert_eq!(group_right("123456"), "123_456");
        assert_eq!(group_left("789012"), "789_012");
        assert_eq!(group_left("789"), "789");
    }

    #[test]
    fn suffix_splitting() {
        assert_eq!(split_suffix("1234567890123_i64"), ("1234567890123", "_i64"));
        assert_eq!(split_suffix("100000u64"), ("100000", "u64"));
        assert_eq!(split_suffix("123456.789f64"), ("123456.789", "f64"));
        assert_eq!(split_suffix("999999"), ("999999", ""));
        assert_eq!(split_suffix("123456.789"), ("123456.789", ""));
    }

    // ── the excessive_precision cross-fire guard (2026-09-21 intake) ──

    #[test]
    fn suffixed_excessive_float_skipped() {
        // `2.141592653589793238462643f32` is excessive_precision's literal
        // (shortest `2.1415927f32`); the separator regroup would mask it.
        assert!(fix("fn f() { let a = 2.141592653589793238462643f32; }").is_none());
        assert!(fix("fn f() { let a = 2.141592653589793238462643f64; }").is_none());
    }

    #[test]
    fn suffixed_non_excessive_float_still_fires() {
        // 12 fraction digits but f64-shortest — regrouping is the real fix.
        let out = fix("fn f() { let a = 123456.789012f64; }").expect("fires");
        assert_eq!(out, "fn f() { let a = 123_456.789_012f64; }");
    }

    #[test]
    fn live_f32_array_fixture_skipped() {
        // The 09-21 riir-ai shape: unsuffixed literals unified to f32 by the
        // array's first SUFFIXED element — excessive_precision (upstream,
        // which knows the type) owns them; the regroup masked the warning.
        let code = "fn f() { let edges = [0.0f32, -0.0, 1.0009765625, 5.960464477539063e-08]; }";
        assert!(fix(code).is_none(), "f32-unified array literal must skip");
    }

    #[test]
    fn annotated_f32_let_skipped() {
        let code = "fn f() { let a: f32 = 1.0009765625; }";
        assert!(fix(code).is_none(), "annotated f32 literal must skip");
    }

    #[test]
    fn annotated_f64_let_still_fires() {
        // The probed firing shape: f64-shortest at 15 digits, not excessive —
        // the annotation path must keep it firing.
        let out = fix("fn f() -> f64 { let a: f64 = 3.14159265358979; a }").expect("fires");
        assert!(out.contains("3.141_592_653_589_79"), "{out}");
    }

    #[test]
    fn unconstrained_unsuffixed_defaults_f64_and_fires() {
        // Rust's inference default for an unconstrained float is f64 — the
        // probed fixture stays firing (f64-shortest, not excessive).
        let out = fix("fn f() { let a = 123456.789; }").expect("fires");
        assert!(out.contains("123_456.789"), "{out}");
    }

    #[test]
    fn integers_never_guarded() {
        // The guard is float-only; integer regrouping is untouched.
        let out = fix("fn f() { let a = 1234567; }").expect("fires");
        assert!(out.contains("1_234_567"), "{out}");
    }
}
