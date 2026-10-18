//! Issue 119 — the corpus coverage-debt sweep: clippy's LIVE lint surface
//! vs the shipped corpus + the compiled fix set.
//!
//! `[no corpus rule]` (Issue 113) is a live gap signal, but it only fires on
//! diagnostics a real run happens to hit — so the corpus's coverage of
//! clippy's **full** lint surface was never quantified. This module derives
//! that surface from the INSTALLED toolchain (`clippy-driver -W help`) and
//! diffs it against the corpus, so the debt is a number instead of a thing
//! users discover one diagnostic at a time.
//!
//! **Report-first, gates nothing** (the R135/Bench 047 frontier-report law):
//! it changes no verdict and stages no rule. Its output is charter INPUT for
//! the existing mining-batch discipline (dual evidence, dedup grep, owner
//! adjudication) — the [`CoverageReport::blind`] refusal is the only exit-1.
//!
//! ## The three coverage axes, never pooled
//!
//! A lint is scored against three independent sets, and the report keeps
//! them apart because they mean different things to a reader:
//!
//! - **corpus** — a `clippy_lints` corpus `rule_id` exists. The retrieval
//!   lane can rank it and the suggest lane can name it.
//! - **compiled fix set** — `FIXABLE_LINTS ∪ VERIFY_LINTS`. The `--fix`
//!   lane can WRITE it. Issue 116's seam is exactly the two disagreeing;
//!   here a fix-set-only row is reported as its own bucket rather than
//!   folded into `covered`, because it is a corpus gap wearing a green.
//! - **another domain's corpus** — `rust_perf` / `kernel_opt` / `sec` /
//!   `docker` / `dist` `rule_id`s. Expected to be 0 (those are kebab-case
//!   concept ids, not lint names); the repo's dedup law demands the check
//!   anyway, and a NON-zero here is a real finding — two domains claiming
//!   one name (the Batch-146 `dispatch-pass-fusion` near-miss class).
//!
//! ## Two floors, because a ceiling is green over whatever it can SEE
//!
//! [`MIN_DERIVED_LINTS`] floors the DERIVATION (a `-W help` format change
//! takes the parse to ~0 and every "uncovered" count collapses to a
//! flattering zero) and [`MIN_CORPUS_RULES`] / [`MIN_FIX_SET`] floor the
//! two things it is diffed AGAINST (an empty corpus makes the debt read as
//! total). They break differently and neither implies the other.
//!
//! ## What is NOT derived, deliberately
//!
//! **Applicability is not in `-W help`.** `MachineApplicable` is a property
//! of an emitted DIAGNOSTIC, not of a lint declaration, so no static sweep
//! can report it — it needs the `--clippy-json` oracle lane over real code.
//! The report says so on its own header instead of printing a column it
//! would have to invent; the derivable axis is the **default level**
//! (allow / warn / deny), which is what actually decides whether a lint
//! fires on anybody's build.
//!
//! **Frequency is joined, never estimated** (T3). The machine-readable
//! real-code frequency evidence is three sources, each carried with its
//! provenance printed — dataset digest, capture date, and the toolchain it
//! was captured under:
//!
//! 1. **local** — the Bench 061 fired histogram
//!    (`.benchmarks/data/049_t01/manifest.json`), captured with
//!    pedantic+nursery ENABLED over already-swept sibling repos, so its hot
//!    rows are opt-in lints and it is structurally blind to the
//!    default-firing debt.
//! 2. **default-eco** — the Bench 094 default-level fired histogram
//!    (`.benchmarks/data/094_t04/manifest.json`, waves 1+2+3+4 merged: 663
//!    compiled-clean crates): plain `cargo clippy` (no `-W` flags) over a
//!    stratified crates.io top-10K-downloads sample. This is the source the
//!    default-firing uncovered set can actually carry — the T4 successor
//!    lead Issue 119's first run filed.
//! 3. **study** — kornel's 100K-crate run (3 clippy-namespace rows) plus
//!    Li et al. ICSE 2024 (8.09M warnings), the latter deliberately NOT
//!    transcribed: its per-lint table is a figure with no printed value
//!    labels and an unpublished supplementary (checked 2026-09-17 — the
//!    refusal is measured, not assumed), so only the SHAPE claim
//!    (top-5 > 80%) is citable, never a per-lint count.
//!
//! Every row outside all three is `uncited`. A count from one population is
//! never summed with another (6 repos on one day ≠ 501 crates ≠ 100K
//! crates — adding them invents a fourth).

use std::collections::{BTreeMap, BTreeSet};

/// Floor on the derived lint surface. clippy 1.98 ships 822 lints; a parse
/// that finds fewer than this is a FORMAT regression, not a smaller clippy.
pub const MIN_DERIVED_LINTS: usize = 300;

/// Floor on the `clippy_lints` corpus. 54 rules at Issue 119; the floor is
/// loose because the corpus only ever grows and the quantity it guards is
/// "did the builder compile in at all".
pub const MIN_CORPUS_RULES: usize = 40;

/// Floor on the compiled fix set (`FIXABLE_LINTS ∪ VERIFY_LINTS`).
pub const MIN_FIX_SET: usize = 20;

/// How many frequency-ranked uncovered rows the head table prints.
pub const FREQ_HEAD_ROWS: usize = 20;
/// Issue 167 — how many pattern-mentioned names the head list prints
/// before the `… N more` line.
pub const PATTERN_MENTIONED_HEAD: usize = 8;

