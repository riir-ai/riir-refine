//! The dogfood rerank driver (riir-refine Plan 202 R2+R3 spike — the client half).
//!
//! Drives the REAL `SeatRerankClient` wire against a live seat edge
//! (`ext_seat_dogfood_server`) in two disclosed passes, and writes the
//! JSONL ledger the R7 freeze reads:
//!
//! - **pass `corpus_pool`** — the REAL fresh-install traffic shape: the
//!   corpus slice for the rule (what `draft_fix` selects from, and the
//!   spine of what `heal()` pools). Measured finding this pass exists to
//!   pin: every rule corpus-wide carries exactly ONE fix, so the real
//!   pool is size 1 — a rerank with nothing to choose between. Rows
//!   carry `pass:"corpus_pool"` + `pool:1`.
//! - **pass `wire_scale`** — 8 REAL fixes per body (sibling rules'
//!   fixes, real code — disclosed: NOT same-site candidates, the pass
//!   exercises payload/echo scale at the seat's 8-label presentation
//!   cap, never candidate quality). Rows carry `pass:"wire_scale"`.
//! - one `abstain_probe` row — the abstain path over the real wire.
//!
//! Consent runs the REAL lane: `RIIR_REFINE_CODE_EGRESS=snippet` must be
//! set or every call declines pre-socket (that refusal is itself an R7
//! row). No `--write`, no repo edits: reads the embedded corpus, speaks
//! HTTP, writes the ledger.
//!
//! ```text
//! cargo run --release --features seat_rerank --example seat_rerank_dogfood -- \
//!     --endpoint http://127.0.0.1:8092 --cases 12 --out dogfood_ledger.jsonl
//! ```

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use riir_refine::domain::HealEntry;
use riir_refine::domains::clippy_lints::corpus::build_seed_corpus;
use riir_refine::seat_rerank::{SeatAnswer, SeatDecline, SeatRerankClient};

/// The seat presentation width — instinct's `effective_cap` is 8 (S8).
const MAX_OPTIONS: usize = 8;

struct Case {
    rule: String,
    pass: &'static str,
    snippet: String,
    options: Vec<String>,
}

/// The corpus grouped by rule: (first buggy site, distinct fixes in rule
/// order). Corpus invariant this spike pins: every rule has exactly one.
type RulePools = BTreeMap<String, (String, Vec<String>)>;

fn corpus_by_rule() -> (RulePools, usize, usize) {
    let corpus = build_seed_corpus();
    let mut by_rule: BTreeMap<&str, (String, Vec<String>)> = BTreeMap::new();
    for e in &corpus.entries {
        by_rule
            .entry(e.rule_id())
            .or_insert_with(|| (e.buggy_code().to_string(), Vec::new()))
            .1
            .push(e.fixed_code().to_string());
    }
    let rules = by_rule.len();
    let multi = by_rule.values().filter(|(_, fixes)| fixes.len() > 1).count();
    (
        by_rule
            .into_iter()
            .map(|(k, (b, f))| (k.to_string(), (b, f)))
            .collect(),
        rules,
        multi,
    )
}

/// Even spread across the sorted rule list (`step = ceil(rules / n)`),
/// so one hot rule cannot dominate the sample.
fn sampled(by_rule: &RulePools, n: usize) -> Vec<&String> {
    let step = by_rule.len().div_ceil(n).max(1);
    by_rule.keys().step_by(step).take(n).collect()
}

