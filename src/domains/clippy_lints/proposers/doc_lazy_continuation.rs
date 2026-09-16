//! `clippy::doc_lazy_continuation` + `clippy::doc_overindented_list_items` —
//! the doc-line state-machine port (Batch 151, Plan 137; the staged
//! follow-up from Research 177 adjudication #7, probe-complete before
//! authoring per P10).
//!
//! Both lints live in ONE clippy source file
//! (`clippy_lints/src/doc/lazy_continuation.rs`, fetched 2026-09-17) and
//! are checked from ONE per-line pass over the paragraph lines inside open
//! markdown containers — so ONE proposer module houses both (the
//! `print_write_with_newline` one-module-two-names precedent). The two
//! public entry points run the SAME state machine and filter the fires.
//!
//! ## The measured model (`.raw/probe151`, real clippy 0.1.98, JSON spans)
//!
//! Clippy walks pulldown-cmark events with a container stack
//! (`Blockquote` / `List(content_col)`). For every PARAGRAPH-TEXT line
//! inside open containers it checks (in order):
//!
//! 1. **Quote** — `>`-marker count in the line < open blockquote depth →
//!    `doc_lazy_continuation` "doc quote line without `>` marker", fix =
//!    insert `"> "` per missing level at the doc-text start,
//!    **`MachineApplicable`**. A line containing ANY `>` (prose like
//!    `a > b`) never fires — the count already reaches the depth.
//! 2. **List** — leading spaces ≠ the open item's content col:
//!    less → `doc_lazy_continuation` "doc list item without indentation"
//!    (insert the deficit, **`MaybeIncorrect`**, help: "add a blank line" if
//!    a separate paragraph was intended); more →
//!    `doc_overindented_list_items` "doc list item overindented" (set the
//!    indent to exactly the content col, **`MaybeIncorrect`**).
//!
//! The cooked doc text is the attr value VERBATIM — the conventional space
//! after `///` counts as leading — so `/// - item` has content col 3, and
//! every deficit/dedent in the probe matrix reconciles exactly. All three
//! fix shapes produce byte-identical output to clippy's suggestions.
//!
//! Boundary shapes measured SILENT (mirrored): continuation at exactly the
//! content col; blank-line-separated paragraphs; `# heading` after an item
//! (a block start ends the container); a fenced code block inside an item
//! AND the line after it (a code block breaks paragraph continuity, so the
//! item ends — laziness cannot attach); the list-marker lines themselves
//! (container events, never paragraph text); consecutive items (the
//! container persists, but each marker line is exempt).
//!
//! Boundary shape measured FIRING (mirrored, with eyes open): +4-over
//! continuation — `CommonMark` would call that an indented code block inside
//! the item, but pulldown (rustdoc's renderer) reads it as paragraph text,
//! so the dedent is render-stable WHERE IT MATTERS (rustdoc output is
//! unchanged); the `MaybeIncorrect` hedge is about author intent on other
//! renderers, which no parse-only tool can arbitrate — same standing as
//! clippy's own suggestion.
//!
//! ## Why FIXABLE (the `doc_markdown` precedent)
//!
//! Comment-local edits can never change compilation, so the compile gate
//! has nothing to revert — VERIFY's protection is structurally vacuous for
//! doc edits and the tier would add friction without adding a gate. The
//! real guards are the bounded state machine (fires only on the measured
//! shapes), the render-stability argument above, and the `context_note`
//! entries that surface clippy's own alternative fixes (blank line /
//! `\>` escape) at review time. Detection is a line-machine APPROXIMATION
//! of clippy's pulldown event walk — every approximation below is
//! skip-direction (we stay silent where clippy may fire), never the
//! reverse.
//!
//! ## Documented divergences (all silent-direction)
//!
//! - **Paragraph continuity required.** Both list fires demand the
//!   previous line left a paragraph open. Across a blank line clippy's
//!   event walk can still fire the `OVERindent` check (a re-opened paragraph
//!   inside the item); we stay silent — unmeasured in the probe matrix.
//! - **Quote/list mixing.** While a blockquote is open we run ONLY the
//!   quote check (clippy returns early the same way when the line carries
//!   any `>`; a list line can also interrupt and would never be quote
//!   paragraph text). A partially-marked dedent (`> > a` then `> b`) is
//!   clippy's `find('>')` dance — bounded out, silent.
//! - **One nesting axis at a time.** Nested lists track the top-of-stack
//!   content col (the measured `not indented` case reconciles exactly);
//!   lists nested inside blockquotes are invisible (quote branch owns the
//!   line).
//! - **Tabs refuse the line** (continuity broken, silent) — tab-indented
//!   markdown is ambiguous by construction.
//! - **`#[doc = "..."]` attribute form skipped** (the `+3` marker slicing
//!   only holds for comment-style docs — the `doc_markdown` precedent).

