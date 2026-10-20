//! The shared directory-name prune convention for every scanner walk —
//! ungated so the config-audit lane (deliberately feature-free) and the
//! latent-retrieval-gated `draft/span_source` walk consume ONE definition
//! instead of drifting copies (the heal.rs `collect_named_walk` twin is the
//! third consumer).
//!
//! Found live 2026-10-20 (P78, the reflex sweep): the suggest/fix walk
//! pruned only exact-name `target`/`vendor` + dot-dirs, so a custom
//! `CARGO_TARGET_DIR` named `target-cuda-w2` walked — the sweep scanned
//! serde/thiserror/pulp build-script OUTPUT (`target-cuda-w2/release/
//! build/*/out/*.rs`) as if it were first-party source, inflating the
//! findings roll-up with heuristic citations and a bogus auto-fix CTA.

/// Prunes: dot-directories; cargo build trees — `target` AND the
/// `CARGO_TARGET_DIR`-renamed convention `target-*` (a custom target dir
/// like `target-cuda-w2` is build output, never source); vendored
/// third-party trees; and `node_modules`.
///
/// Deliberately NAME-based, not gitignore-aware: a gitignored directory
/// whose name escapes this convention still walks (disclosed residual —
/// check-ignore awareness would be a per-dir subprocess on the hot walk).
#[must_use]
pub fn is_pruned_dir_name(name: &str) -> bool {
    name.starts_with('.')
        || name == "target"
        || name.starts_with("target-")
        || name == "vendor"
        || name == "node_modules"
}

#[cfg(test)]
mod tests {
    use super::is_pruned_dir_name;

    #[test]
    fn prunes_the_build_tree_convention_and_third_party_dirs() {
        assert!(is_pruned_dir_name("target"));
        assert!(is_pruned_dir_name("target-cuda-w2"));
        assert!(is_pruned_dir_name("target-release"));
        assert!(is_pruned_dir_name(".git"));
        assert!(is_pruned_dir_name(".hidden"));
        assert!(is_pruned_dir_name("vendor"));
        assert!(is_pruned_dir_name("node_modules"));
    }

    #[test]
    fn keeps_source_shaped_names() {
        // `targets` (plural, no dash) is a legitimate source dir name.
        assert!(!is_pruned_dir_name("targets"));
        assert!(!is_pruned_dir_name("src"));
        assert!(!is_pruned_dir_name("tests"));
        assert!(!is_pruned_dir_name("target_dir"));
        assert!(!is_pruned_dir_name("vendor_docs"));
    }
}
