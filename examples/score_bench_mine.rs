//! clippy-bench fixture mining — Plan 014 Phase 1.6.
//!
//! Extracts REAL code spans from a sibling repo's PRE-SWEEP commit, labels
//! them with real `cargo clippy` in a bare stub crate (the T3.5.5 oracle —
//! the same `wrap()` the bench uses), and emits ready-to-paste `RawFixture`
//! entries with provenance.
//!
//! The oracle's `compiles` flag IS the std-only filter: spans referencing
//! crate-internal types fail to compile in the stub and are dropped by
//! construction (Phase 3 adds per-fixture stub manifests).
//!
//! Run (from the riir-clippy repo root):
//!
//! ```text
//! cargo run --example score_bench_mine --features score_bench -- \
//!     ../katgpt-rs <sha> crates/katgpt-pruners/src/decision_explainer.rs
//! ```
//!
//! Curation discipline (Plan 014): keep a DIVERSE set (≥6 distinct lint
//! classes, ≤4 fixtures per lint per source) — the bench measures healing
//! competence, not one lint's frequency.

use std::collections::BTreeSet;
use std::process::Command;

use syn::spanned::Spanned;

use riir_clippy::score_bench::{vocabulary, wrap};

fn main() {
    let mut args = std::env::args().skip(1);
    // Plan 014 freeze mode: oracle-label the T3.5.5 composite spans + emit
    // RawFixture entries (provenance authored-composite — the realistic
    // multi-lint set, NOT the degenerate minimal one).
    if args.next().as_deref() == Some("--freeze-composite") {
        freeze_composite();
        return;
    }
    let mut args = std::env::args().skip(1);
    let (Some(repo), Some(sha)) = (args.next(), args.next()) else {
        eprintln!("usage: score_bench_mine <repo> <sha> <path...> | --freeze-composite");
        std::process::exit(2);
    };
    let paths: Vec<String> = args.collect();
    if paths.is_empty() {
        eprintln!("usage: score_bench_mine <repo> <sha> <path...>");
        std::process::exit(2);
    }

    let oracle = riir_clippy::draft::ClippyOracle::new()
        .with_work_root(std::env::temp_dir().join("riir_clippy_score_bench"));
    if !oracle.is_available() {
        eprintln!("cargo not on PATH — the oracle needs real clippy");
        std::process::exit(1);
    }
    let vocab = vocabulary();
    let repo_name = repo.trim_end_matches('/').rsplit('/').next().unwrap_or(&repo).to_string();

    for path in paths {
        let show = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["show", &format!("{sha}:{path}")])
            .output()
            .unwrap_or_else(|e| {
                eprintln!("git spawn failed: {e}");
                std::process::exit(1);
            });
        if !show.status.success() {
            eprintln!("git show {sha}:{path} failed — skipping {path}");
            continue;
        }
        let src = String::from_utf8_lossy(&show.stdout).into_owned();
        let Ok(file) = syn::parse_file(&src) else {
            eprintln!("{path}: parse failed — skipping");
            continue;
        };
        let stem = path.rsplit('/').next().unwrap_or(&path).trim_end_matches(".rs");
        let mut idx = 0usize;
        // Top-level fns only: impl blocks carry self-types/trait bounds that
        // rarely compile standalone — the oracle filter drops most anyway.
        for item in &file.items {
            let syn::Item::Fn(f) = item else { continue };
            // Skip trivial one-liners (no body substance to heal).
            if f.block.stmts.is_empty() {
                continue;
            }
            let range = item.span().byte_range();
            let Some(text) = src.get(range.clone()) else { continue };
            let text = text.to_string();
            idx += 1;
            let verdict = oracle.lints_for(&wrap(&text));
            if !verdict.ran || !verdict.compiles {
                continue; // std-only filter: crate-internal spans drop here
            }
            let hits: BTreeSet<String> = verdict.lints.intersection(&vocab).cloned().collect();
            emit_entry(&repo_name, &sha, stem, idx, &text, &hits);
        }
        eprintln!("# {path}: {idx} candidate fn span(s) evaluated");
    }
}

/// Oracle-label the composite spans at freeze time + emit RawFixture
/// entries (stdout) for pasting into fixtures.rs.
fn freeze_composite() {
    use riir_clippy::draft::eval_fixtures_composite::COMPOSITE_SPANS;
    let oracle = riir_clippy::draft::ClippyOracle::new()
        .with_work_root(std::env::temp_dir().join("riir_clippy_score_bench"));
    if !oracle.is_available() {
        eprintln!("cargo not on PATH");
        std::process::exit(1);
    }
    let vocab = vocabulary();
    for (tag, code) in COMPOSITE_SPANS {
        let v = oracle.lints_for(&wrap(code));
        if !v.ran || !v.compiles {
            eprintln!("// SKIP {tag}: does not compile standalone");
            continue;
        }
        let hits: Vec<String> = v
            .lints
            .intersection(&vocab)
            .map(|s| format!("{s:?}"))
            .collect();
        if hits.is_empty() {
            eprintln!("// SKIP {tag}: no vocabulary lints (clean candidate — use as decline fixture)");
            continue;
        }
        let code_lit = rust_string_literal(code);
        eprintln!(
            "    RawFixture {{\n        id: \"composite-{tag}\",\n        provenance: \"authored-composite:T3.5.5:{tag}\",\n        code: {code_lit},\n        expect_lints: &[{}],\n        clean: false,\n    }},\n",
            hits.join(", ")
        );
    }
}

/// Emit one `RawFixture` entry as Rust source (paste into fixtures.rs).
fn emit_entry(
    repo: &str,
    sha: &str,
    stem: &str,
    idx: usize,
    code: &str,
    hits: &BTreeSet<String>,
) {
    let clean = hits.is_empty();
    let emit_clean = std::env::var_os("RIIR_MINE_EMIT_CLEAN").is_some();
    if !clean {
        // Only emit interesting candidates: ≥1 vocabulary lint.
        let lints: Vec<String> = hits.iter().map(|s| format!("{s:?}")).collect();
        let code_lit = rust_string_literal(code);
        eprintln!(
            "    RawFixture {{\n        id: \"{repo}-{stem}-{idx:03}\",\n        provenance: \"{repo}@{sha}:{stem}.rs\",\n        code: {code_lit},\n        expect_lints: &[{}],\n        clean: false,\n    }},\n",
            lints.join(", ")
        );
    } else if emit_clean {
        // Decline-axis candidates: real code with ZERO vocabulary lints —
        // correct behavior is proposing nothing.
        let code_lit = rust_string_literal(code);
        eprintln!(
            "    RawFixture {{\n        id: \"{repo}-{stem}-{idx:03}-clean\",\n        provenance: \"{repo}@{sha}:{stem}.rs\",\n        code: {code_lit},\n        expect_lints: &[],\n        clean: true,\n    }},\n"
        );
    } else {
        // Clean candidates print compactly (one line) for curation picks.
        let first_line = code.lines().find(|l| l.trim().len() > 8).unwrap_or("…");
        eprintln!(
            "    // CLEAN candidate {repo}-{stem}-{idx:03}: {}",
            first_line.trim()
        );
    }
}

/// Render source as a Rust string literal (raw string with enough `#`s).
fn rust_string_literal(code: &str) -> String {
    let mut hashes = 1;
    while code.contains(&format!("\"{}", "#".repeat(hashes))) {
        hashes += 1;
    }
    let h = "#".repeat(hashes);
    format!("r{h}\"{code}\"{h}")
}
