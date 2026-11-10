//! Issue 061 T4.3 (Plan 066): the install copy has ONE home, and every
//! other document mirrors it.
//!
//! [`.proposals/003_binary_only_distribution.md`] owns the distribution
//! surface, so it owns the two one-liners. This audit fails on any URL in
//! any tracked `*.md` that ends in `install.sh` / `install.ps1` and is not
//! one of them.
//!
//! **Filed from a real defect, not from tidiness.** Proposal 005's TL;DR
//! advertised `https://cargo-refine.gist.rs/install.sh` — a vanity host that
//! does not exist — while Proposal 011 §8.1 advertised the
//! `raw.githubusercontent.com` form. Two documents, two one-liners, one of
//! them unrunnable, and nothing could catch it: install copy is a string
//! literal in prose, with no structure to disagree with. This is that
//! structure.
//!
//! Deliberately a **source-text** audit. It cannot tell whether the URL
//! resolves (it does not, yet — the dist repo has never been created; see
//! 003 T1.1), only whether every document agrees about what it will be.
//! Weaker than fetching, stronger than nothing, and it catches exactly the
//! class that occurred.

use std::path::{Path, PathBuf};

/// The canonical base every install URL must sit under — declared in
/// Proposal 003 §The canonical install one-liners.
const CANONICAL_BASE: &str = "https://raw.githubusercontent.com/riir-ai/cargo-refine/main/";

/// The two canonical URLs.
fn canonical() -> [String; 2] {
    [
        format!("{CANONICAL_BASE}install.sh"),
        format!("{CANONICAL_BASE}install.ps1"),
    ]
}

/// Directories that are not ours to audit.
///
/// `.raw/` is vendored third-party source (upstream READMEs carry their own
/// installers, and rewriting them would be a lie about what upstream says);
/// `target/` is build output; `.worktrees/` holds sibling-repo checkouts
/// (ndb's dist README advertises `gist-rs/ndb` — foreign copy, not ours to
/// audit; the walk is filesystem-based, so gitignore alone hides nothing).
const SKIP: &[&str] = &[".raw", "target", ".git", ".worktrees"];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if SKIP.contains(&name) {
            continue;
        }
        match p.is_dir() {
            // `is_dir` first: pruning here is what keeps this cheap — a
            // filter applied after the walk still descends (the
            // `find -not -path` lesson).
            true => markdown_files(&p, out),
            false if name.ends_with(".md") => out.push(p),
            false => {}
        }
    }
}

/// Every `http(s)` URL on `line` whose path ends in `install.sh`/`.ps1`.
///
/// Terminates the URL at whitespace and at the characters that end a URL in
/// this repo's markdown (backtick, pipe in a table cell, closing paren,
/// comma). A bare mention like `` `install.ps1` `` with no scheme is NOT a
/// match — Proposal 003's own diagram and the rustup citation both do that,
/// and demanding a full URL there would be noise.
fn installer_urls(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, _) in line.match_indices("http") {
        let rest = &line[i..];
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '`' | '|' | ')' | ',' | '"' | '\''))
            .unwrap_or(rest.len());
        let url = &rest[..end];
        if url.ends_with("install.sh") || url.ends_with("install.ps1") {
            out.push(url.to_string());
        }
    }
    out.shrink_to_fit();
    out
}

