//! `vec-with-capacity` — `Vec::new()`/`vec![]` + push loop → `Vec::with_capacity(n)`.
//!
//! Pattern: a `let mut v = Vec::new();` (or `vec![]`) binding followed, in
//! the same block OR ANY NESTED STATEMENT BLOCK of the remaining region
//! (Plan 097 T3 — the gen-7 probe found both shapes silent), by a `for` loop
//! that pushes to `v`. The capacity hint is inferred from the loop header:
//!
//! | Loop header | Capacity |
//! |---|---|
//! | `for x in 0..n` | `n` |
//! | `for x in 0..=n` | `n + 1` |
//! | `for x in &items` | `items.len()` |
//! | `for x in items.iter()` / `.iter_mut()` | `items.len()` |
//!
//! `while` loops + unknown iterator expressions are skipped (no capacity
//! source). A shadowing re-`let` of `v` between binding and loop stops the
//! scan — the pushes may belong to the shadow (per level, including inside
//! the descended blocks).
//!
//! ## Nested-block coverage (Plan 097 T3)
//!
//! Two shapes the flat scan missed, both probe-measured silent at Plan 096:
//! (A) the binding AND the loop both inside an `if`/match-arm block — the
//! scan now recurses into nested statement blocks (nested `fn` items and
//! closure bodies stay excluded); (B) the binding at fn level with the loop
//! one block deeper — the later-scan descends block-bearing statements,
//! first-hit in document order. Idents bound in nested blocks join the
//! scope gate (out of scope at the init position; also declines a capacity
//! source naming a param shadowed inside the descended block).
//!
//! ## Parse-only safety gates (Plan 005 T6.2 / Bench 019 G5)
//!
//! The proposer has no type checker, so a rewrite is only emitted when the
//! capacity is **provably usize and in scope at the init position** by
//! syntax alone. Both gates below were added after a real-file G5 run on
//! katgpt-pruners caught compile breaks the 36 fixtures missed:
//!
//! 1. **Scope gate** — the rewritten init sits at the binding, but the
//!    capacity expression may reference a `let` declared LATER in the block
//!    (`E0425`: `Vec::with_capacity(min_regions)` one line before
//!    `let min_regions = ...`). Any single-segment path in the capacity
//!    expression that is let-bound anywhere after the vec binding skips
//!    the rewrite. (Shadowing ambiguity is resolved conservatively:
//!    module-level consts sharing a name with a later `let` are skipped
//!    too.)
//! 2. **usize gate** — `with_capacity` takes `usize`, but a range bound
//!    can be any integer type (`E0308`: `0..BOARD_SIZE` with a `u8` const
//!    — the literal `0` infers to `u8` in the range but `with_capacity`
//!    rejects it). Only these forms are provably usize:
//!    - an integer literal,
//!    - `<expr>.len()` (the std `len` convention; a custom non-usize
//!      `len` is out of scope for this rule),
//!    - usize-preserving arithmetic (`.saturating_sub/.saturating_add/
//!      `.wrapping_sub/.wrapping_add`) on a provably-usize receiver with
//!      an integer-literal argument,
//!    - a plain path naming a fn param explicitly annotated `usize`.
//!      Plain-path bounds that are consts, associated consts, or locals
//!      (`0..n` where `n` is a `let`) are skipped — syn cannot know their
//!      type.
//!
//!    **Verify-mode loosening (Issue 015):** under `--fix --verify`
//!    (`ProposerOpts::untyped_paths`), the usize gate accepts ANY plain
//!    path (single-segment consts/locals, multi-segment associated consts
//!    like `Strategy::COUNT`) — a wrong type surfaces as rustc E0308 and
//!    the [`crate::fix_verify`] loop auto-reverts just that edit. The
//!    scope gate stays ON in both modes (use-before-decl is a real
//!    correctness issue, not a type question).
//!
//! Span-preserving: the edit replaces exactly the `Vec::new()` init text;
//! the capacity fragment is the ORIGINAL source text of the inferred node.

use super::{edit, src, SourceEdit};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprLit, File, Lit, Local, Pat, RangeLimits, Signature, Stmt};

/// Byte range of any node's span (Range<usize> form; convert to a pair when
/// hashing).
fn byte_range<T: Spanned>(t: &T) -> std::ops::Range<usize> {
    t.span().byte_range()
}

use super::ProposerOpts;

pub(crate) fn collect_edits_opts(
    source: &str,
    file: &File,
    opts: ProposerOpts,
) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    super::for_each_fn(file, |sig, block| {
        collect_block_edits(source, sig, block, block, &mut edits, opts);
    });
    edits
}

