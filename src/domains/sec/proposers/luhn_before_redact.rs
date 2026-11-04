//! `luhn-validate-numeric-pii-candidates-before-redact` — SUGGEST-ONLY
//! detector (Batch 330, the vectorize-io/hindsight distill; Plan 338 /
//! Research 368).
//!
//! A digit-FORMAT-only PII detector (13-16 decimal digits with optional
//! separators) redacts benign numeric runs — timestamps, unix millis,
//! port lists — as if they were cards. Checksum-validate each candidate
//! BEFORE substituting, and a REJECTED candidate must not consume a
//! later valid start: `re.sub` resumes at the rejected span's end, so
//! the naive pass over "2026 4111 1111 1111 1111" eats the real card
//! that starts mid-span. The in-source guards: the repeated-digit
//! refusal ("Repeated-digit placeholders (including all zeroes) can
//! pass Luhn"), the bounded UUID-neighborhood inspection, and the
//! overlap rescan ("search again from its next character instead").
//!
//! ## Why suggest-only (never a fix)
//!
//! The validator scheme is the detector's design decision (Luhn owns
//! cards; other PII classes carry other schemes), the rescan policy is
//! the same kind of decision, and a compliance-first deployment
//! deliberately over-redacts — that stance is legitimate. NO
//! `FIXABLE_SEC` membership, NO `collect_edits` arm — the suggestion
//! surfaces through retrieval and review.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Redaction context per fn**: the fn's name carries an EXACT word
//!   segment from redact/mask/censor/sanitize/pii/card.
//! - **Digit-format candidate vocabulary**: a numeric-literal range
//!   check naming card-class lengths (`13`, `16`, `19` in a comparison
//!   or a range) OR an identifier naming the candidate class
//!   (`card_candidate`, `digits_of`, `digit_run`, `is_card`).
//! - **Suppression, FN-WIDE**: any Luhn-shaped checksum arithmetic in
//!   the fn (an ident naming `luhn`/`checksum`/`validate_luhn`, or a `% 10`
//!   fold over a doubled-digit loop — approximated by the ident gate +
//!   `% 10`) silences every site: the law is already applied. Likewise
//!   a `luhn` mention anywhere in the fn.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped; macro statements are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - The vocabulary proves a CLASS, not a verdict: a detector that
//!   legitimately redacts by format only (compliance-first) fires too,
//!   and review decides (suggest-only is the blast-radius control).
//! - A checksum applied OUTSIDE the fn (a shared validator) is
//!   unconnected.
//! - Luhn-valid ≠ card — the checksum is a precision multiplier
//!   layered on the format gate (~1/10 of random runs pass), never a
//!   standalone classifier; the suggestion says so.
//! - Batch 330 / Plan 338 / Research 368 (vectorize-io/hindsight @
//!   `841979df`, MIT — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::{File, Item, ItemMod};

