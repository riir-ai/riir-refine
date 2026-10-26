//! `jwt-insecure-validation-flags` — SUGGEST-ONLY detector
//! (Batch 291, the danoszz/penthera distill; Research 329 / Plan 295).
//!
//! A JWT decode configured to NOT verify. Three live surfaces, spanning
//! the crate's version history (the CHANGELOG is the record):
//!
//! - `Validation::insecure_disable_signature_validation()` — the 9.x-era
//!   method (the field form too); REMOVED at jsonwebtoken 10.0. Accepts
//!   forged unsigned tokens: the alg:none bypass class, exactly the
//!   runtime face the penthera jwt probe measures (a forged `alg: "none"`
//!   token accepted on an authed route).
//! - `dangerous::insecure_decode(..)` — the 10.x replacement, the same
//!   class one namespace over (the name carries the confession).
//! - `validate_exp = false` — the expiry-skip half; accepts expired
//!   tokens (penthera's expired-JWT finding is the runtime face).
//!
//! ## Why suggest-only (never a fix)
//!
//! Some debug/tooling decodes deliberately skip validation; a mechanical
//! rewrite cannot know which. This module ships a parse-only DETECTION
//! walk only: NO `FIXABLE_SEC` membership, NO `collect_edits` arm — the
//! suggestion surfaces through retrieval (`--suggest` / the corpus entry)
//! and review.
//!
//! ## The bounded form
//!
//! ```text
//! let validation = Validation::new(Algorithm::HS256); // sig + exp ON
//! let data = decode::<Claims>(token, key, &validation)?;
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Signature surfaces fire on their unique names** (any receiver):
//!   the method/field token `insecure_disable_signature_validation`, and
//!   a `dangerous::insecure_decode` call path.
//! - **`validate_exp = false` is jwt-vocabulary-gated**: fires only when
//!   the enclosing fn carries a `jsonwebtoken`-prefixed path segment or a
//!   `Validation` / `DecodingKey` / `Algorithm` type token — other
//!   crates' fields stay silent. Explicit `validate_exp = true` is
//!   silent.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Statement/field-init shapes only: a validation flag flipped inside a
//!   helper the caller never sees is an honest NO (cross-fn flag flows
//!   are unconnected parse-only).
//! - `Validation` struct-literal re-creations that merely shadow a
//!   hardened one (two validations in one fn) fire per-site and review
//!   decides — the rule flags the configuration, not intent.
//! - Batch 291 / Plan 295 / Research 329 (danoszz/penthera @ `07421f9`,
//!   MIT — cited as class provenance; the fixture is an original
//!   synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item, Stmt};

/// One detected insecure-JWT-validation site.
#[derive(Debug, Clone)]
pub struct JwtInsecureFinding {
    /// Byte range of the offending statement/expression.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// Which insecure surface fired.
    pub surface: &'static str,
    /// The advice line (the default posture + the class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "decode with verification ON: pin the algorithm list \
(`Validation::new(Algorithm::..)`), never call \
`insecure_disable_signature_validation()` / `dangerous::insecure_decode` \
(the alg:none bypass class), and leave `validate_exp` enabled (expired \
tokens are not credentials; Batch 291, suggest-only)";

/// The fn-level jwt-vocabulary tokens gating the `validate_exp` surface.
const JWT_TYPE_TOKENS: &[&str] = &["Validation", "DecodingKey", "Algorithm"];

/// Detect insecure JWT validation configurations (suggest-only; no edits
/// are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<JwtInsecureFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<JwtInsecureFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.sig, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.sig, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production decode paths.
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

fn scan_fn(
    source: &str,
    sig: &syn::Signature,
    block: &syn::Block,
    findings: &mut Vec<JwtInsecureFinding>,
) {
    // fn-level jwt vocabulary (token text, space-stripped): gates the
    // `validate_exp` surface only — the signature surfaces are unique
    // enough to fire bare.
    let sig_ts = quote::ToTokens::to_token_stream(sig).to_string();
    let block_ts = quote::ToTokens::to_token_stream(block).to_string();
    let fn_text = format!("{sig_ts}{block_ts}").replace(' ', "");
    let jwt_vocab =
        fn_text.contains("jsonwebtoken::") || JWT_TYPE_TOKENS.iter().any(|t| fn_text.contains(t));

    scan_stmts(source, &block.stmts, jwt_vocab, findings);
}

fn scan_stmts(
    source: &str,
    stmts: &[Stmt],
    jwt_vocab: bool,
    findings: &mut Vec<JwtInsecureFinding>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr(source, &init.expr, jwt_vocab, findings);
                }
            }
            Stmt::Item(item) => collect_in_item(source, item, findings),
            Stmt::Expr(e, _) => scan_expr(source, e, jwt_vocab, findings),
            Stmt::Macro(_) => {}
        }
    }
}

