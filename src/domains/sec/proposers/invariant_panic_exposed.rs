//! `unsafe-invariant-panic-exposed` — SUGGEST-ONLY detector
//! (Batch 353, the FORGE Lab std memory-safety post distill; Research 397 /
//! Plan 363).
//!
//! Unsafe code breaks an owned invariant for a moment (an uninitialized
//! buffer under an ALREADY-ARMED destructor; a length prefix not yet
//! written) and invokes code it does not control — `clone`, `drop`,
//! comparisons, closures, iterator methods — before restoring. That call
//! is a possible early exit: if it panics, unwinding runs destructors
//! over the broken state, and a drop over uninitialized memory is
//! undefined behavior. Source law, verbatim (FORGE Lab post, CC BY 4.0):
//! "treat every call to code you don't control, such as `clone`, `drop`,
//! comparisons, closures, and iterator methods, as a possible early exit.
//! Either finish the work that may panic before breaking an invariant, or
//! make the broken state safe to unwind through: hold it in
//! `ManuallyDrop`, or use a guard whose destructor restores the invariant,
//! like `Vec`'s internal `SetLenOnDrop`." Origin instance:
//! `MapWindows::clone` (rust-lang/rust #156501) — the new `Buffer`'s slots
//! are uninitialized while its destructor is armed, and the user's
//! `T::clone` runs before the slots are written.
//!
//! ## Why suggest-only (never a fix)
//!
//! Which fix applies is a per-site design decision: hold the half-built
//! state in `ManuallyDrop` until fully written (leak, not UB), use a drop
//! guard restoring the invariant (`SetLenOnDrop` shape), or reorder so
//! the panicking work finishes before the invariant breaks. NO
//! `FIXABLE_SEC` membership, NO `collect_edits` arm — the suggestion
//! surfaces through retrieval and review.
//!
//! ## The bounded form
//!
//! ```text
//! fn push_batch(&mut self, xs: &[T]) {
//!     let mut buf: [MaybeUninit<T>; N] = [const { MaybeUninit::uninit() }; N];
//!     for (slot, x) in buf.iter_mut().zip(xs) {
//!         slot.write(x.clone());   // user code INSIDE the broken-invariant window
//!     }
//!     let out = unsafe { buf.transpose().assume_init() }; // the drop obligation
//! }
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Manual-init / drop-obligation vocabulary (gate a)**: the fn body
//!   contains `MaybeUninit` / `uninit(` / `as_mut_ptr` (memory the author
//!   must initialize by hand) or `set_len(` / `from_raw_parts(` (a length
//!   prefix or view that ARMS a destructor over not-yet-valid slots —
//!   plain `as_ptr` reads are NOT gate-a: no init obligation rides a
//!   shared read).
//! - **User call inside the window (gate b)**: a `.clone()` /
//!   `.to_owned()` / `format!` call site at or after the first manual-init
//!   site and at or before the last initialization-completing site
//!   (`write(` / `assume_init` / `copy_from_nonoverlapping` /
//!   `copy_nonoverlapping` / `write_unaligned`); when no completing site
//!   exists in the fn, the window is open-ended and still counts — the
//!   invariant is left broken past the user call. Line granularity: a
//!   feeding `slot.write(x.clone())` (the user call IS the data source for
//!   the completing write) is inside the window.
//! - **Suppression screens (gate c, conservative direction)**: any
//!   `ManuallyDrop` / `SetLenOnDrop` / `scopeguard` / `catch_unwind`
//!   token in the fn stays silent — the window is already made
//!   leak-safe or the unwind contained by the author.
//! - `#[cfg(test)]` mods are skipped; `Stmt::Macro` bodies are never
//!   descended into (house discipline); nested fn bodies are skipped
//!   (per-fn scope).
//!
//! ## Honest recall limits (stated in-rule)
//!
//! - **Per-fn only, destructor invisible**: whether the type's `Drop`
//!   actually reaches the uninit region is invisible — the rule flags the
//!   hardening QUESTION, not a proven UB path. Infallible user calls
//!   (`Arc::clone`) inside the window fire as the same question and
//!   review decides — the shape is the hazard's precondition, not its
//!   proof. Same-line receiver chains (`.assume_init().clone()`) sit on
//!   the line-granularity boundary and may over-fire; review decides.
//! - Panicking user calls beyond `.clone()` / `.to_owned()` / `format!`
//!   (generic comparisons, arbitrary closures, iterator adapters) are
//!   out of the bounded shape — vocabulary-gated. A NON-init `.write(`
//!   (`io::Write`) in the same fn closes the window early — an honest
//!   false negative, review decides.
//! - Cross-fn windows (init in one fn, the user call in a helper)
//!   unconnected; nested fns unscanned.
//! - Distinct-from `drop-impl-no-panic` (the INVERSE direction: the drop
//!   glue itself panicking during unwind) — this rule owns a FOREIGN
//!   panic exposing the unsafe code's own broken invariant.
//!   Distinct-from `panic-handler-nested-catch-unwind` (the handler path
//!   guarding itself) and `ffi-extern-fn-catch-unwind` (containment AT
//!   the FFI boundary).
//! - Batch 353 / Plan 363 / Research 397 (Microsoft FORGE Lab, "Memory
//!   Safety Bugs in Rust Standard Library", 2026-11-09, CC BY 4.0 —
//!   cited as class provenance; the fixture is an original synthesis).

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{File, Item};