fn main() {
    let mut endpoint = "http://127.0.0.1:8092".to_string();
    let mut suite = "sst5".to_string();
    let mut cases_wanted = 12usize;
    let mut deadline_ms = 2_000u64;
    let mut out_path = String::from("dogfood_ledger.jsonl");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut val = || args.next().unwrap_or_else(|| panic!("{arg} needs a value"));
        match arg.as_str() {
            "--endpoint" => endpoint = val(),
            "--suite" => suite = val(),
            "--cases" => cases_wanted = val().parse().expect("--cases <N>"),
            "--deadline-ms" => deadline_ms = val().parse().expect("--deadline-ms <MS>"),
            "--out" => out_path = val(),
            other => panic!("unknown arg {other}"),
        }
    }

    let client = SeatRerankClient::new(&endpoint, &suite).expect("endpoint parses");
    let (by_rule, rules, multi) = corpus_by_rule();
    println!(
        "dogfood: corpus has {rules} rules ({multi} with >1 fix) — \
         sampling {cases_wanted}/pass -> {endpoint} suite={suite} deadline={deadline_ms}ms",
    );

    let mut ledger = String::new();
    let (mut picks, mut abstains, mut declines) = (0usize, 0usize, 0usize);

    // ── pass corpus_pool: the real fresh-install shape ────────────────
    for rule in sampled(&by_rule, cases_wanted) {
        let (snippet, fixes) = &by_rule[rule];
        let case = Case {
            rule: rule.clone(),
            pass: "corpus_pool",
            snippet: snippet.clone(),
            options: fixes.clone(),
        };
        run_case(&client, &suite, &case, &mut ledger, &mut picks, &mut abstains, &mut declines);
    }

    // ── pass wire_scale: payload/echo scale at the presentation cap ───
    // 8 REAL fixes per body — sibling rules' fixes, disclosed as NOT
    // same-site candidates.
    for rule in sampled(&by_rule, cases_wanted) {
        let (snippet, _) = &by_rule[rule];
        let options: Vec<String> = by_rule
            .values()
            .filter_map(|(_, fixes)| fixes.first().cloned())
            .take(MAX_OPTIONS)
            .collect();
        let case = Case {
            rule: rule.clone(),
            pass: "wire_scale",
            snippet: snippet.clone(),
            options,
        };
        run_case(&client, &suite, &case, &mut ledger, &mut picks, &mut abstains, &mut declines);
    }

    // ── the abstain probe ─────────────────────────────────────────────
    if let Some((rule, (_, fixes))) = by_rule.iter().next() {
        let case = Case {
            rule: rule.clone(),
            pass: "abstain_probe",
            snippet: format!("ABSTAIN: no confident pick for {rule}"),
            options: fixes.clone(),
        };
        run_case(&client, &suite, &case, &mut ledger, &mut picks, &mut abstains, &mut declines);
    }

    if let Some(parent) = std::path::Path::new(&out_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&out_path, &ledger).expect("write ledger");
    println!(
        "dogfood done: {picks} picks / {abstains} abstains / {declines} declines — \
         ledger -> {out_path}"
    );
}

fn run_case(
    client: &SeatRerankClient,
    suite: &str,
    case: &Case,
    ledger: &mut String,
    picks: &mut usize,
    abstains: &mut usize,
    declines: &mut usize,
) {
    let deadline = Instant::now() + Duration::from_millis(2_000);
    let t0 = Instant::now();
    let body_bytes = client
        .build_body(&case.snippet, &case.options)
        .map(|b| b.len())
        .unwrap_or(0);
    // The FIRST attempt is recorded verbatim (the lazy 503 window is real
    // first-customer behavior — R7 row); the retry loop mirrors what the
    // R6 wiring must decide (budgeted readiness retry vs eager load),
    // bounded by the same deadline.
    let mut outcome = client.rerank(&case.snippet, &case.options, deadline);
    let first_class = class_of(&outcome).to_string();
    let mut attempts = 1usize;
    while matches!(&outcome, Err(SeatDecline::Wire(m)) if m == "loading")
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(50));
        outcome = client.rerank(&case.snippet, &case.options, deadline);
        attempts += 1;
    }
    let us = t0.elapsed().as_micros();
    let (class, detail) = match &outcome {
        Ok(SeatAnswer::Pick {
            index,
            candidate,
            receipt: ref receipt_opt,
        }) => {
            *picks += 1;
            let audit = receipt_opt
                .as_ref()
                .map(|r| {
                    format!(
                        " receipt(build={:?} us={:?})",
                        r.build.as_deref().unwrap_or("-"),
                        r.us
                    )
                })
                .unwrap_or_else(|| " receipt=none".to_string());
            ("pick", format!("index={index} bytes={} {audit}", candidate.len()))
        }
        Ok(SeatAnswer::Abstained { .. }) => {
            *abstains += 1;
            ("abstain", "modelless answer stands".to_string())
        }
        Err(SeatDecline::Egress(_)) => {
            // Consent refused pre-socket — record once loudly, then stop:
            // every row would be the same refusal.
            eprintln!(
                "REFUSED loud: code-egress consent unset — set \
                 RIIR_REFINE_CODE_EGRESS=snippet (the consent field is an \
                 R7 ledger row; refusing is the correct lane)"
            );
            std::process::exit(2);
        }
        Err(e) => {
            *declines += 1;
            ("decline", e.to_string())
        }
    };
    ledger.push_str(&format!(
        "{{\"rule\":\"{}\",\"pass\":\"{}\",\"suite\":\"{}\",\"pool\":{},\"body_bytes\":{},\
         \"class\":\"{class}\",\"first_class\":\"{first_class}\",\"attempts\":{attempts},\"us\":{},\"detail\":\"{}\"}}\n",
        case.rule,
        case.pass,
        suite,
        case.options.len(),
        body_bytes,
        us,
        detail.replace('\\', "\\\\").replace('"', "'"),
    ));
    println!(
        "  {:<28} {} pool={} {class} {us}µs (first={first_class}, tries={attempts}) {detail}",
        case.rule,
        case.pass,
        case.options.len()
    );
}

fn class_of(outcome: &Result<SeatAnswer, SeatDecline>) -> &'static str {
    match outcome {
        Ok(SeatAnswer::Pick { .. }) => "pick",
        Ok(SeatAnswer::Abstained { .. }) => "abstain",
        Err(_) => "decline",
    }
}