fn scan_expr(source: &str, e: &Expr, jwt_vocab: bool, findings: &mut Vec<JwtInsecureFinding>) {
    match e {
        // `v.insecure_disable_signature_validation()` — the 9.x method form.
        Expr::MethodCall(m) => {
            if m.method == "insecure_disable_signature_validation" {
                push(source, e, "signature_validation_disabled", findings);
            }
            scan_expr(source, &m.receiver, jwt_vocab, findings);
            for a in &m.args {
                scan_expr(source, a, jwt_vocab, findings);
            }
        }
        // `dangerous::insecure_decode(..)` — the 10.x replacement.
        Expr::Call(c) => {
            if let Expr::Path(p) = c.func.as_ref() {
                let segs: Vec<String> = p
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                if segs.len() >= 2
                    && segs[segs.len() - 2] == "dangerous"
                    && segs[segs.len() - 1] == "insecure_decode"
                {
                    push(source, e, "insecure_decode", findings);
                }
            }
            scan_expr(source, &c.func, jwt_vocab, findings);
            for a in &c.args {
                scan_expr(source, a, jwt_vocab, findings);
            }
        }
        // `v.validate_exp = false` — the expiry skip (jwt-vocabulary-gated).
        Expr::Assign(a) => {
            if let Expr::Field(f) = a.left.as_ref() {
                if is_member(&f.member, "validate_exp") && is_false_expr(&a.right) && jwt_vocab {
                    push(source, e, "expiry_validation_disabled", findings);
                }
            }
            scan_expr(source, &a.left, jwt_vocab, findings);
            scan_expr(source, &a.right, jwt_vocab, findings);
        }
        // Struct-literal field forms: `Validation { validate_exp: false, .. }`
        // and `Validation { insecure_disable_signature_validation: true, .. }`.
        Expr::Struct(s) => {
            for field in &s.fields {
                if is_member(&field.member, "validate_exp")
                    && is_false_expr(&field.expr)
                    && jwt_vocab
                {
                    push(source, e, "expiry_validation_disabled", findings);
                    break;
                }
                if is_member(&field.member, "insecure_disable_signature_validation")
                    && is_true_expr(&field.expr)
                {
                    push(source, e, "signature_validation_disabled", findings);
                    break;
                }
            }
            for field in &s.fields {
                scan_expr(source, &field.expr, jwt_vocab, findings);
            }
        }
        Expr::If(i) => {
            scan_expr(source, &i.cond, jwt_vocab, findings);
            scan_stmts(source, &i.then_branch.stmts, jwt_vocab, findings);
            if let Some((_, else_e)) = &i.else_branch {
                scan_expr(source, else_e, jwt_vocab, findings);
            }
        }
        Expr::Match(m) => {
            scan_expr(source, &m.expr, jwt_vocab, findings);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    scan_expr(source, guard, jwt_vocab, findings);
                }
                scan_expr(source, &arm.body, jwt_vocab, findings);
            }
        }
        Expr::Block(b) => scan_stmts(source, &b.block.stmts, jwt_vocab, findings),
        Expr::Unsafe(u) => scan_stmts(source, &u.block.stmts, jwt_vocab, findings),
        Expr::Loop(l) => scan_stmts(source, &l.body.stmts, jwt_vocab, findings),
        Expr::While(w) => {
            scan_expr(source, &w.cond, jwt_vocab, findings);
            scan_stmts(source, &w.body.stmts, jwt_vocab, findings);
        }
        Expr::ForLoop(fl) => {
            scan_expr(source, &fl.expr, jwt_vocab, findings);
            scan_stmts(source, &fl.body.stmts, jwt_vocab, findings);
        }
        _ => {}
    }
}

