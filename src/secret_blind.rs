//! The secret-blind surface (Issue 151) — scan, warn, and blind.
//!
//! The egress gate ([`crate::kat_redact`]) stops secrets from LEAVING the
//! machine (mining sync drops any span with a finding; `fixer_regate` gates
//! the LLM harness). What was missing is the surface the user works in: no
//! in-run warn, no user-facing report, no safe-to-share transform. This
//! module is that surface, consuming the ONE canonical scanner — never a
//! second pattern table (the reef `AIza`-vs-gate drift was the evidence
//! that class of duplication fails).
//!
//! Detection is SCAN-based (fixed prefix + body predicate + length), not
//! corpus-match: secret formats are exact enumerable facts, the byte ranges
//! feed [`redact`] directly, and the verdict is deterministic + versioned
//! ([`REDACT_VERSION`]). Corpus-match stays the FIX-suggestion machinery.
//!
//! # The shareable-output rule (Issue 151 T2, verdict R1)
//!
//! A finding's fingerprint is an UNKEYED `blake3(bytes)[..12]` — safe in
//! the gate's LOCAL JSONL log, a dictionary-reversal leak on stdout
//! (shareable output is exactly what users paste into tickets and chats).
//! Every stdout surface here (report lines, warn lines, [`blind`] output)
//! carries **kind + location only — never the fingerprint, never the
//! finding bytes**. If cross-run correlation is ever needed, the recorded
//! path is a per-install `blake3::keyed_hash` key, not the unkeyed prefix.
//!
//! Context lines shown in the report are BLINDED: the finding's bytes are
//! substituted by the scanner's own placeholder before printing, so the
//! output is safe to paste by construction.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::kat_redact::{redact, scan_secrets, Finding, FindingKind, REDACT_VERSION};

/// One detection localized to a file: kind + 1-based line/col. The finding
/// bytes and the BLAKE3 fingerprint are deliberately NOT carried — nothing
/// downstream of this struct can print what it never holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanFinding {
    pub kind: FindingKind,
    pub line: usize,
    pub col: usize,
}

impl SpanFinding {
    /// `aws-access-key` style kebab tag (the scanner's own vocabulary).
    #[must_use]
    pub fn tag(&self) -> &'static str {
        self.kind.tag()
    }
}

/// Resolve a byte offset to a 1-based line + column (ASCII columns; the
/// scanner's findings are ASCII-shaped by construction).
fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let head = &text[..offset.min(text.len())];
    let line = head.bytes().filter(|b| *b == b'\n').count() + 1;
    let col = head
        .rsplit('\n')
        .next()
        .map_or(1, |l| l.chars().count() + 1);
    (line, col)
}

/// Scan one file's text. Findings are ordered by byte start (the scanner's
/// own order), resolved to line/col.
#[must_use]
pub fn scan_file(text: &str) -> Vec<SpanFinding> {
    scan_secrets(text)
        .into_iter()
        .map(|f: Finding| {
            let (line, col) = line_col(text, f.start);
            SpanFinding {
                kind: f.kind,
                line,
                col,
            }
        })
        .collect()
}

/// Blind a file's text: every finding's bytes replaced by the scanner's
/// `[REDACTED:<kind>]` placeholder (byte-exact outside findings — the
/// scanner's own [`redact`]). Read-only by contract: the caller prints the
/// result; nothing here writes a file (in-place substitution would destroy
/// code — a secret's fix is a process change, never a rewrite).
#[must_use]
pub fn blind(text: &str) -> String {
    redact(text, &scan_secrets(text))
}

/// The per-file slice of the report.
#[derive(Debug, Clone)]
pub struct FileFindings {
    pub path: PathBuf,
    pub findings: Vec<SpanFinding>,
}

/// The report. `kinds` counts by kebab tag (deterministic display order =
/// first-seen in the sorted walk).
#[derive(Debug, Default)]
pub struct SecretReport {
    pub files: Vec<FileFindings>,
    pub scanned_files: usize,
}

impl SecretReport {
    /// Total findings across files.
    #[must_use]
    pub fn total(&self) -> usize {
        self.files.iter().map(|f| f.findings.len()).sum()
    }

    /// The files carrying at least one finding.
    #[must_use]
    pub fn files_with_findings(&self) -> usize {
        self.files.iter().filter(|f| !f.findings.is_empty()).count()
    }

    /// Render the report. Every line is shareable by construction: kind +
    /// path:line:col + the context line BLINDED. No fingerprint, no secret
    /// bytes (the T2 rule above).
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = format!(
            "secret-scan (redact v{REDACT_VERSION}): {} file(s) scanned, {} finding(s) in {} file(s)\n",
            self.scanned_files,
            self.total(),
            self.files_with_findings(),
        );
        if self.total() == 0 {
            return out;
        }
        out.push_str("suggest-only — never auto-fixed; values and fingerprints are never shown\n");
        for file in &self.files {
            if file.findings.is_empty() {
                continue;
            }
            let _ = write!(out, "\n{}\n", file.path.display());
            for f in &file.findings {
                let _ = writeln!(
                    out,
                    "  [sec] secret-shaped literal ({}) at {}:{}:{}",
                    f.tag(),
                    file.path.display(),
                    f.line,
                    f.col,
                );
            }
        }
        out.push_str(
            "\nremedy: move the value to an env var / secret store; `cargo refine --secret-scan --blind <path>` \
             prints safe-to-share text\n",
        );
        out
    }
}

