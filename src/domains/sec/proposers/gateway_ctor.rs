//! `gateway-type-forces-security-check` — SUGGEST-ONLY detector
//! (Batch 327, the firezone/sanstun distill; Research 364 / Plan 332).
//!
//! A security-relevant message/handler type whose CONSTRUCTOR is PUBLIC
//! lets any caller synthesize inputs and walk around the gateway's
//! checkpoint — the rate limit, MAC verification or replay check exists,
//! but it is skippable because the dangerous handler stays reachable
//! without it. The fix is crate-private constructors + the gateway type
//! as the ONLY producer, so the handler is UNREACHABLE without passing
//! the checkpoint.
//!
//! ## The source anchor (firezone/sanstun @ `35cb1932948bd0776`)
//!
//! message.rs keeps every message-type constructor crate-private:
//! `pub(crate) fn new<const N: usize>` on `Datagram`, and `pub(crate)`
//! fields on `HandshakeResponse`/`CookieReply`/`TransportData` with no
//! public ctor anywhere. The doc pins the law: `Messages can only be
//! obtained through that method and cannot be cloned or constructed
//! directly.` interface.rs is the single parse dispatch:
//! `pub fn receive(&mut self, src_addr: SocketAddr, datagram: &[u8],
//! now: Instant)` runs the guard sequence — mac1 `ct_eq`, the
//! under-load window, the cookie check, `rate_limiter.allow` — before a
//! parsed message exists for any caller.
//!
//! ## Why suggest-only, and why precision is honestly poor
//!
//! The pub-vs-`pub(crate)` flip ripples to every caller — an API-design
//! decision, never mechanical. And the detection itself is
//! vocabulary-gated: WHICH handler is security-relevant cannot be
//! proven from a parse. The gate is a CONJUNCTION (message vocabulary
//! on the struct name + a pub ctor + a same-file gateway fn with check
//! vocabulary that references the struct), and still a genuinely-public
//! DTO fires and review decides — suggest-only is the blast-radius
//! control. NO `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! ## Strict gates (ALL must hold)
//!
//! - A `pub` struct whose name carries an EXACT word segment from
//!   handshake/response/reply/datagram/packet/frame/ticket/claim/
//!   envelope/message.
//! - A `pub` associated constructor on it (`new`, or `from_*`).
//! - A same-file gateway fn: name segments hit
//!   receive/verify/gateway/checkpoint/accept/authorize, it takes
//!   `&self`/`&mut self`, its body carries check tokens
//!   (rate/limit/mac/verif/auth/replay/cookie/allow), and it references
//!   the struct's type name by EXACT ident (`ReceivedMessage` never
//!   matches `Message`).
//! - Suppression: the fixed shape — the ctor already `pub(crate)`/
//!   private, or no constructor found in the file — stays silent.
//! - `#[cfg(test)]` mods are skipped.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Cross-file gateway relationships are invisible (same-file only).
//! - Struct-literal construction through pub fields is not detected —
//!   the ctor is the only reachability face.
//! - Ctor-name coverage is `new`/`from_*` — a `pub fn parse` ctor is an
//!   honest miss.
//! - Batch 327 / Plan 332 / Research 364 (firezone/sanstun @
//!   `35cb1932948bd0776`, Apache-2.0 — cited as class provenance; the
//!   fixture is an original synthesis).

use std::ops::Range;

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{File, Item, Type, Visibility};

/// One detected public constructor that bypasses a same-file gateway.
#[derive(Debug, Clone)]
pub struct GatewayCtorFinding {
    /// Byte range of the `pub fn new` (the fix site).
    pub site_span: Range<usize>,
    /// The constructor's source text.
    pub site_text: String,
    /// The message type whose checkpoint the ctor skips.
    pub struct_name: String,
    /// The advice line (crate-private + the single-producer law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a PUBLIC constructor on a message type lets any \
caller synthesize inputs that walk around the gateway's checkpoint (rate \
limit / MAC verify / replay check) — the handler stays reachable without \
the check. Make the ctor crate-private and let the gateway be the only \
producer, so the handler is unreachable without passing it (Batch 327, \
suggest-only)";

/// The message vocabulary (exact word segments of the struct name).
const MESSAGE_VOCAB: &[&str] = &[
    "handshake",
    "response",
    "reply",
    "datagram",
    "packet",
    "frame",
    "ticket",
    "claim",
    "envelope",
    "message",
];

/// The gateway fn vocabulary (exact word segments of the fn name).
const GATEWAY_VOCAB: &[&str] = &[
    "receive",
    "verify",
    "gateway",
    "checkpoint",
    "accept",
    "authorize",
];

/// The check tokens the gateway body must carry (substring scan,
/// lowercase).
const CHECK_TOKENS: &[&str] = &[
    "rate", "limit", "mac", "verif", "auth", "replay", "cookie", "allow",
];

/// Detect message-type pub constructors beside a same-file gateway
/// (suggest-only; no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<GatewayCtorFinding> {
    // Message-vocabulary pub structs + their pub ctor sites (if any).
    let mut ctors: Vec<(String, Range<usize>)> = Vec::new();
    collect_pub_ctors(&file.items, &mut ctors);
    if ctors.is_empty() {
        return Vec::new();
    }
    // Same-file gateway fns: the type idents each one references.
    let mut gateways: Vec<Vec<String>> = Vec::new();
    collect_gateways(&file.items, &mut gateways);
    let mut findings = Vec::new();
    for (name, span) in ctors {
        let referenced = gateways
            .iter()
            .any(|idents| idents.iter().any(|i| i == &name));
        if !referenced {
            continue;
        }
        if span.end > source.len() {
            continue;
        }
        findings.push(GatewayCtorFinding {
            site_text: source[span.clone()].to_string(),
            struct_name: name,
            site_span: span,
            advice: ADVICE,
        });
    }
    findings
}

