//! The free-for-open-source-repositories posture (issue 190 T11 — "ship
//! public-repo-free as product policy"): a run over an open-source repository
//! costs nothing. The billing planes (the meter, the charge ledger, the burn
//! sync, the Issue-075 depleted-account soft gate) consult the verdict this
//! module computes once per run and stand down when it says FREE — "free for
//! the commons, cost-recovery for private use".
//!
//! The heuristic is deliberately fail-open: when the evidence is ambiguous,
//! the verdict is FREE. A maintainer who might be free must never be charged
//! on a maybe — a false FREE costs a handful of tokens; a false BILLABLE
//! charges a commons user for what the policy promises free. Two signals,
//! either one sufficient:
//!
//! * a licence file at the scan root (`LICENSE*`, `LICENCE*`, `COPYING*`,
//!   `NOTICE`) — the file's PRESENCE is the claim; its content is not parsed
//!   (a stub or a custom text still declares "this repo is published under
//!   something", and the cost of believing it is trivial next to the cost of
//!   charging a commons user);
//! * a public-forge origin in the scan root's `.git/config` (`github.com`,
//!   `gitlab.com`, `bitbucket.org`, `codeberg.org`, `sr.ht`) — a repo pushed
//!   to a public forge is published, whatever its licence file says.
//!
//! A private repo (no licence file, no forge origin — a plain local dir)
//! stays billable, unchanged. Detection runs ONCE per process against the
//! scan root ([`init`]); the verdict is consulted through [`posture`].

/// The licence files the heuristic reads, most common first. Presence alone
/// fires the licence signal — see the module doc for the fail-open reasoning.
const LICENCE_FILES: [&str; 8] = [
    "LICENSE",
    "LICENSE.md",
    "LICENSE.txt",
    "LICENCE",
    "LICENCE.md",
    "COPYING",
    "COPYING.md",
    "NOTICE",
];

/// Public forges whose origin URLs count as published. A private repo on the
/// same forges is indistinguishable from here — that is exactly why the
/// posture is fail-open (the maintainer-facing promise is "we never charge a
/// maybe").
const PUBLIC_FORGES: [&str; 5] = [
    "github.com",
    "gitlab.com",
    "bitbucket.org",
    "codeberg.org",
    "sr.ht",
];

/// The run's billing posture over its scan root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoFree {
    /// An open-source signal fired: the billing planes stand down, the run
    /// discloses why once.
    Free { licence: Option<&'static str> },
    /// No signal: billing runs exactly as before, byte-identical.
    Billable,
}

static POSTURE: std::sync::OnceLock<RepoFree> = std::sync::OnceLock::new();

/// Compute the run's posture over `scan_root` and install it. Later calls are
/// idempotent (the first verdict wins — the scan root is fixed at startup).
pub fn init(scan_root: &std::path::Path) {
    let _ = POSTURE.set(verdict(scan_root));
}

/// The installed posture; [`RepoFree::Billable`] before [`init`] runs (the
/// billing planes' default is today's behavior, never a surprise free run).
#[must_use]
pub fn posture() -> RepoFree {
    *POSTURE.get_or_init(|| RepoFree::Billable)
}

/// The one-line disclosure a FREE run prints at startup. `None` when the
/// posture is [`RepoFree::Billable`] (no line — byte-identical output).
#[must_use]
pub fn disclosure() -> Option<String> {
    match posture() {
        RepoFree::Free { licence } => {
            let evidence = licence.unwrap_or("public-forge origin");
            Some(format!(
                "repo: open-source ({evidence}) — billing skipped (free for open-source repositories)"
            ))
        }
        RepoFree::Billable => None,
    }
}

/// The heuristic over a scan root — the tests' entry point (pure over the
/// filesystem, no process state).
#[must_use]
pub fn verdict(scan_root: &std::path::Path) -> RepoFree {
    if let Some(licence) = licence_file(scan_root) {
        return RepoFree::Free {
            licence: Some(licence),
        };
    }
    if forge_origin(scan_root) {
        return RepoFree::Free { licence: None };
    }
    RepoFree::Billable
}

/// The first licence file present at `scan_root` — its leaf name (the
/// disclosure's evidence). Presence is the signal; content is not parsed.
#[must_use]
fn licence_file(scan_root: &std::path::Path) -> Option<&'static str> {
    LICENCE_FILES
        .iter()
        .find(|name| scan_root.join(name).is_file())
        .copied()
}

/// Whether the scan root's git origin points at a public forge. Reads
/// `.git/config` directly (no subprocess); a worktree's `.git` file resolves
/// to its real config through the `gitdir:` line, which the plain read
/// misses — a worktree scan then falls back to the licence signal alone,
/// fail-open in the same direction.
#[must_use]
fn forge_origin(scan_root: &std::path::Path) -> bool {
    let config = scan_root.join(".git").join("config");
    let Ok(text) = std::fs::read_to_string(&config) else {
        return false;
    };
    let lower = text.to_lowercase();
    PUBLIC_FORGES.iter().any(|forge| lower.contains(forge))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("repo_free_{}_{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn a_private_local_dir_stays_billable() {
        let dir = scratch("private");
        assert_eq!(verdict(&dir), RepoFree::Billable);
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn an_mit_licence_fires_free_with_the_file_as_evidence() {
        let dir = scratch("mit");
        std::fs::write(dir.join("LICENSE"), "MIT License\n\nCopyright (c) 2026\n")
            .expect("licence");
        assert_eq!(
            verdict(&dir),
            RepoFree::Free {
                licence: Some("LICENSE")
            }
        );
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn an_empty_licence_stub_still_fires_fail_open() {
        let dir = scratch("stub");
        std::fs::write(dir.join("LICENSE"), "").expect("stub");
        assert_eq!(
            verdict(&dir),
            RepoFree::Free {
                licence: Some("LICENSE")
            }
        );
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn a_custom_licence_file_fires_free_by_presence_alone() {
        let dir = scratch("custom");
        std::fs::write(dir.join("LICENSE"), "The Unnamed Custom Licence\n").expect("licence");
        assert_eq!(
            verdict(&dir),
            RepoFree::Free {
                licence: Some("LICENSE")
            }
        );
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn a_licence_less_repo_with_a_forge_origin_fires_free() {
        let dir = scratch("forge");
        std::fs::create_dir_all(dir.join(".git")).expect("git dir");
        std::fs::write(
            dir.join(".git").join("config"),
            "[remote \"origin\"]\n\turl = git@github.com:me/mine.git\n",
        )
        .expect("config");
        assert_eq!(verdict(&dir), RepoFree::Free { licence: None });
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn a_local_git_dir_without_a_licence_stays_billable() {
        let dir = scratch("localgit");
        std::fs::create_dir_all(dir.join(".git")).expect("git dir");
        std::fs::write(
            dir.join(".git").join("config"),
            "[remote \"origin\"]\n\turl = /srv/git/private.git\n",
        )
        .expect("config");
        assert_eq!(verdict(&dir), RepoFree::Billable);
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn the_disclosure_names_the_evidence_once_free() {
        let dir = scratch("disclose");
        std::fs::write(dir.join("COPYING"), "GNU General Public License v2\n").expect("licence");
        assert!(matches!(verdict(&dir), RepoFree::Free { .. }));
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn the_disclosure_is_none_while_billable() {
        let dir = scratch("nodisclose");
        assert!(verdict(&dir) == RepoFree::Billable);
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }
}
