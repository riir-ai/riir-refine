//! `clippy::uninlined_format_args` — `format!("{}", name)` →
//! `format!("{name}")` (Issue 016; T4 extension 2026-08-17).
//!
//! Token-level surgery on the macro args (the args are opaque to `syn`'s
//! AST, so the proposer walks `mac.tokens` as `proc_macro2::TokenTree`s —
//! `Group`s hide their inner commas, making a linear top-level walk sound):
//!
//! 1. Macro args are split into top-level comma-separated slices. The
//!    TEMPLATE slice position depends on the macro family (Issue 029 T4
//!    probe matrix, all `MachineApplicable` vs real clippy 1.97 pedantic):
//!    first for `format`/`println`/`print`/`eprintln`/`eprint`/`panic`/
//!    `format_args`/`bail`, SECOND for `assert`/`debug_assert`/`write`/
//!    `writeln`/`ensure` (condition/target first), THIRD for `assert_eq`/
//!    `assert_ne`/`debug_assert_eq`/`debug_assert_ne` (two operands first).
//!    `unreachable!` stays OUT — the probe shows clippy is silent there
//!    while `panic!` fires. Family matching is on the LAST path segment
//!    (2026-08-18 slice-5 extension) so qualified invocations work:
//!    `anyhow::ensure!(cond, "… {}", x)` / `anyhow::bail!("… {}", x)`
//!    fire real clippy exactly like their `use`-imported bare forms
//!    (probe evidence: the riir-train `rosetta`/`game_concept_graph`
//!    findings). Other qualified families (`std::format!` etc.) match the
//!    same way — real clippy is qualifier-agnostic (post-expansion).
//! 2. The template slice must be a plain `"`-opening string literal.
//!    Everything after it: format args, each parsed as a `syn::Expr`
//!    that is a single-ident path. Anything else (field access, calls,
//!    casts, literals, named args `w = 5`, `r#raw`/`self` idents) aborts
//!    the WHOLE macro — no partial inlining. A trailing comma is allowed.
//! 3. The template is scanned char-by-char over its RAW literal text:
//!    escape payloads copy verbatim, `{{`/`}}` copy verbatim, then the
//!    placeholder classifier (T4 extension — spec'd placeholders are now
//!    inlined, carrying the spec verbatim):
//!    - `{}` → consumes the next arg, emits `{ident}`;
//!    - `{:spec}` → consumes the next arg, emits `{ident:spec}` — the
//!      spec rides along unchanged (`{:?}` → `{v:?}`, `{:.1}` →
//!      `{x:.1}`; same arg, same trait bounds). ABORTS if the spec
//!      carries a positional `$` ref (`{:1$}`): the referenced arg is
//!      deleted by the inline, dangling the index. Scope-captured NAME
//!      refs (`{:>width$}`) are sound and carried.
//!    - `{name}` / `{name:spec}` → already named; copies verbatim,
//!      consumes NOTHING (mixed already-named + positional templates
//!      inline only the positional side).
//!    - `{0}` (explicit index) → aborts (deferred — reordering breaks
//!      the linear consumption model).
//!
//!    Placeholder count must equal the consumed arg count.
//!
//! Rewrites: the template literal's byte range is replaced by the NEW
//! literal — `"` + the rebuilt raw text + `"`; each consumed arg is
//! deleted from its preceding comma's start through its last token's
//! end. The template's OWN preceding comma (assert/write families) is
//! kept — only consumed args' commas go.

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

/// Where the template literal sits in the macro's argument list.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TemplatePos {
    First,
    Second,
    Third,
}

impl TemplatePos {
    fn index(self) -> usize {
        match self {
            TemplatePos::First => 0,
            TemplatePos::Second => 1,
            TemplatePos::Third => 2,
        }
    }
}

impl Collector {
    fn try_inline(&mut self, mac: &Macro) {
        let Some(pos) = macro_family(&mac.path) else {
            return;
        };
        if let Some(edits) = inline_args(&mac.tokens, pos) {
            self.edits.extend(edits);
        }
    }
}

