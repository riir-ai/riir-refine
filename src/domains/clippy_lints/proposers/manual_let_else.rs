//! `clippy::manual_let_else` — a let-bound two-arm match whose fallback
//! visibly diverges is better written as a `let ... else` statement (Batch
//! 90; the Plan 049 T0.1 fired histogram: ~40 real instances).
//!
//! Probe-verified firing shapes (clippy 1.97, pedantic, JSON; applicability
//! `Unspecified` — real `cargo clippy --fix` does NOT apply it):
//! `let v = match x { Some(v) => v, None => return 0 };` fires, as do the
//! `panic!("..")`, `continue`, `break`, and `Err(_) => return` fallbacks.
//!
//! Real clippy's silent shapes — each maps 1:1 onto a parse-only guard here
//! (probe-confirmed on `target/probe_b90/`):
//! - fallback arm USES its pattern bindings (`Err(e) => { eprintln!("{e}");
//!   return 0 }`) — a let-else block cannot see the arm's bindings, so the
//!   rewrite would E0425. Guard: arm-2 pattern must bind ZERO idents
//!   (`None`, `_`, `Err(_)`, `Err(..)`, literals).
//! - arm-1 body is not the bound ident (`Some(v) => v + 1`) — the let-else
//!   form has no place for the computation. Guard: arm-1 body source text
//!   must equal the bound ident.
//! - three arms / guard arms — out of scope (exactly-2-arms, no guards).
//! - non-diverging fallback (`None => 0`) — not this lint (that shape is
//!   `manual_unwrap_or`/`option_if_let_else` territory).
//!
//! Divergence is judged VISIBLY (parse-only): the fallback body is a
//! `return`/`break`/`continue` expression, or a panic-family macro
//! (`panic!`/`unreachable!`/`todo!`/`unimplemented!` — `assert!` is
//! deliberately excluded: it returns `()` on success, it does not diverge).
//! A fallback that calls a `!`-returning function diverges too, but
//! parse-only cannot prove the return type — those stay silent (the
//! `single_char_pattern` receiver-type-blind class; a false negative is
//! safe).
//!
//! The let target must be a plain ident — `mut`, type annotations,
//! attributes, and existing let-else (`diverge`) forms are skipped.
//! Emitted form: `let {pat1} = {scrutinee} else { {arm2_body} };` with all
//! three fragments byte-verbatim from the original source; the whole
//! Local statement (Stmt span includes the trailing `;`) is one atomic
//! edit. An arm-2 body that is a block has its braces stripped (the match
//! arm's block braces would double up inside `else { .. }`).
//!
//! Interior-comment guard (Issue 027/028/029 class): the splice keeps only
//! the arm-1 pattern, the scrutinee, and the arm-2 body verbatim — a
//! comment anywhere else in the Local would be deleted. Such Locals are
//! refused.

use super::helpers::{byte_range, drops_interior_comment};
use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprMacro, File, Local, Pat, Stmt};

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
    /// The default traversal reaches every statement in every block (fn
    /// bodies, loop bodies, if branches, match arms) — the
    /// `continue`-in-loop shape lives several blocks deep. Each Local edit
    /// is self-contained, so descension cannot produce overlapping edits
    /// from THIS proposer.
    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if let Stmt::Local(local) = stmt {
            if let Some(replacement) = self.try_local(local) {
                self.edits.push(edit(stmt, replacement));
            }
        }
        syn::visit::visit_stmt(self, stmt);
    }
}