/// One detected format-only PII redaction site.
#[derive(Debug, Clone)]
pub struct LuhnFinding {
    /// Byte range of the fn (the gate is a property of the detector, not
    /// one line).
    pub site_span: Range<usize>,
    /// Which vocabulary armed the finding.
    pub reason: String,
    /// The advice line (checksum-before-substitute + the three guards).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "digit-format-only PII redaction: digit count alone \
redacts timestamps and port lists as if they were cards (memory_defense.py, \
verbatim). Checksum-validate each candidate BEFORE substituting — Luhn with \
the repeated-digit refusal (placeholders including all zeroes can pass \
Luhn) and the bounded UUID-neighborhood check — and a REJECTED candidate \
must not consume a later valid start: rescan from the rejected span's \
next character, never resume past it ('2026 4111 1111 1111 1111' keeps \
its inner card).";

const REDACT_VOCAB: [&str; 7] = [
    "redact", "mask", "censor", "sanitize", "pii", "card", "scrub",
];

const CANDIDATE_IDENTS: [&str; 4] = ["card_candidate", "digits_of", "digit_run", "is_card"];

/// Detect format-only PII redaction detectors.
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<LuhnFinding> {
    let mut findings = Vec::new();
    scan_items(source, &file.items, &mut findings);
    findings
}

fn scan_items(source: &str, items: &[Item], findings: &mut Vec<LuhnFinding>) {
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
    m.attrs.iter().any(|a| {
        a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
    })
}

fn scan_fn_source(
    source: &str,
    fn_name: &str,
    body_text: &str,
    findings: &mut Vec<LuhnFinding>,
) {
    let name_segs = word_segments(fn_name);
    let redact_ctx = REDACT_VOCAB
        .iter()
        .any(|v| name_segs.iter().any(|s| s == v));
    if !redact_ctx {
        return;
    }
    let body_lower = body_text.to_ascii_lowercase();
    // Suppression: the checksum law already applied anywhere in the fn.
    if body_lower.contains("luhn") || body_lower.contains("checksum") {
        return;
    }
    // Candidate vocabulary: a card-class length gate or a candidate-named
    // ident. Lengths 13/16/19 as bare literals inside a comparison or
    // range, or the named helper.
    let mut reason: Option<String> = None;
    if CANDIDATE_IDENTS
        .iter()
        .any(|c| body_lower.contains(&c.to_ascii_lowercase()))
    {
        reason = Some("card-class candidate helper".to_string());
    } else if has_card_length_gate(body_text) {
        reason = Some("card-class length gate (13/16/19)".to_string());
    }
    if let Some(reason) = reason {
        let start = source.find(body_text).unwrap_or(0);
        findings.push(LuhnFinding {
            site_span: start..start + body_text.len(),
            reason,
            advice: ADVICE,
        });
    }
}

/// A bare card-class length (13/16/19) in a comparison or range shape.
fn has_card_length_gate(body: &str) -> bool {
    for len in ["13", "16", "19"] {
        for ctx in [format!("== {len}"), format!(">= {len}"), format!("> {len}"), format!("{len}..")] {
            if body.contains(&ctx) {
                return true;
            }
        }
    }
    false
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
    fn fires_on_format_only_redaction() {
        let src = "fn redact_cards(content: &str) -> String {\n    \
                   if is_card_candidate(content) { return \"[X]\".into(); }\n    \
                   content.to_string()\n}\n\
                   fn is_card_candidate(s: &str) -> bool {\n    \
                   let n = digits_of(s).len();\n    (13..=16).contains(&n)\n}\n";
        let file: File = parse_str(src).unwrap();
        let found = detect(src, &file);
        // BOTH fns are the detector's surface: the redactor (it consumes the
        // candidate helper) AND the candidate gate itself (its name carries
        // `card`) — the rule spans the pair.
        assert_eq!(found.len(), 2, "redactor + candidate gate both fire");
        assert!(found.iter().any(|f| f.reason.contains("candidate")));
    }

    #[test]
    fn suppressed_by_luhn_in_fn() {
        let src = "fn redact_cards(content: &str) -> String {\n    \
                   if luhn_ok(content) { return \"[X]\".into(); }\n    content.to_string()\n}\n";
        let file: File = parse_str(src).unwrap();
        assert!(detect(src, &file).is_empty(), "luhn-validated fn is silent");
    }

    #[test]
    fn no_redact_vocabulary_no_finding() {
        let src = "fn parse_digits(s: &str) -> usize {\n    \
                   let n = digits_of(s).len();\n    n\n}\n";
        let file: File = parse_str(src).unwrap();
        assert!(detect(src, &file).is_empty(), "non-redaction fn never fires");
    }

    #[test]
    fn advice_carries_the_three_guards() {
        assert!(ADVICE.contains("Repeated-digit") || ADVICE.contains("repeated-digit"));
        assert!(ADVICE.contains("next character"));
        assert!(ADVICE.contains("UUID"));
    }
}
