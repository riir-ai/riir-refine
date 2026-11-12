//! `process-global-write-under-shared-lock` — SUGGEST-ONLY detector
//! (Batch 353, the FORGE Lab std memory-safety post distill; Research 397 /
//! Plan 363).
//!
//! A PROCESS-GLOBAL is written while holding only the READ side of a
//! shared lock. Readers take the same lock believing the write cannot
//! happen concurrently (they hold "just a read guard"); two writers under
//! read guards race each other (lost updates), and a save-then-restore
//! path can leave the global dangling. Source origin: `CommandExt::exec`
//! (rust-lang/rust #156951) — "Line 242 takes only the read side of the
//! environment lock, yet line 410 writes the process-global `environ`";
//! a 2018 fix held the exclusive lock, the 2021 RW-lock migration kept
//! the read side, and "two failed `exec` calls racing each other can
//! leave `environ` pointing at a freed array that a later
//! `env::vars_os()` reads". The consumer face is named by the same post:
//! the 2024 edition marked `env::set_var` unsafe "because C functions on
//! most Unix platforms read the environment without going through Rust's
//! lock" — the environment IS the process global, and no application
//! read-lock covers what libc reads without any lock at all.
//!
//! ## Why suggest-only (never a fix)
//!
//! The fix is a locking-TIER decision (take the write side / move the
//! mutation before threads spawn / drop the global for a message queue) —
//! the right answer depends on the concurrency design, and the
//! `static`-generalization axis is invisible to a parse-only walk. NO
//! `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! ## The bounded form
//!
//! ```text
//! pub fn reload_program_env(key: &str, val: &str) {
//!     let _lock = CFG.read().unwrap();   // the shared tier
//!     std::env::set_var(key, val);       // the process-global write
//! }
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Read guard in scope (gate a)**: the fn body contains a `.read()`
//!   lock acquisition (`RwLock::read` — the shared tier).
//! - **Process-global write in the same body (gate b)**: an
//!   `env::set_var` / `env::remove_var` call (the environment IS the
//!   named process global of the origin bug; in edition 2024 both are
//!   `unsafe`, and the shape is the same wrong tier).
//! - **Suppression screen (gate c, conservative direction)**: any
//!   `.write()` lock acquisition anywhere in the body stays silent — the
//!   body demonstrably knows the exclusive tier.
//! - `#[cfg(test)]` mods are skipped; `Stmt::Macro` bodies are never
//!   descended into (house discipline); nested fns are their own scope.
//!
//! ## Honest recall limits (stated in-rule)
//!
//! - **The env family only**: the wider axis — a `static` swapped under a
//!   read guard, a global save/restore slot racing (the exec bug's exact
//!   shape) — needs alias analysis to separate the locked data from an
//!   unrelated counter; out of the bounded shape, carried as the rule's
//!   generalization.
//! - **Same-fn only**: the read guard in one fn and the env write in a
//!   helper unconnected.
//! - Whether other threads are actually running (single-threaded init is
//!   a legitimate `set_var` posture) is invisible — the rule flags the
//!   tier QUESTION, review decides.
//! - Distinct-from `ffi-callback-invoke-outside-lock` (Batch 325 — the
//!   cross-FFI callback invoked under a live guard, the deadlock axis);
//!   THIS owns the WRITE-TIER axis (a global mutated under the shared
//!   tier, the race axis). Distinct-from
//!   `credential-material-in-command-argv` (Batch 342 — the env value is
//!   the secret's TRANSPORT there; here the env WRITE TIER is the
//!   defect, whatever the value).
//! - Batch 353 / Plan 363 / Research 397 (Microsoft FORGE Lab, "Memory
//!   Safety Bugs in Rust Standard Library", 2026-11-09, CC BY 4.0 —
//!   cited as class provenance; the fixture is an original synthesis).

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{File, Item};

/// One detected process-global write under a shared lock tier.
#[derive(Debug, Clone)]
pub struct GlobalWriteFinding {
    /// The fn carrying the read guard + the env write.
    pub fn_name: String,
    /// Source line of the env write.
    pub line: usize,
    /// The flagged shape (retrieval token).
    pub kind: &'static str,
}

/// The flagged shape (single source, tested).
pub const KIND: &str = "process-global-write-under-read-lock";

/// The shared-tier vocabulary (gate a).
const READ_TIER: &str = ".read(";
/// The exclusive-tier suppression screen (gate c).
const WRITE_TIER: &str = ".write(";
/// The process-global write vocabulary (gate b) — call-path name
/// suffixes (matched with `ends_with` on the compact func-path text;
/// the parens live in the call syntax, not the path tokens).
const GLOBAL_WRITES: &[&str] = &["set_var", "remove_var"];

