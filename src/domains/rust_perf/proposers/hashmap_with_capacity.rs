//! `hashmap-with-capacity` — `let mut m = HashMap::new();` + a
//! known-count insert loop → `HashMap::with_capacity(src.len())`.
//!
//! Pattern: within one block, `let mut VAR [: TY] = HashMap::new();`
//! (plain two-segment path only — `HashMap::new()`, no turbofish, no
//! `std::collections::` qualification; the rewrite emits the same short
//! path, so the file must already have `HashMap` in scope) whose first
//! later mentioning statement is `for PAT in SRC { .. VAR.insert(..) .. }`
//! where SRC yields a `len()`: a bare path (`for &it in items` over a
//! slice — or a by-value `Vec`), `&path`, `path.iter()`, or
//! `path.iter_mut()`. The initializer becomes
//! `HashMap::with_capacity(PATH.len())` — the loop survives untouched.
//! Bench 038/B53: ParetoDominant 2.4-3.1x, allocs 12→1 at n=4096 (the
//! rehash ladder re-SipHashes ALL stored entries at each of ~log2(n)
//! resizes — a different cost class from the Vec sibling's
//! realloc+memcpy).
//!
//! ## Gates
//! - **Capacity is a hint**: `with_capacity(n)` is semantically `new()`
//!   for ANY n — the rewrite can never change behavior, only allocation
//!   counts. The len is derived from the loop's own iterable and is
//!   evaluated BEFORE the loop (a by-value iterable's `.len()` still
//!   reads it pre-move), so every accepted SRC form is sound even when
//!   the loop consumes the iterable.
//! - **First-mention:** the first later statement mentioning VAR must BE
//!   the for-loop — an intervening insert/seed skips the pattern (the
//!   len→capacity association is muddied). Unrelated statements are
//!   skipped over. A later re-binding of VAR stops the scan (shadowing).
//! - **Simple len-bases only**: a path or a named-field chain rooted at
//!   one (`s.rows`, `self.items`) with no generic arguments anywhere and
//!   no leading `::`. Range loops (`for i in 0..n`) have no collection to
//!   len — skip. `into_iter()` and arbitrary methods skip conservatively
//!   (iter/iter_mut only).
//! - **Body gate**: the loop body must mention `VAR.insert` — a loop
//!   that doesn't insert into the map doesn't justify its capacity.
//! - **Non-`mut` binding skips** (an insert loop over it would not
//!   compile — skip rather than rewrite non-compiling code).
//!
//! Out of scope: turbofish/qualified `HashMap::new()` forms (the
//! span-preserving splice would have to rewrite the path — kept
//! conservative), `HashMap::default()`, and a `for` living as a non-
//! first mention below nested blocks.

use syn::visit::Visit;
use syn::{Block, Expr, File, Local, Pat, Path, Stmt};

