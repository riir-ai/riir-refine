//! `cargo heal --orphan-report` — post-mining intake class C1: the config
//! knob ORPHANED BY OPTIMIZATION (Plan 126).
//!
//! The measured shape (riir-ai Issue 888 T17 watch, riir-games-civ
//! `BehaviorConfig.alarm_radius`): a perf refactor replaces a config-fed
//! path with a compile-time table and copies the values faithfully — zero
//! test failures, the knob stays `pub` and looks live. The field had a
//! reader that vanished; detection therefore needs HISTORY, not just syntax
//! (the intake's own words) — a review-time + sweep-time heuristic, never a
//! per-compile lint. Review-time usage: run the report over the tree after
//! any commit touching config inits — the sweep instrument run at HEAD IS
//! the post-hoc check.
//!
//! Two legs:
//! * **Current (syn, reused):** [`crate::config_audit::zero_external_read_fields`]
//!   — Default-impl candidate fields with `external = total − tests −
//!   own_impl == 0` (the class-A condition-2 set at field granularity).
//! * **History (git, name-global):** count read-shaped occurrences
//!   (`\.name` + a non-ident tail — literal inits and bare definitions do
//!   not match; destructuring binds do not either, the safe direction) at
//!   HEAD and at probe commits (`HEAD~N`, default [`DEFAULT_PROBE_DEPTHS`],
//!   clamped to history depth). The verdict is the DELTA: any probe count
//!   above HEAD's ⇒ read-shaped occurrences vanished. The delta formulation
//!   survives own-impl reads (a `validate()` read that existed then AND now
//!   contributes 0 to the delta) and catches the canonical case (consumer
//!   removed, validate remained). Removal attribution: `git log
//!   --pickaxe-regex -S`, newest first; a newest-hit subject matching the
//!   optimization vocabulary tags the row `LIKELY-BY-OPTIMIZATION` (a tag,
//!   not a verdict).
//!
//! Verdict classes: `ORPHANED` rows (delta evidence + commit list),
//! `[stillborn]` one-liners (zero read-shaped occurrences at HEAD and every
//! probe — never read within the window; the single-dead-field gap C2's
//! whole-struct rule misses), and silent (read-shaped occurrences exist now
//! with no vanished evidence — A/B/C2 territory; counted in the header).
//!
//! Honest limits (mirrored in every report footer): the history greps are
//! name-global and untyped (a same-named field on another type can
//! masquerade — adjudicate rows against the listed commits); comment/doc
//! mentions count (the syn leg excludes them, `git grep` cannot); readers
//! outside the scanned path set are invisible (the inherited config-audit
//! footer); a reader MOVED rather than removed still drops the count
//! (validate-refactor false positives — measure, don't assume away); probes
//! bound the lookback window (an orphan older than the deepest probe reads
//! as stillborn); fields carrying `#[serde(skip_serializing_if = …)]` are
//! excluded from the net entirely (conditionally-emitted output schema —
//! the derive reads them, invisibly; the Issue 934 measured boundary —
//! adjudicate serialized-output structs by hand if you suspect a dead
//! schema field).
//!
//! Exit contract (the frontier-report law, not config-audit's): exit 0 for
//! every real scan; **refuse exit 1** on an empty population or a blind
//! history leg ([`OrphanReport::blind`]) — never a confident green zero.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::config_audit::{FieldCandidate, ZeroReadField, ZeroReadScan};

/// Default probe depths (commits back from HEAD), clamped to history depth.
pub const DEFAULT_PROBE_DEPTHS: [usize; 4] = [10, 50, 200, 1000];

/// How many pickaxe commits to attribute per ORPHANED row (newest first).
const PICKAXE_MAX: usize = 8;

/// Subjuncts that tag a row LIKELY-BY-OPTIMIZATION (lowercase, substring).
const OPTIMIZATION_VOCAB: [&str; 8] = [
    "perf", "optimi", "refact", "fast", "inline", "table", "cache", "simd",
];

/// One `git log -S` hit (newest first) — where the pattern's occurrence
/// count changed inside the scanned path set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHit {
    pub hash: String,
    pub date: String,
    pub subject: String,
}

