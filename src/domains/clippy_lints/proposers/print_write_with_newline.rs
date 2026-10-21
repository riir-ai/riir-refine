//! `clippy::print_with_newline` / `clippy::write_with_newline` —
//! `print!("x\n")` → `println!("x")`, `write!(f, "x\n")` → `writeln!(f, "x")`
//! (Batch 150; Bench 094 default-level evidence: `write_with_newline` 248
//! firings — 1-crate concentration caveat — and the whole print/newline
//! family is uncovered despite clippy-MachineApplicable suggestions).
//!
//! ## Probe matrix (real clippy 0.1.98, JSON applicability)
//!
//! FIRES — every plain string literal ending in the `\n` escape:
//! `print!("x\n")`, `eprint!("err\n")`, `write!(f, "x\n")`, with or without
//! trailing format args (`print!("{}\n", x)`, `print!("value {x}\n")`),
//! and the full-literal form `print!("\n")` → `println!()` (the empty
//! literal is DELETED, matching the cascade terminal — the produced
//! `println!("")` would re-fire `println_empty_string` on the next pass).
//! SILENT — `println!`/`eprintln!`/`writeln!` (nothing to suggest: the
//! macro already appends the newline), interior `\n` (`"a\nb"`), raw
//! strings (`r"raw\nliteral"` — backslash-n chars, not a newline), and
//! `"\\n"` endings (escaped backslash: the VALUE ends with `\` + `n`
//! chars, not a newline).
//!
//! Suggestion shape: TWO `MachineApplicable` children — the macro-name span
//! and the `\n` inside the literal (or the whole literal token when it
//! becomes empty). Both applied here as one atomic edit pair.
//!
//! Divergences (all skip-direction): concatenated literals
//! (`print!("a" "b\n")` — the first literal does not end in `\n`), raw
//! strings carrying a REAL trailing newline, `\x0a`/`\u{a}` encodings of
//! the trailing newline (real clippy fires on the value; the rewrite here
//! only recognizes the `\n` escape), and macro bodies (syn does not
//! descend — the crate-wide standard).

use super::SourceEdit;
use proc_macro2::{TokenStream, TokenTree};
use syn::visit::Visit;
use syn::{ExprMacro, File, LitStr, Macro, StmtMacro};

/// `print!`/`eprint!` → `println!`/`eprintln!` (`print_with_newline`).
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    collect_family(source, file, Family::Print)
}

/// `write!(f, ..)` → `writeln!(f, ..)` (`write_with_newline`).
pub(crate) fn collect_writeln_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    collect_family(source, file, Family::Write)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Print,
    Write,
}

impl Family {
    fn fires_on(self, name: &str) -> bool {
        match self {
            Self::Print => matches!(name, "print" | "eprint"),
            Self::Write => name == "write",
        }
    }

    fn rename(self, name: &str) -> &'static str {
        match self {
            Self::Print if name == "print" => "println",
            Self::Print => "eprintln",
            Self::Write => "writeln",
        }
    }

    /// Template index: FIRST for the print family, SECOND for `write!`
    /// (the destination expression is first — the `uninlined_format_args`
    /// probe matrix, same family split).
    fn template_index(self) -> usize {
        match self {
            Self::Print => 0,
            Self::Write => 1,
        }
    }
}

