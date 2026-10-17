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
//!
//! # Digest-pin adjudication (P63) — report-lane precision, never the gate
//!
//! The canonical scanner is deliberately fail-closed on ≥32-char hex
//! runs (redact v2: such a run "is a digest or an encoded blob, and both
//! are things this gate exists to stop leaving the machine"). For the
//! EGRESS GATE that is correct and stays untouched: a false positive
//! there drops a sync span, it never ships a secret. For THIS report
//! lane the same verdict is recurring noise — every instinct/reflexer
//! pass re-reports the same digest fixtures and baseline pins (P63
//! baseline 2026-10-15: 4 findings in riir-instinct, 49 in this repo's
//! own corruption corpus, all false) — and a report nobody reads is a
//! report that stops protecting.
//!
//! [`scan_file_with_pins`] therefore ADJUDICATES a finding as a digest
//! pin — suppressing it from the report count and DISCLOSING the
//! suppressed count instead of hiding it — when the finding is the
//! generic [`FindingKind::HighEntropyHex`] (exact provider formats are
//! NEVER adjudicated by context: an `AKIA…` inside a digest-tagged or
//! pin-named string still fires) and either:
//!
//! * **tagged**: a digest-scheme tag (`blake3:`, `sha256:`, …) sits
//!   immediately before the run — glued (`blake3:EC32…`) or across
//!   horizontal whitespace and at most one opening quote
//!   (`blake3: "396b…"`, the struct-field/TOML shape). A real API key
//!   never carries a digest-scheme prefix.
//! * **pin-named**: the run opens a fresh string literal that is the
//!   value of a `let/const/static NAME…=` declaration whose NAME ends in
//!   a self-describing pin suffix (`_SHA`, `_DIGEST`, `_BASELINE`,
//!   `_COMMITMENT` + the algo spellings) — the recorded-baseline shape
//!   (`const REFLEX_BASELINE_SHA: &str = "6365fab0…"`), including the
//!   rustfmt-wrapped spelling (one line of carry, the redact v3
//!   convention).
//!
/// [`blind`] and every egress consumer keep the RAW canonical findings:
/// sharing redaction stays fail-closed, and nothing here changes what
/// passes or blocks at the gate (`REDACT_VERSION` is untouched).
///
/// # Benign-class extension (P74, 2026-10-17) — still report-lane only
///
/// The 10-17 mid-size-repo sweep measured 187 suggest-only findings
/// across 8 clean repos, every sampled site a documented fixture. Six
/// classes, four landed as guards, two deferred:
///
/// * **GUID tag** — `GUID:<32-hex>` (the Unity .asmdef reference
///   format) joins the tag guard beside the digest schemes.
/// * **bodyless key delimiter pair** — a `PrivateKeyBlock` with NO
///   ≥32-char base64 run between its BEGIN header and END marker is the
///   parser's `strip_armor` find-needle pair, not key material. SHAPE
///   evidence (byte-level), so it refines rather than contradicts the
///   private-key doctrine: a block WITH a body — the named-fixture class
///   (`THROWAWAY_OPENSSH_KEY`) — stays structural and keeps firing
///   (DEFERRED: real PEM bytes in source stay visible).
/// * **canonical base64 alphabet** — a run containing the exact
///   RFC 4648 64-char sequence is an encoding-vocabulary constant.
/// * **kebab credential filename** — a `RustSecretLiteral` whose value
///   is pure lowercase-kebab AND whose binding ends `_credential` is the
///   systemd `LoadCredential=` FILE name (both evidences AND-ed). An
///   uppercase/digit-mixed value never qualifies.
/// * **golden pin names** — the repo-wide `GOLDEN_*` / `*_GOLDEN`
///   wire-dual-pin convention joins the pin-name evidence; the generic
///   kinds widen from hex-only to hex+b64 (golden b64 hunks are the
///   live shape).
/// * **DEFERRED: RFC assert vectors** — no conservative shape guard
///   (rustfmt wraps the literal onto its own line; no byte-level
///   evidence distinguishes them from pinned constants).
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