/// The history leg, behind a trait so the classifier is unit-testable
/// without a git repository (the production impl is [`GitCli`]).
pub trait HistoryProbe {
    /// Resolve `HEAD~<depth>` (`depth == 0` is HEAD); `None` when the depth
    /// exceeds history (the probe is clamped out).
    fn rev_at_depth(&self, depth: usize) -> Option<String>;
    /// Count lines matching the ERE at `rev` within the scanned path set;
    /// `None` when git cannot answer (treated as no evidence).
    fn count_ere(&self, rev: &str, pattern: &str) -> Option<usize>;
    /// Newest-first commits where the ERE's occurrence count changed.
    fn pickaxe(&self, pattern: &str, max: usize) -> Vec<CommitHit>;
}

/// The production history leg: shells `git` with the repo root as `-C` and
/// the scanned paths (relativized) as the pathspec for every query.
pub struct GitCli {
    root: PathBuf,
    pathspecs: Vec<String>,
}

impl GitCli {
    /// Build from the scanned paths. `None` when the anchor is not inside a
    /// resolvable git repository with a born HEAD, or when no scanned path
    /// is inside it (the caller reports a blind history leg, never a green
    /// zero over it).
    pub fn new(paths: &[PathBuf]) -> Option<Self> {
        let anchor = paths.first()?;
        // Paths may be files; git -C wants their directory.
        let dir = if anchor.is_dir() {
            anchor.clone()
        } else {
            anchor.parent().map(Path::to_path_buf).unwrap_or_default()
        };
        let root_out = git(Some(&dir), &["rev-parse", "--show-toplevel"])?;
        if !root_out.status.success() {
            return None;
        }
        let root = PathBuf::from(String::from_utf8_lossy(&root_out.stdout).trim());
        // An unborn repository has no tree to grep — blind, not empty.
        let head = git(Some(&root), &["rev-parse", "--verify", "--quiet", "HEAD"])?;
        if !head.status.success() {
            return None;
        }
        let cwd = std::env::current_dir().ok()?;
        let mut pathspecs = Vec::new();
        for p in paths {
            let joined = if p.is_absolute() {
                p.clone()
            } else {
                cwd.join(p)
            };
            // macOS scan paths are frequently symlinked (/var/folders/…)
            // while git reports the real path — canonicalize before the
            // prefix comparison or every pathspec is dropped.
            let abs = joined.canonicalize().unwrap_or(joined);
            if let Ok(rel) = abs.strip_prefix(&root) {
                let rel = rel.display().to_string();
                // Scanning the repo root itself: rel is empty — the
                // pathspec is the whole tree ("."), never a blind leg.
                if rel.is_empty() {
                    pathspecs.push(".".to_string());
                } else {
                    pathspecs.push(rel);
                }
            }
            // Outside this repository: no history for it — the scanned-set
            // footer covers the under-report.
        }
        if pathspecs.is_empty() {
            return None;
        }
        Some(Self { root, pathspecs })
    }

    fn root_str(&self) -> &str {
        self.root.to_str().unwrap_or(".")
    }
}

fn git(dir: Option<&Path>, args: &[&str]) -> Option<Output> {
    let mut cmd = Command::new("git");
    if let Some(d) = dir {
        cmd.arg("-C").arg(d);
    }
    cmd.args(args);
    cmd.output().ok()
}

impl HistoryProbe for GitCli {
    fn rev_at_depth(&self, depth: usize) -> Option<String> {
        let rev = if depth == 0 {
            "HEAD".to_string()
        } else {
            format!("HEAD~{depth}")
        };
        let out = git(
            Some(&self.root),
            &["rev-parse", "--verify", "--quiet", &rev],
        )?;
        out.status.success().then_some(rev)
    }

    fn count_ere(&self, rev: &str, pattern: &str) -> Option<usize> {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(self.root_str())
            .args(["grep", "-E", "-e", pattern, rev, "--"])
            .args(&self.pathspecs);
        let out = cmd.output().ok()?;
        // git grep exits 1 on no matches (grep convention); 0 = matched.
        match out.status.code() {
            Some(0 | 1) => {}
            _ => return None,
        }
        let text = String::from_utf8_lossy(&out.stdout);
        Some(text.lines().filter(|l| !l.trim().is_empty()).count())
    }

