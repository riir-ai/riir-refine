//! Issue 093 T1 — per-run strategy-posterior PoC (defend-wrong).
//!
//! Pits a per-run `NonergodicFilter` (katgpt-rs Plan 592, Research 545 §4's
//! "file-level posterior, nonergodic upgrade of the router") against the
//! SHIPPED static shape-conditional `BetaPosterior` ordering (the real
//! `traj_store::shape_conditional_order` over a seeded `TrajStore`) and the
//! no-posterior min-delta ordering (`driver::order_candidates` semantics),
//! on a toy multi-error file corpus with scripted fix outcomes.
//!
//! NO cargo, NO rustc — the oracle is a frozen outcome table per
//! (file, site, strategy). The driver loop mirrors
//! `domains/rustc_errors/driver.rs` faithfully: propose → order → apply best
//! → recheck → keep|revert (keep iff the attempt resolved its error), the
//! `tried` suppression set, `max_iters = 5`.
//!
//! Arms:
//! - (A) no-posterior — static min-delta ordering (line/code/len/id).
//! - (B) shipped static shape-conditional ordering — the REAL function, over
//!   a seeded temp `TrajStore` (cross-run prior, frozen within the run).
//! - (C) per-run `NonergodicFilter<4, 2>` over 4 regime generators, uniform
//!   prior — within-run belief sharpening only, plus revive-on-contradiction
//!   (a suppressed candidate may retry when the filter's committed regime
//!   flips AND `revive_gate()` clears the threshold).
//! - (D) composition — the production shape T2 would wire: score blends the
//!   store's shape-conditional belief with the filter's predicted success,
//!   λ_t = t/(t+1) shifting from store to filter as the run accumulates
//!   attempts. (B is λ≡0, C is λ≡1.)
//!
//! Scenarios (SAME world/scripted outcomes across all three — the store is
//! the only thing that changes):
//! - aligned — store history drawn from the world's true regime tables
//!   (arm B's best case: near-oracle priors).
//! - stale — same shapes, outcomes drawn from regime-swapped tables (R0↔R1,
//!   R2↔R3): the world changed under the store; arm B is systematically
//!   misled.
//! - cold — empty store: unseen everywhere, arm B degrades to min-delta.
//!
//! Defend-wrong discipline: the corpus must give the BASELINES their shot —
//! the aligned scenario is arm B's home ground (the filter starts uniform
//! with ≤5 observations; if it "wins" there too, the harness is rigged, and
//! that is the wrong this PoC exists to defend against). A misspecification
//! pass perturbs the filter's model tables ±20% (world unchanged) to check
//! the filter's win — if any — is not an oracle artifact.
//!
//! Metrics: resolved-within-budget % (all sites cleared in ≤5 attempts) and
//! wasted-attempt rate (attempts that did not resolve). Paired per-file
//! McNemar tables vs arms A and B. Deterministic: SplitMix64 streams, fixed
//! draw order, sorted iteration everywhere.
//!
//! ```text
//! cargo run --example nonergodic_driver_poc --features nonergodic_poc
//! cargo run --example nonergodic_driver_poc --features nonergodic_poc -- --misspec
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use katgpt_micro_belief::nonergodic::{ComponentModel, NonergodicFilter};
use riir_refine::domains::rustc_errors::evolve::traj_key;
use riir_refine::domains::rustc_errors::playbooks::FixCandidate;
use riir_refine::traj_store::{TrajStore, shape_conditional_order, shape_conditional_score};

// ─────────────────────────────────────────────────────────────────────────────
// Constants — the toy world
// ─────────────────────────────────────────────────────────────────────────────

/// The strategy menu: REAL playbook ids (borrowck.rs + e0597.rs), with each
/// strategy's typical replacement text (the min-delta `total_len` key).
const STRATS: [(&str, &str, &str); N_STRATS] = [
    ("e0502-deref-removal", "E0502", ""),
    ("e0502-clone-out", "E0502", "head.clone()"),
    (
        "e0499-swap-second-decl",
        "E0499",
        "let head = v; use(head);",
    ),
    ("e0505-clone-at-move", "E0505", "let v = v.clone();"),
    ("e0597-anon-lifetime", "E0597", "fn f<'_>(x: &'_ T)"),
    ("e0597-move", "E0597", "move |x| x"),
    ("e0597-move-clone", "E0597", "move |x| x.clone()"),
    ("e0597-drop-the-box", "E0597", "let owned = *boxed;"),
    ("e0597-shared-ownership", "E0597", "Rc::new(value)"),
    (
        "e0597-scope-hoist",
        "E0597",
        "{ let inner = make(); take(inner); }",
    ),
];
const N_STRATS: usize = 10;
const CODES: [&str; 4] = ["E0502", "E0499", "E0505", "E0597"];

