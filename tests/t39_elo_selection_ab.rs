//! Issue 039 T5 — the ELO selection challenge gate: BetaPosterior (the
//! promoted default) vs ELO-weighted reliability, at REAL pool sparsity.
//!
//! The issue's design verdict: ELO is FIRST a reported axis; it may challenge
//! BetaPosterior for selection ONLY after beating it on the same bench that
//! killed entropic (Bench 035 / `tests/t26_entropic_max_metric.rs`). This is
//! that bench. A loss keeps ELO report-only (the T2–T4 state: elo math +
//! audit leaderboard + ruliology tie-break, no selection wiring); a strict
//! win would promote it.
//!
//! ## Grounding — the REAL evidence regime (measured 2026-08-25, 4 stores)
//!
//! t26/t33 drew pre-observations uniform 0..=3 — ~40× denser than reality.
//! The real stores (riir-ai 101 rows / riir-game-sdk 8117 / riir-clippy 8 /
//! riir-dapps 2 = 8228 rows) hold `{0: 8106, 1: 110, 2: 9, 3: 2, 4: 1}`
//! outcomes per (rule, fix) row: 98.5% of candidates are UNSEEN and 1.3%
//! carry exactly one observation. The harness draws pre-observations from
//! that measured distribution ([`REAL_ROW_COUNTS`]) — the "real pool
//! sparsity" the issue names as the gate's regime.
//!
//! The rule-level aggregation asymmetry is measured too ([`REAL_RULE_MATCHES`]):
//! ELO ratings key by rule and aggregate across ALL spans (heavy tail — one
//! 86-match rule, a mid band, mostly 1-2-match rules), while the Beta counts
//! key per (rule, fixed_code) and stay thin. The dense cell grants the ELO
//! arm that production form: dense rule history replayed at the rule's
//! long-run rate vs thin per-fix counts for the incumbent, with the current
//! span's true rate deviating by `span_dev`.
//!
//! ## Arms
//!
//! - `LinearBlend` — the raw-rate baseline (t26's).
//! - `BetaPosterior` — the incumbent default.
//! - `EloStatic` — ratings replayed from the SAME pre-observations the
//!   incumbent sees (canonical losses-then-wins order, `recompute_elo`'s
//!   protocol), frozen during the trial — the batch-loaded store form
//!   (`TrajStore::elo_table()`).
//! - `EloOnline` — the same, updated per attempt (the incremental form).
//!
//! ## Metrics (the t26/t33/t93 set, so numbers are comparable)
//!
//! - **Discovery** — `P(≥1 success within BUDGET=3 attempts)`, the max metric.
//! - **Paired McNemar** — common random numbers (pool + outcome table drawn
//!   once, shared by all arms); `z = (b−c)/√(b+c)`.
//! - **Selection regret** — mean `p(best available) − p(first pick)`.
//!
//! ## Promotion gate (the decision rule, encoded + tested)
//!
//! Strict win = discovery strictly greater in BOTH evo-noise cells AND
//! paired McNemar `z > +1.96` in both AND no regret regression. Anything
//! else keeps ELO report-only. The recorded verdict + tables live in
//! `.benchmarks/047_elo_selection_challenge_gate.md`.
//!
//! ```bash
//! cargo test --release --features latent_retrieval --test t39_elo_selection_ab -- --nocapture
//! ```

#![allow(clippy::cast_precision_loss)] // loop-index / count fixtures

use riir_clippy::elo::{default_elo, expected_score, update_pair};

/// Fixed per-span attempt budget — t26/t33's, so the max metric compares.
const BUDGET: u32 = 3;
/// Trials per (arm, noise) cell — t26/t33's.
const TRIALS: usize = 20_000;
/// Trials per paired-McNemar cell — t33's.
const PAIRED_TRIALS: usize = 200_000;
const EPSILON: f32 = 0.05;
/// The pinned production weights (`HealPipeline` Step 4).
const W_EVO: f32 = 0.6;
const W_RATE: f32 = 0.4;

// ── Real-store grounding (frozen 2026-08-25; provenance in Bench 047) ──

/// Per-candidate pre-observation distribution — `(S+F, row count)` over the
/// 8228 rows of the 4 real stores. THE "real pool sparsity": 98.5% unseen,
/// 1.3% single-observation. t26's uniform 0..=3 was ~40× denser.
const REAL_ROW_COUNTS: [(u32, u64); 5] = [(0, 8106), (1, 110), (2, 9), (3, 2), (4, 1)];
/// Total rows backing [`REAL_ROW_COUNTS`].
const REAL_ROW_TOTAL: u64 = 8228;

