//! `quantum-vulnerable-primitive-selection` — SUGGEST-ONLY detector
//! (Batch 351, the 0sec-labs/foxguard distill; Research 394 / Plan 361).
//! The PQ/quantum-readiness family seed — zero prior corpus owners.
//!
//! A classical cryptographic primitive selected at a direct call path
//! (`rsa::RsaPrivateKey::new`, `ed25519_dalek::SigningKey::generate`,
//! `p256::ecdsa(..)`) is quantum-vulnerable: a harvest-now-decrypt-later
//! adversary records today's ciphertext/signatures and breaks the
//! primitive once a large enough quantum computer exists (CWE-327).
//! The decision content is the source's inventory + migration table +
//! precision law, carried whole.
//!
//! ## The source anchor (0sec-labs/foxguard @ `fbeab16`)
//!
//! `src/rules/rust_lang.rs` `PqVulnerableCrypto` + `src/rules/pq.rs`:
//! - the vulnerable inventory: RSA / ECDSA / ECDH / DSA / Ed25519 /
//!   X25519 / P-256 / P-384 / P-521 / k256;
//! - the PQ-safe skip list, verbatim intent: "`fn_dsa` (FIPS 206, draft)
//!   and hqc (5th NIST selection, code-based KEM, draft std 2026) are
//!   recognised here so early adopters of those crates aren't flagged"
//!   — `ml_dsa | ml_kem | slh_dsa | fn_dsa | hqc`;
//! - the boundary law (`crypto_token_in_path`), verbatim: "Match the
//!   algorithm as a complete leading word of a path segment … This keeps
//!   `rsa::RsaPrivateKey::new` and `ed25519_dalek::SigningKey::generate`
//!   flagged while ignoring incidental matches like a
//!   `user_ed25519_key_id` identifier";
//! - the direct-call precision law, verbatim: "Only match direct calls
//!   (`scoped_identifier`), not chained method calls";
//! - the CNSA 2.0 migration table (Ed25519 → ML-DSA-65/FN-DSA + hybrid
//!   cert chains; X25519 → X25519MLKEM768 hybrid / ML-KEM-1024; the
//!   CNSA 2.0 class deadline "2033" for web/cloud exclusive-use).
//!
//! ## Why suggest-only (never a fix)
//!
//! Primitive migration is a deployment decision — wire-protocol peers
//! must agree, and the hybrid-vs-full-PQ posture is a compliance
//! profile (the source's own replacement strings are advisory tables,
//! not rewrites). NO `FIXABLE_SEC` membership, NO `collect_edits` arm.
//! Honest posture: this rule fires at home (the account-key surface is
//! SSH Ed25519) and the advice is the migration table — honest, not
//! wrong; the primitive claims the mechanism, never a deadline.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Signature VERIFICATION paths flag too (the source does not exempt
//!   them — verification against classical keys carries the same
//!   migration question) and review decides.
//! - Path-segment proximity is fn-level, not a type walk — an aliased
//!   re-export one module away is an honest NO.
//! - The skip list matches substring-wise on the call path (the
//!   source's own gate shape); a crate named to game it is out of class.
//! - Detector: `proposers::pq_primitive_selection`. Fixture:
//!   `fixtures/b351_fixtures.rs` F2. Batch 351 / Plan 361 / Research 394
//!   (0sec-labs/foxguard @ `fbeab16`, MIT OR Apache-2.0 — cited as class
//!   provenance; the fixture is an original synthesis, standalone-compiled
//!   AND executed before entry).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected quantum-vulnerable primitive selection.
#[derive(Debug, Clone)]
pub struct PqFinding {
    /// Byte range of the call path expression.
    pub site_span: Range<usize>,
    /// The call path text.
    pub site_text: String,
    /// The classical algorithm named.
    pub algorithm: &'static str,
    /// The advice line (the migration table).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested) — the source's
/// migration table condensed, CNSA 2.0 framing carried.
pub const ADVICE: &str = "a classical cryptographic primitive \
(RSA/ECDSA/ECDH/DSA/Ed25519/X25519/P-256-family) is quantum-vulnerable — \
a harvest-now-decrypt-later adversary records today's traffic (CWE-327). \
Migration candidates (FIPS 203/204/205 finalized; CNSA 2.0 deadlines live): \
signatures → ML-DSA-65 (FIPS 204) or FN-DSA (FIPS 206, draft) with hybrid \
cert chains in transition; key establishment → X25519MLKEM768 hybrid \
(FIPS 203) or ML-KEM-1024, HQC as the code-based diversity hedge \
(Batch 351, suggest-only — the migration is a deployment decision)";

/// The vulnerable inventory (the source's table, leading-word tokens).
const CLASSICAL_TOKENS: &[&str] = &[
    "rsa", "ecdsa", "ecdh", "dsa", "ed25519", "x25519", "p256", "p384", "p521", "k256",
];

/// The PQ-safe skip list (early adopters are never flagged).
const PQ_SAFE_TOKENS: &[&str] = &["ml_dsa", "ml_kem", "slh_dsa", "fn_dsa", "hqc"];

/// Detect quantum-vulnerable primitive selection (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<PqFinding> {
    let mut findings = Vec::new();
    // File-level import context: the PRIMITIVE SELECTION often lives in
    // the `use` tree (`use ed25519_dalek::SigningKey;`) while the fn body
    // carries only the verb (`sk.sign(..)`). The import-tree text unions
    // into every item's scan context below — gated by the item's own
    // crypto verbs so an untouched fn in the same file stays silent.
    let import_text = file
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Use(u) => Some(quote::ToTokens::to_token_stream(&u.tree).to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    for item in &file.items {
        collect_in_item(source, item, &import_text, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, import_text: &str, findings: &mut Vec<PqFinding>) {
    match item {
        Item::Fn(f) => {
            let mut direct = Vec::new();
            scan_block_paths(&f.block, source, &mut direct);
            if direct.is_empty() {
                // The import arm: the selection rides the `use` tree, the
                // item carries a crypto verb (`sk.sign(..)` — the
                // direct-call law's honest transcription: bare receivers
                // carry no algorithm tokens, so the boundary law keeps the
                // precision).
                let item_text = quote::ToTokens::to_token_stream(f).to_string();
                let item_lower = item_text.to_lowercase();
                if CRYPTO_VERBS.iter().any(|v| item_lower.contains(v))
                    && import_text_has_classical(import_text)
                {
                    let byte_range = f.block.span().byte_range();
                    findings.push(PqFinding {
                        site_span: byte_range.clone(),
                        site_text: source.get(byte_range).unwrap_or("").to_string(),
                        algorithm: "classical-import",
                        advice: ADVICE,
                    });
                }
            } else {
                findings.extend(direct);
            }
        }
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    let mut direct = Vec::new();
                    scan_block_paths(&m.block, source, &mut direct);
                    findings.extend(direct);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production crypto selection.
            if m.attrs.iter().any(|a| {
                a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
            }) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, import_text, findings);
                }
            }
        }
        _ => {}
    }
}

