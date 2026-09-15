//! Scratch-probe identity witness — the tripwire for Issue 110's class.
//!
//! Issue 110 fixed the *known* trigger for scratch-crate unit collision (a
//! fixed package name plus an inherited `CARGO_TARGET_DIR`) by keying the
//! package name with the same BLAKE3 stem as the directory. That repair is
//! correct and it is not a witness: it makes the collision impossible *by
//! construction*, so nothing observes whether the construction still holds.
//! The whole reason the class survived was that its failure mode is a
//! **silent false PASS** — `Finished` in 0.00s, exit 0, zero diagnostics,
//! `lint_resolved = true`. Anything that re-opens the door (a de-keyed name,
//! a reachable `.cargo/config.toml` with `build.target-dir`, a future cargo
//! freshness change) would be just as silent.
//!
//! This module is the witness. It answers one question per probe: **did
//! cargo compile the source I just wrote?**
//!
//! ## What is and is not observable (measured 2026-09-16, shikuwa)
//!
//! Two scratch crates named `clippy_verify v0.0.0` in one `CARGO_TARGET_DIR`,
//! crate A `pub fn f() -> i32 { return 1; }`, crate B `pub fn g() …`:
//!
//! | field of B's `--message-format=json` | says | usable? |
//! |---|---|---|
//! | `package_id` | `…/probewit/b#clippy_verify@0.0.0` | ⛔ **lies** — cargo rewrites it to the manifest we asked for |
//! | `target.src_path` | `…/probewit/b/src/lib.rs` | ⛔ **lies**, same reason |
//! | span `file_name` | `src\lib.rs` | ⛔ relative, identical for every probe |
//! | span `text[].text` | `pub fn f() -> i32 { return 1; }` | ⚠ the truth — but **A's** source line, and only present when a diagnostic fires |
//! | `compiler-artifact.fresh` | `true` | ✅ **the witness** |
//!
//! So the two metadata fields a reader would reach for first are exactly the
//! ones cargo rewrites, and the span text — the one field that does betray
//! the replay — is absent in the silent case that matters (zero diagnostics).
//! `fresh` is the field that is both present on every run and true only when
//! cargo did not compile.
//!
//! ## Why `fresh` alone is not enough, and what makes it exact
//!
//! `fresh: true` is *legitimate* on a repeat probe of identical code — the
//! BLAKE3-keyed directory exists precisely so cargo reuses that build cache,
//! and the cached diagnostics are then genuinely that crate's own. A witness
//! that reds there would fire on the intended fast path.
//!
//! The exact predicate is the **join**: the probe knows whether it just
//! *changed* `src/lib.rs`. Cargo cannot legitimately call a unit fresh whose
//! source changed under it. So:
//!
//! - source changed + `fresh` → [`ProbeWitness::StaleAfterWrite`], a proof
//!   that the verdict describes other code;
//! - source unchanged + `fresh` → [`ProbeWitness::CachedUnchanged`], the
//!   intended cache hit;
//! - no `compiler-artifact`, but an unsuccessful `build-finished` with real
//!   error diagnostics → [`ProbeWitness::CompileFailed`]: the probe's code
//!   does not compile, which is the verdict both verifiers exist to produce;
//! - no `compiler-artifact` and no such evidence → [`ProbeWitness::NoArtifact`],
//!   nothing was measured, which is never the same as a clean result.
//!
//! ⛔ That fourth arm is not a refinement thought up in advance — the first
//! version of this witness had three, and the `--ignored` real-cargo arms
//! caught it making both verifiers refuse on broken input, which is the one
//! input they are most often handed.
//!
//! ## The `cargo run` site needs a different witness
//!
//! [`crate::domains::rust_perf::bench`] spawns `cargo run --release` and
//! reads the child's own stdout, so there is no cargo JSON stream to read a
//! `fresh` flag out of. Its collision is also a strictly worse thing — not a
//! wrong verdict but *a measurement that never happened, reported as one* —
//! and it has a stronger witness available for free: the generated program
//! can state its own identity. [`probe_id_line`] / [`parse_probe_id`] are
//! that echo; the bench parser requires the stem it generated.