/// One detected broken-invariant window with a user call inside it.
#[derive(Debug, Clone)]
pub struct InvariantPanicFinding {
    /// The fn carrying the manual-init window.
    pub fn_name: String,
    /// Source line of the user call inside the window.
    pub line: usize,
    /// The flagged shape (retrieval token).
    pub kind: &'static str,
}

/// The flagged shape (single source, tested).
pub const KIND: &str = "uninit-under-armed-drop-user-call-in-window";

/// Manual-init / drop-obligation vocabulary (gate a) — method names.
const ACQUIRE_METHODS: &[&str] = &["as_mut_ptr", "set_len"];
/// Manual-init / drop-obligation vocabulary (gate a) — compact-text
/// tokens (path calls like `MaybeUninit :: uninit ( )`, `from_raw_parts (`).
const ACQUIRE_TOKENS: &[&str] = &["MaybeUninit", "uninit(", "from_raw_parts", "set_len("];
/// Initialization-completing vocabulary (gate b's window end) — method
/// names (`MaybeUninit::write`, bulk copies, unaligned writes).
const COMPLETE_METHODS: &[&str] = &[
    "write",
    "assume_init",
    "copy_from_nonoverlapping",
    "copy_nonoverlapping",
    "write_unaligned",
];
/// User-call vocabulary (gate b) — method names (`Clone::clone`,
/// `ToOwned::to_owned`).
const USER_CALL_METHODS: &[&str] = &["clone", "to_owned"];
/// User-call vocabulary (gate b) — macros.
const USER_CALL_MACROS: &[&str] = &["format"];
/// Suppression screens (gate c) — the window is already contained.
const SUPPRESS_TOKENS: &[&str] = &["ManuallyDrop", "SetLenOnDrop", "scopeguard", "catch_unwind"];

/// Detect broken-invariant windows with a user call inside (suggest-only;
/// no edits are produced by this module, ever).
#[must_use]
pub fn detect(_source: &str, file: &File) -> Vec<InvariantPanicFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(item, &mut findings);
    }
    findings
}

fn collect_in_item(item: &Item, findings: &mut Vec<InvariantPanicFinding>) {
    match item {
        Item::Fn(f) => scan_fn(&f.sig.ident.to_string(), &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(&m.sig.ident.to_string(), &m.block, findings);
                }
            }
        }
        Item::Trait(tr) => {
            for ti in &tr.items {
                if let syn::TraitItem::Fn(m) = ti {
                    if let Some(block) = &m.default {
                        scan_fn(&m.sig.ident.to_string(), block, findings);
                    }
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production unsafe code.
            if m.attrs.iter().any(|a| {
                a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
            }) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(nested, findings);
                }
            }
        }
        _ => {}
    }
}

/// One call site inside the fn body: its line and which class it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Site {
    /// Manual-init acquisition / drop-obligation arming.
    Acquire,
    /// Initialization completion (`write` / `assume_init` / bulk copy).
    Complete,
    /// User code the author does not control (`.clone()`/`.to_owned()`/`format!`).
    UserCall,
}

