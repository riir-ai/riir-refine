//! `length-prefix-validate-before-alloc` — SUGGEST-ONLY detector
//! (Batch 131, the Girder distill; Plan 112 T4).
//!
//! A length parsed inside a fn (`u32::from_be_bytes(..)`-class byte-swap,
//! or a `.parse()` of a header field) flowing into a `vec![0u8; len]`
//! repeat allocation with NO intervening comparison of that length against
//! a bound is an allocation-size DoS: one peer-controlled header can
//! demand a multi-GB buffer (OOM/abort). The cost axis is security, not
//! cycles.
//!
//! ## Why suggest-only (never a fix)
//!
//! The bounded repair must INVENT a limit constant and an error-return
//! path the enclosing fn may not have (`read_frame` may return `()`) —
//! a semantic decision, never mechanical (the
//! `ffi-export-panicking-body` precedent). This module therefore ships a
//! parse-only DETECTION walk only: NO `FIXABLE_SEC` membership, NO
//! `collect_edits` arm — the suggestion surfaces through retrieval
//! (`--suggest` / the corpus entry) and review.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Same-fn chain only**: the parse and the allocation must share the
//!   enclosing fn (the `nan-unsafe-sort-comparator` helper-fn precedent —
//!   cross-fn length origins are an honest recall limit, not a match).
//! - **Identity tracking**: the alloc's repeat count must be a bare path
//!   ident bound (transitively) to the parse — through `as usize` casts,
//!   `?`/`ok_or`/`unwrap` unwraps, `Some(..)` wrappers and assignments
//!   (the `Option<usize>` header-variable shape), and arithmetic
//!   (`len * 4` stays a peer-controlled magnitude).
//! - **NO intervening validation**: any comparison (`<`/`>`/`<=`/`>=`/
//!   `==`/`!=`) or `match`/`assert!` scrutinee involving the tracked
//!   ident between the parse and the alloc marks it validated — the
//!   bounded twin (`if len == 0 || len > MAX { return Err(..) }` before
//!   `vec![0; len]`) never fires. Validating a derived length validates
//!   its sources (`n = len * 4` checked bounds `len`).
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped
//!   (frame-parsing fixtures live there by design); since this detector
//!   emits no edits, interior comments cannot be displaced.
//! - **`vec!` repeat form only**: `with_capacity(n)` is excluded by
//!   design (trusted-metadata sizing is the convention; the negative
//!   suite pins it).
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Cross-fn origins: a helper that parses and a caller that allocates
//!   are not connected parse-only.
//! - Bare-ident counts only: `self.header_len`-class field/method origins
//!   are out (their types are unprovable parse-only).
//! - The shape proves a MISSING VALIDATION, not an untrusted source — a
//!   site parsing a trusted local file's length fires too, and review
//!   decides. Suggest-only is the blast-radius control.

use std::ops::Range;

use proc_macro2::{Delimiter, Group};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{BinOp, Expr, ExprMethodCall, File, Item, Stmt};

/// One detected unbounded length-prefix allocation.
#[derive(Debug, Clone)]
pub struct FrameAllocFinding {
    /// Byte range of the `vec![..; len]` expression.
    pub alloc_span: Range<usize>,
    /// The alloc expression's source text.
    pub alloc_text: String,
    /// The tracked length ident at the repeat count.
    pub length_ident: String,
    /// The advice line (the bounded form + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "validate the length BEFORE the allocation: \
`if len == 0 || len > MAX { return Err(..) }` — a peer-controlled length \
prefix flowing into vec![0u8; len] is an allocation-size DoS (Batch 131, \
suggest-only)";

/// Detect unbounded `vec![..; parsed_len]` allocations (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<FrameAllocFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<FrameAllocFinding>) {
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
            // fixture habitat, not production wire paths.
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
    m.attrs.iter().any(|a| {
        a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
    })
}

/// The per-fn tracking state.
#[derive(Default)]
struct FnState {
    /// ident -> its tracking parents (sources it was derived from).
    tracked: std::collections::HashMap<String, Vec<String>>,
    /// idents that saw a bound comparison/match/assert before an alloc.
    validated: std::collections::HashSet<String>,
}

impl FnState {
    fn track(&mut self, ident: &str, parents: Vec<String>) {
        self.tracked.insert(ident.to_string(), parents);
    }

    fn is_tracked(&self, ident: &str) -> bool {
        self.tracked.contains_key(ident)
    }

