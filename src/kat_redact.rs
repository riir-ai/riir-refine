//! KAT redaction gate — Proposal 005 Phase 0 T0.2 (Plan 026).
//!
//! **No payload leaves the machine before passing this gate** (Proposal 005
//! §Redaction — "the landmine the brief missed"). Every span the service
//! planes would sync or send to a remote LLM goes through:
//!
//! 1. **Eligibility (deny-by-default):** sync is opt-in per project via a
//!    `.heal-sync-policy` allowlist. No policy file → nothing syncs. The
//!    policy allows by **origin** (`own-repo` / `authored-fixture` /
//!    `mined-corpus`; `external` = user-submitted code is absent from the
//!    shipped policy by design) and, when the span carries a source path,
//!    by **path glob**. A span-size cap enforces minimization (the
//!    AST-chunked span, never whole files).
//! 2. **Content scan:** secret-pattern detection (keys, tokens, env values,
//!    high-entropy strings) + identifier policy (secret-shaped identifiers
//!    carrying string literals). Any finding blocks the span by default
//!    (`allow_redacted: false`); with `allow_redacted: true` the finding
//!    bytes are replaced by `[REDACTED:<kind>]` placeholders — byte-exact
//!    outside the finding ranges.
//! 3. **Logging:** every block/redaction lands in
//!    `<root>/.heal/redaction_log.jsonl` carrying the finding **kind + byte
//!    range + BLAKE3 fingerprint — never the finding bytes**. The log is
//!    proof without leakage.
//!
//! # Versioning (G9b discipline)
//!
//! The pattern set is part of [`REDACT_VERSION`]: adding/removing a
//! detection pattern is a version bump + changelog entry, never a silent
//! detection change. The version rides the policy file's `version` field —
//! a policy written for a different gate version **denies** (fail-closed;
//! the opposite of the meter's fresh-start, because a stale allowlist must
//! never accidentally allow).
//!
//! # Honest scope (what v1 does NOT catch)
//!
//! - No base64/gzip layer-stripping: a secret encoded then embedded is only
//!   caught if the encoded blob itself trips the entropy rule. **That
//!   conditional is weaker than it reads — measured 2026-08-28 ([Bench
//!   052](../.benchmarks/052_t32_external_leak_posture.md)): over a
//!   deterministic 4 000-key sweep of AWS-shaped credentials, hex-wrapping
//!   is caught only **37.0%** of the time (entropy range 2.349..3.412
//!   against the 3.000 hexish threshold), so the same technique succeeds or
//!   fails on the VALUE and an attacker picks the value. **Closed in v2**
//!   for the hex layer (now unconditional; 4 000/4 000 in the sweep).
//!   base64 and the arbitrary-transform classes (byte arrays, xor,
//!   reversal, split literals) remain uncaught — treat them as uncaught,
//!   not as probabilistically caught.
//! - Entropy thresholds: **hex ≥ 32 chars blocks UNCONDITIONALLY (v2 — no
//!   entropy test)**, base64-ish ≥ 32 chars @ ≥ 4.5 bits/char. v1 gated hex
//!   at ≥ 3.0 and that measured as a coin flip rather than a threshold
//!   (37.0% of hex-wrapped AWS-shaped keys caught, range 2.349..3.412 —
//!   Bench 052); a boundary whose verdict depends on a value the attacker
//!   chooses is not a threshold, so v2 is fail-closed and deterministic
//!   there. **Committed hash literals in test fixtures (BLAKE3/GGUF
//!   digests) WILL block** — fail-closed by design, and now for every
//!   digest rather than the high-entropy ones only; blocked + logged until
//!   a future version adds allow-by-fingerprint. The 4.5 cut on the
//!   non-hex arm is deliberately kept: it is what stops long `snake_case`
//!   identifiers from false-positiving (`_` is not a hex digit, so those
//!   runs land on that arm).
//! - Re-scanning a redacted payload can re-flag the placeholder site (the
//!   secret-shaped *name* survives value redaction) — also fail-closed;
//!   treat a redacted payload as terminal, do not re-gate it.
//! - The scanner is line- and token-based, not AST-based. **v3 rescoped the
//!   identifier-policy rule from the physical line to the declaration**, so
//!   the `rustfmt`-wrapped spelling
//!   (`const NAME: &str =` ⏎ `    "literal";`) is caught — before v3 the
//!   same secret in the same declaration blocked at one line width and was
//!   ALLOWED at another, which made the verdict a property of formatting
//!   rather than of the span (Bench 053). Still uncaught, deliberately: a
//!   secret split across two lines *inside* the literal (`"aa\` ⏎ `bb"`),
//!   and shell `\`-continuation of an env assignment — no formatter emits
//!   either and no shell path is in the shipped `allow_paths`. Entropy +
//!   known formats remain the backstop for both.
//!
//! # Wiring (the standing surfaces)
//!
//! - `fixer_regate` (Issue 040 harness) gates every sample BEFORE any arm
//!   runs — arms A/B send prompts to CF Workers AI (real egress) and every
//!   arm writes corpus rows; a blocked span reaches neither.
//! - T0.3's sync payload builder consumes [`RedactionGate::check`] the same
//!   way when it opens.
//!
//! G4 (the standing CI gate): adversarial fixtures with planted AWS keys /
//! private keys / connection strings assert zero secret bytes in any
//! synced payload — `tests/kat_redact_g4.rs`.
//!
//! # The `external` origin is walled off by POLICY, not by this scanner
//!
//! [`RedactionGate::check`] denies on origin BEFORE scanning content, and
//! `external` is absent from the shipped allowlist — so for user-submitted
//! code the load-bearing control is that one policy line, and this
//! scanner carries no evidence about the class. Proposal 005 T3.2 asked
//! whether the scanner could take over that job if `external` were
//! admitted; it was measured and the answer is **no, structurally**
//! ([Bench 052](../.benchmarks/052_t32_external_leak_posture.md),
//! suite `tests/kat_redact_t32_external.rs`): for consumer code the
//! confidential asset is the SPAN ITSELF, so a span can carry zero
//! secrets, pass every rule, and still be exactly what must not egress
//! (measured: PII / trade-secret spans are `Allow` with zero findings).
//! No pattern set closes that, because there is no pattern for "this
//! source belongs to someone else". **Do not admit `external` on the
//! strength of a stronger pattern set** — that is the specific dangerous
//! move Bench 052 names.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// The gate definition version (G9b discipline — the pattern set + policy
/// semantics are part of the version). Bump ONLY when detection or policy
/// semantics change what passes/blocks for some input, and record the delta
/// in [`REDACT_CHANGELOG`] in the same change.
pub const REDACT_VERSION: u32 = 3;

