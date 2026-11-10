//! The Issue 220 G3 cell instrument (katgpt-rs plan 633 T3) — REPORT-ONLY.
//!
//! Reads one or more REAL outcome windows (`adoption_outcomes.jsonl`
//! sidecars — sibling repos are read-only to this run), prints the
//! active-set demote/readmit consult census across an α grid, an α-flip
//! census, leave-one-out verdict locality, and a seeded judge-noise
//! stability sweep. The generic pre-registered bars (issue 220
//! `## G3 cell — pre-registered bars`: α-stability, noise thresholds,
//! population floors) are evaluated in-instrument and printed PASS/FAIL;
//! the exact-shape pins are adjudicated by the bench record against the
//! printed numbers (the scorer facts they were hand-computed from are
//! pinned in the issue, written BEFORE this instrument existed).
//!
//! Never writes: a loaded window only ever reads; perturbation replays go
//! through [`riir_refine::adoption_gate::AdoptionWindow::from_rows`]
//! in-memory. Exit codes: 0 report, 2 instrument failure (bad args, an
//! unloadable window, or the determinism self-check red — a bar FAIL is a
//! REPORTED verdict, not an exit code).
//!
//! ```text
//! cargo run --features adoption_gate --example adoption_g3_cell -- \
//!   /git/riir-dao/.refine/adoption_outcomes.jsonl ...
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use riir_refine::adoption_gate::{AdoptionWindow, GateVerdict, OutcomeRow};

/// Deterministic xorshift64 — the noise harness's ONLY randomness source
/// (seeded per arm; no global RNG, so every run of the same command is
/// bit-identical — the global-RNG law).
struct XorShift(u64);

impl XorShift {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in [0, 1) — 53 mantissa bits, no modulo bias.
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

const ALPHA_DEFAULT: &str = "0,0.25,0.5,0.75,1";
const NOISE_DEFAULT: &str = "0.1,0.2";
const SEEDS_DEFAULT: usize = 32;
const ALPHA0_DEFAULT: f64 = 0.5;
const MIN_STABILITY: f64 = 0.90;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    Demote,
    Readmit,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Demote => "demote",
            Kind::Readmit => "readmit",
        }
    }
}

fn shape(v: &GateVerdict) -> &'static str {
    match v {
        GateVerdict::Proceed(_) => "PROCEED",
        GateVerdict::Refuse(_) => "REFUSE",
        GateVerdict::Abstain => "ABSTAIN",
    }
}

fn detail(v: &GateVerdict) -> String {
    match v {
        GateVerdict::Proceed(s) | GateVerdict::Refuse(s) => format!(
            "retained={:.4} repeated={:.4} score={:.4}",
            s.retained, s.repeated, s.score
        ),
        GateVerdict::Abstain => "no-evidence".to_string(),
    }
}

/// One consult identity within a window.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Consult {
    kind: Kind,
    tag: String,
}

fn consult(window: &AdoptionWindow, c: &Consult, alpha: f64) -> GateVerdict {
    match c.kind {
        Kind::Demote => window.demote_verdict(&c.tag, alpha),
        Kind::Readmit => window.readmit_verdict(&c.tag, alpha),
    }
}

/// Every consult over a window's tags, both kinds, sorted (stable order
/// for every census below).
fn consult_population(window: &AdoptionWindow) -> Vec<Consult> {
    let mut out = Vec::new();
    for tag in window.tags() {
        out.push(Consult {
            kind: Kind::Demote,
            tag: tag.to_string(),
        });
        out.push(Consult {
            kind: Kind::Readmit,
            tag: tag.to_string(),
        });
    }
    out.sort();
    out.shrink_to_fit();
    out
}

/// Flip one row's verdict bit (judge noise: the oracle kept what the
/// ground truth would revert, or reverted what it would keep). The digest
/// is untouched — the same output, judged the other way.
fn flip(rows: &mut [OutcomeRow], i: usize) {
    rows[i].accepted = !rows[i].accepted;
}

struct Args {
    alphas: Vec<f64>,
    alpha0: f64,
    noises: Vec<f64>,
    seeds: usize,
    expect_windows: usize,
    expect_tags: usize,
    windows: Vec<PathBuf>,
}

fn parse_floats(csv: &str, what: &str) -> Result<Vec<f64>, String> {
    csv.split(',')
        .map(|s| {
            s.trim()
                .parse::<f64>()
                .map_err(|e| format!("{what}: {s:?}: {e}"))
        })
        .collect()
}

