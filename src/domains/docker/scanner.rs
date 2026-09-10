//! Dockerfile **stanza scanner** — Plan 030 T0.2.
//!
//! The `docker` domain has no `syn`. This module is the domain's parser: a
//! hand-written line scanner that turns a Dockerfile into instruction
//! stanzas with byte-exact spans, so proposers can emit [`SourceEdit`]s
//! through the same span-preserving emitter every other domain uses.
//!
//! It is deliberately **not** a Dockerfile evaluator. It answers exactly the
//! questions the rules ask: where does each instruction begin and end, which
//! build stage is it in, what comment block precedes it, and what does its
//! body look like with continuations joined and inner comments stripped.
//!
//! ## Precision-first (the Issue 025 WGSL-scan precedent)
//!
//! The WGSL scanner's rule holds here: a false positive on a clean file
//! poisons the mining loop's garbage-in guard, so the scanner is biased to
//! miss rather than to over-fire. Three shapes are handled exactly because
//! getting them wrong silently mis-spans a stanza:
//!
//! - **line continuations** (`\` by default, `` ` `` under an `escape`
//!   parser directive), including comment lines *inside* a continuation,
//!   which the real parser strips;
//! - **heredocs** (`<<EOF` / `<<-'EOF'`, `BuildKit` >= 1.4) — the body is part
//!   of the stanza even though its lines carry no continuation marker;
//! - **parser directives** (`# syntax=`, `# escape=`, `# check=`), which are
//!   only directives while they precede every comment and instruction.
//!
//! ## Scope
//!
//! Instructions before the first `FROM` are [`Scope::Global`] (where `ARG`
//! legally lives); everything after belongs to the `FROM` stage that opened
//! it. Stage identity is what makes "the runtime stage" a question the rules
//! can ask at all.

use std::collections::BTreeSet;

/// The 18 Dockerfile instructions (the builtin frontend's set).
///
/// A line whose first token is not one of these does not open a stanza —
/// the precision guard that keeps prose, shell fragments and heredoc bodies
/// from being read as instructions.
pub const INSTRUCTIONS: &[&str] = &[
    "ADD",
    "ARG",
    "CMD",
    "COPY",
    "ENTRYPOINT",
    "ENV",
    "EXPOSE",
    "FROM",
    "HEALTHCHECK",
    "LABEL",
    "MAINTAINER",
    "ONBUILD",
    "RUN",
    "SHELL",
    "STOPSIGNAL",
    "USER",
    "VOLUME",
    "WORKDIR",
];

/// Which build stage a stanza belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Before the first `FROM` — the global `ARG` region.
    Global,
    /// Inside the stage with this 0-based index.
    Stage(usize),
}

/// One parser directive (`# syntax=docker/dockerfile:1.7`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserDirective {
    /// Lowercased key (`syntax`, `escape`, `check`).
    pub key: String,
    /// Everything right of the `=`, trimmed.
    pub value: String,
    /// 1-based line.
    pub line: usize,
}

/// One instruction stanza with byte-exact spans into the scanned source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stanza {
    /// Byte length of the instruction keyword as written. The keyword text
    /// itself is `source[start..start + keyword_len]` — a LENGTH rather
    /// than a `String` so scanning a file allocates nothing per stanza
    /// (the G4 contract; the casing rule still sees the original spelling
    /// through [`Stanza::keyword`]).
    pub keyword_len: usize,
    /// Byte offset of the keyword's first byte.
    pub start: usize,
    /// Byte offset one past the stanza's last byte (excludes the trailing
    /// newline).
    pub end: usize,
    /// 1-based line of the keyword.
    pub line: usize,
    /// 1-based line of the stanza's last line.
    pub end_line: usize,
    /// The stage this stanza belongs to.
    pub scope: Scope,
    /// Byte offset of the first byte of the comment block immediately above
    /// (no blank line between) — the retrieval context Proposal 006 calls
    /// load-bearing. `None` when no comment precedes.
    pub comment_start: Option<usize>,
}

