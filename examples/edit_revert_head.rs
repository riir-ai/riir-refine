//! Issue 250 T2 — the v1 BCE trained head (REPORT-ONLY, exit 0 on every
//! successful read; a BLIND read refuses exit 1).
//!
//! The T1 similarity floor FAILED on the real pool (issue 250, 2026-11-24):
//! similarity-to-past-accepted-fixes does not separate compile-reverts from
//! keeps. T2 is the pre-registered next arm: a BCE linear head over HASHED
//! FEATURES (hunk n-grams + token-type + rule-hash + edit-shape), trained by
//! plain SGD on the FIT side, scored on the EVAL side with the SAME
//! decline-guard simulation and the SAME pre-registered bars as T1
//! (G3 >= 10% realized check reduction, false declines <= 1% of kept, and the
//! 3.9% per-rule prior to beat). The escalation stop rides the issue text:
//! v1 PR-AUC ~= the similarity floor within noise => STOP (the T3 GPU head
//! never opens).
//!
//! Std-only: FNV-1a feature hashing, logistic SGD, no deps. The row schema is
//! the shared `EditEventRow` (read through the crate, never re-parsed by
//! hand). Class imbalance is disclosed: PR-AUC over the EVAL reverts, with
//! the support count printed beside it — a thin positive class reads as
//! honest UNDECIDABLE-thin, never folded into a verdict.
//!
//! Run:
//!   cargo run --release --example edit_revert_head -- \
//!     [--events <path/to/.refine/edit_events.jsonl>]... [--min-runs <n>] \
//!     [--epochs <n>] [--l2 <f>] [--lr <f>] [--seed <u64>] [ROOT]...
//!
//! Positional ROOTs resolve to `<ROOT>/.refine/edit_events.jsonl` like the
//! census; explicit `--events` files must exist (exit 2 otherwise); below
//! `--min-runs` (default 4) it prints the accumulation note and exits 0.

use std::collections::HashMap;
use std::path::PathBuf;

use riir_refine::blueprint::{EDIT_EVENT_VERSION, EditEventRow};

const USAGE: &str = "usage: edit_revert_head [--events <edit_events.jsonl>]... [--min-runs <n>] \
[--epochs <n>] [--l2 <f>] [--lr <f>] [--seed <u64>] [REPO_ROOT]...";

/// False-decline bar: fraction of kept edits a wrong decline may burn (T1a law).
const FALSE_DECLINE_BAR: f64 = 0.01;
/// G3 bar: realized check reduction the decline-guard must show (derived at T0).
const G3_BAR: f64 = 0.10;
/// A run spending more than OUTLIER_MULT x the median `run_checks` leaves the
/// headline reduction (both columns printed) — the census posture verbatim.
const OUTLIER_MULT: f64 = 5.0;
/// The per-rule prior's realized reduction (T1a, Bench 162 addendum) — the bar to beat.
const PRIOR_TO_BEAT: f64 = 0.039;

const FEATURE_BUCKETS: u64 = 1 << 18;

// ---------- input ----------

struct SourceLog {
    name: String,
    rows: Vec<EditEventRow>,
}

fn collect_logs(events: &[PathBuf], roots: &[PathBuf]) -> Vec<SourceLog> {
    let mut logs = Vec::new();
    for path in events {
        match std::fs::read_to_string(path) {
            Ok(text) => logs.push(parse_log(&path.display().to_string(), &text)),
            Err(e) => {
                eprintln!("refusal: --events {} unreadable: {e}", path.display());
                std::process::exit(2);
            }
        }
    }
    for root in roots {
        let path = root.join(".refine").join("edit_events.jsonl");
        if !path.exists() {
            eprintln!("skip: {} missing (no runs yet)", path.display());
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => logs.push(parse_log(&path.display().to_string(), &text)),
            Err(e) => {
                eprintln!("refusal: {} unreadable: {e}", path.display());
                std::process::exit(2);
            }
        }
    }
    logs
}

fn parse_log(name: &str, text: &str) -> SourceLog {
    let mut rows = Vec::new();
    let mut malformed = 0usize;
    let mut other_version = 0usize;
    for l in text.lines() {
        if l.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<EditEventRow>(l) {
            Ok(r) if r.v == EDIT_EVENT_VERSION => rows.push(r),
            Ok(_) => other_version += 1,
            Err(_) => malformed += 1,
        }
    }
    if malformed > 0 || other_version > 0 {
        eprintln!(
            "note: {name}: {malformed} malformed, {other_version} other-version row(s) skipped"
        );
    }
    SourceLog {
        name: name.to_string(),
        rows,
    }
}

