//! `clippy::needless_range_loop` — `for i in 0..v.len() { body_using(v[i]) }`
//! → `for __elem in &v { body_with_elem }`.
//!
//! Span-preserving (T6.4): ONE atomic edit covering the whole loop, whose
//! replacement is the loop's original text with three substitution kinds
//! spliced in — the loop pat (`i` → the fresh binding), the iterable
//! (`0..v.len()` → the collection, by-reference form chosen by param
//! resolution), and each exact `v[i]` index expression (→ `*__elem`, the
//! universal deref splice, Plan 097 T1). All replacement fragments are
//! ORIGINAL source text.
//!
//! ## The slot splice (Plan 097 T1 — the P11 comparison-slot class)
//!
//! The reference binding `__elem: &T` fails E0308 in more positions than
//! one might expect — measured (clippy 1.98.1 probe): comparison operands
//! (`if __elem > 2`), typed lets (`let x: u32 = __elem`), AND value-arg
//! method receivers (`__elem.max(3)` — `&T: Ord` resolves on the reference
//! and the value argument mismatches). The universal splice `*__elem` is
//! DESUGAR-IDENTICAL to `v[i]` (`*Index::index(&v, i)`) in every rvalue
//! context, so it is correct wherever the original compiled. Parenthesized
//! `(*__elem)` ONLY where the slot is the direct base of a postfix
//! expression (method receiver, field base, call func — `*` binds looser
//! than `.`); everywhere else parenless (bare parens trigger rustc's
//! `unused_parens` at assigned-value positions, probe-measured). Known
//! remaining classes, documented-not-fixed (the `--verify` compile gate
//! auto-reverts them): `&mut`-method receivers through the slot on a shared
//! iteration (undecidable parse-only) and `assert_eq!(v[i], 2)` (the macro
//! takes references).
//!
//! ## Atomicity (Issue 032)
//!
//! The header + slot substitutions must land TOGETHER — the rewrite is
//! invalid half-applied. As separate edits, a foreign same-pass fix
//! overlapping one slot (`identity_op` on `hits[i] * 1` — same start, longer
//! end) was dropped by `apply_edits_traced`'s outer-first overlap selection
//! while the header edits survived → `for __elem in hits { hits[i] }`
//! (dangling `i`). One whole-loop edit is all-or-nothing under the same
//! selection; a nested foreign edit is dropped instead and re-fires on the
//! next fixpoint pass (the documented nesting semantics — an outer edit's
//! replacement keeps inner nodes' ORIGINAL text).
//!
//! ## Detection (exact AST analysis, replacing the token heuristic)
//!
//! The mutation-based version counted identifier occurrences in the body's
//! token stream — substring-based, so `minimum` counted as a use of `i`
//! (false negatives) and nested `v[v[i]]` slipped through as a false
//! POSITIVE (it rewrote to `v[__elem]`, indexing by an element). It also
//! emitted `&v` unconditionally — for a `v: &Vec<_>` param that iterates
//! `&&Vec`, which is not an iterator (the fixture compiled only because
//! nothing ever compiled it). The exact visitor tracks:
//!
//! - `exact` — `Expr::Index` nodes whose base is the collection path AND
//!   whose index is the bare loop ident (the substitution slots);
//! - `total` — every bare `Expr::Path` that IS the loop ident in the body
//!   (the default traversal counts the ident inside every index too — an
//!   index into a DIFFERENT collection, `w[i]`, therefore makes
//!   `total > exact` and blocks the rewrite);
//! - `complex` — any `Expr::Index` whose index mentions the ident without
//!   being the bare ident itself (`v[i + 1]`, `v[v[i]]`);
//! - `blocked` — an exact slot is written through (`v[i] = x`, `v[i] += x`,
//!   including through a field/index place chain — `v[i].field = x`) or has
//!   its address taken (`&v[i]`, `&mut v[i]`): the element binding is a
//!   shared reference and cannot stand in for a place expression.
//!
//! The rewrite fires iff `exact >= 1 && total == exact && !complex && !blocked`.
//!
//! ## Macro-arg descent (2026-08-19, the run-#2 miss map)
//!
//! The run-#2 bench fixture `for i in 0..names.len() {
//! out.push(format!("{}", names[i])) }` was refused: `names[i]` lives in
//! `format!`'s token stream — opaque to syn's visitor — so the scan saw zero
//! uses and `exact.is_empty()` blocked. Two rules close the gap:
//!
//! - **format-family macros** (`format!`/`println!`/`log::*`/… — the
//!   `uninlined_format_args` family table): the post-template args are
//!   expressions by definition; each is parsed as an `Expr` (file-relative
//!   spans survive the token round-trip) and scanned by the SAME visitor —
//!   index slots inside macros become substitution slots; any other `i`
//!   use refuses. Nested macros inside parsed args recurse the same way.
//!   The template literal itself is checked for the ident as a standalone
//!   word (`{i}` named captures, `{i.field}`, `{:i$}` width refs) — any hit
//!   refuses (the capture would dangle after the pat rename).
//! - **every other macro**: its full token stream is walked (recursing into
//!   groups); a standalone `i` ident token ANYWHERE refuses. Conservative —
//!   opaque stays refused, it just gets SEEN now.
//!
//! ## Fresh-binding uniquing (Plan 097 T2 — the P11 nested-loop persist)
//!
//! The old binary guard refused whenever the body mentioned ANY
//! `__elem`-prefixed identifier — which made a nested `needless_range_loop` a
//! hard persist: level 1's rewrite introduces `__elem`, and level 2 (whose
//! body references it) refused forever. The guard is now a TAKEN-SET
//! enumeration over the family `__elem` | `__elem<digits>` (path idents,
//! pattern bindings, macro token idents) plus the format-family template
//! literals (an implicit `{__elem}` capture re-binds to our fresh name —
//! each candidate is checked against the template words). The rewrite picks
//! the smallest free suffix, so level 2 becomes `__elem2` and heals at the
//! next fixpoint pass. Nonfamily prefix mentions (`__elem_x`) are distinct
//! identifiers and no longer block (the old guard was over-broad).
//!
//! ## Iterable form (param resolution)
//!
//! `--fix` has no type checker, so the collection ident is resolved against
//! the enclosing fn's parameters (the only source of truth available):
//! a `&`-typed param iterates as `v`, an owned param as `&v`. Anything the
//! resolver cannot pin (locals, fields, non-ident paths) skips the rewrite.

