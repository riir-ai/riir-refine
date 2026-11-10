//! Issue 215 G0 — the try-next pool census (REPORT-ONLY, exit 0).
//!
//! Measures the three pre-registered G0 numbers over the REAL pool
//! proposer (`HealPipeline::heal_with_pool` via `RegateHarness::classify_outcome`
//! — the arm-H / HARVEST pool birth, no second pipeline):
//!
//! - (a) fraction of pools with ≥2 candidates;
//! - (b) of those, the fraction whose per-candidate compile outcomes DIFFER
//!   (signal density — the number the lane lives or dies by);
//! - (c) compile cost per pool (try-all, try-next-on-fail, and K≤4 budget).
//!
//! Per candidate: `apply_edit` merges it into the module (the harvest's
//! merge, `verbatim_ok = false`), then `ClippyVerifier::verify` compiles the
//! merged module (the same oracle `score_case` uses). A candidate that does
//! not MERGE is a compile FAIL at zero cost (production's `FixVerifier`
//! would splice it and compile it — that cost is NOT measured here; it is
//! disclosed as `merge_fail` so the cost column under-counts honestly).
//! `ran = false` is UNSEEN — excluded from (b), counted loud, never a fail.
//!
//! Populations (each printed with its own floor):
//! - `v2edit` — the lint-LABELLED v2edit test chunks (`load_v2edit`, row
//!   label `lints[0]` as the target; no discovery pass);
//! - `score_bench` — the frozen v1 score-bench fixtures (healable only,
//!   `expect_lints[0]` as the target).
//!
//! A SECOND pool source (Issue 215 G0 follow-up): the production seat-lane
//! pool `bin/heal.rs` builds — `suggest_file` spans → `seat_rule_book_pool`
//! (the shared lib builder) → candidates spliced over the span bytes.
//!
//! The pipeline is NOT memory-warmed between cases (fresh-install pool
//! shape); Bench 114's HARVEST warmed it — the two are disclosed separately.
//!
//! ```text
//! cargo run --release --features cf_workers_ai,score_bench,escalation_manifest \
//!     --example try_next_pool_census -- [--data-dir ../riir-train/data] \
//!     [--stride N] [--work-root /tmp/i215_g0/verify]
//! ```

use std::path::PathBuf;
use std::time::Instant;

use riir_refine::ClippyVerifier;
use riir_refine::cf_workers_ai::{RegateHarness, Sample, SampleSource, apply_edit, load_v2edit};
use riir_refine::domains::clippy_lints::proposers::propose_fix;
use riir_refine::draft::latent_matcher::line_span_bytes;
use riir_refine::escalation_manifest::DEFAULT_MIN_POOL;
use riir_refine::score_bench::fixtures::fixtures;

/// The K budget the issue pre-registers for the gated arm.
const K_BUDGET: usize = 4;
/// `bin/heal.rs`'s default `--top-k` (the seat lane's `suggest_file` width).
const SEAT_TOP_K: usize = 3;
/// Population floors — a census over fewer cases is a blind read.
const MIN_V2EDIT: usize = 300;
const MIN_FIXTURES: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    MergeFail,
    CompileFail,
    Compiles,
    Unseen,
}

struct PoolRow {
    pool: usize,
    outcomes: Vec<Outcome>,
    cost_ms: Vec<f64>,
    proposer_idx: Option<usize>,
    served_idx: Option<usize>,
}

#[derive(Default)]
struct Census {
    cases: usize,
    baseline_fail: usize,
    rows: Vec<PoolRow>,
}

fn percentile(sorted: &[f64], p: f64) -> (f64, usize) {
    match sorted.len() {
        0 => (f64::NAN, 0),
        n => {
            let idx = ((n - 1) as f64 * p).round() as usize;
            (sorted[idx], n - idx)
        }
    }
}

/// Compile one candidate module (`None` = it never became a module —
/// merge-fail / no-op splice — a FAIL at zero measured cost).
fn compile_outcome(
    verifier: &ClippyVerifier,
    module: Option<String>,
    lint: &str,
    outcomes: &mut Vec<Outcome>,
    cost_ms: &mut Vec<f64>,
) {
    let Some(module) = module else {
        outcomes.push(Outcome::MergeFail);
        cost_ms.push(0.0);
        return;
    };
    let t = Instant::now();
    let r = verifier.verify(&module, lint);
    cost_ms.push(t.elapsed().as_secs_f64() * 1e3);
    outcomes.push(match (r.ran, r.compiled) {
        (false, _) => Outcome::Unseen,
        (true, true) => Outcome::Compiles,
        (true, false) => Outcome::CompileFail,
    });
}

