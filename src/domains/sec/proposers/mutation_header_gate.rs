//! `custom-header-mutation-gate-unanswered-preflight` — SUGGEST-ONLY
//! detector (Batch 329, the tembo/`voxl` distill; Research 367 / Plan 337).
//!
//! A browser-reachable mutating route (the upload class) that performs NO
//! custom-header / CSRF-token check is writable by ANY web page the
//! victim's browser visits: a hostile page can POST a file upload
//! cross-site (an auto-submitted form or a simple fetch) and the browser
//! attaches the victim's ambient credentials. The defense is MECHANICAL:
//! require a custom header on the mutating route and never answer the
//! CORS preflight — browsers cannot attach custom headers cross-site
//! without a successful preflight, so an unanswered preflight closes the
//! write path. `voxl`'s upload law is the class description (verbatim, at
//! the pin): "Uploads must carry this header. Browsers only send custom
//! headers cross-origin after a CORS preflight, which we never answer, so
//! other sites can't upload."
//!
//! ## Why suggest-only (never a fix)
//!
//! The header name is a deployment decision (server and clients must
//! agree), and an API already protected by bearer-token auth, CSRF
//! tokens, or SameSite=Strict cookies deliberately skips the gate. This
//! module ships a parse-only DETECTION walk only: NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! pub async fn upload(State(f): State<Arc<Files>>, Query(q): Q,
//!                     headers: HeaderMap, body: Body) -> Response {
//!     if !headers.contains_key(UPLOAD_HEADER) {          // <- the gate
//!         return err(StatusCode::FORBIDDEN, "missing upload header");
//!     }
//!     // ... stream body to disk under the resolved dir ...
//! }
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Upload vocabulary (the FP control)**: the fn's token text must
//!   carry upload/multipart vocabulary (`upload` / `multipart`) — the
//!   browser-writable file-write class this rule owns.
//! - **Sink or handler marker**: a file-write sink shape in the same fn
//!   (`fs::write` / `File::create` / `create_dir` / `OpenOptions` /
//!   `write_all`) OR HTTP-handler vocabulary (`StatusCode` / status-code
//!   tokens / `Response`) — an upload-NAMED helper that neither writes
//!   nor answers stays silent.
//! - **Header-gate suppression (fn-wide)**: any header-presence or
//!   site-check token (`contains_key(` / `csrf` / `xsrf` /
//!   `authorization` / `bearer` / `sec_fetch` / `origin` / `referer`)
//!   means the route is already gated by one of the mechanism-class
//!   defenses (custom header, CSRF token, bearer auth, site
//!   classification) — silent.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Both gates are fn-level: a route whose upload handler delegates the
//!   disk write to a helper with no upload vocabulary, or whose
//!   gate/middleware lives in another fn (an axum middleware layer —
//!   `voxl` itself gates Host/Origin in `access.rs`, cross-fn to this
//!   handler), is unconnected parse-only.
//! - The vocabulary proves a CLASS, not a verdict: an offline batch
//!   uploader (no browser ever reaches it) fires too, and review decides.
//! - The broader mutating-route surface (rename/delete/create endpoints
//!   without an upload file-write shape) is OUT of this rule's fire set —
//!   same class, deliberately refused for precision; the corpus entry
//!   names it as the family's frontier.
//! - Batch 329 / Plan 337 / Research 367 (tembo/`voxl` @ `eba3b7ce`,
//!   MIT — cited as class provenance; the fixtures are original
//!   syntheses). The B258 CSRF-bulk drop adjudicated at authoring: the
//!   declined bulk is session ARCHITECTURE (rate limiting, IDOR, authz,
//!   session management, `OpenAPI` exposure); THIS is the specific
//!   mechanical header-gate form with an in-source mechanism reason — a
//!   different fix-space, zero corpus owner (csrf/preflight/
//!   custom-header greps all zero, 2026-11-04).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;

