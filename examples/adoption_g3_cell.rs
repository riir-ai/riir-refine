//! The Issue 220 G3 cell instrument (katgpt-rs plan 633 T3) — REPORT-ONLY.
//!
//! Reads one or more REAL outcome windows (`adoption_outcomes.jsonl`
//! sidecars — sibling repos are read-only to this run), prints the
//! active-set demote/readmit consult census across an α grid, an α-flip
//! census with the T3.3 mechanical classification, leave-one-out verdict
//! locality, and a seeded judge-noise stability sweep. The T3 bar set
//! (issue 220 + plan 633 T3.3, pre-registered BEFORE the cell's sweeps and
//! run) is evaluated in-instrument and printed PASS/FAIL; the exact-shape
//! pins are adjudicated by the bench record against the printed numbers.
//!
//! T3.3 extension (plan 633 Bar A–D): every demote consult's score
//! difference is LINEAR in α (retained/repeated are α-invariant set
//! counts), so the instrument replays the gate from the rows, reads each
//! demote consult's four α-invariant score facts once, derives the exact
//! crossing α* = Δr/(Δr+Δp), predicts every grid point (PROCEED iff
//! score_ch > score_inc, the pinned strict-`>` rule), and classifies each
//! α-flip ENDPOINT-0 (Δr = 0 — the α=0 identity) or INTERIOR; abstained
//! readmits are classified audit-only vs ring-eviction; accepted-carrier
//! demote refusals get per-consult min noise stability.
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

use katgpt_core::adoption_gate::{AdoptionGate, OutcomeDigest};
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

/// Decode a loaded row's digest hex (the loader already validated the
/// round-trip; a panic here is the same corruption the loader refuses
/// loud, reached only through an already-loaded window's own rows).
fn decode_digest(hex: &str) -> OutcomeDigest {
    let mut bytes = [0u8; 32];
    for (i, pair) in hex.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let chunk = std::str::from_utf8(pair).expect("window rows are UTF-8");
        bytes[i] = u8::from_str_radix(chunk, 16).expect("loaded rows round-trip");
    }
    OutcomeDigest::from_bytes(bytes)
}

/// The α-invariant score facts of one demote consult — read once at any α
/// (retained/repeated are set counts; only the α mix changes), the
/// incumbent's from the replay rings. The pinned producer expression
/// (`(1−α)·retained + α·(1−repeated)`) is linear in α, so these four
/// numbers predict every grid point exactly.
struct DemoteFacts {
    retained_ch: f64,
    repeated_ch: f64,
    retained_inc: f64,
    repeated_inc: f64,
}

impl DemoteFacts {
    fn score_ch(&self, alpha: f64) -> f64 {
        (1.0 - alpha) * self.retained_ch + alpha * (1.0 - self.repeated_ch)
    }

    fn score_inc(&self, alpha: f64) -> f64 {
        (1.0 - alpha) * self.retained_inc + alpha * (1.0 - self.repeated_inc)
    }

    /// Δr = retained_ch − retained_inc — zero EXACTLY on the α=0 endpoint
    /// identity (score_ch(0) == score_inc(0): tie → REFUSE under the
    /// strict-`>` rule, strict challenger win for every α > 0 when Δp < 0).
    fn endpoint0(&self) -> bool {
        self.retained_ch == self.retained_inc
    }

