//! katgpt-rs Issue 983 T1 — the heal-lane **corpus-gap event**: the first
//! consumer of katgpt-core's `coverage_manifest` substrate.
//!
//! The blog protocol (Issue 983's mechanism section), mapped onto this repo
//! with the T1 pin's inventory/coverage mapping:
//!
//! - **source inventory** — the installed `clippy-driver`'s lint surface
//!   (the Issue-119 derivation; never hand-typed, so the manifest moves as
//!   clippy ships lints);
//! - **covered keys** — the `clippy_lints` corpus `rule_id`s;
//! - **the gap** — surface − covered, computed ONCE per epoch by the
//!   substrate's `stamp` (F2: never a per-query re-enumeration) as counts +
//!   BLAKE3 digests over the sorted key sets, in the versioned canonical
//!   encoding (the pinned row format), persisted digest-cross-checked;
//! - **the event** — a run's `[no corpus rule]` anchors that the compiled
//!   corpus does not cover AND the surface names become content-addressed
//!   intake rows under `.refine/misses/corpus-gap/` (the `--misses` intake
//!   queue the issue names). Set-membership escalate — the coverage-space
//!   complement to the retrieval lane's distance-based ranking, not a
//!   substitute for it (Issue 983's consumer-surfaces note).
//!
//! ## The three-way anchor partition (never pooled)
//!
//! An anchor — a lint the suggest lane could not rank this run — lands in
//! exactly one bucket:
//!
//! - **freshness** — covered by the compiled corpus. The Issue-116 seam
//!   speaking: the leased suggest corpus lags the compiled fix lane, and
//!   `--fix` heals these today. NOT an event — a fire here would be the
//!   cries-wolf instrument the corpus-delta module documents.
//! - **gap** — in the manifest's gap set. The real mining intake: the event
//!   row.
//! - **unknown** — not in the corpus, not in the gap set (a rustc lint, a
//!   tool lint, a renamed key). Out of the protocol's scope: counted loud,
//!   never bucketed into a neighbour (the corpus-coverage dedup law).
//!
//! ## Honest posture
//!
//! Report-only first: the lane writes only the canonical manifest + the
//! intake rows; it exits unchanged and never gates a fix (the
//! frontier-report law). A blind surface derivation or a floor breach
//! refuses loud — a `-W help` format regression must never make every gap
//! read total (the flattering-zero direction), so the floors are the
//! Issue-119 constants. Idempotent per epoch: a row's filename embeds the
//! manifest digest, so re-running against the same corpus epoch queues
//! nothing new, and a corpus that moves starts a fresh epoch's rows.
//!
//! The manifest is stamped with gap-key RETENTION (the default policy): the
//! classify half reads gap-key membership, which the compact posture drops.
//! [`classify`] refuses a compact manifest loud rather than reading every
//! uncovered anchor as unknown — the silent-wrong direction.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use katgpt_core::coverage_manifest::{CoverageManifest, SectionInput, StampPolicy, StampRefusal};

use crate::corpus_coverage::{self as cc, LintSurface};

/// The one section this lane stamps. The `--misses` per-domain queue names
/// the same domain `clippy` — one vocabulary, two readers.
pub const SECTION: &str = "clippy";

/// Where the canonical manifest persists. The strict parser
/// ([`CoverageManifest::from_canonical`]) cross-checks the digest on every
/// read, so the persisted artifact is F4-verifiable at any later run.
#[must_use]
pub fn canonical_path(root: &Path) -> PathBuf {
    root.join(".refine")
        .join("coverage_manifest")
        .join("clippy.canonical")
}

/// Where the intake rows land — the `--misses` queue the issue names.
#[must_use]
pub fn events_dir(root: &Path) -> PathBuf {
    root.join(".refine").join("misses").join("corpus-gap")
}

/// Loud refusals. A blind derivation, a floor breach, or a compact manifest
/// is never a confident answer (the corpus-coverage law).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GapLaneError {
    /// The surface derivation could not be read (`LintSurface::blind`) —
    /// the lint surface is UNKNOWN, not empty.
    BlindSurface(String),
    /// The derived surface is below the Issue-119 floor — a `-W help`
    /// format regression, not a smaller clippy.
    SurfaceFloor { derived: usize, floor: usize },
    /// The manifest was stamped with the compact policy, which drops the
    /// gap keys the classify half reads. The lane stamps with retention.
    CompactManifest,
    /// The substrate refused the stamp (duplicate / covered / dangling law).
    Stamp(StampRefusal),
    /// The canonical persist or the row write failed.
    Io(String),
}

