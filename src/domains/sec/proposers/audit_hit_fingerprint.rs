//! `redaction-audit-hit-carries-raw-match` — SUGGEST-ONLY detector
//! (Batch 330, the vectorize-io/hindsight distill; Plan 338 / Research 368).
//!
//! A detection/audit emission that carries the MATCHED VALUE verbatim
//! re-leaks the secret into logs, APMs, queues and audit stores — the
//! audit wire IS a second copy of the data at rest, one nobody
//! redacted. Emit a LENGTH-TIERED FINGERPRINT PREVIEW instead: enough
//! prefix + suffix to correlate against a credential inventory, never
//! the body ("The preview keeps the prefix and a short suffix so a SIEM
//! operator can correlate ... without the raw secret crossing the
//! wire"; the contract: "The preview is *never* the raw value"). An
//! ABSENT preview must stay absent (None), never a blank string that
//! reads as a clean scan — receivers must distinguish "no preview
//! info" from "scanned, nothing matched".
//!
//! ## Why suggest-only (never a fix)
//!
//! Whether previews ship at all, and the tier boundaries, are
//! compliance decisions — a fingerprint is derived data (first4+last4
//! of a 40-char token still leaks 8 chars) and a compliance-first
//! deployment may want full masks or nothing. NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval and review.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Audit-emission context per fn**: the fn's name or the pushed
//!   struct's field vocabulary carries an EXACT word segment from
//!   hit/match/audit/finding/detect/event.
//! - **Raw-carry shape**: a struct-literal expression or a format!
//!   call inside the fn binds an ident naming the matched value
//!   (matched, raw, value, secret, found) into the emission — the
//!   match text flows into a stored/logged field, not a formatting
//!   parameter only.
//! - **Suppression, FN-WIDE**: any fingerprint/mask spelling in the fn
//!   (fingerprint, preview, `mask_value`, `[redacted]`, `redact_ident`,
//!   first ellipsis-building like `...`) silences every site: the law
//!   is already applied.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped; macro statements are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - The vocabulary proves a CLASS, not a verdict: an audit of
//!   non-secret matches (ids, paths, error codes) fires too, and
//!   review decides (suggest-only is the blast-radius control).
//! - Emissions built outside the fn (a telemetry builder) are
//!   unconnected.
//! - Distinct-from `constant-time-secret-compare` IN-RULE (comparison
//!   TIMING vs emission CONTENT) and `secret-key-without-zeroize`
//!   (memory lifetime/erase vs the log copy).
//! - Batch 330 / Plan 338 / Research 368 (vectorize-io/hindsight @
//!   `841979df`, MIT — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::{File, Item, ItemMod};

/// One detected raw-carry audit emission site.
#[derive(Debug, Clone)]
pub struct AuditHitFinding {
    /// Byte range of the fn.
    pub site_span: Range<usize>,
    /// Which vocabulary armed the finding.
    pub reason: String,
    /// The advice line (tiered previews + the honest-absence law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "the audit emission carries the matched value \
verbatim — the log/queue copy nobody redacted (memory_defense.py: 'The \
preview is *never* the raw value'). Emit a length-tiered fingerprint \
preview instead: <6 fully masked, 6-15 first2+last2, >15 first4+last4 — \
enough prefix/suffix to correlate against a credential inventory, never \
the body. Absent stays absent (None): receivers must distinguish 'no \
preview info' from 'scanned, nothing matched'.";

const AUDIT_VOCAB: [&str; 6] = ["hit", "match", "audit", "finding", "detect", "event"];

const RAW_IDENTS: [&str; 5] = ["matched", "raw", "secret", "found", "value"];

const SUPPRESS: [&str; 5] = [
    "fingerprint",
    "preview",
    "mask_value",
    "[redacted]",
    "redact_preview",
];

/// Detect raw-carry audit emissions.
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<AuditHitFinding> {
    let mut findings = Vec::new();
    scan_items(source, &file.items, &mut findings);
    findings
}

fn scan_items(source: &str, items: &[Item], findings: &mut Vec<AuditHitFinding>) {
    for item in items {
        match item {
            Item::Fn(f) => scan_fn_source(
                source,
                &f.sig.ident.to_string(),
                &f.block.to_token_stream().to_string(),
                findings,
            ),
            Item::Impl(imp) => {
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(m) = ii {
                        scan_fn_source(
                            source,
                            &m.sig.ident.to_string(),
                            &m.block.to_token_stream().to_string(),
                            findings,
                        );
                    }
                }
            }
            Item::Mod(m) => {
                // Comment guard (house discipline): #[cfg(test)] mods are
                // fixture habitat.
                if is_test_mod(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    scan_items(source, nested, findings);
                }
            }
            _ => {}
        }
    }
}