/// Menu per code (indices into `STRATS`).
fn menu(code: &str) -> &'static [usize] {
    match code {
        "E0502" => &[0, 1],
        "E0499" => &[2],
        "E0505" => &[3],
        _ => &[4, 5, 6, 7, 8, 9],
    }
}

/// Outcome of one attempt (mirrors the driver's keep/revert rule: keep iff
/// the error count strictly decreased — `Resolved` is the only keep).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Outcome {
    Resolved,
    NoProgress,
    NewErrors,
}
const OUTCOMES: [Outcome; 3] = [Outcome::Resolved, Outcome::NoProgress, Outcome::NewErrors];

const K_REGIMES: usize = 4;
/// Inner-state dimension per regime model: {tractable, intractable}. The
/// world's per-file difficulty is FIXED; the sticky-transition HMM is the
/// filter's model of it (misspecified on dynamics, correct on emissions).
const D_STATES: usize = 2;

/// Success prob per (regime, strategy) — each regime has a distinct winning
/// family (deref / clone / scope / lifetime), the structure ordering must
/// discover. Shared by the world oracle AND the well-specified filter models
/// (one source of truth; the misspec pass perturbs the model copy only).
const SUCCESS: [[f32; N_STRATS]; K_REGIMES] = [
    [0.85, 0.35, 0.60, 0.55, 0.50, 0.30, 0.40, 0.25, 0.20, 0.45],
    [0.25, 0.88, 0.40, 0.80, 0.35, 0.30, 0.60, 0.30, 0.70, 0.30],
    [0.30, 0.40, 0.85, 0.35, 0.55, 0.60, 0.35, 0.30, 0.25, 0.88],
    [0.25, 0.30, 0.35, 0.30, 0.85, 0.70, 0.45, 0.60, 0.40, 0.50],
];
/// P(new errors) per regime — the wasted-attempt amplifier.
const NEW_ERR: [f32; K_REGIMES] = [0.08, 0.08, 0.08, 0.15];
/// Regime-conditional site code distribution (E0597 is the rich 6-strategy
/// menu — the regime where ordering matters most gets the most sites).
const CODE_DIST: [[f32; 4]; K_REGIMES] = [
    [0.45, 0.20, 0.15, 0.20],
    [0.30, 0.10, 0.30, 0.30],
    [0.15, 0.40, 0.15, 0.30],
    [0.10, 0.10, 0.10, 0.70],
];

/// Emission table per (regime, hard): `[strat][outcome]` with outcome order
/// [Resolved, NoProgress, NewErrors]. The hard-file transform is uniformly
/// worse AND rank-shifted — the blunt bandaid strategies (drop-the-box,
/// scope-hoist) gain on messy files. The rank shift is what makes the inner
/// state matter for ORDERING, not just level.
fn outcome_table(regime: usize, hard: bool) -> [[f32; 3]; N_STRATS] {
    let mut t = [[0.0f32; 3]; N_STRATS];
    for s in 0..N_STRATS {
        let (mut succ, mut new) = (SUCCESS[regime][s], NEW_ERR[regime]);
        if hard {
            succ = (succ * 0.35).max(0.03);
            if s == 7 || s == 9 {
                succ = (succ + 0.15).min(0.9);
            }
            new = (new * 1.5).min(0.5);
        }
        let no = (1.0 - succ - new).max(0.02);
        let tot = succ + new + no;
        t[s] = [succ / tot, no / tot, new / tot];
    }
    t
}