/// Per-rule aggregate match counts (rules with ≥1 verdict, workspace-wide):
/// `uninlined_format_args` 86, `match_bool` 17, `map_unwrap_or` 15,
/// `single_match_else` 6, then the 1-2-match tail. The evidence density an
/// ELO rule rating actually encodes — the dense cell's popularity source.
const REAL_RULE_MATCHES: [u32; 8] = [86, 17, 15, 6, 2, 2, 2, 1];

// ── Harness machinery (t26/t33's, verbatim where possible) ──

/// Deterministic LCG — identical to t26/t33's.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 40) as f32) / 16_777_216.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.f32() * n as f32) as usize % n.max(1)
    }
}

/// One candidate: a true quality `p`, an observed record, an evolution score.
#[derive(Clone)]
struct Cand {
    p: f32,
    s: u32,
    f: u32,
    evo: f32,
}

fn belief(c: &Cand) -> f32 {
    katgpt_core::best_belief_score(c.s, c.f, EPSILON)
}

fn rate(c: &Cand) -> f32 {
    let total = c.s + c.f;
    if total == 0 {
        0.5
    } else {
        c.s as f32 / total as f32
    }
}

/// The arms under test.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Arm {
    /// `0.6·evo + 0.4·rate` — the raw-rate baseline.
    LinearBlend,
    /// `0.6·evo + 0.4·belief` — the current default (the incumbent).
    BetaPosterior,
    /// `0.6·evo + 0.4·expected_score(rule_elo, span_elo)` — ratings replayed
    /// from the same pre-observations, frozen during the trial (the
    /// batch-loaded store form — what `TrajStore::elo_table()` serves).
    EloStatic,
    /// The ELO arm with per-attempt match updates (the incremental form).
    EloOnline,
}

fn arm_name(arm: Arm) -> &'static str {
    match arm {
        Arm::LinearBlend => "LinearBlend",
        Arm::BetaPosterior => "BetaPosterior",
        Arm::EloStatic => "Elo-static",
        Arm::EloOnline => "Elo-online",
    }
}

/// Per-trial ELO state: one rating per candidate (the rule side) + the shared
/// span rating (the opponent). Built by replaying each candidate's
/// pre-observations vs the span in candidate order, losses-then-wins per
/// candidate — `TrajStore::recompute_elo`'s exact canonical protocol.
struct EloArmState {
    rule: Vec<f32>,
    span: f32,
}

impl EloArmState {
    fn from_preobs(pairs: &[(u32, u32)]) -> Self {
        let mut rule = vec![default_elo(); pairs.len()];
        let mut span = default_elo();
        for (i, &(s, f)) in pairs.iter().enumerate() {
            for _ in 0..f {
                let (r, sp) = update_pair(rule[i], span, false);
                rule[i] = r;
                span = sp;
            }
            for _ in 0..s {
                let (r, sp) = update_pair(rule[i], span, true);
                rule[i] = r;
                span = sp;
            }
        }
        Self { rule, span }
    }

    fn reliability(&self, i: usize) -> f32 {
        expected_score(self.rule[i], self.span)
    }

    fn play(&mut self, i: usize, won: bool) {
        let (r, sp) = update_pair(self.rule[i], self.span, won);
        self.rule[i] = r;
        self.span = sp;
    }
}

/// One paired trial's pool + outcome table (common random numbers — drawn
/// once, shared by every arm: candidate `c`'s `k`-th attempt has the same
/// outcome whichever arm attempted it).
struct Pool {
    cands: Vec<Cand>,
    outcomes: Vec<[bool; BUDGET as usize]>,
}

/// Sample a pre-observation count from [`REAL_ROW_COUNTS`] (inverse-CDF over
/// the LCG draw) — the grounding half of the harness.
fn sample_real_n(rng: &mut Lcg) -> u32 {
    let x = f64::from(rng.f32()) * REAL_ROW_TOTAL as f64;
    let mut acc = 0u64;
    for &(n, c) in &REAL_ROW_COUNTS {
        acc += c;
        if x < acc as f64 {
            return n;
        }
    }
    0
}

