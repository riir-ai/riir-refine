//! Plan 049 T0.1 dataset integrity — the committed real-repo oracle dataset
//! (`.benchmarks/data/049_t01/`) must match its BLAKE3 commitment, satisfy
//! the schema, and carry the plan-bar row count. Pure file checks: no cargo
//! spawn, CI-safe.
//!
//! The dataset rows are real workspace spans labeled by IN-CONTEXT clippy
//! (the production distribution — see the harness header for why isolated
//! temp-crate labeling measured ~0 and was replaced). The in-vocabulary
//! band (~1%) is the Plan 049 T0.3 stop-rule input, recorded — not asserted
//! high here.

use std::collections::BTreeSet;
use std::path::PathBuf;

const DIR: &str = ".benchmarks/data/049_t01";
/// The plan bar: ≥300 finding-bearing real spans.
const MIN_ROWS: usize = 300;

#[derive(serde::Deserialize)]
struct Row {
    repo: String,
    path: String,
    start_line: usize,
    end_line: usize,
    code_blake3: String,
    code: String,
    lints: Vec<String>,
    label: Option<String>,
    label_fixable: bool,
}

fn dataset_dir() -> PathBuf {
    // Tests run with CWD = crate root.
    PathBuf::from(DIR)
}

#[test]
fn t01_dataset_matches_its_blake3_commitment_and_schema() {
    let dir = dataset_dir();
    let spans_path = dir.join("spans.jsonl");
    let manifest_path = dir.join("manifest.json");
    let bytes = std::fs::read(&spans_path)
        .unwrap_or_else(|e| panic!("dataset missing ({e}) — regenerate via the t01 example"));
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).expect("manifest readable"))
            .expect("manifest parses");

    // 1. The commitment is over the EXACT file bytes.
    let got = blake3::hash(&bytes).to_hex().to_string();
    let want = manifest["dataset_blake3"].as_str().expect("blake3 field");
    assert_eq!(got, want, "dataset bytes do not match the committed BLAKE3");

    // 2. The plan bar.
    let rows: Vec<Row> = bytes
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice(l).expect("row parses"))
        .collect();
    assert!(
        rows.len() >= MIN_ROWS,
        "dataset has {} rows, bar is ≥{MIN_ROWS}",
        rows.len()
    );
    assert_eq!(
        rows.len(),
        manifest["row_count"].as_u64().unwrap_or_default() as usize,
        "row_count drift between manifest and file"
    );

    // 3. Schema invariants per row + duplicate accounting (cross-repo code
    // copies are legitimate in real-repo data; they must stay a small
    // fraction so the denominator is not silently repeated content).
    let mut seen_hashes = BTreeSet::new();
    let mut in_vocab = 0usize;
    let mut duplicates = 0usize;
    for r in &rows {
        assert!(!r.repo.is_empty() && !r.path.is_empty(), "empty provenance");
        assert!(r.start_line >= 1 && r.end_line >= r.start_line, "bad range");
        assert_eq!(
            blake3::hash(r.code.as_bytes()).to_hex().to_string(),
            r.code_blake3,
            "code_blake3 mismatch at {}:{}",
            r.path,
            r.start_line
        );
        assert!(!r.lints.is_empty(), "finding-bearing row with no lints");
        for l in &r.lints {
            assert!(!l.starts_with("clippy::"), "bare ids expected, got {l}");
        }
        if let Some(label) = &r.label {
            in_vocab += 1;
            assert!(
                r.lints.contains(label),
                "label {label} not among the row's fired lints"
            );
        } else {
            assert!(!r.label_fixable, "None label cannot be fixable");
        }
        if !seen_hashes.insert(r.code_blake3.clone()) {
            duplicates += 1;
        }
    }
    assert!(
        duplicates * 20 <= rows.len(),
        "duplicate content {duplicates}/{} exceeds 5% — dedup before committing",
        rows.len()
    );

    // 4. The in-vocab band is RECORDED in the manifest and consistent.
    let recorded = manifest["counts"]["in_vocab_labeled"]
        .as_u64()
        .unwrap_or_default() as usize;
    assert_eq!(in_vocab, recorded, "in_vocab_labeled drift");
}

#[test]
fn t01_manifest_records_the_stop_rule_inputs() {
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dataset_dir().join("manifest.json")).expect("manifest readable"),
    )
    .expect("manifest parses");
    // Provenance: ≥1 repo, every repo has a commit + event count.
    let repos = manifest["source_repos"].as_array().expect("repos");
    assert!(!repos.is_empty(), "no source repos recorded");
    for r in repos {
        assert!(!r["commit"].as_str().unwrap_or_default().is_empty());
        assert!(
            r["lint_events"].as_u64().unwrap_or(0) > 0,
            "a repo scanned but fired nothing — check its clippy exit"
        );
    }
    // The expansion roadmap is present and count-sorted.
    let hist = manifest["fired_histogram_top"]
        .as_array()
        .expect("histogram");
    assert!(hist.len() >= 10, "fired histogram too thin to route on");
}