impl Stanza {
    /// The instruction keyword exactly as written (`RUN`, `run`, `Run`).
    #[must_use]
    pub fn keyword<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.start + self.keyword_len]
    }

    /// The instruction keyword uppercased (the match key).
    ///
    /// Takes the source because the stanza stores a length, not the text.
    #[must_use]
    pub fn instruction_of(&self, source: &str) -> String {
        self.keyword(source).to_ascii_uppercase()
    }

    /// Whether the keyword is written in canonical upper case.
    #[must_use]
    pub fn is_upper(&self, source: &str) -> bool {
        self.keyword(source)
            .bytes()
            .all(|b| !b.is_ascii_lowercase())
    }

    /// The stanza's exact source text (keyword included).
    #[must_use]
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    /// The stanza's argument text — everything after the keyword, verbatim
    /// (continuations and inner comments intact).
    #[must_use]
    pub fn raw_args<'a>(&self, source: &'a str) -> &'a str {
        let text = self.text(source);
        text[self.keyword_len..].trim_start_matches([' ', '\t'])
    }
}

/// One `FROM` stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    /// 0-based stage index.
    pub index: usize,
    /// The base image reference as written (`rust:1.97.0-alpine`, `scratch`,
    /// `$BUILDER`). Empty when the `FROM` line is malformed.
    pub base: String,
    /// The `AS <name>` alias, lowercased (stage names are case-insensitive).
    pub alias: Option<String>,
    /// The `--platform=` flag value, if written.
    pub platform: Option<String>,
    /// Index into [`DockerfileScan::stanzas`] of the opening `FROM`.
    pub stanza_idx: usize,
}

/// The scan result.
#[derive(Debug, Clone, Default)]
pub struct DockerfileScan {
    /// Every instruction stanza, in source order.
    pub stanzas: Vec<Stanza>,
    /// Parser directives (only those in the leading directive block).
    pub directives: Vec<ParserDirective>,
    /// Every `FROM` stage, in source order.
    pub stages: Vec<Stage>,
    /// The escape character in force (`\` unless an `escape` directive says
    /// otherwise).
    pub escape: char,
}

impl DockerfileScan {
    /// The `syntax` directive's value, if the file pins one.
    #[must_use]
    pub fn syntax(&self) -> Option<&str> {
        self.directives
            .iter()
            .find(|d| d.key == "syntax")
            .map(|d| d.value.as_str())
    }

    /// Stanzas belonging to one stage, in source order.
    pub fn stage_stanzas(&self, stage: usize) -> impl Iterator<Item = &Stanza> {
        self.stanzas
            .iter()
            .filter(move |s| s.scope == Scope::Stage(stage))
    }

    /// The **runtime stage**: the last stage that no other stage copies
    /// from (`COPY --from=`) and that no other stage bases on (`FROM x AS`
    /// then `FROM <alias>`).
    ///
    /// Returns `None` for a single-stage file — deliberately: "the runtime
    /// stage" is a multi-stage question, and single-stage files are a
    /// different rule's territory.
    #[must_use]
    pub fn runtime_stage(&self, source: &str) -> Option<usize> {
        if let 0 | 1 = self.stages.len() {
            None
        } else {
            let consumed = self.consumed_stage_names(source);
            self.stages
                .iter()
                .rev()
                .find(|st| {
                    !st.alias.as_deref().is_some_and(|a| consumed.contains(a))
                        && !consumed.contains(&st.index.to_string())
                })
                .map(|st| st.index)
        }
    }

    /// Stage names/indices referenced by `--from=` or by a later `FROM`.
    fn consumed_stage_names(&self, source: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for s in &self.stanzas {
            let args = s.raw_args(source);
            match s.instruction_of(source).as_str() {
                "COPY" | "ADD" | "RUN" => {
                    for flag in args.split_whitespace().filter(|t| t.starts_with("--")) {
                        if let Some(v) = flag.strip_prefix("--from=") {
                            out.insert(v.to_ascii_lowercase());
                        }
                        // `RUN --mount=type=bind,from=builder,...`
                        if flag.starts_with("--mount=") {
                            for kv in flag.split(',') {
                                if let Some(v) = kv.strip_prefix("from=") {
                                    out.insert(v.to_ascii_lowercase());
                                }
                            }
                        }
                    }
                }
                "FROM" => {
                    if let Some(base) = args.split_whitespace().find(|t| !t.starts_with("--")) {
                        out.insert(base.to_ascii_lowercase());
                    }
                }
                _ => {}
            }
        }
        out
    }
}