/// One detected ungated browser-writable upload site.
#[derive(Debug, Clone)]
pub struct MutationHeaderGateFinding {
    /// Byte range of the upload handler's signature block.
    pub site_span: Range<usize>,
    /// The fn signature's source text.
    pub site_text: String,
    /// The advice line (the unanswered-preflight regime + the class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "gate this browser-reachable upload route on a \
custom header and never answer the CORS preflight: browsers cannot attach \
custom headers cross-site without a preflight, so the gate closes the \
cross-site write path while same-origin clients keep working \
(`if !headers.contains_key(UPLOAD_HEADER) { return 403; }` shape; \
alternatives — CSRF token, bearer auth, SameSite=Strict — are deliberate \
choices review may prefer) (Batch 329, suggest-only)";

/// The upload/multipart vocabulary (token-text substrings, lowercase).
const UPLOAD_TOKENS: &[&str] = &["upload", "multipart"];

/// The sink/handler markers (token-text substrings, lowercase).
const SINK_TOKENS: &[&str] = &[
    "fs::write",
    "file::create",
    "create_dir",
    "openoptions",
    "write_all",
    "statuscode",
    "status_code",
    "bad_request",
    "forbidden",
    "response",
];

/// The header-gate / site-check suppression markers (fn-wide, lowercase).
const GATE_TOKENS: &[&str] = &[
    "contains_key(",
    "csrf",
    "xsrf",
    "authorization",
    "bearer",
    "sec_fetch",
    "sec-fetch-site",
    "origin",
    "referer",
];

/// Detect ungated browser-writable upload routes (suggest-only; no edits
/// are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &syn::File) -> Vec<MutationHeaderGateFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &syn::Item, findings: &mut Vec<MutationHeaderGateFinding>) {
    match item {
        syn::Item::Fn(f) => scan_fn(source, f, findings),
        syn::Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, m, findings);
                }
            }
        }
        syn::Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production routes.
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

fn scan_fn(source: &str, f: &dyn FnLike, findings: &mut Vec<MutationHeaderGateFinding>) {
    let sig_ts = f.sig_tokens();
    let sig_lower = sig_ts.to_ascii_lowercase();
    let body_ts = f.body_tokens();
    // Token streams stringify with spaces between every token (`fs :: write`,
    // `contains_key (`) — flatten so the substring gates read naturally.
    let fn_lower = format!("{sig_lower} {}", body_ts.to_ascii_lowercase()).replace(' ', "");

    // Gate 1: upload/multipart vocabulary.
    if !UPLOAD_TOKENS.iter().any(|t| fn_lower.contains(t)) {
        return;
    }
    // Gate 2: a write-sink or HTTP-handler marker.
    if !SINK_TOKENS.iter().any(|t| fn_lower.contains(t)) {
        return;
    }
    // Gate 3 (fn-wide): a header-gate / site-check token means the route
    // is already gated by a mechanism-class defense.
    if GATE_TOKENS.iter().any(|t| fn_lower.contains(t)) {
        return;
    }

    let span = f.span_range();
    if span.end <= source.len() {
        findings.push(MutationHeaderGateFinding {
            site_text: source[span.clone()].to_string(),
            site_span: span,
            advice: ADVICE,
        });
    }
}

/// The sig/body/span triple the scanner needs, implemented by both
/// free fns and inherent impl methods so the walk stays one shape.
trait FnLike {
    fn sig_tokens(&self) -> String;
    fn body_tokens(&self) -> String;
    fn span_range(&self) -> Range<usize>;
}

impl FnLike for syn::ItemFn {
    fn sig_tokens(&self) -> String {
        self.sig.to_token_stream().to_string()
    }
    fn body_tokens(&self) -> String {
        self.block.to_token_stream().to_string()
    }
    fn span_range(&self) -> Range<usize> {
        self.sig.span().byte_range()
    }
}

impl FnLike for syn::ImplItemFn {
    fn sig_tokens(&self) -> String {
        self.sig.to_token_stream().to_string()
    }
    fn body_tokens(&self) -> String {
        self.block.to_token_stream().to_string()
    }
    fn span_range(&self) -> Range<usize> {
        self.sig.span().byte_range()
    }
}

#[cfg(test)]
mod tests {
    use super::{detect, ADVICE};

