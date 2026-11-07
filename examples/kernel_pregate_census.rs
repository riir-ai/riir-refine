//! Issue 214 T2 — the C0 census instrument (REPORT-ONLY; no bar, exit 0).
//!
//! Walks the four issue-214 roots, computes per file the OLD signal set
//! (the pre-fix `| 0x20` fold, inlined below verbatim from the pre-214
//! `kernel_pregate`) and the NEW signal set (the crate's fixed
//! [`riir_refine::kernel_pregate::signal_lines`]), and reports:
//!
//! - signal verdict flips OFF->ON (files that start signaling),
//! - signal verdict flips ON->OFF (a removal would be a defect — G1's
//!   recall law; this fix can only add `@` positions, so 0 is expected),
//! - Scope span flips, measured at **1-line spans** through the crate's
//!   own [`riir_refine::kernel_pregate::span_in_scope`]. `span_in_scope`
//!   is monotone in its line range, so any real matcher span's verdict is
//!   the OR of its lines' verdicts: a real span flips iff it contains at
//!   least one flipped line-seat. Flip counts are additive over spans.
//!
//! A missing root prints UNSEEN — never zero. Run:
//!
//! ```sh
//! cargo run --release --example kernel_pregate_census
//! ```

use std::path::Path;
use std::process::Command;

/// (root dir, git pathspec). `.` with `src/*.rs` is the Bench-137
/// registered walk — the root whose C0 verdict decides the branch.
const ROOTS: [(&str, &str); 4] = [
    (".", "src/*.rs"),
    ("../riir-shader", "*.rs"),
    ("../seal-render", "*.rs"),
    ("../riir-gpu", "*.rs"),
];

/// The ±24 span-scope window (kept equal to the crate's
/// `SPAN_SCOPE_WINDOW_LINES` by construction — the span arm calls the
/// crate's `span_in_scope`, which owns the real constant).
const SPAN_SCOPE_WINDOW_LINES: usize = 24;

// ---- the OLD (pre-214) arm, inlined verbatim ----------------------------
// Byte-faithful to the pre-fix module: `| 0x20` on every scanned byte and
// in the comparison. Kept as the census's OLD oracle; do not "fix" it.

const OLD_SIGNAL_NEEDLES: [&str; 10] = [
    "kernel",
    "wgpu",
    "cube",
    "wgsl",
    "workgroup",
    "threadgroup",
    "compute_pass",
    "computepipeline",
    "dispatch_compute",
    "@compute",
];

const OLD_NEEDLE_FIRST_BYTES: [u8; 6] = *b"kwctd@";

fn old_ci_matches(hay: &[u8], pos: usize, needle: &[u8]) -> bool {
    if pos + needle.len() > hay.len() {
        return false;
    }
    hay[pos..pos + needle.len()]
        .iter()
        .zip(needle.iter())
        .all(|(a, b)| (a | 0x20) == (b | 0x20))
}

fn old_signal_lines(src: &str, path: &str) -> Vec<u32> {
    if Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wgsl"))
    {
        return vec![1];
    }
    let hay = src.as_bytes();
    let mut lines = Vec::new();
    let mut last = 0u32;
    let mut line: u32 = 1;
    let mut i = 0usize;
    while i < hay.len() {
        let b = hay[i] | 0x20;
        if OLD_NEEDLE_FIRST_BYTES.contains(&b) {
            for needle in OLD_SIGNAL_NEEDLES {
                if needle.as_bytes()[0] | 0x20 == b && old_ci_matches(hay, i, needle.as_bytes()) {
                    if line > last {
                        lines.push(line);
                        last = line;
                    }
                    break;
                }
            }
        }
        if hay[i] == b'\n' {
            line += 1;
        }
        i += 1;
    }
    lines
}

// ---- the census ---------------------------------------------------------

struct RootCensus {
    files: usize,
    off_to_on: Vec<String>,
    on_to_off: Vec<String>,
    span_seat_flips: usize,
    span_flip_files: Vec<String>,
    unreadable: usize,
}

