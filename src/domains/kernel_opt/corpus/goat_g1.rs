//! The `kernel_opt` GOAT gate G1 harness — the standing fixture validator
//! (Issue 191 T2).
//!
//! The corpus's newer rules (Batch ~249+, `entries_gpu_110`–`entries_gpu_147`)
//! carry **executable `#[test]` fixtures** embedded in their `ref_shape` /
//! `opt_pattern` strings — std-only Rust reproductions with deterministic
//! inputs and ref-vs-opt asserts (many `to_bits()` bit-identity, some
//! documented tolerance classes). The entry discipline validated each fixture
//! ONCE manually at landing (`rustc --test --edition 2021`, the file headers);
//! this harness makes that discipline **standing**: extraction, ONE compile +
//! run per fixture-bearing rule under a two-posture CASCADE, and the
//! census/floor/reconciliation assertions of the ratified T1 bars.
//!
//! ## The standing verdict store (Issue 191 T3)
//!
//! `--emit-status <path>` writes the digest-keyed verdict table committed at
//! [`G1_STATUS_TABLE`] (`g1_status.tsv`): one row per rule — status,
//! `rule_id`, blake3 of `ref‖opt`, blake3 of `opt`. The [`g1_record_for`]
//! lookup (the `context_notes` annotation channel) matches the `rule_id` then
//! FAILS CLOSED to [`G1Record::Unverified`] on a digest mismatch — a corpus
//! edit that moves a rule's strings past the last G1 run reads as unverified,
//! never as green. Digests key CONTENT, not position.
//!
//! ## The compile cascade (measured at the first full runs)
//!
//! The corpus carries BOTH fixture shapes, and neither is universal:
//!
//! 1. **PAIR posture** (tried first) — the REF arm at the crate root, the OPT
//!    arm wrapped in a module that glob-imports the root (`use super::*;`):
//!    the root's private items are visible to every descendant module, so the
//!    opt side's cross-arm tests resolve the ref side's naive forms, while the
//!    opt arm's own redefinitions (each arm stays self-READING) legally shadow
//!    the glob. (Two sibling modules + `use super::__ref::*` does NOT work —
//!    siblings cannot see each other's private items; measured.)
//! 2. **PER-ARM posture** (fallback, only when the pair fails to COMPILE) —
//!    each test-bearing arm compiled standalone, the corpus's historical
//!    discipline: the older batches' ref arms are illustrative fragments or
//!    shape sketches, and their opt tests are fully self-contained. A pair
//!    that fails at RUN does not fall back — a failing test fails everywhere.
//!
//! The deciding posture is recorded; the failed pair attempt is disclosed in
//! `discarded` when per-arm validated the rule.
//!
//! ## The ratified G1 contract (Issue 191, T1 verdict 2026-11-01)
//!
//! - **Anti-circularity:** the floors are pinned against an INDEPENDENT
//!   structural census (a raw-text count of `#[test]` tokens per corpus source
//!   file, never the extraction path), and the harness asserts per-file
//!   reconciliation — `raw_tokens − extracted_tokens` equals a membership pin
//!   (zero unless pinned in [`CENSUS_DELTA_PINS`]). A rule whose strings fail
//!   to extract counts as **FAIL, never absent from population** — a new
//!   `entries_gpu_NN` module added to [`super::build_kernel_seed_corpus`] but
//!   missing from [`expected_corpus_files`] (or vice versa) reds the gate
//!   loudly instead of silently shrinking it.
//! - **Floors fire downward only:** `MIN_FIXTURE_RULES` (248) and
//!   `MIN_FIXTURE_FILES` (72) — pinned to the harness's own exact first
//!   census (T1's 290/70 were the pre-harness estimate); adding fixtures
//!   never reds; dropping below does.
//! - **Verdict classes:** every fixture-bearing rule must compile AND pass
//!   its embedded tests under the cascade (pair preferred, per-arm fallback
//!   on pair compile failure). Parse-only rules are disclosed
//!   (`parse_only`), never counted as validated. A compile/run/timeout
//!   failure under BOTH postures is the rule's G1 verdict FAIL.
//! - **Epistemic caveat (recorded at ratification):** G1 proves opt preserves
//!   ref on the fixture's own input distribution; ref correctness is inherited
//!   from vendored provenance — ref and opt can share a blind spot the fixture
//!   author never probed.
//!
//! ## Box law
//!
//! Every compile/run lands in a pid-suffixed temp dir
//! ([`work_dir`]) — the shared-temp-path law (a fixed path is safe against
//! sibling tests in one binary, not against another PROCESS running the same
//! test). Compile posture matches the entry discipline exactly: plain
//! `rustc --edition 2021 --test` (debug profile, overflow checks on — the
//! stricter arm).

use serde::Serialize;

use crate::domains::kernel_opt::KernelRule;

/// The corpus source modules the extraction must see — `entries_popcorn` plus
/// every `entries_gpu_NN` batch file (Issue 191 T1 census law: membership
/// here MUST equal the `extend` list in [`super::build_kernel_seed_corpus`];
/// any divergence is a loud census mismatch, never a silent shrink).
///
/// Generated: `entries_gpu_01`..`entries_gpu_09` zero-padded, `entries_gpu_10`
/// onward natural.
#[must_use]
pub fn expected_corpus_files() -> Vec<String> {
    let mut names = vec!["entries_popcorn".to_string()];
    for n in 1..=157 {
        if n < 10 {
            names.push(format!("entries_gpu_{n:02}"));
        } else {
            names.push(format!("entries_gpu_{n}"));
        }
    }
    names
}

/// G1 floors — fire DOWNWARD only (Issue 191 bar 1, census-pinned). Pinned to
/// the harness's own EXACT per-rule census at first measurement: 248
/// fixture-bearing rules across 72 files. (T1's ≥290/≥70 were the pre-harness
/// per-file-density ESTIMATE; the ratified bar pins the census — any fixture
/// rule or fixture file lost from the standing population reds.)
pub const MIN_FIXTURE_RULES: usize = 248;
pub const MIN_FIXTURE_FILES: usize = 72;

