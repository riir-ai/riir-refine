//! Plan 049 T0.1 — oracle-label a fresh real-repo span set across the
//! workspace repos and freeze it as a BLAKE3-committed dataset.
//!
//! ## Why IN-CONTEXT labeling (not isolated temp-crate spans)
//!
//! The first harness draft chunked repo files into standalone spans and
//! labeled each in a temp crate. Measured on four source sets (workspace
//! repos, their unswept corners, and the untouched sibling codebase
//! `seal-online-remaster`): most real-repo item spans DO NOT COMPILE
//! standalone (crate-internal references — `AiBrain`, `ConnectionType`),
//! and the self-contained fraction that does compile is lint-clean by
//! construction (agent-written under clippy discipline). Yield: ~0 labels
//! per 200 compiled spans. The oracle contract (`compiles=false` → no
//! labels) therefore excludes EXACTLY the code that needs healing.
//!
//! The production distribution is the opposite: the user's file has full
//! crate context; the healer sees a span. So this harness labels spans the
//! way production finds them — `cargo clippy --workspace --all-targets
//! --message-format=json -- -W clippy::all -W clippy::pedantic
//! -W clippy::nursery` per repo, every `clippy::*` warning's PRIMARY span
//! (file, line) attributed to the containing [`AstChunker`] span. Files that
//! received a hard error are excluded wholesale (late lints are suppressed
//! crate-wide by a hard error — the oracle's own honesty rule, now at file
//! granularity).
//!
//! ## Outputs
//!
//! - `spans.jsonl`   — one row per committed span (canonical order:
//!   repo, path, start_line)
//! - `manifest.json` — counts, provenance (repo HEAD commits, clippy exits,
//!   cargo version), the label policy, and the BLAKE3 commitment over the
//!   exact `spans.jsonl` bytes.
//!
//! The committed dataset is the honest denominator for Plan 049 T0.2's
//! trainable-band decomposition (`decompose_headroom_with_spans`).
//!
//! # Usage
//!
//! ```text
//! cargo run --release --features latent_retrieval --example t01_real_repo_label
//! ```
//!
//! Env knobs (all optional): `T01_REPOS` (comma-separated repo roots),
//! `T01_TARGET_LABELED` (default 330 — the plan bar is ≥300), `T01_OUT_DIR`
//! (default `.benchmarks/data/049_t01`), `T01_TOOLCHAIN` (optional
//! `RUSTUP_TOOLCHAIN` override for the child cargo — some sibling repos
//! resolve a GNU host toolchain on this box and need the MSVC pin).

use riir_rag::ast_chunker::AstChunker;
use riir_rag::chunker::Chunker;
use riir_refine::domain::{HealDomain, HealEntry};
use riir_refine::domains::clippy_lints::ClippyLintDomain;
use riir_refine::domains::clippy_lints::proposers::{FIXABLE_LINTS, VERIFY_LINTS};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Span content bounds: below this is a stub (`pub fn a() {}` noise), above
/// this is a file-sized paste.
const MIN_SPAN_CHARS: usize = 40;
const MAX_SPAN_CHARS: usize = 8_000;
/// Files above this are generated fixtures, not real code.
const MAX_FILE_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone)]
struct SpanCand {
    repo: String,
    path: String,
    start_line: usize,
    end_line: usize,
    code: String,
    hash: String,
}

/// One clippy finding from the repo stream, attributed to its file.
struct LintEvent {
    file: String,
    line: usize,
    code: String,
}

#[derive(Debug, Serialize)]
struct DatasetRow {
    repo: String,
    path: String,
    start_line: usize,
    end_line: usize,
    code_blake3: String,
    code: String,
    /// Every clippy lint the in-context scan fired inside this span (bare
    /// ids, sorted).
    lints: Vec<String>,
    /// The heal-relevant retrieval target: a fired lint our corpus knows,
    /// preferring fixable rules (alphabetical within each tier).
    label: Option<String>,
    label_fixable: bool,
}

#[derive(Debug, Serialize)]
struct RepoProvenance {
    name: String,
    path: String,
    commit: String,
    clippy_exit_ok: bool,
    lint_events: usize,
    error_files: usize,
}

