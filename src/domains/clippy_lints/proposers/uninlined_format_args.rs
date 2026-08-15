//! `clippy::uninlined_format_args` — `format!("{}", name)` →
//! `format!("{name}")` (Issue 016).
//!
//! Token-level surgery on the macro args (the args are opaque to `syn`'s
//! AST, so the proposer walks `mac.tokens` as `proc_macro2::TokenTree`s —
//! `Group`s hide their inner commas, making a linear top-level walk sound):
//!
//! 1. The first token must be a plain string literal (a `"`-opening one —
//!    raw/byte/c-strings are rejected) — the TEMPLATE.
//! 2. After it: `(Comma, arg-tokens…)` repeats; each arg must parse as a
//!    `syn::Expr` that is a single-ident path. Anything else (field
//!    access, calls, casts, literals, `r#raw` or `self` idents) aborts
//!    the WHOLE macro — no partial inlining. A trailing comma is allowed
//!    and left alone.
//! 3. The template is scanned char-by-char over its RAW literal text
//!    (the bytes between the quotes, escapes intact — scanning the
//!    UNESCAPED `.value()` would erase the `{{`-vs-`{}` distinction):
//!    escape payloads (`\n`, `\xNN`, `\u{…}` — braces inside!) copy
//!    verbatim, `{{`/`}}` copy verbatim, an EMPTY placeholder `{}`
//!    consumes the next arg and emits `{ident}`, any NON-empty
//!    placeholder (`{:?}`, `{0}`, `{name}`) aborts. Placeholder count
//!    must equal arg count.
//!
//! Rewrites: the template literal's byte range is replaced by the NEW
//! literal — `"` + the rebuilt raw text + `"`, where every untouched byte
//! is an ORIGINAL source fragment (escapes keep the caller's spelling);
//! each arg is deleted from its preceding comma's start through its last
//! token's end.

use super::SourceEdit;
use proc_macro2::{TokenStream, TokenTree};
use syn::visit::Visit;
use syn::{Expr, ExprMacro, File, LitStr, Macro, Path};

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
        self.try_inline(&m.mac);
    }

    /// Statement-position macros (`println!(…);`) parse as `Stmt::Macro`,
    /// not `Expr::Macro` — the print family lives there, so both shapes
    /// are visited.
    fn visit_stmt_macro(&mut self, m: &'ast syn::StmtMacro) {
        syn::visit::visit_stmt_macro(self, m);
        self.try_inline(&m.mac);
    }
}

impl Collector {
    fn try_inline(&mut self, mac: &Macro) {
        if !is_format_family(&mac.path) {
            return;
        }
        if let Some(edits) = inline_args(&mac.tokens) {
            self.edits.extend(edits);
        }
    }
}

/// The macro family this rewrite applies to.
fn is_format_family(path: &Path) -> bool {
    ["format", "println", "print", "eprintln", "eprint"]
        .iter()
        .any(|name| path.is_ident(name))
}

/// One inlineable argument: its preceding comma's start byte, its last
/// token's end byte, and its ident.
struct ArgSpan {
    comma_start: usize,
    arg_end: usize,
    name: String,
}

/// Compute the edits for one macro invocation. `None` = abort (the macro
/// keeps its original text).
fn inline_args(tokens: &TokenStream) -> Option<Vec<SourceEdit>> {
    let toks: Vec<TokenTree> = tokens.clone().into_iter().collect();
    let first = toks.first()?;
    let text = match first {
        TokenTree::Literal(l) if l.to_string().starts_with('"') => l.to_string(),
        _ => return None, // raw/byte/c-string or non-literal first token
    };
    // Validate the token is a well-formed plain string literal (also
    // rejects suffixed/malformed shapes), then take its raw inner text.
    syn::parse_str::<LitStr>(&text).ok()?;
    let raw = &text[1..text.len() - 1];
    let template_range = first.span().byte_range();
    let args = scan_args(&toks)?;

    let (new_raw, used) = build_inlined(raw, &args)?;
    if used == 0 {
        return None; // zero placeholders — nothing to inline here
    }

    let mut edits = vec![SourceEdit {
        start: template_range.start,
        end: template_range.end,
        replacement: format!("\"{new_raw}\""),
    }];
    for a in &args {
        edits.push(SourceEdit {
            start: a.comma_start,
            end: a.arg_end,
            replacement: String::new(),
        });
    }
    Some(edits)
}

/// Walk `(Comma, arg…)*` after the template. `None` on any malformed
/// shape or non-inlinable argument (all-or-nothing).
fn scan_args(toks: &[TokenTree]) -> Option<Vec<ArgSpan>> {
    let mut args = Vec::new();
    let mut i = 1;
    while i < toks.len() {
        let comma_start = match &toks[i] {
            TokenTree::Punct(p) if p.as_char() == ',' => p.span().byte_range().start,
            _ => return None,
        };
        i += 1;
        let start = i;
        while i < toks.len() && !is_comma(&toks[i]) {
            i += 1;
        }
        if start == i {
            // Nothing between this comma and the next/end. A trailing
            // comma after the last arg is fine; anything else is malformed.
            return if i == toks.len() { Some(args) } else { None };
        }
        let stream: TokenStream = toks[start..i].iter().cloned().collect();
        let expr = syn::parse2::<Expr>(stream).ok()?;
        let Expr::Path(p) = &expr else {
            return None; // field access / call / cast / literal … — no partial inlining
        };
        let ident = p.path.get_ident()?.to_string();
        // `{r#raw}` and `{self}` are not valid format-arg names — skip.
        if ident.contains('#') || ident == "self" {
            return None;
        }
        args.push(ArgSpan {
            comma_start,
            arg_end: toks[i - 1].span().byte_range().end,
            name: ident,
        });
    }
    Some(args)
}

