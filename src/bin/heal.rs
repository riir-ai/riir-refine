//! `cargo heal` — the unified healing CLI (Plan 005 Phase 2 + Phase 6).
//!
//! `--suggest` mode: walk real `.rs` files, chunk them at fn/impl/struct
//! boundaries, retrieve matching corpus rules via the latent substrate
//! (fan-out across all feature-enabled domains; the router verdict
//! annotates each span), and print top-K rules with scores + the
//! fixed/optimized pattern snippet for manual review.
//!
//! `--fix` mode (Phase 6 T6.1 + T6.4): run the compiled-in proposers on
//! each file + apply their span-preserving edits — rust_perf rules plus the
//! structural clippy lints (the three type-blind clippy proposers stay
//! suggest-only; see the clippy proposers' module docs). Without `--write`,
//! fixed files print a per-edit review (rule + the changed span); with
//! `--write`, the spliced file is written in place. Everything outside a
//! matched span stays byte-identical. `--fix` ALSO runs the suggest pass
//! unless `--quiet` is given, so rules without proposers still surface.
//!
//! Run (in-repo):
//!
//! ```text
//! cargo run --features latent_retrieval --bin cargo-heal -- --suggest src/
//! cargo run --features latent_retrieval --bin cargo-heal -- --fix src/          # review
//! cargo run --features latent_retrieval --bin cargo-heal -- --fix --write src/  # apply
//! ```
//!
//! Installed as `cargo-heal` (via `cargo install --path . --features
//! latent_retrieval`), cargo's subcommand convention makes it `cargo heal
//! --suggest src/`.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use riir_clippy::domains::clippy_lints::proposers::FIXABLE_LINTS;
use riir_clippy::domains::rust_perf::proposers::FIXABLE_RULES;
use riir_clippy::edit_emitter::{apply_edits, RuleEdit};
use riir_clippy::draft::{DomainId, LatentMatcher, SpanSuggestion};

/// Print usage + exit 2 (arg-error convention).
fn usage() -> ! {
    eprintln!(
        "cargo-heal (Plan 005) — modelless code healing via latent corpus retrieval

USAGE:
    cargo heal --suggest [--domain <auto|clippy|perf|kernel>] [--top-k <N>] <path>...
    cargo heal --fix [--write] [--quiet] [--top-k <N>] <path>...

    --suggest       Suggest mode: print matching rules per code span (no edits).
    --fix           Fix mode: apply the compiled-in proposers' span-preserving edits
                    ({} perf rules + {} clippy lints: {} | {}). The proposer's visitor
                    is the gate — latent suggestions never auto-apply.
    --write         With --fix: write the spliced file in place. Without it, print a
                    per-edit review (default: review only).
    --quiet         With --fix: skip the --suggest pass (only show fixes).
    --domain        Filter suggest output to one domain (default: auto — all enabled).
    --top-k <N>     Rules per span per domain (default 3).
    <path>...       Files or directories to scan (.rs files only).

Domains are feature-gated: clippy + perf are default-on; kernel needs
--features kernel_heal. Requesting a domain whose feature is off is an
error (T2.4 gating).",
        FIXABLE_RULES.len(),
        FIXABLE_RULES.join(", "),
        FIXABLE_LINTS.len(),
        FIXABLE_LINTS.join(", ")
    );
    std::process::exit(2);
}

struct Args {
    suggest: bool,
    fix: bool,
    write: bool,
    quiet: bool,
    domain: Option<DomainId>,
    top_k: usize,
    paths: Vec<PathBuf>,
}

fn parse_args() -> Args {
    let mut args = Args {
        suggest: false,
        fix: false,
        write: false,
        quiet: false,
        domain: None,
        top_k: 3,
        paths: Vec::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--suggest" => args.suggest = true,
            "--fix" => args.fix = true,
            "--write" => args.write = true,
            "--quiet" => args.quiet = true,
            "--domain" => {
                let Some(value) = it.next() else {
                    eprintln!("--domain requires a value");
                    usage();
                };
                match value.as_str() {
                    "auto" => args.domain = None,
                    "clippy" => args.domain = Some(DomainId::Clippy),
                    "perf" => args.domain = Some(DomainId::Perf),
                    "kernel" => args.domain = Some(DomainId::Kernel),
                    other => {
                        eprintln!("unknown --domain {other:?} (want auto|clippy|perf|kernel)");
                        usage();
                    }
                }
            }
            "--top-k" => {
                let Some(value) = it.next() else {
                    eprintln!("--top-k requires a value");
                    usage();
                };
                args.top_k = value.parse().unwrap_or_else(|_| {
                    eprintln!("--top-k expects a number, got {value:?}");
                    usage();
                });
            }
            flag if flag.starts_with("--") => {
                eprintln!("unknown flag {flag:?}");
                usage();
            }
            path => args.paths.push(PathBuf::from(path)),
        }
    }
    if args.paths.is_empty() {
        eprintln!("no paths given");
        usage();
    }
    args
}

/// Recursively collect `.rs` files (std-only walker — no new dep). Skips
/// hidden dirs + `target`/`vendor` build noise.
fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    if root.is_file() {
        out.push(root.to_path_buf());
        return;
    }
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root.to_path_buf());
    while let Some(dir) = queue.pop_front() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with('.') || name == "target" || name == "vendor" {
                continue;
            }
            if path.is_dir() {
                queue.push_back(path);
            } else if name.ends_with(".rs") {
                out.push(path);
            }
        }
    }
}

/// One-line summary of a multi-line snippet (first non-trivial line).
fn snippet_head(text: &str) -> &str {
    text.lines()
        .map(str::trim)
        .find(|l| l.len() > 8)
        .unwrap_or("…")
}