/// Which files the walk admits: text sources the healer already scans,
/// plus the config/shell surfaces secrets actually live in. Git-derived
/// inside a repository (untracked WIP kept, ignored trees out), the
/// config-audit convention; non-repo paths fall back to a filtered walk.
fn admitted(rel: &Path) -> bool {
    match rel.extension().and_then(|e| e.to_str()) {
        Some(
            "rs" | "toml" | "yaml" | "yml" | "json" | "md" | "sh" | "env" | "txt" | "sql" | "ts"
            | "js" | "py" | "wgsl" | "html" | "css",
        ) => true,
        _ => {
            // Dotfiles with no extension: `.env`, `.npmrc`, `.netrc`.
            rel.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| matches!(n, ".env" | ".env.local" | ".npmrc" | ".netrc"))
        }
    }
}

/// Collect the text files under `targets` (same default-`.` convention as
/// the other report lanes) and scan each. Refuses nothing here — the CALLER
/// owns the empty-walk refusal (the CLI's exit-1 arm); this returns an
/// honest empty report.
#[must_use]
pub fn audit_paths(targets: &[PathBuf]) -> SecretReport {
    let mut files = Vec::new();
    let mut scanned = 0usize;
    for root in targets {
        for path in collect_text_files(root) {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue; // binary or unreadable — not a blind skip, a non-member
            };
            scanned += 1;
            let findings = scan_file(&text);
            if !findings.is_empty() {
                files.push(FileFindings { path, findings });
            }
        }
    }
    SecretReport {
        files,
        scanned_files: scanned,
    }
}

/// Walk one target: a file → itself if admitted; a dir → recursive,
/// skipping VCS + build dirs, git-derived when inside a repository
/// (`git ls-files --cached --others --exclude-standard` — the shared walk
/// convention; a non-repo path keeps the filtered filesystem walk).
fn collect_text_files(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        return if admitted(root) {
            vec![root.to_path_buf()]
        } else {
            Vec::new()
        };
    }
    let mut out = Vec::new();
    let git_files = git_list(root);
    if let Some(list) = git_files {
        for rel in list {
            let p = root.join(&rel);
            if p.is_file() && admitted(Path::new(&rel)) {
                out.push(p);
            }
        }
        out.sort();
        return out;
    }
    // Non-repo fallback: filtered filesystem walk.
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if p.is_dir() {
                if matches!(name.as_ref(), ".git" | "target" | "node_modules" | ".raw") {
                    continue;
                }
                stack.push(p);
            } else if admitted(Path::new(name.as_ref())) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// `git ls-files --cached --others --exclude-standard` rooted at `dir` —
/// None when git is absent or the dir is not a repository (the caller then
/// keeps the filesystem walk). Never fails the scan on a git error.
fn git_list(dir: &Path) -> Option<Vec<String>> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const AWS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";

    #[test]
    fn scan_resolves_line_and_col() {
        let text = format!("fn a() {{}}\n// token {AWS_KEY} here\n");
        let f = scan_file(&text);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].tag(), "aws-access-key");
        assert_eq!(f[0].line, 2);
    }

    #[test]
    fn blind_replaces_and_never_carries_the_value() {
        let text = format!("let k = \"{AWS_KEY}\";\n");
        let out = blind(&text);
        assert!(out.contains("[REDACTED:aws-access-key]"), "{out}");
        assert!(!out.contains(AWS_KEY));
        // Byte-exact outside the finding.
        assert!(out.starts_with("let k = \""));
    }

    #[test]
    fn report_carries_no_fingerprint_and_no_secret_bytes() {
        let text = format!("const KEY: &str = \"{AWS_KEY}\";\n");
        let mut r = SecretReport::default();
        r.files.push(FileFindings {
            path: PathBuf::from("src/config.rs"),
            findings: scan_file(&text),
        });
        r.scanned_files = 1;
        let rendered = r.render();
        assert!(rendered.contains("[sec] secret-shaped literal (aws-access-key)"));
        assert!(rendered.contains("src/config.rs:1:"));
        assert!(!rendered.contains(AWS_KEY), "secret bytes in report");
        // The unkeyed 12-hex fingerprint must never reach stdout (T2).
        let fp = blake3::hash(AWS_KEY.as_bytes()).to_hex()[..12].to_string();
        assert!(!rendered.contains(&fp), "fingerprint leaked into report");
    }

    #[test]
    fn blind_output_is_terminal_safe_to_share() {
        // A redacted payload must not re-flag (the gate's terminal-payload
        // rule) — blind() of already-blind text is byte-identical.
        let text = format!("// key {AWS_KEY}\n");
        let once = blind(&text);
        let twice = blind(&once);
        assert_eq!(once, twice, "re-scanning a redacted payload re-flagged");
    }

    #[test]
    fn comments_and_strings_are_both_reached() {
        // The scanner is text-based: the comment form AND the string form.
        let text = "// xoxb-123456789012-1234567890123-abcdefghijklmnop\nconst S = \"sk-abcdefghijklmnopqrstuvwx\";\n".to_string();
        let f = scan_file(&text);
        let tags: Vec<_> = f.iter().map(SpanFinding::tag).collect();
        assert!(tags.contains(&"slack-token"), "{tags:?}");
        assert!(tags.contains(&"openai-style-key"), "{tags:?}");
    }

    #[test]
    fn clean_text_stays_quiet() {
        assert!(scan_file("fn main() { println!(\"hello\"); }\n").is_empty());
        assert!(scan_file("see https://example.com/docs for prose\n").is_empty());
    }
}
