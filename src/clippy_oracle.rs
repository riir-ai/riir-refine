//! The auto-oracle (Issue 113): a scanning run over a cargo project lints it
//! with REAL clippy itself, so oracle-anchored spans (the ones that reproduce
//! under the proof oracle and can credit) become the default instead of a
//! flag nobody discovers.
//!
//! Before this module the feed was manual-only (`--clippy-json`): the caller
//! ran clippy and piped the stream in, because a guessed invocation is
//! silently blind (wrong feature set → cfg-gated code compiles to nothing;
//! toolchain pins; target-dir locks). The owner call (2026-09-16, the
//! "agave 6951 rows → +1 credit" session): `cargo refine --mine` must do every
//! good for the user — the manual feed kept that promise only for
//! AGENTS.md readers. The resolution keeps BOTH: the auto-run is the default
//! (spawn `cargo clippy --workspace --no-deps --message-format=json --quiet`
//! at DEFAULT features in the detected project root, `$CARGO`-resolved),
//! falls back LOUD to heuristic-only whenever it cannot run (no cargo,
//! spawn failure, compile error with zero parseable diagnostics), and the
//! explicit kill switch `RIIR_REFINE_AUTO_ORACLE=0` restores the old
//! never-spawn posture. A manual `--clippy-json` still wins outright.
//!
//! Precedent: `--fix-compile` already spawns cargo through
//! [`crate::compile_oracle`] — the healer spawning the user's toolchain is
//! not a new class. What stays true: the auto-run never GUESSES features
//! for you (default features unless you say otherwise, said so in the
//! status line) and never silently changes the verdict of a run it could
//! not anchor.
//!
//! Default-features scoping was honest, not complete: cfg-gated code compiles
//! to nothing under it (the workspace's own blindness class —
//! `consumer_residue_sweep.sh` runs `--all-features` for exactly this
//! reason). Issue 115 added the operator re-aim:
//! `RIIR_REFINE_AUTO_ORACLE_FEATURES=<csv>` spawns with `--features <csv>`
//! (a loud status line when active — a feature-superset clippy run is
//! materially slower; unknown feature names fail the clippy spawn, which
//! degrades LOUD to heuristic-only naming the knob). `--all-features`
//! stays a deliberate non-default (cost + sibling feature-combo explosion);
//! the auto-run never widens its own scope that far.
//!
//! GAP-ARMING is the default since the owner mandate of 2026-10-28 (the
//! plan-306 Phase-2 rider's measured miss: `ptr_arg` on a gated BM25 bench
//! never anchored until the oracle was re-aimed by hand): after the base
//! pass, the oracle runs `cargo metadata --no-deps` (manifest data only, no
//! compile), censuses every workspace target whose `required-features` the
//! posture does not enable (the SAME parser the P43 post-verify audit uses —
//! one home, the two surfaces cannot disagree), and arms bounded extra
//! passes over the distinct missing feature sets — one pass per set,
//! `RIIR_REFINE_AUTO_ORACLE_GAP_MAX_PASSES` (default 3), diagnostics merged
//! and deduped line-wise. A failed gap pass is disclosed and covers NOTHING
//! (a compile failure lints nothing — honest uncovered, never a green pass
//! over absent work); targets no pass covered are named with the exact env
//! csv to arm them. `RIIR_REFINE_AUTO_ORACLE_GAP_ARM=0` restores the
//! single-pass posture (census + disclosure only). The census also ships as
//! `cargo refine --oracle-census` — the sweep-side instrument that keeps a
//! "clean" clippy sweep from reading green over skipped gated targets.
//!
//! `--all-targets` IS the default (2026-09-21, the riir-chain/riir-ai intake
//! pair): without it the spawn lints only lib targets, so every `mod tests`
//! span — where fixture code lives, and where BOTH intake incidents sat — is
//! invisible to the anchor lane (a `manual_range_contains` in `mod tests`
//! was hand-applied while the oracle ran blind beside it). A knob nobody
//! passes is not a repair, so the WIDE posture is the default and
//! `RIIR_REFINE_AUTO_ORACLE_NO_ALL_TARGETS=1` restores the fast lib-only one
//! (only explicit truthy spellings disable; a typo must not silently
//! restore the blind posture).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Kill-switch predicate over the raw `RIIR_REFINE_AUTO_ORACLE` value.
/// Only explicit falsy spellings disable; unknown values default ON (a typo
/// must not silently restore the low-yield posture this module exists to fix).
#[must_use]
pub fn enabled_from(v: Option<&str>) -> bool {
    !matches!(v.map(str::trim), Some("0" | "false" | "no" | "off"))
}

/// The live env read.
#[must_use]
pub fn auto_oracle_enabled() -> bool {
    enabled_from(std::env::var("RIIR_REFINE_AUTO_ORACLE").ok().as_deref())
}

