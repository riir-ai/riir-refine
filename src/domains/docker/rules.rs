//! Rule detection over scanned stanzas — Plan 030 T1.2/T1.3.
//!
//! Detection and fix emission live together, per rule, so a rule cannot
//! detect one shape and rewrite another. Each detector returns
//! [`DockerFinding`]s; a finding carries `fix: Some(..)` only when the
//! rewrite is **bounded** — derivable from the stanza's own text with no
//! data we would have to guess.
//!
//! ## The caller gate (Proposal 006 caveat / Plan 030 T1.4)
//!
//! Several rules are only decidable with evidence from outside the
//! Dockerfile: whether a `Cargo.lock` is committed, whether the manifest is
//! a workspace root, whether an entrypoint script is visible, what the build
//! context contains. Those rules read [`ScanContext`] and **stay silent when
//! the evidence is absent** — a text-only scan never fires them. That is
//! what keeps the in-tree negative controls clean without weakening the
//! rule: with evidence supplied, it fires exactly as designed.
//!
//! ## The apt-coupling exception to the DL3059 refusal
//!
//! [`super::refuses`] forbids merging consecutive `RUN`s. `apt-update-install-same-run`
//! merges exactly two, and the distinction is not a loophole: a separate
//! `apt-get update` layer is a **correctness** bug (a cached update layer
//! pairs stale package lists with a fresh install), documented by Docker as
//! cache busting. The refusal is against merging for LAYER-COUNT reasons;
//! this merge is for CACHE-CORRECTNESS reasons, it is scoped to the
//! adjacent update/install pair alone, and `tests/docker_refusal_gate.rs`
//! pins that no other pair is ever merged.

use super::scanner::{self, logical_args, DockerfileScan, Scope, Stanza};
use super::SyntaxVersion;
use crate::edit_emitter::SourceEdit;

/// Evidence from outside the Dockerfile. Absent fields keep their rules
/// silent — never guessed.
#[derive(Debug, Clone, Default)]
pub struct ScanContext {
    /// `Some(true)` when a `Cargo.lock` is committed next to the manifest
    /// the build compiles (enables `cargo-locked-flag`).
    pub lockfile_committed: Option<bool>,
    /// `Some(true)` when the built manifest is a workspace ROOT with more
    /// than one member (enables `workspace-single-member-build`).
    pub workspace_multi_member: Option<bool>,
    /// `Some(true)` when the build context directory has a `.dockerignore`
    /// (a `false` enables `dockerignore-missing`).
    pub dockerignore_present: Option<bool>,
    /// `Some(true)` when the caller builds this file for MORE THAN ONE
    /// platform (`docker buildx --platform a,b`). Nothing in a Dockerfile
    /// states the build matrix — it is an invocation fact — so
    /// `build-stage-missing-buildplatform` is gated on it. Without the
    /// evidence the rule is silent, which is why a deliberately
    /// single-arch file (the in-tree `authority-container`) stays clean.
    pub multi_platform_build: Option<bool>,
}

/// One finding: a rule citation at one line, with the bounded fix when the
/// rule has one.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerFinding {
    /// The corpus rule this finding fronts (the citation contract).
    pub rule_id: &'static str,
    /// 1-based line in the scanned source.
    pub line: usize,
    /// The matched text, trimmed — the human-review excerpt.
    pub excerpt: String,
    /// The bounded rewrite, when the rule has one.
    pub fix: Option<SourceEdit>,
}

/// Every rule this module can emit, in documentation order.
pub const DOCKER_RULES: &[&str] = &[
    // ── Class A: hadolint/BuildKit detect, we fix ──
    "apt-lists-not-removed",
    "apt-no-install-recommends",
    "apk-add-no-cache",
    "add-local-path-to-copy",
    "cmd-entrypoint-exec-form",
    "apt-not-apt-get",
    "maintainer-to-label",
    "pipefail-before-piped-run",
    "workdir-absolute-path",
    "instruction-casing",
    "legacy-key-value-format",
    "no-empty-continuation",
    // ── Class B: both incumbents silent ──
    "apt-update-install-same-run",
    "cache-mount-missing-on-cargo-build",
    "missing-syntax-directive",
    "cargo-locked-flag",
    "workspace-single-member-build",
    "dockerignore-missing",
    "arg-secret-to-secret-mount",
    "manifest-first-layer-split",
    "cache-mount-id-scope",
    "entrypoint-shell-form-signal-block",
    "build-stage-missing-buildplatform",
    "distroless-distro-suffix-pin",
    "musl-scratch-ca-certs",
    "runtime-stage-missing-user",
    "chef-stage-toolchain-skew",
];

