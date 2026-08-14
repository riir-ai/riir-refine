//! `fxhashmap` — std `HashMap` (SipHash) → `FxHashMap` (FxHash).
//!
//! Pattern: a file with a private `use std::collections::HashMap;` (plain or
//! `{.., HashMap, ..}` group) and at least one `HashMap` path reference in
//! the body. The rewrite swaps the use to `use rustc_hash::FxHashMap;`,
//! removes `HashMap` from any use group (preserving the sibling imports +
//! appending the FxHashMap use), and rewrites every path reference:
//!
//! | Body form | Rewritten |
//! |---|---|
//! | `HashMap<K, V>` (type) | `FxHashMap<K, V>` |
//! | `HashMap::new()` / `HashMap::<K, V>::new()` | `FxHashMap::new()` (turbofish + trailing segments preserved — only the ident's span is replaced) |
//! | `std::collections::HashMap::with_capacity(..)` | `rustc_hash::FxHashMap::with_capacity(..)` |
//!
//! FxHash is 1 multiply + 1 xor per hash vs SipHash-1-3's full compression
//! rounds — the win for non-adversarial keys (game state, internal caches),
//! used pervasively in rustc itself (Bench 013: ParetoDominant 3.3-4.2x).
//!
//! **Honest scope limits:**
//! - The healed crate must have `rustc-hash` in its `Cargo.toml` — a
//!   parse-only pruner cannot check this (documented module caveat).
//! - `pub use` re-exports, glob uses (`use std::collections::*;`),
//!   `pub(crate)` visibility, fn-local `use` stmts, and nested-mod uses are
//!   skipped (each either changes public semantics or needs sibling-item
//!   surgery the MVP doesn't do).
//! - The visitors skip all `use` items by construction — the use surgery
//!   runs first, directly over `file.items`.
//!
//! Span-preserving: the use item is replaced by the swapped/slimmed text
//! (with the appended FxHashMap use on its own line at the same indent),
//! and each path edit replaces exactly the ident-prefix span.

use syn::visit::Visit;
use syn::{File, Item, ItemUse, Path, UseTree, Visibility};

use super::{line_indent, span_range, SourceEdit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    // Gate 1: a private file-level use that handle_use can actually
    // transform (plain HashMap leaf, or a group containing one). Glob /
    // deeper-path / nested-group uses are honestly skipped — rewriting body
    // paths without the use swap would emit unimported idents.
    let has_transformable_use = file.items.iter().any(|i| match i {
        Item::Use(u) => handle_use(u).is_some(),
        _ => false,
    });
    if !has_transformable_use {
        return Vec::new();
    }
    // Gate 2: at least one HashMap path reference outside use items —
    // otherwise the use is dead and swapping it is noise.
    let mut scan = BareRefScan { count: 0 };
    scan.visit_file(file);
    if scan.count == 0 {
        return Vec::new();
    }

    let mut edits = Vec::new();
    use_edits(source, &mut edits, file);

    let mut paths = PathScan { edits: Vec::new() };
    paths.visit_file(file);
    if paths.edits.is_empty() {
        // Gate 2 counted refs but none were rewritable shapes — no-op rather
        // than a use swap that leaves mixed spellings.
        return Vec::new();
    }
    edits.extend(paths.edits);
    edits
}

// ── Use-item surgery (direct iteration over file.items) ──────────────────

/// Private (inherited-visibility) `use std::collections::<...>` — plain
/// HashMap leaf or a group that may contain it.
fn is_private_std_collections_prefix(u: &ItemUse) -> bool {
    if !matches!(u.vis, Visibility::Inherited) {
        return false;
    }
    let UseTree::Path(p1) = &u.tree else { return false };
    if p1.ident != "std" {
        return false;
    }
    let UseTree::Path(p2) = &*p1.tree else { return false };
    p2.ident == "collections"
}

enum UseOutcome {
    /// `use std::collections::HashMap;` → `use rustc_hash::FxHashMap;`
    Swapped,
    /// HashMap removed from a group; the kept siblings are returned (the
    /// replacement appends the FxHashMap use on its own line).
    GroupSlimmed(Vec<String>),
}

