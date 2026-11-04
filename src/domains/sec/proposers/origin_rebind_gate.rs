//! `browser-reachable-service-host-origin-rebinding-gate` — SUGGEST-ONLY
//! detector (Batch 329, the tembo/voxl distill; Plan 337 / Research 367).
//!
//! A browser-reachable local service (a dev tool, dashboard, debugger,
//! agent API — anything bound to localhost/0.0.0.0 with a browser-facing
//! API or WS endpoint) answering requests whose `Host` it never checks:
//! ANY web page can open a WebSocket to it or send it requests, and a
//! page whose own domain is DNS-rebound to the service's address even
//! counts as the service's own page — a same-origin-looking `Host`. WS
//! handshakes are NOT CORS-gated (no preflight exists, so response-
//! header CORS policy is irrelevant): the server itself must gate. The
//! bounded repair (the source's gate, verbatim structure):
//!
//! ```ignore
//! // Host gate: IP literal || the localhost family || the allowlist
//! let ok = host.parse::<IpAddr>().is_ok()
//!     || host == "localhost" || host.ends_with(".localhost")
//!     || ALLOWED_HOSTS.contains(&host); // ".example.com" covers
//!                                       // subdomains, "*" any
//! if !ok {
//!     return forbidden("--allowed-hosts: host cannot be answered");
//! }
//! // Origin gate: the canonical key == the requested host:port
//! if let Some(origin) = req.origin {
//!     if origin_key(origin) != Some(&canonical) && !allowed(origin) {
//!         return forbidden("--allowed-origins: cross-origin request");
//!     }
//! }
//! ```
//!
//! the 403 refusal naming the fix flag. A request with NO `Origin` is
//! allowed — fail-open BY DESIGN, the reason in-source, verbatim:
//! "Browsers always send a Host, so one without it comes from another
//! kind of client." — the regime boundary: a non-browser client is
//! allowed; the gate is anti-browser-drive-by, not auth.
//!
//! ## Why suggest-only (never a fix)
//!
//! The allowlist is deployment policy (which hosts/origins are
//! legitimate is the operator's answer) and the missing-Origin fail-open
//! stance is the same kind of decision — a mechanical rewrite would have
//! to invent both, plus the refusal shape (403 body, error type, the fix
//! flag's name). This module ships a parse-only DETECTION walk only: NO
//! `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Handler shape per fn**: the fn takes a request-ish param (an
//!   ident or a type name carrying the `req`/`request` word segments,
//!   or a param named into the host/origin vocabulary) AND is
//!   response-ish — a return type spelling String/Response/Html/Json
//!   (`-> String`, `-> Response`, `impl IntoResponse`,
//!   `Option<String>`, `Result<String, _>` all carry one) or a
//!   `format!(..)` body.
//! - **Request-header surface per fn**: the fn names BOTH words —
//!   `host` AND `origin` as binding/field/param vocabulary
//!   (case-insensitive word segments) — OR takes a param whose type is
//!   a same-file struct whose fields name the pair (the
//!   `RequestContext { host, origin }` shape: the request type IS the
//!   header surface).
//! - **Gate-vocabulary markers suppress the WHOLE fn** (one gated
//!   member hides a sibling; conservative): `host_allowed` /
//!   `allowed_hosts` / `allowed_origins` / `origin_key` / `IpAddr` /
//!   `refuse` / `rebind` / `allowlist` / `is_loopback` anywhere in the
//!   fn (ident-substring, lowercase).
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped (fixture handlers live there by design); no edits are
//!   produced, so interior comments cannot be displaced.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Same-fn only: a gate assembled across fns (a Host check in a
//!   helper, a middleware layer wrapping the handler) is undetected —
//!   the ungated-looking handler still fires; review decides.
//! - The shape proves a MISSING GATE, not an attacker — an
//!   internal-only tool (never browser-reachable) fires too; the regime
//!   IS browser-reachability and is unknowable parse-only.
//!   Suggest-only is the blast-radius control.
//! - A handler that never reads Host/Origin vocabulary (no field
//!   access, no request type carrying the pair) is an honest NO —
//!   parse-only cannot prove the headers exist off-vocabulary.
//! - The two repair faces ride the ADVICE text (canonical default-port
//!   keys compared BEFORE the string comparison; non-UTF-8 header
//!   values read as empty and refuse fail-closed) — they are not
//!   separately detected.
//! - Batch 329 / Plan 337 / Research 367 (tembo/voxl @ `eba3b7ce`, MIT
//!   — cited as class provenance; the fixture is an original
//!   synthesis).

use std::collections::HashSet;
use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{File, Item};

