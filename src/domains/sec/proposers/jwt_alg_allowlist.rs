//! `jwt-alg-family-confusion-allowlist` — SUGGEST-ONLY detector
//! (Batch 350, the 0sec-labs/0 distill; Research 388 / Plan 356).
//!
//! A JWT validation configured with a WRONG or MULTI-FAMILY algorithm
//! allowlist: the `algorithms` list mixing a symmetric HMAC family
//! (`Algorithm::HS256`-class) with an asymmetric family
//! (`RS*`/`ES*`/`PS*`) — the HS-vs-RS confusion where an attacker
//! re-signs a forged token using the public key AS the HMAC secret and
//! the verifier accepts it (CWE-347's algorithm-confusion face) — or an
//! explicit `Algorithm::NONE` in the accepted set, the unsigned-token
//! forgery arm.
//!
//! ## The source anchor (0sec-labs/0 @ `f7020eb2`)
//!
//! `packages/core/src/crypto-misuse-detector.ts` detector 4
//! (`detectJwtAlgConfusion`): three faces at the pin — `alg: "none"`
//! acceptance, a verify allow-list mixing `HS\d{3}` with `(RS|ES|PS)\d{3}`
//! (their own regex is the transcription source for the family split),
//! and a verify with NO allow-list at all. The Rust face moves the
//! vocabulary to jsonwebtoken's `Validation`/`Algorithm` API; the
//! no-allowlist face is an honest recall limit here (see below).
//!
//! ## MANDATORY in-rule adjudication vs `jwt-insecure-validation-flags`
//! (the B325 discipline — plan 356 T2 R4)
//!
//! `jwt-insecure-validation-flags` (Batch 291) owns the DISABLED axis:
//! validation turned OFF (`insecure_disable_signature_validation()`,
//! `dangerous::insecure_decode(..)`, `validate_exp = false`) — the
//! verifier's checks are removed. THIS rule owns the WRONG-ALLOWLIST
//! axis: validation is ON and the verifier runs, but the accepted
//! algorithm SET is misconfigured (multi-family or `none`) — the check
//! passes and the forgery still verifies. Both directions are distinct:
//! flipping a disabled-validation site back ON does not fix a mixed
//! list, and pinning a clean list does not rescue a disabled validator.
//! They can co-occur on one fn and compose; the surfaces share no
//! tokens (`insecure_*`/`validate_exp` vs `Algorithm::`-list shapes).
//!
//! ## Why suggest-only (never a fix)
//!
//! The single algorithm to pin is the deployment's signing choice —
//! only the operator knows whether tokens are HS256 (shared secret) or
//! RS256 (key pair). NO `FIXABLE_SEC` membership, NO `collect_edits`
//! arm.
//!
//! ## The bounded form
//!
//! ```text
//! let mut validation = Validation::new(Algorithm::RS256); // ONE family
//! // (or: validation.algorithms = vec![Algorithm::ES256];)
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Mixed-family gate**: an `algorithms` field init (struct-literal
//!   field or assignment) whose array/vec literal carries at least one
//!   `HS*` variant AND at least one `RS*`/`ES*`/`PS*` variant — both
//!   families in one accepted set, any order.
//! - **None-arm gate**: `Algorithm::NONE` appearing in an
//!   `algorithms` list OR in a `Validation::new(..)` argument — the
//!   unsigned-token arm, by construction.
//! - **jwt-vocabulary gate** (the `validate_exp` shape from Batch 291):
//!   both gates require the enclosing fn to carry jwt vocabulary (a
//!   `jsonwebtoken`-prefixed path or a `Validation`/`DecodingKey`/
//!   `Algorithm` token) — another crate's `algorithms` field stays
//!   silent.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped; macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - **The no-allowlist face is an honest NO here**: a
//!   `decode::<T>(..)` call with no visible `algorithms` init in the fn
//!   is UNDECIDABLE parse-only (jsonwebtoken's `Validation::new`
//!   defaults the list to the one passed algorithm — the safe default —
//!   so absence is most plausibly CORRECT at this API, unlike the JS
//!   library the source audits, where absence trusts the header). The
//!   source's third face does not transfer.
//! - An allowlist assembled across helpers (one fn builds the vec, the
//!   validation consumes it in another) is unconnected parse-only.
//! - A deliberate multi-family migration window (accepting HS256 AND
//!   ES256 while tokens rotate) fires and review decides — suggest-only
//!   is the blast-radius control.
//! - Batch 350 / Plan 356 / Research 388 (0sec-labs/0 @ `f7020eb2`,
//!   MIT OR Apache-2.0 — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected wrong/multi-family JWT algorithm allowlist site.
#[derive(Debug, Clone)]
pub struct JwtAlgFinding {
    /// Byte range of the offending expression.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// Which gate fired.
    pub gate: &'static str,
    /// The advice line (the single-family pin + the confusion law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "the JWT algorithm allowlist accepts MULTIPLE \
families (or `none`): validation runs, but an attacker re-signs a forged \
token with the public key AS an HMAC secret (HS-vs-RS confusion) or ships \
an unsigned `none` token and the verifier accepts both (CWE-347). Pin the \
allowlist to the ONE algorithm actually in use \
(`Validation::new(Algorithm::..)`) (Batch 350, suggest-only)";

/// The jwt-vocabulary tokens gating both faces (the Batch-291 shape).
const JWT_TYPE_TOKENS: &[&str] = &["Validation", "DecodingKey", "Algorithm"];

/// Detect multi-family / none-accepting JWT allowlists (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<JwtAlgFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<JwtAlgFinding>) {
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
            // fixture habitat, not production token verification.
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

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<JwtAlgFinding>) {
    let fn_ts = quote::ToTokens::to_token_stream(block).to_string();
    let jwt_vocab =
        fn_ts.contains("jsonwebtoken::") || JWT_TYPE_TOKENS.iter().any(|t| fn_ts.contains(t));
    if !jwt_vocab {
        return;
    }
    scan_stmts(&block.stmts, source, findings);
}

fn scan_stmts(stmts: &[syn::Stmt], source: &str, findings: &mut Vec<JwtAlgFinding>) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr(&init.expr, source, findings);
                }
            }
            syn::Stmt::Item(Item::Fn(f)) => scan_stmts(&f.block.stmts, source, findings),
            syn::Stmt::Expr(e, _) => scan_expr(e, source, findings),
            _ => {}
        }
    }
}