/// Versioned deltas of the gate definition.
pub const REDACT_CHANGELOG: &[(u32, &str)] = &[
    (
        1,
        "initial definition: deny-by-default policy (origin allowlist + path \
         glob + span-size cap), 12 detection rules (aws access key, github / \
         gitlab / slack / openai-style tokens, private key blocks, \
         credentialed connection strings, bearer tokens, env secret \
         assignments, rust secret literals, high-entropy hex + base64), \
         byte-exact redaction, fingerprint-only JSONL log.",
    ),
    (
        2,
        "hexish high-entropy rule made UNCONDITIONAL: an all-hex run of 32+ \
         chars now blocks with no entropy test. v1's >= 3.0 bits/char cut \
         measured as a coin flip, not a threshold — 37.0% of AWS-shaped \
         credentials caught when hex-wrapped, entropy range 2.349..3.412 \
         straddling the cut (Bench 052). Fail-closed + deterministic \
         instead. Non-hex runs keep the 4.5 threshold (it is what protects \
         long snake_case identifiers). Measured cost on riir-clippy: 2 \
         additional blocked spans repo-wide, both synthetic fixtures.",
    ),
    (
        3,
        "identifier-policy rule (rust secret literals) rescoped from the \
         PHYSICAL LINE to the DECLARATION: a secret-shaped `let/const/static \
         NAME =` whose literal `rustfmt` pushed to the next line is now \
         caught. v1/v2 required name, `=` and literal on one line, so the \
         SAME secret in the SAME declaration was blocked at one width and \
         allowed at another — the verdict was decided by formatting, and \
         `rustfmt` generates the allowed spelling automatically past \
         `max_width` (Bench 053). One line of lookahead, the exact shape \
         `rustfmt` emits. Detection is otherwise unchanged; no pattern was \
         added. Measured cost on riir-clippy: 0 additional blocked spans \
         across 230 .rs files (12 rust-secret-literal findings, none \
         wrapped). Shell `\\`-continuation of env assignments stays \
         UNCAUGHT and out of scope — no formatter emits it and the shipped \
         allow_paths carry no shell path.",
    ),
];

/// The policy file name a project drops at its root to opt into sync.
pub const POLICY_FILE_NAME: &str = ".heal-sync-policy";

/// Default span-size cap (span minimization — whole files refuse).
pub const DEFAULT_MAX_SPAN_BYTES: usize = 65_536;

// ── Origins ────────────────────────────────────────────────────────────────

/// Where a span came from — the policy's primary allow axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanOrigin {
    /// A span from a file inside the project that owns the policy file
    /// (path-gated on top of the origin).
    OwnRepo,
    /// In-repo authored eval fixtures (e.g. the `reachable:` regate set) —
    /// written by us, never user code.
    AuthoredFixture,
    /// The pre-vetted research corpus (e.g. the v2edit chunks — public
    /// crates.io provenance, already through the research pipeline).
    MinedCorpus,
    /// User-submitted / third-party code — the trust class the gate exists
    /// to wall off. Absent from the shipped policy by design.
    External,
}

impl SpanOrigin {
    /// Policy-file spelling (also the log row's `origin` field).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OwnRepo => "own-repo",
            Self::AuthoredFixture => "authored-fixture",
            Self::MinedCorpus => "mined-corpus",
            Self::External => "external",
        }
    }
}

impl fmt::Display for SpanOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Findings ───────────────────────────────────────────────────────────────

/// What tripped the scanner. Display is the kebab tag used in placeholders
/// (`[REDACTED:aws-access-key]`) and log rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    AwsAccessKey,
    GithubToken,
    GitlabToken,
    SlackToken,
    OpenAiStyleKey,
    PrivateKeyBlock,
    ConnStringCreds,
    BearerToken,
    EnvSecretAssign,
    RustSecretLiteral,
    HighEntropyHex,
    HighEntropyB64,
}

impl FindingKind {
    /// Kebab tag for placeholders + logs.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::AwsAccessKey => "aws-access-key",
            Self::GithubToken => "github-token",
            Self::GitlabToken => "gitlab-token",
            Self::SlackToken => "slack-token",
            Self::OpenAiStyleKey => "openai-style-key",
            Self::PrivateKeyBlock => "private-key-block",
            Self::ConnStringCreds => "conn-string-creds",
            Self::BearerToken => "bearer-token",
            Self::EnvSecretAssign => "env-secret",
            Self::RustSecretLiteral => "rust-secret-literal",
            Self::HighEntropyHex => "high-entropy-hex",
            Self::HighEntropyB64 => "high-entropy-b64",
        }
    }
}

impl fmt::Display for FindingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

/// One detection: kind + byte range + a BLAKE3 fingerprint of the finding
/// bytes. The bytes themselves are deliberately NOT retained — nothing
/// downstream (log, decision) can leak what it never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: FindingKind,
    /// Byte range within the scanned span.
    pub start: usize,
    pub len: usize,
    /// `blake3(finding_bytes)[..12]` — identifies WHAT was found without
    /// carrying it.
    pub fingerprint: String,
}

impl Finding {
    fn new(kind: FindingKind, start: usize, end: usize, span: &str) -> Self {
        let bytes = &span.as_bytes()[start..end];
        Self {
            kind,
            start,
            len: end - start,
            fingerprint: blake3::hash(bytes).to_hex()[..12].to_string(),
        }
    }

    /// The placeholder this finding redacts to.
    #[must_use]
    pub fn placeholder(&self) -> String {
        format!("[REDACTED:{}]", self.kind.tag())
    }
}

// ── The scanner ────────────────────────────────────────────────────────────

/// Scan a span for secret-shaped content. Pure + deterministic: same bytes,
/// same findings, same order. Findings are ordered by byte start.
///
/// Overlaps are kept (an env-assigned AWS secret yields both the env finding
/// and the entropy finding); [`redact`] de-overlaps deterministically.
#[must_use]
pub fn scan_secrets(span: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    scan_prefixed_tokens(span, &mut out);
    scan_private_key_blocks(span, &mut out);
    scan_conn_strings(span, &mut out);
    scan_bearer(span, &mut out);
    scan_env_assignments(span, &mut out);
    scan_rust_secret_literals(span, &mut out);
    scan_entropy(span, &mut out);
    out.sort_by_key(|f| (f.start, std::cmp::Reverse(f.len)));
    out
}

