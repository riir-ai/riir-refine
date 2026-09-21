//! Cross-repo consumer-surface hint (the FP-class #3 intake row — the
//! 943 scoped-scan family's biggest measured share: 4-of-7 FPs were
//! cross-repo production reads, fields DEFINED in the scanned repo and
//! READ in a downstream sibling that imports its crates).
//!
//! `--config-audit` / `--orphan-report` adjudicate field reads against
//! the SCANNED repo only. A sibling repo that path-deps into the scanned
//! one can hold production reads of its fields — invisible to the report
//! no matter how wide the intra-repo scope is (the 943 pass was a
//! FULL-scope scan and still carried 4 cross-repo FPs, e.g.
//! `SftWarmupConfig` dying at riir-train-gpu `pipeline.rs` while living
//! in riir-ai). This hint makes that blind half CONCRETE: it names the
//! downstream repos that exist ON DISK and path-dep into the scanned
//! repo, so a TP claim can be paired with a downstream reference grep
//! before anyone deletes a "dead" knob.
//!
//! Discovery uses the MEASURED dep graph — `path = "..."` values in the
//! sibling's manifests, canonicalized and prefix-matched against the
//! scanned repo root — NOT the BOUNDARY.md tables: prose drifts (the
//! boundary-guard skill exists for exactly that), manifests cannot.
//! The repo set is whatever is present next to the scanned repo
//! (partial-clone boxes name their subset — "on disk" is in the message
//! for that reason); registry deps on internal crates do not exist
//! (`publish = false` everywhere), git deps are out of scope.
//!
//! Manifest discovery reads the sibling's ROOT Cargo.toml plus its
//! workspace-member manifests (declared `members` globs + conventional
//! `crates/*`/`examples/*`/`benches/*` probes) — NEVER a recursive tree
//! walk: the live probe on a 24-sibling Windows box measured a full-tree
//! `find` at over two minutes. dev-dependencies count (the FP class
//! includes test readers). The count reported per repo is the number of
//! DISTINCT dep targets (crate directories) inside the scanned repo.
//!
//! Like the intra-repo `scope_hint`, this NEVER widens the scan — it
//! only makes the blind half visible at report time.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Sibling consumers listed before the "+N more" tail.
const MAX_LISTED: usize = 6;

/// The one-line cross-repo consumer hint, or `""` when no sibling repo
/// on disk path-deps into the scanned repo (a leaf scan stays silent).
///
/// The scanned repo is inferred from the FIRST target: the TOPMOST
/// Cargo.toml-holding ancestor (the workspace root, not the nearest
/// crate — a scan of `crates/x` must name consumers of ANY crate in the
/// repo, since reads of `crates/x`'s types can sit in a sibling that
/// deps a different crate and re-exports).
#[must_use]
pub fn downstream_consumers_hint(targets: &[PathBuf]) -> String {
    let Some(root) = scanned_repo_root(targets) else {
        return String::new();
    };
    let Some(parent) = root.parent() else {
        return String::new();
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return String::new();
    };
    let mut consumers: Vec<(String, usize)> = Vec::new();
    for entry in entries.flatten() {
        let sibling = entry.path();
        if sibling == root {
            continue;
        }
        // Repos only (a `.git` FILE counts — a linked worktree is still a
        // repo whose manifests are real read surfaces).
        if !sibling.join(".git").exists() {
            continue;
        }
        if !sibling.join("Cargo.toml").is_file() {
            continue;
        }
        let crates = count_dep_targets_into(&sibling, &root);
        if crates == 0 {
            continue;
        }
        let Some(name) = sibling.file_name() else {
            continue;
        };
        consumers.push((name.to_string_lossy().into_owned(), crates));
    }
    if consumers.is_empty() {
        return String::new();
    }
    consumers.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let total = consumers.len();
    let mut parts: Vec<String> = consumers
        .iter()
        .take(MAX_LISTED)
        .map(|(name, crates)| format!("{name} ({crates} crate{s})", s = if *crates == 1 { "" } else { "s" }))
        .collect();
    if total > MAX_LISTED {
        parts.push(format!("+{} more", total - MAX_LISTED));
    }
    format!(
        "cross-repo hint: {total} sibling repo(s) on disk path-dep into this scanned repo — {} — field reads there are invisible to this report; pair any TP claim with a downstream reference grep before acting",
        parts.join(", ")
    )
}