    fn findings(src: &str) -> Vec<super::MutationHeaderGateFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 329 gate — the corpus `buggy_code` MUST fire: an upload
    /// handler that streams a body to disk with no header gate.
    #[test]
    fn batch329_ungated_upload_route_fires() {
        let src = "pub async fn upload_file(state: State<Store>, body: Body) -> Response {\n    \
             let dir = state.root.join(\"incoming\");\n    \
             create_dir_all(&dir).await;\n    \
             fs::write(dir.join(\"blob\"), axum::body::to_bytes(body, 1 << 20).await.unwrap_or_default()).await;\n    \
             StatusCode::OK\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].advice, ADVICE);
    }

    /// The fixed shape (a header-presence gate) stays silent.
    #[test]
    fn header_gated_upload_silent() {
        let src = "pub async fn upload_file(headers: HeaderMap, body: Body) -> Response {\n    \
             if !headers.contains_key(\"x-app-upload\") {\n        \
             return StatusCode::FORBIDDEN.into_response();\n    \
             }\n    \
             fs::write(\"/srv/blob\", axum::body::to_bytes(body, 1 << 20).await.unwrap_or_default()).await;\n    \
             StatusCode::OK\n}\n";
        assert!(findings(src).is_empty(), "header gate present: silent");
    }

    /// A CSRF-token-checked upload stays silent.
    #[test]
    fn csrf_gated_upload_silent() {
        let src = "pub async fn upload(form: Multipart) -> Response {\n    \
             if form.field(\"csrf\") != expected() { return bad_request(); }\n    \
             save(form).await;\n    \
             ok()\n}\n";
        assert!(findings(src).is_empty(), "csrf gate present: silent");
    }

    /// A bearer-authenticated upload stays silent (browsers cannot attach
    /// Authorization cross-site without a preflight — same mechanism class).
    #[test]
    fn bearer_authed_upload_silent() {
        let src = "pub async fn upload(auth: Authorization, body: Body) -> Response {\n    \
             fs::write(\"blob\", axum::body::to_bytes(body, 1 << 20).await.unwrap_or_default()).await;\n    \
             StatusCode::OK\n}\n";
        assert!(findings(src).is_empty(), "bearer auth present: silent");
    }

    /// A site-classifying upload stays silent (origin check in-fn).
    #[test]
    fn origin_checked_upload_silent() {
        let src = "pub async fn upload(origin: Option<String>, body: Body) -> Response {\n    \
             if origin.as_deref() != Some(\"https://app.example\") { return forbidden(); }\n    \
             fs::write(\"blob\", axum::body::to_bytes(body, 1 << 20).await.unwrap_or_default()).await;\n    \
             ok()\n}\n";
        assert!(findings(src).is_empty(), "origin check present: silent");
    }

    /// An upload-named fn with no sink and no handler vocabulary stays
    /// silent (gate 2 unmet).
    #[test]
    fn upload_named_helper_without_sink_silent() {
        let src = "pub fn upload_plan(kind: &str) -> usize {\n    \
             if kind == \"bulk\" { 4 } else { 1 }\n}\n";
        assert!(
            findings(src).is_empty(),
            "no sink, no handler vocab: silent"
        );
    }

    /// A plain fs write with no upload vocabulary stays silent (gate 1
    /// unmet — offline writers are not the browser class).
    #[test]
    fn plain_write_without_upload_vocab_silent() {
        let src = "pub fn save_report(rows: &[u8]) -> std::io::Result<()> {\n    \
             fs::write(\"report.bin\", rows)?;\n    \
             Ok(())\n}\n";
        assert!(findings(src).is_empty(), "no upload vocab: silent");
    }

    /// Inherent impl methods are scanned.
    #[test]
    fn impl_method_ungated_upload_fires() {
        let src = "struct Vault { root: PathBuf }\n\n\
             impl Vault {\n    \
             pub async fn multipart_store(&self, body: Body) -> Response {\n        \
             create_dir_all(&self.root).await;\n        \
             write_all(&self.root.join(\"part\"), body).await;\n        \
             StatusCode::CREATED\n        \
             }\n\
             }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "impl-method finding expected: {f:?}");
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn upload(x: Body) { fs::write(\"t\", x); }\n\
             }\n";
        assert!(findings(src).is_empty());
    }
}