fn print_suggestion(s: &SpanSuggestion, matcher: &LatentMatcher, domain_filter: Option<DomainId>) {
    let routed = s.routed_domain.label();
    println!("{}:{}-{} (routed: {routed})", s.path, s.start_line, s.end_line);
    for (domain, rules) in &s.by_domain {
        if domain_filter.is_some_and(|want| want != *domain) {
            continue;
        }
        for rule in rules {
            let info = matcher.rule_book(&rule.rule_id);
            let fixed = info.map_or("?", |i| snippet_head(&i.fixed_code));
            println!(
                "  [{:<6}] {:<32} score {:>8.2}  →  {fixed}",
                domain.label(),
                rule.rule_id,
                rule.score
            );
        }
    }
}

fn main() {
    let args = parse_args();
    if !args.suggest && !args.fix {
        eprintln!("no mode given — use --suggest or --fix");
        usage();
    }
    if args.write && !args.fix {
        eprintln!("--write requires --fix");
        usage();
    }

    let mut matcher = LatentMatcher::from_enabled_domains().unwrap_or_else(|e| {
        eprintln!("index build failed: {e}");
        std::process::exit(1);
    });

    // T2.4 domain gating: an explicitly requested domain whose feature is
    // off is an error, not a silent skip.
    if let Some(want) = args.domain {
        let enabled = matcher.domains().contains(&want);
        if !enabled {
            eprintln!(
                "--domain {} requested but its Cargo feature is not enabled (have: {:?})",
                want.label(),
                matcher
                    .domains()
                    .iter()
                    .map(|d| d.label())
                    .collect::<Vec<_>>()
            );
            std::process::exit(2);
        }
    }

    let mut files = Vec::new();
    for path in &args.paths {
        collect_rs_files(path, &mut files);
    }
    if files.is_empty() {
        eprintln!("no .rs files under the given paths");
        std::process::exit(1);
    }

    let mut n_chunks = 0usize;
    let mut n_suggestions = 0usize;
    let mut n_fixed_files = 0usize;
    let mut n_fixed_edits = 0usize;
    for file in &files {
        let Ok(src) = std::fs::read_to_string(file) else {
            eprintln!("skip (unreadable): {}", file.display());
            continue;
        };
        let display = file.display().to_string();

        // ── --fix: run the compiled-in proposers (the visitor IS the gate).
        // Both domains: rust_perf rules + the structural clippy lints (T6.4).
        // Cross-domain overlaps resolve in apply_edits' outer-first selection.
        //
        // Fixpoint iteration: an outer edit's replacement keeps the inner
        // nodes' ORIGINAL text, so fixes nested inside a replaced region
        // (`return Cfg { x: x };` — the field-shorthand edits live inside the
        // return-strip's span) are dropped by overlap selection on pass 1.
        // Re-parsing the spliced output exposes them; iterate until no new
        // edits (bounded, and the unchanged-output guard stops oscillation).
        const MAX_FIX_PASSES: usize = 4;
        let mut current = src.clone();
        let mut all_edits: Vec<(String, usize)> = Vec::new(); // (rule_id, line)
        if args.fix {
            for _ in 0..MAX_FIX_PASSES {
                let Ok(parsed) = syn::parse_file(&current) else {
                    eprintln!("skip (unparseable): {display}");
                    break;
                };
                let mut edits: Vec<RuleEdit> = Vec::new();
                for rule in FIXABLE_RULES {
                    edits.extend(riir_clippy::domains::rust_perf::proposers::collect_edits(
                        rule, &current, &parsed,
                    ));
                }
                for lint in FIXABLE_LINTS {
                    edits.extend(
                        riir_clippy::domains::clippy_lints::proposers::collect_edits(
                            lint, &current, &parsed,
                        ),
                    );
                }
                if edits.is_empty() {
                    break;
                }
                let next = apply_edits(
                    &current,
                    edits.iter().map(|re| re.edit.clone()).collect(),
                );
                for re in &edits {
                    let line = current[..re.edit.start].matches('\n').count() + 1;
                    all_edits.push((re.rule_id.clone(), line));
                }
                let changed = next != current;
                current = next;
                if !changed {
                    break;
                }
            }
        }
        let all_edits = all_edits;
        if !all_edits.is_empty() {
            n_fixed_files += 1;
            n_fixed_edits += all_edits.len();
            println!("fix {} ({} edit(s)):", display, all_edits.len());
            for (rule_id, line) in &all_edits {
                println!("  [{:<24}] line {line}", rule_id);
            }
            if args.write {
                std::fs::write(file, &current).unwrap_or_else(|e| {
                    eprintln!("write failed for {display}: {e}");
                });
            }
        }

        // ── suggest pass (unless --fix --quiet). Runs over the ORIGINAL
        // source — suggestions reference the user's current file; the fix
        // review above is the change record.
        if args.suggest || (args.fix && !args.quiet) {
            for s in matcher.suggest_file(&src, &display, args.top_k) {
                n_chunks += 1;
                if s.all_rules().is_empty() {
                    continue;
                }
                n_suggestions += 1;
                print_suggestion(&s, &matcher, args.domain);
            }
        }
    }
    let mode = if args.fix {
        format!(
            ", fixed {} file(s) / {} edit(s){}",
            n_fixed_files,
            n_fixed_edits,
            if args.write { " (written)" } else { " (review only — add --write)" }
        )
    } else {
        String::new()
    };
    println!(
        "\nscanned {} file(s), {} span(s), {} with suggestions (top-{} per domain){}",
        files.len(),
        n_chunks,
        n_suggestions,
        args.top_k,
        mode
    );
}