fn scan_fn(fn_name: &str, block: &syn::Block, findings: &mut Vec<InvariantPanicFinding>) {
    // Gate c first (the suppression screens): a contained window is not a
    // finding. Compact before matching — token streams render with spaces.
    let compact: String = {
        let ts = block.to_token_stream().to_string();
        ts.chars().filter(|c| !c.is_whitespace()).collect()
    };
    if SUPPRESS_TOKENS.iter().any(|t| compact.contains(t)) {
        return;
    }

    // Collect the per-site lines.
    let mut sites: Vec<(usize, Site)> = Vec::new();
    let mut v = SiteVisitor { sites: &mut sites };
    v.visit_block(block);

    // Gate a: manual-init / drop-obligation vocabulary, per-site or via
    // the body text (a constructor path like `Buf::uninit(n)`).
    let body_gate = ACQUIRE_TOKENS.iter().any(|t| compact.contains(t));
    let has_acquire = sites.iter().any(|(_, s)| *s == Site::Acquire) || body_gate;
    if !has_acquire {
        return;
    }
    let first_acquire = sites
        .iter()
        .find(|(_, s)| *s == Site::Acquire)
        .map(|(l, _)| *l);
    let last_complete = sites
        .iter()
        .rev()
        .find(|(_, s)| *s == Site::Complete)
        .map(|(l, _)| *l);
    let has_complete = last_complete.is_some();

    // Gate b: a user call inside the window — at/after the first acquire
    // and at/before the last completion site (line granularity; a feeding
    // `slot.write(x.clone())` shares the write's line and stays inside).
    for (line, site) in &sites {
        if *site != Site::UserCall {
            continue;
        }
        let after_acquire = match first_acquire {
            Some(a) => *line >= a,
            None => true, // acquire arrived only through the body-text gate
        };
        let before_complete = !has_complete || last_complete.is_some_and(|c| *line <= c);
        if after_acquire && before_complete {
            findings.push(InvariantPanicFinding {
                fn_name: fn_name.to_string(),
                line: *line,
                kind: KIND,
            });
            break; // one finding per fn — the window is the unit
        }
    }
}

/// Collects per-site lines for the three classes.
struct SiteVisitor<'a> {
    sites: &'a mut Vec<(usize, Site)>,
}

impl Visit<'_> for SiteVisitor<'_> {
    fn visit_expr_method_call(&mut self, m: &syn::ExprMethodCall) {
        let name = m.method.to_string();
        let line = m.method.span().start().line;
        if USER_CALL_METHODS.contains(&name.as_str()) {
            self.sites.push((line, Site::UserCall));
        } else if COMPLETE_METHODS.contains(&name.as_str()) {
            self.sites.push((line, Site::Complete));
        } else if ACQUIRE_METHODS.contains(&name.as_str()) {
            self.sites.push((line, Site::Acquire));
        }
        syn::visit::visit_expr_method_call(self, m);
    }

    fn visit_expr_macro(&mut self, mac: &syn::ExprMacro) {
        let path = mac.mac.path.to_token_stream().to_string();
        let compact: String = path.chars().filter(|c| !c.is_whitespace()).collect();
        let line = mac.mac.path.span().start().line;
        if USER_CALL_MACROS.iter().any(|t| compact.ends_with(t)) {
            self.sites.push((line, Site::UserCall));
        }
        // House discipline: never descend into macro tokens.
    }

    fn visit_expr_path(&mut self, p: &syn::ExprPath) {
        let compact: String = {
            let ts = p.to_token_stream().to_string();
            ts.chars().filter(|c| !c.is_whitespace()).collect()
        };
        let line = p.span().start().line;
        if compact.contains("MaybeUninit") {
            self.sites.push((line, Site::Acquire));
        }
        syn::visit::visit_expr_path(self, p);
    }

    fn visit_expr_call(&mut self, c: &syn::ExprCall) {
        let compact: String = {
            let ts = c.func.to_token_stream().to_string();
            ts.chars().filter(|c| !c.is_whitespace()).collect()
        };
        let line = c.func.span().start().line;
        if compact.ends_with("uninit") {
            self.sites.push((line, Site::Acquire));
        }
        syn::visit::visit_expr_call(self, c);
    }

