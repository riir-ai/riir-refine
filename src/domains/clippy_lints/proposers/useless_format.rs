//! `clippy::useless_format` — `format!("done")` → `"done".to_string()`,
//! `format!("")` → `String::new()` (Issue 016); and the single-positional-
//! arg form `format!("{}", e)` → `e.to_string()` (2026-08-19, the run-#2
//! miss map's `range_loop_plus_format_args` fixture — clippy's own
//! suggestion, probe-verified).
//!
//! Patterns:
//! - EXACTLY one token — a plain string literal (`"`-opening; raw/byte/
//!   c-strings rejected) — and no args. A no-placeholder `format!` is just
//!   an expensive string constructor.
//! - EXACTLY two tokens' args — template `"{}"` (single bare positional
//!   placeholder, no literal text, no spec) + ONE expression arg. The arg
//!   must NOT be a bare ident: for plain identifiers clippy reports
//!   `uninlined_format_args` instead (probe L28) and `uninlined`'s own fix
//!   (`format!("{v}")`) is terminal (clippy does NOT fire `useless_format` on
//!   the inlined form — probe P-K). Firing here for idents would race the
//!   sibling proposer on overlapping spans.
//!
//! The literal-only replacement reuses the ORIGINAL literal token text
//! verbatim (already correctly escaped, caller's spelling preserved). Any
//! brace in the content aborts (escaped `{{` would need re-emission —
//! conservative v1); the empty string maps to `String::new()`.
//!
//! The single-arg replacement is `<arg>.to_string()` — semantically
//! identical to `format!("{}", e)` for every Display type (both go through
//! `Display::fmt`). Honest caveat: when the arg derefs to a cheap-clone
//! type (`&String`, `String`), real clippy fires `implicit_clone` on the
//! output (`.clone()` would avoid a re-render) — the SAME cascade clippy's
//! own `MachineApplicable` fix produces (its suggestion IS `e.to_string()`);
//! recorded as expected created-lint noise in the bench docs.

use super::{edit, src, SourceEdit};
use proc_macro2::TokenTree;
use syn::visit::Visit;
use syn::{Expr, ExprMacro, File, LitStr};

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
    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        syn::visit::visit_expr_macro(self, m);
        if !m.mac.path.is_ident("format") {
            return;
        }
        let toks: Vec<TokenTree> = m.mac.tokens.clone().into_iter().collect();
        if toks.is_empty() {
            return;
        }
        if toks.len() == 1 {
            self.literal_only(m, &toks[0]);
            return;
        }
        self.single_positional_arg(m, &toks);
    }
}