/// Per-file census row — the same two numbers measured on both paths
/// (raw disk text vs extracted rule strings).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileCensus {
    /// Corpus module name (e.g. `entries_gpu_117`).
    pub file: String,
    /// Rules declared in the file (extraction path only; raw path carries 0 —
    /// a rule count is not a text-level property).
    pub rules: usize,
    /// Rules whose `ref_shape` or `opt_pattern` carries `#[test]`.
    pub fixture_rules: usize,
    /// Total `#[test]` tokens (raw text path: every occurrence in the file;
    /// extraction path: every occurrence inside the rules' strings).
    pub test_tokens: usize,
}

/// The full census over all corpus files.
#[derive(Debug, Clone, Serialize)]
pub struct G1Census {
    pub per_file: Vec<FileCensus>,
    /// Total declared rules (extraction path).
    pub total_rules: usize,
    /// Rules carrying at least one fixture arm.
    pub fixture_rules: usize,
    /// Files carrying at least one fixture rule.
    pub fixture_files: usize,
    /// Total `#[test]` tokens across the corpus.
    pub test_tokens: usize,
}

impl G1Census {
    fn empty() -> Self {
        Self {
            per_file: Vec::new(),
            total_rules: 0,
            fixture_rules: 0,
            fixture_files: 0,
            test_tokens: 0,
        }
    }
}

/// The corpus directory on disk (compile-time anchor — the census reads the
/// crate's own source, so the population cannot drift with the CWD).
fn corpus_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domains/kernel_opt/corpus")
}

/// Count `#[test]` token occurrences in raw text.
fn count_test_tokens(text: &str) -> usize {
    text.matches("#[test]").count()
}

/// The INDEPENDENT structural census: raw-text reads of every expected corpus
/// file. This path never touches the corpus API — it is the pin the extraction
/// is asserted against (Issue 191 T1 amendment 1).
///
/// # Errors
/// A missing expected file (a module declared but its `.rs` absent, or a
/// renamed batch) is a census error, never a silent zero.
pub fn structural_census() -> std::io::Result<G1Census> {
    let dir = corpus_dir();
    let mut census = G1Census::empty();
    for name in expected_corpus_files() {
        let path = dir.join(format!("{name}.rs"));
        let text = std::fs::read_to_string(&path).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "G1 census: expected corpus file {} unreadable ({}); \
                     if the module was renamed/removed, update expected_corpus_files() \
                     AND build_kernel_seed_corpus() in the same commit",
                    path.display(),
                    e
                ),
            )
        })?;
        let tokens = count_test_tokens(&text);
        census.per_file.push(FileCensus {
            file: name,
            rules: 0,                               // not a text-level property
            fixture_rules: usize::from(tokens > 0), // file-level flag
            test_tokens: tokens,
        });
        census.test_tokens += tokens;
        if tokens > 0 {
            census.fixture_files += 1;
        }
    }
    Ok(census)
}

/// The extraction census: the SAME token population measured over the rules
/// the corpus API actually hands out, grouped per module via the `pub(super)`
/// per-file entry builders (per-file attribution the whole-corpus path cannot
/// give). ALL THREE string fields count (ref + opt + `gpu_concept` prose) — the
/// reconciliation quantity is `raw text minus string fields`, i.e. exactly the
/// tokens that live in the file as REAL code rather than inside a rule string.
/// `fixture_rules`/`fixture_files` stay ref+opt-only: the floor counts
/// EXECUTABLE fixtures, never prose mentions. Order follows
/// [`expected_corpus_files`]; per-file delta reconciliation against the
/// independent raw census is asserted by [`compare`].
#[must_use]
pub fn extracted_census() -> G1Census {
    let mut census = G1Census::empty();
    for (name, entries) in per_file_entries() {
        let mut fixture_rules = 0usize;
        let mut tokens = 0usize;
        for rule in &entries {
            let arm_tokens =
                count_test_tokens(&rule.ref_shape) + count_test_tokens(&rule.opt_pattern);
            if arm_tokens > 0 {
                fixture_rules += 1;
            }
            tokens += arm_tokens + count_test_tokens(&rule.gpu_concept);
        }
        census.per_file.push(FileCensus {
            file: name,
            rules: entries.len(),
            fixture_rules,
            test_tokens: tokens,
        });
        census.total_rules += entries.len();
        census.fixture_rules += fixture_rules;
        if fixture_rules > 0 {
            census.fixture_files += 1;
        }
        census.test_tokens += tokens;
    }
    census
}

