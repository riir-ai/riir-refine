//! `predictable-rng-for-secret-material` — SUGGEST-ONLY detector
//! (Batch 350, the 0sec-labs/0 distill; Research 388 / Plan 356).
//!
//! A predictably-seeded PRNG feeding secret material: a token, nonce,
//! salt, OTP, or key derived from `seed_from_u64(..)` (any deterministic
//! seed — a clock, a counter, a constant) or a `StepRng`-class counter
//! RNG. The output is GUESSABLE: `StdRng` is reproducible from its seed,
//! and the seeds on these sites are exactly the values an attacker also
//! knows (unix time, a row id, a loop counter) — the CWE-338 class. The
//! fix is a CSPRNG: `OsRng` / `getrandom::getrandom` (the house pattern —
//! `password-hash-argon2`'s `fixed_code` already salts via
//! `SaltString::generate(&mut OsRng)`).
//!
//! ## The source anchor (0sec-labs/0 @ `f7020eb2`)
//!
//! `packages/core/src/crypto-misuse-detector.ts` detector 5
//! (`detectPredictableRng`): the JS face is `Math.random()`, gated on
//! their security-context vocabulary — their own precision law, verbatim
//! at the pin: "A `Math.random()` driving an animation is NOT flagged."
//! The Rust face differs by API, not by law: there is no `Math.random`
//! in Rust — the predictable-PRNG shapes are `seed_from_u64(..)` /
//! `StepRng::new(..)` / `from_seed([..; N])` on a non-crypto or
//! deterministically-seeded generator, and the CONTEXT gate (their
//! `SECURITY_CONTEXT_RX` identifier list) transcribes as this rule's
//! secret-vocabulary gate, not a line regex.
//!
//! ## Why suggest-only (never a fix)
//!
//! Which CSPRNG to adopt (`OsRng` vs `getrandom` vs `rand::rngs::OsRng`
//! via a feature flag) is a dependency decision, and some secret-shaped
//! values are deliberately deterministic (test fixtures, hash-map seeds,
//! rate-limit buckets — the vocabulary gate's honest FP face). NO
//! `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! ## Strict gates (ALL must hold)
//!
//! - **Predictable-PRNG gate**: the fn calls `seed_from_u64(`,
//!   `from_seed(` (with a non-`OsRng` argument form), or constructs a
//!   `StepRng`/`SmallRng`/`Pcg`/`XorShift`-class generator — the
//!   deterministically-seeded shapes, exact name tokens.
//! - **Secret-vocabulary gate** (their `SECURITY_CONTEXT_RX` transcribed
//!   as fn-scope word segments): the fn's name or a local binding
//!   carries `token`/`nonce`/`salt`/`otp`/`secret`/`api_key`/`session_key`/`reset_code`/
//!   password as EXACT segments — the value the PRNG feeds is
//!   security-relevant. A PRNG driving jitter/sampling/an array shuffle
//!   with no secret vocabulary stays silent (their animation law).
//! - **Suppression, fn-wide (the fix's own vocabulary)**: any
//!   `OsRng`/`getrandom`/`rand_core::os` marker in the fn silences it —
//!   the CSPRNG is already in use, and a mixed fn is most plausibly
//!   seeding a NON-secret stream beside it.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped; macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Data-flow is fn-level name proximity, not a taint walk: a seed
//!   produced in one fn and the secret built in another is an honest
//!   NO (`wire_taint` is the shared walker, but its sinks are
//!   allocation/egress shapes, not PRNG construction — sink-distinct).
//! - A CONSTANT-seeded generator feeding a KNOWN-NON-SECRET stream
//!   (deterministic tests, simulation replays) with secret-vocabulary
//!   NAMED locals (a `token_bucket` rate limiter!) fires and review
//!   decides — suggest-only is the blast-radius control; the vocabulary
//!   proves a NAME CLASS, not a secret.
//! - `from_entropy` is deliberately NOT a gate token (it seeds from the
//!   OS entropy pool — the safe shape); only the deterministic-seed
//!   spellings fire.
//! - Batch 350 / Plan 356 / Research 388 (0sec-labs/0 @ `f7020eb2`,
//!   MIT OR Apache-2.0 — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected predictable-RNG-for-secret-material site.
#[derive(Debug, Clone)]
pub struct PredictableRngFinding {
    /// Byte range of the PRNG construction call.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the CSPRNG swap + the class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a deterministically-seeded PRNG \
(`seed_from_u64`/`StepRng`-class) feeding secret material produces GUESSABLE \
values — the seed (a clock, a counter, a constant) is known to the attacker, \
so every token/nonce/salt it ever emits is reproducible (CWE-338). Use a \
CSPRNG instead: `OsRng` / `getrandom::getrandom` (Batch 350, suggest-only)";

/// The deterministic-seed PRNG construction tokens (exact names).
const PRNG_SEED_TOKENS: &[&str] = &[
    "seed_from_u64",
    "seed_from_u128",
    "from_seed",
    "StepRng",
    "SmallRng",
    "Pcg64",
    "Pcg32",
    "XorShiftRng",
];

/// The secret-output vocabulary (exact word segments of the fn name or a
/// local binding — the source's `SECURITY_CONTEXT_RX` transcribed to
/// fn scope; their camelCase note is honored by segment splitting).
const SECRET_VOCAB: &[&str] = &[
    "token",
    "nonce",
    "salt",
    "otp",
    "secret",
    "apikey",
    "api_key",
    "session_key",
    "sessionkey",
    "reset_code",
    "password",
    "passwd",
    "signature",
    "hmac",
    "csrf",
    "invite",
    "captcha",
];

/// The CSPRNG suppression markers (the fix's own vocabulary), read off
/// the fn's raw token text.
const CSPRNG_MARKERS: &[&str] = &["OsRng", "getrandom", "rand_core::os"];

/// Detect predictable PRNG use feeding secret vocabulary (suggest-only;
/// no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<PredictableRngFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<PredictableRngFinding>) {
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
            // fixture habitat, not production secret derivation.
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

/// The name's word segments (`snake_case` AND `CamelCase` split —
/// `SessionKey` -> session + key), the `gateway_ctor` helper's shape.
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

fn scan_fn(
    source: &str,
    sig: &syn::Signature,
    block: &syn::Block,
    findings: &mut Vec<PredictableRngFinding>,
) {
    let block_ts = quote::ToTokens::to_token_stream(block).to_string();
    // The fn-wide CSPRNG suppression marker.
    if CSPRNG_MARKERS.iter().any(|m| block_ts.contains(m)) {
        return;
    }
    // The secret-vocabulary gate: fn name segments OR any local binding
    // in the body (a `let reset_code = ..` shape).
    let sig_segs = name_segments(&sig.ident.to_string());
    let sig_hit = sig_segs.iter().any(|s| SECRET_VOCAB.contains(&s.as_str()));
    let body_idents = collect_local_idents(block);
    let body_hit = body_idents.iter().any(|id| {
        name_segments(id)
            .iter()
            .any(|s| SECRET_VOCAB.contains(&s.as_str()))
    });
    if !sig_hit && !body_hit {
        return;
    }
    // The PRNG gate: walk for the deterministic-seed construction sites.
    let mut prng_spans: Vec<Range<usize>> = Vec::new();
    collect_prng_sites(&block.stmts, source, &mut prng_spans);
    for span in prng_spans {
        findings.push(PredictableRngFinding {
            site_text: source[span.clone()].to_string(),
            site_span: span,
            advice: ADVICE,
        });
    }
}

fn collect_in_stmts_local_idents(stmts: &[syn::Stmt], out: &mut Vec<String>) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                // `let nonce: u64 = ..` parses as Pat::Type (the type
                // annotation wraps the ident pat) — unwrap to the inner
                // pat before the Ident check.
                let pat: &syn::Pat = match &l.pat {
                    syn::Pat::Type(pt) => &pt.pat,
                    other => other,
                };
                if let syn::Pat::Ident(pi) = pat {
                    out.push(pi.ident.to_string());
                }
            }
            syn::Stmt::Item(Item::Fn(f)) => {
                collect_in_stmts_local_idents(&f.block.stmts, out);
            }
            syn::Stmt::Expr(e, _) => collect_expr_idents(e, out),
            _ => {}
        }
    }
}