/// The production seat-lane pool (`bin/heal.rs` Plan 225 T5.2), built by
/// the SAME lib builders: `suggest_file` spans → the FIRST span whose
/// `seat_rule_book_pool` reaches the min-pool and whose byte span is
/// valid (heal.rs consults ONE span per file). With no eligible span the
/// row carries the first span's pool (size < min-pool), or 0 with none.
/// Each candidate is spliced over the span bytes (heal.rs's
/// `replace_range`); a splice equal to the module is a no-op FAIL
/// (`FixVerifier` reports `kept_edits = 0` on an untouched file).
fn seat_row(harness: &mut RegateHarness, verifier: &ClippyVerifier, sample: &Sample) -> PoolRow {
    let src = &sample.module;
    let lint = &sample.lints[0];
    let spans = harness.matcher.suggest_file(src, "census.rs", SEAT_TOP_K);
    let mut chosen: Option<(Vec<String>, usize, usize)> = None;
    let mut fallback_pool = 0usize;
    for (i, s) in spans.iter().enumerate() {
        let pool = harness.matcher.seat_rule_book_pool(s);
        if i == 0 {
            fallback_pool = pool.len();
        }
        if pool.len() < DEFAULT_MIN_POOL {
            continue;
        }
        let Some((b, e)) = line_span_bytes(src, s.start_line, s.end_line) else {
            continue;
        };
        if e <= b || e > src.len() {
            continue;
        }
        chosen = Some((pool, b, e));
        break;
    }
    let Some((pool, b, e)) = chosen else {
        return PoolRow {
            pool: fallback_pool.min(DEFAULT_MIN_POOL - 1),
            outcomes: Vec::new(),
            cost_ms: Vec::new(),
            proposer_idx: None,
            served_idx: None,
        };
    };
    let mut outcomes = Vec::with_capacity(pool.len());
    let mut cost_ms = Vec::with_capacity(pool.len());
    for cand in &pool {
        let mut fixed = src.clone();
        fixed.replace_range(b..e, cand);
        let module = (fixed != *src).then_some(fixed);
        compile_outcome(verifier, module, lint, &mut outcomes, &mut cost_ms);
    }
    PoolRow {
        pool: pool.len(),
        outcomes,
        cost_ms,
        proposer_idx: None,
        served_idx: None,
    }
}

fn run_case(
    harness: &mut RegateHarness,
    verifier: &ClippyVerifier,
    sample: &Sample,
    census: &mut Census,
    seat: &mut Census,
) {
    census.cases += 1;
    seat.cases += 1;
    let lint = &sample.lints[0];
    // Control: a module that does not compile as-is cannot carry a compile
    // contrast — excluded (counted), never folded into a pool row.
    let base = verifier.verify(&sample.module, lint);
    if !(base.ran && base.compiled) {
        census.baseline_fail += 1;
        seat.baseline_fail += 1;
        return;
    }
    let outcome = harness.classify_outcome(lint, &sample.module);
    let proposer = propose_fix(lint, &sample.module).filter(|f| f != &sample.module);
    let proposer_idx = proposer
        .as_ref()
        .and_then(|p| outcome.pool.iter().position(|c| c == p));
    let served_idx = outcome
        .served
        .as_ref()
        .and_then(|s| outcome.pool.iter().position(|c| c == s));
    let mut outcomes = Vec::with_capacity(outcome.pool.len());
    let mut cost_ms = Vec::with_capacity(outcome.pool.len());
    for cand in &outcome.pool {
        let merged = apply_edit(&sample.module, cand, false).map(|(m, _)| m);
        compile_outcome(verifier, merged, lint, &mut outcomes, &mut cost_ms);
    }
    census.rows.push(PoolRow {
        pool: outcome.pool.len(),
        outcomes,
        cost_ms,
        proposer_idx,
        served_idx,
    });
    seat.rows.push(seat_row(harness, verifier, sample));
}