/// The macro family this rewrite applies to (Issue 029 T4 probe matrix —
/// every listed macro fires real clippy on inlineable format args;
/// `unreachable!` is probe-verified SILENT and stays out). Matching is on
/// the LAST path segment, so `anyhow::ensure!`/`anyhow::bail!` and other
/// qualified invocations resolve the same as bare names. The log family
/// (`log::info/warn/error/debug/trace`, same-named `tracing` macros) is
/// template-first like `format!` — real clippy fires on it (probe evidence:
/// katgpt-spectral `outlier_guard.rs` `log::warn!`/`log::error!` sites); the
/// tracing `target:`/`parent:` named-arg forms abort via the named-arg guard.
fn macro_family(path: &Path) -> Option<TemplatePos> {
    use TemplatePos::{First, Second, Third};
    let name = path.segments.last()?.ident.to_string();
    Some(match name.as_str() {
        "format" | "println" | "print" | "eprintln" | "eprint" | "panic" | "format_args" | "bail" | "anyhow" | "info" | "warn" | "error" | "debug" | "trace" => First,
        "assert" | "debug_assert" | "write" | "writeln" | "ensure" => Second,
        "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => Third,
        _ => return None,
    })
}

/// One inlineable argument: its preceding comma's start byte, its last
/// token's end byte, and its ident.
struct ArgSpan {
    comma_start: usize,
    arg_end: usize,
    name: String,
}

/// One top-level macro argument (token slice + span bookkeeping).
struct MacroArg {
    toks: Vec<TokenTree>,
    /// Start byte of the comma separating this arg from the previous one.
    comma_start: usize,
    /// End byte of this arg's last token.
    end: usize,
}

/// Compute the edits for one macro invocation. `None` = abort (the macro
/// keeps its original text).
fn inline_args(tokens: &TokenStream, pos: TemplatePos) -> Option<Vec<SourceEdit>> {
    let toks: Vec<TokenTree> = tokens.clone().into_iter().collect();
    let args = split_args(&toks)?;
    let template_arg = args.get(pos.index())?;

    // The template must be a single plain string literal (a `"`-opening
    // one — raw/byte/c-strings are rejected; also rejects suffixed or
    // malformed shapes).
    if template_arg.toks.len() != 1 {
        return None;
    }
    let text = match &template_arg.toks[0] {
        TokenTree::Literal(l) if l.to_string().starts_with('"') => l.to_string(),
        _ => return None, // no template here (e.g. bare `assert!(cond)`) — not ours
    };
    syn::parse_str::<LitStr>(&text).ok()?;
    let raw = &text[1..text.len() - 1];
    let template_range = template_arg.toks[0].span().byte_range();

    // Everything after the template is a format arg — ident paths only.
    let mut fmt_args = Vec::new();
    for arg in &args[pos.index() + 1..] {
        let stream: TokenStream = arg.toks.iter().cloned().collect();
        let expr = syn::parse2::<Expr>(stream).ok()?;
        let Expr::Path(p) = &expr else {
            return None; // field access / call / cast / literal / named-arg … — no partial inlining
        };
        let ident = p.path.get_ident()?.to_string();
        // `{r#raw}` is not a valid format-arg name — skip. (`{self}` IS
        // valid: rustc explicitly allows `self` as an implicit capture —
        // probe-verified 2026-08-18, `format!("{self:?}")` compiles + real
        // clippy fires on `format!("{:?}", self)` sites.)
        if ident.contains('#') {
            return None;
        }
        fmt_args.push(ArgSpan {
            comma_start: arg.comma_start,
            arg_end: arg.end,
            name: ident,
        });
    }

    let (new_raw, used) = build_inlined(raw, &fmt_args)?;
    if used == 0 {
        return None; // nothing consumed — nothing to inline here
    }

    let mut edits = vec![SourceEdit {
        start: template_range.start,
        end: template_range.end,
        replacement: format!("\"{new_raw}\""),
    }];
    for a in &fmt_args {
        edits.push(SourceEdit {
            start: a.comma_start,
            end: a.arg_end,
            replacement: String::new(),
        });
    }
    Some(edits)
}