/// Per-file entry sets, in [`expected_corpus_FILES`] order. Lives inside the
/// `corpus` module precisely so the `pub(super)` per-file builders are
/// reachable — this is the attribution the whole-corpus `build_corpus()` path
/// cannot provide.
// plan 329 integration: the batch! macro expansion reads as 160 body
// lines but is one declarative list — the allow is the fix, not a split.
#[allow(clippy::too_many_lines)]
fn per_file_entries() -> Vec<(String, Vec<KernelRule>)> {
    macro_rules! batch {
        ($($mod:ident),* $(,)?) => {
            vec![ $( (stringify!($mod).to_string(), super::$mod::entries()) ),* ]
        };
    }
    // NOTE (Issue 191 T2): a NEW batch file must be added here AND to
    // build_kernel_seed_corpus() in the SAME commit — the census comparison
    // fails loud on divergence, by the ratified anti-circularity law.
    let mut all = batch![
        entries_gpu_38,
        entries_gpu_39,
        entries_gpu_40,
        entries_gpu_41,
        entries_gpu_42,
        entries_gpu_43,
        entries_gpu_44,
        entries_gpu_45,
        entries_gpu_46,
        entries_gpu_47,
        entries_gpu_48,
        entries_gpu_49,
        entries_gpu_50,
        entries_gpu_51,
        entries_gpu_52,
        entries_gpu_53,
        entries_gpu_54,
        entries_gpu_55,
        entries_gpu_56,
        entries_gpu_57,
        entries_gpu_58,
        entries_gpu_59,
        entries_gpu_60,
        entries_gpu_61,
        entries_gpu_62,
        entries_gpu_63,
        entries_gpu_64,
        entries_gpu_65,
        entries_gpu_66,
        entries_gpu_67,
        entries_gpu_68,
        entries_gpu_69,
        entries_gpu_70,
        entries_gpu_71,
        entries_gpu_72,
        entries_gpu_73,
        entries_gpu_74,
        entries_gpu_75,
        entries_gpu_76,
        entries_gpu_77,
        entries_gpu_78,
        entries_gpu_79,
        entries_gpu_80,
        entries_gpu_81,
        entries_gpu_82,
        entries_gpu_83,
        entries_gpu_84,
        entries_gpu_85,
        entries_gpu_86,
        entries_gpu_87,
        entries_gpu_88,
        entries_gpu_89,
        entries_gpu_90,
        entries_gpu_91,
        entries_gpu_92,
        entries_gpu_93,
        entries_gpu_94,
        entries_gpu_95,
        entries_gpu_96,
        entries_gpu_97,
        entries_gpu_98,
        entries_gpu_99,
        entries_gpu_100,
        entries_gpu_101,
        entries_gpu_102,
        entries_gpu_103,
        entries_gpu_104,
        entries_gpu_105,
        entries_gpu_106,
        entries_gpu_107,
        entries_gpu_108,
        entries_gpu_109,
        entries_gpu_110,
        entries_gpu_111,
        entries_gpu_112,
        entries_gpu_113,
        entries_gpu_114,
        entries_gpu_115,
        entries_gpu_116,
        entries_gpu_117,
        entries_gpu_118,
        entries_gpu_119,
        entries_gpu_120,
        entries_gpu_121,
        entries_gpu_122,
        entries_gpu_123,
        entries_gpu_124,
        entries_gpu_125,
        entries_gpu_126,
        entries_gpu_127,
        entries_gpu_128,
        entries_gpu_129,
        entries_gpu_130,
        entries_gpu_131,
        entries_gpu_132,
        entries_gpu_133,
        entries_gpu_134,
        entries_gpu_135,
        entries_gpu_136,
        entries_gpu_137,
        entries_gpu_138,
        entries_gpu_139,
        entries_gpu_140,
        entries_gpu_141,
        entries_gpu_142,
        entries_gpu_143,
        entries_gpu_144,
        entries_gpu_145,
        entries_gpu_146,
        entries_gpu_147,
        entries_gpu_148,
        entries_gpu_149,
        entries_gpu_150,
        entries_gpu_151,
        entries_gpu_152,
        entries_gpu_153,
        entries_gpu_154,
        entries_gpu_155,
        entries_gpu_156,
        entries_gpu_157,
        entries_gpu_37,
        entries_gpu_36,
        entries_gpu_01,
        entries_gpu_02,
        entries_gpu_03,
        entries_gpu_04,
        entries_gpu_05,
        entries_gpu_06,
        entries_gpu_07,
        entries_gpu_08,
        entries_gpu_09,
        entries_gpu_10,
        entries_gpu_11,
        entries_gpu_12,
        entries_gpu_13,
        entries_gpu_14,
        entries_gpu_15,
        entries_gpu_16,
        entries_gpu_17,
        entries_gpu_18,
        entries_gpu_19,
        entries_gpu_20,
        entries_gpu_21,
        entries_gpu_22,
        entries_gpu_23,
        entries_gpu_24,
        entries_gpu_25,
        entries_gpu_26,
        entries_gpu_27,
        entries_gpu_28,
        entries_gpu_29,
        entries_gpu_30,
        entries_gpu_31,
        entries_gpu_32,
        entries_gpu_33,
        entries_gpu_34,
        entries_gpu_35,
    ];
    let popcorn = (
        String::from("entries_popcorn"),
        super::entries_popcorn::entries(),
    );
    all.insert(0, popcorn);
    all
}

/// Known per-file reconciliation deltas: `raw #[test]` tokens MINUS string-field
/// tokens, i.e. tokens living in the file as REAL in-tree code (`#[cfg(test)]`
/// modules, top-level `#[test]` items) rather than inside a rule's strings.
/// Membership pins — red BOTH directions: a measured delta off its pin FAILS,
/// a pin that stops measuring FAILS, and an unpinned file with a nonzero delta
/// FAILS (extraction lost content, or new in-tree code landed unpinned).
const CENSUS_DELTA_PINS: &[(&str, usize, &str)] = &[
    // (file, expected_delta, reason)
    (
        "entries_gpu_63",
        1,
        "file-level #[cfg(test)] mod batch195_slice_tests — one in-tree \
         structural #[test], real code outside every rule string (stands under \
         cargo test --lib; recorded, never re-run by the harness)",
    ),
    (
        "entries_gpu_64",
        1,
        "file-level #[cfg(test)] mod batch196_slice_tests — one in-tree \
         structural #[test], real code outside every rule string (stands under \
         cargo test --lib; recorded, never re-run by the harness)",
    ),
];