const MAX_ITERS: usize = 5;
const N_FILES: usize = 240;
const N_SEEDS: u64 = 5;
/// One fix can cascade (clear one more site) — same dynamics for every arm.
const CASCADE_P: f32 = 0.2;
/// Revive threshold: `revive_gate() = sigmoid(-margin)` must clear this AND
/// the filter's committed regime must have flipped since suppression.
const REVIVE_GATE: f32 = 0.35;
const P_HARD: f32 = 0.3;

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic RNG (SplitMix64 — fixed draw order everywhere)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }
    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[(self.next_u64() % items.len() as u64) as usize]
    }
    /// Index draw from a normalized weight row.
    fn weighted(&mut self, w: &[f32]) -> usize {
        let u = self.f32();
        let mut acc = 0.0f32;
        for (i, wi) in w.iter().enumerate() {
            acc += wi;
            if u < acc {
                return i;
            }
        }
        w.len() - 1
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shape text — regime-correlated vocabulary (gives the REAL shape-conditional
// ordering its retrieval signal: same-regime shapes share words)
// ─────────────────────────────────────────────────────────────────────────────

const REGIME_VOCAB: [[&str; 4]; K_REGIMES] = [
    ["copy", "deref", "index", "slice"],
    ["clone", "box", "arc", "heap"],
    ["scope", "block", "hoist", "reorder"],
    ["lifetime", "closure", "escape", "return"],
];
const CODE_VOCAB: [[&str; 3]; 4] = [
    ["immutable", "borrow", "push"],
    ["second", "mutable", "swap"],
    ["move", "value", "later"],
    ["borrowed", "live", "long"],
];
const NOISE_VOCAB: [&str; 10] = [
    "let", "tmp", "calc", "ctx", "res", "item", "idx", "buf", "ptr", "val",
];

fn shape_text(rng: &mut Rng, regime: usize, code: &str) -> String {
    let ci = CODES.iter().position(|c| *c == code).unwrap_or(3);
    let words: Vec<&str> = vec![
        rng.pick(&REGIME_VOCAB[regime]),
        rng.pick(&CODE_VOCAB[ci]),
        rng.pick(&NOISE_VOCAB),
        rng.pick(&REGIME_VOCAB[regime]),
        rng.pick(&CODE_VOCAB[ci]),
        rng.pick(&NOISE_VOCAB),
    ];
    words.join(" ")
}

// ─────────────────────────────────────────────────────────────────────────────
// The frozen world
// ─────────────────────────────────────────────────────────────────────────────

struct Site {
    code: &'static str,
    line: usize,
    shape: String,
    cascade_u: f32,
}

struct FileCase {
    path: PathBuf,
    regime: usize,
    hard: bool,
    sites: Vec<Site>,
}

/// The frozen (regime, difficulty) emission table for a file — the oracle's
/// outcome draws are made against it, keyed by (site, strategy, context).
fn file_table(file: &FileCase) -> [[f32; 3]; N_STRATS] {
    outcome_table(file.regime, file.hard)
}

/// Context tag: the hash of the sorted remaining-site set. The real
/// compiler's verdict on a re-proposed edit depends on the CURRENT file
/// state — once other edits land, the same edit can flip outcome. Keying the
/// draw on this hash preserves the tried-set's core guarantee (same bytes +
/// same context → same outcome) while making context-change retries honest.
fn ctx_hash(remaining: &[usize]) -> u64 {
    let mut h = blake3::Hasher::new();
    for &r in remaining {
        h.update(&r.to_le_bytes());
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&h.finalize().as_bytes()[..8]);
    u64::from_le_bytes(b)
}

/// The scripted outcome for (site, strategy) under the CURRENT context:
/// a deterministic draw from the file's frozen table, keyed by
/// (world_seed, site, strategy, context). Identical context → identical
/// outcome (the paired-world guarantee); changed context → fresh draw
/// (the world responding to edits, as a real compiler would).
fn outcome_for(
    table: &[[f32; 3]; N_STRATS],
    world_seed: u64,
    site: usize,
    strat: usize,
    ctx: u64,
) -> Outcome {
    let mut h = blake3::Hasher::new();
    h.update(&world_seed.to_le_bytes());
    h.update(&site.to_le_bytes());
    h.update(&strat.to_le_bytes());
    h.update(&ctx.to_le_bytes());
    let mut b = [0u8; 8];
    b.copy_from_slice(&h.finalize().as_bytes()[..8]);
    let u = (u64::from_le_bytes(b) >> 40) as f32 / 16_777_216.0;
    if u < table[strat][0] {
        Outcome::Resolved
    } else if u < table[strat][0] + table[strat][2] {
        Outcome::NewErrors
    } else {
        Outcome::NoProgress
    }
}

fn build_world(seed: u64) -> (Vec<FileCase>, u64) {
    let mut rng = Rng::new(seed.wrapping_mul(1_000_003));
    let mut files = Vec::with_capacity(N_FILES);
    for fi in 0..N_FILES {
        let regime = rng.weighted(&[0.25, 0.25, 0.25, 0.25]);
        let hard = rng.f32() < P_HARD;
        let n_sites = 2 + (rng.next_u64() % 3) as usize; // 2..=4
        let mut sites = Vec::with_capacity(n_sites);
        for si in 0..n_sites {
            let ci = rng.weighted(&CODE_DIST[regime]);
            let code = CODES[ci];
            let shape = shape_text(&mut rng, regime, code);
            let cascade_u = rng.f32();
            sites.push(Site {
                code,
                line: (si + 1) * 10,
                shape,
                cascade_u,
            });
        }
        files.push(FileCase {
            path: PathBuf::from(format!("src/f{fi:04}.rs")),
            regime,
            hard,
            sites,
        });
    }
    (files, seed ^ 0x5EED_CAFE_F00D)
}

// ─────────────────────────────────────────────────────────────────────────────
// Store seeding — the scenario axis (aligned / stale / cold)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    Aligned,
    Stale,
    Cold,
}
impl Scenario {
    fn label(self) -> &'static str {
        match self {
            Self::Aligned => "aligned",
            Self::Stale => "stale",
            Self::Cold => "cold",
        }
    }
}