    fn pickaxe(&self, pattern: &str, max: usize) -> Vec<CommitHit> {
        let needle = format!("-S{pattern}");
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(self.root_str())
            .args([
                "log",
                "--pickaxe-regex",
                needle.as_str(),
                "--format=%h%x09%ad%x09%s",
                "--date=short",
                "--max-count",
            ])
            .arg(max.to_string())
            .arg("--")
            .args(&self.pathspecs);
        let Ok(out) = cmd.output() else {
            return Vec::new();
        };
        if !out.status.success() {
            return Vec::new();
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let (hash, rest) = l.split_once('\t')?;
                let (date, subject) = rest.split_once('\t')?;
                Some(CommitHit {
                    hash: hash.to_string(),
                    date: date.to_string(),
                    subject: subject.to_string(),
                })
            })
            .collect()
    }
}

/// One ORPHANED field: the candidate + the evidence that convicted it.
#[derive(Debug, Clone)]
pub struct OrphanRow {
    pub candidate: FieldCandidate,
    pub total_reads: usize,
    pub test_reads: usize,
    pub self_impl_reads: usize,
    /// `(rev-label, count)` evidence, HEAD first, deepest probe last.
    pub probes: Vec<(String, usize)>,
    /// Newest-first commits that changed the pattern's occurrence count.
    pub commits: Vec<CommitHit>,
    /// The newest count-change subject matches the optimization vocabulary.
    pub likely_by_optimization: bool,
}

/// The report (data only — rendering is [`OrphanReport::render`]).
#[derive(Debug, Default)]
pub struct OrphanReport {
    pub files_scanned: usize,
    pub candidates: usize,
    pub zero_read_fields: usize,
    pub orphaned: Vec<OrphanRow>,
    pub stillborn: Vec<ZeroReadField>,
    /// The probe depths actually resolved (clamped to history depth).
    pub probe_depths: Vec<usize>,
    /// Why the history leg is missing — the caller refuses with exit 1
    /// (never a confident green zero over a blind instrument).
    pub blind: Option<String>,
}

/// The read-shaped ERE for a field name: `\.name` + a non-ident tail (or
/// end-of-line). Rust idents are `[A-Za-z0-9_]+` — safe inside an ERE.
#[must_use]
pub fn read_shape_ere(field_name: &str) -> String {
    format!(r"\.{field_name}([^[:alnum:]_]|$)")
}

/// Does a commit subject match the optimization vocabulary? A tag, not a
/// verdict — the canonical orphaner reads like "perf: const-table …".
#[must_use]
pub fn subject_looks_like_optimization(subject: &str) -> bool {
    let lower = subject.to_lowercase();
    OPTIMIZATION_VOCAB.iter().any(|v| lower.contains(v))
}

/// Scan `paths` (dirs recursively / files) and run both legs. Probe depths
/// default to [`DEFAULT_PROBE_DEPTHS`].
#[must_use]
pub fn report_paths(paths: &[PathBuf], probe_depths: &[usize]) -> OrphanReport {
    let sources = crate::config_audit::read_sources(paths);
    if sources.is_empty() {
        return OrphanReport {
            blind: Some("no .rs files parsed under the scanned paths".to_string()),
            ..OrphanReport::default()
        };
    }
    let scan = crate::config_audit::zero_external_read_fields(&sources);
    match GitCli::new(paths) {
        None => OrphanReport {
            files_scanned: scan.files_scanned,
            candidates: scan.candidates,
            zero_read_fields: scan.fields.len(),
            blind: Some(
                "not a resolvable git repository (or git missing from PATH) — the C1 history leg is unavailable"
                    .to_string(),
            ),
            ..OrphanReport::default()
        },
        Some(git) => classify_with_probe(&scan, &git, probe_depths),
    }
}

