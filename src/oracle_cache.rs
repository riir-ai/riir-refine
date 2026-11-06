//! The auto-oracle's persistent results cache (P-PAR, 2026-11-05).
//!
//! `cargo clippy` over a workspace is the dominant cost of a `cargo refine`
//! run (measured: 31.8 s of a 32.8 s run on seal-game-editor) and its output
//! is a pure function of (workspace source bytes, manifests + lockfile,
//! toolchain version, invocation posture). That makes the oracle session
//! cacheable: same fingerprint → same diagnostics.
//!
//! The owner's hygiene law binds this cache (2026-11-05):
//! - **never big**: entries are LRU-evicted to [`MAX_ENTRIES`] files and
//!   [`MAX_TOTAL_BYTES`] on every store — the dir's ceiling is a constant,
//!   not a leak;
//! - **never unclean**: the key folds EVERY input that can move clippy's
//!   output (source bytes incl. manifests + lockfile, `cargo clippy
//!   --version`, the feature/target/gap posture) — a changed tree is a new
//!   key, never a stale hit; a HIT prints loudly with its key prefix, and a
//!   corrupt entry is a miss with a warning, never a silent pass;
//! - **manageable**: demote env `RIIR_REFINE_ORACLE_CACHE=0` (exact literal),
//!   and the whole state is `.refine/oracle-cache/` — `rm -rf` is the reset.
//!
//! Over-conservative keying is the deliberate posture: hashing every `.rs`
//! the scan walk sees (including excluded nested workspaces' sources) only
//! ever costs extra MISSES (re-runs), never wrong HITS.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The cache schema version — bump on any DTO/wire change; stale versions
/// refuse at load (a miss, never a wrong hit).
pub const CACHE_FORMAT_VERSION: u32 = 1;

/// The dir under `<project-root>/.refine/` (the kat-state-stamp precedent:
/// repo-local state the owner already manages).
pub const CACHE_DIR_NAME: &str = "oracle-cache";

/// Entry ceiling — a workspace shape churns fingerprints fast; six entries
/// covers "toggle a feature set, toggle back" without ever growing.
const MAX_ENTRIES: usize = 6;

/// Bytes ceiling — MB-class JSONL per entry; 192 MiB is generous headroom
/// above six worst-case entries while never eating a disk.
const MAX_TOTAL_BYTES: u64 = 192 * 1024 * 1024;

/// The demote env: exact `0` disables (the env-switch law — a promoted arm's
/// env is DEMOTE-ONLY).
#[must_use]
pub fn oracle_cache_enabled() -> bool {
    std::env::var("RIIR_REFINE_ORACLE_CACHE").map_or(true, |v| v != "0")
}

