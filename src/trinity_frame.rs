//! Issue 243 — the G3-cell evaluation trinity (T1 retention-hit × T2
//! window-mass × T3 flip-identity) computed over one
//! [`AdoptionWindow`].
//!
//! Frame provenance: arXiv:2611.07624 D.4's frozen-pool admission replay —
//! admission quality requires inspecting preservation, set size and
//! candidate identity together; a size-normalized score alone can rise
//! while the admission discards its best candidates. This module is the
//! per-consult version of that lesson over the adoption gate's windows:
//! every consult gets its verdict shape, the mass of evidence backing it,
//! and its perturbation identity (which single-row flips move it, how much
//! judge noise it survives).
//!
//! REPORT-ONLY census — this computes no gate decision and no posture. The
//! perturbation ensemble reproduces the executed G3-cell instrument
//! (`examples/adoption_g3_cell.rs`, Benches 148–150) exactly: same
//! `XorShift`, same per-(p, arm, seed) seeding formula, same arm filters, so
//! the stability numbers here are directly comparable to the disclosed
//! Bench-150 fractions. T1's reference here is the PROXY arm (perturbation
//! stability ≥ the bar counts as "reliably best") — committed windows
//! carry no forward rows, so the forward-outcome reference is
//! structurally unavailable and the proxy labeling is mandatory
//! disclosure, never a quality claim.

use std::collections::BTreeMap;

use crate::adoption_gate::{AdoptionWindow, GateVerdict, OutcomeRow};

/// The ensemble posture — defaults are the executed Bench-150 posture, so
/// numbers computed at the default are directly comparable to the record.
#[derive(Debug, Clone)]
pub struct TrinityConfig {
    /// The consult posture α (the executed cell's α₀ = 0.5).
    pub alpha0: f64,
    /// Judge-noise arms (per-row Bernoulli flip probability).
    pub noises: Vec<f64>,
    /// Monte-Carlo seeds per (p, arm).
    pub seeds: usize,
    /// The "reliably best" stability bar (T1 proxy hit threshold).
    pub min_stability: f64,
}

impl Default for TrinityConfig {
    fn default() -> Self {
        Self {
            alpha0: 0.5,
            noises: vec![0.1, 0.2],
            seeds: 32,
            min_stability: 0.90,
        }
    }
}

/// One consult identity within a window (kind + tag, sorted order).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConsultId {
    pub kind: &'static str,
    pub tag: String,
}

/// One leave-one-out attribution: flipping this single row moves the
/// consult's verdict.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LooMove {
    pub row_index: usize,
    pub row_tag: String,
    pub row_accepted: bool,
}

/// One consult's full trinity row.
#[derive(Debug, Clone)]
pub struct ConsultRow {
    pub id: ConsultId,
    /// Baseline verdict shape at α₀: "PROCEED" | "REFUSE" | "ABSTAIN".
    pub verdict: &'static str,
    /// Backing mass: the tag's own rows (the demote delta / the readmit
    /// evidence). Ring residency can differ — disclosed in the record, not
    /// modeled here.
    pub mass_rows: usize,
    pub mass_accepted: usize,
    pub mass_rejected: usize,
    /// Carrier classification of the backing mass.
    pub single_row_carrier: bool,
    pub all_rejected_carrier: bool,
    pub accepted_carrier: bool,
    /// Per-consult MINIMUM noise stability across all (p, arm) pairs;
    /// 1.0 when no arm had eligible rows (the executed instrument's
    /// convention — an arm with no eligible rows cannot lower stability).
    pub min_stability: f64,
    /// Single-row flips (leave-one-out) that move this consult's verdict.
    pub loo_moved_by: Vec<LooMove>,
}

/// One window's full frame.
#[derive(Debug, Clone)]
pub struct WindowFrame {
    pub rows: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub tags: usize,
    pub consults: Vec<ConsultRow>,
}

impl WindowFrame {
    /// T1 proxy hits: consults whose verdict carries a winner (not
    /// ABSTAIN) and whose min stability meets the bar.
    #[must_use]
    pub fn t1_hits(&self, cfg: &TrinityConfig) -> usize {
        self.consults
            .iter()
            .filter(|c| c.verdict != "ABSTAIN" && c.min_stability >= cfg.min_stability)
            .count()
    }