/// t26's pool shapes (rare-good half / uniform half — comparability with the
/// recorded t26/t33 baselines), with pre-observations at REAL sparsity.
fn draw_pool(rng: &mut Lcg, evo_noise: f32) -> Pool {
    let n_cands = 2 + rng.below(4);
    let rare_good = rng.f32() < 0.5;
    let mut cands = Vec::with_capacity(n_cands);
    let mut outcomes = Vec::with_capacity(n_cands);
    for idx in 0..n_cands {
        let p = if rare_good && idx == 0 {
            0.4 + rng.f32() * 0.5
        } else if rare_good {
            0.02 + rng.f32() * 0.18
        } else {
            rng.f32()
        };
        let n = sample_real_n(rng);
        let s = (0..n).filter(|_| rng.f32() < p).count() as u32;
        let evo = (p + (rng.f32() - 0.5) * evo_noise).clamp(0.0, 1.0);
        let mut draws = [false; BUDGET as usize];
        for d in &mut draws {
            *d = rng.f32() < p;
        }
        cands.push(Cand {
            p,
            s,
            f: n - s,
            evo,
        });
        outcomes.push(draws);
    }
    Pool { cands, outcomes }
}

/// Mirror of `HealPipeline::heal` Step 4 for the count-based arms.
fn pick_counts(arm: Arm, cands: &[Cand]) -> usize {
    let mut best = (0usize, f32::NEG_INFINITY);
    for (i, c) in cands.iter().enumerate() {
        let reliability = if arm == Arm::LinearBlend {
            rate(c)
        } else {
            belief(c)
        };
        let score = W_EVO * c.evo + W_RATE * reliability;
        if score > best.1 {
            best = (i, score);
        }
    }
    best.0
}

/// Step 4 for the ELO arms.
fn pick_elo(cands: &[Cand], elo: &EloArmState) -> usize {
    let mut best = (0usize, f32::NEG_INFINITY);
    for (i, c) in cands.iter().enumerate() {
        let score = W_EVO * c.evo + W_RATE * elo.reliability(i);
        if score > best.1 {
            best = (i, score);
        }
    }
    best.0
}

/// One trial outcome for the metric set.
struct TrialOut {
    found: bool,
    first_pick: usize,
    /// Attempts used (BUDGET+1 if never found) — t26's second column.
    attempts: u32,
}

/// Replay one arm over a shared pool. `elo_preobs` is what the ELO arm's
/// ratings replay from: the counts themselves in the sparse cells (equal
/// evidence — the parameterization A/B), or the dense rule history in the
/// dense cell (the production form).
fn replay_with(
    arm: Arm,
    cands: &[Cand],
    elo_preobs: &[(u32, u32)],
    outcomes: &[[bool; BUDGET as usize]],
) -> TrialOut {
    let mut cands: Vec<Cand> = cands.to_vec();
    let mut used = vec![0usize; cands.len()];
    let mut elo = EloArmState::from_preobs(elo_preobs);
    let mut first_pick = usize::MAX;
    for attempt in 1..=BUDGET {
        let choice = match arm {
            Arm::LinearBlend | Arm::BetaPosterior => pick_counts(arm, &cands),
            Arm::EloStatic | Arm::EloOnline => pick_elo(&cands, &elo),
        };
        if first_pick == usize::MAX {
            first_pick = choice;
        }
        let k = used[choice].min(BUDGET as usize - 1);
        used[choice] += 1;
        let hit = outcomes[choice][k];
        if arm == Arm::EloOnline {
            elo.play(choice, hit);
        }
        if hit {
            return TrialOut {
                found: true,
                first_pick,
                attempts: attempt,
            };
        }
        cands[choice].f += 1;
    }
    TrialOut {
        found: false,
        first_pick,
        attempts: BUDGET + 1,
    }
}

fn replay(arm: Arm, pool: &Pool) -> TrialOut {
    let preobs: Vec<(u32, u32)> = pool.cands.iter().map(|c| (c.s, c.f)).collect();
    replay_with(arm, &pool.cands, &preobs, &pool.outcomes)
}