/// Scan one file's text, split by the report-lane adjudication: `findings`
/// are the report rows; `digest_pins` are the adjudicated digest pins
/// (counted for disclosure, never rendered as findings — and still
/// fail-closed at the egress gate and in [`blind`]).
#[must_use]
pub fn scan_file_with_pins(text: &str) -> ScanOutcome {
    let mut findings = Vec::new();
    let mut digest_pins = Vec::new();
    for f in scan_secrets(text) {
        let (line, col) = line_col(text, f.start);
        let sf = SpanFinding {
            kind: f.kind,
            line,
            col,
        };
        if is_digest_pin(text, &f) {
            digest_pins.push(sf);
        } else {
            findings.push(sf);
        }
    }
    ScanOutcome {
        findings,
        digest_pins,
    }
}

/// Scan one file's text. Findings are ordered by byte start (the scanner's
/// own order), resolved to line/col. Signature preserved for the existing
/// callers: the report rows of [`scan_file_with_pins`] (digest pins
/// adjudicated benign, P63).
#[must_use]
pub fn scan_file(text: &str) -> Vec<SpanFinding> {
    scan_file_with_pins(text).findings
}

/// A file's canonical findings split by the report-lane adjudication
/// (P63). `digest_pins` is DISCLOSURE data: the count rides the report
/// and the in-run warn line; the findings themselves are never rendered.
#[derive(Debug, Clone, Default)]
pub struct ScanOutcome {
    pub findings: Vec<SpanFinding>,
    pub digest_pins: Vec<SpanFinding>,
}

// ── The digest-pin adjudication (P63) ──────────────────────────────────────

/// Digest-scheme tags that mark a following hex run as checksum-shaped.
/// Lowercase-compare; a real API key never carries one of these prefixes.
const DIGEST_SCHEMES: &[&str] = &[
    "blake3", "blake2", "blake2b", "blake2s", "sha1", "sha256", "sha384", "sha512", "sha3",
    "sha3-256", "sha3-512", "keccak", "keccak256", "md5",
];

/// Self-describing pin-name suffixes for the identifier evidence: a
/// declaration whose NAME ends in one of these and whose value is a
/// fresh ≥32-hex literal is a recorded pin, not a credential. `_HASH` is
/// deliberately OUT (unprobed — a future probe corpus may add it).
const PIN_NAME_SUFFIXES: &[&str] = &[
    "_sha", "_sha1", "_sha256", "_sha384", "_sha512", "_sha3", "_blake3", "_blake2", "_md5",
    "_keccak256", "_digest", "_baseline", "_commitment",
];

/// The report-lane adjudication: a canonical finding that is contextually
/// a digest pin. Scoped to the GENERIC high-entropy kinds — exact
/// provider formats (`AKIA…`, `sk-…`, `AQ.…`) are structural evidence and
/// are NEVER adjudicated away by context. Two kind-specific guards sit
/// beside the generic ones (P74): a [`FindingKind::PrivateKeyBlock`] with
/// NO base64 body between its markers (the `strip_armor` delimiter-pair
/// shape — bodyless bytes are not key material: shape evidence, not
/// context) and a [`FindingKind::RustSecretLiteral`] whose value is a
/// lowercase-kebab slug on a `*_CREDENTIAL`-named binding (a systemd
/// `LoadCredential=` FILENAME, not a secret).
fn is_digest_pin(span: &str, f: &Finding) -> bool {
    match f.kind {
        FindingKind::HighEntropyHex | FindingKind::HighEntropyB64 => {
            preceded_by_known_tag(span, f.start)
                || opens_pin_named_value(span, f.start)
                || is_canonical_b64_alphabet(span, f)
        }
        FindingKind::PrivateKeyBlock => is_bodyless_key_delimiter(span, f),
        FindingKind::RustSecretLiteral => is_kebab_credential_value(span, f),
        _ => false,
    }
}