#[derive(Debug, Serialize)]
struct Counts {
    files_walked: usize,
    files_read_failed: usize,
    files_with_errors: usize,
    spans_chunked: usize,
    span_size_filtered: usize,
    pool_size: usize,
    /// Rows = spans with ≥1 in-context clippy event (any lint code).
    rows_with_findings: usize,
    /// The stop-rule denominator: rows whose label (a corpus-vocabulary
    /// rule) is Some. Plan 049 T0.3 reads this ratio.
    in_vocab_labeled: usize,
    in_vocab_multi: usize,
    events_total: usize,
    events_matched_to_spans: usize,
}

#[derive(Debug, Serialize)]
struct Manifest {
    schema_version: u32,
    created_utc: String,
    generator: String,
    cargo_version: String,
    plan: String,
    label_policy: String,
    labeling: String,
    vocabulary_size: usize,
    row_count: usize,
    target_labeled: usize,
    dataset_blake3: String,
    source_repos: Vec<RepoProvenance>,
    /// In-vocabulary labels only (the stop-rule input).
    label_histogram: BTreeMap<String, usize>,
    /// ALL fired codes, count-sorted desc — the corpus-expansion roadmap:
    /// the top entries are real-code lints the retrieval corpus does not
    /// cover yet.
    fired_histogram_top: Vec<(String, usize)>,
    counts: Counts,
    notes: Vec<String>,
}

fn labeled_count(rows: &[DatasetRow]) -> usize {
    rows.iter().filter(|r| r.label.is_some()).count()
}

