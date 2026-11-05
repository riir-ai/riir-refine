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
use riir_refine::corpus_referee::{
    split_windows, AttemptRow, Enrollment, EnrollmentFile, RefereeInputs, Verdict, WindowTally,
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

fn fold_store(root: &Path) -> Result<(StoreFold, Vec<AttemptRow>), String> {
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
    let mut referee_rows = Vec::new();
    for row in store.dump() {
        fold.rows += 1;
        fold.fold_row(&row);
        referee_rows.push(AttemptRow::from(&row));
    }
    Ok((fold, referee_rows))
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

/// The enrolled rule's corpus domain — clippy first, then perf (the same
/// resolution order the bind arm uses; the 11-04 candidate is a rust_perf
/// rule). Unknown to both = perf default (the report still runs; the epoch
/// label is advisory bookkeeping).
#[must_use]
fn domain_of(rule: &str, clippy_ids: &[&str], perf_ids: &[&str]) -> &'static str {
    if clippy_ids.contains(&rule) {
        "clippy_lints"
    } else if perf_ids.contains(&rule) {
        "rust_perf"
    } else {
        // Unknown to both: default the perf domain (the epoch label is
        // advisory bookkeeping; the report still runs).
        "rust_perf"
    }
}

/// The rule's post-window failure rate (current − snapshot), for the
/// report line.
#[must_use]
fn post_rate_of(case: &Enrollment, current: &WindowTally) -> f64 {
    current
        .saturating_sub(&case.pre)
        .failure_rate()
        .unwrap_or(0.0)
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
    let mut referee: Option<String> = None;
    let mut referee_set = false;
    let mut enroll: Option<String> = None;
    let mut edit_epoch: Option<String> = None;
    let mut edit_ts: Option<u64> = None;
    let mut pre_attempts: Option<u32> = None;
    let mut pre_fails: Option<u32> = None;
    let mut batches: Option<u32> = None;
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
            "--referee" => {
                // Optional rule filter; bare --referee reports all cases.
                // A following token counts as the filter ONLY when it is
                // not another flag (the classic optional-value trap —
                // peeking, never blind next()).
                referee_set = true;
                if let Some(next) = argv.peek() {
                    if !next.starts_with('-') {
                        referee = Some(argv.next().expect("peeked"));
                    }
                }
            }
            "--enroll" => match argv.next() {
                Some(v) => enroll = Some(v),
                None => {
                    eprintln!("--enroll needs a rule id");
                    std::process::exit(2);
                }
            },
            "--edit-epoch" => match argv.next() {
                Some(v) => edit_epoch = Some(v),
                None => {
                    eprintln!("--edit-epoch needs the note's 8-hex epoch label");
                    std::process::exit(2);
                }
            },
            "--edit-ts" => match argv.next().and_then(|v| v.parse::<u64>().ok()) {
                Some(v) => edit_ts = Some(v),
                None => {
                    eprintln!("--edit-ts needs a unix-seconds number");
                    std::process::exit(2);
                }
            },
            "--pre-attempts" => match argv.next().and_then(|v| v.parse::<u32>().ok()) {
                Some(v) => pre_attempts = Some(v),
                None => {
                    eprintln!("--pre-attempts needs a number");
                    std::process::exit(2);
                }
            },
            "--pre-fails" => match argv.next().and_then(|v| v.parse::<u32>().ok()) {
                Some(v) => pre_fails = Some(v),
                None => {
                    eprintln!("--pre-fails needs a number");
                    std::process::exit(2);
                }
            },
            "--batches" => match argv.next().and_then(|v| v.parse::<u32>().ok()) {
                Some(v) => batches = Some(v),
                None => {
                    eprintln!("--batches needs a number (the corpus epochs observed since the edit)");
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
    // The referee's per-row view (controls + vintage need last_seen stamps;
    // the summary fold above aggregates them away) — ONE pass, two views.
    let mut referee_rows: Vec<AttemptRow> = Vec::new();
    for root in &roots {
        match fold_store(root) {
            Ok((f, rows)) => {
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
                referee_rows.extend(rows);
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

    // ── The enroll arm (issue 209 T5): seed a case from the note's own ────
    // diagnose-time values. The snapshot values ARE the landed note's
    // numbers (case #1: 9/18 at compile gate — the note text carries them).
    if let Some(rule) = enroll.as_deref() {
        let (Some(edit_epoch), Some(ts), Some(pa), Some(pf)) =
            (edit_epoch.clone(), edit_ts, pre_attempts, pre_fails)
        else {
            eprintln!(
                "--enroll needs --edit-epoch <8hex> --edit-ts <unix> \
                 --pre-attempts <N> --pre-fails <M> (the landed note's values)"
            );
            std::process::exit(2);
        };
        if pa < pf {
            eprintln!("--pre-attempts {pa} must be ≥ --pre-fails {pf}");
            std::process::exit(2);
        }
        // The snapshot's stage attribution: the 11-04 candidates were all
        // W-class compile-gate — the compile bucket carries the fails, the
        // applied bucket the successes.
        let mut file = EnrollmentFile::load(std::path::Path::new(".")).unwrap_or_else(|e| {
            eprintln!("enroll refused: {e}");
            std::process::exit(2);
        });
        file.upsert(Enrollment {
            rule: rule.to_string(),
            edit_epoch_short: edit_epoch.clone(),
            edit_ts: ts,
            pre: riir_refine::corpus_referee::WindowTally {
                parse_failures: 0,
                compile_failures: pf,
                verify_failures: 0,
                applied: pa - pf,
            },
            batches_observed: batches.unwrap_or(0),
            last_seen_epoch: None,
            snapshot_population: String::new(),
        });
        if let Err(e) = file.save(std::path::Path::new(".")) {
            eprintln!("enroll save failed: {e}");
            std::process::exit(2);
        }
        println!(
            "enrolled: {rule} epoch={edit_epoch} ts={ts} pre={} fail={} batches={} → {}",
            pa,
            pf,
            batches.unwrap_or(0),
            EnrollmentFile::path(std::path::Path::new(".")).display(),
        );
        return;
    }

    // ── The referee arm (issue 209 T4): read the enrolled cases. ─────────
    if referee_set {
        let want = referee.as_deref();
        let mut file = EnrollmentFile::load(std::path::Path::new(".")).unwrap_or_else(|e| {
            eprintln!("referee refused: {e}");
            std::process::exit(2);
        });
        if file.cases.is_empty() {
            println!("referee: no enrolled cases (enroll one with --enroll)");
            return;
        }
        let clippy_corpus = riir_refine::domains::clippy_lints::corpus::build_seed_corpus();
        let perf_corpus = riir_refine::domains::rust_perf::corpus::build_perf_seed_corpus();
        // Per-rule rows for the split + controls (the sweep above already
        // collected them once).
        let mut by_rule: BTreeMap<String, Vec<AttemptRow>> = BTreeMap::new();
        for row in referee_rows {
            let rule = row.rule_id.clone();
            by_rule.entry(rule).or_default().push(row);
        }
        // The live corpus epoch per domain (clippy first, then perf — the
        // same order the bind arm resolves; the batch counter tracks the
        // enrolled rule's OWN domain).
        let clippy_ids = clippy_corpus.rule_ids();
        let perf_ids = perf_corpus.rule_ids();
        let epoch_hex = |c: &[u8; 32]| c.iter().map(|b| format!("{b:02x}")).collect::<String>();
        // Both epochs computed eagerly — the two corpora are different
        // Corpus<D> instantiations and cannot share a binding.
        let (clippy_epoch, perf_epoch) = (epoch_hex(&clippy_corpus.commitment), epoch_hex(&perf_corpus.commitment));
        let live_epoch = |domain: &str| -> &str {
            if domain == "clippy_lints" { &clippy_epoch } else { &perf_epoch }
        };
        let mut any = false;
        for case in &mut file.cases {
            if let Some(w) = want {
                if case.rule != w {
                    continue;
                }
            }
            any = true;
            // Batch-counter bump: a corpus epoch the file has not seen
            // counts one observed batch (the first run seeds the stamp;
            // undercounting delays starvation — the conservative side).
            let live = live_epoch(domain_of(&case.rule, &clippy_ids, &perf_ids)).to_string();
            match &case.last_seen_epoch {
                None => case.last_seen_epoch = Some(live.clone()),
                Some(seen) if *seen != live => {
                    case.batches_observed += 1;
                    case.last_seen_epoch = Some(live.clone());
                }
                _ => {}
            }
            let rows = by_rule.get(&case.rule).map(Vec::as_slice).unwrap_or(&[]);
            let (rule_pre_rows, _rule_post_rows) = split_windows(rows, case.edit_ts);
            let _ = rule_pre_rows; // the snapshot IS the pre side (the split informs controls)
            let current = WindowTally::from_rule_outcomes(
                merged.get(&case.rule).unwrap_or(&RuleOutcomes::default()),
            );
            let rule_last_seen = merged_seen.get(&case.rule).copied().unwrap_or(0);
            // The control set: every OTHER rule with evidence on both sides
            // of the edit (the fold's split), never the enrolled rule.
            let mut controls: Vec<(String, WindowTally, WindowTally)> = Vec::new();
            for (rule, rows) in &by_rule {
                if rule == &case.rule {
                    continue;
                }
                let (cpre, cpost) = split_windows(rows, case.edit_ts);
                if cpre.attempts() > 0 && cpost.attempts() > 0 {
                    controls.push((rule.clone(), cpre, cpost));
                }
            }
            let inputs = RefereeInputs {
                rule_current: &current,
                rule_last_seen,
                controls: &controls,
                now,
                recent_floor: window_floor,
            };
            let verdict = riir_refine::corpus_referee::read(case, &inputs);
            println!();
            println!(
                "referee: {} (edit epoch {}, ts {}) batches={} controls={}",
                case.rule,
                case.edit_epoch_short,
                case.edit_ts,
                case.batches_observed,
                controls.len(),
            );
            if !case.snapshot_population.is_empty() {
                println!("  snapshot population: {}", case.snapshot_population);
            }
            match &verdict {
                Verdict::Pass { lcb_drop, drop, control_drift } | Verdict::Fail { lcb_drop, drop, control_drift } => {
                    println!(
                        "  {}: drop {drop:.3} (LCB95 {lcb_drop:.3}) vs control drift {control_drift:.3} — \
                         pre {:.3} → post {:.3} (n={} post-edit attempts)",
                        verdict.state(),
                        case.pre.failure_rate().unwrap_or(0.0),
                        post_rate_of(case, &current),
                        current.saturating_sub(&case.pre).attempts(),
                    );
                }
                Verdict::Pending { reason } => {
                    let post_attempts = current.saturating_sub(&case.pre).attempts();
                    println!(
                        "  PENDING ({reason:?}) — post-edit attempts so far: {post_attempts}/{}",
                        riir_refine::corpus_referee::MIN_POST_ATTEMPTS,
                    );
                }
                Verdict::Starved { post_attempts, batches_observed } => {
                    println!(
                        "  UNDECIDED-BY-STARVATION — {post_attempts}/{} post-edit attempts \
                         over {batches_observed} observed corpus epochs",
                        riir_refine::corpus_referee::MIN_POST_ATTEMPTS,
                    );
                }
            }
        }
        if !any {
            eprintln!("referee: no enrolled case names {}", want.unwrap_or(""));
            std::process::exit(2);
        }
        // Persist the bumped batch counters (a report that forgets its own
        // observations would re-count the same epoch forever).
        if let Err(e) = file.save(std::path::Path::new(".")) {
            eprintln!("referee save failed (counters): {e}");
            std::process::exit(2);
        }
    }
}