/// The request-header surface words — binding/field/param vocabulary,
/// matched as case-insensitive word segments. Order is the bit order in
/// `FnScan::header_bits`.
const HEADER_WORDS: &[&str] = &["host", "origin"];

/// The request-ish param words (a handler takes a request-ish param).
const REQUEST_WORDS: &[&str] = &["req", "request"];

/// The per-fn gate markers — ANY one suppresses the whole fn (one gated
/// member hides a sibling; conservative; ident-substring, lowercase).
const GATE_MARKERS: &[&str] = &[
    "host_allowed",
    "allowed_hosts",
    "allowed_origins",
    "origin_key",
    "ipaddr",
    "refuse",
    "rebind",
    "allowlist",
    "is_loopback",
];

/// The response-ish return-type spellings (substring over the return
/// type's token text — `-> String`, `-> Response`, `impl IntoResponse`,
/// `Option<String>`, `Result<String, _>` all carry one).
const RESPONSE_TYPE_WORDS: &[&str] = &["String", "Response", "Html", "Json"];

/// One detected ungated browser-reachable handler.
#[derive(Debug, Clone)]
pub struct OriginRebindFinding {
    /// Byte range of the ungated handler fn.
    pub call_span: Range<usize>,
    /// The fn's source text.
    pub call_text: String,
    /// The advice line (the gate shape + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "gate the browser-reachable service against \
host/origin rebinding: refuse the Host unless it parses as an IP literal, \
is the localhost family (`localhost`/`*.localhost`), or the allowlist names \
it (`--allowed-hosts`; entries like `.example.com` cover subdomains, `*` \
any); refuse the Origin unless its canonical key — default port folded, \
`https://a.example` == `https://a.example:443`: compare canonical keys, \
never raw header strings — equals the requested host:port or the \
allowlist names it (`--allowed-origins`); the 403 refusal names the fix \
flag; a missing Origin stays allowed, fail-open BY DESIGN (browsers \
always send a Host, so one without it comes from another kind of client — \
the gate is anti-browser-drive-by, not auth); a non-UTF-8 header value \
reads as empty and is refused fail-closed; WS handshakes are NOT \
CORS-gated — the server itself must gate (Batch 329, suggest-only)";

/// Detect ungated handlers on a browser-reachable service (suggest-only;
/// no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<OriginRebindFinding> {
    let surface = surface_structs(&file.items);
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &surface, &mut findings);
    }
    findings
}

/// The same-file structs whose fields name the request-header surface
/// (the `RequestContext { host, origin }` shape) — a handler taking one
/// of these types IS on the header surface even when its body reads
/// neither field.
fn surface_structs(items: &[Item]) -> HashSet<String> {
    let mut out = HashSet::new();
    collect_surface(items, &mut out);
    out
}

fn collect_surface(items: &[Item], out: &mut HashSet<String>) {
    for item in items {
        match item {
            Item::Struct(s) => {
                let fields_hit = s.fields.iter().any(|f| {
                    f.ident.as_ref().is_some_and(|i| {
                        let lower = i.to_string().to_ascii_lowercase();
                        has_word(&lower, "host") || has_word(&lower, "origin")
                    })
                });
                if fields_hit {
                    out.insert(s.ident.to_string());
                }
            }
            Item::Mod(m) => {
                if is_test_mod(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    collect_surface(nested, out);
                }
            }
            _ => {}
        }
    }
}

fn collect_in_item(
    source: &str,
    item: &Item,
    surface: &HashSet<String>,
    findings: &mut Vec<OriginRebindFinding>,
) {
    match item {
        Item::Fn(f) => scan_fn(
            source,
            f.span().byte_range(),
            &f.sig,
            &f.block,
            surface,
            findings,
        ),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(
                        source,
                        m.span().byte_range(),
                        &m.sig,
                        &m.block,
                        surface,
                        findings,
                    );
                }
            }
        }
        Item::Mod(m) => {
            if is_test_mod(m) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, surface, findings);
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

/// The word segments of one identifier — split on `_`, then on the
/// camelCase boundaries, lowercased (`RequestContext` → `request`,
/// `context`; `host_ok` → `host`, `ok`). The input is already lowercased.
fn word_segments(name_lower: &str) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in name_lower.split('_') {
        let mut cur = String::new();
        for c in chunk.chars() {
            if c.is_ascii_uppercase() && !cur.is_empty() {
                out.push(std::mem::take(&mut cur).to_ascii_lowercase());
            }
            cur.push(c);
        }
        if !cur.is_empty() {
            out.push(cur.to_ascii_lowercase());
        }
    }
    out
}