fn parse_args<I: Iterator<Item = String>>(mut it: I) -> Result<Args, String> {
    let mut args = Args {
        alphas: parse_floats(ALPHA_DEFAULT, "--alpha")?,
        alpha0: ALPHA0_DEFAULT,
        noises: parse_floats(NOISE_DEFAULT, "--noise")?,
        seeds: SEEDS_DEFAULT,
        expect_windows: 5,
        expect_tags: 11,
        windows: Vec::new(),
    };
    let mut alphas: Option<String> = None;
    let mut noises: Option<String> = None;
    while let Some(a) = it.next() {
        let value = |it: &mut I, name: &str| {
            it.next().ok_or(format!("{name} requires a value"))
        };
        match a.as_str() {
            "--alpha" => alphas = Some(value(&mut it, "--alpha")?),
            "--alpha0" => {
                args.alpha0 = value(&mut it, "--alpha0")?
                    .parse()
                    .map_err(|e| format!("--alpha0: {e}"))?;
            }
            "--noise" => noises = Some(value(&mut it, "--noise")?),
            "--seeds" => {
                args.seeds = value(&mut it, "--seeds")?
                    .parse()
                    .map_err(|e| format!("--seeds: {e}"))?;
            }
            "--expect-windows" => {
                args.expect_windows = value(&mut it, "--expect-windows")?
                    .parse()
                    .map_err(|e| format!("--expect-windows: {e}"))?;
            }
            "--expect-tags" => {
                args.expect_tags = value(&mut it, "--expect-tags")?
                    .parse()
                    .map_err(|e| format!("--expect-tags: {e}"))?;
            }
            other if other.starts_with('-') => return Err(format!("unknown flag {other:?}")),
            other => args.windows.push(PathBuf::from(other)),
        }
    }
    if let Some(csv) = alphas {
        args.alphas = parse_floats(&csv, "--alpha")?;
    }
    if let Some(csv) = noises {
        args.noises = parse_floats(&csv, "--noise")?;
    }
    if args.windows.is_empty() {
        return Err(
            "usage: adoption_g3_cell [--alpha CSV] [--alpha0 F] [--noise CSV] \
                    [--seeds N] [--expect-windows N] [--expect-tags N] <window.jsonl>..."
                .to_string(),
        );
    }
    if args.alphas.is_empty() || args.noises.is_empty() || args.seeds == 0 {
        return Err("--alpha/--noise must be non-empty and --seeds > 0".to_string());
    }
    if !args.alphas.contains(&args.alpha0) {
        return Err(format!(
            "--alpha0 {} must be a member of the --alpha grid",
            args.alpha0
        ));
    }
    Ok(args)
}