// ---------- run grouping + split (the census protocol verbatim) ----------

#[derive(Clone)]
struct Run {
    ts: u64,
    /// indices into the FLAT global row vec
    rows: Vec<usize>,
    run_checks: u32,
}

fn group_runs(logs: &[SourceLog]) -> (Vec<EditEventRow>, Vec<Run>) {
    let mut flat: Vec<EditEventRow> = Vec::new();
    let mut order: Vec<String> = Vec::new();
    let mut map: HashMap<String, Run> = HashMap::new();
    for log in logs {
        for row in &log.rows {
            let idx = flat.len();
            flat.push(row.clone());
            let key = format!("{}::{}", log.name, row.run_id);
            if !map.contains_key(&key) {
                order.push(key.clone());
                map.insert(
                    key.clone(),
                    Run {
                        ts: row.ts,
                        rows: Vec::new(),
                        run_checks: row.run_checks,
                    },
                );
            }
            let run = map.get_mut(&key).expect("just inserted");
            run.rows.push(idx);
            run.ts = run.ts.min(row.ts);
        }
    }
    (
        flat,
        order
            .into_iter()
            .map(|k| map.remove(&k).expect("present"))
            .collect(),
    )
}

// ---------- features (hashed, std-only) ----------

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Tokenize a text hunk into lowercase-ish word tokens (std-only).
fn tokens(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .map(|t| if t.len() > 24 { &t[..24] } else { t })
        .collect()
}

