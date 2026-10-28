//! Issue 102 T2 + T3 — the corpus active-set marker: the operator-owned
//! demotion state of the retrieval fan-out.
//!
//! A corpus rule that consistently fails to resolve its spans can be
//! DEMOTED — dropped from the similarity fan-out — without leaving the
//! corpus (the fix lane's proposer + oracle anchors still see it; the
//! demotion is a fan-out lifecycle operation, never a corpus deletion).
//! This module owns the marker's file format and the three lifecycle
//! mechanisms:
//!
//! - **T2 — auto re-admit** ([`ActiveSet::auto_readmit_lints`]): a demoted
//!   clippy rule whose lint name re-appears in FRESH unresolved diagnostics
//!   is re-admitted (Research 165 rule 3 verbatim — the demotion was
//!   evidence-based, so fresh evidence of need undoes it).
//! - **T3 — the active cap** ([`ActiveSet::enforce_cap`]): a per-domain
//!   ceiling on live rules, demoting the LOWEST-contribution rules first.
//! - **The operator path**: [`ActiveSet::demote`] / [`ActiveSet::readmit`]
//!   over the marker file — the only demotion entry point until the
//!   fade-predictive gate opens. NOTHING in this module demotes
//!   automatically: `enforce_cap` acts only when an operator wrote a cap,
//!   and the default posture (no marker file at all) is an empty set.
//!
//! Library-Drift warning encoded as the default: over-aggressive retirement
//! measured WORSE than the nuisance it removed, so an absent cap and a `0`
//! cap both mean OFF, and an absent marker file means nothing is demoted.
//!
//! # Domain keys
//!
//! Domain keys are the exact [`crate::draft::DomainId::label`] spellings
//! (`"clippy"`, `"perf"`, `"kernel"`, `"sec"`, `"docker"`, `"dist"`,
//! `"shader"`). That module is `latent_retrieval`-gated and this one is
//! UNGATED, so the types are not shared: the keys are plain strings here,
//! [`CLIPPY_DOMAIN_KEY`] pins the one spelling the auto path needs, and the
//! gated matcher carries the single `DomainId → key` translation (its
//! `active_set_domain_key` helper, test-pinned against
//! [`CLIPPY_DOMAIN_KEY`]) so the vocabularies cannot drift.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The on-disk format version of [`ACTIVE_SET_FILE`]. Written on every
/// save; load accepts any value it can decode (the maps are the operative
/// contract — a future version's extra fields are ignored by serde, its
/// removed fields would fail loudly as a corrupt file, which is the right
/// posture for an operator-authored marker).
pub const ACTIVE_SET_VERSION: u32 = 1;

/// The marker file path, RELATIVE to the repo root the caller resolves
/// (the heal CLI's project root — the same root the `.refine/` trajectory
/// store hangs off).
pub const ACTIVE_SET_FILE: &str = ".refine/active_set.json";

/// The domain key of the clippy domain — the only domain with a
/// fresh-unresolved-span signal (its findings list is the fix lane's only
/// evidence of what still needs a rule), so the only domain
/// [`ActiveSet::auto_readmit_lints`] serves. MUST stay byte-identical to
/// the router's clippy label; pinned by the `latent_matcher` test
/// `active_set_domain_key_matches_the_marker_vocabulary`. Every other
/// domain re-admits via the operator path (edit the marker file).
pub const CLIPPY_DOMAIN_KEY: &str = "clippy";

/// Every domain key the marker may carry — the router's `DomainId::label`
/// spellings. Duplicate-by-necessity: `DomainId` lives behind the
/// `latent_retrieval` feature and this module is ungated, so the list is
/// typed here and the DRIFT is pinned at the one place that CAN see both
/// sides — the `latent_matcher` test
/// `every_domain_label_is_a_known_active_set_key`, which asserts
/// `DomainId::ALL`'s labels against this table. A domain added to the
/// router without a row here fails that test instead of silently
/// rejecting its operator ids at the CLI.
pub const KNOWN_DOMAIN_KEYS: &[&str] = &[
    "clippy", "perf", "kernel", "sec", "docker", "dist", "shader",
];