/// The `clippy::all` group is the UNION of style/correctness/suspicious/
/// complexity/perf — never a lint's own group, and folding it in would
/// double-count every one of its members.
const UNION_GROUP: &str = "all";

/// A lint's on-by-default level, as `-W help` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefaultLevel {
    /// Off unless explicitly enabled — `restriction`, most of `pedantic`
    /// and `nursery`. Never fires on an ordinary build.
    Allow,
    /// Fires on a plain `cargo clippy`.
    Warn,
    /// Fires and fails the build — the `correctness` group.
    Deny,
}

impl DefaultLevel {
    /// Parse the `-W help` `default` column. Unknown spellings are `None`
    /// (a level this code cannot read is never guessed at as `Allow`,
    /// which would silently move a firing lint into the quiet bucket).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allow" => Some(Self::Allow),
            "warn" => Some(Self::Warn),
            "deny" | "forbid" => Some(Self::Deny),
            _ => None,
        }
    }

    /// The column label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
        }
    }

    /// Does this level fire on a plain `cargo clippy` run?
    #[must_use]
    pub const fn fires_by_default(self) -> bool {
        matches!(self, Self::Warn | Self::Deny)
    }
}

/// One lint from the installed driver's own surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LintRow {
    /// Normalized name — `clippy::` stripped, kebab folded to snake, so it
    /// compares directly against a corpus `rule_id`.
    pub name: String,
    /// The lint's own group (`style`, `pedantic`, …). `clippy::all` is
    /// never recorded here.
    pub group: String,
    pub level: DefaultLevel,
}

/// The parsed `clippy-driver -W help` surface.
#[derive(Clone, Debug, Default)]
pub struct LintSurface {
    pub lints: Vec<LintRow>,
    /// Group → member count, from the groups section.
    pub groups: BTreeMap<String, usize>,
    /// Lints whose `default` column this parser could not read — counted,
    /// never defaulted into a level (an unreadable level is unknown).
    pub level_unparsed: usize,
    /// Lints the groups section put in 2+ non-union groups. Zero under
    /// clippy 1.98 (measured: 822/822 in exactly one). A non-zero here
    /// means the group model changed and the per-group table is a guess.
    pub ambiguous_group: usize,
    /// Lints present in the checks section with no non-union group.
    /// Reported as `(ungrouped)`, never bucketed into a neighbour.
    pub ungrouped: usize,
    /// Why the derivation cannot be read. Never a confident zero.
    pub blind: Option<String>,
}

/// Normalize a lint name for comparison: strip the `clippy::` tool prefix,
/// fold kebab to snake, lowercase. `-W help` prints kebab
/// (`clippy::needless-return`), the corpus stores snake (`needless_return`)
/// — a diff over the raw spellings reports 100% debt.
#[must_use]
pub fn normalize(name: &str) -> String {
    name.trim()
        .trim_start_matches("clippy::")
        .replace('-', "_")
        .to_ascii_lowercase()
}

/// Issue 167 — single-pass word-boundary mention scan. Every maximal
/// identifier run in `haystack` is extracted and looked up in `candidates`,
/// so `len_zero` matches and `manual_len_zero` does not — by construction,
/// not by a substring check. Returns the matched candidates sorted.
#[must_use]
pub fn mentioned_names(haystack: &str, candidates: &BTreeSet<String>) -> Vec<String> {
    let mut found = BTreeSet::new();
    let bytes = haystack.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut i = 0;
    while i < bytes.len() {
        if is_word(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            if let Ok(word) = std::str::from_utf8(&bytes[start..i]) {
                if candidates.contains(word) {
                    found.insert(word.to_string());
                }
            }
        } else {
            i += 1;
        }
    }
    found.into_iter().collect()
}

/// Join the three frequency sources into one evidence value. Neither is
/// defaulted to zero in the others' absence — a missing source is
/// *unobserved*, and a zero would rank it below a genuinely-rare lint that
/// WAS observed.
///
/// Precedence, each step documented:
/// 1. `fired` + `study` → [`FreqEvidence::Both`] — two independent sources
///    agreeing is maximum breadth and stays top whatever the third says
///    (the default-eco count it shadows remains in its own manifest).
/// 2. any `fired_default` → [`FreqEvidence::FiredDefault`] — same-level
///    evidence beats a lone opt-in-captured local count, and also beats a
///    lone published count for the same reason.
/// 3. lone `fired` → [`FreqEvidence::Fired`]; lone `study` →
///    [`FreqEvidence::Study`].
#[must_use]
pub const fn join_evidence(
    fired: Option<u32>,
    fired_default: Option<u32>,
    study: Option<u32>,
) -> FreqEvidence {
    match (fired, fired_default, study) {
        (Some(fired), _, Some(study)) => FreqEvidence::Both { fired, study },
        (_, Some(d), _) => FreqEvidence::FiredDefault(d),
        (Some(f), None, None) => FreqEvidence::Fired(f),
        (None, None, Some(s)) => FreqEvidence::Study(s),
        (None, None, None) => FreqEvidence::Uncited,
    }
}