/// Census reconciliation — the anti-circularity assertion (T1 amendment 1,
/// as reconciled): per-file `raw_tokens − extracted_tokens` must equal the
/// pinned delta (zero unless pinned in [`CENSUS_DELTA_PINS`]), and the file
/// sets must match. A divergence names the remedy in its error text.
///
/// # Errors
/// Any per-file delta off-pin, a negative delta (extraction INVENTED content),
/// an orphan pin row, any expected file missing from the extraction, or any
/// extracted file not expected — each is a loud `Err` naming the file.
pub fn compare(raw: &G1Census, extracted: &G1Census) -> Result<(), String> {
    let raw_by: std::collections::HashMap<&str, &FileCensus> =
        raw.per_file.iter().map(|f| (f.file.as_str(), f)).collect();
    let ext_by: std::collections::HashMap<&str, &FileCensus> = extracted
        .per_file
        .iter()
        .map(|f| (f.file.as_str(), f))
        .collect();
    let mut problems = Vec::new();
    for f in &raw.per_file {
        match ext_by.get(f.file.as_str()) {
            None => problems.push(format!(
                "file {} in the raw census but ABSENT from extraction — a module \
                 added to expected_corpus_files() without per_file_entries() (or the \
                 corpus extend list): FAIL, never absent-from-population",
                f.file
            )),
            Some(e) => {
                let Some(measured) = f.test_tokens.checked_sub(e.test_tokens) else {
                    problems.push(format!(
                        "file {}: extraction carries {} `#[test]` tokens but the raw \
                         text holds only {} — extraction INVENTED fixture content",
                        f.file, e.test_tokens, f.test_tokens
                    ));
                    continue;
                };
                match CENSUS_DELTA_PINS
                    .iter()
                    .find(|(name, _, _)| name == &f.file)
                {
                    Some((_, expected, reason)) => {
                        if measured != *expected {
                            problems.push(format!(
                                "file {}: in-tree `#[test]` delta moved — pin says {} \
                                 ({}), measured {} — fix the corpus or move the pin \
                                 in the same commit",
                                f.file, expected, reason, measured
                            ));
                        }
                    }
                    None if measured != 0 => problems.push(format!(
                        "file {}: raw text carries {} `#[test]` token(s) beyond its \
                         rule strings (in-tree code) and NO pin row — extraction lost \
                         content, or new in-tree corpus code landed unpinned: add a \
                         CENSUS_DELTA_PINS row (count + reason) in the same commit",
                        f.file, measured
                    )),
                    None => {}
                }
            }
        }
    }
    for f in &extracted.per_file {
        if !raw_by.contains_key(f.file.as_str()) {
            problems.push(format!(
                "file {} extracted but NOT in expected_corpus_files() — add it to \
                 the expected list AND build_kernel_seed_corpus() in the same commit",
                f.file
            ));
        }
    }
    for (name, _, _) in CENSUS_DELTA_PINS {
        if !raw_by.contains_key(name) {
            problems.push(format!(
                "pin row names file {name} but no such file is in the raw census — \
                 an orphan pin (renamed/removed corpus file): fix or delete the row \
                 in the same commit"
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "G1 census reconciliation failed (per-file raw-minus-extracted `#[test]` \
             deltas must equal their membership pins):\n{}",
            problems.join("\n")
        ))
    }
}

/// A rule's standing G1 validation record — the annotation vocabulary of the
/// digest-keyed verdict store (Issue 191 T3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G1Record {
    /// The cascade compiled AND ran the rule's fixture; every embedded test
    /// passed.
    Green,
    /// The fixture failed to compile or its own asserts failed (Issue 193).
    Fail,
    /// No executable fixture (the parse-only disclosed class).
    ParseOnly,
    /// No store row matches this rule's CURRENT strings — the corpus moved
    /// past the last committed G1 run. Fail-closed: never reads as green.
    Unverified,
}

impl G1Record {
    /// The `validated:` annotation text (the `context_notes` channel —
    /// propagated end-to-end via `DraftedFix::context_notes`).
    #[must_use]
    pub fn annotation(self) -> &'static str {
        match self {
            Self::Green => {
                "validated: fixture-green — the G1 harness compiled AND executed \
                 this rule's embedded fixture; the ref-vs-opt claim is \
                 machine-checked on the fixture's own inputs (Issue 191 T2)"
            }
            Self::Fail => {
                "validated: fixture-defect — the G1 harness caught this rule's \
                 fixture failing compile or its own asserts (Issue 193); the \
                 suggestion is parse-valid, its fixture claim UNVERIFIED"
            }
            Self::ParseOnly => {
                "validated: parse-only — this rule carries no executable fixture; \
                 the suggestion is syntax-validated only"
            }
            Self::Unverified => {
                "validated: unverified — the corpus strings moved past the \
                 committed G1 record (digest mismatch); re-run `cargo run \
                 --release --example kernel_goat_g1 --features kernel_opt -- \
                 --full --emit-status <path>`"
            }
        }
    }
}

/// The committed, generated verdict store (never hand-edited — regenerate via
/// `kernel_goat_g1 --full --emit-status`). Rows: `status\trule_id\tpair\topt`;
/// digests are blake3 hex over `ref\u{1f}opt` and `opt` respectively.
const G1_STATUS_TABLE: &str = include_str!("g1_status.tsv");

/// blake3 over `ref_src \u{1f} opt_src` — the pair-content key.
#[must_use]
pub fn pair_digest(ref_src: &str, opt_src: &str) -> String {
    let mut h = blake3::Hasher::new();
    h.update(ref_src.as_bytes());
    h.update(&[0x1F]);
    h.update(opt_src.as_bytes());
    h.finalize().to_string()
}

/// blake3 over `opt_src` — the draft-content key (a draft byte-identical to a
/// rule's opt side inherits that rule's standing record).
#[must_use]
pub fn opt_digest(opt_src: &str) -> String {
    blake3::Hasher::new()
        .update(opt_src.as_bytes())
        .finalize()
        .to_string()
}

