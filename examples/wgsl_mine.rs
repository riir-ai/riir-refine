//! Issue 025 T6 — the WGSL/cube buggy→fixed MINING harness (git-history
//! differential scan). This is the repeatable integration the mining loop
//! runs: it walks a repo's git history for non-syn-surface changes, scans
//! each commit's pre/post versions with the FULL pattern set (default +
//! experimental — in mining the DIFF adjudicates), and reports the pairs
//! where a rule FIRES pre-fix and is SILENT post-fix **and** the fired line
//! was REMOVED by the commit (the "pair rules with diffs, not files with
//! history" discipline from Bench 033, mechanized).
//!
//! Surfaces:
//! - default: `.wgsl` files (`git log --diff-filter=M -- '*.wgsl'`)
//! - `--rs`: `#[cube]` bodies in `.rs` files (commits whose diff touched a
//!   `#[cube` line — `git log -G'#[cube' -- '*.rs'`)
//!
//! Output: one row per discriminating (rule, commit, file) candidate +
//! introduced-findings info (post fires, pre silent — new anti-patterns
//! landing, NOT pairs) + a summary. Every candidate still needs human
//! adjudication of the diff (the harness proves the pattern discriminates;
//! the commit message + diff prove the fix is genuine).
//!
//! ```bash
//! cargo run --features kernel_heal --example wgsl_mine -- ../riir-ai
//! cargo run --features kernel_heal --example wgsl_mine -- ../riir-ai --rs
//! ```

use std::process::Command;

use riir_refine::domains::kernel_opt::wgsl_scan::{WgslFinding, WgslScanner};

/// One mined candidate: a rule that fired pre-fix, went silent post-fix, and
/// whose fired line is diff-associated (removed directly or inside a changed
/// hunk's span).
#[derive(Debug)]
struct PairCandidate {
    commit: String,
    subject: String,
    file: String,
    rule: &'static str,
    pre_lines: Vec<usize>,
    evidence: Evidence,
}