/// The on-disk marker shape — what [`ActiveSet::save`] writes and
/// [`ActiveSet::load`] reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSetFile {
    /// Format version — always [`ACTIVE_SET_VERSION`] on write.
    pub version: u32,
    /// Domain key → demoted rule ids (dropped from the similarity fan-out).
    pub demoted: BTreeMap<String, BTreeSet<String>>,
    /// Domain key → the ceiling on live (non-demoted) rules. Absent or `0`
    /// = OFF for that domain (the Library-Drift default).
    pub max_active_per_domain: BTreeMap<String, u32>,
}

/// The runtime view of the marker — what the matcher and the fix lane
/// consume. Construct empty ([`ActiveSet::empty`], the default posture) or
/// load from disk ([`ActiveSet::load`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveSet {
    demoted: BTreeMap<String, BTreeSet<String>>,
    max_active_per_domain: BTreeMap<String, u32>,
}

impl From<ActiveSetFile> for ActiveSet {
    fn from(file: ActiveSetFile) -> Self {
        Self {
            demoted: file.demoted,
            max_active_per_domain: file.max_active_per_domain,
        }
    }
}

impl From<&ActiveSet> for ActiveSetFile {
    fn from(set: &ActiveSet) -> Self {
        Self {
            version: ACTIVE_SET_VERSION,
            demoted: set.demoted.clone(),
            max_active_per_domain: set.max_active_per_domain.clone(),
        }
    }
}

/// Load / save failures for the active-set marker. LOUD by design — a
/// corrupt marker is never silently read as empty (that would make a typo'd
/// demotion look like a working default posture; the crate's never-silence
/// law).
#[derive(Debug)]
pub enum ActiveSetError {
    /// The file exists but is not the expected JSON shape.
    Corrupt {
        /// The offending path.
        path: PathBuf,
        /// The serde error underneath.
        source: serde_json::Error,
    },
    /// The file exists but could not be read (permissions, a directory at
    /// the path, …). An ABSENT file is NOT an error — see [`ActiveSet::load`].
    Io {
        /// The offending path.
        path: PathBuf,
        /// The io error underneath.
        source: std::io::Error,
    },
}

impl std::fmt::Display for ActiveSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Corrupt { path, source } => {
                write!(
                    f,
                    "active-set marker {} is corrupt: {source}",
                    path.display()
                )
            }
            Self::Io { path, source } => {
                write!(
                    f,
                    "active-set marker {} unreadable: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ActiveSetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Corrupt { source, .. } => Some(source),
            Self::Io { source, .. } => Some(source),
        }
    }
}