/// Scan one block for the `let v = Vec::new()` + push-loop pattern, then
/// recurse into the block's nested statement blocks (Plan 097 T3, shape A:
/// the collection itself created inside an `if`/match arm block). `root` is
/// the owning fn's body block — the temp-binding collision domain stays
/// fn-wide (conservative superset). Nested `fn` items are NOT recursed here
/// (`for_each_fn` visits their bodies separately) and neither are closure
/// bodies (their params are invisible to `usize_typed_params`).
fn collect_block_edits(
    source: &str,
    sig: &Signature,
    block: &syn::Block,
    root: &syn::Block,
    edits: &mut Vec<SourceEdit>,
    opts: ProposerOpts,
) {
    let stmts = &block.stmts;
    let usize_params = usize_typed_params(sig);
    for (i, stmt) in stmts.iter().enumerate() {
        let Stmt::Local(local) = stmt else { continue };
        let Some(ident) = fresh_vec_binding(local) else { continue };
        // Idents let-bound at this block level after the vec binding: none of
        // them are in scope at the init position (scope gate). Plan 097 T3:
        // extended with every ident bound in the NESTED blocks of the
        // remaining region — they are equally out of scope at the init
        // position, and a nested re-bind of a usize param name must not
        // vouch for the param's type.
        let mut later_bound: Vec<String> = stmts[i + 1..]
            .iter()
            .filter_map(stmt_local_ident)
            .collect();
        collect_nested_locals(&stmts[i + 1..], &mut later_bound);
        // P2 extension (2026-08-28): the immediately-following statement is
        // a for loop whose iterable is a BARE ARRAY LITERAL (the katgpt-rs
        // bench_120/test_120_toast shape — multi-line `[..]` with non-Copy
        // elements). The literal has no name for `.len()`, so the rewrite
        // binds a temp first: `let tmp = [..];` + `Vec::with_capacity(
        // tmp.len())` + `for .. in tmp`. Safe only when the loop is the
        // IMMEDIATELY-following statement: the literal's evaluation then
        // moves across exactly one statement (the fresh-vec init) — zero
        // reordering across side-effectful code, identical name resolution.
        if i + 1 < stmts.len() {
            if let Some((fl, arr)) = array_iter_push_loop(&stmts[i + 1], ident) {
                if let Some(pair) = temp_binding_edits(source, local, arr, fl, root) {
                    edits.push(pair.0);
                    edits.push(pair.1);
                    continue;
                }
            }
        }
        // Plan 097 T3 (shape B): the pushing loop may live one or more
        // blocks DEEPER than the binding (the flat scan saw only the
        // block-bearing stmt and skipped over it). First-hit semantics
        // preserved: the first capacity-bearing loop in document order
        // wins, even if `cap_replacement` then declines it.
        if let Some(cap) = find_loop_capacity(&stmts[i + 1..], ident) {
            if let Some(replacement) =
                cap_replacement(source, &cap, &usize_params, &later_bound, opts)
            {
                if let Some(init) = &local.init {
                    edits.push(edit(&init.expr, replacement));
                }
            }
        }
    }
    // Shape A: recurse into the nested statement blocks so locals created
    // INSIDE them are scanned with this block's own gates.
    for stmt in &block.stmts {
        for nested in super::stmt_nested_blocks(stmt) {
            collect_block_edits(source, sig, nested, root, edits, opts);
        }
    }
}

/// Every ident let-bound at any nesting depth under these statements
/// (excluding the statements' own top level — the caller collects that
/// directly). Feeds the scope gate for shape-B capacity sources.
fn collect_nested_locals(stmts: &[Stmt], out: &mut Vec<String>) {
    for stmt in stmts {
        for nested in super::stmt_nested_blocks(stmt) {
            for s in &nested.stmts {
                if let Some(id) = stmt_local_ident(s) {
                    out.push(id);
                }
            }
            collect_nested_locals(&nested.stmts, out);
        }
    }
}

/// First capacity-bearing push loop for `ident` in document order,
/// descending through block-bearing statements; a shadow re-bind of
/// `ident` stops the scan at its level.
fn find_loop_capacity(stmts: &[Stmt], ident: &syn::Ident) -> Option<Cap> {
    for stmt in stmts {
        if stmt_binds_ident(stmt, ident) {
            return None;
        }
        if let Some(cap) = loop_capacity(stmt, ident) {
            return Some(cap);
        }
        for nested in super::stmt_nested_blocks(stmt) {
            if let Some(cap) = find_loop_capacity(&nested.stmts, ident) {
                return Some(cap);
            }
        }
    }
    None
}

/// If `stmt` is a `for` loop whose iterable is a BARE array literal and
/// whose body pushes to `ident`, return the loop + the literal.
fn array_iter_push_loop<'a>(stmt: &'a Stmt, ident: &syn::Ident) -> Option<(&'a syn::ExprForLoop, &'a Expr)> {
    let Stmt::Expr(Expr::ForLoop(fl), _) = stmt else { return None };
    let Expr::Array(_) = &*fl.expr else { return None };
    let mut scan = PushScan { ident, found: false };
    scan.visit_block(&fl.body);
    if !scan.found {
        return None;
    }
    Some((fl, &fl.expr))
}