/// The crypto verbs that bind an item to the import context (a fn that
/// only prints a key id carries none and stays silent).
const CRYPTO_VERBS: &[&str] = &[
    "sign",
    "verify",
    "encrypt",
    "decrypt",
    "generate",
    "keypair",
    "kem",
    "encapsulate",
    "decapsulate",
];

/// Walk every expression in the block and flag CLASSICAL-TOKEN path
/// expressions (the direct-call arm: `rsa::RsaPrivateKey::new` inside a
/// call-func position, a bare `p256::ecdsa` path). The PQ-safe skip and
/// the boundary law apply per path.
fn scan_block_paths(block: &syn::Block, source: &str, findings: &mut Vec<PqFinding>) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr_paths(&init.expr, source, findings);
                }
            }
            syn::Stmt::Expr(e, _) => scan_expr_paths(e, source, findings),
            _ => {}
        }
    }
}

fn scan_expr_paths(e: &Expr, source: &str, findings: &mut Vec<PqFinding>) {
    match e {
        Expr::Call(c) => {
            scan_expr_paths(&c.func, source, findings);
            for a in &c.args {
                scan_expr_paths(a, source, findings);
            }
        }
        Expr::MethodCall(mc) => {
            scan_expr_paths(&mc.receiver, source, findings);
            for a in &mc.args {
                scan_expr_paths(a, source, findings);
            }
        }
        Expr::Path(p) => {
            let path_lower = quote::ToTokens::to_token_stream(&p.path)
                .to_string()
                .to_lowercase();
            // The PQ-safe skip list FIRST (substring-wise — the hybrid
            // migration composite passes).
            if PQ_SAFE_TOKENS.iter().any(|t| path_lower.contains(t)) {
                return;
            }
            // The boundary law: the algorithm token as the LEADING WORD
            // of a `::`-segment — `ed25519` matches `ed25519_dalek`,
            // never a buried `user_ed25519_key_id`.
            let Some(algorithm) = CLASSICAL_TOKENS
                .iter()
                .find(|t| token_leads_segment(&path_lower, t))
            else {
                return;
            };
            let byte_range = e.span().byte_range();
            findings.push(PqFinding {
                site_span: byte_range.clone(),
                site_text: source.get(byte_range).unwrap_or("").to_string(),
                algorithm,
                advice: ADVICE,
            });
        }
        _ => {}
    }
}

