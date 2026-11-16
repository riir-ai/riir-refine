//! `stale-proposal-merges-against-shown-base` — SUGGEST-ONLY detector
//! (Batch 367, the opal harness transcription; Research 413 / Plan 381).
//!
//! The class: a proposal is applied as its DELTA against the base it was
//! shown, never wholesale onto current state (opal's wrapper.py `_apply`,
//! verbatim: "three-way merge at file level: apply only what the agent
//! changed relative to the files it was shown, so an answer computed
//! before a revert does not bring the reverted program back").
//!
//! The detector flags wholesale-apply SHAPE in proposal-admission code:
//! a `for` loop that writes every proposal entry onto current state with
//! no base/delta parameter in scope. Suggest-only: whether a stale base
//! can actually occur (are answers queued? can reverts interleave?) is
//! runtime knowledge a parse-only pass cannot have.
//!
//! Detector: `proposers::stale_proposal_shown_base_merge`. Fixture:
//! `fixtures/b367_fixtures.rs` F3. Batch 367 / Plan 381 / Research 413.

use std::ops::Range;

use quote::ToTokens;
use syn::visit::Visit;
use syn::{Expr, File, Item};

/// One detected wholesale-apply site.
#[derive(Debug, Clone)]
pub struct WholesaleApplyFinding {
    /// Byte range of the apply loop.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the delta-against-shown-base law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "proposal admission applies the proposal's DELTA against \
the base it was shown, never the proposal wholesale onto current state — an \
answer computed before a revert must not bring the reverted state back; for \
each name in proposal ∪ shown_base, where the proposal differs from the \
SHOWN BASE, the proposal's version lands (inserted when missing), where it \
deleted a base file the removal lands, unchanged names carry current; the \
named regime condition: a full-file rewrite of a since-reverted file \
degenerates the merge to a wholesale write and carries the stale view — \
protection holds where edits are local (Batch 367, suggest-only)";

/// The apply-loop vocabulary (fn names that move proposals onto state).
const APPLY_TOKENS: &[&str] = &[
    "apply",
    "install",
    "admit",
    "accept_proposal",
    "merge_proposal",
];

#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<WholesaleApplyFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<WholesaleApplyFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, f, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(
                        source,
                        &syn::ItemFn {
                            attrs: Vec::new(),
                            vis: syn::Visibility::Inherited,
                            sig: m.sig.clone(),
                            block: Box::new(m.block.clone()),
                        },
                        findings,
                    );
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production sites.
            if m.attrs.iter().any(|a| {
                a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
            }) {
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

fn scan_fn(source: &str, f: &syn::ItemFn, findings: &mut Vec<WholesaleApplyFinding>) {
    let fname = f.sig.ident.to_string().to_lowercase();
    // the admission vocabulary gate: an apply-shaped fn (or an fn taking a
    // proposal-shaped parameter)
    let apply_named = APPLY_TOKENS.iter().any(|t| fname.contains(t));
    let takes_proposal = f.sig.inputs.iter().any(|a| {
        let ts = a.to_token_stream().to_string().to_lowercase();
        ts.contains("proposal") || ts.contains("candidate") || ts.contains("patch")
    });
    if !apply_named && !takes_proposal {
        return;
    }
    // the delta-shaped suppression: the fn's body names the base it was
    // shown (base / shown / delta / three_way) — the law is applied
    let body_ts = f.block.to_token_stream().to_string().to_lowercase();
    if ["shown_base", "base_version", "three_way", "delta"]
        .iter()
        .any(|t| body_ts.contains(t))
    {
        return;
    }
    // the wholesale shape: a for loop writing proposal entries onto
    // current state
    let mut walk = WholesaleScan { hit: None };
    walk.visit_block(&f.block);
    if let Some(expr) = walk.hit {
        let span = syn::spanned::Spanned::span(&expr).byte_range();
        findings.push(WholesaleApplyFinding {
            site_span: span.clone(),
            site_text: source.get(span).unwrap_or("").to_string(),
            advice: ADVICE,
        });
    }
}

struct WholesaleScan {
    hit: Option<Expr>,
}

impl<'ast> Visit<'ast> for WholesaleScan {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if self.hit.is_some() {
            return;
        }
        if let Expr::ForLoop(f) = e {
            let ts = f.to_token_stream().to_string();
            let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
            let lc = compact.to_lowercase();
            let over_proposal = ["proposal", "candidate", "patch"]
                .iter()
                .any(|p| lc.contains(&format!("for{p}")) || lc.contains(&format!("in{p}")))
                // an answer-shaped aggregate's files member (`for f in &answer.files`)
                || lc.contains(".files") || lc.contains(".patches");
            let writes_current = ["current", "live", "working"]
                .iter()
                .any(|c| compact.to_lowercase().contains(c));
            if over_proposal && writes_current {
                self.hit = Some(e.clone());
            }
        }
        syn::visit::visit_expr(self, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn findings_of(src: syn::File) -> Vec<WholesaleApplyFinding> {
        let text = quote::ToTokens::to_token_stream(&src).to_string();
        detect(&text, &src)
    }

    #[test]
    fn wholesale_apply_loop_is_flagged() {
        let src: syn::File = parse_quote! {
            fn apply_proposal(current: &mut State, proposal: &[Edit]) {
                for edit in proposal {
                    current.write(edit);
                }
            }
        };
        let f = findings_of(src);
        assert_eq!(f.len(), 1, "the wholesale loop is flagged: {f:?}");
    }

    #[test]
    fn delta_apply_names_the_shown_base_and_is_suppressed() {
        let src: syn::File = parse_quote! {
            fn apply_proposal(current: &mut State, proposal: &[Edit], shown_base: &[Edit]) {
                for edit in proposal {
                    if edit.differs_from(shown_base) {
                        current.write(edit);
                    }
                }
            }
        };
        assert!(
            findings_of(src).is_empty(),
            "a delta-vs-shown-base apply is the fixed shape, suppressed"
        );
    }

    #[test]
    fn unrelated_fns_are_not_flagged() {
        let src: syn::File = parse_quote! {
            fn tick(state: &mut State) {
                state.advance();
            }
        };
        assert!(findings_of(src).is_empty());
    }
}