/// Parse `clippy-driver -W help` stdout.
///
/// Only the two `loaded by this crate` sections are read — the `provided by
/// rustc` sections are a different lint namespace that this corpus has
/// never claimed, and pooling them would inflate the debt by ~250 rows
/// nothing here is meant to cover.
#[must_use]
pub fn parse_lint_help(text: &str) -> LintSurface {
    const CHECKS_HDR: &str = "Lint checks loaded by this crate:";
    const GROUPS_HDR: &str = "Lint groups loaded by this crate:";

    let mut surface = LintSurface::default();

    let Some(checks_at) = text.find(CHECKS_HDR) else {
        surface.blind = Some(format!("no `{CHECKS_HDR}` section in the driver output"));
        return surface;
    };
    let groups_at = text.find(GROUPS_HDR);
    let checks_body = match groups_at {
        Some(g) if g > checks_at => &text[checks_at + CHECKS_HDR.len()..g],
        _ => &text[checks_at + CHECKS_HDR.len()..],
    };

    // ── the groups section first: a lint's group is read from it ────────
    let mut lint_group: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Some(g) = groups_at {
        for line in text[g + GROUPS_HDR.len()..].lines() {
            let Some((name, members)) = split_row(line) else {
                continue;
            };
            let group = normalize(&name);
            if group == UNION_GROUP {
                continue;
            }
            let mut n = 0usize;
            for member in members.split(',') {
                let member = member.trim();
                if member.is_empty() {
                    continue;
                }
                n += 1;
                lint_group
                    .entry(normalize(member))
                    .or_default()
                    .insert(group.clone());
            }
            surface.groups.insert(group, n);
        }
    }

    // ── the checks section: one row per lint ────────────────────────────
    for line in checks_body.lines() {
        let Some((name, rest)) = split_row(line) else {
            continue;
        };
        let name = normalize(&name);
        let Some(level_tok) = rest.split_whitespace().next() else {
            continue;
        };
        let Some(level) = DefaultLevel::parse(level_tok) else {
            surface.level_unparsed += 1;
            continue;
        };
        let group = match lint_group.get(&name) {
            Some(gs) if gs.len() == 1 => gs.iter().next().cloned().unwrap_or_default(),
            Some(gs) if gs.len() > 1 => {
                surface.ambiguous_group += 1;
                "(ambiguous)".to_string()
            }
            _ => {
                surface.ungrouped += 1;
                "(ungrouped)".to_string()
            }
        };
        surface.lints.push(LintRow { name, group, level });
    }

    if surface.lints.len() < MIN_DERIVED_LINTS {
        surface.blind = Some(format!(
            "derived {} clippy lint(s), floor is {MIN_DERIVED_LINTS} — the \
             `-W help` format changed or the driver is not clippy",
            surface.lints.len()
        ));
    }
    surface
}

/// Split a `-W help` table row into `(first column, remainder)`.
///
/// Header (`name  default  meaning`), rule (`----  -------`) and blank rows
/// return `None`; so does any row whose first column is not a `clippy::`
/// name, which is what keeps the rustc namespace out.
fn split_row(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with("---") {
        return None;
    }
    let (first, rest) = line.split_once(char::is_whitespace)?;
    if !first.starts_with("clippy::") {
        return None;
    }
    Some((first.to_string(), rest.trim().to_string()))
}

/// The published frequency evidence `.benchmarks/021` cites, restricted to
/// the rows in THIS report's namespace.
///
/// kornel's 100K+-crate clippy run (clippy 1.80–1.83, September 2024,
/// `users.rust-lang.org/t/ive-ran-clippy-on-100k-crates/117127`) published a
/// top-10 by crate-count. Seven of those ten are **rustc** lints
/// (`unused_imports`, `unused_qualifications`, `missing_docs` ×3,
/// `elided_lifetimes_in_paths`, `unused_crate_dependencies`) and this report
/// derives only the `clippy::` namespace — pooling them would attribute a
/// rustc lint's frequency to a clippy gap. The three that survive are
/// transcribed here WITH their counts, because a citation whose number lives
/// only in prose cannot be joined to a row.
///
/// Li et al. (ICSE 2024, arXiv:2310.11738 — 8,095,259 warnings over 94,715
/// crates) is the second source `.benchmarks/021` cites and is deliberately
/// NOT transcribed: that paper's contribution there is the SHAPE of the
/// distribution (top-5 > 80% of all warnings), not a per-lint table, and
/// inventing per-lint counts from a shape claim is the thing this axis
/// exists to refuse.
pub const STUDY_KORNEL_2024: &[(&str, u32)] = &[
    ("missing_errors_doc", 56_975),
    ("missing_panics_doc", 43_360),
    ("semicolon_if_nothing_returned", 41_601),
];

/// The citation printed next to a [`FreqEvidence::Study`] row.
pub const STUDY_SOURCE: &str =
    "kornel 100K-crate clippy run, Sep 2024 (.benchmarks/021 §selection methodology)";

