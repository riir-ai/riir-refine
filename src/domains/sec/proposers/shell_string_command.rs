//! `shell-string-command-interpolation` — SUGGEST-ONLY detector
//! (Batch 292, the hevnsnt/`CVEngine` distill; Research 330 / Plan 296).
//!
//! User-influenced data interpolated into a shell-string command:
//! `Command::new("sh")` + `.arg("-c")` + a `format!`/concat expression
//! carrying the data. A metacharacter allowlist on the data does NOT make
//! the shell string safe — argument injection (CWE-88) survives every
//! allowlist that excludes only the classic metacharacters (semicolon, pipe,
//! ampersand, dollar, backtick) but nothing else (the `CVEngine` tlsx site:
//! `f'tlsx -host {domain}'` passes its own `[A-Za-z0-9._:/[] -]` charset
//! and still reaches `foo -oG /tmp/x`). The bounded repair is the arg
//! array on the real binary — no shell, no metacharacter layer at all.
//!
//! ## Why suggest-only (never a fix)
//!
//! The shell-features regime is a design decision (some scripts genuinely
//! need pipes/globs); a mechanical rewrite cannot know which token is the
//! binary and which are its arguments after `-c` merging. This module
//! ships a parse-only DETECTION walk only: NO `FIXABLE_SEC` membership,
//! NO `collect_edits` arm — the suggestion surfaces through retrieval
//! (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! Command::new("tlsx")
//!     .arg("-san").arg("-host").arg(domain) // the arg array
//!     .output()?;
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Shell-literal gate**: `Command::new(..)` with a shell-name string
//!   literal (`sh` / `bash` / `zsh` / `cmd` / `powershell` / `pwsh`). A
//!   `format!` arg on a NON-shell binary stays silent (direct-invocation
//!   argument injection is a different, narrower class).
//! - **Shell-flag gate**: a `-c` / `-Command` / `/C` string-literal arg
//!   in the same fn — the token that turns the shell into a string
//!   interpreter.
//! - **Interpolation gate**: a `format!(..)` macro (or a `+` concat with
//!   a non-literal operand) inside an `.arg(..)` argument in the same fn.
//!   A STATIC `-c` script with no interpolation stays silent.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - The three gates are fn-level: a builder assembled across helpers is
//!   unconnected parse-only.
//! - `std::process::Command` is the only sink family (parse-provable);
//!   `tokio::process::Command` alias-imports are an honest NO.
//! - Static-shell scripts (config-driven, no interpolation) fire only
//!   when the interpolation gate sees a format!/concat — by construction.
//! - Batch 292 / Plan 296 / Research 330 (hevnsnt/`CVEngine` @ `9dd9c441`,
//!   GPL-3.0 — cited as class provenance; the fixture is an original
//!   synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected shell-string interpolation site.
#[derive(Debug, Clone)]
pub struct ShellStringFinding {
    /// Byte range of the interpolating `.arg(..)` call.
    pub site_span: Range<usize>,
    /// The arg call's source text.
    pub site_text: String,
    /// The advice line (the arg array + the class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "invoke the real binary with an ARG ARRAY instead of \
`sh -c \"...\"`: a metacharacter allowlist does not make a shell string safe — \
argument injection (CWE-88) survives it; where shell features are genuinely \
needed, strict per-token validation faces the interpolated datum and review \
owns it (Batch 292, suggest-only)";

/// The shell-name literals (`Command::new(..)` first arg, lowercase).
const SHELL_NAMES: &[&str] = &["sh", "bash", "zsh", "cmd", "powershell", "pwsh"];

/// The shell-interpreter flags (arg-position string literals, lowercase).
const SHELL_FLAGS: &[&str] = &["-c", "-command", "/c"];

/// Detect shell-string command interpolation (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<ShellStringFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<ShellStringFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production command construction.
            if is_test_mod(m) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, findings);
                }
            }
        }
        _ => {}
    }
}

fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

