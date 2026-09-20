//! `length-prefix-validate-before-alloc` — SUGGEST-ONLY detector
//! (Batch 131, the Girder distill; Plan 112 T4).
//!
//! A length parsed inside a fn (`u32::from_be_bytes(..)`-class byte-swap,
//! or a `.parse()` of a header field) flowing into a `vec![0u8; len]`
//! repeat allocation with NO intervening comparison of that length against
//! a bound is an allocation-size `DoS`: one peer-controlled header can
//! demand a multi-GB buffer (OOM/abort). The cost axis is security, not
//! cycles.
//!
//! The taint walk is the SHARED `wire_taint` substrate (Batch 165, Plan
//! 156 T2 — one walker, two sinks); this module contributes the ALLOC
//! sink (`vec![..; tracked_len]`) and nothing else.
//!
//! ## Why suggest-only (never a fix)
//!
//! The bounded repair must INVENT a limit constant and an error-return
//! path the enclosing fn may not have (`read_frame` may return `()`) —
//! a semantic decision, never mechanical (the
//! `ffi-export-panicking-body` precedent). This module therefore ships a
//! parse-only DETECTION walk only: NO `FIXABLE_SEC` membership, NO
//! `collect_edits` arm — the suggestion surfaces through retrieval
//! (`--suggest` / the corpus entry) and review.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Same-fn chain only**: the parse and the allocation must share the
//!   enclosing fn (the `nan-unsafe-sort-comparator` helper-fn precedent —
//!   cross-fn length origins are an honest recall limit, not a match).
//! - **Identity tracking**: the alloc's repeat count must be a bare path
//!   ident bound (transitively) to the parse — through `as usize` casts,
//!   `?`/`ok_or`/`unwrap` unwraps, `Some(..)` wrappers and assignments
//!   (the `Option<usize>` header-variable shape), and arithmetic
//!   (`len * 4` stays a peer-controlled magnitude).
//! - **NO intervening validation**: any comparison (`<`/`>`/`<=`/`>=`/
//!   `==`/`!=`) or `match`/`assert!` scrutinee involving the tracked
//!   ident between the parse and the alloc marks it validated — the
//!   bounded twin (`if len == 0 || len > MAX { return Err(..) }` before
//!   `vec![0; len]`) never fires. Validating a derived length validates
//!   its sources (`n = len * 4` checked bounds `len`).
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped
//!   (frame-parsing fixtures live there by design); since this detector
//!   emits no edits, interior comments cannot be displaced.
//! - **`vec!` repeat form only**: `with_capacity(n)` is excluded by
//!   design (trusted-metadata sizing is the convention; the negative
//!   suite pins it).
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Cross-fn origins: a helper that parses and a caller that allocates
//!   are not connected parse-only.
//! - Bare-ident counts only: `self.header_len`-class field/method origins
//!   are out (their types are unprovable parse-only).
//! - The shape proves a MISSING VALIDATION, not an untrusted source — a
//!   site parsing a trusted local file's length fires too, and review
//!   decides. Suggest-only is the blast-radius control.

use std::ops::Range;

use syn::Expr;
use syn::File;

use super::wire_taint::{self, FnState, WireSink};

/// One detected unbounded length-prefix allocation.
#[derive(Debug, Clone)]
pub struct FrameAllocFinding {
    /// Byte range of the `vec![..; len]` expression.
    pub alloc_span: Range<usize>,
    /// The alloc expression's source text.
    pub alloc_text: String,
    /// The tracked length ident at the repeat count.
    pub length_ident: String,
    /// The advice line (the bounded form + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "validate the length BEFORE the allocation: \
`if len == 0 || len > MAX { return Err(..) }` — a peer-controlled length \
prefix flowing into vec![0u8; len] is an allocation-size DoS (Batch 131, \
suggest-only)";

/// The alloc sink: a `vec![init; count]` whose count is a bare tracked,
/// unvalidated ident (the Batch-131 shape, unchanged by the Batch-165
/// walker extraction — the 14-test suite below pins that).
struct AllocSink<'a> {
    findings: &'a mut Vec<FrameAllocFinding>,
}