/// Fixed-prefix token formats: AWS access keys, GitHub/GitLab/Slack tokens,
/// OpenAI-style `sk-` keys.
fn scan_prefixed_tokens(span: &str, out: &mut Vec<Finding>) {
    /// One fixed-prefix token rule: prefix, kind, body-char predicate, min body
    /// length.
    type PrefixRule = (&'static str, FindingKind, fn(u8) -> bool, usize);

    const PREFIX_RULES: &[PrefixRule] = &[
        // AWS access key ids: AKIA/ASIA + ≥16 [0-9A-Z] (canonical 20 total;
        // longer runs also flag — fail-closed).
        ("AKIA", FindingKind::AwsAccessKey, is_alnum_upper as fn(u8) -> bool, 16),
        ("ASIA", FindingKind::AwsAccessKey, is_alnum_upper, 16),
        ("ghp_", FindingKind::GithubToken, is_alnum, 20),
        ("gho_", FindingKind::GithubToken, is_alnum, 20),
        ("ghu_", FindingKind::GithubToken, is_alnum, 20),
        ("ghs_", FindingKind::GithubToken, is_alnum, 20),
        ("ghr_", FindingKind::GithubToken, is_alnum, 20),
        ("glpat-", FindingKind::GitlabToken, is_gitlab_body, 20),
        ("xoxb-", FindingKind::SlackToken, is_slack_body, 10),
        ("xoxa-", FindingKind::SlackToken, is_slack_body, 10),
        ("xoxp-", FindingKind::SlackToken, is_slack_body, 10),
        ("xoxr-", FindingKind::SlackToken, is_slack_body, 10),
        ("xoxs-", FindingKind::SlackToken, is_slack_body, 10),
        ("sk-", FindingKind::OpenAiStyleKey, is_gitlab_body, 20),
    ];
    let bytes = span.as_bytes();
    for (prefix, kind, body_ok, min_body) in PREFIX_RULES {
        let mut from = 0;
        while let Some(pos) = find_sub(bytes, prefix.as_bytes(), from) {
            let body_start = pos + prefix.len();
            let mut end = body_start;
            while end < bytes.len() && body_ok(bytes[end]) {
                end += 1;
            }
            if end - body_start >= *min_body {
                out.push(Finding::new(*kind, pos, end, span));
            }
            from = pos + prefix.len();
        }
    }
}

/// `-----BEGIN … PRIVATE KEY-----` … `-----END … PRIVATE KEY-----` (whole
/// block: the base64 body is key material even without the header).
fn scan_private_key_blocks(span: &str, out: &mut Vec<Finding>) {
    let bytes = span.as_bytes();
    let mut from = 0;
    while let Some(begin) = find_sub(bytes, b"-----BEGIN", from) {
        let header_end = match find_sub(bytes, b"PRIVATE KEY-----", begin) {
            Some(e) if e < begin + 128 => e + "PRIVATE KEY-----".len(),
            _ => {
                from = begin + 10;
                continue;
            }
        };
        // Extend to the matching END marker when present, else end of span.
        let end = match find_sub(bytes, b"-----END", header_end) {
            Some(e) if e < begin + 65_536 => {
                match find_sub(bytes, b"-----", e + 8) {
                    Some(t) => t + 5,
                    None => bytes.len(),
                }
            }
            _ => bytes.len(),
        };
        out.push(Finding::new(FindingKind::PrivateKeyBlock, begin, end, span));
        from = end.max(header_end);
    }
}

/// `scheme://user:pass@…` — userinfo with a password before the path.
/// `$`/`{` in the userinfo marks a template, not a literal (skipped).
fn scan_conn_strings(span: &str, out: &mut Vec<Finding>) {
    let bytes = span.as_bytes();
    let mut from = 0;
    while let Some(pos) = find_sub(bytes, b"://", from) {
        let mut scheme_start = pos;
        while scheme_start > 0 && is_scheme_char(bytes[scheme_start - 1]) {
            scheme_start -= 1;
        }
        let mut j = pos + 3;
        let limit = (pos + 3 + 512).min(bytes.len());
        let mut at = None;
        while j < limit {
            match bytes[j] {
                b'@' => {
                    at = Some(j);
                    break;
                }
                b'/' | b'"' | b'\'' | b'`' => break,
                b if b.is_ascii_whitespace() => break,
                _ => j += 1,
            }
        }
        if let Some(at) = at {
            let userinfo = &span[pos + 3..at];
            let looks_literal =
                userinfo.contains(':') && !userinfo.contains('$') && !userinfo.contains('{');
            if pos - scheme_start > 0 && looks_literal {
                out.push(Finding::new(FindingKind::ConnStringCreds, scheme_start, at + 1, span));
            }
        }
        from = pos + 3;
    }
}

/// `Bearer <token>` / `bearer <token>` — ≥16 token-charset bytes.
fn scan_bearer(span: &str, out: &mut Vec<Finding>) {
    let bytes = span.as_bytes();
    for prefix in [b"Bearer ".as_slice(), b"bearer ".as_slice()] {
        let mut from = 0;
        while let Some(pos) = find_sub(bytes, prefix, from) {
            let mut end = pos + prefix.len();
            while end < bytes.len() && is_bearer_body(bytes[end]) {
                end += 1;
            }
            if end - (pos + prefix.len()) >= 16 {
                out.push(Finding::new(
                    FindingKind::BearerToken,
                    pos,
                    end,
                    span,
                ));
            }
            from = pos + prefix.len();
        }
    }
}

/// `.env`-shape lines: `[export ]NAME=VALUE` where NAME is secret-shaped and
/// VALUE is a literal (not `$VAR` templating, not empty, ≥ 4 bytes).
fn scan_env_assignments(span: &str, out: &mut Vec<Finding>) {
    let offset_of = |s: &str| s.as_ptr() as usize - span.as_ptr() as usize;
    for line in span.split('\n') {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some(eq) = rest.find('=') else { continue };
        let name = &rest[..eq];
        if !is_bare_ident(name) || !is_secret_name(name) {
            continue;
        }
        let value = rest[eq + 1..].trim();
        let (inner_start, inner_len) = match value.as_bytes().first() {
            Some(&q @ (b'"' | b'\'')) => match value[1..].find(q as char) {
                Some(rel) => (1, rel),
                None => (1, value.len() - 1),
            },
            _ => (0, value.len()),
        };
        let inner = &value[inner_start..inner_start + inner_len];
        if inner.len() < 4 || inner.starts_with('$') || inner.starts_with("${") {
            continue;
        }
        let start = offset_of(value) + inner_start;
        out.push(Finding::new(
            FindingKind::EnvSecretAssign,
            start,
            start + inner.len(),
            span,
        ));
    }
}

/// The value half of the identifier-policy rule: a `"LITERAL"` carrying ≥ 8
/// bytes inside the quotes. Factored out so the same-line and the
/// wrapped-continuation paths share ONE derivation — a rule that decides
/// differently depending on where a formatter broke the line is the exact
/// defect v3 closes, so the two paths must not be able to drift apart.
///
/// `value` must be a subslice of `span` (offsets are derived by pointer
/// arithmetic against `span`, the `offset_of` convention used throughout).
fn push_rust_secret_literal(value: &str, span: &str, out: &mut Vec<Finding>) -> bool {
    let value = value.trim_start();
    if !value.starts_with('"') {
        return false;
    }
    let Some(close) = value[1..].find('"') else { return false };
    if close < 8 {
        return false;
    }
    let start = value.as_ptr() as usize - span.as_ptr() as usize;
    out.push(Finding::new(
        FindingKind::RustSecretLiteral,
        start,
        start + close + 2,
        span,
    ));
    true
}

/// `let/const/static NAME…= "LITERAL";` where NAME is secret-shaped and the
/// literal is ≥ 8 bytes — the identifier-policy rule.
///
/// The unit of analysis is the **declaration**, not the physical line (v3).
/// `rustfmt` breaks after the `=` whenever the declaration exceeds
/// `max_width`, putting the literal on the next line — measured 2026-08-28
/// ([Bench 053](../.benchmarks/053_redact_v3_declaration_scope.md)): the same
/// secret in the same declaration was `Blocked` on one line and `Allow`ed once
/// wrapped, so the verdict was decided by line width. One line of lookahead
/// (exactly the shape `rustfmt` emits — it never inserts a blank line there)
/// makes the two spellings agree. Single pass, no extra allocation.
fn scan_rust_secret_literals(span: &str, out: &mut Vec<Finding>) {
    // Set when the previous line was a secret-shaped declaration whose `=`
    // ended the line; the value is expected on the line now being read.
    let mut carried = false;
    for line in span.split('\n') {
        let trimmed = line.trim_start();
        // A carried declaration consumes this line only if it really is the
        // literal; otherwise fall through — the line may open its own
        // declaration.
        if std::mem::take(&mut carried) && push_rust_secret_literal(trimmed, span, out) {
            continue;
        }
        let rest = strip_rust_prefixes(trimmed);
        let ws = rest.find(|c: char| c.is_whitespace() || c == ':').unwrap_or(rest.len());
        let name = &rest[..ws];
        if !is_bare_ident(name) || !is_secret_name(name) {
            continue;
        }
        // Optional `: TYPE` between the name and `=`.
        let after_name = &rest[ws..];
        let Some(eq) = after_name.find('=') else { continue };
        let between = &after_name[..eq];
        if !between.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, ':' | '&' | '\'' | ' ' | '_' | '<' | '>')
        }) {
            continue;
        }
        let value = after_name[eq + 1..].trim_start();
        // Nothing after the `=` (bar a CRLF `\r`) — the wrapped shape.
        if value.trim_end().is_empty() {
            carried = true;
            continue;
        }
        push_rust_secret_literal(value, span, out);
    }
}

