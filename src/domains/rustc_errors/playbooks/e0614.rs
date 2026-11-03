//! E0614 — deref of a non-pointer — the deref-at-use playbook (Issue 100).
//!
//! The organic shape: a binding is by value (`let head = v[0];`, or a healed
//! `v[0].clone()`) but a use site still dereferences it (`*head`). rustc
//! reports exactly one E0614 per deref site, primary span covering the whole
//! `*head` expression, label "can't be dereferenced", NO suggestion (measured
//! on rustc 1.98.1 — the probe transcript lives in Issue 100; the shapes are
//! not derived from documentation).
//!
//! The fix is the stale-star deletion: `*head` → `head`. It is load-bearing
//! as the UNMASKING half of a pair: while the E0614 stands, typeck failure
//! blocks borrowck entirely, so every borrowck error behind it is invisible.
//! Removing the star is what reveals them — which is why the driver's keep
//! rule ranks the fix by the lexicographic `(masking, count)` measure and
//! this keep is legal at equal count (`1 E0614 → 1 E0499` is progress, not
//! a no-progress revert — the exact trade the count-only rule could not see
//! and Bench 089 recorded as the F1/F3 stall).
//!
//! Scope guard: fires ONLY when the primary span's source text is exactly
//! `*ident` (one star, one ascii identifier). `**x`, `*x.f`, `*f(x)`, `*x[0]`
//! stay misses — a multi-token operand needs a real edit decision this
//! template refuses to guess.

use std::path::Path;

use super::{cand, CompileProposer, FixCandidate};
use crate::domains::rustc_errors::diag::RustcErrorDiag;

/// `*head` → `head`: delete the stale deref star at the use the diagnostic
/// points at. No semantic note — the star was a type error; reading the
/// binding by name is the only compiling form and moves no ownership.
pub struct E0614DerefAtUse;

impl CompileProposer for E0614DerefAtUse {
    fn name(&self) -> &'static str {
        "e0614-deref-at-use"
    }

    fn code(&self) -> &'static str {
        "E0614"
    }

    fn propose(&self, diag: &RustcErrorDiag, file: &Path, source: &str) -> Vec<FixCandidate> {
        if diag.code != self.code() {
            return Vec::new();
        }
        // The anchor is the diagnostic's OWN primary span (the oracle's
        // location knowledge — never guessed): labeled "can't be
        // dereferenced", in the primary file, with byte offsets.
        let Some((pf, _)) = diag.primary.as_ref() else {
            return Vec::new();
        };
        let Some(span) = diag
            .labeled_spans
            .iter()
            .find(|ls| ls.file == *pf && ls.is_primary && ls.label.contains("dereferenced"))
        else {
            return Vec::new();
        };
        let Some(text) = source.get(span.byte_start..span.byte_end) else {
            return Vec::new();
        };
        // Exactly `*ident` — the measured shape. One star, then a single
        // ascii identifier (first char not a digit).
        let ident = match text.strip_prefix('*') {
            Some(rest) if is_plain_ident(rest) => rest,
            _ => return Vec::new(),
        };
        // Delete the star only: span text `*ident` → `ident`. The primary
        // edit covers the star's single byte; the identifier is untouched.
        debug_assert_eq!(span.byte_end - span.byte_start, 1 + ident.len());
        vec![cand(
            diag,
            file,
            span.line_start,
            (span.byte_start, span.byte_start + 1),
            String::new(),
            self.name(),
        )]
    }
}