use super::uninlined_format_args::post_template_expr_args;
use super::{apply_edits, edit, for_each_fn, src, SourceEdit};
use std::collections::BTreeSet;
use proc_macro2::TokenTree;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{BinOp, Expr, ExprForLoop, ExprLit, ExprMacro, ExprRange, File, Lit, Signature};

/// Fresh binding introduced by the rewrite. The binding NAME is uniqued
/// against the family `__elem` | `__elem<digits>` (Plan 097 T2): a nested
/// loop whose body already mentions an outer rewrite's binding picks the
/// next free suffix instead of refusing.
const FRESH_PREFIX: &str = "__elem";
/// Uniquing bound: past this many candidate suffixes something is deeply
/// wrong — refuse rather than emit an opaque name.
const FRESH_NAME_LIMIT: u32 = 64;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for_each_fn(file, |sig, block| {
        let mut v = Collector { source, sig, edits: &mut edits };
        v.visit_block(block);
    });
    edits
}

struct Collector<'a, 'b> {
    source: &'a str,
    sig: &'b Signature,
    edits: &'b mut Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_, '_> {
    fn visit_expr_for_loop(&mut self, fl: &'ast ExprForLoop) {
        syn::visit::visit_expr_for_loop(self, fl);
        if let Some(mut loop_edits) = self.try_rewrite_for(fl) {
            self.edits.append(&mut loop_edits);
        }
    }
}

impl Collector<'_, '_> {
    fn try_rewrite_for(&self, fl: &ExprForLoop) -> Option<Vec<SourceEdit>> {
        if !fl.attrs.is_empty() {
            return None; // attrs would be lost by the header splice
        }
        let Expr::Range(range) = &*fl.expr else {
            return None;
        };
        check_range_start_zero(range)?;
        let collection_path = check_range_end_len(range)?;

        // The loop ident: a bare (non-ref, no subpat) binding.
        let syn::Pat::Ident(pi) = &*fl.pat else {
            return None;
        };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            return None;
        }
        let idx_ident = &pi.ident;

        // Exact usage analysis over the BODY only (the header's `i` is not a use).
        let usage = analyze_index_usage(&fl.body, collection_path, idx_ident);
        if !usage.ok() {
            return None;
        }

        // Fresh binding: uniqued against the family, not a binary refuse —
        // the nested-loop case (level 1's `__elem` visible in level 2's body)
        // now picks `__elem2` instead of persisting (Plan 097 T2).
        let fresh = fresh_binding_name(&fl.body)?;

        // Iterable form: resolve the collection against the fn params —
        // a `&`-typed param iterates bare, an owned param by reference.
        // Unresolvable collections are skipped (no type checker to ask).
        let coll_text = src(self.source, collection_path)?;
        let iter_text = match self.param_is_reference(collection_path) {
            Some(true) => coll_text,
            Some(false) => format!("&{coll_text}"),
            None => return None,
        };

        // Issue 032: ONE atomic edit spanning the whole loop. The header +
        // every slot substitution must land together; as separate edits a
        // foreign same-pass fix overlapping one slot (identity_op on
        // `hits[i] * 1`) dropped just that slot while the headers survived
        // → `for __elem in hits { hits[i] }` (dangling `i`). One edit is
        // all-or-nothing under the outer-first overlap selection; nested
        // foreign edits re-fire on the next fixpoint pass.
        let span = fl.span().byte_range();
        let loop_text = self.source.get(span.start..span.end)?;
        let mut subs = vec![
            edit(&fl.pat, fresh.clone()),
            edit(&fl.expr, iter_text),
        ];
        for (start, end) in usage.exact {
            // Universal deref splice (Plan 097 T1): `*__elem` is
            // desugar-identical to `v[i]` in every rvalue context (the bare
            // reference binding fails E0308 on comparison operands, typed
            // lets, and value-arg method receivers). Parens only where the
            // slot is a postfix base — `*` binds looser than `.`.
            let replacement = if usage.paren.contains(&(start, end)) {
                format!("(*{fresh})")
            } else {
                format!("*{fresh}")
            };
            subs.push(SourceEdit { start, end, replacement });
        }
        let replacement = apply_edits(
            loop_text,
            subs.into_iter()
                .map(|e| SourceEdit {
                    start: e.start - span.start,
                    end: e.end - span.start,
                    replacement: e.replacement,
                })
                .collect(),
        );
        Some(vec![SourceEdit { start: span.start, end: span.end, replacement }])
    }