fn report(name: &str, c: &Census, floor: usize) {
    let scored = c.rows.len();
    let ge2: Vec<&PoolRow> = c.rows.iter().filter(|r| r.pool >= 2).collect();
    let unseen_pools = ge2
        .iter()
        .filter(|r| r.outcomes.contains(&Outcome::Unseen))
        .count();
    let seen: Vec<&&PoolRow> = ge2
        .iter()
        .filter(|r| !r.outcomes.contains(&Outcome::Unseen))
        .collect();
    let differ: Vec<&&&PoolRow> = seen
        .iter()
        .filter(|r| {
            let any_pass = r.outcomes.contains(&Outcome::Compiles);
            let any_fail = r.outcomes.iter().any(|o| *o != Outcome::Compiles);
            any_pass && any_fail
        })
        .collect();
    let mut hist = std::collections::BTreeMap::<usize, usize>::new();
    for r in &c.rows {
        *hist.entry(r.pool).or_default() += 1;
    }
    println!("== population `{name}` ==");
    println!(
        "cases {} · baseline-not-compiling (excluded) {} · pooled rows {scored} · \
         pool-size histogram {hist:?}",
        c.cases, c.baseline_fail
    );
    if c.cases < floor {
        println!("⚠ POPULATION FLOOR: {} < {floor} — BLIND READ", c.cases);
    }
    let frac = |a: usize, b: usize| {
        if b == 0 {
            f64::NAN
        } else {
            a as f64 / b as f64
        }
    };
    println!(
        "(a) pool≥2: {}/{scored} = {:.3}",
        ge2.len(),
        frac(ge2.len(), scored)
    );
    println!(
        "(b) compile outcomes DIFFER: {}/{} = {:.3}  (UNSEEN pools excluded: {unseen_pools})",
        differ.len(),
        seen.len(),
        frac(differ.len(), seen.len())
    );

    // Decomposition of (b): what the contrast IS.
    let first_fails = seen
        .iter()
        .filter(|r| r.outcomes[0] != Outcome::Compiles)
        .count();
    let recoverable = seen
        .iter()
        .filter(|r| {
            r.outcomes[0] != Outcome::Compiles && r.outcomes[1..].contains(&Outcome::Compiles)
        })
        .count();
    let recoverable_k = seen
        .iter()
        .filter(|r| {
            r.outcomes[0] != Outcome::Compiles
                && r.outcomes[1..r.pool.min(K_BUDGET)].contains(&Outcome::Compiles)
        })
        .count();
    let only_proposer_compiles = differ
        .iter()
        .filter(|r| {
            r.proposer_idx.is_some_and(|p| {
                r.outcomes
                    .iter()
                    .enumerate()
                    .all(|(i, o)| (*o == Outcome::Compiles) == (i == p))
            })
        })
        .count();
    let proposer_last = seen
        .iter()
        .filter(|r| r.proposer_idx == Some(r.pool - 1))
        .count();
    let merge_fail_cands: usize = seen
        .iter()
        .map(|r| {
            r.outcomes
                .iter()
                .filter(|o| **o == Outcome::MergeFail)
                .count()
        })
        .sum();
    let cands: usize = seen.iter().map(|r| r.pool).sum();
    let compile_fail_cands: usize = seen
        .iter()
        .map(|r| {
            r.outcomes
                .iter()
                .filter(|o| **o == Outcome::CompileFail)
                .count()
        })
        .sum();
    let served_fails = seen
        .iter()
        .filter(|r| {
            r.served_idx
                .is_some_and(|s| r.outcomes[s] != Outcome::Compiles)
        })
        .count();
    let served_present = seen.iter().filter(|r| r.served_idx.is_some()).count();
    let proposer_first_ok = seen
        .iter()
        .filter(|r| {
            r.proposer_idx
                .is_some_and(|p| r.outcomes[p] == Outcome::Compiles)
        })
        .count();
    let any_ok = seen
        .iter()
        .filter(|r| r.outcomes.contains(&Outcome::Compiles))
        .count();
    println!(
        "    decomposition: candidates {cands} = merge-fail {merge_fail_cands} + compile-fail \
         {compile_fail_cands} + compiles {}",
        cands - merge_fail_cands - compile_fail_cands
    );
    println!(
        "    first (proposal-order) candidate fails: {first_fails}/{} · recoverable by \
         try-next (unbounded) {recoverable} · within K={K_BUDGET} {recoverable_k}",
        seen.len()
    );
    println!(
        "    proposer member is LAST in pool: {proposer_last}/{} · differing pools where ONLY \
         the proposer member compiles: {only_proposer_compiles}/{}",
        seen.len(),
        differ.len()
    );
    println!(
        "    pick-success (≥1 candidate compiles): oracle-any {any_ok} · static proposer-first \
         (1 compile) {proposer_first_ok} · modelless served present {served_present}, \
         served fails compile {served_fails}"
    );

    let mut first_ok = std::collections::BTreeMap::<String, usize>::new();
    for r in &seen {
        let key = r
            .outcomes
            .iter()
            .position(|o| *o == Outcome::Compiles)
            .map_or_else(|| "none".to_string(), |i| i.to_string());
        *first_ok.entry(key).or_default() += 1;
    }
    println!("    first-compiling position (0-based, proposal order): {first_ok:?}");

    // (c) cost per pool (compile wall-clock only; merge-fail costs 0 here).
    let mut try_all: Vec<f64> = seen.iter().map(|r| r.cost_ms.iter().sum()).collect();
    let try_next_cost = |cap: usize| -> Vec<f64> {
        let mut v: Vec<f64> = seen
            .iter()
            .map(|r| {
                let mut s = 0.0;
                for (i, o) in r.outcomes.iter().enumerate().take(cap) {
                    s += r.cost_ms[i];
                    if *o == Outcome::Compiles {
                        break;
                    }
                }
                s
            })
            .collect();
        v.sort_by(f64::total_cmp);
        v
    };
    try_all.sort_by(f64::total_cmp);
    let mut per_compile: Vec<f64> = seen
        .iter()
        .flat_map(|r| {
            r.outcomes
                .iter()
                .zip(&r.cost_ms)
                .filter(|(o, _)| matches!(o, Outcome::Compiles | Outcome::CompileFail))
                .map(|(_, c)| *c)
        })
        .collect();
    per_compile.sort_by(f64::total_cmp);
    for (label, v) in [
        ("per compile", per_compile),
        ("pool try-all", try_all),
        ("pool try-next (unbounded)", try_next_cost(usize::MAX)),
        ("pool try-next (K=4)", try_next_cost(K_BUDGET)),
    ] {
        let mean = if v.is_empty() {
            f64::NAN
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        };
        let (p50, _) = percentile(&v, 0.5);
        let (p90, t90) = percentile(&v, 0.9);
        println!(
            "(c) {label}: n {} · mean {mean:.0} ms · p50 {p50:.0} ms · p90 {p90:.0} ms \
             (tail support {t90})",
            v.len()
        );
    }
}