fn push_ngrams(feats: &mut Vec<u64>, text: &str, tag: &[u8]) {
    let toks = tokens(text);
    for t in &toks {
        let mut buf = tag.to_vec();
        buf.extend_from_slice(t.as_bytes());
        feats.push(fnv1a(&buf) % FEATURE_BUCKETS);
    }
    for w in toks.windows(2) {
        let mut buf = tag.to_vec();
        buf.extend_from_slice(w[0].as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(w[1].as_bytes());
        feats.push(fnv1a(&buf) % FEATURE_BUCKETS);
    }
}

/// Token-type census: identifier / digit share buckets.
fn push_shape(feats: &mut Vec<u64>, text: &str, tag: &[u8]) {
    let mut ident = 0usize;
    let mut digits = 0usize;
    let mut punct = 0usize;
    for c in text.chars() {
        if c.is_alphabetic() || c == '_' {
            ident += 1;
        } else if c.is_ascii_digit() {
            digits += 1;
        } else if !c.is_whitespace() {
            punct += 1;
        }
    }
    let total = (ident + digits + punct).max(1);
    let mut buf = tag.to_vec();
    buf.extend_from_slice(format!("id{}", ident * 8 / total).as_bytes());
    feats.push(fnv1a(&buf) % FEATURE_BUCKETS);
    let mut buf = tag.to_vec();
    buf.extend_from_slice(format!("dg{}", digits * 8 / total).as_bytes());
    feats.push(fnv1a(&buf) % FEATURE_BUCKETS);
}

fn features(row: &EditEventRow) -> Vec<u64> {
    let mut f = Vec::with_capacity(64);
    // rule hash (unseen rules land in unseen buckets — fine for a linear head)
    let mut buf = b"rule:".to_vec();
    buf.extend_from_slice(row.rule.as_bytes());
    f.push(fnv1a(&buf) % FEATURE_BUCKETS);
    // path extension (edit-shape context)
    let ext = std::path::Path::new(&row.path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let mut buf = b"ext:".to_vec();
    buf.extend_from_slice(ext.as_bytes());
    f.push(fnv1a(&buf) % FEATURE_BUCKETS);
    // edit-shape scalars, bucketed
    let dl = row.restore.lines().count() as u64;
    let nl = row.replacement.lines().count() as u64;
    for (tag, v) in [("dl", dl), ("nl", nl)] {
        let mut buf = tag.as_bytes().to_vec();
        buf.extend_from_slice(format!("{}", v.next_power_of_two()).as_bytes());
        f.push(fnv1a(&buf) % FEATURE_BUCKETS);
    }
    if row.whole_file {
        f.push(fnv1a(b"whole_file") % FEATURE_BUCKETS);
    }
    // hunk n-grams, restore and replacement tagged separately
    push_ngrams(&mut f, &row.restore, b"r:");
    push_ngrams(&mut f, &row.replacement, b"w:");
    push_shape(&mut f, &row.restore, b"sr:");
    push_shape(&mut f, &row.replacement, b"sw:");
    f
}

// ---------- the head: logistic SGD over hashed features ----------

struct Head {
    w: Vec<f32>,
    bias: f32,
}

impl Head {
    fn new() -> Self {
        Self {
            w: vec![0.0; FEATURE_BUCKETS as usize],
            bias: 0.0,
        }
    }

    fn train(&mut self, samples: &[(Vec<u64>, bool)], epochs: usize, lr: f64, l2: f64, seed: u64) {
        let mut state = seed;
        let mut next = move || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        let n = samples.len();
        if n == 0 {
            return;
        }
        for _ in 0..epochs {
            let mut idx: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let j = (next() % (i as u64 + 1)) as usize;
                idx.swap(i, j);
            }
            for &i in &idx {
                let (feats, reverted) = &samples[i];
                let z = self.bias + feats.iter().map(|&f| self.w[f as usize]).sum::<f32>();
                let p = 1.0 / (1.0 + (-z as f64).exp());
                let g = (p - (*reverted as i32 as f64)) as f32;
                self.bias -= (lr as f32) * g;
                for &f in feats {
                    let wi = f as usize;
                    self.w[wi] -= (lr as f32) * (g + (l2 as f32) * self.w[wi]);
                }
            }
        }
    }

    fn score(&self, feats: &[u64]) -> f64 {
        let z = self.bias + feats.iter().map(|&f| self.w[f as usize]).sum::<f32>();
        1.0 / (1.0 + (-z as f64).exp())
    }
}

// ---------- PR-AUC ----------

fn pr_auc(scores: &[(f64, bool)]) -> Option<f64> {
    let pos = scores.iter().filter(|(_, r)| *r).count();
    if pos == 0 {
        return None;
    }
    let mut ranked: Vec<(f64, bool)> = scores.to_vec();
    // descending by score, ties broken positives-first (the optimistic
    // convention, disclosed)
    ranked.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.1.cmp(&a.1))
    });
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut pts: Vec<(f64, f64)> = Vec::new(); // (recall, precision)
    for (_, r) in &ranked {
        if *r {
            tp += 1;
        } else {
            fp += 1;
        }
        pts.push((tp as f64 / pos as f64, tp as f64 / (tp + fp) as f64));
    }
    let mut auc = 0.0;
    let mut prev = (0.0_f64, pts[0].1);
    for &(rec, prec) in pts.iter().skip(1) {
        auc += (rec - prev.0) * (prec + prev.1) / 2.0;
        prev = (rec, prec);
    }
    Some(auc)
}

// ---------- decline-guard simulation (the census cost semantics verbatim) ----------

fn sweep(eval_runs: &[Run], flat: &[EditEventRow], scores: &HashMap<usize, f64>, tau_grid: &[f64]) {
    let mut checks: Vec<u32> = eval_runs.iter().map(|r| r.run_checks).collect();
    checks.sort_unstable();
    let checks_median = *checks.get(checks.len() / 2).unwrap_or(&0) as f64;
    println!(
        "  tau  decl  corr false  false%   red%  red_ex%  (outlier = > {}x median {})",
        OUTLIER_MULT, checks_median
    );
    let mut best = None;
    for &tau in tau_grid {
        let mut false_declines = 0usize;
        let mut kept = 0usize;
        let mut correct = 0usize;
        for run in eval_runs {
            for &idx in &run.rows {
                let row = &flat[idx];
                let s = scores.get(&idx).copied().unwrap_or(0.0);
                if !row.reverted {
                    kept += 1;
                    if s >= tau {
                        false_declines += 1;
                    }
                } else if s >= tau {
                    correct += 1;
                }
            }
        }
        if kept == 0 {
            continue;
        }
        let false_pct = 100.0 * false_declines as f64 / kept as f64;
        let (red, red_ex) = realized_reduction(eval_runs, flat, scores, tau);
        println!(
            " {:>5.2} {:>5} {:>5} {:>5} {:>7.2} {:>6.1} {:>7.1}",
            tau,
            correct + false_declines,
            correct,
            false_declines,
            false_pct,
            100.0 * red,
            100.0 * red_ex
        );
        if false_pct <= 100.0 * FALSE_DECLINE_BAR
            && red_ex >= G3_BAR
            && (correct + false_declines) > 0
        {
            best = Some((tau, false_pct, red_ex));
        }
    }
    match best {
        Some((tau, fp, red)) => println!(
            "best tau: {} — false declines {:.2}% <= {} bar AND realized reduction {:.1}% >= {} bar (declines > 0 required — decline-nothing is vacuous)",
            tau,
            fp,
            100.0 * FALSE_DECLINE_BAR,
            100.0 * red,
            100.0 * G3_BAR
        ),
        None => println!(
            "best tau: none adjudicable — every tau either misses a bar or declines nothing (decline-nothing clears the false bar vacuously; its red% column is the 2-check model baseline, not savings)"
        ),
    }
}

