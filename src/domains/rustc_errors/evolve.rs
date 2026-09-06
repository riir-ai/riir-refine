//! Phase 4 self-evolution wiring for `--fix-compile` (Plan 019 T4.1 +
//! T4.4) — recording driver outcomes into the [`EvolveRecorder`] memory
//! and dumping misses into the authoring queue.
//!
//! ## The trajectory mapping (the plan's `lint_name`/`rule_id` contract)
//!
//! The plan fixes `lint_name := error_code`, `rule_id := strategy_id` so
//! the memory layer works unmodified. [`FixTrajectory`] carries ONE id,
//! so the compile domain keys trajectories on `"{code}:{strategy_id}"`
//! (`"E0502:e0502-deref-removal"`) — both halves of the contract in one
//! bucket key, and the strategy ids are already family-prefixed so plain
//! greps stay unambiguous either way.
//!
//! ## Verdict soundness (the strongest tier we have)
//!
//! - **applied** → `record_outcome_verified(success = true, verified =
//!   true)`: the driver's keep criterion IS a full-oracle verdict — cargo
//!   check passed on the real crate with strictly fewer coded errors.
//!   That is a Certified trajectory (the only promotion path, Issue 021).
//! - **reverted** → `record_outcome(success = false)`: the oracle rejected
//!   the move at this shape — a plain failure (counts toward the Withdrawn
//!   streak; never certifies).
//!
//! ## Miss capture (T4.4 — the corpus-growth loop)
//!
//! A miss (no playbook proposed, or the playbooks' edits all reverted) is
//! the authoring queue for the NEXT template: `<repo>/.heal/misses/
//! {code}-{ts}.md` with the rendered diagnostic verbatim + a source
//! excerpt. The `wgsl_mine` precedent — mine fix-pairs out of history;
//! here history is handed to you pre-packaged.

use std::path::{Path, PathBuf};

use super::driver::{FixCompileReport, MissRecord};
use crate::recorder::EvolveRecorder;

/// Trajectory bucket key: `"{code}:{strategy_id}"` (the plan's
/// `lint_name`/`rule_id` contract folded into the memory's single id).
#[must_use]
pub fn traj_key(code: &str, strategy_id: &str) -> String {
    format!("{code}:{strategy_id}")
}

/// Record a completed `--fix-compile --write` run into `rec`.
///
/// Review-mode reports record NOTHING (the Issue 018 rule — only edits
/// that were actually written are trajectories), so this is only called
/// from the write path. Reverts are component-attributed at record time
/// (Issue 047): a no-progress revert means the drafted fix was defective
/// (W); an I/O revert means the gate rejected without evaluating the fix
/// (C — an unnecessary, gate-side revert).
pub fn record_run(rec: &mut EvolveRecorder, report: &FixCompileReport) {
    for a in &report.applied {
        rec.record_outcome_verified(
            &traj_key(&a.code, &a.strategy_id),
            &a.shape,
            &a.fix,
            true,
            true, // cargo-check-passed = a full-oracle verdict
        );
    }
    for r in &report.reverted {
        let signal = match r.reason {
            super::driver::RevertReason::NoProgress => {
                crate::component::FailureSignal::reverted(crate::component::RevertKind::CompileGate)
            }
            super::driver::RevertReason::Io => {
                crate::component::FailureSignal::reverted(crate::component::RevertKind::Infrastructure)
            }
        };
        rec.record_failure_signal(&traj_key(&r.code, &r.strategy_id), &r.shape, &r.fix, &signal);
    }
}

/// Where misses land: `<root>/.heal/misses/`. `root` is the workspace (or
/// package) root the caller resolved for the run.
#[must_use]
pub fn misses_dir(root: &Path) -> PathBuf {
    root.join(".heal").join("misses")
}