/// Per-cell aggregate: (discovery, mean attempts, mean first-pick regret).
fn run_cell(arm: Arm, seed: u64, evo_noise: f32) -> (f64, f64, f64) {
    let mut rng = Lcg::new(seed);
    let (mut found, mut attempts, mut regret) = (0u64, 0u64, 0.0f64);
    for _ in 0..TRIALS {
        let pool = draw_pool(&mut rng, evo_noise);
        let out = replay(arm, &pool);
        let max_p = pool
            .cands
            .iter()
            .map(|c| c.p)
            .fold(f32::NEG_INFINITY, f32::max);
        regret += f64::from(max_p - pool.cands[out.first_pick].p);
        if out.found {
            found += 1;
        }
        attempts += u64::from(out.attempts);
    }
    let n = TRIALS as f64;
    (found as f64 / n, attempts as f64 / n, regret / n)
}

// ── The promotion gate (the decision rule, encoded) ──

/// One challenger-vs-incumbent cell result feeding [`promotion_gate`].
#[derive(Clone, Copy, Debug)]
struct GateCell {
    challenger_discovery: f64,
    incumbent_discovery: f64,
    /// Paired McNemar z (positive = challenger better).
    z: f64,
    challenger_regret: f64,
    incumbent_regret: f64,
}

/// The T5 decision rule: promote ONLY on a strict win — discovery strictly
/// greater in EVERY cell, paired `z > +1.96` in every cell, and no regret
/// regression. Anything else keeps ELO report-only (the issue's honest-loss
/// branch).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GateVerdict {
    Promote,
    ReportOnly,
}

fn promotion_gate(cells: &[GateCell]) -> GateVerdict {
    let strict = cells.iter().all(|c| {
        c.challenger_discovery > c.incumbent_discovery
            && c.z > 1.96
            && c.challenger_regret <= c.incumbent_regret + 1e-9
    });
    if strict {
        GateVerdict::Promote
    } else {
        GateVerdict::ReportOnly
    }
}

// ── Tests ──

/// The headline cells: discovery + attempts + regret at REAL sparsity, both
/// evo-noise regimes, all four arms. Determinism pinned by re-run.
#[test]
fn t39_real_sparsity_discovery_ab() {
    println!("\n=== Issue 039 T5 — discovery at REAL pool sparsity (BUDGET {BUDGET}, TRIALS {TRIALS}) ===");
    println!(
        "  pre-obs from REAL_ROW_COUNTS {{0:8106, 1:110, 2:9, 3:2, 4:1}} / 8228 — 98.5% unseen"
    );
    for &evo_noise in &[0.6f32, 0.2] {
        println!("  evo_noise = {evo_noise}:");
        for arm in [
            Arm::LinearBlend,
            Arm::BetaPosterior,
            Arm::EloStatic,
            Arm::EloOnline,
        ] {
            let (disc, att, reg) = run_cell(arm, 0x5EED_0039, evo_noise);
            println!(
                "    {:<13} discovery {:.4}  mean-attempts {:.3}  first-pick regret {:.4}",
                arm_name(arm),
                disc,
                att,
                reg
            );
        }
        // Determinism pin: two arms' cells reproduce bit-identically.
        let (d1, _, r1) = run_cell(Arm::BetaPosterior, 0x5EED_0039, evo_noise);
        let (d2, _, r2) = run_cell(Arm::EloStatic, 0x5EED_0039, evo_noise);
        let (d1b, _, r1b) = run_cell(Arm::BetaPosterior, 0x5EED_0039, evo_noise);
        let (d2b, _, r2b) = run_cell(Arm::EloStatic, 0x5EED_0039, evo_noise);
        assert_eq!(
            d1.to_bits(),
            d1b.to_bits(),
            "BetaPosterior cell must be deterministic"
        );
        assert_eq!(
            d2.to_bits(),
            d2b.to_bits(),
            "Elo-static cell must be deterministic"
        );
        assert_eq!(r1.to_bits(), r1b.to_bits(), "regret must be deterministic");
        assert_eq!(
            r2.to_bits(),
            r2b.to_bits(),
            "ELO regret must be deterministic"
        );
    }
}

// ── The recorded verdict (Bench 047; measured 2026-08-25, release, this
// box's stores — the seeds are fixed so re-runs reproduce these cells; the
// live gate assertion in `t39_paired_mcnemar_vs_beta` re-derives the verdict
// every run) ──

