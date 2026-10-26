//! `packed-field-silent-overflow-wrong-key` — SUGGEST-ONLY detector
//! (Batch 294, the wasmtime RUSTSEC-2026-0380 distill; Research 332 /
//! Plan 298 T1.2).
//!
//! A count/ID masked to a fixed bit-width (`& 0xFF` … `& 0xFFFF_FFFF`, the
//! 2^k−1 literals) and then used as a LOOKUP KEY silently loses every bit
//! above the mask: values whose legitimate range exceeds the field width
//! alias onto wrong keys — wrong-item access with no panic, the exact
//! wasmtime 24-bit packed value-number failure (CVE-2026-115889: packed
//! fields overflow silently in release builds, the corrupt value keys a
//! lookup, type confusion follows). The mask IS the silent truncation:
//! `&` is deliberately absent from `unchecked-wire-arithmetic`'s operator
//! set, so this site is invisible to that rule.
//!
//! ## The B258 recorded-drop adjudication (in-rule, per the plan)
//!
//! Batch 258 dropped "integer overflow/truncation 64→32" as owned: the
//! bare-op overflow half by `unchecked-wire-arithmetic`, the blanket
//! `as`-narrowing half by clippy `cast_possible_truncation`. THIS rule
//! owns neither half: the PACKING SITE — a mask literal keying a lookup —
//! is a third sink neither instrument sees (`cast_possible_truncation` is
//! a blanket `as`-cast pedantic lint with no key-usage scoping and no fix;
//! the mask never mentions `as`). Sink-level novelty per the Plan 156 T2
//! pattern. Research 161's renewal check also clears: its dropped set
//! (recursion / `set_len` / uninit / symlink) does not overlap.
//!
//! ## Why suggest-only (never a fix)
//!
//! The bounded repair is a range assert at the packing site
//! (`assert!(id <= MASK)`), a checked newtype that refuses construction
//! past the field width, or widening the field — which one fits is a
//! protocol decision (the corrupt key may be a designed alias — see the
//! honest FP classes). NO `FIXABLE_SEC` membership, NO `collect_edits`
//! arm — the suggestion surfaces through retrieval and review.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Lookup-key scoping**: the masked expression must BE an index key
//!   (`table[key]`, direct or through a binding) — a mask used for a
//!   checksum byte, a bitfield round-trip, or a bucket count stays silent.
//! - **Origin gate**: the unmasked operand must be a same-fn parse origin
//!   (`from_{be,le,ne}_bytes` / `.parse()`, transitively through plain
//!   locals) OR hit the counter/ID vocabulary (count/counter/id/idx/index/
//!   seq/slot/key/ord segments) — `crc & 0xFF` and hash-bucket masking
//!   (`fnv & 0xFF`) stay silent.
//! - **2^k−1 mask literals only**: the packing-width spellings; `flags &
//!   0x80` bit tests are not 2^k−1 and stay silent.
//! - **Checked/validated suppression**: any `try_from`/`checked_` in the
//!   fn, or ANY comparison/match-scrutinee/assert naming the origin (or
//!   the binding), suppresses — one guarded site hides the fn, the house
//!   conservative rule.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits + the designed-alias FP class (stated in-rule)
//!
//! - **Designed wraps fire and review decides**: a ring buffer
//!   (`ring[i & (SIZE-1)]`) and a hash-bucket mask are DESIGNED aliases —
//!   the vocabulary gate keeps the common hash/checksum spellings out, a
//!   counter-named ring index still fires. Suggest-only is the
//!   blast-radius control.
//! - Type-blind: `b: u8 & 0xFF` (no-op) fires — parse-only cannot see the
//!   operand width.
//! - Named-const masks (`id & VALUE_MASK`) are unprovable values — silent.
//! - Method-based lookups (`map.get(&(id & MASK))`) are out — the index
//!   form only.
//! - Cross-fn packing (mask in a helper, lookup in the caller) undetected.
//! - Batch 294 / Plan 298 / Research 332 (RUSTSEC-2026-0380 /
//!   CVE-2026-115889, CC0 — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{BinOp, Expr, File, Item, Lit, Stmt};

