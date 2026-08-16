//! Issue 025 — WGSL surface scanner calibration smoke (T2/T5 building block).
//!
//! Scans every `.wgsl` file under a directory (default: the riir-gpu kernels
//! dir) with the precision-vetted pattern set, printing per-file findings and
//! a per-rule total. The T5 GOAT gate runs this over the live tree (≥5 real
//! anti-patterns) and the clean-file controls (post-fix versions must not
//! fire).
//!
//! ```bash
//! cargo run --features kernel_heal --example wgsl_scan_smoke -- [dir]
//! ```

use std::collections::BTreeMap;

use riir_clippy::domains::kernel_opt::wgsl_scan::{WgslFinding, WgslScanner};

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../riir-ai/crates/riir-gpu/src/kernels".to_string());
    let scanner = WgslScanner::default();
    let mut total = 0usize;
    let mut files = 0usize;
    let mut firing_files = 0usize;
    let mut by_rule: BTreeMap<&'static str, usize> = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {dir}: {e}")) {
        let p = entry.expect("dir entry").path();
        if p.extension().and_then(|e| e.to_str()) != Some("wgsl") {
            continue;
        }
        files += 1;
        let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
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