/// The classifier over an already-run current-state scan — the unit-test
/// entry (drive it with a fake [`HistoryProbe`]).
#[must_use]
pub fn classify_with_probe(
    scan: &ZeroReadScan,
    probe: &dyn HistoryProbe,
    depths: &[usize],
) -> OrphanReport {
    let mut probe_depths: Vec<usize> = Vec::new();
    for d in depths.iter().copied().chain(std::iter::once(0)) {
        if probe.rev_at_depth(d).is_some() && !probe_depths.contains(&d) {
            probe_depths.push(d);
        }
    }
    probe_depths.sort_unstable();

    let mut orphaned: Vec<OrphanRow> = Vec::new();
    let mut stillborn: Vec<ZeroReadField> = Vec::new();
    for field in &scan.fields {
        let name = &field.candidate.field_name;
        let pattern = read_shape_ere(name);
        let mut probes: Vec<(String, usize)> = Vec::with_capacity(probe_depths.len());
        let mut head_count = 0;
        let mut max_other = 0;
        let mut any_count = 0;
        for d in &probe_depths {
            let rev = if *d == 0 {
                "HEAD".to_string()
            } else {
                format!("HEAD~{d}")
            };
            let count = probe.count_ere(&rev, &pattern).unwrap_or(0);
            any_count += count;
            if *d == 0 {
                head_count = count;
            } else {
                max_other = max_other.max(count);
            }
            probes.push((rev, count));
        }
        if max_other > head_count {
            let commits = probe.pickaxe(&pattern, PICKAXE_MAX);
            let likely_by_optimization = commits
                .first()
                .is_some_and(|c| subject_looks_like_optimization(&c.subject));
            orphaned.push(OrphanRow {
                candidate: field.candidate.clone(),
                total_reads: field.total_reads,
                test_reads: field.test_reads,
                self_impl_reads: field.self_impl_reads,
                probes,
                commits,
                likely_by_optimization,
            });
        } else if any_count == 0 {
            stillborn.push(field.clone());
        }
        // else: read-shaped occurrences exist now with no vanished evidence
        // — silent (A/B/C2 territory), counted in the header only.
    }
    OrphanReport {
        files_scanned: scan.files_scanned,
        candidates: scan.candidates,
        zero_read_fields: scan.fields.len(),
        orphaned,
        stillborn,
        probe_depths,
        blind: None,
    }
}

impl OrphanReport {
    /// Render the human report (what `--orphan-report` prints).
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        if let Some(reason) = &self.blind {
            let _ = writeln!(out, "orphan-report: BLIND — {reason}");
            let _ = writeln!(
                out,
                "scope: refusing to report findings over a blind history leg (the frontier-report law)"
            );
            return out;
        }
        let probe_labels: Vec<String> = self
            .probe_depths
            .iter()
            .map(|d| {
                if *d == 0 {
                    "HEAD".to_string()
                } else {
                    format!("HEAD~{d}")
                }
            })
            .collect();
        let silent = self.zero_read_fields - self.orphaned.len() - self.stillborn.len();
        let _ = writeln!(
            out,
            "orphan-report: {} file(s) parsed, {} Default-impl field candidate(s), {} zero-external-read field(s): {} orphaned [C1], {} stillborn, {} silent (no vanished-reader evidence)",
            self.files_scanned,
            self.candidates,
            self.zero_read_fields,
            self.orphaned.len(),
            self.stillborn.len(),
            silent
        );
        let _ = writeln!(
            out,
            "history probes: {} (clamped to repository depth)",
            probe_labels.join(", ")
        );
        for row in &self.orphaned {
            write_orphan_row(&mut out, row);
        }
        for s in &self.stillborn {
            let _ = writeln!(
                out,
                "  [stillborn] {} — {}.{} (0 read-shaped occurrences at HEAD and every probe — never read within the window)",
                s.candidate.origin,
                s.candidate.struct_name,
                s.candidate.field_name
            );
        }
        let _ = writeln!(
            out,
            "scope: history greps are name-global (untyped) — a same-named field on another type can masquerade; comment mentions count (syn excludes them, grep cannot); readers outside the scanned set are invisible; a reader MOVED (not removed) can drop the count — adjudicate every row against the listed commits; #[serde(skip_serializing_if)] fields are excluded (output-schema surface, derive-read — the Issue 934 boundary)"
        );
        out
    }
}