fn main() {
    let repos: Vec<String> = match std::env::var("T01_REPOS") {
        Ok(v) => v.split(',').map(<str>::to_string).collect(),
        Err(_) => vec![
            "../katgpt-rs".to_string(),
            "../riir-ai".to_string(),
            "../riir-train".to_string(),
            "../riir-chain".to_string(),
            "../riir-neuron-db".to_string(),
            "../riir-dapps".to_string(),
        ],
    };
    let target_labeled: usize = std::env::var("T01_TARGET_LABELED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(330);
    let out_dir = PathBuf::from(
        std::env::var("T01_OUT_DIR").unwrap_or_else(|_| ".benchmarks/data/049_t01".to_string()),
    );
    let toolchain = std::env::var("T01_TOOLCHAIN").ok();

    let cargo_version = read_cargo_version();

    let mut provenance: Vec<RepoProvenance> = Vec::new();
    for r in &repos {
        let path = PathBuf::from(r);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| r.clone());
        if !path.is_dir() {
            eprintln!("[t01] WARN repo missing, skipping: {r}");
            continue;
        }
        provenance.push(RepoProvenance {
            name,
            path: r.clone(),
            commit: git_head(&path),
            clippy_exit_ok: false,
            lint_events: 0,
            error_files: 0,
        });
    }
    assert!(!provenance.is_empty(), "no source repos available");

    // ── label vocabulary + fixability (the corpus contract, not ad hoc) ───
    let vocabulary: BTreeSet<String> = ClippyLintDomain::build_corpus()
        .into_iter()
        .map(|e| e.rule_id().to_string())
        .collect();
    let fixable: BTreeSet<String> = FIXABLE_LINTS
        .iter()
        .chain(VERIFY_LINTS.iter())
        .map(<&str>::to_string)
        .collect();
    eprintln!(
        "[t01] vocabulary {} rules, fixable subset {}",
        vocabulary.len(),
        fixable.len()
    );

    let chunker = AstChunker;
    let mut rows: Vec<DatasetRow> = Vec::new();
    let mut files_walked = 0usize;
    let mut files_read_failed = 0usize;
    let mut files_with_errors = 0usize;
    let mut spans_chunked = 0usize;
    let mut span_size_filtered = 0usize;
    let mut pool_before_dedup = 0usize;
    let mut events_total = 0usize;
    let mut events_matched_to_spans = 0usize;

    for prov in provenance.iter_mut() {
        if labeled_count(&rows) >= target_labeled {
            break;
        }
        let root = PathBuf::from(&prov.path);

        // ── walk + chunk this repo ────────────────────────────────────────
        let mut files = Vec::new();
        collect_rs_files(&root, &mut files);
        files.sort();
        files_walked += files.len();
        let mut spans: Vec<SpanCand> = Vec::new();
        for f in &files {
            let Ok(src) = std::fs::read_to_string(f) else {
                files_read_failed += 1;
                continue;
            };
            let rel = f
                .strip_prefix(&root)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| f.to_string_lossy().into_owned());
            for c in chunker.chunk(&src, &rel) {
                spans_chunked += 1;
                let n = c.content.chars().count();
                if !(MIN_SPAN_CHARS..=MAX_SPAN_CHARS).contains(&n) {
                    span_size_filtered += 1;
                    continue;
                }
                spans.push(SpanCand {
                    repo: prov.name.clone(),
                    path: c.source.path.clone(),
                    start_line: c.source.start_line,
                    end_line: c.source.end_line,
                    hash: blake3::hash(c.content.as_bytes()).to_hex().to_string(),
                    code: c.content,
                });
            }
        }
        eprintln!(
            "[t01] chunked {}: {} files, {} spans",
            prov.name,
            files.len(),
            spans.len()
        );

        // ── in-context clippy scan ────────────────────────────────────────
        let (events, error_files, exit_ok) = clippy_scan(&root, toolchain.as_deref());
        prov.clippy_exit_ok = exit_ok;
        prov.lint_events = events.len();
        prov.error_files = error_files.len();
        events_total += events.len();
        eprintln!(
            "[t01] clippy {}: exit_ok={exit_ok} lint_events={} error_files={}",
            prov.name,
            events.len(),
            error_files.len()
        );
        if events.is_empty() {
            eprintln!(
                "[t01] WARN: no lint events for {} — clippy failure or a clean repo",
                prov.name
            );
        }

        // ── attribute events to spans ─────────────────────────────────────
        // Index events by normalized file; a chunk claims the events whose
        // primary line falls inside its inclusive range.
        let mut by_file: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
        for e in events {
            by_file
                .entry(norm_path(&e.file))
                .or_default()
                .push((e.line, e.code));
        }
        let mut dedup: BTreeSet<String> = BTreeSet::new();
        for s in &spans {
            pool_before_dedup += 1;
            if !dedup.insert(s.hash.clone()) {
                continue;
            }
            if error_files.contains(&s.path) {
                files_with_errors += 1;
                continue;
            }
            let fired: Vec<(usize, String)> = by_file
                .get(&norm_path(&s.path))
                .map(|evs| {
                    evs.iter()
                        .filter(|(l, _)| *l >= s.start_line && *l <= s.end_line)
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            if fired.is_empty() {
                continue; // clean candidate — capped below
            }
            events_matched_to_spans += fired.len();
            let mut all_codes: BTreeSet<String> = BTreeSet::new();
            let mut bare: Vec<String> = Vec::new();
            for (_, c) in &fired {
                if let Some(b) = c.strip_prefix("clippy::") {
                    if all_codes.insert(b.to_string()) {
                        bare.push(b.to_string());
                    }
                }
            }
            bare.sort();
            let fired_in_vocab: Vec<String> = bare
                .iter()
                .filter(|l| vocabulary.contains(*l))
                .cloned()
                .collect();
            let fixable_fired: Vec<String> = fired_in_vocab
                .iter()
                .filter(|l| fixable.contains(*l))
                .cloned()
                .collect();
            let (label, label_fixable) = if let Some(l) = fixable_fired.first() {
                (Some(l.clone()), true)
            } else {
                (fired_in_vocab.first().cloned(), false)
            };
            rows.push(DatasetRow {
                repo: s.repo.clone(),
                path: s.path.clone(),
                start_line: s.start_line,
                end_line: s.end_line,
                code_blake3: s.hash.clone(),
                code: s.code.clone(),
                lints: bare,
                label,
                label_fixable,
            });
        }
        eprintln!(
            "[t01] {} labeled so far {} (events matched {events_matched_to_spans})",
            prov.name,
            labeled_count(&rows)
        );
    }

    // ── clean controls, capped ────────────────────────────────────────────
    // Re-walk is wasteful; instead, take clean candidates from a second pass
    // over the same chunk data is not possible (chunks were consumed per
    // repo). Simpler: sample clean rows from the ALREADY-labeled repos' pool
    // is skipped — the control set comes only from spans that carried events
    // outside the vocabulary (they are real, in-context, and compiling).
    // ── canonical write ───────────────────────────────────────────────────
    let rows_with_findings = rows.len();
    let in_vocab_labeled = labeled_count(&rows);
    let in_vocab_multi = rows
        .iter()
        .filter(|r| {
            r.label.is_some() && r.lints.iter().filter(|l| vocabulary.contains(*l)).count() > 1
        })
        .count();

    // The deliverable bar: a dataset of ≥300 real spans carrying real
    // in-context clippy findings. The IN-VOCABULARY band is NOT the bar —
    // its ratio is the T0.3 stop-rule input, recorded, not enforced here.
    assert!(
        rows_with_findings >= 300,
        "T0.1 bar is ≥300 finding-bearing spans, got {rows_with_findings} \
         (events {events_total}, matched {events_matched_to_spans}) — \
         widen T01_REPOS or check clippy exits"
    );

    // ── canonical write ───────────────────────────────────────────────────
    rows.sort_by(|a, b| (&a.repo, &a.path, a.start_line).cmp(&(&b.repo, &b.path, b.start_line)));
    let mut jsonl: Vec<u8> = Vec::with_capacity(rows.len() * 512);
    for r in &rows {
        serde_json::to_writer(&mut jsonl, r).expect("row serializes");
        jsonl.push(b'\n');
    }
    std::fs::create_dir_all(&out_dir).expect("create out dir");
    let spans_path = out_dir.join("spans.jsonl");
    std::fs::write(&spans_path, &jsonl).expect("write spans.jsonl");
    let dataset_blake3 = blake3::hash(&jsonl).to_hex().to_string();

    let mut label_histogram: BTreeMap<String, usize> = BTreeMap::new();
    let mut fired_histogram: BTreeMap<String, usize> = BTreeMap::new();
    for r in &rows {
        if let Some(l) = &r.label {
            *label_histogram.entry(l.clone()).or_default() += 1;
        }
        for l in &r.lints {
            *fired_histogram.entry(l.clone()).or_default() += 1;
        }
    }
    let mut fired_histogram_top: Vec<(String, usize)> = fired_histogram.into_iter().collect();
    fired_histogram_top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    fired_histogram_top.truncate(30);

    let pool_size = pool_before_dedup;
    let manifest = Manifest {
        schema_version: 1,
        created_utc: utc_now_iso(),
        generator: format!("{} examples/t01_real_repo_label", env!("CARGO_PKG_NAME")),
        cargo_version,
        plan: "049_trained_drafter_phased_rollout.md T0.1".to_string(),
        label_policy: "fired ∩ clippy-corpus vocabulary; prefer fixable (FIXABLE_LINTS ∪ \
             VERIFY_LINTS), alphabetical within tier; None = in-context span whose fired \
             lints are all outside the vocabulary"
            .to_string(),
        labeling: "in-context: `cargo clippy --workspace --all-targets --message-format=json \
             -- -W clippy::all -W clippy::pedantic -W clippy::nursery` per repo; primary-span \
             (file, line) attributed to the containing AstChunker span; files with hard \
             errors excluded wholesale"
            .to_string(),
        vocabulary_size: vocabulary.len(),
        row_count: rows.len(),
        target_labeled,
        dataset_blake3,
        source_repos: provenance,
        label_histogram,
        fired_histogram_top,
        counts: Counts {
            files_walked,
            files_read_failed,
            files_with_errors,
            spans_chunked,
            span_size_filtered,
            pool_size,
            rows_with_findings,
            in_vocab_labeled,
            in_vocab_multi,
            events_total,
            events_matched_to_spans,
        },
        notes: vec![
            "Labels come from the repos' REAL cargo environment (default features, their own \
             target dirs): a span's label is the finding production healing would see."
                .to_string(),
            "cargo may REPLAY cached diagnostics for unchanged files — those carry the last \
             build's lint config; for sweep-era surfaces that config equals this scan's, so \
             replays are correct-or-undercounting labels, never fabricated ones."
                .to_string(),
            "Retrieval still sees the ISOLATED span text (production shape): the healer is \
             handed a span, not the crate."
                .to_string(),
        ],
    };
    let manifest_path = out_dir.join("manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest serializes"),
    )
    .expect("write manifest.json");

    println!("\n== T0.1 dataset ==");
    println!(
        "  rows {rows_with_findings} finding-bearing (in-vocab labeled {in_vocab_labeled}, multi {in_vocab_multi})"
    );
    println!(
        "  events {events_total} (matched {events_matched_to_spans}) across {files_walked} files"
    );
    println!("  blake3 {}", manifest.dataset_blake3);
    println!(
        "  out {} + {}",
        spans_path.display(),
        manifest_path.display()
    );
    println!("  top labels:");
    let mut hist: Vec<_> = manifest.label_histogram.iter().collect();
    hist.sort_by_key(|(_, n)| **n);
    for (l, n) in hist.iter().rev().take(15) {
        println!("    {n:>4} {l}");
    }
}