    /// Is the single-ident path a fn parameter of reference type?
    /// `None` = not a resolvable parameter (skip the rewrite).
    fn param_is_reference(&self, path: &syn::ExprPath) -> Option<bool> {
        let ident = path.path.get_ident()?;
        for arg in &self.sig.inputs {
            let syn::FnArg::Typed(pat) = arg else { continue };
            let syn::Pat::Ident(pi) = &*pat.pat else { continue };
            if pi.ident != *ident {
                continue;
            }
            return Some(matches!(&*pat.ty, syn::Type::Reference(_) | syn::Type::Paren(_)));
        }
        None
    }
}

fn check_range_start_zero(range: &ExprRange) -> Option<()> {
    let start = range.start.as_deref()?;
    if matches!(start, Expr::Lit(ExprLit { lit: Lit::Int(li), .. }) if li.base10_digits() == "0") {
        Some(())
    } else {
        None
    }
}

fn check_range_end_len(range: &ExprRange) -> Option<&syn::ExprPath> {
    let end = range.end.as_deref()?;
    let end_call = match end {
        Expr::MethodCall(mc) if mc.method == "len" && mc.args.is_empty() => &*mc.receiver,
        _ => return None,
    };
    if let Expr::Path(p) = end_call {
        Some(p)
    } else {
        None
    }
}

struct IndexUsage {
    /// Whole `collection[idx]` byte ranges — the substitution slots. Spans,
    /// not AST refs: slots inside format-family macro args live in locally
    /// parsed `Expr`s (different lifetime than the loop body).
    exact: Vec<(usize, usize)>,
    /// Slot ranges that are the direct base of a postfix expression (method
    /// receiver, field base, call func) — those need the parenthesized
    /// `(*__elem)` form; every other slot takes parenless `*__elem` (Plan
    /// 097 T1).
    paren: BTreeSet<(usize, usize)>,
    /// Every bare `Expr::Path` that IS the loop ident in the body.
    total: usize,
    /// An index expression mentions the ident without being the bare ident.
    complex: bool,
    /// An exact slot is written through (directly or through a field/index
    /// place chain), has its address taken, or a macro mentions the ident
    /// somewhere descent cannot account for.
    blocked: bool,
}

impl IndexUsage {
    fn ok(&self) -> bool {
        !self.exact.is_empty() && self.total == self.exact.len() && !self.complex && !self.blocked
    }
}

fn analyze_index_usage(
    body: &syn::Block,
    collection: &syn::ExprPath,
    idx: &syn::Ident,
) -> IndexUsage {
    let mut scan = Scan {
        collection,
        idx,
        usage: IndexUsage {
            exact: Vec::new(),
            paren: BTreeSet::new(),
            total: 0,
            complex: false,
            blocked: false,
        },
    };
    scan.visit_block(body);
    scan.usage
}

struct Scan<'b> {
    collection: &'b syn::ExprPath,
    idx: &'b syn::Ident,
    usage: IndexUsage,
}

// `Visit<'ast>` is implemented for EVERY 'ast — the visitor holds no AST
// refs (slots are byte ranges), so it can scan the loop body AND locally
// parsed macro-arg exprs in one pass.
impl<'ast> Visit<'ast> for Scan<'_> {
    fn visit_expr_index(&mut self, e: &'ast syn::ExprIndex) {
        syn::visit::visit_expr_index(self, e);
        let idx_is_bare = matches!(&*e.index, Expr::Path(p) if p.path.is_ident(self.idx));
        if !idx_is_bare {
            // `v[i + 1]`, `v[v[i]]` — index mentions the ident but isn't
            // the bare ident: substitution cannot cover it.
            if expr_mentions_ident(&e.index, self.idx) {
                self.usage.complex = true;
            }
            return;
        }
        // `w[i]` (other base) is NOT pushed as an exact slot, but the
        // traversal already counted its inner `i` path toward `total` —
        // so `total > exact.len()` blocks the rewrite, as it must.
        if matches!(&*e.expr, Expr::Path(p) if paths_equal(p, self.collection)) {
            let r = e.span().byte_range();
            self.usage.exact.push((r.start, r.end));
        }
    }

    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path.is_ident(self.idx) {
            self.usage.total += 1;
        }
        syn::visit::visit_expr_path(self, p);
    }

    // `v[i] = x` — the slot is a place expression; the element binding
    // is a shared reference and cannot be assigned through.
    // Plan 097 T1: the guard extends to place CHAINS — `v[i].field = x`
    // (and `v[i][j] = x`) write through the slot just the same.
    fn visit_expr_assign(&mut self, e: &'ast syn::ExprAssign) {
        syn::visit::visit_expr_assign(self, e);
        if self.place_chain_contains_slot(&e.left) {
            self.usage.blocked = true;
        }
    }

    // `v[i] += x` etc. — compound assigns are place writes too (chains
    // included).
    fn visit_expr_binary(&mut self, e: &'ast syn::ExprBinary) {
        syn::visit::visit_expr_binary(self, e);
        if is_assign_op(&e.op) && self.place_chain_contains_slot(&e.left) {
            self.usage.blocked = true;
        }
        // Comparison operands are not place writes — no marking needed
        // under the universal deref (the splice is type-identical to the
        // index everywhere).
    }

    // `&v[i]` / `&mut v[i]` — address-of through the slot changes the
    // referent (`&T` → `&&T` under the deref splice it would become
    // `&*__elem` = the same `&&T` shape change).
    fn visit_expr_reference(&mut self, e: &'ast syn::ExprReference) {
        syn::visit::visit_expr_reference(self, e);
        if self.is_exact_slot(&e.expr) {
            self.usage.blocked = true;
        }
    }

    // Postfix bases need the parenthesized splice: `v[i].len()` →
    // `(*__elem).len()` (bare `*__elem.len()` would deref the RESULT).
    fn visit_expr_method_call(&mut self, e: &'ast syn::ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, e);
        if self.is_exact_slot(&e.receiver) {
            let r = e.receiver.span().byte_range();
            self.usage.paren.insert((r.start, r.end));
        }
    }

    fn visit_expr_field(&mut self, e: &'ast syn::ExprField) {
        syn::visit::visit_expr_field(self, e);
        if self.is_exact_slot(&e.base) {
            let r = e.base.span().byte_range();
            self.usage.paren.insert((r.start, r.end));
        }
    }

    // `v[i](args)` — an element holding a callable: same postfix rule.
    fn visit_expr_call(&mut self, e: &'ast syn::ExprCall) {
        syn::visit::visit_expr_call(self, e);
        if self.is_exact_slot(&e.func) {
            let r = e.func.span().byte_range();
            self.usage.paren.insert((r.start, r.end));
        }
    }

    /// Macro-arg descent: a format-family macro's post-template args parse
    /// as exprs and join the scan (their index slots carry file-relative
    /// spans); any other macro mentioning `i` (or an unparseable/malformed
    /// shape) refuses.
    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        if !self.scan_macro(&m.mac) {
            self.usage.blocked = true;
        }
    }

    /// Statement-position macros (`println!(…);`) — same handling.
    fn visit_stmt_macro(&mut self, m: &'ast syn::StmtMacro) {
        if !self.scan_macro(&m.mac) {
            self.usage.blocked = true;
        }
    }
}

