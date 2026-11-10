//! `clippy::missing_errors_doc` + `clippy::missing_panics_doc` — append the
//! missing `# Errors` / `# Panics` section to an exported fn's doc comment
//! (plan 354, Bench 141 item E first stage: the ONLY two dual-evidence
//! coverage-debt rows — the local Bench 061 fired histogram AND kornel's
//! 100K-crate study top-10).
//!
//! Same family as the landed Issue-120 `doc_markdown` precedent: both lints
//! are PEDANTIC (allow-by-default), so the `lint_level` machinery gates the
//! fixes to crates that actually enable pedantic — the self-selected audience
//! that asked for doc standards. Zero default-posture cost. Comment-local
//! insertions can never change compilation (the edit splices doc text only),
//! but the rules are VERIFY-class (see the tier note in `proposers::mod`):
//! the edit GENERATES content, and the verify lane's real-clippy re-run is
//! the only proof the section actually satisfied the lint.
//!
//! ## The section-present predicate (probe-pinned, 2026-11-09, clippy 1.99.0)
//!
//! A doc line whose trimmed form matches `^#+\s+(errors|panics)`
//! CASE-INSENSITIVELY means the section is present. Measured boundaries
//! (the `/tmp/i354_probe` scratch-crate matrix, recorded in the research note):
//!
//! | doc shape | clippy verdict |
//! |---|---|
//! | no section | FIRES |
//! | `# Errors` / `# errors` / `## Errors` / `#  Errors` (extra space) | SILENT — accepted |
//! | `#Errors` (no space after the hashes) | **FIRES — not a section** |
//!
//! `#` + REQUIRED-whitespace + the case-insensitive section word is what the
//! predicate mirrors; anything else is read as NOT-a-section and the edit
//! fires (the skip direction is "don't touch compliant docs", never "add a
//! second section").
//!
//! ## Fire scope (parse-only, clippy-aligned where parse-only can be)
//!
//! - **Exported-at-site** fn items (`pub` at the file root). syn cannot see
//!   re-exports; clippy's own `effective_visibility` is typeck territory —
//!   a private-site fire inserts into a doc clippy never reads (harmless,
//!   disclosed; prose edits are compile-invisible either way).
//! - `missing_errors_doc`: returns `Result<T, E>` where E is a CONCRETE
//!   type path (no generic params/`impl Trait`/`dyn`/refs, and not a
//!   single-short-uppercase name — `E`/`Err` refuse as type PARAMETERS).
//!   The section names E's last path segment verbatim. A generic/opaque E
//!   refuses by absence — generated prose naming a type parameter would be
//!   a lie (the bounded-fix-space law); those sites stay for the
//!   drafter/self-evolve lane.
//! - `missing_panics_doc`: the body (a `syn::visit::Visit` walk) carries
//!   ≥1 RECOGNIZED panic source — `.expect("literal")`, `.unwrap()`,
//!   `panic!("literal")`, `assert!(*, "literal")` and the eq/ne twins —
//!   and each becomes one prose line. `todo!`/`unreachable!`/
//!   `unimplemented!`, format-arg messages (`{}` placeholders) and
//!   non-literal messages REFUSE the whole edit (v1 divergence, loud —
//!   inventing prose for an unreadable message is worse than none).
//! - Insertion: a whole-doc-comment insert before an undocumented fn's
//!   signature, or a section append at the END of the fn's LAST doc line —
//!   the splice point is the end of the final doc attr's line, byte-stable
//!   (everything outside the inserted text is byte-identical).
//! - The section-present predicate REFUSES the whole edit (nothing to fix).

use crate::edit_emitter::SourceEdit;
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, Item, ItemFn, Lit};

/// The section-present predicate: any doc line whose trimmed form matches
/// `^#+\s+<word>` case-insensitively (`word` = `errors` / `panics`).
#[must_use]
pub(crate) fn lines_have_section(lines: &[&str], word: &str) -> bool {
    lines.iter().any(|line| {
        let t = line.trim_start();
        let Some(after) = t.strip_prefix('#') else {
            return false;
        };
        // Whitespace REQUIRED after the hashes (`#Errors` is not a section —
        // probe-pinned): the remainder must start with whitespace.
        let Some(non_ws_pos) = after.find(|c: char| !c.is_whitespace()) else {
            return false; // hashes only — not a section either way
        };
        if non_ws_pos == 0 {
            return false; // `#Errors` — required space missing
        }
        after[non_ws_pos..]
            .split_whitespace()
            .next()
            .is_some_and(|w| w.eq_ignore_ascii_case(word))
    })
}