/// Whether the identifier's word segments carry the word (segments, not
/// substrings — `recipient` does not carry `req`).
fn has_word(name_lower: &str, word: &str) -> bool {
    word_segments(name_lower).iter().any(|s| s == word)
}

/// The per-fn scan state — one pass over the fn's signature inputs and
/// body collects every gate the fire decision needs. `header_bits` is
/// the seen-set of `HEADER_WORDS` (bit `idx` = word `idx` seen).
#[derive(Default)]
struct FnScan {
    header_bits: u8,
    request_ish: bool,
    gated: bool,
}

impl<'ast> Visit<'ast> for FnScan {
    fn visit_ident(&mut self, i: &'ast proc_macro2::Ident) {
        let lower = i.to_string().to_ascii_lowercase();
        if GATE_MARKERS.iter().any(|m| lower.contains(m)) {
            self.gated = true;
        }
        for (idx, w) in HEADER_WORDS.iter().enumerate() {
            if has_word(&lower, w) {
                self.header_bits |= 1u8 << idx;
            }
        }
        if REQUEST_WORDS.iter().any(|w| has_word(&lower, w)) {
            self.request_ish = true;
        }
        syn::visit::visit_ident(self, i);
    }

    fn visit_expr_field(&mut self, f: &'ast syn::ExprField) {
        if let syn::Member::Named(name) = &f.member {
            let lower = name.to_string().to_ascii_lowercase();
            for (idx, w) in HEADER_WORDS.iter().enumerate() {
                if has_word(&lower, w) {
                    self.header_bits |= 1u8 << idx;
                }
            }
        }
        syn::visit::visit_expr_field(self, f);
    }
}

/// Whether the fn body carries a `format!(..)` call (the response-ish
/// body fallback).
fn block_formats(block: &syn::Block) -> bool {
    struct F(bool);
    impl<'ast> Visit<'ast> for F {
        fn visit_expr_macro(&mut self, m: &'ast syn::ExprMacro) {
            if m.mac
                .path
                .segments
                .last()
                .is_some_and(|s| s.ident == "format")
            {
                self.0 = true;
            }
            syn::visit::visit_expr_macro(self, m);
        }
    }
    let mut f = F(false);
    f.visit_block(block);
    f.0
}

/// Whether the param's type names a same-file request-header surface
/// struct (a `RequestContext`-shaped param).
fn param_type_in_surface(arg: &syn::FnArg, surface: &HashSet<String>) -> bool {
    let syn::FnArg::Typed(pat) = arg else {
        return false;
    };
    let syn::Type::Path(tp) = pat.ty.as_ref() else {
        return false;
    };
    tp.path
        .segments
        .last()
        .is_some_and(|s| surface.contains(&s.ident.to_string()))
}

/// Whether the return type spells a response-ish value.
fn response_ish_return(sig: &syn::Signature) -> bool {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return false;
    };
    let text = ty.to_token_stream().to_string();
    RESPONSE_TYPE_WORDS.iter().any(|w| text.contains(w))
}

