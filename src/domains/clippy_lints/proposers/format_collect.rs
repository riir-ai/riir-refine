//! `clippy::format_collect` — `it.map(|x| format!(..)).collect::<String>()`
//! → `it.fold(String::new(), |mut out, x| { let _ = write!(out, ..); out })`
//! plus a `use std::fmt::Write as _;` insertion when the trait is not
//! already in scope (P11 post-mining intake, 2026-09-13).
//!
//! INSERTION SCOPE (P35 post-mining intake, 2026-09-21 — the cfg(test)
//! incident class): fold sites confined to nested `mod`s get their import
//! at the INNERMOST enclosing mod's top, one per mod — a FILE-level import
//! whose only usage lives under `#[cfg(test)]` is an `unused_imports`
//! warning in every non-test compilation, and the `--verify` gate (which
//! runs `--all-targets`) is green on it by construction. Mixed shapes
//! (any top-level site present) keep the single file-level import: the
//! top-level site uses it in every configuration.
//!
//! The FIRST `clippy_lints` rule whose fix is TWO spans (the fold rewrite +
//! the import insertion) — mechanically the pipeline already supports it:
//! `SourceEdit` insertions are start==end spans (the `fxhashmap` use-append
//! precedent), and the `fix_verify` loop reverts per-edit with the import's
//! `restore` being the empty string (a clean line deletion).
//!
//! Probe-mapped trigger surface (rustc/clippy 1.98.1, 2026-09-13, explicit
//! `-W clippy::format_collect` — the oracle's query mode):
//! - fires on `.map(<closure>).collect::<String>()` where the closure body
//!   is a bare `format!(..)` call — ANY single closure input pattern rides
//!   verbatim (plain idents, `mut` idents, tuple destructures — probed on
//!   the zip/tuple shape);
//! - real clippy is type-aware and ALSO fires on the let-annotated
//!   `.collect()` form and on block-bodied closures whose tail is a
//!   `format!` — both stay SILENT here (documented divergences: parse-only
//!   cannot see the collect target type; the block-body template would
//!   need statement-prefix movement);
//! - `collect::<Vec<String>>()` and non-`format!` bodies (`.to_string()`)
//!   are silent in real clippy too (probed) — matched here.
//!
//! Why the ANONYMOUS import form (`as _`): a named `use std::fmt::Write`
//! can collide with an existing `Write` name (E0252) — `as _` introduces
//! no name. Method ambiguity is impossible for this rewrite: the `write!`
//! receiver is ALWAYS the `String` accumulator, and `String` implements
//! `io::Write` nowhere in std (probed 2026-09-13: `write_all` on `String`
//! is E0599) — only `fmt::Write` can resolve, whatever else is imported.
//! The insertion is SKIPPED when `std::fmt::Write` (any form, any alias,
//! incl. the `std::fmt::*` glob) is already imported at file level (covers
//! every nested scope) or — for a mod-scoped insertion — at that mod's own
//! top.
//!
//! Semantics: `fold` calls the closure once per item with a running
//! `String` accumulator; `write!` appends the same formatted text
//! `format!` produced — one allocation per item becomes zero (the lint's
//! purpose), output byte-identical.
//!
//! VERIFY-tier ([`super::VERIFY_LINTS`]): real clippy's suggestion is
//! help-only (no machine-applicable replacement — probed), so the fold
//! rewrite + import pair rides the compile gate; an import edit that
//! misbehaves reverts independently of its fold edit (per-edit revert,
//! insertion restore = "").