/// One row of the committed verdict store.
#[derive(Debug, Clone, Serialize)]
pub struct G1StatusRow {
    #[serde(serialize_with = "serialize_record")] // serde hands &T; the enum is Copy
    pub status: G1Record,
    pub rule_id: String,
    /// blake3(`ref\u{1f}opt`) — the corpus-drift key.
    pub pair: String,
    /// blake3(`opt`) — the draft-inheritance key.
    pub opt: String,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde's serialize_with hands &T
fn serialize_record<S: serde::Serializer>(r: &G1Record, s: S) -> Result<S::Ok, S::Error> {
    let name = match r {
        G1Record::Green => "green",
        G1Record::Fail => "fail",
        G1Record::ParseOnly => "parse-only",
        G1Record::Unverified => "unverified",
    };
    s.serialize_str(name)
}

/// Build the status rows for a FULL report (a sampled run covers a subset —
/// the emitter refuses anything but the whole population).
#[must_use]
pub fn status_rows(report: &G1Report) -> Vec<G1StatusRow> {
    let verdict_of: std::collections::HashMap<&str, G1Record> = report
        .rules
        .iter()
        .map(|r| (r.rule_id.as_str(), r.outcome().into()))
        .collect();
    let mut rows = Vec::new();
    for (file, entries) in per_file_entries() {
        let _ = file;
        for rule in entries {
            let Some(&status) = verdict_of.get(rule.rule_id.as_str()) else {
                continue; // not covered by this report (sampled run)
            };
            rows.push(G1StatusRow {
                status,
                rule_id: rule.rule_id.clone(),
                pair: pair_digest(&rule.ref_shape, &rule.opt_pattern),
                opt: opt_digest(&rule.opt_pattern),
            });
        }
    }
    rows
}

/// Emit the committed-store TSV. Deterministic: corpus build order.
///
/// # Errors
/// A write failure is the caller's loud error.
pub fn emit_status_table(report: &G1Report, path: &std::path::Path) -> std::io::Result<()> {
    use std::fmt::Write as _;
    let mut out = String::from(
        "# g1_status v1 — generated by kernel_goat_g1 --emit-status; do not hand-edit\n",
    );
    for row in status_rows(report) {
        let status = match row.status {
            G1Record::Green => "green",
            G1Record::Fail => "fail",
            G1Record::ParseOnly => "parse-only",
            G1Record::Unverified => unreachable!("store rows are measured, never unverified"),
        };
        let _ = writeln!(out, "{status}\t{}\t{}\t{}", row.rule_id, row.pair, row.opt);
    }
    std::fs::write(path, out)
}

/// The parsed store, keyed by `rule_id` (digests live in the value). Built
/// once per process; keys borrow the committed static text.
fn status_table()
-> &'static std::collections::HashMap<&'static str, (G1Record, &'static str, &'static str)> {
    static TABLE: std::sync::OnceLock<
        std::collections::HashMap<&'static str, (G1Record, &'static str, &'static str)>,
    > = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut map = std::collections::HashMap::new();
        for line in G1_STATUS_TABLE.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut f = line.split('\t');
            let (Some(status), Some(rule_id), Some(pair), Some(opt)) =
                (f.next(), f.next(), f.next(), f.next())
            else {
                continue; // a malformed row reads as absent, never as green
            };
            let status = match status {
                "green" => G1Record::Green,
                "fail" => G1Record::Fail,
                "parse-only" => G1Record::ParseOnly,
                _ => continue,
            };
            map.insert(rule_id, (status, pair, opt));
        }
        map
    })
}

/// The standing record for one corpus rule: match by `rule_id`, then FAIL
/// CLOSED on a pair-digest mismatch (the corpus moved past the store).
#[must_use]
pub fn g1_record_for(rule_id: &str, ref_src: &str, opt_src: &str) -> G1Record {
    match status_table().get(rule_id) {
        Some((status, pair, _)) if *pair == pair_digest(ref_src, opt_src) => *status,
        _ => G1Record::Unverified,
    }
}

/// Whether a DRAFT (byte-identical to some rule's opt side) inherits a GREEN
/// standing record — the `g1_backed` disclosure of the `kernel_verify`
/// verdict. The pair-green law applies: the RULE's fixture compiled and ran;
/// the draft alone may still reference its ref arm's symbols (cross-arm
/// fixture shape) and is not proven to compile standalone.
#[must_use]
pub fn g1_opt_backed(opt_src: &str) -> bool {
    let digest = opt_digest(opt_src);
    status_table()
        .values()
        .any(|(status, _, opt)| *status == G1Record::Green && *opt == digest)
}

/// One fixture rule's execution verdict (the PAIRED compile unit).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ArmVerdict {
    /// `pair` — the ref+opt sibling-module unit.
    pub arm: String,
    pub compile_ok: bool,
    pub run_ok: bool,
    /// Wall-clock seconds (compile + run), for the record.
    pub seconds: f64,
    /// Failure detail (rustc stderr tail, test-failure tail, or timeout note).
    pub error: Option<String>,
}

/// One rule's G1 verdict.
#[derive(Debug, Clone, Serialize)]
pub struct RuleVerdict {
    pub rule_id: String,
    pub file: String,
    /// true when at least one arm carries `#[test]`.
    pub fixture_bearing: bool,
    /// The DECIDING posture's arm verdicts (`pair`, or the per-arm set).
    pub arms: Vec<ArmVerdict>,
    /// A pair attempt that failed to COMPILE while the per-arm fallback
    /// validated the rule — disclosed, never pooled into the verdict.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub discarded: Vec<ArmVerdict>,
}

impl RuleVerdict {
    /// The rule's G1 outcome: `Green`, `Fail`, or `ParseOnly` (disclosed,
    /// never counted as validated).
    #[must_use]
    pub fn outcome(&self) -> G1Outcome {
        if !self.fixture_bearing {
            return G1Outcome::ParseOnly;
        }
        if self.arms.iter().all(|a| a.compile_ok && a.run_ok) && !self.arms.is_empty() {
            G1Outcome::Green
        } else {
            G1Outcome::Fail
        }
    }
}

/// The three G1 outcome classes.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum G1Outcome {
    Green,
    Fail,
    ParseOnly,
}

impl From<G1Outcome> for G1Record {
    fn from(o: G1Outcome) -> Self {
        match o {
            G1Outcome::Green => Self::Green,
            G1Outcome::Fail => Self::Fail,
            G1Outcome::ParseOnly => Self::ParseOnly,
        }
    }
}