/// Whether cargo compiled the source the probe just wrote.
///
/// Returned by [`witness_cargo_json`]. Only [`Self::Compiled`] and
/// [`Self::CachedUnchanged`] license reading the diagnostics as this
/// probe's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeWitness {
    /// The probe's source changed and cargo rebuilt it. The normal path.
    Compiled,
    /// The probe's source was byte-identical to the previous run and cargo
    /// reused its cache. The cached diagnostics are this crate's own.
    CachedUnchanged,
    /// ⛔ The source CHANGED and cargo still reported the unit `fresh`.
    /// Whatever came back describes **other code** — the Issue 110 class,
    /// by whatever mechanism re-opened it.
    StaleAfterWrite,
    /// The probe's code did not compile: no artifact, but cargo reported
    /// `build-finished` unsuccessfully with real error diagnostics. That is
    /// a **verdict**, not blindness — "this code does not compile" is
    /// precisely what both verifiers exist to find out — so it is
    /// trustworthy and the caller reads the diagnostics as usual.
    ///
    /// ⚠ The residual, named rather than assumed away: a replay whose
    /// donor crate ALSO failed would land here carrying the donor's errors.
    /// That direction is both rarer (a failed unit has no successful
    /// fingerprint to replay from, so cargo rebuilds) and conservative (it
    /// rejects a fix rather than accepting one). The silent-false-PASS
    /// direction, which is the one that shipped, cannot reach this arm:
    /// it has an artifact and no errors.
    CompileFailed,
    /// No `compiler-artifact`, and no evidence cargo compiled anything —
    /// it aborted before producing a unit (bad manifest, resolver error) or
    /// never started. Nothing was measured. Distinct from a clean result and
    /// never folded into one.
    NoArtifact,
}

impl ProbeWitness {
    /// Whether the probe's output may be read as a verdict about its own code.
    #[must_use]
    pub fn trustworthy(self) -> bool {
        matches!(
            self,
            Self::Compiled | Self::CachedUnchanged | Self::CompileFailed
        )
    }

    /// A one-line reason, for the `error` field of a did-not-run result.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::Compiled => "probe compiled",
            Self::CachedUnchanged => "probe source unchanged, cargo cache reused",
            Self::StaleAfterWrite => {
                "probe source changed but cargo reported the unit fresh — the output \
                 describes other code (Issue 110 scratch-crate unit collision); check for \
                 a shared CARGO_TARGET_DIR or a de-keyed package name"
            }
            Self::CompileFailed => "probe source did not compile (a verdict, not blindness)",
            Self::NoArtifact => "cargo emitted no compiler-artifact — nothing was compiled",
        }
    }
}

/// Write `code` to `path`, reporting whether the bytes on disk **changed**.
///
/// The return value is the load-bearing half of [`witness_cargo_json`]: it
/// is what separates a legitimate cache hit from a replay. Reading before
/// writing also avoids touching the file's mtime on an unchanged repeat,
/// which is what keeps the intended cargo cache hit intended.
///
/// # Errors
///
/// Propagates the write error. A read error is **not** an error — an absent
/// or unreadable file is treated as "changed", the conservative direction.
pub fn write_probe_source(path: &std::path::Path, code: &str) -> Result<bool, String> {
    let changed = match std::fs::read_to_string(path) {
        Ok(existing) => existing != code,
        Err(_) => true,
    };
    if changed {
        std::fs::write(path, code).map_err(|e| format!("write {} failed: {e}", path.display()))?;
    }
    Ok(changed)
}

