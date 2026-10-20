//! Plan 049 T0.0b — the span-source walk.
//!
//! The trained-drafter Phase 0 needs two things the tree did not have: a
//! headroom span source that accepts sets other than the hardcoded fixtures
//! (the `*_with_spans` twins on the probe — see `headroom_probe`), and a
//! repo walk the oracle's batch labeler can consume. This module is the
//! walk half, hoisted VERBATIM from the `cargo refine` bin
//! (`src/bin/heal.rs`, which now delegates here) so the bin, the
//! [`crate::draft::clippy_oracle`] batch labeler, and the Plan 049 T0.1
//! dataset builder share ONE walker instead of drifting copies.
//!
//! Semantics preserved from the bin: hidden dirs, `target/` and `vendor/`
//! are pruned (the prune set is [`crate::scan_prune::is_pruned_dir_name`],
//! shared by every scanner walk — see that fn for the full convention); a
//! FILE root matching the extension pushes itself; output order is
//! `read_dir` traversal order (the bin's long-standing behavior — do not
//! change it silently). Dataset code that needs reproducibility across
//! machines uses [`collect_source_files_sorted`]: a BLAKE3-committed span
//! set must not depend on filesystem enumeration order.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Recursive walk pushing every non-hidden file whose name ends with `ext`
/// under `root` (or `root` itself when `root` is a matching file).
/// Build/vendored/dot directories are pruned — [`is_pruned_dir_name`].
///
/// Unreadable directories are skipped silently — a walk over a live
/// workspace must not fail because one dir lacks permissions. Output order
/// is `read_dir` traversal order; see [`collect_source_files_sorted`] for
/// the deterministic form.
pub fn collect_source_files(root: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    if root.is_file() {
        let name = root.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(ext) {
            out.push(root.to_path_buf());
        }
        return;
    }
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root.to_path_buf());
    while let Some(dir) = queue.pop_front() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if crate::scan_prune::is_pruned_dir_name(name) {
                continue;
            }
            if path.is_dir() {
                queue.push_back(path);
            } else if name.ends_with(ext) {
                out.push(path);
            }
        }
    }
}

/// [`collect_source_files`] with a deterministic result: sorted paths.
///
/// The Plan 049 T0.1 dataset half freezes BLAKE3-committed span sets; a
/// commitment over an enumeration-order-dependent sequence is not
/// reproducible across machines, so dataset walks take this form.
#[must_use]
pub fn collect_source_files_sorted(root: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_source_files(root, ext, &mut out);
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A private scratch tree: a.rs, sub/b.rs, target/c.rs, .hidden/d.rs,
    /// e.txt, plus the P78 prune-set members: target-cuda-w2/g.rs (the
    /// CARGO_TARGET_DIR-renamed build tree — the live reflex specimen) and
    /// `node_modules/n.rs`. Unique per test (parallel tests must not delete
    /// each other's tree mid-walk).
    fn temp_tree(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "riir_span_source_test_{tag}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sub")).expect("mkdir sub");
        std::fs::create_dir_all(root.join("target")).expect("mkdir target");
        std::fs::create_dir_all(root.join(".hidden")).expect("mkdir .hidden");
        std::fs::create_dir_all(root.join("target-cuda-w2")).expect("mkdir target-cuda-w2");
        std::fs::create_dir_all(root.join("node_modules")).expect("mkdir node_modules");
        std::fs::write(root.join("a.rs"), "fn a() {}").expect("write a.rs");
        std::fs::write(root.join("sub").join("b.rs"), "fn b() {}").expect("write b.rs");
        std::fs::write(root.join("target").join("c.rs"), "fn c() {}").expect("write c.rs");
        std::fs::write(root.join(".hidden").join("d.rs"), "fn d() {}").expect("write d.rs");
        std::fs::write(root.join("e.txt"), "not rust").expect("write e.txt");
        std::fs::write(root.join("target-cuda-w2").join("g.rs"), "fn g() {}")
            .expect("write g.rs");
        std::fs::write(root.join("node_modules").join("n.rs"), "fn n() {}")
            .expect("write n.rs");
        root
    }

    fn base_names(paths: &[PathBuf]) -> Vec<String> {
        paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn walk_finds_rs_files_and_prunes_target_and_hidden() {
        let root = temp_tree("walk");
        let mut out = Vec::new();
        collect_source_files(&root, ".rs", &mut out);
        let names = base_names(&out);
        assert!(
            names.contains(&"a.rs".to_string()),
            "top-level .rs must be found"
        );
        assert!(
            names.contains(&"b.rs".to_string()),
            "nested .rs must be found"
        );
        assert!(
            !names.contains(&"c.rs".to_string()),
            "target/ must be pruned"
        );
        assert!(
            !names.contains(&"g.rs".to_string()),
            "target-cuda-w2/ (the CARGO_TARGET_DIR-renamed build tree, P78) must be pruned"
        );
        assert!(
            !names.contains(&"n.rs".to_string()),
            "node_modules/ must be pruned"
        );
        assert!(
            !names.contains(&"d.rs".to_string()),
            "hidden dirs must be pruned"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn extension_filters_and_a_matching_file_root_push_itself() {
        let root = temp_tree("ext");
        let mut out = Vec::new();
        collect_source_files(&root, ".txt", &mut out);
        assert_eq!(base_names(&out), vec!["e.txt".to_string()]);

        // A FILE root matching the extension is itself the result.
        let mut one = Vec::new();
        collect_source_files(&root.join("a.rs"), ".rs", &mut one);
        assert_eq!(one.len(), 1);

        // A FILE root NOT matching pushes nothing.
        let mut none = Vec::new();
        collect_source_files(&root.join("e.txt"), ".rs", &mut none);
        assert!(none.is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn sorted_wrapper_is_sorted_and_complete() {
        let root = temp_tree("sorted");
        let sorted = collect_source_files_sorted(&root, ".rs");
        assert_eq!(sorted.len(), 2, "exactly a.rs + sub/b.rs");
        let mut check = sorted.clone();
        check.sort();
        assert_eq!(sorted, check, "output must already be sorted");
        std::fs::remove_dir_all(&root).ok();
    }
}