impl Scan<'_> {
    fn is_exact_slot(&self, e: &Expr) -> bool {
        matches!(e, Expr::Index(ix)
            if matches!(&*ix.index, Expr::Path(p) if p.path.is_ident(self.idx))
                && matches!(&*ix.expr, Expr::Path(p) if paths_equal(p, self.collection)))
    }

    /// Does this PLACE chain (`v[i].field`, `v[i][j]`, direct `v[i]`)
    /// contain an exact slot anywhere along the base chain? Place writes
    /// through the slot must refuse — the deref splice is an rvalue.
    fn place_chain_contains_slot(&self, e: &Expr) -> bool {
        let mut cur = e;
        loop {
            if self.is_exact_slot(cur) {
                return true;
            }
            match cur {
                Expr::Field(f) => cur = &f.base,
                Expr::Index(ix) => cur = &ix.expr,
                Expr::Paren(p) => cur = &p.expr,
                _ => return false,
            }
        }
    }

    /// `false` = this macro mentions the idx somewhere substitution cannot
    /// account for → refuse the loop rewrite.
    fn scan_macro(&mut self, mac: &syn::Macro) -> bool {
        // Malformed family macro — opaque, be conservative about the WHOLE
        // token stream (template included).
        let Some(args) = post_template_expr_args(mac) else {
            return !tokens_mention_ident(mac.tokens.clone().into_iter(), self.idx);
        };
        // The template literal: any standalone-word mention of the idx is a
        // named capture / width ref (`{i}`, `{i.field}`, `{:i$}`) that would
        // dangle after the pat rename.
        if let Some(t) = template_literal(mac) {
            if str_mentions_word(&t, &self.idx.to_string()) {
                return false;
            }
        } else {
            // Template slot not a plain literal (tracing `target:` form) —
            // fall back to the conservative whole-stream check.
            return !tokens_mention_ident(mac.tokens.clone().into_iter(), self.idx);
        }
        for arg in args {
            let Ok(expr) = syn::parse2::<Expr>(arg.toks.iter().cloned().collect()) else {
                // Unparseable arg (named `k = v`, …): if it mentions the idx,
                // it's a use we cannot substitute.
                if tokens_mention_ident(arg.toks.iter().cloned(), self.idx) {
                    return false;
                }
                continue;
            };
            // Recursion: the parsed arg may itself contain macros (nested
            // `println!("{}", format!("{}", i))`) — visit_expr_macro fires
            // again through this same visitor.
            self.visit_expr(&expr);
        }
        true
    }
}

/// The template-position literal's text, when it is a single plain string
/// literal token (the format-family contract position).
fn template_literal(mac: &syn::Macro) -> Option<String> {
    let toks: Vec<TokenTree> = mac.tokens.clone().into_iter().collect();
    // The family table's position is post_template_expr_args-internal; for
    // the template check, accept the first-OR-second single-literal token
    // (format/println-family = first; assert/write = second) — anything
    // else is None (conservative path).
    for pos in [0usize, 1] {
        if let Some(TokenTree::Literal(l)) = toks.get(pos) {
            let t = l.to_string();
            if t.starts_with('"') {
                return Some(t);
            }
        }
    }
    None
}