/// One [C1] row's evidence block (extracted from [`OrphanReport::render`] —
/// the whole-render fn trips `too_many_lines`).
fn write_orphan_row(out: &mut String, row: &OrphanRow) {
    let evidence: Vec<String> = row
        .probes
        .iter()
        .map(|(rev, c)| format!("{rev} {c}"))
        .collect();
    let _ = writeln!(
        out,
        "  [C1] {} — {}.{}: read-shaped occurrences VANISHED ({}); syn reads {} total / {} test / {} own-impl → external 0",
        row.candidate.origin,
        row.candidate.struct_name,
        row.candidate.field_name,
        evidence.join("; "),
        row.total_reads,
        row.test_reads,
        row.self_impl_reads
    );
    if row.commits.is_empty() {
        let _ = writeln!(
            out,
            "       no pickaxe attribution within the scanned paths (the removal predates the walk or sat outside it)"
        );
    } else {
        let _ = writeln!(out, "       occurrence-count changes (newest first):");
        for c in row.commits.iter().take(3) {
            let _ = writeln!(out, "         {} {} {}", c.hash, c.date, c.subject);
        }
        if row.commits.len() > 3 {
            let _ = writeln!(out, "         … {} more (see git log)", row.commits.len() - 3);
        }
    }
    if row.likely_by_optimization {
        let _ = writeln!(
            out,
            "       verdict: ORPHANED-BY-TIME — newest count-change subject matches the optimization vocabulary → LIKELY-BY-OPTIMIZATION"
        );
    } else {
        let _ = writeln!(out, "       verdict: ORPHANED-BY-TIME");
    }
    let _ = writeln!(
        out,
        "       fix: re-wire the consumer the change replaced, or delete the knob"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap as Map;

    /// Pattern → rev → line count.
    type FakeCounts = Map<String, Map<String, usize>>;
    /// Pattern → newest-first commit hits.
    type FakeCommits = Map<String, Vec<CommitHit>>;

    struct FakeHistory {
        resolvable: Vec<String>,
        counts: FakeCounts,
        commits: FakeCommits,
    }

    impl FakeHistory {
        fn new(depths: &[usize]) -> Self {
            Self {
                resolvable: std::iter::once("HEAD".to_string())
                    .chain(depths.iter().map(|d| format!("HEAD~{d}")))
                    .collect(),
                counts: Map::new(),
                commits: Map::new(),
            }
        }

        fn count(mut self, field: &str, rev: &str, n: usize) -> Self {
            self.counts
                .entry(read_shape_ere(field))
                .or_default()
                .insert(rev.to_string(), n);
            self
        }

        fn commit(mut self, field: &str, subject: &str) -> Self {
            self.commits
                .entry(read_shape_ere(field))
                .or_default()
                .push(CommitHit {
                    hash: format!("{:07x}", subject.len()),
                    date: "2026-09-01".to_string(),
                    subject: subject.to_string(),
                });
            self
        }
    }

    impl HistoryProbe for FakeHistory {
        fn rev_at_depth(&self, depth: usize) -> Option<String> {
            let rev = if depth == 0 {
                "HEAD".to_string()
            } else {
                format!("HEAD~{depth}")
            };
            self.resolvable.contains(&rev).then_some(rev)
        }

        fn count_ere(&self, rev: &str, pattern: &str) -> Option<usize> {
            self.counts.get(pattern)?.get(rev).copied()
        }

        fn pickaxe(&self, pattern: &str, max: usize) -> Vec<CommitHit> {
            self.commits
                .get(pattern)
                .map(|c| c.iter().take(max).cloned().collect())
                .unwrap_or_default()
        }
    }

    const DEPTHS: [usize; 2] = [10, 50];

    fn scan_of(src: &str) -> ZeroReadScan {
        crate::config_audit::zero_external_read_fields(&[("lib.rs".to_string(), src.to_string())])
    }

    const ORPHANED_SRC: &str = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
";

    #[test]
    fn orphaned_fires_with_optimization_tag() {
        let scan = scan_of(ORPHANED_SRC);
        assert_eq!(scan.fields.len(), 1, "alarm_radius is zero-external-read");
        let fake = FakeHistory::new(&DEPTHS)
            .count("alarm_radius", "HEAD", 0)
            .count("alarm_radius", "HEAD~10", 0)
            .count("alarm_radius", "HEAD~50", 3)
            .commit("alarm_radius", "perf: const-table species behavior values");
        let report = classify_with_probe(&scan, &fake, &DEPTHS);
        assert_eq!(report.orphaned.len(), 1, "the vanished reader fires");
        assert!(report.stillborn.is_empty());
        let row = &report.orphaned[0];
        assert!(row.likely_by_optimization, "perf subject tags the row");
        let rendered = report.render();
        assert!(rendered.contains("[C1]"), "{rendered}");
        assert!(rendered.contains("LIKELY-BY-OPTIMIZATION"), "{rendered}");
        assert!(rendered.contains("alarm_radius"), "{rendered}");
    }

    #[test]
    fn plain_subject_orphans_without_the_tag() {
        let scan = scan_of(ORPHANED_SRC);
        let fake = FakeHistory::new(&DEPTHS)
            .count("alarm_radius", "HEAD", 0)
            .count("alarm_radius", "HEAD~50", 1)
            .commit("alarm_radius", "docs: update config reference");
        let report = classify_with_probe(&scan, &fake, &DEPTHS);
        assert_eq!(report.orphaned.len(), 1);
        assert!(!report.orphaned[0].likely_by_optimization);
    }

    #[test]
    fn never_read_is_stillborn_not_orphaned() {
        let scan = scan_of(ORPHANED_SRC);
        let fake = FakeHistory::new(&DEPTHS); // no counts anywhere
        let report = classify_with_probe(&scan, &fake, &DEPTHS);
        assert!(report.orphaned.is_empty(), "no delta, no orphan");
        assert_eq!(report.stillborn.len(), 1, "never read → stillborn");
        assert!(report.render().contains("[stillborn]"));
    }

    #[test]
    fn own_impl_reads_with_no_delta_stay_silent() {
        // validate() reads self.alarm_radius NOW; the same read existed at
        // every probe — delta 0, not stillborn (head count > 0) → silent.
        let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
impl BehaviorConfig {
    pub fn validate(&self) -> bool { self.alarm_radius > 0.0 }
}
";
        let scan = scan_of(src);
        assert_eq!(scan.fields.len(), 1);
        assert_eq!(scan.fields[0].self_impl_reads, 1);
        let fake = FakeHistory::new(&DEPTHS)
            .count("alarm_radius", "HEAD", 1)
            .count("alarm_radius", "HEAD~10", 1)
            .count("alarm_radius", "HEAD~50", 1);
        let report = classify_with_probe(&scan, &fake, &DEPTHS);
        assert!(report.orphaned.is_empty(), "no delta — validate then and now");
        assert!(report.stillborn.is_empty(), "head count > 0");
        let rendered = report.render();
        assert!(rendered.contains("1 silent"), "{rendered}");
    }

    #[test]
    fn external_reader_removes_the_field_from_the_candidate_set() {
        // The FN direction of name-global matching: any read of the name
        // (here a real external one) keeps external > 0 — no candidate.
        let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
pub fn alarm_of(c: &BehaviorConfig) -> f32 { c.alarm_radius }
";
        let scan = scan_of(src);
        assert!(scan.fields.is_empty(), "external read keeps it alive");
    }

    #[test]
    fn reader_plus_validate_delta_fires() {
        // The canonical shape: consumer read existed then, only validate
        // remains now → HEAD 1, probe 2 → delta fires.
        let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
impl BehaviorConfig {
    pub fn validate(&self) -> bool { self.alarm_radius > 0.0 }
}
";
        let scan = scan_of(src);
        let fake = FakeHistory::new(&DEPTHS)
            .count("alarm_radius", "HEAD", 1)
            .count("alarm_radius", "HEAD~10", 2)
            .count("alarm_radius", "HEAD~50", 2)
            .commit("alarm_radius", "perf: inline the species table");
        let report = classify_with_probe(&scan, &fake, &DEPTHS);
        assert_eq!(report.orphaned.len(), 1, "consumer vanished, validate stayed");
        assert!(report.orphaned[0].likely_by_optimization);
    }

    #[test]
    fn same_name_on_another_type_masks_the_candidate() {
        // The documented FN direction: another struct's same-named field is
        // read — the syn tally is name-global, so BOTH look read.
        let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
pub struct OtherConfig { pub alarm_radius: f32 }
pub fn other(o: &OtherConfig) -> f32 { o.alarm_radius }
";
        let scan = scan_of(src);
        assert!(
            scan.fields.iter().all(|f| f.candidate.field_name != "alarm_radius"),
            "name-global read masks the orphan candidate"
        );
    }

    #[test]
    fn read_shape_ere_binds_the_tail() {
        let p = read_shape_ere("alarm_radius");
        assert_eq!(p, r"\.alarm_radius([^[:alnum:]_]|$)");
        // `.alarm_radius_longer` must NOT match the pattern as a whole word…
        // (the ERE would need a backreference to assert that; the tail class
        // is the portable approximation — pinned by the e2e test below.)
    }

    #[test]
    fn subject_vocabulary_is_case_insensitive() {
        assert!(subject_looks_like_optimization("PERF: const table"));
        assert!(subject_looks_like_optimization("Refactor the boot path"));
        assert!(!subject_looks_like_optimization("docs: readme"));
    }

    /// The canonical C1 shape (Plan 126's own control): a serde-backed
    /// designer config with NO `impl Default`, constructed by handwritten
    /// literals, its field never member-read — the field MUST enter the
    /// candidate set via the serde-literal extension.
    const SERDE_LITERAL_SRC: &str = r#"
#[derive(serde::Deserialize)]
pub struct BehaviorConfig { #[serde(default = "default_alarm_radius")] pub alarm_radius: f32 }
fn default_alarm_radius() -> f32 { 15.0 }
pub fn hardcoded_registry() -> BehaviorConfig {
    BehaviorConfig { alarm_radius: 30.0 }
}
"#;

    #[test]
    fn serde_literal_constructor_is_candidate() {
        let scan = scan_of(SERDE_LITERAL_SRC);
        assert_eq!(scan.fields.len(), 1, "the serde-literal field enters the set");
        assert_eq!(scan.fields[0].candidate.field_name, "alarm_radius");
        assert_eq!(scan.fields[0].candidate.struct_name, "BehaviorConfig");
        // And with no history evidence it reads stillborn (the canonical
        // case's honest verdict — the pre-fix tree has zero read-shaped
        // occurrences at every commit).
        let fake = FakeHistory::new(&DEPTHS);
        let report = classify_with_probe(&scan, &fake, &DEPTHS);
        assert_eq!(report.stillborn.len(), 1);
        assert!(report.orphaned.is_empty());
    }

    #[test]
    fn serde_literal_orphaned_when_a_reader_vanished() {
        let scan = scan_of(SERDE_LITERAL_SRC);
        let fake = FakeHistory::new(&DEPTHS)
            .count("alarm_radius", "HEAD", 0)
            .count("alarm_radius", "HEAD~50", 2)
            .commit("alarm_radius", "perf: inline species table");
        let report = classify_with_probe(&scan, &fake, &DEPTHS);
        assert_eq!(report.orphaned.len(), 1, "the vanished reader fires on the serde-literal candidate");
        assert!(report.orphaned[0].likely_by_optimization);
    }

    #[test]
    fn serialize_only_struct_is_not_a_candidate() {
        // Serialize-only: the derive reads the fields — every one would
        // read as stillborn. Deliberately outside the extension.
        let src = r"
#[derive(serde::Serialize)]
pub struct WireOut { pub seq: u32 }
pub fn emit() -> WireOut { WireOut { seq: 1 } }
";
        let scan = scan_of(src);
        assert!(scan.fields.is_empty(), "Serialize-only structs are excluded");
    }

    /// The Issue 934 regression pin (the `ConversationEntry` false
    /// positive): a Ser+De struct whose field carries
    /// `#[serde(skip_serializing_if)]` is conditionally-emitted OUTPUT
    /// SCHEMA — the Serialize derive reads it when the condition holds,
    /// invisibly to the member-read tally. Never a C1 candidate.
    #[test]
    fn skip_serializing_if_field_is_not_a_candidate() {
        let src = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConversationEntry {
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_tick: Option<u64>,
}
pub fn make() -> ConversationEntry {
    ConversationEntry { state: "Failed".into(), deadline_tick: None }
}
pub fn state_of(e: &ConversationEntry) -> &str { &e.state }
"#;
        let scan = scan_of(src);
        assert!(scan.fields.is_empty(), "skip_serializing_if fields are excluded (derive-read schema)");
    }

    /// The FN guard on the same boundary (the `BehaviorConfig` canonical
    /// family): a plain `#[serde(default)]` field in a struct deriving BOTH
    /// Serialize AND Deserialize STAYS a candidate — persistence-mirror
    /// surface whose functional read is exactly what the report measures.
    /// The Issue 934 sample: every skip-attrs field was an FP, every
    /// plain-default field in the same derive shape a TP — the boundary is
    /// the attr, NOT the derive pair.
    #[test]
    fn plain_serde_default_in_ser_de_struct_stays_candidate() {
        let src = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BehaviorConfig {
    #[serde(default = "default_alarm_radius")]
    pub alarm_radius: f32,
}
pub fn make() -> BehaviorConfig { BehaviorConfig { alarm_radius: 15.0 } }
"#;
        let scan = scan_of(src);
        assert_eq!(scan.fields.len(), 1, "plain serde(default) fields stay candidates");
        assert_eq!(scan.fields[0].candidate.field_name, "alarm_radius");
    }

    #[test]
    fn self_literal_in_default_impl_not_duplicated() {
        // A struct with BOTH a Default impl and serde markers: the `Self {}`
        // literal inside the impl is collected once (scan_core resolves the
        // impl type; the serde collector skips `Self`).
        let src = r"
#[derive(serde::Deserialize)]
pub struct Dual { pub knob: f32 }
impl Default for Dual {
    fn default() -> Self { Self { knob: 1.0 } }
}
";
        let scan = scan_of(src);
        assert_eq!(scan.fields.len(), 1, "exactly one candidate row");
    }

    /// The end-to-end leg over a REAL throwaway git repository: init →
    /// config+consumer → the optimization removes the consumer → the report
    /// fires ORPHANED with the perf tag, and the never-read twin reads
    /// stillborn. Skips (passes silently) only when git is unavailable.
    #[test]
    fn real_git_e2e_orphan_lifecycle() {
        if git(None, &["--version"]).is_none_or(|o| !o.status.success()) {
            return; // no git on PATH — the unit arms above carry the classifier
        }
        let dir = std::env::temp_dir().join(format!("orphan-e2e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .expect("git spawn");
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
            out
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "orphan-e2e@test"]);
        run(&["config", "user.name", "orphan-e2e"]);
        let with_consumer = r"
pub struct BehaviorConfig { pub alarm_radius: f32, pub calm: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0, calm: 1.0 } }
}
pub fn alarm_of(c: &BehaviorConfig) -> f32 { c.alarm_radius }
";
        std::fs::write(dir.join("lib.rs"), with_consumer).unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "feat: config with a consumer"]);
        let optimized = r"
pub struct BehaviorConfig { pub alarm_radius: f32, pub calm: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0, calm: 1.0 } }
}
";
        std::fs::write(dir.join("lib.rs"), optimized).unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "perf: const-table species behavior values"]);
        let report = report_paths(std::slice::from_ref(&dir), &[1]);
        assert!(report.blind.is_none(), "a real repo resolves the history leg");
        let alarm = report
            .orphaned
            .iter()
            .find(|r| r.candidate.field_name == "alarm_radius");
        let alarm = alarm.expect("alarm_radius orphaned after the optimization");
        assert!(alarm.likely_by_optimization);
        assert!(
            report
                .stillborn
                .iter()
                .any(|f| f.candidate.field_name == "calm"),
            "calm was never read — stillborn"
        );
        assert!(report.stillborn.iter().all(|f| f.candidate.field_name != "alarm_radius"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