/// Per-window cell results, collected for the final bar table (the
/// per-window detail is printed as the run goes; only the bar-decidable
/// numbers survive the struct).
struct WindowCell {
    tags: usize,
    seat_rows: usize,
    alpha_flips: usize,
    dominant: Option<(Consult, &'static str)>,
    dominant_min_stability: f64,
    dominant_survived_loo: bool,
    demote_refusals: usize,
    readmit_refusals: usize,
    abstains: usize,
}

fn run_window(path: &Path, args: &Args) -> Result<WindowCell, String> {
    let window = AdoptionWindow::load_file(path.to_path_buf())?;
    let rows = window.rows().to_vec();
    let accepted = rows.iter().filter(|r| r.accepted).count();
    let rejected = rows.len() - accepted;
    let seat_rows = rows
        .iter()
        .filter(|r| r.tag == riir_refine::adoption_gate::SEAT_TAG)
        .count();
    let population = consult_population(&window);

    println!(
        "== window {} ({} rows: {accepted} accepted / {rejected} rejected; {} tags; \
         seat rows: {seat_rows}{})",
        path.display(),
        rows.len(),
        window.tags().len(),
        if seat_rows == 0 {
            " — seat leg unmeasured on real data (disclosed)"
        } else {
            ""
        },
    );

    // Determinism self-check: a from_rows replay of the same rows must
    // consult identically to the disk-loaded window (the perturbation
    // harness's premise — if replay diverges, every number below is
    // evidence of nothing).
    let replay = AdoptionWindow::from_rows(window.rows().to_vec());
    for c in &population {
        if shape(&consult(&window, c, args.alpha0)) != shape(&consult(&replay, c, args.alpha0)) {
            return Err(format!(
                "self-check FAILED: from_rows replay diverges on {} {}",
                c.kind.name(),
                c.tag
            ));
        }
    }

    // α grid census.
    let mut alpha_flips = 0usize;
    let mut demote_refusals = 0usize;
    let mut readmit_refusals = 0usize;
    let mut abstains = 0usize;
    for c in &population {
        let baseline = consult(&window, c, args.alpha0);
        let mut shapes = Vec::new();
        for &alpha in &args.alphas {
            let v = consult(&window, c, alpha);
            if alpha == args.alpha0 {
                println!(
                    "  {:8} {:28} α={:.2} {:8} {}",
                    c.kind.name(),
                    c.tag,
                    alpha,
                    shape(&v),
                    detail(&v)
                );
            }
            shapes.push(shape(&v));
        }
        shapes.dedup();
        if shapes.len() > 1 {
            alpha_flips += 1;
        }
        match (&baseline, c.kind) {
            (GateVerdict::Refuse(_), Kind::Demote) => demote_refusals += 1,
            (GateVerdict::Refuse(_), Kind::Readmit) => readmit_refusals += 1,
            (GateVerdict::Abstain, _) => abstains += 1,
            _ => {}
        }
    }

    // Dominant-carrier refusal: among the demote REFUSALS at α0, the rule
    // carrying the most accepted rows (tie → lexicographically first).
    let accepted_of = |tag: &str| rows.iter().filter(|r| r.tag == tag && r.accepted).count();
    let dominant = population
        .iter()
        .filter(|c| {
            c.kind == Kind::Demote
                && matches!(consult(&window, c, args.alpha0), GateVerdict::Refuse(_))
        })
        .max_by(|a, b| {
            accepted_of(&a.tag)
                .cmp(&accepted_of(&b.tag))
                .then_with(|| b.tag.cmp(&a.tag))
        })
        .cloned();
    let dominant = dominant.map(|c| {
        let s = shape(&consult(&window, &c, args.alpha0));
        (c, s)
    });

    // Leave-one-out: flip each row once, re-consult at α0, print which
    // consults each flip moves.
    let mut dominant_survived_loo = true;
    for i in 0..rows.len() {
        let mut perturbed = rows.clone();
        flip(&mut perturbed, i);
        let w = AdoptionWindow::from_rows(perturbed);
        for c in &population {
            let base = shape(&consult(&window, c, args.alpha0));
            let now = shape(&consult(&w, c, args.alpha0));
            if base != now {
                println!(
                    "  [leave-one-out] row {i} ({}, accepted={}) moves {} {}: \
                     {base} → {now}",
                    rows[i].tag,
                    rows[i].accepted,
                    c.kind.name(),
                    c.tag
                );
                if Some(c) == dominant.as_ref().map(|(dc, _)| dc) {
                    dominant_survived_loo = false;
                }
            }
        }
    }

    // Judge-noise Monte-Carlo: per-row Bernoulli(p) verdict flips, both
    // arms, seeded per (p, arm, seed) — every run bit-identical.
    let mut dominant_min_stability = 1.0f64;
    for &p in &args.noises {
        for (arm_name, arm_filter) in [("acc→rej", true), ("rej→acc", false)] {
            let eligible: Vec<usize> = rows
                .iter()
                .enumerate()
                .filter(|(_, r)| r.accepted == arm_filter)
                .map(|(i, _)| i)
                .collect();
            if eligible.is_empty() {
                println!("  [noise p={p:.1} arm={arm_name}] no eligible rows — skipped");
                continue;
            }
            let mut agrees = vec![0usize; population.len()];
            for seed in 1..=args.seeds {
                let mut rng = XorShift(
                    0x9E37_79B9_7F4A_7C15u64
                        .wrapping_mul(seed as u64)
                        .wrapping_add((p.to_bits() ^ (arm_filter as u64)) | 1),
                );
                let mut perturbed = rows.clone();
                for &i in &eligible {
                    if rng.unit() < p {
                        flip(&mut perturbed, i);
                    }
                }
                let w = AdoptionWindow::from_rows(perturbed);
                for (k, c) in population.iter().enumerate() {
                    if shape(&consult(&window, c, args.alpha0))
                        == shape(&consult(&w, c, args.alpha0))
                    {
                        agrees[k] += 1;
                    }
                }
            }
            if let Some((dc, ds)) = &dominant {
                let k = population
                    .iter()
                    .position(|c| c == dc)
                    .expect("in population");
                let stab = agrees[k] as f64 / args.seeds as f64;
                dominant_min_stability = dominant_min_stability.min(stab);
                println!(
                    "  [noise p={p:.1} arm={arm_name} seeds={}] dominant refusal \
                     {} {} (baseline {ds}) stability {}/{} = {:.3}",
                    args.seeds,
                    dc.kind.name(),
                    dc.tag,
                    agrees[k],
                    args.seeds,
                    stab
                );
            }
            let below: Vec<String> = population
                .iter()
                .zip(&agrees)
                .filter(|(_, &a)| (a as f64 / args.seeds as f64) < MIN_STABILITY)
                .map(|(c, a)| format!("{} {} ({}/{})", c.kind.name(), c.tag, a, args.seeds))
                .collect();
            println!(
                "  [noise p={p:.1} arm={arm_name}] consults below \
                 {MIN_STABILITY:.2}: {}{}",
                below.len(),
                if below.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", below.join(", "))
                }
            );
        }
    }