    /// Validating `ident` also validates every tracked ancestor
    /// (`n = len * 4` checked bounds `len`).
    fn validate(&mut self, ident: &str) {
        let mut queue = vec![ident.to_string()];
        while let Some(id) = queue.pop() {
            if self.validated.insert(id.clone()) {
                if let Some(parents) = self.tracked.get(&id) {
                    queue.extend(parents.iter().cloned());
                }
            }
        }
    }
}

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<FrameAllocFinding>) {
    let mut state = FnState::default();
    scan_stmts(source, &block.stmts, &mut state, findings);
}

/// Walk statements IN ORDER: bindings track, validations suppress, allocs
/// fire — the same-name-later ordering is what makes the gates honest.
fn scan_stmts(source: &str, stmts: &[Stmt], state: &mut FnState, findings: &mut Vec<FrameAllocFinding>) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    // `let len = <parse-chain>` → track the bound idents.
                    for pat_ident in local_idents(&l.pat) {
                        if let Some(parents) = harvest_chain(&init.expr, state) {
                            state.track(&pat_ident, parents);
                        }
                    }
                    check_validation(&init.expr, state);
                    scan_expr(source, &init.expr, state, findings);
                }
            }
            Stmt::Item(item) => collect_in_item(source, item, findings),
            Stmt::Expr(e, _) => {
                // `content_length = Some(parse(..))` — the header-variable
                // assignment shape tracks the assigned ident too.
                if let Expr::Assign(a) = e {
                    if let Some(parents) = harvest_chain(&a.right, state) {
                        if let Some(name) = path_ident(&a.left) {
                            state.track(&name, parents);
                        }
                    }
                    check_validation(&a.right, state);
                    scan_expr(source, &a.right, state, findings);
                } else {
                    check_validation(e, state);
                    scan_expr(source, e, state, findings);
                }
            }
            Stmt::Macro(_) => {}
        }
    }
}

/// The `let`-pattern's plain idents (bare + tuple patterns; wildcards and
/// struct patterns are skipped — conservative).
fn local_idents(pat: &syn::Pat) -> Vec<String> {
    let mut out = Vec::new();
    walk_pat(pat, &mut out);
    out
}

fn walk_pat(pat: &syn::Pat, out: &mut Vec<String>) {
    match pat {
        syn::Pat::Ident(p) => out.push(p.ident.to_string()),
        syn::Pat::Tuple(t) => {
            for p in &t.elems {
                walk_pat(p, out);
            }
        }
        syn::Pat::Type(p) => walk_pat(&p.pat, out),
        syn::Pat::Paren(p) => walk_pat(&p.pat, out),
        syn::Pat::Reference(r) => walk_pat(&r.pat, out),
        _ => {}
    }
}

fn path_ident(e: &Expr) -> Option<String> {
    if let Expr::Path(p) = e {
        if p.qself.is_none() && p.path.segments.len() == 1 {
            return Some(p.path.segments[0].ident.to_string());
        }
    }
    None
}

/// Is this expr a length-parse site (`from_be_bytes(..)` assoc call or any
/// `.parse(..)` method call)?
fn is_parse_site(e: &Expr) -> bool {
    match e {
        Expr::Call(c) => match c.func.as_ref() {
            Expr::Path(p) => p
                .path
                .segments
                .last()
                .is_some_and(|s| {
                    matches!(
                        s.ident.to_string().as_str(),
                        "from_be_bytes" | "from_le_bytes" | "from_ne_bytes"
                    )
                }),
            _ => false,
        },
        Expr::MethodCall(m) => m.method == "parse",
        _ => false,
    }
}