/// The subset carrying a bounded fix (`--fix`-eligible).
pub const FIXABLE_DOCKER: &[&str] = &[
    "apt-lists-not-removed",
    "apt-no-install-recommends",
    "apk-add-no-cache",
    "add-local-path-to-copy",
    "cmd-entrypoint-exec-form",
    "apt-not-apt-get",
    "maintainer-to-label",
    "pipefail-before-piped-run",
    "workdir-absolute-path",
    "instruction-casing",
    "legacy-key-value-format",
    "apt-update-install-same-run",
    "cache-mount-missing-on-cargo-build",
    "missing-syntax-directive",
    // Bounded, but only reachable WITH caller evidence (a committed
    // lockfile). A text-only scan never gets here.
    "cargo-locked-flag",
];

/// Scan a Dockerfile for every rule, text-only (no external evidence).
#[must_use]
pub fn scan_source(source: &str) -> Vec<DockerFinding> {
    scan_source_with(source, &ScanContext::default())
}

/// Scan with caller-supplied evidence (enables the gated rules).
#[must_use]
pub fn scan_source_with(source: &str, ctx: &ScanContext) -> Vec<DockerFinding> {
    let scan = scanner::scan(source);
    let mut out = Vec::new();
    detect(&scan, source, ctx, &mut out);
    out
}

/// The detector proper — writes into the caller's buffer so both the
/// allocating and the scratch-reusing entry points share ONE body.
fn detect(
    scan: &DockerfileScan,
    source: &str,
    ctx: &ScanContext,
    out: &mut Vec<DockerFinding>,
) {
    for (idx, st) in scan.stanzas.iter().enumerate() {
        let args = logical_args(st, source, scan.escape);
        casing(st, source, out);
        legacy_key_value(st, source, &args, out);
        empty_continuation(st, source, out);
        match st.instruction_of(source).as_str() {
            "RUN" => {
                apt_rules(st, source, &args, out);
                apk_no_cache(st, source, &args, out);
                pipefail(st, source, &args, scan, out);
                cargo_rules(st, source, &args, ctx, out);
                apt_update_split(scan, idx, source, out);
                manifest_first(scan, idx, source, out);
            }
            "ADD" => add_to_copy(st, source, &args, out),
            "CMD" | "ENTRYPOINT" => exec_form(st, source, &args, out),
            "MAINTAINER" => maintainer(st, source, &args, out),
            "WORKDIR" => workdir_absolute(st, source, &args, out),
            "ARG" => arg_secret(st, &args, out),
            _ => {}
        }
    }
    file_level(scan, source, ctx, out);
    out.sort_by(|a, b| (a.line, a.rule_id).cmp(&(b.line, b.rule_id)));
}

// ────────────────────────── Class A ──────────────────────────

/// DL3009 + DL3015 + DL3027 — apt hygiene inside one `RUN`.
fn apt_rules(st: &Stanza, source: &str, args: &str, out: &mut Vec<DockerFinding>) {
    let installs = args.contains("apt-get install") || args.contains("apt install");
    if args.contains("apt install") || args.contains("apt upgrade") {
        // DL3027: `apt` is the interactive CLI; its output is unstable
        // across releases. The rewrite is a token swap.
        if let Some(edit) = replace_first(st, source, "apt install", "apt-get install") {
            out.push(finding("apt-not-apt-get", st, source, Some(edit)));
        } else {
            out.push(finding("apt-not-apt-get", st, source, None));
        }
    }
    if !installs {
        return;
    }
    if !args.contains("--no-install-recommends") {
        let fix = insert_flag_after(st, source, "apt-get install", " --no-install-recommends")
            .or_else(|| insert_flag_after(st, source, "apt install", " --no-install-recommends"));
        out.push(finding("apt-no-install-recommends", st, source, fix));
    }
    let removes_lists = args.contains("rm -rf /var/lib/apt/lists")
        || args.contains("--mount=type=cache,target=/var/lib/apt/lists")
        || args.contains("--mount=type=cache,target=/var/cache/apt");
    if !removes_lists {
        let fix = append_to_run(st, source, " \\\n && rm -rf /var/lib/apt/lists/*");
        out.push(finding("apt-lists-not-removed", st, source, fix));
    }
}

