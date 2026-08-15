//! `clippy::useless_format` — `format!("done")` → `"done".to_string()`,
//! `format!("")` → `String::new()` (Issue 016).
//!
//! Pattern: a `format!` invocation with EXACTLY one token — a plain
//! string literal (`"`-opening; raw/byte/c-strings rejected) — and no
//! args. A no-placeholder `format!` is just an expensive string
//! constructor.
//!
//! The replacement reuses the ORIGINAL literal token text verbatim
//! (already correctly escaped, caller's spelling preserved). Any brace in
//! the content aborts (escaped `{{` would need re-emission — conservative
//! v1); the empty string maps to `String::new()`.

use super::{edit, SourceEdit};
use proc_macro2::TokenTree;
use syn::visit::Visit;
use syn::{ExprMacro, File, LitStr};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(_source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector {
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        syn::visit::visit_expr_macro(self, m);
        if !m.mac.path.is_ident("format") {
            return;
        }
        let toks: Vec<TokenTree> = m.mac.tokens.clone().into_iter().collect();
        if toks.len() != 1 {
            return; // has args (or is empty `format!()`) — not our shape
        }
        let text = match &toks[0] {
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
    fn with_args_unchanged() {
        let out = propose_fix("useless_format", "fn f(x: u8) -> String { format!(\"{}\", x) }");
        assert!(out.is_none(), "args present — uninlined_format_args' shape: {out:?}");
    }

    #[test]
    fn braces_unchanged() {
        let out = propose_fix("useless_format", "fn f() -> String { format!(\"{{}}\") }");
        assert!(out.is_none(), "brace content aborts (v1): {out:?}");
    }
}
