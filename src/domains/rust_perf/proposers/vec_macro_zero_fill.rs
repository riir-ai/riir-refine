//! `vec-macro-zero-fill` — `Vec::new()` + `resize(n, 0)` explicit memset →
//! `vec![0; n]` alloc_zeroed (calloc).
//!
//! Pattern: within one block,
//! `let mut VAR [: TY] = Vec::new();` (or `= vec![];`) whose first later
//! mention is `VAR.resize(LEN, ZERO_LIT);` → the initializer becomes
//! `vec![ZERO_LIT; LEN]` (original texts verbatim) and the `resize`
//! statement is deleted. `vec![0; n]` rides the `IsZero` specialization —
//! the allocation goes through `alloc_zeroed` (calloc) and the explicit
//! memset pass disappears. Bench 020/B48: ParetoDominant 32-2505x.
//!
//! Unlike `collect-append-to-extend`, `VAR` stays alive: the rewrite only
//! pre-sizes the same binding, so any number of later uses (pushes, reads,
//! the tail `v`) survive untouched. The gates:
//! - **First-mention:** the first later statement in the block mentioning
//!   `VAR` must BE the resize — an intervening use (`v.push(..)` before the
//!   resize, the classic incremental-grow shape) skips the pattern (that
//!   shape belongs to `vec-with-capacity`, not this rule).
//! - **Zero fill only:** the fill argument must be a zero int/float literal
//!   (`0`, `0u8`, `0.0`, `0.0f32` — every base-10 digit zero). A non-zero
//!   fill is a real initialization, not a memset; `-0.0` (unary) and
//!   `0 as T` (cast) forms are skipped conservatively.
//! - A later re-binding of `VAR` stops the scan (shadowing ambiguity).
//!
//! `LEN` may be any expression (runtime arithmetic included — the eval span
//! uses a plain param, the tests exercise `cells * 2 + 1`).
//!
//! Out of scope: `resize_with`, resizes on non-path receivers, and the
//! resize living in a nested block below the `let`.

use syn::visit::Visit;
use syn::{Block, Expr, ExprLit, File, Lit, Local, Pat, Stmt};

use super::{delete_stmt_edit, edit, span_range, src, SourceEdit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut scan = BlockScan { source, edits: Vec::new() };
    scan.visit_file(file);
    scan.edits
}

struct BlockScan<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for BlockScan<'_> {
    fn visit_block(&mut self, b: &'ast Block) {
        self.scan_block(b);
        syn::visit::visit_block(self, b);
    }
}

impl BlockScan<'_> {
    fn scan_block(&mut self, block: &Block) {
        let stmts = &block.stmts;
        for (i, stmt) in stmts.iter().enumerate() {
            let Stmt::Local(local) = stmt else { continue };
            let Some(var) = fresh_vec_binding(local) else { continue };

            for later in stmts.iter().skip(i + 1) {
                if stmt_binds_ident(later, &var) {
                    break; // shadowing re-bind — stop the scan
                }
                if !self.stmt_mentions(later, &var) {
                    continue;
                }
                let Stmt::Expr(Expr::MethodCall(rz), _) = later else { break };
                if rz.method != "resize" || rz.args.len() != 2 {
                    break;
                }
                let Expr::Path(p) = &*rz.receiver else { break };
                if !p.path.is_ident(&var) {
                    break;
                }
                let (Some(len), Some(fill)) = (rz.args.first(), rz.args.last()) else { break };
                let Some(fill_txt) = zero_literal_text(self.source, fill) else { break };
                let Some(len_txt) = src(self.source, len) else { break };

                let replacement = format!("vec![{fill_txt}; {len_txt}]");
                let Some(init) = &local.init else { break };
                self.edits.push(edit(&init.expr, replacement));
                self.edits.push(delete_stmt_edit(self.source, later));
                break;
            }
        }
    }

    /// Word-boundary mention of `var` anywhere in the stmt's source text.
    fn stmt_mentions(&self, stmt: &Stmt, var: &str) -> bool {
        let (s, e) = span_range(stmt);
        match self.source.get(s..e) {
            Some(text) => contains_ident_word(text, var),
            // Defensive: treat unsliceable spans as mentions (skip the
            // rewrite rather than risk one).
            None => true,
        }
    }
}

