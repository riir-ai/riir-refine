//! `ascii-token-unicode-word-boundary-scan-miss` — SUGGEST-ONLY detector
//! (Batch 330, the vectorize-io/hindsight distill; Plan 338 / Research 368).
//!
//! A token-anchored secret/PII scan whose boundary is Unicode-aware
//! silently MISSES the token when it is butted against non-ASCII word
//! characters: `regex`'s `\b` (and `\w`) key on Unicode by default, so a
//! CJK character counts as a word character and there is no word
//! boundary between 为 and `s` — `\bsk_test_...\b` fails to match in
//! `凭证为sk_test_ABC...`, and secrets embedded in CJK prose reach the store
//! unredacted. The bounded spellings (review picks): `(?-u:\b)` (the
//! ASCII word boundary), fancy-regex lookarounds (dep-gated), or
//! post-match flanking-byte validation (`is_ascii_alphanumeric`/`_` on
//! the neighboring bytes — no regex change at all). The rewrite is safe
//! because the ASCII alphabet is a SUBSET of Unicode `\w`: nothing that
//! matched before stops matching.
//!
//! ## Why suggest-only (never a fix)
//!
//! The choice among the three Rust spellings is a dependency +
//! correctness decision, and a Unicode-aware boundary is sometimes the
//! intent (a CJK word-segmenter wants Unicode `\b`). NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval and review.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Scan context per fn**: the fn's name or its receiver/param
//!   vocabulary carries an EXACT word segment from the scan lane
//!   (secret, token, redact, scan, pattern, censor, mask, detect) —
//!   the class is the SECRET-SCAN surface, not every `\b` on earth.
//! - **Boundary vocabulary in the fn**: a string literal containing
//!   `\\b` (the regex-boundary escape, plain `str` literals only) OR an
//!   identifier/field naming boundary semantics (`word_boundary`,
//!   `boundary_before`, `token_start`, `token_end`, `is_word_char`,
//!   `word_char`) OR a call to `is_alphanumeric` (the std Unicode twin)
//!   in a predicate position. One of these must hit.
//! - **Suppression, FN-WIDE**: an explicit ASCII-alphabet spelling
//!   anywhere in the fn (`is_ascii_alphanumeric`, `(?-u:`, `A-Za-z0-9`,
//!   `ascii_boundary`, `ascii_token`) means the law is already applied
//!   — silence every site in it. Likewise any lookaround spelling
//!   (`(?<` / `(?!`).
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped; macro statements are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Boundary logic built in another module (a shared matcher helper)
//!   is unconnected — the fn must carry the vocabulary.
//! - A regex built at runtime from config (the pattern arrives as
//!   data) is invisible: only literal `\\b` in a source string literal
//!   is seen.
//! - The vocabulary proves a CLASS, not a verdict: a CJK segmenter
//!   with deliberately Unicode boundaries fires too, and review
//!   decides (suggest-only is the blast-radius control).
//! - Distinct-from the corpus's identifier-vocabulary gates
//!   (`secret_file_create`, `secret_ct_eq`) IN-RULE: those gate WHERE a
//!   detector looks (code identifiers), this gates WHAT a pattern's
//!   boundary means (scan/regex boundary semantics).
//! - Batch 330 / Plan 338 / Research 368 (vectorize-io/hindsight @
//!   `841979df`, MIT — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::{File, Item, ItemMod};