fn box_state() -> String {
    let run = |cmd: &str, args: &[&str]| {
        std::process::Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    };
    let power = run("pmset", &["-g", "batt"]);
    let power = power.lines().next().unwrap_or("").to_string();
    let load = run("sysctl", &["-n", "vm.loadavg"]);
    let mode = run("pmset", &["-g"]);
    let mode = mode
        .lines()
        .find(|l| l.contains("powermode"))
        .map(str::trim)
        .unwrap_or("powermode=?")
        .to_string();
    format!("{power} · load {load} · {mode}")
}

fn main() {
    let mut data_dir = PathBuf::from("../riir-train/data");
    let mut stride = 1usize;
    let mut work_root: Option<PathBuf> = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut val = || -> String { it.next().expect("flag needs a value") };
        match a.as_str() {
            "--data-dir" => data_dir = PathBuf::from(val()),
            "--stride" => stride = val().parse::<usize>().expect("--stride integer").max(1),
            "--work-root" => work_root = Some(PathBuf::from(val())),
            other => panic!("unknown flag {other}"),
        }
    }
    let t0 = Instant::now();
    let mut harness = RegateHarness::new().expect("harness init");
    let verifier = match &work_root {
        Some(r) => ClippyVerifier::new().with_work_root(r),
        None => ClippyVerifier::new(),
    };
    println!(
        "PROVENANCE: commit {} · {} · stride {stride} · work-root {:?} · K {K_BUDGET}",
        option_env!("RIIR_REFINE_BUILD_COMMIT").unwrap_or("?"),
        box_state(),
        work_root
    );

    let v2: Vec<Sample> = load_v2edit(&data_dir, &[0, 1, 2, 3, 4, 5, 6], 0)
        .expect("v2edit test chunks")
        .into_iter()
        .step_by(stride)
        .collect();
    let mut c_v2 = Census::default();
    let mut seat_v2 = Census::default();
    let n_v2 = v2.len();
    for (i, s) in v2.iter().enumerate() {
        run_case(&mut harness, &verifier, s, &mut c_v2, &mut seat_v2);
        if (i + 1) % 25 == 0 {
            eprintln!(
                "[census] v2edit {}/{n_v2} ({:.0}s)",
                i + 1,
                t0.elapsed().as_secs_f64()
            );
        }
    }

    let mut c_fx = Census::default();
    let mut seat_fx = Census::default();
    for f in fixtures().into_iter().filter(|f| !f.clean) {
        let Some(lint) = f.expect_lints.first() else {
            continue;
        };
        let s = Sample {
            id: f.id.to_string(),
            lints: vec![(*lint).to_string()],
            module: f.code.to_string(),
            expected: None,
            source: SampleSource::V2edit,
        };
        run_case(&mut harness, &verifier, &s, &mut c_fx, &mut seat_fx);
    }

    report("v2edit (labelled test chunks)", &c_v2, MIN_V2EDIT / stride);
    report("score_bench v1 fixtures (healable)", &c_fx, MIN_FIXTURES);
    report(
        "SEAT-LANE pool × v2edit (labelled test chunks)",
        &seat_v2,
        MIN_V2EDIT / stride,
    );
    report(
        "SEAT-LANE pool × score_bench v1 fixtures (healable)",
        &seat_fx,
        MIN_FIXTURES,
    );
    println!("wall {:.0}s", t0.elapsed().as_secs_f64());
}