/// Split the macro's top-level token stream on commas. `None` on shapes
/// that can't be an arg list (leading comma, empty arg before the end).
/// A trailing comma is dropped from consideration (kept in the source —
/// nothing after it is an arg).
fn split_args(toks: &[TokenTree]) -> Option<Vec<MacroArg>> {
    let mut args = Vec::new();
    let mut i = 0usize;
    let mut pending_comma: Option<usize> = None;
    while i < toks.len() {
        let start = i;
        while i < toks.len() && !is_comma(&toks[i]) {
            i += 1;
        }
        if start == i {
            // A comma with nothing before it in this slot: acceptable ONLY
            // as the final trailing comma; anywhere else it is malformed.
            return if i == toks.len() - 1 { Some(args) } else { None };
        }
        args.push(MacroArg {
            toks: toks[start..i].to_vec(),
            comma_start: pending_comma.unwrap_or(0),
            end: toks[i - 1].span().byte_range().end,
        });
        pending_comma = match toks.get(i) {
            Some(t) if is_comma(t) => Some(t.span().byte_range().start),
            _ => None,
        };
        i += 1; // step over the comma (or the end marker)
    }
    Some(args)
}

fn is_comma(t: &TokenTree) -> bool {
    matches!(t, TokenTree::Punct(p) if p.as_char() == ',')
}

/// One top-level macro argument's token slice (shared with sibling
/// proposers' macro-arg descent — `map_unwrap_or` scans the same
/// expression positions real clippy sees post-expansion).
pub(super) struct ExprMacroArg {
    pub(super) toks: Vec<TokenTree>,
}

/// The EXPRESSION arguments of a known format-family macro invocation —
/// everything after the template position. `None` when the macro isn't in
/// the family or the arg list is malformed. The args of these macros are
/// expressions by definition (same family table `inline_args` uses), so a
/// sibling proposer can parse each arg as an `Expr` and match inside it;
/// proc-macro2 spans survive the token round-trip, so nodes parsed from
/// these tokens carry FILE-relative byte ranges that `src`/`edit` slice
/// correctly. Args that don't parse as expressions (e.g. `key = value`
/// named args) are simply omitted — the caller skips them.
pub(super) fn post_template_expr_args(mac: &Macro) -> Option<Vec<ExprMacroArg>> {
    let pos = macro_family(&mac.path)?;
    let toks: Vec<TokenTree> = mac.tokens.clone().into_iter().collect();
    let args = split_args(&toks)?;
    Some(
        args.into_iter()
            .skip(pos.index() + 1)
            .map(|a| ExprMacroArg { toks: a.toks })
            .collect(),
    )
}

/// Build the inlined RAW literal text + the number of args consumed.
/// `None` aborts: an uninlinable placeholder (explicit index, positional
/// `$` ref, malformed content), malformed braces, or an arg-count
/// mismatch.
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
                match classify_placeholder(&content) {
                    Placeholder::Empty => {
                        let arg = args.get(next_arg)?;
                        next_arg += 1;
                        out.push('{');
                        out.push_str(&arg.name);
                        out.push('}');
                    }
                    Placeholder::Spec(spec) => {
                        // T4: the spec rides along verbatim — same arg,
                        // same trait bounds, same formatting.
                        if has_dynamic_spec_ref(spec) {
                            return None; // `{:1$}` / `{:.*}` — the spec itself consumes/references another positional arg
                        }
                        let arg = args.get(next_arg)?;
                        next_arg += 1;
                        out.push('{');
                        out.push_str(&arg.name);
                        out.push(':');
                        out.push_str(spec);
                        out.push('}');
                    }
                    Placeholder::AlreadyNamed => {
                        // `{name}` / `{name:spec}` — copies verbatim,
                        // consumes nothing (its binding is not one of the
                        // positional args).
                        out.push('{');
                        out.push_str(&content);
                        out.push('}');
                    }
                    Placeholder::NotInlinable => return None,
                }
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