impl Collector<'_> {
    /// `format!("lit")` — the Issue 016 shape.
    fn literal_only(&mut self, m: &ExprMacro, tok: &TokenTree) {
        let text = match tok {
            TokenTree::Literal(l) if l.to_string().starts_with('"') => l.to_string(),
            _ => return, // raw/byte/c-string or non-literal
        };
        // Validate the token is a well-formed plain string literal, then
        // read its unescaped value for the content checks.
        let Ok(lit) = syn::parse_str::<LitStr>(&text) else {
            return;
        };
        let value = lit.value();
        let replacement = if value.is_empty() {
            "String::new()".to_string()
        } else if value.contains(['{', '}']) {
            return; // escaped braces would need re-emission — skip (v1)
        } else {
            // The original token text IS the escaped literal — reuse it.
            format!("{text}.to_string()")
        };
        self.edits.push(edit(m, replacement));
    }

    /// `format!("{}", e)` where `e` is not a bare ident/path — clippy's own
    /// fix is `e.to_string()`. The arg's parsed expr carries file-relative
    /// spans (the macro-token round-trip preserves them), so `src` slices
    /// the ORIGINAL source for the replacement text.
    fn single_positional_arg(&mut self, m: &ExprMacro, toks: &[TokenTree]) {
        // Template is always toks[0] for `format!` — the single bare `{}`
        // placeholder, nothing else in the string.
        if toks[0].to_string() != "\"{}\"" {
            return;
        }
        // Exactly one following arg: `, <expr>` with an optional trailing
        // comma. Never split inside the expr — commas there live in Groups.
        if !matches!(&toks.get(1), Some(TokenTree::Punct(p)) if p.as_char() == ',') {
            return;
        }
        let mut expr_toks = &toks[2..];
        if matches!(expr_toks.last(), Some(TokenTree::Punct(p)) if p.as_char() == ',') {
            expr_toks = &expr_toks[..expr_toks.len() - 1];
        }
        let Ok(expr) = syn::parse2::<Expr>(expr_toks.iter().cloned().collect()) else {
            return; // named args (`k = v`) / malformed — conservative skip
        };
        // Bare idents are `uninlined_format_args` territory (clippy's own
        // precedence — probe L28); skipping avoids a same-span race with
        // the sibling proposer, and uninlined's fix is terminal (clippy does
        // NOT fire useless_format on the inlined form — probe P-K).
        if matches!(&expr, Expr::Path(p) if p.path.get_ident().is_some()) {
            return;
        }
        let Some(text) = src(self.source, &expr) else {
            return;
        };
        self.edits.push(edit(m, format!("{text}.to_string()")));
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn plain_string_becomes_to_string() {
        let out =
            propose_fix("useless_format", "fn f() -> String { format!(\"done\") }")
                .expect("pattern matched");
        assert_eq!(out, "fn f() -> String { \"done\".to_string() }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn empty_string_becomes_string_new() {
        let out = propose_fix("useless_format", "fn f() -> String { format!(\"\") }")
            .expect("pattern matched");
        assert_eq!(out, "fn f() -> String { String::new() }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn escapes_keep_caller_spelling() {
        let out = propose_fix(
            "useless_format",
            "fn f() -> String { format!(\"a\\tb done\") }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f() -> String { \"a\\tb done\".to_string() }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn single_positional_arg_complex_becomes_to_string() {
        // The run-#2 bench fixture shape: indexing expr arg — clippy's own
        // suggestion is `names[i].to_string()` (probe-verified).
        let input = r#"fn f(names: &[String]) -> String { format!("{}", names[0]) }"#;
        let out = propose_fix("useless_format", input).expect("pattern matched");
        assert!(out.contains("names[0].to_string()"), "{out}");
        assert!(!out.contains("format!"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn single_positional_arg_method_call_becomes_to_string() {
        let input = "struct S { n: u32 }\nfn f(s: &S) -> String { format!(\"{}\", s.n) }";
        let out = propose_fix("useless_format", input).expect("pattern matched");
        assert!(out.contains("s.n.to_string()"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn bare_ident_arg_left_to_uninlined() {
        // For plain identifiers clippy reports uninlined_format_args and its
        // fix (`format!("{v}")`) is terminal — skip to avoid racing the
        // sibling proposer on the same span.
        let input = r#"fn f(v: u32) -> String { format!("{}", v) }"#;
        let out = propose_fix("useless_format", input);
        assert!(out.is_none(), "ident arg is uninlined territory: {out:?}");
    }

    #[test]
    fn multi_placeholder_unchanged() {
        let out = propose_fix(
            "useless_format",
            r#"fn f(a: u8, b: u8) -> String { format!("{} {}", a, b) }"#,
        );
        assert!(out.is_none(), "two placeholders are a real format: {out:?}");
    }

    #[test]
    fn spec_or_literal_text_unchanged() {
        let out = propose_fix(
            "useless_format",
            r#"fn f(v: u32) -> String { format!("n={}", v * 2) }"#,
        );
        assert!(out.is_none(), "literal text keeps the format meaningful: {out:?}");
        let out2 = propose_fix(
            "useless_format",
            r#"fn f(v: u32) -> String { format!("{:>4}", v) }"#,
        );
        assert!(out2.is_none(), "spec'd placeholder is a real format: {out2:?}");
    }

    #[test]
    fn with_args_unchanged() {
        let out = propose_fix(
            "useless_format",
            r#"fn f(x: u8) -> String { format!("x {}", x) }"#,
        );
        assert!(out.is_none(), "literal-text format is not useless: {out:?}");
    }

    #[test]
    fn braces_unchanged() {
        let out = propose_fix("useless_format", "fn f() -> String { format!(\"{{}}\") }");
        assert!(out.is_none(), "brace content aborts (v1): {out:?}");
    }
}
