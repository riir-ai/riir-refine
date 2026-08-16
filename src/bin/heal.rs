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
use riir_clippy::edit_emitter::RuleEdit;
use riir_clippy::draft::{DomainId, LatentMatcher, SpanSuggestion};
use riir_clippy::EvolveRecorder;

#[cfg(feature = "fix_verify")]
use riir_clippy::domains::clippy_lints::proposers::VERIFY_LINTS;

/// Resolve the self-evolve memory file (Issue 018): `RIIR_HEAL_MEMORY`
/// overrides; otherwise the platform cache dir
/// (`$XDG_CACHE_HOME` / `~/.cache` / `%LOCALAPPDATA%` / temp fallback).
fn default_memory_path() -> PathBuf {
    if let Ok(p) = std::env::var("RIIR_HEAL_MEMORY") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join("riir-clippy").join("memory.json")
}

/// Print the per-edit fix review + (with --write) write the spliced file.
/// `edits` are `(rule_id, final line, replacement, restore)` — only the
/// first two fields print here; the last two exist for the verify path.
fn print_fix_review(
    display: &str,
    edits: &[(String, usize, String, String)],
    write: bool,
    file: &Path,
    current: &str,
) {
    if edits.is_empty() {
        return;
    }
    println!("fix {} ({} edit(s)):", display, edits.len());
    for (rule_id, line, _, _) in edits {
        println!("  [{:<24}] line {line}", rule_id);
    }
    if write {
        std::fs::write(file, current).unwrap_or_else(|e| {
            eprintln!("write failed for {display}: {e}");
        });
    }
}