impl std::fmt::Display for GapLaneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlindSurface(why) => write!(f, "lint surface is blind: {why}"),
            Self::SurfaceFloor { derived, floor } => write!(
                f,
                "derived surface has {derived} lints, below the {floor}-lint floor \
                 — a `-W help` format regression, not a smaller clippy"
            ),
            Self::CompactManifest => write!(
                f,
                "the manifest was stamped with the compact policy (no gap keys) — \
                 the classify half needs retention; re-stamp with StampPolicy::default()"
            ),
            Self::Stamp(r) => write!(f, "stamp refused: {r}"),
            Self::Io(why) => write!(f, "I/O: {why}"),
        }
    }
}

impl std::error::Error for GapLaneError {}

/// The stamp result: the manifest plus the stale-corpus report.
#[derive(Debug)]
pub struct StampOutcome {
    pub manifest: CoverageManifest,
    /// Corpus `rule_id`s the derived surface does not name (renamed or
    /// removed lints — the stale-coverage class the coverage report tracks
    /// as its own bucket). Pre-filtered out of `covered` so the stamp can
    /// proceed; the caller reports them loud (capped), never silently — a
    /// stale corpus key is a real finding about the corpus, not noise.
    pub stale_covered: Vec<String>,
}

/// Stamp the clippy section: the installed driver's surface against the
/// compiled corpus, once per epoch (F2), with gap-key retention.
///
/// # Errors
///
/// A blind surface, a floor breach, or a substrate stamp refusal (duplicate
/// keys, a covered key outside the source, a dangling gap key) — all loud,
/// all caller-reported, none clamped.
pub fn stamp_clippy(
    surface: &LintSurface,
    corpus_rules: &[String],
) -> Result<StampOutcome, GapLaneError> {
    if let Some(why) = &surface.blind {
        return Err(GapLaneError::BlindSurface(why.clone()));
    }
    if surface.lints.len() < cc::MIN_DERIVED_LINTS {
        return Err(GapLaneError::SurfaceFloor {
            derived: surface.lints.len(),
            floor: cc::MIN_DERIVED_LINTS,
        });
    }

    // Names are compared after `normalize` so a kebab-spelled surface row
    // and a snake-spelled corpus `rule_id` meet on one spelling (the
    // corpus_delta / corpus_coverage comparison law — one normalize, always
    // theirs).
    let source: Vec<String> = surface.lints.iter().map(|r| r.name.clone()).collect();
    let source_set: BTreeSet<&str> = source.iter().map(String::as_str).collect();
    let mut covered_names: Vec<String> = Vec::with_capacity(corpus_rules.len());
    let mut stale_covered: Vec<String> = Vec::new();
    for rule in corpus_rules {
        let name = cc::normalize(rule);
        if source_set.contains(name.as_str()) {
            covered_names.push(name);
        } else {
            stale_covered.push(name);
        }
    }
    stale_covered.sort();

    let source_refs: Vec<&str> = source.iter().map(String::as_str).collect();
    let covered_refs: Vec<&str> = covered_names.iter().map(String::as_str).collect();

    // F1's seam: the targeted lookup for a gap key is "query the corpus by
    // rule_id" — executable for every well-formed lint key (a normalized
    // lint name); a key outside that shape is a builder bug and refuses
    // loud, which is the safe direction.
    let lookup_executable = |key: &str| {
        !key.is_empty()
            && key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    };

    let manifest = CoverageManifest::stamp(
        &[SectionInput {
            section_id: SECTION,
            source_keys: &source_refs,
            covered_keys: &covered_refs,
        }],
        StampPolicy::default(),
        &lookup_executable,
    )
    .map_err(GapLaneError::Stamp)?;

    Ok(StampOutcome {
        manifest,
        stale_covered,
    })
}