/// High-entropy blobs: ≥ 32-char runs of hex (entropy ≥ 3.0) or base64ish
/// (entropy ≥ 4.5). Catches unknown-format keys; also trips committed hash
/// literals (fail-closed — see the module doc).
fn scan_entropy(span: &str, out: &mut Vec<Finding>) {
    let bytes = span.as_bytes();
    let mut start: Option<usize> = None;
    let flush = |out: &mut Vec<Finding>, s: usize, e: usize| {
        let token = &span[s..e];
        let len = e - s;
        if len >= 32 {
            let hexish = token.bytes().all(|b| b.is_ascii_hexdigit());
            // v2: hexish runs are flagged UNCONDITIONALLY — no entropy test.
            //
            // v1 gated them at >= 3.0 bits/char, which measured as a coin
            // flip rather than a threshold: over a deterministic 4 000-key
            // sweep of AWS-shaped credentials, hex-wrapping was caught only
            // 37.0% of the time, entropy range 2.349..3.412 straddling the
            // 3.000 cut (Bench 052). A security boundary whose verdict
            // depends on the specific value — which an attacker chooses —
            // is not a threshold, and the fail-closed philosophy the rest of
            // this gate follows says an all-hex run of 32+ chars should
            // block regardless: it is a digest or an encoded blob, and both
            // are things this gate exists to stop leaving the machine.
            //
            // Cost, measured over this repo's own src/ + tests/ before the
            // change: 17 hexish runs >= 32 chars, 15 already blocking, so
            // exactly 2 newly block — both the same synthetic `4242…`
            // fixture. Identifiers cannot be caught by this branch: `_` is
            // not an ASCII hex digit, so any snake_case run falls to the
            // non-hex arm and keeps the 4.5 threshold that protects long
            // identifiers from false positives.
            if hexish {
                out.push(Finding::new(FindingKind::HighEntropyHex, s, e, span));
            } else if shannon_entropy(&bytes[s..e]) >= 4.5 {
                out.push(Finding::new(FindingKind::HighEntropyB64, s, e, span));
            }
        }
    };
    for (i, &b) in bytes.iter().enumerate() {
        if is_entropy_body(b) {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            flush(out, s, i);
        }
    }
    if let Some(s) = start {
        flush(out, s, bytes.len());
    }
}

/// Replace every finding's bytes with its placeholder. Byte-exact outside
/// the finding ranges. Overlapping findings are resolved deterministically:
/// sorted by (start, longest-first), later overlaps dropped.
#[must_use]
pub fn redact(span: &str, findings: &[Finding]) -> String {
    let mut kept: Vec<&Finding> = findings.iter().collect();
    kept.sort_by_key(|f| (f.start, std::cmp::Reverse(f.len)));
    let mut chosen: Vec<&Finding> = Vec::with_capacity(kept.len());
    let mut cursor = 0;
    for f in kept {
        if f.start >= cursor {
            chosen.push(f);
            cursor = f.start + f.len;
        }
    }
    let mut out =
        String::with_capacity(span.len().saturating_sub(cursor).saturating_add(chosen.len() * 24));
    let mut at = 0;
    for f in chosen {
        out.push_str(&span[at..f.start]);
        out.push_str(&f.placeholder());
        at = f.start + f.len;
    }
    out.push_str(&span[at.min(span.len())..]);
    out
}

// ── Scanner helpers ────────────────────────────────────────────────────────

fn is_alnum_upper(b: u8) -> bool {
    b.is_ascii_uppercase() || b.is_ascii_digit()
}

fn is_alnum(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

fn is_gitlab_body(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

fn is_slack_body(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-'
}

fn is_bearer_body(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'~' | b'+' | b'/' | b'-' | b'=')
}

fn is_scheme_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'+' | b'.' | b'-')
}

fn is_entropy_body(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'_' | b'-' | b'.' | b'=')
}

fn find_sub(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() || from > haystack.len() - needle.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn is_bare_ident(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with(|c: char| c.is_ascii_digit())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Segment-exact word list for the identifier policy (on `_`-split parts).
const SECRET_NAME_SEGMENTS: &[&str] = &[
    "password", "passwd", "pwd", "secret", "secrets", "token", "tokens", "credential",
    "credentials", "auth",
];

/// Substring word list — specific multi-word names.
const SECRET_NAME_SUBSTRINGS: &[&str] = &[
    "api_key", "apikey", "access_key", "private_key", "client_secret", "signing_key",
    "session_key", "secret_key", "auth_token", "bearer",
];

/// The identifier-policy word list. Segment-exact matches (on `_`-split
/// parts) for standalone words; substring matches for specific multi-word
/// names. `author`/`authorization` deliberately do NOT match (segment-exact
/// `auth` only).
fn is_secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.split('_').any(|seg| SECRET_NAME_SEGMENTS.contains(&seg))
        || SECRET_NAME_SUBSTRINGS.iter().any(|s| lower.contains(s))
}

/// Strip `pub `, `const `, `static `, `let `, `mut ` prefixes from a
/// statement head (best-effort, whitespace-run agnostic).
fn strip_rust_prefixes(mut s: &str) -> &str {
    loop {
        let next = s
            .strip_prefix("pub ")
            .or_else(|| single_space_variants(s, "pub"))
            .or_else(|| single_space_variants(s, "const"))
            .or_else(|| single_space_variants(s, "static"))
            .or_else(|| single_space_variants(s, "let"))
            .or_else(|| single_space_variants(s, "mut"));
        match next {
            Some(n) => s = n.trim_start(),
            None => return s,
        }
    }
}

/// `keyword` followed by any positive run of spaces/tabs.
fn single_space_variants<'a>(s: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = s.strip_prefix(keyword)?;
    let n = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    (n > 0).then(|| &rest[n..])
}

