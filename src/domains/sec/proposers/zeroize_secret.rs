//! `secret-key-without-zeroize` — SUGGEST-ONLY detector (Batch 327, the
//! firezone/sanstun distill; Research 364 / Plan 332).
//!
//! A struct in the secret vocabulary (`*Key`/`*Secret`/`*Seed`/`*Token`/
//! `*Password`/`*Credential`/`*Priv*` as word segments) holding RAW key
//! material (`[u8; N]`, `Vec<u8>`, `String`) with no `Drop` impl and no
//! zeroization wrapper keeps the secret bytes alive in FREED MEMORY after
//! the struct drops — heap spray, core dumps and swap all read them back.
//! The fix is a lifetime, not a value change: wrap the field in
//! `zeroize::Zeroizing` (drop zeroes the bytes) or derive
//! `Zeroize`/`ZeroizeOnDrop`.
//!
//! ## The source anchor (firezone/sanstun @ `35cb1932948bd0776`)
//!
//! keys.rs carries the law as doc comments and as types: `An X25519
//! static private key, zeroized on drop.` on `StaticSecret`, and
//! `A WireGuard preshared key, zeroized on drop.` over
//! `pub struct PresharedKey(Zeroizing<[u8; 32]>);`. The census at the
//! pin: `Zeroizing` wraps owned `[u8; 32]` key material across 8 files
//! (crypto 10 / keys 8 / interface 8 / handshake 7 / session 3 / rng 2 /
//! message 2 spellings) — fields, params and returns alike. The pattern
//! is not an ornament: the handshake state machine hands chaining keys
//! and secrets through `Zeroizing<[u8; KEY_LEN]>` end to end.
//!
//! ## Why suggest-only (never a fix)
//!
//! The wrapper choice is an API decision the struct's users feel:
//! `Zeroizing::new(..)` changes the field type, `#[derive(Zeroize,
//! ZeroizeOnDrop)]` adds trait bounds and drop glue, a custom Drop
//! changes move semantics. NO `FIXABLE_SEC` membership, NO
//! `collect_edits` arm — the suggestion surfaces through retrieval and
//! review.
//!
//! ## Dep gate (the `constant-time-secret-compare` house shape)
//!
//! Fires only when the file already carries a `zeroize` import (any
//! `use ... zeroize ...` form). The import proves the dependency
//! resolves in this compilation unit; the suggestion cannot break the
//! build. A crate doing secret hygiene WITHOUT zeroize here is missed
//! (false negative = safe skip).
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Secret vocabulary**: the struct name's word segments (`snake_case`
//!   AND `CamelCase` split — `SessionKey` -> session + key) hit
//!   key/secret/seed/token/password/credential/priv as EXACT segments,
//!   never substrings.
//! - **Raw material**: the field type is a `[u8; N]` array (u8
//!   element), `Vec<u8>`, or `String`, NOT already wrapped in
//!   `Zeroizing<..>`.
//! - **Suppression, struct-wide**: any `impl Drop for <Struct>` in the
//!   file, OR any non-doc attribute on the struct mentioning zeroize
//!   (`#[derive(Zeroize)]`, `#[derive(ZeroizeOnDrop)]`,
//!   `#[zeroize(drop)]`), silences the struct — the lifetime is already
//!   owned. Doc comments never count (`/// zeroized on drop` is prose,
//!   not a derive); a field already NAMED zeroize-something skips that
//!   field.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped; macro statements are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Wrapper-typed secrets (`x25519_dalek::StaticSecret` fields) parse
//!   as opaque and stay silent — only the three raw shapes fire.
//! - A Drop impl in ANOTHER file is invisible (same-file walk).
//! - Findings are per-field: a struct with one wrapped and one raw
//!   field still fires on the raw one (partial wrapping is the bug).
//! - A file using zeroize only through fully-qualified paths (no `use`
//!   item) is an honest miss — the dep gate reads imports, mirroring
//!   `secret_ct_eq`.
//! - The vocabulary proves a NAME CLASS, not a secret: a `TokenBucket`
//!   rate-limit struct over `Vec<u8>` fires too, and review decides
//!   (suggest-only is the blast-radius control).
//! - Distinct-from `constant-time-secret-compare` IN-RULE: that rule
//!   owns the COMPARE primitive (the timing leak at comparison); this
//!   owns the LIFETIME/ERASE axis (bytes persisting after drop).
//!   Complementary defects, distinct repairs.
//! - Batch 327 / Plan 332 / Research 364 (firezone/sanstun @
//!   `35cb1932948bd0776`, Apache-2.0 — cited as class provenance; the
//!   fixture is an original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{File, Item, Type};