/// DL3019 — `apk add` without `--no-cache` leaves the index in the layer.
fn apk_no_cache(st: &Stanza, source: &str, args: &str, out: &mut Vec<DockerFinding>) {
    if args.contains("apk add") && !args.contains("--no-cache") {
        let fix = insert_flag_after(st, source, "apk add", " --no-cache");
        out.push(finding("apk-add-no-cache", st, source, fix));
    }
}

/// DL3020 — `ADD` for a local path. `ADD` also fetches URLs and unpacks
/// tarballs, so the rewrite fires ONLY when neither applies.
fn add_to_copy(st: &Stanza, source: &str, args: &str, out: &mut Vec<DockerFinding>) {
    let tokens: Vec<&str> = args.split_whitespace().filter(|t| !t.starts_with("--")).collect();
    let remote = tokens
        .iter()
        .any(|t| t.starts_with("http://") || t.starts_with("https://") || t.starts_with("git@"));
    let archive = tokens.iter().any(|t| {
        [".tar", ".tgz", ".tar.gz", ".tar.bz2", ".tar.xz", ".tbz2", ".txz"]
            .iter()
            .any(|e| t.ends_with(e))
    });
    // A JSON-array form or a checksum flag means the caller knows what
    // ADD does; leave it alone.
    let deliberate = args.contains("--checksum=") || args.trim_start().starts_with('[');
    if !remote && !archive && !deliberate && !tokens.is_empty() {
        out.push(finding(
            "add-local-path-to-copy",
            st,
            source,
            Some(SourceEdit {
                start: st.start,
                end: st.start + st.keyword_len,
                replacement: "COPY".to_string(),
            }),
        ));
    }
}

/// DL3025 / `BuildKit` `JSONArgsRecommended` — shell-form `CMD`/`ENTRYPOINT`
/// makes the process a child of `sh -c`, so it never receives SIGTERM.
///
/// The rewrite fires only when the command line carries NO shell
/// metacharacter — with a pipe, `&&`, a redirect or a variable, the shell
/// is doing real work and dropping it would change behavior.
fn exec_form(st: &Stanza, source: &str, args: &str, out: &mut Vec<DockerFinding>) {
    let trimmed = args.trim();
    if trimmed.starts_with('[') || trimmed.is_empty() {
        return;
    }
    let shellish = trimmed.contains(['|', '&', ';', '>', '<', '$', '*', '`']);
    let quoted = trimmed.contains('\'') || trimmed.contains('"');
    let fix = if shellish || quoted { None } else {
            let parts: Vec<String> = trimmed
                .split_whitespace()
                .map(|t| format!("\"{t}\""))
                .collect();
            Some(SourceEdit {
                start: st.start,
                end: st.end,
                replacement: format!("{} [{}]", st.keyword(source), parts.join(", ")),
            })
        };
    let rule = match st.instruction_of(source).as_str() {
        "ENTRYPOINT" => "entrypoint-shell-form-signal-block",
        _ => "cmd-entrypoint-exec-form",
    };
    // ENTRYPOINT's signal semantics are the sharper statement of the same
    // defect; both cite the exec-form rule when a bounded fix exists.
    let rule = match (rule, fix.is_some()) {
        ("entrypoint-shell-form-signal-block", true) => "cmd-entrypoint-exec-form",
        (r, _) => r,
    };
    out.push(finding(rule, st, source, fix));
}

/// DL4000 — `MAINTAINER` is deprecated; `LABEL maintainer=` replaces it.
fn maintainer(st: &Stanza, source: &str, args: &str, out: &mut Vec<DockerFinding>) {
    let who = args.trim().trim_matches('"');
    out.push(finding(
        "maintainer-to-label",
        st,
        source,
        Some(SourceEdit {
            start: st.start,
            end: st.end,
            replacement: format!("LABEL maintainer=\"{who}\""),
        }),
    ));
}