/// The full G1 report.
#[derive(Debug, Clone, Serialize)]
pub struct G1Report {
    pub census_check: Result<(), String>,
    pub fixture_rules: usize,
    pub fixture_files: usize,
    pub green: usize,
    pub fail: usize,
    pub parse_only: usize,
    pub seconds: f64,
    pub rustc_version: String,
    pub rules: Vec<RuleVerdict>,
}

impl G1Report {
    /// The gate verdict: census equality + floors + every fixture rule green.
    ///
    /// # Errors
    /// Census mismatch, a floor firing downward, or the named FAIL rules.
    pub fn verdict(&self) -> Result<(), String> {
        if let Err(e) = &self.census_check {
            return Err(e.clone());
        }
        let mut problems = Vec::new();
        if self.fixture_rules < MIN_FIXTURE_RULES {
            problems.push(format!(
                "floor: fixture-bearing rules {} < {} (floors fire downward only)",
                self.fixture_rules, MIN_FIXTURE_RULES
            ));
        }
        if self.fixture_files < MIN_FIXTURE_FILES {
            problems.push(format!(
                "floor: fixture-bearing files {} < {} (floors fire downward only)",
                self.fixture_files, MIN_FIXTURE_FILES
            ));
        }
        if self.fail > 0 {
            let fails: Vec<String> = self
                .rules
                .iter()
                .filter(|r| r.outcome() == G1Outcome::Fail)
                .map(|r| r.rule_id.clone())
                .collect();
            problems.push(format!(
                "{} fixture rule(s) FAILED the compile/run cascade: {}",
                fails.len(),
                fails.join(", ")
            ));
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("\n"))
        }
    }
}

/// A pid-suffixed work dir (the shared-temp-path law: safe against sibling
/// processes running the same harness).
#[must_use]
pub fn work_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("riir_refine_kgoat_g1_{}", std::process::id()))
}

/// The per-file extraction, exposed for the G2 tranche selector (the SAME
/// extraction the census reconciles — never a second parser).
#[must_use]
pub fn per_file_entries_for_tranche() -> Vec<(String, Vec<KernelRule>)> {
    per_file_entries()
}

fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map_or_else(
            |_| "rustc unavailable".to_string(),
            |o| String::from_utf8_lossy(&o.stdout).trim().to_string(),
        )
}

/// Spawn + poll with output to FILES (the G1 law: a large child stderr
/// through a pipe would fill the 64 KiB buffer, block the child, and read as
/// a false TIMEOUT).
///
/// # Errors
/// A spawn/`try_wait` failure, a `File::create` failure, or the timeout
/// firing — named in the `String`.
pub fn run_with_timeout(
    mut cmd: std::process::Command,
    timeout: std::time::Duration,
    out_path: &std::path::Path,
    err_path: &std::path::Path,
) -> Result<std::process::ExitStatus, String> {
    // std has no timeout on Command::output; spawn + poll, with output going
    // to FILES (a large rustc stderr through a pipe would fill the 64 KiB
    // buffer, block the child, and read as a false TIMEOUT).
    use std::process::Stdio;
    let out_f = std::fs::File::create(out_path)
        .map_err(|e| format!("create {} failed: {e}", out_path.display()))?;
    let err_f = std::fs::File::create(err_path)
        .map_err(|e| format!("create {} failed: {e}", err_path.display()))?;
    cmd.stdin(Stdio::null()).stdout(out_f).stderr(err_f);
    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timeout after {}s", timeout.as_secs_f64().round()));
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(e) => return Err(format!("try_wait failed: {e}")),
        }
    }
}

fn read_file(p: &std::path::Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

pub const COMPILE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);
pub const RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

fn tail(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.len() <= max {
        s.to_string()
    } else {
        let cut = s.len() - max;
        // avoid splitting a UTF-8 boundary
        let cut = (cut..s.len())
            .find(|&i| s.is_char_boundary(i))
            .unwrap_or(s.len());
        format!("…{}", &s[cut..])
    }
}

/// The paired compile unit: the REF arm sits at the crate ROOT and the OPT
/// arm wraps in a module that glob-imports the root. This shape is load-bearing:
/// sibling modules cannot see each other's PRIVATE items (`use
/// super::__ref::*` would import nothing — measured), but the root's private
/// items are visible to every descendant module, so the opt side's equivalence
/// tests resolve the ref side's naive forms. The opt arm's own redefinitions
/// (deliberate — each arm stays self-READING) legally shadow the glob import.
fn pair_source(ref_src: &str, opt_src: &str) -> String {
    let mut s = String::new();
    s.push_str(ref_src);
    s.push_str("\nmod __opt {\nuse super::*;\n");
    s.push_str(opt_src);
    s.push_str("\n}\n");
    s
}