/// `EloOnline` (the best challenger): tiny significant win at evo-noise 0.6
/// (+0.0004, z = +3.63), NO significant difference at 0.2 (z = +0.16).
const RECORDED_ELO_ONLINE: [GateCell; 2] = [
    GateCell {
        challenger_discovery: 0.9207,
        incumbent_discovery: 0.9203,
        z: 3.63,
        challenger_regret: 0.0291,
        incumbent_regret: 0.0292,
    },
    GateCell {
        challenger_discovery: 0.9423,
        incumbent_discovery: 0.9422,
        z: 0.16,
        challenger_regret: 0.0027,
        incumbent_regret: 0.0028,
    },
];

/// `EloStatic` (the exact store form — batch-loaded, frozen):
/// SIGNIFICANTLY WORSE at evo-noise 0.6 (z = −8.94), NS at 0.2. The frozen
/// ratings give up the within-trial learning that is nearly all the signal
/// there is at real sparsity (98.5% of candidates are unseen — the arms'
/// pre-observations carry almost nothing; the difference is what the arm
/// learns DURING the 3 attempts).
const RECORDED_ELO_STATIC: [GateCell; 2] = [
    GateCell {
        challenger_discovery: 0.9189,
        incumbent_discovery: 0.9203,
        z: -8.94,
        challenger_regret: 0.0291,
        incumbent_regret: 0.0292,
    },
    GateCell {
        challenger_discovery: 0.9423,
        incumbent_discovery: 0.9422,
        z: 0.35,
        challenger_regret: 0.0027,
        incumbent_regret: 0.0028,
    },
];

/// The recorded T5 verdict (Bench 047): **ReportOnly** — no ELO variant
/// achieves the strict win, so ELO stays exactly where T2–T4 put it (audit
/// §1b leaderboard + ruliology tie-break; no selection wiring). This pins
/// the DECISION against accidental future promotion without re-running the
/// gate; the live assertion in `t39_paired_mcnemar_vs_beta` re-derives it
/// from a fresh measurement every run.
#[test]
fn t39_recorded_verdict_is_report_only() {
    assert_eq!(
        promotion_gate(&RECORDED_ELO_ONLINE),
        GateVerdict::ReportOnly,
        "Elo-online: z=+0.16 at evo-noise 0.2 is not a strict win"
    );
    assert_eq!(
        promotion_gate(&RECORDED_ELO_STATIC),
        GateVerdict::ReportOnly,
        "Elo-static: z=\u{2212}8.94 at evo-noise 0.6 is significantly worse"
    );
    // The mechanism pin: the store's exact batch form is the WORSE arm —
    // within-trial learning, not the rating parameterization, is what
    // moves selection at real sparsity.
    assert!(RECORDED_ELO_STATIC[0].z < -1.96);
}