/// Parsed `RIIR_REFINE_AUTO_ORACLE_FEATURES` value (Issue 115): `None` = the
/// default-features posture (unset, blank, or nothing but empty segments);
/// `Some(non-empty)` = spawn clippy with `--features <csv>`. The oracle can
/// only name features the TARGET project's manifests carry — unknown names
/// fail the clippy spawn, which the caller degrades to heuristic-only LOUD
/// (never a silent un-anchored run that looks anchored).
#[must_use]
pub fn features_from(v: Option<&str>) -> Option<Vec<String>> {
    let raw = v?.trim();
    if raw.is_empty() {
        return None;
    }
    let list: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if list.is_empty() {
        None
    } else {
        Some(list)
    }
}

/// The live env read.
#[must_use]
pub fn auto_oracle_features() -> Option<Vec<String>> {
    features_from(
        std::env::var("RIIR_REFINE_AUTO_ORACLE_FEATURES")
            .ok()
            .as_deref(),
    )
}

/// Kill-switch predicate over the raw
/// `RIIR_REFINE_AUTO_ORACLE_NO_ALL_TARGETS` value (2026-09-21): the
/// `--all-targets` posture is the DEFAULT (test modules are half the
/// healing surface); only explicit truthy spellings disable it. Unknown
/// values keep it ON — a typo must not silently restore the blind posture.
#[must_use]
pub fn all_targets_from(v: Option<&str>) -> bool {
    !matches!(v.map(str::trim), Some("1" | "true" | "yes" | "on"))
}

/// The live env read.
#[must_use]
pub fn auto_oracle_all_targets() -> bool {
    all_targets_from(
        std::env::var("RIIR_REFINE_AUTO_ORACLE_NO_ALL_TARGETS")
            .ok()
            .as_deref(),
    )
}

/// Kill-switch predicate over the raw `RIIR_REFINE_AUTO_ORACLE_GAP_ARM`
/// value (owner mandate 2026-10-28): gap-arming is DEFAULT-ON; only explicit
/// falsy spellings disable — the same typo-proof contract as
/// [`enabled_from`].
#[must_use]
pub fn gap_arm_from(v: Option<&str>) -> bool {
    !matches!(v.map(str::trim), Some("0" | "false" | "no" | "off"))
}

/// The live env read.
#[must_use]
pub fn auto_oracle_gap_arm() -> bool {
    gap_arm_from(
        std::env::var("RIIR_REFINE_AUTO_ORACLE_GAP_ARM")
            .ok()
            .as_deref(),
    )
}

/// The gap-pass bound over the raw `RIIR_REFINE_AUTO_ORACLE_GAP_MAX_PASSES`
/// value: default 3; `0` = off; a garbage value parses to the DEFAULT — a
/// typo must not silently disable the coverage (the `enabled_from` law).
#[must_use]
pub fn gap_max_passes_from(v: Option<&str>) -> usize {
    match v.map(str::trim) {
        None | Some("") => 3,
        Some(s) => s.parse::<usize>().unwrap_or(3),
    }
}

/// The live env read.
#[must_use]
pub fn auto_oracle_gap_max_passes() -> usize {
    gap_max_passes_from(
        std::env::var("RIIR_REFINE_AUTO_ORACLE_GAP_MAX_PASSES")
            .ok()
            .as_deref(),
    )
}

/// One cargo target the invocation posture does not compile (the finding-side
/// twin of `fix_verify`'s P43 audit — see the module doc): an explicit
/// `[[test]]`/`[[bench]]`/`[[example]]`/`[[bin]]` row whose
/// `required-features` are not all enabled. `--all-targets` SKIPS it
/// silently — it compiles to nothing — so every clippy finding inside it is
/// invisible to the anchor lane (measured 2026-10-28: `ptr_arg` on a gated
/// BM25 bench never anchored until the oracle was re-aimed by hand — the
/// plan-306 Phase-2 rider).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedTarget {
    pub package: String,
    /// Target name (or its declared path when the row names neither).
    pub target: String,
    /// `test` / `bench` / `example` / `bin` / `lib` / `custom-build`.
    pub kind: String,
    /// Required-features not enabled at this posture, manifest order.
    pub missing: Vec<String>,
}

/// One extra pass armed for a distinct missing-feature set.
#[derive(Debug)]
pub struct GapRun {
    pub features: Vec<String>,
    /// `None` = killed by signal (or never spawned — see `spawn_error`).
    pub exit: Option<i32>,
    /// stdout lines unique to this pass (deduped against every earlier pass).
    pub new_lines: usize,
    pub spawn_error: Option<String>,
}

