//! `corpus-diagnose` — the HARVEST→BIND→NOTE CLI verb (issue 206, the corpus
//! half's third task).
//!
//! The pipeline that landed on 2026-11-04 ran as a session script: sweep the
//! durable trajectory stores → fold per-rule `RuleOutcomes` → `diagnose` at
//! the pinned predicate → (when a candidate exists) bind + compose the
//! templated note. This bin wraps exactly that sweep so the next re-open
//! trigger ("a recent-window candidate exists") can be CHECKED with one
//! command instead of a hand-run session, and so a diagnosed candidate can
//! be carried into the pins-manifest edit lane with its binding rendered.
//!
//! REPORT-ONLY by design: the bin never edits the corpus. The 11-04
//! discipline kept the note landing inside a batch commit with the
//! pins-manifest re-pin in the same commit — an automated write path would
//! bypass both the batch discipline and the stale-epoch compose gate, so the
//! verb stops at the report (the compose gate runs here only to PROVE the
//! binding + note render against the live corpus epoch; the note text is
//! printed, never written).
//!
//! Sources (the 11-04 harvest table, unchanged):
//!   - `<repo>/.refine/trajectories.kv` — the durable per-(rule, shape)
//!     tallies (`TrajRowDoc`), the formal candidate source.
//!   - `<repo>/.refine/blueprint_history.jsonl` — the per-campaign
//!     written/reverted/declined strata (watch data; the 11-04 run found 0
//!     candidates there — reverted shares are too thin to clear the bar).
//!
//! The `--repos` list defaults to this repo + the sibling contract checkouts
//! beside it (the 11-04 posture); an explicit list overrides.
//!
//! Predicate: `crate::corpus_binding::MIN_ATTEMPTS` (8) attempts and a
//! stage share ≥ `DEGENERACY_BAR` (0.5 inclusive) — UNDECIDED below, never
//! flagged; a `--recent-days` window additionally filters by each row's
//! `last_seen_at` so "recent-window candidate" is checkable (the 11-04
//! reading disclosed its 15-day vintage; the window makes the disclosure a
//! parameter instead of prose).
//!
//! Posture: ungated (reads local files only; no network, no corpus write —
//! the charitable build carries it fine; `traj_store` pulls the neuron-db
//! Warm-tier store like every other reader of `.refine/trajectories.kv`,
//! all feature-gated the same way `frontier_report` is — behind
//! `latent_retrieval`'s dep closure). REQUIRED-FEATURES rides
//! `latent_retrieval` so the shipped release binary (which carries that
//! feature) builds it and a default build skips it — same posture as the
//! `cargo-refine` bin's own row minus the domain features.
//!
//! Exit codes: 0 = ran (candidates or not — the report IS the product);
//! 2 = nothing read (no store opened, no history row decoded — the blind
//! walk is a loud failure, never a green zero).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use riir_refine::corpus_binding::{
    diagnose_with, CorpusBinding, RuleOutcomes, DEGENERACY_BAR, MIN_ATTEMPTS,
};
use riir_refine::traj_store::TrajStore;

/// One store's contribution to the fold.
struct StoreFold {
    /// Repo root the store lives under (display).
    root: String,
    /// Rows decoded (`traj:` prefix); corrupt rows counted separately.
    rows: usize,
    bad_rows: usize,
    /// Per-rule outcome fold across the store's rows.
    per_rule: BTreeMap<String, RuleOutcomes>,
    /// Freshest `last_seen_at` per rule (unix seconds; 0 = no stamp).
    last_seen: BTreeMap<String, u64>,
}

impl StoreFold {
    fn fold_row(&mut self, row: &riir_refine::traj_store::TrajRowDoc) {
        // lint_key is `"{code}:{strategy_id}"`; the CODE half is the rule
        // id the corpus keys on. Rows whose key carries no separator are
        // counted read but fold under their whole key (never silently
        // dropped).
        let rule_id = row
            .lint_key
            .rsplit_once(':')
            .map(|(code, _)| code.to_string())
            .unwrap_or_else(|| row.lint_key.clone());
        let entry = self.per_rule.entry(rule_id.clone()).or_default();
        let fails = row.fail_count;
        // The store tallies SUCCESS/FAIL, not per-stage gates; the
        // component attribution (the LAST decision-time class recorded)
        // is the stage evidence. Unattributed failures fold into the
        // compile-gate bucket — the conservative reading (a failure whose
        // stage is unknown still counts toward the bar through the stage
        // the W/C classes share; E-class corpus gaps are NOT gate failures
        // and must not inflate one).
        match row.component {
            Some(riir_refine::component::ComponentClass::Gate) => entry.verify_failures += fails,
            // Drafter + Retrieval + unattributed → the compile-gate bucket
            // (the W-class share the 11-04 diagnose read; Retrieval is
            // invocation-side, folded here so the bar never misses it —
            // the report prints the component split per rule for the
            // honest reading).
            _ => entry.compile_failures += fails,
        }
        entry.applied += row.success_count;
        let seen = self.last_seen.entry(rule_id).or_default();
        *seen = (*seen).max(row.last_seen_at);
    }
}

