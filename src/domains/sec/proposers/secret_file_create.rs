//! `secret-file-create-without-0600` — SUGGEST-ONLY detector
//! (Batch 131, the Girder distill secondary; Plan 112 T5).
//!
//! A credential-named file created world-readable: `File::create` (and
//! `fs::write`, and an `OpenOptions` `create`/`create_new` chain WITHOUT
//! `.mode(..)`) makes the file 0644-by-umask — every local user can read
//! it. The bounded repairs: harden at create time
//! (`#[cfg(unix)] OpenOptions::mode(0o600)`), or portably AFTER create
//! with `#[cfg(unix)] set_permissions(Permissions::from_mode(0o600))` —
//! the house `kat_account::write_private` form.
//!
//! ## Why suggest-only (never a fix)
//!
//! A portable mechanical rewrite is not parse-only-provable: Windows
//! `Permissions::from_mode` maps coarsely (the read-only bit), so the
//! same inserted code hardens differently per-OS — hardening semantics
//! are a per-OS decision (the `extern-fn-reference-param-to-pointer`
//! refuse-outright class, cross-platform instead of cross-language).
//! This module ships a parse-only DETECTION walk only: NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Narrow credential vocabulary** (word-boundary, split on `_`,
//!   case-insensitive — the `constant-time-secret-compare` discipline):
//!   `secret` / `password` / `passwd` / `credential` / `token` / `hmac`
//!   in the path operand OR the enclosing fn's name. The ambiguous words
//!   are deliberately EXCLUDED: `key` (public keys, key ids, keymaps),
//!   `identity` (public metadata), `private`/`pem` (public certs) — so
//!   `api_key`/`private_key` writes are MISSED (false negatives, honest).
//! - **Create-class write only**: `File::create(..)`, `fs::write(.., ..)`,
//!   or an `.open(..)` on an `OpenOptions` chain carrying
//!   `create`/`create_new`/`write`. Reads (`File::open`, `fs::read`) and
//!   appends are out — this rule owns the create half only.
//! - **No hardening anywhere in the fn**: ANY `mode(` / `set_permissions`
//!   / `from_mode` / `umask` marker suppresses the WHOLE fn — one
//!   hardened file hides a sibling hazard (conservative; the negative
//!   suite pins the compliant forms).
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped
//!   (fixture secrets live there by design); no edits are produced, so
//!   interior comments cannot be displaced.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - The narrow vocabulary misses `key`-family credential files on
//!   purpose (precision over recall — a public-key write must never
//!   fire).
//! - Hardened-elsewhere fns suppress unhardened siblings (the gate above).
//! - The shape proves a MISSING HARDENING, not a secret — a same-named
//!   fixture/sample file fires, and review decides. Suggest-only is the
//!   blast-radius control.

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, File, Item};

/// The credential vocabulary (word-boundary segments, lowercase).
const CREDENTIAL_WORDS: &[&str] = &[
    "secret",
    "password",
    "passwd",
    "credential",
    "token",
    "hmac",
];

/// The per-fn hardening markers — ANY one suppresses the whole fn.
const HARDENING_MARKERS: &[&str] = &["mode", "set_permissions", "from_mode", "umask"];