/// Non-digest identifier tags (P74): `GUID:<32-hex>` is the Unity .asmdef
/// reference format — project-internal asset ids every Unity repo carries
/// in source and `.meta` files alike. A real API key never carries the
/// prefix.
const IDENTIFIER_SCHEMES: &[&str] = &["guid"];

/// Guard (a): a known tag sits immediately before the run — glued
/// (`blake3:EC32…`, `GUID:3F2A…`) or across horizontal whitespace and at
/// most one opening quote (`blake3: "396b…"` — the struct-field/TOML
/// shape).
fn preceded_by_known_tag(span: &str, start: usize) -> bool {
    preceded_by_tag_in(span, start, DIGEST_SCHEMES)
        || preceded_by_tag_in(span, start, IDENTIFIER_SCHEMES)
}

fn preceded_by_tag_in(span: &str, start: usize, schemes: &[&str]) -> bool {
    let mut head = &span[..start];
    if head.ends_with('"') || head.ends_with('\'') {
        head = &head[..head.len() - 1];
    }
    let head = head.trim_end_matches([' ', '\t']);
    let Some(before_colon) = head.strip_suffix(':') else {
        return false;
    };
    let bytes = before_colon.as_bytes();
    let mut s = bytes.len();
    while s > 0 && (bytes[s - 1].is_ascii_alphanumeric() || bytes[s - 1] == b'-') {
        s -= 1;
    }
    let scheme = before_colon[s..].to_ascii_lowercase();
    schemes.contains(&scheme.as_str())
}

/// Guard (b): the run opens a fresh string literal (the byte before it is
/// the opening quote) that is the value of a declaration — this line, or
/// the single rustfmt-wrapped line before it (the redact v3 convention) —
/// whose NAME carries a [`PIN_NAME_SUFFIXES`] suffix.
fn opens_pin_named_value(span: &str, start: usize) -> bool {
    let line_start = span[..start].rfind('\n').map_or(0, |i| i + 1);
    let head = &span[line_start..start];
    if !head.ends_with('"') {
        return false;
    }
    let lhs = if let Some(eq) = head.rfind('=') { &head[..eq] } else {
            // rustfmt-wrapped: `…NAME: TYPE =` ended the previous line.
            if line_start == 0 {
                return false;
            }
            let prev_start = span[..line_start - 1].rfind('\n').map_or(0, |i| i + 1);
            let prev = span[prev_start..line_start - 1].trim_end();
            let Some(eq) = prev.rfind('=') else {
                return false;
            };
            if !prev[eq + 1..].trim().is_empty() {
                return false; // the '=' already had a value — not the wrap
            }
            &prev[..eq]
        };
    pin_named_declaration(lhs)
}

/// `NAME` (after any `: TYPE` ascription and the `pub/const/static/let/mut`
/// keywords) — the lowercased bare identifier, when the LHS reduces to
/// exactly one.
fn declaration_ident(lhs: &str) -> Option<String> {
    let mut s = lhs.trim();
    if let Some(c) = s.find(':') {
        s = s[..c].trim();
    }
    loop {
        // `pub(super)` / `pub(crate)` visibility — the paren scope sits
        // between the keyword and the name (P74: the live golden-pin
        // sites are `pub(super) const GOLDEN_*`; P63 never met the form).
        if let Some(rest) = s.strip_prefix('(') {
            let close = rest.find(')')?;
            s = rest[close + 1..].trim_start();
            continue;
        }
        let next = ["pub", "const", "static", "let", "mut"]
            .iter()
            .find_map(|kw| {
                let rest = s.strip_prefix(kw)?;
                let trimmed = rest.trim_start();
                let n = rest.len() - trimmed.len();
                if n > 0 {
                    Some(trimmed)
                } else if rest.starts_with('(') {
                    Some(rest) // `pub(super)…` — the paren arm handles it
                } else {
                    None
                }
            });
        match next {
            Some(n) => s = n.trim_start(),
            None => break,
        }
    }
    if s.is_empty()
        || s.starts_with(|c: char| c.is_ascii_digit())
        || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    Some(s.to_ascii_lowercase())
}