/// One detected masked-key packing site.
#[derive(Debug, Clone)]
pub struct PackedKeyFinding {
    /// Byte range of the masked expression (the packing site).
    pub pack_span: Range<usize>,
    /// The masked expression's source text.
    pub pack_text: String,
    /// The origin ident whose high bits the mask discards.
    pub origin_ident: String,
    /// The mask width in bits (k, where the literal is 2^k − 1).
    pub mask_bits: u32,
    /// The advice line (the bounded form + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a count/ID masked to a fixed bit-width then used \
as a lookup key silently loses the high bits (wasmtime CVE-2026-115889 \
class): values past the field width alias to WRONG keys — wrong-item \
access, no panic. Assert the range at the packing site \
(`assert!(id <= MASK)`), use a checked newtype / try_from, or widen the \
field; if the wrap is DESIGNED (ring buffer, bucket count), say so in a \
comment (Batch 294, suggest-only)";

/// The counter/ID vocabulary (exact `_`-segments, lowercase).
const KEY_VOCAB: &[&str] = &[
    "count", "counter", "id", "idx", "index", "seq", "slot", "key", "ord",
];

/// Detect masked keys packing into lookups (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<PackedKeyFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<PackedKeyFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.sig.ident.to_string(), &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.sig.ident.to_string(), &m.block, findings);
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
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

/// One fn's scan state, populated by one in-order pass.
#[derive(Default)]
struct FnScan {
    /// Locals bound (transitively) to a parse site.
    parse_origins: Vec<String>,
    /// Binding name -> (origin ident, mask bits, masked-expr span).
    mask_bindings: Vec<(String, String, u32, Range<usize>)>,
    /// Idents seen in a comparison / match scrutinee / assert.
    validated: Vec<String>,
    /// Index sites whose key is a masked expression (origin-free shape).
    masked_indices: Vec<(String, u32, Range<usize>, usize)>,
    /// Index sites whose key is a bare ident (binding candidates).
    bare_indices: Vec<(String, Range<usize>, usize)>,
}

fn scan_fn(source: &str, _name: &str, block: &syn::Block, findings: &mut Vec<PackedKeyFinding>) {
    // Fn-wide checked token: any try_from / checked_ spelling suppresses
    // (one guarded site hides the fn — the house conservative rule).
    let fn_ts = block.to_token_stream().to_string();
    if fn_ts.contains("try_from") || fn_ts.contains("checked_") {
        return;
    }
    let mut scan = FnScan::default();
    walk_stmts(&block.stmts, &mut scan);
    adjudicate(source, &scan, findings);
}

fn walk_stmts(stmts: &[Stmt], scan: &mut FnScan) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    // Mask-binding shape: `let slot = (id & 0xFFF) as usize;`
                    if let Some((origin, bits)) = masked_shape(&init.expr, scan) {
                        let bind_span = init.expr.span().byte_range();
                        for pat in local_pat_idents(&l.pat) {
                            scan.mask_bindings
                                .push((pat, origin.clone(), bits, bind_span.clone()));
                        }
                    }
                    // Parse-origin tracking: the init subtree contains a
                    // parse site, or references a known origin.
                    let fresh_parse = expr_contains_parse(&init.expr);
                    let names = local_pat_idents(&l.pat);
                    let derived = !fresh_parse
                        && deep_idents(&init.expr).any(|i| scan.parse_origins.contains(&i));
                    if fresh_parse || derived {
                        for n in &names {
                            scan.parse_origins.push(n.clone());
                        }
                    }
                    walk_expr(&init.expr, scan);
                }
            }
            Stmt::Item(item) => walk_item(item, scan),
            Stmt::Expr(e, _) => walk_expr(e, scan),
            Stmt::Macro(m) => {
                // `assert!(id < CAP);` as a STATEMENT — the comparison
                // lives in macro tokens; parse it for validation only.
                let last = m.mac.path.segments.last().map(|s| s.ident.to_string());
                if last.is_some_and(|p| p.starts_with("assert")) {
                    if let Ok(inner) = syn::parse2::<Expr>(m.mac.tokens.clone()) {
                        collect_validation(&inner, scan);
                    }
                }
            }
        }
    }
}