fn scan_expr(e: &Expr, source: &str, findings: &mut Vec<JwtAlgFinding>) {
    match e {
        // `algorithms: vec![Algorithm::HS256, Algorithm::RS256]` /
        // `v.algorithms = vec![..]` — the mixed-family gate.
        Expr::Assign(a) => {
            if let Expr::Field(f) = a.left.as_ref() {
                if is_member(&f.member, "algorithms") {
                    check_list(source, e, &a.right, findings);
                }
            }
            scan_expr(&a.left, source, findings);
            scan_expr(&a.right, source, findings);
        }
        Expr::Struct(s) => {
            for field in &s.fields {
                if is_member(&field.member, "algorithms") {
                    check_list(source, e, &field.expr, findings);
                }
            }
            for field in &s.fields {
                scan_expr(&field.expr, source, findings);
            }
        }
        // `Validation::new(Algorithm::NONE)` / `Validation::new(..)` arg
        // lists — the none-arm gate at the constructor.
        Expr::Call(c) => {
            if let Expr::Path(p) = c.func.as_ref() {
                let segs: Vec<String> = p
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                if segs.last().is_some_and(|s| s == "new") && segs.iter().any(|s| s == "Validation")
                {
                    for arg in &c.args {
                        check_list(source, e, arg, findings);
                    }
                }
            }
            scan_expr(&c.func, source, findings);
            for a in &c.args {
                scan_expr(a, source, findings);
            }
        }
        Expr::If(i) => {
            scan_expr(&i.cond, source, findings);
            scan_stmts(&i.then_branch.stmts, source, findings);
            if let Some((_, else_e)) = &i.else_branch {
                scan_expr(else_e, source, findings);
            }
        }
        Expr::Block(b) => scan_stmts(&b.block.stmts, source, findings),
        Expr::Match(m) => {
            scan_expr(&m.expr, source, findings);
            for arm in &m.arms {
                scan_expr(&arm.body, source, findings);
            }
        }
        _ => {}
    }
}