/// The TOPMOST Cargo.toml-holding ancestor of the first target
/// (canonicalized). `None` when no ancestor is a cargo root — the hint
/// never guesses.
fn scanned_repo_root(targets: &[PathBuf]) -> Option<PathBuf> {
    let first = targets.first()?;
    let abs = first.canonicalize().ok()?;
    let start = if abs.is_file() {
        abs.parent()?.to_path_buf()
    } else {
        abs
    };
    let mut root = None;
    let mut dir = Some(start.as_path());
    while let Some(d) = dir {
        if d.join("Cargo.toml").is_file() {
            root = Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    root
}

/// Distinct canonical dep-target directories under `scanned_root`
/// referenced by any manifest of `sibling` (root + members).
fn count_dep_targets_into(sibling: &Path, scanned_root: &Path) -> usize {
    let root_manifest = sibling.join("Cargo.toml");
    let mut manifests = vec![root_manifest.clone()];
    manifests.extend(member_manifests(&root_manifest));
    let mut targets = BTreeSet::new();
    for manifest in &manifests {
        let Ok(text) = std::fs::read_to_string(manifest) else {
            continue;
        };
        let Some(base) = manifest.parent() else {
            continue;
        };
        for rel in path_dep_values(&text) {
            if let Ok(canon) = base.join(rel).canonicalize() {
                if canon.starts_with(scanned_root) {
                    targets.insert(canon);
                }
            }
        }
    }
    targets.len()
}

/// The sibling's member manifests: declared workspace `members` (globs
/// expanded one level) plus conventional member roots probed
/// unconditionally (covers undeclared layouts). Sorted + deduped.
fn member_manifests(root_manifest: &Path) -> Vec<PathBuf> {
    let Some(sibling) = root_manifest.parent() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for dir in ["crates", "examples", "benches"] {
        push_subdir_manifests(&mut out, &sibling.join(dir));
    }
    if let Ok(text) = std::fs::read_to_string(root_manifest) {
        for member in workspace_members(&text) {
            if let Some((prefix, _)) = member.split_once('*') {
                push_subdir_manifests(&mut out, &sibling.join(prefix.trim_end_matches(['/', '\\'])));
            } else {
                let m = sibling.join(member).join("Cargo.toml");
                if m.is_file() {
                    out.push(m);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out.shrink_to_fit();
    out
}

/// Every immediate subdirectory of `dir` holding a Cargo.toml.
fn push_subdir_manifests(out: &mut Vec<PathBuf>, dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let m = entry.path().join("Cargo.toml");
        if m.is_file() {
            out.push(m);
        }
    }
}

/// Quoted strings of the `[workspace]` section's `members` key (handles
/// multi-line arrays). Textual — no toml dep (the dist-scanner
/// precedent); a quoted member containing `]` would truncate, which no
/// workspace layout produces.
fn workspace_members(text: &str) -> Vec<&str> {
    let mut members = Vec::new();
    let mut in_workspace = false;
    let mut collecting = false;
    for line in text.lines() {
        let t = line.trim();
        if collecting {
            members.extend(quoted_values(t));
            if t.contains(']') {
                collecting = false;
            }
            continue;
        }
        if t.starts_with('[') {
            in_workspace = t == "[workspace]";
            continue;
        }
        if in_workspace {
            if let Some((key, rest)) = t.split_once('=') {
                if key.trim() == "members" {
                    members.extend(quoted_values(rest));
                    if !rest.contains(']') {
                        collecting = true;
                    }
                }
            }
        }
    }
    members
}

/// Values of every `path = "..."` key on the line — inline tables
/// (`x = { path = "...", version = "1" }`) included. Key boundaries are
/// identifier-guarded (`xpath = "..."` is NOT a path dep).
fn path_dep_values(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for line in text.lines() {
        let bytes = line.as_bytes();
        let mut i = 0usize;
        while let Some(pos) = find_ident(bytes, i, b"path") {
            let mut j = pos + 4;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'=' {
                j += 1;
                while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'"' {
                    let start = j + 1;
                    if let Some(end) = line[start..].find('"') {
                        out.push(&line[start..start + end]);
                        i = start + end + 1;
                        continue;
                    }
                }
            }
            i = pos + 4;
        }
    }
    out.shrink_to_fit();
    out
}

/// Next occurrence of `key` in `bytes[from..]` whose byte neighbors are
/// not identifier characters (word-boundary match).
fn find_ident(bytes: &[u8], from: usize, key: &[u8]) -> Option<usize> {
    let ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
    let mut i = from;
    while i + key.len() <= bytes.len() {
        if &bytes[i..i + key.len()] == key
            && (i == 0 || !ident(bytes[i - 1]))
            && (i + key.len() >= bytes.len() || !ident(bytes[i + key.len()]))
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Every `"..."` substring of `s` (byte-safe: quotes are ASCII).
fn quoted_values(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < s.len() {
        if s.as_bytes()[i] == b'"' {
            if let Some(end) = s[i + 1..].find('"') {
                out.push(&s[i + 1..i + 1 + end]);
                i = i + 1 + end + 1;
                continue;
            }
        }
        i += 1;
    }
    out.shrink_to_fit();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const PKG: &str = "[package]\nname = \"x\"\nversion = \"0.1.0\"\n";

    fn fake_parent(tag: &str) -> PathBuf {
        let parent = std::env::temp_dir()
            .join(format!("downstream_hint_{tag}"))
            .join(format!("p_{}", std::process::id()));
        let _ = fs::remove_dir_all(&parent);
        fs::create_dir_all(&parent).unwrap();
        parent
    }

    fn fake_repo(parent: &Path, name: &str, root_manifest: &str) -> PathBuf {
        let root = parent.join(name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("Cargo.toml"), root_manifest).unwrap();
        root
    }

    fn fake_member(repo: &Path, member: &str, manifest: &str) -> PathBuf {
        let dir = repo.join(member);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Cargo.toml"), manifest).unwrap();
        dir
    }

    #[test]
    fn consumer_repo_named_with_unique_crate_count() {
        let parent = fake_parent("consumer");
        let a = fake_repo(&parent, "repoA", PKG);
        // repoB only needs to EXIST as the consumer (side effect); the
        // scan root is repoA's src.
        fake_repo(
            &parent,
            "repoB",
            &format!("{PKG}\n[dependencies]\nx = {{ path = \"../repoA\" }}\n"),
        );
        let hint = downstream_consumers_hint(&[a.join("src")]);
        assert!(hint.contains("repoB (1 crate)"), "names the consumer: {hint}");
        assert!(hint.contains("1 sibling repo"), "counts repos: {hint}");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn leaf_scan_is_silent() {
        let parent = fake_parent("leaf");
        let a = fake_repo(&parent, "repoA", PKG);
        // A non-repo sibling (no .git) that deps repoA must not be named —
        // not a read surface anyone can grep.
        let plain = parent.join("notarepo");
        fs::create_dir_all(&plain).unwrap();
        fs::write(
            plain.join("Cargo.toml"),
            format!("{PKG}\n[dependencies]\nx = {{ path = \"../repoA\" }}\n"),
        )
        .unwrap();
        assert_eq!(downstream_consumers_hint(&[a.join("src")]), "");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn dep_into_a_third_repo_is_not_counted() {
        let parent = fake_parent("third");
        let a = fake_repo(&parent, "repoA", PKG);
        fake_repo(&parent, "repoC", PKG);
        fake_repo(
            &parent,
            "repoB",
            &format!("{PKG}\n[dependencies]\nc = {{ path = \"../repoC\" }}\n"),
        );
        assert_eq!(downstream_consumers_hint(&[a.join("src")]), "");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn internal_path_dep_is_not_counted() {
        let parent = fake_parent("internal");
        let a = fake_repo(&parent, "repoA", PKG);
        fake_repo(
            &parent,
            "repoB",
            &format!("{PKG}\n[dependencies]\nloc = {{ path = \"src\" }}\n"),
        );
        assert_eq!(downstream_consumers_hint(&[a.join("src")]), "");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn declared_member_manifest_is_discovered() {
        // Multi-line members array + glob expansion — the scanner shape
        // every real workspace manifest uses.
        let parent = fake_parent("member");
        let a = fake_repo(&parent, "repoA", PKG);
        let b = fake_repo(&parent, "repoB", "[workspace]\nmembers = [\n  \"crates/*\",\n]\n");
        fake_member(
            &b,
            "crates/b",
            &format!("{PKG}\n[dependencies]\nx = {{ path = \"../../../repoA\" }}\n"),
        );
        let hint = downstream_consumers_hint(&[a.join("src")]);
        assert!(hint.contains("repoB"), "member manifest found: {hint}");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn undeclared_conventional_member_is_discovered() {
        // No [workspace] declaration at all — the conventional crates/*
        // probe still finds the member manifest.
        let parent = fake_parent("conventional");
        let a = fake_repo(&parent, "repoA", PKG);
        let b = fake_repo(&parent, "repoB", PKG);
        fake_member(
            &b,
            "crates/b",
            &format!("{PKG}\n[dependencies]\nx = {{ path = \"../../../repoA\" }}\n"),
        );
        let hint = downstream_consumers_hint(&[a.join("src")]);
        assert!(hint.contains("repoB"), "conventional probe: {hint}");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn dev_dependencies_count() {
        // The FP class includes TEST readers — a dev-dep edge is still a
        // read surface.
        let parent = fake_parent("devdep");
        let a = fake_repo(&parent, "repoA", PKG);
        fake_repo(
            &parent,
            "repoB",
            &format!("{PKG}\n[dev-dependencies]\nx = {{ path = \"../repoA\" }}\n"),
        );
        let hint = downstream_consumers_hint(&[a.join("src")]);
        assert!(hint.contains("repoB"), "dev-dep counts: {hint}");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn duplicate_targets_dedupe_to_one_crate() {
        let parent = fake_parent("dedupe");
        let a = fake_repo(&parent, "repoA", PKG);
        let b = fake_repo(
            &parent,
            "repoB",
            &format!("{PKG}\n[dependencies]\nx = {{ path = \"../repoA\" }}\n"),
        );
        fake_member(
            &b,
            "crates/b",
            &format!("{PKG}\n[dependencies]\nx = {{ path = \"../../../repoA\" }}\n"),
        );
        let hint = downstream_consumers_hint(&[a.join("src")]);
        assert!(hint.contains("repoB (1 crate)"), "dedupes: {hint}");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn topmost_root_walk_names_consumers_of_any_crate() {
        // Scan ONLY repoA/crates/x — a sibling consuming repoA/crates/y
        // (a different crate of the same repo) must still be named: the
        // hint's unit is the REPO, not the nearest crate.
        let parent = fake_parent("topmost");
        let a = fake_repo(&parent, "repoA", "[workspace]\nmembers = [\"crates/*\"]\n");
        fake_member(&a, "crates/x", PKG);
        fake_member(&a, "crates/y", PKG);
        fake_repo(
            &parent,
            "repoB",
            &format!("{PKG}\n[dependencies]\ny = {{ path = \"../repoA/crates/y\" }}\n"),
        );
        let hint = downstream_consumers_hint(&[a.join("crates").join("x")]);
        assert!(hint.contains("repoB"), "repo-unit root: {hint}");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn worktree_git_file_sibling_counts() {
        // A linked worktree carries `.git` as a FILE — still a repo.
        let parent = fake_parent("worktree");
        let a = fake_repo(&parent, "repoA", PKG);
        let b = fake_repo(
            &parent,
            "repoB",
            &format!("{PKG}\n[dependencies]\nx = {{ path = \"../repoA\" }}\n"),
        );
        fs::remove_dir_all(b.join(".git")).unwrap();
        fs::write(b.join(".git"), "gitdir: ../elsewhere\n").unwrap();
        let hint = downstream_consumers_hint(&[a.join("src")]);
        assert!(hint.contains("repoB"), "worktree counts: {hint}");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn relative_target_fires() {
        // The scope_hint live-probe lesson: a relative target (the
        // `--config-audit ../sibling/repo` shape) must survive
        // canonicalization. The fixture root is `fake_parent` (temp_dir),
        // like every sibling test here — a fixture placed under
        // `cwd.parent()` nests inside a REAL repo when this crate is a
        // linked worktree (`.worktrees/<wt>` has the workspace root one
        // climb above it), and the topmost walk then correctly claims the
        // outer repo — the nested-repo shape, not the relative-path shape
        // under test. The `..`-prefix is manufactured by walking `cwd`
        // and the fixture to their common ancestor; cross-tree on every
        // real box, so the result is `..`-heavy by construction.
        let cwd = std::env::current_dir().unwrap();
        let parent = fake_parent("rel");
        let a = fake_repo(&parent, "repoA", PKG);
        fake_repo(
            &parent,
            "repoB",
            &format!("{PKG}\n[dependencies]\nx = {{ path = \"../repoA\" }}\n"),
        );
        let abs = a.join("src");
        let mut rest: Vec<_> = abs.components().collect();
        let mut from: Vec<_> = cwd.components().collect();
        while !rest.is_empty() && !from.is_empty() && rest[0] == from[0] {
            rest.remove(0);
            from.remove(0);
        }
        let mut rel = PathBuf::new();
        for _ in &from {
            rel.push("..");
        }
        rel.extend(rest);
        let hint = downstream_consumers_hint(&[rel]);
        assert!(hint.contains("repoB"), "relative target fires: {hint}");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn live_shape_relative_src_leaf_is_silent() {
        // The exact live invocation shape (`cargo heal --config-audit
        // src`, CWD = this repo root) against the REAL tree: riir-clippy
        // is a measured LEAF (no sibling path-deps into it) — the hint
        // must be silent through the real sibling enumeration.
        assert_eq!(downstream_consumers_hint(&[PathBuf::from("src")]), "");
    }

    #[test]
    fn empty_targets_are_silent() {
        assert_eq!(downstream_consumers_hint(&[]), "");
    }

    #[test]
    fn path_dep_scanner_shapes() {
        let text = "[dependencies]\nx = { path = \"../a\", version = \"1\" }\n\
                    pathy = { xpath = \"no\" }\n\
                    [dev-dependencies]\nz = {path=\"../b\"}\n";
        let vals = path_dep_values(text);
        assert!(vals.contains(&"../a"), "inline table: {vals:?}");
        assert!(vals.contains(&"../b"), "no-space form: {vals:?}");
        assert!(!vals.contains(&"no"), "xpath is not a path key: {vals:?}");
    }

    #[test]
    fn workspace_members_multiline_and_section_scoped() {
        let text = "[package]\nname = \"x\"\nmembers = [\"not-workspace\"]\n\
                    [workspace]\nmembers = [\n  \"crates/*\",\n  \"tools/tidy\",\n]\n\
                    [workspace.metadata]\nmembers = [\"no\"]\n";
        let members = workspace_members(text);
        assert!(members.contains(&"crates/*"), "{members:?}");
        assert!(members.contains(&"tools/tidy"), "{members:?}");
        assert!(!members.contains(&"not-workspace"), "package section: {members:?}");
        assert!(!members.contains(&"no"), "workspace.metadata section: {members:?}");
    }
}