/// Message-vocabulary pub structs that carry a PUBLIC associated ctor in
/// this item list — `(struct name, ctor span)`. The fixed shape (a
/// `pub(crate)`/private ctor, or no ctor in the file) never lands here.
fn collect_pub_ctors(items: &[Item], out: &mut Vec<(String, Range<usize>)>) {
    for item in items {
        match item {
            Item::Struct(s) => {
                if !matches!(s.vis, Visibility::Public(_)) {
                    continue;
                }
                let segs = name_segments(&s.ident.to_string());
                if !segs.iter().any(|sg| MESSAGE_VOCAB.contains(&sg.as_str())) {
                    continue;
                }
                if let Some(span) = pub_ctor_span(&s.ident.to_string(), items) {
                    out.push((s.ident.to_string(), span));
                }
            }
            Item::Mod(m) => {
                // Comment guard (house discipline): #[cfg(test)] mods
                // are fixture habitat.
                if is_test_mod(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    collect_pub_ctors(nested, out);
                }
            }
            _ => {}
        }
    }
}

/// The span of a PUBLIC `new`/`from_*` associated fn on the type, if the
/// impl block lives in this item list.
fn pub_ctor_span(struct_name: &str, items: &[Item]) -> Option<Range<usize>> {
    for item in items {
        if let Item::Impl(imp) = item {
            let self_name = last_type_ident(&imp.self_ty);
            if self_name.as_deref() != Some(struct_name) {
                continue;
            }
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    if !matches!(m.vis, Visibility::Public(_)) {
                        continue;
                    }
                    let name = m.sig.ident.to_string();
                    if name == "new" || name.starts_with("from_") {
                        let span = m.span().byte_range();
                        if span.start < span.end {
                            return Some(span);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Same-file gateway fns: the type idents each one references.
fn collect_gateways(items: &[Item], out: &mut Vec<Vec<String>>) {
    for item in items {
        match item {
            Item::Impl(imp) => {
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(m) = ii {
                        if is_gateway_method(m) {
                            let mut idents = Vec::new();
                            collect_idents(m.to_token_stream(), &mut idents);
                            out.push(idents);
                        }
                    }
                }
            }
            Item::Mod(m) => {
                if is_test_mod(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    collect_gateways(nested, out);
                }
            }
            _ => {}
        }
    }
}

/// The gateway face: vocabulary name + `&self`/`&mut self` receiver +
/// check tokens in the body. A free fn takes no `self` and never
/// qualifies.
fn is_gateway_method(m: &syn::ImplItemFn) -> bool {
    let segs = name_segments(&m.sig.ident.to_string());
    if !segs.iter().any(|sg| GATEWAY_VOCAB.contains(&sg.as_str())) {
        return false;
    }
    let takes_self_ref = m
        .sig
        .inputs
        .iter()
        .any(|arg| matches!(arg, syn::FnArg::Receiver(r) if r.reference.is_some()));
    if !takes_self_ref {
        return false;
    }
    let body = m.block.to_token_stream().to_string().to_ascii_lowercase();
    CHECK_TOKENS.iter().any(|tok| body.contains(tok))
}

/// Every ident in the token stream (exact words — `ReceivedMessage`
/// never matches `Message`).
fn collect_idents(ts: TokenStream, out: &mut Vec<String>) {
    for tt in ts {
        match tt {
            TokenTree::Ident(i) => out.push(i.to_string()),
            TokenTree::Group(g) => collect_idents(g.stream(), out),
            _ => {}
        }
    }
}

fn last_type_ident(ty: &Type) -> Option<String> {
    if let Type::Path(tp) = ty {
        if tp.qself.is_none() {
            return tp.path.segments.last().map(|s| s.ident.to_string());
        }
    }
    None
}

fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

/// The name's word segments: `snake_case` splits on `_`, `CamelCase`
/// splits at case boundaries (`LoginClaim` -> login + claim). The
/// vocabulary gates match EXACT segments, never substrings.
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

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::GatewayCtorFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    const TICKET_GATE: &str = "\
pub struct LoginClaim {
    user: u64,
}

impl LoginClaim {
    pub fn new(user: u64) -> Self {
        Self { user }
    }
}

pub struct Gate {
    rate_budget: u32,
}

impl Gate {
    pub fn authorize(&mut self, claim: &LoginClaim) -> bool {
        if self.rate_budget == 0 {
            return false;
        }
        self.rate_budget -= 1;
        true
    }
}
";

    #[test]
    fn pub_ctor_beside_gateway_fires() {
        let f = findings(TICKET_GATE);
        assert_eq!(f.len(), 1, "the pub ctor beside the checkpoint fires");
        assert_eq!(f[0].struct_name, "LoginClaim");
        assert!(f[0].site_text.contains("pub fn new"));
    }

    #[test]
    fn fixed_shape_pub_crate_ctor_is_silent() {
        let src = TICKET_GATE.replace("pub fn new", "pub(crate) fn new");
        assert!(findings(&src).is_empty(), "the fixed shape stays silent");
    }

    #[test]
    fn no_gateway_fn_is_silent() {
        let src = "\
pub struct LoginClaim {
    user: u64,
}

impl LoginClaim {
    pub fn new(user: u64) -> Self {
        Self { user }
    }
}
";
        assert!(findings(src).is_empty(), "no gateway fn, no fire");
    }

    #[test]
    fn gateway_without_check_vocab_is_silent() {
        // the fn references the type but carries no checkpoint vocabulary
        let src = "\
pub struct LoginClaim {
    user: u64,
}

impl LoginClaim {
    pub fn new(user: u64) -> Self {
        Self { user }
    }
}

pub struct Gate {
    hops: u32,
}

impl Gate {
    pub fn authorize(&mut self, claim: &LoginClaim) -> u32 {
        self.hops += 1;
        self.hops
    }
}
";
        assert!(findings(src).is_empty(), "no check tokens in the body");
    }

    #[test]
    fn non_message_struct_is_silent() {
        // a Config/Cache pub ctor + a verify fn — the struct name misses
        // the message vocabulary.
        let src = "\
pub struct CacheConfig {
    capacity: usize,
}

impl CacheConfig {
    pub fn new(capacity: usize) -> Self {
        Self { capacity }
    }
}

pub struct Store {
    seen: Vec<String>,
}

impl Store {
    pub fn verify(&mut self, cfg: &CacheConfig) -> bool {
        !self.seen.contains(&cfg.capacity.to_string())
    }
}
";
        assert!(
            findings(src).is_empty(),
            "the struct name misses the vocabulary"
        );
    }

    #[test]
    fn free_fn_gateway_never_fires() {
        // a FREE fn takes no &self — the gateway face is a method.
        let src = "\
pub struct LoginClaim {
    user: u64,
}