use super::helpers::{doc_lines, DocLine};
use super::SourceEdit;
use syn::File;

/// The doc-line state machine's cross-line state. One instance per doc
/// block; every field's reset discipline is in the method that owns it.
struct Machine {
    /// Inside a fenced code block (` ``` ` / `~~~`) — lines are code, no
    /// containers update, no fires.
    in_fence: bool,
    /// Open blockquote nesting depth (0 = none).
    quote_depth: usize,
    /// A quote PARAGRAPH is open: the next plain line is a
    /// lazy-continuation candidate. Broken by blank lines, fences, and
    /// block starts.
    quote_armed: bool,
    /// Open list items' content columns (innermost last). Marker lines
    /// push; dedents and block starts pop.
    list_stack: Vec<usize>,
    /// The innermost container's paragraph is open (the marker line itself
    /// opens it). Broken by blank lines, fences, and block starts.
    para_open: bool,
}

impl Machine {
    fn new() -> Self {
        Self {
            in_fence: false,
            quote_depth: 0,
            quote_armed: false,
            list_stack: Vec::new(),
            para_open: false,
        }
    }

    /// A gap between doc attrs = a new doc block: no state leaks.
    fn new_block(&mut self) {
        self.in_fence = false;
        self.quote_depth = 0;
        self.quote_armed = false;
        self.list_stack.clear();
        self.para_open = false;
    }

    /// Blank line: paragraphs end; containers persist (an indented line
    /// may re-open the item's paragraph; a `>` line may continue the
    /// quote).
    fn blank_line(&mut self) {
        self.para_open = false;
        self.quote_armed = false;
    }

    /// Quote-marker line: re-arms the quote paragraph. Quote and list
    /// tracking do not mix (the divergence note in the module doc).
    fn quote_marker_line(&mut self, trimmed: &str) {
        self.quote_depth = quote_markers(trimmed);
        self.quote_armed = true;
        self.para_open = false;
        self.list_stack.clear();
    }

    /// List-marker line: a container event, never a paragraph-text fire.
    /// `content_col` is pre-computed by the dispatcher's classification.
    fn list_marker_line(&mut self, content_col: usize, indent: usize) {
        while self.list_stack.last().is_some_and(|&cc| cc > indent) {
            self.list_stack.pop();
        }
        self.list_stack.push(content_col);
        self.para_open = true;
        self.quote_depth = 0;
        self.quote_armed = false;
    }

    /// Block starts (heading, thematic break, table row, HTML block) end
    /// every paragraph and pop containers above the line's indent. The
    /// top-level indented-code approximation only applies with no
    /// containers open (inside a list item the overindent check owns
    /// deep indents — the measured +4-over fire).
    fn block_start_line(&mut self, indent: usize) {
        self.para_open = false;
        self.quote_armed = false;
        self.quote_depth = 0;
        while self.list_stack.last().is_some_and(|&cc| cc > indent) {
            self.list_stack.pop();
        }
    }

    /// A plain text line under an open blockquote — the quote half.
    /// Clippy's count quirk: any `>` in the line reaches the depth, so
    /// prose like `a > b` never fires. Mirror exactly.
    fn quote_plain_line(&mut self, line: &DocLine<'_>, indent: usize, fires: &mut Vec<(Fire, SourceEdit)>) {
        if line.text.trim_start().contains('>') {
            return;
        }
        if self.quote_armed {
            fires.push((
                Fire::LazyQuote,
                // Clippy's measured zero-width span sits at the CONTENT
                // start (after the conventional space — probe151 c5 on
                // `/// lazy line`), so the insert lands there and
                // `/// > lazy` comes out.
                insert_at(line.base + indent, "> ".repeat(self.quote_depth)),
            ));
            // The line joins the quote paragraph: further plain lines stay
            // lazy candidates.
        }
    }