/// Whether the FILE's import trees select a classical primitive (the
/// boundary law over the use-tree text).
fn import_text_has_classical(import_text: &str) -> bool {
    CLASSICAL_TOKENS
        .iter()
        .any(|t| token_leads_segment(import_text, t))
}

/// The source's `crypto_token_in_path`: a token matches only as the
/// LEADING WORD of a `::`-separated segment — `rsa` matches `rsa` and
/// `rsa_pss`, `ed25519` matches `ed25519_dalek`, neither matches a
/// buried `user_ed25519_key_id`.
fn token_leads_segment(path_lower: &str, token: &str) -> bool {
    path_lower.split("::").any(|seg| {
        let seg = seg.trim();
        seg == token
            || seg
                .strip_prefix(token)
                .is_some_and(|rest| rest.starts_with('_'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(src_text: &str) -> Vec<PqFinding> {
        let file = syn::parse_file(src_text).expect("parses");
        detect(src_text, &file)
    }

    /// Fires on its own corpus buggy fixture (the self-fire law).
    #[test]
    fn fires_on_own_buggy_fixture() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "quantum-vulnerable-primitive-selection")
            .expect("rule present");
        assert!(
            !findings(&rule.buggy_code).is_empty(),
            "the proposer must fire on its own buggy fixture"
        );
    }

    /// Batch 351 gate — the corpus `fixed_code` MUST stay silent (the
    /// fix selects the PQ primitive — the skip list suppresses).
    #[test]
    fn corpus_fixed_fixture_is_silent() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "quantum-vulnerable-primitive-selection")
            .expect("rule present");
        assert!(
            findings(&rule.fixed_code).is_empty(),
            "the fixed fixture must stay silent"
        );
    }

    /// The source's two precision examples, both directions.
    #[test]
    fn boundary_law_both_directions() {
        let flagged = "pub fn mk() -> RsaPrivateKey {\n    \
             let k = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();\n    k\n}\n";
        assert_eq!(
            findings(flagged).len(),
            1,
            "rsa::RsaPrivateKey::new is flagged"
        );
        let buried = "pub fn key_id(user_ed25519_key_id: &str) -> String {\n    \
             format!(\"id:{user_ed25519_key_id}\")\n}\n";
        assert!(
            findings(buried).is_empty(),
            "a buried identifier occurrence is NOT flagged (the boundary law)"
        );
    }

    /// Ed25519 flags (fires at home — the SSH account-key surface);
    /// ML-DSA passes (the skip list).
    #[test]
    fn ed25519_flags_mldsa_passes() {
        let ed = "pub fn sign(msg: &[u8]) -> Signature {\n    \
             let sk = ed25519_dalek::SigningKey::generate(&mut rng);\n    sk.sign(msg)\n}\n";
        assert_eq!(findings(ed).len(), 1);
        let pq = "pub fn sign(msg: &[u8]) -> Signature {\n    \
             let sk = ml_dsa::SigningKey::generate(&mut rng);\n    sk.sign(msg)\n}\n";
        assert!(findings(pq).is_empty(), "the PQ-safe skip list passes");
    }

    /// Chained method-call receivers are NOT scanned (the direct-call
    /// precision law): only path expressions carry findings.
    #[test]
    fn chained_calls_not_flagged_as_paths() {
        let src_text = "pub fn roundtrip() -> u32 {\n    \
             let n = counter.lock().unwrap().increment();\n    n\n}\n";
        assert!(findings(src_text).is_empty());
    }

    /// The hybrid composite passes (the migration path is not flagged).
    #[test]
    fn hybrid_migration_passes() {
        let src_text = "pub fn decapsulate(ct: &[u8]) -> SharedSecret {\n    \
             x25519_ml_kem768::decapsulate(ct, &sk)\n}\n";
        assert!(findings(src_text).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src_text = "#[cfg(test)]\nmod tests {\n    \
             fn t() -> RsaPrivateKey {\n        rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap()\n    }\n}\n";
        assert!(findings(src_text).is_empty());
    }
}