impl Collector<'_> {
    fn try_local(&self, local: &Local) -> Option<String> {
        // Plain `let v = ...;` target only — `mut`, a type annotation, or
        // attributes would be lost by the rewrite; an existing let-else
        // (`diverge`) is already in the target form.
        let Pat::Ident(pi) = &local.pat else {
            return None;
        };
        if pi.mutability.is_some() || pi.by_ref.is_some() || pi.subpat.is_some() {
            return None;
        }
        if !local.attrs.is_empty() {
            return None;
        }
        let init = local.init.as_ref()?;
        if init.diverge.is_some() {
            return None; // already `let ... else`
        }
        let Expr::Match(m) = &*init.expr else {
            return None;
        };
        if m.arms.len() != 2 {
            return None;
        }
        let [a1, a2] = &m.arms[..] else {
            return None;
        };
        if !a1.attrs.is_empty() || !a2.attrs.is_empty() {
            return None;
        }
        if a1.guard.is_some() || a2.guard.is_some() {
            return None;
        }
        // Arm 1: pattern binds EXACTLY one ident, body is exactly that ident.
        let bound = pattern_bindings(&a1.pat);
        if bound.len() != 1 {
            return None;
        }
        let name = &bound[0];
        let body1 = src(self.source, &a1.body)?;
        if body1.trim() != *name {
            return None;
        }
        // Arm 2: binds nothing, diverges visibly.
        if !pattern_bindings(&a2.pat).is_empty() {
            return None;
        }
        let body2 = diverging_body_text(self.source, &a2.body)?;
        // Comment guard: kept fragments are arm-1 pattern, scrutinee, and
        // the arm-2 body — everything else is rebuilt scaffolding.
        let local_range = byte_range(local);
        let kept = [
            byte_range(&a1.pat),
            byte_range(&m.expr),
            byte_range(&a2.body),
        ];
        if drops_interior_comment(self.source, local_range, &kept) {
            return None;
        }
        let scrutinee = src(self.source, &m.expr)?;
        let pat1 = src(self.source, &a1.pat)?;
        Some(format!("let {pat1} = {scrutinee} else {body2};"))
    }
}

/// Idents bound by a pattern (conservative walk; or-patterns, tuples, and
/// tuple-structs all count their inner bindings — the callers need the
/// COUNT, not the positions). Uppercase-initial idents are EXCLUDED: in
/// pattern position the compiler treats them as constructor/const paths,
/// never bindings — and syn parses bare ctors like `None` as
/// `Pat::Ident` (the Issue 029 T3 syn-ambiguity lesson), so counting them
/// would make every `None` fallback look binding-carrying.
fn pattern_bindings(pat: &Pat) -> Vec<String> {
    struct Seeker {
        out: Vec<String>,
    }
    impl<'ast> Visit<'ast> for Seeker {
        fn visit_pat(&mut self, p: &'ast Pat) {
            if let Pat::Ident(pi) = p {
                let name = pi.ident.to_string();
                // `mut v` / `ref v` bind v; a subpattern `v @ P` binds v too
                // (the visitor descends into P separately). `_` parses as
                // Pat::Wild and never reaches here; `_x` binds and starts
                // with `_`.
                let binds = name.starts_with('_')
                    || name.chars().next().is_some_and(char::is_lowercase);
                if binds {
                    self.out.push(name);
                }
            }
            syn::visit::visit_pat(self, p);
        }
    }
    let mut s = Seeker { out: Vec::new() };
    s.visit_pat(pat);
    s.out
}

/// The `else { .. }` block text for the fallback arm — `None` when the body
/// does not VISIBLY diverge. Block bodies have their braces stripped.
fn diverging_body_text(source: &str, body: &Expr) -> Option<String> {
    let diverges = match body {
        Expr::Return(_) | Expr::Break(_) | Expr::Continue(_) => true,
        Expr::Macro(ExprMacro { mac, .. }) => {
            matches!(
                mac.path.segments.last().map(|s| s.ident.to_string()).as_deref(),
                Some("panic" | "unreachable" | "todo" | "unimplemented")
            )
        }
        Expr::Block(b) => b
            .block
            .stmts
            .last()
            .is_some_and(visibly_diverges_stmt),
        _ => false,
    };
    if !diverges {
        return None;
    }
    let text = src(source, body)?;
    // A match-arm block body `{ return 0 }` becomes `return 0` inside the
    // emitted `else { .. }` (stripping the outer braces avoids doubling).
    if let Expr::Block(_) = body {
        let inner = text.trim().strip_prefix('{')?.strip_suffix('}')?;
        return Some(format!("{{ {} }}", inner.trim()));
    }
    Some(format!("{{ {text} }}"))
}