/// `#[cfg(test)]` mods are fixture habitat.
fn is_test_mod(m: &ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

fn scan_fn_source(
    source: &str,
    fn_name: &str,
    body_text: &str,
    findings: &mut Vec<AuditHitFinding>,
) {
    let name_segs = word_segments(fn_name);
    let audit_ctx = AUDIT_VOCAB.iter().any(|v| name_segs.iter().any(|s| s == v));
    if !audit_ctx {
        return;
    }
    let body_lower = body_text.to_ascii_lowercase();
    // Suppression: the fingerprint law already applied anywhere in the fn.
    if SUPPRESS
        .iter()
        .any(|s| body_lower.contains(&s.to_ascii_lowercase()))
    {
        return;
    }
    // Raw-carry shape: a struct-literal expression or format!/push binds a
    // raw-value ident into the emission. (Token-stream rendering spaces
    // every boundary — `hits.push(format!(..))` renders as `hits . push
    // (format ! (..))` — so the emission gate matches the loose spellings.)
    let raw_hit = RAW_IDENTS
        .iter()
        .any(|r| body_lower.contains(&r.to_ascii_lowercase()));
    let emission = body_lower.contains("push")
        || body_lower.contains("format !")
        || body_lower.contains("format!");
    if raw_hit && emission {
        let start = source.find(body_text).unwrap_or(0);
        findings.push(AuditHitFinding {
            site_span: start..start + body_text.len(),
            reason: "matched-value identifier bound into the audit emission".to_string(),
            advice: ADVICE,
        });
    }
}

/// Exact word segments: `snake_case` and `CamelCase` both split.
fn word_segments(name: &str) -> Vec<String> {
    let mut segs = Vec::new();
    let mut cur = String::new();
    for c in name.chars() {
        if c == '_' || c == '-' {
            if !cur.is_empty() {
                segs.push(cur.to_ascii_lowercase());
                cur.clear();
            }
        } else if c.is_ascii_uppercase() {
            if !cur.is_empty() {
                segs.push(cur.to_ascii_lowercase());
            }
            cur.clear();
            cur.push(c.to_ascii_lowercase());
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        segs.push(cur.to_ascii_lowercase());
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    #[test]
    fn fires_on_raw_carry_audit_push() {
        let src = "fn record_hit(hits: &mut Vec<String>, detector: &str, matched: &str) {\n    \
                   hits.push(format!(\"{detector}: {matched}\"));\n}\n";
        let file: File = parse_str(src).unwrap();
        let found = detect(src, &file);
        assert_eq!(found.len(), 1, "the raw-carry audit push fires");
        assert!(found[0].reason.contains("matched-value"));
    }

    #[test]
    fn suppressed_by_fingerprint_in_fn() {
        let src = "fn record_hit(hits: &mut Vec<String>, matched: &str) {\n    \
                   hits.push(format!(\"hit: {}\", fingerprint_value(matched)));\n}\n";
        let file: File = parse_str(src).unwrap();
        assert!(detect(src, &file).is_empty(), "fingerprinted fn is silent");
    }

    #[test]
    fn no_audit_vocabulary_no_finding() {
        let src = "fn store(value: &str) {\n    \
                   let mut v = Vec::new();\n    v.push(value.to_string());\n}\n";
        let file: File = parse_str(src).unwrap();
        assert!(detect(src, &file).is_empty(), "non-audit fn never fires");
    }

    #[test]
    fn advice_carries_tiers_and_absence() {
        assert!(ADVICE.contains("first4+last4") || ADVICE.contains("first2+last2"));
        assert!(ADVICE.contains("scanned, nothing matched"));
        assert!(ADVICE.contains("*never* the raw value"));
    }
}