/// One detected unwrapped secret-material field.
#[derive(Debug, Clone)]
pub struct SecretZeroizeFinding {
    /// Byte range of the raw-material field.
    pub site_span: Range<usize>,
    /// The field's source text.
    pub site_text: String,
    /// The struct that holds it.
    pub struct_name: String,
    /// The advice line (the wrap forms + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a secret-vocabulary struct holds RAW key material \
with no zeroization — the bytes persist in freed memory (heap spray, core \
dumps, swap) after the struct drops. Wrap the field in `zeroize::Zeroizing` \
or derive `Zeroize`/`ZeroizeOnDrop` so drop erases them (Batch 327, \
suggest-only)";

/// The secret vocabulary (exact word segments of the struct name).
const SECRET_VOCAB: &[&str] = &[
    "key",
    "secret",
    "seed",
    "token",
    "password",
    "credential",
    "priv",
];

/// Detect secret structs whose key material never zeroes (suggest-only;
/// no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<SecretZeroizeFinding> {
    // Dep gate (the constant-time-secret-compare shape): no zeroize
    // import, no fire — the suggestion could not resolve.
    if !imports_zeroize(file) {
        return Vec::new();
    }
    let mut drops = Vec::new();
    collect_drop_impls(&file.items, &mut drops);
    let mut findings = Vec::new();
    scan_structs(source, &file.items, &drops, &mut findings);
    findings
}

/// Any `use ... zeroize ...` import path proves the dep resolves (the
/// `secret_ct_eq` walker shape).
fn imports_zeroize(file: &File) -> bool {
    use syn::visit::Visit;
    struct ImportFinder(bool);
    impl<'ast> Visit<'ast> for ImportFinder {
        fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
            let ts = u.tree.to_token_stream();
            if ts
                .to_string()
                .split_whitespace()
                .any(|seg| seg.trim_end_matches(',') == "zeroize")
            {
                self.0 = true;
            }
        }
    }
    let mut f = ImportFinder(false);
    f.visit_file(file);
    f.0
}

/// Every type a same-file `impl Drop for T` targets (any non-test mod).
fn collect_drop_impls(items: &[Item], out: &mut Vec<String>) {
    for item in items {
        match item {
            Item::Impl(imp) => {
                if let Some((_, tp, _)) = &imp.trait_ {
                    if tp.segments.last().is_some_and(|s| s.ident == "Drop") {
                        if let Some(name) = type_last_ident(&imp.self_ty) {
                            out.push(name);
                        }
                    }
                }
            }
            Item::Mod(m) => {
                // Comment guard (house discipline): #[cfg(test)] mods
                // are fixture habitat.
                if is_test_mod(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    collect_drop_impls(nested, out);
                }
            }
            _ => {}
        }
    }
}

fn scan_structs(
    source: &str,
    items: &[Item],
    drops: &[String],
    findings: &mut Vec<SecretZeroizeFinding>,
) {
    for item in items {
        match item {
            Item::Struct(s) => scan_struct(source, s, drops, findings),
            Item::Mod(m) => {
                if is_test_mod(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    scan_structs(source, nested, drops, findings);
                }
            }
            _ => {}
        }
    }
}