    /// T1 proxy misses: consults whose verdict carries a winner but whose
    /// min stability is below the bar (no reliably-best winner).
    #[must_use]
    pub fn t1_misses(&self, cfg: &TrinityConfig) -> usize {
        self.consults
            .iter()
            .filter(|c| c.verdict != "ABSTAIN" && c.min_stability < cfg.min_stability)
            .count()
    }

    /// Consults excluded from T1 (ABSTAIN carries no winner to test).
    #[must_use]
    pub fn t1_abstains(&self) -> usize {
        self.consults
            .iter()
            .filter(|c| c.verdict == "ABSTAIN")
            .count()
    }

    /// T3: consults that flip under any perturbation — fragile under
    /// noise OR moved by at least one single-row flip.
    #[must_use]
    pub fn fragile(&self, cfg: &TrinityConfig) -> Vec<&ConsultRow> {
        self.consults
            .iter()
            .filter(|c| c.min_stability < cfg.min_stability || !c.loo_moved_by.is_empty())
            .collect()
    }
}

fn verdict_shape(v: &GateVerdict) -> &'static str {
    match v {
        GateVerdict::Proceed(_) => "PROCEED",
        GateVerdict::Refuse(_) => "REFUSE",
        GateVerdict::Abstain => "ABSTAIN",
    }
}

fn consult_one(window: &AdoptionWindow, c: &ConsultId, alpha: f64) -> GateVerdict {
    match c.kind {
        "demote" => window.demote_verdict(&c.tag, alpha),
        _ => window.readmit_verdict(&c.tag, alpha),
    }
}

/// Deterministic `XorShift` — the executed instrument's generator, verbatim
/// (identical stream → identical perturbation ensemble → comparable
/// stability numbers).
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

    /// Uniform in [0, 1) — 53 mantissa bits, no modulo bias. The u64→f64
    /// cast is VERBATIM from the executed instrument — identical stream,
    /// never a precision claim (53 bits are exact).
    #[allow(clippy::cast_precision_loss)]
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Flip one row's verdict bit (judge noise: the same output, judged the
/// other way; the digest is untouched).
fn flip(rows: &mut [OutcomeRow], i: usize) {
    rows[i].accepted = !rows[i].accepted;
}

/// Compute the full trinity frame for one window. Deterministic: `BTreeMap`
/// ordering, seeded RNG, stable sort — two calls on the same rows produce
/// identical frames.
///
/// # Errors
/// When the `from_rows` replay self-check diverges from the disk-loaded
/// window (the perturbation harness's premise — replay must consult
/// identically or every number below is evidence of nothing).
pub fn compute_frame(window: &AdoptionWindow, cfg: &TrinityConfig) -> Result<WindowFrame, String> {
    let rows = window.rows().to_vec();
    let accepted = rows.iter().filter(|r| r.accepted).count();
    let rejected = rows.len() - accepted;

    // Consult population: every tag × both kinds, sorted (the executed
    // instrument's stable order).
    let population = population_of(window);

    // Replay self-check (same premise as the executed cell).
    let replay = AdoptionWindow::from_rows(window.rows().to_vec());
    for c in &population {
        if verdict_shape(&consult_one(window, c, cfg.alpha0))
            != verdict_shape(&consult_one(&replay, c, cfg.alpha0))
        {
            return Err(format!(
                "self-check FAILED: from_rows replay diverges on {} {}",
                c.kind, c.tag
            ));
        }
    }

    // Per-tag backing mass (tag rows, both consult kinds).
    let mass = mass_of(&rows);

    // Baselines.
    let baselines: Vec<&'static str> = population
        .iter()
        .map(|c| verdict_shape(&consult_one(window, c, cfg.alpha0)))
        .collect();

    // T3's identity half: which single-row flips move which consult.
    let mut loo_moved = leave_one_out(window, &rows, &population, cfg.alpha0);
    // T1's proxy reference: per-consult minimum judge-noise stability
    // across all (p, arm) pairs.
    let min_agrees = noise_min_agrees(window, &rows, &population, cfg);

    let consults = population
        .into_iter()
        .zip(baselines)
        .enumerate()
        .map(|(k, (id, verdict))| {
            let (acc, rej) = mass.get(id.tag.as_str()).copied().unwrap_or((0, 0));
            // Report-lane scale (counts ≤ seeds, never a precision claim).
            #[allow(clippy::cast_precision_loss)]
            let min_stability = if min_agrees[k] == usize::MAX {
                1.0
            } else {
                min_agrees[k] as f64 / cfg.seeds as f64
            };
            ConsultRow {
                loo_moved_by: loo_moved.remove(&k).unwrap_or_default(),
                id,
                verdict,
                mass_rows: acc + rej,
                mass_accepted: acc,
                mass_rejected: rej,
                single_row_carrier: acc + rej == 1,
                all_rejected_carrier: acc == 0 && rej > 0,
                accepted_carrier: acc > 0,
                min_stability,
            }
        })
        .collect();

    Ok(WindowFrame {
        rows: rows.len(),
        accepted,
        rejected,
        tags: mass.len(),
        consults,
    })
}