/// If `expr` transparently derives from a tracked ident (or IS a parse
/// site), return the parent idents for the newly tracked binding.
/// Transparent wrappers: casts, `?`, `ok_or`/`ok_or_else`/`unwrap`/
/// `unwrap_or`/`unwrap_or_else`/`expect`/`map_err`, `Some(..)`, and
/// arithmetic on a tracked magnitude (a peer-controlled magnitude squared
/// is still one).
fn harvest_chain(expr: &Expr, state: &FnState) -> Option<Vec<String>> {
    // A fresh parse site binds with no parents.
    if is_parse_site(expr) {
        return Some(Vec::new());
    }
    let from = |e: &Expr| -> Option<Vec<String>> {
        let id = path_ident(e)?;
        state.is_tracked(&id).then(|| vec![id])
    };
    match expr {
        Expr::Cast(c) => harvest_chain(&c.expr, state),
        Expr::Try(t) => harvest_chain(&t.expr, state),
        Expr::Paren(p) => harvest_chain(&p.expr, state),
        Expr::Call(c) => {
            // `Some(<tracked>)`
            if matches!(c.func.as_ref(), Expr::Path(p) if p.path.segments.len() == 1 && p.path.segments[0].ident == "Some")
            {
                if let Some(first) = c.args.first() {
                    return harvest_chain(first, state);
                }
            }
            None
        }
        Expr::MethodCall(m) => {
            if matches!(
                m.method.to_string().as_str(),
                "ok_or"
                    | "ok_or_else"
                    | "unwrap"
                    | "unwrap_or"
                    | "unwrap_or_else"
                    | "expect"
                    | "map_err"
            ) {
                return harvest_chain(&m.receiver, state);
            }
            None
        }
        Expr::Binary(b) => {
            if matches!(b.op, BinOp::Add(_) | BinOp::Mul(_) | BinOp::Shl(_) | BinOp::Sub(_) | BinOp::Div(_) | BinOp::Shr(_))
            {
                let mut parents = harvest_chain(&b.left, state).unwrap_or_default();
                parents.extend(harvest_chain(&b.right, state).unwrap_or_default());
                if parents.is_empty() { None } else { Some(parents) }
            } else {
                None
            }
        }
        _ => from(expr),
    }
}

/// Mark every tracked ident mentioned in a comparison / match scrutinee /
/// assert as validated (the bounded twin's gate).
fn check_validation(expr: &Expr, state: &mut FnState) {
    struct V<'a>(&'a mut FnState);
    impl Visit<'_> for V<'_> {
        fn visit_expr(&mut self, e: &Expr) {
            if let Expr::Binary(b) = e {
                if matches!(
                    b.op,
                    BinOp::Lt(_) | BinOp::Gt(_) | BinOp::Le(_) | BinOp::Ge(_) | BinOp::Eq(_) | BinOp::Ne(_)
                ) {
                    let mut found = Vec::new();
                    collect_tracked(&b.left, &self.0.tracked, &mut found);
                    collect_tracked(&b.right, &self.0.tracked, &mut found);
                    for id in found {
                        self.0.validate(&id);
                    }
                }
            }
            if let Expr::Match(m) = e {
                let mut found = Vec::new();
                collect_tracked(&m.expr, &self.0.tracked, &mut found);
                for id in found {
                    self.0.validate(&id);
                }
            }
            syn::visit::visit_expr(self, e);
        }
    }
    // `assert!`/`debug_assert!` carry their comparison inside macro tokens.
    if let Expr::Macro(mac) = expr {
        if mac
            .mac
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident.to_string().starts_with("assert"))
        {
            if let Ok(inner) = syn::parse2::<Expr>(mac.mac.tokens.clone()) {
                check_validation(&inner, state);
            }
        }
    }
    let mut v = V(state);
    v.visit_expr(expr);
}

fn collect_tracked(e: &Expr, tracked: &std::collections::HashMap<String, Vec<String>>, out: &mut Vec<String>) {
    struct W<'a> {
        tracked: &'a std::collections::HashMap<String, Vec<String>>,
        out: &'a mut Vec<String>,
    }
    impl Visit<'_> for W<'_> {
        fn visit_expr(&mut self, e: &Expr) {
            if let Some(id) = path_ident(e) {
                if self.tracked.contains_key(&id) && !self.out.contains(&id) {
                    self.out.push(id);
                }
            }
            syn::visit::visit_expr(self, e);
        }
    }

    if let Some(id) = path_ident(e) {
        if tracked.contains_key(&id) {
            out.push(id);
        }
        return;
    }
    let mut w = W { tracked, out };
    w.visit_expr(e);
}