/// DL4006 — a pipe inside `RUN` hides the exit status of every stage but
/// the last unless `pipefail` is set.
///
/// The `SHELL` insert is emitted ONLY for a debian-family stage (bash is
/// present). On alpine/busybox `/bin/sh` has no `pipefail` and the fix
/// would break the build, so the finding ships without one — hadolint's own
/// message says the same.
fn pipefail(
    st: &Stanza,
    source: &str,
    args: &str,
    scan: &DockerfileScan,
    out: &mut Vec<DockerFinding>,
) {
    let piped = args
        .split("&&")
        .any(|seg| seg.contains('|') && !seg.contains("||"));
    if !piped {
        return;
    }
    let Scope::Stage(idx) = st.scope else { return };
    if stage_shell_has_pipefail(scan, source, idx) {
        return;
    }
    let fix = if stage_is_debian_family(scan, idx) { Some(SourceEdit {
            start: st.start,
            end: st.start,
            replacement: "SHELL [\"/bin/bash\", \"-o\", \"pipefail\", \"-c\"]\n".to_string(),
        }) } else { None };
    out.push(finding("pipefail-before-piped-run", st, source, fix));
}

/// DL3000 — a relative `WORKDIR` resolves against whatever the base image
/// left behind.
fn workdir_absolute(st: &Stanza, source: &str, args: &str, out: &mut Vec<DockerFinding>) {
    let path = args.trim();
    // A variable-led path is not decidable here.
    if path.is_empty() || path.starts_with('/') || path.starts_with('$') || path.starts_with("${") {
        return;
    }
    let rel_start = st.end - path.len();
    out.push(finding(
        "workdir-absolute-path",
        st,
        source,
        Some(SourceEdit {
            start: rel_start,
            end: st.end,
            replacement: format!("/{path}"),
        }),
    ));
}

/// `BuildKit` `FileConsistentCommandCasing` — instructions are conventionally
/// upper case, and the mixed form is what makes a generated Dockerfile read
/// as machine-written.
fn casing(st: &Stanza, source: &str, out: &mut Vec<DockerFinding>) {
    if !st.is_upper(source) {
        out.push(DockerFinding {
            rule_id: "instruction-casing",
            line: st.line,
            excerpt: st.keyword(source).to_string(),
            fix: Some(SourceEdit {
                start: st.start,
                end: st.start + st.keyword_len,
                replacement: st.instruction_of(source),
            }),
        });
    }
}

/// `BuildKit` `LegacyKeyValueFormat` — `ENV key value` is the pre-1.0 form.
fn legacy_key_value(st: &Stanza, source: &str, args: &str, out: &mut Vec<DockerFinding>) {
    if st.instruction_of(source) != "ENV" && st.instruction_of(source) != "LABEL" {
        return;
    }
    let trimmed = args.trim();
    if trimmed.contains('=') || trimmed.is_empty() {
        return;
    }
    let Some((key, value)) = trimmed.split_once(char::is_whitespace) else {
        return;
    };
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    let args_start = st.end - trimmed.len();
    out.push(finding(
        "legacy-key-value-format",
        st,
        source,
        Some(SourceEdit {
            start: args_start,
            end: st.end,
            replacement: format!("{key}={}", quote_if_needed(value)),
        }),
    ));
}

/// `BuildKit` `NoEmptyContinuation` — a continuation followed by a blank line
/// is silently dropped, so the author's intent and the build diverge.
fn empty_continuation(st: &Stanza, source: &str, out: &mut Vec<DockerFinding>) {
    let text = st.text(source);
    let mut lines = text.lines().peekable();
    let mut n = 0usize;
    while let Some(line) = lines.next() {
        if line.trim_end().ends_with('\\') {
            if let Some(next) = lines.peek() {
                if next.trim().is_empty() {
                    out.push(DockerFinding {
                        rule_id: "no-empty-continuation",
                        line: st.line + n,
                        excerpt: line.trim().to_string(),
                        fix: None,
                    });
                }
            }
        }
        n += 1;
    }
}

// ────────────────────────── Class B ──────────────────────────