/// Where a row's frequency number comes from. Never inferred: a lint no
/// source observed is [`FreqEvidence::Uncited`], **not zero** — "not seen in
/// six repos on one day" is not "does not fire".
///
/// The four variants are a TIER, and the top one is the point: a lint
/// carrying BOTH a local real-code count and an independent published one
/// is the dual-evidence shape the mining batch discipline stages first, and
/// pooling the sources into one number would destroy exactly that signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreqEvidence {
    /// Local real-code histogram AND the published study agree it fires.
    Both { fired: u32, study: u32 },
    /// Observed count from the DEFAULT-LEVEL ecosystem histogram (Bench
    /// 094: plain `cargo clippy` over a crates.io sample) — the same level
    /// the uncovered row fires at, which is the evidence the local
    /// histogram (opt-in-captured, already-swept repos) cannot supply for
    /// the default-firing debt.
    ///
    /// Shadows a lone [`FreqEvidence::Fired`] count (same-level evidence is
    /// the axis this report's charter needs); the shadowed count remains in
    /// its own manifest, never summed.
    FiredDefault(u32),
    /// Observed count from the joined real-code fired histogram (Bench 061).
    Fired(u32),
    /// Published crate-ecosystem count only ([`STUDY_KORNEL_2024`]).
    Study(u32),
    /// No machine-readable evidence, in this repo or in a cited study.
    Uncited,
}

impl FreqEvidence {
    /// Tier ordinal — lower sorts first. Tier dominates the count: a
    /// dual-evidence row at 90 firings outranks a single-source row at
    /// 1011, because the batch discipline admits on evidence breadth.
    #[must_use]
    pub const fn tier(self) -> u8 {
        match self {
            Self::Both { .. } => 0,
            Self::FiredDefault(_) => 1,
            Self::Fired(_) => 2,
            Self::Study(_) => 3,
            Self::Uncited => 4,
        }
    }

    /// The within-tier sort key: the LOCAL count where one exists, else the
    /// observed count of whichever source the variant carries. Never a sum
    /// — the sources are different populations (6 repos on one day vs 184
    /// crates vs 100K crates) and adding them invents a third.
    #[must_use]
    pub const fn rank(self) -> u32 {
        match self {
            Self::Both { fired, .. } | Self::Fired(fired) => fired,
            Self::FiredDefault(n) | Self::Study(n) => n,
            Self::Uncited => 0,
        }
    }

    /// Does this row carry any evidence at all?
    #[must_use]
    pub const fn is_cited(self) -> bool {
        !matches!(self, Self::Uncited)
    }

    /// The report's evidence column.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Both { .. } => "local+study",
            Self::FiredDefault(_) => "default-eco",
            Self::Fired(_) => "local",
            Self::Study(_) => "study",
            Self::Uncited => "uncited",
        }
    }
}

/// Provenance of the joined frequency evidence — printed, because the
/// histogram was captured under a DIFFERENT toolchain than the one being
/// swept and a frequency column with no date is a claim about today.
#[derive(Clone, Debug, Default)]
pub struct HistogramProvenance {
    pub source: String,
    pub captured_utc: String,
    pub toolchain: String,
    pub dataset_blake3: String,
    pub repos: usize,
    pub rows: usize,
}

/// One uncovered lint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverageRow {
    pub lint: String,
    pub group: String,
    pub level: DefaultLevel,
    pub freq: FreqEvidence,
}

/// Per-group debt, split by default level so the reader can tell a
/// 130-row `restriction` backlog (fires for nobody) from a 12-row `style`
/// one (fires for everybody).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupDebt {
    pub group: String,
    pub total: usize,
    pub covered: usize,
    pub uncovered: usize,
    pub uncovered_firing: usize,
}

/// The inputs — one struct so the argument list stays readable and every
/// axis is named at the call site.
pub struct CoverageInputs<'a> {
    /// `clippy-driver --version`, verbatim.
    pub toolchain: &'a str,
    pub surface: LintSurface,
    /// `clippy_lints` corpus `rule_id`s.
    pub corpus_rules: &'a [String],
    /// `FIXABLE_LINTS ∪ VERIFY_LINTS` — the compiled fix lane's truth.
    pub fix_set: &'a [String],
    /// Every OTHER domain's corpus, `domain → rule_ids` (the dedup law).
    pub other_domains: &'a BTreeMap<String, Vec<String>>,
    /// `(lint, fired count)` from the real-code histogram, plus provenance.
    pub histogram: &'a [(String, u32)],
    pub histogram_provenance: Option<HistogramProvenance>,
    /// `(lint, fired count)` from the DEFAULT-LEVEL ecosystem histogram
    /// (Bench 094), plus provenance — the source the default-firing
    /// uncovered set can actually carry. OPTIONAL like the local half: its
    /// absence costs the default-eco axis and nothing else.
    pub default_histogram: &'a [(String, u32)],
    pub default_histogram_provenance: Option<HistogramProvenance>,
    /// Issue 167 — corpus rule BODY text (the compiled entries' buggy/fixed/
    /// context notes, plus the proposers' source doc comments when the
    /// source tree is locatable). The name-keyed buckets are a LOWER BOUND:
    /// the corpus heals by PATTERN, so a rule can fully cover a lint it
    /// never keys (measured precedent: `len_zero` fully covered by
    /// `comparison_to_empty`, which names it only in doc comments). Each
    /// uncovered lint whose name appears (word-boundary) in any blob lands
    /// in [`CoverageReport::pattern_mentioned`] — a text proxy, never
    /// folded into `covered`.
    pub corpus_blobs: &'a [String],
    /// Whether the doc-comment half of `corpus_blobs` was actually read
    /// (the source tree was locatable beside this binary). `false` means
    /// the scan is pattern-literals-only — disclosed, never a silent zero.
    pub corpus_docs_scanned: bool,
}

