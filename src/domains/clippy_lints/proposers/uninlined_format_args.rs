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
//!    Everything after it: format args, each parsed as a `syn::Expr`.
//!    Three shapes are accepted (the rest abort the WHOLE macro — no
//!    partial inlining):
//!    - a plain single-ident path — a positional arg, consumed by `{}` /
//!      `{:spec}`;
//!    - a redundant named arg `n = n` — deleted, template untouched;
//!    - a named-arg-with-RENAME `n = v` (both plain idents, `n != v`) —
//!      every template occurrence of `n` is rewritten to `v` (placeholder
//!      names AND width/precision `$` refs inside specs) and the arg is
//!      deleted (P3 post-mining unit, 2026-08-29; oracle = real clippy
//!      1.97 probe: const/param/`self` values all fire `MachineApplicable`,
//!      field-access values `val = s.0` are SILENT so they abort, and an
//!      is referenced). `r#raw` on either side aborts. Field access,
//!      calls, casts, literals (`w = 5`) abort the WHOLE macro. A
//!      trailing comma is allowed.
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
        self.descend_nested_macros(&m.mac);
    }

    /// Statement-position macros (`println!(…);`) parse as `Stmt::Macro`,
    /// not `Expr::Macro` — the print family lives there, so both shapes
    /// are visited.
    fn visit_stmt_macro(&mut self, m: &'ast syn::StmtMacro) {
        syn::visit::visit_stmt_macro(self, m);
        self.try_inline(&m.mac);
        self.descend_nested_macros(&m.mac);
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
            Self::First => 0,
            Self::Second => 1,
            Self::Third => 2,
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

    /// Recurse into nested family macros one level DOWN from this
    /// macro's args. The oracle boundary (probed against real clippy
    /// 1.97, 2026-08-28) is the `format_args!` boundary — the args AFTER
    /// the template feed `format_args!`, so a family macro DIRECTLY in
    /// one of those positions is owned by `format_in_format_args` and
    /// does NOT fire uninlined (direct `Expr::Macro` children there are
    /// skipped). Family macros nested DEEPER (inside a call inside an
    /// arg — the riir-engine `cwm_runtime/refinement.rs` live shape,
    /// `truncate(&format!("{:?}", x), 120)`) are standalone `format!`
    /// calls and DO fire. Pre-template positions don't feed
    /// `format_args!` — the `assert_eq!(format!("{}", e), ..)` comparand
    /// fires even as a direct child (the `eql_symbolic` live shape), so
    /// no skip applies there. Visiting parsed args with `self` reaches
    /// nested macros at any depth, applying the same rule at each level.
    fn descend_nested_macros(&mut self, mac: &Macro) {
        let Some(pos) = macro_family(&mac.path) else {
            return;
        };
        let toks: Vec<TokenTree> = mac.tokens.clone().into_iter().collect();
        let Some(args) = split_args(&toks) else {
            return;
        };
        for (i, arg) in args.iter().enumerate() {
            let stream: TokenStream = arg.toks.iter().cloned().collect();
            let Ok(expr) = syn::parse2::<Expr>(stream) else {
                continue;
            };
            if i > pos.index() && matches!(expr, Expr::Macro(_)) {
                continue; // direct post-template child — format_args! boundary
            }
            self.visit_expr(&expr);
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

    // Everything after the template is a format arg. A named arg `n = n`
    // (BOTH sides the same plain ident) is REDUNDANT under implicit
    // capture: real clippy's MachineApplicable fix deletes the arg and
    // leaves the template untouched (probe-verified clippy 1.95 on the
    // live riir-data sites). A named-arg-with-RENAME `n = v` is INLINED
    // into the template (P3, probe-verified clippy 1.97 — see the module
    // header). A rename value that is not a plain ident aborts.
    let mut fmt_args = Vec::new();
    let mut redundant_named: Vec<(usize, usize)> = Vec::new();
    let mut renames: Vec<RenameArg> = Vec::new();
    for arg in &args[pos.index() + 1..] {
        let stream: TokenStream = arg.toks.iter().cloned().collect();
        let expr = syn::parse2::<Expr>(stream).ok()?;
        let ident = match &expr {
            Expr::Path(p) => p.path.get_ident()?.to_string(),
            Expr::Assign(a) => {
                let (Expr::Path(l), Expr::Path(r)) = (&*a.left, &*a.right) else {
                    return None; // named arg with a complex value — abort
                };
                let (Some(l), Some(r)) = (l.path.get_ident(), r.path.get_ident()) else {
                    return None;
                };
                let (l, r) = (l.to_string(), r.to_string());
                if l == r {
                    redundant_named.push((arg.comma_start, arg.end));
                } else {
                    // `{r#x}` is not a valid placeholder name on either side.
                    if l.contains('#') || r.contains('#') {
                        return None;
                    }
                    renames.push(RenameArg {
                        key: l,
                        val: r,
                        comma_start: arg.comma_start,
                        end: arg.end,
                    });
                }
                continue;
            }
            _ => return None, // field access / call / cast / literal … — no partial inlining
        };
        // `{r#raw}` is not a valid format-arg name — skip. (`{self}` IS
        // valid: rustc explicitly allows `self` as an implicit capture —
        // probe-verified 2026-08-18, `format!("{self:?")}` compiles + real
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

    let mut referenced: Vec<usize> = Vec::new();
    let (new_raw, used) = build_inlined(raw, &fmt_args, &renames, &mut referenced)?;
    if used == 0 && redundant_named.is_empty() && referenced.is_empty() {
        return None; // nothing consumed — nothing to inline here
    }

    let mut edits = Vec::with_capacity(fmt_args.len());
    if used > 0 || !referenced.is_empty() {
        edits.push(SourceEdit {
            start: template_range.start,
            end: template_range.end,
            replacement: format!("\"{new_raw}\""),
        });
    }
    if used > 0 {
        for a in &fmt_args {
            edits.push(SourceEdit {
                start: a.comma_start,
                end: a.arg_end,
                replacement: String::new(),
            });
        }
    }
    for &i in &referenced {
        let r = &renames[i];
        edits.push(SourceEdit {
            start: r.comma_start,
            end: r.end,
            replacement: String::new(),
        });
    }
    for (start, end) in redundant_named {
        edits.push(SourceEdit { start, end, replacement: String::new() });
    }
    Some(edits)
}

/// A named-arg-with-rename: the template names it `key`, the value ident
/// is `val`. The rewrite inlines `val` at every `key` occurrence (P3).
struct RenameArg {
    key: String,
    val: String,
    comma_start: usize,
    end: usize,
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

/// EVERY argument of a known format-family macro as expression tokens —
/// pre-template operands INCLUDED (`assert!`/`write!` conditions,
/// `assert_eq!` comparands). Shared with sibling proposers whose lint
/// fires real clippy in those positions (probe 2026-08-28: `match_bool`
/// fires on `assert!(match b { .. })` — the assert operand — as well as
/// post-template args). The template slice itself fails to parse as an
/// `Expr` and is skipped by the caller's `ok()` filter.
pub(super) fn all_expr_args(mac: &Macro) -> Option<Vec<ExprMacroArg>> {
    macro_family(&mac.path)?;
    let toks: Vec<TokenTree> = mac.tokens.clone().into_iter().collect();
    let args = split_args(&toks)?;
    Some(
        args.into_iter()
            .map(|a| ExprMacroArg { toks: a.toks })
            .collect(),
    )
}

/// Build the inlined RAW literal text + the number of args consumed.
/// `None` aborts: an uninlinable placeholder (explicit index, positional
/// `$` ref, malformed content), malformed braces, or an arg-count
/// mismatch. Renames: every placeholder named `key` becomes `val`, and
/// every width/precision `$` ref to a `key` inside a spec becomes `val`;
/// each applied rename's index is pushed to `referenced` (only referenced
/// renames get their arg deleted).
fn build_inlined(
    raw: &str,
    args: &[ArgSpan],
    renames: &[RenameArg],
    referenced: &mut Vec<usize>,
) -> Option<(String, usize)> {
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
                        // same trait bounds, same formatting — except that
                        // width/precision `$` refs to a RENAME key are
                        // re-pointed at the value ident (P3; clippy 1.97
                        // rewrites `{:>w$}` + `w = width` → `{:>width$}`).
                        if has_dynamic_spec_ref(spec) {
                            return None; // `{:1$}` / `{:.*}` — the spec itself consumes/references another positional arg
                        }
                        let spec = rewrite_spec_refs(spec, renames, referenced);
                        let arg = args.get(next_arg)?;
                        next_arg += 1;
                        out.push('{');
                        out.push_str(&arg.name);
                        out.push(':');
                        out.push_str(&spec);
                        out.push('}');
                    }
                    Placeholder::AlreadyNamed => {
                        // `{name}` / `{name:spec}` — copies verbatim and
                        // consumes nothing (its binding is not one of the
                        // positional args), UNLESS `name` is a rename key:
                        // then the name (and any `$` refs in its spec) are
                        // re-pointed at the rename's value ident (P3).
                        let rewritten = rewrite_named_content(&content, renames, referenced);
                        out.push('{');
                        out.push_str(&rewritten);
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

/// Rebuild an `{name}` / `{name:spec}` placeholder's content: the name
/// maps through a rename key when one matches, and the spec's `$` refs
/// re-point likewise. Everything else copies verbatim.
fn rewrite_named_content(
    content: &str,
    renames: &[RenameArg],
    referenced: &mut Vec<usize>,
) -> String {
    let (name_part, spec_part) = match content.find(':') {
        Some(i) => (&content[..i], Some(&content[i + 1..])),
        None => (content, None),
    };
    let mut new_name: &str = name_part;
    for (i, r) in renames.iter().enumerate() {
        if r.key == name_part {
            new_name = &r.val;
            if !referenced.contains(&i) {
                referenced.push(i);
            }
            break;
        }
    }
    let mut out = String::new();
    out.push_str(new_name);
    if let Some(spec) = spec_part {
        let spec = rewrite_spec_refs(spec, renames, referenced);
        out.push(':');
        out.push_str(&spec);
    }
    out
}

/// Re-point every width/precision `ident$` ref inside a spec whose
/// `ident` is a rename key at that rename's value ident (P3). The ident
/// run is matched EXACTLY (walk back from `$` over ident-constituent
/// chars; `{:>w$}` matches `w`, not `aw`), so `{a2}` never maps a rename
/// keyed `a`. Positional digit refs are unreachable here —
/// `has_dynamic_spec_ref` aborted them first.
fn rewrite_spec_refs(
    spec: &str,
    renames: &[RenameArg],
    referenced: &mut Vec<usize>,
) -> String {
    if renames.is_empty() {
        return spec.to_string();
    }
    let chars: Vec<char> = spec.chars().collect();
    // Pass 1 — locate every `ident$` ref whose ident is a rename key.
    // Ranges are non-overlapping (format specs separate width/precision
    // refs with `.` or other non-ident chars).
    let mut repls: Vec<(usize, usize, &str)> = Vec::new();
    for (i, &c) in chars.iter().enumerate() {
        if c != '$' {
            continue;
        }
        let mut j = i;
        while j > 0 && (chars[j - 1].is_ascii_alphanumeric() || chars[j - 1] == '_') {
            j -= 1;
        }
        let ident: String = chars[j..i].iter().collect();
        if ident.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            for (ri, r) in renames.iter().enumerate() {
                if r.key == ident {
                    repls.push((j, i, &r.val));
                    if !referenced.contains(&ri) {
                        referenced.push(ri);
                    }
                    break;
                }
            }
        }
    }
    if repls.is_empty() {
        return spec.to_string();
    }
    // Pass 2 — splice the value idents in place of the matched runs (the
    // `$` itself survives: `w$` → `width$`).
    let mut out = String::with_capacity(spec.len());
    let mut k = 0usize;
    for (j, i, val) in repls {
        while k < j {
            out.push(chars[k]);
            k += 1;
        }
        out.push_str(val);
        out.push('$');
        k = i + 1; // past the run and the original `$`
    }
    while k < chars.len() {
        out.push(chars[k]);
        k += 1;
    }
    out
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
    fn redundant_identical_named_arg_deleted() {
        // `n = n` (BOTH sides the same plain ident) is redundant under
        // implicit capture — clippy's MachineApplicable fix deletes the arg
        // and leaves the template untouched (probe-verified clippy 1.95 on
        // the live riir-data `compression = compression` sites).
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(c: f64) { eprintln!(\"ratio {c:.6}\", c = c); }",
        )
        .expect("pattern matched");
        assert!(
            out.contains("eprintln!(\"ratio {c:.6}\");"),
            "redundant named arg not deleted: {out}"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn redundant_named_arg_composes_with_positional_inline() {
        // Mixed shape: a positional `{}` inlines while the trailing `x = x`
        // deletes — both edits in one invocation (named args are always
        // last per rustc's named-after-positional rule, so indices hold).
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(y: u8, x: u8) -> String { format!(\"{} {x}\", y, x = x) }",
        )
        .expect("pattern matched");
        assert!(
            out.contains("format!(\"{y} {x}\")"),
            "composed inline+delete wrong: {out}"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn named_arg_with_rename_inlined_into_template() {
        // P3 (probe 2026-08-29, real clippy 1.97, all MachineApplicable):
        // `a = b` renames are INLINED — the placeholder name is rewritten
        // to the value ident and the arg is deleted. Const values fire
        // exactly like locals (implicit capture resolves consts).
        let out = propose_fix(
            "uninlined_format_args",
            "fn f() { println!(\"{iters}\", iters = ITERS); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f() { println!(\"{ITERS}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_with_format_spec_carries_spec() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(g: f32) { println!(\"{spread:.3}\", spread = g); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(g: f32) { println!(\"{g:.3}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_rewrites_positional_spec_dollar_ref() {
        // The width ref `width$` on a POSITIONAL placeholder re-points at
        // the rename's value ident; the positional arg itself is untouched.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8, w: usize) -> String { format!(\"{:>width$}\", x, width = w) }",
        )
        .expect("pattern matched");
        assert_eq!(
            out,
            "fn f(x: u8, w: usize) -> String { format!(\"{x:>w$}\") }"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_rewrites_named_placeholder_and_its_own_spec_ref() {
        // clippy probe L46: `{a:>w$}` + `a = x, w = width` — BOTH the
        // placeholder name and the width ref are rewritten, both args
        // deleted.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u8, width: usize) { println!(\"{a:>w$}\", a = x, w = width); }",
        )
        .expect("pattern matched");
        assert_eq!(
            out,
            "fn f(x: u8, width: usize) { println!(\"{x:>width$}\"); }"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_rewrites_precision_dollar_ref() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: f64, p: usize) { println!(\"{:.p$}\", x, p = prec); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(x: f64, p: usize) { println!(\"{x:.prec$}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn multi_rename_all_rewritten() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u32, y: u32) { println!(\"{a} {b}\", a = x, b = y); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(x: u32, y: u32) { println!(\"{x} {y}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_chain_both_sides() {
        // `a = x, b = c`: each placeholder maps via its OWN key — no
        // cascade (`{a}`→`{x}`, `{b}`→`{c}`), both in scope by construction.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u32, c: u32) { println!(\"{a} {b}\", a = x, b = c); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(x: u32, c: u32) { println!(\"{x} {c}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_escaped_braces_untouched() {
        // clippy probe L26: only the REAL placeholder is rewritten; the
        // `{{a}}` escape payload survives verbatim.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(v: u32) { println!(\"{{{{a}}}} {a}\", a = v); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(v: u32) { println!(\"{{{{a}}}} {v}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_repeated_placeholder_all_occurrences() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(v: u32) { println!(\"{a}{a}\", a = v); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(v: u32) { println!(\"{v}{v}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_composes_with_positional_inline() {
        // clippy probe L38 (c3 mixed): the positional `{}` inlines AND the
        // `{n}` rename rewrites in one invocation.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(v: u32, v2: u32) { println!(\"{} {n}\", v, n = v2); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(v: u32, v2: u32) { println!(\"{v} {v2}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_exact_ident_boundary() {
        // clippy probe L42: `{a2}` is an unrelated implicit capture — only
        // the exact `a` placeholder maps.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(v: u32, a2: u32) { println!(\"{a2} {a}\", a = v); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(v: u32, a2: u32) { println!(\"{a2} {v}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_self_value_fires() {
        // clippy probe L63: `me = self` inlines to `{self}`.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f() { let s = 1; let _ = &s; println!(\"{me}\", me = s); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f() { let s = 1; let _ = &s; println!(\"{s}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_composes_with_redundant_delete() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(v: u32, w: u32) { println!(\"{v} {n}\", v = v, n = w); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(v: u32, w: u32) { println!(\"{v} {w}\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn rename_field_access_value_still_aborts() {
        // clippy probe: `val = s.0` is SILENT in real clippy (implicit
        // capture cannot express a field access) — the whole macro aborts.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(s: &S) { println!(\"{val:?}\", val = s.0); }",
        );
        assert!(out.is_none(), "field-access value must stay silent: {out:?}");
    }

    #[test]
    fn rename_raw_ident_value_aborts() {
        // `{r#type}` is not a valid placeholder name.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f() { println!(\"{a}\", a = r#type); }",
        );
        assert!(out.is_none(), "raw-ident value must abort: {out:?}");
    }

    #[test]
    fn rename_raw_ident_key_aborts() {
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(b: u32) { println!(\"{a}\", r#a = b); }",
        );
        assert!(out.is_none(), "raw-ident key must abort: {out:?}");
    }

    #[test]
    fn rename_with_literal_positional_arg_still_untouched() {
        // The rename itself is now inlinable, but the positional literal
        // `1` aborts the whole macro before any edit (no partial inlining) —
        // real clippy would fire a partial fix here; we stay conservative
        // (same policy as every literal-positional shape).
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(w: usize) { println!(\"{:<width$}\", 1, width = w); }",
        );
        assert!(out.is_none(), "literal positional must abort: {out:?}");
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

    #[test]
    fn assert_eq_operand_format_inlined() {
        // The riir-train `eql_symbolic` production shape — real clippy 1.97
        // fires uninlined_format_args on the format! in the assert_eq!
        // COMPARAND position (probed 2026-08-28); the operand descent
        // reaches it.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(e: E) { assert_eq!(format!(\"{}\", e), \"E(0)\"); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(e: E) { assert_eq!(format!(\"{e}\"), \"E(0)\"); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn call_nested_format_inlined() {
        // The riir-engine `cwm_runtime/refinement.rs` production shape —
        // a format! inside a CALL inside a macro arg. Probed: real clippy
        // 1.97 FIRES on it (the inner format! is a standalone call past
        // the format_args! boundary, unlike a direct macro-arg child).
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(x: u32, t: usize) -> String { format!(\"a={:?}\", truncate(format!(\"{:?}\", x), t)) }",
        )
        .expect("pattern matched");
        assert!(
            out.contains("truncate(format!(\"{x:?}\"), t)"),
            "call-nested format! not inlined: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn assert_eq_operand_both_comparands_reached() {
        // Either comparand position can carry the nested macro.
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(a: &str, e: E) { assert_eq!(a, format!(\"{}\", e)); }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(a: &str, e: E) { assert_eq!(a, format!(\"{e}\")); }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn print_family_post_template_macro_untouched() {
        // Probe-verified divergence: a nested format! in the print
        // family's POST-template args is owned by clippy's
        // `format_in_format_args` — real clippy does NOT fire
        // uninlined_format_args there, so the descent must NOT fix it
        // (fixing oracle-silent sites would outrun the oracle).
        let out = propose_fix(
            "uninlined_format_args",
            "fn f(k: &str, p: f64) { println!(\"entry {}\", format!(\"{}={:.4}\", k, p)); }",
        );
        assert!(out.is_none(), "print-family post-template descent must stay silent: {out:?}");
    }
}