    fn visit_item(&mut self, _i: &Item) {
        // Never descend past the fn boundary (nested fns are their own
        // scope; per-fn discipline).
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings_of(src: &str) -> Vec<InvariantPanicFinding> {
        let file = syn::parse_file(src).expect("parses");
        detect(src, &file)
    }

    #[test]
    fn fires_on_uninit_window_with_clone_inside() {
        let src = r#"
            pub struct Ring<T> { buf: [std::mem::MaybeUninit<T>; 4] }
            impl<T: Clone> Ring<T> {
                fn fill(&mut self, xs: &[T]) -> [T; 4] {
                    let mut buf: [std::mem::MaybeUninit<T>; 4] =
                        [const { std::mem::MaybeUninit::uninit() }; 4];
                    for (slot, x) in buf.iter_mut().zip(xs) {
                        slot.write(x.clone());
                    }
                    unsafe { buf.transpose().assume_init() }
                }
            }
        "#;
        let f = findings_of(src);
        assert_eq!(f.len(), 1, "the window must fire once");
        assert_eq!(f[0].kind, KIND);
        assert_eq!(f[0].fn_name, "fill");
    }

    #[test]
    fn fires_on_set_len_window_before_user_call() {
        let src = r#"
            pub fn render(rows: &[String]) -> Vec<u8> {
                let mut out: Vec<u8> = Vec::with_capacity(rows.len() * 4);
                out.set_len(rows.len() * 4);
                let banner = rows[0].clone();
                out[..banner.len()].copy_from_slice(banner.as_bytes());
                out
            }
        "#;
        // set_len arms the length-prefix invariant; the clone runs inside
        // the armed window.
        assert_eq!(findings_of(src).len(), 1);
    }

    #[test]
    fn silent_when_manuallydrop_wraps_the_window() {
        let src = r#"
            pub struct Ring<T> { buf: [std::mem::MaybeUninit<T>; 4] }
            impl<T: Clone> Ring<T> {
                fn fill(&mut self, xs: &[T]) -> [T; 4] {
                    let mut buf = std::mem::ManuallyDrop::new(
                        [const { std::mem::MaybeUninit::uninit() }; 4]
                    );
                    for (slot, x) in buf.iter_mut().zip(xs) {
                        slot.write(x.clone());
                    }
                    let out = unsafe { (*buf).transpose().assume_init() };
                    std::mem::forget(buf);
                    out
                }
            }
        "#;
        assert!(findings_of(src).is_empty(), "ManuallyDrop suppresses");
    }

    #[test]
    fn silent_when_init_completes_before_any_user_call() {
        let src = r#"
            pub fn finish(a: String, b: String) -> [String; 2] {
                let mut buf: [std::mem::MaybeUninit<String>; 2] =
                    [const { std::mem::MaybeUninit::uninit() }; 2];
                buf[0].write(a);
                buf[1].write(b);
                unsafe { buf.transpose().assume_init() }
            }
        "#;
        // No user call at all: nothing the author doesn't control runs
        // inside the window.
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn silent_when_user_call_comes_after_completion() {
        let src = r#"
            pub fn finish_then_label(a: String, b: String) -> String {
                let mut buf: [std::mem::MaybeUninit<String>; 2] =
                    [const { std::mem::MaybeUninit::uninit() }; 2];
                buf[0].write(a);
                buf[1].write(b);
                let done = unsafe { buf.transpose().assume_init() };
                format!("{}-{}", done[0], done[1])
            }
        "#;
        // The format! runs AFTER assume_init: the invariant is restored
        // before the user call.
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn silent_without_manual_init_vocabulary() {
        let src = r#"
            pub fn plain(xs: &[String]) -> Vec<String> {
                let mut out = Vec::with_capacity(xs.len());
                for x in xs {
                    out.push(x.clone());
                }
                out
            }
        "#;
        assert!(findings_of(src).is_empty(), "no gate-a vocabulary = silent");
    }

    #[test]
    fn fires_when_the_window_is_open_ended() {
        let src = r#"
            pub fn stage(templates: &[String]) -> staging::Buf<String> {
                let mut buf = staging::Buf::uninit(2);
                let first = templates[0].clone();
                buf.set(0, first);
                buf
            }
        "#;
        // `uninit(` in the body (the Buf::uninit constructor) opens a
        // window no completing site closes; the clone inside fires.
        let f = findings_of(src);
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn test_mods_are_skipped() {
        let src = r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn t() {
                    let mut buf: [std::mem::MaybeUninit<u8>; 2] =
                        [const { std::mem::MaybeUninit::uninit() }; 2];
                    let _ = "x".to_owned();
                }
            }
        "#;
        assert!(findings_of(src).is_empty(), "cfg(test) habitat is skipped");
    }
}