/// `apt-get update` alone in a `RUN`, with the install in the NEXT `RUN`.
///
/// This is the cache-staleness bug: the update layer is cached, so a later
/// rebuild pairs old package lists with a new install. It merges exactly
/// that adjacent pair — see the module doc on why it is not a DL3059
/// violation.
fn apt_update_split(scan: &DockerfileScan, idx: usize, source: &str, out: &mut Vec<DockerFinding>) {
    let st = &scan.stanzas[idx];
    let args = logical_args(st, source, scan.escape);
    let updates = args.contains("apt-get update") || args.contains("apt update");
    if !updates || args.contains("install") {
        return;
    }
    let Some(next) = scan.stanzas.get(idx + 1) else {
        return;
    };
    if next.instruction_of(source) != "RUN" || next.scope != st.scope {
        return;
    }
    let next_args = logical_args(next, source, scan.escape);
    if !next_args.contains("apt-get install") && !next_args.contains("apt install") {
        return;
    }
    let next_body = next.raw_args(source).trim_start().to_string();
    out.push(DockerFinding {
        rule_id: "apt-update-install-same-run",
        line: st.line,
        excerpt: st.text(source).lines().next().unwrap_or_default().trim().to_string(),
        fix: Some(SourceEdit {
            start: st.start,
            end: next.end,
            replacement: format!("{} \\\n && {}", st.text(source).trim_end(), next_body),
        }),
    });
}

/// A `cargo build` with no cache mount at all pays the full registry +
/// target rebuild on every source change. The registry mount is
/// path-independent in the official rust images (`CARGO_HOME` is fixed);
/// the target-dir mount is NOT (it depends on `WORKDIR`), so the fix
/// inserts the two path-independent mounts and the finding's note carries
/// the target-dir half.
fn cargo_rules(
    st: &Stanza,
    source: &str,
    args: &str,
    ctx: &ScanContext,
    out: &mut Vec<DockerFinding>,
) {
    if !args.contains("cargo build") && !args.contains("cargo install") {
        return;
    }
    if !args.contains("--mount=type=cache") {
        let after_kw = st.start + st.keyword_len;
        out.push(DockerFinding {
            rule_id: "cache-mount-missing-on-cargo-build",
            line: st.line,
            excerpt: first_line(st, source),
            fix: Some(SourceEdit {
                start: after_kw,
                end: after_kw,
                replacement: " --mount=type=cache,target=/usr/local/cargo/registry \\\n    --mount=type=cache,target=/usr/local/cargo/git \\\n   ".to_string(),
            }),
        });
    }
    // Caller-gated: `--locked` fails the build outright when no lockfile is
    // committed, so text alone must not propose it.
    if ctx.lockfile_committed == Some(true) && !args.contains("--locked") {
        let fix = insert_flag_after(st, source, "cargo build", " --locked");
        out.push(finding("cargo-locked-flag", st, source, fix));
    }
    if ctx.workspace_multi_member == Some(true)
        && !args.contains(" -p ")
        && !args.contains("--package")
        && !args.contains("--bin")
    {
        out.push(finding("workspace-single-member-build", st, source, None));
    }
}

/// `COPY . .` before the build invalidates the dependency layer on every
/// source edit.
///
/// Suppressed when the build `RUN` carries a **target-dir** cache mount:
/// the persistent target dir buys the same incrementality by another
/// route, and both in-tree production Dockerfiles do exactly that
/// deliberately. Without this guard the rule fires on known-good files.
fn manifest_first(
    scan: &DockerfileScan,
    idx: usize,
    source: &str,
    out: &mut Vec<DockerFinding>,
) {
    let st = &scan.stanzas[idx];
    let args = logical_args(st, source, scan.escape);
    if !args.contains("cargo build") {
        return;
    }
    let has_target_cache = mount_targets(&args).iter().any(|t| t.contains("target"));
    if has_target_cache {
        return;
    }
    // Already split: an EARLIER `cargo build` in the same stage is the
    // dependency pre-build, which is exactly what this rule asks for.
    let already_split = scan
        .stanzas
        .iter()
        .take(idx)
        .filter(|s| s.scope == st.scope && s.instruction_of(source) == "RUN")
        .any(|s| logical_args(s, source, scan.escape).contains("cargo build"));
    if already_split {
        return;
    }
    let whole_context_copy = scan
        .stanzas
        .iter()
        .take(idx)
        .filter(|s| s.scope == st.scope && s.instruction_of(source) == "COPY")
        .any(|s| {
            let a = s.raw_args(source);
            let first = a.split_whitespace().find(|t| !t.starts_with("--"));
            matches!(first, Some("." | "./"))
        });
    if whole_context_copy {
        out.push(finding("manifest-first-layer-split", st, source, None));
    }
}