impl ActiveSet {
    /// The default posture: nothing demoted, no caps. Every matcher starts
    /// here.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Read the marker at `path`. An ABSENT file is the default posture,
    /// not an error — most repos never write one. A file that is present
    /// but unreadable or corrupt is LOUD ([`ActiveSetError`]); a silent
    /// empty would make a typo'd marker indistinguishable from "nothing
    /// demoted".
    ///
    /// # Errors
    ///
    /// [`ActiveSetError::Corrupt`] when the file is not the expected JSON
    /// shape; [`ActiveSetError::Io`] when a present file cannot be read.
    pub fn load(path: &Path) -> Result<Self, ActiveSetError> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::empty());
            }
            Err(source) => {
                return Err(ActiveSetError::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };
        let file: ActiveSetFile =
            serde_json::from_str(&text).map_err(|source| ActiveSetError::Corrupt {
                path: path.to_path_buf(),
                source,
            })?;
        Ok(Self::from(file))
    }

    /// [`ActiveSet::load`] at the conventional repo-root-relative path
    /// ([`ACTIVE_SET_FILE`] under `cwd`).
    ///
    /// # Errors
    ///
    /// Same contract as [`ActiveSet::load`].
    pub fn load_default(cwd: &Path) -> Result<Self, ActiveSetError> {
        Self::load(&cwd.join(ACTIVE_SET_FILE))
    }

    /// Whether nothing is demoted AND no cap is set — the byte-identical
    /// posture: a matcher holding an empty set returns exactly what the
    /// default constructor returns (the fan-out honor is skipped outright).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.demoted.is_empty() && self.max_active_per_domain.is_empty()
    }

    /// Whether `rule_id` is demoted out of `domain_key`'s fan-out.
    #[must_use]
    pub fn excludes(&self, domain_key: &str, rule_id: &str) -> bool {
        self.demoted
            .get(domain_key)
            .is_some_and(|set| set.contains(rule_id))
    }

    /// Demote `rule_id` out of `domain_key`'s fan-out (the operator path —
    /// nothing in the crate calls this automatically).
    pub fn demote(&mut self, domain_key: &str, rule_id: &str) {
        self.demoted
            .entry(domain_key.to_string())
            .or_default()
            .insert(rule_id.to_string());
    }

    /// Re-admit `rule_id` into `domain_key`'s fan-out. Returns whether a
    /// demoted entry was actually removed (`false` = was not demoted).
    /// Dropping a domain's LAST demoted entry removes the domain row — the
    /// on-disk file stays minimal.
    pub fn readmit(&mut self, domain_key: &str, rule_id: &str) -> bool {
        let removed = match self.demoted.get_mut(domain_key) {
            Some(set) => set.remove(rule_id),
            None => false,
        };
        if removed {
            let now_empty = self.demoted.get(domain_key).is_some_and(BTreeSet::is_empty);
            if now_empty {
                self.demoted.remove(domain_key);
            }
        }
        removed
    }

    /// The demoted rule ids for `domain_key`, sorted. Empty when none.
    #[must_use]
    pub fn demoted_rules(&self, domain_key: &str) -> Vec<String> {
        self.demoted
            .get(domain_key)
            .map_or_else(Vec::new, |set| set.iter().cloned().collect())
    }

    /// Total demoted rules across every domain — the run-start banner's
    /// count.
    #[must_use]
    pub fn demoted_total(&self) -> usize {
        self.demoted.values().map(BTreeSet::len).sum()
    }

    /// The configured per-domain caps, sorted by domain key — the list
    /// rendering. Empty map = no cap anywhere (the T3 off-by-default
    /// posture).
    #[must_use]
    pub fn caps(&self) -> Vec<(String, u32)> {
        self.max_active_per_domain
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Remove every demotion and every cap — the `--active-set-clear`
    /// body. The corpus itself is never touched (entries are compile-time;
    /// "kept forever" holds by construction).
    pub fn clear(&mut self) {
        self.demoted.clear();
        self.max_active_per_domain.clear();
    }

    /// Set (or replace) the per-domain active ceiling — the
    /// `--active-set-cap domain:N` body. The cap BINDS only when the
    /// admission path enforces it ([`ActiveSet::enforce_cap`]); writing
    /// the marker alone never demotes anything.
    pub fn set_cap(&mut self, domain_key: &str, cap: u32) {
        self.max_active_per_domain
            .insert(domain_key.to_string(), cap);
    }

    /// Remove the per-domain ceiling (`--active-set-cap domain:0`) —
    /// back to the off-by-default posture for that domain.
    pub fn remove_cap(&mut self, domain_key: &str) -> bool {
        self.max_active_per_domain.remove(domain_key).is_some()
    }

    /// Issue 102 T2 — the auto re-admit (Research 165 rule 3 verbatim:
    /// re-admit on the first fresh unresolved span the rule could serve).
    /// Every DEMOTED clippy-domain rule whose rule id appears in
    /// `fired_lints` (fresh unresolved clippy diagnostics) is re-admitted;
    /// returns the re-admitted ids SORTED.
    ///
    /// Scope is clippy-only by construction: the fix lane's only
    /// fresh-unresolved-span signal is the clippy findings list, so only
    /// the [`CLIPPY_DOMAIN_KEY`] domain has an automatic undo. Every other
    /// domain re-admits via the operator path (edit the marker file).
    pub fn auto_readmit_lints(&mut self, fired_lints: &[&str]) -> Vec<String> {
        let Some(demoted) = self.demoted.get(CLIPPY_DOMAIN_KEY) else {
            return Vec::new();
        };
        // BTreeSet iteration is sorted + dedup'd — the return contract falls
        // out of the container.
        let readmit: Vec<String> = demoted
            .iter()
            .filter(|rule| fired_lints.contains(&rule.as_str()))
            .cloned()
            .collect();
        for rule in &readmit {
            self.readmit(CLIPPY_DOMAIN_KEY, rule);
        }
        readmit
    }

    /// Issue 102 T3 — enforce `domain_key`'s active cap. When the domain
    /// carries a cap `C > 0` ([`ActiveSetFile::max_active_per_domain`]) and
    /// more than `C` of `active_rules` are still live (not already
    /// demoted), the LOWEST-contribution live rules are demoted until the
    /// live count fits `C`. Contributions come from `contributions`
    /// (missing = 0); ties break by `rule_id` ascending — deterministic.
    ///
    /// Returns the newly demoted ids in demotion order (lowest contribution
    /// first). Never demotes a rule absent from `active_rules`; never
    /// re-demotes an already-demoted rule (those do not count against the
    /// cap either).
    ///
    /// OFF by default: a cap that is absent, `0`, or ≥ the live count
    /// demotes nothing and returns an empty vec — the Library-Drift
    /// warning (over-aggressive retirement measured WORSE) encoded as the
    /// default posture. Only an operator-written cap acts.
    pub fn enforce_cap(
        &mut self,
        domain_key: &str,
        active_rules: &[String],
        contributions: &BTreeMap<String, u64>,
    ) -> Vec<String> {
        let cap = match self.max_active_per_domain.get(domain_key) {
            Some(&cap) if cap > 0 => cap as usize,
            _ => return Vec::new(),
        };
        let mut live: Vec<&String> = active_rules
            .iter()
            .filter(|rule| !self.excludes(domain_key, rule))
            .collect();
        if live.len() <= cap {
            return Vec::new();
        }
        live.sort_unstable_by(|a, b| {
            let contrib_a = contributions.get(*a).copied().unwrap_or(0);
            let contrib_b = contributions.get(*b).copied().unwrap_or(0);
            contrib_a
                .cmp(&contrib_b)
                .then_with(|| a.as_str().cmp(b.as_str()))
        });
        let excess = live.len() - cap;
        let mut demoted_now = Vec::with_capacity(excess);
        for rule in &live[..excess] {
            self.demote(domain_key, rule);
            demoted_now.push((*rule).clone());
        }
        demoted_now
    }

    /// Write the marker at `path` (pretty JSON, trailing newline, parent
    /// dirs created). Round-trips through [`ActiveSet::load`].
    ///
    /// # Errors
    ///
    /// Propagates [`std::io::Error`] from the dir create or the write.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = ActiveSetFile::from(self);
        let mut text = serde_json::to_string_pretty(&file)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        text.push('\n');
        std::fs::write(path, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pid-scoped scratch dir — concurrent test processes share one
    /// system temp dir, so a fixed path is a truncation race (the
    /// shared-temp-path rule).
    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir_refine_active_set_{}_{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    fn rule(id: &str) -> String {
        id.to_string()
    }

    #[test]
    fn load_absent_file_is_empty() {
        let dir = scratch_dir("absent");
        let marker = dir.join("nope").join("active_set.json");
        let set = ActiveSet::load(&marker).expect("absent is the default posture, not an error");
        assert!(set.is_empty());
        assert!(!set.excludes(CLIPPY_DOMAIN_KEY, "needless_return"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_default_joins_the_marker_path() {
        let dir = scratch_dir("load_default");
        // Same absent-file contract through the convenience path.
        let set = ActiveSet::load_default(&dir).expect("absent stays OK");
        assert!(set.is_empty());
        // And the path it joined is the documented one.
        let mut set = ActiveSet::empty();
        set.demote(CLIPPY_DOMAIN_KEY, "needless_return");
        let marker = dir.join(ACTIVE_SET_FILE);
        set.save(&marker).expect("save");
        assert!(
            ActiveSet::load_default(&dir)
                .expect("reload")
                .excludes(CLIPPY_DOMAIN_KEY, "needless_return")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_is_a_loud_error_never_a_silent_empty() {
        let dir = scratch_dir("corrupt");
        let marker = dir.join(ACTIVE_SET_FILE);
        std::fs::create_dir_all(marker.parent().expect("has parent")).expect("dir");
        std::fs::write(&marker, "{ not json").expect("write garbage");
        match ActiveSet::load(&marker) {
            Err(ActiveSetError::Corrupt { path, .. }) => {
                assert_eq!(path, marker);
            }
            other => panic!("a corrupt marker must be a loud Corrupt error, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_but_present_file_is_an_io_error() {
        let dir = scratch_dir("io_err");
        // A DIRECTORY at the marker path: present (not NotFound), unreadable
        // as a file — exercises the Io arm without chmod games.
        std::fs::create_dir_all(dir.join(ACTIVE_SET_FILE)).expect("plant a dir");
        match ActiveSet::load(&dir.join(ACTIVE_SET_FILE)) {
            Err(ActiveSetError::Io { .. }) => {}
            other => {
                panic!("a present-but-unreadable marker must be a loud Io error, got {other:?}")
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn demote_save_load_round_trip() {
        let dir = scratch_dir("round_trip");
        let mut set = ActiveSet::empty();
        set.demote(CLIPPY_DOMAIN_KEY, "needless_return");
        set.demote(CLIPPY_DOMAIN_KEY, "manual_clamp");
        set.demote("perf", "vec-with-capacity");
        set.max_active_per_domain.insert("perf".to_string(), 40);
        let marker = dir.join(ACTIVE_SET_FILE);
        set.save(&marker).expect("save");
        let raw = std::fs::read_to_string(&marker).expect("read back");
        assert!(raw.ends_with('\n'), "trailing newline on write");
        assert!(raw.contains("\"version\": 1"), "version recorded");
        let reloaded = ActiveSet::load(&marker).expect("reload");
        assert_eq!(reloaded, set, "round-trip preserves demotions + caps");
        assert!(reloaded.excludes(CLIPPY_DOMAIN_KEY, "needless_return"));
        assert_eq!(
            reloaded.demoted_rules(CLIPPY_DOMAIN_KEY),
            vec![rule("manual_clamp"), rule("needless_return")],
            "sorted"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn readmit_removes_and_reports() {
        let mut set = ActiveSet::empty();
        set.demote(CLIPPY_DOMAIN_KEY, "needless_return");
        assert!(
            set.readmit(CLIPPY_DOMAIN_KEY, "needless_return"),
            "known demotion removed"
        );
        assert!(
            !set.readmit(CLIPPY_DOMAIN_KEY, "needless_return"),
            "second readmit reports false"
        );
        assert!(
            !set.readmit(CLIPPY_DOMAIN_KEY, "never_demoted"),
            "unknown rule reports false"
        );
        assert!(
            !set.readmit("perf", "needless_return"),
            "unknown domain reports false"
        );
        // Dropping the last demoted entry drops the domain row.
        set.demote(CLIPPY_DOMAIN_KEY, "last_one");
        set.readmit(CLIPPY_DOMAIN_KEY, "last_one");
        assert!(
            set.is_empty(),
            "empty demoted map + no caps = the default posture"
        );
    }

    #[test]
    fn auto_readmit_fires_only_on_demoted_clippy_rules_sorted_and_deduped() {
        let mut set = ActiveSet::empty();
        set.demote(CLIPPY_DOMAIN_KEY, "needless_return");
        set.demote(CLIPPY_DOMAIN_KEY, "zombie_rule");
        set.demote(CLIPPY_DOMAIN_KEY, "manual_clamp");
        set.demote("perf", "vec-with-capacity");
        // Duplicates in the fired list dedup; the return is sorted; rules
        // not demoted (fresh_clamp) are ignored; the perf demotion is
        // untouched — other domains re-admit via the operator path only.
        let readmitted = set.auto_readmit_lints(&[
            "zombie_rule",
            "fresh_clamp",
            "zombie_rule",
            "needless_return",
        ]);
        assert_eq!(
            readmitted,
            vec![rule("needless_return"), rule("zombie_rule")],
            "sorted, deduped, demoted-clippy-only"
        );
        assert_eq!(
            set.demoted_rules(CLIPPY_DOMAIN_KEY),
            vec![rule("manual_clamp")],
            "only the fired rules were re-admitted"
        );
        assert_eq!(
            set.demoted_rules("perf"),
            vec![rule("vec-with-capacity")],
            "non-clippy domains are never auto re-admitted"
        );
    }

    #[test]
    fn auto_readmit_with_nothing_demoted_is_a_noop() {
        let mut set = ActiveSet::empty();
        set.demote("perf", "vec-with-capacity");
        assert_eq!(
            set.auto_readmit_lints(&["vec-with-capacity"]),
            Vec::<String>::new()
        );
        assert_eq!(set.auto_readmit_lints(&[]), Vec::<String>::new());
        assert_eq!(set.demoted_rules("perf"), vec![rule("vec-with-capacity")]);
    }

    #[test]
    fn enforce_cap_demotes_the_lowest_contribution_first() {
        // THE planted-bug arm: flipping the contribution comparison
        // (descending instead of ascending) demotes the HIGH contributor
        // and this test fails. One demotion slot, two candidates with
        // clearly different contributions.
        let mut set = ActiveSet::empty();
        set.max_active_per_domain
            .insert(CLIPPY_DOMAIN_KEY.to_string(), 1);
        let active = vec![rule("b_low"), rule("a_high")];
        let mut contributions = BTreeMap::new();
        contributions.insert("b_low".to_string(), 1);
        contributions.insert("a_high".to_string(), 100);
        let demoted = set.enforce_cap(CLIPPY_DOMAIN_KEY, &active, &contributions);
        assert_eq!(
            demoted,
            vec![rule("b_low")],
            "lowest contribution goes first"
        );
        assert!(set.excludes(CLIPPY_DOMAIN_KEY, "b_low"));
        assert!(
            !set.excludes(CLIPPY_DOMAIN_KEY, "a_high"),
            "the high contributor stays live"
        );
    }

    #[test]
    fn enforce_cap_missing_contribution_counts_as_zero() {
        let mut set = ActiveSet::empty();
        set.max_active_per_domain
            .insert(CLIPPY_DOMAIN_KEY.to_string(), 1);
        let active = vec![rule("listed"), rule("unlisted")];
        let mut contributions = BTreeMap::new();
        contributions.insert("listed".to_string(), 5);
        // "unlisted" is absent from the map → contributes 0 → demoted first.
        let demoted = set.enforce_cap(CLIPPY_DOMAIN_KEY, &active, &contributions);
        assert_eq!(demoted, vec![rule("unlisted")]);
    }

    #[test]
    fn enforce_cap_ties_break_by_rule_id_ascending() {
        let mut set = ActiveSet::empty();
        set.max_active_per_domain.insert("perf".to_string(), 1);
        let active = vec![rule("zeta"), rule("alpha"), rule("mid")];
        // No contributions at all: every rule ties at 0 → pure id order.
        let contributions = BTreeMap::new();
        let demoted = set.enforce_cap("perf", &active, &contributions);
        assert_eq!(
            demoted,
            vec![rule("alpha"), rule("mid")],
            "id-ascending, deterministic"
        );
        assert!(set.excludes("perf", "alpha") && set.excludes("perf", "mid"));
        assert!(!set.excludes("perf", "zeta"));
    }

    #[test]
    fn enforce_cap_respects_the_cap_exactly_and_reports_demotion_order() {
        let mut set = ActiveSet::empty();
        set.max_active_per_domain
            .insert(CLIPPY_DOMAIN_KEY.to_string(), 2);
        let active: Vec<String> = ["a", "b", "c", "d", "e"].iter().map(|s| rule(s)).collect();
        let mut contributions = BTreeMap::new();
        for (id, contrib) in [("a", 9), ("b", 7), ("c", 5), ("d", 3), ("e", 1)] {
            contributions.insert(rule(id), contrib);
        }
        let demoted = set.enforce_cap(CLIPPY_DOMAIN_KEY, &active, &contributions);
        assert_eq!(
            demoted,
            vec![rule("e"), rule("d"), rule("c")],
            "demotion order = ascending contribution"
        );
        let live: Vec<&String> = active
            .iter()
            .filter(|r| !set.excludes(CLIPPY_DOMAIN_KEY, r))
            .collect();
        assert_eq!(live.len(), 2, "live count fits the cap exactly");
    }

    #[test]
    fn enforce_cap_is_a_noop_when_absent_zero_or_loose() {
        let active = vec![rule("a"), rule("b")];
        let contributions = BTreeMap::new();

        // Absent cap: OFF.
        let mut set = ActiveSet::empty();
        assert_eq!(
            set.enforce_cap(CLIPPY_DOMAIN_KEY, &active, &contributions),
            Vec::<String>::new()
        );
        assert!(!set.excludes(CLIPPY_DOMAIN_KEY, "a"));

        // Zero cap: OFF (the Library-Drift default posture).
        set.max_active_per_domain
            .insert(CLIPPY_DOMAIN_KEY.to_string(), 0);
        assert_eq!(
            set.enforce_cap(CLIPPY_DOMAIN_KEY, &active, &contributions),
            Vec::<String>::new()
        );
        assert!(!set.excludes(CLIPPY_DOMAIN_KEY, "b"));

        // Cap ≥ live count: nothing to do.
        set.max_active_per_domain
            .insert(CLIPPY_DOMAIN_KEY.to_string(), 2);
        assert_eq!(
            set.enforce_cap(CLIPPY_DOMAIN_KEY, &active, &contributions),
            Vec::<String>::new()
        );
        // A noop pass demotes nothing — the CAP map may hold the armed cap,
        // but the demoted set (what the fan-out reads) stays empty.
        assert_eq!(set.demoted_rules(CLIPPY_DOMAIN_KEY), Vec::<String>::new());
    }

    #[test]
    fn enforce_cap_never_demotes_rules_absent_from_active() {
        let mut set = ActiveSet::empty();
        set.max_active_per_domain
            .insert(CLIPPY_DOMAIN_KEY.to_string(), 1);
        let active = vec![rule("in_active")];
        // Contributions name rules that are NOT active — they must never be
        // demoted, and an active rule absent from contributions counts as 0.
        let mut contributions = BTreeMap::new();
        contributions.insert(rule("phantom"), 0);
        let demoted = set.enforce_cap(CLIPPY_DOMAIN_KEY, &active, &contributions);
        assert!(
            demoted.is_empty(),
            "one live rule fits cap 1 — nothing demoted"
        );
        let two = vec![rule("in_active"), rule("also_active")];
        let demoted = set.enforce_cap(CLIPPY_DOMAIN_KEY, &two, &contributions);
        assert_eq!(
            demoted,
            vec![rule("also_active")],
            "tie at 0 breaks by id; phantom is unreachable"
        );
        assert!(
            !set.excludes(CLIPPY_DOMAIN_KEY, "phantom"),
            "non-active rules are never demoted"
        );
    }

    #[test]
    fn enforce_cap_ignores_already_demoted_rules() {
        let mut set = ActiveSet::empty();
        set.demote(CLIPPY_DOMAIN_KEY, "b_already_gone");
        set.max_active_per_domain
            .insert(CLIPPY_DOMAIN_KEY.to_string(), 2);
        // 4 active but one is already demoted → 3 live → cap 2 demotes ONE,
        // and the already-demoted rule neither counts against the cap nor
        // appears in the return.
        let active = vec![
            rule("a_high"),
            rule("b_already_gone"),
            rule("c_high"),
            rule("d_low"),
        ];
        let mut contributions = BTreeMap::new();
        for (id, contrib) in [
            ("a_high", 50),
            ("b_already_gone", 40),
            ("c_high", 30),
            ("d_low", 1),
        ] {
            contributions.insert(rule(id), contrib);
        }
        let demoted = set.enforce_cap(CLIPPY_DOMAIN_KEY, &active, &contributions);
        // d_low (contribution 1) is the lowest LIVE rule — b is demoted and
        // skipped no matter its contribution — so d demotes, and a/c stay
        // live at the cap of 2.
        assert_eq!(
            demoted,
            vec![rule("d_low")],
            "only the lowest LIVE rule demotes"
        );
        assert!(set.excludes(CLIPPY_DOMAIN_KEY, "d_low"));
        assert!(!set.excludes(CLIPPY_DOMAIN_KEY, "a_high"));
        assert!(
            !set.excludes(CLIPPY_DOMAIN_KEY, "c_high"),
            "both survivors fill the cap"
        );
    }

    #[test]
    fn active_set_file_serde_round_trip_including_version() {
        let file = ActiveSetFile {
            version: ACTIVE_SET_VERSION,
            demoted: [
                (
                    CLIPPY_DOMAIN_KEY.to_string(),
                    BTreeSet::from(["needless_return".to_string()]),
                ),
                (
                    "perf".to_string(),
                    BTreeSet::from(["vec-with-capacity".to_string()]),
                ),
            ]
            .into_iter()
            .collect(),
            max_active_per_domain: [("perf".to_string(), 40)].into_iter().collect(),
        };
        let text = serde_json::to_string(&file).expect("serialize");
        let decoded: ActiveSetFile = serde_json::from_str(&text).expect("deserialize");
        assert_eq!(decoded, file);
        assert_eq!(decoded.version, ACTIVE_SET_VERSION);
        // An empty file decodes to its default (all fields present).
        let empty = ActiveSetFile::default();
        let text = serde_json::to_string(&empty).expect("serialize default");
        assert_eq!(
            serde_json::from_str::<ActiveSetFile>(&text).expect("deserialize default"),
            empty
        );
    }

    #[test]
    fn clear_demoted_total_and_caps_report_the_runtime_view() {
        let mut set = ActiveSet::empty();
        assert_eq!(set.demoted_total(), 0);
        assert_eq!(set.caps(), Vec::<(String, u32)>::new());
        set.demote("clippy", "needless_return");
        set.demote("perf", "vec-with-capacity");
        set.max_active_per_domain.insert("perf".to_string(), 40);
        assert_eq!(set.demoted_total(), 2);
        assert_eq!(
            set.caps(),
            vec![("perf".to_string(), 40)],
            "caps render sorted-by-domain"
        );
        set.clear();
        assert!(set.is_empty());
        assert_eq!(set.demoted_total(), 0);
        assert_eq!(set.caps(), Vec::<(String, u32)>::new());
    }

    #[test]
    fn every_known_domain_key_is_nonempty_and_unique() {
        // The CLI validates operator ids against this table — a duplicate or
        // empty key would misroute a demotion silently.
        let mut seen = std::collections::BTreeSet::new();
        for key in KNOWN_DOMAIN_KEYS {
            assert!(!key.is_empty(), "empty domain key in KNOWN_DOMAIN_KEYS");
            assert!(seen.insert(*key), "duplicate domain key {key:?}");
        }
        assert!(seen.contains(CLIPPY_DOMAIN_KEY));
    }
}