/// One detected unhardened secret-file create.
#[derive(Debug, Clone)]
pub struct SecretCreateFinding {
    /// Byte range of the create/write/open call expression.
    pub call_span: std::ops::Range<usize>,
    /// The call's source text.
    pub call_text: String,
    /// The advice line (the bounded forms + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "harden the secret file: \
`#[cfg(unix)] OpenOptions::new().mode(0o600)..` at create time, or \
portably AFTER create `#[cfg(unix)] std::fs::set_permissions(path, \
std::fs::Permissions::from_mode(0o600))` — File::create is 0644-by-umask, \
world-readable (Batch 131, suggest-only)";

/// Detect unhardened credential-file creates (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<SecretCreateFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<SecretCreateFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, f.sig.ident.to_string().as_str(), &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, m.sig.ident.to_string().as_str(), &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            if is_test_mod(m) {
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

fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

/// Word-boundary vocabulary hit over one identifier (split on `_`,
/// case-insensitive — the `constant-time-secret-compare` discipline).
/// A trailing-`s` stem ("secrets" → "secret") counts — secret FILES are
/// conventionally plural (`secrets.json`) while the vocabulary is singular.
fn is_credential_name(name: &str) -> bool {
    name.to_ascii_lowercase().split('_').any(vocab_hit)
}

/// One vocabulary segment (with the plural-stem allowance).
fn vocab_hit(segment: &str) -> bool {
    if CREDENTIAL_WORDS.contains(&segment) {
        return true;
    }
    match segment.strip_suffix('s') {
        Some(stem) => CREDENTIAL_WORDS.contains(&stem),
        None => false,
    }
}

/// Path-literal vocabulary hit: split the literal on the path/word
/// separators and match segments (a literal carries no `_`-only
/// convention — `/tmp/session_token` segments on `/` and `_`).
fn is_credential_literal(lit: &str) -> bool {
    lit.to_ascii_lowercase()
        .split(|c: char| !(c.is_ascii_alphanumeric()))
        .any(vocab_hit)
}

/// Whether any ident/path/literal in the expr tree hits the vocabulary.
fn expr_hits_vocabulary(e: &Expr) -> bool {
    struct V(bool);
    impl<'ast> Visit<'ast> for V {
        fn visit_expr(&mut self, e: &'ast Expr) {
            match e {
                Expr::Path(p) => {
                    for seg in &p.path.segments {
                        if is_credential_name(&seg.ident.to_string()) {
                            self.0 = true;
                        }
                    }
                }
                Expr::Lit(l) => {
                    if let syn::Lit::Str(s) = &l.lit {
                        if is_credential_literal(&s.value()) {
                            self.0 = true;
                        }
                    }
                }
                _ => {}
            }
            syn::visit::visit_expr(self, e);
        }
    }
    let mut v = V(false);
    v.visit_expr(e);
    v.0
}

/// Whether the fn body carries ANY hardening marker (method names on any
/// receiver + free-fn calls) — conservative whole-fn suppression.
fn fn_hardened(block: &syn::Block) -> bool {
    struct H(bool);
    impl<'ast> Visit<'ast> for H {
        fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
            if HARDENING_MARKERS.contains(&m.method.to_string().as_str()) {
                self.0 = true;
            }
            syn::visit::visit_expr_method_call(self, m);
        }
        fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
            if let Expr::Path(p) = c.func.as_ref() {
                if let Some(last) = p.path.segments.last() {
                    if HARDENING_MARKERS.contains(&last.ident.to_string().as_str()) {
                        self.0 = true;
                    }
                }
            }
            syn::visit::visit_expr_call(self, c);
        }
    }
    let mut h = H(false);
    h.visit_block(block);
    h.0
}

fn scan_fn(
    source: &str,
    fn_name: &str,
    block: &syn::Block,
    findings: &mut Vec<SecretCreateFinding>,
) {
    if fn_hardened(block) {
        return;
    }
    let fn_named = is_credential_name(fn_name);
    // Pass 1 (once per fn): OpenOptions chains — locals with a
    // create/create_new/write call and NO mode call; then their `.open()`
    // sites. Runs BEFORE the plain call walk so nested blocks cannot
    // double-report the same open.
    let create_locals = unhardened_openoptions_locals(block);
    if !create_locals.is_empty() {
        let mut opener = Opener {
            locals: create_locals,
            source,
            fn_named,
            findings,
        };
        opener.visit_block(block);
    }
    // Pass 2: the plain `File::create` / `fs::write` call walk.
    let mut w = FnWalker {
        source,
        fn_named,
        findings,
    };
    w.visit_block(block);
}

/// Locals whose `OpenOptions` builder chain carries a create-class call but
/// NO `mode` call (the fn-level hardening gate already ran).
fn unhardened_openoptions_locals(block: &syn::Block) -> Vec<String> {
    struct Scan {
        hardened: Vec<String>,
        create: Vec<String>,
    }
    impl<'ast> Visit<'ast> for Scan {
        fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
            let method = m.method.to_string();
            if let Some(id) = receiver_local(&m.receiver) {
                if method == "mode" {
                    self.hardened.push(id);
                } else if method == "create" || method == "create_new" || method == "write" {
                    self.create.push(id);
                }
            }
            syn::visit::visit_expr_method_call(self, m);
        }
    }
    fn receiver_local(mut e: &Expr) -> Option<String> {
        while let Expr::MethodCall(m) = e {
            e = &m.receiver;
        }
        if let Expr::Path(p) = e {
            if p.qself.is_none() && p.path.segments.len() == 1 {
                return Some(p.path.segments[0].ident.to_string());
            }
        }
        None
    }
    let mut scan = Scan {
        hardened: Vec::new(),
        create: Vec::new(),
    };
    scan.visit_block(block);
    scan.create
        .into_iter()
        .filter(|id| !scan.hardened.contains(id))
        .collect()
}