/// Name fragments that mark a build `ARG` as carrying a secret.
const SECRET_NAME_MARKERS: &[&str] = &[
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "PASSWD",
    "APIKEY",
    "API_KEY",
    "PRIVATE_KEY",
];

/// A build `ARG` whose NAME says secret. The fix is a caller change (the
/// `docker build --secret` invocation), so this is suggest-only by
/// construction — rewriting the Dockerfile alone would break the build.
fn arg_secret(st: &Stanza, args: &str, out: &mut Vec<DockerFinding>) {
    let name = args
        .split('=')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_uppercase();
    if SECRET_NAME_MARKERS.iter().any(|m| name.contains(m)) {
        out.push(DockerFinding {
            rule_id: "arg-secret-to-secret-mount",
            line: st.line,
            excerpt: name,
            fix: None,
        });
    }
}

/// File-level rules: the ones whose subject is the file or the stage graph
/// rather than a single stanza.
fn file_level(
    scan: &DockerfileScan,
    source: &str,
    ctx: &ScanContext,
    out: &mut Vec<DockerFinding>,
) {
    // ── missing-syntax-directive ──
    // Scoped HARD: it fires only when the file USES an above-builtin
    // feature. Every in-tree Dockerfile omits the directive and is correct
    // to; an unconditional rule would fire on all of them and teach the
    // corpus that correct files are buggy.
    let declared = scan.syntax().map(SyntaxVersion::from_directive);
    let needed = required_syntax(scan, source);
    if needed > SyntaxVersion::Builtin && declared.unwrap_or(SyntaxVersion::Builtin) < needed {
        let directive = needed.directive().unwrap_or("docker/dockerfile:1.4");
        out.push(DockerFinding {
            rule_id: "missing-syntax-directive",
            line: 1,
            excerpt: format!("file uses a feature requiring {directive}"),
            fix: Some(SourceEdit {
                start: 0,
                end: 0,
                replacement: format!("# syntax={directive}\n"),
            }),
        });
    }

    // ── dockerignore-missing (caller-gated) ──
    if ctx.dockerignore_present == Some(false) {
        out.push(DockerFinding {
            rule_id: "dockerignore-missing",
            line: 1,
            excerpt: "build context has no .dockerignore".to_string(),
            fix: None,
        });
    }

    // ── cache-mount-id-scope ──
    // Two stages mounting the SAME cache target without an `id=` share one
    // cache across different toolchains/targets — the silent corruption
    // shape. Suggest-only: the right id is the author's naming call.
    let mut seen: Vec<(usize, String)> = Vec::new();
    for st in &scan.stanzas {
        let Scope::Stage(stage) = st.scope else { continue };
        for (id, target) in cache_mounts(&logical_args(st, source, scan.escape)) {
            // An explicitly-scoped mount is the FIX, not the defect.
            if id.is_some() {
                continue;
            }
            if seen.iter().any(|(s, t)| *s != stage && *t == target) {
                out.push(finding("cache-mount-id-scope", st, source, None));
                break;
            }
            seen.push((stage, target));
        }
    }

    runtime_stage_rules(scan, source, out);

    // ── build-stage-missing-buildplatform (annotation, never a rewrite) ──
    // Emitted only for a MULTI-stage file whose builder compiles and whose
    // FROM carries no platform intent at all. Both strategies are valid
    // (caveat 6) — this is a decision-table citation, not a defect claim.
    if scan.stages.len() > 1 && ctx.multi_platform_build == Some(true) {
        for stage in &scan.stages {
            let compiles = scan
                .stage_stanzas(stage.index)
                .any(|s| logical_args(s, source, scan.escape).contains("cargo build"));
            let arch_aware = scan.stanzas.iter().any(|s| {
                let a = s.raw_args(source);
                a.contains("TARGETPLATFORM") || a.contains("TARGETARCH") || a.contains("BUILDPLATFORM")
            });
            if compiles && stage.platform.is_none() && !arch_aware {
                out.push(DockerFinding {
                    rule_id: "build-stage-missing-buildplatform",
                    line: scan.stanzas[stage.stanza_idx].line,
                    excerpt: stage.base.clone(),
                    fix: None,
                });
            }
        }
    }

    // ── chef-stage-toolchain-skew ──
    // A cargo-chef cook stage on a different base tag than the build stage
    // recooks the whole dependency layer on every build — the exact cost
    // chef exists to remove.
    let cook_stages: Vec<&scanner::Stage> = scan
        .stages
        .iter()
        .filter(|s| {
            scan.stage_stanzas(s.index)
                .any(|st| logical_args(st, source, scan.escape).contains("cargo chef"))
        })
        .collect();
    if cook_stages.len() > 1 {
        let first = &cook_stages[0].base;
        if let Some(skewed) = cook_stages.iter().find(|s| &s.base != first) {
            out.push(DockerFinding {
                rule_id: "chef-stage-toolchain-skew",
                line: scan.stanzas[skewed.stanza_idx].line,
                excerpt: skewed.base.clone(),
                fix: None,
            });
        }
    }
}