fn collect_family(_source: &str, file: &File, family: Family) -> Vec<SourceEdit> {
    let mut v = Collector {
        family,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector {
    family: Family,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        syn::visit::visit_expr_macro(self, m);
        self.try_rewrite(&m.mac);
    }

    /// Statement-position macros (`print!(…);`) parse as `Stmt::Macro`,
    /// not `Expr::Macro` — the print family lives there.
    fn visit_stmt_macro(&mut self, m: &'ast StmtMacro) {
        syn::visit::visit_stmt_macro(self, m);
        self.try_rewrite(&m.mac);
    }
}

impl Collector {
    fn try_rewrite(&mut self, mac: &Macro) {
        let Some(name) = mac.path.get_ident().map(syn::Ident::to_string) else {
            return; // qualified invocations (`std::print!`) — skip
        };
        if !self.family.fires_on(&name) {
            return;
        }
        let Some(rewrite) = rewrite_macro(&mac.tokens, self.family.template_index()) else {
            return;
        };
        // Macro-name edit: the single path segment's own span.
        let ident = mac
            .path
            .segments
            .last()
            .expect("single segment")
            .ident
            .clone();
        let r = ident.span().byte_range();
        self.edits.push(SourceEdit {
            start: r.start,
            end: r.end,
            replacement: self.family.rename(&name).to_string(),
        });
        self.edits.push(rewrite);
    }
}

/// Locate the template literal and emit the `\n`-strip edit (or the
/// whole-token deletion when the literal becomes empty). `None` = the
/// macro is not ours (no literal template, raw string, interior `\n` only).
fn rewrite_macro(tokens: &TokenStream, template_index: usize) -> Option<SourceEdit> {
    let toks: Vec<TokenTree> = tokens.clone().into_iter().collect();
    let args = split_args(&toks)?;
    let template = args.get(template_index)?;

    // Plain `"`-opening string literal only (raw/byte/c-strings and any
    // multi-token template abort).
    if template.toks.len() != 1 {
        return None;
    }
    let text = match &template.toks[0] {
        TokenTree::Literal(l) if l.to_string().starts_with('"') => l.to_string(),
        _ => return None,
    };
    syn::parse_str::<LitStr>(&text).ok()?;
    let range = template.toks[0].span().byte_range();

    // The raw text between the quotes must end in the `\n` ESCAPE — not
    // an escaped backslash followed by `n` (`"\\n"` ends with `\` + `n`
    // chars, whose value does not end in a newline; real clippy is
    // silent there and so are we).
    let raw = &text[1..text.len() - 1];
    if !raw.ends_with("\\n") || raw.ends_with("\\\\n") {
        return None;
    }
    let stripped = &raw[..raw.len() - 2];
    if stripped.is_empty() {
        // `print!("\n")` → `println!()` / `write!(f, "\n")` → `writeln!(f)`
        // — delete the literal token so the cascade terminal form is
        // produced directly. In the write family the literal's PRECEDING
        // comma goes with it (no dangling trailing comma).
        let start = if template_index > 0 {
            template.comma_start.unwrap_or(range.start)
        } else {
            range.start
        };
        Some(SourceEdit {
            start,
            end: range.end,
            replacement: String::new(),
        })
    } else {
        Some(SourceEdit {
            start: range.start,
            end: range.end,
            replacement: format!("\"{stripped}\""),
        })
    }
}

/// Split a macro token stream into top-level comma-separated arguments,
/// recording each argument's preceding comma's start byte (the
/// `uninlined_format_args` walk — `Group`s hide inner commas, making the
/// linear top-level scan sound).
struct MacroArg {
    toks: Vec<TokenTree>,
    /// Start byte of the comma separating this arg from the previous one
    /// (`None` for the first argument).
    comma_start: Option<usize>,
}

fn split_args(toks: &[TokenTree]) -> Option<Vec<MacroArg>> {
    if toks.is_empty() {
        return None;
    }
    let mut args = Vec::new();
    let mut current: Vec<TokenTree> = Vec::new();
    let mut pending_comma: Option<usize> = None;
    for t in toks {
        match t {
            TokenTree::Punct(p) if p.as_char() == ',' => {
                args.push(MacroArg {
                    toks: std::mem::take(&mut current),
                    comma_start: pending_comma,
                });
                pending_comma = Some(p.span().byte_range().start);
            }
            other => current.push(other.clone()),
        }
    }
    args.push(MacroArg {
        toks: current,
        comma_start: pending_comma,
    });
    Some(args)
}

#[cfg(test)]
mod tests {
    use super::super::FIXABLE_LINTS;

    fn fix(lint: &str, code: &str) -> Option<String> {
        crate::domains::clippy_lints::proposers::propose_fix(lint, code)
    }

    #[test]
    fn fires_on_every_probe_firing_shape() {
        // (buggy, fixed) — every FIRES row of the probe matrix.
        for (buggy, fixed) in [
            (
                "fn f() { print!(\"ready\\n\"); }",
                "fn f() { println!(\"ready\"); }",
            ),
            (
                "fn f() { eprint!(\"err\\n\"); }",
                "fn f() { eprintln!(\"err\"); }",
            ),
            ("fn f() { print!(\"\\n\"); }", "fn f() { println!(); }"),
            (
                "fn f() { print!(\"{} items\\n\", 3); }",
                "fn f() { println!(\"{} items\", 3); }",
            ),
            (
                "fn f(x: u32) { print!(\"value {x}\\n\"); }",
                "fn f(x: u32) { println!(\"value {x}\"); }",
            ),
            (
                "fn f(f: &mut String) { write!(f, \"line\\n\"); }",
                "fn f(f: &mut String) { writeln!(f, \"line\"); }",
            ),
            (
                "fn f(f: &mut String) { write!(f, \"\\n\"); }",
                "fn f(f: &mut String) { writeln!(f); }",
            ),
            (
                "fn f(f: &mut String, x: u32) { write!(f, \"{}\\n\", x); }",
                "fn f(f: &mut String, x: u32) { writeln!(f, \"{}\", x); }",
            ),
        ] {
            let out = fix("print_with_newline", buggy).or_else(|| fix("write_with_newline", buggy));
            assert_eq!(out.as_deref(), Some(fixed), "case: {buggy}");
        }
    }

    #[test]
    fn silent_on_every_probe_silent_shape() {
        for code in [
            "fn f() { println!(\"double newline intended\\n\"); }",
            "fn f() { eprintln!(\"already\\n\"); }",
            "fn f() { print!(\"interior\\nnewline\"); }",
            "fn f() { print!(r\"raw\\nliteral\"); }",
            "fn f() { print!(\"escaped backslash n\\\\n\"); }",
            "fn f() { print!(\"plain\"); }",
            "fn f(f: &mut String) { writeln!(f, \"already\"); }",
            "fn f(f: &mut String) { write!(f, \"plain\"); }",
        ] {
            let out = fix("print_with_newline", code).or_else(|| fix("write_with_newline", code));
            assert_eq!(out, None, "must stay silent: {code}");
        }
    }

    #[test]
    fn both_lint_names_in_the_fixable_table() {
        assert!(FIXABLE_LINTS.contains(&"print_with_newline"));
        assert!(FIXABLE_LINTS.contains(&"write_with_newline"));
    }

    #[test]
    fn edits_are_span_disjoint_and_apply() {
        let code = "fn f() { eprint!(\"a\\nb\\n\"); }";
        let out = fix("print_with_newline", code).expect("fires");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
        assert_eq!(out, "fn f() { eprintln!(\"a\\nb\"); }");
    }
}