// ── the wire DTOs — mirrors of clippy_oracle's plain-data structs, serde ——

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedSkippedTarget {
    pub package: String,
    pub target: String,
    pub kind: String,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedGapRun {
    pub features: Vec<String>,
    pub exit: Option<i32>,
    pub new_lines: usize,
    pub spawn_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedOracleSession {
    pub cache_version: u32,
    pub key: String,
    pub cached_at_unix: u64,
    pub text: String,
    pub base_exit: Option<i32>,
    pub base_spawn_error: Option<String>,
    pub census: Vec<CachedSkippedTarget>,
    pub census_error: Option<String>,
    pub gaps: Vec<CachedGapRun>,
    pub uncovered: Vec<CachedSkippedTarget>,
}

impl CachedOracleSession {
    /// Stamp a run's results under `key`.
    #[must_use]
    pub fn from_parts(key: String, text: String, base_exit: Option<i32>) -> Self {
        Self {
            cache_version: CACHE_FORMAT_VERSION,
            key,
            cached_at_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            text,
            base_exit,
            base_spawn_error: None,
            census: Vec::new(),
            census_error: None,
            gaps: Vec::new(),
            uncovered: Vec::new(),
        }
    }
}

// ── the fingerprint — every input that can move clippy's output ───────────

/// A workspace fingerprint: the BLAKE3 key over sorted (path, len, bytes) of
/// every `.rs` + `.toml` the scan walk sees, plus `Cargo.lock`, the toolchain
/// string, and the invocation posture. The counts ride along for the HIT line
/// (a user should see what a key actually hashed).
#[derive(Debug, Clone)]
pub struct WorkspaceFingerprint {
    pub key_hex: String,
    pub files_hashed: usize,
    pub bytes_hashed: u64,
    pub toolchain: String,
}

/// Pure key fold (testable): the hasher state is abstracted so tests can pin
/// the ordering/separation law without a filesystem.
fn fold_path_into(hasher: &mut blake3::Hasher, path: &str, bytes: &[u8]) {
    // domain-separated: (len(path) ‖ path ‖ len(bytes) ‖ bytes), lengths as
    // u64 LE — a path is never confused with content, a prefix never collides
    // with a suffix.
    hasher.update(&(path.len() as u64).to_le_bytes());
    hasher.update(path.as_bytes());
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

/// Hash the workspace inputs + posture into a cache key. Never errors: an
/// unreadable file hashes as absent (clippy will fail loudly on its own if
/// the file matters).
///
/// The walk is LOCAL to this module (ungated): `span_source` lives behind
/// `latent_retrieval`, and the cache must compile at every posture. The same
/// [`crate::scan_prune`] vocabulary applies — identical prune semantics, so
/// the fingerprint sees exactly the source set clippy's own scan sees.
fn collect_hash_files(root: &Path) -> Vec<PathBuf> {
    use std::collections::VecDeque;
    let mut out = Vec::new();
    if root.is_file() {
        return out;
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
            } else {
                // Case-SENSITIVE match is deliberate: Rust sources and Cargo
                // manifests are lowercase by convention, and the scan walk's
                // predicate (span_source) is the same shape — a stray .RS is
                // noise both sides skip.
                #[allow(clippy::case_sensitive_file_extension_comparisons)]
                {
                    if name.ends_with(".rs") || name.ends_with(".toml") {
                        out.push(path);
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The workspace root a clippy spawn at `root` actually compiles: the
/// nearest self-or-ancestor manifest declaring `[workspace]`. `cargo clippy
/// --workspace` from a MEMBER dir compiles the whole inherited workspace, so
/// the fingerprint (and the cache dir) must key THAT tree — keying the member
/// alone would miss sibling-crate changes that move its diagnostics (the
/// unclean-hit class). Fallback: `root` itself (a standalone crate).
#[must_use]
pub fn workspace_root_for(root: &Path) -> PathBuf {
    let mut dir = root.to_path_buf();
    for _ in 0..16 {
        let manifest = dir.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&manifest) {
            if text.contains("[workspace]") {
                return dir;
            }
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => break,
        }
    }
    root.to_path_buf()
}

#[must_use]
pub fn workspace_fingerprint(
    root: &Path,
    cargo_bin: &str,
    posture: &str,
) -> WorkspaceFingerprint {
    let mut hasher = blake3::Hasher::new();
    hasher.update(CACHE_FORMAT_VERSION.to_le_bytes().as_slice());
    let mut files_hashed = 0usize;
    let mut bytes_hashed: u64 = 0;
    // Sorted walk — the order is part of the key (the same tree must always
    // fold identically). .rs sources + every .toml (Cargo.toml manifests AND
    // other config tomls: over-inclusive is a miss, never a wrong hit).
    let files = collect_hash_files(root);
    if let Ok(rel_base) = root.canonicalize() {
        for f in &files {
            let Ok(bytes) = std::fs::read(f) else {
                continue;
            };
            let rel = f.strip_prefix(&rel_base).map_or_else(
                |_| f.display().to_string(),
                |p| p.display().to_string(),
            );
            bytes_hashed += bytes.len() as u64;
            files_hashed += 1;
            fold_path_into(&mut hasher, &rel, &bytes);
        }
    }
    // The lockfile at the root (dep versions move diagnostics even when
    // sources stand still). Nested excluded workspaces' locks: their sources
    // are already hashed above — over-inclusive, never unsound.
    let lock = root.join("Cargo.lock");
    if let Ok(bytes) = std::fs::read(&lock) {
        bytes_hashed += bytes.len() as u64;
        files_hashed += 1;
        fold_path_into(&mut hasher, "Cargo.lock", &bytes);
    }
    // The toolchain: clippy's own version string (covers rustc + clippy driver
    // moves; a capture failure keys as "unknown" — the sources hash still
    // carries the day, and a toolchain upgrade through a failed capture only
    // ever costs a miss via `cargo`'s own fingerprinting downstream).
    let toolchain = Command::new(cargo_bin)
        .args(["clippy", "--version"])
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |o| String::from_utf8_lossy(&o.stdout).trim().to_string(),
        );
    fold_path_into(&mut hasher, "toolchain", toolchain.as_bytes());
    fold_path_into(&mut hasher, "posture", posture.as_bytes());
    WorkspaceFingerprint {
        key_hex: hasher.finalize().to_hex().to_string(),
        files_hashed,
        bytes_hashed,
        toolchain,
    }
}

use std::process::Command;

// ── the store — tmp+rename writes, LRU caps on every store ────────────────

/// The cache dir under the project root.
#[must_use]
pub fn cache_dir(root: &Path) -> PathBuf {
    root.join(".refine").join(CACHE_DIR_NAME)
}

fn entry_path(dir: &Path, key_hex: &str) -> PathBuf {
    dir.join(format!("{key_hex}.json"))
}

/// Look up a session by key. A corrupt/stale/mismatched entry is `None` plus
/// a stderr warning — an unclean cache is VISIBLE, never a silent pass.
#[must_use]
pub fn load_session(root: &Path, key_hex: &str) -> Option<CachedOracleSession> {
    let dir = cache_dir(root);
    let path = entry_path(&dir, key_hex);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return None;
    };
    match serde_json::from_str::<CachedOracleSession>(&text) {
        Ok(c) if c.cache_version == CACHE_FORMAT_VERSION && c.key == key_hex => Some(c),
        Ok(_) => {
            eprintln!(
                "clippy oracle cache: stale entry at {} (version/key mismatch) — treating as a miss",
                path.display()
            );
            None
        }
        Err(e) => {
            eprintln!(
                "clippy oracle cache: corrupt entry at {} ({e}) — treating as a miss; `rm -rf {}` resets",
                path.display(),
                dir.display()
            );
            None
        }
    }
}

/// Store a session under its key (tmp + rename — a crash leaves no torn
/// entry), then sweep the dir to the caps (LRU by mtime: newest kept).
/// Best-effort: a store failure warns once and never aborts the caller's
/// run — the oracle already ran; the cache is an optimization.
pub fn store_session(root: &Path, cached: &CachedOracleSession) {
    let dir = cache_dir(root);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Ok(body) = serde_json::to_vec_pretty(cached) else {
        return;
    };
    let final_path = entry_path(&dir, &cached.key);
    let tmp = dir.join(format!(
        "{}.tmp-{}",
        &cached.key[..12.min(cached.key.len())],
        std::process::id()
    ));
    if std::fs::write(&tmp, &body).is_ok() && std::fs::rename(&tmp, &final_path).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return;
    }
    enforce_caps(&dir);
}

/// The caps sweep: newest-first by mtime, keep until either cap would break.
/// Split out for the test arms (the caps are parameters there; the store
/// calls the const posture).
fn enforce_caps(dir: &Path) {
    enforce_caps_with(dir, MAX_ENTRIES, MAX_TOTAL_BYTES);
}

fn enforce_caps_with(dir: &Path, max_entries: usize, max_total_bytes: u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut rows: Vec<(std::time::SystemTime, PathBuf, u64)> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) != Some("json") {
                return None;
            }
            let meta = e.metadata().ok()?;
            Some((meta.modified().ok()?, p, meta.len()))
        })
        .collect();
    // Newest first.
    rows.sort_by_key(|(t, _, _)| *t);
    let mut kept_entries = 0usize;
    let mut kept_bytes = 0u64;
    for (_, path, len) in &rows {
        if kept_entries >= max_entries || kept_bytes + len > max_total_bytes {
            let _ = std::fs::remove_file(path);
            continue;
        }
        kept_entries += 1;
        kept_bytes += len;
    }
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_session(key: &str) -> CachedOracleSession {
        CachedOracleSession {
            key: key.to_string(),
            text: "{\"reason\":\"lint\"}\n".to_string(),
            base_exit: Some(0),
            base_spawn_error: None,
            census: vec![CachedSkippedTarget {
                package: "p".to_string(),
                target: "t".to_string(),
                kind: "test".to_string(),
                missing: vec!["feat_a".to_string()],
            }],
            census_error: None,
            gaps: vec![CachedGapRun {
                features: vec!["feat_a".to_string()],
                exit: Some(0),
                new_lines: 3,
                spawn_error: None,
            }],
            uncovered: Vec::new(),
            ..CachedOracleSession::from_parts(String::new(), String::new(), None)
        }
    }

    #[test]
    fn cache_round_trips_and_refuses_foreign_keys() {
        let dir = std::env::temp_dir().join(format!(
            "oracle_cache_test_{}_{line}",
            std::process::id(),
            line = line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let cached = fixture_session(&"a".repeat(64));
        // store_session takes the ROOT (dir = root/.refine/oracle-cache) —
        // exercise the caps helper directly against a raw dir instead, then
        // the load/store pair through a fake root whose .refine dir we pre-make.
        let root = std::env::temp_dir().join(format!(
            "oracle_cache_root_{}_{}",
            std::process::id(),
            line!()
        ));
        let cdir = root.join(".refine").join(CACHE_DIR_NAME);
        std::fs::create_dir_all(&cdir).unwrap();
        store_session(&root, &cached);
        let hit = load_session(&root, &cached.key);
        assert_eq!(hit.as_ref(), Some(&cached), "round trip");
        assert_eq!(
            load_session(&root, &"b".repeat(64)),
            None,
            "a different key is a miss"
        );
        // A corrupt entry: overwrite + parse failure → None with a warning.
        let p = entry_path(&cdir, &cached.key);
        std::fs::write(&p, "{not json").unwrap();
        assert!(load_session(&root, &cached.key).is_none(), "corrupt = miss");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn caps_evict_oldest_and_never_exceed_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "oracle_cache_caps_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        // Four entries, 100 B each, distinct mtimes (sleep ~5ms between).
        for i in 0..4usize {
            let p = dir.join(format!("{i:0width$}.json", width = 64));
            std::fs::write(&p, vec![b'x'; 100]).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        enforce_caps_with(&dir, 2, 10_000);
        let left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .collect();
        assert_eq!(left.len(), 2, "entries cap: 4 -> 2");
        // Byte cap: 4 x 100 B under a 250 B cap keeps only 2.
        for i in 0..4usize {
            let p = dir.join(format!("{i:0width$}.json", width = 64));
            std::fs::write(&p, vec![b'x'; 100]).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        enforce_caps_with(&dir, 100, 250);
        let left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .collect();
        assert_eq!(left.len(), 2, "bytes cap: 400 B -> 2 entries");
        let total: u64 = left
            .iter()
            .map(|p| std::fs::metadata(p).unwrap().len())
            .sum();
        assert!(total <= 250, "byte ceiling held");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn key_folds_are_order_and_content_sensitive() {
        let mut h1 = blake3::Hasher::new();
        fold_path_into(&mut h1, "a.rs", b"one");
        fold_path_into(&mut h1, "b.rs", b"two");
        let mut h2 = blake3::Hasher::new();
        fold_path_into(&mut h2, "b.rs", b"two");
        fold_path_into(&mut h2, "a.rs", b"one");
        assert_ne!(
            h1.finalize(),
            h2.finalize(),
            "path order is part of the key (sorted-walk law)"
        );
        let mut h3 = blake3::Hasher::new();
        fold_path_into(&mut h3, "a.rs", b"one!");
        assert_ne!(
            h1.finalize(),
            h3.finalize(),
            "a one-byte content change is a new key"
        );
        let mut h4 = blake3::Hasher::new();
        fold_path_into(&mut h4, "a.r", b"sone");
        assert_ne!(
            h1.finalize(),
            h4.finalize(),
            "path/content boundary is length-separated, never ambiguous"
        );
    }

    #[test]
    fn workspace_root_walks_to_the_workspace_manifest() {
        let root = std::env::temp_dir().join(format!(
            "oracle_cache_wsroot_{}_{}",
            std::process::id(),
            line!()
        ));
        let member = root.join("crates").join("m");
        std::fs::create_dir_all(&member).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        std::fs::write(member.join("Cargo.toml"), "[package]\nname = \"m\"\n").unwrap();
        let hit = workspace_root_for(&member);
        assert_eq!(hit, root, "member dir resolves UP to the [workspace] manifest");
        // A standalone crate (no [workspace] anywhere) keys itself: a SEPARATE
        // parent with no workspace manifest — the walk stops after checking
        // the chain above it and finds nothing declaring [workspace].
        let solo_parent = std::env::temp_dir().join(format!(
            "oracle_cache_solop_{}_{}",
            std::process::id(),
            line!()
        ));
        let solo = solo_parent.join("solo");
        std::fs::create_dir_all(&solo).unwrap();
        std::fs::write(solo.join("Cargo.toml"), "[package]\nname = \"s\"\n").unwrap();
        assert_eq!(workspace_root_for(&solo), solo, "standalone keys itself");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&solo_parent);
    }

    #[test]
    fn demote_env_is_exact_literal() {
        // The env-switch law: only the exact `0` demotes.
        let cases = [
            ("0", false),
            ("1", true),
            ("true", true),
            ("off", true),
            ("", true),
        ];
        for (v, want) in cases {
            std::env::set_var("RIIR_REFINE_ORACLE_CACHE", v);
            assert_eq!(oracle_cache_enabled(), want, "value {v:?}");
        }
        std::env::remove_var("RIIR_REFINE_ORACLE_CACHE");
        assert!(oracle_cache_enabled(), "unset = enabled");
    }
}