/// Rules whose subject is the RUNTIME stage: the base it ships and what it
/// declares about identity and trust roots.
fn runtime_stage_rules(scan: &DockerfileScan, source: &str, out: &mut Vec<DockerFinding>) {
    let Some(rt) = scan.runtime_stage(source) else {
        return;
    };
        let stage = &scan.stages[rt];
        let base = stage.base.to_ascii_lowercase();
        let line = scan.stanzas[stage.stanza_idx].line;
        if base.starts_with("gcr.io/distroless/") && !base.contains("-debian") && !base.contains('@')
        {
            out.push(DockerFinding {
                rule_id: "distroless-distro-suffix-pin",
                line,
                excerpt: stage.base.clone(),
                fix: None,
            });
        }
        if !base.is_empty()
            && base != "scratch"
            && !scan.stage_stanzas(rt).any(|s| s.instruction_of(source) == "USER")
        {
            out.push(DockerFinding {
                rule_id: "runtime-stage-missing-user",
                line,
                excerpt: stage.base.clone(),
                fix: None,
            });
        }
        if base == "scratch" {
            let copies_certs = scan
                .stage_stanzas(rt)
                .any(|s| s.raw_args(source).contains("ca-certificates"));
            if !copies_certs {
                out.push(DockerFinding {
                    rule_id: "musl-scratch-ca-certs",
                    line,
                    excerpt: "FROM scratch".to_string(),
                    fix: None,
                });
            }
        }
}

// ────────────────────────── helpers ──────────────────────────

/// The highest frontend feature the file actually uses.
fn required_syntax(scan: &DockerfileScan, source: &str) -> SyntaxVersion {
    let mut needed = SyntaxVersion::Builtin;
    for st in &scan.stanzas {
        let text = st.text(source);
        if text.contains("--exclude=") || text.contains("--exclude ") {
            needed = needed.max(SyntaxVersion::V1_19);
        }
        if scanner::heredoc_tag(text.lines().next().unwrap_or_default()).is_some()
            || text.contains("--link")
        {
            needed = needed.max(SyntaxVersion::V1_4);
        }
    }
    needed
}

/// `target=` values of every cache mount in the stanza's args.
fn mount_targets(args: &str) -> Vec<String> {
    cache_mounts(args).into_iter().map(|(_, t)| t).collect()
}

/// `(id, target)` for every cache mount. `id` is `None` when the mount is
/// UNSCOPED — which is the whole subject of `cache-mount-id-scope`: a mount
/// that already carries an explicit `id=` is scoped and must never fire.
fn cache_mounts(args: &str) -> Vec<(Option<String>, String)> {
    args.split_whitespace()
        .filter(|t| t.starts_with("--mount=") && t.contains("type=cache"))
        .filter_map(|t| {
            let target = t
                .split(',')
                .find_map(|kv| kv.strip_prefix("target=").map(str::to_string))?;
            let id = t
                .split(',')
                .find_map(|kv| kv.strip_prefix("id=").map(str::to_string));
            Some((id, target))
        })
        .collect()
}