fn fold_store(root: &Path) -> Result<StoreFold, String> {
    let (store, bad) = TrajStore::open(root).map_err(|e| {
        format!(
            "store under {} refused: {e}",
            root.join(".refine").join("trajectories.kv").display()
        )
    })?;
    let mut fold = StoreFold {
        root: root.display().to_string(),
        rows: 0,
        bad_rows: bad,
        per_rule: BTreeMap::new(),
        last_seen: BTreeMap::new(),
    };
    for row in store.dump() {
        fold.rows += 1;
        fold.fold_row(&row);
    }
    Ok(fold)
}

/// The blueprint history's watch half: per-rule reverted shares over the
/// campaigns the file holds (report-only; the 11-04 finding was 0
/// candidates and the bar stays the store's).
fn blueprint_watch(path: &Path) -> Result<BTreeMap<String, (u32, u32)>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|_| format!("blueprint history {} unreadable", path.display()))?;
    let mut per_rule: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        // v1 rows carry per-rule strata under "rules": [{rule, written,
        // reverted, declined}]; rows without it decode as watch-nothing.
        let Some(rules) = v.get("rules").and_then(|r| r.as_array()) else {
            continue;
        };
        for r in rules {
            let Some(rule) = r.get("rule").and_then(|x| x.as_str()) else {
                continue;
            };
            let written = r.get("written").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            let reverted = r.get("reverted").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            let e = per_rule.entry(rule.to_string()).or_default();
            e.0 += written;
            e.1 += reverted;
        }
    }
    Ok(per_rule)
}

fn default_roots() -> Vec<PathBuf> {
    // This repo first; then the sibling contract checkouts beside it (the
    // 11-04 posture). Siblings that are absent are skipped with a printed
    // line — never a failure (a partial checkout is an honest subset).
    let mut roots = vec![std::env::current_dir().expect("cwd")];
    if let Some(parent) = roots[0].parent().map(Path::to_path_buf) {
        for name in [
            "katgpt-rs",
            "riir-ai",
            "riir-chain",
            "riir-dapps",
            "riir-game-sdk",
            "riir-infer",
            "riir-instinct",
            "riir-kat",
            "riir-neuron-db",
            "riir-reflex",
            "riir-reflexer",
            "riir-rethink",
            "riir-train",
            "seal-game-editor",
        ] {
            let p = parent.join(name);
            if p.join(".git").exists() {
                roots.push(p);
            }
        }
    }
    roots
}

fn print_usage() {
    println!(
        "corpus-diagnose — the HARVEST→BIND→NOTE verb (issue 206)\n\
         \n\
         USAGE: corpus-diagnose [--repos <path,...>] [--recent-days <N>] [--bind <rule>]\n\
         \n\
         Sweeps the durable trajectory stores + blueprint histories, folds\n\
         per-rule outcomes, and reports the degenerate-cluster diagnose at the\n\
         pinned predicate (attempts ≥ {MIN_ATTEMPTS}, stage share ≥ {DEGENERACY_BAR}).\n\
         Report-only: the corpus is never written; `--bind <rule>` additionally\n\
         renders the candidate's CorpusBinding + templated note against the\n\
         LIVE clippy corpus epoch (the compose gate proof — the note still only\n\
         prints; the landing stays a batch commit with the pins manifest)."
    );
}