/// The paired gate: challenger vs incumbent under common random numbers,
/// McNemar discordance — the promotion gate's z source. Both challengers,
/// both noise regimes. Self-verifying: the LIVE cells must reproduce the
/// recorded verdict (ReportOnly) — re-measured every run, robust to ulp-level
/// platform drift (the margins are far from the threshold).
#[test]
fn t39_paired_mcnemar_vs_beta() {
    println!("\n=== Issue 039 T5 — paired discovery vs BetaPosterior (common random numbers, {PAIRED_TRIALS} trials) ===");
    let mut live_cells: Vec<(Arm, [GateCell; 2])> = Vec::new();
    for &evo_noise in &[0.6f32, 0.2] {
        println!("  evo_noise = {evo_noise}:");
        for challenger in [Arm::EloStatic, Arm::EloOnline] {
            let mut rng = Lcg::new(0x9A17_0039);
            let (mut b, mut c, mut both, mut neither) = (0u64, 0u64, 0u64, 0u64);
            let (mut inc_found, mut ch_found) = (0u64, 0u64);
            let (mut inc_regret, mut ch_regret) = (0.0f64, 0.0f64);
            for _ in 0..PAIRED_TRIALS {
                let pool = draw_pool(&mut rng, evo_noise);
                let inc = replay(Arm::BetaPosterior, &pool);
                let ch = replay(challenger, &pool);
                let max_p = pool
                    .cands
                    .iter()
                    .map(|x| x.p)
                    .fold(f32::NEG_INFINITY, f32::max);
                inc_regret += f64::from(max_p - pool.cands[inc.first_pick].p);
                ch_regret += f64::from(max_p - pool.cands[ch.first_pick].p);
                inc_found += u64::from(inc.found);
                ch_found += u64::from(ch.found);
                match (ch.found, inc.found) {
                    (true, true) => both += 1,
                    (true, false) => b += 1,
                    (false, true) => c += 1,
                    (false, false) => neither += 1,
                }
            }
            let n = PAIRED_TRIALS as f64;
            let disc = (b + c) as f64;
            let z = if disc > 0.0 {
                (b as f64 - c as f64) / disc.sqrt()
            } else {
                0.0
            };
            println!(
                "    {:<13} {:.4} vs {:.4}  (both {both}, challenger-only {b}, incumbent-only {c}, neither {neither})",
                arm_name(challenger),
                ch_found as f64 / n,
                inc_found as f64 / n
            );
            println!(
                "      discordant {disc:.0}  z = {z:+.2}  regret {:.4} vs {:.4}  -> {}",
                ch_regret / n,
                inc_regret / n,
                if z.abs() < 1.96 {
                    "NO significant difference (|z| < 1.96)"
                } else if z > 0.0 {
                    "challenger SIGNIFICANTLY better"
                } else {
                    "challenger SIGNIFICANTLY worse"
                }
            );
            let cell = GateCell {
                challenger_discovery: ch_found as f64 / n,
                incumbent_discovery: inc_found as f64 / n,
                z,
                challenger_regret: ch_regret / n,
                incumbent_regret: inc_regret / n,
            };
            match live_cells.iter_mut().find(|(a, _)| *a == challenger) {
                Some((_, cells)) => cells[usize::from(evo_noise < 0.4)] = cell,
                None => {
                    let mut cells = [GateCell {
                        challenger_discovery: 0.0,
                        incumbent_discovery: 0.0,
                        z: 0.0,
                        challenger_regret: 0.0,
                        incumbent_regret: 0.0,
                    }; 2];
                    cells[usize::from(evo_noise < 0.4)] = cell;
                    live_cells.push((challenger, cells));
                }
            }
        }
    }
    // Self-verifying: the LIVE measurement must reproduce the recorded
    // verdict — ELO stays report-only (Bench 047). Robust to ulp-level
    // platform drift: the failing margins (z=+0.16 / z=+0.35 at 0.2; the
    // \u{2212}8.94 loss at 0.6) are far from the 1.96 threshold.
    for (arm, cells) in &live_cells {
        assert_eq!(
            promotion_gate(cells),
            GateVerdict::ReportOnly,
            "live gate must keep {:?} report-only",
            arm
        );
    }
}

/// The production-form asymmetry cell: the ELO arm sees DENSE rule ratings
/// (history replayed at the rule's long-run rate, popularity from
/// [`REAL_RULE_MATCHES`]) while the incumbent keeps its thin per-fix counts —
/// the seam a promoted ELO selection would actually wire. `span_dev` controls
/// how far the current span's true rate deviates from the rule's long-run
/// rate (the quantity dense history CANNOT see).
#[test]
fn t39_dense_rule_evidence_cells() {
    println!("\n=== Issue 039 T5 — dense rule evidence (the production form) ===");
    println!("  rule popularity from REAL_RULE_MATCHES [86, 17, 15, 6, 2, 2, 2, 1]; Beta keeps thin per-fix counts");
    for &span_dev in &[0.2f32, 0.6] {
        for &evo_noise in &[0.6f32, 0.2] {
            let mut rng = Lcg::new(0x0D3E_0039);
            let (mut b, mut c, mut both, mut neither) = (0u64, 0u64, 0u64, 0u64);
            let (mut inc_found, mut elo_found, mut elo_on_found) = (0u64, 0u64, 0u64);
            for _ in 0..TRIALS {
                // Pool: rule long-run rate q from t26's shapes; current-span
                // p = clamp(q + dev); thin counts Bernoulli(q) at REAL
                // sparsity; evo tracks p; dense history Bernoulli(q) × pop.
                let n_cands = 2 + rng.below(4);
                let rare_good = rng.f32() < 0.5;
                let mut cands = Vec::with_capacity(n_cands);
                let mut hist = Vec::with_capacity(n_cands);
                let mut outcomes = Vec::with_capacity(n_cands);
                for idx in 0..n_cands {
                    let q = if rare_good && idx == 0 {
                        0.4 + rng.f32() * 0.5
                    } else if rare_good {
                        0.02 + rng.f32() * 0.18
                    } else {
                        rng.f32()
                    };
                    let p = (q + (rng.f32() - 0.5) * span_dev).clamp(0.0, 1.0);
                    let n = sample_real_n(&mut rng);
                    let s = (0..n).filter(|_| rng.f32() < q).count() as u32;
                    let evo = (p + (rng.f32() - 0.5) * evo_noise).clamp(0.0, 1.0);
                    let pop = REAL_RULE_MATCHES[rng.below(REAL_RULE_MATCHES.len())];
                    let hs = (0..pop).filter(|_| rng.f32() < q).count() as u32;
                    let mut draws = [false; BUDGET as usize];
                    for d in &mut draws {
                        *d = rng.f32() < p;
                    }
                    cands.push(Cand {
                        p,
                        s,
                        f: n - s,
                        evo,
                    });
                    hist.push((hs, pop - hs));
                    outcomes.push(draws);
                }
                let inc = replay_with(Arm::BetaPosterior, &cands, &hist, &outcomes);
                let elo = replay_with(Arm::EloStatic, &cands, &hist, &outcomes);
                let elo_on = replay_with(Arm::EloOnline, &cands, &hist, &outcomes);
                inc_found += u64::from(inc.found);
                elo_found += u64::from(elo.found);
                elo_on_found += u64::from(elo_on.found);
                match (elo.found, inc.found) {
                    (true, true) => both += 1,
                    (true, false) => b += 1,
                    (false, true) => c += 1,
                    (false, false) => neither += 1,
                }
            }
            let n = TRIALS as f64;
            let disc = (b + c) as f64;
            let z = if disc > 0.0 {
                (b as f64 - c as f64) / disc.sqrt()
            } else {
                0.0
            };
            println!(
                "  span_dev {span_dev} evo_noise {evo_noise}: Beta {:.4}  Elo-dense {:.4}  Elo-dense-online {:.4}",
                inc_found as f64 / n,
                elo_found as f64 / n,
                elo_on_found as f64 / n
            );
            println!(
                "    paired (Elo-dense vs Beta): both {both}, elo-only {b}, beta-only {c}, neither {neither}  z = {z:+.2}"
            );
        }
    }
}