impl LoginClaim {
    pub fn new(user: u64) -> Self {
        Self { user }
    }
}

pub fn authorize(claim: &LoginClaim, budget: &mut u32) -> bool {
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    true
}
";
        assert!(findings(src).is_empty(), "a free fn is not a gateway");
    }

    #[test]
    fn exact_ident_reference_fires() {
        // `Message::new` is an EXACT ident mention — the sanstun enum
        // spelling (`ReceivedMessage`) beside it cannot block the fire.
        let src = "\
pub struct Message {
    body: [u8; 4],
}

impl Message {
    pub fn new(body: [u8; 4]) -> Self {
        Self { body }
    }
}

pub struct Gateway {
    budget: u32,
}

impl Gateway {
    pub fn receive(&mut self, raw: &[u8]) -> Option<ReceivedMessage> {
        if !self.rate_allow(raw) {
            return None;
        }
        Some(ReceivedMessage::Message(Message::new([0; 4])))
    }
}
";
        assert_eq!(findings(src).len(), 1, "the exact `Message` ident fires");
    }

    #[test]
    fn wrapper_type_mention_alone_is_silent() {
        // the gateway only names the WRAPPER enum — never the bare type.
        let src = "\
pub struct Message {
    body: [u8; 4],
}

impl Message {
    pub fn new(body: [u8; 4]) -> Self {
        Self { body }
    }
}

pub struct Gateway {
    budget: u32,
}

impl Gateway {
    pub fn receive(&mut self, raw: &[u8]) -> Option<ReceivedMessage> {
        if !self.rate_allow(raw) {
            return None;
        }
        Some(ReceivedMessage::Done)
    }
}
";
        assert!(
            findings(src).is_empty(),
            "the wrapper enum is not the struct"
        );
    }

    #[test]
    fn test_mod_is_skipped() {
        let src = format!("#[cfg(test)]\nmod tests {{\n{TICKET_GATE}}}\n");
        assert!(findings(&src).is_empty(), "fixture habitat stays silent");
    }

    #[test]
    fn corpus_buggy_fixture_fires() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "gateway-type-forces-security-check")
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