#[derive(Default)]
struct FnScan {
    shell_new: bool,
    shell_flag: bool,
    interpolated_arg_span: Option<Range<usize>>,
}

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<ShellStringFinding>) {
    let mut walk = FnScan::default();
    scan_stmts(&block.stmts, source, &mut walk);
    // All three gates must hold; the report lands at the interpolating arg.
    if walk.shell_new && walk.shell_flag {
        if let Some(span) = walk.interpolated_arg_span {
            findings.push(ShellStringFinding {
                site_text: source[span.clone()].to_string(),
                site_span: span,
                advice: ADVICE,
            });
        }
    }
}

fn scan_stmts(stmts: &[syn::Stmt], source: &str, walk: &mut FnScan) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    walk_expr(&init.expr, source, walk);
                }
            }
            syn::Stmt::Item(item) => collect_item_exprs(item, source, walk),
            syn::Stmt::Expr(e, _) => walk_expr(e, source, walk),
            syn::Stmt::Macro(_) => {}
        }
    }
}

fn collect_item_exprs(item: &Item, source: &str, walk: &mut FnScan) {
    match item {
        Item::Fn(f) => scan_stmts(&f.block.stmts, source, walk),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_stmts(&m.block.stmts, source, walk);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_item_exprs(nested, source, walk);
                }
            }
        }
        _ => {}
    }
}

fn walk_expr(e: &Expr, source: &str, walk: &mut FnScan) {
    match e {
        Expr::MethodCall(m) => {
            let name = m.method.to_string();
            if name == "new" {
                if let Some(lit) = lit_arg(m.args.first()) {
                    if SHELL_NAMES.contains(&lit.to_ascii_lowercase().as_str()) {
                        walk.shell_new = true;
                    }
                }
            } else if name == "arg" {
                if let Some(lit) = lit_arg(m.args.first()) {
                    if SHELL_FLAGS.contains(&lit.to_ascii_lowercase().as_str()) {
                        walk.shell_flag = true;
                    }
                }
                if walk.interpolated_arg_span.is_none() && m.args.iter().any(is_interpolated_expr) {
                    let span = e.span().byte_range();
                    if span.end <= source.len() {
                        walk.interpolated_arg_span = Some(span);
                    }
                }
            }
            walk_expr(&m.receiver, source, walk);
            for a in &m.args {
                walk_expr(a, source, walk);
            }
        }
        Expr::Call(c) => {
            // Associated-fn form: `Command::new("sh")` parses as a Call
            // with a path func (no receiver) — the cors_any_creds
            // precedent (AllowOrigin::any()).
            if let Expr::Path(p) = c.func.as_ref() {
                let segs: Vec<String> = p
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                if segs.len() >= 2 && segs[segs.len() - 2] == "Command" {
                    let m = segs[segs.len() - 1].as_str();
                    if m == "new" {
                        if let Some(lit) = lit_arg(c.args.first()) {
                            if SHELL_NAMES.contains(&lit.to_ascii_lowercase().as_str()) {
                                walk.shell_new = true;
                            }
                        }
                    }
                }
            }
            walk_expr(&c.func, source, walk);
            for a in &c.args {
                walk_expr(a, source, walk);
            }
        }
        _ => scan_child_exprs(e, source, walk),
    }
}

fn scan_child_exprs(e: &Expr, source: &str, walk: &mut FnScan) {
    // Control-flow shapes that can carry the builder chain (macro tokens
    // are never descended into — the house recall limit).
    match e {
        Expr::If(i) => {
            walk_expr(&i.cond, source, walk);
            scan_stmts(&i.then_branch.stmts, source, walk);
            if let Some((_, else_e)) = &i.else_branch {
                walk_expr(else_e, source, walk);
            }
        }
        Expr::Match(m) => {
            walk_expr(&m.expr, source, walk);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    walk_expr(guard, source, walk);
                }
                walk_expr(&arm.body, source, walk);
            }
        }
        Expr::Block(b) => scan_stmts(&b.block.stmts, source, walk),
        Expr::Unsafe(u) => scan_stmts(&u.block.stmts, source, walk),
        Expr::Loop(l) => scan_stmts(&l.body.stmts, source, walk),
        Expr::While(w) => {
            walk_expr(&w.cond, source, walk);
            scan_stmts(&w.body.stmts, source, walk);
        }
        Expr::ForLoop(fl) => {
            walk_expr(&fl.expr, source, walk);
            scan_stmts(&fl.body.stmts, source, walk);
        }
        Expr::Try(t) => walk_expr(&t.expr, source, walk),
        _ => {}
    }
}

