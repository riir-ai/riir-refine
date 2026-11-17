//! `agent-code-static-boundary-diagnostics-never-gate` — SUGGEST-ONLY
//! detector (Batch 367, the opal harness transcription; Research 413 /
//! Plan 381).
//!
//! The class: static source diagnostics over agent-generated code,
//! framed as observations that never decide (opal's boundary.py header,
//! verbatim: "Best-effort static diagnostics for program boundary
//! crossings, never an execution policy or a sandbox" / a candidate
//! "that subsequently fails to build can still have flags; these are
//! observations, not rejections" / `check()` "Never raises" — "even a
//! broken stderr must not turn a diagnostic into a training failure").
//!
//! Why suggest-only: the scan is an OBSERVATION the consumer records —
//! what (if anything) gates on it is the consumer's policy, never this
//! detector. NO `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! Detector: `proposers::agent_code_static_boundary`. Fixture:
//! `fixtures/b367_fixtures.rs` F2. Batch 367 / Plan 381 / Research 413.

use std::ops::Range;

use syn::visit::Visit;
use syn::{Expr, File};

/// One detected boundary-crossing surface in agent-authored code.
#[derive(Debug, Clone)]
pub struct BoundaryFinding {
    /// Byte range of the expression that names the surface.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (diagnostics-never-gate posture).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "agent-generated code names a boundary-crossing surface \
(process/fs/net spawn or file I/O, dynamic evaluation, introspection, or a \
protected-object mutation): record the diagnostic BEFORE acceptance and let \
review decide — observations, never a gate; the category VOCABULARY (not the \
walker) is the bound, so indirectly-constructed names evade and a flag can be \
benign (Batch 367, suggest-only)";

/// The io/process category vocabulary (root names; the scan is name-
/// surface only — the vocabulary, not the walker, is the bound).
const IO_ROOTS: &[&str] = &[
    "process",
    "fs",
    "net",
    "std::process",
    "std::fs",
    "std::net",
];

/// The dynamic-evaluation analogs.
const DYNAMIC_CALLS: &[&str] = &["eval", "exec", "dyn_load", "load_dynamic"];

/// The introspection vocabulary.
const INTROSPECTION: &[&str] = &[
    "type_name_of",
    "frames",
    "globals_of",
    "backtrace_unchecked",
];

/// Protected-object surfaces (a receiver named like the guarded object
/// with a mutation-shaped method).
const PROTECTED_RECEIVERS: &[&str] = &["guarded", "protected", "model"];
const PROTECTED_METHODS: &[&str] = &["set_param", "register_hook", "zero_", "overwrite"];

#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<BoundaryFinding> {
    let mut findings = Vec::new();
    let mut visit = BoundaryScan {
        source,
        findings: &mut findings,
    };
    for item in &file.items {
        visit.visit_item(item);
    }
    findings
}

struct BoundaryScan<'a> {
    source: &'a str,
    findings: &'a mut Vec<BoundaryFinding>,
}

impl BoundaryScan<'_> {
    fn record(&mut self, expr: &Expr, kind: &str) {
        let span = span_of(expr);
        self.findings.push(BoundaryFinding {
            site_span: span.clone(),
            site_text: self.source.get(span).unwrap_or("").to_string(),
            advice: ADVICE,
        });
        let _ = kind;
    }
}

fn span_of(e: &Expr) -> Range<usize> {
    use syn::spanned::Spanned;
    e.span().byte_range()
}

