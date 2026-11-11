//! `ecb-block-mode-selection` — SUGGEST-ONLY detector
//! (Batch 350, the 0sec-labs/0 distill; Research 388 / Plan 356).
//!
//! A block cipher selected in ECB mode: an `Ecb<..>`-spelled type (the
//! `RustCrypto` `ecb` crate's `Ecb<Aes128>` alias, or a cipher shorthand
//! carrying an `Ecb`/`ecb` segment like `Aes128Ecb`), or a cipher-name
//! string literal in the `aes-*-ecb` shape. ECB encrypts each block
//! independently with no IV or chaining, so identical plaintext blocks
//! map to identical ciphertext blocks — the structure of the plaintext
//! leaks through the ciphertext (the canonical "ECB penguin"), and the
//! mode provides no semantic security at all (CWE-327). The fix is an
//! authenticated mode: `AesGcm` / `ChaCha20Poly1305` with a fresh random
//! nonce per message.
//!
//! ## The source anchor (0sec-labs/0 @ `f7020eb2`)
//!
//! `packages/core/src/crypto-misuse-detector.ts` detector 3
//! (`detectEcbMode`), verbatim law at the pin: "ECB encrypts each block
//! independently, so it leaks plaintext structure... Always
//! security-relevant — no extra context gate needed." Their JS face is
//! the cipher-name string (`aes-128-ecb`, `mode: ECB`); the Rust face is
//! the TYPE spelling (`ecb::Ecb<Aes128>`, `Aes128Ecb` ciphersuite
//! shorthands) — the vocabulary moves, the law does not.
//!
//! ## Why suggest-only (never a fix)
//!
//! The AEAD choice (`AES-GCM` vs `ChaCha20Poly1305` vs `XChaCha`) is a
//! deployment/interop decision (hardware AES-NI, protocol peer
//! support), and the nonce discipline (random vs counter) is the
//! protocol's. NO `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Type gate**: an ident whose exact text is `Ecb` or ENDS in
//!   `Ecb` (case-sensitive — `Aes128Ecb`, `Ecb`, `TdesEcb`), in a type
//!   position (type alias, fn signature, let binding annotation, path
//!   segment). The segment match keeps the substring hazard bounded:
//!   `TrebIeCeb`-class innocuous idents never carry an `Ecb` suffix.
//! - **Literal gate**: a string literal matching the cipher-name shape
//!   `aes`/`des`/`bf`/`camellia`/`rc2`/`seed` stem plus an `-ecb`
//!   suffix, word-anchored (the source's ECB-RX transcribed).
//! - **No context gate, deliberately**: ECB is always
//!   security-relevant per the source's own law — no vocabulary gate.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped; macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Mode selection by RUNTIME string (a `CipherSuite::from_name(..)`
//!   dispatch over config data) is an honest NO — the enum arm spelling
//!   the ECB variant fires via the ident gate only if it carries `Ecb`
//!   verbatim.
//! - A LEGACY-interop type alias (`type LegacyPeerCipher = Ecb<Aes128>`
//!   kept for a protocol peer that cannot move) fires and review
//!   decides — suggest-only is the blast-radius control; the class has
//!   no legitimate arm in NEW code, which is why there is no
//!   suppression marker at all (the source's own no-context-gate law).
//! - Cross-file cipher selection (mode chosen in one file, cipher in
//!   another) is unconnected parse-only.
//! - Batch 350 / Plan 356 / Research 388 (0sec-labs/0 @ `f7020eb2`,
//!   MIT OR Apache-2.0 — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{File, Item};