/// Structure pin: an all-unseen pool (the 98.5% regime) is decided by the
/// evo axis identically in every arm — the count-based reliabilities tie at
/// `belief(0,0)` and the ELO reliabilities tie at `expected_score(1200,1200)`
/// exactly. At real sparsity this is why ~96% of pools are concordant by
/// construction and the paired metric lives on the rare evidence-bearing pool.
#[test]
fn t39_unseen_pool_all_arms_pick_identically() {
    let pool = Pool {
        cands: vec![
            Cand {
                p: 0.30,
                s: 0,
                f: 0,
                evo: 0.40,
            },
            Cand {
                p: 0.85,
                s: 0,
                f: 0,
                evo: 0.75,
            },
            Cand {
                p: 0.50,
                s: 0,
                f: 0,
                evo: 0.55,
            },
        ],
        outcomes: vec![[false; BUDGET as usize]; 3],
    };
    for arm in [
        Arm::LinearBlend,
        Arm::BetaPosterior,
        Arm::EloStatic,
        Arm::EloOnline,
    ] {
        let out = replay(arm, &pool);
        assert_eq!(
            out.first_pick, 1,
            "{:?} must pick the max-evo candidate",
            arm
        );
    }
    // Equal evos too: every arm ties completely and takes the first index
    // (production `select_best_candidate` keeps the earlier on `>`).
    let tied = Pool {
        cands: vec![
            Cand {
                p: 0.3,
                s: 0,
                f: 0,
                evo: 0.5,
            },
            Cand {
                p: 0.9,
                s: 0,
                f: 0,
                evo: 0.5,
            },
        ],
        outcomes: vec![[false; BUDGET as usize]; 2],
    };
    for arm in [
        Arm::LinearBlend,
        Arm::BetaPosterior,
        Arm::EloStatic,
        Arm::EloOnline,
    ] {
        assert_eq!(
            replay(arm, &tied).first_pick,
            0,
            "{:?} ties must keep first index",
            arm
        );
    }
}

