//! Issue 025 — WGSL surface scanner calibration smoke (T2/T3/T5 building block).
//!
//! Two modes:
//! - default: scan every `.wgsl` file under a directory (default: the
//!   riir-gpu kernels dir) with the precision-vetted pattern set.
//! - `--rs [dir]`: recursively extract + scan every `#[cube]` item in `.rs`
//!   files under a directory (default: the riir-gpu src tree) — the T3
//!   calibration pass (item counts vs the T1 inventory, findings, absolute
//!   line mapping).
//!
//! The T5 GOAT gate runs the `.wgsl` mode over the live tree (≥5 real
//! anti-patterns) and the clean-file controls (post-fix versions must not
//! fire).
//!
//! ```bash
//! cargo run --features kernel_heal --example wgsl_scan_smoke -- [dir]
//! cargo run --features kernel_heal --example wgsl_scan_smoke -- --rs [dir]
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use riir_refine::domains::kernel_opt::cube_extract::extract_cube_items;
use riir_refine::domains::kernel_opt::wgsl_scan::{WgslFinding, WgslScanner};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--rs") {
        let dir = args
            .get(1)
            .cloned()
            .unwrap_or_else(|| "../riir-ai/crates/riir-gpu/src".to_string());
        run_rs_mode(&dir);
    } else {
        let dir = args
            .first()
            .cloned()
            .unwrap_or_else(|| "../riir-ai/crates/riir-gpu/src/kernels".to_string());
        run_wgsl_mode(&dir);
    }
}

fn run_wgsl_mode(dir: &str) {
    let scanner = WgslScanner::default();
    let mut total = 0usize;
    let mut files = 0usize;
    let mut firing_files = 0usize;
    let mut by_rule: BTreeMap<&'static str, usize> = BTreeMap::new();
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {dir}: {e}")) {
        let p = entry.expect("dir entry").path();
        if p.extension().and_then(|e| e.to_str()) != Some("wgsl") {
            continue;
        }
        files += 1;
        let src =
            std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        let found: Vec<WgslFinding> = scanner.scan(&src);
        if !found.is_empty() {
            firing_files += 1;
            println!("== {}", p.file_name().unwrap().to_string_lossy());
            for f in &found {
                println!("  L{} [{}] {}", f.line, f.rule_id, f.excerpt.trim());
                *by_rule.entry(f.rule_id).or_insert(0) += 1;
                total += 1;
            }
        }
    }
    println!("\nscanned {files} files ({firing_files} firing); total findings: {total}");
    for (r, n) in &by_rule {
        println!("  {r}: {n}");
    }
}

fn run_rs_mode(dir: &str) {
    let scanner = WgslScanner::default();
    let mut rs_files = Vec::new();
    collect_rs_files(Path::new(dir), &mut rs_files);
    rs_files.sort();

    let mut files_with_items = 0usize;
    let mut total_items = 0usize;
    let mut total_item_lines = 0usize;
    let mut launch_items = 0usize;
    let mut findings = 0usize;
    let mut by_rule: BTreeMap<&'static str, usize> = BTreeMap::new();

    for p in &rs_files {
        let Ok(src) = std::fs::read_to_string(p) else {
            continue;
        };
        let items = extract_cube_items(&src);
        if items.is_empty() {
            continue;
        }
        files_with_items += 1;
        total_items += items.len();
        launch_items += items.iter().filter(|i| i.launch).count();
        for it in &items {
            total_item_lines += it.end_line - it.start_line + 1;
        }
        let found = scanner.scan_cube_rust(&src);
        if !found.is_empty() {
            println!("== {}", p.display());
            for f in &found {
                println!("  L{} [{}] {}", f.line, f.rule_id, f.excerpt.trim());
                *by_rule.entry(f.rule_id).or_insert(0) += 1;
                findings += 1;
            }
        }
    }

    println!(
        "\nscanned {} rs files; {} files with cube items; {} items ({} launch) spanning {} lines",
        rs_files.len(),
        files_with_items,
        total_items,
        launch_items,
        total_item_lines
    );
    println!("findings: {findings}");
    for (r, n) in &by_rule {
        println!("  {r}: {n}");
    }
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs_files(&p, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}