/// Shannon entropy in bits/byte.
fn shannon_entropy(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut counts = [0u32; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let n = f64::from(u32::try_from(bytes.len()).unwrap_or(u32::MAX));
    let mut h = 0.0;
    for &c in &counts {
        if c > 0 {
            let p = f64::from(c) / n;
            h -= p * p.log2();
        }
    }
    h
}

// ── The policy ─────────────────────────────────────────────────────────────

/// The `.heal-sync-policy` allowlist. Missing fields default DENY
/// (`sync_enabled` defaults false, lists default empty) — a hand-edited
/// policy that drops a field tightens, never loosens.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SyncPolicy {
    pub version: u32,
    #[serde(default)]
    pub sync_enabled: bool,
    #[serde(default)]
    pub allow_origins: Vec<String>,
    #[serde(default)]
    pub allow_paths: Vec<String>,
    #[serde(default = "default_max_span_bytes")]
    pub max_span_bytes: usize,
    #[serde(default)]
    pub allow_redacted: bool,
}

fn default_max_span_bytes() -> usize {
    DEFAULT_MAX_SPAN_BYTES
}

impl Default for SyncPolicy {
    fn default() -> Self {
        Self {
            version: REDACT_VERSION,
            sync_enabled: false,
            allow_origins: Vec::new(),
            allow_paths: Vec::new(),
            max_span_bytes: DEFAULT_MAX_SPAN_BYTES,
            allow_redacted: false,
        }
    }
}

/// Glob match with `**` (crosses `/`), `*` (within a segment), `?` (one
/// non-`/` char). Patterns are config-file-sized; the recursive matcher is
/// not on any hot path.
#[must_use]
pub fn glob_match(pattern: &str, path: &str) -> bool {
    glob_inner(pattern.as_bytes(), path.as_bytes())
}

fn glob_inner(p: &[u8], s: &[u8]) -> bool {
    match p.first() {
        None => s.is_empty(),
        Some(b'*') => {
            if p.len() > 1 && p[1] == b'*' {
                // `**` — any run, crossing `/`. Consume extra stars, then
                // both the `**/`-joined and bare-`**` continuations.
                let mut q = 1;
                while q < p.len() && p[q] == b'*' {
                    q += 1;
                }
                let rest = &p[q..];
                let no_slash = rest.strip_prefix(b"/").unwrap_or(rest);
                (0..=s.len()).any(|i| glob_inner(no_slash, &s[i..]))
                    || (0..=s.len()).any(|i| glob_inner(rest, &s[i..]))
            } else {
                (0..=s.len())
                    .take_while(|&i| i == 0 || s[i - 1] != b'/')
                    .any(|i| glob_inner(&p[1..], &s[i..]))
            }
        }
        Some(b'?') => !s.is_empty() && s[0] != b'/' && glob_inner(&p[1..], &s[1..]),
        Some(&c) => !s.is_empty() && s[0] == c && glob_inner(&p[1..], &s[1..]),
    }
}

// ── The gate ───────────────────────────────────────────────────────────────

/// Why a span was denied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    /// No `.heal-sync-policy` found — deny-by-default.
    NoPolicy,
    /// Policy has `sync_enabled: false`.
    SyncDisabled,
    /// Origin not on the allowlist.
    OriginNotAllowed(SpanOrigin),
    /// Span carries a path that matches no allow glob.
    PathNotAllowed(String),
    /// Span exceeds the minimization cap.
    SpanTooLarge { bytes: usize, max: usize },
    /// Secret findings present and the policy refuses redacted payloads.
    SecretsFound,
}

impl fmt::Display for BlockReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPolicy => {
                write!(f, "no .heal-sync-policy — deny-by-default")
            }
            Self::SyncDisabled => write!(f, "sync disabled by policy"),
            Self::OriginNotAllowed(o) => write!(f, "origin '{o}' not allowed"),
            Self::PathNotAllowed(p) => write!(f, "path '{p}' not allowed"),
            Self::SpanTooLarge { bytes, max } => {
                write!(f, "span {bytes}B exceeds cap {max}B (minimization)")
            }
            Self::SecretsFound => write!(f, "secret findings present"),
        }
    }
}

/// The gate's verdict for one span.
#[derive(Debug, Clone, PartialEq)]
pub enum GateDecision {
    /// Eligible + zero findings — sync as-is.
    Allow,
    /// Denied — stripped from the sync batch; log it.
    Blocked { reason: BlockReason, findings: Vec<Finding> },
    /// Eligible, findings present, policy allows redacted payloads: the
    /// returned payload has every finding replaced by its placeholder.
    Redacted { payload: String, findings: Vec<Finding> },
}

impl GateDecision {
    /// The findings that tripped (empty for `Allow` and non-secret blocks).
    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        match self {
            Self::Allow => &[],
            Self::Blocked { findings, .. } | Self::Redacted { findings, .. } => findings,
        }
    }

    /// True when the span may proceed (as-is or redacted).
    #[must_use]
    pub fn is_syncable(&self) -> bool {
        matches!(self, Self::Allow | Self::Redacted { .. })
    }
}

/// A loaded (or defaulted) gate + how it was loaded, for the caller's
/// logging. [`Self::path`] = `None` means deny-all (no policy file found).
#[derive(Debug)]
pub struct LoadedPolicy {
    pub gate: RedactionGate,
    pub path: Option<PathBuf>,
    /// Non-fatal load problem (parse failure, version mismatch) — the gate
    /// denies everything when set; surfaced so the CLI can warn loudly.
    pub warning: Option<String>,
}

impl LoadedPolicy {
    /// The root the policy was found under (its parent dir) — where the
    /// `.heal/` log belongs. Falls back to the cwd when no policy loaded.
    #[must_use]
    pub fn root(&self) -> PathBuf {
        self.path
            .as_ref()
            .and_then(|p| p.parent()).map_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")), Path::to_path_buf)
    }
}

/// The redaction gate: eligibility (policy) + content scan + redaction.
#[derive(Debug, Clone)]
pub struct RedactionGate {
    policy: SyncPolicy,
}

impl RedactionGate {
    /// A gate from an already-built policy.
    #[must_use]
    pub fn from_policy(policy: SyncPolicy) -> Self {
        Self { policy }
    }

    /// The deny-everything gate (no policy / load failure).
    #[must_use]
    pub fn deny_all() -> Self {
        Self::from_policy(SyncPolicy::default())
    }