/// Classify one probe run from cargo's `--message-format=json` stdout.
///
/// `source_changed` is [`write_probe_source`]'s verdict for this probe.
///
/// A unit is read as fresh only from a `compiler-artifact` message; any
/// fresh artifact is enough to condemn a changed-source run, because every
/// unit of a scratch crate whose source moved must rebuild.
#[must_use]
pub fn witness_cargo_json(stdout: &str, source_changed: bool) -> ProbeWitness {
    let mut saw_artifact = false;
    let mut saw_fresh = false;
    let mut saw_error = false;
    let mut build_failed = false;

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue; // human-readable progress lines, etc.
        };
        match v.get("reason").and_then(serde_json::Value::as_str) {
            Some("compiler-artifact") => {
                saw_artifact = true;
                if v.get("fresh").and_then(serde_json::Value::as_bool) == Some(true) {
                    saw_fresh = true;
                }
            }
            Some("compiler-message")
                if v.pointer("/message/level")
                    .and_then(serde_json::Value::as_str)
                    == Some("error") =>
            {
                saw_error = true;
            }
            Some("build-finished")
                if v.get("success").and_then(serde_json::Value::as_bool) == Some(false) =>
            {
                build_failed = true;
            }
            _ => {}
        }
    }

    if !saw_artifact {
        // A crate that does not compile produces no artifact — and that is
        // the answer both verifiers were asking for, not a blind spot.
        return if build_failed && saw_error {
            ProbeWitness::CompileFailed
        } else {
            ProbeWitness::NoArtifact
        };
    }
    match (source_changed, saw_fresh) {
        (true, true) => ProbeWitness::StaleAfterWrite,
        (false, true) => ProbeWitness::CachedUnchanged,
        // A rebuild is correct whether or not the source moved: cargo may
        // rebuild an unchanged crate for reasons of its own (toolchain bump,
        // a cleaned target dir) and that is never the failure direction.
        (_, false) => ProbeWitness::Compiled,
    }
}

/// The identity line a generated `main` prints so the parser can prove the
/// binary that ran is the one it generated.
///
/// Emitted as the first JSON line of the bench subprocess's stdout, where
/// [`parse_probe_id`] reads it back. `cargo run` gives no `--message-format`
/// stream, so this echo is the only witness available at that site — and it
/// is a stronger one, asserting the executed *program* rather than the
/// compiled unit.
#[must_use]
pub fn probe_id_line(stem: &str) -> String {
    format!("{{\"probe\":\"{stem}\"}}")
}

