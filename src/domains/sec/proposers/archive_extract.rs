//! `untrusted-archive-extract-paths` — SUGGEST-ONLY detector
//! (Batch 328, the google/oss-fuzz distill; Plan 336 / Research 366).
//!
//! A path extracted from an UNTRUSTED ARCHIVE (remote download, user
//! upload) whose member names are attacker-chosen: `ZipArchive::extract`
//! (the all-members bulk call) or a `by_index`/`by_name` loop joining
//! `entry.name()` into a destination path — a member named
//! `../../etc/cron.d/x` (or an absolute path, or a Windows prefix)
//! escapes the destination directory: arbitrary file write (the
//! zip-slip class). The bounded repair, first choice the crate's own
//! screened accessor:
//!
//! ```ignore
//! let Some(rel) = entry.enclosed_name() else { continue; };
//! let out_path = dest.join(rel);
//! ```
//!
//! or the generic `components()` walk refusing everything but
//! `Component::Normal`, or mangled extraction (sanitize separators
//! before the join).
//!
//! ## Why suggest-only (never a fix)
//!
//! The screen IS the deployment's trust decision — which archive
//! sources are legitimate (a remote download? a user upload? an
//! internal build artifact?) is policy, and the repair choice
//! (skip-and-log vs refuse-loud vs mangle) changes what the caller
//! observes. A mechanical rewrite would have to invent both. This
//! module ships a parse-only DETECTION walk only: NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Zip-surface vocabulary per fn**: the fn names the zip surface —
//!   a `ZipArchive` ident, or a `zip::` path prefix (a `zip` segment
//!   that is not the path's last segment, in an expr path OR a type
//!   path) — OR the hazard is direct: an `.extract(` method call whose
//!   receiver text mentions archive/zip vocabulary.
//! - **Two hazard shapes**: (A) an `.extract(` method call in a gated
//!   fn — the bulk call extracts EVERY member under one unguarded
//!   call; (B) a `.join(` whose argument subtree carries an
//!   entry-name expression (a `.name()` method call — the
//!   archive-member read) in a gated fn — the per-member join.
//! - **Screening markers suppress the WHOLE fn** (one screened member
//!   hides a sibling — the `secret-file-create` conservative rule):
//!   `enclosed_name` / `components` / `mangled` / `sanitize` anywhere
//!   in the fn (ident-substring match, lowercase).
//! - **Lexical `starts_with` / `strip_prefix` do NOT suppress** (by
//!   design, the join-escape rule's subtle half): a joined `..`
//!   survives both lexical checks — firing there is a TRUE finding.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped (fixture archives live there by design); no edits are
//!   produced, so interior comments cannot be displaced.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Parse-provable vocabulary fires: the zip-crate naming
//!   (`ZipArchive` / `zip::` paths, `.extract(` on an archive-typed
//!   receiver, `by_index`/`by_name` + `entry.name()`-shaped joins). A
//!   tar/7z extractor with no zip vocabulary is an honest miss (the
//!   class generalizes; the vocabulary is the zip crate's).
//! - A safe helper fn CALLED from the hazardous fn is an honest NO —
//!   same-fn screening only, cross-fn flows undetected.
//! - `entry.file_name()` joins miss (the method ident is `file_name`,
//!   not `name` — deliberate, the `read_dir` field-idiom FP guard from
//!   the join-escape rule).
//! - The shape proves a MISSING SCREEN, not an attacker — a
//!   hardened-for-internal-use extractor (an internal-only archive)
//!   fires too, and review decides. Suggest-only is the blast-radius
//!   control.
//! - Batch 328 / Plan 336 / Research 366 (google/oss-fuzz @
//!   `a380569f`, Apache-2.0 — cited as class provenance; the fixture
//!   is an original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, File, Item};

/// The per-fn screening markers — ANY one suppresses the whole fn
/// (one screened member hides a sibling; conservative).
const SUPPRESSION_MARKERS: &[&str] = &["enclosed_name", "components", "mangled", "sanitize"];

