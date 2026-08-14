//! `fxhashmap` — std `HashMap` (SipHash) → `FxHashMap` (FxHash).
//!
//! Pattern: a file with a private `use std::collections::HashMap;` (plain or
//! `{.., HashMap, ..}` group) and at least one `HashMap` path reference in
//! the body. The rewrite swaps the use to `use rustc_hash::FxHashMap;`,
//! removes `HashMap` from any use group (preserving the sibling imports),
//! and rewrites every path reference:
//!
//! | Body form | Rewritten |
//! |---|---|
//! | `HashMap<K, V>` (type) | `FxHashMap<K, V>` |
//! | `HashMap::new()` / `HashMap::<K, V>::new()` | `FxHashMap::new()` (turbofish preserved) |
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
//! - The visitor skips all `use` items by construction — the use surgery
//!   runs first, directly over `file.items`.

use proc_macro2::Span;
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::punctuated::Punctuated;
use syn::{File, Ident, Item, ItemUse, Path, PathSegment, UseTree, Visibility};

use quote::format_ident;

pub(crate) fn rewrite(file: &mut File) -> bool {
    // Gate 1: a private file-level std::collections::HashMap use must exist.
    if !file.items.iter().any(is_swappable_use) {
        return false;
    }
    // Gate 2: at least one HashMap path reference outside use items —
    // otherwise the use is dead and swapping it is noise.
    let mut scan = BareRefScan { count: 0 };
    scan.visit_file(file);
    if scan.count == 0 {
        return false;
    }

    let use_changed = rewrite_uses(file);
    let mut rewriter = PathRewriter { changed: false };
    rewriter.visit_file_mut(file);
    use_changed && rewriter.changed
}

// ── Use-item surgery (direct iteration over file.items) ──────────────────

fn is_swappable_use(item: &Item) -> bool {
    match item {
        Item::Use(u) => is_private_std_collections_prefix(u),
        _ => false,
    }
}

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
    /// HashMap removed from a group; a separate `use rustc_hash::FxHashMap;`
    /// is appended after this item.
    GroupSlimmed,
    /// std::collections use with no plain HashMap leaf (glob, deeper path,
    /// nested group) — left untouched.
    NotOurForm,
}

fn rewrite_uses(file: &mut File) -> bool {
    let mut changed = false;
    let mut rebuilt: Vec<Item> = Vec::with_capacity(file.items.len() + 1);
    for item in file.items.drain(..) {
        let Item::Use(mut u) = item else {
            rebuilt.push(item);
            continue;
        };
        match handle_use(&mut u) {
            UseOutcome::Swapped => {
                rebuilt.push(Item::Use(u));
                changed = true;
            }
            UseOutcome::GroupSlimmed => {
                rebuilt.push(Item::Use(u));
                rebuilt.push(Item::Use(parse_fx_use()));
                changed = true;
            }
            UseOutcome::NotOurForm => rebuilt.push(Item::Use(u)),
        }
    }
    file.items = rebuilt;
    changed
}

fn parse_fx_use() -> ItemUse {
    syn::parse_quote! { use rustc_hash::FxHashMap; }
}