/// One detected ECB-mode selection site.
#[derive(Debug, Clone)]
pub struct EcbModeFinding {
    /// Byte range of the ECB-spelled type or literal.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// Which gate fired.
    pub gate: &'static str,
    /// The advice line (the AEAD swap + the leak law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "ECB encrypts each block independently — identical \
plaintext blocks map to identical ciphertext blocks, so the plaintext's \
structure leaks through the ciphertext and the mode provides no semantic \
security (the ECB-penguin class, CWE-327). Use an authenticated mode: \
`AesGcm` / `ChaCha20Poly1305` with a fresh random nonce per message \
(Batch 350, suggest-only)";

/// Detect ECB block-mode selection (suggest-only; no edits are produced
/// by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<EcbModeFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<EcbModeFinding>) {
    match item {
        Item::Fn(f) => scan_fn_body(source, &f.block, findings),
        Item::Type(t) => scan_type_expr(source, &t.ty, findings),
        Item::Struct(s) => {
            for f in &s.fields {
                scan_type_expr(source, &f.ty, findings);
            }
        }
        Item::Impl(imp) => {
            for ii in &imp.items {
                match ii {
                    syn::ImplItem::Fn(m) => scan_fn_body(source, &m.block, findings),
                    syn::ImplItem::Type(t) => scan_type_expr(source, &t.ty, findings),
                    _ => {}
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production cipher selection.
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

fn scan_fn_body(source: &str, block: &syn::Block, findings: &mut Vec<EcbModeFinding>) {
    use syn::visit::Visit;
    struct V<'a> {
        source: &'a str,
        findings: &'a mut Vec<EcbModeFinding>,
    }
    impl Visit<'_> for V<'_> {
        fn visit_expr(&mut self, e: &syn::Expr) {
            if let syn::Expr::Lit(l) = e {
                if let syn::Lit::Str(s) = &l.lit {
                    if literal_is_ecb_cipher_name(&s.value()) {
                        self.push(e.span().byte_range(), "cipher-name literal");
                    }
                }
            }
            // The shorthand-cipher face: `Aes128Ecb::new_from_slice(..)`
            // is an expression PATH, not a type — the segments are
            // checked here.
            if let syn::Expr::Path(p) = e {
                self.check_path(p);
            }
            syn::visit::visit_expr(self, e);
        }
        fn visit_type(&mut self, t: &syn::Type) {
            check_type(self.source, t, self.findings);
            syn::visit::visit_type(self, t);
        }
    }
    impl V<'_> {
        fn check_path(&mut self, p: &syn::ExprPath) {
            for seg in &p.path.segments {
                let name = seg.ident.to_string();
                if name == "Ecb" || (name.len() > 3 && name.ends_with("Ecb")) {
                    let span = seg.span().byte_range();
                    self.push(span, "Ecb-typed cipher");
                }
            }
        }
        fn push(&mut self, span: Range<usize>, gate: &'static str) {
            if span.end > self.source.len() || span.start >= span.end {
                return;
            }
            self.findings.push(EcbModeFinding {
                site_text: self.source[span.clone()].to_string(),
                site_span: span,
                gate,
                advice: ADVICE,
            });
        }
    }
    let mut v = V { source, findings };
    v.visit_block(block);
}

fn scan_type_expr(source: &str, ty: &syn::Type, findings: &mut Vec<EcbModeFinding>) {
    use syn::visit::Visit;
    struct V<'a> {
        source: &'a str,
        findings: &'a mut Vec<EcbModeFinding>,
    }
    impl Visit<'_> for V<'_> {
        fn visit_type(&mut self, t: &syn::Type) {
            check_type(self.source, t, self.findings);
            syn::visit::visit_type(self, t);
        }
    }
    let mut v = V { source, findings };
    v.visit_type(ty);
}

/// The type gate: a path segment whose ident is `Ecb` or ends in `Ecb`
/// (case-sensitive exact suffix — `Aes128Ecb` fires, `TrebIeCeb` never).
fn check_type(source: &str, ty: &syn::Type, findings: &mut Vec<EcbModeFinding>) {
    if let syn::Type::Path(tp) = ty {
        for seg in &tp.path.segments {
            let name = seg.ident.to_string();
            if name == "Ecb" || (name.len() > 3 && name.ends_with("Ecb")) {
                let span = seg.span().byte_range();
                if span.end <= source.len() && span.start < span.end {
                    findings.push(EcbModeFinding {
                        site_text: source[span.clone()].to_string(),
                        site_span: span,
                        gate: "Ecb-typed cipher",
                        advice: ADVICE,
                    });
                }
            }
        }
    }
}

/// The literal gate: an `aes`/`des`/`bf`/`camellia`/`rc2`/`seed` stem
/// plus optional key size plus an `-ecb`/`_ecb` suffix, word-anchored on
/// the suffix (the source's `ECB_RX` transcribed; the recorded hazard).
fn literal_is_ecb_cipher_name(lit: &str) -> bool {
    const STEMS: &[&str] = &["aes", "des", "bf", "camellia", "rc2", "seed"];
    let l = lit.to_ascii_lowercase();
    for stem in STEMS {
        let Some(rest) = l.strip_prefix(stem) else {
            continue;
        };
        // (sep)? (size)? (sep)? then `ecb` + boundary — the cipher-name
        // conventions: `aes-128-ecb`, `aes128ecb`, `aes-ecb`, `aes_ecb`.
        let after_sep1 = rest
            .strip_prefix('-')
            .or_else(|| rest.strip_prefix('_'))
            .unwrap_or(rest);
        let after_size = ["128", "192", "256"]
            .iter()
            .find_map(|s| after_sep1.strip_prefix(s))
            .unwrap_or(after_sep1);
        let after_sep2 = after_size
            .strip_prefix('-')
            .or_else(|| after_size.strip_prefix('_'))
            .unwrap_or(after_size);
        if let Some(r) = after_sep2.strip_prefix("ecb") {
            if r.is_empty() || !r.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::EcbModeFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 350 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch350_ecb_type_alias_fires() {
        let src = "type LegacyCipher = Ecb<Aes128>;\n\n\
             pub fn seal(key: &Key, pt: &[u8]) -> Vec<u8> {\n    \
             encrypt::<LegacyCipher>(key, pt)\n}\n";
        let f = findings(src);
        assert!(!f.is_empty(), "the Ecb type fires: {f:?}");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The ciphersuite shorthand spelling fires via the suffix gate.
    #[test]
    fn aes128ecb_shorthand_fires() {
        let src = "pub fn seal(key: &Key, pt: &[u8]) -> Vec<u8> {\n    \
             let c = Aes128Ecb::new_from_slice(key).unwrap();\n    \
             encrypt_block(c, pt)\n}\n";
        assert!(!findings(src).is_empty());
    }

    /// The cipher-name literal gate fires.
    #[test]
    fn cipher_name_literal_fires() {
        let src = "pub fn suite_name() -> String {\n    \
             String::from(\"aes-128-ecb\")\n}\n";
        assert_eq!(findings(src).len(), 1);
    }

    /// AEAD spellings stay silent (the fixed posture).
    #[test]
    fn aead_is_silent() {
        let src = "pub fn seal(key: &Key, pt: &[u8], nonce: &[u8]) -> Vec<u8> {\n    \
             let c = AesGcm::<Aes128, U12>::new_from_slice(key).unwrap();\n    \
             let _ = nonce;\n    encrypt_aead(c, pt)\n}\n";
        assert!(findings(src).is_empty(), "AEAD stays silent");
    }

    /// The substring hazard is bounded: innocuous `Ecb`-substring idents
    /// and unrelated `-ecb`-less names stay silent.
    #[test]
    fn substring_hazard_bounded() {
        let src = "pub fn treble_ceb() -> u32 {\n    \
             let treb_ie_ceb = 7;\n    \
             let suite = \"aes-128-gcm\";\n    \
             treb_ie_ceb + suite.len() as u32\n}\n";
        assert!(findings(src).is_empty(), "no Ecb exact-suffix, no fire");
    }

    /// A struct field typed over the ECB alias fires (the storage face).
    #[test]
    fn struct_field_type_fires() {
        let src = "pub struct SealedBlob {\n    cipher: Ecb<Aes128>,\n}\n";
        assert!(!findings(src).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             type T = Ecb<Aes128>;\n    fn t() {}\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Batch 350 gate — the corpus `fixed_code` MUST stay silent.
    #[test]
    fn corpus_fixed_fixture_is_silent() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "ecb-block-mode-selection")
            .expect("rule present");
        let fixed = syn::parse_file(&rule.fixed_code).expect("fixed parses");
        assert!(
            detect(&rule.fixed_code, &fixed).is_empty(),
            "the fixed fixture must stay silent"
        );
    }
}