fn scan_struct(
    source: &str,
    s: &syn::ItemStruct,
    drops: &[String],
    findings: &mut Vec<SecretZeroizeFinding>,
) {
    let segs = name_segments(&s.ident.to_string());
    if !segs.iter().any(|sg| SECRET_VOCAB.contains(&sg.as_str())) {
        return;
    }
    let name = s.ident.to_string();
    // Suppression: a non-doc attribute mentioning zeroize (a derive or
    // a `#[zeroize(drop)]` marker) owns the lifetime. Doc comments are
    // prose, never a derive.
    let attr_zeroize = s.attrs.iter().any(|a| {
        !a.path().is_ident("doc")
            && a.to_token_stream()
                .to_string()
                .to_ascii_lowercase()
                .contains("zeroize")
    });
    if attr_zeroize || drops.iter().any(|d| d == &name) {
        return;
    }
    for field in &s.fields {
        // An already-wrapped field skips; a field already NAMED
        // zeroize-something skips too (the owner claims it).
        if is_zeroizing_ty(&field.ty) {
            continue;
        }
        if field
            .ident
            .as_ref()
            .is_some_and(|i| i.to_string().to_ascii_lowercase().contains("zeroiz"))
        {
            continue;
        }
        if !is_raw_secret_ty(&field.ty) {
            continue;
        }
        let span = field.span().byte_range();
        if span.end > source.len() || span.start >= span.end {
            continue;
        }
        findings.push(SecretZeroizeFinding {
            site_text: source[span.clone()].to_string(),
            site_span: span,
            struct_name: name.clone(),
            advice: ADVICE,
        });
    }
}

fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

/// The name's word segments: `snake_case` splits on `_`, `CamelCase`
/// splits at case boundaries (`SessionKey` -> session + key). The
/// vocabulary gates match EXACT segments, never substrings — `monkey`
/// never matches `key`.
fn name_segments(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut segs = Vec::new();
    let mut cur = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let prev = if i > 0 { chars[i - 1] } else { c };
        let next = chars.get(i + 1).copied();
        if c == '_' {
            if !cur.is_empty() {
                segs.push(std::mem::take(&mut cur));
            }
            continue;
        }
        let boundary = i > 0
            && c.is_ascii_uppercase()
            && (prev.is_ascii_lowercase()
                || (prev.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase())));
        if boundary && !cur.is_empty() {
            segs.push(std::mem::take(&mut cur));
        }
        cur.push(c.to_ascii_lowercase());
    }
    if !cur.is_empty() {
        segs.push(cur);
    }
    segs
}

fn last_path_segment(ty: &Type) -> Option<&syn::PathSegment> {
    if let Type::Path(tp) = ty {
        if tp.qself.is_none() {
            return tp.path.segments.last();
        }
    }
    None
}

fn type_last_ident(ty: &Type) -> Option<String> {
    last_path_segment(ty).map(|s| s.ident.to_string())
}

fn is_zeroizing_ty(ty: &Type) -> bool {
    last_path_segment(ty).is_some_and(|s| s.ident == "Zeroizing")
}