/// Write one miss as a self-contained authoring-queue entry.
///
/// # Errors
///
/// I/O — the caller warns and continues (a miss dump is an aid, never a
/// gate).
pub fn write_miss(dir: &Path, miss: &MissRecord) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = dir.join(format!("{}-{ts}.md", miss.code));
    let rendered = miss.rendered.as_deref().unwrap_or(&miss.message);
    let body = format!
        (
        "# {code} — unresolved by the rustc_errors playbooks\n\n \
         - **file:** {file}:{line}\n\
         - **message:** {msg}\n\
         - **component:** {component} — no candidate answered this error (corpus gap; author the next template here)\n\
         - **captured:** by `cargo heal --fix-compile --write` (no candidate kept)\n\n \
         ## Rendered diagnostic\n\n```text\n{rendered}\n```\n",
        code = miss.code,
        file = miss.file.display(),
        line = miss.line,
        msg = miss.message,
        component = miss.component,
    );
    std::fs::write(&path, body)?;
    Ok(path)
}

/// The standing README for the misses queue (idempotent — written only
/// when absent so its content stays the documented contract, not a
/// growing log).
///
/// # Errors
///
/// I/O — caller warns and continues.
pub fn write_misses_readme(dir: &Path) -> std::io::Result<()> {
    let readme = dir.join("README.md");
    if readme.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        readme,
        "# The rustc_errors miss queue\n\n\
         Every file here is a compile error the shipped playbooks could not\n\
         fix (no candidate proposed, or every candidate reverted). Each is a\n\
         self-contained authoring request: the rendered diagnostic + location.\n\
         The queue is the input for the next playbook template — grab one,\n\
         reproduce it, probe what rustc actually labels (`cargo check\n\
         --message-format=json`), and author the proposer + fixtures per the\n\
         oracle-first discipline (every FIXED form probe-verified to compile\n\
         BEFORE becoming a fixture).\n\n\
         Files are never edited in place; delete one once its template ships.\n",
    )
}

/// Dump every miss of a report into `root`'s queue. Returns the paths
/// written (empty when there were no misses). Failures are collected as
/// warnings — a miss dump is an aid, never a gate.
#[must_use]
pub fn dump_misses(root: &Path, misses: &[MissRecord]) -> (Vec<PathBuf>, Vec<String>) {
    if misses.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let dir = misses_dir(root);
    let mut written = Vec::with_capacity(misses.len());
    let mut warnings = Vec::new();
    if let Err(e) = write_misses_readme(&dir) {
        warnings.push(format!("misses README: {e}"));
    }
    for m in misses {
        match write_miss(&dir, m) {
            Ok(p) => written.push(p),
            Err(e) => warnings.push(format!("miss {}:{}: {e}", m.file.display(), m.line)),
        }
    }
    (written, warnings)
}

/// One parsed authoring-queue entry (Plan 091 T1.1).
#[derive(Debug, Clone)]
pub struct MissEntry {
    pub path: PathBuf,
    /// The error code — filename stem up to the last `-<digits>` suffix
    /// (`E0502-1730000000000000000.md` → `E0502`); the whole stem when no
    /// digit suffix (hand-authored entries stay queueable).
    pub code: String,
    /// Captured timestamp from the filename suffix, when parseable.
    pub captured_unix_nanos: Option<u128>,
    /// `- **file:** {path}:{line}` from the body, when parseable.
    pub file_line: Option<(String, u32)>,
}