use super::helpers::is_postfix_safe;
use super::{edit, src, SourceEdit};
use std::collections::BTreeSet;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprMacro, ExprMethodCall, File, Item, ItemMod, UseTree};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        mods: Vec::new(),
        stack: Vec::new(),
        scopes: Vec::new(),
    };
    v.visit_file(file);
    if v.edits.is_empty() || fmt_write_in_scope(file) {
        return v.edits;
    }
    // P35 innermost-scope rule: when EVERY fold site sits inside a nested
    // mod, the import goes to each innermost mod's top (deduped, skipped
    // where the mod already imports the trait) — never the file root. Any
    // top-level site keeps the previous single file-level import (it uses
    // the import in every compilation, so the unused-import hazard cannot
    // arise).
    if v.scopes.iter().all(Option::is_some) {
        for idx in v.scopes.iter().filter_map(|s| *s).collect::<BTreeSet<_>>() {
            let ctx = &v.mods[idx];
            if ctx.has_fmt_write_use {
                continue;
            }
            v.edits.push(SourceEdit {
                start: ctx.offset,
                end: ctx.offset,
                replacement: ctx.text.clone(),
            });
        }
        return v.edits;
    }
    let offset = import_insert_offset(source, file);
    // Prepend form at the first code line (nothing above to append to);
    // append form after a use item / inner attr (newline first).
    let text = if offset == first_code_line_offset(source) {
        "use std::fmt::Write as _;\n".to_string()
    } else {
        "\nuse std::fmt::Write as _;".to_string()
    };
    v.edits.push(SourceEdit {
        start: offset,
        end: offset,
        replacement: text,
    });
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Arena of mod insertion contexts (P35), indexed by the stack.
    mods: Vec<ModImportCtx>,
    /// Innermost-mod stack as arena indices; `None` = a content-less
    /// `mod foo;` declaration (cannot host edit sites).
    stack: Vec<Option<usize>>,
    /// Innermost enclosing mod per edit, parallel to `edits`; `None` = a
    /// file-top-level site.
    scopes: Vec<Option<usize>>,
}

/// Where and how a fresh `use` line lands at one mod's top (P35).
struct ModImportCtx {
    offset: usize,
    text: String,
    has_fmt_write_use: bool,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) = self.try_fold(m) {
            self.edits.push(edit(m, replacement));
            self.scopes.push(self.stack.last().copied().flatten());
        }
    }

    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        let idx = i.content.as_ref().map(|(brace, items)| {
            let ctx = ModImportCtx {
                offset: mod_import_offset(self.source, items, brace),
                text: mod_import_text(self.source, items, brace),
                has_fmt_write_use: items
                    .iter()
                    .any(|item| matches!(item, Item::Use(u) if use_tree_imports_fmt_write(&u.tree, &mut Vec::new()))),
            };
            self.mods.push(ctx);
            self.mods.len() - 1
        });
        self.stack.push(idx);
        syn::visit::visit_item_mod(self, i);
        self.stack.pop();
    }
}

impl Collector<'_> {
    /// The fold rewrite for one `.map(<format! closure>).collect::<String>()`
    /// chain — None when the shape does not match.
    fn try_fold(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.method != "collect" || !m.args.is_empty() {
            return None;
        }
        // Turbofish must name exactly `String` (bare ident only — the
        // let-annotated `.collect()` form is the documented divergence).
        let tf = m.turbofish.as_ref()?;
        if tf.args.len() != 1 {
            return None;
        }
        let syn::GenericArgument::Type(syn::Type::Path(tp)) = &tf.args[0] else {
            return None;
        };
        let last = tp.path.segments.last()?;
        if last.ident != "String" || !matches!(last.arguments, syn::PathArguments::None) {
            return None;
        }
        // Receiver must be a plain `.map(<closure>)` call.
        let Expr::MethodCall(map) = &*m.receiver else {
            return None;
        };
        if !map.attrs.is_empty()
            || map.turbofish.is_some()
            || map.method != "map"
            || map.args.len() != 1
        {
            return None;
        }
        let Expr::Closure(c) = &map.args[0] else {
            return None;
        };
        if !c.attrs.is_empty()
            || c.movability.is_some()
            || c.capture.is_some()
            || c.asyncness.is_some()
        {
            return None;
        }
        if c.inputs.len() != 1 {
            return None;
        }
        // Body must be a BARE `format!(..)` call (block bodies are the
        // documented divergence — real clippy fires, we stay silent).
        let Expr::Macro(ExprMacro { mac, .. }) = &*c.body else {
            return None;
        };
        if !mac.path.is_ident("format") {
            return None;
        }
        let pat_text = src(self.source, &c.inputs[0])?;
        let body_text = src(self.source, &*c.body)?;
        let inner_fmt = strip_format_call(&body_text)?;
        let acc = accumulator_name(&pat_text)?;
        let recv = src(self.source, &*map.receiver)?;
        let recv = if is_postfix_safe(&map.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        Some(format!(
            "{recv}.fold(String::new(), |mut {acc}, {pat_text}| {{ let _ = write!({acc}, {inner_fmt}); {acc} }})"
        ))
    }
}