/// Compile ONE arm standalone (the per-arm fallback posture) and run its
/// embedded tests (`rustc --edition 2021 --test`).
#[allow(clippy::too_many_lines)] // one linear compile-then-run sequence; splitting it hides the ordering
fn run_arm(dir: &std::path::Path, stem: &str, source: &str) -> ArmVerdict {
    let started = std::time::Instant::now();
    let src_path = dir.join(format!("{stem}.rs"));
    let bin_path = dir.join(format!("{stem}_t.exe"));
    if let Err(e) = std::fs::write(&src_path, source) {
        return ArmVerdict {
            arm: stem_arm(stem),
            compile_ok: false,
            run_ok: false,
            seconds: started.elapsed().as_secs_f64(),
            error: Some(format!("write {} failed: {e}", src_path.display())),
        };
    }
    let c_out = dir.join(format!("{stem}.c_out"));
    let c_err = dir.join(format!("{stem}.c_err"));
    let mut compile_cmd = std::process::Command::new("rustc");
    compile_cmd
        .arg("--edition")
        .arg("2021")
        .arg("--test")
        .arg(&src_path)
        .arg("-o")
        .arg(&bin_path)
        .current_dir(dir);
    let compile = match run_with_timeout(compile_cmd, COMPILE_TIMEOUT, &c_out, &c_err) {
        Ok(status) => {
            if !status.success() {
                return ArmVerdict {
                    arm: stem_arm(stem),
                    compile_ok: false,
                    run_ok: false,
                    seconds: started.elapsed().as_secs_f64(),
                    error: Some(tail(&read_file(&c_err), 1200)),
                };
            }
            status
        }
        Err(e) => {
            return ArmVerdict {
                arm: stem_arm(stem),
                compile_ok: false,
                run_ok: false,
                seconds: started.elapsed().as_secs_f64(),
                error: Some(format!(
                    "rustc: {} | stderr: {}",
                    e,
                    tail(&read_file(&c_err), 400)
                )),
            };
        }
    };
    let _ = compile;
    let r_out = dir.join(format!("{stem}.r_out"));
    let r_err = dir.join(format!("{stem}.r_err"));
    let mut run_cmd = std::process::Command::new(&bin_path);
    run_cmd.current_dir(dir);
    match run_with_timeout(run_cmd, RUN_TIMEOUT, &r_out, &r_err) {
        Ok(status) if status.success() => ArmVerdict {
            arm: stem_arm(stem),
            compile_ok: true,
            run_ok: true,
            seconds: started.elapsed().as_secs_f64(),
            error: None,
        },
        Ok(_) => ArmVerdict {
            arm: stem_arm(stem),
            compile_ok: true,
            run_ok: false,
            seconds: started.elapsed().as_secs_f64(),
            error: Some(failure_detail(&r_out, &r_err)),
        },
        Err(e) => ArmVerdict {
            arm: stem_arm(stem),
            compile_ok: true,
            run_ok: false,
            seconds: started.elapsed().as_secs_f64(),
            error: Some(format!(
                "run: {} | stderr: {}",
                e,
                tail(&read_file(&r_err), 400)
            )),
        },
    }
}

fn stem_arm(stem: &str) -> String {
    if stem.ends_with("_ref") {
        "ref".to_string()
    } else {
        "opt".to_string()
    }
}

/// The two-posture compile cascade for one fixture-bearing rule: the PAIR
/// unit first (the modern cross-arm shape); on a pair COMPILE failure, the
/// test-bearing arms standalone (the historical discipline). A pair that
/// fails at RUN does not fall back — a failing test fails everywhere.
/// Returns `(deciding, discarded)`.
fn run_rule_cascade(
    dir: &std::path::Path,
    rule_id: &str,
    ref_src: &str,
    opt_src: &str,
    arms_src: &[(&str, &String)],
) -> (Vec<ArmVerdict>, Vec<ArmVerdict>) {
    let stem = format!("k000_{}_pair", sanitize(rule_id));
    let pair = run_pair(dir, &stem, ref_src, opt_src);
    if pair.compile_ok {
        return (vec![pair], Vec::new());
    }
    let mut deciding = Vec::with_capacity(arms_src.len() + 1);
    for (i, (arm, src)) in arms_src.iter().enumerate() {
        let stem = format!("k{:03}_{}_{}", i + 1, sanitize(rule_id), arm);
        deciding.push(run_arm(dir, &stem, src));
    }
    (deciding, vec![pair])
}

/// Compile one rule's PAIRED fixture unit — ref + opt as sibling modules —
/// and run its embedded tests (`rustc --edition 2021 --test`).
#[allow(clippy::too_many_lines)] // one linear compile-then-run sequence; splitting it hides the ordering
fn run_pair(dir: &std::path::Path, stem: &str, ref_src: &str, opt_src: &str) -> ArmVerdict {
    let started = std::time::Instant::now();
    let source = pair_source(ref_src, opt_src);
    let src_path = dir.join(format!("{stem}.rs"));
    let bin_path = dir.join(format!("{stem}_t.exe"));
    if let Err(e) = std::fs::write(&src_path, source) {
        return ArmVerdict {
            arm: "pair".to_string(),
            compile_ok: false,
            run_ok: false,
            seconds: started.elapsed().as_secs_f64(),
            error: Some(format!("write {} failed: {e}", src_path.display())),
        };
    }

    let c_out = dir.join(format!("{stem}.c_out"));
    let c_err = dir.join(format!("{stem}.c_err"));
    let mut compile_cmd = std::process::Command::new("rustc");
    compile_cmd
        .arg("--edition")
        .arg("2021")
        .arg("--test")
        .arg(&src_path)
        .arg("-o")
        .arg(&bin_path)
        .current_dir(dir);
    let compile = match run_with_timeout(compile_cmd, COMPILE_TIMEOUT, &c_out, &c_err) {
        Ok(status) => {
            if !status.success() {
                return ArmVerdict {
                    arm: "pair".to_string(),
                    compile_ok: false,
                    run_ok: false,
                    seconds: started.elapsed().as_secs_f64(),
                    error: Some(tail(&read_file(&c_err), 1200)),
                };
            }
            status
        }
        Err(e) => {
            return ArmVerdict {
                arm: "pair".to_string(),
                compile_ok: false,
                run_ok: false,
                seconds: started.elapsed().as_secs_f64(),
                error: Some(format!(
                    "rustc: {} | stderr: {}",
                    e,
                    tail(&read_file(&c_err), 400)
                )),
            };
        }
    };
    let _ = compile;

    let r_out = dir.join(format!("{stem}.r_out"));
    let r_err = dir.join(format!("{stem}.r_err"));
    let mut run_cmd = std::process::Command::new(&bin_path);
    run_cmd.current_dir(dir);
    match run_with_timeout(run_cmd, RUN_TIMEOUT, &r_out, &r_err) {
        Ok(status) if status.success() => ArmVerdict {
            arm: "pair".to_string(),
            compile_ok: true,
            run_ok: true,
            seconds: started.elapsed().as_secs_f64(),
            error: None,
        },
        Ok(_) => ArmVerdict {
            arm: "pair".to_string(),
            compile_ok: true,
            run_ok: false,
            seconds: started.elapsed().as_secs_f64(),
            // libtest's `failures:` report goes to STDOUT — read that first,
            // stderr only as the fallback.
            error: Some(failure_detail(&r_out, &r_err)),
        },
        Err(e) => ArmVerdict {
            arm: "pair".to_string(),
            compile_ok: true,
            run_ok: false,
            seconds: started.elapsed().as_secs_f64(),
            error: Some(format!(
                "run: {} | stderr: {}",
                e,
                tail(&read_file(&r_err), 400)
            )),
        },
    }
}