    /// Load the gate from an exact policy path. Missing/corrupt/unknown
    /// version → deny-all + a warning (fail-closed: a stale allowlist must
    /// never accidentally allow).
    #[must_use]
    pub fn load(path: &Path) -> LoadedPolicy {
        let deny = |warning: String| LoadedPolicy {
            gate: Self::deny_all(),
            path: Some(path.to_path_buf()),
            warning: Some(warning),
        };
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => return deny(format!("unreadable policy {}: {e}", path.display())),
        };
        let policy: SyncPolicy = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(e) => return deny(format!("unparseable policy {}: {e}", path.display())),
        };
        if policy.version != REDACT_VERSION {
            return deny(format!(
                "policy {} carries version {} (this build defines {REDACT_VERSION}) — \
                 deny-by-default until the policy is migrated",
                path.display(),
                policy.version
            ));
        }
        LoadedPolicy { gate: Self { policy }, path: Some(path.to_path_buf()), warning: None }
    }

    /// `--policy`/env override when given, else walk up from the cwd looking
    /// for [`POLICY_FILE_NAME`]. Nothing found → deny-all, no warning (the
    /// no-policy state is the documented default, not an error).
    #[must_use]
    pub fn discover(explicit: Option<&Path>) -> LoadedPolicy {
        if let Some(p) = explicit {
            return Self::load(p);
        }
        if let Some(p) = std::env::var_os("RIIR_HEAL_SYNC_POLICY") {
            return Self::load(&PathBuf::from(p));
        }
        let Ok(cwd) = std::env::current_dir() else {
            return LoadedPolicy { gate: Self::deny_all(), path: None, warning: None };
        };
        let mut dir: Option<&Path> = Some(cwd.as_path());
        while let Some(d) = dir {
            let cand = d.join(POLICY_FILE_NAME);
            if cand.is_file() {
                return Self::load(&cand);
            }
            dir = d.parent();
        }
        LoadedPolicy { gate: Self::deny_all(), path: None, warning: None }
    }

    /// The policy in force (inspector).
    #[must_use]
    pub fn policy(&self) -> &SyncPolicy {
        &self.policy
    }

    /// Run one span through the gate: eligibility → content scan → verdict.
    #[must_use]
    pub fn check(&self, origin: SpanOrigin, path: Option<&str>, span: &str) -> GateDecision {
        if !self.policy.sync_enabled {
            return GateDecision::Blocked {
                reason: BlockReason::SyncDisabled,
                findings: Vec::new(),
            };
        }
        if !self.policy.allow_origins.iter().any(|a| a == origin.as_str()) {
            return GateDecision::Blocked {
                reason: BlockReason::OriginNotAllowed(origin),
                findings: Vec::new(),
            };
        }
        if let Some(p) = path {
            if !self.policy.allow_paths.is_empty()
                && !self.policy.allow_paths.iter().any(|g| glob_match(g, p))
            {
                return GateDecision::Blocked {
                    reason: BlockReason::PathNotAllowed(p.to_string()),
                    findings: Vec::new(),
                };
            }
        }
        if span.len() > self.policy.max_span_bytes {
            return GateDecision::Blocked {
                reason: BlockReason::SpanTooLarge {
                    bytes: span.len(),
                    max: self.policy.max_span_bytes,
                },
                findings: Vec::new(),
            };
        }
        let findings = scan_secrets(span);
        if findings.is_empty() {
            return GateDecision::Allow;
        }
        if self.policy.allow_redacted {
            let payload = redact(span, &findings);
            GateDecision::Redacted { payload, findings }
        } else {
            GateDecision::Blocked { reason: BlockReason::SecretsFound, findings }
        }
    }
}

// ── The log ────────────────────────────────────────────────────────────────

/// Append-only JSONL block/redaction log at `<root>/.heal/redaction_log.jsonl`
/// (`RIIR_HEAL_REDACT_LOG` overrides). Rows carry finding kind + byte range +
/// BLAKE3 fingerprint — never finding bytes (impossible by construction:
/// [`Finding`] does not retain them).
#[derive(Debug)]
pub struct RedactionLog {
    path: PathBuf,
}

impl RedactionLog {
    /// The log file name under the `.heal` root.
    pub const FILE_NAME: &'static str = "redaction_log.jsonl";

    /// Open the log at `<root>/.heal/redaction_log.jsonl` (or the
    /// `RIIR_HEAL_REDACT_LOG` override when set).
    #[must_use]
    pub fn open(root: &Path) -> Self {
        if let Some(p) = std::env::var_os("RIIR_HEAL_REDACT_LOG") {
            return Self::at(PathBuf::from(p));
        }
        Self::at(root.join(".heal").join(Self::FILE_NAME))
    }

    /// Open the log at an exact path (tests).
    #[must_use]
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// The log file path (diagnostics).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one decision row. A failed write never blocks the run (the
    /// meter precedent: bookkeeping warns, it does not abort) — but the
    /// caller is expected to surface the error.
    ///
    /// # Errors
    ///
    /// The underlying `io::Error` from directory creation or the append.
    pub fn record(
        &self,
        decision: &GateDecision,
        origin: SpanOrigin,
        span_id: &str,
        span_fingerprint: &str,
    ) -> io::Result<()> {
        use std::io::Write as _;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let (event, reason) = match decision {
            GateDecision::Allow => ("allow", None),
            GateDecision::Blocked { reason, .. } => ("blocked", Some(reason.to_string())),
            GateDecision::Redacted { .. } => ("redacted", None),
        };
        let findings: Vec<serde_json::Value> = decision
            .findings()
            .iter()
            .map(|f| {
                serde_json::json!({
                    "kind": f.kind.tag(),
                    "start": f.start,
                    "len": f.len,
                    "fingerprint": f.fingerprint,
                })
            })
            .collect();
        let row = serde_json::json!({
            "ts_unix": ts,
            "gate_version": REDACT_VERSION,
            "event": event,
            "reason": reason,
            "origin": origin.as_str(),
            "span_id": span_id,
            "span_fingerprint": span_fingerprint,
            "findings": findings,
        });
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&self.path)?;
        writeln!(f, "{row}")
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Fake but format-valid secrets (the gitleaks-fixture convention).

    #[test]
    fn aws_access_key_flagged() {
        let span = "let client = connect(\"AKIAIOSFODNN7EXAMPLE\");";
        let f = scan_secrets(span);
        assert!(f.iter().any(|x| x.kind == FindingKind::AwsAccessKey), "{f:?}");
        // The finding range covers exactly the key bytes.
        let k = f.iter().find(|x| x.kind == FindingKind::AwsAccessKey).unwrap();
        assert_eq!(&span[k.start..k.start + k.len], "AKIAIOSFODNN7EXAMPLE");
    }

    #[test]
    fn longer_aws_shaped_run_still_flags_fail_closed() {
        let span = "AKIAIOSFODNN7EXAMPLEXYZ";
        assert!(scan_secrets(span).iter().any(|f| f.kind == FindingKind::AwsAccessKey));
    }

    #[test]
    fn github_and_slack_and_openai_tokens_flagged() {
        let span = "gh ghp_16C7eBAF1q9XZKL3m0yv8u2oF4R6tYhUw glpat-AbCdEf12345678901234 \
                    xoxb-123456789012-abcdef sk-proj-abcdefghijklmnopqrstuvwxyz12";
        let f = scan_secrets(span);
        assert!(f.iter().any(|x| x.kind == FindingKind::GithubToken), "{f:?}");
        assert!(f.iter().any(|x| x.kind == FindingKind::GitlabToken));
        assert!(f.iter().any(|x| x.kind == FindingKind::SlackToken));
        assert!(f.iter().any(|x| x.kind == FindingKind::OpenAiStyleKey));
    }

    #[test]
    fn short_prefix_lookalikes_do_not_flag() {
        // Prefix + too-short body = not a token format we know.
        let span = "let sk = sk_short; let gh = ghp_tiny; pass AKIA_SHORT;";
        assert!(!scan_secrets(span).iter().any(|f| matches!(
            f.kind,
            FindingKind::AwsAccessKey
                | FindingKind::GithubToken
                | FindingKind::OpenAiStyleKey
        )));
    }