fn census_root(dir: &str, pathspec: &str) -> Option<RootCensus> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["ls-files", "--", pathspec])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let listing = String::from_utf8_lossy(&out.stdout);
    let mut c = RootCensus {
        files: 0,
        off_to_on: Vec::new(),
        on_to_off: Vec::new(),
        span_seat_flips: 0,
        span_flip_files: Vec::new(),
        unreadable: 0,
    };
    for rel in listing.lines() {
        if rel.is_empty() {
            continue;
        }
        let path = Path::new(dir).join(rel);
        let Ok(src) = std::fs::read_to_string(&path) else {
            c.unreadable += 1;
            continue;
        };
        c.files += 1;
        let old_lines = old_signal_lines(&src, rel);
        let new_lines = riir_refine::kernel_pregate::signal_lines(&src, rel);
        let old_signaled = !old_lines.is_empty();
        let new_signaled = !new_lines.is_empty();
        let rel_disp = if dir == "." {
            rel.to_string()
        } else {
            format!("{dir}/{rel}")
        };
        if !old_signaled && new_signaled {
            let first = new_lines.first().map_or(String::new(), |l| l.to_string());
            c.off_to_on.push(format!("{rel_disp}:{first}"));
        }
        if old_signaled && !new_signaled {
            c.on_to_off.push(rel_disp.clone());
        }
        // Span flips at 1-line granularity through the crate's own
        // span_in_scope; span_in_scope on the empty OLD set is always
        // false, so these seats are exactly the ±24 neighborhoods of the
        // new-only signal lines.
        let n_lines = src.lines().count();
        let mut seats = 0usize;
        for l in 1..=n_lines {
            let old_v = riir_refine::kernel_pregate::span_in_scope(l, l, &old_lines);
            let new_v = riir_refine::kernel_pregate::span_in_scope(l, l, &new_lines);
            if old_v != new_v {
                seats += 1;
            }
        }
        if seats > 0 {
            // The new-only signal lines drive the widened band — name them
            // (cap 10) so the record shows the flips are `@compute` matches.
            let new_only: Vec<String> = new_lines
                .iter()
                .filter(|l| !old_lines.contains(l))
                .take(10)
                .map(|l| l.to_string())
                .collect();
            c.span_flip_files.push(format!(
                "{rel_disp}: new-only signal lines {new_only:?} -> {seats} seats"
            ));
        }
        c.span_seat_flips += seats;
    }
    Some(c)
}

fn print_root(dir: &str, pathspec: &str, c: &RootCensus) {
    println!("== root {dir}  (pathspec {pathspec})");
    println!("   files scanned: {}", c.files);
    if c.unreadable > 0 {
        println!("   unreadable (non-UTF-8, skipped loud): {}", c.unreadable);
    }
    println!(
        "   signal OFF->ON flips: {}{}",
        c.off_to_on.len(),
        if c.off_to_on.is_empty() {
            ""
        } else {
            "  (path:first-new-signal-line, cap 20)"
        }
    );
    for row in c.off_to_on.iter().take(20) {
        println!("     {row}");
    }
    if c.off_to_on.len() > 20 {
        println!("     … and {} more", c.off_to_on.len() - 20);
    }
    println!(
        "   signal ON->OFF flips: {} (G1: a removal here is a defect)",
        c.on_to_off.len()
    );
    for row in &c.on_to_off {
        println!("     {row}");
    }
    println!(
        "   span-seat flips (1-line spans, ±{}): {} across {} files",
        SPAN_SCOPE_WINDOW_LINES,
        c.span_seat_flips,
        c.span_flip_files.len()
    );
    for row in c.span_flip_files.iter().take(20) {
        println!("     {row}");
    }
    if c.span_flip_files.len() > 20 {
        println!("     … and {} more", c.span_flip_files.len() - 20);
    }
}

fn main() {
    println!("== C0 census (issue 214 T2): OLD `| 0x20` fold vs NEW ASCII-letter-only fold");
    println!("   SPANS-MEASURED-AT: 1-line spans through the crate's own span_in_scope (±24);");
    println!("   a real matcher span flips iff it contains at least one flipped line-seat.");
    let mut registered_flips = 0usize;
    let mut registered_span_flips = 0usize;
    for (dir, pathspec) in ROOTS {
        match census_root(dir, pathspec) {
            None => {
                println!("== root {dir} UNSEEN (absent on this box) — recorded UNSEEN, never zero")
            }
            Some(c) => {
                if dir == "." {
                    registered_flips = c.off_to_on.len();
                    registered_span_flips = c.span_seat_flips;
                }
                print_root(dir, pathspec, &c);
            }
        }
    }
    println!();
    println!("== VERDICT (the branch decider): registered walk (./src)");
    println!(
        "   signal OFF->ON flips = {registered_flips}, span-seat flips = {registered_span_flips}"
    );
    if registered_flips == 0 && registered_span_flips == 0 {
        println!(
            "   -> C0 = 0: behavior-neutral on the registered corpus (issue-214 T4 may close)."
        );
    } else {
        println!(
            "   -> C0 > 0: the G5 cost run is required (orchestrator queues the rig; never m3)."
        );
    }
}