/// Run-failure diagnosis: the test harness prints its failure report on
/// stdout; a crashed binary prints on stderr. Whichever carries text wins.
fn failure_detail(out_path: &std::path::Path, err_path: &std::path::Path) -> String {
    let out = read_file(out_path);
    let err = read_file(err_path);
    if out.trim().is_empty() {
        tail(&err, 1200)
    } else {
        tail(&out, 1200)
    }
}

/// Run the G1 harness.
///
/// - `filter`: substring match on `rule_id` (None = all rules).
/// - `limit`: cap on fixture-bearing rules executed (None = all). The census
///   assertions run REGARDLESS of the filter/limit — the population law binds
///   even a sampled run; the sampled run's `verdict()` with a limit still
///   checks floors against the FULL census numbers (recorded in the report).
#[must_use]
pub fn run_g1(filter: Option<&str>, limit: Option<usize>) -> G1Report {
    let started = std::time::Instant::now();
    let raw = structural_census().map_err(|e| format!("structural census failed: {e}"));
    let extracted = extracted_census();
    let census_check = raw.and_then(|r| compare(&r, &extracted));

    let mut rules = Vec::new();
    for (file, entries) in per_file_entries() {
        for rule in entries {
            if let Some(f) = filter {
                if !rule.rule_id.contains(f) {
                    continue;
                }
            }
            let mut arms_src: Vec<(&str, &String)> = Vec::new();
            if rule.ref_shape.contains("#[test]") {
                arms_src.push(("ref", &rule.ref_shape));
            }
            if rule.opt_pattern.contains("#[test]") {
                arms_src.push(("opt", &rule.opt_pattern));
            }
            let fixture_bearing = !arms_src.is_empty();
            if let Some(cap) = limit {
                if fixture_bearing
                    && rules
                        .iter()
                        .filter(|r: &&RuleVerdict| r.fixture_bearing)
                        .count()
                        >= cap
                {
                    // keep iterating only to finish the current rule set? No —
                    // the population census already ran; a sampled execution
                    // stops here.
                    return finish(started, census_check, &extracted, rules);
                }
            }
            let mut arms = Vec::new();
            let mut discarded = Vec::new();
            if fixture_bearing {
                let dir = work_dir();
                let _ = std::fs::create_dir_all(&dir);
                let (deciding, failed_pair) = run_rule_cascade(
                    &dir,
                    &rule.rule_id,
                    &rule.ref_shape,
                    &rule.opt_pattern,
                    &arms_src,
                );
                arms = deciding;
                discarded = failed_pair;
                let _ = std::fs::remove_dir_all(&dir);
            }
            rules.push(RuleVerdict {
                rule_id: rule.rule_id.clone(),
                file: file.clone(),
                fixture_bearing,
                arms,
                discarded,
            });
        }
    }
    finish(started, census_check, &extracted, rules)
}

fn sanitize(rule_id: &str) -> String {
    rule_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn finish(
    started: std::time::Instant,
    census_check: Result<(), String>,
    extracted: &G1Census,
    rules: Vec<RuleVerdict>,
) -> G1Report {
    let green = rules
        .iter()
        .filter(|r| r.outcome() == G1Outcome::Green)
        .count();
    let fail = rules
        .iter()
        .filter(|r| r.outcome() == G1Outcome::Fail)
        .count();
    let parse_only = rules
        .iter()
        .filter(|r| r.outcome() == G1Outcome::ParseOnly)
        .count();
    G1Report {
        census_check,
        fixture_rules: extracted.fixture_rules,
        fixture_files: extracted.fixture_files,
        green,
        fail,
        parse_only,
        seconds: started.elapsed().as_secs_f64(),
        rustc_version: rustc_version(),
        rules,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_corpus_files_cover_all_batches() {
        let names = expected_corpus_files();
        assert_eq!(names.len(), 158); // popcorn + gpu_01..154 (the B335 repair caught 148; B336 keeps the census current at landing; B338 continued; B339 continued; B337 continued; B341 continued; B345 continues)
        assert_eq!(names[0], "entries_popcorn");
        assert!(names.contains(&"entries_gpu_09".to_string()));
        assert!(names.contains(&"entries_gpu_10".to_string()));
        assert!(names.contains(&"entries_gpu_147".to_string()));
    }

    #[test]
    fn extraction_reconciles_with_the_independent_structural_census() {
        // The T1 anti-circularity law, standing: per-file raw-minus-extracted
        // `#[test]` deltas equal their membership pins (zero unless pinned).
        let raw = structural_census().expect("census reads its own crate sources");
        let extracted = extracted_census();
        compare(&raw, &extracted).expect("census reconciliation");
    }

    #[test]
    fn floors_hold_at_the_current_corpus() {
        let extracted = extracted_census();
        assert!(
            extracted.fixture_rules >= MIN_FIXTURE_RULES,
            "fixture rules {} below floor {}",
            extracted.fixture_rules,
            MIN_FIXTURE_RULES
        );
        assert!(
            extracted.fixture_files >= MIN_FIXTURE_FILES,
            "fixture files {} below floor {}",
            extracted.fixture_files,
            MIN_FIXTURE_FILES
        );
    }
}