/// Does the token stream contain the ident as a standalone token?
fn tokens_mention_ident(toks: impl Iterator<Item = TokenTree>, idx: &syn::Ident) -> bool {
    for t in toks {
        match t {
            TokenTree::Ident(id) if id == *idx => return true,
            TokenTree::Group(g) if tokens_mention_ident(g.stream().into_iter(), idx) => {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Does the string mention the word (rust-ident boundaries)? Conservative
/// for prose ("i" as an English word refuses too — safe direction).
fn str_mentions_word(s: &str, word: &str) -> bool {
    let b = s.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = s[from..].find(word) {
        let at = from + rel;
        let end = at + word.len();
        let before_ok = at == 0 || !is_ident_byte(b[at - 1]);
        let after_ok = end >= b.len() || !is_ident_byte(b[end]);
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_assign_op(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::AddAssign(_)
            | BinOp::SubAssign(_)
            | BinOp::MulAssign(_)
            | BinOp::DivAssign(_)
            | BinOp::RemAssign(_)
            | BinOp::BitXorAssign(_)
            | BinOp::BitAndAssign(_)
            | BinOp::BitOrAssign(_)
            | BinOp::ShlAssign(_)
            | BinOp::ShrAssign(_)
    )
}

/// Does the expression mention the ident as a bare path anywhere?
fn expr_mentions_ident(e: &Expr, idx: &syn::Ident) -> bool {
    struct M<'b> {
        idx: &'b syn::Ident,
        found: bool,
    }
    impl<'ast> Visit<'ast> for M<'_> {
        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
            if p.path.is_ident(self.idx) {
                self.found = true;
            }
            syn::visit::visit_expr_path(self, p);
        }
    }
    let mut m = M { idx, found: false };
    m.visit_expr(e);
    m.found
}

/// Pick the fresh binding name for the rewrite: the smallest name in the
/// family `__elem` | `__elem<2..>` not taken by the body (Plan 097 T2).
/// "Taken" = a path ident or macro token ident in the family, or the exact
/// candidate word inside a format-family template literal (an implicit
/// `{__elem}` / `{__elem2}` capture would silently re-bind to our fresh
/// binding — the per-candidate template check closes that hole). Returns
/// None past the uniquing bound.
fn fresh_binding_name(body: &syn::Block) -> Option<String> {
    let (taken, templates) = body_family_names(body);
    for k in 1..=FRESH_NAME_LIMIT {
        let name = fresh_name_for(k);
        if !taken.contains(&k) && !templates.iter().any(|t| str_mentions_word(t, &name)) {
            return Some(name);
        }
    }
    None
}

fn fresh_name_for(k: u32) -> String {
    if k == 1 {
        FRESH_PREFIX.to_string()
    } else {
        format!("{FRESH_PREFIX}{k}")
    }
}

/// The body's occupied family suffix indices + the format-family template
/// texts (for the per-candidate implicit-capture check).
type BodyNames = (BTreeSet<u32>, Vec<String>);

fn body_family_names(body: &syn::Block) -> BodyNames {
    struct P {
        taken: BTreeSet<u32>,
        templates: Vec<String>,
    }
    impl<'ast> Visit<'ast> for P {
        fn visit_pat(&mut self, p: &'ast syn::Pat) {
            if let syn::Pat::Ident(pi) = p {
                if let Some(idx) = family_index(Some(&pi.ident)) {
                    self.taken.insert(idx);
                }
            }
            syn::visit::visit_pat(self, p);
        }

        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
            if let Some(idx) = family_index(p.path.get_ident()) {
                self.taken.insert(idx);
            }
            syn::visit::visit_expr_path(self, p);
        }

        fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
            collect_token_indices(m.mac.tokens.clone().into_iter(), &mut self.taken);
            if let Some(t) = template_literal(&m.mac) {
                self.templates.push(t);
            }
            // Descend into the macro's parsed args — a nested macro inside
            // an arg carries its own template.
            if let Some(args) = post_template_expr_args(&m.mac) {
                for arg in args {
                    if let Ok(expr) = syn::parse2::<Expr>(arg.toks.iter().cloned().collect()) {
                        self.visit_expr(&expr);
                    }
                }
            }
        }

        fn visit_stmt_macro(&mut self, m: &'ast syn::StmtMacro) {
            collect_token_indices(m.mac.tokens.clone().into_iter(), &mut self.taken);
            if let Some(t) = template_literal(&m.mac) {
                self.templates.push(t);
            }
            if let Some(args) = post_template_expr_args(&m.mac) {
                for arg in args {
                    if let Ok(expr) = syn::parse2::<Expr>(arg.toks.iter().cloned().collect()) {
                        self.visit_expr(&expr);
                    }
                }
            }
        }
    }
    let mut p = P { taken: BTreeSet::new(), templates: Vec::new() };
    p.visit_block(body);
    (p.taken, p.templates)
}

/// `__elem` → 1, `__elem2` → 2; other identifiers (including
/// `__elem_x`-style nonfamily prefixes) → None — they are distinct
/// identifiers and cannot be captured by the rewrite.
fn family_index(ident: Option<&syn::Ident>) -> Option<u32> {
    let id = ident?;
    let s = id.to_string();
    let suffix = s.strip_prefix(FRESH_PREFIX)?;
    if suffix.is_empty() {
        return Some(1);
    }
    if !suffix.bytes().all(|b| b.is_ascii_digit()) || suffix.len() > 9 {
        return None;
    }
    suffix.parse::<u32>().ok().filter(|n| *n >= 2)
}

fn collect_token_indices(toks: impl Iterator<Item = TokenTree>, taken: &mut BTreeSet<u32>) {
    for t in toks {
        match t {
            TokenTree::Ident(id) => {
                if let Some(idx) = family_index(Some(&id)) {
                    taken.insert(idx);
                }
            }
            TokenTree::Group(g) => {
                collect_token_indices(g.stream().into_iter(), taken);
            }
            _ => {}
        }
    }
}