    #[test]
    fn private_key_block_flags_whole_block() {
        let span = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA7somefakedata\n\
                    -----END RSA PRIVATE KEY-----\n";
        let f = scan_secrets(span);
        let k = f.iter().find(|x| x.kind == FindingKind::PrivateKeyBlock).unwrap();
        let covered = &span[k.start..k.start + k.len];
        assert!(covered.starts_with("-----BEGIN"));
        assert!(covered.contains("-----END RSA PRIVATE KEY-----"));
    }

    #[test]
    fn public_cert_headers_do_not_flag() {
        let span = "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----";
        assert!(!scan_secrets(span).iter().any(|f| f.kind == FindingKind::PrivateKeyBlock));
    }

    #[test]
    fn conn_string_with_password_flags() {
        let span = "let url = \"postgres://admin:hunter2sturdy@db.internal:5432/prod\";";
        let f = scan_secrets(span);
        let k = f.iter().find(|x| x.kind == FindingKind::ConnStringCreds).unwrap();
        assert_eq!(&span[k.start..k.start + k.len], "postgres://admin:hunter2sturdy@");
    }

    #[test]
    fn templated_conn_string_does_not_flag() {
        let span = "let url = format!(\"postgres://user:${{DB_PASS}}@host/db\");";
        assert!(!scan_secrets(span).iter().any(|f| f.kind == FindingKind::ConnStringCreds));
    }

    #[test]
    fn bearer_token_flags() {
        let span = "Authorization: Bearer abcdef1234567890abcdef";
        assert!(scan_secrets(span).iter().any(|f| f.kind == FindingKind::BearerToken));
    }

    #[test]
    fn env_assignment_flags_value_not_name() {
        let span = "export DATABASE_PASSWORD=hunter2sturdy\nOTHER=plain";
        let f = scan_secrets(span);
        let k = f.iter().find(|x| x.kind == FindingKind::EnvSecretAssign).unwrap();
        assert_eq!(&span[k.start..k.start + k.len], "hunter2sturdy");
    }

    #[test]
    fn env_quoted_value_flags_inner() {
        let span = "API_TOKEN=\"gh-style-not-needed-just-long-value\"";
        let f = scan_secrets(span);
        assert!(f.iter().any(|x| x.kind == FindingKind::EnvSecretAssign));
    }

    #[test]
    fn non_secret_env_names_do_not_flag() {
        let span = "DATABASE_URL_HOST=localhost\nMAX_CONNECTIONS=32\nAUTHOR=bob";
        assert!(!scan_secrets(span).iter().any(|f| f.kind == FindingKind::EnvSecretAssign));
    }

    #[test]
    fn rust_secret_literal_flags() {
        let span = "const API_KEY: &str = \"super-secret-value-42\";";
        let f = scan_secrets(span);
        let k = f.iter().find(|x| x.kind == FindingKind::RustSecretLiteral).unwrap();
        assert_eq!(&span[k.start..k.start + k.len], "\"super-secret-value-42\"");
    }

    #[test]
    fn rust_secret_let_mut_flags() {
        let span = "let mut password = \"tr0ub4dor&3xyz\";";
        assert!(scan_secrets(span).iter().any(|f| f.kind == FindingKind::RustSecretLiteral));
    }

    #[test]
    fn rust_typed_binding_without_literal_does_not_flag() {
        let span = "fn connect(api_key: &str) {}\nstruct C { password: String }";
        assert!(!scan_secrets(span).iter().any(|f| f.kind == FindingKind::RustSecretLiteral));
    }

    #[test]
    fn high_entropy_hex_and_b64_flag() {
        let hexish = "9f2c7a4e1b8d3f6a0c5e2b9d7f4a1c8e3b6d9f2c7a4e1b8d3f6";
        let b64ish = "qP7mZ2xQvN4kR9tLwY3jH6sD1fG5aX0bE8cU2iO4pT7nM9rK";
        assert!(scan_secrets(hexish).iter().any(|f| f.kind == FindingKind::HighEntropyHex));
        assert!(scan_secrets(b64ish).iter().any(|f| f.kind == FindingKind::HighEntropyB64));
    }

    #[test]
    fn normal_rust_code_has_zero_findings() {
        let span = "/// Computes the frobnicate transform.\npub fn frob(n: u32) -> u32 {\n    \
                    let VeryLongCamelCaseStructTypeName = n.wrapping_mul(3);\n    \
                    VeryLongCamelCaseStructTypeName.wrapping_add(1)\n}\n";
        assert_eq!(scan_secrets(span), Vec::new());
    }

    #[test]
    fn scan_is_deterministic() {
        let span = "PASSWORD=x\
                    AKIAIOSFODNN7EXAMPLE";
        let a = scan_secrets(span);
        for _ in 0..50 {
            assert_eq!(scan_secrets(span), a);
        }
    }

    #[test]
    fn redaction_is_byte_exact_outside_ranges() {
        let secret = "AKIAIOSFODNN7EXAMPLE";
        let span = format!("fn a() {{}}\nlet k = \"{secret}\";\nfn b() {{}}\n");
        let findings = scan_secrets(&span);
        let out = redact(&span, &findings);
        assert!(out.contains("fn a() {}"));
        assert!(out.contains("fn b() {}"));
        assert!(!out.contains(secret));
        assert!(out.contains("[REDACTED:aws-access-key]"));
    }

    #[test]
    fn overlapping_findings_redact_once() {
        let secret = "wJalrXUtnFEMIK7MDENGbPxRfiCYEXAMPLEKEY2";
        let span = format!("AWS_SECRET_ACCESS_KEY={secret}\n");
        let findings = scan_secrets(&span);
        assert!(findings.len() >= 2, "expected env + entropy overlap, got {findings:?}");
        let out = redact(&span, &findings);
        assert!(!out.contains(secret), "{out}");
        assert_eq!(out.matches("[REDACTED").count(), 1);
    }

    #[test]
    fn changelog_covers_every_version() {
        for v in 1..=REDACT_VERSION {
            assert!(REDACT_CHANGELOG.iter().any(|(cv, _)| *cv == v), "version {v} missing");
        }
    }

    #[test]
    fn glob_basics() {
        assert!(glob_match("src/**/*.rs", "src/a.rs"));
        assert!(glob_match("src/**/*.rs", "src/x/y/z.rs"));
        assert!(!glob_match("src/**/*.rs", "lib/a.rs"));
        assert!(glob_match("**/*.wgsl", "a/b/c.wgsl"));
        assert!(glob_match("*.rs", "main.rs"));
        assert!(!glob_match("*.rs", "src/main.rs"));
        assert!(glob_match("src/?.rs", "src/a.rs"));
        assert!(!glob_match("src/?.rs", "src/ab.rs"));
        assert!(glob_match("**", "any/thing/at/all"));
    }

    #[test]
    fn missing_policy_denies_everything() {
        let gate = RedactionGate::deny_all();
        let d = gate.check(SpanOrigin::OwnRepo, Some("src/a.rs"), "fn main() {}");
        assert!(!d.is_syncable());
    }