fn git(repo: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("spawn git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Diff-association evidence for a mined candidate.
#[derive(Debug, PartialEq, Eq)]
enum Evidence {
    /// The fired line itself was removed by the commit (direct).
    Removed,
    /// The fired line sits inside a changed hunk's old-side span (the fix
    /// touched the finding's neighborhood — e.g. the 698 fix changed the
    /// STRIDE line, leaving the accumulation as unchanged context).
    Hunk,
}

/// Old-side line spans (start..=end, context included) of every hunk in a
/// `-U3` unified diff, plus the directly-REMOVED line numbers.
fn hunk_spans_and_removed(diff: &str) -> (Vec<(usize, usize)>, Vec<usize>) {
    let mut spans = Vec::new();
    let mut removed = Vec::new();
    let mut old: Option<usize> = None;
    for line in diff.lines() {
        if let Some(h) = line.strip_prefix("@@") {
            let body = h.split("@@").next().unwrap_or("").trim();
            if let Some(oldspec) = body.strip_prefix('-') {
                let start: usize = oldspec
                    .split(',')
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let count: usize = oldspec
                    .split(',')
                    .nth(1)
                    .and_then(|s| s.trim().split(' ').next().and_then(|t| t.parse().ok()))
                    .unwrap_or(1);
                if count > 0 {
                    spans.push((start, start + count - 1));
                }
                old = Some(start);
            }
        } else if line.starts_with("diff ") || line.starts_with("index ") {
            old = None;
        } else if line.starts_with("---") || line.starts_with("+++") {
            // file headers
        } else if let (Some(cursor), true) = (old, line.starts_with('-')) {
            removed.push(cursor);
            old = Some(cursor + 1);
        } else if let Some(cursor) = old {
            if !line.starts_with('+') {
                old = Some(cursor + 1);
            }
        }
    }
    (spans, removed)
}

/// Enumerate (commit, subject, file) triples for the mining surfaces.
/// - `.wgsl`: every modified `.wgsl` file.
/// - cube bodies (`--rs`): every commit touching a file that contains `#[cube`
///   in the CURRENT tree (via one `git grep -l`), plus any `--extra-path`
///   historical seams (e.g. the pre-split `weaver_gpu.rs`, whose cube items
///   predate the `weaver_gpu/mod.rs` split). Deleted-and-gone cube files
///   beyond supplied seams are a documented gap — add them via `--extra-path`.
fn enumerate_commits(
    repo: &str,
    rs_mode: bool,
    extra_paths: &[String],
) -> Result<Vec<(String, String, String)>, String> {
    if !rs_mode {
        let log = git(
            repo,
            &[
                "log",
                "--diff-filter=M",
                "--name-only",
                "--pretty=format:@@@%h %s",
                "--",
                "*.wgsl",
            ],
        )?;
        return parse_log(&log, &|f| f.ends_with(".wgsl"));
    }

    // Current-tree cube files + caller-supplied historical seams.
    let grep = git(repo, &["grep", "-l", r"#\[cube", "HEAD"])?;
    let mut cube_paths: Vec<String> = grep
        .lines()
        .filter_map(|l| l.split_once(':').map(|(_, p)| p.trim().to_string()))
        .filter(|p| p.ends_with(".rs"))
        .collect();
    cube_paths.extend(extra_paths.iter().cloned());
    cube_paths.sort();
    cube_paths.dedup();
    if cube_paths.is_empty() {
        return Ok(Vec::new());
    }

    let mut log_args: Vec<&str> = vec!["log", "--name-only", "--pretty=format:@@@%h %s", "--"];
    log_args.extend(cube_paths.iter().map(String::as_str));
    let log = git(repo, &log_args)?;
    let set: Vec<String> = cube_paths;
    parse_log(&log, &move |f| set.iter().any(|p| p == f))
}

fn parse_log(
    log: &str,
    keep: &dyn Fn(&str) -> bool,
) -> Result<Vec<(String, String, String)>, String> {
    let mut out = Vec::new();
    let mut commit = String::new();
    let mut subject = String::new();
    for line in log.lines() {
        if let Some(rest) = line.strip_prefix("@@@") {
            let (h, s) = rest.split_once(' ').unwrap_or((rest, ""));
            commit = h.to_string();
            subject = s.to_string();
        } else if keep(line) && !commit.is_empty() {
            out.push((commit.clone(), subject.clone(), line.to_string()));
        }
    }
    Ok(out)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rs_mode = args.iter().any(|a| a == "--rs");
    // Positional args = flags stripped + flag VALUES (`--extra-path <v>`)
    // consumed, so the first remaining positional is the repo path.
    let mut positional: Vec<String> = Vec::new();
    let mut skip_next = false;
    for a in &args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a == "--extra-path" {
            skip_next = true;
            continue;
        }
        if !a.starts_with("--") {
            positional.push(a.clone());
        }
    }
    let extra_paths: Vec<String> = args
        .iter()
        .skip_while(|a| *a != "--extra-path")
        .skip(1)
        .take(1)
        .cloned()
        .collect();
    let repo = positional
        .first()
        .cloned()
        .unwrap_or_else(|| "../riir-ai".to_string());

    let entries = enumerate_commits(&repo, rs_mode, &extra_paths).unwrap_or_else(|e| {
        eprintln!("enumerate failed: {e}");
        std::process::exit(1);
    });
    println!(
        "mining {} surface: {} (commit,file) pairs over {}",
        if rs_mode { "cube-body" } else { ".wgsl" },
        entries.len(),
        repo
    );

    let scanner = WgslScanner::default().with_experimental();
    let mut candidates: Vec<PairCandidate> = Vec::new();
    let mut introduced: Vec<String> = Vec::new();
    let mut moved: Vec<String> = Vec::new();
    let mut skipped = 0usize;

    for (commit, subject, file) in &entries {
        let show = |rev: &str| git(&repo, &["show", &format!("{rev}:{file}")]);
        let (Ok(pre), Ok(post)) = (show(&format!("{commit}^")), show(commit)) else {
            skipped += 1; // added/deleted file or rewrite — no pair side
            continue;
        };

        let (pre_findings, post_findings): (Vec<WgslFinding>, Vec<WgslFinding>) = if rs_mode {
            (scanner.scan_cube_rust(&pre), scanner.scan_cube_rust(&post))
        } else {
            (scanner.scan(&pre), scanner.scan(&post))
        };

        // FINDING-level pairing: a pre-finding at line L pairs when NO
        // post-finding of the same rule sits within ±10 lines (file-level
        // rule silence is too strict — a multi-kernel file keeps firing the
        // rule on its OTHER unchanged kernels).
        const NEAR: i64 = 10;
        for f in &pre_findings {
            let near_post = post_findings
                .iter()
                .any(|p| p.rule_id == f.rule_id && (p.line as i64 - f.line as i64).abs() <= NEAR);
            if near_post {
                continue; // the finding persists — nothing to pair
            }
            // MOVE check: identical excerpt fires elsewhere in post — the
            // shape was relocated, not fixed.
            if post_findings
                .iter()
                .any(|p| p.rule_id == f.rule_id && p.excerpt == f.excerpt)
            {
                moved.push(format!("{} {} L{} [{}]", commit, file, f.line, f.rule_id));
                continue;
            }
            // …whose fired line is diff-ASSOCIATED (removed directly, or inside
            // a changed hunk's old-side span — the 698 stride fix leaves the
            // accumulation as unchanged context inside the hunk).
            let diff = git(
                &repo,
                &["diff", "-U3", &format!("{commit}^"), commit, "--", file],
            )
            .unwrap_or_default();
            let (spans, removed) = hunk_spans_and_removed(&diff);
            let evidence = if removed.contains(&f.line) {
                Evidence::Removed
            } else if spans.iter().any(|&(a, b)| f.line >= a && f.line <= b) {
                Evidence::Hunk
            } else {
                continue; // diff-untouched finding — the commit did not fix it
            };
            candidates.push(PairCandidate {
                commit: commit.clone(),
                subject: subject.clone(),
                file: file.clone(),
                rule: f.rule_id,
                pre_lines: vec![f.line],
                evidence,
            });
        }
        // Introduced findings (post fires with no nearby pre) — info, not pairs.
        for f in &post_findings {
            let near_pre = pre_findings
                .iter()
                .any(|p| p.rule_id == f.rule_id && (p.line as i64 - f.line as i64).abs() <= NEAR);
            if !near_pre {
                introduced.push(format!("{} {} L{} [{}]", commit, file, f.line, f.rule_id));
            }
        }
    }

    println!("\n== PAIR CANDIDATES (fires pre, silent post, diff-associated) ==\n");
    for c in &candidates {
        println!(
            "{} {} [{}] pre-L{:?} ({:?})\n    {}",
            c.commit, c.file, c.rule, c.pre_lines, c.evidence, c.subject
        );
    }
    println!(
        "\ncandidates: {} | moved (shape relocated, NOT fixed): {} | introduced (info): {} | skipped (no pre/post): {}",
        candidates.len(),
        moved.len(),
        introduced.len(),
        skipped
    );
    if !moved.is_empty() {
        println!("\n== MOVED (same excerpt fires elsewhere in post — refuted) ==");
        for l in &moved {
            println!("  {l}");
        }
    }
    if !introduced.is_empty() {
        println!("\n== INTRODUCED (post fires, pre silent — new anti-patterns, NOT pairs) ==");
        for l in introduced {
            println!("  {l}");
        }
    }
}