/// A plain ascii identifier: non-empty, `[A-Za-z_][A-Za-z0-9_]*`. Raw
/// identifiers (`r#type`) and unicode idents stay misses (multi-token
/// decision refused, same as `**x`).
fn is_plain_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::rustc_errors::diag::{LabeledSpan, SuggestionSpan};

    fn e0614_diag(byte_start: usize, byte_end: usize, label: &str) -> RustcErrorDiag {
        RustcErrorDiag {
            code: "E0614".to_string(),
            message: "type `i32` cannot be dereferenced".to_string(),
            primary: Some(("src/lib.rs".to_string(), 7)),
            suggestions: vec![SuggestionSpan {
                file: "src/lib.rs".to_string(),
                line_start: 7,
                byte_start: None,
                byte_end: None,
                replacement: String::new(),
                applicability: crate::domains::rustc_errors::diag::Applicability::Unknown,
                message: String::new(),
            }],
            labeled_spans: vec![LabeledSpan {
                file: "src/lib.rs".to_string(),
                line_start: 7,
                is_primary: true,
                byte_start,
                byte_end,
                label: label.to_string(),
            }],
            rendered: None,
        }
    }

    const SRC: &str = "pub fn probe(v: &mut Vec<i32>) -> i32 {\n    let head = v[0];\n    let a = &mut *v;\n    let b = &mut *v;\n    a.push(4);\n    b.push(5);\n    *head + v[0]\n}\n";

    /// The star's byte offset: `*head` sits at the start of the `*head +
    /// v[0]` line (line 7 → byte offset of the line's first char + 4 spaces).
    fn star_at() -> usize {
        SRC.find("*head").unwrap()
    }

    #[test]
    fn deletes_exactly_the_star() {
        let at = star_at();
        let diag = e0614_diag(at, at + 5, "can't be dereferenced");
        let out = E0614DerefAtUse.propose(&diag, Path::new("src/lib.rs"), SRC);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].strategy_id, "e0614-deref-at-use");
        assert_eq!(out[0].byte_span, (at, at + 1));
        assert_eq!(out[0].replacement, "");
        assert_eq!(out[0].extra_edits, []);
        // The spliced result reads `head + v[0]`.
        let spliced = format!("{}{}{}", &SRC[..at], "", &SRC[at + 1..]);
        assert!(spliced.contains("head + v[0]"), "{spliced}");
        assert!(!spliced.contains("*head"));
    }

    #[test]
    fn refuses_non_identifier_operands() {
        let at = star_at();
        // `**x` — a double deref is a different decision.
        let src = "fn f(x: i32) -> i32 { **x }\n";
        let at2 = src.find("**x").unwrap();
        let _ = at;
        let diag = e0614_diag(at2, at2 + 3, "can't be dereferenced");
        assert_eq!(
            E0614DerefAtUse.propose(&diag, Path::new("src/lib.rs"), src),
            []
        );
        // `*x.f` — field access under the star.
        let src = "struct S { f: i32 }\nfn g(x: S) -> i32 { *x.f }\n";
        let at3 = src.find("*x.f").unwrap();
        let diag = e0614_diag(at3, at3 + 4, "can't be dereferenced");
        assert_eq!(
            E0614DerefAtUse.propose(&diag, Path::new("src/lib.rs"), src),
            []
        );
        // `*f(x)` — a call operand.
        let src = "fn h() -> i32 { 1 }\nfn i() -> i32 { *h() }\n";
        let at4 = src.find("*h()").unwrap();
        let diag = e0614_diag(at4, at4 + 4, "can't be dereferenced");
        assert_eq!(
            E0614DerefAtUse.propose(&diag, Path::new("src/lib.rs"), src),
            []
        );
    }

    #[test]
    fn refuses_foreign_codes_and_missing_anchor() {
        // Wrong code (the propose guard).
        let at = star_at();
        let mut diag = e0614_diag(at, at + 5, "can't be dereferenced");
        diag.code = "E0502".to_string();
        assert_eq!(
            E0614DerefAtUse.propose(&diag, Path::new("src/lib.rs"), SRC),
            []
        );
        // No labeled span with the deref label → silent (miss, not a guess).
        let diag = e0614_diag(at, at + 5, "first mutable borrow occurs here");
        assert_eq!(
            E0614DerefAtUse.propose(&diag, Path::new("src/lib.rs"), SRC),
            []
        );
        // Labeled but the span does not carry bytes-identical source (drift).
        let diag = e0614_diag(at, at + 9, "can't be dereferenced");
        assert_eq!(
            E0614DerefAtUse.propose(&diag, Path::new("src/lib.rs"), SRC),
            []
        );
    }
}