    #[test]
    fn external_origin_blocked_by_shipped_shape() {
        let policy = SyncPolicy {
            version: REDACT_VERSION,
            sync_enabled: true,
            allow_origins: vec!["own-repo".into(), "authored-fixture".into()],
            allow_paths: vec!["src/**/*.rs".into()],
            max_span_bytes: 1024,
            allow_redacted: false,
        };
        let gate = RedactionGate::from_policy(policy);
        assert!(gate
            .check(SpanOrigin::AuthoredFixture, None, "fn f() {}")
            .is_syncable());
        assert!(!gate
            .check(SpanOrigin::External, None, "fn f() {}")
            .is_syncable());
        // Path not on the allowlist.
        assert!(!gate.check(SpanOrigin::OwnRepo, Some("secrets/prod.rs"), "fn f() {}").is_syncable());
        // Secrets block even allowed origins.
        let d = gate.check(
            SpanOrigin::OwnRepo,
            Some("src/a.rs"),
            "let k = \"AKIAIOSFODNN7EXAMPLE\";",
        );
        assert!(!d.is_syncable());
        assert_eq!(
            d.findings().len(),
            1,
            "the 20-char key trips the format rule (below the 32-char entropy floor)"
        );
    }

    /// v3 — the property the declaration rescope buys: the gate's verdict is
    /// a function of the SPAN, not of where a formatter broke the line.
    ///
    /// This is written as an invariance over spellings rather than as a
    /// fixture list on purpose. A fixture pins one shape; the invariant pins
    /// the *class*, and it cannot be satisfied by bolting on a pattern — the
    /// only way to hold it is for the rule to scope to the declaration.
    #[test]
    fn v3_verdict_is_invariant_to_rustfmt_wrapping() {
        // Low entropy, no known credential format, 25 bytes — this value is
        // reachable ONLY by the identifier-policy rule, so the entropy and
        // format backstops cannot mask a regression here.
        let secret = "correcthorsebatterystaple";
        let spellings = [
            ("same line", format!("const DATABASE_PASSWORD: &str = \"{secret}\";")),
            ("wrapped after =", format!("const DATABASE_PASSWORD: &str =\n    \"{secret}\";")),
            ("let, wrapped", format!("let api_secret =\n    \"{secret}\";")),
            (
                "static with lifetime, wrapped",
                format!("static SERVICE_ACCOUNT_TOKEN: &'static str =\n    \"{secret}\";"),
            ),
        ];
        let gate = RedactionGate::from_policy(SyncPolicy {
            version: REDACT_VERSION,
            sync_enabled: true,
            allow_origins: vec!["own-repo".into()],
            allow_paths: vec![],
            max_span_bytes: 4096,
            allow_redacted: false,
        });
        for (name, span) in &spellings {
            let findings = scan_secrets(span);
            assert_eq!(
                findings.len(),
                1,
                "{name}: expected exactly one identifier-policy finding, got {findings:?}"
            );
            assert_eq!(findings[0].kind, FindingKind::RustSecretLiteral, "{name}");
            // The finding must cover the literal INCLUDING both quotes — an
            // off-by-one here would leak the first or last secret byte
            // through redaction.
            let f = &findings[0];
            assert_eq!(&span[f.start..f.start + f.len], format!("\"{secret}\""), "{name}: range");
            assert!(
                !gate.check(SpanOrigin::OwnRepo, None, span).is_syncable(),
                "{name}: must not sync"
            );
        }
    }

    /// The carried declaration must not swallow the line that follows when
    /// that line is not its literal — otherwise one wrapped declaration would
    /// blind the scanner to the next one.
    #[test]
    fn v3_carried_declaration_does_not_shadow_the_next_line() {
        // Line 1 opens a wrapped declaration; line 2 is NOT a literal (so the
        // carry must lapse) and is itself a complete secret declaration.
        let span = "let api_secret =\nconst DB_PASSWORD: &str = \"correcthorsebatterystaple\";";
        let findings = scan_secrets(span);
        assert_eq!(
            findings.len(),
            1,
            "the second declaration must still be found: {findings:?}"
        );
        assert_eq!(findings[0].kind, FindingKind::RustSecretLiteral);

        // And a lapsed carry must not fabricate a finding out of a plain
        // continuation that merely happens to be a short string.
        let benign = "let secret_len =\n    \"ab\".len();";
        assert!(
            scan_secrets(benign).is_empty(),
            "an 8-byte floor still applies on the carried path"
        );
    }

    #[test]
    fn oversized_span_blocked() {
        let policy = SyncPolicy {
            version: REDACT_VERSION,
            sync_enabled: true,
            allow_origins: vec!["own-repo".into()],
            allow_paths: vec![],
            max_span_bytes: 16,
            allow_redacted: false,
        };
        let gate = RedactionGate::from_policy(policy);
        let d = gate.check(SpanOrigin::OwnRepo, None, "fn main() { /* much bigger than 16 */ }");
        assert!(matches!(
            d,
            GateDecision::Blocked { reason: BlockReason::SpanTooLarge { .. }, .. }
        ));
    }

    #[test]
    fn allow_redacted_emits_redacted_payload() {
        let secret = "AKIAIOSFODNN7EXAMPLE";
        let policy = SyncPolicy {
            version: REDACT_VERSION,
            sync_enabled: true,
            allow_origins: vec!["own-repo".into()],
            allow_paths: vec![],
            max_span_bytes: 4096,
            allow_redacted: true,
        };
        let gate = RedactionGate::from_policy(policy);
        let span = format!("let k = \"{secret}\";");
        match gate.check(SpanOrigin::OwnRepo, None, &span) {
            GateDecision::Redacted { payload, findings } => {
                assert!(!payload.contains(secret));
                assert!(payload.contains("[REDACTED:aws-access-key]"));
                assert!(!findings.is_empty());
            }
            other => panic!("expected Redacted, got {other:?}"),
        }
    }

    #[test]
    fn unknown_policy_version_denies() {
        let dir = std::env::temp_dir().join(format!("kat_redact_t_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("policy.json");
        std::fs::write(&p, "{\"version\": 999, \"sync_enabled\": true}").unwrap();
        let loaded = RedactionGate::load(&p);
        assert!(loaded.warning.is_some());
        assert!(!loaded
            .gate
            .check(SpanOrigin::OwnRepo, None, "fn f() {}")
            .is_syncable());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn log_row_carries_fingerprint_not_secret() {
        let dir = std::env::temp_dir().join(format!("kat_redact_log_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = RedactionLog::at(dir.join("log.jsonl"));
        let secret = "AKIAIOSFODNN7EXAMPLE";
        let span = format!("let k = \"{secret}\";");
        let gate = RedactionGate::from_policy(SyncPolicy {
            version: REDACT_VERSION,
            sync_enabled: true,
            allow_origins: vec!["own-repo".into()],
            allow_paths: vec![],
            max_span_bytes: 4096,
            allow_redacted: false,
        });
        let d = gate.check(SpanOrigin::OwnRepo, None, &span);
        log.record(&d, SpanOrigin::OwnRepo, "span-1", "abc123def456").unwrap();
        let text = std::fs::read_to_string(log.path()).unwrap();
        assert!(!text.contains(secret), "log leaked the finding bytes: {text}");
        assert!(text.contains("aws-access-key"));
        assert!(text.contains("blocked"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
