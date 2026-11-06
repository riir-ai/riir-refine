//! The corpus-gap lane's gates, katgpt-rs Issue 983 style: the stamp
//! refuses the flattering-zero directions, the partition never pools, the
//! emission is idempotent per epoch and moves with it, and the persisted
//! canonical artifact verifies itself before any row references it.

use std::collections::BTreeSet;

use super::{
    canonical_path, classify, emit_events, events_dir, misses_section, stamp_clippy, GapLaneError,
    GapPartition, SECTION,
};
use crate::corpus_coverage::{DefaultLevel, LintRow, LintSurface, MIN_DERIVED_LINTS};
use katgpt_core::coverage_manifest::{CoverageManifest, SectionInput, StampPolicy};

/// A synthetic surface above the Issue-119 floor (300): `lint_000` …
/// `lint_{n-1}`. The floor is a product constant — the test generates past
/// it rather than weakening the gate.
fn synthetic_surface(n: usize) -> LintSurface {
    LintSurface {
        lints: (0..n)
            .map(|i| LintRow {
                name: format!("lint_{i:03}"),
                group: "style".to_string(),
                level: DefaultLevel::Warn,
            })
            .collect(),
        ..LintSurface::default()
    }
}

fn corpus_of(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

/// Temp roots are pid+line-namespaced (the `shared_temp_path` law: a fixed
/// temp path is safe against sibling tests in one binary and NOT against
/// another PROCESS running the same test).
fn tmp_root(line: u32) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "riir_refine_corpus_gap_{}_{}",
        std::process::id(),
        line
    ))
}

#[test]
fn stamp_refuses_blind_and_floor_breaches() {
    // Blind: the surface is UNKNOWN, never empty.
    let mut blind = synthetic_surface(320);
    blind.blind = Some("driver not installed".to_string());
    match stamp_clippy(&blind, &corpus_of(&["lint_000"])) {
        Err(GapLaneError::BlindSurface(why)) => assert_eq!(why, "driver not installed"),
        Err(other) => panic!("expected the blind refusal, got: {other}"),
        Ok(_) => panic!("a blind surface stamped"),
    }

    // Floor: a tiny surface is a format regression, not a smaller clippy —
    // stamping it would read every gap as total (the flattering zero).
    let tiny = synthetic_surface(10);
    match stamp_clippy(&tiny, &corpus_of(&["lint_000"])) {
        Err(GapLaneError::SurfaceFloor { derived, floor }) => {
            assert_eq!(derived, 10);
            assert_eq!(floor, MIN_DERIVED_LINTS);
        }
        Err(other) => panic!("expected the floor refusal, got: {other}"),
        Ok(_) => panic!("a 10-lint surface stamped"),
    }
}

#[test]
fn stamp_partitions_loud_and_pins_epoch_identity() {
    let surface = synthetic_surface(320);
    let corpus = corpus_of(&["lint_000", "lint_001", "lint_002", "stale_renamed_lint"]);
    let out = stamp_clippy(&surface, &corpus).expect("stamp");

    // The stale corpus key is REPORTED, never silently folded.
    assert_eq!(out.stale_covered, vec!["stale_renamed_lint".to_string()]);

    let row = out.manifest.row(SECTION).expect("section stamped");
    assert_eq!(row.census_count, 320);
    assert_eq!(row.covered_count, 3);
    assert_eq!(row.gap_count, 317);
    // Retention policy: the gap keys the classify half reads are present.
    let gaps = row.gap_keys.as_ref().expect("retention");
    assert!(gaps.contains(&"lint_100".to_string()));
    assert!(!gaps.contains(&"lint_000".to_string()));

    // Determinism: identical inputs stamp byte-identical epochs (the F2/G1
    // pin rides the substrate; assert the identity the rows embed).
    let again = stamp_clippy(&surface, &corpus).expect("stamp");
    assert_eq!(
        again.manifest.manifest_digest(),
        out.manifest.manifest_digest()
    );
    // And a corpus that moves is a DIFFERENT epoch.
    let moved = stamp_clippy(&surface, &corpus_of(&["lint_000", "lint_001"])).expect("stamp");
    assert_ne!(
        moved.manifest.manifest_digest(),
        out.manifest.manifest_digest()
    );
}

#[test]
fn classify_splits_freshness_gap_unknown_never_pooled() {
    let surface = synthetic_surface(320);
    let corpus = corpus_of(&["lint_000", "lint_001"]);
    let manifest = stamp_clippy(&surface, &corpus).expect("stamp").manifest;

    let per_span = [
        vec!["lint_000".to_string(), "lint_000".to_string()], // freshness, 2 spans
        vec!["clippy::lint-100".to_string()],                 // gap — raw feed spelling normalizes
        vec!["clippy::lint-101".to_string()],                 // gap
        vec!["some_rustc_lint".to_string()],                  // unknown — out of scope
    ];
    let p = classify(&manifest, &corpus, per_span.iter().map(Vec::as_slice)).expect("classify");

    assert_eq!(p.freshness, vec![("lint_000".to_string(), 2)]);
    assert_eq!(
        p.gaps,
        vec![("lint_100".to_string(), 1), ("lint_101".to_string(), 1),]
    );
    assert_eq!(p.unknown, vec![("some_rustc_lint".to_string(), 1)]);
}