/// `format!("..", args)` source → the `"..", args` interior (the write!
/// arguments). Handles all three macro delimiter forms.
fn strip_format_call(text: &str) -> Option<&str> {
    let rest = text.trim().strip_prefix("format")?.strip_prefix('!')?;
    let rest = rest.trim_start();
    let (open, close) = match rest.chars().next()? {
        '(' => ('(', ')'),
        '[' => ('[', ']'),
        '{' => ('{', '}'),
        _ => return None,
    };
    let inner = rest.strip_prefix(open)?.strip_suffix(close)?;
    Some(inner)
}

/// Accumulator name that cannot collide with the closure's own bindings.
fn accumulator_name(pat_text: &str) -> Option<&'static str> {
    for name in ["out", "acc", "buf"] {
        // Word-boundary check over the pattern text — `out_x` must not
        // rule out `out`, but a bare/tuple-bound `out` must.
        let collides = pat_text
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|word| word == name);
        if !collides {
            return Some(name);
        }
    }
    None
}

/// Does the file already import `std::fmt::Write` (any form, any alias)
/// or a covering `std::fmt::*` glob?
fn fmt_write_in_scope(file: &File) -> bool {
    file.items.iter().any(|item| match item {
        Item::Use(u) => use_tree_imports_fmt_write(&u.tree, &mut Vec::new()),
        _ => false,
    })
}

/// Walk one use tree with its accumulated prefix segments.
fn use_tree_imports_fmt_write(tree: &UseTree, prefix: &mut Vec<String>) -> bool {
    match tree {
        UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            let hit = use_tree_imports_fmt_write(&p.tree, prefix);
            prefix.pop();
            hit
        }
        UseTree::Name(n) => prefix_is_std_fmt(prefix) && n.ident == "Write",
        UseTree::Rename(r) => prefix_is_std_fmt(prefix) && r.ident == "Write",
        UseTree::Glob(_) => {
            // `use std::fmt::*;` covers Write; `std::*` does not (it
            // imports the module, not the trait).
            prefix_is_std_fmt(prefix)
        }
        UseTree::Group(g) => g
            .items
            .iter()
            .any(|t| use_tree_imports_fmt_write(t, prefix)),
    }
}

/// Is the accumulated prefix exactly `std::fmt`?
fn prefix_is_std_fmt(prefix: &[String]) -> bool {
    prefix.len() == 2 && prefix[0] == "std" && prefix[1] == "fmt"
}

/// Byte offset just past the last top-level `use` item, if any.
fn last_use_end(file: &File) -> Option<usize> {
    file.items.iter().rev().find_map(|it| match it {
        Item::Use(u) => Some(u.span().byte_range().end),
        _ => None,
    })
}

/// Byte offset where a fresh file-level `use` line belongs: after the last
/// top-level use item; else after inner attributes; else at the first
/// non-comment, non-blank line.
fn import_insert_offset(source: &str, file: &File) -> usize {
    if let Some(end) = last_use_end(file) {
        return end;
    }
    if let Some(last_attr_end) = file.attrs.last().map(|a| a.span().byte_range().end) {
        return last_attr_end;
    }
    first_code_line_offset(source)
}

/// Byte offset where a fresh `use` line belongs at ONE mod's top: after the
/// mod's last use item; else at the first item's line start (P35). The
/// empty-mod fallback sits right after the opening brace — unreachable for
/// fold sites (an edit site implies ≥1 item) but kept total.
fn mod_import_offset(source: &str, items: &[Item], brace: &syn::token::Brace) -> usize {
    if let Some(end) = items.iter().rev().find_map(|it| match it {
        Item::Use(u) => Some(u.span().byte_range().end),
        _ => None,
    }) {
        return end;
    }
    if let Some(first) = items.first() {
        let start = first.span().byte_range().start;
        return start_of_line(source, start);
    }
    brace.span.close().byte_range().end
}