/// The regime swap that makes the stale store lie (the "world changed under
/// the store" adversarial case — e.g. a compiler upgrade flipped which
/// strategies work).
const STALE_SWAP: [usize; K_REGIMES] = [1, 0, 3, 2];

fn open_store(tag: &str) -> TrajStore {
    let root =
        std::env::temp_dir().join(format!("riir_nonergodic_poc_{}_{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (store, bad) = TrajStore::open(&root).expect("temp traj store");
    assert_eq!(bad, 0, "fresh store must replay clean");
    store
}

/// Seed a store with cross-run history. `Aligned`: outcomes from the true
/// tables. `Stale`: same shapes, outcomes from the swapped tables. `Cold`:
/// no rows. Two events per shape (denser counters, as a warmed store would
/// have). Shapes come from the SAME generator — BM25 retrieval finds
/// same-regime neighbors, giving arm B its fair signal.
fn seed_store(scenario: Scenario, seed: u64) -> TrajStore {
    let store = open_store(scenario.label());
    if scenario == Scenario::Cold {
        return store;
    }
    let mut rng = Rng::new(seed.wrapping_mul(2_000_003).wrapping_add(7));
    for _ in 0..400 {
        let regime = rng.weighted(&[0.25, 0.25, 0.25, 0.25]);
        let truth_regime = if scenario == Scenario::Stale {
            STALE_SWAP[regime]
        } else {
            regime
        };
        let table = outcome_table(truth_regime, false);
        let ci = rng.weighted(&CODE_DIST[regime]);
        let code = CODES[ci];
        let strat = rng.pick(menu(code));
        let shape = shape_text(&mut rng, regime, code);
        let fix = STRATS[strat].2;
        for _ in 0..2 {
            let u = rng.f32();
            let success = u < table[strat][0];
            store
                .record(
                    &traj_key(code, STRATS[strat].0),
                    &shape,
                    fix,
                    success,
                    success,
                    "nonergodic-poc-seed",
                )
                .expect("seed record");
        }
    }
    store
}

// ─────────────────────────────────────────────────────────────────────────────
// The regime ComponentModel (K=4 generators, D=2 inner states)
// ─────────────────────────────────────────────────────────────────────────────

/// One regime generator as a `ComponentModel`: token = `strat*3 + outcome`,
/// inner states {tractable, intractable} with sticky transitions.
///
/// `T^(token=(s,o))[i][j] = (1/N_STRATS) · q[i][s][o] · a[i][j]` — the
/// uniform action factor makes every regime's full token-operator family
/// row-stochastic (Σ over all tokens = 1) while cancelling in the posterior,
/// so the filter's evidence is the outcome-conditional-on-action part only.
struct RegimeModel {
    /// `q[state][strat][outcome]` — normalized over outcomes.
    q: [[[f32; 3]; N_STRATS]; D_STATES],
    /// Sticky state transitions (the world's difficulty is fixed per file).
    trans: [[f32; D_STATES]; D_STATES],
}

impl RegimeModel {
    fn new(regime: usize) -> Self {
        let tract = outcome_table(regime, false);
        let intract = outcome_table(regime, true);
        let mut q = [[[0.0f32; 3]; N_STRATS]; D_STATES];
        q[0] = tract;
        q[1] = intract;
        Self {
            q,
            trans: [[0.97, 0.03], [0.03, 0.97]],
        }
    }
    /// Misspec copy: every emission entry perturbed ±20%, renormalized per
    /// (state, strategy) over outcomes with a floor (never exactly 0 — a 0
    /// would permanently exclude the component from the posterior).
    fn perturbed(regime: usize, rng: &mut Rng) -> Self {
        let mut m = Self::new(regime);
        for i in 0..D_STATES {
            for s in 0..N_STRATS {
                let mut tot = 0.0f32;
                for o in 0..3 {
                    m.q[i][s][o] = (m.q[i][s][o] * (0.8 + 0.4 * rng.f32())).max(0.02);
                    tot += m.q[i][s][o];
                }
                for o in 0..3 {
                    m.q[i][s][o] /= tot;
                }
            }
        }
        m
    }
    /// Predicted P(Resolved | try `strat`) given the inner belief — the
    /// ordering score readout (the marginal the driver would act on).
    fn pred_success(&self, eta: &[f32], strat: usize) -> f32 {
        eta[0] * self.q[0][strat][0] + eta[1] * self.q[1][strat][0]
    }
}

impl ComponentModel for RegimeModel {
    fn likelihood(&self, eta: &[f32], token: u8) -> f32 {
        let s = (token / 3) as usize;
        let o = (token % 3) as usize;
        if s >= N_STRATS {
            return 0.0;
        }
        let mut m = 0.0f32;
        for (i, ei) in eta.iter().enumerate().take(D_STATES) {
            m += ei * self.q[i][s][o];
        }
        m / N_STRATS as f32
    }
    fn update_into(&self, eta: &[f32], token: u8, out: &mut [f32]) {
        let s = (token / 3) as usize;
        let o = (token % 3) as usize;
        if s >= N_STRATS {
            out[0] = 1.0;
            out[1] = 0.0;
            return;
        }
        let mut acc = [0.0f32; D_STATES];
        for (j, accj) in acc.iter_mut().enumerate() {
            for (i, ei) in eta.iter().enumerate().take(D_STATES) {
                *accj += ei * self.q[i][s][o] * self.trans[i][j];
            }
        }
        let t = acc[0] + acc[1];
        if t > 0.0 && t.is_finite() {
            for (oj, accj) in out.iter_mut().zip(acc.iter()) {
                *oj = accj / t;
            }
        } else {
            out[0] = 1.0;
            out[1] = 0.0;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The arms
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Arm {
    /// No-posterior: static min-delta.
    MinDelta,
    /// Shipped static shape-conditional BetaPosterior (the real function).
    StaticShape,
    /// Per-run NonergodicFilter, uniform prior, revive-on-contradiction.
    Nonergodic,
    /// Composition: store belief blended toward the filter as λ_t = t/(t+1).
    Composed,
}
impl Arm {
    fn label(self) -> &'static str {
        match self {
            Self::MinDelta => "A min-delta",
            Self::StaticShape => "B static-shape",
            Self::Nonergodic => "C nonergodic",
            Self::Composed => "D composed",
        }
    }
    fn uses_filter(self) -> bool {
        matches!(self, Self::Nonergodic | Self::Composed)
    }
}

/// Per-run stats for one file.
struct RunStats {
    resolved: bool,
    attempts: usize,
    wasted: usize,
    errors_remaining: usize,
    revives: usize,
    revive_helped: usize,
    revive_hurt: usize,
}

#[derive(Clone)]
struct Cand {
    site: usize,
    strat: usize,
    /// True when this candidate is a REVIVED entry (a suppressed candidate
    /// re-entering the set under the contradiction rule).
    revived: bool,
}

struct TriedEntry {
    /// The filter's committed regime argmax at suppression time.
    sup_argmax: usize,
    /// The context hash at suppression time — a retry under the SAME
    /// context is the same bytes against the same file (provably the same
    /// outcome); only a context change makes a retry a new question.
    sup_ctx: u64,
    /// One revive per candidate per run.
    revive_used: bool,
}

/// Run the driver loop (mirrors `driver.rs`: propose → order → apply best →
/// keep|revert, tried-set suppression, max_iters=5) under one arm.
fn run_file(
    file: &FileCase,
    arm: Arm,
    store: &TrajStore,
    models: &[RegimeModel; K_REGIMES],
    world_seed: u64,
) -> RunStats {
    let model_refs: [&dyn ComponentModel; K_REGIMES] =
        [&models[0], &models[1], &models[2], &models[3]];
    let mut filter = NonergodicFilter::<K_REGIMES, D_STATES>::new(model_refs, [0.25; K_REGIMES])
        .with_initial_eta([[1.0 - P_HARD, P_HARD]; K_REGIMES]);

    let mut remaining: Vec<usize> = (0..file.sites.len()).collect();
    let mut ctx = ctx_hash(&remaining);
    let table = file_table(file);
    let mut tried: HashMap<(usize, usize), TriedEntry> = HashMap::new();
    let mut attempts = 0usize;
    let mut wasted = 0usize;
    let (mut revives, mut revive_helped, mut revive_hurt) = (0usize, 0usize, 0usize);
    let mut t_observed = 0usize; // λ schedule input (arm D)

    while attempts < MAX_ITERS && !remaining.is_empty() {
        let cur_argmax = filter.committed().0;
        let gate = filter.revive_gate();
        // Propose: every (remaining site, menu strategy), minus suppressed —
        // with the revive-on-contradiction exception (arms C/D): the
        // suppression's regime commit AND its file context must both have
        // moved before the same edit is worth re-asking.
        let mut cands: Vec<Cand> = Vec::new();
        for &si in &remaining {
            for &strat in menu(file.sites[si].code) {
                match tried.get(&(si, strat)) {
                    None => cands.push(Cand {
                        site: si,
                        strat,
                        revived: false,
                    }),
                    Some(e) => {
                        if arm.uses_filter()
                            && !e.revive_used
                            && e.sup_argmax != cur_argmax
                            && e.sup_ctx != ctx
                            && gate >= REVIVE_GATE
                        {
                            cands.push(Cand {
                                site: si,
                                strat,
                                revived: true,
                            });
                        }
                    }
                }
            }
        }
        if cands.is_empty() {
            break;
        }

        // Order (the arm seam). Scores are precomputed per candidate — the
        // store-backed ones must not re-retrieve per sort comparison.
        match arm {
            Arm::MinDelta => {
                cands.sort_by(|a, b| mindelta_key(file, a).cmp(&mindelta_key(file, b)));
            }
            Arm::StaticShape => {
                let orig = cands.clone();
                let mut fcs: Vec<FixCandidate> =
                    cands.iter().map(|c| to_fix_candidate(file, c)).collect();
                shape_conditional_order(&mut fcs, store);
                // Re-associate: (line, strategy_id) is unique per candidate.
                let mut ordered = Vec::with_capacity(orig.len());
                for fc in &fcs {
                    let si = remaining
                        .iter()
                        .copied()
                        .find(|&si| file.sites[si].line == fc.line)
                        .expect("line maps back to a site");
                    let strat = STRATS
                        .iter()
                        .position(|(id, _, _)| *id == fc.strategy_id)
                        .expect("strategy maps back");
                    let revived = orig
                        .iter()
                        .find(|c| c.site == si && c.strat == strat)
                        .is_some_and(|c| c.revived);
                    ordered.push(Cand {
                        site: si,
                        strat,
                        revived,
                    });
                }
                cands = ordered;
            }
            Arm::Nonergodic => {
                let scored: Vec<(f32, Cand)> = cands
                    .iter()
                    .map(|c| (filter_score(&filter, models, c.strat), c.clone()))
                    .collect();
                cands = sort_by_score_desc(scored, file);
            }
            Arm::Composed => {
                let lambda = t_observed as f32 / (t_observed as f32 + 1.0);
                let scored: Vec<(f32, Cand)> = cands
                    .iter()
                    .map(|c| {
                        let s = composed_score(&filter, models, store, file, c, lambda);
                        (s, c.clone())
                    })
                    .collect();
                cands = sort_by_score_desc(scored, file);
            }
        }

        let best = cands[0].clone();
        attempts += 1;
        let outcome = outcome_for(&table, world_seed, best.site, best.strat, ctx);
        if arm.uses_filter() {
            let oi = OUTCOMES.iter().position(|o| *o == outcome).unwrap();
            filter.tick((best.strat * 3 + oi) as u8);
            t_observed += 1;
        }
        if best.revived {
            revives += 1;
        }
        match outcome {
            Outcome::Resolved => {
                if best.revived {
                    revive_helped += 1;
                }
                remaining.retain(|&s| s != best.site);
                // Cascade: the fix may clear one more (dependent) site —
                // lowest-index remaining site with a frozen cascade draw.
                if let Some(&j) = remaining
                    .iter()
                    .find(|&&j| file.sites[j].cascade_u < CASCADE_P)
                {
                    remaining.retain(|&s| s != j);
                }
                if !remaining.is_empty() {
                    ctx = ctx_hash(&remaining);
                }
                tried.remove(&(best.site, best.strat));
            }
            _ => {
                wasted += 1;
                if best.revived {
                    revive_hurt += 1;
                }
                // Suppress. An occupied entry means this attempt WAS a
                // revive — its one revive right is spent. A fresh entry
                // keeps its revive right for the contradiction rule.
                let (sup, sctx) = (filter.committed().0, ctx);
                match tried.entry((best.site, best.strat)) {
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        let e = e.get_mut();
                        e.revive_used = true;
                        e.sup_argmax = sup;
                        e.sup_ctx = sctx;
                    }
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(TriedEntry {
                            sup_argmax: sup,
                            sup_ctx: sctx,
                            revive_used: false,
                        });
                    }
                }
            }
        }
    }

    RunStats {
        resolved: remaining.is_empty(),
        attempts,
        wasted,
        errors_remaining: remaining.len(),
        revives,
        revive_helped,
        revive_hurt,
    }
}

/// Score-descending sort with the min-delta tiebreak (the same composition
/// `shape_conditional_order` uses: belief desc, then the static keys).
fn sort_by_score_desc(scored: Vec<(f32, Cand)>, file: &FileCase) -> Vec<Cand> {
    let mut scored = scored;
    scored.sort_by(|(sa, a), (sb, b)| {
        sb.total_cmp(sa)
            .then_with(|| mindelta_key(file, a).cmp(&mindelta_key(file, b)))
    });
    scored.into_iter().map(|(_, c)| c).collect()
}

/// Min-delta ordering key, mirroring `driver::order_candidates` within one
/// file: (line, code, total replacement length, strategy_id).
fn mindelta_key(file: &FileCase, c: &Cand) -> (usize, &'static str, usize, &'static str) {
    let (id, code, repl) = STRATS[c.strat];
    (file.sites[c.site].line, code, repl.len(), id)
}

fn to_fix_candidate(file: &FileCase, c: &Cand) -> FixCandidate {
    let (id, code, repl) = STRATS[c.strat];
    FixCandidate {
        strategy_id: id.to_string(),
        code: code.to_string(),
        file: file.path.clone(),
        line: file.sites[c.site].line,
        byte_span: (0, 0),
        replacement: repl.to_string(),
        shape: file.sites[c.site].shape.clone(),
        semantic_note: None,
        diag_digest: None,
        extra_edits: Vec::new(),
    }
}

/// Arm C score: the filter-marginalized predicted resolve probability.
fn filter_score(
    filter: &NonergodicFilter<'_, K_REGIMES, D_STATES>,
    models: &[RegimeModel; K_REGIMES],
    strat: usize,
) -> f32 {
    let mut s = 0.0f32;
    for (n, model) in models.iter().enumerate() {
        s += filter.weights()[n] * model.pred_success(filter.component(n), strat);
    }
    s
}

/// Arm D score: (1-λ)·store belief + λ·filter prediction.
fn composed_score(
    filter: &NonergodicFilter<'_, K_REGIMES, D_STATES>,
    models: &[RegimeModel; K_REGIMES],
    store: &TrajStore,
    file: &FileCase,
    c: &Cand,
    lambda: f32,
) -> f32 {
    let (id, code, _) = STRATS[c.strat];
    let store_belief = shape_conditional_score(store, code, id, &file.sites[c.site].shape);
    (1.0 - lambda) * store_belief + lambda * filter_score(filter, models, c.strat)
}

// ─────────────────────────────────────────────────────────────────────────────
// Aggregation + main
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
struct ArmTotals {
    files: usize,
    resolved: usize,
    attempts: usize,
    wasted: usize,
    errors_left: usize,
    revives: usize,
    revive_helped: usize,
    revive_hurt: usize,
    /// Per-file resolution for McNemar pairing (worlds concatenated).
    resolved_vec: Vec<bool>,
    /// Per-regime breakdown (R0 copy / R1 clone / R2 scope / R3 lifetime).
    resolved_by_regime: [usize; K_REGIMES],
    files_by_regime: [usize; K_REGIMES],
    /// Hard-file breakdown (the inner-state axis).
    resolved_hard: usize,
    files_hard: usize,
}

impl ArmTotals {
    fn add(&mut self, s: &RunStats, regime: usize, hard: bool) {
        self.files += 1;
        self.resolved += s.resolved as usize;
        self.attempts += s.attempts;
        self.wasted += s.wasted;
        self.errors_left += s.errors_remaining;
        self.revives += s.revives;
        self.revive_helped += s.revive_helped;
        self.revive_hurt += s.revive_hurt;
        self.resolved_vec.push(s.resolved);
        self.files_by_regime[regime] += 1;
        self.resolved_by_regime[regime] += s.resolved as usize;
        if hard {
            self.files_hard += 1;
            self.resolved_hard += s.resolved as usize;
        }
    }
    fn resolved_pct(&self) -> f32 {
        if self.files == 0 {
            return 0.0;
        }
        self.resolved as f32 / self.files as f32 * 100.0
    }
    fn wasted_rate(&self) -> f32 {
        if self.attempts == 0 {
            return 0.0;
        }
        self.wasted as f32 / self.attempts as f32 * 100.0
    }
    fn mean_attempts(&self) -> f64 {
        if self.files == 0 {
            return 0.0;
        }
        self.attempts as f64 / self.files as f64
    }
    fn mean_errors_left(&self) -> f64 {
        if self.files == 0 {
            return 0.0;
        }
        self.errors_left as f64 / self.files as f64
    }
}

/// McNemar discordant pairs: (X won while baseline lost, X lost while
/// baseline won) — the paired test that respects the frozen world.
fn mcnemar(x: &[bool], base: &[bool]) -> (usize, usize) {
    let mut wins = 0;
    let mut losses = 0;
    for (x, b) in x.iter().zip(base.iter()) {
        if *x && !*b {
            wins += 1;
        } else if !*x && *b {
            losses += 1;
        }
    }
    (wins, losses)
}

fn main() {
    let misspec = std::env::args().any(|a| a == "--misspec");
    let scenarios = [Scenario::Aligned, Scenario::Stale, Scenario::Cold];
    let arms = [
        Arm::MinDelta,
        Arm::StaticShape,
        Arm::Nonergodic,
        Arm::Composed,
    ];

    println!("# Issue 093 T1 — per-run strategy-posterior PoC");
    println!();
    println!(
        "- world: {N_FILES} files × {N_SEEDS} seeds (paired: identical frozen outcomes per arm) · \
         sites 2..=4 · max_iters {MAX_ITERS} · P(hard file) {P_HARD}"
    );
    println!(
        "- arms: A = min-delta (no posterior) · B = the REAL `shape_conditional_order` over a \
         seeded TrajStore (static within run) · C = `NonergodicFilter<K=4, D=2>` uniform prior \
         (within-run only) · D = (1-λ)·B + λ·C, λ_t = t/(t+1)"
    );
    println!(
        "- revive-on-contradiction (C/D): a suppressed candidate retries when the filter's \
         committed regime flips AND revive_gate ≥ {REVIVE_GATE}; at most once per candidate"
    );
    if misspec {
        println!("- **misspec pass**: filter model tables perturbed ±20% (world unchanged)");
    }
    println!();

    for &scenario in &scenarios {
        if misspec && scenario != Scenario::Aligned {
            continue; // misspec sensitivity measured on the aligned store only
        }
        println!("## scenario: {} store", scenario.label());
        println!();
        println!(
            "| arm | resolved % | wasted-attempt % | mean attempts | mean errors left | revives (helped/hurt) | McNemar vs A (W/L) | vs B (W/L) |"
        );
        println!("|---|---|---|---|---|---|---|---|");

        let mut totals: Vec<ArmTotals> = arms.iter().map(|_| ArmTotals::default()).collect();
        for seed in 1..=N_SEEDS {
            let (world, world_seed) = build_world(seed);
            let store = seed_store(scenario, seed);
            let mut rng = Rng::new(seed.wrapping_mul(3_000_003));
            let models: [RegimeModel; K_REGIMES] = if misspec {
                std::array::from_fn(|k| RegimeModel::perturbed(k, &mut rng))
            } else {
                std::array::from_fn(RegimeModel::new)
            };
            for (ai, arm) in arms.iter().enumerate() {
                for file in &world {
                    let stats = run_file(file, *arm, &store, &models, world_seed);
                    totals[ai].add(&stats, file.regime, file.hard);
                }
            }
            let _ = std::fs::remove_dir_all(&store.root); // temp-store hygiene
        }
        let a_vec = &totals[0].resolved_vec;
        let b_vec = &totals[1].resolved_vec;
        for (ai, arm) in arms.iter().enumerate() {
            let t = &totals[ai];
            let (wa, la) = mcnemar(&t.resolved_vec, a_vec);
            let (wb, lb) = mcnemar(&t.resolved_vec, b_vec);
            println!(
                "| {} | {:.1} | {:.1} | {:.2} | {:.2} | {} ({}/{}) | {}/{} | {}/{} |",
                arm.label(),
                t.resolved_pct(),
                t.wasted_rate(),
                t.mean_attempts(),
                t.mean_errors_left(),
                t.revives,
                t.revive_helped,
                t.revive_hurt,
                wa,
                la,
                wb,
                lb,
            );
        }
        println!();
        // Per-regime + hard-file diagnostic (which world structure drives
        // the deltas — the defend-wrong transparency read).
        println!(
            "  per-regime resolved % (R0 copy / R1 clone / R2 scope / R3 lifetime) + hard files:"
        );
        for (ai, arm) in arms.iter().enumerate() {
            let t = &totals[ai];
            let pct = |r: usize| {
                if t.files_by_regime[r] == 0 {
                    0.0
                } else {
                    t.resolved_by_regime[r] as f32 / t.files_by_regime[r] as f32 * 100.0
                }
            };
            let hard_pct = if t.files_hard == 0 {
                0.0
            } else {
                t.resolved_hard as f32 / t.files_hard as f32 * 100.0
            };
            println!(
                "    {}: {:.0} / {:.0} / {:.0} / {:.0} · hard {:.0} ({}/{})",
                arm.label(),
                pct(0),
                pct(1),
                pct(2),
                pct(3),
                hard_pct,
                t.resolved_hard,
                t.files_hard,
            );
        }
        println!();
    }
}