fn stage_is_debian_family(scan: &DockerfileScan, stage: usize) -> bool {
    let base = scan.stages[stage].base.to_ascii_lowercase();
    ["debian", "ubuntu", "bookworm", "trixie", "bullseye", "slim", "buildpack-deps"]
        .iter()
        .any(|m| base.contains(m))
        && !base.contains("alpine")
}

fn stage_shell_has_pipefail(scan: &DockerfileScan, source: &str, stage: usize) -> bool {
    scan.stage_stanzas(stage)
        .any(|s| s.instruction_of(source) == "SHELL" && s.raw_args(source).contains("pipefail"))
}

fn first_line(st: &Stanza, source: &str) -> String {
    st.text(source).lines().next().unwrap_or_default().trim().to_string()
}

fn finding(
    rule_id: &'static str,
    st: &Stanza,
    source: &str,
    fix: Option<SourceEdit>,
) -> DockerFinding {
    DockerFinding {
        rule_id,
        line: st.line,
        excerpt: first_line(st, source),
        fix,
    }
}

/// Insert `flag` immediately after the first occurrence of `needle` in the
/// stanza's source text.
fn insert_flag_after(
    st: &Stanza,
    source: &str,
    needle: &str,
    flag: &str,
) -> Option<SourceEdit> {
    let text = st.text(source);
    let rel = text.find(needle)?;
    let at = st.start + rel + needle.len();
    Some(SourceEdit {
        start: at,
        end: at,
        replacement: flag.to_string(),
    })
}

/// Replace the first occurrence of `needle` inside the stanza.
fn replace_first(st: &Stanza, source: &str, needle: &str, with: &str) -> Option<SourceEdit> {
    let text = st.text(source);
    let rel = text.find(needle)?;
    Some(SourceEdit {
        start: st.start + rel,
        end: st.start + rel + needle.len(),
        replacement: with.to_string(),
    })
}

/// Append a shell fragment at the end of a `RUN` stanza.
fn append_to_run(st: &Stanza, source: &str, tail: &str) -> Option<SourceEdit> {
    let text = st.text(source);
    // Refuse when the stanza carries a heredoc — appending after the
    // terminator would land outside the script.
    if text.lines().any(|l| scanner::heredoc_tag(l).is_some()) {
        return None;
    }
    Some(SourceEdit {
        start: st.end,
        end: st.end,
        replacement: tail.to_string(),
    })
}

fn quote_if_needed(value: &str) -> String {
    if value.contains(char::is_whitespace) && !value.starts_with('"') { format!("\"{value}\"") } else { value.to_string() }
}

// ────────────────────── G4: the reusable scan path ──────────────────────

/// Reusable scan buffers — the [`SuggestScratch`](crate::draft::SuggestScratch)
/// precedent applied to the docker scan path (Proposal 006 G4).
///
/// Each scan `clear()`s and refills; capacity persists, so the steady state
/// performs no Vec growth. `capacities()` is the observable the G4 test
/// asserts stability on.
///
/// ## What still allocates, stated precisely
///
/// One `String` per emitted **finding** (its `excerpt`) and one per emitted
/// **fix** (the replacement text). Those are the OUTPUT, they are
/// proportional to findings rather than to file size, and a clean file
/// therefore allocates nothing at all in the scan. The per-STANZA
/// allocation is gone: [`Stanza`] carries a keyword LENGTH, not a `String`.
#[derive(Debug, Default)]
pub struct DockerScratch {
    scan: super::scanner::DockerfileScan,
    findings: Vec<DockerFinding>,
}

impl DockerScratch {
    /// `(stanza capacity, finding capacity)` — the G4 observable.
    #[must_use]
    pub fn capacities(&self) -> (usize, usize) {
        (self.scan.stanzas.capacity(), self.findings.capacity())
    }
}

/// Scan into reusable buffers, returning the findings borrowed from the
/// scratch (the `suggest_into` contract).
pub fn scan_into<'s>(
    source: &str,
    ctx: &ScanContext,
    scratch: &'s mut DockerScratch,
) -> &'s [DockerFinding] {
    scanner::scan_into(source, &mut scratch.scan);
    scratch.findings.clear();
    detect(&scratch.scan, source, ctx, &mut scratch.findings);
    &scratch.findings
}