/// One detected unicode-boundary scan site.
#[derive(Debug, Clone)]
pub struct SensitiveDataBoundaryFinding {
    /// Byte range of the fn (the site is fn-grained: the boundary is a
    /// property of the pattern/predicate, not one line).
    pub site_span: Range<usize>,
    /// Which vocabulary armed the finding.
    pub reason: String,
    /// The advice line (the three bounded spellings + the subset law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "unicode-aware boundary in a token-anchored scan: \
`\\b`/`\\w` key on Unicode, so a CJK char adjacent to the token kills the \
boundary and the secret passes unredacted (memory_defense.py: 'there is no \
word boundary between 为 and s'). Anchor on the ASCII token alphabet — \
`(?-u:\\b)` in regex, fancy-regex lookarounds `(?<![A-Za-z0-9_])`, or \
flanking-byte `is_ascii_alphanumeric()` checks after the match. The ASCII \
alphabet is a subset of Unicode `\\w`: nothing that matched before stops \
matching.";

const SCAN_VOCAB: [&str; 8] = [
    "secret", "token", "redact", "scan", "pattern", "censor", "mask", "detect",
];

const BOUNDARY_IDENTS: [&str; 6] = [
    "word_boundary",
    "boundary_before",
    "token_start",
    "token_end",
    "is_word_char",
    "word_char",
];

const SUPPRESS: [&str; 7] = [
    "is_ascii_alphanumeric",
    "(?-u:",
    "A-Za-z0-9",
    "ascii_boundary",
    "ascii_token",
    "(?<",
    "(?!",
];

/// Detect fn-grained unicode-boundary scan sites.
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<SensitiveDataBoundaryFinding> {
    let mut findings = Vec::new();
    scan_items(source, &file.items, &mut findings);
    findings
}

fn scan_items(source: &str, items: &[Item], findings: &mut Vec<SensitiveDataBoundaryFinding>) {
    for item in items {
        match item {
            Item::Fn(f) => scan_fn_source(
                source,
                &f.sig.ident.to_string(),
                &f.to_token_stream().to_string(),
                findings,
            ),
            Item::Impl(imp) => {
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(m) = ii {
                        scan_fn_source(
                            source,
                            &m.sig.ident.to_string(),
                            &m.to_token_stream().to_string(),
                            findings,
                        );
                    }
                }
            }
            Item::Mod(m) => {
                // Comment guard (house discipline): #[cfg(test)] mods are
                // fixture habitat — a test fn named `redact_tokens` must
                // never fire on its own detector.
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
    findings: &mut Vec<SensitiveDataBoundaryFinding>,
) {
    let name_segs = word_segments(fn_name);
    let scan_ctx = SCAN_VOCAB.iter().any(|v| name_segs.iter().any(|s| s == v));
    if !scan_ctx {
        return;
    }
    let body_lower = body_text.to_ascii_lowercase();
    // Suppression: the ASCII law already applied anywhere in the fn.
    if SUPPRESS.iter().any(|s| body_text.contains(s)) {
        return;
    }
    // Boundary vocabulary: a literal `\\b` escape (str literals carry the
    // escaped backslash-b), a boundary-named ident, or is_alphanumeric
    // in predicate position.
    let mut reason: Option<String> = None;
    if body_text.contains("\\\\b") {
        reason = Some("regex-boundary escape `\\\\b` in a pattern literal".to_string());
    } else if BOUNDARY_IDENTS
        .iter()
        .any(|b| body_lower.contains(&b.to_ascii_lowercase()))
    {
        reason = Some("boundary-named identifier".to_string());
    } else if body_lower.contains("is_alphanumeric") {
        reason = Some("Unicode-aware `is_alphanumeric` boundary predicate".to_string());
    }
    if let Some(reason) = reason {
        let start = source.find(body_text).unwrap_or(0);
        findings.push(SensitiveDataBoundaryFinding {
            site_span: start..start + body_text.len(),
            reason,
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
    fn fires_on_unicode_boundary_in_scan_context() {
        let src = "fn redact_tokens(text: &str, needle: &str) -> String {\n    \
                   let pat = \"\\\\bsk_test_\\\\b\";\n    text.replace(pat, \"[REDACTED]\")\n}\n";
        let file: File = parse_str(src).unwrap();
        let found = detect(src, &file);
        assert_eq!(found.len(), 1, "the scan fn with a \\\\b literal fires");
        assert!(found[0].reason.contains("regex-boundary"));
    }

    #[test]
    fn fires_on_is_alphanumeric_predicate_in_scan_context() {
        let src = "fn scan_secret(text: &str) -> bool {\n    \
                   text.chars().next().is_some_and(|c| c.is_alphanumeric())\n}\n";
        let file: File = parse_str(src).unwrap();
        let found = detect(src, &file);
        assert_eq!(found.len(), 1, "is_alphanumeric in a scan fn fires");
        assert!(found[0].reason.contains("is_alphanumeric"));
    }

    #[test]
    fn suppressed_by_ascii_law_in_fn() {
        let src = "fn redact_tokens(text: &str) -> String {\n    \
                   let ok = text.chars().all(|c| c.is_ascii_alphanumeric());\n    \
                   let pat = \"\\\\bsk_test\\\\b\";\n    text.replace(pat, \"\")\n}\n";
        let file: File = parse_str(src).unwrap();
        assert!(detect(src, &file).is_empty(), "ascii-alphabet fn is silent");
    }

    #[test]
    fn no_scan_vocabulary_no_finding() {
        let src = "fn split_words(text: &str) -> Vec<&str> {\n    \
                   let pat = \"\\\\bw\\\\+\";\n    vec![text]\n}\n";
        let file: File = parse_str(src).unwrap();
        assert!(detect(src, &file).is_empty(), "non-scan fn never fires");
    }

    #[test]
    fn advice_names_the_three_spellings() {
        assert!(ADVICE.contains("(?-u:"));
        assert!(ADVICE.contains("A-Za-z0-9"));
        assert!(ADVICE.contains("is_ascii_alphanumeric"));
    }
}