fn use_edits(source: &str, edits: &mut Vec<SourceEdit>, file: &File) {
    for item in &file.items {
        let Item::Use(u) = item else { continue };
        let Some(outcome) = handle_use(u) else { continue };
        let (start, end) = span_range(u);
        let replacement = match outcome {
            UseOutcome::Swapped => "use rustc_hash::FxHashMap;".to_string(),
            UseOutcome::GroupSlimmed(kept) => {
                let indent = line_indent(source, start);
                format!(
                    "use std::collections::{{{}}};\n{indent}use rustc_hash::FxHashMap;",
                    kept.join(", ")
                )
            }
        };
        edits.push(SourceEdit {
            start,
            end,
            replacement,
        });
    }
}

fn handle_use(u: &ItemUse) -> Option<UseOutcome> {
    if !is_private_std_collections_prefix(u) {
        return None;
    }
    // Navigate std :: collections :: <rest>.
    let UseTree::Path(p1) = &u.tree else { return None };
    let UseTree::Path(p2) = &*p1.tree else { return None };
    match &*p2.tree {
        // Plain leaf: swap the whole use.
        UseTree::Name(n) if n.ident == "HashMap" => Some(UseOutcome::Swapped),
        // Group containing a plain HashMap leaf: remove it. If the group
        // empties, the whole use becomes the FxHashMap use; else keep the
        // slimmed group + append a separate FxHashMap use line.
        UseTree::Group(g) => {
            let hash_idx = g
                .items
                .iter()
                .position(|t| matches!(t, UseTree::Name(n) if n.ident == "HashMap"));
            let idx = hash_idx?;
            let kept: Vec<String> = g
                .items
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != idx)
                .map(|(_, t)| use_tree_text(t))
                .collect::<Option<Vec<_>>>()?;
            if kept.is_empty() {
                Some(UseOutcome::Swapped)
            } else {
                Some(UseOutcome::GroupSlimmed(kept))
            }
        }
        _ => None,
    }
}

/// Text for a kept use-tree sibling (group leaves are simple idents/paths;
/// their token spelling is the source-faithful form).
fn use_tree_text(t: &UseTree) -> Option<String> {
    match t {
        UseTree::Name(n) => Some(n.ident.to_string()),
        UseTree::Path(p) => Some(format!("{}::{}", p.ident, use_tree_text(&p.tree)?)),
        _ => None,
    }
}

// ── Pre-scan: count HashMap path refs outside use items ──────────────────

struct BareRefScan {
    count: usize,
}

impl<'ast> Visit<'ast> for BareRefScan {
    fn visit_item_use(&mut self, _u: &'ast ItemUse) {
        // Uses are handled by the surgery, not the scan.
    }

    fn visit_path(&mut self, p: &'ast Path) {
        if is_bare_hashmap(p) || is_std_collections_hashmap(p) {
            self.count += 1;
        }
        syn::visit::visit_path(self, p);
    }
}

fn is_bare_hashmap(p: &Path) -> bool {
    p.leading_colon.is_none() && !p.segments.is_empty() && p.segments[0].ident == "HashMap"
}

fn is_std_collections_hashmap(p: &Path) -> bool {
    p.leading_colon.is_none()
        && p.segments.len() >= 3
        && p.segments[0].ident == "std"
        && p.segments[1].ident == "collections"
        && p.segments[2].ident == "HashMap"
}

// ── Path edit scan ───────────────────────────────────────────────────────