/// Build the two-part temp-binding rewrite. Returns (binding edit, iterable
/// edit) or None when a collision-free temp name cannot be found.
fn temp_binding_edits(
    source: &str,
    local: &Local,
    arr: &Expr,
    fl: &syn::ExprForLoop,
    block: &syn::Block,
) -> Option<(SourceEdit, SourceEdit)> {
    // Collision-free temp name: scan every ident in the whole fn block.
    let mut seen = std::collections::HashSet::new();
    collect_idents(block, &mut seen);
    let mut tmp = String::from("cap_iter");
    let mut n = 1;
    while seen.contains(&tmp) {
        n += 1;
        if n > 9 {
            return None;
        }
        tmp = format!("cap_iter{n}");
    }
    let local_span = src(source, local)?;
    let init = local.init.as_ref()?;
    let local_range = byte_range(local);
    let init_range = byte_range(&*init.expr);
    let rel_start = init_range.start.checked_sub(local_range.start)?;
    let rel_end = init_range.end.checked_sub(local_range.start)?;
    if rel_start > local_span.len() || rel_end > local_span.len() {
        return None;
    }
    let arr_text = src(source, arr)?;
    let indent: String = {
        let line_start = source[..local_range.start]
            .rfind('\n')
            .map_or(0, |p| p + 1);
        source[line_start..local_range.start]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect()
    };
    let new_local = format!(
        "{}Vec::with_capacity({tmp}.len()){}",
        &local_span[..rel_start],
        &local_span[rel_end..]
    );
    let binding_replacement =
        format!("let {tmp} = {arr_text};\n{indent}{new_local}");
    let iterable_replacement = tmp;
    Some((edit(local, binding_replacement), edit(&*fl.expr, iterable_replacement)))
}

/// Every identifier used or bound anywhere in the block (paths, patterns,
/// method names, macro tokens' idents are NOT walked — token streams are
/// opaque, and a synthesized `cap_iter` colliding with an ident inside a
/// macro's tokens would have to also be a real binding to matter, which
/// macros can't bind into this scope).
fn collect_idents(block: &syn::Block, out: &mut std::collections::HashSet<String>) {
    struct IdScan<'a> {
        out: &'a mut std::collections::HashSet<String>,
    }
    impl<'ast> Visit<'ast> for IdScan<'_> {
        fn visit_ident(&mut self, i: &'ast syn::Ident) {
            self.out.insert(i.to_string());
        }
    }
    let mut scan = IdScan { out };
    scan.visit_block(block);
}

/// The capacity, as a node whose ORIGINAL source text becomes the
/// `with_capacity` argument (formatting-preserving).
enum Cap {
    /// Use this node's source text verbatim (`0..n` → `n`).
    Node(syn::Expr),
    /// `<node source> + 1` (`0..=n` → `n + 1`).
    NodePlusOne(syn::Expr),
    /// `<node source>.len()` (`&items` / `items.iter()`).
    LenOf(syn::Expr),
}

/// The binding is `let mut <ident> = Vec::new();` / `= vec![];` (no `else`).
fn fresh_vec_binding(local: &Local) -> Option<&syn::Ident> {
    let Pat::Ident(pi) = &local.pat else { return None };
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
                Some(&pi.ident)
            } else {
                None
            }
        }
        Expr::Macro(mac) if mac.mac.path.is_ident("vec") && mac.mac.tokens.is_empty() => {
            Some(&pi.ident)
        }
        _ => None,
    }
}

/// If `stmt` is a `let`, the first bound ident (for the later-bound set).
fn stmt_local_ident(stmt: &Stmt) -> Option<String> {
    let Stmt::Local(local) = stmt else { return None };
    first_pat_ident(&local.pat)
}

/// First `Pat::Ident` bound anywhere in the pattern (tuple/struct patterns
/// included).
fn first_pat_ident(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(pi) => Some(pi.ident.to_string()),
        Pat::Or(po) => po.cases.iter().find_map(first_pat_ident),
        Pat::Tuple(pt) => pt.elems.iter().find_map(first_pat_ident),
        Pat::Reference(pr) => first_pat_ident(&pr.pat),
        Pat::Slice(ps) => ps.elems.iter().find_map(first_pat_ident),
        Pat::Struct(ps) => ps.fields.iter().find_map(|pf| first_pat_ident(&pf.pat)),
        Pat::Type(pt) => first_pat_ident(&pt.pat),
        _ => None,
    }
}