fn collect_expr_idents(e: &Expr, out: &mut Vec<String>) {
    use syn::visit::Visit;
    struct V(Vec<String>);
    impl<'ast> Visit<'ast> for V {
        fn visit_ident(&mut self, i: &'ast syn::Ident) {
            self.0.push(i.to_string());
        }
    }
    let mut v = V(Vec::new());
    v.visit_expr(e);
    out.append(&mut v.0);
}

fn collect_local_idents(block: &syn::Block) -> Vec<String> {
    let mut out = Vec::new();
    collect_in_stmts_local_idents(&block.stmts, &mut out);
    out
}

fn collect_prng_sites(stmts: &[syn::Stmt], source: &str, out: &mut Vec<Range<usize>>) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_prng_expr(&init.expr, source, out);
                }
            }
            syn::Stmt::Expr(e, _) => scan_prng_expr(e, source, out),
            syn::Stmt::Item(Item::Fn(f)) => {
                collect_prng_sites(&f.block.stmts, source, out);
            }
            _ => {}
        }
    }
}

fn scan_prng_expr(e: &Expr, source: &str, out: &mut Vec<Range<usize>>) {
    match e {
        Expr::Call(c) => {
            if let Expr::Path(p) = c.func.as_ref() {
                let segs: Vec<String> = p
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                let last = segs.last().map_or("", String::as_str);
                let prev = segs.len().checked_sub(2).and_then(|i| segs.get(i));
                let hit = PRNG_SEED_TOKENS.contains(&last)
                    || prev.is_some_and(|p| PRNG_SEED_TOKENS.contains(&p.as_str()));
                if hit {
                    let span = e.span().byte_range();
                    if span.end <= source.len() {
                        out.push(span);
                    }
                }
            }
            if let Expr::MethodCall(m) = c.func.as_ref() {
                scan_prng_expr(&Expr::MethodCall(m.clone()), source, out);
            }
            for a in &c.args {
                scan_prng_expr(a, source, out);
            }
        }
        Expr::MethodCall(m) => {
            let name = m.method.to_string();
            if PRNG_SEED_TOKENS.contains(&name.as_str()) {
                let span = e.span().byte_range();
                if span.end <= source.len() {
                    out.push(span);
                }
            }
            scan_prng_expr(&m.receiver, source, out);
            for a in &m.args {
                scan_prng_expr(a, source, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::PredictableRngFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 350 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch350_seeded_token_fires() {
        let src = "use rand::rngs::StdRng;\nuse rand::SeedableRng;\n\n\
             pub fn issue_token(user_id: u64) -> String {\n    \
             let mut rng = StdRng::seed_from_u64(user_id);\n    \
             let raw: [u8; 16] = rand::Rng::gen(&mut rng);\n    \
             hex_encode(raw)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the seeded PRNG fires: {f:?}");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The fixed posture — OsRng — suppresses fn-wide.
    #[test]
    fn osrng_marker_suppresses() {
        let src = "pub fn issue_token() -> String {\n    \
             let raw = getrandom_bytes(16);\n    \
             hex_encode(raw)\n}\n";
        assert!(findings(src).is_empty(), "the CSPRNG marker suppresses");
    }

    /// No secret vocabulary anywhere: the same PRNG over a shuffle/sim
    /// stays silent (their animation law, transcribed).
    #[test]
    fn no_secret_vocab_silent() {
        let src = "use rand::rngs::StdRng;\nuse rand::SeedableRng;\n\n\
             pub fn shuffle_deck(seed: u64) -> Vec<u8> {\n    \
             let mut rng = StdRng::seed_from_u64(seed);\n    \
             let mut cards: Vec<u8> = (0..52).collect();\n    \
             rand::seq::SliceRandom::shuffle(&mut cards, &mut rng);\n    \
             cards\n}\n";
        assert!(findings(src).is_empty(), "no secret vocabulary, no fire");
    }

    /// The secret vocabulary via a LOCAL binding (not the fn name) fires.
    #[test]
    fn local_binding_vocab_fires() {
        let src = "use rand::rngs::StdRng;\nuse rand::SeedableRng;\n\n\
             pub fn mint(turn: u64) -> String {\n    \
             let mut rng = StdRng::seed_from_u64(turn);\n    \
             let nonce: u64 = rand::Rng::gen(&mut rng);\n    \
             format!(\"{nonce}\")\n}\n";
        assert_eq!(findings(src).len(), 1);
    }

    /// A StepRng counter construction in a secret-named fn fires.
    #[test]
    fn step_rng_fires() {
        let src = "use rand::rngs::mock::StepRng;\n\n\
             pub fn gen_otp(state: u64) -> String {\n    \
             let mut rng = StepRng::new(state, 1);\n    \
             format!(\"{:06}\", rand::Rng::gen_range(&mut rng, 0..1_000_000))\n}\n";
        assert_eq!(findings(src).len(), 1);
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t() -> String {\n        \
             let mut rng = StdRng::seed_from_u64(42);\n        \
             format!(\"{}\", rand::Rng::gen::<u64>(&mut rng))\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Batch 350 gate — the corpus `fixed_code` MUST stay silent.
    #[test]
    fn corpus_fixed_fixture_is_silent() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "predictable-rng-for-secret-material")
            .expect("rule present");
        let fixed = syn::parse_file(&rule.fixed_code).expect("fixed parses");
        assert!(
            detect(&rule.fixed_code, &fixed).is_empty(),
            "the fixed fixture must stay silent"
        );
    }
}