/// `let mut VAR [: TY] = Vec::new();` or `= vec![];` → the binding ident.
/// Mirrors vec-with-capacity's `fresh_vec_binding` gates, plus Pat::Type
/// unwrapping so annotated lets fire (a type annotation wraps the pattern).
fn fresh_vec_binding(local: &Local) -> Option<String> {
    let mut pat = &local.pat;
    while let Pat::Type(pt) = pat {
        pat = &pt.pat;
    }
    let Pat::Ident(pi) = pat else { return None };
    if pi.by_ref.is_some() || pi.subpat.is_some() || pi.mutability.is_none() {
        return None;
    }
    let init = local.init.as_ref()?;
    if init.diverge.is_some() {
        return None;
    }
    match &*init.expr {
        Expr::Call(call) => {
            let Expr::Path(p) = &*call.func else { return None };
            let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
            if segs == ["Vec", "new"] && call.args.is_empty() {
                Some(pi.ident.to_string())
            } else {
                None
            }
        }
        Expr::Macro(mac) if mac.mac.path.is_ident("vec") && mac.mac.tokens.is_empty() => {
            Some(pi.ident.to_string())
        }
        _ => None,
    }
}

/// The original text of a zero int/float literal (suffix preserved —
/// `0u8` → `vec![0u8; n]`), or `None` for any other expression.
fn zero_literal_text(source: &str, expr: &Expr) -> Option<String> {
    let Expr::Lit(ExprLit { lit, .. }) = expr else { return None };
    match lit {
        Lit::Int(li) if digits_all_zero(li.base10_digits()) => src(source, expr),
        Lit::Float(lf) if float_digits_all_zero(lf.base10_digits()) => src(source, expr),
        _ => None,
    }
}

fn digits_all_zero(digits: &str) -> bool {
    !digits.is_empty() && digits.chars().all(|c| c == '0')
}

/// Float `base10_digits` KEEPS the `.`/`e`/sign separators (syn's
/// `parse_lit_float`) — judge only the digit characters: "0.0" → digits
/// "00" → zero; "0.5" → "05" → non-zero; "1e-3" → "13" → non-zero.
fn float_digits_all_zero(digits: &str) -> bool {
    let mut any_zero = false;
    for c in digits.chars() {
        match c {
            '0' => any_zero = true,
            '0'..='9' => return false,
            _ => {}
        }
    }
    any_zero
}

/// Does this stmt re-bind `ident` (any mutability)?
fn stmt_binds_ident(stmt: &Stmt, ident: &str) -> bool {
    match stmt {
        Stmt::Local(local) => matches!(&local.pat, Pat::Ident(pi) if pi.ident == ident),
        _ => false,
    }
}

fn is_ident_boundary_byte(b: u8) -> bool {
    !b.is_ascii_alphanumeric() && b != b'_'
}