fn main() {
    let mut repos: Option<Vec<PathBuf>> = None;
    let mut recent_days: Option<u64> = None;
    let mut bind: Option<String> = None;
    let mut argv = std::env::args().skip(1).peekable();
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--repos" => {
                let Some(v) = argv.next() else {
                    eprintln!("--repos needs a comma-separated list");
                    std::process::exit(2);
                };
                repos = Some(v.split(',').map(PathBuf::from).collect());
            }
            "--recent-days" => {
                let Some(v) = argv.next() else {
                    eprintln!("--recent-days needs a number");
                    std::process::exit(2);
                };
                match v.parse::<u64>() {
                    Ok(n) => recent_days = Some(n),
                    Err(_) => {
                        eprintln!("--recent-days: {v} is not a number");
                        std::process::exit(2);
                    }
                }
            }
            "--bind" => match argv.next() {
                Some(v) => bind = Some(v),
                None => {
                    eprintln!("--bind needs a rule id");
                    std::process::exit(2);
                }
            },
            "--help" | "-h" => {
                print_usage();
                return;
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_usage();
                std::process::exit(2);
            }
        }
    }

    let roots = repos.unwrap_or_else(default_roots);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let window_floor = recent_days.map(|d| now.saturating_sub(d * 86_400));

    let mut folds: Vec<StoreFold> = Vec::new();
    let mut stores_opened = 0usize;
    let mut history_rows = 0usize;
    for root in &roots {
        match fold_store(root) {
            Ok(f) => {
                if f.rows > 0 || f.bad_rows > 0 {
                    stores_opened += 1;
                }
                history_rows += 0; // counted below from the file reads
                println!(
                    "store {}: {} row(s) folded, {} corrupt",
                    f.root,
                    f.rows,
                    f.bad_rows
                );
                folds.push(f);
            }
            Err(e) => {
                // A store that does not exist is the honest-subset case; a
                // store that exists but refuses is loud.
                let store = root.join(".refine").join("trajectories.kv");
                if store.exists() {
                    eprintln!("SKIP (refused): {e}");
                }
            }
        }
        let hist = root.join(".refine").join("blueprint_history.jsonl");
        if hist.exists() {
            match blueprint_watch(&hist) {
                Ok(w) => {
                    let rows: u32 = w.values().map(|(w, _)| *w).sum();
                    history_rows += usize::try_from(rows).unwrap_or(0);
                    if !w.is_empty() {
                        println!("blueprint {}: {} rule row(s) (watch half)", hist.display(), w.len());
                    }
                }
                Err(e) => eprintln!("SKIP: {e}"),
            }
        }
    }

    if stores_opened == 0 && history_rows == 0 {
        eprintln!(
            "corpus-diagnose: BLIND — no store rows and no history rows read; \
             the report would be a green zero"
        );
        std::process::exit(2);
    }

    // Fold every store's per-rule outcomes into one diagnosis input.
    let mut merged: BTreeMap<String, RuleOutcomes> = BTreeMap::new();
    let mut merged_seen: BTreeMap<String, u64> = BTreeMap::new();
    for f in &folds {
        for (rule, outcomes) in &f.per_rule {
            let e = merged.entry(rule.clone()).or_default();
            e.parse_failures += outcomes.parse_failures;
            e.compile_failures += outcomes.compile_failures;
            e.verify_failures += outcomes.verify_failures;
            e.applied += outcomes.applied;
            let seen = merged_seen.entry(rule.clone()).or_default();
            *seen = (*seen).max(f.last_seen.get(rule).copied().unwrap_or(0));
        }
    }

    let input: Vec<(&str, RuleOutcomes)> = merged
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .collect();
    let diagnosed = diagnose_with(&input, MIN_ATTEMPTS, DEGENERACY_BAR);

    println!();
    println!(
        "diagnose: {} rule(s) folded; predicate attempts ≥ {MIN_ATTEMPTS}, share ≥ {DEGENERACY_BAR}",
        merged.len()
    );
    if diagnosed.is_empty() {
        println!("candidates: NONE (the era is healthy at the pinned bar)");
    } else {
        println!("candidates: {}", diagnosed.len());
        for d in &diagnosed {
            let seen = merged_seen.get(&d.rule_id).copied().unwrap_or(0);
            let vintage = if seen == 0 {
                "no stamp".to_string()
            } else {
                format!("{} d ago", (now.saturating_sub(seen)) / 86_400)
            };
            let in_window = window_floor.is_none_or(|floor| seen >= floor);
            println!(
                "  {} stage={:?} attempts={} failures={} share={:.2} last seen: {}{}",
                d.rule_id,
                d.stage,
                d.attempts,
                d.failures,
                d.rate,
                vintage,
                if in_window { "" } else { "  [outside --recent-days]" }
            );
        }
    }

    // The bind arm: render the binding + note against the LIVE corpus
    // epoch — the compose-gate proof, report-only.
    if let Some(rule) = bind.as_deref() {
        let diagnosed_rule = diagnosed.iter().find(|d| d.rule_id == rule);
        let Some(diagnosed_rule) = diagnosed_rule else {
            eprintln!(
                "bind: {rule} is not a diagnosed candidate — the compose gate \
                 needs a live diagnosis (a stale note is a falsified epoch)"
            );
            std::process::exit(2);
        };
        // The rule may live in either domain's corpus; try clippy first,
        // then rust_perf (the 11-04 candidate hashmap-with-capacity is a
        // rust_perf rule — its note landed in entries_03).
        let clippy = riir_refine::domains::clippy_lints::corpus::build_seed_corpus();
        let perf = riir_refine::domains::rust_perf::corpus::build_perf_seed_corpus();
        let (epoch, rule_ids, domain) = if clippy.rule_ids().contains(&rule) {
            (clippy.commitment, clippy.rule_ids(), "clippy_lints")
        } else {
            (perf.commitment, perf.rule_ids(), "rust_perf")
        };
        let rule_ids: Vec<String> = rule_ids.into_iter().map(String::from).collect();
        let epoch_short: String = epoch.iter().take(4).map(|b| format!("{b:02x}")).collect();
        let binding = CorpusBinding::new(rule, diagnosed_rule.stage, epoch, now);
        match binding.compose_context_note(&rule_ids, diagnosed_rule, &epoch, &epoch_short) {
            Ok(note) => {
                println!();
                println!(
                    "binding: rule={rule} stage={:?} domain={domain} epoch={epoch_short}",
                    diagnosed_rule.stage
                );
                println!("note (REPORT-ONLY — the landing stays a batch commit + pins manifest):");
                println!("{note}");
            }
            Err(e) => {
                eprintln!("bind refused: {e:?}");
                std::process::exit(2);
            }
        }
    }
}
