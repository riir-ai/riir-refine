//! `clippy::single_char_pattern` — `.contains("x")` → `.contains('x')`.
//!
//! VERIFY-tier: method resolution is type-blind — a custom type could have
//! `contains(&str)` without `contains(char)`; rustc backs the revert under
//! `--verify`.
//!
//! Only PLAIN one-char ASCII string literals fire: any backslash in the
//! literal's source (escapes, `\u{..}`) is skipped, as are multi-char,
//! empty, non-ASCII, `"` and `'` payloads (the char-literal spelling of the
//! latter two would need escapes). Span-preserving (T6.4): ONLY the argument
//! literal's byte range is replaced by `'<char>'` — the receiver, method,
//! and any turbofish stay as untouched bytes.

use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, Lit};

/// Pattern-taking methods with exactly one pattern argument.
const PATTERN_METHODS_1ARG: &[&str] = &[
    "contains",
    "starts_with",
    "ends_with",
    "find",
    "rfind",
    "split",
    "rsplit",
    "split_terminator",
    "rsplit_terminator",
    "matches",
    "rmatches",
    "trim_start_matches",
    "trim_end_matches",
];

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
        if let Some(replacement) = self.try_char_literal(m) {
            self.edits.push(edit(&m.args[0], replacement));
        }
    }
}

impl Collector<'_> {
    fn try_char_literal(&self, m: &ExprMethodCall) -> Option<String> {
        let method = m.method.to_string();
        if !PATTERN_METHODS_1ARG.contains(&method.as_str()) || m.args.len() != 1 {
            return None;
        }
        let Expr::Lit(lit) = &m.args[0] else {
            return None;
        };
        if !lit.attrs.is_empty() {
            return None; // attrs would be dropped by the splice
        }
        let Lit::Str(s) = &lit.lit else {
            return None; // byte strings / other literals stay untouched
        };
        let raw = src(self.source, &m.args[0])?;
        if raw.contains('\\') {
            return None; // escaped payload — char form would differ
        }
        let value = s.value();
        let mut chars = value.chars();
        let c = chars.next()?;
        if chars.next().is_some() {
            return None; // multi-char pattern
        }
        if !c.is_ascii() || c == '"' || c == '\'' {
            return None; // char-literal spelling would need escapes
        }
        Some(format!("'{c}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_single_char_string_to_char() {
        let input = "fn f(s: &str) -> bool { s.contains(\"x\") }";
        let out = propose_fix("single_char_pattern", input).expect("matched");
        assert!(
            out.contains("s.contains('x')"),
            "char literal not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn fires_on_split() {
        let input = "fn g(s: &str) { for _ in s.split(\",\") {} }";
        let out = propose_fix("single_char_pattern", input).expect("matched");
        assert!(out.contains("s.split(',')"), "split not converted: {out}");
    }

    #[test]
    fn multi_char_unchanged() {
        let input = "fn f(s: &str) -> bool { s.contains(\"xy\") }";
        let out = propose_fix("single_char_pattern", input);
        assert!(out.is_none(), "multi-char pattern must not fire: {out:?}");
    }

    #[test]
    fn empty_unchanged() {
        let input = "fn f(s: &str) -> bool { s.contains(\"\") }";
        let out = propose_fix("single_char_pattern", input);
        assert!(out.is_none(), "empty pattern must not fire: {out:?}");
    }

    #[test]
    fn escaped_unchanged() {
        let input = "fn f(s: &str) -> bool { s.contains(\"\\n\") }";
        let out = propose_fix("single_char_pattern", input);
        assert!(out.is_none(), "escaped pattern must not fire: {out:?}");
    }
}