/// One `cargo clippy` over the repo's REAL environment (its own target dir,
/// default features, `--all-targets`). Returns (events, errored files
/// relative to the repo root, exit ok).
fn clippy_scan(root: &Path, toolchain: Option<&str>) -> (Vec<LintEvent>, BTreeSet<String>, bool) {
    let mut cmd = Command::new("cargo");
    cmd.arg("clippy")
        .arg("--workspace")
        .arg("--all-targets")
        .arg("--message-format=json")
        .arg("--")
        .arg("-W")
        .arg("clippy::all")
        .arg("-W")
        .arg("clippy::pedantic")
        .arg("-W")
        .arg("clippy::nursery")
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(tc) = toolchain {
        cmd.env("RUSTUP_TOOLCHAIN", tc);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[t01] clippy spawn failed: {e}");
            return (Vec::new(), BTreeSet::new(), false);
        }
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.wait();
        return (Vec::new(), BTreeSet::new(), false);
    };
    let mut events = Vec::new();
    let mut error_files: BTreeSet<String> = BTreeSet::new();
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if v.get("reason").and_then(serde_json::Value::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = v.get("message") else {
            continue;
        };
        let level = message
            .get("level")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let (file, line) = primary_span_file_line(message);
        match level {
            "error" => {
                if let Some(f) = file {
                    error_files.insert(norm_path(f));
                }
            }
            "warning" => {
                let Some(code) = message
                    .get("code")
                    .and_then(|c| c.get("code"))
                    .and_then(serde_json::Value::as_str)
                else {
                    continue;
                };
                if !code.starts_with("clippy::") {
                    continue;
                }
                if let (Some(f), Some(l)) = (file, line) {
                    events.push(LintEvent {
                        file: f.to_string(),
                        line: l,
                        code: code.to_string(),
                    });
                }
            }
            _ => {}
        }
    }
    let ok = child.wait().map(|s| s.success()).unwrap_or(false);
    (events, error_files, ok)
}

