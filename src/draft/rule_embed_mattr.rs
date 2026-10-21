//! Issue 133 T1 — Matryoshka-k randomized-budget supervision for the
//! two-tower span↔rule scorer (`rule_embed`).
//!
//! The shipped trainer (`super::rule_embed::train`) optimizes a FIXED
//! objective: every (span, rule) pair is a positive with in-batch negatives,
//! at one implicit operating point. The `MAttr` recipe (`arXiv:2609.25518`,
//! katgpt-rs Research 584 / Issue 879) randomizes the budget k during
//! training so ONE run yields a ranking calibrated at every slate size.
//!
//! Here the randomized budget takes the `exact_mass_admit` shape:
//! `katgpt_core::exact_mass_admit` (Σσ = k by bisection) produces the soft
//! admission mask over a span's candidate-rule scores at the drawn k, and
//! that mask WEIGHTS a per-span structured BCE (labels from the oracle lint
//! arrays). The mask is detached (stop-gradient through the bisection
//! threshold) — it decides WHICH candidates are supervised at a given step,
//! never the label direction.
//!
//! Arms (all behind `mattr_k_poc`, which implies `rule_embed_poc`):
//! - [`KBudget::Full`] — k = pool every step: the un-randomized spanwise
//!   full-BCE control (diagnostic: isolates the budget-randomization axis
//!   from the loss-shape axis; the SHIPPED pair-structured control stays in
//!   `super::rule_embed::train`).
//! - [`KBudget::Uniform`] — per-step uniform draw over a budget set, e.g.
//!   {1,2,3,5,8}; keep-first (top-k) supervision only.
//! - [`KBudget::LogBoth`] — per-step draw P(k) ∝ 1/k over the set PLUS the
//!   exclude-last side: the bottom (pool − k) candidates carry DEMOTE
//!   pressure (pushed toward exclusion regardless of label — the
//!   corpus-lifecycle shape; the complement-mask-with-labels form was
//!   measured to degenerate to [`KBudget::Full`], see the loss comment).
//!   Measured for the record only — never wired into `active_set`.
//!
//! The withheld-rule discipline carries over unchanged: a gold referencing
//! a withheld rule is REFUSED, and a span's other-split golds
//! (`SpanRow::forbidden`) are excluded from its supervised pool in BOTH
//! directions (never a positive, never a pushed-down negative) — a withheld
//! rule's only gradient surface is its public ψ row, exactly like the
//! shipped trainer.
//!
//! Everything else is the shipped two-tower law: shared hashed-bag
//! featurization (`sparse_features`), τ = 0.1 shared between training and
//! scoring, flat `AdamW` on the warmup+cosine schedule, absmax-to-i8 (×127)
//! artifact through the same wire (`encode_artifact`) so `run_split_eval`
//! and `score_rule` consume control and arms byte-identically.

// Clippy clean: the trainer is a hand-rolled kernel (the `rule_embed` allow
// set, kept identical so both trainers read the same).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::needless_range_loop
)]

use std::collections::HashMap;

use katgpt_core::exact_mass_admit::exact_mass_admit_into;

use super::choice_scorer::{AdamW, Rng};
use super::rule_embed::{
    decode_verified, dot, encode_artifact, fixture_is_withheld, matvec, quantize, scatter_grad,
    sigmoid, sparse_features, FlatAdamW, TrainConfig, Trained, EMBED_DIM, FEATURE_BUCKETS,
    SIGMOID_TAU,
};

/// One supervised span: text, THIS split's gold rules, and the other split's
/// true-positive rules (excluded from supervision entirely — see the module
/// header).
#[derive(Debug, Clone, Default)]
pub struct SpanRow {
    pub text: String,
    /// Gold rule indices into [`SpanTrainData::rule_texts`] — must all be
    /// NON-withheld (the trainer refuses otherwise, mirroring `train`).
    pub golds: Vec<usize>,
    /// True-positive rules of the OTHER split — withheld rules this span
    /// genuinely exhibits. Never supervised in either direction.
    pub forbidden: Vec<usize>,
}