/// The three-way anchor partition — never pooled (the module doc carries
/// the bucket meanings).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GapPartition {
    /// Covered by the compiled corpus — the Issue-116 freshness seam; the
    /// consumer does NOT queue these. `(lint, spans this run)`, hottest
    /// first.
    pub freshness: Vec<(String, usize)>,
    /// In the manifest's gap set — the real mining intake. Same shape.
    pub gaps: Vec<(String, usize)>,
    /// Out of the protocol's scope — counted loud, never bucketed.
    pub unknown: Vec<(String, usize)>,
}

/// Partition a run's `[no corpus rule]` anchors against the stamped
/// manifest and the compiled corpus. Names normalize on the way in (raw
/// feed spellings — `clippy::needless-return` — meet the corpus spelling).
///
/// Covered membership reads the CALLER's corpus list (the manifest carries
/// counts + digests, not the covered key set — the caller holds the
/// inventory it just stamped); gap membership reads the manifest's retained
/// gap keys. A compact manifest refuses loud
/// ([`GapLaneError::CompactManifest`]) rather than reading every uncovered
/// anchor as unknown.
///
/// # Errors
///
/// [`GapLaneError::CompactManifest`] when the manifest dropped its gap keys.
pub fn classify<'a>(
    manifest: &CoverageManifest,
    corpus_rules: &[String],
    per_span_missing: impl IntoIterator<Item = &'a [String]>,
) -> Result<GapPartition, GapLaneError> {
    let Some(row) = manifest.row(SECTION) else {
        return Ok(GapPartition::default());
    };
    let Some(gap_keys) = row.gap_keys.as_deref() else {
        return Err(GapLaneError::CompactManifest);
    };

    let covered: BTreeSet<String> = corpus_rules.iter().map(|r| cc::normalize(r)).collect();
    let gap_set: BTreeSet<&str> = gap_keys.iter().map(String::as_str).collect();

    let mut spans_of: BTreeMap<String, usize> = BTreeMap::new();
    for missing in per_span_missing {
        for lint in missing {
            *spans_of.entry(cc::normalize(lint)).or_default() += 1;
        }
    }

    let mut out = GapPartition::default();
    for (lint, spans) in spans_of {
        if covered.contains(&lint) {
            out.freshness.push((lint, spans));
        } else if gap_set.contains(lint.as_str()) {
            out.gaps.push((lint, spans));
        } else {
            out.unknown.push((lint, spans));
        }
    }
    // Hottest first, lint as the tiebreak (the corpus_delta row order).
    out.freshness
        .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    out.gaps
        .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    out.unknown.sort();
    Ok(out)
}

/// The emission result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmissionOutcome {
    /// Intake rows written this run (one per NEW `(lint, epoch)` pair).
    pub written: Vec<PathBuf>,
    /// Rows already queued for this epoch — the idempotent-per-epoch law.
    pub skipped_existing: usize,
    /// The persisted manifest's digest, hex (the epoch identity).
    pub canonical_digest_hex: String,
}