/// Join a stanza's argument text into one logical line: continuations
/// collapsed to a single space, comment lines inside the continuation
/// dropped (exactly what the real parser does).
///
/// Heredoc bodies are **kept**, newline-separated, after the joined header —
/// a rule that inspects `RUN <<EOF` needs the script.
#[must_use]
pub fn logical_args(stanza: &Stanza, source: &str, escape: char) -> String {
    let mut out = String::with_capacity(stanza.end - stanza.start);
    let mut in_heredoc = false;
    for (i, raw) in stanza.raw_args(source).lines().enumerate() {
        let line = raw.trim_end();
        match () {
            () if in_heredoc => {
                out.push('\n');
                out.push_str(line);
            }
            // A comment line inside a continuation is stripped by the parser.
            () if i > 0 && line.trim_start().starts_with('#') => {}
            () => {
                let (body, continues) = match line.strip_suffix(escape) {
                    Some(b) => (b, true),
                    None => (line, false),
                };
                if i > 0 && !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(body.trim());
                if !continues && heredoc_tag(line).is_some() {
                    in_heredoc = true;
                }
            }
        }
    }
    out
}

/// The heredoc terminator a line opens (`<<EOF`, `<<-'EOF'`, `<<"EOF"`), if
/// any. Returns `(tag, strip_tabs)`.
#[must_use]
pub fn heredoc_tag(line: &str) -> Option<(String, bool)> {
    let idx = line.find("<<")?;
    let rest = &line[idx + 2..];
    let (strip_tabs, rest) = match rest.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, rest),
    };
    let rest = rest.trim_start();
    let (quote, rest) = match rest.chars().next() {
        Some(q @ ('\'' | '"')) => (Some(q), &rest[1..]),
        _ => (None, rest),
    };
    let tag: String = match quote {
        Some(q) => rest.chars().take_while(|c| *c != q).collect(),
        None => rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect(),
    };
    if tag.is_empty() {
        None
    } else {
        Some((tag, strip_tabs))
    }
}

/// Scan a Dockerfile into stanzas, directives and stages.
#[must_use]
pub fn scan(source: &str) -> DockerfileScan {
    let mut out = DockerfileScan::default();
    scan_into(source, &mut out);
    out
}

/// Scan into an EXISTING [`DockerfileScan`], reusing its buffers.
///
/// The G4 path: the three `Vec`s are cleared and refilled, so their
/// capacity persists across calls and a steady-state scan performs no
/// growth. `scan` is this with a fresh buffer.
pub fn scan_into(source: &str, out: &mut DockerfileScan) {
    out.stanzas.clear();
    out.directives.clear();
    out.stages.clear();
    out.escape = '\\';
    scan_inner(source, out);
}

fn scan_inner(source: &str, out: &mut DockerfileScan) {
    // Line table: (byte start, text without the line terminator).
    let mut lines: Vec<(usize, &str)> = Vec::new();
    let mut off = 0usize;
    for raw in source.split_inclusive('\n') {
        lines.push((off, raw.trim_end_matches(['\n', '\r'])));
        off += raw.len();
    }

    // ── Leading parser-directive block ──────────────────────────────────
    // A directive is only a directive while every preceding line is one.
    let mut i = 0usize;
    while i < lines.len() {
        let text = lines[i].1.trim();
        match parse_directive(text) {
            Some((key, value)) => {
                if key == "escape" {
                    if let Some(c) = value.chars().next() {
                        out.escape = c;
                    }
                }
                out.directives.push(ParserDirective {
                    key,
                    value,
                    line: i + 1,
                });
                i += 1;
            }
            None => break,
        }
    }

    // ── Stanzas ─────────────────────────────────────────────────────────
    let escape = out.escape;
    let mut comment_block: Option<usize> = None;
    let mut current_stage: Option<usize> = None;
    let mut i = 0usize;
    while i < lines.len() {
        let (start, text) = lines[i];
        let trimmed = text.trim_start();

        // Blank line: breaks the comment block's adjacency.
        if trimmed.is_empty() {
            comment_block = None;
            i += 1;
            continue;
        }
        // Comment line: opens or extends the pending block.
        if trimmed.starts_with('#') {
            if comment_block.is_none() {
                comment_block = Some(start);
            }
            i += 1;
            continue;
        }

        let Some(keyword) = leading_instruction(trimmed) else {
            // Not an instruction — stray content. Do not guess.
            comment_block = None;
            i += 1;
            continue;
        };

        let kw_start = start + (text.len() - trimmed.len());
        let (last, end) = stanza_extent(&lines, i, escape);

        let scope = match current_stage {
            Some(s) => Scope::Stage(s),
            None => Scope::Global,
        };
        let stanza = Stanza {
            keyword_len: keyword.len(),
            start: kw_start,
            end,
            line: i + 1,
            end_line: last + 1,
            scope,
            comment_start: comment_block,
        };

        if stanza.instruction_of(source) == "FROM" {
            let index = out.stages.len();
            let stage = parse_from(&stanza, source, index, out.stanzas.len());
            current_stage = Some(index);
            out.stages.push(stage);
            // The FROM itself belongs to the stage it opens.
            out.stanzas.push(Stanza {
                scope: Scope::Stage(index),
                ..stanza
            });
        } else {
            out.stanzas.push(stanza);
        }

        comment_block = None;
        i = last + 1;
    }
}