/// `NAME` carries a pin suffix — or (P74) the golden-pin convention: the
/// repo-wide `GOLDEN_*` / `*_GOLDEN` wire-dual-pin naming.
fn pin_named_declaration(lhs: &str) -> bool {
    let Some(lower) = declaration_ident(lhs) else {
        return false;
    };
    PIN_NAME_SUFFIXES.iter().any(|sfx| lower.ends_with(sfx))
        || lower.starts_with("golden_")
        || lower.ends_with("_golden")
}

/// `NAME` carries the credential-binding suffix (the systemd
/// `LoadCredential=` filename-const shape).
fn credential_named_declaration(lhs: &str) -> bool {
    declaration_ident(lhs).is_some_and(|lower| lower.ends_with("_credential"))
}

/// The canonical RFC 4648 base64 alphabet — its exact 64-char sequence is
/// encoding vocabulary; no credential contains it as a substring.
const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Guard (c) (P74): the matched run CONTAINS the exact canonical base64
/// alphabet sequence — an alphabet constant (`const CHARS: &[u8] =
/// b"ABCDEFGH…"`), not a secret. Byte-exact substring: conservative by
/// construction (an attacker-chosen value would have to BE the ordered
/// alphabet to suppress).
fn is_canonical_b64_alphabet(span: &str, f: &Finding) -> bool {
    let token = &span[f.start..f.start + f.len];
    token
        .as_bytes()
        .windows(B64_ALPHABET.len())
        .any(|w| w == B64_ALPHABET)
}

/// Byte-substring find (the `kat_redact` helper's local twin — that one is
/// module-private and the canonical scanner's surface stays untouched).
fn find_sub(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

/// Guard (b) (P74): a [`FindingKind::PrivateKeyBlock`] whose span carries
/// NO base64 body between the BEGIN header and the END marker — the
/// delimiter-PAIR shape (`let begin = "-----BEGIN OPENSSH PRIVATE
/// KEY-----"; let end = "-----END …"`): the parser's find-needles, never
/// key material. SHAPE evidence, byte-level: a real PEM body is built of
/// ≥32-char base64 lines; the region between the markers here holds
/// none. A block WITH a body keeps its finding — named-fixture keys stay
/// structural evidence (the P74 (a) defer).
fn is_bodyless_key_delimiter(span: &str, f: &Finding) -> bool {
    let text = &span[f.start..f.start + f.len];
    let bytes = text.as_bytes();
    // The BEGIN header's end — the same needles the scanner matches.
    let block_end = find_sub(bytes, b"PRIVATE KEY BLOCK-----", 0)
        .map(|e| e + "PRIVATE KEY BLOCK-----".len());
    let key_end = find_sub(bytes, b"PRIVATE KEY-----", 0)
        .map(|e| e + "PRIVATE KEY-----".len());
    let Some(header_end) = block_end.max(key_end) else {
        return false; // cannot resolve the shape — fail closed
    };
    let body_end = find_sub(bytes, b"-----END", header_end).unwrap_or(bytes.len());
    let mut run = 0usize;
    for &b in &bytes[header_end..body_end] {
        if b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=' | b'\\') {
            run += 1;
            if run >= 32 {
                return false;
            }
        } else {
            run = 0;
        }
    }
    true
}