impl WireSink for AllocSink<'_> {
    fn check(&mut self, source: &str, expr: &Expr, state: &FnState) {
        let Some(rep_len) = wire_taint::vec_repeat_len(expr) else {
            return;
        };
        // Macro tokens are never descended into by the shared walk, so
        // this check is the ONLY look inside a vec! (same as pre-extraction).
        if let Some(id) = wire_taint::path_ident(&rep_len) {
            if state.is_tracked(&id) && !state.validated.contains(&id) {
                if let Some(span) = wire_taint::bounded_span(expr, source) {
                    self.findings.push(FrameAllocFinding {
                        alloc_text: source[span.clone()].to_string(),
                        alloc_span: span,
                        length_ident: id,
                        advice: ADVICE,
                    });
                }
            }
        }
    }
}

/// Detect unbounded `vec![..; parsed_len]` allocations (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<FrameAllocFinding> {
    let mut findings = Vec::new();
    wire_taint::walk_file(source, file, &mut AllocSink { findings: &mut findings });
    findings
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::FrameAllocFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 131 gate — the UNBOUNDED wire-frame shape (original fixture,
    /// the corpus `buggy_code`) MUST fire.
    #[test]
    fn batch131_unbounded_from_be_bytes_fires() {
        let src = "pub fn read_frame(stream: &mut std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n    \
             let mut prefix = [0_u8; 4];\n    \
             std::io::Read::read_exact(stream, &mut prefix)?;\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let mut body = vec![0_u8; len];\n    \
             std::io::Read::read_exact(stream, &mut body)?;\n    \
             Ok(body)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].length_ident, "len");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The header-string shape (`.parse()` through an Option variable and
    /// `ok_or_else(..)?`) MUST fire — the second evidence form.
    #[test]
    fn batch131_header_parse_through_option_fires() {
        let src = "pub fn read_body(reader: &mut std::io::BufReader<std::net::TcpStream>) -> Result<Vec<u8>, String> {\n    \
             let mut content_length: Option<usize> = None;\n    \
             let header = String::from(\"Content-Length: 4\");\n    \
             let rest = header.strip_prefix(\"Content-Length: \").unwrap_or(\"\");\n    \
             content_length = Some(rest.trim().parse().map_err(|e: std::num::ParseIntError| e.to_string())?);\n    \
             let len = content_length.ok_or_else(|| String::from(\"missing\"))?;\n    \
             let body = vec![0u8; len];\n    \
             Ok(body)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].length_ident, "len");
    }

    // ── NEGATIVE SUITE — every class must stay SILENT (zero-FP bar) ──

    /// The BOUNDED twin: the validation comparison precedes the alloc.
    #[test]
    fn batch131_bounded_twin_is_silent() {
        let src = "pub const MAX_FRAME_BYTES: usize = 1 << 20;\n\n\
             pub fn read_frame(stream: &mut std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n    \
             let mut prefix = [0_u8; 4];\n    \
             std::io::Read::read_exact(stream, &mut prefix)?;\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             if len == 0 || len > MAX_FRAME_BYTES {\n        \
             return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, \"bad\"));\n    \
             }\n    \
             let mut body = vec![0_u8; len];\n    \
             std::io::Read::read_exact(stream, &mut body)?;\n    \
             Ok(body)\n}\n";
        assert!(findings(src).is_empty(), "bounded twin must not fire");
    }

    /// Trusted length: `vec![0; buf.len()]` — the count is not a tracked
    /// parse local.
    #[test]
    fn batch131_trusted_buf_len_is_silent() {
        let src = "pub fn pad(buf: &[u8]) -> Vec<u8> {\n    \
             let mut out = vec![0_u8; buf.len()];\n    \
             out[..buf.len()].copy_from_slice(buf);\n    \
             out\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Const-size allocations (array literal + literal vec) are silent.
    #[test]
    fn batch131_const_sizes_are_silent() {
        let src = "pub fn headers() -> ([u8; 64], Vec<u8>) {\n    \
             let arr = [0_u8; 64];\n    \
             let v = vec![0_u8; 64];\n    \
             (arr, v)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Empty vecs (`Vec::new()` / `vec![]`) are silent.
    #[test]
    fn batch131_empty_vecs_are_silent() {
        let src = "pub fn fresh() -> (Vec<u8>, Vec<u8>) {\n    \
             (Vec::new(), vec![])\n}\n";
        assert!(findings(src).is_empty());
    }

    /// `with_capacity` from trusted metadata is OUT OF SCOPE by design —
    /// the negative suite pins the boundary.
    #[test]
    fn batch131_with_capacity_is_out_of_scope() {
        let src = "pub fn build(n: usize) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(n * 16);\n    \
             v.resize(n * 16, 0);\n    \
             v\n}\n";
        assert!(findings(src).is_empty());
    }

    /// HONEST RECALL LIMIT pinned as a test: the parse in a helper fn and
    /// the alloc in the caller are NOT connected parse-only (the
    /// `nan_safe_cmp` helper-fn precedent).
    #[test]
    fn batch131_cross_fn_origin_is_undetected() {
        let src = "fn parse_len(prefix: [u8; 4]) -> usize {\n    \
             u32::from_be_bytes(prefix) as usize\n}\n\n\
             pub fn read_frame(stream: &mut std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n    \
             let mut prefix = [0_u8; 4];\n    \
             std::io::Read::read_exact(stream, &mut prefix)?;\n    \
             let len = parse_len(prefix);\n    \
             let mut body = vec![0_u8; len];\n    \
             std::io::Read::read_exact(stream, &mut body)?;\n    \
             Ok(body)\n}\n";
        assert!(findings(src).is_empty(), "cross-fn origins stay undetected");
    }

    /// A field/method length origin (`self.header_len`) is out — bare
    /// idents only.
    #[test]
    fn batch131_field_origin_is_undetected() {
        let src = "pub struct Wire { pub header_len: usize }\n\n\
             impl Wire {\n    \
             pub fn body(&self) -> Vec<u8> {\n        vec![0_u8; self.header_len]\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// #[cfg(test)] mods are skipped (the comment guard, house discipline):
    /// the same unbounded shape inside a test mod never fires.
    #[test]
    fn batch131_test_mod_is_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             #[test]\n    fn t() {\n        \
             let n: usize = \"8\".parse().unwrap();\n        \
             let _b = vec![0u8; n];\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A length used in a `match` scrutinee counts as validated.
    #[test]
    fn batch131_match_scrutinee_validates() {
        let src = "pub fn f(prefix: [u8; 4]) -> Vec<u8> {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             match len {\n        0 => Vec::new(),\n        _ => vec![0_u8; len],\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Validating a DERIVED length bounds the source too.
    #[test]
    fn batch131_derived_validation_bounds_the_source() {
        let src = "pub const ROW: usize = 16;\n\n\
             pub fn f(prefix: [u8; 4]) -> Vec<u8> {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let n = len * ROW;\n    \
             if n > 1 << 20 {\n        return Vec::new();\n    }\n    \
             vec![0_u8; n]\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A derived magnitude with NO validation still fires
    /// (`len * 4` is as peer-controlled as `len`).
    #[test]
    fn batch131_derived_magnitude_without_validation_fires() {
        let src = "pub fn f(prefix: [u8; 4]) -> Vec<u8> {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let n = len * 4;\n    \
             vec![0_u8; n]\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].length_ident, "n");
    }

    /// The findings carry byte spans into the source (span integrity).
    #[test]
    fn batch131_finding_spans_slice_the_source() {
        let src = "pub fn f(prefix: [u8; 4]) -> Vec<u8> {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let body = vec![0_u8; len];\n    \
             body\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1);
        assert_eq!(&src[f[0].alloc_span.clone()], "vec![0_u8; len]");
    }
}