#[test]
fn classify_refuses_a_compact_manifest_loud() {
    // The compact posture drops the gap keys the classify half reads —
    // stamping one directly must refuse, never read as "everything unknown".
    let source: Vec<String> = (0..320).map(|i| format!("lint_{i:03}")).collect();
    let source_refs: Vec<&str> = source.iter().map(String::as_str).collect();
    let covered_refs: Vec<&str> = vec!["lint_000"];
    let manifest = CoverageManifest::stamp(
        &[SectionInput {
            section_id: SECTION,
            source_keys: &source_refs,
            covered_keys: &covered_refs,
        }],
        StampPolicy {
            retain_gap_keys: false,
        },
        &|_| true,
    )
    .expect("compact stamp");
    let err = classify(
        &manifest,
        &corpus_of(&["lint_007"]),
        std::iter::empty::<&[String]>(),
    )
    .expect_err("refuse");
    assert_eq!(err, GapLaneError::CompactManifest);
}

#[test]
fn emission_is_idempotent_per_epoch_and_moves_with_the_corpus() {
    let root = tmp_root(line!());
    let _ = std::fs::remove_dir_all(&root);

    let surface = synthetic_surface(320);
    let corpus = corpus_of(&["lint_000", "lint_001"]);
    let manifest = stamp_clippy(&surface, &corpus).expect("stamp").manifest;
    let anchors = [vec!["lint_100".to_string()], vec!["lint_101".to_string()]];
    let partition =
        classify(&manifest, &corpus, anchors.iter().map(Vec::as_slice)).expect("classify");
    assert_eq!(partition.gaps.len(), 2);

    let first = emit_events(&root, &partition, &manifest).expect("emit");
    assert_eq!(first.written.len(), 2);
    assert_eq!(first.skipped_existing, 0);

    // Same epoch, second run: nothing new queues.
    let second = emit_events(&root, &partition, &manifest).expect("emit");
    assert_eq!(second.written, Vec::<std::path::PathBuf>::new());
    assert_eq!(second.skipped_existing, partition.gaps.len());
    assert_eq!(second.canonical_digest_hex, first.canonical_digest_hex);

    // The canonical artifact persists and verifies against its own digest.
    let bytes = std::fs::read(canonical_path(&root)).expect("canonical");
    let parsed = CoverageManifest::from_canonical(&bytes).expect("strict parse");
    assert_eq!(parsed.manifest_digest(), manifest.manifest_digest());
    // Tampered bytes refuse loud (the digest cross-check).
    let mut tampered = bytes.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    assert!(CoverageManifest::from_canonical(&tampered).is_err());

    // A corpus that moves: a new epoch, fresh rows alongside the old ones.
    let moved_corpus = corpus_of(&["lint_000"]);
    let moved = stamp_clippy(&surface, &moved_corpus)
        .expect("stamp")
        .manifest;
    let moved_partition =
        classify(&moved, &moved_corpus, anchors.iter().map(Vec::as_slice)).expect("classify");
    let third = emit_events(&root, &moved_partition, &moved).expect("emit");
    assert_eq!(third.written.len(), 2);
    assert_ne!(third.canonical_digest_hex, first.canonical_digest_hex);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn misses_section_renders_newest_first_and_caps() {
    let root = tmp_root(line!());
    let _ = std::fs::remove_dir_all(&root);
    let dir = events_dir(&root);
    std::fs::create_dir_all(&dir).expect("mkdir");

    // Nothing queued → None (the printer skips silently).
    assert!(misses_section(&root, None).is_none());

    // Three rows across two epochs, staggered mtimes so the order is
    // observable.
    let rows = [
        ("lint_a", "aaaa1111", 1_000u64),
        ("lint_b", "aaaa1111", 2_000),
        ("lint_c", "bbbb2222", 3_000),
    ];
    for (lint, epoch, mtime_ms) in rows {
        let path = dir.join(format!("corpus-gap-{lint}-{epoch}.md"));
        std::fs::write(&path, "row").expect("write");
        let st = std::fs::FileTimes::new()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_millis(mtime_ms));
        std::fs::File::options()
            .write(true)
            .open(&path)
            .and_then(|f| f.set_times(st))
            .expect("mtime");
    }

    let full = misses_section(&root, None).expect("section");
    assert!(full.contains("3 queued gap event(s)"));
    assert!(full.contains("2 corpus epoch(s)"));
    // Newest first: lint_c (3000), lint_b (2000), lint_a (1000).
    let c = full.find("| 1 | lint_c |").expect("newest first");
    let b = full.find("| 2 | lint_b |").expect("second");
    let a = full.find("| 3 | lint_a |").expect("third");
    assert!(c < b && b < a);

    let capped = misses_section(&root, Some(2)).expect("section");
    assert!(capped.contains("| 2 | lint_b |"));
    assert!(!capped.contains("| 3 | lint_a |"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn unknown_sections_and_empty_inputs_read_empty_never_panic() {
    let surface = synthetic_surface(320);
    let manifest = stamp_clippy(&surface, &corpus_of(&["lint_000"]))
        .expect("stamp")
        .manifest;

    // No anchors: every bucket empty.
    let p = classify(&manifest, &corpus_of(&["lint_000"]), []).expect("classify");
    assert_eq!(p, GapPartition::default());

    // Emitting an empty partition persists the canonical artifact and
    // writes nothing.
    let root = tmp_root(line!());
    let _ = std::fs::remove_dir_all(&root);
    let out = emit_events(&root, &p, &manifest).expect("emit");
    assert_eq!(out.written, Vec::<std::path::PathBuf>::new());
    assert!(canonical_path(&root).exists());
    let _ = std::fs::remove_dir_all(&root);

    // The row's digest set has no false members (the BTreeSet law).
    let gaps: BTreeSet<&str> = manifest
        .row(SECTION)
        .and_then(|r| r.gap_keys.as_deref())
        .map(|keys| keys.iter().map(String::as_str).collect())
        .unwrap_or_default();
    assert!(!gaps.contains("lint_000"));
}