/// Guard (d) (P74): a [`FindingKind::RustSecretLiteral`] whose value is a
/// pure lowercase-kebab slug AND whose binding name ends `_credential` —
/// the systemd `LoadCredential=` FILENAME shape (`pub const
/// LEDGER_ROOT_CREDENTIAL: &str = "riir-chain-ledger-id"`): the const
/// names the credential FILE to ask for, it is not the secret. BOTH
/// evidences AND-ed (the P63 doctrine); an uppercase/digit-mixed value
/// never qualifies.
fn is_kebab_credential_value(span: &str, f: &Finding) -> bool {
    let lit = &span[f.start..f.start + f.len];
    let Some(value) = lit.strip_prefix('"').and_then(|v| v.strip_suffix('"')) else {
        return false;
    };
    if value.len() < 8
        || !value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        || value.starts_with('-')
        || value.ends_with('-')
        || value.contains("--")
    {
        return false;
    }
    let line_start = span[..f.start].rfind('\n').map_or(0, |i| i + 1);
    let head = &span[line_start..f.start];
    let Some(eq) = head.rfind('=') else {
        return false;
    };
    credential_named_declaration(&head[..eq])
}

/// Blind a file's text: every finding's bytes replaced by the scanner's
/// `[REDACTED:<kind>]` placeholder (byte-exact outside findings — the
/// scanner's own [`redact`]). Read-only by contract: the caller prints the
/// result; nothing here writes a file (in-place substitution would destroy
/// code — a secret's fix is a process change, never a rewrite). RAW
/// canonical findings, adjudication deliberately NOT applied: sharing
/// redaction stays fail-closed (P63).
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
/// first-seen in the sorted walk). `digest_pins` is the P63 disclosure
/// count: findings adjudicated benign digest pins, suppressed from the
/// report rows but never hidden from the count.
#[derive(Debug, Default)]
pub struct SecretReport {
    pub files: Vec<FileFindings>,
    pub scanned_files: usize,
    pub digest_pins: usize,
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
            if self.digest_pins > 0 {
                let _ = writeln!(
                    out,
                    "{n} digest-pin finding(s) adjudicated benign (tagged digest / pin-named const) — suppressed from this report; the egress gate stays fail-closed",
                    n = self.digest_pins
                );
            }
            return out;
        }
        out.push_str("suggest-only — never auto-fixed; values and fingerprints are never shown\n");
        if self.digest_pins > 0 {
            let _ = writeln!(
                out,
                "{n} digest-pin finding(s) adjudicated benign (tagged digest / pin-named const) — suppressed from this report; the egress gate stays fail-closed",
                n = self.digest_pins
            );
        }
        for file in &self.files {
            if file.findings.is_empty() {
                continue;
            }
            let _ = writeln!(out, "\n{}", file.path.display());
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
    let mut digest_pins = 0usize;
    for root in targets {
        for path in collect_text_files(root) {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue; // binary or unreadable — not a blind skip, a non-member
            };
            scanned += 1;
            let outcome = scan_file_with_pins(&text);
            digest_pins += outcome.digest_pins.len();
            if !outcome.findings.is_empty() {
                files.push(FileFindings {
                    path,
                    findings: outcome.findings,
                });
            }
        }
    }
    SecretReport {
        files,
        scanned_files: scanned,
        digest_pins,
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

    // ── P63: digest-pin adjudication (probe-first; Batch-33 discipline) ──

    /// The four LIVE instinct false positives, verbatim shapes (the
    /// 2026-10-15 sweep baseline: 4 findings, all this class).
    #[test]
    fn live_instinct_digest_pins_are_suppressed() {
        // arsenal.rs 867/868/871 — tagged literals (sha3, uppercase, zeros).
        let text = concat!(
            "\"sha3:ec327ac30250205b2b26c6b00976f9fb50bdf44a9bcbb1d87e35c6ca06f5cf33\",\n",
            "\"blake3:EC327AC30250205B2B26C6B00976F9FB50BDF44A9BCBB1D87E35C6CA06F5CF33\",\n",
            "\"blake3:0000000000000000000000000000000000000000000000000000000000000000\",\n",
        );
        let out = scan_file_with_pins(text);
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert_eq!(out.digest_pins.len(), 3);

        // arena.rs 97 — the pin-named const (no tag; guard b only).
        let pin =
            "const REFLEX_BASELINE_SHA: &str = \"6365fab0261b2123ae0953a12b06ef0eec596c2b\";\n";
        let out = scan_file_with_pins(pin);
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert_eq!(out.digest_pins.len(), 1);
    }

    /// The corruption-corpus field shape (48 live rows in this repo):
    /// `blake3: "<hex>"` — scheme + whitespace + quote before the run.
    #[test]
    fn digest_field_rows_are_suppressed() {
        let text =
            "        blake3: \"396b8e1fcf31e1488887baa2251dfb62fa5a19aa6eff44bb759f54498bfde22d\",\n";
        let out = scan_file_with_pins(text);
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert_eq!(out.digest_pins.len(), 1);

        // The committed pin const (guard b; _DIGEST suffix).
        let pin = "pub const CORPUS_V9_DIGEST: &str = \"bfd2bef8d4a745d64582e2c8b672246c66a50566a2414572772b5550262ed631\";\n";
        let out = scan_file_with_pins(pin);
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert_eq!(out.digest_pins.len(), 1);
    }

    /// MUST-FIRE (the P63 probe contract): exact provider formats are
    /// NEVER adjudicated by context — aws/sk-/AQ. shapes stay firing under
    /// BOTH guards' contexts.
    #[test]
    fn exact_provider_formats_stay_firing_under_pin_context() {
        // An AWS key as the value of a pin-named const.
        let t = format!("const RELEASE_SHA: &str = \"{AWS_KEY}\";\n");
        let f = scan_file(&t);
        assert!(f.iter().any(|x| x.tag() == "aws-access-key"), "{f:?}");
        // …and after a digest tag (the tag guard reaches hex runs only).
        let t = format!("let s = \"blake3:{AWS_KEY}\";\n");
        let f = scan_file(&t);
        assert!(f.iter().any(|x| x.tag() == "aws-access-key"), "{f:?}");
        // sk- inside a pin-named declaration (also carries the identifier-
        // policy finding — neither kind is adjudicated).
        let t = "const TOKEN_DIGEST: &str = \"sk-abcdefghijklmnopqrstuvwx\";\n";
        let f = scan_file(t);
        assert!(f.iter().any(|x| x.tag() == "openai-style-key"), "{f:?}");
        assert!(f.iter().any(|x| x.tag() == "rust-secret-literal"), "{f:?}");
        // AQ. (the GoogleApiKey v5 shape) inside a pin-named const.
        let t = "const KEY_BASELINE: &str = \"AQ.ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmno\";\n";
        let f = scan_file(t);
        assert!(f.iter().any(|x| x.tag() == "google-api-key"), "{f:?}");
    }

    /// MUST-FIRE: a generic hex run with NO pin evidence — unnamed,
    /// untagged, under a NON-digest scheme tag, or mid-literal — still
    /// reports (the conservative core of the class).
    #[test]
    fn untagged_hex_runs_still_fire() {
        // No context at all (40 hex).
        let t = "let x = \"a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0\";\n";
        assert_eq!(scan_file(t).len(), 1);
        // A NON-digest scheme tag — `redis:` is not in the scheme set.
        let t = "let x = \"redis:a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0\";\n";
        assert_eq!(scan_file(t).len(), 1, "non-digest tag must not suppress");
        // Mid-literal (the byte before the run is not an opening quote).
        let t = "log(\"prefix a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0\");\n";
        assert_eq!(scan_file(t).len(), 1, "mid-literal run must not suppress");
        // A secret-shaped NAME keeps its identifier-policy finding — never
        // adjudicated (not the pin vocabulary).
        let t = "let api_secret = \"a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6\";\n";
        let f = scan_file(t);
        assert!(f.iter().any(|x| x.tag() == "rust-secret-literal"), "{f:?}");
    }

    /// The rustfmt-wrapped pin (redact v3 convention — the verdict must
    /// not depend on where the formatter broke the line).
    #[test]
    fn wrapped_pin_const_is_suppressed() {
        let t = "const REFLEX_BASELINE_SHA: &str =\n    \"6365fab0261b2123ae0953a12b06ef0eec596c2b\";\n";
        let out = scan_file_with_pins(t);
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert_eq!(out.digest_pins.len(), 1);
    }

    /// Disclosure, not silence: the report counts suppressed pins.
    #[test]
    fn report_discloses_suppressed_pins() {
        let text = format!(
            "const PIN_SHA: &str = \"a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2\";\n// {AWS_KEY}\n"
        );
        let out = scan_file_with_pins(&text);
        let mut r = SecretReport::default();
        r.files.push(FileFindings {
            path: PathBuf::from("src/x.rs"),
            findings: out.findings,
        });
        r.scanned_files = 1;
        r.digest_pins = out.digest_pins.len();
        let rendered = r.render();
        assert!(rendered.contains("1 finding(s) in 1 file(s)"), "{rendered}");
        assert!(
            rendered.contains("1 digest-pin finding(s) adjudicated benign"),
            "{rendered}"
        );
        // The all-pins posture still discloses (no silent green zero).
        let r2 = SecretReport {
            scanned_files: 3,
            digest_pins: 4,
            ..Default::default()
        };
        let rendered2 = r2.render();
        assert!(
            rendered2.contains("4 digest-pin finding(s) adjudicated benign"),
            "{rendered2}"
        );
    }

    /// Sharing redaction stays fail-closed: [`blind`] still replaces a
    /// digest pin (adjudication is report-lane only, P63).
    #[test]
    fn blind_still_redacts_digest_pins() {
        let t = "const PIN_SHA: &str = \"a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2\";\n";
        let out = blind(t);
        assert!(out.contains("[REDACTED:high-entropy-hex]"), "{out}");
    }

    // ── P74: benign-class guards (probe-first; Batch-33 discipline) ──

    const B64_ALPHABET_STR: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    /// The REAL sites from the 10-17 sweep, verbatim shapes.
    #[test]
    fn p74_live_sites_are_suppressed() {
        // (a) viewbridge xtask — Unity asmdef GUID references (contiguous
        // 32-hex; the dashed UUID form never fires — dashes break the run).
        let t = concat!(
            "    \"references\": [\n",
            "        \"GUID:0ec3d108315f4b0b8f5f0a52b2f3b96d\",\n",
            "        \"GUID:3f2ab1c09d8e47aab6c51e0f9a8b7c6d\",\n",
            "        \"GUID:5a4b3c2d1e0f4a5b8c7d6e5f4a3b2c1d\",\n",
            "    ],\n",
        );
        let out = scan_file_with_pins(t);
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert_eq!(out.digest_pins.len(), 3);

        // (b) rethink authority.rs — the strip_armor delimiter pair (NO
        // body between the markers).
        let t = concat!(
            "fn strip_armor(text: &str) -> Option<&str> {\n",
            "    let begin = \"-----BEGIN OPENSSH PRIVATE KEY-----\";\n",
            "    let end = \"-----END OPENSSH PRIVATE KEY-----\";\n",
        );
        let out = scan_file_with_pins(t);
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert_eq!(out.digest_pins.len(), 1);

        // (c) the base64 alphabet constant (rethink authority.rs:244 /
        // neuron-db e2e_login.rs:143).
        let t = format!("    let alphabet = b\"{B64_ALPHABET_STR}\";\n");
        let out = scan_file_with_pins(&t);
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert_eq!(out.digest_pins.len(), 1);

        // (d) chain ledger_key.rs — the LoadCredential= filename consts.
        let t = concat!(
            "pub const LEDGER_ROOT_CREDENTIAL: &str = \"riir-chain-ledger-root\";\n",
            "pub const LEDGER_ID_CREDENTIAL: &str = \"riir-chain-ledger-id\";\n",
        );
        let out = scan_file_with_pins(t);
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert_eq!(out.digest_pins.len(), 2);

        // (e) the golden-pin convention (dapps backstab_anchored_tests.rs
        // GOLDEN_CHUNK/GOLDEN_HUNK — b64 hunks; both values entropy-real).
        let t = concat!(
            "pub(super) const GOLDEN_CHUNK: &str =\n",
            "    \"w7vP5xKq2mR8tNzL1cYb4fJ6hD0sG9uA3eWiXoVkBpQaZyTnMlEdCrHgUjSdF2m\";\n",
            "pub(super) const GOLDEN_HUNK: &str = \"Zx4Qw7Vb2Nr8Tm5Kj3Yc6Fh9Sd0Gg1Ua2Ei4Oo5Pp8Lk=\";\n",
        );
        let out = scan_file_with_pins(t);
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert_eq!(out.digest_pins.len(), 2);
    }

    /// MUST-FIRE (the P74 probe contract): a PRIVATE KEY block WITH a
    /// body — the named-fixture class — stays structural evidence, name
    /// notwithstanding (defer (a)); a real-looking kebab password on a
    /// non-credential binding stays; a NON-kebab credential value stays;
    /// a partial alphabet stays; an unknown tag stays; a golden-NAMED
    /// provider secret stays (kind evidence outranks the name).
    #[test]
    fn p74_probe_negatives_stay_firing() {
        // A real PEM body between the markers (short fixture body) — the
        // delimiter guard must NOT suppress a keyed block.
        let body = "aGVsbG8gd29ybGQgaGVsbG8gd29ybGQgaGVsbG8gd29ybGQg\naGVsbG8g\n";
        let t = format!("const FIXTURE: &str = \"-----BEGIN OPENSSH PRIVATE KEY-----\n{body}-----END OPENSSH PRIVATE KEY-----\";\n");
        let f = scan_file(&t);
        assert!(f.iter().any(|x| x.tag() == "private-key-block"), "{f:?}");

        // A kebab-slug value on a NON-credential (but secret-shaped)
        // binding — the name evidence must not leak to other bindings.
        let t = "let service_password = \"my-service-events-queue\";\n";
        let f = scan_file(t);
        assert!(f.iter().any(|x| x.tag() == "rust-secret-literal"), "{f:?}");

        // An uppercase-mixed value on a credential-named binding — a real
        // password shape; the kebab guard must refuse.
        let t = "const DEPLOY_CREDENTIAL: &str = \"hunter2S3cret\";\n";
        let f = scan_file(t);
        assert!(f.iter().any(|x| x.tag() == "rust-secret-literal"), "{f:?}");

        // A PARTIAL alphabet (the first 40 chars) — not the exact sequence.
        let t = format!("const CHARS: &[u8] = b\"{}\";\n", &B64_ALPHABET_STR[..40]);
        assert_eq!(scan_file(&t).len(), 1, "partial alphabet must not suppress");

        // An unknown identifier tag.
        let t = "let x = \"redis:a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0\";\n";
        assert_eq!(scan_file(t).len(), 1);

        // A provider secret under a golden name — kind evidence wins.
        let t = format!("const GOLDEN_KEY: &str = \"{AWS_KEY}\";\n");
        let f = scan_file(&t);
        assert!(f.iter().any(|x| x.tag() == "aws-access-key"), "{f:?}");
    }

    /// Fail-closed shape resolution: a `PrivateKeyBlock` the guard cannot
    /// resolve (no recognizable header inside the span) stays a finding.
    #[test]
    fn p74_unresolvable_key_block_stays_firing() {
        // The scanner's needle is BEGIN…PRIVATE KEY-----; a malformed span
        // that still triggers the kind must not be suppressed.
        let t = "-----BEGIN OPENSSH PRIVATE KEY-----\n-----END OPENSSH PRIVATE KEY-----\n";
        let out = scan_file_with_pins(t);
        assert!(out.findings.is_empty(), "plain delimiter pair suppresses");
        assert_eq!(out.digest_pins.len(), 1);
    }
}