impl<'ast> Visit<'ast> for BoundaryScan<'_> {
    fn visit_expr(&mut self, e: &'ast Expr) {
        // the GATE-POSTURE defect: a scan-result collection consumed as a
        // boolean verdict (`.is_empty()` as the fn's tail, flags.len() ==
        // 0 driving acceptance) — diagnostics deciding, the
        // observations-never-gate violation
        if let Expr::MethodCall(m) = e {
            let name = m.method.to_string();
            if name == "is_empty" {
                let recv = quote::ToTokens::to_token_stream(&m.receiver).to_string();
                let recv_lc = recv.to_lowercase();
                if ["flags", "diags", "diagnostics", "findings"]
                    .iter()
                    .any(|g| recv_lc.contains(g))
                {
                    self.record(e, "gate-posture");
                }
            }
        }
        match e {
            Expr::Path(p) => {
                let ts = quote::ToTokens::to_token_stream(&p.path).to_string();
                let compact: String = ts.chars().filter(|c: &char| *c != ' ').collect();
                let root = compact.split("::").next().unwrap_or(&compact).to_string();
                let root_full = compact.split("::").take(2).collect::<Vec<_>>().join("::");
                if IO_ROOTS
                    .iter()
                    .any(|r| compact.starts_with(r) || root_full == *r)
                    || IO_ROOTS
                        .iter()
                        .any(|r| r.starts_with(&root) && !root.is_empty())
                {
                    self.record(e, "io");
                }
            }
            Expr::Call(c) => {
                let ts = quote::ToTokens::to_token_stream(&c.func).to_string();
                let compact: String = ts.chars().filter(|c: &char| *c != ' ').collect();
                let name = compact.rsplit("::").next().unwrap_or(&compact).to_string();
                if DYNAMIC_CALLS.iter().any(|d| name.contains(d)) {
                    self.record(e, "dynamic");
                }
                // getattr-style dynamic dispatch on a protected object
                if compact.contains("getattr") || compact.contains("call_dynamic") {
                    self.record(e, "dynamic");
                }
            }
            Expr::MethodCall(m) => {
                let recv = quote::ToTokens::to_token_stream(&m.receiver).to_string();
                let recv_compact: String = recv.chars().filter(|c: &char| *c != ' ').collect();
                let method = m.method.to_string();
                let introspect = INTROSPECTION.iter().any(|i| method.contains(i))
                    || method.contains("backtrace");
                let protected = PROTECTED_RECEIVERS.iter().any(|p| recv_compact.contains(p))
                    && PROTECTED_METHODS.iter().any(|pm| method.starts_with(pm));
                if introspect {
                    self.record(e, "introspection");
                }
                if protected {
                    self.record(e, "protected-mutation");
                }
            }
            _ => {}
        }
        syn::visit::visit_expr(self, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn findings_of(src: &syn::File) -> Vec<BoundaryFinding> {
        let text = quote::ToTokens::to_token_stream(src).to_string();
        detect(&text, src)
    }

    #[test]
    fn io_surface_is_flagged() {
        let src: syn::File = parse_quote! {
            fn candidate() {
                process::spawn_child("ls");
            }
        };
        let f = findings_of(&src);
        assert_eq!(f.len(), 1, "the io/process surface is flagged: {f:?}");
    }

    #[test]
    fn benign_code_is_clean() {
        let src: syn::File = parse_quote! {
            fn candidate(x: u32) -> u32 {
                x.wrapping_add(1)
            }
        };
        assert!(findings_of(&src).is_empty(), "benign code scans clean");
    }

    #[test]
    fn protected_mutation_is_flagged() {
        let src: syn::File = parse_quote! {
            fn candidate(guarded: &Thing) {
                guarded.set_param(2.0);
            }
        };
        let f = findings_of(&src);
        assert_eq!(
            f.len(),
            1,
            "the protected-object mutation is flagged: {f:?}"
        );
    }

    #[test]
    fn indirection_evades_the_vocabulary_the_documented_bound() {
        let src: syn::File = parse_quote! {
            fn candidate(name: &str) {
                let surface = build(name);
                surface.call();
            }
        };
        assert!(
            findings_of(&src).is_empty(),
            "an indirect surface evades the category vocabulary (the honest bound)"
        );
    }

    #[test]
    fn gate_posture_flags_gating_scanner() {
        let src: syn::File = parse_quote! {
            fn review_candidate(src: &str) -> bool {
                let flags = scan_surfaces(src);
                flags.is_empty()
            }
        };
        let text = quote::ToTokens::to_token_stream(&src).to_string();
        let f = detect(&text, &src);
        assert!(
            !f.is_empty(),
            "the gate posture (flags.is_empty as the verdict) must fire: {f:?}"
        );
    }
}