/// Every tag × both consult kinds, sorted — the executed instrument's
/// stable population order.
fn population_of(window: &AdoptionWindow) -> Vec<ConsultId> {
    let mut out: Vec<ConsultId> = Vec::new();
    for tag in window.tags() {
        out.push(ConsultId {
            kind: "demote",
            tag: tag.to_string(),
        });
        out.push(ConsultId {
            kind: "readmit",
            tag: tag.to_string(),
        });
    }
    out.sort();
    out.shrink_to_fit();
    out
}

/// Per-tag backing mass: (accepted, rejected) row counts.
fn mass_of(rows: &[OutcomeRow]) -> BTreeMap<&str, (usize, usize)> {
    let mut mass: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for r in rows {
        let e = mass.entry(r.tag.as_str()).or_insert((0, 0));
        if r.accepted {
            e.0 += 1;
        } else {
            e.1 += 1;
        }
    }
    mass
}

/// Leave-one-out: flip each row once, record which consults move
/// (consult index → the moves that hit it).
fn leave_one_out(
    window: &AdoptionWindow,
    rows: &[OutcomeRow],
    population: &[ConsultId],
    alpha0: f64,
) -> BTreeMap<usize, Vec<LooMove>> {
    let mut loo_moved: BTreeMap<usize, Vec<LooMove>> = BTreeMap::new();
    for (i, row) in rows.iter().enumerate() {
        let mut perturbed = rows.to_vec();
        flip(&mut perturbed, i);
        let w = AdoptionWindow::from_rows(perturbed);
        for (k, c) in population.iter().enumerate() {
            let base = verdict_shape(&consult_one(window, c, alpha0));
            let now = verdict_shape(&consult_one(&w, c, alpha0));
            if base != now {
                loo_moved.entry(k).or_default().push(LooMove {
                    row_index: i,
                    row_tag: row.tag.clone(),
                    row_accepted: row.accepted,
                });
            }
        }
    }
    loo_moved
}

/// Judge-noise Monte-Carlo — the executed instrument's exact procedure
/// (same seeding formula, same arm filters, same eligible-row rule);
/// returns per-consult MINIMUM agreement counts across all (p, arm)
/// pairs (`usize::MAX` where no arm had eligible rows).
fn noise_min_agrees(
    window: &AdoptionWindow,
    rows: &[OutcomeRow],
    population: &[ConsultId],
    cfg: &TrinityConfig,
) -> Vec<usize> {
    let mut min_agrees: Vec<usize> = vec![usize::MAX; population.len()];
    for &p in &cfg.noises {
        for arm_filter in [true, false] {
            let eligible: Vec<usize> = rows
                .iter()
                .enumerate()
                .filter(|(_, r)| r.accepted == arm_filter)
                .map(|(i, _)| i)
                .collect();
            if eligible.is_empty() {
                continue;
            }
            let mut agrees = vec![0usize; population.len()];
            for seed in 1..=cfg.seeds {
                // The executed instrument's seeding formula, verbatim
                // (`u64::from(bool)` ≡ the `as u64` it wrote — same 0/1).
                let mut rng = XorShift(
                    0x9E37_79B9_7F4A_7C15u64
                        .wrapping_mul(seed as u64)
                        .wrapping_add((p.to_bits() ^ u64::from(arm_filter)) | 1),
                );
                let mut perturbed = rows.to_vec();
                for &i in &eligible {
                    if rng.unit() < p {
                        flip(&mut perturbed, i);
                    }
                }
                let w = AdoptionWindow::from_rows(perturbed);
                for (k, c) in population.iter().enumerate() {
                    if verdict_shape(&consult_one(window, c, cfg.alpha0))
                        == verdict_shape(&consult_one(&w, c, cfg.alpha0))
                    {
                        agrees[k] += 1;
                    }
                }
            }
            for (k, a) in agrees.iter().enumerate() {
                min_agrees[k] = min_agrees[k].min(*a);
            }
        }
    }
    min_agrees
}
