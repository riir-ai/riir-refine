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

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{ExprLit, File, Lit};

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
    fn visit_expr_lit(&mut self, e: &'ast ExprLit) {
        syn::visit::visit_expr_lit(self, e);
        if !matches!(e.lit, Lit::Int(_) | Lit::Float(_)) {
            return;
        }
        let Some(text) = src(self.source, e) else {
            return;
        };
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
    use super::*;

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
}