use super::{edit, span_range, src, SourceEdit};

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
            let Some(var) = fresh_hashmap_binding(local) else { continue };
            let Some(init) = &local.init else { continue };

            for later in stmts.iter().skip(i + 1) {
                if stmt_binds_ident(later, &var) {
                    break; // shadowing re-bind — stop the scan
                }
                if !self.stmt_mentions(later, &var) {
                    continue; // unrelated statement — skip over
                }
                let Stmt::Expr(Expr::ForLoop(fl), _) = later else { break };
                // Body must insert into VAR (word-boundary `VAR.insert`).
                let (bs, be) = span_range(&fl.body);
                let Some(body_txt) = self.source.get(bs..be) else { break };
                if !contains_ident_word(body_txt, &format!("{var}.insert")) {
                    break;
                }
                // Iterable must yield a len.
                let Some(len_expr) = len_expr_text(&fl.expr, self.source) else { break };
                self.edits.push(edit(
                    &init.expr,
                    format!("HashMap::with_capacity({len_expr})"),
                ));
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

/// `let mut VAR [: TY] = HashMap::new();` → the binding ident. Only the
/// plain two-segment path with no generic arguments anywhere — the
/// emitted `HashMap::with_capacity(..)` assumes the short name is in
/// scope, which the matched source itself guarantees.
fn fresh_hashmap_binding(local: &Local) -> Option<String> {
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
    let Expr::Call(call) = &*init.expr else { return None };
    let Expr::Path(p) = &*call.func else { return None };
    let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
    if segs == ["HashMap", "new"] && call.args.is_empty() && simple_path(&p.path) {
        Some(pi.ident.to_string())
    } else {
        None
    }
}

/// `PATH.len()` text for iterable forms that expose a len: a bare
/// len-base (path or named-field chain like `self.items`), `&base`, and
/// `base.iter()` / `base.iter_mut()`. Everything else — ranges,
/// `into_iter`, method chains, calls — yields `None`.
fn len_expr_text(e: &Expr, source: &str) -> Option<String> {
    match e {
        Expr::Path(_) | Expr::Field(_) if simple_len_base(e) => {
            src(source, e).map(|s| format!("{s}.len()"))
        }
        Expr::Reference(r) if simple_len_base(&r.expr) => {
            src(source, &r.expr).map(|s| format!("{s}.len()"))
        }
        Expr::MethodCall(mc) if mc.method == "iter" || mc.method == "iter_mut" => {
            if simple_len_base(&mc.receiver) {
                src(source, &mc.receiver).map(|s| format!("{s}.len()"))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// A len-able base: a simple path (no generic args, no leading `::`) or a
/// named-field chain rooted at one (`s.rows`, `self.items.items`).
fn simple_len_base(e: &Expr) -> bool {
    match e {
        Expr::Path(p) => simple_path(&p.path),
        Expr::Field(f) => matches!(f.member, syn::Member::Named(_)) && simple_len_base(&f.base),
        _ => false,
    }
}

fn simple_path(p: &Path) -> bool {
    p.leading_colon.is_none() && p.segments.iter().all(|s| s.arguments.is_none())
}

/// Does this stmt re-bind `ident` (any mutability)?
fn stmt_binds_ident(stmt: &Stmt, ident: &str) -> bool {
    match stmt {
        Stmt::Local(local) => {
            let mut pat = &local.pat;
            while let Pat::Type(pt) = pat {
                pat = &pt.pat;
            }
            matches!(pat, Pat::Ident(pi) if pi.ident == ident)
        }
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

    fn parses(out: &str) {
        assert!(syn::parse_file(out).is_ok(), "output does not parse: {out}");
    }

    /// The corpus fixture shape (slice param iterable, trailing comments).
    #[test]
    fn corpus_fixture_rewrites() {
        let out = propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn index_items(items: &[u32]) -> HashMap<u32, u32> \
             {\n    let mut map = HashMap::new(); // starts at capacity ~3\n    for &it in items \
             {\n        map.insert(it, it.wrapping_mul(31)); // rehash ladder: ~log2(n) resizes, \
             each re-SipHashing all entries\n    }\n    map\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("HashMap::with_capacity(items.len())"), "{out}");
        // Both trailing comments + the loop + the tail survive verbatim.
        assert!(out.contains("// starts at capacity ~3"), "{out}");
        assert!(out.contains("map.insert(it, it.wrapping_mul(31));"), "{out}");
        assert!(out.contains("// rehash ladder"), "{out}");
        parses(&out);
    }

    /// The eval-span shape (named map `weights`, iterable `zone_ids`).
    #[test]
    fn eval_span_shape_fires() {
        let out = propose_fix(
            "hashmap-with-capacity",
            "pub fn zone_lookup_table(zone_ids: &[ZoneId]) -> HashMap<ZoneId, u32> {\n    \
             let mut weights = HashMap::new();\n    for &z in zone_ids {\n        \
             weights.insert(z, z.index() as u32);\n    }\n    weights\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("HashMap::with_capacity(zone_ids.len())"), "{out}");
        parses(&out);
    }

    #[test]
    fn iter_and_iter_mut_forms_fire() {
        let out = propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(v: &Vec<(u32, u32)>) -> HashMap<u32, u32> {\n \
             \x20   let mut m = HashMap::new();\n    for (k, val) in v.iter() {\n        \
             m.insert(*k, *val);\n    }\n    m\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("HashMap::with_capacity(v.len())"), "{out}");

        let out = propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn g(v: &mut Vec<(u32, u32)>) -> HashMap<u32, u32> \
             {\n    let mut m = HashMap::new();\n    for (k, val) in v.iter_mut() {\n        \
             m.insert(*k, *val);\n    }\n    m\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("HashMap::with_capacity(v.len())"), "{out}");
        parses(&out);
    }

    #[test]
    fn reference_iterable_fires() {
        let out = propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(v: Vec<(u32, u32)>) -> HashMap<u32, u32> {\n  \
             \x20  let mut m = HashMap::new();\n    for (k, val) in &v {\n        \
             m.insert(*k, *val);\n    }\n    m\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("HashMap::with_capacity(v.len())"), "{out}");
        parses(&out);
    }

    #[test]
    fn field_path_iterable_fires() {
        let out = propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nstruct S { rows: Vec<u32> }\nfn f(s: &S) -> \
             HashMap<u32, u32> {\n    let mut m = HashMap::new();\n    for r in s.rows.iter() {\n  \
             \x20      m.insert(*r, 1);\n    }\n    m\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("HashMap::with_capacity(s.rows.len())"), "{out}");
        parses(&out);
    }

    #[test]
    fn type_annotated_binding_fires() {
        let out = propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(items: &[u32]) -> HashMap<u32, u32> {\n    \
             let mut m: HashMap<u32, u32> = HashMap::new();\n    for &it in items {\n        \
             m.insert(it, it);\n    }\n    m\n}",
        )
        .expect("pattern matched");
        // The type annotation survives byte-identically; only the init
        // expr is replaced.
        assert!(out.contains("let mut m: HashMap<u32, u32> = HashMap::with_capacity(items.len());"), "{out}");
        parses(&out);
    }

    #[test]
    fn range_loop_skips() {
        // No collection to len() — the count is not derivable.
        assert!(propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(n: u64) -> HashMap<u64, u64> {\n    \
             let mut m = HashMap::new();\n    for i in 0..n {\n        m.insert(i, i);\n    }\n    \
             m\n}",
        )
        .is_none());
    }

    #[test]
    fn intervening_seed_insert_skips() {
        // First mention is a seed insert, not the loop — the len→capacity
        // association is muddied.
        assert!(propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(items: &[u32]) -> HashMap<u32, u32> {\n    \
             let mut m = HashMap::new();\n    m.insert(0, 0);\n    for &it in items {\n        \
             m.insert(it, it);\n    }\n    m\n}",
        )
        .is_none());
    }

    #[test]
    fn loop_without_insert_skips() {
        assert!(propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(items: &[u32], m: &HashMap<u32, u32>) -> usize \
             {\n    let mut m2 = HashMap::new();\n    for &it in items {\n        \
             let _ = m.get(&it);\n    }\n    m2.len()\n}",
        )
        .is_none());
    }

    #[test]
    fn turbofish_form_skips() {
        assert!(propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(items: &[u32]) -> HashMap<u32, u32> {\n    \
             let mut m = HashMap::<u32, u32>::new();\n    for &it in items {\n        \
             m.insert(it, it);\n    }\n    m\n}",
        )
        .is_none());
    }

    #[test]
    fn qualified_path_form_skips() {
        // The splice would have to rewrite the path — conservative skip.
        assert!(propose_fix(
            "hashmap-with-capacity",
            "fn f(items: &[u32]) -> std::collections::HashMap<u32, u32> {\n    \
             let mut m = std::collections::HashMap::new();\n    for &it in items {\n        \
             m.insert(it, it);\n    }\n    m\n}",
        )
        .is_none());
    }

    #[test]
    fn non_mut_binding_skips() {
        assert!(propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(items: &[u32]) -> HashMap<u32, u32> {\n    \
             let m = HashMap::new();\n    m\n}",
        )
        .is_none());
    }

    #[test]
    fn shadowing_rebind_skips() {
        assert!(propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(items: &[u32]) -> usize {\n    \
             let mut m = HashMap::new();\n    let m = 3;\n    let _ = m;\n    items.len()\n}",
        )
        .is_none());
    }

    #[test]
    fn unrelated_statements_between_are_skipped_over() {
        let out = propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(items: &[u32], other: u32) -> HashMap<u32, u32> \
             {\n    let mut m = HashMap::new();\n    let seed = other + 1;\n    for &it in items \
             {\n        m.insert(it, seed);\n    }\n    m\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("HashMap::with_capacity(items.len())"), "{out}");
        assert!(out.contains("let seed = other + 1;"), "{out}");
        parses(&out);
    }

    #[test]
    fn method_chain_iterable_skips() {
        // `.clone()` receiver is a call, not a simple path — and len would
        // double-clone. Conservative skip.
        assert!(propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(items: &Vec<u32>) -> HashMap<u32, u32> {\n    \
             let mut m = HashMap::new();\n    for it in items.clone().iter() {\n        \
             m.insert(*it, 1);\n    }\n    m\n}",
        )
        .is_none());
    }

    #[test]
    fn into_iter_skips() {
        assert!(propose_fix(
            "hashmap-with-capacity",
            "use std::collections::HashMap;\nfn f(items: Vec<u32>) -> HashMap<u32, u32> {\n    \
             let mut m = HashMap::new();\n    for it in items.into_iter() {\n        \
             m.insert(it, 1);\n    }\n    m\n}",
        )
        .is_none());
    }
}
