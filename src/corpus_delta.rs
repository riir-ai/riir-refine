//! Issue 116 — the corpus-truth delta surface: lints the FIX lane can write
//! that the SUGGEST lane cannot rank.
//!
//! The two lanes read two corpus truths **by design**. The fix lane consumes
//! the compiled-in `FIXABLE_LINTS ∪ VERIFY_LINTS`, which is why a
//! freshly-landed lint heals the moment it ships; the suggest lane ranks
//! against the retrieval rule book, whose per-domain corpus a leased epoch
//! REPLACES wholesale ([`crate::draft::latent_matcher::DomainOverlay`]), so
//! it lags by the lease cadence. That split is load-bearing and this module
//! does not touch it — **it names the seam, it does not close it.**
//!
//! The gap it closes is DISCOVERABILITY. `[no corpus rule]` (Issue 113) is
//! the right per-span signal, but a reader meets it one diagnostic at a time
//! and it reads as *"not supported"* when it can mean *"not leased yet"* —
//! and the two demand opposite actions. Nothing aggregated it, so the only
//! way to learn the delta was to switch modes and notice the difference.
//!
//! ## The partition, and why it is a partition and not a filter
//!
//! Every `anchored_missing` lint lands in exactly one bucket:
//!
//! - **[`FixTier::Fixable`] / [`FixTier::Verify`]** — the fix lane HAS a
//!   proposer. The suggest output is stale, not authoritative:
//!   `cargo refine --fix` would rewrite this today. This is the freshness
//!   delta, and the tier is kept because the two are not interchangeable
//!   advice — a `Verify` row only fires under `--fix --verify`, so telling
//!   a reader "just run --fix" would be wrong for half the set.
//! - **[`unfixable`](CorpusDelta::unfixable)** — neither lane covers it.
//!   This is the genuine coverage gap and it is the mining intake's input
//!   (Issue 119 quantifies the same thing against clippy's whole surface).
//!
//! Reporting only the first bucket would be the more flattering number and
//! the less useful one: a reader needs to know which of the two messages
//! `[no corpus rule]` is delivering, and the answer is per-lint.
//!
//! ## Never noise
//!
//! [`CorpusDelta::summary_line`] returns `None` when the delta is empty, and
//! the delta is EXPECTED to be empty most of the time — it is transient by
//! design, bounded by the lease cadence. A line that printed `0` every run
//! would be the cries-wolf instrument this repo's own docs warn gets
//! ignored, and it would do so on the runs where the signal matters least.

use std::collections::BTreeMap;

/// Which fix-lane tier carries the lint. Kept apart because the remedy
/// differs: `Fixable` runs on a plain `--fix`, `Verify` needs
/// `--fix --verify` (and the `fix_verify` feature behind it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FixTier {
    Fixable,
    Verify,
}

impl FixTier {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fixable => "fixable",
            Self::Verify => "verify",
        }
    }

    /// The invocation that would actually apply this row.
    #[must_use]
    pub const fn remedy(self) -> &'static str {
        match self {
            Self::Fixable => "cargo refine --fix",
            Self::Verify => "cargo refine --fix --verify",
        }
    }
}

/// One delta row: a lint the fix lane can write and the suggest lane could
/// not rank, with the number of spans that hit it this run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeltaRow {
    pub lint: String,
    pub tier: FixTier,
    /// Span occurrences this run. A lint firing at five spans is one row
    /// with `spans = 5` — the ROW is the actionable unit, the count is the
    /// blast radius.
    pub spans: usize,
}

/// The aggregated delta for one run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CorpusDelta {
    /// Fix-lane-covered rows, hottest first.
    pub rows: Vec<DeltaRow>,
    /// Distinct `anchored_missing` lints seen this run (both buckets).
    pub missing_lints: usize,
    /// Span occurrences across both buckets — matches the run summary's
    /// existing `no-corpus-rule` count, so the two can be read together.
    pub missing_spans: usize,
    /// Distinct lints NEITHER lane covers — the real coverage gap.
    pub unfixable: usize,
    /// Span occurrences behind [`Self::unfixable`].
    pub unfixable_spans: usize,
}

impl CorpusDelta {
    /// Distinct lints in the freshness delta.
    #[must_use]
    pub fn delta_lints(&self) -> usize {
        self.rows.len()
    }

    /// Span occurrences in the freshness delta.
    #[must_use]
    pub fn delta_spans(&self) -> usize {
        self.rows.iter().map(|r| r.spans).sum()
    }