/// In-order expr walk: validations first (a comparison anywhere in this
/// expr suppresses everything below it), then the alloc check, then
/// recursion.
fn scan_expr(source: &str, expr: &Expr, state: &mut FnState, findings: &mut Vec<FrameAllocFinding>) {
    check_validation(expr, state);
    if let Expr::Macro(mac) = expr {
        if mac
            .mac
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident == "vec")
        {
            // `vec![init; count]`: the macro tokens carry `[init; count]`
            // WITHOUT the delimiters — re-wrap in brackets so the token
            // stream parses as the array-repeat expression it expands to.
            let wrapped = Group::new(Delimiter::Bracket, mac.mac.tokens.clone()).to_token_stream();
            if let Ok(Expr::Repeat(rep)) = syn::parse2::<Expr>(wrapped) {
                if let Some(id) = path_ident(&rep.len) {
                    if state.is_tracked(&id) && !state.validated.contains(&id) {
                        let span = expr.span().byte_range();
                        if span.end <= source.len() {
                            findings.push(FrameAllocFinding {
                                alloc_text: source[span.clone()].to_string(),
                                alloc_span: span,
                                length_ident: id,
                                advice: ADVICE,
                            });
                        }
                    }
                }
            }
            return; // do not descend into the macro tokens
        }
    }
    // Recurse in syntactic order.
    match expr {
        Expr::If(i) => {
            scan_expr(source, &i.cond, state, findings);
            scan_stmts(source, &i.then_branch.stmts, state, findings);
            if let Some((_, else_e)) = &i.else_branch {
                scan_expr(source, else_e, state, findings);
            }
        }
        Expr::Match(m) => {
            scan_expr(source, &m.expr, state, findings);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    scan_expr(source, guard, state, findings);
                }
                scan_expr(source, &arm.body, state, findings);
            }
        }
        Expr::Block(b) => scan_stmts(source, &b.block.stmts, state, findings),
        Expr::Unsafe(u) => scan_stmts(source, &u.block.stmts, state, findings),
        Expr::Loop(l) => scan_stmts(source, &l.body.stmts, state, findings),
        Expr::While(w) => {
            scan_expr(source, &w.cond, state, findings);
            scan_stmts(source, &w.body.stmts, state, findings);
        }
        Expr::ForLoop(f) => {
            scan_expr(source, &f.expr, state, findings);
            scan_stmts(source, &f.body.stmts, state, findings);
        }
        Expr::MethodCall(ExprMethodCall { receiver, args, .. }) => {
            scan_expr(source, receiver, state, findings);
            for a in args {
                scan_expr(source, a, state, findings);
            }
        }
        Expr::Call(c) => {
            scan_expr(source, &c.func, state, findings);
            for a in &c.args {
                scan_expr(source, a, state, findings);
            }
        }
        Expr::Assign(a) => {
            scan_expr(source, &a.right, state, findings);
        }
        Expr::Binary(b) => {
            scan_expr(source, &b.left, state, findings);
            scan_expr(source, &b.right, state, findings);
        }
        Expr::Closure(c) => scan_expr(source, &c.body, state, findings),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::FrameAllocFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 131 gate — the UNBOUNDED wire-frame shape (original fixture,
    /// the corpus buggy_code) MUST fire.
    #[test]
    fn batch131_unbounded_from_be_bytes_fires() {
        let src = "pub fn read_frame(stream: &mut std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n    \
             let mut prefix = [0_u8; 4];\n    \
             std::io::Read::read_exact(stream, &mut prefix)?;\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let mut body = vec![0_u8; len];\n    \
             std::io::Read::read_exact(stream, &mut body)?;\n    \
             Ok(body)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].length_ident, "len");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The header-string shape (`.parse()` through an Option variable and
    /// `ok_or_else(..)?`) MUST fire — the second evidence form.
    #[test]
    fn batch131_header_parse_through_option_fires() {
        let src = "pub fn read_body(reader: &mut std::io::BufReader<std::net::TcpStream>) -> Result<Vec<u8>, String> {\n    \
             let mut content_length: Option<usize> = None;\n    \
             let header = String::from(\"Content-Length: 4\");\n    \
             let rest = header.strip_prefix(\"Content-Length: \").unwrap_or(\"\");\n    \
             content_length = Some(rest.trim().parse().map_err(|e: std::num::ParseIntError| e.to_string())?);\n    \
             let len = content_length.ok_or_else(|| String::from(\"missing\"))?;\n    \
             let body = vec![0u8; len];\n    \
             Ok(body)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].length_ident, "len");
    }

    // ── NEGATIVE SUITE — every class must stay SILENT (zero-FP bar) ──

    /// The BOUNDED twin: the validation comparison precedes the alloc.
    #[test]
    fn batch131_bounded_twin_is_silent() {
        let src = "pub const MAX_FRAME_BYTES: usize = 1 << 20;\n\n\
             pub fn read_frame(stream: &mut std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n    \
             let mut prefix = [0_u8; 4];\n    \
             std::io::Read::read_exact(stream, &mut prefix)?;\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             if len == 0 || len > MAX_FRAME_BYTES {\n        \
             return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, \"bad\"));\n    \
             }\n    \
             let mut body = vec![0_u8; len];\n    \
             std::io::Read::read_exact(stream, &mut body)?;\n    \
             Ok(body)\n}\n";
        assert!(findings(src).is_empty(), "bounded twin must not fire");
    }

    /// Trusted length: `vec![0; buf.len()]` — the count is not a tracked
    /// parse local.
    #[test]
    fn batch131_trusted_buf_len_is_silent() {
        let src = "pub fn pad(buf: &[u8]) -> Vec<u8> {\n    \
             let mut out = vec![0_u8; buf.len()];\n    \
             out[..buf.len()].copy_from_slice(buf);\n    \
             out\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Const-size allocations (array literal + literal vec) are silent.
    #[test]
    fn batch131_const_sizes_are_silent() {
        let src = "pub fn headers() -> ([u8; 64], Vec<u8>) {\n    \
             let arr = [0_u8; 64];\n    \
             let v = vec![0_u8; 64];\n    \
             (arr, v)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Empty vecs (`Vec::new()` / `vec![]`) are silent.
    #[test]
    fn batch131_empty_vecs_are_silent() {
        let src = "pub fn fresh() -> (Vec<u8>, Vec<u8>) {\n    \
             (Vec::new(), vec![])\n}\n";
        assert!(findings(src).is_empty());
    }

    /// `with_capacity` from trusted metadata is OUT OF SCOPE by design —
    /// the negative suite pins the boundary.
    #[test]
    fn batch131_with_capacity_is_out_of_scope() {
        let src = "pub fn build(n: usize) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(n * 16);\n    \
             v.resize(n * 16, 0);\n    \
             v\n}\n";
        assert!(findings(src).is_empty());
    }

    /// HONEST RECALL LIMIT pinned as a test: the parse in a helper fn and
    /// the alloc in the caller are NOT connected parse-only (the
    /// nan_safe_cmp helper-fn precedent).
    #[test]
    fn batch131_cross_fn_origin_is_undetected() {
        let src = "fn parse_len(prefix: [u8; 4]) -> usize {\n    \
             u32::from_be_bytes(prefix) as usize\n}\n\n\
             pub fn read_frame(stream: &mut std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n    \
             let mut prefix = [0_u8; 4];\n    \
             std::io::Read::read_exact(stream, &mut prefix)?;\n    \
             let len = parse_len(prefix);\n    \
             let mut body = vec![0_u8; len];\n    \
             std::io::Read::read_exact(stream, &mut body)?;\n    \
             Ok(body)\n}\n";
        assert!(findings(src).is_empty(), "cross-fn origins stay undetected");
    }

    /// A field/method length origin (`self.header_len`) is out — bare
    /// idents only.
    #[test]
    fn batch131_field_origin_is_undetected() {
        let src = "pub struct Wire { pub header_len: usize }\n\n\
             impl Wire {\n    \
             pub fn body(&self) -> Vec<u8> {\n        vec![0_u8; self.header_len]\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// #[cfg(test)] mods are skipped (the comment guard, house discipline):
    /// the same unbounded shape inside a test mod never fires.
    #[test]
    fn batch131_test_mod_is_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             #[test]\n    fn t() {\n        \
             let n: usize = \"8\".parse().unwrap();\n        \
             let _b = vec![0u8; n];\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A length used in a `match` scrutinee counts as validated.
    #[test]
    fn batch131_match_scrutinee_validates() {
        let src = "pub fn f(prefix: [u8; 4]) -> Vec<u8> {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             match len {\n        0 => Vec::new(),\n        _ => vec![0_u8; len],\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Validating a DERIVED length bounds the source too.
    #[test]
    fn batch131_derived_validation_bounds_the_source() {
        let src = "pub const ROW: usize = 16;\n\n\
             pub fn f(prefix: [u8; 4]) -> Vec<u8> {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let n = len * ROW;\n    \
             if n > 1 << 20 {\n        return Vec::new();\n    }\n    \
             vec![0_u8; n]\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A derived magnitude with NO validation still fires
    /// (`len * 4` is as peer-controlled as `len`).
    #[test]
    fn batch131_derived_magnitude_without_validation_fires() {
        let src = "pub fn f(prefix: [u8; 4]) -> Vec<u8> {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let n = len * 4;\n    \
             vec![0_u8; n]\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].length_ident, "n");
    }

    /// The findings carry byte spans into the source (span integrity).
    #[test]
    fn batch131_finding_spans_slice_the_source() {
        let src = "pub fn f(prefix: [u8; 4]) -> Vec<u8> {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let body = vec![0_u8; len];\n    \
             body\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1);
        assert_eq!(&src[f[0].alloc_span.clone()], "vec![0_u8; len]");
    }
}