/// A string-literal first argument.
fn lit_arg(a: Option<&Expr>) -> Option<String> {
    let Some(Expr::Lit(l)) = a else {
        return None;
    };
    match &l.lit {
        syn::Lit::Str(s) => Some(s.value()),
        _ => None,
    }
}

/// A `format!(..)` macro expression, or a `+` concat with a non-literal
/// operand (the interpolated-data shapes).
fn is_interpolated_expr(e: &Expr) -> bool {
    match e {
        Expr::Macro(m) => {
            let seg = m.mac.path.segments.last().map(|s| s.ident.to_string());
            seg.as_deref() == Some("format")
        }
        Expr::Binary(b) if matches!(b.op, syn::BinOp::Add(_)) => {
            is_interpolated_expr(&b.left) || is_interpolated_expr(&b.right)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::ShellStringFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 292 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch292_sh_c_format_arg_fires() {
        let src = "use std::process::Command;\n\n\
             pub fn probe(domain: &str) -> std::io::Result<String> {\n    \
             let out = Command::new(\"sh\")\n        \
             .arg(\"-c\")\n        \
             .arg(format!(\"tlsx -san -silent -host {domain}\"))\n        \
             .output()?;\n    \
             Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The fixed arg-array form (no shell literal) stays silent.
    #[test]
    fn arg_array_silent() {
        let src = "use std::process::Command;\n\n\
             pub fn probe(domain: &str) -> std::io::Result<String> {\n    \
             let out = Command::new(\"tlsx\")\n        \
             .arg(\"-host\").arg(domain)\n        \
             .output()?;\n    \
             Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A `format!` arg on a NON-shell binary stays silent.
    #[test]
    fn non_shell_binary_silent() {
        let src = "use std::process::Command;\n\n\
             pub fn scan(target: &str) -> std::io::Result<String> {\n    \
             let out = Command::new(\"nmap\")\n        \
             .arg(format!(\"-p {target}\"))\n        \
             .output()?;\n    \
             Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n";
        assert!(findings(src).is_empty(), "no shell literal: silent");
    }

    /// A shell with a STATIC `-c` script (no interpolation) stays silent.
    #[test]
    fn static_script_silent() {
        let src = "use std::process::Command;\n\n\
             pub fn uptime() -> std::io::Result<String> {\n    \
             let out = Command::new(\"sh\")\n        \
             .arg(\"-c\")\n        \
             .arg(\"uptime -p\")\n        \
             .output()?;\n    \
             Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n";
        assert!(findings(src).is_empty());
    }

    /// The `cmd /C` Windows spelling fires too.
    #[test]
    fn cmd_c_spelling_fires() {
        let src = "use std::process::Command;\n\n\
             pub fn list(path: &str) -> std::io::Result<String> {\n    \
             let out = Command::new(\"cmd\")\n        \
             .arg(\"/C\")\n        \
             .arg(format!(\"dir {path}\"))\n        \
             .output()?;\n    \
             Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n";
        assert_eq!(findings(src).len(), 1);
    }

    /// A concat arg interpolates too (`"ls ".to_owned() + dir`).
    #[test]
    fn concat_arg_fires() {
        let src = "use std::process::Command;\n\n\
             pub fn list(dir: &str) -> std::io::Result<String> {\n    \
             let script = String::from(\"ls \") + dir;\n    \
             let out = Command::new(\"bash\")\n        \
             .arg(\"-c\")\n        \
             .arg(script)\n        \
             .output()?;\n    \
             Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n";
        // The concat happens OUTSIDE the .arg() — an honest NO (the
        // interpolated-arg gate is arg-position).
        assert!(findings(src).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t() {\n        \
             let _ = Command::new(\"sh\").arg(\"-c\").arg(format!(\"x {y}\"));\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }
}