/// Read back the identity emitted by [`probe_id_line`], if present.
///
/// Returns `None` when no `probe` line appears at all, which — exactly as
/// [`ProbeWitness::NoArtifact`] — must not be read as agreement.
#[must_use]
pub fn parse_probe_id(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(id) = v.get("probe").and_then(serde_json::Value::as_str) {
            return Some(id.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The captured shape of crate A's run in the Issue 110 fixture: a real
    /// compile, `fresh: false`.
    const ARTIFACT_BUILT: &str = r#"{"reason":"compiler-message","message":{"code":{"code":"clippy::needless_return"}}}
{"reason":"compiler-artifact","package_id":"path+file:///tmp/probewit/a#clippy_verify@0.0.0","fresh":false}
{"reason":"build-finished","success":true}"#;

    /// The captured shape of crate B's run in the same fixture — the
    /// collision. Note that `package_id` names B while the diagnostic is A's:
    /// this is exactly why the witness reads `fresh` and not the metadata.
    const ARTIFACT_REPLAYED: &str = r#"{"reason":"compiler-message","message":{"code":{"code":"clippy::needless_return"}}}
{"reason":"compiler-artifact","package_id":"path+file:///tmp/probewit/b#clippy_verify@0.0.0","fresh":true}
{"reason":"build-finished","success":true}"#;

    #[test]
    fn changed_source_reported_fresh_is_the_collision() {
        assert_eq!(
            witness_cargo_json(ARTIFACT_REPLAYED, true),
            ProbeWitness::StaleAfterWrite
        );
        assert!(!ProbeWitness::StaleAfterWrite.trustworthy());
    }

    #[test]
    fn unchanged_source_reported_fresh_is_the_intended_cache_hit() {
        // The same bytes that condemn a changed-source run license an
        // unchanged one. Without this arm the witness would red on the
        // BLAKE3-keyed fast path the directory exists to provide.
        assert_eq!(
            witness_cargo_json(ARTIFACT_REPLAYED, false),
            ProbeWitness::CachedUnchanged
        );
        assert!(ProbeWitness::CachedUnchanged.trustworthy());
    }

    #[test]
    fn a_real_compile_is_trustworthy_either_way() {
        assert_eq!(
            witness_cargo_json(ARTIFACT_BUILT, true),
            ProbeWitness::Compiled
        );
        assert_eq!(
            witness_cargo_json(ARTIFACT_BUILT, false),
            ProbeWitness::Compiled
        );
    }

    #[test]
    fn no_artifact_is_not_a_pass() {
        // cargo aborted before producing a unit (bad manifest, resolver
        // error). `build-finished` alone is not evidence anything compiled.
        let out = r#"{"reason":"build-finished","success":false}"#;
        assert_eq!(witness_cargo_json(out, true), ProbeWitness::NoArtifact);
        assert_eq!(witness_cargo_json("", true), ProbeWitness::NoArtifact);
        assert!(!ProbeWitness::NoArtifact.trustworthy());
    }

    /// ⛔ The false positive the `--ignored` real-cargo arms caught on the
    /// first run: a probe whose code does not compile emits NO artifact, and
    /// the first version of this witness read that as blindness and made
    /// both verifiers refuse. But "does not compile" is the verdict they
    /// exist to produce — `verify_compilation_error_reports_not_resolved`
    /// asserts exactly that. Evidence of a real failed compile is errors
    /// plus an unsuccessful `build-finished`.
    #[test]
    fn a_failed_compile_is_a_verdict_not_blindness() {
        let out = r#"{"reason":"compiler-message","message":{"level":"error","code":null}}
{"reason":"build-finished","success":false}"#;
        assert_eq!(witness_cargo_json(out, true), ProbeWitness::CompileFailed);
        assert!(ProbeWitness::CompileFailed.trustworthy());
        // Warnings are not errors: a warning-only run that produced no unit
        // is still unexplained, and must not be laundered into a verdict.
        let warn = r#"{"reason":"compiler-message","message":{"level":"warning"}}
{"reason":"build-finished","success":false}"#;
        assert_eq!(witness_cargo_json(warn, true), ProbeWitness::NoArtifact);
        // And errors without a failed build-finished (cargo killed mid-run)
        // are not one either.
        let partial = r#"{"reason":"compiler-message","message":{"level":"error"}}"#;
        assert_eq!(witness_cargo_json(partial, true), ProbeWitness::NoArtifact);
    }

    /// The silent-false-PASS direction — the one that actually shipped —
    /// cannot reach [`ProbeWitness::CompileFailed`]: it has an artifact and
    /// no errors, so it lands on the freshness arm regardless of what any
    /// `build-finished` says.
    #[test]
    fn the_collision_cannot_hide_in_the_compile_failed_arm() {
        let out =
            format!("{ARTIFACT_REPLAYED}\n{{\"reason\":\"build-finished\",\"success\":false}}");
        assert_eq!(
            witness_cargo_json(&out, true),
            ProbeWitness::StaleAfterWrite
        );
    }

    #[test]
    fn non_json_lines_do_not_decide_anything() {
        let out = format!("    Checking clippy_verify_abc v0.0.0\n{ARTIFACT_BUILT}\nnot json");
        assert_eq!(witness_cargo_json(&out, true), ProbeWitness::Compiled);
    }

    #[test]
    fn any_fresh_unit_condemns_a_changed_probe() {
        // `cargo test` emits an artifact per unit (lib, then the test bin).
        // One rebuilt unit does not license a replayed one.
        let out = r#"{"reason":"compiler-artifact","fresh":false}
{"reason":"compiler-artifact","fresh":true}"#;
        assert_eq!(witness_cargo_json(out, true), ProbeWitness::StaleAfterWrite);
    }

    #[test]
    fn write_probe_source_reports_change_then_stability() {
        let dir = std::env::temp_dir().join("riir_clippy_probe_witness_write");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("lib.rs");
        let _ = std::fs::remove_file(&path);

        // Absent → changed.
        assert!(write_probe_source(&path, "fn a() {}").unwrap());
        // Identical → unchanged, and the file is left alone (mtime intact,
        // which is what keeps cargo's cache hit legitimate).
        let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert!(!write_probe_source(&path, "fn a() {}").unwrap());
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), mtime);
        // Different → changed, and the new bytes are on disk.
        assert!(write_probe_source(&path, "fn b() {}").unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "fn b() {}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn probe_id_round_trips_and_absence_is_not_agreement() {
        let stem = "0123456789abcdef";
        let out = format!(
            "{}\n{{\"size\":10,\"buggy_ns\":5,\"fixed_ns\":3}}",
            probe_id_line(stem)
        );
        assert_eq!(parse_probe_id(&out).as_deref(), Some(stem));
        // A binary built before the echo existed, or someone else's binary,
        // yields None — which the caller must not read as a match.
        assert_eq!(
            parse_probe_id(r#"{"size":10,"buggy_ns":5,"fixed_ns":3}"#),
            None
        );
    }
}
