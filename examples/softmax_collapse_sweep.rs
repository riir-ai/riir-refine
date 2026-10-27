//! Issue 185 T4 — the softmax-collapse family's workspace sweep (READ-ONLY).
//!
//! Walks every TRACKED `*.rs` (`git ls-files`, so gitignored scratch and
//! nested non-repos never count) of each sibling repo, parses it, and runs
//! the three rules: `sigmoid-minus-one` (the mechanical proposer's edits —
//! counted, never applied), `softmax-onehot-collapse` and
//! `neg-ln-softmax-prob` (the suggest-only detectors). Prints one row per
//! hit and a per-repo × per-rule tally.
//!
//! ```text
//! cargo run --release --no-default-features --features sec \
//!     --example softmax_collapse_sweep -- [<workspace-dir>] [<repo>...]
//! ```
//!
//! Default workspace: the parent of this repo; default population: every
//! child named `riir-*`, `katgpt-rs`, `seal-*` that is a git checkout.
//! A repo whose walk yields zero `.rs` files is printed as `EMPTY-WALK`,
//! never as a clean zero; a file that does not parse is counted as
//! `unparsed`, never folded into the clean column.
//!
//! Issue 185 T4b: a site carrying the family's allow marker
//! (`// softmax-collapse: allow — <reason>`, same line or the line
//! directly above) prints as an `ALLOW` row with its reason and counts in
//! its own `allowed` column — suppressed, never folded into zero.

use std::path::{Path, PathBuf};
use std::process::Command;

use riir_refine::domains::sec::proposers::{sigmoid_minus_one, softmax_collapse};

const RULES: [&str; 3] = [
    "sigmoid-minus-one",
    softmax_collapse::RULE_SOFTMAX_ONEHOT,
    softmax_collapse::RULE_NEG_LN,
];

fn tracked_rs(repo: &Path) -> Option<Vec<PathBuf>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["ls-files", "-z", "--", "*.rs"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        out.stdout
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| repo.join(String::from_utf8_lossy(s).as_ref()))
            .collect(),
    )
}

fn line_of(src: &str, byte: usize) -> usize {
    src.get(..byte).map_or(0, |s| s.matches('\n').count()) + 1
}

fn one_line(t: &str) -> String {
    let s: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.len() > 110 {
        let cut = (0..=110)
            .rev()
            .find(|&i| s.is_char_boundary(i))
            .unwrap_or(0);
        format!("{}…", &s[..cut])
    } else {
        s
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let here = std::env::current_dir().expect("cwd");
    let (ws, picked): (PathBuf, Vec<String>) = match args.split_first() {
        Some((w, rest)) => (PathBuf::from(w), rest.to_vec()),
        None => (here.parent().expect("parent").to_path_buf(), Vec::new()),
    };
    let mut repos: Vec<String> = if picked.is_empty() {
        std::fs::read_dir(&ws)
            .expect("read workspace")
            .filter_map(Result::ok)
            .filter(|e| e.path().join(".git").exists())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("riir-") || n == "katgpt-rs" || n.starts_with("seal-"))
            .collect()
    } else {
        picked
    };
    repos.sort();

    let mut totals = [0usize; 3];
    let mut total_allowed = 0usize;
    let mut table = Vec::new();
    for name in &repos {
        let repo = ws.join(name);
        let Some(files) = tracked_rs(&repo) else {
            println!("{name}: NOT-A-CHECKOUT");
            continue;
        };
        if files.is_empty() {
            println!("{name}: EMPTY-WALK (0 tracked .rs)");
            table.push((name.clone(), [0; 3], 0, 0, 0));
            continue;
        }
        let mut counts = [0usize; 3];
        let mut allowed = 0usize;
        let mut unparsed = 0usize;
        for f in &files {
            let Ok(src) = std::fs::read_to_string(f) else {
                unparsed += 1;
                continue;
            };
            let Ok(file) = syn::parse_file(&src) else {
                unparsed += 1;
                continue;
            };
            let rel = f.strip_prefix(&ws).unwrap_or(f).display().to_string();
            for (e, allow) in sigmoid_minus_one::collect_edits_with_allow(&src, &file) {
                let text = src.get(e.start..e.end).unwrap_or("");
                let at = line_of(&src, e.start);
                match allow {
                    Some(why) => {
                        allowed += 1;
                        println!(
                            "ALLOW {rel}:{at} [{}] {}  ::  {why}",
                            RULES[0],
                            one_line(text)
                        );
                    }
                    None => {
                        counts[0] += 1;
                        println!(
                            "HIT {rel}:{at} [{}] {}  =>  {}",
                            RULES[0],
                            one_line(text),
                            one_line(&e.replacement)
                        );
                    }
                }
            }
            for h in softmax_collapse::detect_all(&src, &file) {
                let at = line_of(&src, h.span.start);
                if let Some(why) = &h.allowed {
                    allowed += 1;
                    println!(
                        "ALLOW {rel}:{at} [{}] {}  ::  {why}",
                        h.rule_id,
                        one_line(&h.text)
                    );
                    continue;
                }
                let i = RULES.iter().position(|r| *r == h.rule_id).unwrap_or(1);
                counts[i] += 1;
                println!("HIT {rel}:{at} [{}] {}", h.rule_id, one_line(&h.text));
            }
        }
        for (t, c) in totals.iter_mut().zip(counts) {
            *t += c;
        }
        total_allowed += allowed;
        table.push((name.clone(), counts, files.len(), unparsed, allowed));
    }

    println!();
    println!(
        "{:<24} {:>7} {:>9} {:>17} {:>23} {:>19} {:>8}",
        "repo", "files", "unparsed", RULES[0], RULES[1], RULES[2], "allowed"
    );
    for (name, c, n, u, a) in &table {
        println!(
            "{name:<24} {n:>7} {u:>9} {:>17} {:>23} {:>19} {a:>8}",
            c[0], c[1], c[2]
        );
    }
    println!(
        "TOTAL over {} repos: {} = {}, {} = {}, {} = {}; allowed (marker + reason, not counted above) = {}",
        table.len(),
        RULES[0],
        totals[0],
        RULES[1],
        totals[1],
        RULES[2],
        totals[2],
        total_allowed
    );
}