fn is_comma(t: &TokenTree) -> bool {
    matches!(t, TokenTree::Punct(p) if p.as_char() == ',')
}

/// Build the inlined RAW literal text + the number of args consumed.
/// `None` aborts: non-empty placeholder content, malformed braces, or an
/// arg-count mismatch.
fn build_inlined(raw: &str, args: &[ArgSpan]) -> Option<(String, usize)> {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    let mut next_arg = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                // Escape payload copies verbatim — the caller's spelling
                // survives byte-for-byte (`\t` stays `\t`, not a raw TAB).
                out.push('\\');
                i += 1;
                match chars.get(i) {
                    Some('u') => {
                        out.push('u');
                        i += 1;
                        // `\u{…}` — braces belong to the escape, not the
                        // template grammar; copy through the closer.
                        if chars.get(i) == Some(&'{') {
                            out.push('{');
                            i += 1;
                            while i < chars.len() && chars[i] != '}' {
                                out.push(chars[i]);
                                i += 1;
                            }
                            if i >= chars.len() {
                                return None; // malformed — unreachable in compiled code
                            }
                            out.push('}');
                            i += 1;
                        }
                    }
                    Some(&n) => {
                        out.push(n); // covers `\n` `\x`NN (digits ride along as plain chars)
                        i += 1;
                    }
                    None => return None,
                }
            }
            '{' if chars.get(i + 1) == Some(&'{') => {
                out.push_str("{{");
                i += 2;
            }
            '}' if chars.get(i + 1) == Some(&'}') => {
                out.push_str("}}");
                i += 2;
            }
            '{' => {
                let mut content = String::new();
                i += 1;
                loop {
                    match chars.get(i) {
                        Some('}') => {
                            i += 1;
                            break;
                        }
                        Some(&ch) => {
                            content.push(ch);
                            i += 1;
                        }
                        None => return None, // unterminated — unreachable in compiled code
                    }
                }
                if !content.is_empty() {
                    return None; // `{:?}` / `{0}` / `{name}` — conservative v1
                }
                let arg = args.get(next_arg)?;
                next_arg += 1;
                out.push('{');
                out.push_str(&arg.name);
                out.push('}');
            }
            '}' => return None, // lone `}` — invalid in a format string
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    if next_arg != args.len() {
        return None; // leftover args (or placeholders ≠ args) — defensive
    }
    Some((out, next_arg))
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn single_implicit_placeholder_inlined() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(name: &str) -> String { format!(\"{}\", name) }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(name: &str) -> String { format!(\"{name}\") }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn two_placeholders_inlined_in_order() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(a: u8, b: u8) { println!(\"{} {}\", a, b); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(a: u8, b: u8) { println!(\"{a} {b}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn surrounding_text_preserved() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(name: &str) -> String { format!(\"hi {}\", name) }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(name: &str) -> String { format!(\"hi {name}\") }");
    }

    #[test]
    fn escaped_braces_copy_verbatim() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> String { format!(\"{{}} {}\", x) }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(x: u8) -> String { format!(\"{{}} {x}\") }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn escape_sequences_keep_caller_spelling() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> String { format!(\"a\\tb {}\", x) }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(x: u8) -> String { format!(\"a\\tb {x}\") }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn trailing_comma_left_alone() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(name: &str) -> String { format!(\"{}\", name,) }",
        )
        .expect("pattern matched");
        assert!(out.contains("\"{name}\""), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn format_spec_aborts() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(v: u8) -> String { format!(\"{:?}\", v) }",
        );
        assert!(out.is_none(), "{{:?}} spec is not inlinable: {out:?}");
    }

    #[test]
    fn field_access_arg_aborts() {
        // `self.x` parses but is not a bare ident path — no partial inlining.
        let out = propose_fix(
            "uninlined_format_args",
            "struct S { x: u8 }\nimpl S { fn f(&self) -> String { format!(\"{}\", self.x) } }",
        );
        assert!(out.is_none(), "field access must abort: {out:?}");
    }

    #[test]
    fn already_inlined_unchanged() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> String { format!(\"{x}\") }",
        );
        assert!(out.is_none(), "named placeholder is not our pattern: {out:?}");
    }

    #[test]
    fn no_placeholders_unchanged() {
        // `useless_format`'s territory — nothing to inline here.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f() -> String { format!(\"done\") }",
        );
        assert!(out.is_none(), "no placeholders, no edits: {out:?}");
    }
}