fn visibly_diverges_stmt(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expr(e, _) => matches!(
            e,
            Expr::Return(_) | Expr::Break(_) | Expr::Continue(_)
        ) || matches!(e, Expr::Macro(ExprMacro { mac, .. }) if matches!(
            mac.path.segments.last().map(|s| s.ident.to_string()).as_deref(),
            Some("panic" | "unreachable" | "todo" | "unimplemented")
        )),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn option_return_fires() {
        let out = propose_fix(
            "manual_let_else",
            "fn f(x: Option<u32>) -> u32 { let v = match x { Some(v) => v, None => return 0 }; v + 1 }",
        )
        .expect("pattern matched");
        assert!(
            out.contains("let Some(v) = x else { return 0 };"),
            "{out}"
        );
        assert!(!out.contains("match"), "{out}");
        assert!(out.contains("v + 1 }"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn result_wildcard_fires() {
        let out = propose_fix(
            "manual_let_else",
            "fn f(r: Result<u32, String>) -> u32 { let v = match r { Ok(v) => v, Err(_) => return 0 }; v }",
        )
        .expect("pattern matched");
        assert!(out.contains("let Ok(v) = r else { return 0 };"), "{out}");
    }

    #[test]
    fn panic_fallback_fires() {
        let out = propose_fix(
            "manual_let_else",
            "fn f(x: Option<u32>) -> u32 { let v = match x { Some(v) => v, None => panic!(\"missing\") }; v }",
        )
        .expect("pattern matched");
        assert!(out.contains("else { panic!(\"missing\") };"), "{out}");
    }

    #[test]
    fn continue_in_loop_fires() {
        let out = propose_fix(
            "manual_let_else",
            "fn f(items: Vec<Option<u32>>) -> u32 { let mut a = 0; for it in items { let v = match it { Some(v) => v, None => continue }; a += v; } a }",
        )
        .expect("pattern matched");
        assert!(out.contains("let Some(v) = it else { continue };"), "{out}");
    }

    #[test]
    fn block_body_braces_stripped() {
        let out = propose_fix(
            "manual_let_else",
            "fn f(x: Option<u32>) -> u32 { let v = match x { Some(v) => v, None => { return 0 } }; v }",
        )
        .expect("pattern matched");
        assert!(out.contains("else { return 0 };"), "{out}");
        assert!(!out.contains("else { {"), "{out}");
    }

    #[test]
    fn nondiverging_fallback_silent() {
        let out = propose_fix(
            "manual_let_else",
            "fn f(x: Option<u32>) -> u32 { let v = match x { Some(v) => v, None => 0 }; v }",
        );
        assert!(out.is_none(), "value fallback must not fire: {out:?}");
    }

    #[test]
    fn binding_use_fallback_silent() {
        // `Err(e)` binds e and the body uses it — the let-else block cannot
        // see e. Real clippy skips this shape too (probe-confirmed).
        let out = propose_fix(
            "manual_let_else",
            "fn f(r: Result<u32, String>) -> u32 { let v = match r { Ok(v) => v, Err(e) => { eprintln!(\"{e}\"); return 0 } }; v }",
        );
        assert!(out.is_none(), "binding fallback must not fire: {out:?}");
    }

    #[test]
    fn body_expr_arm1_silent() {
        // `Some(v) => v + 1` — the let-else form has no slot for the `+ 1`.
        // Real clippy skips this shape too (probe-confirmed).
        let out = propose_fix(
            "manual_let_else",
            "fn f(x: Option<u32>) -> u32 { let v = match x { Some(v) => v + 1, None => return 0 }; v }",
        );
        assert!(out.is_none(), "computed arm-1 body must not fire: {out:?}");
    }

    #[test]
    fn three_arm_match_silent() {
        let out = propose_fix(
            "manual_let_else",
            "fn f(x: Result<u32, String>) -> u32 { let v = match x { Ok(v) => v, Err(e) if e.is_empty() => 0, Err(_) => return 1 }; v }",
        );
        assert!(out.is_none(), "3-arm must not fire: {out:?}");
    }

    #[test]
    fn target_form_silent() {
        let out = propose_fix(
            "manual_let_else",
            "fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v }",
        );
        assert!(out.is_none(), "already let-else must not fire: {out:?}");
    }

    #[test]
    fn comment_between_arms_refused() {
        let out = propose_fix(
            "manual_let_else",
            "fn f(x: Option<u32>) -> u32 { let v = match x { Some(v) => v, // absent\n None => return 0 }; v }",
        );
        assert!(out.is_none(), "comment-displacing rewrite must be refused: {out:?}");
    }

    #[test]
    fn mut_target_silent() {
        let out = propose_fix(
            "manual_let_else",
            "fn f(x: Option<u32>) -> u32 { let mut v = match x { Some(v) => v, None => return 0 }; v += 1; v }",
        );
        assert!(out.is_none(), "mut target must not fire (mut would be lost): {out:?}");
    }
}