/// The report.
#[derive(Clone, Debug, Default)]
pub struct CoverageReport {
    pub toolchain: String,
    pub lints_derived: usize,
    pub groups_derived: usize,
    pub corpus_rules: usize,
    pub fix_set: usize,
    pub other_domain_rules: usize,
    /// Derived lints with a `clippy_lints` corpus rule.
    pub covered_corpus: usize,
    /// Derived lints the fix lane can WRITE but the corpus does not name —
    /// Issue 116's seam, in the other direction. Never folded into
    /// `covered_corpus`: the fix lane heals it and suggest cannot rank it.
    pub covered_fix_only: usize,
    /// Derived lints whose name collides with ANOTHER domain's `rule_id`
    /// (the dedup law). Expected 0; a non-zero is a finding.
    pub collides_other_domain: Vec<(String, String)>,
    /// Corpus rules naming a lint the installed driver does not ship —
    /// the opposite drift (a renamed/removed lint), reported so the
    /// coverage number is not read as symmetric.
    pub corpus_rules_not_in_surface: Vec<String>,
    pub uncovered: Vec<CoverageRow>,
    pub group_debt: Vec<GroupDebt>,
    pub histogram_provenance: Option<HistogramProvenance>,
    /// Histogram rows naming a lint the driver no longer ships.
    pub histogram_rows_stale: usize,
    /// The default-eco histogram's provenance + stale-row count — the same
    /// two axes the local histogram carries, for the same reasons.
    pub default_histogram_provenance: Option<HistogramProvenance>,
    pub default_histogram_rows_stale: usize,
    pub surface_level_unparsed: usize,
    pub surface_ambiguous_group: usize,
    pub surface_ungrouped: usize,
    /// Issue 167 — uncovered lints whose name appears in some corpus rule
    /// BODY (pattern literals or doc comments): pattern-covered under a
    /// different key, so the UNCOVERED bucket overstates authoring
    /// opportunity by exactly these. A text proxy, never folded into
    /// `covered`; sorted, deduped.
    pub pattern_mentioned: Vec<String>,
    /// Whether the doc-comment half of the body scan was actually read.
    pub corpus_docs_scanned: bool,
    /// The refusal (frontier-report law): never a confident green zero.
    pub blind: Option<String>,
}

impl CoverageReport {
    /// Uncovered lints that fire on a plain `cargo clippy` — the debt that
    /// costs a real user something today.
    #[must_use]
    pub fn uncovered_firing(&self) -> usize {
        self.uncovered
            .iter()
            .filter(|r| r.level.fires_by_default())
            .count()
    }

    /// Uncovered rows carrying frequency evidence, dual-evidence first and
    /// hottest within each tier. Uncited rows never enter.
    #[must_use]
    pub fn freq_ranked(&self) -> Vec<&CoverageRow> {
        let mut rows: Vec<&CoverageRow> = self
            .uncovered
            .iter()
            .filter(|r| r.freq.is_cited())
            .collect();
        rows.sort_by(|a, b| {
            a.freq
                .tier()
                .cmp(&b.freq.tier())
                .then_with(|| b.freq.rank().cmp(&a.freq.rank()))
                .then_with(|| a.lint.cmp(&b.lint))
        });
        rows
    }

    /// Uncovered lints that BOTH fire by default AND carry frequency
    /// evidence — the intersection a staging decision actually wants.
    ///
    /// It is reported because it is the number the two headline axes do not
    /// give you and can be badly wrong to assume: the local histogram was
    /// captured with pedantic+nursery ENABLED over already-clippy-swept
    /// repos, so its hot rows are opt-in lints, while the debt that costs an
    /// ordinary user is the default-firing set. The two can be disjoint, and
    /// a reader who assumes they overlap will stage the wrong rows.
    #[must_use]
    pub fn firing_with_evidence(&self) -> usize {
        self.uncovered
            .iter()
            .filter(|r| r.level.fires_by_default() && r.freq.is_cited())
            .count()
    }

    /// Of the default-firing uncovered rows, how many carry the DEFAULT-ECO
    /// source specifically — the join the Issue-119 first run said was
    /// missing for the entire set. Reported beside the cross-tab so the
    /// reader can see WHICH source closed the gap.
    #[must_use]
    pub fn firing_with_default_evidence(&self) -> usize {
        self.uncovered
            .iter()
            .filter(|r| {
                r.level.fires_by_default() && matches!(r.freq, FreqEvidence::FiredDefault(_))
            })
            .count()
    }

    /// The dual-evidence head — the rows the mining batch discipline stages
    /// first (T4), because each carries two independent sources.
    #[must_use]
    pub fn dual_evidence(&self) -> Vec<&CoverageRow> {
        self.freq_ranked()
            .into_iter()
            .filter(|r| matches!(r.freq, FreqEvidence::Both { .. }))
            .collect()
    }
}