fn walk_item(item: &Item, scan: &mut FnScan) {
    match item {
        Item::Fn(f) => walk_stmts(&f.block.stmts, scan),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    walk_stmts(&m.block.stmts, scan);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    walk_item(nested, scan);
                }
            }
        }
        _ => {}
    }
}

fn walk_expr(e: &Expr, scan: &mut FnScan) {
    // Validation collection FIRST (fn-wide conservative clearing).
    collect_validation(e, scan);
    match e {
        Expr::Index(ix) => {
            // Direct masked key: `table[id & 0xFFF]`
            if let Some((origin, bits)) = masked_shape(&ix.index, scan) {
                let key_span = ix.index.span().byte_range();
                scan.masked_indices
                    .push((origin, bits, key_span, scan.masked_indices.len()));
            } else if let Some(id) = bare_ident(&ix.index) {
                // Via-binding key: `table[slot]`
                let key_span = ix.index.span().byte_range();
                scan.bare_indices
                    .push((id, key_span, scan.bare_indices.len()));
            }
            // NOTE: not recursing into the base/key — the walker never
            // descends into macro tokens and the index's own key was just
            // inspected; nested `t[a[i]]` is an honest recall limit.
        }
        Expr::Binary(b) => {
            walk_expr(&b.left, scan);
            walk_expr(&b.right, scan);
        }
        Expr::Paren(p) => walk_expr(&p.expr, scan),
        Expr::Cast(c) => walk_expr(&c.expr, scan),
        Expr::MethodCall(m) => {
            walk_expr(&m.receiver, scan);
            for a in &m.args {
                walk_expr(a, scan);
            }
        }
        Expr::Call(c) => {
            walk_expr(&c.func, scan);
            for a in &c.args {
                walk_expr(a, scan);
            }
        }
        Expr::If(i) => {
            walk_expr(&i.cond, scan);
            walk_stmts(&i.then_branch.stmts, scan);
            if let Some((_, else_e)) = &i.else_branch {
                walk_expr(else_e, scan);
            }
        }
        Expr::Match(m) => {
            walk_expr(&m.expr, scan);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    walk_expr(guard, scan);
                }
                walk_expr(&arm.body, scan);
            }
        }
        Expr::Block(b) => walk_stmts(&b.block.stmts, scan),
        Expr::Unsafe(u) => walk_stmts(&u.block.stmts, scan),
        Expr::Loop(l) => walk_stmts(&l.body.stmts, scan),
        Expr::While(w) => {
            walk_expr(&w.cond, scan);
            walk_stmts(&w.body.stmts, scan);
        }
        Expr::ForLoop(fl) => {
            walk_expr(&fl.expr, scan);
            walk_stmts(&fl.body.stmts, scan);
        }
        Expr::Assign(a) => {
            // `slot = id & MASK` re-binding shape.
            if let Some((origin, bits)) = masked_shape(&a.right, scan) {
                if let Some(name) = bare_ident(&a.left) {
                    let key_span = a.right.span().byte_range();
                    scan.mask_bindings.push((name, origin, bits, key_span));
                }
            }
            walk_expr(&a.right, scan);
        }
        Expr::Try(t) => walk_expr(&t.expr, scan),
        Expr::Reference(r) => walk_expr(&r.expr, scan),
        Expr::Unary(u) => walk_expr(&u.expr, scan),
        _ => {}
    }
}

/// Collect comparison / match-scrutinee / assert idents as validated.
fn collect_validation(e: &Expr, scan: &mut FnScan) {
    match e {
        Expr::Binary(b)
            if matches!(
                b.op,
                BinOp::Lt(_)
                    | BinOp::Gt(_)
                    | BinOp::Le(_)
                    | BinOp::Ge(_)
                    | BinOp::Eq(_)
                    | BinOp::Ne(_)
            ) =>
        {
            for id in deep_idents(&b.left).chain(deep_idents(&b.right)) {
                scan.validated.push(id);
            }
        }
        Expr::Match(m) => {
            for id in deep_idents(&m.expr) {
                scan.validated.push(id);
            }
        }
        Expr::Macro(mac) => {
            let last = mac.mac.path.segments.last().map(|s| s.ident.to_string());
            if last.is_some_and(|p| p.starts_with("assert")) {
                if let Ok(inner) = syn::parse2::<Expr>(mac.mac.tokens.clone()) {
                    collect_validation(&inner, scan);
                    for id in deep_idents(&inner) {
                        scan.validated.push(id);
                    }
                }
            }
        }
        _ => {}
    }
}

