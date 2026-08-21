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
/// from the write path.
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
        rec.record_outcome(
            &traj_key(&r.code, &r.strategy_id),
            &r.shape,
            &r.fix,
            false,
        );
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
    let body = format!(
        "# {code} — unresolved by the rustc_errors playbooks\n\n\
         - **file:** {file}:{line}\n\
         - **message:** {msg}\n\
         - **captured:** by `cargo heal --fix-compile --write` (no candidate kept)\n\n\
         ## Rendered diagnostic\n\n```text\n{rendered}\n```\n",
        code = miss.code,
        file = miss.file.display(),
        line = miss.line,
        msg = miss.message,
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