/// Span-structured supervision: spans with gold sets + the rule book.
#[derive(Debug, Clone, Default)]
pub struct SpanTrainData {
    pub spans: Vec<SpanRow>,
    /// `(rule_id, buggy+fixed text)` — EVERY rule that needs a ψ row,
    /// including withheld ones (ψ comes from a rule's own public text).
    pub rule_texts: Vec<(String, String)>,
}

/// The budget schedule. Draws are PER STEP (one k shared by every span in
/// the step — with 3000 steps each budget in the set is visited hundreds of
/// times; per-span draws add variance without changing the mechanism).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KBudget {
    /// k = pool every step — the un-randomized spanwise full-BCE control.
    /// Isolates the randomization axis from the loss-shape axis.
    Full,
    /// Per-step uniform draw over the given budgets; keep-first (top-k)
    /// supervision only. The issue's arm (b).
    Uniform(&'static [u32]),
    /// Per-step draw P(k) ∝ 1/k over the set (log-weighted toward small
    /// budgets) + keep-first AND exclude-last masks. The issue's arm (c);
    /// measured for the record, never wired into `active_set`.
    LogBoth(&'static [u32]),
}

/// Trainer knobs: the shipped [`TrainConfig`] schedule + the budget arm and
/// the admission temperature (the σ width of the mass mask over the ranking
/// scores z = φᵀψ/τ; 1.0 ≈ soft top-k with a ±3-z boundary — pinned a
/// priori, not swept on any eval split).
#[derive(Debug, Clone, Copy)]
pub struct MattrConfig {
    pub base: TrainConfig,
    pub budget: KBudget,
    pub admit_temp: f32,
}

impl Default for MattrConfig {
    fn default() -> Self {
        Self {
            base: TrainConfig::default(),
            budget: KBudget::Full,
            admit_temp: 1.0,
        }
    }
}

/// Per-span precomputed pool: the supervised candidate rule rows (all rules
/// minus the span's forbidden entries) + a gold membership row.
struct SpanPool {
    gold_flags: Vec<bool>,
    pool: Vec<u32>,
}

impl SpanPool {
    fn build(
        span: &SpanRow,
        n_rules: usize,
        rule_id_of: &dyn Fn(usize) -> String,
    ) -> Result<Self, String> {
        if span.golds.is_empty() {
            return Err("a span carries no golds — drop it or label it".into());
        }
        let mut gold_flags = vec![false; n_rules];
        for &g in &span.golds {
            if g >= n_rules {
                return Err(format!("gold index {g} out of range ({n_rules} rules)"));
            }
            let id = rule_id_of(g);
            if fixture_is_withheld(&id) {
                return Err(format!(
                    "gold references withheld rule {id:?} — filter the train split first"
                ));
            }
            gold_flags[g] = true;
        }
        let mut forbidden = span.forbidden.clone();
        forbidden.sort_unstable();
        forbidden.dedup();
        let mut pool = Vec::with_capacity(n_rules);
        let mut next = 0_u32;
        for &f in &forbidden {
            if f >= n_rules {
                return Err(format!(
                    "forbidden index {f} out of range ({n_rules} rules)"
                ));
            }
            let id = rule_id_of(f);
            if !fixture_is_withheld(&id) {
                return Err(format!(
                    "forbidden entry references NON-withheld rule {id:?} — a builder bug \
                     silently dropping real supervision"
                ));
            }
            pool.extend(next..f as u32);
            next = f as u32 + 1;
        }
        pool.extend(next..n_rules as u32);
        Ok(Self { gold_flags, pool })
    }
}

/// Train the two-tower scorer under a (possibly randomized) mass budget.
///
/// # Errors
///
/// Empty inputs, duplicate rule ids, a gold referencing a withheld rule, a
/// forbidden entry referencing a non-withheld rule (a builder bug — it would
/// silently drop real supervision), a span with no golds, or an index out of
/// range. Also fails if the produced artifact fails its own decode check.
pub fn train_spanwise(data: &SpanTrainData, cfg: &MattrConfig) -> Result<Trained, String> {
    if data.spans.is_empty() {
        return Err("SpanTrainData.spans is empty — nothing to train on".into());
    }
    if data.rule_texts.is_empty() {
        return Err("SpanTrainData.rule_texts is empty — no ψ rows to ship".into());
    }
    let n_rules = data.rule_texts.len();
    let mut rule_row: HashMap<&str, usize> = HashMap::with_capacity(n_rules);
    for (i, (id, _)) in data.rule_texts.iter().enumerate() {
        if rule_row.insert(id.as_str(), i).is_some() {
            return Err(format!("duplicate rule id {id:?} in rule_texts"));
        }
    }
    let rule_id_of = |i: usize| data.rule_texts[i].0.clone();
    let mut pools: Vec<SpanPool> = Vec::with_capacity(data.spans.len());
    for span in &data.spans {
        pools.push(SpanPool::build(span, n_rules, &rule_id_of)?);
    }

    // Featurize once (build-time; the step loop allocates nothing).
    let mut acc: Vec<f32> = Vec::with_capacity(FEATURE_BUCKETS);
    let ys: Vec<(Vec<u32>, Vec<f32>)> = data
        .rule_texts
        .iter()
        .map(|(_, text)| sparse_features(text, &mut acc))
        .collect();
    let xs: Vec<(Vec<u32>, Vec<f32>)> = data
        .spans
        .iter()
        .map(|s| sparse_features(&s.text, &mut acc))
        .collect();

    let mut rng = Rng::new(cfg.base.seed);
    let mut w_span: Vec<f32> = (0..FEATURE_BUCKETS * EMBED_DIM)
        .map(|_| rng.normal() * 0.05)
        .collect();
    let mut w_rule: Vec<f32> = (0..FEATURE_BUCKETS * EMBED_DIM)
        .map(|_| rng.normal() * 0.05)
        .collect();
    let mut ws = SpanWorkspace::new(xs.len(), ys.len());
    let mut opt_span = FlatAdamW::new(w_span.len());
    let mut opt_rule = FlatAdamW::new(w_rule.len());
    let mut final_loss = 0.0_f32;
    for step in 0..cfg.base.steps {
        let lr = AdamW::lr_at(
            cfg.base.lr,
            step as u64,
            cfg.base.steps as u64,
            cfg.base.warmup as u64,
            cfg.base.min_lr_ratio,
        );
        let (k, both_sides) = match cfg.budget {
            KBudget::Full => (0_u32, false),
            KBudget::Uniform(_) => (draw_k(&mut rng, cfg.budget), false),
            KBudget::LogBoth(_) => (draw_k(&mut rng, cfg.budget), true),
        };
        final_loss = loss_and_grad_spanwise(
            &w_span,
            &w_rule,
            &xs,
            &ys,
            &pools,
            k,
            both_sides,
            cfg.admit_temp,
            &mut ws,
        );
        opt_span.step(&mut w_span, &ws.g_span, lr, cfg.base.weight_decay);
        opt_rule.step(&mut w_rule, &ws.g_rule, lr, cfg.base.weight_decay);
    }

    // ψ rows for EVERY rule through the trained rule tower — the shipped
    // finalize path, byte-for-byte the same wire.
    let mut row_buf = [0.0_f32; EMBED_DIM];
    let mut psi_rows: Vec<(String, f32, Vec<i8>)> = Vec::with_capacity(ys.len());
    for ((id, _), y) in data.rule_texts.iter().zip(ys.iter()) {
        matvec(&w_rule, &y.0, &y.1, &mut row_buf);
        let (q, scale) = quantize(&row_buf);
        psi_rows.push((id.clone(), scale, q));
    }
    psi_rows.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let (span_i8, span_scale) = quantize(&w_span);
    let bytes = encode_artifact(
        FEATURE_BUCKETS,
        EMBED_DIM,
        SIGMOID_TAU,
        &span_i8,
        span_scale,
        &psi_rows,
    );
    let tables = decode_verified(&bytes).map_err(|e| format!("self-check failed: {e}"))?;
    Ok(Trained {
        tables,
        bytes,
        final_loss,
        n_pairs: xs.len(),
        n_rules: ys.len(),
        steps: cfg.base.steps,
    })
}

/// One budget draw for the step.
fn draw_k(rng: &mut Rng, budget: KBudget) -> u32 {
    match budget {
        KBudget::Full => 0, // sentinel — every span uses k = pool
        KBudget::Uniform(set) => set[rng.uniform_range(0, set.len())],
        KBudget::LogBoth(set) => {
            // P(k) ∝ 1/k over the set, resolved on a 0..1000 integer grid —
            // deterministic, no float-accumulation ambiguity.
            let inv: Vec<f64> = set.iter().map(|&k| 1.0 / f64::from(k)).collect();
            let total: f64 = inv.iter().sum();
            let mut draw =
                f64::from(u32::try_from(rng.uniform_range(0, 1000)).unwrap_or(0)) / 1000.0;
            for (i, &w) in inv.iter().enumerate() {
                draw -= w / total;
                if draw <= 0.0 {
                    return set[i];
                }
            }
            set[set.len() - 1]
        }
    }
}

/// Span-structured masked-BCE forward + backward. Returns the mean per-span
/// loss; leaves analytic grads in `ws.g_span` / `ws.g_rule`.
///
/// Per span: the pool's scores `z_i = φ_s·ψ_i/τ` get the mass mask
/// `m⁺ = exact_mass_admit(z, k, T)` (Σm⁺ = k; k = pool when `k_draw == 0`)
/// and, under `both_sides`, the bottom mask `m⁻ = exact_mass_admit(−z,
/// pool−k, T)`. The keep side takes label BCE weighted by m⁺; the exclude
/// side (`both_sides`) adds label-blind DEMOTE pressure `−log(1−σ(z))`
/// weighted by m⁻ — the two masks are exact complements, so weighting the
/// label BCE by both would degenerate to the full BCE (measured:
/// byte-identical artifact to `Full`, caught 2026-10-08). The mask is
/// detached — it gates WHICH candidates are responsible, never the label.
// The kernel mirrors the shipped `loss_and_grad` signature shape; the two
// budget parameters (draw + both-sides) ride along rather than a config
// struct so the hot loop reads flat.
#[allow(clippy::too_many_arguments)]
fn loss_and_grad_spanwise(
    w_span: &[f32],
    w_rule: &[f32],
    xs: &[(Vec<u32>, Vec<f32>)],
    ys: &[(Vec<u32>, Vec<f32>)],
    pools: &[SpanPool],
    k_draw: u32,
    both_sides: bool,
    admit_temp: f32,
    ws: &mut SpanWorkspace,
) -> f32 {
    let d = EMBED_DIM;
    ws.g_span.fill(0.0);
    ws.g_rule.fill(0.0);
    ws.gphi.fill(0.0);
    ws.gpsi.fill(0.0);
    for (i, x) in xs.iter().enumerate() {
        matvec(w_span, &x.0, &x.1, &mut ws.phi[i * d..(i + 1) * d]);
    }
    for (i, y) in ys.iter().enumerate() {
        matvec(w_rule, &y.0, &y.1, &mut ws.psi[i * d..(i + 1) * d]);
    }
    let mut loss = 0.0_f32;
    for (s, pool) in pools.iter().enumerate() {
        let n = pool.pool.len();
        let phi_s = &ws.phi[s * d..(s + 1) * d];
        let z = &mut ws.z[..n];
        for (i, &r) in pool.pool.iter().enumerate() {
            z[i] = dot(phi_s, &ws.psi[r as usize * d..(r as usize + 1) * d]) / SIGMOID_TAU;
        }
        // Budgets: the sentinel draw (k = 0) means Full — k = pool, which is
        // the admit's own k ≥ n all-ones branch; a drawn k clamps to the pool.
        let kf = if k_draw == 0 {
            n as f32
        } else {
            (k_draw as f32).min(n as f32)
        };
        let m_plus = &mut ws.m_plus[..n];
        exact_mass_admit_into(z, kf, admit_temp, m_plus);
        let m_minus = if both_sides {
            for (zn, &v) in ws.z_neg[..n].iter_mut().zip(z.iter()) {
                *zn = -v;
            }
            let m = &mut ws.m_minus[..n];
            exact_mass_admit_into(&ws.z_neg[..n], (n as f32 - kf).max(0.0), admit_temp, m);
            Some(&ws.m_minus[..n])
        } else {
            None
        };
        let inv = 1.0 / n as f32;
        let gphi_s = &mut ws.gphi[s * d..(s + 1) * d];
        for (i, &r) in pool.pool.iter().enumerate() {
            let r = r as usize;
            let y = if pool.gold_flags[r] { 1.0_f32 } else { 0.0 };
            let pz = sigmoid(z[i]);
            // Keep-first: label BCE weighted by the admission mask.
            // Exclude-last (LogBoth only): DEMOTE pressure — the excluded
            // rules are pushed toward exclusion regardless of label (the
            // corpus-lifecycle shape; Research 584: "which nodes to keep
            // FIRST and which to EXCLUDE LAST", the Issue-102 demotion
            // question). Weighting label BCE by the complement mask instead
            // is a measured NO-OP: the two admits are exact complements
            // (σ(−x) = 1 − σ(x), same bisection), the weights sum to 1 per
            // element, and the arm degenerates to `Full` — caught by the
            // byte-identical artifact, 2026-10-08.
            let (m_keep, m_demote) = (m_plus[i], m_minus.map_or(0.0, |m| m[i]));
            let wgt = m_keep + m_demote;
            if wgt <= f32::EPSILON {
                continue;
            }
            let bce = -(y * pz.max(1e-7).ln() + (1.0 - y) * (1.0 - pz).max(1e-7).ln());
            let demote = -(1.0 - pz).max(1e-7).ln();
            loss += (m_keep * bce + m_demote * demote) * inv;
            // ∂/∂z: keep side (σ(z) − y); demote side σ(z) (always y = 0).
            let dz = (m_keep * (pz - y) + m_demote * pz) * inv / SIGMOID_TAU;
            let psi_r = &ws.psi[r * d..(r + 1) * d];
            for b in 0..d {
                gphi_s[b] += dz * psi_r[b];
                ws.gpsi[r * d + b] += dz * phi_s[b];
            }
        }
    }
    for (i, x) in xs.iter().enumerate() {
        scatter_grad(&mut ws.g_span, &ws.gphi[i * d..(i + 1) * d], &x.0, &x.1);
    }
    for (i, y) in ys.iter().enumerate() {
        scatter_grad(&mut ws.g_rule, &ws.gpsi[i * d..(i + 1) * d], &y.0, &y.1);
    }
    loss / xs.len() as f32
}

/// Preallocated per-step scratch (the `choice_scorer` rule: capacities
/// stabilize, the loop allocates nothing).
struct SpanWorkspace {
    phi: Vec<f32>,  // S × D — span tower outputs
    psi: Vec<f32>,  // R × D — rule tower outputs
    gphi: Vec<f32>, // S × D
    gpsi: Vec<f32>, // R × D
    g_span: Vec<f32>,
    g_rule: Vec<f32>,
    z: Vec<f32>,       // R — per-span pool scores (scratch)
    z_neg: Vec<f32>,   // R — negated scores (LogBoth scratch)
    m_plus: Vec<f32>,  // R — keep-first mask
    m_minus: Vec<f32>, // R — exclude-last mask
}

impl SpanWorkspace {
    fn new(n_spans: usize, n_rules: usize) -> Self {
        Self {
            phi: vec![0.0; n_spans * EMBED_DIM],
            psi: vec![0.0; n_rules * EMBED_DIM],
            gphi: vec![0.0; n_spans * EMBED_DIM],
            gpsi: vec![0.0; n_rules * EMBED_DIM],
            g_span: vec![0.0; FEATURE_BUCKETS * EMBED_DIM],
            g_rule: vec![0.0; FEATURE_BUCKETS * EMBED_DIM],
            z: vec![0.0; n_rules],
            z_neg: vec![0.0; n_rules],
            m_plus: vec![0.0; n_rules],
            m_minus: vec![0.0; n_rules],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic tiny rule set: enough non-withheld names for golds and
    /// at least one withheld name for the forbidden/refusal arms.
    fn tiny_rules(n: usize) -> Vec<(String, String)> {
        let mut rules = Vec::new();
        let mut i = 0_u32;
        while rules.len() < n {
            let name = format!("zz_mattr_probe_{i:04}");
            rules.push((name, format!("fn probe_{i}() {{ let x = {i}; }}")));
            i += 1;
        }
        rules
    }

    fn find_withheld(rules: &[(String, String)]) -> Option<usize> {
        rules.iter().position(|(id, _)| fixture_is_withheld(id))
    }

    #[test]
    fn refuses_withheld_gold_and_nonwithheld_forbidden() {
        let rules = tiny_rules(8);
        let Some(w) = find_withheld(&rules) else {
            panic!("fixture set must contain a withheld name");
        };
        // A withheld gold is refused.
        let bad = SpanTrainData {
            spans: vec![SpanRow {
                text: "let x = 1;".into(),
                golds: vec![w],
                forbidden: vec![],
            }],
            rule_texts: rules.clone(),
        };
        assert!(train_spanwise(&bad, &MattrConfig::default())
            .err()
            .unwrap_or_default()
            .contains("withheld"));
        // A forbidden NON-withheld rule is refused (builder bug).
        let nw = (0..8)
            .find(|&i| i != w && !fixture_is_withheld(&rules[i].0))
            .unwrap();
        let gold = (0..8)
            .find(|&i| i != w && i != nw && !fixture_is_withheld(&rules[i].0))
            .unwrap();
        let bad2 = SpanTrainData {
            spans: vec![SpanRow {
                text: "let x = 1;".into(),
                golds: vec![gold],
                forbidden: vec![nw],
            }],
            rule_texts: rules,
        };
        assert!(train_spanwise(&bad2, &MattrConfig::default())
            .err()
            .unwrap_or_default()
            .contains("NON-withheld"));
    }

    #[test]
    fn full_budget_trains_and_decodes() {
        // The book INCLUDES withheld rules (they need ψ rows); golds are the
        // non-withheld rows, the withheld ones ride as `forbidden`.
        let rules = tiny_rules(10);
        let nonw: Vec<usize> = (0..10)
            .filter(|i| !fixture_is_withheld(&rules[*i].0))
            .collect();
        let gold = nonw[0];
        let gold2 = nonw[1 % nonw.len()];
        let forbidden: Vec<usize> = (0..10)
            .filter(|i| fixture_is_withheld(&rules[*i].0))
            .collect();
        let data = SpanTrainData {
            spans: vec![
                SpanRow {
                    text: rules[gold].1.clone(),
                    golds: vec![gold],
                    forbidden: forbidden.clone(),
                },
                SpanRow {
                    text: rules[gold2].1.clone(),
                    golds: vec![gold2],
                    forbidden: vec![],
                },
            ],
            rule_texts: rules,
        };
        let cfg = MattrConfig {
            base: TrainConfig {
                steps: 40,
                ..TrainConfig::default()
            },
            ..MattrConfig::default()
        };
        let tr = train_spanwise(&data, &cfg).expect("full-budget spanwise training");
        assert_eq!(tr.n_rules, data.rule_texts.len());
        // The magnitude floor (the Bench 099 lesson — an ordering assertion
        // does not pin a scale): the trained positive must score > 0.75
        // through the ARTIFACT path.
        let phi = tr.tables.project_span(&data.spans[0].text);
        let gold_id = data.rule_texts[data.spans[0].golds[0]].0.as_str();
        let s = tr.tables.score_rule(&phi, gold_id).expect("gold row");
        assert!(
            s > 0.75,
            "self-pair σ = {s} — the artifact broke the weights"
        );
    }

    #[test]
    fn uniform_budget_trains_and_separates() {
        let rules: Vec<(String, String)> = tiny_rules(10)
            .into_iter()
            .filter(|(id, _)| !fixture_is_withheld(id))
            .collect();
        let data = SpanTrainData {
            spans: (0..4)
                .map(|i| SpanRow {
                    text: rules[i].1.clone(),
                    golds: vec![i],
                    forbidden: vec![],
                })
                .collect(),
            rule_texts: rules,
        };
        let cfg = MattrConfig {
            base: TrainConfig {
                steps: 60,
                ..TrainConfig::default()
            },
            budget: KBudget::Uniform(&[1, 2, 3]),
            ..MattrConfig::default()
        };
        let tr = train_spanwise(&data, &cfg).expect("uniform-budget training");
        let phi = tr.tables.project_span(&data.spans[0].text);
        let s = tr
            .tables
            .score_rule(&phi, &data.rule_texts[0].0)
            .expect("gold row");
        assert!(s > 0.75, "uniform-budget self-pair σ = {s}");
    }

    #[test]
    fn logboth_budget_trains_and_separates() {
        let rules: Vec<(String, String)> = tiny_rules(10)
            .into_iter()
            .filter(|(id, _)| !fixture_is_withheld(id))
            .collect();
        let data = SpanTrainData {
            spans: (0..4)
                .map(|i| SpanRow {
                    text: rules[i].1.clone(),
                    golds: vec![i],
                    forbidden: vec![],
                })
                .collect(),
            rule_texts: rules,
        };
        let cfg = MattrConfig {
            base: TrainConfig {
                steps: 60,
                ..TrainConfig::default()
            },
            budget: KBudget::LogBoth(&[1, 2, 3]),
            ..MattrConfig::default()
        };
        let tr = train_spanwise(&data, &cfg).expect("logboth training");
        let phi = tr.tables.project_span(&data.spans[0].text);
        let s = tr
            .tables
            .score_rule(&phi, &data.rule_texts[0].0)
            .expect("gold row");
        assert!(s > 0.75, "logboth self-pair σ = {s}");
    }

    #[test]
    fn budget_draws_hit_every_value() {
        let set: &'static [u32] = &[1, 2, 3, 5, 8];
        let mut rng = Rng::new(126);
        let mut seen = [false; 9];
        for _ in 0..4000 {
            seen[draw_k(&mut rng, KBudget::Uniform(set)) as usize] = true;
        }
        for &k in set {
            assert!(seen[k as usize], "uniform draws never produced k={k}");
        }
        let mut rng = Rng::new(126);
        let mut seen = [false; 9];
        for _ in 0..4000 {
            seen[draw_k(&mut rng, KBudget::LogBoth(set)) as usize] = true;
        }
        for &k in set {
            assert!(seen[k as usize], "logboth draws never produced k={k}");
        }
    }
}