/// One detected unguarded archive-extraction path.
#[derive(Debug, Clone)]
pub struct ArchiveExtractFinding {
    /// Byte range of the hazardous call (the `.extract(` bulk call or
    /// the member `.join(`).
    pub call_span: Range<usize>,
    /// The call's source text.
    pub call_text: String,
    /// The advice line (the screened-accessor repair + the failure
    /// class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "screen archive member paths before they touch the \
filesystem: prefer the zip crate's own screened accessor \
`entry.enclosed_name()` (`Some(p) => dest.join(p), None => skip/refuse` — \
refuses `..`/absolute/Windows-prefix components), or the generic \
`.components()` walk refusing everything but `Component::Normal`, or \
mangled extraction (sanitize separators before the join) — a member named \
`../../x` escapes the destination directory (zip-slip; untrusted archive \
source; Batch 328, suggest-only)";

/// Detect unguarded untrusted-archive extraction paths (suggest-only;
/// no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<ArchiveExtractFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<ArchiveExtractFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.block, findings);
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

/// ANY screening marker anywhere in the fn suppresses the WHOLE fn —
/// one screened member hides a sibling (conservative; the negative
/// suite pins the compliant forms).
fn fn_suppressed(block: &syn::Block) -> bool {
    struct V(bool);
    impl<'ast> Visit<'ast> for V {
        fn visit_ident(&mut self, i: &'ast proc_macro2::Ident) {
            let text = i.to_string().to_ascii_lowercase();
            if SUPPRESSION_MARKERS.iter().any(|m| text.contains(m)) {
                self.0 = true;
            }
            syn::visit::visit_ident(self, i);
        }
    }
    let mut v = V(false);
    v.visit_block(block);
    v.0
}

/// The fn names the zip surface: a `ZipArchive` ident, or a `zip::`
/// path prefix (a `zip` segment that is not the path's last segment),
/// anywhere in the body's expressions or type paths.
fn fn_names_zip(block: &syn::Block) -> bool {
    struct V(bool);
    impl V {
        fn check_path(&mut self, path: &syn::Path) {
            let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            if segs.iter().any(|s| s == "ZipArchive") {
                self.0 = true;
            }
            if segs.len() >= 2 && segs[..segs.len() - 1].iter().any(|s| s == "zip") {
                self.0 = true;
            }
        }
    }
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
            self.check_path(&p.path);
            syn::visit::visit_expr_path(self, p);
        }
        fn visit_type_path(&mut self, p: &'ast syn::TypePath) {
            self.check_path(&p.path);
            syn::visit::visit_type_path(self, p);
        }
    }
    let mut v = V(false);
    v.visit_block(block);
    v.0
}

/// Whether the join-argument subtree carries an entry-name expression —
/// a `.name()` method call (the archive-member read being joined).
fn arg_names_entry(e: &Expr) -> bool {
    struct V(bool);
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
            if m.method == "name" {
                self.0 = true;
            }
            syn::visit::visit_expr_method_call(self, m);
        }
    }
    let mut v = V(false);
    v.visit_expr(e);
    v.0
}

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<ArchiveExtractFinding>) {
    if fn_suppressed(block) {
        return;
    }
    let vocab = fn_names_zip(block);
    let mut w = Walker {
        source,
        vocab,
        findings,
    };
    w.visit_block(block);
}

struct Walker<'a> {
    source: &'a str,
    vocab: bool,
    findings: &'a mut Vec<ArchiveExtractFinding>,
}

impl Walker<'_> {
    fn push(&mut self, m: &syn::ExprMethodCall) {
        let expr = Expr::MethodCall(m.clone());
        let span = expr.span().byte_range();
        if span.start < span.end && span.end <= self.source.len() {
            self.findings.push(ArchiveExtractFinding {
                call_text: self.source[span.clone()].to_string(),
                call_span: span,
                advice: ADVICE,
            });
        }
    }
}