    /// A plain text line under an open list item — the list half. Fires:
    /// under-indent (lazy continuation) and over-indent (the dedent), the
    /// latter continuity-gated (the documented divergence: across a blank
    /// line clippy's event walk can still fire — we stay silent).
    fn list_plain_line(&mut self, line: &DocLine<'_>, indent: usize, fires: &mut Vec<(Fire, SourceEdit)>) {
        let Some(&cc) = self.list_stack.last() else {
            // Top-level plain text: no containers, no fires.
            self.para_open = true;
            return;
        };
        if indent >= cc {
            if indent > cc && self.para_open {
                fires.push((Fire::Overindent, dedent_to(line.base, indent, cc)));
            }
            self.para_open = true;
            return;
        }
        if self.para_open {
            // Lazy list continuation: insert the deficit at the content
            // start (after the conventional space — clippy's exact byte
            // shape).
            fires.push((
                Fire::LazyList,
                insert_at(line.base + indent, " ".repeat(cc - indent)),
            ));
            self.para_open = true;
            return;
        }
        // Paragraph broken (blank/fence/block start): the line dedents
        // back into an outer container or leaves every container. Never
        // fires here — the paragraph is not open (the same continuity
        // rule), so this is pure container bookkeeping.
        while self.list_stack.last().is_some_and(|&cc| cc > indent) {
            self.list_stack.pop();
        }
        self.para_open = true;
    }
}

/// The doc-line state machine over one file's doc comments. Returns every
/// measured fire with its edit; the two entry points filter by lint.
fn run(source: &str, file: &File) -> Vec<(Fire, SourceEdit)> {
    let mut fires = Vec::new();
    let mut m = Machine::new();
    for line in doc_lines(source, file) {
        if line.new_block {
            m.new_block();
        }
        if line.text.contains('\t') {
            // Tab-indented markdown is ambiguous — refuse the line whole.
            m.blank_line();
            continue;
        }
        let trimmed = line.text.trim_start();
        let indent = line.text.len() - trimmed.len();
        // Fence toggle / fenced-content skip (the doc_markdown
        // approximation: any ```/~~~ line toggles).
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            m.in_fence = !m.in_fence;
            m.blank_line();
            continue;
        }
        if m.in_fence {
            continue;
        }
        if trimmed.is_empty() {
            m.blank_line();
        } else if trimmed.starts_with('>') {
            m.quote_marker_line(trimmed);
        } else if let Some(content_col) = list_content_col(trimmed, indent) {
            m.list_marker_line(content_col, indent);
        } else if is_block_start(trimmed)
            || (indent >= 4 && m.list_stack.is_empty() && m.quote_depth == 0)
        {
            m.block_start_line(indent);
        } else if m.quote_depth > 0 {
            m.quote_plain_line(&line, indent, &mut fires);
        } else {
            m.list_plain_line(&line, indent, &mut fires);
        }
    }
    fires.shrink_to_fit();
    fires
}

/// Zero-width insertion edit at `at` (the `SourceEdit` insertion shape).
fn insert_at(at: usize, replacement: String) -> SourceEdit {
    SourceEdit {
        start: at,
        end: at,
        replacement,
    }
}

/// Replace a line's whole leading-whitespace run with exactly `cc` spaces
/// (the overindent dedent — byte-identical net effect to clippy's own
/// suggestion, which splices the same final string).
fn dedent_to(base: usize, indent: usize, cc: usize) -> SourceEdit {
    SourceEdit {
        start: base,
        end: base + indent,
        replacement: " ".repeat(cc),
    }
}

/// Which lint a fire belongs to (the two entry points filter on this).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fire {
    LazyQuote,
    LazyList,
    Overindent,
}

/// `clippy::doc_lazy_continuation` — both the quote and the list-lazy fires.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    run(source, file)
        .into_iter()
        .filter(|(f, _)| matches!(f, Fire::LazyQuote | Fire::LazyList))
        .map(|(_, e)| e)
        .collect()
}

/// `clippy::doc_overindented_list_items` — the over-dedent fires only.
pub(crate) fn collect_overindent_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    run(source, file)
        .into_iter()
        .filter(|(f, _)| *f == Fire::Overindent)
        .map(|(_, e)| e)
        .collect()
}

/// Count the leading blockquote markers (`>` with optional spaces between).
fn quote_markers(mut s: &str) -> usize {
    let mut depth = 0usize;
    loop {
        s = s.trim_start_matches(' ');
        if let Some(rest) = s.strip_prefix('>') {
            depth += 1;
            s = rest;
        } else {
            return depth;
        }
    }
}