/// Build the report. Pure over its inputs — the driver spawn and the
/// manifest read live in the CLI half, so every rule here is unit-testable
/// at default features.
#[must_use]
#[allow(clippy::too_many_lines)] // one linear pass per axis; splitting it hides the order
pub fn coverage(inputs: &CoverageInputs<'_>) -> CoverageReport {
    let CoverageInputs {
        toolchain,
        surface,
        corpus_rules,
        fix_set,
        other_domains,
        histogram,
        histogram_provenance,
        default_histogram,
        default_histogram_provenance,
        corpus_blobs,
        corpus_docs_scanned,
    } = inputs;

    let corpus: BTreeSet<String> = corpus_rules.iter().map(|r| normalize(r)).collect();
    let fixes: BTreeSet<String> = fix_set.iter().map(|r| normalize(r)).collect();
    let mut others: BTreeMap<String, String> = BTreeMap::new();
    let mut other_rules = 0usize;
    for (domain, rules) in *other_domains {
        other_rules += rules.len();
        for rule in rules {
            others
                .entry(normalize(rule))
                .or_insert_with(|| domain.clone());
        }
    }
    let freq: BTreeMap<String, u32> = histogram
        .iter()
        .map(|(name, n)| (normalize(name), *n))
        .collect();
    let freq_default: BTreeMap<String, u32> = default_histogram
        .iter()
        .map(|(name, n)| (normalize(name), *n))
        .collect();
    let study: BTreeMap<&str, u32> = STUDY_KORNEL_2024.iter().copied().collect();

    let mut report = CoverageReport {
        toolchain: (*toolchain).to_string(),
        lints_derived: surface.lints.len(),
        groups_derived: surface.groups.len(),
        corpus_rules: corpus.len(),
        fix_set: fixes.len(),
        other_domain_rules: other_rules,
        histogram_provenance: histogram_provenance.clone(),
        default_histogram_provenance: default_histogram_provenance.clone(),
        surface_level_unparsed: surface.level_unparsed,
        surface_ambiguous_group: surface.ambiguous_group,
        surface_ungrouped: surface.ungrouped,
        ..CoverageReport::default()
    };

    // A blind derivation is fatal on its own terms — report the floors
    // rather than a debt measured over a broken parse.
    if let Some(reason) = &surface.blind {
        report.blind = Some(reason.clone());
        return report;
    }
    if corpus.len() < MIN_CORPUS_RULES {
        report.blind = Some(format!(
            "clippy_lints corpus read {} rule(s), floor is {MIN_CORPUS_RULES} — \
             the domain is not compiled in, so the debt would read as total",
            corpus.len()
        ));
        return report;
    }
    if fixes.len() < MIN_FIX_SET {
        report.blind = Some(format!(
            "compiled fix set read {} lint(s), floor is {MIN_FIX_SET} — \
             FIXABLE_LINTS/VERIFY_LINTS did not compile in",
            fixes.len()
        ));
        return report;
    }

    let surface_names: BTreeSet<&str> = surface.lints.iter().map(|l| l.name.as_str()).collect();
    let mut per_group: BTreeMap<String, GroupDebt> = BTreeMap::new();

    for lint in &surface.lints {
        let debt = per_group
            .entry(lint.group.clone())
            .or_insert_with(|| GroupDebt {
                group: lint.group.clone(),
                ..GroupDebt::default()
            });
        debt.total += 1;

        let in_corpus = corpus.contains(&lint.name);
        let in_fix = fixes.contains(&lint.name);
        if let Some(domain) = others.get(&lint.name) {
            report
                .collides_other_domain
                .push((lint.name.clone(), domain.clone()));
        }
        match (in_corpus, in_fix) {
            (true, _) => {
                report.covered_corpus += 1;
                debt.covered += 1;
            }
            (false, true) => {
                report.covered_fix_only += 1;
                debt.covered += 1;
            }
            (false, false) => {
                debt.uncovered += 1;
                if lint.level.fires_by_default() {
                    debt.uncovered_firing += 1;
                }
                report.uncovered.push(CoverageRow {
                    lint: lint.name.clone(),
                    group: lint.group.clone(),
                    level: lint.level,
                    freq: join_evidence(
                        freq.get(&lint.name).copied(),
                        freq_default.get(&lint.name).copied(),
                        study.get(lint.name.as_str()).copied(),
                    ),
                });
            }
        }
    }

    // The reverse drift: a corpus rule the installed clippy no longer ships
    // (renamed or removed). Without it the coverage fraction reads as if
    // every corpus rule were still live.
    for rule in &corpus {
        if !surface_names.contains(rule.as_str()) {
            report.corpus_rules_not_in_surface.push(rule.clone());
        }
    }
    report.histogram_rows_stale = freq
        .keys()
        .filter(|k| !surface_names.contains(k.as_str()))
        .count();
    report.default_histogram_rows_stale = freq_default
        .keys()
        .filter(|k| !surface_names.contains(k.as_str()))
        .count();

    // Issue 167 — the pattern-mentioned tier. The name-keyed buckets are a
    // LOWER BOUND (the corpus heals by pattern), so every uncovered lint
    // whose name shows up in a corpus rule BODY is disclosed here — never
    // folded into `covered`. One joined haystack, one pass.
    report.corpus_docs_scanned = *corpus_docs_scanned;
    if !report.uncovered.is_empty() && !corpus_blobs.is_empty() {
        let uncovered_names: BTreeSet<String> =
            report.uncovered.iter().map(|r| r.lint.clone()).collect();
        let mut mentioned = BTreeSet::new();
        for blob in *corpus_blobs {
            for name in mentioned_names(blob, &uncovered_names) {
                mentioned.insert(name);
            }
        }
        report.pattern_mentioned = mentioned.into_iter().collect();
    }

    report.group_debt = per_group.into_values().collect();
    report.group_debt.sort_by(|a, b| {
        b.uncovered
            .cmp(&a.uncovered)
            .then_with(|| a.group.cmp(&b.group))
    });
    report.uncovered.sort_by(|a, b| a.lint.cmp(&b.lint));
    report
}