/// The whole oracle posture: the base pass the knobs asked for, plus (by
/// default) the bounded gap passes over the required-features census.
pub struct OracleSession {
    /// Merged stdout of every pass — unique lines, base-pass order first;
    /// feeds the same parse path a manual `--clippy-json` feed uses.
    pub text: String,
    /// The base pass's exit (`None` = killed by signal or never spawned).
    pub base_exit: Option<i32>,
    pub base_spawn_error: Option<String>,
    /// Targets the base posture skips. Empty alongside a `census_error`
    /// means the census could not run — never a clean census.
    pub census: Vec<SkippedTarget>,
    pub census_error: Option<String>,
    pub gaps: Vec<GapRun>,
    /// Census entries no successful pass covered (arming off, the bound
    /// exhausted, or every pass over their set failed).
    pub uncovered: Vec<SkippedTarget>,
}

/// `(package name, manifest path)` for every workspace member: exactly the
/// packages `cargo clippy --workspace` visits, from `cargo metadata
/// --format-version 1 --no-deps` (no network, no compile — manifest data
/// only).
///
/// # Errors
///
/// The spawn failure verbatim, a non-zero `cargo metadata` exit, or a body
/// that is not the expected JSON shape — the caller discloses; a silent
/// empty list would read as "no gaps".
pub fn workspace_manifest_paths(
    root: &Path,
    cargo_bin: &str,
) -> Result<Vec<(String, PathBuf)>, String> {
    let output = Command::new(cargo_bin)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot spawn `{cargo_bin} metadata`: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo metadata` exited {} in {}",
            output
                .status
                .code()
                .map_or_else(|| "a signal".to_string(), |c| c.to_string()),
            root.display()
        ));
    }
    parse_metadata_manifest_paths(&String::from_utf8_lossy(&output.stdout))
}

/// Pure half of [`workspace_manifest_paths`] (testable).
fn parse_metadata_manifest_paths(text: &str) -> Result<Vec<(String, PathBuf)>, String> {
    let v: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("`cargo metadata` output is not JSON: {e}"))?;
    let Some(packages) = v.get("packages").and_then(serde_json::Value::as_array) else {
        return Err("`cargo metadata` output has no packages array".to_string());
    };
    let mut out = Vec::new();
    for p in packages {
        let name = p.get("name").and_then(serde_json::Value::as_str);
        let path = p.get("manifest_path").and_then(serde_json::Value::as_str);
        if let (Some(name), Some(path)) = (name, path) {
            if !name.is_empty() && !path.is_empty() {
                out.push((name.to_string(), PathBuf::from(path)));
            }
        }
    }
    Ok(out)
}

/// The skipped-target census over one workspace: for each member package,
/// every explicit target row whose `required-features` survive the
/// invocation's feature view — the package's defaults plus the CLI features
/// that name it (plain, or `pkg/feat` / `pkg?/feat`-qualified; the shared
/// [`crate::fix_verify::feature_set_from_args`] ignores qualified forms by
/// design because they name OTHER crates' features, so the ones naming THIS
/// package are resolved here). Manifest reads only — no cargo, no compile.
#[must_use]
pub fn skipped_targets(
    packages: &[(String, PathBuf)],
    cli_features: &[String],
) -> Vec<SkippedTarget> {
    let mut out = Vec::new();
    for (pkg, manifest) in packages {
        let Ok(text) = std::fs::read_to_string(manifest) else {
            continue; // unreadable manifest: the clippy spawn fails loudly anyway
        };
        let args = if cli_features.is_empty() {
            None
        } else {
            Some(format!("--features {}", cli_features.join(",")))
        };
        let mut set = crate::manifest_gate::feature_set_from_args(&text, args.as_deref());
        for cli in cli_features {
            let Some((owner, feat)) = cli.split_once('/') else {
                continue;
            };
            if owner.trim_end_matches('?') == pkg {
                set.names.insert(feat.to_string());
            }
        }
        for (target, kind, missing) in
            crate::manifest_gate::required_features_skipped_targets(&text, &set)
        {
            out.push(SkippedTarget {
                package: pkg.clone(),
                target,
                kind,
                missing,
            });
        }
    }
    out
}

/// Distinct missing-feature sets across the census, first-appearance order,
/// UNQUALIFIED names — `cargo --features name` enables `name` in every
/// workspace package that declares it, which is exactly the arming a gap
/// pass needs (an unrelated package sharing the name gains the feature too;
/// that only widens the lint surface, never narrows it).
#[must_use]
pub fn gap_feature_sets(skipped: &[SkippedTarget]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    for t in skipped {
        let mut set = t.missing.clone();
        set.sort();
        if !out.contains(&set) {
            out.push(set);
        }
    }
    out
}

fn sorted_covers(gap_sorted: &[String], missing_sorted: &[String]) -> bool {
    missing_sorted
        .iter()
        .all(|m| gap_sorted.iter().any(|g| g == m))
}

/// The cache fingerprint for one oracle posture (`None` = the cache is
/// demoted by `RIIR_REFINE_ORACLE_CACHE=0`). Every knob that can move
/// clippy's output folds into the posture string — a knob change is a new
/// key, never a stale hit.
fn cache_fingerprint(
    root: &Path,
    cargo_bin: &str,
    features: Option<&[String]>,
    all_targets: bool,
) -> Option<crate::oracle_cache::WorkspaceFingerprint> {
    if !crate::oracle_cache::oracle_cache_enabled() {
        return None;
    }
    let posture = format!(
        "features={};all_targets={};gap_arm={};gap_max={};",
        features.map_or(String::new(), |f| f.join(",")),
        all_targets,
        auto_oracle_gap_arm(),
        auto_oracle_gap_max_passes(),
    );
    Some(crate::oracle_cache::workspace_fingerprint(
        root, cargo_bin, &posture,
    ))
}

/// Run the full oracle posture in `root`: the base pass exactly as the knobs
/// request it, then (default) the bounded gap passes. Never panics, never
/// aborts the caller's run — every failure mode lands in a field the caller
/// discloses.
///
/// The session is CACHE-KEYED (P-PAR, 2026-11-05): a fingerprint over the
/// workspace sources + manifests + lockfile + toolchain + invocation posture
/// admits a loud cache hit (`clippy oracle (auto): cache HIT …`) that
/// rebuilds the session from `.refine/oracle-cache/` instead of spawning
/// clippy. `RIIR_REFINE_ORACLE_CACHE=0` restores spawn-always.
#[must_use]
pub fn run_oracle_session(
    root: &Path,
    cargo_bin: &str,
    features: Option<&[String]>,
    all_targets: bool,
) -> OracleSession {
    // ── the cache lookup (P-PAR): every knob that can move the output is in
    // the posture string — a knob change is a new key, never a stale hit.
    // The fingerprint (and the cache dir) key the INHERITED WORKSPACE root —
    // `--workspace` from a member dir compiles the whole workspace, so a
    // member-local key would miss sibling-crate changes.
    let cache_root = crate::oracle_cache::workspace_root_for(root);
    let fingerprint = cache_fingerprint(&cache_root, cargo_bin, features, all_targets);
    if let Some(fp) = &fingerprint {
        if let Some(cached) = crate::oracle_cache::load_session(&cache_root, &fp.key_hex) {
            eprintln!(
                "clippy oracle (auto): cache HIT {} — {} file(s)/{} hashed, {} gap pass(es) reused (RIIR_REFINE_ORACLE_CACHE=0 disables)",
                &fp.key_hex[..12],
                fp.files_hashed,
                fp.toolchain,
                cached.gaps.len()
            );
            return hit_session(cached);
        }
    }
    let mut session = OracleSession {
        text: String::new(),
        base_exit: None,
        base_spawn_error: None,
        census: Vec::new(),
        census_error: None,
        gaps: Vec::new(),
        uncovered: Vec::new(),
    };
    match run_clippy(root, cargo_bin, features, all_targets) {
        Ok(run) => {
            session.text = run.text;
            session.base_exit = run.exit;
        }
        Err(e) => session.base_spawn_error = Some(e),
    }
    let cli: &[String] = features.unwrap_or(&[]);
    match workspace_manifest_paths(root, cargo_bin) {
        Ok(packages) => session.census = skipped_targets(&packages, cli),
        Err(e) => session.census_error = Some(e),
    }
    let max_passes = auto_oracle_gap_max_passes();
    let armed = session.base_spawn_error.is_none() && auto_oracle_gap_arm() && max_passes > 0;
    if armed {
        let mut seen: HashSet<String> = session.text.lines().map(str::to_owned).collect();
        for set in gap_feature_sets(&session.census)
            .into_iter()
            .take(max_passes)
        {
            match run_clippy(root, cargo_bin, Some(set.as_slice()), all_targets) {
                Ok(run) => {
                    let mut new_lines = 0usize;
                    for line in run.text.lines() {
                        if seen.insert(line.to_owned()) {
                            new_lines += 1;
                            session.text.push_str(line);
                            session.text.push('\n');
                        }
                    }
                    session.gaps.push(GapRun {
                        features: set,
                        exit: run.exit,
                        new_lines,
                        spawn_error: None,
                    });
                }
                Err(e) => session.gaps.push(GapRun {
                    features: set,
                    exit: None,
                    new_lines: 0,
                    spawn_error: Some(e),
                }),
            }
        }
    }
    // Coverage: a census entry is covered only by a gap pass that SPAWNED,
    // RAN, and whose feature set contains every feature it was missing — a
    // failed pass covers nothing (a compile failure lints nothing).
    session.uncovered = session
        .census
        .iter()
        .filter(|t| {
            let mut missing = t.missing.clone();
            missing.sort();
            !session.gaps.iter().any(|g| {
                g.spawn_error.is_none() && g.exit.is_some() && {
                    let mut gap_set = g.features.clone();
                    gap_set.sort();
                    sorted_covers(&gap_set, &missing)
                }
            })
        })
        .cloned()
        .collect();
    // ── the cache store (P-PAR): best-effort, never aborts — the run already
    // paid for the clippy passes; the cache only serves the NEXT run.
    if let Some(fp) = &fingerprint {
        store_cache(&cache_root, fp, &session);
    }
    session
}

/// The cache-side mirror of a run's results, stored under the fingerprint.
fn store_cache(
    root: &Path,
    fp: &crate::oracle_cache::WorkspaceFingerprint,
    session: &OracleSession,
) {
    let mut cached = crate::oracle_cache::CachedOracleSession::from_parts(
        fp.key_hex.clone(),
        session.text.clone(),
        session.base_exit,
    );
    cached
        .base_spawn_error
        .clone_from(&session.base_spawn_error);
    cached.census_error.clone_from(&session.census_error);
    cached.census = session
        .census
        .iter()
        .map(|c| crate::oracle_cache::CachedSkippedTarget {
            package: c.package.clone(),
            target: c.target.clone(),
            kind: c.kind.clone(),
            missing: c.missing.clone(),
        })
        .collect();
    cached.gaps = session
        .gaps
        .iter()
        .map(|g| crate::oracle_cache::CachedGapRun {
            features: g.features.clone(),
            exit: g.exit,
            new_lines: g.new_lines,
            spawn_error: g.spawn_error.clone(),
        })
        .collect();
    cached.uncovered = session
        .uncovered
        .iter()
        .map(|c| crate::oracle_cache::CachedSkippedTarget {
            package: c.package.clone(),
            target: c.target.clone(),
            kind: c.kind.clone(),
            missing: c.missing.clone(),
        })
        .collect();
    crate::oracle_cache::store_session(root, &cached);
}

/// Rebuild a live session from a cache entry (the HIT path).
#[must_use]
pub fn hit_session(cached: crate::oracle_cache::CachedOracleSession) -> OracleSession {
    OracleSession {
        text: cached.text,
        base_exit: cached.base_exit,
        base_spawn_error: cached.base_spawn_error,
        census: cached
            .census
            .into_iter()
            .map(|c| SkippedTarget {
                package: c.package,
                target: c.target,
                kind: c.kind,
                missing: c.missing,
            })
            .collect(),
        census_error: cached.census_error,
        gaps: cached
            .gaps
            .into_iter()
            .map(|g| GapRun {
                features: g.features,
                exit: g.exit,
                new_lines: g.new_lines,
                spawn_error: g.spawn_error,
            })
            .collect(),
        uncovered: cached
            .uncovered
            .into_iter()
            .map(|c| SkippedTarget {
                package: c.package,
                target: c.target,
                kind: c.kind,
                missing: c.missing,
            })
            .collect(),
    }
}

/// The `--oracle-census` report: what the oracle's CURRENT posture cannot
/// see. Manifest data only (metadata + manifest reads, no compile) — the
/// sweep-side instrument that keeps a "clean" clippy sweep from reading
/// green over skipped gated targets.
///
/// # Errors
///
/// [`workspace_manifest_paths`] failures — the caller refuses rather than
/// printing a blind report.
pub fn census_report(
    root: &Path,
    cargo_bin: &str,
    cli_features: &[String],
) -> Result<String, String> {
    use std::fmt::Write as _;
    let packages = workspace_manifest_paths(root, cargo_bin)?;
    let census = skipped_targets(&packages, cli_features);
    let posture = if cli_features.is_empty() {
        "default features".to_string()
    } else {
        format!("features [{}]", cli_features.join(","))
    };
    let mut out = String::new();
    let _ = writeln!(out, "oracle census: {} at {}", root.display(), posture);
    if census.is_empty() {
        let _ = writeln!(
            out,
            "oracle census: 0 gated targets skipped — the posture sees every explicit target's required-features"
        );
        return Ok(out);
    }
    let _ = writeln!(
        out,
        "oracle census: {} gated target(s) INVISIBLE at this posture (compiled to nothing):",
        census.len()
    );
    for t in &census {
        let _ = writeln!(
            out,
            "  {} [{}] {} ← missing: {}",
            t.package,
            t.kind,
            t.target,
            t.missing.join(",")
        );
    }
    let sets = gap_feature_sets(&census);
    let rendered = sets
        .iter()
        .take(8)
        .map(|s| s.join("+"))
        .collect::<Vec<_>>()
        .join(" | ");
    let _ = writeln!(
        out,
        "arm with: RIIR_REFINE_AUTO_ORACLE_FEATURES=<csv> — distinct sets (first 8): {rendered}"
    );
    Ok(out)
}

/// Nearest cargo project root for the scan targets (v1: the FIRST target —
/// multi-project scans get one oracle run, loudly scoped by the status
/// line). A directory is probed for its own manifest, then up to
/// [`MAX_ANCESTOR_STEPS`] parents; a file starts at its parent. Bounded so a
/// stray scan under an unrelated ancestor never picks up that project.
const MAX_ANCESTOR_STEPS: usize = 4;

#[must_use]
pub fn detect_project_root(paths: &[PathBuf], cwd: &Path) -> Option<PathBuf> {
    let start: &Path = paths.first().map_or(cwd, |p| p.as_path());
    let mut dir: PathBuf = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };
    // Anchor to the caller's cwd BEFORE walking up: a RELATIVE 1-component
    // target (`cargo refine --suggest src/` — the documented invocation form)
    // parents to the EMPTY path, and `"".join("Cargo.toml")` then resolves
    // against the process cwd — returning `Some("")` as the "root", whose
    // `Command::current_dir("")` spawn fails with os error 123 on Windows /
    // ENOENT on Unix. The exact class `compile_oracle::owning_package_root`
    // already fixed for the --fix-compile lane (the Issue 015 GOAT run's
    // spawn failure); ported here when the 2026-09-20 4090 sweep hit it on
    // the auto-oracle.
    if dir.as_os_str().is_empty() {
        dir = cwd.to_path_buf();
    } else if dir.is_relative() {
        // `cwd.join(".")` would leave a trailing CurDir component
        // ("<cwd>/." — spawns fine but is neither displayable nor
        // PathBuf-equal to the root it names); normalize the bare-dot
        // input to cwd itself.
        dir = if dir == Path::new(".") {
            cwd.to_path_buf()
        } else {
            cwd.join(dir)
        };
    }
    for _ in 0..=MAX_ANCESTOR_STEPS {
        if dir.join("Cargo.toml").is_file() {
            return Some(dir);
        }
        dir = dir.parent()?.to_path_buf();
    }
    None
}

/// One oracle invocation's raw result: the JSON stream (lossy-decoded —
/// clippy escapes non-ASCII in JSON, lossy only defends against a
/// misbehaving toolchain) and the exit code (None = killed by signal).
pub struct OracleRun {
    pub text: String,
    pub exit: Option<i32>,
}

/// Spawn `cargo clippy --workspace --no-deps --message-format=json --quiet
/// [--all-targets] [--features <csv>]` in `root`.
///
/// `--all-targets` (the default posture — see [`all_targets_from`]) lints
/// test/bench/example targets too, so `mod tests` spans anchor like any
/// other. `features` is the Issue-115 re-aim (`None` = default features).
///
/// # Errors
///
/// Returns the spawn failure verbatim — the caller degrades to
/// heuristic-only with this message; a spawned-but-failing clippy is NOT an
/// error here (its exit code travels in [`OracleRun`]).
pub fn run_clippy(
    root: &Path,
    cargo_bin: &str,
    features: Option<&[String]>,
    all_targets: bool,
) -> Result<OracleRun, String> {
    let mut cmd = Command::new(cargo_bin);
    cmd.args([
        "clippy",
        "--workspace",
        "--no-deps",
        "--message-format=json",
        "--quiet",
    ]);
    if all_targets {
        cmd.arg("--all-targets");
    }
    if let Some(set) = features {
        cmd.arg("--features").arg(set.join(","));
    }
    let output = cmd
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot spawn `{cargo_bin} clippy`: {e}"))?;
    Ok(OracleRun {
        text: String::from_utf8_lossy(&output.stdout).into_owned(),
        exit: output.status.code(),
    })
}

/// What a finished run means for the feed.
pub enum OracleOutcome {
    /// Parsed ≥1 clippy diagnostic — anchor them (a non-zero exit alongside
    /// parsed diagnostics is a PARTIAL compile: crates that built were
    /// linted; the diagnostics are real either way).
    Feed(usize),
    /// Exit 0 with zero diagnostics — a genuinely clean tree at default
    /// features; an empty feed (marks covered files heuristic, same as the
    /// manual 0-diagnostic feed).
    Clean,
    /// Non-zero exit and nothing parseable — the compile itself failed;
    /// heuristic-only, never a fabricated "clean".
    Failed(String),
}

#[must_use]
pub fn classify(exit: Option<i32>, n_parsed: usize) -> OracleOutcome {
    if n_parsed > 0 {
        return OracleOutcome::Feed(n_parsed);
    }
    match exit {
        Some(0) => OracleOutcome::Clean,
        Some(code) => OracleOutcome::Failed(format!(
            "clippy exited {code} with no parseable diagnostics (compile error?)"
        )),
        None => OracleOutcome::Failed(
            "clippy was killed by a signal with no parseable diagnostics".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join("cargo-refine-clippy-oracle-tests")
            .join(format!("{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn kill_switch_only_explicit_falsy_spellings() {
        assert!(enabled_from(None));
        assert!(enabled_from(Some("")));
        assert!(enabled_from(Some("1")));
        assert!(enabled_from(Some("on")));
        assert!(enabled_from(Some(" yes ")), "trimmed truthy stays on");
        assert!(!enabled_from(Some("0")));
        assert!(!enabled_from(Some("false")));
        assert!(!enabled_from(Some("off")));
        assert!(
            enabled_from(Some("2")),
            "unknown values default ON, not silent-off"
        );
    }

    #[test]
    fn all_targets_default_on_only_explicit_truthy_disables() {
        assert!(all_targets_from(None));
        assert!(all_targets_from(Some("")));
        assert!(all_targets_from(Some("0")));
        assert!(
            all_targets_from(Some("false")),
            "falsy keeps the wide posture"
        );
        assert!(all_targets_from(Some("2")), "unknown values keep it ON");
        assert!(!all_targets_from(Some("1")));
        assert!(!all_targets_from(Some("true")));
        assert!(!all_targets_from(Some(" on ")), "trimmed truthy disables");
    }

    #[test]
    fn features_knob_unset_blank_or_empty_segments_is_default_posture() {
        assert_eq!(features_from(None), None);
        assert_eq!(features_from(Some("")), None);
        assert_eq!(features_from(Some("   ")), None);
        assert_eq!(
            features_from(Some(",, ,,")),
            None,
            "nothing but empty segments"
        );
    }

    #[test]
    fn features_knob_parses_trimmed_csv_dropping_empty_segments() {
        assert_eq!(features_from(Some("gated")), Some(vec!["gated".to_owned()]));
        assert_eq!(
            features_from(Some(" a , b ,c")),
            Some(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()])
        );
        assert_eq!(
            features_from(Some("a ,, b")),
            Some(vec!["a".to_owned(), "b".to_owned()]),
            "interior empty segments drop, the rest keep their order"
        );
    }

    #[test]
    fn detects_a_direct_project_dir() {
        let d = tmp_dir("direct");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        assert_eq!(
            detect_project_root(std::slice::from_ref(&d), Path::new("/nowhere")),
            Some(d)
        );
    }

    #[test]
    fn detects_the_parent_of_a_file_target() {
        let d = tmp_dir("file");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let f = d.join("src").join("main.rs");
        fs::create_dir_all(f.parent().unwrap()).unwrap();
        fs::write(&f, "fn main() {}").unwrap();
        assert_eq!(detect_project_root(&[f], Path::new("/nowhere")), Some(d));
    }

    #[test]
    fn empty_paths_fall_back_to_cwd() {
        let d = tmp_dir("cwd");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        assert_eq!(detect_project_root(&[], &d), Some(d));
    }

    #[test]
    fn no_project_within_bounds_is_none_not_a_walk_to_the_root() {
        let base = tmp_dir("none");
        let nested = base.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(detect_project_root(&[nested], Path::new("/nowhere")), None);
    }

    #[test]
    fn one_component_relative_target_resolves_against_cwd_never_empty() {
        // The documented invocation shape: `cargo refine --suggest src/` run
        // from the repo root. Before the cwd-anchor fix the walk went
        // `src` → `""` → `"".join("Cargo.toml")` resolves against the
        // process cwd → `Some("")` — an empty "root" whose spawn failed
        // with os error 123 (the 2026-09-20 4090 sweep find). Both arms of
        // the start ("src" resolves as a dir at process level, or not)
        // must now converge on the ABSOLUTE project root.
        let d = tmp_dir("relative");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        fs::create_dir_all(d.join("src")).unwrap();
        let root = detect_project_root(&[PathBuf::from("src")], &d).expect("root found via cwd");
        assert_eq!(root, d, "the cwd-anchored walk finds the tmp project");
        assert!(
            root.is_absolute(),
            "a relative root would spawn against the healer's own cwd"
        );
    }

    #[test]
    fn relative_file_target_resolves_via_cwd_not_the_empty_parent() {
        // `cargo refine --suggest src/lib.rs` from the repo root: the file's
        // parent ("src") is relative; the empty-path walk used to fire only
        // for 1-component DIRS, but the anchor must hold for files too.
        let d = tmp_dir("relative-file");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        fs::create_dir_all(d.join("src")).unwrap();
        fs::write(d.join("src").join("lib.rs"), "").unwrap();
        let root =
            detect_project_root(&[PathBuf::from("src/lib.rs")], &d).expect("root found via cwd");
        assert_eq!(root, d);
        assert!(root.is_absolute());
    }

    #[test]
    fn bare_dot_target_is_the_cwd_project() {
        // `cargo refine --suggest .` — the dir "." is relative; anchored to
        // cwd it IS the project. Must never walk through "" to find it.
        let d = tmp_dir("dot");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let root = detect_project_root(&[PathBuf::from(".")], &d).expect("root found via cwd");
        assert_eq!(root, d);
        assert!(root.is_absolute());
    }

    #[test]
    fn classify_splits_feed_clean_failed() {
        assert!(matches!(classify(Some(0), 3), OracleOutcome::Feed(3)));
        // Partial compile: diagnostics exist despite the failure.
        assert!(matches!(classify(Some(101), 2), OracleOutcome::Feed(2)));
        assert!(matches!(classify(Some(0), 0), OracleOutcome::Clean));
        assert!(matches!(classify(Some(101), 0), OracleOutcome::Failed(_)));
        assert!(matches!(classify(None, 0), OracleOutcome::Failed(_)));
    }

    #[test]
    fn gap_arm_knob_only_explicit_falsy_spellings_disable() {
        assert!(gap_arm_from(None));
        assert!(gap_arm_from(Some("")));
        assert!(gap_arm_from(Some("2")), "unknown values default ON");
        assert!(!gap_arm_from(Some("0")));
        assert!(!gap_arm_from(Some("false")));
        assert!(!gap_arm_from(Some(" off ")));
    }

    #[test]
    fn gap_max_passes_defaults_on_garbage_and_zero_disables() {
        assert_eq!(gap_max_passes_from(None), 3);
        assert_eq!(gap_max_passes_from(Some("")), 3);
        assert_eq!(
            gap_max_passes_from(Some("junk")),
            3,
            "a typo must not disable"
        );
        assert_eq!(gap_max_passes_from(Some(" 2 ")), 2);
        assert_eq!(gap_max_passes_from(Some("0")), 0);
    }

    fn metadata_fixture() -> String {
        r#"{
          "packages": [
            {"name": "alpha", "manifest_path": "/w/alpha/Cargo.toml"},
            {"name": "beta", "manifest_path": "/w/beta/Cargo.toml"}
          ]
        }"#
        .to_string()
    }

    #[test]
    fn metadata_parse_extracts_members_in_order() {
        let parsed = parse_metadata_manifest_paths(&metadata_fixture()).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "alpha");
        assert_eq!(parsed[0].1, PathBuf::from("/w/alpha/Cargo.toml"));
        assert_eq!(parsed[1].0, "beta");
    }

    #[test]
    fn metadata_parse_rejects_garbage_loud_never_a_silent_empty_list() {
        assert!(parse_metadata_manifest_paths("not json").is_err());
        assert!(parse_metadata_manifest_paths("{}").is_err());
    }

    fn census_project(tag: &str) -> PathBuf {
        let d = tmp_dir(tag);
        fs::write(
            d.join("Cargo.toml"),
            concat!(
                "[package]\nname = \"mycrate\"\n",
                "[features]\ndefault = [\"base\"]\nbase = []\nbm25_facet = []\n",
                "[[bench]]\nname = \"gated_bench\"\nrequired-features = [\"bm25_facet\"]\n",
                "[[test]]\nname = \"dual\"\nrequired-features = [\"base\", \"bm25_facet\"]\n",
                "[[test]]\nname = \"plain\"\n",
            ),
        )
        .unwrap();
        d
    }

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn census_skips_gated_rows_honors_defaults_and_qualified_cli() {
        let d = census_project("census-basic");
        let manifest = d.join("Cargo.toml");
        let packages = vec![("mycrate".to_string(), manifest.clone())];
        // Default posture: the bench is skipped; the dual test is missing
        // only bm25_facet (base rides the default set); the plain test never
        // appears.
        let skipped = skipped_targets(&packages, &[]);
        assert_eq!(skipped.len(), 2, "bench + dual, never plain: {skipped:?}");
        assert_eq!(skipped[0].target, "gated_bench");
        assert_eq!(skipped[0].kind, "bench");
        assert_eq!(skipped[0].missing, strs(&["bm25_facet"]));
        assert_eq!(skipped[1].target, "dual");
        assert_eq!(skipped[1].missing, strs(&["bm25_facet"]));
        // The feature armed: nothing skipped.
        assert_eq!(skipped_targets(&packages, &strs(&["bm25_facet"])), []);
        // The qualified form resolving THIS package: also covered.
        assert_eq!(
            skipped_targets(&packages, &strs(&["mycrate/bm25_facet"])),
            [],
            "pkg/feat form must count for its own package"
        );
        // A qualified form for a DIFFERENT package never covers it.
        assert_eq!(
            skipped_targets(&packages, &strs(&["other/bm25_facet"])).len(),
            2
        );
    }

    #[test]
    fn gap_sets_dedupe_and_preserve_first_appearance_order() {
        let skipped = vec![
            SkippedTarget {
                package: "a".into(),
                target: "t1".into(),
                kind: "test".into(),
                missing: strs(&["z", "m"]),
            },
            SkippedTarget {
                package: "a".into(),
                target: "t2".into(),
                kind: "test".into(),
                missing: strs(&["z", "m"]),
            },
            SkippedTarget {
                package: "b".into(),
                target: "t3".into(),
                kind: "bench".into(),
                missing: strs(&["solo"]),
            },
        ];
        let sets = gap_feature_sets(&skipped);
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0], strs(&["m", "z"]));
        assert_eq!(sets[1], strs(&["solo"]));
    }

    #[test]
    fn sorted_covers_is_subset_not_equality() {
        let gap = strs(&["a", "b", "c"]);
        assert!(sorted_covers(&gap, &strs(&["b"])));
        assert!(!sorted_covers(&gap, &strs(&["b", "x"])));
        assert!(sorted_covers(&gap, &[]));
    }
}