fn realized_reduction(
    eval_runs: &[Run],
    flat: &[EditEventRow],
    scores: &HashMap<usize, f64>,
    tau: f64,
) -> (f64, f64) {
    let mut checks: Vec<u32> = eval_runs.iter().map(|r| r.run_checks).collect();
    checks.sort_unstable();
    let checks_median = *checks.get(checks.len() / 2).unwrap_or(&0) as f64;
    let mut act_total = 0.0f64;
    let mut sim_total = 0.0f64;
    let mut act_ex = 0.0f64;
    let mut sim_ex = 0.0f64;
    let mut any_ex = false;
    for run in eval_runs {
        let remaining_kept = run
            .rows
            .iter()
            .filter(|&&i| {
                let row = &flat[i];
                !row.reverted && scores.get(&i).copied().unwrap_or(0.0) < tau
            })
            .count();
        let sim = if remaining_kept == 0 { 0.0 } else { 2.0 };
        let act = run.run_checks as f64;
        act_total += act;
        sim_total += sim;
        let outlier = checks_median > 0.0 && act > OUTLIER_MULT * checks_median;
        if !outlier {
            act_ex += act;
            sim_ex += sim;
            any_ex = true;
        }
    }
    let red = if act_total > 0.0 {
        (act_total - sim_total) / act_total
    } else {
        0.0
    };
    let red_ex = if any_ex && act_ex > 0.0 {
        (act_ex - sim_ex) / act_ex
    } else {
        red
    };
    (red, red_ex)
}

// ---------- main ----------