impl<'ast> Visit<'ast> for Walker<'_> {
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        let method = m.method.to_string();
        if method == "extract" {
            // Shape A: the bulk extract. The fn-level zip vocabulary
            // gates, or the hazard is direct — the receiver itself
            // names the archive/zip surface.
            let recv = m
                .receiver
                .to_token_stream()
                .to_string()
                .to_ascii_lowercase();
            if self.vocab || recv.contains("archive") || recv.contains("zip") {
                self.push(m);
            }
        } else if method == "join" && self.vocab {
            // Shape B: the per-member join — the argument subtree
            // reads the member name (a `.name()` call) into the
            // destination path.
            if m.args.first().is_some_and(arg_names_entry) {
                self.push(m);
            }
        }
        syn::visit::visit_expr_method_call(self, m);
    }
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::ArchiveExtractFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 328 gate — the per-member JOIN shape (the corpus
    /// `buggy_code`, shape B) MUST fire at the join span.
    #[test]
    fn batch328_member_join_fires() {
        let src = "use std::path::Path;\nuse zip::ZipArchive;\n\npub fn \
unpack_bundle<R: std::io::Read + std::io::Seek>(reader: R, dest: &Path) -> \
std::io::Result<()> {\n    let mut archive = ZipArchive::new(reader)?;\n    \
for i in 0..archive.len() {\n        let mut entry = archive.by_index(i)?;\n        \
let out_path = dest.join(entry.name().to_string());\n        let mut out = \
std::fs::File::create(out_path)?;\n        std::io::copy(&mut entry, &mut out)?;\n    \
}\n    Ok(())\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].advice, super::ADVICE);
        assert!(
            f[0].call_text.contains(".join("),
            "shape B fires at the join span: {:?}",
            f[0].call_text
        );
        assert_eq!(
            &src[f[0].call_span.clone()],
            "dest.join(entry.name().to_string())"
        );
    }

    /// Shape A — the bare bulk call fires too (`archive.extract(dest)`).
    #[test]
    fn batch328_bulk_extract_fires() {
        let src = "pub fn unpack<R: std::io::Read + std::io::Seek>(reader: R, \
dest: &std::path::Path) -> std::io::Result<()> {\n    let mut archive = \
zip::ZipArchive::new(reader)?;\n    archive.extract(dest)?;\n    Ok(())\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the bulk extract fires: {f:?}");
        assert_eq!(&src[f[0].call_span.clone()], "archive.extract(dest)");
    }

    // ── NEGATIVE SUITE — every class must stay SILENT (zero-FP bar) ──

    /// The FIXED shape (`enclosed_name`) suppresses the whole fn.
    #[test]
    fn batch328_enclosed_name_is_silent() {
        let src = "use std::path::Path;\nuse zip::ZipArchive;\n\npub fn \
unpack_bundle<R: std::io::Read + std::io::Seek>(reader: R, dest: &Path) -> \
std::io::Result<()> {\n    let mut archive = ZipArchive::new(reader)?;\n    \
for i in 0..archive.len() {\n        let mut entry = archive.by_index(i)?;\n        \
let Some(rel) = entry.enclosed_name() else { continue; };\n        let out_path = \
dest.join(rel);\n        let mut out = std::fs::File::create(out_path)?;\n        \
std::io::copy(&mut entry, &mut out)?;\n    }\n    Ok(())\n}\n";
        assert!(findings(src).is_empty(), "the screened form stays silent");
    }

    /// A `components()` walk suppresses the whole fn — one screened
    /// member hides a sibling (conservative).
    #[test]
    fn batch328_components_walk_is_silent() {
        let src = "pub fn unpack<R: std::io::Read + std::io::Seek>(reader: R, \
dest: &std::path::Path) -> std::io::Result<()> {\n    let mut archive = \
zip::ZipArchive::new(reader)?;\n    let joined = dest.join(\"ok.txt\");\n    \
if joined.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {\n  \
      return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, \"escape\"));\n  \
  }\n    archive.extract(dest)?;\n    Ok(())\n}\n";
        assert!(
            findings(src).is_empty(),
            "one screened member suppresses the whole fn"
        );
    }

    /// A mangled-extraction marker (`mangled` ident) suppresses the
    /// whole fn — the sanitize/mangle repair is a bounded form.
    #[test]
    fn batch328_mangled_marker_suppresses() {
        let src = "pub fn unpack<R: std::io::Read + std::io::Seek>(reader: R, \
dest: &std::path::Path) -> std::io::Result<()> {\n    let mut archive = \
zip::ZipArchive::new(reader)?;\n    let named = mangled_member_names(&archive);\n    \
let _ = named;\n    archive.extract(dest)?;\n    Ok(())\n}\n";
        assert!(findings(src).is_empty(), "the mangled form stays silent");
    }

    /// No zip vocabulary → silent even with the hazard shape present.
    #[test]
    fn batch328_no_zip_vocabulary_is_silent() {
        let src = "pub fn stage(entry: &Entry, base: &std::path::Path) {\n    \
let p = base.join(entry.name());\n    let _ = p;\n}\n";
        assert!(findings(src).is_empty(), "the vocab gate holds");
    }

    /// A non-zip `.extract(` helper (no archive/zip receiver vocabulary,
    /// no fn vocab) is an honest NO.
    #[test]
    fn batch328_non_zip_extract_helper_is_silent() {
        let src = "pub fn pull(cache: &Cache, key: &str) -> Option<Vec<u8>> {\n    \
cache.extract(key)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (fixture archives live there).
    #[test]
    fn batch328_test_mod_is_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    fn t(reader: \
std::io::Cursor<Vec<u8>>, dest: &std::path::Path) {\n        let mut a = \
zip::ZipArchive::new(reader).unwrap();\n        a.extract(dest).unwrap();\n    \
}\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Lexical `starts_with` does NOT suppress — the join-escape rule's
    /// subtle half (a joined `..` survives the lexical check).
    #[test]
    fn batch328_lexical_check_does_not_suppress() {
        let src = "pub fn unpack<R: std::io::Read + std::io::Seek>(reader: R, \
dest: &std::path::Path) -> std::io::Result<()> {\n    let mut archive = \
zip::ZipArchive::new(reader)?;\n    let out = dest.join(member.name());\n    \
let _ = archive;\n    if out.starts_with(dest) {\n        std::fs::write(out, b\"x\")?;\n  \
  }\n    Ok(())\n}\n";
        assert_eq!(findings(src).len(), 1, "lexical checks do not screen");
    }

    /// The corpus buggy fixture fires; the fixed fixture stays silent.
    #[test]
    fn corpus_buggy_fixture_fires() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "untrusted-archive-extract-paths")
            .expect("rule present");
        let f = findings(&rule.buggy_code);
        assert!(!f.is_empty(), "the corpus buggy fixture must fire");
        let fixed = syn::parse_file(&rule.fixed_code).expect("fixed parses");
        assert!(
            detect(&rule.fixed_code, &fixed).is_empty(),
            "the fixed fixture must stay silent"
        );
    }
}
