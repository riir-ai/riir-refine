//! Plan 356 / riir-train issue 695 step (2) — the extra-spans pool's
//! box-local producer + validator.
//!
//! Two modes:
//!
//! - **Dump (default):** writes the lived-experience store spans
//!   (`store_spans` — parse-validated, BLAKE3-deduped, withdrawn-filtered)
//!   as extra-spans JSONL rows, so a box with a lived
//!   `.refine/trajectories.kv` can donate its shapes to a campaign running
//!   anywhere (the 4090 store reads 0; the pool file committed in
//!   riir-train `data/pav356/extra_spans.jsonl` carries the donating box's
//!   rows instead). The reachable half of the pool is authored by hand in
//!   the pool file — a frozen snapshot of `src/cf_workers_ai.rs REACHABLE`
//!   plus the two t51-only re-picks, provenance per row: training corpora
//!   are snapshots, not live views, so an upstream re-pick must never
//!   silently rewrite ingested data.
//! - **Validate (`--validate <file>`):** re-reads any extra-spans JSONL and
//!   asserts every row parses standalone (the daemon's prompt contract)
//!   with unique ids and non-empty code — exit 1 names the failing rows.
//!
//! # Run
//!
//! ```sh
//! cargo run --release --features pav_data --example pav356_extra_spans -- \
//!     --store-root . --out /tmp/pav356_store_spans.jsonl
//! cargo run --release --features pav_data --example pav356_extra_spans -- \
//!     --validate ../riir-train/data/pav356/extra_spans.jsonl
//! ```

#![cfg(feature = "pav_data")]

use riir_refine::pav_data::store_spans;
use riir_refine::pruner::RustAnalyzerPruner;
use serde::Serialize;

fn usage() -> ! {
    eprintln!(
        "usage: pav356_extra_spans --out <file> [--store-root <dir>]   # dump store spans as JSONL\n       \
         pav356_extra_spans --validate <file>                          # assert rows parse + ids unique"
    );
    std::process::exit(2);
}

fn flag(args: &[String], name: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == name {
            return it.next().cloned();
        }
    }
    None
}

#[derive(Serialize)]
struct Row<'a> {
    id: &'a str,
    code: &'a str,
    provenance: &'a str,
}

/// The validator's view of a pool row — `id` + `code` only; unknown fields
/// (e.g. `provenance`) are ignored by serde's default and pass through.
#[derive(serde::Deserialize)]
struct PoolRow {
    id: String,
    code: String,
}

fn dump(out_path: &std::path::Path, store_root: &std::path::Path) -> Result<(), String> {
    let (spans, skipped) = store_spans(store_root)?;
    let file = std::fs::File::create(out_path)
        .map_err(|e| format!("create {}: {e}", out_path.display()))?;
    let mut w = std::io::BufWriter::new(file);
    for s in &spans {
        let row = Row {
            id: &s.id,
            code: &s.code,
            provenance: &s.provenance,
        };
        serde_json::to_writer(&mut w, &row)
            .map_err(|e| format!("encode {}: {e}", out_path.display()))?;
        use std::io::Write;
        writeln!(w).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    }
    println!(
        "[store] {} rows -> {} ({} skipped: withdrawn/unparseable/empty/duplicate)",
        spans.len(),
        out_path.display(),
        skipped
    );
    if spans.is_empty() {
        eprintln!(
            "[store] 0 rows — the store under {} is empty (existing-but-empty is honest; check --store-root if unexpected)",
            store_root.display()
        );
    }
    Ok(())
}

fn validate(path: &std::path::Path) -> Result<(), String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let pruner = RustAnalyzerPruner::new();
    let mut rows = 0usize;
    let mut failures: Vec<String> = Vec::new();
    let mut ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        rows += 1;
        let row: PoolRow =
            serde_json::from_str(line).map_err(|e| format!("{}:{}: {e}", path.display(), i + 1))?;
        if row.code.trim().is_empty() {
            failures.push(format!("{}:{}: empty code", path.display(), i + 1));
        }
        if !ids.insert(row.id.clone()) {
            failures.push(format!(
                "{}:{}: duplicate id {}",
                path.display(),
                i + 1,
                row.id
            ));
        }
        if !pruner.validate(&row.code, "").parses {
            failures.push(format!(
                "{}:{}: id {} does not parse standalone (daemon prompt contract)",
                path.display(),
                i + 1,
                row.id
            ));
        }
    }
    println!(
        "[validate] {}: {rows} rows, {} failures",
        path.display(),
        failures.len()
    );
    for f in &failures {
        eprintln!("[validate] FAIL {f}");
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("{} failing row(s)", failures.len()))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(p) = flag(&args, "--validate") {
        match validate(std::path::Path::new(&p)) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("[validate] REFUSED: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    let Some(out) = flag(&args, "--out") else {
        usage();
    };
    let store_root = flag(&args, "--store-root")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    if let Err(e) = dump(std::path::Path::new(&out), &store_root) {
        eprintln!("[store] REFUSED: {e}");
        std::process::exit(2);
    }
}