/// Persist the canonical encoding (deterministic overwrite — same epoch =
/// same bytes), verify the persisted artifact strict-parses with its digest
/// intact (F4: verify BEFORE any row references it), then write one
/// content-addressed intake row per gap anchor. Row filename
/// `corpus-gap-{lint}-{epoch8}.md`: re-running against the same epoch
/// queues nothing new; a corpus that moves starts a fresh epoch's rows.
///
/// # Errors
///
/// [`GapLaneError::Io`] on any filesystem failure, or when the persisted
/// canonical bytes fail their own strict re-parse (a write that cannot
/// verify itself never names itself verified).
pub fn emit_events(
    root: &Path,
    partition: &GapPartition,
    manifest: &CoverageManifest,
) -> Result<EmissionOutcome, GapLaneError> {
    let canonical = canonical_path(root);
    if let Some(parent) = canonical.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| GapLaneError::Io(format!("{}: {e}", parent.display())))?;
    }
    let bytes = manifest.canonical_bytes();
    std::fs::write(&canonical, &bytes)
        .map_err(|e| GapLaneError::Io(format!("{}: {e}", canonical.display())))?;
    let re_read = std::fs::read(&canonical)
        .map_err(|e| GapLaneError::Io(format!("{}: {e}", canonical.display())))?;
    let verified = CoverageManifest::from_canonical(&re_read).map_err(|e| {
        GapLaneError::Io(format!("persisted canonical failed its own re-parse: {e}"))
    })?;
    debug_assert_eq!(verified.manifest_digest(), manifest.manifest_digest());

    let dir = events_dir(root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| GapLaneError::Io(format!("{}: {e}", dir.display())))?;

    let digest_hex = digest_hex(&manifest.manifest_digest());
    let epoch8 = digest_hex.get(..8).unwrap_or(&digest_hex).to_string();
    let row = manifest.row(SECTION);
    let (covered, census, gap_count) = row.map_or((0, 0, 0), |r| {
        (r.covered_count, r.census_count, r.gap_count)
    });

    let mut written = Vec::new();
    let mut skipped_existing = 0usize;
    for (lint, spans) in &partition.gaps {
        let path = dir.join(format!("corpus-gap-{lint}-{epoch8}.md"));
        if path.exists() {
            skipped_existing += 1;
            continue;
        }
        let body = format!(
            "# corpus-gap: {lint} — no corpus rule covers it\n\n\
             - **section:** {SECTION} — coverage {covered}/{census} ({gap_count} gap keys at stamp time)\n\
             - **manifest:** {digest_hex}\n\
             - **canonical:** {} (strict-parse verified at write time)\n\
             - **spans this run:** {spans}\n\
             - **captured:** by `cargo refine --corpus-gap` (the katgpt-rs Issue 983 consumer)\n\n\
             A `[no corpus rule]` anchor the compiled `clippy_lints` corpus does not\n\
             cover and the installed driver's surface names — the real mining intake.\n\
             Freshness anchors (corpus-covered) are NOT queued: `--fix` heals those\n\
             today (the Issue-116 seam). Frequency evidence for this lint:\n\
             `cargo refine --coverage-report`. Authoring discipline: dual evidence,\n\
             dedup grep, owner adjudication.\n",
            canonical.display(),
        );
        std::fs::write(&path, body)
            .map_err(|e| GapLaneError::Io(format!("{}: {e}", path.display())))?;
        written.push(path);
    }

    Ok(EmissionOutcome {
        written,
        skipped_existing,
        canonical_digest_hex: digest_hex,
    })
}

/// The `--misses` printer's corpus-gap section: the queued intake rows,
/// newest first, table rows capped at `limit`. `None` when nothing is
/// queued (the expected state — the printer skips silently, the
/// corpus-delta noise law).
#[must_use]
pub fn misses_section(root: &Path, limit: Option<usize>) -> Option<String> {
    let dir = events_dir(root);
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut rows: Vec<(String, String, Option<std::time::SystemTime>)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        // `corpus-gap-{lint}-{epoch8}.md` — the row name IS the data.
        let name = path.file_name()?.to_string_lossy().to_string();
        let stem = name.strip_prefix("corpus-gap-")?.strip_suffix(".md")?;
        let Some((lint, epoch)) = stem.rsplit_once('-') else {
            continue;
        };
        let mtime = entry.metadata().ok().and_then(|m| m.modified().ok());
        rows.push((lint.to_string(), epoch.to_string(), mtime));
    }
    if rows.is_empty() {
        return None;
    }
    rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

    let total = rows.len();
    let epochs: BTreeSet<&str> = rows.iter().map(|(_, e, _)| e.as_str()).collect();
    let mut out = String::with_capacity(64 + rows.len() * 48);
    let _ = write!(
        out,
        "## Corpus-gap intake (katgpt-rs Issue 983 consumer)\n\
         \n\
         {total} queued gap event(s) across {} corpus epoch(s) — {}\n\
         One row per (lint, corpus epoch): re-running against the same epoch queues nothing.\n\
         \n\
         | rank | lint | epoch |\n|---|---|---|\n",
        epochs.len(),
        dir.display(),
    );
    for (i, (lint, epoch, _)) in rows.iter().enumerate() {
        if let Some(n) = limit {
            if i >= n {
                break;
            }
        }
        let _ = writeln!(out, "| {} | {lint} | {epoch} |", i + 1);
    }
    Some(out)
}

fn digest_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests;
