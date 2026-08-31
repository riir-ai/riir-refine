//! `clippy::useless_format` — `format!("done")` → `"done".to_string()`,
//! `format!("")` → `String::new()` (Issue 016); and the single-positional-
//! arg form `format!("{}", x.to_string())` → `x.to_string()` (2026-08-19,
//! the run-#2 miss map's `range_loop_plus_format_args` fixture; NARROWED
//! 2026-08-31 per the P15 clippy-1.97 oracle alignment, see below).
//!
//! Patterns:
//! - EXACTLY one token — a plain string literal (`"`-opening; raw/byte/
//!   c-strings rejected) — and no args. A no-placeholder `format!` is just
//!   an expensive string constructor. The oracle fires unconditionally
//!   here (its `([], [_])` branch has no type gate) — always safe.
//! - EXACTLY two tokens' args — template `"{}"` + ONE expression arg that
//!   is a `.to_string()` METHOD CALL. The arg is emitted VERBATIM (the
//!   whole `format!` call is replaced by the arg text itself): the oracle's
//!   `is_new_string` branch recognizes a `to_string` method call as already
//!   producing a fresh `String` and suggests the bare snippet (probe round
//!   4: `format!("{}", v.to_string())` → `v.to_string()`, MA). Append-safe
//!   by construction — a method call needs no parens, and `ToString::
//!   to_string` returns `String` regardless of the receiver's type.
//!
//! ### The P15 narrowing (2026-08-31) — why every other arg shape refuses
//!
//! Clippy 1.97's single-arg branch is TYPE-GATED: it fires only when the
//! arg's PEELED type is `str` or `String` (its own source: `peel_refs()`
//! match on `ty::Str` / the `String` lang item). That gate is invisible to
//! syn. The pre-narrowing proposer fired on ALL non-ident args, which (a)
//! outran the oracle on its silent shapes and (b) produced the P15 live
//! break: `Some(format!("{}", v as i64))` → `Some(v as i64.to_string())`
//! — `as` binds looser than `.`, so the emitted text was syntax garbage
//! (riir-train `loss_uopsd/extractor.rs:329`, reverted by the verify gate).
//! Probe rounds 1–3 vs clippy 1.97 pinned the silent set: cast (`v as i64`),
//! binary (`v * 2`), unary (`!v`), paren (`(v as i64)`), and int-typed
//! method calls (`s.len()`) are ALL silent; only `str`/`String`-typed args
//! (`names[0]`, `s.trim()`, `a + b`, `if c { a } else { b }`) fire. A
//! parse-only proposer cannot evaluate the type gate, so firing those
//! shapes would phantom-fix oracle-silent code (and the pre-P11 emitter
//! form broke compiles). The one shape whose oracle verdict is provable at
//! syntax level is the `to_string()` method call — it fires for EVERY
//! receiver type. Everything else refuses. The old `x.to_string().
//! to_string()` double-call emission is gone with it (the oracle emits the
//! arg bare).
//!
//! Bare-ident args stay skipped: clippy reports `uninlined_format_args`
//! there (probe L28) and `uninlined`'s own fix (`format!("{v}")`) is
//! terminal (clippy does NOT fire `useless_format` on the inlined form —
//! probe P-K); firing here would race the sibling proposer on overlapping
//! spans.
//!
//! The literal-only replacement reuses the ORIGINAL literal token text
//! verbatim (already correctly escaped, caller's spelling preserved). Any
//! brace in the content aborts (escaped `{{` would need re-emission —
//! conservative v1); the empty string maps to `String::new()`.
//!
//! Honest caveat (the `to_string`-arg shape): replacing
//! `format!("{}", x.to_string())` with `x.to_string()` is semantically
//! identical; when real clippy would fire `implicit_clone` on the result
//! (`x: String`) its own fix carries the same cascade — recorded as
//! expected created-lint noise in the bench docs.

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

    /// `format!("{}", x.to_string())` → `x.to_string()` — the ONLY
    /// single-arg shape whose oracle verdict is provable at syntax level.
    /// The template is always toks[0] for `format!` — the single bare `{}`
    /// placeholder, nothing else in the string.
    ///
    /// P15: every other non-ident arg shape REFUSES. Clippy 1.97 type-gates
    /// this branch (peeled type ∈ {str, String}) — `s.trim()` fires,
    /// `s.len()`/`v * 2`/`v as i64`/`!v` are silent — and the gate is
    /// invisible to syn. Emitting into a silent shape is a phantom fix at
    /// best and was, unparenthesized, the P15 compile break.
    fn single_positional_arg(&mut self, m: &ExprMacro, toks: &[TokenTree]) {
        // Template is always toks[0] for `format!` — the single bare `{}
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
        // The P15 narrowing: fire only on a `.to_string()` method call —
        // `ToString::to_string` is String-typed for every receiver, so the
        // oracle's type gate always passes; its `is_new_string` branch then
        // emits the arg VERBATIM (no `.to_string()` appended — the arg
        // already produces a fresh String). Turbofish on `to_string` itself
        // is a weird inherent-method shape — refuse (safe direction).
        let Expr::MethodCall(mc) = &expr else {
            return; // type gate invisible for index/field/call/binary/cast/…
        };
        if mc.method != "to_string" || mc.turbofish.is_some() {
            return;
        }
        let Some(text) = src(self.source, &expr) else {
            return;
        };
        self.edits.push(edit(m, text));
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
    fn to_string_arg_emitted_verbatim_p15() {
        // The P15 firing shape: `.to_string()` method-call args are
        // String-typed by construction (ToString), so the oracle's type gate
        // always passes and its is_new_string branch emits the arg VERBATIM
        // (probe round 4: `v.to_string()`, MA). The pre-P15 emitter produced
        // the double-call `v.to_string().to_string()` here.
        let input = r#"fn f(v: u32) -> String { format!("{}", v.to_string()) }"#;
        let out = propose_fix("useless_format", input).expect("pattern matched");
        assert_eq!(out, "fn f(v: u32) -> String { v.to_string() }");
        assert!(!out.contains(".to_string().to_string"), "{out}");
        syn::parse_file(&out).expect("re-parses");

        let input2 = r#"fn f(s: &str) -> String { format!("{}", s.trim().to_string()) }"#;
        let out2 = propose_fix("useless_format", input2).expect("pattern matched");
        assert_eq!(out2, "fn f(s: &str) -> String { s.trim().to_string() }");
        syn::parse_file(&out2).expect("re-parses");
    }

    #[test]
    fn cast_arg_refused_p15() {
        // THE P15 live break (riir-train loss_uopsd/extractor.rs:329): the
        // old emitter produced `v as i64.to_string()` — syntax garbage (`as`
        // binds looser than `.`). Real clippy 1.97 is SILENT on this shape
        // (peeled type i64 fails its str/String gate) — probe round 1.
        let input = r#"fn f(v: u64) -> String { format!("{}", v as i64) }"#;
        let out = propose_fix("useless_format", input);
        assert!(out.is_none(), "cast arg is oracle-silent: {out:?}");
    }

    #[test]
    fn binary_unary_paren_args_refused_p15() {
        // Probe rounds 1/3: binary (u32), unary (bool), and parenthesized
        // (i64) args are all oracle-silent (type gate) — refuse. Binary
        // String concat (`a + b`) DOES fire upstream with a bare-snippet
        // fix, but the operand types are invisible to syn, so the whole
        // binary subfamily refuses.
        for (name, input) in [
            ("binary", r#"fn f(v: u32) -> String { format!("{}", v * 2) }"#),
            ("unary", r#"fn f(v: bool) -> String { format!("{}", !v) }"#),
            ("paren", r#"fn f(v: u64) -> String { format!("{}", (v as i64)) }"#),
        ] {
            let out = propose_fix("useless_format", input);
            assert!(out.is_none(), "{name} arg is oracle-silent: {out:?}");
        }
    }

    #[test]
    fn int_typed_method_call_refused_p15() {
        // `s.len()` is postfix-safe syntactically but usize-typed — the
        // oracle is SILENT (probe round 1) while `s.trim()` (&str) fires.
        // The receiver's return type is invisible to syn → refuse both.
        let input = r#"fn f(s: &str) -> String { format!("{}", s.len()) }"#;
        let out = propose_fix("useless_format", input);
        assert!(out.is_none(), "int-typed method arg is oracle-silent: {out:?}");
    }

    #[test]
    fn index_and_field_args_refused_p15() {
        // The two pre-P15 fixtures FIRED here (names[0] → names[0].to_string()
        // was probe-verified on the older clippy; the 1.97 oracle type-gates
        // both: names[0] fires only because the element type is String,
        // s.n is u32 → silent). Element/field types are invisible to syn —
        // the shapes refuse (the composite eval's range_loop_plus_format_args
        // useless_format half is the honest persist).
        let input = r#"fn f(names: &[String]) -> String { format!("{}", names[0]) }"#;
        let out = propose_fix("useless_format", input);
        assert!(out.is_none(), "index arg: element type invisible: {out:?}");

        let input2 = "struct S { n: u32 }\nfn f(s: &S) -> String { format!(\"{}\", s.n) }";
        let out2 = propose_fix("useless_format", input2);
        assert!(out2.is_none(), "field arg: oracle-silent (u32): {out2:?}");
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