/// If `expr` (paren/cast-unwrapped) is `X & 2^k-1` (either order) with a
/// candidate origin on the other side, return (origin, k).
fn masked_shape(e: &Expr, scan: &FnScan) -> Option<(String, u32)> {
    let inner = unwrap_paren_cast(e)?;
    let Expr::Binary(b) = inner else {
        return None;
    };
    if !matches!(b.op, BinOp::BitAnd(_)) {
        return None;
    }
    let (mask_val, other) = match (lit_int(&b.left), lit_int(&b.right)) {
        (Some(v), _) => (v, &b.right),
        (_, Some(v)) => (v, &b.left),
        (None, None) => return None,
    };
    let bits = pow2_minus1_bits(mask_val)?;
    let origin = deep_idents(other).find(|i| is_candidate(i, scan))?;
    Some((origin, bits))
}

/// The candidate gate: a same-fn parse origin OR a counter/ID vocabulary
/// ident (`packet_count` -> segment `count`).
fn is_candidate(ident: &str, scan: &FnScan) -> bool {
    if scan.parse_origins.iter().any(|o| o == ident) {
        return true;
    }
    let lower = ident.to_ascii_lowercase();
    lower.split('_').any(|seg| KEY_VOCAB.contains(&seg))
}

/// The 2^k−1 test: v > 0 and v+1 is a power of two. Returns k.
fn pow2_minus1_bits(v: u128) -> Option<u32> {
    if v == 0 {
        return None;
    }
    // checked_add: a u128::MAX mask literal must not overflow the +1.
    v.checked_add(1)
        .is_some_and(|vp1| v & vp1 == 0)
        .then(|| v.trailing_ones())
}

fn lit_int(e: &Expr) -> Option<u128> {
    let Expr::Lit(syn::ExprLit {
        lit: Lit::Int(i), ..
    }) = e
    else {
        return None;
    };
    i.base10_parse::<u128>().ok()
}

fn unwrap_paren_cast(e: &Expr) -> Option<&Expr> {
    match e {
        Expr::Paren(p) => unwrap_paren_cast(&p.expr),
        Expr::Cast(c) => unwrap_paren_cast(&c.expr),
        _ => Some(e),
    }
}

fn bare_ident(e: &Expr) -> Option<String> {
    let inner = unwrap_paren_cast(e)?;
    if let Expr::Path(p) = inner {
        if p.qself.is_none() && p.path.segments.len() == 1 {
            return Some(p.path.segments[0].ident.to_string());
        }
    }
    None
}

/// Deep single-segment path idents (macro tokens never descended into).
fn deep_idents(e: &Expr) -> impl Iterator<Item = String> {
    let mut out = Vec::new();
    collect_idents(e, &mut out);
    out.into_iter()
}

