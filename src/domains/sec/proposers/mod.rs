//! Stage 2 proposal mechanism for the `sec` domain (Plan 018 T1.3) —
//! per-rule `syn`-level rewrites, mirroring the `rust_perf` proposers.
//!
//! Four of the six Phase-1 rules carry proposers. The other two are
//! suggest-only BY DESIGN:
//!
//! | Rule | Proposer | Why |
//! |---|---|---|
//! | `ffi-extern-fn-catch-unwind` | ✓ | signature-preserving wrap |
//! | `ptr-deref-null-check-first` | ✓ | prologue-only, signature-preserving |
//! | `drop-impl-no-panic` | ✓ | method-segment swap, compile-gated |
//! | `slice-from-raw-parts-len-guard` | ✓ | expression-local additive guard |
//! | `extern-fn-reference-param-to-pointer` | ✗ | signature-changing — the C-side caller ripples (the `static-str` caller-gate class, but the caller is in ANOTHER LANGUAGE; refuse outright) |
//! | `rc-backedge-field-prefer-weak` | ✗ | field-type change — every constructor site breaks; unbounded |
//!
//! ## Phase 2 (Batch 88) — one proposer joins
//!
//! | Rule | Proposer | Why |
//! |---|---|---|
//! | `unsafe-impl-send-sync-unsound-field` | ✗ | the fix is a type-level redesign (Arc/Mutex swap) — every constructor/usage site ripples |
//! | `ffi-export-panicking-body` | ✗ | the fix needs a caller-chosen sentinel value — a semantic decision, never mechanical |
//! | `weak-hash-to-blake3` | ✗ | output TYPE changes (`[u8; N]` → `blake3::Hash`) — signature-changing |
//! | `password-hash-argon2` | ✗ | argon2 is a salted KDF with its own API — a redesign, not a swap |
//! | `constant-time-secret-compare` | ✓ | `==` → `.ct_eq(..).into()` is signature-preserving (bool → bool), strict-gated (subtle import + plain-ident operands + secret vocabulary) |
//! | `nan-unsafe-sort-comparator` (P8) | ✓ | `partial_cmp(..).unwrap_or(Equal)` → `total_cmp(..)` is order-identical on non-NaN and NaN-deterministic after; strict-gated (sort context + exact chain + `Equal`-only default + comment guard); non-float receivers fail E0599 → the `--verify` gate reverts |
//!
//! ## Conservative gates (Proposal 004 caveat 3)
//!
//! A wrong security "fix" is worse than no fix. Every proposer refuses
//! more than it fires: signature-changing rewrites are refused outright
//! (not merely caller-gated — the FFI ripple is cross-language), guards
//! already present suppress the rule, and every applied fix rides the
//! `--fix --verify` compile gate.

use syn::File;

pub mod drop_no_panic;
pub mod ffi_catch_unwind;
pub mod nan_safe_cmp;
pub mod ptr_deref_null_check;
pub mod secret_ct_eq;
pub mod slice_len_guard;

// Shared emitter substrate (re-exported like rust_perf's proposers).
pub use crate::edit_emitter::{
    apply_edits, edit, line_indent, span_range, src, RuleEdit, SourceEdit,
};

/// The `--fix`-eligible sec rules.
pub const FIXABLE_SEC: &[&str] = &[
    "ffi-extern-fn-catch-unwind",
    "ptr-deref-null-check-first",
    "drop-impl-no-panic",
    "slice-from-raw-parts-len-guard",
    "constant-time-secret-compare",
    "nan-unsafe-sort-comparator",
];

/// Collect the rule's edits for the file (empty for unknown rules).
#[must_use]
pub fn collect_edits(rule_id: &str, source: &str, file: &File) -> Vec<RuleEdit> {
    let edits = match rule_id {
        "ffi-extern-fn-catch-unwind" => ffi_catch_unwind::collect_edits(source, file),
        "ptr-deref-null-check-first" => ptr_deref_null_check::collect_edits(source, file),
        "drop-impl-no-panic" => drop_no_panic::collect_edits(source, file),
        "slice-from-raw-parts-len-guard" => slice_len_guard::collect_edits(source, file),
        "constant-time-secret-compare" => secret_ct_eq::collect_edits(source, file),
        "nan-unsafe-sort-comparator" => nan_safe_cmp::collect_edits(source, file),
        _ => Vec::new(),
    };
    edits
        .into_iter()
        .map(|e| RuleEdit {
            rule_id: rule_id.to_string(),
            edit: e,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixable_sec_are_corpus_rules() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        for id in FIXABLE_SEC {
            assert!(
                corpus.entries.iter().any(|r| r.rule_id == *id),
                "{id} must be a corpus rule"
            );
        }
    }

    #[test]
    fn every_fixable_rule_fires_on_its_fixture() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        for id in FIXABLE_SEC {
            let rule = corpus
                .entries
                .iter()
                .find(|r| r.rule_id == *id)
                .expect("rule");
            let file = syn::parse_file(&rule.buggy_code).expect("buggy parses");
            let edits = collect_edits(id, &rule.buggy_code, &file);
            assert!(
                !edits.is_empty(),
                "{id} proposer must fire on its own buggy fixture"
            );
        }
    }

    #[test]
    fn clean_spans_fire_no_lint() {
        // Real production excerpts (shaped from riir-viewbridge /
        // riir-chain patterns) — none of the four proposers may fire.
        let clean = [
            // Already-contained extern fn (the #[riir_ffi] output shape).
            "#[no_mangle]\npub extern \"C\" fn rb_shutdown(handle: u32) {\n    let _ = std::panic::catch_unwind(|| inner(handle));\n}\n",
            // Guarded pointer read.
            "pub unsafe fn peek(p: *const u32) -> Option<u32> {\n    if p.is_null() { return None; }\n    Some(*p)\n}\n",
            // Non-Drop unwrap (clippy restriction's territory, not ours).
            "pub fn cfg_of(m: &std::collections::HashMap<String, String>, k: &str) -> String {\n    m.get(k).cloned().unwrap_or_default()\n}\n",
            // Literal-len slice construction.
            "pub unsafe fn head(b: *const u8) -> &'static [u8] {\n    std::slice::from_raw_parts(b, 16)\n}\n",
        ];
        for src_text in clean {
            let file = syn::parse_file(src_text).expect("clean parses");
            for id in FIXABLE_SEC {
                assert!(
                    collect_edits(id, src_text, &file).is_empty(),
                    "{id} fired on a clean span:\n{src_text}"
                );
            }
        }
    }
}