/// Detect process-global writes under a read lock (suggest-only; no edits
/// are produced by this module, ever).
#[must_use]
pub fn detect(_source: &str, file: &File) -> Vec<GlobalWriteFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(item, &mut findings);
    }
    findings
}

fn collect_in_item(item: &Item, findings: &mut Vec<GlobalWriteFinding>) {
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
            // Comment guard (house discipline).
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

fn scan_fn(fn_name: &str, block: &syn::Block, findings: &mut Vec<GlobalWriteFinding>) {
    let compact: String = {
        let ts = block.to_token_stream().to_string();
        ts.chars().filter(|c| !c.is_whitespace()).collect()
    };
    // Gate c first: the body knows the exclusive tier.
    if compact.contains(WRITE_TIER) {
        return;
    }
    // Gate a: a shared-tier read guard in scope.
    if !compact.contains(READ_TIER) {
        return;
    }
    // Gate b: the process-global write, with its line.
    let mut v = WriteVisitor { lines: Vec::new() };
    v.visit_block(block);
    for line in v.lines {
        findings.push(GlobalWriteFinding {
            fn_name: fn_name.to_string(),
            line,
            kind: KIND,
        });
    }
}

/// Finds the `set_var` / `remove_var` call sites.
struct WriteVisitor {
    lines: Vec<usize>,
}

impl Visit<'_> for WriteVisitor {
    fn visit_expr_call(&mut self, c: &syn::ExprCall) {
        let compact: String = {
            let ts = c.func.to_token_stream().to_string();
            ts.chars().filter(|c| !c.is_whitespace()).collect()
        };
        if GLOBAL_WRITES.iter().any(|t| compact.ends_with(t)) {
            self.lines.push(c.func.span().start().line);
        }
        syn::visit::visit_expr_call(self, c);
    }

    fn visit_expr_method_call(&mut self, m: &syn::ExprMethodCall) {
        let name = m.method.to_string();
        if name == "set_var" || name == "remove_var" {
            self.lines.push(m.method.span().start().line);
        }
        syn::visit::visit_expr_method_call(self, m);
    }

    fn visit_item(&mut self, _i: &Item) {
        // Never descend past the fn boundary.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings_of(src: &str) -> Vec<GlobalWriteFinding> {
        let file = syn::parse_file(src).expect("parses");
        detect(src, &file)
    }

    #[test]
    fn fires_on_set_var_under_read_guard() {
        let src = r#"
            static CFG: std::sync::RwLock<u32> = std::sync::RwLock::new(0);

            pub fn reload_program_env(key: &str, val: &str) {
                let _lock = CFG.read().unwrap();
                std::env::set_var(key, val);
            }
        "#;
        let f = findings_of(src);
        assert_eq!(f.len(), 1, "the shared-tier env write must fire once");
        assert_eq!(f[0].kind, KIND);
        assert_eq!(f[0].fn_name, "reload_program_env");
    }

    #[test]
    fn fires_on_remove_var_under_read_guard() {
        let src = r#"
            static CFG: std::sync::RwLock<u32> = std::sync::RwLock::new(0);

            pub fn clear_program_env(key: &str) {
                let _g = CFG.read().expect("cfg");
                std::env::remove_var(key);
            }
        "#;
        assert_eq!(findings_of(src).len(), 1);
    }

    #[test]
    fn silent_when_the_exclusive_tier_is_taken() {
        let src = r#"
            static CFG: std::sync::RwLock<u32> = std::sync::RwLock::new(0);

            pub fn reload_program_env(key: &str, val: &str) {
                let _lock = CFG.write().unwrap();
                std::env::set_var(key, val);
            }
        "#;
        // The fix posture: the mutation rides the exclusive tier.
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn silent_without_a_read_guard_in_scope() {
        let src = r#"
            pub fn bootstrap(key: &str, val: &str) {
                // Single-threaded init before any reader exists.
                std::env::set_var(key, val);
            }
        "#;
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn silent_when_no_env_write_happens() {
        let src = r#"
            static CFG: std::sync::RwLock<u32> = std::sync::RwLock::new(0);

            pub fn describe() -> String {
                let _lock = CFG.read().unwrap();
                format!("cfg={}", *_lock)
            }
        "#;
        // A read guard over a plain read: the shared tier is CORRECT here.
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn test_mods_are_skipped() {
        let src = r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn t() {
                    let _g = std::sync::RwLock::new(0u32).read();
                    std::env::set_var("K", "v");
                }
            }
        "#;
        assert!(findings_of(src).is_empty(), "cfg(test) habitat is skipped");
    }
}