fn paths_equal(a: &syn::ExprPath, b: &syn::ExprPath) -> bool {
    a.path.segments.len() == b.path.segments.len()
        && a.path
            .segments
            .iter()
            .zip(&b.path.segments)
            .all(|(sa, sb)| sa.ident == sb.ident)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_range_loop_ref_param_bare_iter() {
        // `v: &Vec<_>` — the collection is already a reference; `&v` would
        // iterate `&&Vec` (not an iterator). The old proposer had this bug.
        let input = "fn double_all(v: &Vec<i32>) -> Vec<i32> {\n    let mut out = Vec::new();\n    for i in 0..v.len() { out.push(v[i] * 2); }\n    out\n}";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(
            out.contains("for __elem in v { out.push(*__elem * 2); }"),
            "ref-param iter loop not produced: {out}"
        );
        assert!(!out.contains("0.."), "range not removed: {out}");
    }

    #[test]
    fn converts_range_loop_owned_param_ref_iter() {
        let input = "fn sum(v: Vec<i32>) -> i32 { let mut s = 0; for i in 0..v.len() { s += v[i]; } s }";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(out.contains("for __elem in &v { s += *__elem; }"), "{out}");
    }

    #[test]
    fn slice_param_bare_iter() {
        let input = "fn sum(v: &[i32]) -> i32 { let mut s = 0; for i in 0..v.len() { s += v[i]; } s }";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(out.contains("for __elem in v {"), "{out}");
        assert!(out.contains("s += *__elem;"), "{out}");
    }

    #[test]
    fn leaves_non_range_loop_unchanged() {
        let input = "fn go(v: Vec<i32>) { for x in v { println!(\"{}\", x); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "non-range loop touched: {out:?}");
    }

    #[test]
    fn leaves_loop_with_arithmetic_index() {
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { println!(\"{}\", v[i] + i); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "arithmetic-index loop touched: {out:?}");
    }

    /// `v[i + 1]` — complex index mentioning the ident: rejected exactly.
    #[test]
    fn leaves_loop_with_offset_index() {
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { println!(\"{}\", v[i + 1]); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "offset-index loop touched: {out:?}");
    }

    /// `v[v[i]]` — the outer index is not the bare ident: the old token
    /// heuristic false-positived here (it rewrote to `v[__elem]`).
    #[test]
    fn leaves_loop_with_nested_index() {
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { println!(\"{}\", v[v[i]]); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "nested-index loop touched: {out:?}");
    }

    /// `w[i]` — the ident indexes a DIFFERENT collection: cannot rewrite.
    #[test]
    fn leaves_loop_indexing_other_collection() {
        let input =
            "fn go(v: Vec<i32>, w: Vec<i32>) { for i in 0..v.len() { println!(\"{}\", w[i]); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "other-collection index touched: {out:?}");
    }

    /// `v[i] = 5` — a place write: the element binding is a shared ref.
    #[test]
    fn leaves_loop_with_slot_write() {
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { v[i] = 5; } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "slot-writing loop touched: {out:?}");
    }

    /// `v[i] += 1` — compound-assign place write.
    #[test]
    fn leaves_loop_with_slot_compound_write() {
        let input = "fn go(v: Vec<i32>) -> i32 { let mut s = 0; for i in 0..v.len() { v[i] += 1; s += v[i]; } s }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "compound-write loop touched: {out:?}");
    }

    /// `&v[i]` — address-of through the slot (`&T` → `&&T` under rewrite).
    #[test]
    fn leaves_loop_with_slot_ref() {
        let input = "fn go(v: Vec<i32>) -> i32 { let mut s = 0; for i in 0..v.len() { s += *(&v[i]); } s }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "slot-ref loop touched: {out:?}");
    }

    /// A local (non-param) collection cannot be resolved to ref-or-owned —
    /// skip rather than guess the iterable form.
    #[test]
    fn leaves_loop_over_local_collection() {
        let input = "fn go() -> i32 { let v = vec![1, 2, 3]; let mut s = 0; for i in 0..v.len() { s += v[i]; } s }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "unresolvable local collection must skip: {out:?}");
    }

    /// `minimum` contains but is not `i` — the old token heuristic treated
    /// it as an idx use (false negative); the exact visitor does not.
    #[test]
    fn substring_identifiers_do_not_count_as_uses() {
        let input = "fn go(v: &[i32], minimum: i32) -> i32 { let mut s = 0; for i in 0..v.len() { if v[i] > minimum { s += v[i]; } } s }";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(out.contains("for __elem in v {"), "{out}");
        assert!(out.contains("if *__elem > minimum"), "{out}");
        assert!(out.contains("s += *__elem"), "{out}");
    }

    /// Plan 097 T1 — the P11 headline class: the comparison slot now emits
    /// the deref splice (`if *__elem > 2`), which COMPILES — the bare
    /// reference binding was E0308 here (`&u32` vs the integer literal).
    #[test]
    fn comparison_slot_splices_deref() {
        let input = "pub fn measure(hits: &[u32]) -> u32 { let mut n = 0; for i in 0..hits.len() { if hits[i] > 2 { n += hits[i]; } } n }";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(out.contains("if *__elem > 2"), "{out}");
        assert!(out.contains("n += *__elem;"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    /// Plan 097 T1 — typed-let slots splice parenless (the deref binds
    /// tighter than the type ascription; probe: zero clippy lints) while a
    /// postfix receiver keeps required parens (`*` binds looser than `.`).
    #[test]
    fn typed_let_parenless_postfix_parenthesized() {
        let input = "pub fn stats(v: &[u32]) -> u32 { let mut t = 0u32; for i in 0..v.len() { let x: u32 = v[i]; t += v[i].max(x); } t }";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(out.contains("let x: u32 = *__elem;"), "{out}");
        assert!(out.contains("(*__elem).max(x)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    /// Plan 097 T2 — a body that already binds `__elem` no longer refuses:
    /// the rewrite picks the next free family suffix (`__elem2`) and the
    /// pre-existing binding keeps its meaning.
    #[test]
    fn fresh_prefix_collision_uniques_fresh_name() {
        let input = "fn go(v: &[i32]) -> i32 { let mut s = 0; let __elem = 1; for i in 0..v.len() { s += v[i] + __elem; } s }";
        let out = propose_fix("needless_range_loop", input).expect("uniqued rewrite");
        assert!(out.contains("for __elem2 in v"), "{out}");
        assert!(out.contains("s += *__elem2 + __elem;"), "{out}");
    }

    /// Plan 097 T2 — the nested-loop persist: level 1 introduces `__elem`,
    /// level 2's body references it; level 2 now heals with `__elem2` at the
    /// next fixpoint pass instead of persisting (the former gen7-ref-nrl-nrl
    /// shape, now gen7-d2-nrl-nrl).
    #[test]
    fn nested_range_loop_uniques_fresh_binding() {
        let input = "pub fn grid(outer: &[u32], inner: &[u32]) -> u32 { let mut acc = 0; for i in 0..outer.len() { for j in 0..inner.len() { acc += outer[i] * inner[j]; } } acc }";
        let healed = fixpoint_over(&["needless_range_loop"], input);
        assert!(healed.contains("for __elem in outer"), "{healed}");
        assert!(healed.contains("for __elem2 in inner"), "{healed}");
        assert!(healed.contains("acc += *__elem * *__elem2;"), "{healed}");
        assert!(!healed.contains("outer["), "{healed}");
        assert!(!healed.contains("inner["), "{healed}");
        syn::parse_file(&healed).expect("re-parses");
    }

    /// Plan 097 T2 — a `{__elem}` implicit capture in a template would
    /// re-bind to the fresh name: the candidate is checked against the
    /// template words and the rewrite bumps to `__elem2`.
    #[test]
    fn template_implicit_capture_bumps_name() {
        let input = "pub fn go(v: &[i32]) { for __elem in v { for i in 0..v.len() { println!(\"{__elem}: {}\", v[i]); } } }";
        let healed = fixpoint_over(&["needless_range_loop"], input);
        assert!(healed.contains("for __elem2 in v"), "{healed}");
        assert!(healed.contains("println!(\"{__elem}: {}\", *__elem2)"), "{healed}");
        syn::parse_file(&healed).expect("re-parses");
    }

    #[test]
    fn unused_idx_in_range_loop_unchanged() {
        // The index never appears in the body — rewriting to `for _ in &v`
        // is a different lint (unused loop variable is not ours to rewrite).
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { println!(\"tick\"); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "unused-idx loop touched: {out:?}");
    }

    #[test]
    fn impl_method_param_resolves() {
        let input = "struct S;\nimpl S {\n    fn total(&self, v: &Vec<i32>) -> i32 { let mut s = 0; for i in 0..v.len() { s += v[i]; } s }\n}";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(out.contains("for __elem in v { s += *__elem; }"), "{out}");
    }

    /// Plan 097 T1 — a place write through a FIELD chain on the slot
    /// (`v[i].field = x`) must refuse: the deref splice is an rvalue, and
    /// `(*__elem).field = x` is E0594.
    #[test]
    fn slot_field_write_refuses() {
        let input = "struct P { f: u32 }\nfn go(v: Vec<P>) { for i in 0..v.len() { v[i].f = 1; } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "slot field write must refuse: {out:?}");
    }

    // ── Issue 032: same-pass composition must not half-apply the rewrite ──

    /// The rewrite is ONE atomic edit covering the whole loop. As separate
    /// header/slot edits, a foreign same-pass fix overlapping one slot
    /// (`identity_op` on `hits[i] * 1`) dropped just that slot while the
    /// header edits survived → `for __elem in hits { total += hits[i]; }`
    /// (dangling `i`). Found by clippy-bench run #1 (Issue 032).
    #[test]
    fn rewrite_is_one_atomic_whole_loop_edit() {
        let input =
            "pub fn total_damage(hits: &[u32]) -> u32 {\n    let mut total = 0;\n    for i in 0..hits.len() {\n        total += hits[i];\n    }\n    total\n}";
        let file = syn::parse_file(input).expect("fixture parses");
        let edits = super::collect_edits(input, &file);
        assert_eq!(edits.len(), 1, "must be exactly one edit: {edits:?}");
        let e = &edits[0];
        assert_eq!(&input[e.start..e.end],
            "for i in 0..hits.len() {\n        total += hits[i];\n    }",
            "the one edit must span the whole loop");
        assert_eq!(e.replacement,
            "for __elem in hits {\n        total += *__elem;\n    }",
            "replacement carries header + slot rewrites together");
    }

    /// Reproduces the frozen bench fixture `composite-range_loop_plus_identity_op`
    /// through the production fixpoint shape (all lints collected per pass,
    /// overlap-selected splices): the healed loop must compile-ready — every
    /// `hits[i]` rewritten, no dangling `i`.
    #[test]
    fn composes_with_identity_op_without_dangling_index() {
        let input = "pub fn total_damage(hits: &[u32]) -> u32 {\n    let mut total = 0;\n    for i in 0..hits.len() {\n        total += hits[i] * 1;\n    }\n    total\n}\n";
        let healed = fixpoint_over(&["identity_op", "double_parens", "needless_range_loop"], input);
        assert!(healed.contains("for __elem in hits"), "header rewritten: {healed}");
        assert!(!healed.contains("hits["), "dangling index survived: {healed}");
        assert!(!healed.contains("* 1"), "identity_op must land too: {healed}");
    }

    /// Reproduces the frozen bench fixture
    /// `composite-range_loop_plus_double_parens` — the paren edit's span
    /// contains the slot edit, same partial-application hazard.
    #[test]
    fn composes_with_double_parens_without_dangling_index() {
        let input = "pub fn offsets(values: &[u32]) -> Vec<u32> {\n    let mut out = Vec::new();\n    for i in 0..values.len() {\n        out.push(((values[i] + 1)));\n    }\n    out\n}\n";
        let healed = fixpoint_over(&["identity_op", "double_parens", "needless_range_loop"], input);
        assert!(healed.contains("for __elem in values"), "header rewritten: {healed}");
        assert!(!healed.contains("values["), "dangling index survived: {healed}");
    }

    /// Reproduces the frozen bench fixture
    /// `composite-range_loop_plus_format_args` — the run-#2 miss-map entry:
    /// `names[i]` inside `format!` args was invisible to the visitor
    /// (opaque token stream), so the rewrite was refused with
    /// `exact.is_empty()`. The macro-arg descent parses the post-template
    /// args as exprs (file-relative spans) and substitutes inside them.
    #[test]
    fn converts_loop_with_format_macro_args() {
        let input = "pub fn describe_all(names: &[String]) -> Vec<String> {\n    let mut out = Vec::new();\n    for i in 0..names.len() {\n        out.push(format!(\"{}\", names[i]));\n    }\n    out\n}\n";
        let out = propose_fix("needless_range_loop", input).expect("pattern matched");
        assert!(out.contains("for __elem in names"), "header rewritten: {out}");
        assert!(out.contains("format!(\"{}\", *__elem)"), "macro slot substituted: {out}");
        assert!(!out.contains("names["), "dangling index survived: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn println_statement_macro_args_descend() {
        // Statement-position macro (StmtMacro) with the slot inside.
        let input = "pub fn dump(v: &[u32]) {\n    for i in 0..v.len() {\n        println!(\"{}\", v[i]);\n    }\n}\n";
        let out = propose_fix("needless_range_loop", input).expect("pattern matched");
        assert!(out.contains("println!(\"{}\", *__elem)"), "slot substituted: {out}");
        assert!(!out.contains("v["), "{out}");
    }

    #[test]
    fn format_named_capture_refuses() {
        // `{i}` in the template is a named capture — renaming the pat would
        // dangle it. Refuse.
        let input = "pub fn show(names: &[String]) -> String {\n    let mut out = String::new();\n    for i in 0..names.len() {\n        out.push_str(&format!(\"{i}: {}\", names[i]));\n    }\n    out\n}\n";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "named capture must refuse: {out:?}");
    }

    #[test]
    fn bare_ident_macro_arg_refuses() {
        // `format!("{}", i)` — the ident used directly (not as an index):
        // substitution cannot cover it.
        let input = "pub fn show(v: &[u32]) {\n    for i in 0..v.len() {\n        println!(\"{} {}\", i, v[i]);\n    }\n}\n";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "bare ident arg must refuse: {out:?}");
    }

    #[test]
    fn opaque_macro_mentioning_ident_refuses() {
        // Non-format-family macro whose tokens mention `i` — opaque stays
    // refused, it just gets SEEN now (the conservative token walk).
        let input = "macro_rules! eat { ($e:expr) => { let _ = $e; } }\npub fn f(v: &[u32]) {\n    for i in 0..v.len() {\n        eat!(i);\n        let _ = v[i];\n    }\n}\n";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "opaque macro mention must refuse: {out:?}");
    }

    #[test]
    fn nonfamily_prefix_mention_no_longer_blocks() {
        // `__elem_x` is a DISTINCT identifier (not in the `__elem<digits>`
        // family) — the old binary guard refused on any prefix mention; the
        // enumeration ignores it and the rewrite heals.
        let input = "pub fn show(names: &[String]) -> String {\n    let mut out = String::new();\n    for i in 0..names.len() {\n        out.push_str(&format!(\"{}\", { let __elem_x = names[i]; __elem_x }));\n    }\n    out\n}\n";
        let out = propose_fix("needless_range_loop", input).expect("nonfamily mention must not block");
        assert!(out.contains("let __elem_x = *__elem; __elem_x"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    /// The production fixpoint shape from `src/bin/heal.rs` / `score_bench`'s
    /// `heal_fixpoint`: per pass, collect every lint's edits against the
    /// current parse, apply with overlap selection, repeat until quiet.
    fn fixpoint_over(lints: &[&str], source: &str) -> String {
        let mut current = source.to_string();
        for _ in 0..4 {
            let Ok(parsed) = syn::parse_file(&current) else {
                panic!("intermediate pass stopped parsing: {current}");
            };
            let mut edits = Vec::new();
            for lint in lints {
                edits.extend(
                    super::super::collect_edits(lint, &current, &parsed)
                        .into_iter()
                        .map(|re| re.edit),
                );
            }
            if edits.is_empty() {
                break;
            }
            let (next, _) = crate::edit_emitter::apply_edits_traced(&current, edits);
            if next == current {
                break;
            }
            current = next;
        }
        current
    }
}