/// Print usage + exit 2 (arg-error convention).
fn usage() -> ! {
    #[cfg(feature = "fix_verify")]
    let verify_help = format!(
        "    --verify        With --fix: write, cargo-check, auto-revert the offending edits\n\
         (adds {} + looser capacity bounds). Implies --write.\n\
    --verify-args <s> Extra cargo args for --verify (whitespace-split, e.g. \"--features a,b,c\").\n",
        VERIFY_LINTS.join(", ")
    );
    #[cfg(not(feature = "fix_verify"))]
    let verify_help = String::new();
    eprintln!(
        "cargo-heal (Plan 005) — modelless code healing via latent corpus retrieval

USAGE:
    cargo heal --suggest [--domain <auto|clippy|perf|kernel>] [--top-k <N>] <path>...
    cargo heal --fix [--write] [--quiet] [--verify [--verify-args <s>]] [--top-k <N>] <path>...

    --suggest       Suggest mode: print matching rules per code span (no edits).
    --fix           Fix mode: apply the compiled-in proposers' span-preserving edits
                    ({} perf rules + {} clippy lints: {} | {}). The proposer's visitor
                    is the gate — latent suggestions never auto-apply.
    --write         With --fix: write the spliced file in place. Without it, print a
                    per-edit review (default: review only).
    --quiet         With --fix: skip the --suggest pass (only show fixes).
{}    --domain        Filter suggest output to one domain (default: auto — all enabled).
    --top-k <N>     Rules per span per domain (default 3).
    <path>...       Files or directories to scan (.rs files only).

Domains are feature-gated: clippy + perf are default-on; kernel needs
--features kernel_heal. Requesting a domain whose feature is off is an
error (T2.4 gating). --verify needs --features fix_verify.",
        FIXABLE_RULES.len(),
        FIXABLE_RULES.join(", "),
        FIXABLE_LINTS.len(),
        FIXABLE_LINTS.join(", "),
        verify_help
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
    verify: bool,
    verify_args: Option<String>,
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
        verify: false,
        verify_args: None,
        paths: Vec::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--suggest" => args.suggest = true,
            "--fix" => args.fix = true,
            "--write" => args.write = true,
            "--quiet" => args.quiet = true,
            "--verify" => args.verify = true,
            "--verify-args" => {
                let Some(value) = it.next() else {
                    eprintln!("--verify-args requires a value");
                    usage();
                };
                args.verify_args = Some(value);
            }
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
    if args.verify && !args.fix {
        eprintln!("--verify requires --fix");
        usage();
    }
    if args.verify_args.is_some() && !args.verify {
        eprintln!("--verify-args requires --verify");
        usage();
    }
    #[cfg(not(feature = "fix_verify"))]
    if args.verify {
        eprintln!("--verify requires a build with --features fix_verify");
        std::process::exit(2);
    }
    #[cfg(not(feature = "fix_verify"))]
    if args.verify_args.is_some() {
        eprintln!("--verify-args requires --verify (built with --features fix_verify)");
        std::process::exit(2);
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

    // Issue 018: self-evolve recording — the memory half runs in every
    // build (records applied/reverted fix trajectories + persists across
    // runs); the EMA learning half degrades gracefully without an oracle
    // (one log line). `RIIR_HEAL_SELF_EVOLVE=0` makes this fully inert.
    let mut evolve_rec = EvolveRecorder::from_env();
    let memory_path = default_memory_path();
    if evolve_rec.is_enabled() {
        match evolve_rec.load_verified(&memory_path) {
            Err(e) => eprintln!(
                "self-evolve: ignoring unreadable memory file ({}): {e}",
                memory_path.display()
            ),
            // T7.4: the commitment is advisory (the learned state is a hint,
            // not ground truth), so a mismatch warns and continues rather
            // than aborting a heal run.
            Ok(riir_clippy::CommitmentStatus::Mismatch) => eprintln!(
                "self-evolve: WARNING — memory file ({}) failed its BLAKE3 commitment; \
                 it was edited or corrupted since it was written. Continuing with it.",
                memory_path.display()
            ),
            Ok(_) => {}
        }
    }

    let mut n_chunks = 0usize;
    let mut n_suggestions = 0usize;
    let mut n_fixed_files = 0usize;
    let mut n_fixed_edits = 0usize;

    // Issue 015: under --verify the fix collection loosens the
    // vec-with-capacity usize gate (rustc backs the revert) and adds the
    // VERIFY_LINTS (type-blind under parse-only, revertible under rustc).
    #[cfg(feature = "fix_verify")]
    let proposer_opts = if args.verify {
        riir_clippy::domains::rust_perf::proposers::ProposerOpts { untyped_paths: true }
    } else {
        riir_clippy::domains::rust_perf::proposers::ProposerOpts::default()
    };

    // Issue 015: cheap Cargo.toml dep gate for fxhashmap — the rewrite
    // emits `use rustc_hash::FxHashMap`, which cannot resolve in a crate
    // without the dependency (the measured 422-edit katgpt-rs instance).
    // Hard-skip per owning crate; no cargo invocation needed.
    #[cfg(feature = "fix_verify")]
    let mut pkg_dep_cache: std::collections::HashMap<PathBuf, bool> =
        std::collections::HashMap::new();

    // Collected fixes per file — printed immediately when NOT verifying;
    // under --verify they are written + verified + reported after the
    // loop (reverted edits must not appear as "fixed" in the review).
    #[cfg(feature = "fix_verify")]
    let mut verify_batches: Vec<riir_clippy::fix_verify::FileEditSet> = Vec::new();

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
        //
        // Review line numbers are FINAL-source coordinates: each edit's byte
        // offset is recorded against its own pass's input, then remapped
        // through its own + every later pass's applied splices (own-pass
        // mapping is safe — see below; some fixes change line counts —
        // manual_map collapses multi-line matches, fxhashmap's use-append
        // inserts a line — so per-pass numbers would drift).
        const MAX_FIX_PASSES: usize = 4;
        let mut current = src.clone();
        // (pass, rule_id, start, end, replacement) — end + replacement for
        // the Issue 015 span-level revert records.
        let mut all_edits: Vec<(usize, String, usize, usize, String)> = Vec::new();
        // Per-pass applied splices (in that pass's input coordinates).
        let mut pass_splices: Vec<Vec<riir_clippy::edit_emitter::SourceEdit>> = Vec::new();
        // Per-pass INPUT texts (the span-revert restore source).
        let mut pass_texts: Vec<String> = Vec::new();
        if args.fix {
            // fxhashmap dep gate (Issue 015): skip the rule for files in
            // crates without the rustc-hash dependency.
            #[cfg(feature = "fix_verify")]
            let fxhashmap_allowed = {
                let pkg =
                    riir_clippy::fix_verify::owning_package_root(file);
                match pkg {
                    None => false,
                    Some(pkg) => *pkg_dep_cache.entry(pkg.clone()).or_insert_with(|| {
                        std::fs::read_to_string(pkg.join("Cargo.toml"))
                            .map(|t| {
                                riir_clippy::fix_verify::manifest_has_dep(&t, "rustc-hash")
                            })
                            .unwrap_or(false)
                    }),
                }
            };
            #[cfg(not(feature = "fix_verify"))]
            let fxhashmap_allowed = true;

            for pass in 0..MAX_FIX_PASSES {
                let Ok(parsed) = syn::parse_file(&current) else {
                    eprintln!("skip (unparseable): {display}");
                    break;
                };
                let mut edits: Vec<RuleEdit> = Vec::new();
                for rule in FIXABLE_RULES {
                    if *rule == "fxhashmap" && !fxhashmap_allowed {
                        continue;
                    }
                    #[cfg(feature = "fix_verify")]
                    {
                        edits.extend(
                            riir_clippy::domains::rust_perf::proposers::collect_edits_opts(
                                rule,
                                &current,
                                &parsed,
                                proposer_opts,
                            ),
                        );
                    }
                    #[cfg(not(feature = "fix_verify"))]
                    {
                        edits.extend(riir_clippy::domains::rust_perf::proposers::collect_edits(
                            rule, &current, &parsed,
                        ));
                    }
                }
                // `mut` only under fix_verify (the VERIFY_LINTS extend below).
                #[cfg(feature = "fix_verify")]
                let mut lint_set: Vec<&str> = FIXABLE_LINTS.to_vec();
                #[cfg(not(feature = "fix_verify"))]
                let lint_set: Vec<&str> = FIXABLE_LINTS.to_vec();
                #[cfg(feature = "fix_verify")]
                if args.verify {
                    lint_set.extend(VERIFY_LINTS.iter().copied());
                }
                for lint in lint_set {
                    edits.extend(
                        riir_clippy::domains::clippy_lints::proposers::collect_edits(
                            lint, &current, &parsed,
                        ),
                    );
                }
                if edits.is_empty() {
                    break;
                }
                let (next, splices) = riir_clippy::edit_emitter::apply_edits_traced(
                    &current,
                    edits.iter().map(|re| re.edit.clone()).collect(),
                );
                // Report only edits that SURVIVED selection — a pass-1 edit
                // dropped as overlapping (nested inside another fix's span)
                // gets re-collected + applied on pass 2; recording both
                // inflates the count + duplicates the review line.
                let applied: std::collections::HashSet<(usize, usize)> =
                    splices.iter().map(|s| (s.start, s.end)).collect();
                for re in &edits {
                    if applied.contains(&(re.edit.start, re.edit.end)) {
                        all_edits.push((
                            pass,
                            re.rule_id.clone(),
                            re.edit.start,
                            re.edit.end,
                            re.edit.replacement.clone(),
                        ));
                    }
                }
                pass_texts.push(current.clone());
                pass_splices.push(splices);
                let changed = next != current;
                current = next;
                if !changed {
                    break;
                }
            }
        }
        // Remap each edit's offset through ALL passes from its own onward
        // (own-pass mapping is safe: the edit's own splice breaks at
        // `start >= p` without self-applying), then express it as a line
        // number in the final text.
        let all_edits: Vec<(String, usize, String, String)> = all_edits
            .into_iter()
            .map(|(pass, rule_id, start, end, replacement)| {
                let mut pos = start;
                for splices in pass_splices.iter().skip(pass) {
                    pos = riir_clippy::edit_emitter::map_through_splices(pos, splices);
                }
                let line = current[..pos].matches('\n').count() + 1;
                let restore = pass_texts[pass]
                    .get(start..end)
                    .map(str::to_string)
                    .unwrap_or_default();
                (rule_id, line, replacement, restore)
            })
            .collect();

        #[cfg(feature = "fix_verify")]
        if args.verify {
            if !all_edits.is_empty() {
                verify_batches.push(riir_clippy::fix_verify::FileEditSet {
                    path: file.clone(),
                    original: src.clone(),
                    fixed: current.clone(),
                    edits: all_edits
                        .iter()
                        .map(|(rule, line, repl, restore)| {
                            riir_clippy::fix_verify::EditAtLine {
                                rule_id: rule.clone(),
                                final_line: *line,
                                replacement: repl.clone(),
                                restore: restore.clone(),
                            }
                        })
                        .collect(),
                });
            }
            // Suggest pass still runs below (unless --quiet) — the fix
            // review prints after verification.
        } else {
            print_fix_review(&display, &all_edits, args.write, file, &current);
            if !all_edits.is_empty() {
                n_fixed_files += 1;
                n_fixed_edits += all_edits.len();
            }
            // Issue 018: only WRITTEN edits are applied fixes — review-only
            // runs record nothing.
            if args.write {
                evolve_rec.record_reviewed(&all_edits);
            }
        }

        #[cfg(not(feature = "fix_verify"))]
        {
            print_fix_review(&display, &all_edits, args.write, file, &current);
            if !all_edits.is_empty() {
                n_fixed_files += 1;
                n_fixed_edits += all_edits.len();
            }
            // Issue 018: only WRITTEN edits are applied fixes — review-only
            // runs record nothing.
            if args.write {
                evolve_rec.record_reviewed(&all_edits);
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

    // ── Issue 015: the verify loop — write, cargo check, auto-revert.
    #[cfg(feature = "fix_verify")]
    if args.verify {
        let extra = args
            .verify_args
            .as_deref()
            .map(|s| s.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        let verifier = riir_clippy::fix_verify::FixVerifier::new().with_extra_args(extra);
        eprintln!(
            "verify: {} file(s) / {} edit(s) — baseline + compile-gated apply",
            verify_batches.len(),
            verify_batches.iter().map(|b| b.edits.len()).sum::<usize>()
        );
        let report = verifier.verify(&verify_batches);
        if let Some(err) = &report.error {
            eprintln!("verify: FAILED — {err}");
            std::process::exit(1);
        }
        if !report.ran {
            eprintln!("verify: nothing to verify (no edits)");
        } else {
            eprintln!(
                "verify: {} cargo check(s) — kept {} file(s) / {} edit(s), reverted {} edit(s)",
                report.n_checks, report.kept_files, report.kept_edits, report.reverted.len()
            );
            for r in &report.reverted {
                eprintln!(
                    "  reverted {}: {} @ line {}{}",
                    r.path.display(),
                    r.rule_id,
                    r.line,
                    if r.whole_file { " (whole file)" } else { "" }
                );
            }
        }
        // Post-verify review: report what SURVIVED (the fix review for
        // reverted edits would be misleading).
        for b in &verify_batches {
            let reverted_lines: std::collections::HashSet<usize> = report
                .reverted
                .iter()
                .filter(|r| r.path == b.path && !r.whole_file)
                .map(|r| r.line)
                .collect();
            let whole = report
                .reverted
                .iter()
                .any(|r| r.path == b.path && r.whole_file);
            if whole {
                println!("fix {} — REVERTED (whole file):", b.path.display());
                for e in &b.edits {
                    println!("  [{:<24}] line {} (reverted)", e.rule_id, e.final_line);
                }
                // Issue 018: a whole-file revert is a definitive fail verdict
                // for every edit in the file (attribution impossible — the
                // compile failure itself is the verdict).
                for e in &b.edits {
                    evolve_rec.record_outcome(&e.rule_id, &e.restore, &e.replacement, false);
                }
                continue;
            }
            let kept: Vec<&riir_clippy::fix_verify::EditAtLine> = b
                .edits
                .iter()
                .filter(|e| !reverted_lines.contains(&e.final_line))
                .collect();
            if kept.is_empty() {
                continue;
            }
            n_fixed_files += 1;
            n_fixed_edits += kept.len();
            // Issue 018: kept edits join the memory (a rustc pass proves
            // non-breakage, not lint resolution — memory-only, no verdict);
            // span-level reverts are definitive fails (sound to learn from
            // when an oracle build enabled learning).
            for e in &kept {
                evolve_rec.record_applied(&e.rule_id, &e.restore, &e.replacement);
            }
            for e in b.edits.iter().filter(|e| reverted_lines.contains(&e.final_line)) {
                evolve_rec.record_outcome(&e.rule_id, &e.restore, &e.replacement, false);
            }
            println!("fix {} ({} edit(s) verified):", b.path.display(), kept.len());
            for e in &kept {
                println!("  [{:<24}] line {}", e.rule_id, e.final_line);
            }
        }
    }
    // Issue 018: persist recorded trajectories (skip when nothing new).
    if evolve_rec.recorded_count() > 0 {
        if let Some(parent) = memory_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match evolve_rec.save(&memory_path) {
            Ok(()) => eprintln!(
                "self-evolve: {} trajectory(ies) → {}",
                evolve_rec.recorded_count(),
                memory_path.display()
            ),
            Err(e) => eprintln!(
                "self-evolve: memory save failed ({}): {e}",
                memory_path.display()
            ),
        }
    }

    let mode = if args.fix {
        #[cfg(feature = "fix_verify")]
        let applied_note = if args.verify {
            " (verified — compile-gated)"
        } else if args.write {
            " (written)"
        } else {
            " (review only — add --write)"
        };
        #[cfg(not(feature = "fix_verify"))]
        let applied_note = if args.write {
            " (written)"
        } else {
            " (review only — add --write)"
        };
        format!(
            ", fixed {} file(s) / {} edit(s){}",
            n_fixed_files, n_fixed_edits, applied_note
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