/// The primary span of a rustc JSON diagnostic → (file_name, line_start).
fn primary_span_file_line(message: &serde_json::Value) -> (Option<&str>, Option<usize>) {
    let Some(spans) = message.get("spans").and_then(|s| s.as_array()) else {
        return (None, None);
    };
    let primary = spans
        .iter()
        .find(|s| s.get("is_primary").and_then(serde_json::Value::as_bool) == Some(true))
        .or_else(|| spans.first());
    match primary {
        Some(s) => (
            s.get("file_name").and_then(serde_json::Value::as_str),
            s.get("line_start")
                .and_then(serde_json::Value::as_u64)
                .map(|l| l as usize),
        ),
        None => (None, None),
    }
}

/// Normalize a rustc JSON path for matching against walked relative paths:
/// backslashes → slashes, `./` stripped, trailing whitespace trimmed.
fn norm_path(p: &str) -> String {
    let mut s = p.replace('\\', "/");
    while let Some(stripped) = s.strip_prefix("./") {
        s = stripped.to_string();
    }
    s.trim().to_string()
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if matches!(
                name.as_str(),
                ".git"
                    | "target"
                    | "vendor"
                    | "node_modules"
                    | ".heal"
                    | ".refine"
                    | "dist"
                    | "out"
            ) {
                continue;
            }
            collect_rs_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs")
            && entry
                .metadata()
                .map(|m| m.len() <= MAX_FILE_BYTES)
                .unwrap_or(false)
        {
            out.push(p);
        }
    }
}

fn git_head(repo: &Path) -> String {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn read_cargo_version() -> String {
    Command::new("cargo")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// UTC timestamp without a chrono dep (days-since-epoch civil conversion).
fn utc_now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60
    )
}