/// Params explicitly annotated `usize` (provably-usize plain paths).
fn usize_typed_params(sig: &Signature) -> Vec<String> {
    let mut out = Vec::new();
    for arg in &sig.inputs {
        let syn::FnArg::Typed(pt) = arg else { continue };
        let syn::Type::Path(tp) = &*pt.ty else { continue };
        let is_usize = tp.path.segments.len() == 1 && tp.path.segments[0].ident == "usize";
        if is_usize {
            if let Some(name) = first_pat_ident(&pt.pat) {
                out.push(name);
            }
        }
    }
    out
}

/// Does this stmt re-bind `ident` (any mutability)?
fn stmt_binds_ident(stmt: &Stmt, ident: &syn::Ident) -> bool {
    match stmt {
        Stmt::Local(local) => matches!(&local.pat, Pat::Ident(pi) if pi.ident == *ident),
        _ => false,
    }
}

/// If `stmt` is a `for` loop that (a) pushes to `ident` somewhere in its
/// body and (b) has an unambiguous capacity source, return that capacity.
fn loop_capacity(stmt: &Stmt, ident: &syn::Ident) -> Option<Cap> {
    let Stmt::Expr(Expr::ForLoop(fl), _) = stmt else { return None };
    let mut scan = PushScan { ident, found: false };
    scan.visit_block(&fl.body);
    if !scan.found {
        return None;
    }
    capacity_from_iter(&fl.expr)
}