struct Opener<'a> {
    locals: Vec<String>,
    source: &'a str,
    fn_named: bool,
    findings: &'a mut Vec<SecretCreateFinding>,
}

impl Opener<'_> {
    fn push(&mut self, e: &Expr) {
        let span = e.span().byte_range();
        if span.end <= self.source.len() {
            self.findings.push(SecretCreateFinding {
                call_text: self.source[span.clone()].to_string(),
                call_span: span,
                advice: ADVICE,
            });
        }
    }

    fn path_or_fn_named(&self, e: &Expr) -> bool {
        expr_hits_vocabulary(e) || self.fn_named
    }
}

impl<'ast> Visit<'ast> for Opener<'_> {
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if m.method == "open" {
            if let Some(id) = receiver_local(&m.receiver) {
                if self.locals.contains(&id) {
                    if let Some(path_arg) = m.args.first() {
                        if self.path_or_fn_named(path_arg) {
                            self.push(&Expr::MethodCall(m.clone()));
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr_method_call(self, m);
    }
}

fn receiver_local(mut e: &Expr) -> Option<String> {
    while let Expr::MethodCall(m) = e {
        e = &m.receiver;
    }
    if let Expr::Path(p) = e {
        if p.qself.is_none() && p.path.segments.len() == 1 {
            return Some(p.path.segments[0].ident.to_string());
        }
    }
    None
}

struct FnWalker<'a> {
    source: &'a str,
    fn_named: bool,
    findings: &'a mut Vec<SecretCreateFinding>,
}

impl FnWalker<'_> {
    fn push(&mut self, e: &Expr) {
        let span = e.span().byte_range();
        if span.end <= self.source.len() {
            self.findings.push(SecretCreateFinding {
                call_text: self.source[span.clone()].to_string(),
                call_span: span,
                advice: ADVICE,
            });
        }
    }

    fn check_call(&mut self, c: &syn::ExprCall) {
        let Expr::Path(p) = c.func.as_ref() else {
            return;
        };
        let segs: Vec<String> = p
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let last = segs.last().map_or("", String::as_str);
        let is_file_create = last == "create" && segs.iter().any(|s| s == "File");
        let is_fs_write = last == "write" && segs.len() >= 2 && segs[segs.len() - 2] == "fs";
        if !is_file_create && !is_fs_write {
            return;
        }
        // `File::create(path)` / `fs::write(path, data)` — arg 0 is the path.
        let Some(path_arg) = c.args.first() else {
            return;
        };
        if expr_hits_vocabulary(path_arg) || self.fn_named {
            let owned = Expr::Call(c.clone());
            self.push(&owned);
        }
    }
}

impl<'ast> Visit<'ast> for FnWalker<'_> {
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        self.check_call(c);
        syn::visit::visit_expr_call(self, c);
    }
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::SecretCreateFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 131 gate — the UNHARDENED `File::create` on a credential path
    /// (original fixture, the corpus `buggy_code`) MUST fire.
    #[test]
    fn batch131_unhardened_token_create_fires() {
        let src = "pub fn save_session_token(path: &std::path::Path, token: &str) -> std::io::Result<()> {\n    \
             let mut f = std::fs::File::create(path)?;\n    \
             use std::io::Write;\n    \
             f.write_all(token.as_bytes())\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// `fs::write` on a credential-named path fires too.
    #[test]
    fn batch131_fs_write_credential_fires() {
        let src = "pub fn persist(secrets_file: &std::path::Path) -> std::io::Result<()> {\n    \
             std::fs::write(secrets_file, b\"data\")\n}\n";
        assert_eq!(findings(src).len(), 1);
    }

    /// The `OpenOptions` builder WITHOUT `.mode(..)` fires (credential path).
    #[test]
    fn batch131_openoptions_without_mode_fires() {
        let src = "pub fn fresh(token_path: &std::path::Path) -> std::io::Result<()> {\n    \
             let mut options = std::fs::OpenOptions::new();\n    \
             options.write(true).create_new(true);\n    \
             let mut f = options.open(token_path)?;\n    \
             use std::io::Write;\n    \
             f.write_all(b\"x\")\n}\n";
        assert_eq!(findings(src).len(), 1);
    }

    // ── NEGATIVE SUITE — every class must stay SILENT (zero-FP bar) ──

    /// The COMPLIANT `OpenOptions` form (`.mode(0o600)`) is silent — the
    /// hardened twin, the Girder evidence shape.
    #[test]
    fn batch131_openoptions_with_mode_is_silent() {
        let src = "pub fn fresh(token_path: &std::path::Path) -> std::io::Result<()> {\n    \
             let mut options = std::fs::OpenOptions::new();\n    \
             options.write(true).create_new(true);\n    \
             #[cfg(unix)]\n    \
             {\n        \
             use std::os::unix::fs::OpenOptionsExt;\n        \
             options.mode(0o600);\n    }\n    \
             let mut f = options.open(token_path)?;\n    \
             use std::io::Write;\n    \
             f.write_all(b\"x\")\n}\n";
        assert!(findings(src).is_empty(), "hardened form must not fire");
    }

    /// The COMPLIANT post-create `set_permissions` form (the house
    /// `kat_account::write_private` shape) is silent.
    #[test]
    fn batch131_set_permissions_after_create_is_silent() {
        let src =
            "fn write_private(path: &std::path::Path, text: &str) -> Result<(), String> {\n    \
             std::fs::write(path, text).map_err(|e| format!(\"writing: {e}\"))?;\n    \
             #[cfg(unix)]\n    \
             {\n        \
             use std::os::unix::fs::PermissionsExt;\n        \
             std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))\n            \
             .map_err(|e| format!(\"chmod: {e}\"))?;\n    }\n    \
             Ok(())\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Non-credential vocabulary never fires (`report.log`, `build.json`).
    #[test]
    fn batch131_non_credential_names_are_silent() {
        let src = "pub fn log_init(dir: &std::path::Path) -> std::io::Result<()> {\n    \
             let f = std::fs::File::create(dir.join(\"build.json\"))?;\n    \
             let _ = f;\n    \
             std::fs::write(dir.join(\"report.log\"), b\"x\")\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Word-boundary respected: `monkey_bin` carries no credential SEGMENT.
    #[test]
    fn batch131_word_boundary_respected() {
        let src = "pub fn dump(monkey_bin: &std::path::Path) -> std::io::Result<()> {\n    \
             std::fs::File::create(monkey_bin)?;\n    \
             std::io::Result::Ok(())\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Reads (`fs::read`) are out of scope — the rule owns the CREATE
    /// half only.
    #[test]
    fn batch131_reads_are_out_of_scope() {
        let src = "pub fn load(token_path: &std::path::Path) -> std::io::Result<Vec<u8>> {\n    \
             std::fs::read(token_path)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// One hardened file suppresses the whole fn — even a sibling
    /// unhardened create (conservative; stated in-rule).
    #[test]
    fn batch131_any_hardening_suppresses_the_fn() {
        let src = "pub fn persist(a: &std::path::Path, token_path: &std::path::Path) -> std::io::Result<()> {\n    \
             std::fs::File::create(token_path)?;\n    \
             #[cfg(unix)]\n    \
             {\n        \
             use std::os::unix::fs::PermissionsExt;\n        \
             std::fs::set_permissions(a, std::fs::Permissions::from_mode(0o600))?;\n    }\n    \
             std::io::Result::Ok(())\n}\n";
        assert!(
            findings(src).is_empty(),
            "whole-fn suppression is conservative"
        );
    }

    /// `#[cfg(test)]` mods are skipped (fixture secrets live there).
    #[test]
    fn batch131_test_mod_is_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             #[test]\n    fn t() {\n        \
             let _ = std::fs::File::create(\"token\");\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A String-literal path fires when the literal segments carry the
    /// vocabulary.
    #[test]
    fn batch131_credential_literal_path_fires() {
        let src = "pub fn tmp() -> std::io::Result<()> {\n    \
             std::fs::write(\"/tmp/session_token\", b\"x\")?;\n    \
             std::io::Result::Ok(())\n}\n";
        assert_eq!(findings(src).len(), 1);
    }

    /// Findings carry byte spans into the source (span integrity); the
    /// fn-name signal alone fires (the `ct_eq` fn-name precedent).
    #[test]
    fn batch131_finding_spans_slice_the_source() {
        let src = "pub fn store_credential(path: &std::path::Path) -> std::io::Result<()> {\n    \
             std::fs::File::create(path)?;\n    \
             std::io::Result::Ok(())\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "fn-name signal fires: {f:?}");
        assert_eq!(&src[f[0].call_span.clone()], "std::fs::File::create(path)");
    }
}