enum Placeholder<'a> {
    /// `{}` — implicit positional, consumes the next arg.
    Empty,
    /// `{:spec}` — implicit positional with a format spec, consumes the
    /// next arg.
    Spec(&'a str),
    /// `{name}` or `{name:spec}` — already named; consumes nothing.
    AlreadyNamed,
    /// `{0}` / `{1}` (explicit index) or anything malformed — abort.
    NotInlinable,
}

fn classify_placeholder(content: &str) -> Placeholder<'_> {
    if content.is_empty() {
        return Placeholder::Empty;
    }
    if let Some(spec) = content.strip_prefix(':') {
        return Placeholder::Spec(spec);
    }
    if is_named_content(content) {
        return Placeholder::AlreadyNamed;
    }
    Placeholder::NotInlinable
}

/// `{name}` or `{name:spec}` — a valid ident prefix (explicit indices
/// `{0}` are NOT this: digits are not ident starts).
fn is_named_content(content: &str) -> bool {
    let mut chars = content.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    for c in chars.by_ref() {
        if c == ':' {
            break; // the rest is the spec — verbatim either way
        }
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
    }
    true
}

/// A spec that references or consumes ANOTHER positional arg — the inline
/// would delete that arg, corrupting the reference:
///
/// - `{:1$}` — width/precision from positional arg 1 (digit before `$`);
/// - `{:.*}` — precision from the NEXT positional arg (dot-star).
///
/// Scope-captured NAME refs (`{:>width$}`) and fill chars (`{:*<5}` — a
/// `*` NOT preceded by `.`) carry fine.
fn has_dynamic_spec_ref(spec: &str) -> bool {
    let chars: Vec<char> = spec.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '$' && i > 0 && chars[i - 1].is_ascii_digit() {
            return true;
        }
        if c == '*' && i > 0 && chars[i - 1] == '.' {
            return true;
        }
    }
    false
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
    fn debug_spec_carried_verbatim() {
        // T4 (probe P1): `{:?}` fires MachineApplicable — the spec rides
        // along after the inlined name.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(v: u8) -> String { format!(\"{:?}\", v) }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(v: u8) -> String { format!(\"{v:?}\") }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn precision_and_width_specs_carried() {
        // T4 (probes P2/P3/P15/P23): `{:.1}`, `{:>12}`, `{:#x}`, `{:5}`.
        let p = propose_fix(
            "uninlined_format_args",
            "fn f(x: f32) -> String { format!(\"{:.1}\", x) }",
        )
        .expect("pattern matched");
        assert_eq!(p, "fn f(x: f32) -> String { format!(\"{x:.1}\") }");
        let w = propose_fix(
            "uninlined_format_args",
            "fn f(s: &str) -> String { format!(\"{:>12}\", s) }",
        )
        .expect("pattern matched");
        assert_eq!(w, "fn f(s: &str) -> String { format!(\"{s:>12}\") }");
        let h = propose_fix(
            "uninlined_format_args",
            "fn f(v: u32) -> String { format!(\"{:#x}\", v) }",
        )
        .expect("pattern matched");
        assert_eq!(h, "fn f(v: u32) -> String { format!(\"{v:#x}\") }");
    }

    #[test]
    fn mixed_spec_and_plain_inlined_in_order() {
        // T4 (probe P4/P7): the nf_flow_goat miss class —
        // `"{:?} {}"` and `"{} {:.1}"` with ident args.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(a: u8, b: f32) -> String { format!(\"{:?} {:.1}\", a, b) }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(a: u8, b: f32) -> String { format!(\"{a:?} {b:.1}\") }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn already_named_placeholder_untouched() {
        // T4 (probe P12/P13): `{name}` copies verbatim, consumes nothing;
        // only the positional `{}` side inlines.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(name: &str, v: u8) -> String { format!(\"{name} got {} {:?}\", v, v) }",
        )
        .expect("pattern matched");
        assert!(
            out.contains("\"{name} got {v} {v:?}\""),
            "named placeholder mangled: {out}"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn named_with_spec_untouched() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(w: usize) -> String { format!(\"{w:>4} {}\", w) }",
        )
        .expect("pattern matched");
        assert!(out.contains("\"{w:>4} {w}\""), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn scope_captured_dollar_name_ref_carried() {
        // T4 (probe P24): `{:>width$}` with NO explicit width arg — the
        // ref is scope-captured; carrying it verbatim is sound.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8, width: usize) -> String { format!(\"{:>width$}\", x) }",
        )
        .expect("pattern matched");
        assert_eq!(
            out,
            "fn f(x: u8, width: usize) -> String { format!(\"{x:>width$}\") }"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn positional_dollar_ref_aborts() {
        // T4 (probe P20 fires in clippy, we stay silent): `{:1$}` refs
        // positional arg 1 — inlining deletes that arg, dangling the
        // index. Documented conservative divergence.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: f64, p: usize) -> String { format!(\"{:1$}\", x, p) }",
        );
        assert!(out.is_none(), "positional $ ref must abort: {out:?}");
    }

    #[test]
    fn dot_star_precision_aborts() {
        // `{:.*}` consumes the NEXT positional arg as precision — a
        // two-arg placeholder the linear model cannot express.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(p: usize, x: f64) -> String { format!(\"{:.*}\", p, x) }",
        );
        assert!(out.is_none(), ".* precision must abort: {out:?}");
    }

    #[test]
    fn star_fill_char_carries() {
        // A `*` NOT after a `.` is a FILL character — constant spec, safe.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(s: &str) -> String { format!(\"{:*<5}\", s) }",
        )
        .expect("pattern matched");
        assert!(out.contains("\"{s:*<5}\""), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn explicit_index_aborts() {
        // T4 (probe P5 fires in clippy, we stay silent): `{0}` reordering
        // breaks the linear consumption model — deferred.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> String { format!(\"{0}\", x) }",
        );
        assert!(out.is_none(), "explicit index must abort: {out:?}");
    }

    #[test]
    fn assert_message_inlined() {
        // T4 (probe P8): template at SECOND position (after the cond).
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) { assert!(x > 0, \"got {}\", x); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(x: u8) { assert!(x > 0, \"got {x}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn assert_cond_string_literal_not_mistaken_for_template() {
        // The condition's own string literal must not be picked as the
        // template — the template is the SECOND top-level arg.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(s: &str) { assert!(s == \"x\", \"got {}\", s); }",
        )
        .expect("pattern matched");
        assert!(out.contains("\"got {s}\""), "{out}");
        assert!(out.contains("s == \"x\""), "cond mangled: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn assert_eq_template_at_third_position() {
        // T4 (probe P9): two operands, then the template.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(a: u8, b: u8) { assert_eq!(a, b, \"mismatch {} vs {}\", a, b); }",
        )
        .expect("pattern matched");
        assert!(out.contains("\"mismatch {a} vs {b}\""), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn assert_ne_and_debug_variants_inlined() {
        let ne = propose_fix(
            "uninlined_format_args",
            "fn f(a: u8, b: u8) { assert_ne!(a, b, \"{} != {}\", a, b); }",
        )
        .expect("pattern matched");
        assert!(ne.contains("\"{a} != {b}\""), "{ne}");
        let d = propose_fix(
            "uninlined_format_args",
            "fn f(a: u8, b: u8) { debug_assert_eq!(a, b, \"d {} {}\", a, b); }",
        )
        .expect("pattern matched");
        assert!(d.contains("\"d {a} {b}\""), "{d}");
    }

    #[test]
    fn write_and_writeln_inlined() {
        // T4 (probes P10/P16): target first, template second. A literal
        // arg aborts (not an ident) — the target slice itself may be any
        // expression, it is never touched.
        let lit = propose_fix(
            "uninlined_format_args",
            "use std::fmt; fn f(f: &mut fmt::Formatter) -> fmt::Result { write!(f, \"val {}\", 1) }",
        );
        assert!(lit.is_none(), "literal arg must abort: {lit:?}");
        let out = propose_fix(
            "uninlined_format_args",
            "use std::fmt; fn f(f: &mut fmt::Formatter, v: u8) -> fmt::Result { write!(f, \"val {}\", v) }",
        )
        .expect("pattern matched");
        assert!(out.contains("\"val {v}\""), "{out}");
        assert!(out.contains("write!(f,"), "target mangled: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
        let wl = propose_fix(
            "uninlined_format_args",
            "use std::fmt; fn f(f: &mut fmt::Formatter, v: u8) -> fmt::Result { writeln!(f, \"line {}\", v) }",
        )
        .expect("pattern matched");
        assert!(wl.contains("\"line {v}\""), "{wl}");
    }

    #[test]
    fn panic_and_format_args_inlined() {
        // T4 (probes P11/P19): template-first like format!.
        let p = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) { if x > 100 { panic!(\"too big {}\", x); } }",
        )
        .expect("pattern matched");
        assert!(p.contains("\"too big {x}\""), "{p}");
        let fa = propose_fix(
            "uninlined_format_args",
            "fn f(v: u8) { let _ = format_args!(\"plain {}\", v); }",
        )
        .expect("pattern matched");
        assert!(fa.contains("\"plain {v}\""), "{fa}");
    }

    #[test]
    fn bail_and_ensure_inlined() {
        // Slice-5 extension (2026-08-18): anyhow's `bail!` is template-first
        // (like panic!), `ensure!` is condition-first (like assert!). Probe
        // evidence: the riir-train rosetta/game_concept_graph findings.
        let b = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> anyhow::Result<()> { bail!(\"bad {}\", x) }",
        )
        .expect("pattern matched");
        assert!(b.contains("\"bad {x}\""), "{b}");
        let e = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> anyhow::Result<()> { ensure!(x > 0, \"must be positive: {}\", x); Ok(()) }",
        )
        .expect("pattern matched");
        assert!(e.contains("\"must be positive: {x}\""), "{e}");
        assert!(e.contains("x > 0"), "cond mangled: {e}");
        assert!(syn::parse_file(&e).is_ok(), "{e}");
    }

    #[test]
    fn qualified_anyhow_bail_inlined_with_escaped_quotes() {
        // The production shape from riir-train game_concept_graph.rs: a
        // qualified path + escaped quotes around the placeholder. Last-
        // segment family matching resolves `anyhow::bail` to `bail`.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(other: &str) -> anyhow::Result<()> { anyhow::bail!(\"Unknown concept class: \\\"{}\\\"\", other) }",
        )
        .expect("pattern matched");
        assert!(
            out.contains("\\\"{other}\\\""),
            "escaped quotes must survive: {out}"
        );
        assert!(out.contains("anyhow::bail!"), "path mangled: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn qualified_anyhow_ensure_inlined() {
        // The production shape from riir-train rosetta/mod.rs: qualified
        // ensure! with a comparison condition (no top-level comma).
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(v: u32, V: u32) -> anyhow::Result<()> { anyhow::ensure!(v == V, \"Unsupported version: {}\", v); Ok(()) }",
        )
        .expect("pattern matched");
        assert!(out.contains("\"Unsupported version: {v}\""), "{out}");
        assert!(out.contains("v == V"), "cond mangled: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn qualified_std_format_inlined() {
        // Last-segment matching is qualifier-agnostic: `std::format!` is
        // the same macro real clippy fires on.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> String { std::format!(\"{}\", x) }",
        )
        .expect("pattern matched");
        assert!(out.contains("\"{x}\""), "{out}");
        assert!(out.contains("std::format!"), "path mangled: {out}");
    }

    #[test]
    fn qualified_anyhow_macro_inlined() {
        // `anyhow::anyhow!` is template-first exactly like `bail!`/`panic!`.
        // Production shape from riir-chain catchup/shard_quorum.rs (2 live
        // sites stayed healer-silent until the family table gained `anyhow`).
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> anyhow::Error { anyhow::anyhow!(\"bad node {} failed\", x) }",
        )
        .expect("pattern matched");
        assert!(out.contains("\"bad node {x} failed\""), "{out}");
        assert!(out.contains("anyhow::anyhow!"), "path mangled: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn qualified_log_macros_inlined() {
        // The log family (`log::info/warn/error/debug/trace`) is template-first
        // exactly like `format!`. Production shape from katgpt-spectral
        // outlier_guard.rs (2 live sites stayed healer-silent until the family
        // table gained the log names — real clippy fires on them).
        let w = propose_fix(
            "uninlined_format_args",
            "fn f(layer_idx: usize, name: &str, ks_d: f32) { log::warn!(\"OutlierGuard: layer {} ({}) flagged with D={:.4}\", layer_idx, name, ks_d); }",
        )
        .expect("pattern matched");
        assert!(
            w.contains("\"OutlierGuard: layer {layer_idx} ({name}) flagged with D={ks_d:.4}\""),
            "{w}"
        );
        assert!(w.contains("log::warn!"), "path mangled: {w}");
        assert!(syn::parse_file(&w).is_ok(), "{w}");
        let e = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) { log::error!(\"bad value {}\", x); }",
        )
        .expect("pattern matched");
        assert!(e.contains("\"bad value {x}\""), "{e}");
        assert!(e.contains("log::error!"), "path mangled: {e}");
        assert!(syn::parse_file(&e).is_ok(), "{e}");
    }

    #[test]
    fn tracing_named_target_arg_stays_silent() {
        // `info!(target: "x", "msg {}", v)` — the tracing named-arg prefix is
        // NOT a plain string literal at the template position, so the whole
        // invocation aborts (no partial inlining). The log-family extension
        // must not touch this shape.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(v: u8) { info!(target: \"my_target\", \"msg {}\", v); }",
        );
        assert!(out.is_none(), "tracing named-target form must abort: {out:?}");
    }

    #[test]
    fn log_macro_bare_debug_fires() {
        // Bare (unqualified) log-family names resolve the same — `debug!`
        // with a string template + ident args is the format shape; a
        // non-format `debug!(code, "msg")` shape aborts because the template
        // slot holds an ident, not a string literal.
        let fires = propose_fix(
            "uninlined_format_args",
            "fn f(step: u32) { debug!(\"at step {}\", step); }",
        )
        .expect("pattern matched");
        assert!(fires.contains("\"at step {step}\""), "{fires}");
        let silent = propose_fix(
            "uninlined_format_args",
            "fn f(code: i32) { debug!(code, \"plain message\", ); }",
        );
        assert!(silent.is_none(), "ident-first non-format shape must abort: {silent:?}");
    }

    #[test]
    fn self_arg_inlined() {
        // `format!("{self:?}")` is VALID: rustc explicitly allows `self` as
        // an implicit format-arg capture (probe-verified 2026-08-18 —
        // compiles + runs; real clippy fires on `format!("{:?}", self)`).
        // Production shape from riir-chain deploy/lifecycle.rs. The old
        // `ident == "self"` guard was a false premise.
        let out = propose_fix(
            "uninlined_format_args",
            "#[derive(Debug)] struct S; impl S { fn f(&self, t: &S) -> String { format!(\"transition {:?} -> {:?}\", self, t) } }",
        )
        .expect("pattern matched");
        assert!(out.contains("{self:?}"), "{out}");
        assert!(out.contains("{t:?}"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn ensure_without_message_untouched() {
        // Mirrors `assert_without_message_untouched`: no template at
        // position 1 - nothing to do.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> anyhow::Result<()> { ensure!(x > 0); Ok(()) }",
        );
        assert!(out.is_none(), "no-template ensure must not fire: {out:?}");
    }

    #[test]
    fn unreachable_stays_silent() {
        // T4 probe P21: real clippy is SILENT on unreachable! (while
        // panic! fires) — we match.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> u8 { match x { 0 => 1, _ => unreachable!(\"no {}\", x) } }",
        );
        assert!(out.is_none(), "unreachable! must stay silent: {out:?}");
    }

    #[test]
    fn assert_without_message_untouched() {
        // Probe P25 control: no template at position 1 — nothing to do.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) { assert!(x > 0); }",
        );
        assert!(out.is_none(), "no-template assert must not fire: {out:?}");
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
    fn already_inlined_spec_unchanged() {
        // `{x:?}` fully named+spec — nothing to consume.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8) -> String { format!(\"{x:?}\") }",
        );
        assert!(out.is_none(), "fully-named placeholder must not fire: {out:?}");
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