    Ok(WindowCell {
        tags: window.tags().len(),
        seat_rows,
        alpha_flips,
        dominant,
        dominant_min_stability,
        dominant_survived_loo,
        demote_refusals,
        readmit_refusals,
        abstains,
    })
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let mut cells = Vec::new();
    for path in &args.windows {
        match run_window(path, &args) {
            Ok(cell) => cells.push(cell),
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        }
    }

    // The generic pre-registered bars — decidable from the data. The
    // exact-shape pins stay with the bench record (hand-computed from the
    // scorer facts pinned in the issue BEFORE the run).
    let total_flips: usize = cells.iter().map(|c| c.alpha_flips).sum();
    println!();
    println!(
        "[bar G3-b] α-flip census over {} windows × {} consults: {total_flips} flip(s) — {}",
        cells.len(),
        cells.iter().map(|c| c.tags * 2).sum::<usize>(),
        if total_flips == 0 {
            "PASS"
        } else {
            "FAIL — name the crossovers"
        }
    );
    let dom_ok = cells
        .iter()
        .all(|c| c.dominant_min_stability >= MIN_STABILITY);
    let loo_ok = cells.iter().all(|c| c.dominant_survived_loo);
    let dom_present = cells.iter().all(|c| c.dominant.is_some());
    println!(
        "[bar G3-c] dominant-carrier refusals: present {}/{} windows; min noise \
         stability {:.3} (bar ≥ {MIN_STABILITY:.2}); leave-one-out survived: {loo_ok} — {}",
        cells.iter().filter(|c| c.dominant.is_some()).count(),
        cells.len(),
        cells
            .iter()
            .map(|c| c.dominant_min_stability)
            .fold(1.0f64, f64::min),
        if dom_ok && loo_ok && dom_present {
            "PASS"
        } else {
            "FAIL/DISCLOSE — below-threshold consults are not quotable as gate evidence"
        }
    );
    let windows = cells.len();
    let tags: usize = cells.iter().map(|c| c.tags).sum();
    let demote_ref = cells.iter().map(|c| c.demote_refusals).sum::<usize>();
    let readmit_ref = cells.iter().map(|c| c.readmit_refusals).sum::<usize>();
    let abstains = cells.iter().map(|c| c.abstains).sum::<usize>();
    let g3d = windows >= args.expect_windows
        && tags >= args.expect_tags
        && demote_ref > 0
        && readmit_ref > 0
        && abstains > 0;
    println!(
        "[bar G3-d] windows={windows} (≥{}) tags={tags} (≥{}) demote-refusals=\
         {demote_ref} readmit-refusals={readmit_ref} abstains={abstains} — {}",
        args.expect_windows,
        args.expect_tags,
        if g3d {
            "PASS"
        } else {
            "FAIL — a green zero is not a measurement"
        }
    );
    let seat_total: usize = cells.iter().map(|c| c.seat_rows).sum();
    println!(
        "[disclosure] seat-swap leg: {seat_total} seat rows across all windows — \
         the real-store seat leg is UNMEASURED (fixture gates cover the mechanics)"
    );
    ExitCode::SUCCESS
}