/// The list check: `Algorithm::NONE` present, or both an HS variant and
/// an RS/ES/PS variant present.
fn check_list(source: &str, site: &Expr, list: &Expr, findings: &mut Vec<JwtAlgFinding>) {
    let ts = quote::ToTokens::to_token_stream(list).to_string();
    let none_arm = ts.contains("Algorithm :: NONE") || ts.contains("Algorithm::NONE");
    let has_hs = family_variant(&ts, "HS");
    let has_asym = ["RS", "ES", "PS"].iter().any(|f| family_variant(&ts, f));
    let gate = if none_arm {
        Some("Algorithm::NONE accepted")
    } else if has_hs && has_asym {
        Some("mixed HS + asymmetric allowlist")
    } else {
        None
    };
    if let Some(gate) = gate {
        let span = site.span().byte_range();
        if span.end <= source.len() {
            findings.push(JwtAlgFinding {
                site_text: source[span.clone()].to_string(),
                site_span: span,
                gate,
                advice: ADVICE,
            });
        }
    }
}

/// `Algorithm::HS256` / `Algorithm:: HS 256` — the token-stream spacing
/// form of the family check.
fn family_variant(ts: &str, family: &str) -> bool {
    let compressed = ts.replace(' ', "");
    let needle = format!("Algorithm::{family}");
    if compressed.contains(&needle) {
        return true;
    }
    // `HS256` without the Algorithm:: prefix (an imported enum variant).
    let fam = format!("{family}2");
    compressed.contains(&fam)
        || compressed.contains(&format!("{family}3"))
        || compressed.contains(&format!("{family}5"))
}

fn is_member(m: &syn::Member, name: &str) -> bool {
    matches!(m, syn::Member::Named(id) if id == name)
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::JwtAlgFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 350 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch350_mixed_family_list_fires() {
        let src = "use jsonwebtoken::{Algorithm, DecodingKey, Validation};\n\n\
             pub fn verify(token: &str, key: &DecodingKey) -> bool {\n    \
             let mut validation = Validation::new(Algorithm::HS256);\n    \
             validation.algorithms = vec![Algorithm::HS256, Algorithm::RS256];\n    \
             decode::<String>(token, key, &validation).is_ok()\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the mixed list fires: {f:?}");
        assert_eq!(f[0].gate, "mixed HS + asymmetric allowlist");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The single-family pin (the fixed posture) stays silent.
    #[test]
    fn single_family_silent() {
        let src = "use jsonwebtoken::{Algorithm, DecodingKey, Validation};\n\n\
             pub fn verify(token: &str, key: &DecodingKey) -> bool {\n    \
             let validation = Validation::new(Algorithm::RS256);\n    \
             decode::<String>(token, key, &validation).is_ok()\n}\n";
        assert!(findings(src).is_empty());
    }

    /// The `Algorithm::NONE` arm fires (both faces: list + ctor arg).
    #[test]
    fn none_arm_fires() {
        let list = "use jsonwebtoken::{Algorithm, Validation};\n\n\
             fn v() -> Validation {\n    \
             Validation { algorithms: vec![Algorithm::NONE], ..Default::default() }\n}\n";
        assert_eq!(findings(list).len(), 1);
        assert_eq!(findings(list)[0].gate, "Algorithm::NONE accepted");
        let ctor = "use jsonwebtoken::{Algorithm, Validation};\n\n\
             fn v() -> Validation {\n    Validation::new(Algorithm::NONE)\n}\n";
        assert_eq!(findings(ctor).len(), 1);
    }

    /// Without jwt vocabulary the same shapes stay silent (the B291
    /// gate shape — another crate's `algorithms` field is not ours).
    #[test]
    fn no_jwt_vocab_silent() {
        let src = "pub struct Algos { pub algorithms: Vec<u8> }\n\n\
             fn v() -> Algos {\n    \
             Algos { algorithms: vec![1, 2] }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A struct-literal `algorithms` init fires too.
    #[test]
    fn struct_literal_algorithms_fires() {
        let src = "use jsonwebtoken::{Algorithm, Validation};\n\n\
             fn v() -> Validation {\n    \
             Validation { algorithms: vec![Algorithm::HS256, Algorithm::ES256], ..Default::default() }\n}\n";
        assert_eq!(findings(src).len(), 1);
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t() {\n        \
             let v = Validation { algorithms: vec![Algorithm::HS256, Algorithm::RS256] };\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Batch 350 gate — the corpus `fixed_code` MUST stay silent.
    #[test]
    fn corpus_fixed_fixture_is_silent() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "jwt-alg-family-confusion-allowlist")
            .expect("rule present");
        let fixed = syn::parse_file(&rule.fixed_code).expect("fixed parses");
        assert!(
            detect(&rule.fixed_code, &fixed).is_empty(),
            "the fixed fixture must stay silent"
        );
    }
}