/// Is this trimmed line a list marker, and if so what is the item's content
/// column? Marker forms: `- `/`* `/`+ ` and `\d{1,9}.`/`\d{1,9})` + space
/// (`CommonMark` requires the space; the content col = indent + marker + 1).
fn list_content_col(trimmed: &str, indent: usize) -> Option<usize> {
    let bytes = trimmed.as_bytes();
    let marker_len = match bytes.first()? {
        b'-' | b'*' | b'+' => 1usize,
        b'0'..=b'9' => {
            let n = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            if n > 9 {
                return None;
            }
            match bytes.get(n) {
                Some(b'.' | b')') => n + 1,
                _ => return None,
            }
        }
        _ => return None,
    };
    // The space after the marker (or end-of-line for an empty item).
    match bytes.get(marker_len) {
        None | Some(b' ') => Some(indent + marker_len + 1),
        _ => None,
    }
}

/// Block-start lines that can interrupt a paragraph and end the containers:
/// ATX headings, thematic breaks, GFM table rows, HTML blocks. List-marker
/// and quote lines never reach here (handled above).
fn is_block_start(trimmed: &str) -> bool {
    if trimmed.starts_with('#') || trimmed.starts_with('|') || trimmed.starts_with('<') {
        return true;
    }
    // Thematic break: 3+ of one of - * _ (the `- item` marker form was
    // consumed by `list_content_col` before this runs).
    let t = trimmed.trim_end();
    t.len() >= 3 && (t.starts_with("---") || t.starts_with("***") || t.starts_with("___"))
        && t.chars().all(|c| c == '-' || c == '*' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::super::FIXABLE_LINTS;

    fn lazy(code: &str) -> Option<String> {
        super::super::propose_fix("doc_lazy_continuation", code)
    }

    fn over(code: &str) -> Option<String> {
        super::super::propose_fix("doc_overindented_list_items", code)
    }

    #[test]
    fn fires_on_every_probe_firing_shape() {
        // (buggy, fixed) — every FIRES row of the probe151 matrix. The net
        // bytes match clippy's own suggestions exactly (span arithmetic in
        // the module doc).
        for (buggy, fixed) in [
            // Quote: MachineApplicable `> ` insert at the doc-text start.
            (
                "/// > The quote paragraph.\n/// lazy continuation line.\nfn q() {}",
                "/// > The quote paragraph.\n/// > lazy continuation line.\nfn q() {}",
            ),
            // Nested quote: one marker per open level.
            (
                "/// > > deep quote.\n/// lazy line.\nfn q() {}",
                "/// > > deep quote.\n/// > > lazy line.\nfn q() {}",
            ),
            // Lazy list continuation after a marker line (deficit 2).
            (
                "/// - the item text.\n/// bad continuation.\nfn l() {}",
                "/// - the item text.\n///   bad continuation.\nfn l() {}",
            ),
            // The conventional space counts as leading: text " cont" (1)
            // vs content col 3 → deficit 2.
            (
                "/// - item\n/// zero-indent cont\nfn l() {}",
                "/// - item\n///   zero-indent cont\nfn l() {}",
            ),
            // Numbered marker: ` 1. numbered` → content col 4, text " two" (1) → 3.
            (
                "/// 1. numbered item\n/// two-space cont\nfn l() {}",
                "/// 1. numbered item\n///    two-space cont\nfn l() {}",
            ),
            // Container persists across consecutive items.
            (
                "/// - first\n/// - second\n/// tail after second\nfn l() {}",
                "/// - first\n/// - second\n///   tail after second\nfn l() {}",
            ),
            // Nested list: the top-of-stack content col (5) owns the fire.
            (
                "/// - top\n///   - nested\n///     deep three\n/// not indented\nfn l() {}",
                "/// - top\n///   - nested\n///     deep three\n///     not indented\nfn l() {}",
            ),
            // Two consecutive lazy lines both fire (each joins the quote).
            (
                "/// > q\n/// lazy one\n/// lazy two\nfn q() {}",
                "/// > q\n/// > lazy one\n/// > lazy two\nfn q() {}",
            ),
        ] {
            assert_eq!(lazy(buggy).as_deref(), Some(fixed), "case: {buggy}");
        }
    }

    #[test]
    fn overindent_fires_on_every_probe_shape() {
        // (buggy, fixed) — the dedent sets the indent to exactly the
        // content col. +4 over (CommonMark code territory) still fires:
        // pulldown/rustdoc read it as paragraph text (probe151 fixture e).
        for (buggy, fixed) in [
            // 5 spaces vs content col 3 → 3.
            (
                "/// - the item text.\n///     an overindented cont.\nfn l() {}",
                "/// - the item text.\n///   an overindented cont.\nfn l() {}",
            ),
            // +4 over (7 spaces vs 3): clippy fires; the dedent is
            // render-stable in rustdoc (the module doc's eyes-open note).
            (
                "/// - item\n///       seven-space cont\nfn l() {}",
                "/// - item\n///   seven-space cont\nfn l() {}",
            ),
        ] {
            assert_eq!(over(buggy).as_deref(), Some(fixed), "case: {buggy}");
        }
    }

    #[test]
    fn silent_on_every_probe_silent_shape() {
        for code in [
            // Exact content-col continuation (3 = the measured silent row).
            "/// - item\n///   proper continuation.\nfn l() {}",
            // Blank-separated paragraph (the obvious non-fire).
            "/// > quote line\n///\n/// after blank\nfn l() {}",
            "/// - item\n///\n/// after blank\nfn l() {}",
            // Heading block start ends the item (probe151 fixture k).
            "/// - item\n/// # heading after\nfn l() {}",
            // Fenced code inside the item + the line after: all silent
            // (probe151 fixture i — the code block breaks continuity).
            "/// - item\n///   ```\n///   code line\n///   ```\n/// tail\nfn l() {}",
            // Marker lines are container events, never fires.
            "/// - first\n/// - second\nfn l() {}",
            // Prose containing `>` reaches the quote depth (clippy's count
            // quirk) — silent.
            "/// > quote\n/// a > b prose line\nfn l() {}",
            // Partially-marked dedent: bounded out (silent divergence).
            "/// > > deep\n/// > shallow\nfn l() {}",
            // Thematic break after an item: block start.
            "/// - item\n/// ---\nfn l() {}",
            // Top-level indented code (4+ with no container).
            "/// prose line.\n///\n///     code block\n/// more prose.\nfn l() {}",
            // Attribute-form docs skipped entirely (doc_markdown precedent).
            "#[doc = \" - item\\n bad cont\"]\nfn a() {}",
            // A string literal containing doc-shaped text is not a doc.
            "fn s() {\n    let x = \"/// > q\\n/// lazy\";\n}\n",
            // Empty doc lines are blanks, not fires.
            "///\n/// - item\n///\nfn l() {}",
        ] {
            assert_eq!(lazy(code), None, "lazy must stay silent: {code}");
            assert_eq!(over(code), None, "overindent must stay silent: {code}");
        }
    }

    #[test]
    fn blank_gap_overindent_stays_silent_documented_divergence() {
        // Across a blank line clippy's event walk can still fire the OVER
        // check (a re-opened paragraph inside the item); our continuity
        // rule stays silent — the documented skip-direction divergence.
        let code = "/// - item\n///\n///     deep after blank\nfn l() {}";
        assert_eq!(over(code), None);
    }

    #[test]
    fn fixes_are_fixpoint_stable() {
        for (buggy, fixed) in [
            (
                "/// > The quote paragraph.\n/// lazy continuation line.\nfn q() {}",
                "/// > The quote paragraph.\n/// > lazy continuation line.\nfn q() {}",
            ),
            (
                "/// - item\n/// zero-indent cont\nfn l() {}",
                "/// - item\n///   zero-indent cont\nfn l() {}",
            ),
            (
                "/// - the item text.\n///     an overindented cont.\nfn l() {}",
                "/// - the item text.\n///   an overindented cont.\nfn l() {}",
            ),
        ] {
            let first_lazy = lazy(buggy);
            let first_over = over(buggy);
            if let Some(out) = first_lazy.as_deref() {
                assert_eq!(lazy(out), None, "lazy fixpoint broken: {out}");
                assert_eq!(over(out), None, "over after lazy fix: {out}");
            }
            if let Some(out) = first_over.as_deref() {
                assert_eq!(over(out), None, "over fixpoint broken: {out}");
                assert_eq!(lazy(out), None, "lazy after over fix: {out}");
            }
            let _ = fixed;
        }
    }

    #[test]
    fn both_lint_names_in_the_fixable_table() {
        assert!(FIXABLE_LINTS.contains(&"doc_lazy_continuation"));
        assert!(FIXABLE_LINTS.contains(&"doc_overindented_list_items"));
    }
}