    /// The one-line run-summary addition — `None` when the delta is empty,
    /// which is the expected state. See the module note on noise.
    #[must_use]
    pub fn summary_line(&self) -> Option<String> {
        if self.rows.is_empty() {
            return None;
        }
        let lints = self.delta_lints();
        let lint_pl = if lints == 1 { "lint" } else { "lints" };
        let spans = self.delta_spans();
        let span_pl = if spans == 1 { "span" } else { "spans" };
        let verify = self
            .rows
            .iter()
            .filter(|r| r.tier == FixTier::Verify)
            .count();
        let tier_note = if verify == 0 {
            String::new()
        } else if verify == lints {
            " (all --verify tier)".to_string()
        } else {
            format!(" ({verify} --verify tier)")
        };
        Some(format!(
            "corpus delta: {lints} {lint_pl} at {spans} {span_pl} marked [no corpus rule] \
             IS in the compiled fix set{tier_note} — the leased corpus lags, \
             `--fix` heals these today (`--corpus-delta` for the table)"
        ))
    }

    /// The `--corpus-delta` table. Report-only; the caller's exit code is
    /// unchanged by anything here.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::with_capacity(512);
        out.push_str("corpus-truth delta (Issue 116) — report-only, gates nothing\n");
        let _ = writeln!(
            out,
            "  {} distinct [no corpus rule] lint(s) over {} span occurrence(s)",
            self.missing_lints, self.missing_spans
        );

        if self.rows.is_empty() {
            let _ = writeln!(
                out,
                "  freshness delta: NONE — no missing lint is in the compiled fix set"
            );
        } else {
            let _ = writeln!(
                out,
                "\n  FRESHNESS DELTA — the fix lane carries these, the leased corpus does not yet.\n  \
                 `[no corpus rule]` here means \"not leased yet\", NOT \"unsupported\".\n"
            );
            out.push_str("  spans  lint                                    tier      remedy\n");
            for row in &self.rows {
                let _ = writeln!(
                    out,
                    "  {:>5}  {:<38}  {:<8}  {}",
                    row.spans,
                    row.lint,
                    row.tier.label(),
                    row.tier.remedy()
                );
            }
        }

        let _ = writeln!(
            out,
            "\n  COVERAGE GAP — {} lint(s) over {} span(s) that NEITHER lane carries.",
            self.unfixable, self.unfixable_spans
        );
        out.push_str(
            "  Here `[no corpus rule]` does mean unsupported; the rows are mining intake\n  \
             (`--coverage-report` quantifies the same gap against clippy's whole surface).\n\n  \
             The two lanes' corpus truths are DIFFERENT BY DESIGN and this report does not\n  \
             close the seam: the compiled fix set is why a freshly-landed lint heals at once,\n  \
             and the lease is why suggest ranks on live evidence.\n",
        );
        out
    }
}

/// Aggregate one run's per-span `anchored_missing` lists against the
/// compiled fix set.
///
/// `fixable` / `verify` are the fix lane's two tiers, passed in rather than
/// read here so this module stays feature-free and lib-testable at default
/// features (the `frontier_report` / `corpus_coverage` precedent). Names are
/// compared after [`normalize`] so a kebab-spelled feed row cannot read as a
/// coverage gap purely on punctuation.
#[must_use]
pub fn delta<'a>(
    per_span_missing: impl IntoIterator<Item = &'a [String]>,
    fixable: &[String],
    verify: &[String],
) -> CorpusDelta {
    // Verify is checked SECOND and does not overwrite: a lint in both
    // tables is reachable on a plain `--fix`, so reporting the stricter
    // remedy would send a reader to a flag they do not need.
    let mut tier_of: BTreeMap<String, FixTier> = BTreeMap::new();
    for lint in verify {
        tier_of.insert(normalize(lint), FixTier::Verify);
    }
    for lint in fixable {
        tier_of.insert(normalize(lint), FixTier::Fixable);
    }

    let mut spans_of: BTreeMap<String, usize> = BTreeMap::new();
    for missing in per_span_missing {
        for lint in missing {
            *spans_of.entry(normalize(lint)).or_default() += 1;
        }
    }

    let mut out = CorpusDelta {
        missing_lints: spans_of.len(),
        missing_spans: spans_of.values().sum(),
        ..CorpusDelta::default()
    };
    for (lint, spans) in spans_of {
        if let Some(&tier) = tier_of.get(&lint) {
            out.rows.push(DeltaRow { lint, tier, spans });
        } else {
            out.unfixable += 1;
            out.unfixable_spans += spans;
        }
    }
    out.rows.sort_by(|a, b| {
        b.spans
            .cmp(&a.spans)
            .then_with(|| a.tier.cmp(&b.tier))
            .then_with(|| a.lint.cmp(&b.lint))
    });
    out
}

/// Fold a lint name to the corpus spelling: `clippy::` stripped, kebab to
/// snake, lowercase.
///
/// This is [`crate::corpus_coverage::normalize`], re-exported rather than
/// re-implemented. The two reports answer different questions — that one
/// measures the INSTALLED surface, this one a run's feed — but they must
/// agree on what a lint NAME is, or a row can be a coverage gap in one and
/// covered in the other purely on punctuation. A second copy is a second
/// thing to get wrong (this repo's own repeated finding).
pub use crate::corpus_coverage::normalize;

#[cfg(test)]
mod tests;