impl CoverageReport {
    /// Render the human report.
    #[must_use]
    #[allow(clippy::too_many_lines)] // a layout function; the length IS the table
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::with_capacity(4096);

        out.push_str("corpus coverage-debt report (Issue 119) — report-only, gates nothing\n");
        let _ = writeln!(out, "  toolchain      {}", self.toolchain);
        let _ = writeln!(
            out,
            "  derived        {} clippy lint(s) in {} group(s)  (clippy-driver -W help)",
            self.lints_derived, self.groups_derived
        );
        let _ = writeln!(
            out,
            "  corpus         {} clippy_lints rule(s); {} rule(s) in the other domains",
            self.corpus_rules, self.other_domain_rules
        );
        let _ = writeln!(
            out,
            "  compiled fix   {} lint(s) (FIXABLE_LINTS ∪ VERIFY_LINTS)",
            self.fix_set
        );

        if let Some(reason) = &self.blind {
            let _ = writeln!(out, "\n⛔ BLIND — {reason}");
            out.push_str(
                "  refusing to report a coverage number over a blind derivation \
                 (the frontier-report law)\n",
            );
            return out;
        }

        let uncovered = self.uncovered.len();
        let firing = self.uncovered_firing();
        out.push_str("\ncoverage\n");
        let _ = writeln!(
            out,
            "  covered by the clippy_lints corpus   {:>5}",
            self.covered_corpus
        );
        let _ = writeln!(
            out,
            "  fix set only (no corpus rule)        {:>5}   ← suggest cannot rank these",
            self.covered_fix_only
        );
        let _ = writeln!(
            out,
            "  UNCOVERED                            {uncovered:>5}   ← the coverage debt"
        );
        let _ = writeln!(
            out,
            "    of which fire by default           {firing:>5}   ← the debt a plain `cargo clippy` shows a user"
        );
        // Issue 167 — the pattern-mentioned tier, disclosed beside the debt
        // it qualifies. Never folded into `covered` (it is a text proxy).
        if !self.pattern_mentioned.is_empty() {
            let _ = writeln!(
                out,
                "    pattern-mentioned in rule bodies   {:>5}   ← possibly pattern-covered under a different key",
                self.pattern_mentioned.len()
            );
            let head = self.pattern_mentioned.iter().take(PATTERN_MENTIONED_HEAD);
            for chunk in head {
                let _ = writeln!(out, "      · {chunk}");
            }
            if self.pattern_mentioned.len() > PATTERN_MENTIONED_HEAD {
                let _ = writeln!(
                    out,
                    "      … {} more (the full list is this vec, sorted)",
                    self.pattern_mentioned.len() - PATTERN_MENTIONED_HEAD
                );
            }
        }

        out.push_str("\nuncovered by group\n");
        out.push_str("  group             total  covered  uncovered  firing\n");
        for debt in &self.group_debt {
            let _ = writeln!(
                out,
                "  {:<16} {:>5}  {:>7}  {:>9}  {:>6}",
                debt.group, debt.total, debt.covered, debt.uncovered, debt.uncovered_firing
            );
        }