/// `# key=value` in the leading block, or `None`.
fn parse_directive(text: &str) -> Option<(String, String)> {
    let body = text.strip_prefix('#')?.trim();
    let (key, value) = body.split_once('=')?;
    let key = key.trim().to_ascii_lowercase();
    match key.as_str() {
        "syntax" | "escape" | "check" => Some((key, value.trim().to_string())),
        _ => None,
    }
}

/// The instruction keyword a line opens, if it opens one.
fn leading_instruction(trimmed: &str) -> Option<&str> {
    let token = trimmed.split([' ', '\t']).next()?;
    let upper = token.to_ascii_uppercase();
    INSTRUCTIONS
        .binary_search(&upper.as_str())
        .ok()
        .map(|_| token)
}

/// The stanza's last line index and end byte offset, following
/// continuations and heredoc bodies.
fn stanza_extent(lines: &[(usize, &str)], first: usize, escape: char) -> (usize, usize) {
    let mut last = first;
    let mut pending_heredocs: Vec<(String, bool)> = Vec::new();
    let mut i = first;

    loop {
        let (_, text) = lines[i];
        let trimmed_end = text.trim_end();

        // Heredoc bodies opened by this line are consumed after the
        // continuation chain ends (the header may itself continue).
        if !trimmed_end.trim_start().starts_with('#') {
            if let Some(tag) = heredoc_tag(trimmed_end) {
                pending_heredocs.push(tag);
            }
        }

        let continues = trimmed_end.ends_with(escape);
        if continues && i + 1 < lines.len() {
            i += 1;
            last = i;
        } else {
            break;
        }
    }

    // Consume heredoc bodies, in opening order.
    for (tag, strip_tabs) in pending_heredocs {
        let mut j = last + 1;
        while j < lines.len() {
            let text = lines[j].1;
            let candidate = if strip_tabs {
                text.trim_start_matches('\t')
            } else {
                text
            };
            last = j;
            if candidate.trim_end() == tag {
                break;
            }
            j += 1;
        }
    }

    let (start, text) = lines[last];
    (last, start + text.len())
}

/// Parse a `FROM` stanza into a [`Stage`].
fn parse_from(stanza: &Stanza, source: &str, index: usize, stanza_idx: usize) -> Stage {
    let args = logical_args(stanza, source, '\\');
    let mut platform = None;
    let mut base = String::new();
    let mut alias = None;
    let mut tokens = args.split_whitespace().peekable();
    while let Some(tok) = tokens.next() {
        match () {
            () if tok.starts_with("--platform=") => {
                platform = Some(tok["--platform=".len()..].to_string());
            }
            () if tok.starts_with("--") => {}
            () if base.is_empty() => base = tok.to_string(),
            () if tok.eq_ignore_ascii_case("AS") => {
                alias = tokens.next().map(str::to_ascii_lowercase);
            }
            () => {}
        }
    }
    Stage {
        index,
        base,
        alias,
        platform,
        stanza_idx,
    }
}