fn collect_idents(e: &Expr, out: &mut Vec<String>) {
    if let Some(id) = bare_ident(e) {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    match e {
        Expr::Binary(b) => {
            collect_idents(&b.left, out);
            collect_idents(&b.right, out);
        }
        Expr::Paren(p) => collect_idents(&p.expr, out),
        Expr::Cast(c) => collect_idents(&c.expr, out),
        Expr::Unary(u) => collect_idents(&u.expr, out),
        Expr::Index(ix) => {
            collect_idents(&ix.expr, out);
            collect_idents(&ix.index, out);
        }
        Expr::MethodCall(m) => {
            collect_idents(&m.receiver, out);
            for a in &m.args {
                collect_idents(a, out);
            }
        }
        Expr::Call(c) => {
            collect_idents(&c.func, out);
            for a in &c.args {
                collect_idents(a, out);
            }
        }
        _ => {}
    }
}

/// Does the subtree contain a parse site (`from_*_bytes` assoc call or a
/// `.parse(..)` method call)? (the shared walker's site predicate shape)
fn expr_contains_parse(e: &Expr) -> bool {
    let mut found = false;
    contains_parse_walk(e, &mut found);
    found
}

fn contains_parse_walk(e: &Expr, found: &mut bool) {
    if *found {
        return;
    }
    match e {
        Expr::Call(c) => {
            if let Expr::Path(p) = c.func.as_ref() {
                if p.path.segments.last().is_some_and(|s| {
                    matches!(
                        s.ident.to_string().as_str(),
                        "from_be_bytes" | "from_le_bytes" | "from_ne_bytes"
                    )
                }) {
                    *found = true;
                    return;
                }
            }
            for a in &c.args {
                contains_parse_walk(a, found);
            }
        }
        Expr::MethodCall(m) => {
            if m.method == "parse" {
                *found = true;
                return;
            }
            contains_parse_walk(&m.receiver, found);
            for a in &m.args {
                contains_parse_walk(a, found);
            }
        }
        Expr::Binary(b) => {
            contains_parse_walk(&b.left, found);
            contains_parse_walk(&b.right, found);
        }
        Expr::Paren(p) => contains_parse_walk(&p.expr, found),
        Expr::Cast(c) => contains_parse_walk(&c.expr, found),
        Expr::Unary(u) => contains_parse_walk(&u.expr, found),
        Expr::Try(t) => contains_parse_walk(&t.expr, found),
        Expr::Index(ix) => {
            contains_parse_walk(&ix.expr, found);
            contains_parse_walk(&ix.index, found);
        }
        _ => {}
    }
}

fn local_pat_idents(pat: &syn::Pat) -> Vec<String> {
    let mut out = Vec::new();
    pat_walk(pat, &mut out);
    out
}

fn pat_walk(pat: &syn::Pat, out: &mut Vec<String>) {
    match pat {
        syn::Pat::Ident(p) => out.push(p.ident.to_string()),
        syn::Pat::Tuple(t) => {
            for p in &t.elems {
                pat_walk(p, out);
            }
        }
        syn::Pat::Type(p) => pat_walk(&p.pat, out),
        syn::Pat::Paren(p) => pat_walk(&p.pat, out),
        syn::Pat::Reference(r) => pat_walk(&r.pat, out),
        _ => {}
    }
}

/// Adjudicate: a masked index (direct or via an unvalidated binding) with
/// an unvalidated origin fires at the packing site.
fn adjudicate(source: &str, scan: &FnScan, findings: &mut Vec<PackedKeyFinding>) {
    // Direct: the masked expr IS the index key.
    for (origin, bits, span, _) in &scan.masked_indices {
        if scan.validated.contains(origin) {
            continue;
        }
        push(source, findings, origin, *bits, span.clone());
    }
    // Via binding: `let slot = id & MASK; table[slot]` — fire at the
    // packing site when neither the binding nor the origin was validated.
    for (binding, origin, bits, span) in &scan.mask_bindings {
        let used_as_key = scan.bare_indices.iter().any(|(id, _, _)| id == binding);
        if !used_as_key {
            continue;
        }
        if scan.validated.contains(binding) || scan.validated.contains(origin) {
            continue;
        }
        push(source, findings, origin, *bits, span.clone());
    }
}

fn push(
    source: &str,
    findings: &mut Vec<PackedKeyFinding>,
    origin: &str,
    bits: u32,
    span: Range<usize>,
) {
    if span.end > source.len() || findings.iter().any(|f| f.pack_span == span) {
        return;
    }
    findings.push(PackedKeyFinding {
        pack_text: source[span.clone()].to_string(),
        pack_span: span,
        origin_ident: origin.to_string(),
        mask_bits: bits,
        advice: ADVICE,
    });
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::PackedKeyFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    #[test]
    fn wire_origin_masked_binding_index_fires() {
        let src = "pub struct Row { pub kind: u8 }\n\
            pub fn resolve_row(header: [u8; 4], table: &[Row; 4096]) -> &Row {\n\
            \x20   let id = u32::from_be_bytes(header);\n\
            \x20   let slot = (id & 0xFFF) as usize;\n\
            \x20   &table[slot]\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the masked binding key fires");
        assert_eq!(f[0].origin_ident, "id");
        assert_eq!(f[0].mask_bits, 12);
        assert!(f[0].pack_text.contains("& 0xFFF"));
    }

    #[test]
    fn counter_vocab_inline_masked_index_fires() {
        let src = "pub struct Slot;\n\
            pub fn slot_for(counter: usize, slots: &[Slot; 65536]) -> &Slot {\n\
            \x20   &slots[counter & 0xFFFF]\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the inline masked index fires");
        assert_eq!(f[0].origin_ident, "counter");
        assert_eq!(f[0].mask_bits, 16);
    }

    #[test]
    fn validated_origin_is_silent() {
        let src = "pub struct Row { pub kind: u8 }\n\
            pub fn resolve_row(header: [u8; 4], table: &[Row; 4096]) -> Option<&Row> {\n\
            \x20   let id = u32::from_be_bytes(header);\n\
            \x20   assert!(id < 4096);\n\
            \x20   let slot = (id & 0xFFF) as usize;\n\
            \x20   table.get(slot)\n\
            }\n";
        assert!(findings(src).is_empty(), "an assert bounds the origin");
    }

    #[test]
    fn try_from_anywhere_in_fn_is_silent() {
        let src = "pub struct Row { pub kind: u8 }\n\
            pub fn resolve_row(header: [u8; 4], table: &[Row]) -> Option<&Row> {\n\
            \x20   let id = u32::from_be_bytes(header);\n\
            \x20   let slot = usize::try_from(id).ok()?;\n\
            \x20   table.get(slot)\n\
            }\n";
        assert!(findings(src).is_empty(), "the checked twin never fires");
    }

    #[test]
    fn hash_bucket_and_crc_stay_silent() {
        let src = "pub struct Bucket { pub hash: u64 }\n\
            pub fn bucket_of(data: &[u8], buckets: &[Bucket; 256]) -> &Bucket {\n\
            \x20   let fnv = fnv1a(data);\n\
            \x20   &buckets[(fnv & 0xFF) as usize]\n\
            }\n\
            fn fnv1a(data: &[u8]) -> u64 { 0 }\n";
        assert!(
            findings(src).is_empty(),
            "fnv is neither a parse origin nor counter vocabulary"
        );
    }

    #[test]
    fn bit_test_mask_is_silent() {
        let src = "pub fn flag_of(id: usize) -> bool {\n\
            \x20   (id & 0x80) != 0\n\
            }\n";
        assert!(
            findings(src).is_empty(),
            "0x80 is not 2^k-1 and not an index"
        );
    }

    #[test]
    fn no_lookup_stays_silent() {
        let src = "pub fn emit_low(id: u32, out: &mut Vec<u8>) {\n\
            \x20   out.push((id & 0xFF) as u8);\n\
            }\n";
        assert!(
            findings(src).is_empty(),
            "a checksum byte is not a lookup key"
        );
    }

    #[test]
    fn test_mod_is_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n\
            \x20   pub struct Row;\n\
            \x20   #[test]\n\
            \x20   fn t() {\n\
            \x20       let id = 5u64.parse::<u64>().unwrap_or(0);\n\
            \x20       let slot = (id & 0xFFF) as usize;\n\
            \x20       let _ = &TABLE[slot];\n\
            \x20   }\n\
            \x20   static TABLE: [Row; 4096] = [Row; 4096];\n\
            }\n";
        assert!(findings(src).is_empty(), "fixture habitat stays silent");
    }

    #[test]
    fn finding_spans_slice_the_source() {
        let src = "pub struct Row;\n\
            pub fn f(header: [u8; 4], table: &[Row; 4096]) -> &Row {\n\
            \x20   let id = u32::from_be_bytes(header);\n\
            \x20   &table[(id & 0xFFF) as usize]\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1);
        let span = f[0].pack_span.clone();
        assert_eq!(&src[span], "(id & 0xFFF) as usize");
    }

    #[test]
    fn corpus_buggy_fixture_fires() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "packed-field-silent-overflow-wrong-key")
            .expect("rule present");
        let f = findings(&rule.buggy_code);
        assert!(!f.is_empty(), "the corpus buggy fixture must fire");
        let fixed = syn::parse_file(&rule.fixed_code).expect("fixed parses");
        assert!(
            detect(&rule.fixed_code, &fixed).is_empty(),
            "the fixed fixture must stay silent"
        );
    }
}