fn contains_ident_word(text: &str, ident: &str) -> bool {
    let hay = text.as_bytes();
    let needle = ident.as_bytes();
    if needle.is_empty() || hay.len() < needle.len() {
        return false;
    }
    let mut at = 0usize;
    while let Some(rel) = hay[at..].iter().position(|&b| b == needle[0]) {
        let start = at + rel;
        let end = start + needle.len();
        if end <= hay.len()
            && (start == 0 || is_ident_boundary_byte(hay[start - 1]))
            && (end == hay.len() || is_ident_boundary_byte(hay[end]))
            && &hay[start..end] == needle
        {
            return true;
        }
        at = start + 1;
        if at >= hay.len() {
            break;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn corpus_fixture_rewrites() {
        let out = propose_fix(
            "vec-macro-zero-fill",
            "fn zero_buf(n: usize) -> Vec<u8> {\n    let mut v = Vec::new(); // uninit \
             alloc\n    v.resize(n, 0); // explicit memset pass\n    v\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("let mut v = vec![0; n];"), "{out}");
        assert!(!out.contains("resize"), "{out}");
        assert!(out.contains("// uninit alloc"), "{out}");
        assert!(out.contains("// explicit memset pass"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    /// The eval-span shape: no type annotation, bare `0`, tail use of `fog`.
    #[test]
    fn eval_span_shape_fires() {
        let out = propose_fix(
            "vec-macro-zero-fill",
            "pub fn blank_visibility_map(cells: usize) -> Vec<u8> {\n    let mut fog = \
             Vec::new();\n    fog.resize(cells, 0);\n    fog\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("let mut fog = vec![0; cells];"), "{out}");
        assert!(out.contains("    fog\n}"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn float_zero_and_suffixes_carry_verbatim() {
        let out = propose_fix(
            "vec-macro-zero-fill",
            "fn f(n: usize) -> Vec<f32> {\n    let mut v = Vec::new();\n    \
             v.resize(n, 0.0f32);\n    v\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("vec![0.0f32; n]"), "{out}");
    }

    #[test]
    fn arithmetic_len_fires() {
        let out = propose_fix(
            "vec-macro-zero-fill",
            "fn f(cells: usize) -> Vec<u8> {\n    let mut v: Vec<u8> = Vec::new();\n    \
             v.resize(cells * 2 + 1, 0u8);\n    v\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("vec![0u8; cells * 2 + 1]"), "{out}");
        // The type annotation survives byte-identically.
        assert!(out.contains("let mut v: Vec<u8> = vec![0u8;"), "{out}");
    }

    #[test]
    fn empty_vec_macro_init_fires() {
        let out = propose_fix(
            "vec-macro-zero-fill",
            "fn f(n: usize) -> Vec<u64> {\n    let mut v = vec![];\n    v.resize(n, 0);\n    v\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("let mut v = vec![0; n];"), "{out}");
    }

    #[test]
    fn nonzero_fill_skips() {
        assert!(propose_fix(
            "vec-macro-zero-fill",
            "fn f(n: usize) -> Vec<u8> {\n    let mut v = Vec::new();\n    v.resize(n, 1);\n    v\n}",
        )
        .is_none());
    }

    #[test]
    fn cast_zero_skips() {
        // `0 as u8` parses as Expr::Cast — conservatively out of scope.
        assert!(propose_fix(
            "vec-macro-zero-fill",
            "fn f(n: usize) -> Vec<u8> {\n    let mut v = Vec::new();\n    \
             v.resize(n, 0 as u8);\n    v\n}",
        )
        .is_none());
    }

    #[test]
    fn intervening_push_skips() {
        // Incremental grow-then-size shape — vec-with-capacity's territory.
        assert!(propose_fix(
            "vec-macro-zero-fill",
            "fn f(n: usize, x: u8) -> Vec<u8> {\n    let mut v = Vec::new();\n    \
             v.push(x);\n    v.resize(n, 0);\n    v\n}",
        )
        .is_none());
    }

    #[test]
    fn later_uses_survive() {
        let out = propose_fix(
            "vec-macro-zero-fill",
            "fn f(n: usize) -> Vec<u8> {\n    let mut v = Vec::new();\n    \
             v.resize(n, 0);\n    v.push(9);\n    v\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("v.push(9);"), "{out}");
        assert!(!out.contains("resize"), "{out}");
    }

    #[test]
    fn shadowing_rebind_skips() {
        assert!(propose_fix(
            "vec-macro-zero-fill",
            "fn f(n: usize) -> usize {\n    let mut v = Vec::new();\n    let v = 3;\n    \
             let _ = v;\n    n\n}",
        )
        .is_none());
    }

    #[test]
    fn no_resize_no_fire() {
        assert!(propose_fix(
            "vec-macro-zero-fill",
            "fn f() -> Vec<u8> {\n    let mut v = Vec::new();\n    v\n}",
        )
        .is_none());
    }

    #[test]
    fn resize_without_mut_binding_skips() {
        // Non-`mut` binding wouldn't compile against resize — skip rather
        // than rewrite non-compiling code.
        assert!(propose_fix(
            "vec-macro-zero-fill",
            "fn f(n: usize) -> Vec<u8> {\n    let v = Vec::new();\n    v.resize(n, 0);\n    v\n}",
        )
        .is_none());
    }
}