fn main() {
    let mut events: Vec<PathBuf> = Vec::new();
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut min_runs = 4usize;
    let mut epochs = 200usize;
    let mut l2 = 1e-6f64;
    let mut lr = 0.05f64;
    let mut seed = 0x2500u64;
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    while !args.is_empty() {
        let arg = args.remove(0);
        match arg.as_str() {
            "--events" => {
                if args.is_empty() {
                    eprintln!("{USAGE}");
                    std::process::exit(2);
                }
                events.push(PathBuf::from(args.remove(0)));
            }
            "--min-runs" => {
                if args.is_empty() {
                    eprintln!("{USAGE}");
                    std::process::exit(2);
                }
                min_runs = args.remove(0).parse().expect("--min-runs <n>");
            }
            "--epochs" => {
                if args.is_empty() {
                    eprintln!("{USAGE}");
                    std::process::exit(2);
                }
                epochs = args.remove(0).parse().expect("--epochs <n>");
            }
            "--l2" => {
                if args.is_empty() {
                    eprintln!("{USAGE}");
                    std::process::exit(2);
                }
                l2 = args.remove(0).parse().expect("--l2 <f>");
            }
            "--lr" => {
                if args.is_empty() {
                    eprintln!("{USAGE}");
                    std::process::exit(2);
                }
                lr = args.remove(0).parse().expect("--lr <f>");
            }
            "--seed" => {
                if args.is_empty() {
                    eprintln!("{USAGE}");
                    std::process::exit(2);
                }
                seed = args.remove(0).parse().expect("--seed <u64>");
            }
            "--help" => {
                println!("{USAGE}");
                return;
            }
            a => {
                roots.push(PathBuf::from(a));
            }
        }
    }

    println!("== i250 T2 v1 BCE trained head (hashed features, logistic SGD) ==");
    println!(
        "bars: G3 >= {} realized check reduction | false declines <= {} of kept | beat the per-rule prior {} | escalation stop: PR-AUC ~= floor within noise",
        G3_BAR, FALSE_DECLINE_BAR, PRIOR_TO_BEAT
    );

    let logs = collect_logs(&events, &roots);
    if logs.is_empty() {
        eprintln!("refusal: no logs to read (BLIND)");
        std::process::exit(1);
    }
    let (flat, runs) = group_runs(&logs);
    let total_rows = flat.len();
    println!(
        "inputs: {} log(s) · {} row(s) · runs {} (fit/eval split needs >= {})",
        logs.len(),
        total_rows,
        runs.len(),
        min_runs
    );
    if runs.len() < min_runs {
        println!(
            "floor run GATED: {} run(s) accumulated, need >= {} for the FIT/EVAL split — rows accrue from real fix runs; re-run as they land",
            runs.len(),
            min_runs
        );
        return;
    }

    // census split protocol: runs sorted by ts, FIT = ts < median
    let mut ts_sorted: Vec<u64> = runs.iter().map(|r| r.ts).collect();
    ts_sorted.sort_unstable();
    let median = ts_sorted[ts_sorted.len() / 2];
    let mut fit_rows: Vec<usize> = Vec::new();
    let mut eval_runs: Vec<&Run> = Vec::new();
    for run in &runs {
        if run.ts < median {
            fit_rows.extend(run.rows.iter().copied());
        } else {
            eval_runs.push(run);
        }
    }
    let fit_reverts = fit_rows.iter().filter(|&&i| flat[i].reverted).count();
    let eval_edits: usize = eval_runs.iter().map(|r| r.rows.len()).sum();
    let eval_reverts = eval_runs
        .iter()
        .flat_map(|r| r.rows.iter())
        .filter(|&&i| flat[i].reverted)
        .count();
    println!(
        "split: fit runs {} (rows {}, reverts {}) · eval runs {} (edits {}, reverts {})",
        runs.len() - eval_runs.len(),
        fit_rows.len(),
        fit_reverts,
        eval_runs.len(),
        eval_edits,
        eval_reverts
    );
    if fit_reverts == 0 {
        println!(
            "UNDECIDABLE-thin: the FIT side carries NO revert examples — the head has nothing to learn; accrual continues"
        );
        return;
    }
    if fit_reverts < 5 {
        println!(
            "NOTE: the FIT side carries only {} revert example(s) — the head learns from a thin positive class; every verdict below is UNDECIDABLE-thin until more revert-bearing runs accrue",
            fit_reverts
        );
    }

    // degenerate rows (both texts blank) are excluded everywhere, census law
    let degenerate =
        |i: usize| flat[i].restore.trim().is_empty() && flat[i].replacement.trim().is_empty();
    let train: Vec<(Vec<u64>, bool)> = fit_rows
        .iter()
        .filter(|&&i| !degenerate(i))
        .map(|&i| (features(&flat[i]), flat[i].reverted))
        .collect();
    let mut head = Head::new();
    head.train(&train, epochs, lr, l2, seed);

    let mut scores: HashMap<usize, f64> = HashMap::new();
    let mut score_label: Vec<(f64, bool)> = Vec::new();
    for run in &eval_runs {
        for &idx in &run.rows {
            if degenerate(idx) {
                continue;
            }
            let s = head.score(&features(&flat[idx]));
            scores.insert(idx, s);
            score_label.push((s, flat[idx].reverted));
        }
    }

    match pr_auc(&score_label) {
        Some(auc) => println!(
            "head PR-AUC (EVAL): {:.3} over {} scored edits, {} positives (reverts) — support disclosed",
            auc,
            score_label.len(),
            eval_reverts
        ),
        None => println!("head PR-AUC (EVAL): UNDECIDABLE — no positives in the scored pool"),
    }
    sweep(
        &eval_runs.into_iter().cloned().collect::<Vec<_>>(),
        &flat,
        &scores,
        &[0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.75, 0.9, 0.95, 0.99],
    );
    println!(
        "escalation-stop reference: the T1 similarity floor FAILED both variants (no tau adjudicable); this head beats the floor only if a tau clears BOTH bars AND PR-AUC separates beyond noise"
    );
}