/// The instrument checks itself before it reports on anything else.
///
/// A regressed extractor would find no URLs and print a confident green
/// over zero of them — the `bench_doc_audit` lesson. So: the population
/// must be plausible, the extractor must recognise the shapes it depends
/// on, and it must NOT match the shapes it is supposed to ignore.
#[test]
fn the_audit_can_see_what_it_audits() {
    let mut files = Vec::new();
    markdown_files(&repo_root(), &mut files);
    assert!(
        files.len() >= 40,
        "the markdown walk found only {} file(s) — an audit over nothing is a false green",
        files.len()
    );

    // Shapes it MUST match.
    assert_eq!(
        installer_urls(
            "`curl -fsSL https://raw.githubusercontent.com/riir-ai/cargo-refine/main/install.sh | sh`"
        ),
        vec![canonical()[0].clone()],
        "the canonical macOS/Linux line must be recognised inside backticks"
    );
    assert_eq!(
        installer_urls("- Windows: `iwr -useb https://cargo-refine.gist.rs/install.ps1 | iex`"),
        vec!["https://cargo-refine.gist.rs/install.ps1".to_string()],
        "the DRIFTED form must be recognised — otherwise the audit cannot fail"
    );
    // Two on one line.
    assert_eq!(
        installer_urls("http://a/install.sh and https://b/install.ps1").len(),
        2
    );

    // Shapes it must NOT match: a bare filename, an elided path, and a URL
    // that merely lives on the same line as the word `install.ps1`.
    for ignored in [
        "| `iwr -useb …/install.ps1 \\| iex` | `scoop bucket add gist-rs https://github.com/gist-rs/scoop-bucket` |",
        "4. [rustup](https://github.com/rust-lang/rustup) — the `curl | sh` / `install.ps1` pattern.",
        "install.sh       (curl | sh)",
    ] {
        assert!(
            installer_urls(ignored).is_empty(),
            "the extractor matched a non-URL mention: {ignored}"
        );
    }
}

/// The parity assertion.
#[test]
fn every_install_url_is_the_canonical_one() {
    let mut files = Vec::new();
    markdown_files(&repo_root(), &mut files);
    let allowed = canonical();
    let mut offenders = Vec::with_capacity(files.len());
    let mut seen = 0usize;
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            for url in installer_urls(line) {
                seen += 1;
                if !allowed.contains(&url) {
                    offenders.push(format!(
                        "{}:{} → {url}",
                        f.strip_prefix(repo_root()).unwrap_or(f).display(),
                        n + 1
                    ));
                }
            }
        }
    }
    // A zero population would mean the copy vanished, not that it agrees.
    assert!(
        seen >= 2,
        "found {seen} install URL(s) in the whole repo — the onboarding one-liners are the \
         product's front door; their absence is a defect, not a pass"
    );
    assert!(
        offenders.is_empty(),
        "install copy drifted from Proposal 003 §The canonical install one-liners \
         ({CANONICAL_BASE}install.{{sh,ps1}}):\n  {}\n\nProposal 003 OWNS this copy — fix the \
         document, do not widen this test. (`cargo-refine.gist.rs` is a DEFERRED alias: promote \
         it in 003 first if it is ever actually served.)",
        offenders.join("\n  ")
    );
}

/// The surface itself: the copy must point at the PUBLIC dist repo, never
/// at riir-refine's own releases.
///
/// This is the corrected owner ask recorded in Issue 061 (*"8.1 →
/// riir-ai/cargo-refine per 003"*). `riir-ai/riir-refine` is PRIVATE, so its
/// release assets need authentication and cannot be curl'd at all — and if
/// it were ever flipped public to make them work, the entire source would
/// leak. That is the failure this pins against.
#[test]
fn the_copy_never_points_at_the_private_repos_releases() {
    let mut files = Vec::new();
    markdown_files(&repo_root(), &mut files);
    let mut offenders = Vec::with_capacity(files.len());
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            // A release-asset URL on the PRIVATE repo. The proposal is
            // allowed to DISCUSS why that does not work, so a line must
            // carry an actual URL to offend.
            let bad = line.contains("riir-ai/riir-refine/releases")
                || line.contains("riir-refine/releases/download");
            if bad {
                offenders.push(format!(
                    "{}:{}",
                    f.strip_prefix(repo_root()).unwrap_or(f).display(),
                    n + 1
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the install copy points at riir-ai/riir-refine's releases, which is PRIVATE — its \
         assets need auth (so the one-liner cannot work), and flipping the repo public to fix \
         that would leak the whole source. The surface is riir-ai/cargo-refine (Proposal 003). \
         Offending line(s):\n  {}",
        offenders.join("\n  ")
    );
    // And the canonical base must actually name that public repo, so this
    // test cannot pass by the copy pointing somewhere else entirely.
    assert!(
        CANONICAL_BASE.contains("riir-ai/cargo-refine"),
        "{CANONICAL_BASE}"
    );
}