    /// The exact crossing α* = Δr/(Δr+Δp); `None` on identical score lines
    /// (Δr + Δp == 0 — a consult whose verdict cannot flip).
    fn crossing(&self) -> Option<f64> {
        let dr = self.retained_ch - self.retained_inc;
        let dp = self.repeated_ch - self.repeated_inc;
        let den = dr + dp;
        if den == 0.0 { None } else { Some(dr / den) }
    }
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
        let value = |it: &mut I, name: &str| it.next().ok_or(format!("{name} requires a value"));
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
    // T3.3 classification counters (plan 633 bars A–D).
    endpoint0_flips: usize,
    endpoint0_all_rejected: usize,
    endpoint0_shape_ok: usize,
    endpoint0_on_accepted_carriers: usize,
    interior_flips: usize,
    no_facts_flips: usize,
    prediction_mismatches: usize,
    tie_proceeds: usize,
    readmit_grid_flips: usize,
    accepted_carrier_refusals: usize,
    accepted_carrier_below_bar: usize,
    accepted_carrier_min_stability: f64,
    audit_only_abstains: usize,
    eviction_abstains: usize,
    demote_refusals: usize,
    readmit_refusals: usize,
    abstains: usize,
    dominant: Option<(Consult, &'static str)>,
    dominant_min_stability: f64,
    dominant_survived_loo: bool,
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

    // T3.3: replay the gate from the rows — the SAME ingest sequence the
    // loader ran (file order), so the rings match the window's internal
    // state bit-for-bit; the prediction-agreement check below doubles as
    // the replay-fidelity check (a wrong ring lens would diverge from the
    // observed shapes and red Bar A).
    let mut replay_gate = AdoptionGate::new();
    for row in &rows {
        replay_gate.ingest(row.accepted, decode_digest(&row.digest_hex));
    }
    let acc_n = replay_gate.accepted().len();
    let rej_n = replay_gate.rejected().len();
    let retained_inc = acc_n as f64 / acc_n.max(1) as f64;
    let repeated_inc = rej_n as f64 / (acc_n + rej_n).max(1) as f64;

    let accepted_of = |tag: &str| rows.iter().filter(|r| r.tag == tag && r.accepted).count();

    // One consult per demote consult reads its α-invariant facts (a demote
    // abstain can only fire when BOTH rings are empty — α-independent, so
    // it never flips; `None` stays a disclosed assertion counter).
    let mut facts: Vec<Option<DemoteFacts>> = Vec::with_capacity(population.len());
    for c in &population {
        facts.push(match c.kind {
            Kind::Demote => match consult(&window, c, args.alpha0) {
                GateVerdict::Proceed(v) | GateVerdict::Refuse(v) => Some(DemoteFacts {
                    retained_ch: v.retained,
                    repeated_ch: v.repeated,
                    retained_inc,
                    repeated_inc,
                }),
                GateVerdict::Abstain => None,
            },
            Kind::Readmit => None,
        });
    }

    // α grid census.
    let mut alpha_flips = 0usize;
    let mut demote_refusals = 0usize;
    let mut readmit_refusals = 0usize;
    let mut abstains = 0usize;
    let mut endpoint0_flips = 0usize;
    let mut endpoint0_all_rejected = 0usize;
    let mut endpoint0_shape_ok = 0usize;
    let mut endpoint0_on_accepted_carriers = 0usize;
    let mut interior_flips = 0usize;
    let mut no_facts_flips = 0usize;
    let mut prediction_mismatches = 0usize;
    let mut tie_proceeds = 0usize;
    let mut readmit_grid_flips = 0usize;
    let mut audit_only_abstains = 0usize;
    let mut eviction_abstains = 0usize;
    for (k, c) in population.iter().enumerate() {
        let baseline = consult(&window, c, args.alpha0);
        let mut shapes = Vec::new();
        let mut shape_history: Vec<GateVerdict> = Vec::new();
        let mut history_alpha = 0.0f64;
        // The grid index where an ENDPOINT-0 flip started (its first
        // PROCEED shape) — the "every later grid point PROCEED" half of
        // Bar B's shape pin is verified after the loop.
        let mut endpoint0_flip_at: Option<usize> = None;
        for &alpha in &args.alphas {
            let v = consult(&window, c, alpha);
            let v_shape = shape(&v);
            if let Some(f) = &facts[k] {
                // Bar A's strongest decidable form: the linear model must
                // predict every observed grid point exactly (this implies
                // every flip bracket contains its α*).
                let predicted_proceed = f.score_ch(alpha) > f.score_inc(alpha);
                let observed_proceed = v_shape == "PROCEED";
                if predicted_proceed != observed_proceed {
                    prediction_mismatches += 1;
                }
                // The extinct tie→PROCEED class: exact score equality
                // (diff == 0) must read REFUSE (decline-first) — a PROCEED
                // on equality means the old tie rule is alive.
                if f.score_ch(alpha) == f.score_inc(alpha) && observed_proceed {
                    tie_proceeds += 1;
                }
            }
            if shape_history
                .last()
                .map(|prev: &GateVerdict| shape(prev) != v_shape)
                .unwrap_or(false)
            {
                let (class_line, class_kind) = match &facts[k] {
                    Some(f) => match (f.endpoint0(), f.crossing()) {
                        (true, _) => ("ENDPOINT-0 (Δr=0)".to_string(), 0usize),
                        (false, Some(a_star)) => (
                            format!(
                                "INTERIOR α*={a_star:.4} (Δr={:+.4} Δp={:+.4})",
                                f.retained_ch - f.retained_inc,
                                f.repeated_ch - f.repeated_inc
                            ),
                            1usize,
                        ),
                        (false, None) => (
                            "IDENTICAL-LINES (Δr+Δp=0 — cannot flip; assertion)".to_string(),
                            2usize,
                        ),
                    },
                    None => (
                        "NO-FACTS (demote abstain — cannot flip; assertion)".to_string(),
                        3usize,
                    ),
                };
                println!(
                    "  [α-flip] {:8} {:28} crosses between α={:.2} and α={:.2}: {} → {} — {}",
                    c.kind.name(),
                    c.tag,
                    history_alpha,
                    alpha,
                    shape_history
                        .last()
                        .map(|p: &GateVerdict| shape(p))
                        .unwrap(),
                    v_shape,
                    class_line,
                );
                match class_kind {
                    0 => {
                        endpoint0_flips += 1;
                        if accepted_of(&c.tag) > 0 {
                            endpoint0_on_accepted_carriers += 1;
                        }
                        if accepted_of(&c.tag) == 0 {
                            endpoint0_all_rejected += 1;
                        }
                        // The bracket must start at 0 (α* = 0) with REFUSE
                        // there; the PROCEED tail is checked post-loop.
                        if history_alpha == 0.0
                            && shape_history.last().map(|p: &GateVerdict| shape(p))
                                == Some("REFUSE")
                        {
                            endpoint0_flip_at = Some(shapes.len());
                        }
                    }
                    1 => interior_flips += 1,
                    2 => {}
                    _ => no_facts_flips += 1,
                }
                if c.kind == Kind::Readmit {
                    readmit_grid_flips += 1;
                }
            }
            history_alpha = alpha;
            shape_history.push(v.clone());
            if alpha == args.alpha0 {
                println!(
                    "  {:8} {:28} α={:.2} {:8} {}",
                    c.kind.name(),
                    c.tag,
                    alpha,
                    v_shape,
                    detail(&v)
                );
            }
            shapes.push(v_shape);
        }
        if let Some(start) = endpoint0_flip_at {
            if shapes[start..].iter().all(|s| *s == "PROCEED") {
                endpoint0_shape_ok += 1;
            }
        }
        shapes.dedup();
        if shapes.len() > 1 {
            alpha_flips += 1;
        }
        match (&baseline, c.kind) {
            (GateVerdict::Refuse(_), Kind::Demote) => demote_refusals += 1,
            (GateVerdict::Refuse(_), Kind::Readmit) => readmit_refusals += 1,
            (GateVerdict::Abstain, Kind::Demote) => {}
            (GateVerdict::Abstain, Kind::Readmit) => {
                abstains += 1;
                // T3.3 Bar D's mechanical classification: every digest the
                // tag ever produced sits in the held-out audit ring →
                // audit-only; anything else (its T-filed digests evicted by
                // the 32-slot FIFO) is the eviction class — disclosed, never
                // pooled.
                let mut digests: Vec<OutcomeDigest> = Vec::new();
                for r in rows.iter().filter(|r| r.tag == c.tag) {
                    let d = decode_digest(&r.digest_hex);
                    if !digests.contains(&d) {
                        digests.push(d);
                    }
                }
                let audit_only =
                    !digests.is_empty() && digests.iter().all(|d| replay_gate.audit().contains(d));
                if audit_only {
                    audit_only_abstains += 1;
                } else {
                    eviction_abstains += 1;
                }
                println!(
                    "  [abstain] readmit {:28} {} ({} distinct digests)",
                    c.tag,
                    if audit_only {
                        "AUDIT-ONLY"
                    } else {
                        "RING-EVICTION"
                    },
                    digests.len(),
                );
            }
            _ => {}
        }
    }

    // Dominant-carrier refusal: among the demote REFUSALS at α0, the rule
    // carrying the most accepted rows (tie → lexicographically first).
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
    // T3.3 Bar C: per-consult MINIMUM agreement across BOTH arms (an arm
    // with no eligible rows cannot lower a consult's stability).
    let mut min_agrees: Vec<usize> = vec![usize::MAX; population.len()];
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
            for (k, a) in agrees.iter().enumerate() {
                min_agrees[k] = min_agrees[k].min(*a);
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

    // T3.3 Bar C: the accepted-carrier demote refusals' per-consult min
    // stability across both noise arms (the quotable set — per-consult,
    // never pooled).
    let mut accepted_carrier_refusals = 0usize;
    let mut accepted_carrier_below_bar = 0usize;
    let mut accepted_carrier_min_stability = 1.0f64;
    for (k, c) in population.iter().enumerate() {
        if c.kind != Kind::Demote || accepted_of(&c.tag) == 0 {
            continue;
        }
        if shape(&consult(&window, c, args.alpha0)) != "REFUSE" {
            continue;
        }
        accepted_carrier_refusals += 1;
        let stab = if min_agrees[k] == usize::MAX {
            1.0
        } else {
            min_agrees[k] as f64 / args.seeds as f64
        };
        if stab < MIN_STABILITY {
            accepted_carrier_below_bar += 1;
            println!(
                "  [bar C] accepted-carrier refusal below bar: {} {} \
                 min-stability {:.3} ({} / {} worst arm)",
                c.kind.name(),
                c.tag,
                stab,
                min_agrees[k],
                args.seeds
            );
        }
        accepted_carrier_min_stability = accepted_carrier_min_stability.min(stab);
    }

    Ok(WindowCell {
        tags: window.tags().len(),
        seat_rows,
        alpha_flips,
        endpoint0_flips,
        endpoint0_all_rejected,
        endpoint0_shape_ok,
        endpoint0_on_accepted_carriers,
        interior_flips,
        no_facts_flips,
        prediction_mismatches,
        tie_proceeds,
        readmit_grid_flips,
        accepted_carrier_refusals,
        accepted_carrier_below_bar,
        accepted_carrier_min_stability,
        audit_only_abstains,
        eviction_abstains,
        demote_refusals,
        readmit_refusals,
        abstains,
        dominant,
        dominant_min_stability,
        dominant_survived_loo,
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

    // The T3.3 pre-registered bars (plan 633 T3.3, pinned BEFORE the
    // cell's sweeps and run) — decidable from the data. The exact-shape
    // pins stay with the bench record; the G3-b/G3-d aggregate names
    // retire into the classified T3.3-A/T3.3-D forms.
    let total_flips: usize = cells.iter().map(|c| c.alpha_flips).sum();
    let endpoint0: usize = cells.iter().map(|c| c.endpoint0_flips).sum();
    let interior: usize = cells.iter().map(|c| c.interior_flips).sum();
    let no_facts: usize = cells.iter().map(|c| c.no_facts_flips).sum();
    let mismatches: usize = cells.iter().map(|c| c.prediction_mismatches).sum();
    let tie_proceeds: usize = cells.iter().map(|c| c.tie_proceeds).sum();
    let readmit_flips: usize = cells.iter().map(|c| c.readmit_grid_flips).sum();
    println!();
    let bar_a = mismatches == 0
        && tie_proceeds == 0
        && no_facts == 0
        && endpoint0 + interior + no_facts == total_flips
        && readmit_flips == 0;
    println!(
        "[bar T3.3-A] α-flip classification over {} windows × {} consults: \
         flips={total_flips} endpoint0={endpoint0} interior={interior} \
         prediction-mismatches={mismatches} tie-PROCEEDs={tie_proceeds} \
         no-facts-flips={no_facts} readmit-grid-flips={readmit_flips} — {}",
        cells.len(),
        cells.iter().map(|c| c.tags * 2).sum::<usize>(),
        if bar_a { "PASS" } else { "FAIL" }
    );
    let ep0: usize = endpoint0;
    let ep0_all_rej: usize = cells.iter().map(|c| c.endpoint0_all_rejected).sum();
    let ep0_shape_ok: usize = cells.iter().map(|c| c.endpoint0_shape_ok).sum();
    let ep0_carriers: usize = cells.iter().map(|c| c.endpoint0_on_accepted_carriers).sum();
    let bar_b = ep0_carriers == 0 && ep0_shape_ok == ep0 && ep0_all_rej == ep0;
    println!(
        "[bar T3.3-B] α=0 endpoint identity: endpoint0 flips={ep0} on-\
         all-rejected-tags={ep0_all_rej} shape-verified={ep0_shape_ok} \
         on-accepted-carriers={ep0_carriers} (bar 0) — {}",
        if bar_b {
            "PASS"
        } else if ep0 == 0 {
            "NOT EXERCISED — no endpoint identity in this store (disclosed)"
        } else {
            "FAIL"
        }
    );
    let ac_ref: usize = cells.iter().map(|c| c.accepted_carrier_refusals).sum();
    let ac_below: usize = cells.iter().map(|c| c.accepted_carrier_below_bar).sum();
    let ac_min = cells
        .iter()
        .map(|c| c.accepted_carrier_min_stability)
        .fold(1.0f64, f64::min);
    println!(
        "[bar T3.3-C] accepted-carrier refusal stability: refusals={ac_ref} \
         min-stability={ac_min:.3} (bar ≥ {MIN_STABILITY:.2}) below-bar={ac_below} — {}",
        if ac_ref == 0 {
            "NOT EXERCISED — zero accepted-carrier refusals (disclosed)"
        } else if ac_below == 0 {
            "PASS"
        } else {
            "FAIL — the below-bar refusals are named above"
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
    let audit_only = cells.iter().map(|c| c.audit_only_abstains).sum::<usize>();
    let eviction = cells.iter().map(|c| c.eviction_abstains).sum::<usize>();
    let bar_d = windows >= args.expect_windows
        && tags >= args.expect_tags
        && demote_ref > 0
        && readmit_ref > 0
        && audit_only > 0;
    println!(
        "[bar T3.3-D] windows={windows} (≥{}) tags={tags} (≥{}) demote-\
         refusals={demote_ref} readmit-refusals={readmit_ref} \
         abstains={abstains} audit-only={audit_only} ring-eviction={eviction} \
         (bar audit-only ≥ 1) — {}",
        args.expect_windows,
        args.expect_tags,
        if bar_d {
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