/// `syn::Member` is `Named(Ident) | Index(idx)` — compare the named form.
fn is_member(m: &syn::Member, name: &str) -> bool {
    matches!(m, syn::Member::Named(id) if id == name)
}

fn is_false_expr(e: &Expr) -> bool {
    let Expr::Lit(l) = e else {
        return false;
    };
    match &l.lit {
        syn::Lit::Bool(b) => !b.value(),
        _ => false,
    }
}

fn is_true_expr(e: &Expr) -> bool {
    let Expr::Lit(l) = e else {
        return false;
    };
    match &l.lit {
        syn::Lit::Bool(b) => b.value(),
        _ => false,
    }
}

fn push(source: &str, e: &Expr, surface: &'static str, findings: &mut Vec<JwtInsecureFinding>) {
    let span = e.span().byte_range();
    if span.end > source.len() {
        return; // the parse came from another file (the egress guard)
    }
    findings.push(JwtInsecureFinding {
        site_text: source[span.clone()].to_string(),
        site_span: span,
        surface,
        advice: ADVICE,
    });
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::JwtInsecureFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 291 gate — the corpus `buggy_code` MUST fire (both surfaces).
    #[test]
    fn batch291_insecure_method_plus_exp_skip_fires() {
        let src = "use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};\n\n\
             pub fn read_claims(token: &str, key: &DecodingKey) -> Option<String> {\n    \
             let mut validation = Validation::new(Algorithm::HS256);\n    \
             validation.validate_exp = false;\n    \
             validation.insecure_disable_signature_validation();\n    \
             let _ = decode::<String>(token, key, &validation);\n    \
             None\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 2, "both surfaces fire: {f:?}");
        assert!(f
            .iter()
            .any(|x| x.surface == "signature_validation_disabled"));
        assert!(f.iter().any(|x| x.surface == "expiry_validation_disabled"));
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The 10.x `dangerous::insecure_decode` replacement fires.
    #[test]
    fn insecure_decode_path_fires() {
        let src = "use jsonwebtoken::dangerous;\n\n\
             pub fn peek(token: &str) -> Option<String> {\n    \
             dangerous::insecure_decode::<String>(token).ok().map(|d| d.claims)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].surface, "insecure_decode");
    }

    /// The struct-literal field form fires (9.x-era field spellings).
    #[test]
    fn struct_literal_field_form_fires() {
        let src = "fn harden(mut v: Validation) -> Validation {\n    \
             let v = Validation { insecure_disable_signature_validation: true, ..v };\n    \
             v\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].surface, "signature_validation_disabled");
    }

    /// `validate_exp = false` WITHOUT jwt vocabulary stays silent.
    #[test]
    fn exp_skip_outside_jwt_vocabulary_silent() {
        let src = "pub struct Cache { pub validate_exp: bool }\n\n\
             pub fn refresh(c: &mut Cache) {\n    \
             c.validate_exp = false;\n}\n";
        assert!(findings(src).is_empty(), "no jwt vocabulary: silent");
    }

    /// Explicit `validate_exp = true` is silent.
    #[test]
    fn exp_enabled_silent() {
        let src = "fn tighten(v: &mut Validation) {\n    v.validate_exp = true;\n}\n";
        assert!(findings(src).is_empty());
    }

    /// The hardened (corpus fixed) form stays silent.
    #[test]
    fn hardened_decode_silent() {
        let src = "fn read(token: &str, key: &DecodingKey) -> Option<String> {\n    \
             let validation = Validation::new(Algorithm::HS256);\n    \
             decode::<String>(token, key, &validation).ok().map(|d| d.claims)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t() {\n        \
             let mut v = Validation::new(Algorithm::HS256);\n        \
             v.insecure_disable_signature_validation();\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }
}
