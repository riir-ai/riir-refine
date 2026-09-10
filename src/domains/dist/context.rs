//! Caller evidence for the dist domain — the docker `ScanContext` pattern,
//! extended for blind spots B1 + B2 (Bench 082 §3, landed 2026-09-10).
//!
//! The evidence shape:
//!
//! - `under_workspace_root` — B0 (Phase 1): a nearest-ancestor walk found a
//!   manifest declaring `[workspace]` above this one; gates
//!   `member-profile-ignored`.
//! - `autodiscovered_bin` — B1: cargo discovers binary targets from
//!   `src/main.rs` and directly-nested `src/bin/*.rs` without any `[[bin]]`
//!   section, so "does this manifest ship a bin" cannot be answered from
//!   section membership alone. This is TREE evidence (the files exist); the
//!   `[package]` precondition is applied by the rule, not here — see
//!   `rules::ships_bin`.
//! - `root_profile_knobs` — B2: a workspace MEMBER cannot carry effective
//!   `[profile.*]` (root builds ignore them), so the ship-size rules must
//!   consult the ROOT manifest's knobs before advising. Resolved in the same
//!   ancestor walk that produces `under_workspace_root`, from the root
//!   manifest text already in hand — one read, no extra I/O.
//!
//! [`DistContext::derive`] is the per-scan-run cache point: one bounded walk
//! per manifest, the root's knobs frozen into a `Copy` struct, so no rule
//! ever re-reads a file (absent evidence never satisfies a knob).

use std::ffi::OsStr;
use std::path::Path;

use super::rules::{self, ManifestScan};

/// Bound on the ancestor walk. No real workspace sits deeper than this below
/// its root; the bound keeps a pathological (or cyclic, via symlinks) tree
/// from turning a manifest scan into a full-disk walk.
const MAX_ANCESTOR_WALK: usize = 32;

/// The ship-size knobs the dist rules reason about, as presence flags. A key
/// counts when ANY `[profile.*]` section of the manifest sets it — key
/// PRESENCE is the scanner's only value-shaped fact (the rules never read
/// values, so `strip = "symbols"` and `strip = true` are equivalent here).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProfileKnobs {
    /// a `strip` key in some `[profile.*]`
    pub strip: bool,
    /// an `lto` key in some `[profile.*]`
    pub lto: bool,
    /// a `panic` key in some `[profile.*]`
    pub panic: bool,
}

impl ProfileKnobs {
    /// The knobs THIS manifest's own `[profile.*]` sections set.
    #[must_use]
    pub(crate) fn from_scan(scan: &ManifestScan) -> Self {
        Self {
            strip: rules::any_profile_key(scan, "strip"),
            lto: rules::any_profile_key(scan, "lto"),
            panic: rules::any_profile_key(scan, "panic"),
        }
    }
}

/// Caller evidence from outside the manifest (the docker `ScanContext`
/// pattern). Absent evidence keeps its rule silent — never guessed.
#[derive(Debug, Clone, Copy, Default)]
pub struct DistContext {
    /// `true` when a PARENT directory's `Cargo.toml` declares
    /// `[workspace]` — enables `member-profile-ignored`. Derived by walking
    /// up from the manifest's own directory; an `exclude`d member gets the
    /// hedged advice text, accepted at Phase-1 precision.
    pub under_workspace_root: bool,
    /// B1 — cargo binary AUTODISCOVERY evidence: the manifest's directory
    /// carries `src/main.rs`, or a `*.rs` file directly inside `src/bin/`.
    /// Tree evidence only; the `[package]` precondition is the rule's (see
    /// `rules::ships_bin`).
    pub autodiscovered_bin: bool,
    /// B2 — the ship-size knobs carried by the nearest ancestor manifest
    /// declaring `[workspace]`. `None` when there is no such ancestor (or it
    /// was unreadable) — absent evidence never satisfies a knob, so a
    /// standalone manifest behaves exactly as before B2.
    pub root_profile_knobs: Option<ProfileKnobs>,
}

impl DistContext {
    /// Derive the full evidence for one manifest path: the B1 tree check in
    /// the manifest's own directory, then one bounded parent walk for the
    /// nearest ancestor manifest declaring `[workspace]` (B0 + B2 — the B2
    /// knobs come from that same root read). Resolved once per scanned
    /// manifest; every rule consumes the cached result.
    #[must_use]
    pub fn derive(manifest_path: &Path) -> Self {
        let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
        let autodiscovered_bin = autodiscovered_bin_present(manifest_dir);
        // A manifest is never its own ancestor: the walk starts at the
        // PARENT (unchanged Phase-1 semantics for `under_workspace_root`).
        let mut dir = manifest_dir.parent();
        let mut depth = 0usize;
        while let Some(d) = dir {
            if depth >= MAX_ANCESTOR_WALK {
                break;
            }
            depth += 1;
            let candidate = d.join("Cargo.toml");
            if candidate.is_file() {
                if let Ok(src) = std::fs::read_to_string(&candidate) {
                    // Section-presence only — the same cheap scan the lane
                    // uses on the manifest itself.
                    let scan = rules::scan(&src);
                    if scan.has_workspace {
                        return Self {
                            under_workspace_root: true,
                            autodiscovered_bin,
                            root_profile_knobs: Some(ProfileKnobs::from_scan(&scan)),
                        };
                    }
                }
            }
            dir = d.parent();
        }
        Self {
            under_workspace_root: false,
            autodiscovered_bin,
            root_profile_knobs: None,
        }
    }
}

/// B1 evidence: does the manifest's directory carry cargo's autodiscovered
/// binary sources — `src/main.rs`, or a `*.rs` file directly inside
/// `src/bin/`? (Cargo's deeper shapes — `src/bin/<dir>/main.rs` targets and
/// the `autobins = false` opt-out — are deliberately NOT derived here; see
/// the Bench 082 addendum's caveat table.)
fn autodiscovered_bin_present(manifest_dir: &Path) -> bool {
    if manifest_dir.join("src").join("main.rs").is_file() {
        return true;
    }
    let bin_dir = manifest_dir.join("src").join("bin");
    let Ok(entries) = std::fs::read_dir(&bin_dir) else {
        return false;
    };
    entries.flatten().any(|e| {
        let p = e.path();
        p.is_file() && p.extension() == Some(OsStr::new("rs"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_context_is_all_absent() {
        let c = DistContext::default();
        assert!(!c.under_workspace_root);
        assert!(!c.autodiscovered_bin);
        assert!(c.root_profile_knobs.is_none());
    }

    #[test]
    fn knobs_read_key_presence_across_profiles() {
        let scan =
            rules::scan("[profile.release]\nstrip = true\n\n[profile.dist]\nlto = \"fat\"\n");
        let k = ProfileKnobs::from_scan(&scan);
        assert!(k.strip && k.lto && !k.panic);
        // A commented key is not a set key (the scanner clips at `#`).
        let scan = rules::scan("[profile.release]\n# panic = \"abort\"\n");
        let k = ProfileKnobs::from_scan(&scan);
        assert!(!k.strip && !k.lto && !k.panic);
    }
}