/// Infer the capacity from the for-loop's iterable.
fn capacity_from_iter(expr: &Expr) -> Option<Cap> {
    match expr {
        // `0..n` → `n`; `0..=n` → `n + 1`. Start must be a literal 0.
        Expr::Range(r) => {
            let starts_zero = matches!(
                r.start.as_deref(),
                Some(Expr::Lit(ExprLit { lit: Lit::Int(li), .. }))
                    if li.base10_parse::<u64>().is_ok_and(|v| v == 0)
            );
            if !starts_zero {
                return None;
            }
            let end = r.end.as_deref()?.clone();
            match r.limits {
                RangeLimits::HalfOpen(_) => Some(Cap::Node(end)),
                RangeLimits::Closed(_) => Some(Cap::NodePlusOne(end)),
            }
        }
        // `&items` → `items.len()` (plain path receivers only — field
        // accesses like `&self.items` parse as Expr::Field, skipped).
        Expr::Reference(er) => match &*er.expr {
            Expr::Path(_) => Some(Cap::LenOf((*er.expr).clone())),
            _ => None,
        },
        // `items.iter()` / `items.iter_mut()` → `items.len()`.
        Expr::MethodCall(mc)
            if (mc.method == "iter" || mc.method == "iter_mut") && mc.args.is_empty() =>
        {
            match &*mc.receiver {
                Expr::Path(_) => Some(Cap::LenOf((*mc.receiver).clone())),
                Expr::Reference(er) => match &*er.expr {
                    Expr::Path(_) => Some(Cap::LenOf((*er.expr).clone())),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

/// Build `Vec::with_capacity(<cap text>)` from the ORIGINAL source text of
/// the capacity node, gated by the parse-only safety checks (scope + usize).
/// Under `opts.untyped_paths` (verify-mode), the usize gate accepts any
/// plain path — rustc's E0308 backs the auto-revert.
fn cap_replacement(
    source: &str,
    cap: &Cap,
    usize_params: &[String],
    later_bound: &[String],
    opts: ProposerOpts,
) -> Option<String> {
    let node = match cap {
        Cap::Node(node) => {
            if !capacity_is_usize(node, usize_params, opts)
                || references_later_let(node, later_bound)
            {
                return None;
            }
            format!("Vec::with_capacity({})", usize_text(source, node)?)
        }
        Cap::NodePlusOne(node) => {
            if !capacity_is_usize(node, usize_params, opts)
                || references_later_let(node, later_bound)
            {
                return None;
            }
            format!("Vec::with_capacity({} + 1)", usize_text(source, node)?)
        }
        // The emitted text is `<node>.len()` — always usize; the receiver
        // still must not reference a later `let`.
        Cap::LenOf(node) => {
            if references_later_let(node, later_bound) {
                return None;
            }
            format!("Vec::with_capacity({}.len())", src(source, node)?)
        }
    };
    Some(node)
}

/// Source text for a capacity node, with a non-`usize` integer-literal
/// suffix stripped (`5u64` → `5`): `with_capacity` takes `usize`, and an
/// unsuffixed decimal literal infers it in that position (riir-train
/// gepo_goat lesson — the suffixed emit was an E0308).
fn usize_text(source: &str, node: &Expr) -> Option<String> {
    if let Expr::Lit(ExprLit { lit: Lit::Int(li), .. }) = node {
        let text = src(source, node)?;
        let suffix = li.suffix();
        if suffix.is_empty() || suffix == "usize" {
            return Some(text);
        }
        return Some(text.strip_suffix(suffix).map(str::to_string).unwrap_or(text));
    }
    src(source, node)
}

/// Parse-only usize proof (see module docs for the whitelist).
///
/// With `opts.untyped_paths` (verify-mode, Issue 015): ANY plain path is
/// accepted — single-segment consts/locals AND multi-segment associated
/// consts (`Strategy::COUNT`). Wrong types surface as rustc E0308 and the
/// verifier auto-reverts the edit; the scope gate is unaffected.
fn capacity_is_usize(expr: &Expr, usize_params: &[String], opts: ProposerOpts) -> bool {
    match expr {
        Expr::Lit(ExprLit { lit: Lit::Int(_), .. }) => true,
        // `x.len()` — usize by the std `len` convention.
        Expr::MethodCall(mc) if mc.method == "len" && mc.args.is_empty() => true,
        // usize-preserving arithmetic on a usize base with a literal arg.
        Expr::MethodCall(mc)
            if matches!(
                mc.method.to_string().as_str(),
                "saturating_sub" | "saturating_add" | "wrapping_sub" | "wrapping_add"
            ) && mc.args.len() == 1
                && matches!(mc.args.first(), Some(Expr::Lit(ExprLit { lit: Lit::Int(_), .. })))
                && capacity_is_usize(&mc.receiver, usize_params, opts) =>
        {
            true
        }
        // A plain path is usize only when it names an explicitly-`usize`
        // fn param. Consts / locals / associated consts are type-blind —
        // UNLESS verify-mode loosened the gate (any path, any arity).
        Expr::Path(p) => {
            if opts.untyped_paths {
                return true;
            }
            p.path
                .get_ident()
                .is_some_and(|i| usize_params.iter().any(|param| param == &i.to_string()))
        }
        _ => false,
    }
}

/// Does the expression reference a single-segment path that is let-bound
/// later in the block (i.e. NOT in scope at the init position)?
fn references_later_let(expr: &Expr, later_bound: &[String]) -> bool {
    let mut scan = LaterLetScan { later_bound, found: false };
    scan.visit_expr(expr);
    scan.found
}

struct LaterLetScan<'a> {
    later_bound: &'a [String],
    found: bool,
}

impl<'ast> Visit<'ast> for LaterLetScan<'_> {
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if let Some(ident) = p.path.get_ident() {
            if self.later_bound.iter().any(|n| n == &ident.to_string()) {
                self.found = true;
                return;
            }
        }
        syn::visit::visit_expr_path(self, p);
    }
}

/// Immutable scan: is there a `<ident>.push(_)` method call anywhere in the
/// loop body (including nested ifs + closures)?
struct PushScan<'a> {
    ident: &'a syn::Ident,
    found: bool,
}

impl<'ast> Visit<'ast> for PushScan<'_> {
    fn visit_expr_method_call(&mut self, e: &'ast syn::ExprMethodCall) {
        if self.found {
            return;
        }
        if e.method == "push"
            && e.args.len() == 1
            && matches!(&*e.receiver, Expr::Path(p) if p.path.is_ident(self.ident))
        {
            self.found = true;
            return;
        }
        syn::visit::visit_expr_method_call(self, e);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    fn parses(out: &str) {
        assert!(syn::parse_file(out).is_ok(), "output does not parse: {out}");
    }

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn suffixed_int_bound_gets_suffix_stripped() {
        // riir-train gepo_goat lesson: `0..5u64` emitted `with_capacity(5u64)`
        // — an E0308 (capacity is usize). The bare digits carry the same
        // value and infer usize in that position.
        let src = "fn steps() -> Vec<u64> {\n    let mut v = Vec::new();\n    for i in 0..5u64 { v.push(i); }\n    v\n}";
        let out = propose_fix("vec-with-capacity", src).expect("pattern matched");
        assert!(out.contains("Vec::with_capacity(5)"), "{out}");
        assert!(!out.contains("5u64)"), "suffix must be stripped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn usize_suffix_bound_kept() {
        let src = "fn steps() -> Vec<usize> {\n    let mut v = Vec::new();\n    for i in 0..5usize { v.push(i); }\n    v\n}";
        let out = propose_fix("vec-with-capacity", src).expect("pattern matched");
        assert!(out.contains("Vec::with_capacity(5usize)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn range_loop_gets_capacity() {
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for i in 0..n { v.push(i as u32); }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("Vec::with_capacity(n)"), "{out}");
    }

    #[test]
    fn inclusive_range_gets_plus_one() {
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for i in 0..=n { v.push(i as u32); }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains(&squash("Vec::with_capacity(n + 1)")), "{out}");
    }

    #[test]
    fn ref_iter_gets_len() {
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(items: &Vec<u32>) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for x in &items { v.push(*x); }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("Vec::with_capacity(items.len())"), "{out}");
    }

    #[test]
    fn iter_method_gets_len() {
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(items: &[u32]) -> Vec<u32> {\n    let mut v = vec![];\n    \
             for x in items.iter() { v.push(*x); }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("Vec::with_capacity(items.len())"), "{out}");
    }

    #[test]
    fn push_in_nested_if_counts() {
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(items: &[u32]) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for x in items.iter() { if *x > 3 { v.push(*x); } }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("with_capacity"), "{out}");
    }

    /// Plan 097 T3 shape A — the binding AND the loop both inside an
    /// if-block (the gen-7 probe found this silent: only fn-body top-level
    /// statements were scanned).
    #[test]
    fn nested_block_local_and_loop_get_capacity() {
        let out = propose_fix(
            "vec-with-capacity",
            "pub fn cap_a(items: &[u32]) -> usize {\n    let total = if !items.is_empty() {\n    \
             let mut v = Vec::new();\n        for it in items.iter() { v.push(*it); }\n        v.len()\n    } else {\n        0\n    };\n    total\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains(&squash("Vec::with_capacity(items.len())")), "{out}");
        parses(&out);
    }

    /// Plan 097 T3 shape B — the binding at fn level, the pushing loop one
    /// block deeper (the flat later-scan saw only the if-statement).
    #[test]
    fn outer_local_nested_loop_gets_capacity() {
        let out = propose_fix(
            "vec-with-capacity",
            "pub fn cap_b(items: &[u32]) -> usize {\n    let mut v = Vec::new();\n    if !items.is_empty() {\n    \
             for it in items.iter() { v.push(*it); }\n    }\n    v.len()\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains(&squash("Vec::with_capacity(items.len())")), "{out}");
        parses(&out);
    }

    /// Plan 097 T3 — a nested re-bind of a usize param name inside the
    /// descended block must not vouch for the param's type: the capacity
    /// source `n` may resolve to the shadow, so the rewrite declines.
    #[test]
    fn nested_param_shadow_declines_capacity() {
        let out = propose_fix(
            "vec-with-capacity",
            "pub fn cap_s(n: usize, items: &[u32]) -> usize {\n    let mut v = Vec::new();\n    if n > 0 {\n    \
             let n = n * 2;\n        for x in 0..n { v.push(x); }\n    }\n    v.len()\n}",
        );
        assert!(out.is_none(), "shadowed param capacity must decline: {out:?}");
    }

    #[test]
    fn impl_method_rewrites() {
        let out = propose_fix(
            "vec-with-capacity",
            "struct S;\nimpl S {\n    fn build(&self, n: usize) -> Vec<u8> {\n        \
             let mut v = Vec::new();\n        for i in 0..n { v.push(i as u8); }\n        v\n    }\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("Vec::with_capacity(n)"), "{out}");
    }

    #[test]
    fn len_rooted_arithmetic_is_usize() {
        // `.len()`-rooted `saturating_sub(<literal>)` is usize by construction
        // (the katgpt-pruners bandit_bridge / path_sampler shape).
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(t: &Trajectory) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for i in 0..t.path.len().saturating_sub(1) { v.push(i as u32); }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(
            squash(&out).contains(&squash("Vec::with_capacity(t.path.len().saturating_sub(1))")),
            "{out}"
        );
    }

    #[test]
    fn later_let_bound_skipped() {
        // G5 scope-gate: the capacity text would land BEFORE `min_regions`'s
        // `let` (E0425) — the katgpt-pruners bfcp_region_cache shape.
        assert!(propose_fix(
            "vec-with-capacity",
            "fn build(a: &[u32], b: &[u32]) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             let m = a.len().min(b.len());\n    for i in 0..m { v.push(a[i]); }\n    v\n}",
        )
        .is_none());
    }

    #[test]
    fn later_let_bound_len_form_skipped() {
        // Same scope gate for the LenOf form (`&items` where `items` is a
        // later `let`) — the katgpt-pruners reflection.rs shape.
        assert!(propose_fix(
            "vec-with-capacity",
            "fn build(pairs: &[u32]) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             let items: Vec<u32> = pairs.to_vec();\n    for x in &items { v.push(*x); }\n    v\n}",
        )
        .is_none());
    }

    #[test]
    fn non_usize_param_bound_skipped() {
        // G5 usize gate: `n: u32` is a plain path that is provably NOT usize.
        assert!(propose_fix(
            "vec-with-capacity",
            "fn build(n: u32) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for i in 0..n { v.push(i); }\n    v\n}",
        )
        .is_none());
    }

    #[test]
    fn method_bound_not_len_skipped() {
        // `cfg.count()` is not parse-only-provably usize (G5 usize gate) —
        // the katgpt-pruners map_generator `num_floors.saturating_sub(1)`
        // class: arithmetic on a plain-path receiver.
        assert!(propose_fix(
            "vec-with-capacity",
            "fn build(cfg: &Cfg) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for i in 0..cfg.count() { v.push(i as u32); }\n    v\n}",
        )
        .is_none());
    }

    #[test]
    fn no_loop_leaves_unchanged() {
        assert!(
            propose_fix(
                "vec-with-capacity",
                "fn f() { let mut v = Vec::new(); v.push(1); }"
            )
            .is_none()
        );
    }

    #[test]
    fn while_loop_skipped() {
        assert!(
            propose_fix(
                "vec-with-capacity",
                "fn f(mut it: std::vec::IntoIter<u32>) -> Vec<u32> {\n    let mut v = \
                 Vec::new();\n    while let Some(x) = it.next() { v.push(x); }\n    v\n}"
            )
            .is_none(),
            "while loops have no capacity source"
        );
    }

    #[test]
    fn shadowing_stops_scan() {
        assert!(propose_fix(
            "vec-with-capacity",
            "fn f(items: &[u32]) -> Vec<u32> {\n    let mut v = Vec::new();\n    let mut v = \
             Vec::new();\n    for x in items { v.push(*x); }\n    v\n}"
        )
        .is_none());
    }

    #[test]
    fn already_with_capacity_unchanged() {
        assert!(
            propose_fix(
                "vec-with-capacity",
                "fn f(n: usize) -> Vec<u32> { let mut v = Vec::with_capacity(n); v }"
            )
            .is_none()
        );
    }

    #[test]
    fn nonzero_range_start_skipped() {
        assert!(propose_fix(
            "vec-with-capacity",
            "fn f(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    for i in 1..n { \
             v.push(i as u32); }\n    v\n}"
        )
        .is_none());
    }

    #[test]
    fn bare_path_iterable_unchanged() {
        // `for x in items` (bare path): the binding could be an iterator
        // without `.len()` — emitting `items.len()` might not compile, so the
        // safe design skips it. Use `&items` / `.iter()` / a range instead.
        assert!(
            propose_fix(
                "vec-with-capacity",
                "fn build(items: &[u32]) -> Vec<u32> {\n    let mut v = Vec::new();\n    \n                 for x in items { v.push(*x); }\n    v\n}"
            )
            .is_none()
        );
    }

    // ── P2 extension (2026-08-28): bare array-literal iterables — the
    // katgpt-rs bench_120 / test_120_toast_renyi shape (multi-line `[..]`,
    // non-Copy elements, no name for `.len()`). The rewrite binds a temp
    // first; fires only when the loop is the IMMEDIATELY-following
    // statement (zero semantic reordering — the literal's evaluation moves
    // across exactly the fresh-vec init). Probe 2026-08-28: compiles,
    // behavior identical, capacity hint lands. ──────────────────────────
    #[test]
    fn array_literal_iterable_gets_temp_binding() {
        let src = concat!(
            "fn build() -> Vec<Vec<u8>> {\n",
            "    let mut v = Vec::new();\n",
            "    for tok in [\n",
            "        b\"<pad>\".to_vec(),\n",
            "        b\"<bos>\".to_vec(),\n",
            "    ] {\n",
            "        v.push(tok);\n",
            "    }\n",
            "    v\n",
            "}"
        );
        let out = propose_fix("vec-with-capacity", src).expect("array-literal shape matched");
        assert!(out.contains("let cap_iter = ["), "temp binding missing: {out}");
        assert!(
            out.contains("Vec::with_capacity(cap_iter.len())"),
            "capacity hint missing: {out}"
        );
        assert!(out.contains("for tok in cap_iter {"), "iterable not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn array_literal_name_collision_gets_suffix() {
        let src = concat!(
            "fn build(cap_iter: u32) -> Vec<u32> {\n",
            "    let mut v = Vec::new();\n",
            "    for x in [1u32, 2, 3] {\n",
            "        v.push(x + cap_iter);\n",
            "    }\n",
            "    v\n",
            "}"
        );
        let out = propose_fix("vec-with-capacity", src).expect("matched");
        assert!(
            out.contains("let cap_iter2 = [1u32, 2, 3];"),
            "collision-free temp name missing: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn array_literal_non_adjacent_loop_skipped() {
        // The loop is NOT the immediately-following statement — the temp
        // binding would reorder the literal's evaluation across the
        // intervening statement. Stays skipped.
        let src = concat!(
            "fn build(flag: bool) -> Vec<u32> {\n",
            "    let mut v = Vec::new();\n",
            "    if flag { println!(\"go\"); }\n",
            "    for x in [1u32, 2, 3] {\n",
            "        v.push(x);\n",
            "    }\n",
            "    v\n",
            "}"
        );
        assert!(
            propose_fix("vec-with-capacity", src).is_none(),
            "non-adjacent loop must stay guarded"
        );
    }

    #[test]
    fn array_literal_reference_iterable_skipped() {
        // `for x in &[..]` — the temp would bind the ARRAY and rewrite the
        // iterable to a bare path, changing the iteration mode
        // (by-value vs by-reference). Stays skipped.
        let src = concat!(
            "fn build() -> Vec<u32> {\n",
            "    let mut v = Vec::new();\n",
            "    for x in &[1u32, 2, 3] {\n",
            "        v.push(*x);\n",
            "    }\n",
            "    v\n",
            "}"
        );
        assert!(
            propose_fix("vec-with-capacity", src).is_none(),
            "reference-literal iterable must stay guarded"
        );
    }

    // ── Verify-mode (Issue 015): untyped_paths loosens the usize gate,
    // rustc E0308 backs the revert. The scope gate stays ON. ────────────

    fn propose_verify(src: &str) -> Option<String> {
        let file = syn::parse_file(src).ok()?;
        let edits = super::collect_edits_opts(
            src,
            &file,
            super::super::ProposerOpts { untyped_paths: true },
        );
        if edits.is_empty() {
            return None;
        }
        Some(super::super::apply_edits(src, edits.clone()))
    }

    #[test]
    fn untyped_const_bound_fires_under_verify() {
        // The collapse_detector `N_TRACES: usize` shape — skipped by the
        // usize gate, attempted under --verify (compiles: it IS usize).
        let out = propose_verify(
            "fn traces() -> Vec<u32> {\n    const N_TRACES: usize = 24;\n    \n             let mut v = Vec::new();\n    for i in 0..N_TRACES { v.push(i as u32); }\n    v\n}"
        )
        .expect("verify-mode fires on the const bound");
        assert!(squash(&out).contains(&squash("Vec::with_capacity(N_TRACES)")), "{out}");
    }

    #[test]
    fn untyped_associated_const_fires_under_verify() {
        // The monopoly `Strategy::COUNT` shape — multi-segment associated
        // const, skipped by the usize gate (get_ident() is None).
        let out = propose_verify(
            "fn picks() -> Vec<u32> {\n    let mut v = Vec::new();\n    \n             for i in 0..Strategy::COUNT { v.push(i as u32); }\n    v\n}"
        )
        .expect("verify-mode fires on the associated const");
        assert!(
            squash(&out).contains(&squash("Vec::with_capacity(Strategy::COUNT)")),
            "{out}"
        );
    }

    #[test]
    fn untyped_earlier_local_fires_under_verify() {
        // The episode_pruner `min_len` shape — a local `let` BEFORE the vec
        // binding (in scope at the init position, so the scope gate passes).
        let out = propose_verify(
            "fn diff(a: &[u32], b: &[u32]) -> Vec<u32> {\n    let min_len = \n             a.len().min(b.len());\n    let mut v = Vec::new();\n    for pos in 0..min_len { v.push(a[pos]); }\n    v\n}"
        )
        .expect("verify-mode fires on the earlier local");
        assert!(squash(&out).contains(&squash("Vec::with_capacity(min_len)")), "{out}");
    }

    #[test]
    fn untyped_scope_gate_still_blocks_later_let() {
        // Verify-mode does NOT loosen the scope gate — the E0425 class is a
        // correctness issue, not a type question.
        let src = "fn build(a: &[u32], b: &[u32]) -> Vec<u32> {\n    let mut v = Vec::new();\n    \n             let m = a.len().min(b.len());\n    for i in 0..m { v.push(a[i]); }\n    v\n}";
        let file = syn::parse_file(src).unwrap();
        let edits = super::collect_edits_opts(
            src,
            &file,
            super::super::ProposerOpts { untyped_paths: true },
        );
        assert!(edits.is_empty(), "scope gate must hold under verify-mode");
    }

    #[test]
    fn untyped_u8_const_also_fires_relying_on_revert() {
        // The monopoly BOARD_SIZE u8 shape — the E0308 case. Under verify it
        // FIRES (the revert handles it); under parse-only it must not.
        let src = "fn cells() -> Vec<u8> {\n    let mut v = Vec::new();\n    \n             for i in 0..BOARD_SIZE { v.push(i); }\n    v\n}";
        let out = propose_verify(src).expect("verify-mode attempts the u8 const");
        assert!(squash(&out).contains(&squash("Vec::with_capacity(BOARD_SIZE)")), "{out}");
        // Parse-only (the default) still skips it.
        assert!(propose_fix("vec-with-capacity", src).is_none());
    }
}