/// Whether a `Result` error type is CONCRETE-ENOUGH to name in prose: a
/// path with no generic arguments, no `impl Trait`/`dyn`/refs, and not a
/// short all-uppercase parameter name (`E`, `Err`).
fn err_type_is_concrete(ty: &syn::Type) -> bool {
    let syn::Type::Path(tp) = ty else {
        return false;
    };
    if tp.qself.is_some() {
        return false;
    }
    if tp.path.segments.iter().any(|seg| !seg.arguments.is_none()) {
        return false;
    }
    let last = match tp.path.segments.last() {
        Some(s) => &s.ident,
        None => return false,
    };
    let s = last.to_string();
    !(s.len() <= 3 && s.chars().all(|c| c.is_ascii_uppercase()))
}

fn last_segment(ty: &syn::Type) -> Option<String> {
    if let syn::Type::Path(tp) = ty {
        return tp.path.segments.last().map(|s| s.ident.to_string());
    }
    None
}

struct PanicSource {
    note: String,
}

/// Walk a fn body for RECOGNIZED panic sources. `refused` flips when an
/// unrecognized class appears (format-arg messages, todo-family) — the whole
/// edit refuses then (the loud v1 divergence).
struct PanicVisitor {
    sources: Vec<PanicSource>,
    refused: bool,
}

impl Visit<'_> for PanicVisitor {
    fn visit_expr_method_call(&mut self, call: &ExprMethodCall) {
        let name = call.method.to_string();
        match name.as_str() {
            "unwrap" => self.sources.push(PanicSource {
                note: "Panics if an internal unwrap fails.".to_string(),
            }),
            "expect" => {
                if let Some(Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s), ..
                })) = call.args.first()
                {
                    let msg = s.value().trim().to_string();
                    if !msg.is_empty() && !msg.contains('{') {
                        self.sources.push(PanicSource {
                            note: format!("Panics when {msg}."),
                        });
                    } else {
                        self.refused = true;
                    }
                }
            }
            _ => {}
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_macro(&mut self, mac: &syn::Macro) {
        let name = mac.path.segments.last().map(|s| s.ident.to_string());
        match name.as_deref() {
            Some("panic" | "assert" | "assert_eq" | "assert_ne") => {
                // The message literal is the LAST plain-string argument, if
                // any. Parse the macro args as a comma list of exprs; a
                // `{}`-shaped format message or a non-literal message
                // refuses the whole edit.
                // Comma-split at top level: the message literal (if any)
                // is the LAST plain-string expr among the args. A manual
                // comma walk avoids the Parser trait's turbofish posture
                // (E0782 on `parse_multi_expr` without a receiver).
                let mut args: Vec<Expr> = Vec::new();
                {
                    let mut current = proc_macro2::TokenStream::new();
                    for tt in mac.tokens.clone() {
                        match tt {
                            proc_macro2::TokenTree::Punct(p) if p.as_char() == ',' => {
                                if let Ok(e) = syn::parse2::<Expr>(current.clone()) {
                                    args.push(e);
                                }
                                current = proc_macro2::TokenStream::new();
                                continue;
                            }
                            _ => {}
                        }
                        current.extend(std::iter::once(tt));
                    }
                    if !current.is_empty() {
                        if let Ok(e) = syn::parse2::<Expr>(current) {
                            args.push(e);
                        }
                    }
                }
                let last_lit = args.iter().rev().find_map(|e| match e {
                    Expr::Lit(syn::ExprLit { lit, .. }) => Some(lit),
                    _ => None,
                });
                match last_lit {
                    Some(Lit::Str(s)) => {
                        let msg = s.value();
                        if msg.contains('{') {
                            self.refused = true; // format-arg form
                        } else if !msg.trim().is_empty() {
                            self.sources.push(PanicSource {
                                note: format!("Panics with \"{}\".", msg.trim()),
                            });
                        }
                    }
                    _ => self.refused = true, // no literal message
                }
            }
            Some("todo" | "unimplemented" | "unreachable") => {
                self.refused = true;
            }
            _ => {}
        }
    }
}