fn scan_fn(
    source: &str,
    span: Range<usize>,
    sig: &syn::Signature,
    block: &syn::Block,
    surface: &HashSet<String>,
    findings: &mut Vec<OriginRebindFinding>,
) {
    let mut probe = FnScan::default();
    for arg in &sig.inputs {
        probe.visit_fn_arg(arg);
    }
    probe.visit_block(block);
    if probe.gated {
        return;
    }
    // The request-header surface: both words named in the fn, OR a
    // param whose type is a same-file struct carrying the pair.
    let both_words = HEADER_WORDS
        .iter()
        .enumerate()
        .all(|(idx, _)| probe.header_bits & (1u8 << idx) != 0);
    let typed_surface = sig
        .inputs
        .iter()
        .any(|arg| param_type_in_surface(arg, surface));
    if !both_words && !typed_surface {
        return;
    }
    if !probe.request_ish {
        return;
    }
    if !(response_ish_return(sig) || block_formats(block)) {
        return;
    }
    if span.start < span.end && span.end <= source.len() {
        findings.push(OriginRebindFinding {
            call_text: source[span.clone()].to_string(),
            call_span: span,
            advice: ADVICE,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::OriginRebindFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 329 gate — the corpus shape (the request type IS the
    /// header surface; the handler reads neither field) MUST fire at
    /// the handler span.
    #[test]
    fn batch329_surface_struct_handler_fires() {
        let src = "use std::net::IpAddr;\n\npub struct RequestContext<'a> {\n    \
pub host: Option<&'a str>,\n    pub origin: Option<&'a str>,\n    pub path: &'a str,\n}\n\n\
pub fn handle(req: RequestContext<'_>, body: &str) -> String {\n    \
format!(\"{}: {}\", req.path, body)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].advice, super::ADVICE);
        assert!(
            f[0].call_text.contains("fn handle"),
            "fires at the handler span: {:?}",
            f[0].call_text
        );
        assert_eq!(
            &src[f[0].call_span.clone()][..12],
            "pub fn handl",
            "the span resolves to the handler fn"
        );
    }

    /// The direct vocabulary path — the handler READS both header
    /// fields and no same-file struct is needed.
    #[test]
    fn batch329_direct_field_read_fires() {
        let src = "pub fn render(req: WeatherReq, city: &str) -> String {\n    \
let _ = (req.host, req.origin);\n    format!(\"<h1>{city}</h1>\")\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the direct header-vocabulary path fires: {f:?}");
        assert!(f[0].call_text.contains("fn render"));
    }

    /// An impl-method handler fires too (the `-> String` + request
    /// param shape inside an impl block).
    #[test]
    fn batch329_impl_method_handler_fires() {
        let src = "pub struct Ctx {\n    pub host: Option<String>,\n    \
pub origin: Option<String>,\n}\n\npub struct Svc;\n\nimpl Svc {\n    \
pub fn handle(&self, req: Ctx) -> String {\n        format!(\"served\")\n    }\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the impl-method handler fires: {f:?}");
        assert!(f[0].call_text.contains("fn handle"));
    }

    // ── NEGATIVE SUITE — every class must stay SILENT (zero-FP bar) ──

    /// A handler without host/origin vocabulary is an honest NO.
    #[test]
    fn batch329_no_header_vocabulary_is_silent() {
        let src = "pub fn greet(req: HttpRequest, name: &str) -> String {\n    \
format!(\"hi {name}\")\n}\n";
        assert!(findings(src).is_empty(), "no header surface -> silent");
    }

    /// One word without the other is below the both-words gate.
    #[test]
    fn batch329_single_word_vocabulary_is_silent() {
        let src = "pub fn label(host: Option<&str>) -> String {\n    \
format!(\"{}\", host.unwrap_or(\"?\"))\n}\n";
        assert!(
            findings(src).is_empty(),
            "host without origin is below the both-words gate"
        );
    }

    /// A gated fn suppresses whole (the `host_allowed` marker).
    #[test]
    fn batch329_gated_host_marker_suppresses() {
        let src = "pub struct ApiCtx {\n    pub host: Option<String>,\n    \
pub origin: Option<String>,\n}\n\npub fn handle(req: ApiCtx, body: &str) -> String {\n    \
if !host_allowed(req.host.as_deref().unwrap_or(\"\")) {\n        return String::new();\n    \
}\n    format!(\"{body}\")\n}\n";
        assert!(findings(src).is_empty(), "the gate vocabulary suppresses");
    }

    /// The `origin_key` marker suppresses too (the canonicalized-origin
    /// gate form).
    #[test]
    fn batch329_gated_origin_marker_suppresses() {
        let src = "pub struct Ctx {\n    pub host: Option<String>,\n    \
pub origin: Option<String>,\n}\n\npub fn handle(req: Ctx, body: &str) -> String {\n    \
if origin_key(req.origin.as_deref()) != Some(\"a.example\") {\n        return String::new();\n    \
}\n    format!(\"{body}\")\n}\n";
        assert!(findings(src).is_empty(), "the origin gate suppresses");
    }

    /// `#[cfg(test)]` mods are skipped (fixture handlers live there).
    #[test]
    fn batch329_test_mod_is_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    pub struct Ctx {\n        \
pub host: Option<String>,\n        pub origin: Option<String>,\n    }\n\n    \
pub fn handle(req: Ctx) -> String {\n        format!(\"x\")\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// The corpus buggy fixture fires (the finding names the ungated
    /// `handle`); the fixed fixture stays silent (gate vocabulary
    /// present).
    #[test]
    fn corpus_buggy_fixture_fires() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "browser-reachable-service-host-origin-rebinding-gate")
            .expect("rule present");
        let f = findings(&rule.buggy_code);
        assert!(!f.is_empty(), "the corpus buggy fixture must fire");
        assert!(
            f.iter().any(|x| x.call_text.contains("fn handle")),
            "the finding names the ungated handler: {f:?}"
        );
        let fixed = syn::parse_file(&rule.fixed_code).expect("fixed parses");
        assert!(
            detect(&rule.fixed_code, &fixed).is_empty(),
            "the fixed fixture must stay silent"
        );
    }
}