/// Insertion text for the mod-top `use` line, indentation-aware: the
/// append form after the mod's last use (same indent, newline first); the
/// prepend form above the mod's first item (indent + trailing newline).
fn mod_import_text(source: &str, items: &[Item], brace: &syn::token::Brace) -> String {
    if let Some(end) = items.iter().rev().find_map(|it| match it {
        Item::Use(u) => Some(u.span().byte_range().end),
        _ => None,
    }) {
        let indent = line_indent(source, end);
        return format!("\n{indent}use std::fmt::Write as _;");
    }
    if let Some(first) = items.first() {
        let start = first.span().byte_range().start;
        let line_start = start_of_line(source, start);
        let indent = source[line_start..start].to_string();
        return format!("{indent}use std::fmt::Write as _;\n");
    }
    let _ = brace;
    "\nuse std::fmt::Write as _;".to_string()
}

/// Byte offset of the start of the line containing `offset`.
fn start_of_line(source: &str, offset: usize) -> usize {
    source[..offset].rfind('\n').map_or(0, |i| i + 1)
}

/// Leading whitespace of the line containing `offset`.
fn line_indent(source: &str, offset: usize) -> String {
    let line_start = start_of_line(source, offset);
    source[line_start..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

/// First byte offset after leading blank lines, `//` line comments, and
/// bracket-balanced `#![..]` inner attributes — the first place a `use`
/// item may sit without landing above a file header comment.
fn first_code_line_offset(source: &str) -> usize {
    let bytes = source.as_bytes();
    let mut i = 0usize;
    loop {
        // Skip a line comment.
        if source[i..].starts_with("//") {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            if i >= bytes.len() {
                return i;
            }
            continue;
        }
        // Skip an inner attribute `#![..]` (bracket-balanced).
        if source[i..].starts_with("#![") {
            let mut depth = 0i32;
            while i < bytes.len() {
                match bytes[i] {
                    b'[' => depth += 1,
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            continue;
        }
        // Skip ONE newline (blank line), then re-check comment/attr.
        if bytes[i] == b'\n' {
            i += 1;
            continue;
        }
        if bytes[i] == b'\r' {
            i += 1;
            continue;
        }
        return i;
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const BUGGY: &str = "fn render(xs: &[u32]) -> String {\n    xs.iter().map(|x| format!(\"{}\", x)).collect::<String>()\n}";

    #[test]
    fn fires_fold_rewrite_with_import() {
        let out = propose_fix("format_collect", BUGGY).expect("matched");
        assert!(
            out.contains(".fold(String::new(), |mut out, x|"),
            "fold rewrite missing: {out}"
        );
        assert!(
            out.contains("let _ = write!(out, \"{}\", x);"),
            "write! body missing: {out}"
        );
        assert!(
            !out.contains("collect::<String>"),
            "collect chain must be gone: {out}"
        );
        assert!(
            out.starts_with("use std::fmt::Write as _;\n"),
            "anonymous import must be inserted at file head: {out}"
        );
        assert!(syn::parse_file(&out).is_ok(), "output must re-parse: {out}");
    }

    #[test]
    fn import_skipped_when_fmt_write_in_scope() {
        let input = "use std::fmt::Write as _;\n\n".to_string() + BUGGY;
        let out = propose_fix("format_collect", &input).expect("matched");
        assert_eq!(
            out.matches("use std::fmt::Write as _;").count(),
            1,
            "no second import: {out}"
        );
    }

    #[test]
    fn named_io_write_import_does_not_block_or_collide() {
        // String has no io::Write impl (probed) — the anonymous fmt import
        // neither collides (no name) nor shadows.
        let input = "use std::io::Write;\n\n".to_string() + BUGGY;
        let out = propose_fix("format_collect", &input).expect("matched");
        assert!(
            out.contains("use std::io::Write;\nuse std::fmt::Write as _;"),
            "import must land after the last use: {out}"
        );
    }

    #[test]
    fn import_lands_after_last_use() {
        let input = "use std::io::Read;\nuse std::collections::HashMap;\n\n".to_string() + BUGGY;
        let out = propose_fix("format_collect", &input).expect("matched");
        assert!(
            out.contains("use std::collections::HashMap;\nuse std::fmt::Write as _;"),
            "import must follow the LAST use: {out}"
        );
    }

    #[test]
    fn import_lands_after_inner_attrs_and_header_comments() {
        let input = "// copyright header\n#![allow(dead_code)]\n\n".to_string() + BUGGY;
        let out = propose_fix("format_collect", &input).expect("matched");
        let import_line = out
            .find("use std::fmt::Write as _;")
            .expect("import present");
        assert!(
            import_line > out.find("#![allow(dead_code)]").unwrap(),
            "import must follow inner attrs: {out}"
        );
        assert!(
            import_line > out.find("// copyright header").unwrap(),
            "import must follow the header comment: {out}"
        );
    }

    #[test]
    fn silent_on_vec_string_target() {
        let input = "fn f(xs: &[u32]) -> Vec<String> { xs.iter().map(|x| format!(\"{}\", x)).collect::<Vec<String>>() }";
        assert!(propose_fix("format_collect", input).is_none());
    }

    #[test]
    fn silent_on_non_format_body() {
        let input =
            "fn f(xs: &[u32]) -> String { xs.iter().map(|x| x.to_string()).collect::<String>() }";
        assert!(propose_fix("format_collect", input).is_none());
    }

    #[test]
    fn silent_on_let_annotated_collect() {
        // Real clippy fires (type-aware — probed); parse-only cannot see
        // the collect target — documented divergence.
        let input = "fn f(xs: &[u32]) -> String { let s: String = xs.iter().map(|x| format!(\"{}\", x)).collect(); s }";
        assert!(propose_fix("format_collect", input).is_none());
    }

    #[test]
    fn silent_on_move_closure() {
        let input = "fn f(xs: &[u32]) -> String { xs.iter().map(move |x| format!(\"{}\", x)).collect::<String>() }";
        assert!(propose_fix("format_collect", input).is_none());
    }

    #[test]
    fn silent_on_block_body_closure() {
        // Real clippy fires on block bodies whose tail is format! (probed);
        // the statement-prefix template is not shipped — divergence.
        let input = "fn f(xs: &[u32]) -> String { xs.iter().map(|x| { let y = x + 1; format!(\"{}\", y) }).collect::<String>() }";
        assert!(propose_fix("format_collect", input).is_none());
    }

    #[test]
    fn tuple_pattern_reemitted_verbatim() {
        let input = "fn f(xs: &[u32], ys: &[u32]) -> String { xs.iter().zip(ys).map(|(a, b)| format!(\"{}-{}\", a, b)).collect::<String>() }";
        let out = propose_fix("format_collect", input).expect("matched");
        assert!(
            out.contains("|mut out, (a, b)|"),
            "tuple pattern must ride verbatim: {out}"
        );
    }

    #[test]
    fn accumulator_renames_on_collision() {
        let input = "fn f(xs: &[u32]) -> String { xs.iter().map(|out| format!(\"{}\", out)).collect::<String>() }";
        let out = propose_fix("format_collect", input).expect("matched");
        assert!(
            out.contains("|mut acc, out|"),
            "collision must rename the accumulator: {out}"
        );
    }

    #[test]
    fn paren_receiver_rides_verbatim() {
        // `(a..b)` is an Expr::Paren — postfix-safe, so the source text
        // rides verbatim and grouping is preserved.
        let input = "fn f(a: u32, b: u32) -> String { (a..b).map(|x| format!(\"{}\", x)).collect::<String>() }";
        let out = propose_fix("format_collect", input).expect("matched");
        assert!(
            out.contains("(a..b).fold("),
            "range receiver keeps grouping: {out}"
        );
    }

    #[test]
    fn multiple_sites_one_import() {
        let second = BUGGY.replace("fn render", "fn render2");
        let input = format!("{BUGGY}\n{second}");
        let out = propose_fix("format_collect", &input).expect("matched");
        assert_eq!(
            out.matches("use std::fmt::Write as _;").count(),
            1,
            "exactly one import for two sites: {out}"
        );
        assert_eq!(
            out.matches(".fold(String::new()").count(),
            2,
            "both folds applied: {out}"
        );
    }

    // P35 post-mining intake (2026-09-21) — the cfg(test) incident class:
    // a file-scope import whose only usage lives under #[cfg(test)] is an
    // unused_imports warning in every non-test compilation, green through
    // the --all-targets verify gate by construction.
    const TESTMOD_BUGGY: &str = "#[cfg(test)]\nmod deploy_tests {\n    fn render(xs: &[u32]) -> String {\n        xs.iter().map(|x| format!(\"{}\", x)).collect::<String>()\n    }\n\n    #[test]\n    fn t() {\n        assert_eq!(render(&[1]), \"1\");\n    }\n}";

    #[test]
    fn import_lands_inside_enclosing_test_mod_not_file_scope() {
        let input = format!("fn helper(a: u32) -> u32 {{ a + 1 }}\n\n{TESTMOD_BUGGY}");
        let out = propose_fix("format_collect", &input).expect("matched");
        assert!(
            !out.starts_with("use std::fmt::Write as _;"),
            "import must NOT land at file scope when every site is mod-confined: {out}"
        );
        assert!(
            out.contains("mod deploy_tests {\n    use std::fmt::Write as _;"),
            "import must land at the mod top, above its first item: {out}"
        );
        assert!(
            out.matches("use std::fmt::Write as _;").count() == 1,
            "exactly one import: {out}"
        );
        assert!(syn::parse_file(&out).is_ok(), "output must re-parse: {out}");
    }

    #[test]
    fn mod_import_skipped_when_mod_already_imports_fmt_write() {
        let input = "#[cfg(test)]\nmod deploy_tests {\n    use std::fmt::Write as _;\n    fn render(xs: &[u32]) -> String {\n        xs.iter().map(|x| format!(\"{}\", x)).collect::<String>()\n    }\n}";
        let out = propose_fix("format_collect", input).expect("matched");
        assert_eq!(
            out.matches("use std::fmt::Write as _;").count(),
            1,
            "no second import inside the mod, none at file scope: {out}"
        );
    }

    #[test]
    fn mixed_top_level_and_mod_sites_keep_file_level_import() {
        // The top-level site uses the import in every compilation, so the
        // file-level form has no unused-import hazard — and one import
        // covers both sites.
        let input = format!("{BUGGY}\n\n{TESTMOD_BUGGY}");
        let out = propose_fix("format_collect", &input).expect("matched");
        assert_eq!(
            out.matches("use std::fmt::Write as _;").count(),
            1,
            "one file-level import for the mixed shape: {out}"
        );
        assert!(
            out.starts_with("use std::fmt::Write as _;\n"),
            "the single import is at file head (the top-level site uses it in every config): {out}"
        );
        assert!(
            !out.contains("    use std::fmt::Write as _;"),
            "no indented (mod-scoped) import in the mixed shape: {out}"
        );
        assert_eq!(
            out.matches(".fold(String::new()").count(),
            2,
            "both folds applied: {out}"
        );
    }

    #[test]
    fn two_mods_each_get_their_own_scoped_import() {
        let input = format!(
            "{TESTMOD_BUGGY}\n\n{}",
            TESTMOD_BUGGY
                .replace("deploy_tests", "other_tests")
                .replace("fn render", "fn render2")
        );
        let out = propose_fix("format_collect", &input).expect("matched");
        assert_eq!(
            out.matches("use std::fmt::Write as _;").count(),
            2,
            "one scoped import per mod: {out}"
        );
        assert!(
            !out.starts_with("use std::fmt::Write as _;"),
            "no file-head import: {out}"
        );
        assert_eq!(
            out.matches("\nuse std::fmt::Write as _;").count(),
            0,
            "no column-0 (file-level) import: {out}"
        );
    }

    #[test]
    fn mod_import_lands_after_the_mods_last_use() {
        let input = "#[cfg(test)]\nmod deploy_tests {\n    use std::io::Read;\n    fn render(xs: &[u32]) -> String {\n        xs.iter().map(|x| format!(\"{}\", x)).collect::<String>()\n    }\n}";
        let out = propose_fix("format_collect", input).expect("matched");
        assert!(
            out.contains("use std::io::Read;\n    use std::fmt::Write as _;"),
            "mod import must follow the mod's last use at mod indent: {out}"
        );
    }
}