fn handle_use(u: &mut ItemUse) -> UseOutcome {
    if !is_private_std_collections_prefix(u) {
        return UseOutcome::NotOurForm;
    }
    // Navigate std :: collections :: <rest>.
    let UseTree::Path(p1) = &mut u.tree else {
        return UseOutcome::NotOurForm;
    };
    let UseTree::Path(p2) = &mut *p1.tree else {
        return UseOutcome::NotOurForm;
    };
    match &mut *p2.tree {
        // Plain leaf: swap the whole tree.
        UseTree::Name(n) if n.ident == "HashMap" => {
            u.tree = syn::parse_quote! { rustc_hash::FxHashMap };
            UseOutcome::Swapped
        }
        // Group containing a plain HashMap leaf: remove it; if the group
        // empties, the whole use becomes the FxHashMap use; else keep the
        // slimmed group + append a separate FxHashMap use.
        UseTree::Group(g) => {
            let hash_idx = g.items.iter().position(|t| matches!(t, UseTree::Name(n) if n.ident == "HashMap"));
            let Some(idx) = hash_idx else { return UseOutcome::NotOurForm };
            let kept: Punctuated<UseTree, syn::Token![,]> =
                g.items.iter().enumerate().filter(|(i, _)| *i != idx).map(|(_, t)| t.clone()).collect();
            if kept.is_empty() {
                u.tree = syn::parse_quote! { rustc_hash::FxHashMap };
                UseOutcome::Swapped
            } else {
                g.items = kept;
                UseOutcome::GroupSlimmed
            }
        }
        _ => UseOutcome::NotOurForm,
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

// ── Path rewriter (whole file, skipping use items) ───────────────────────

struct PathRewriter {
    changed: bool,
}

impl VisitMut for PathRewriter {
    fn visit_item_use_mut(&mut self, _u: &mut ItemUse) {
        // Use items were already handled by the surgery pass.
    }

    fn visit_path_mut(&mut self, p: &mut Path) {
        let mutated = if is_bare_hashmap(p) {
            p.segments[0].ident = format_ident!("FxHashMap");
            self.changed = true;
            true
        } else if is_std_collections_hashmap(p) {
            // `std::collections::HashMap[..]` → `rustc_hash::FxHashMap[..]`.
            // The original HashMap segment (with any turbofish) is preserved,
            // trailing segments (`::new`) kept.
            let mut rest: Vec<PathSegment> = p.segments.iter().skip(2).cloned().collect();
            rest[0].ident = format_ident!("FxHashMap");
            let mut segs: Punctuated<PathSegment, syn::Token![::]> = Punctuated::new();
            segs.push(PathSegment::from(Ident::new("rustc_hash", Span::call_site())));
            for seg in rest {
                segs.push(seg);
            }
            p.segments = segs;
            self.changed = true;
            true
        } else {
            false
        };
        // Always recurse: turbofish args of the same path may contain nested
        // HashMap types (e.g. `HashMap::<HashMap<u32, u32>, u32>`).
        let _ = mutated;
        syn::visit_mut::visit_path_mut(self, p);
    }
}

#[cfg(test)]
mod tests {
    use super::super::quote_file;
    use super::*;

    fn propose(input: &str) -> Option<String> {
        let mut file = syn::parse_file(input).ok()?;
        if rewrite(&mut file) {
            Some(quote_file(&file))
        } else {
            None
        }
    }

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn corpus_fixture_rewrites() {
        let out = propose(
            "use std::collections::HashMap;\nfn lookup(map: &HashMap<u32, u32>, k: u32) \
             -> Option<u32> {\n    map.get(&k).copied()\n}",
        )
        .expect("pattern matched");
        let s = squash(&out);
        assert!(s.contains(&squash("use rustc_hash::FxHashMap;")), "{out}");
        assert!(s.contains(&squash("map: &FxHashMap<u32, u32>")), "{out}");
        // No std HashMap path remains.
        assert!(!s.contains("std::collections"), "{out}");
    }

    #[test]
    fn new_call_rewrites() {
        let out = propose(
            "use std::collections::HashMap;\nfn make() -> HashMap<u8, u8> { HashMap::new() }",
        )
        .expect("pattern matched");
        let s = squash(&out);
        assert!(s.contains(&squash("FxHashMap<u8, u8>")), "{out}");
        assert!(s.contains("FxHashMap::new()"), "{out}");
    }

    #[test]
    fn group_use_slims() {
        let out = propose(
            "use std::collections::{HashMap, HashSet};\nfn f() -> HashMap<u32, u32> { \
             HashMap::new() }",
        )
        .expect("pattern matched");
        let s = squash(&out);
        // HashSet import preserved (group stays), FxHashMap use added.
        assert!(s.contains("HashSet"), "{out}");
        assert!(s.contains(&squash("use rustc_hash::FxHashMap;")), "{out}");
        assert!(s.contains(&squash("FxHashMap<u32, u32>")), "{out}");
    }

    #[test]
    fn group_only_hashmap_collapses() {
        let out = propose(
            "use std::collections::{HashMap};\nfn f() -> HashMap<u32, u32> { HashMap::new() }",
        )
        .expect("pattern matched");
        let s = squash(&out);
        // Exactly one use remains — the FxHashMap one.
        assert_eq!(s.matches("use").count(), 1, "{out}");
        assert!(s.contains(&squash("use rustc_hash::FxHashMap;")), "{out}");
    }

    #[test]
    fn fully_qualified_body_path_rewrites() {
        let out = propose(
            "use std::collections::HashMap;\nfn f() -> std::collections::HashMap<u16, u16> {\n    \
             std::collections::HashMap::new()\n}",
        )
        .expect("pattern matched");
        let s = squash(&out);
        assert!(s.contains(&squash("rustc_hash::FxHashMap<u16, u16>")), "{out}");
        assert!(s.contains(&squash("rustc_hash::FxHashMap::new()")), "{out}");
        assert!(!s.contains("std::collections"), "{out}");
    }

    #[test]
    fn output_reparses() {
        let out = propose(
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
            propose("fn f() -> std::collections::HashMap<u32, u32> {\n    \
                     std::collections::HashMap::new()\n}")
                .is_none()
        );
    }

    #[test]
    fn dead_use_unchanged() {
        // Use exists but zero body refs — swapping is noise.
        assert!(
            propose("use std::collections::HashMap;\nfn f() -> u32 { 42 }").is_none()
        );
    }

    #[test]
    fn glob_use_unchanged() {
        assert!(
            propose(
                "use std::collections::*;\nfn f(m: &HashMap<u32, u32>) -> u32 { 0 }"
            )
            .is_none()
        );
    }

    #[test]
    fn pub_use_unchanged() {
        // Re-export — changing it alters the crate's public API.
        assert!(
            propose(
                "pub use std::collections::HashMap;\nfn f(m: &HashMap<u32, u32>) -> u32 { 0 }"
            )
            .is_none()
        );
    }
}