        let ranked = self.freq_ranked();
        let dual = self.dual_evidence().len();
        out.push_str("\nfrequency-ranked head — evidence is JOINED, never estimated\n");
        match &self.histogram_provenance {
            Some(p) => {
                let _ = writeln!(
                    out,
                    "  local: {} ({} rows over {} repos, captured {}, {})",
                    p.source, p.rows, p.repos, p.captured_utc, p.toolchain
                );
                let _ = writeln!(out, "         dataset blake3 {}", p.dataset_blake3);
                if !p.toolchain.is_empty() && !self.toolchain.contains(&p.toolchain) {
                    out.push_str(
                        "         ⚠ captured under a DIFFERENT toolchain than the one \
                         swept — the counts RANK, they do not measure today\n",
                    );
                }
            }
            None => out.push_str("  local: NONE joined — no real-code histogram was supplied\n"),
        }
        match &self.default_histogram_provenance {
            Some(p) => {
                let _ = writeln!(
                    out,
                    "  default-eco: {} ({} rows over {} crates, captured {}, {})",
                    p.source, p.rows, p.repos, p.captured_utc, p.toolchain
                );
                let _ = writeln!(out, "         dataset blake3 {}", p.dataset_blake3);
                if !p.toolchain.is_empty() && !self.toolchain.contains(&p.toolchain) {
                    out.push_str(
                        "         ⚠ captured under a DIFFERENT toolchain than the one \\
                         swept — the counts RANK, they do not measure today\n",
                    );
                }
            }
            None => out.push_str(
                "  default-eco: NONE joined — the default-firing set carries no \\
                 same-level evidence\n",
            ),
        }
        let _ = writeln!(
            out,
            "  study: {STUDY_SOURCE} ({} clippy-namespace row(s))",
            STUDY_KORNEL_2024.len()
        );
        if self.histogram_rows_stale > 0 {
            let _ = writeln!(
                out,
                "  ⚠ {} local histogram row(s) name a lint this driver no longer ships",
                self.histogram_rows_stale
            );
        }
        if self.default_histogram_rows_stale > 0 {
            let _ = writeln!(
                out,
                "  ⚠ {} default-eco histogram row(s) name a lint this driver no longer ships",
                self.default_histogram_rows_stale
            );
        }
        if ranked.is_empty() {
            out.push_str("  (no uncovered lint carries frequency evidence from any source)\n");
        } else {
            let _ = writeln!(
                out,
                "  {dual} row(s) carry BOTH sources — the dual-evidence head the batch \
                 discipline stages first"
            );
            out.push_str(
                "  count  lint                                   group        default  evidence\n",
            );
            for row in ranked.iter().take(FREQ_HEAD_ROWS) {
                let _ = writeln!(
                    out,
                    "  {:>5}  {:<38} {:<12} {:<8} {}",
                    row.freq.rank(),
                    row.lint,
                    row.group,
                    row.level.label(),
                    row.freq.label()
                );
            }
            if ranked.len() > FREQ_HEAD_ROWS {
                let _ = writeln!(
                    out,
                    "  … {} more uncovered row(s) with evidence",
                    ranked.len() - FREQ_HEAD_ROWS
                );
            }
        }
        let uncited = uncovered - ranked.len();
        let _ = writeln!(
            out,
            "  {uncited} uncovered lint(s) carry NO evidence from any source — \
             uncited, NOT zero"
        );
        // The cross-tab, because the two headline axes can be DISJOINT and
        // a reader who assumes they overlap stages the wrong rows.
        let fwe = self.firing_with_evidence();
        let _ = writeln!(
            out,
            "  of the {firing} uncovered lint(s) that FIRE by default, {fwe} carry evidence"
        );
        let fwe_default = self.firing_with_default_evidence();
        if fwe_default > 0 {
            let rest = fwe - fwe_default;
            let _ = writeln!(
                out,
                "    ({fwe_default} via the default-eco histogram — the same-level source{})",
                if rest > 0 {
                    format!("; {rest} via local/study")
                } else {
                    String::new()
                }
            );
        }
        if fwe == 0 && firing > 0 {
            out.push_str(
                "  ⚠ DISJOINT: every evidenced row is an opt-in lint and every \
                 default-firing gap is\n    uncited — the local histogram was \
                 captured with pedantic+nursery ENABLED over\n    already-swept \
                 repos, so it ranks the opt-in tail and cannot see the \
                 default-firing\n    debt at all. Staging off the ranked head \
                 alone stages opt-in lints, deliberately\n    or not; the \
                 default-firing set needs its own histogram.\n",
            );
        }

        out.push_str("\nseams + limits\n");
        out.push_str(
            "  name-keyed coverage is a LOWER BOUND — the corpus heals by PATTERN, not lint name,\n  so a rule can fully cover a lint it never keys. Measured precedent: `len_zero`\n  reads UNCOVERED here while `comparison_to_empty` heals exactly that pattern (the\n  name appears only in its doc comments). `pattern-mentioned` is a text proxy over\n  rule bodies, never semantics — stage a dedup grep (Protocol A step 2) BEFORE\n  authoring anything this report calls uncovered.\n",
        );
        if !self.corpus_docs_scanned {
            out.push_str(
                "  ⚠ body scan is pattern-literals-only — the proposers' source tree was not\n    locatable beside this binary, so doc-comment mentions are invisible to it.\n",
            );
        }
        out.push_str(
            "  applicability (MachineApplicable/…) is a property of an EMITTED diagnostic, \
             not of\n  a lint declaration — no static sweep can report it; the \
             `--clippy-json` oracle lane\n  over real code is where that axis lives.\n",
        );
        if self.collides_other_domain.is_empty() {
            out.push_str(
                "  dedup: 0 uncovered lint name collides with another domain's rule_id \
                 (the dedup law)\n",
            );
        } else {
            let _ = writeln!(
                out,
                "  ⛔ dedup: {} lint name(s) collide with another domain's rule_id — \
                 two domains claiming one name:",
                self.collides_other_domain.len()
            );
            for (lint, domain) in &self.collides_other_domain {
                let _ = writeln!(out, "       {lint}  ←  {domain}");
            }
        }
        if !self.corpus_rules_not_in_surface.is_empty() {
            let _ = writeln!(
                out,
                "  ⚠ {} corpus rule(s) name a lint this driver does not ship \
                 (renamed/removed): {}",
                self.corpus_rules_not_in_surface.len(),
                self.corpus_rules_not_in_surface.join(", ")
            );
        }
        for (n, what) in [
            (self.surface_level_unparsed, "default level unreadable"),
            (self.surface_ambiguous_group, "in 2+ non-union groups"),
            (self.surface_ungrouped, "in no group"),
        ] {
            if n > 0 {
                let _ = writeln!(out, "  ⚠ {n} derived lint(s): {what}");
            }
        }
        out.push_str(
            "\n  Output is charter INPUT for the mining batch discipline \
             (dual evidence, dedup grep,\n  owner adjudication) — this report \
             stages nothing and gates nothing.\n",
        );
        out
    }
}

#[cfg(test)]
mod tests;