/// The fn's doc-comment text lines (marker-stripped) + the byte offset where
/// new doc lines for this fn go (the END of the last doc attr's LINE, so an
/// append lands after it on a fresh `///` line), + the marker spelling.
/// `docs.is_empty()` = an undocumented fn: the insert point is the fn's own
/// start, and the caller emits a fresh doc comment.
fn fn_doc_state(fn_item: &ItemFn) -> (Vec<String>, usize, String, bool) {
    let mut docs: Vec<String> = Vec::new();
    let mut marker = "///".to_string();
    let mut insert_at = super::helpers::byte_range(fn_item).start;
    let mut documented = false;
    for attr in &fn_item.attrs {
        if attr.path().is_ident("doc") {
            documented = true;
            marker = if matches!(attr.style, syn::AttrStyle::Inner(_)) {
                "//!"
            } else {
                "///"
            }
            .to_string();
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    docs.push(s.value());
                }
            }
        }
    }
    if documented {
        insert_at = super::helpers::byte_range(fn_item.attrs.last().expect("documented")).end;
    }
    (docs, insert_at, marker, documented)
}

fn exported(fn_item: &ItemFn) -> bool {
    matches!(fn_item.vis, syn::Visibility::Public(_))
}

fn collect_one(source: &str, file: &File, which: &str) -> Vec<SourceEdit> {
    let _ = source; // spans are absolute against `source` via syn
    let mut edits = Vec::new();
    for item in &file.items {
        let Item::Fn(fn_item) = item else {
            continue;
        };
        if !exported(fn_item) {
            continue;
        }
        let (docs, insert_at, marker, documented) = fn_doc_state(fn_item);
        let doc_refs: Vec<&str> = docs.iter().map(String::as_str).collect();
        if lines_have_section(&doc_refs, which) {
            continue; // compliant — nothing to fix
        }
        let body = if which == "errors" {
            errors_body(fn_item)
        } else {
            panics_body(fn_item)
        };
        let Some(body) = body else { continue };
        let mut block = String::new();
        if documented {
            block.push('\n'); // terminate the last doc line's source line
        } else {
            // A fresh doc comment: name the fn first so the doc is not empty.
            let _ = std::fmt::Write::write_fmt(
                &mut block,
                format_args!("{marker} What `{}` does.\n", fn_item.sig.ident),
            );
        }
        for line in body.split('\n') {
            if line.is_empty() {
                let _ = std::fmt::Write::write_fmt(&mut block, format_args!("{marker}\n"));
            } else {
                let _ = std::fmt::Write::write_fmt(&mut block, format_args!("{marker} {line}\n"));
            }
        }
        edits.push(SourceEdit {
            start: insert_at,
            end: insert_at,
            replacement: block,
        });
    }
    edits
}

/// `# Errors` body for a fn returning `Result<T, E>` with a concrete E.
fn errors_body(fn_item: &ItemFn) -> Option<String> {
    let syn::ReturnType::Type(_, ty) = &fn_item.sig.output else {
        return None;
    };
    let syn::Type::Path(tp) = ty.as_ref() else {
        return None;
    };
    let last = tp.path.segments.last()?;
    if last.ident != "Result" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    if args.args.len() != 2 {
        return None;
    }
    let syn::GenericArgument::Type(err_ty) = args.args.last()? else {
        return None;
    };
    if !err_type_is_concrete(err_ty) {
        return None;
    }
    let e = last_segment(err_ty)?;
    Some(format!(
        "# Errors\n\nReturns an error when the operation fails (see `{e}`)."
    ))
}

/// `# Panics` body from the body's recognized panic sources; `None` = no
/// recognized sources, or an unrecognized one (the loud v1 refusal).
fn panics_body(fn_item: &ItemFn) -> Option<String> {
    let mut visitor = PanicVisitor {
        sources: Vec::new(),
        refused: false,
    };
    visitor.visit_item_fn(fn_item);
    if visitor.refused || visitor.sources.is_empty() {
        return None;
    }
    let mut lines = vec!["# Panics".to_string()];
    for s in visitor.sources {
        lines.push(String::new());
        lines.push(s.note);
    }
    Some(lines.join("\n"))
}

/// `clippy::missing_errors_doc` — the exported-Result-without-`# Errors`
/// half.
pub(crate) fn collect_errors_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    collect_one(source, file, "errors")
}

/// `clippy::missing_panics_doc` — the exported-panic-sources-without-`#
/// Panics` half.
pub(crate) fn collect_panics_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    collect_one(source, file, "panics")
}