/// The raw-secret shapes: `[u8; N]` (u8 element), `Vec<u8>`, `String`.
fn is_raw_secret_ty(ty: &Type) -> bool {
    if let Type::Array(ta) = ty {
        return last_path_segment(&ta.elem).is_some_and(|s| s.ident == "u8");
    }
    let Some(seg) = last_path_segment(ty) else {
        return false;
    };
    if seg.ident == "String" {
        return true;
    }
    if seg.ident != "Vec" {
        return false;
    }
    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
        for a in &ab.args {
            if let syn::GenericArgument::Type(inner) = a {
                return last_path_segment(inner).is_some_and(|s| s.ident == "u8");
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::SecretZeroizeFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    const IMPORT: &str = "use zeroize::Zeroizing;\n";

    #[test]
    fn raw_secret_field_fires() {
        let src = format!(
            "{IMPORT}\npub struct SessionKey {{\n    pub tenant: u32,\n    \
             pub material: [u8; 32],\n}}\n"
        );
        let f = findings(&src);
        assert_eq!(f.len(), 1, "the raw [u8; 32] field fires");
        assert_eq!(f[0].struct_name, "SessionKey");
        assert!(f[0].site_text.contains("[u8; 32]"));
    }

    #[test]
    fn vec_and_string_material_fire() {
        let src = format!(
            "{IMPORT}\npub struct ApiSeed {{\n    pub rows: Vec<u8>,\n    \
             pub label: String,\n}}\n"
        );
        assert_eq!(findings(&src).len(), 2, "Vec<u8> and String both fire");
    }

    #[test]
    fn wrapped_field_is_silent() {
        // the fixed shape: the field IS Zeroizing-wrapped (keys.rs's
        // PresharedKey posture).
        let src = format!(
            "{IMPORT}\npub struct PresharedKey {{\n    pub bytes: \
             Zeroizing<[u8; 32]>,\n}}\n"
        );
        assert!(findings(&src).is_empty(), "wrapped material stays silent");
    }

    #[test]
    fn drop_impl_suppresses() {
        let src = format!(
            "{IMPORT}\npub struct SessionKey {{\n    pub material: [u8; 32],\n}}\n\n\
             impl Drop for SessionKey {{\n    fn drop(&mut self) {{\n        \
             self.material = [0_u8; 32];\n    }}\n}}\n"
        );
        assert!(findings(&src).is_empty(), "a Drop impl owns the lifetime");
    }

    #[test]
    fn zeroize_derive_suppresses() {
        let src = format!(
            "{IMPORT}\n#[derive(Clone, zeroize::Zeroize, zeroize::ZeroizeOnDrop)]\n\
             pub struct SessionKey {{\n    pub material: [u8; 32],\n}}\n"
        );
        assert!(findings(&src).is_empty(), "the derive owns the lifetime");
    }

    #[test]
    fn doc_words_do_not_own_the_lifetime() {
        // `/// zeroized on drop` is prose — suppression comes from the
        // WRAP or the derive, never the words.
        let src = format!(
            "{IMPORT}\n/// Claims zeroization in prose only.\npub struct \
             SessionKey {{\n    pub material: [u8; 32],\n}}\n"
        );
        assert_eq!(findings(&src).len(), 1, "a doc mention is not a derive");
    }

    #[test]
    fn non_secret_struct_is_silent() {
        let src =
            format!("{IMPORT}\npub struct CacheConfig {{\n    pub nonce_seed: [u8; 32],\n}}\n");
        assert!(
            findings(&src).is_empty(),
            "a non-secret struct name never fires (field names do not matter)"
        );
    }

    #[test]
    fn no_zeroize_import_is_silent() {
        // The dep gate: identical struct, no import — never fires (the
        // fxhashmap E0432 class).
        let src = "pub struct SessionKey {\n    pub material: [u8; 32],\n}\n";
        assert!(findings(src).is_empty(), "no zeroize import, no fire");
    }

    #[test]
    fn camel_case_name_splits() {
        // SessionKey -> ["session", "key"]: the CamelCase split is the
        // vocabulary gate.
        let src = format!("{IMPORT}\npub struct SessionKey {{\n    pub material: [u8; 32],\n}}\n");
        assert_eq!(findings(&src).len(), 1);
    }

    #[test]
    fn substring_names_stay_silent() {
        // `Monkey` contains `key` as a SUBSTRING — exact segments only.
        let src = format!("{IMPORT}\npub struct Monkey {{\n    pub bananas: [u8; 32],\n}}\n");
        assert!(
            findings(&src).is_empty(),
            "substring vocabulary never fires"
        );
    }

    #[test]
    fn test_mod_is_skipped() {
        let src = format!(
            "{IMPORT}\n#[cfg(test)]\nmod tests {{\n    struct SessionKey {{\n        \
             material: [u8; 32],\n    }}\n}}\n"
        );
        assert!(findings(&src).is_empty(), "fixture habitat stays silent");
    }

    #[test]
    fn corpus_buggy_fixture_fires() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "secret-key-without-zeroize")
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