struct PathScan {
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for PathScan {
    fn visit_item_use(&mut self, _u: &'ast ItemUse) {
        // Use items are handled by the surgery pass.
    }

    fn visit_path(&mut self, p: &'ast Path) {
        if is_bare_hashmap(p) {
            let (s, e) = span_range(&p.segments[0].ident);
            self.edits.push(SourceEdit {
                start: s,
                end: e,
                replacement: "FxHashMap".to_string(),
            });
        } else if is_std_collections_hashmap(p) {
            // Replace `std::collections::HashMap` (up to the HashMap IDENT's
            // end — turbofish + trailing segments keep their own spans).
            let (ps, _) = span_range(p);
            let (_, ie) = span_range(&p.segments[2].ident);
            self.edits.push(SourceEdit {
                start: ps,
                end: ie,
                replacement: "rustc_hash::FxHashMap".to_string(),
            });
        }
        // Always recurse: turbofish args of the same path may contain nested
        // HashMap types (e.g. `HashMap::<HashMap<u32, u32>, u32>`).
        syn::visit::visit_path(self, p);
    }
}

// `src` (the source-text helper) is unused here — path replacements are
// ident literals; use-item replacements are short fixed lines.

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn corpus_fixture_rewrites() {
        let out = propose_fix(
            "fxhashmap",
            "use std::collections::HashMap;\nfn lookup(map: &HashMap<u32, u32>, k: u32) \
             -> Option<u32> {\n    map.get(&k).copied()\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("use rustc_hash::FxHashMap;"), "{out}");
        assert!(out.contains("map: &FxHashMap<u32, u32>"), "{out}");
        assert!(!out.contains("std::collections"), "{out}");
        // The fn body line is byte-identical.
        assert!(out.contains("    map.get(&k).copied()"), "{out}");
    }

    #[test]
    fn new_call_rewrites() {
        let out = propose_fix(
            "fxhashmap",
            "use std::collections::HashMap;\nfn make() -> HashMap<u8, u8> { HashMap::new() }",
        )
        .expect("pattern matched");
        assert!(out.contains("FxHashMap<u8, u8>"), "{out}");
        assert!(out.contains("FxHashMap::new()"), "{out}");
    }

    #[test]
    fn turbofish_preserved() {
        let out = propose_fix(
            "fxhashmap",
            "use std::collections::HashMap;\nfn make() -> HashMap<u8, u8> {\n    \
             HashMap::<u8, u8>::new()\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("FxHashMap::<u8, u8>::new()"), "{out}");
    }

    #[test]
    fn group_use_slims_with_indented_append() {
        let out = propose_fix(
            "fxhashmap",
            "use std::collections::{HashMap, HashSet};\nfn f() -> HashMap<u32, u32> { \
             HashMap::new() }",
        )
        .expect("pattern matched");
        // HashSet import preserved (group stays).
        assert!(out.contains("HashSet"), "{out}");
        assert!(out.contains("use std::collections::{HashSet};"), "{out}");
        assert!(out.contains("use rustc_hash::FxHashMap;"), "{out}");
        assert!(out.contains("FxHashMap<u32, u32>"), "{out}");
    }

    #[test]
    fn group_only_hashmap_collapses() {
        let out = propose_fix(
            "fxhashmap",
            "use std::collections::{HashMap};\nfn f() -> HashMap<u32, u32> { HashMap::new() }",
        )
        .expect("pattern matched");
        // Exactly one use remains — the FxHashMap one.
        assert_eq!(squash(&out).matches("use").count(), 1, "{out}");
        assert!(out.contains("use rustc_hash::FxHashMap;"), "{out}");
    }

    #[test]
    fn fully_qualified_body_path_rewrites() {
        let out = propose_fix(
            "fxhashmap",
            "use std::collections::HashMap;\nfn f() -> std::collections::HashMap<u16, u16> {\n    \
             std::collections::HashMap::new()\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("rustc_hash::FxHashMap<u16, u16>"), "{out}");
        assert!(out.contains("rustc_hash::FxHashMap::new()"), "{out}");
        assert!(!out.contains("std::collections"), "{out}");
    }

    #[test]
    fn output_reparses() {
        let out = propose_fix(
            "fxhashmap",
            "use std::collections::HashMap;\nstruct Store { counts: HashMap<String, \
             Vec<u32>> }\nimpl Store {\n    fn new() -> Self { Store { counts: \
             HashMap::new() } }\n}",
        )
        .expect("pattern matched");
        let reparsed = syn::parse_file(&out).expect("output must re-parse");
        assert!(!reparsed.items.is_empty());
    }

    #[test]
    fn no_use_unchanged() {
        // Body refs but no use item — out of scope (nothing to swap).
        assert!(
            propose_fix(
                "fxhashmap",
                "fn f() -> std::collections::HashMap<u32, u32> {\n    \
                 std::collections::HashMap::new()\n}"
            )
            .is_none()
        );
    }

    #[test]
    fn dead_use_unchanged() {
        // Use exists but zero body refs — swapping is noise.
        assert!(propose_fix("fxhashmap", "use std::collections::HashMap;\nfn f() -> u32 { 42 }").is_none());
    }

    #[test]
    fn glob_use_unchanged() {
        assert!(
            propose_fix(
                "fxhashmap",
                "use std::collections::*;\nfn f(m: &HashMap<u32, u32>) -> u32 { 0 }"
            )
            .is_none()
        );
    }

    #[test]
    fn pub_use_unchanged() {
        // Re-export — changing it alters the crate's public API.
        assert!(
            propose_fix(
                "fxhashmap",
                "pub use std::collections::HashMap;\nfn f(m: &HashMap<u32, u32>) -> u32 { 0 }"
            )
            .is_none()
        );
    }
}