/// Parse one queue entry. Tolerant by design: any `.md` file that is not
/// the README is an entry, and unparsable fields degrade to `None` — the
/// queue is an aid, never a gate. The code is the leading `-`-token of the
/// stem (rustc codes are `E<digits>`, never containing a dash), so both
/// the writer's `{code}-{nanos}.md` form and hand-authored
/// `{code}-note.md` forms pool correctly.
#[must_use]
fn parse_miss_entry(path: PathBuf) -> MissEntry {
    let stem = path
        .file_stem()
        .map_or_else(|| "unknown".to_string(), |s| s.to_string_lossy().into_owned());
    let code = stem.split('-').next().unwrap_or("unknown").to_string();
    let captured = match stem.rsplit_once('-') {
        Some((_, tail)) if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) => {
            tail.parse::<u128>().ok()
        }
        _ => None,
    };
    let body = std::fs::read_to_string(&path).ok();
    let file_line = body.as_deref().and_then(|b| {
        b.lines().find_map(|l| {
            let rest = l.trim().strip_prefix("- **file:** ")?;
            let (f, ln) = rest.rsplit_once(':')?;
            Some((f.to_string(), ln.trim().parse::<u32>().ok()?))
        })
    });
    MissEntry {
        path,
        code,
        captured_unix_nanos: captured,
        file_line,
    }
}