/// Mechanism pin: on the ONE observation real pools actually carry, the Beta
/// belief swing (1/0 vs 0/1 spread ≈ 0.198) is >4× the ELO expected-score
/// swing (±16 Elo ≈ ±0.023, spread ≈ 0.046) — the quantitative reason ELO
/// discriminates thin evidence weaker than the incumbent at real sparsity.
#[test]
fn t39_beta_spread_exceeds_elo_spread_on_one_observation() {
    let beta_up = katgpt_core::best_belief_score(1, 0, EPSILON);
    let beta_down = katgpt_core::best_belief_score(0, 1, EPSILON);
    let beta_spread = beta_up - beta_down;

    // One match from base moves ±K/2 = ±16 exactly (the elo.rs fixture).
    let (r_win, _) = update_pair(default_elo(), default_elo(), true);
    let (r_loss, _) = update_pair(default_elo(), default_elo(), false);
    let elo_spread = expected_score(r_win, default_elo()) - expected_score(r_loss, default_elo());

    println!("\nT5 mechanism: beta spread {beta_spread:.4} vs elo spread {elo_spread:.4}");
    assert!(
        beta_spread > 4.0 * elo_spread,
        "beta one-observation spread {beta_spread:.4} must exceed 4x elo's {elo_spread:.4}"
    );
}

/// The gate logic itself, on synthetic cells — the decision rule is code,
/// not prose, and its both branches are pinned before the recorded verdict
/// uses it.
#[test]
fn t39_promotion_gate_logic() {
    let win = GateCell {
        challenger_discovery: 0.95,
        incumbent_discovery: 0.90,
        z: 3.0,
        challenger_regret: 0.10,
        incumbent_regret: 0.12,
    };
    assert_eq!(promotion_gate(&[win, win]), GateVerdict::Promote);

    // Discovery not strictly better somewhere -> report-only.
    let flat = GateCell {
        challenger_discovery: 0.90,
        incumbent_discovery: 0.90,
        ..win
    };
    assert_eq!(promotion_gate(&[win, flat]), GateVerdict::ReportOnly);

    // Not significant -> report-only ("no per-seed regression beyond noise"
    // cuts both ways: an unproven win is not a win).
    let ns = GateCell { z: 1.5, ..win };
    assert_eq!(promotion_gate(&[win, ns]), GateVerdict::ReportOnly);

    // Regret regression -> report-only.
    let reg = GateCell {
        challenger_regret: 0.13,
        incumbent_regret: 0.12,
        ..win
    };
    assert_eq!(promotion_gate(&[win, reg]), GateVerdict::ReportOnly);

    // Significantly WORSE -> report-only (the honest-loss branch).
    let worse = GateCell {
        challenger_discovery: 0.85,
        incumbent_discovery: 0.90,
        z: -3.0,
        ..win
    };
    assert_eq!(promotion_gate(&[worse]), GateVerdict::ReportOnly);
}

/// Live store grounding (context only — the frozen constants above are the
/// grounding). Walks the sibling repos, and for each store PRESENT opens it
/// read-path (`dump` + `elo_table` — NEVER `recompute_elo`, the T4 decision)
/// and prints the live sparsity + standings. Never creates a store.
#[cfg(feature = "latent_retrieval")]
#[test]
fn t39_store_grounding_report() {
    println!("\n=== Issue 039 T5 — live store grounding (load, never recompute) ===");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let repos = ["riir-ai", "riir-game-sdk", "riir-clippy", "riir-dapps"];
    for repo in repos {
        let root = std::path::Path::new(manifest_dir)
            .parent()
            .map(|p| p.join(repo))
            .unwrap_or_else(|| std::path::PathBuf::from(repo));
        let store_dir = root.join(".heal").join("trajectories.kv");
        if !store_dir.is_dir() {
            continue; // never create — the read path only
        }
        let Ok((store, _bad_rows)) = riir_clippy::traj_store::TrajStore::open(&root) else {
            println!("  {repo}: store present but failed to open");
            continue;
        };
        let rows = store.dump();
        let matched: usize = rows
            .iter()
            .filter(|r| r.success_count.saturating_add(r.fail_count) > 0)
            .count();
        let table = store.elo_table();
        println!(
            "  {repo}: {} rows, {matched} with verdicts, {} rated player(s)",
            rows.len(),
            table.len()
        );
        let mut top: Vec<(f32, String)> = Vec::new();
        // elo_table answers base for unseen players; collect via row keys.
        let mut players: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for r in &rows {
            players.insert(r.lint_key.clone());
        }
        for p in &players {
            if table.contains(p) {
                top.push((table.rating(p), p.clone()));
            }
        }
        top.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (elo, name) in top.iter().take(4) {
            println!("    {elo:7.1}  {name}");
        }
    }
}