/// Read the whole authoring queue (`.heal/misses/*.md`, README excluded),
/// filename-sorted for determinism (the ts suffix is sortable).
///
/// # Errors
///
/// I/O from the directory read — the caller decides loud-vs-skip.
pub fn read_misses(dir: &Path) -> std::io::Result<Vec<MissEntry>> {
    let mut entries = Vec::new();
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let path = e.path();
        let is_entry = path.extension().is_some_and(|x| x == "md")
            && path.file_name().is_some_and(|n| n.to_string_lossy() != "README.md");
        if is_entry {
            entries.push(parse_miss_entry(path));
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

/// Frontier-order the authoring queue (Plan 091 T1.1) — the real-G8
/// enabler: informative code-pools first (resolving THOSE codes converts
/// uncertain candidates), empty next (one probe is cheap), regenerate,
/// saturated last (the template already always-works or always-fails).
/// Ties break by code then path, deterministic.
///
/// `outcomes_by_code` maps an error code to its observed driver outcomes
/// (`Kept` = success-verified, `Reverted` = failure, `Missed` = no
/// candidate kept). The caller builds it from the trajectory store; an
/// empty map means every pool reads `Empty` and the order degenerates to
/// code-sorted — byte-equivalent information to today's FIFO listing.
#[must_use]
pub fn frontier_order_misses(
    entries: &[MissEntry],
    outcomes_by_code: &std::collections::BTreeMap<String, Vec<crate::dispersion::OutcomeClass>>,
) -> Vec<(MissEntry, crate::dispersion::FrontierVerdict)> {
    // One verdict per CODE (the pool granularity); entries inherit it.
    let codes: std::collections::BTreeSet<&str> =
        entries.iter().map(|e| e.code.as_str()).collect();
    let pool: Vec<crate::dispersion::PoolEntry> = codes
        .iter()
        .map(|&c| {
            crate::dispersion::PoolEntry::new(
                c.to_string(),
                outcomes_by_code.get(c).cloned().unwrap_or_default(),
            )
        })
        .collect();
    let verdicts: std::collections::BTreeMap<String, crate::dispersion::FrontierVerdict> =
        crate::dispersion::frontier_order(&pool)
            .into_iter()
            .map(|v| (v.id.clone(), v))
            .collect();
    let mut ordered: Vec<(MissEntry, crate::dispersion::FrontierVerdict)> = entries
        .iter()
        .filter_map(|e| {
            verdicts.get(&e.code).map(|v| (e.clone(), v.clone()))
        })
        .collect();
    let rank = |r: crate::dispersion::Regime| match r {
        crate::dispersion::Regime::Informative => 0_u8,
        crate::dispersion::Regime::Empty => 1,
        crate::dispersion::Regime::Regenerate => 2,
        crate::dispersion::Regime::Saturated => 3,
    };
    ordered.sort_by(|(a, va), (b, vb)| {
        rank(va.regime)
            .cmp(&rank(vb.regime))
            .then(vb.entropy.total_cmp(&va.entropy))
            .then(a.code.cmp(&b.code))
            .then(a.path.cmp(&b.path))
    });
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traj_key_folds_the_contract() {
        assert_eq!(
            traj_key("E0502", "e0502-deref-removal"),
            "E0502:e0502-deref-removal"
        );
    }

    fn miss(code: &str) -> MissRecord {
        MissRecord {
            code: code.to_string(),
            file: PathBuf::from("/x/src/lib.rs"),
            line: 7,
            message: "cannot borrow".to_string(),
            rendered: Some("error[E0502]...rendered".to_string()),
            component: crate::component::classify(
                &crate::component::FailureSignal::miss_no_candidates(),
            ),
        }
    }

    #[test]
    fn dump_writes_entries_and_readme_once() {
        let root = std::env::temp_dir().join(format!(
            "riir_p19_miss_{}_{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let m = miss("E0502");
        let (written, warns) = dump_misses(&root, std::slice::from_ref(&m));
        assert!(warns.is_empty(), "{warns:?}");
        assert_eq!(written.len(), 1);
        assert!(written[0].file_name().unwrap().to_str().unwrap().starts_with("E0502-"));
        let body = std::fs::read_to_string(&written[0]).unwrap();
        assert!(body.contains("error[E0502]...rendered"), "rendered verbatim");
        assert!(body.contains("/x/src/lib.rs:7"));
        assert!(
            body.contains("**component:** E (corpus/proposer)"),
            "Issue 047 attribution line present: {body}"
        );
        assert!(root.join(".heal").join("misses").join("README.md").exists());

        // Second dump: README NOT rewritten (idempotent), new entry lands.
        let m2 = miss("E0499");
        let (written2, warns2) = dump_misses(&root, std::slice::from_ref(&m2));
        assert!(warns2.is_empty());
        assert_eq!(written2.len(), 1);
        let n: usize = std::fs::read_dir(root.join(".heal").join("misses"))
            .unwrap()
            .filter(|e| {
                e.as_ref().is_ok_and(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    name.to_ascii_lowercase().ends_with(".md") && name != "README.md"
                })
            })
            .count();
        assert_eq!(n, 2, "two entries, one README");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dump_on_no_misses_touches_nothing() {
        let root = std::env::temp_dir().join(format!("riir_p19_miss_none_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (written, warns) = dump_misses(&root, &[]);
        assert!(written.is_empty() && warns.is_empty());
        assert!(!root.exists(), "no dir created for an empty queue");
    }

    // ── Plan 091 T1.1 — read_misses + frontier_order_misses ───────────

    fn seed_queue(dir: &Path, name: &str, body: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, body).unwrap();
        p
    }

    const MISS_BODY: &str = "# E0502 — unresolved by the rustc_errors playbooks\n\
         - **file:** /x/src/lib.rs:7\n\
         - **message:** cannot borrow\n";

    #[test]
    fn read_misses_parses_code_ts_and_file_line_excluding_readme() {
        let dir = std::env::temp_dir().join(format!("p091_read_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        seed_queue(&dir, "E0502-1730000000000000000.md", MISS_BODY);
        seed_queue(&dir, "README.md", "the contract");
        seed_queue(&dir, "E9999-hand-authored.md", "- **file:** /y/a.rs:12\n");
        seed_queue(&dir, "weird.md", "no structured fields\n");
        let mut entries = read_misses(&dir).unwrap();
        assert_eq!(entries.len(), 3, "README excluded");
        entries.sort_by(|a, b| a.code.cmp(&b.code));
        let e = entries.iter().find(|e| e.code == "E0502").unwrap();
        assert_eq!(e.captured_unix_nanos, Some(1_730_000_000_000_000_000));
        assert_eq!(e.file_line, Some(("/x/src/lib.rs".into(), 7)));
        let h = entries.iter().find(|e| e.code == "E9999").unwrap();
        assert_eq!(h.file_line, Some(("/y/a.rs".into(), 12)));
        let w = entries.iter().find(|e| e.code == "weird").unwrap();
        assert_eq!(w.captured_unix_nanos, None, "no digit suffix → whole stem");
        assert_eq!(w.file_line, None, "tolerant of unstructured bodies");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn frontier_order_puts_informative_codes_first_and_orders_within() {
        use crate::dispersion::OutcomeClass;
        let dir = std::env::temp_dir().join(format!("p091_ord_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Filenames in DELIBERATELY anti-frontier order: saturated code
        // (all Kept) first, informative second, empty third.
        seed_queue(&dir, "E0001-1.md", "x");
        seed_queue(&dir, "E0002-2.md", "x");
        seed_queue(&dir, "E0003-3.md", "x");
        let entries = read_misses(&dir).unwrap();
        let mut outcomes = std::collections::BTreeMap::new();
        outcomes.insert(
            "E0001".to_string(),
            vec![OutcomeClass::Kept, OutcomeClass::Kept],
        );
        outcomes.insert(
            "E0002".to_string(),
            vec![OutcomeClass::Kept, OutcomeClass::Reverted],
        );
        // E0003: no record → Empty.
        let ordered = frontier_order_misses(&entries, &outcomes);
        let codes: Vec<&str> = ordered.iter().map(|(e, _)| e.code.as_str()).collect();
        assert_eq!(codes, vec!["E0002", "E0003", "E0001"], "informative, empty, saturated");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn frontier_order_with_no_evidence_degenerates_to_code_sorted() {
        let dir = std::env::temp_dir().join(format!("p091_deg_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        seed_queue(&dir, "E0009-1.md", "x");
        seed_queue(&dir, "E0001-2.md", "x");
        let entries = read_misses(&dir).unwrap();
        let empty = std::collections::BTreeMap::new();
        let ordered = frontier_order_misses(&entries, &empty);
        let codes: Vec<&str> = ordered.iter().map(|(e, _)| e.code.as_str()).collect();
        assert_eq!(codes, vec!["E0001", "E0009"], "all Empty → tie-break by code");
        // Determinism: a second run is identical.
        let again = frontier_order_misses(&entries, &empty);
        assert_eq!(
            ordered.iter().map(|(e, _)| e.path.clone()).collect::<Vec<_>>(),
            again.iter().map(|(e, _)| e.path.clone()).collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_run_buckets_by_code_and_strategy() {
        let mut rec = EvolveRecorder::new();
        let report = FixCompileReport {
            applied: vec![super::super::driver::AppliedRecord {
                file: PathBuf::from("/x/a.rs"),
                line: 3,
                code: "E0502".to_string(),
                strategy_id: "e0502-deref-removal".to_string(),
                errors: (1, 0),
                shape: "&v[0]".to_string(),
                fix: String::new(),
                semantic_note: None,
            }],
            reverted: vec![super::super::driver::RevertedRecord {
                file: PathBuf::from("/x/a.rs"),
                line: 3,
                code: "E0502".to_string(),
                strategy_id: "e0502-clone-out".to_string(),
                reason: super::super::driver::RevertReason::NoProgress,
                shape: "&v[0]".to_string(),
                fix: "v[0].clone()".to_string(),
                semantic_note: None,
            }],
            ..FixCompileReport::default()
        };
        record_run(&mut rec, &report);
        assert_eq!(rec.recorded_count(), 2);
        let mem = rec.memory();
        let applied = mem
            .iter()
            .find(|t| t.lint_name == "E0502:e0502-deref-removal")
            .expect("applied trajectory keyed by code:strategy");
        assert_eq!(applied.success_count, 1, "verified success counted");
        let reverted = mem
            .iter()
            .find(|t| t.lint_name == "E0502:e0502-clone-out")
            .expect("reverted trajectory keyed by code:strategy");
        assert_eq!(reverted.fail_count, 1, "oracle rejection counted");
    }
}
