//! Issue 062 T3 — ALS fit of the healer corpus into role-filler form, with
//! **admissibility measured before structure**.
//!
//! Preregistered in `.benchmarks/063_tpr_structure_admissibility.md`.
//!
//! ## The precondition T4 assumed
//!
//! Issue 062 T4 proposes scoring retrieval candidates against a TPR
//! **projection** of the span state. A TPR fit reconstructs the observed state
//! `e ∈ R^D` from a core `c ∈ R^{m·d}` through a fitted `W ∈ R^{D×(m·d)}`.
//! When `m·d ≳ D` that map is under-determined: it reproduces every training
//! state, the residual collapses toward zero, and the fit's apparent quality
//! carries no information about generalization. Reading an OOD number off such
//! a fit is the "verify the instrument first" failure.
//!
//! So [`admissibility`] runs first and is **two-sided**:
//!
//! - **over-parameterization** — `params < n_states · D` (scalar observations);
//! - **per-role starvation** — every role block needs `≥ d + 1` states, because
//!   a `D×d` readout fitted from `≤ d` state vectors is locally
//!   under-determined even when the global count passes.
//!
//! An arm failing either bound is INADMISSIBLE and its residual / OOD numbers
//! are evidence for nothing. A one-sided check passes happily on a vacuous fit,
//! which is exactly the reading this module exists to refuse.
//!
//! ## Axes: role = shape-class, filler = rule identity
//!
//! Inverted relative to Bench 062's Phase 1 naming, and the inversion is
//! forced. [`TprScheme::Orthogonal`] has `n_bind_slots == arity`, so the role
//! count IS the core block count: role = rule identity means `m = 45` and
//! `K ≥ 45` against a `D = 8` consumer state — inadmissible before `d` is
//! chosen — and `kernel_opt`'s 426 rules are not expressible at all. Filler
//! count is unconstrained, so rule identity belongs on the filler axis. The
//! (role, filler) *pair* is unchanged; only which side is a block moves.
//!
//! ## Two state spaces
//!
//! [`StateSpace::Latent8`] is the **consumer** space (`riir_rag::LATENT_DIM`) —
//! the only arm whose result can feed T4. [`StateSpace::TokenBag256`] is a
//! **control**: an inadmissible `latent8` result cannot distinguish "TPR does
//! not help this corpus" from "the 8-D state is too narrow to fit anything
//! structured", and those have opposite consequences. It is deliberately not a
//! T4 candidate — the reranker does not score in 256-D.
//!
//! The crate-local [`crate::embedder`] is unusable here at any width: it mixes
//! the lint name into the vector, which would leak the role into the state.
//!
//! Modelless throughout: BLAKE3 + DFT + closed-form ridge-ALS. No gradients.

use std::collections::{BTreeMap, BTreeSet};

use katgpt_core::tpr::als::param_count;
use katgpt_core::tpr::{
    AlsConfig, AlsInput, AlsReport, BicSelection, BowRouterReport, ShuffledRoleReport, TprArtifact,
    TprBindings, TprError, TprScheme, als_fit, bic_select, bow_router, shuffled_role_control,
};
use riir_rag::{Embedder, ModellessEmbedder, query::LATENT_DIM};

use super::withheld_pair_ood::{Binding, normalized_tokens};

/// Bag width of the [`StateSpace::TokenBag256`] control.
pub const TOKEN_BAG_DIM: usize = 256;

/// `bow_router` / `shuffled_role_control` tolerance — a ratio within 5% of 1.0
/// is "the roles bought nothing". Fixed here so it is not tuned after the fact.
pub const STRUCTURE_EPS: f32 = 0.05;

/// Deterministic seed for the role-shuffle control.
pub const SHUFFLE_SEED: u64 = 0x0062_5F54_5052_0003;

// ── state spaces ────────────────────────────────────────────────────────

/// Which state vector the fit reconstructs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateSpace {
    /// The shipped 8-D `ModellessEmbedder` direction — the consumer space.
    Latent8,
    /// Signed BLAKE3 hashing of [`normalized_tokens`] into
    /// [`TOKEN_BAG_DIM`] bins, L2-normalized. The admissibility control.
    TokenBag256,
}

impl StateSpace {
    /// Every space this bench runs, in report order.
    pub const ALL: [StateSpace; 2] = [StateSpace::Latent8, StateSpace::TokenBag256];

    #[must_use]
    pub fn dim(self) -> usize {
        match self {
            StateSpace::Latent8 => LATENT_DIM,
            StateSpace::TokenBag256 => TOKEN_BAG_DIM,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            StateSpace::Latent8 => "latent8",
            StateSpace::TokenBag256 => "tokenbag256",
        }
    }

    /// Is this arm the one T4 would consume?
    #[must_use]
    pub fn is_consumer(self) -> bool {
        matches!(self, StateSpace::Latent8)
    }

    /// Write `code`'s state into `out` (length [`StateSpace::dim`]).
    ///
    /// # Panics
    ///
    /// Panics if `out.len() != self.dim()`.
    pub fn embed_into(self, code: &str, out: &mut [f32]) {
        assert_eq!(out.len(), self.dim(), "state buffer width");
        match self {
            StateSpace::Latent8 => out.copy_from_slice(&ModellessEmbedder.embed(code)),
            StateSpace::TokenBag256 => token_bag_into(code, out),
        }
    }
}

/// Signed feature hashing of the normalized token stream.
///
/// Signed rather than unsigned because unsigned counts make every collision a
/// one-directional bias, so the bag's inner products drift upward with token
/// count; the sign bit makes collisions cancel in expectation. L2-normalized so
/// the state energy does not track span length — the fit's residual *fraction*
/// would otherwise be dominated by long spans.
fn token_bag_into(code: &str, out: &mut [f32]) {
    out.fill(0.0);
    for tok in normalized_tokens(code) {
        let h = blake3::hash(tok.as_bytes());
        let b = h.as_bytes();
        let idx = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize % out.len();
        let sign = if b[4] & 1 == 0 { 1.0f32 } else { -1.0f32 };
        out[idx] += sign;
    }
    let norm = out.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 1e-12 {
        for v in out.iter_mut() {
            *v /= norm;
        }
    }
}

// ── corpus ──────────────────────────────────────────────────────────────

/// Role (shape-class) and filler (rule) id vocabularies, both derived from the
/// population so the harness is domain-agnostic — `rustc_errors` ships a
/// different shape-label set than `clippy_lints`.
#[derive(Debug, Clone, Default)]
pub struct Vocab {
    pub roles: Vec<&'static str>,
    pub fillers: Vec<String>,
}

/// The fit's input, materialized once and shared by every `d` in the grid.
#[derive(Debug, Clone)]
pub struct FitCorpus {
    pub space: StateSpace,
    /// `n_states × dim` row-major.
    pub states: Vec<f32>,
    pub bindings: Vec<TprBindings>,
    pub vocab: Vocab,
    /// States per role id, parallel to `vocab.roles`.
    pub role_states: Vec<usize>,
    /// States per filler id, parallel to `vocab.fillers`.
    pub filler_states: Vec<usize>,
}

impl FitCorpus {
    #[must_use]
    pub fn n_states(&self) -> usize {
        self.bindings.len()
    }

    #[must_use]
    pub fn dim(&self) -> usize {
        self.space.dim()
    }

    #[must_use]
    pub fn input(&self) -> AlsInput<'_> {
        AlsInput {
            dim: self.dim(),
            n_fillers: self.vocab.fillers.len(),
            states: &self.states,
            bindings: &self.bindings,
        }
    }

    #[must_use]
    pub fn scheme(&self) -> TprScheme {
        TprScheme::Orthogonal {
            arity: self.vocab.roles.len(),
        }
    }
}

/// Build the fit corpus from a Bench 062 population.
///
/// Deduplicates by text hash for the same reason Bench 062's protocol does:
/// two identical texts are one observation, and counting them twice inflates
/// both the observation count the admissibility bound is measured against and
/// the per-role census.
///
/// # Panics
///
/// Panics if the population carries more than `u16::MAX` distinct shape
/// classes or rules — [`TprBindings`] ids are `u16`, so a larger vocabulary is
/// not representable and silently truncating it would mis-bind every id past
/// the wrap.
#[must_use]
pub fn build_corpus(population: Vec<Binding>, space: StateSpace) -> FitCorpus {
    let mut seen = BTreeSet::new();
    let pop: Vec<Binding> = population
        .into_iter()
        .filter(|b| seen.insert(*blake3::hash(b.text.as_bytes()).as_bytes()))
        .collect();

    let roles: Vec<&'static str> = pop
        .iter()
        .map(|b| b.shape)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let fillers: Vec<String> = pop
        .iter()
        .map(|b| b.rule.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let role_id: BTreeMap<&str, u16> = roles
        .iter()
        .enumerate()
        .map(|(i, r)| (*r, u16::try_from(i).expect("role ids fit u16")))
        .collect();
    let filler_id: BTreeMap<&str, u16> = fillers
        .iter()
        .enumerate()
        .map(|(i, r)| (r.as_str(), u16::try_from(i).expect("filler ids fit u16")))
        .collect();

    let dim = space.dim();
    let mut states = vec![0.0f32; pop.len() * dim];
    let mut bindings = Vec::with_capacity(pop.len());
    let mut role_states = vec![0usize; roles.len()];
    let mut filler_states = vec![0usize; fillers.len()];
    for (i, b) in pop.iter().enumerate() {
        space.embed_into(&b.text, &mut states[i * dim..(i + 1) * dim]);
        let p = role_id[b.shape];
        let v = filler_id[b.rule.as_str()];
        role_states[p as usize] += 1;
        filler_states[v as usize] += 1;
        bindings.push(TprBindings::from_pairs(&[(p, v)]));
    }

    FitCorpus {
        space,
        states,
        bindings,
        vocab: Vocab { roles, fillers },
        role_states,
        filler_states,
    }
}

// ── role coalescing ─────────────────────────────────────────────────────

/// What [`coalesce_for`] did, so a coarser vocabulary is never read as the
/// original one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoalesceReport {
    /// States-per-role cutoff at or below which a role loses its own block.
    pub threshold: usize,
    /// Roles merged into the synthetic `other` block.
    pub merged: Vec<(&'static str, usize)>,
    /// States discarded because even `other` could not be fitted.
    pub dropped: usize,
    /// Role count after coalescing (`m'`).
    pub m_after: usize,
    /// Nothing merged and nothing dropped — the coalesced corpus IS the
    /// original, so its numbers must match the uncoalesced arm exactly.
    pub identity: bool,
}

/// Synthetic block for roles too sparse to carry a readout.
pub const OTHER_ROLE: &str = "other";

/// Coalesce roles with `≤ threshold` states into one `other` block, dropping
/// its states if it is still unfittable, and remap to contiguous ids.
///
/// Preregistered in Bench 063 §10.2, amended by §10.2a: the threshold is
/// **fixed** at the grid maximum rather than set per `d`. Per-`d` coalescing
/// changes `n_states`, and BIC's `N·ln(RSS/N) + p·ln N` is not comparable
/// across different `N` — so a per-`d` rule cannot answer "which `d` does BIC
/// prefer", which is the question coalescing was added to make answerable.
///
/// A role with `≤ threshold` states has a `D×d` readout and too few
/// independent states to fit it, so the honest options are "no block of its
/// own" then "no states". Both are taken, in that order, and the second is
/// counted — a silent drop would be the defect this reports its way out of.
///
/// Contiguity is not cosmetic: [`TprScheme::Orthogonal`] uses the role id AS
/// the core block index, so a gap would leave a permanently-zero block still
/// costing `D·d` parameters against the admissibility bound.
///
/// # Panics
///
/// Panics if the surviving role count exceeds `u16::MAX` — see
/// [`build_corpus`]; coalescing only shrinks it, so this cannot fire on a
/// corpus that built successfully.
#[must_use]
pub fn coalesce_for(c: &FitCorpus, threshold: usize) -> (FitCorpus, CoalesceReport) {
    let merged: Vec<(&'static str, usize)> = c
        .vocab
        .roles
        .iter()
        .zip(c.role_states.iter())
        .filter(|(_, &n)| n <= threshold)
        .map(|(&r, &n)| (r, n))
        .collect();

    if merged.is_empty() {
        return (
            c.clone(),
            CoalesceReport {
                threshold,
                merged,
                dropped: 0,
                m_after: c.vocab.roles.len(),
                identity: true,
            },
        );
    }

    let merged_set: BTreeSet<&'static str> = merged.iter().map(|&(r, _)| r).collect();
    let other_states: usize = merged.iter().map(|&(_, n)| n).sum();
    // `other` needs MORE than `threshold` states for the same reason every
    // other role does — merging does not exempt the merged block from the
    // bound it was merged to satisfy.
    let keep_other = other_states > threshold;

    let survivors: Vec<&'static str> = c
        .vocab
        .roles
        .iter()
        .copied()
        .filter(|r| !merged_set.contains(r))
        .chain(keep_other.then_some(OTHER_ROLE))
        .collect();
    let new_id: BTreeMap<&'static str, u16> = survivors
        .iter()
        .enumerate()
        .map(|(i, &r)| (r, u16::try_from(i).expect("coalesced role ids fit u16")))
        .collect();

    let dim = c.dim();
    let mut states = Vec::with_capacity(c.states.len());
    let mut bindings = Vec::with_capacity(c.bindings.len());
    let mut role_states = vec![0usize; survivors.len()];
    let mut filler_states = vec![0usize; c.vocab.fillers.len()];
    let mut dropped = 0usize;
    for (i, b) in c.bindings.iter().enumerate() {
        let old = c.vocab.roles[b.roles[0] as usize];
        let target = if merged_set.contains(old) {
            if !keep_other {
                dropped += 1;
                continue;
            }
            OTHER_ROLE
        } else {
            old
        };
        let p = new_id[target];
        let v = b.fillers[0];
        states.extend_from_slice(&c.states[i * dim..(i + 1) * dim]);
        role_states[p as usize] += 1;
        filler_states[v as usize] += 1;
        bindings.push(TprBindings::from_pairs(&[(p, v)]));
    }

    (
        FitCorpus {
            space: c.space,
            states,
            bindings,
            vocab: Vocab {
                roles: survivors.clone(),
                fillers: c.vocab.fillers.clone(),
            },
            role_states,
            filler_states,
        },
        CoalesceReport {
            threshold,
            merged,
            dropped,
            m_after: survivors.len(),
            identity: false,
        },
    )
}

// ── admissibility ───────────────────────────────────────────────────────

/// Two-sided admissibility of a `(corpus, d)` pair — computed **before** any
/// fit runs, from geometry alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admissibility {
    pub d: usize,
    /// Role blocks (`m`).
    pub m: usize,
    /// Core width (`m·d`).
    pub k: usize,
    pub dim: usize,
    pub n_states: usize,
    pub n_fillers: usize,
    /// `dim·k + dim + n_fillers·d` (Orthogonal contributes no role params).
    pub params: usize,
    /// `n_states · dim` — scalar observations, matching `bic_select`'s `N`.
    pub observations: usize,
    /// Roles with `≤ d` states, i.e. a locally under-determined readout.
    pub starved_roles: Vec<(&'static str, usize)>,
    /// `k >= dim`: the `W` block alone can reproduce any state.
    pub core_at_least_state_width: bool,
}

impl Admissibility {
    #[must_use]
    pub fn over_parameterized(&self) -> bool {
        self.params >= self.observations
    }

    #[must_use]
    pub fn starved(&self) -> bool {
        !self.starved_roles.is_empty()
    }

    #[must_use]
    pub fn admissible(&self) -> bool {
        !self.over_parameterized() && !self.starved() && !self.core_at_least_state_width
    }

    /// Human-readable reason, or `"admissible"`.
    #[must_use]
    pub fn why(&self) -> String {
        let mut why = Vec::new();
        if self.core_at_least_state_width {
            why.push(format!(
                "core K={} >= state dim D={} (W reproduces any state)",
                self.k, self.dim
            ));
        }
        if self.over_parameterized() {
            why.push(format!(
                "params {} >= observations {}",
                self.params, self.observations
            ));
        }
        if self.starved() {
            let list: Vec<String> = self
                .starved_roles
                .iter()
                .map(|(r, n)| format!("{r}={n}"))
                .collect();
            why.push(format!(
                "{} role(s) with <= d={} states: {}",
                self.starved_roles.len(),
                self.d,
                list.join(", ")
            ));
        }
        if why.is_empty() {
            "admissible".to_string()
        } else {
            why.join("; ")
        }
    }
}

/// Geometry-only admissibility — no fit, no data-dependent quantity.
#[must_use]
pub fn admissibility(c: &FitCorpus, d: usize) -> Admissibility {
    let m = c.vocab.roles.len();
    let dim = c.dim();
    let k = m * d;
    let n_fillers = c.vocab.fillers.len();
    let starved_roles: Vec<(&'static str, usize)> = c
        .vocab
        .roles
        .iter()
        .zip(c.role_states.iter())
        .filter(|(_, &n)| n <= d)
        .map(|(&r, &n)| (r, n))
        .collect();
    Admissibility {
        d,
        m,
        k,
        dim,
        n_states: c.n_states(),
        n_fillers,
        params: dim * k + dim + n_fillers * d,
        observations: c.n_states() * dim,
        starved_roles,
        core_at_least_state_width: k >= dim,
    }
}

// ── the fit ─────────────────────────────────────────────────────────────

/// One `d` of the grid: its admissibility verdict and, when the fit ran, the
/// certificate bundle.
#[derive(Debug, Clone)]
pub struct GridPoint {
    pub adm: Admissibility,
    /// Present only on the coalesced arm.
    pub coalesce: Option<CoalesceReport>,
    /// `None` when the fit itself errored (recorded, never swallowed).
    pub report: Option<AlsReport>,
    pub error: Option<String>,
    /// `param_count` of the shipped artifact — cross-checks
    /// [`Admissibility::params`] against the upstream formula rather than
    /// trusting this module's arithmetic.
    pub params_shipped: Option<usize>,
    pub commitment: Option<[u8; 32]>,
    /// A second, independent fit produced byte-identical bytes (upstream G1,
    /// re-checked at the consumer).
    pub double_fit_identical: Option<bool>,
}

impl GridPoint {
    /// Residual energy fraction, or `None` if the fit did not run.
    #[must_use]
    pub fn residual(&self) -> Option<f32> {
        self.report.as_ref().map(|r| r.residual_energy_fraction)
    }
}

fn config(c: &FitCorpus, d: usize) -> AlsConfig {
    AlsConfig::new(d, c.scheme())
}

/// Fit at `d`, returning the frozen artifact alongside the grid point.
///
/// The fit runs **even when inadmissible** — an inadmissible fit's residual is
/// the evidence FOR the inadmissibility reading (a vacuous fit reproduces its
/// training states), so refusing to run it would hide the confirmation. What
/// the caller must not do is read it as a quality signal; [`GridPoint::adm`]
/// travels with it for exactly that reason.
#[must_use]
pub fn fit_at(c: &FitCorpus, d: usize) -> (Option<TprArtifact>, GridPoint) {
    let adm = admissibility(c, d);
    let cfg = config(c, d);
    match als_fit(c.input(), &cfg) {
        Err(e) => (
            None,
            GridPoint {
                adm,
                coalesce: None,
                report: None,
                error: Some(e.to_string()),
                params_shipped: None,
                commitment: None,
                double_fit_identical: None,
            },
        ),
        Ok((art, report)) => {
            let again = als_fit(c.input(), &cfg).ok().map(|(a, _)| a.to_bytes());
            let identical = again.map(|b| b == art.to_bytes());
            let point = GridPoint {
                adm,
                coalesce: None,
                report: Some(report),
                error: None,
                params_shipped: Some(param_count(&art)),
                commitment: Some(art.commitment()),
                double_fit_identical: identical,
            };
            (Some(art), point)
        }
    }
}

// ── the cross-state role control ────────────────────────────────────────

/// Result of [`cross_state_role_control`] — and the vacuity verdict for the
/// upstream per-state control it replaces.
#[derive(Debug, Clone, PartialEq)]
pub struct CrossStateRoleReport {
    pub r_true: f32,
    pub r_permuted: f32,
    pub ratio: f32,
    /// `ratio > 1 + eps`: breaking the (role, filler) pairing across states
    /// destroyed real structure.
    pub degraded: bool,
    /// The permutation actually moved at least one state's role — asserted
    /// rather than assumed, because a control that cannot move is the failure
    /// mode this whole type exists to route around.
    pub moved: usize,
}

/// Is the upstream [`shuffled_role_control`] vacuous on this corpus?
///
/// It permutes each state's role list **within that state**
/// (`for i in (1..n).rev()`), which is an empty loop at `n == 1`. Every state
/// here carries exactly one binding, so the permutation is a provable identity
/// and the report comes back `degraded = false` with `ratio == 1.0` — which is
/// **indistinguishable from "roles carry nothing"**. That is the same
/// vacuous-control signature Bench 062 §caveat-1 found in the atomic null, one
/// level up, and it must be detected rather than read. See the
/// `shuffled_role_control` source: `for i in (1..n).rev()`.
#[must_use]
pub fn per_state_shuffle_is_vacuous(bindings: &[TprBindings]) -> bool {
    bindings.iter().all(|b| b.len() <= 1)
}

/// The control a single-binding corpus actually needs: permute the role
/// assignment **across** states, leaving each state's filler alone, and refit.
///
/// This breaks the (role, filler) pairing globally — which is what "are the
/// roles load-bearing?" means when no state has two bindings to swap. The
/// permutation is a deterministic Fisher-Yates over `SplitMix64` (same generator
/// shape as upstream) so the control is reproducible, and it is re-drawn until
/// it moves at least one role, so an identity draw cannot masquerade as a
/// no-structure verdict.
///
/// # Errors
///
/// Propagates any [`TprError`] from either of the two fits (same geometry as
/// [`fit_at`], so a failure here means the true fit would have failed too).
///
/// # Panics
///
/// Panics if a Fisher-Yates index does not fit `usize` — impossible for a
/// corpus this size, asserted rather than silently truncated because a wrapped
/// index would quietly bias the permutation the control depends on.
pub fn cross_state_role_control(
    c: &FitCorpus,
    d: usize,
    seed: u64,
    eps: f32,
) -> Result<CrossStateRoleReport, TprError> {
    let cfg = config(c, d);
    let (_, true_rep) = als_fit(c.input(), &cfg)?;

    let mut roles: Vec<u16> = c.bindings.iter().flat_map(|b| b.roles.clone()).collect();
    let original = roles.clone();
    let mut state = seed | 1;
    let mut next = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    for i in (1..roles.len()).rev() {
        let j = usize::try_from(next() % (i as u64 + 1)).expect("index fits usize");
        roles.swap(i, j);
    }
    let moved = original
        .iter()
        .zip(roles.iter())
        .filter(|(a, b)| a != b)
        .count();

    let mut cursor = 0usize;
    let permuted: Vec<TprBindings> = c
        .bindings
        .iter()
        .map(|b| {
            let n = b.len();
            let slice = roles[cursor..cursor + n].to_vec();
            cursor += n;
            TprBindings {
                roles: slice,
                fillers: b.fillers.clone(),
            }
        })
        .collect();
    let perm_input = AlsInput {
        dim: c.dim(),
        n_fillers: c.vocab.fillers.len(),
        states: &c.states,
        bindings: &permuted,
    };
    let (_, perm_rep) = als_fit(perm_input, &cfg)?;

    let r_true = true_rep.residual_energy_fraction;
    let r_permuted = perm_rep.residual_energy_fraction;
    let ratio = if r_true > 1e-9 {
        r_permuted / r_true
    } else if r_permuted > 1e-9 {
        f32::MAX
    } else {
        1.0
    };
    Ok(CrossStateRoleReport {
        r_true,
        r_permuted,
        ratio,
        degraded: ratio > 1.0 + eps,
        moved,
    })
}

// ── the run ─────────────────────────────────────────────────────────────

/// Whether the structure probes ran, and if not, why not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeStatus {
    Ran,
    /// Every `d` in the grid was inadmissible — a structure verdict here would
    /// be read off a vacuous fit.
    SkippedInadmissible,
    SkippedFitError(String),
}

/// One (population, state space) arm.
#[derive(Debug, Clone)]
pub struct SpaceReport {
    pub domain: &'static str,
    /// `None` = the roles are the classifier's own; `Some` = roles below the
    /// threshold were coalesced, so every verdict here is a verdict over a
    /// COARSER vocabulary (Bench 063 §10.4).
    pub coalesce: Option<CoalesceReport>,
    pub space: StateSpace,
    pub n_states: usize,
    pub n_roles: usize,
    pub n_fillers: usize,
    /// Sparsest / densest role census, for the starvation reading.
    pub role_census: Vec<(&'static str, usize)>,
    pub ds: Vec<usize>,
    pub grid: Vec<GridPoint>,
    /// BIC over the admissible `d`s only — scoring inadmissible candidates
    /// would let a vacuous fit win on residual.
    pub bic: Option<BicSelection>,
    pub bic_ds: Vec<usize>,
    /// The `d` the structure probes ran at (BIC winner among admissible).
    pub chosen_d: Option<usize>,
    pub bow: Option<BowRouterReport>,
    /// The upstream per-state control. **Vacuous on this corpus** — see
    /// [`per_state_shuffle_is_vacuous`]; kept only so the vacuity is visible
    /// in the record instead of being silently read as a no-structure verdict.
    pub shuffled: Option<ShuffledRoleReport>,
    pub shuffled_vacuous: bool,
    /// The control that actually applies here.
    pub cross_state: Option<CrossStateRoleReport>,
    pub probe_status: ProbeStatus,
}

impl SpaceReport {
    /// The `d`s that passed both admissibility bounds.
    #[must_use]
    pub fn admissible_ds(&self) -> Vec<usize> {
        self.grid
            .iter()
            .filter(|g| g.adm.admissible())
            .map(|g| g.adm.d)
            .collect()
    }

    #[must_use]
    pub fn any_admissible(&self) -> bool {
        self.grid.iter().any(|g| g.adm.admissible())
    }

    /// Preregistered verdict: is this arm structured, on an admissible fit?
    ///
    /// Requires BOTH the `BoW` router (do roles buy anything over a bag of
    /// fillers?) and the cross-state control (is the SPECIFIC pairing
    /// load-bearing?). A router verdict alone would credit any residual drop
    /// that extra per-block parameters can buy, structure or not.
    #[must_use]
    pub fn structured(&self) -> Option<bool> {
        match (&self.bow, &self.cross_state) {
            (Some(b), Some(x)) => Some(b.structured && x.degraded),
            _ => None,
        }
    }
}

/// Run the grid + structure probes for one arm.
///
/// # Panics
///
/// Panics if `ds` is empty.
#[must_use]
pub fn run(
    population: Vec<Binding>,
    space: StateSpace,
    domain: &'static str,
    ds: &[usize],
    coalesce_threshold: Option<usize>,
) -> SpaceReport {
    assert!(!ds.is_empty(), "grid must not be empty");
    let base = build_corpus(population, space);
    let (corpus, coalesce) = match coalesce_threshold {
        None => (base, None),
        Some(t) => {
            let (c, rep) = coalesce_for(&base, t);
            (c, Some(rep))
        }
    };
    let grid: Vec<GridPoint> = ds
        .iter()
        .map(|&d| {
            let mut g = fit_at(&corpus, d).1;
            g.coalesce.clone_from(&coalesce);
            g
        })
        .collect();

    let mut role_census: Vec<(&'static str, usize)> = corpus
        .vocab
        .roles
        .iter()
        .copied()
        .zip(corpus.role_states.iter().copied())
        .collect();
    role_census.sort_by_key(|&(_, n)| n);

    let bic_ds: Vec<usize> = grid
        .iter()
        .filter(|g| g.adm.admissible() && g.report.is_some())
        .map(|g| g.adm.d)
        .collect();
    let cfgs: Vec<AlsConfig> = bic_ds.iter().map(|&d| config(&corpus, d)).collect();
    let bic = if cfgs.is_empty() {
        None
    } else {
        bic_select(corpus.input(), &cfgs).ok()
    };
    let chosen_d = bic.as_ref().map(|b| bic_ds[b.best]);

    let (bow, shuffled, cross_state, probe_status) = match chosen_d {
        None => (None, None, None, ProbeStatus::SkippedInadmissible),
        Some(d) => {
            let cfg = config(&corpus, d);
            match (
                bow_router(corpus.input(), &cfg, STRUCTURE_EPS),
                shuffled_role_control(corpus.input(), &cfg, SHUFFLE_SEED, STRUCTURE_EPS),
                cross_state_role_control(&corpus, d, SHUFFLE_SEED, STRUCTURE_EPS),
            ) {
                (Ok(b), Ok(s), Ok(x)) => (Some(b), Some(s), Some(x), ProbeStatus::Ran),
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => (
                    None,
                    None,
                    None,
                    ProbeStatus::SkippedFitError(TprError::to_string(&e)),
                ),
            }
        }
    };

    SpaceReport {
        domain,
        coalesce,
        space,
        n_states: corpus.n_states(),
        n_roles: corpus.vocab.roles.len(),
        n_fillers: corpus.vocab.fillers.len(),
        role_census,
        ds: ds.to_vec(),
        grid,
        bic,
        bic_ds,
        chosen_d,
        bow,
        shuffled,
        shuffled_vacuous: per_state_shuffle_is_vacuous(&corpus.bindings),
        cross_state,
        probe_status,
    }
}

/// The `--nocapture` table.
///
/// # Panics
///
/// Panics if the upstream `param_count` disagrees with this module's analytic
/// parameter bound — the admissibility verdict is computed from that bound
/// before any fit runs, so a divergence means the verdict was decided by the
/// wrong arithmetic and every row above it is void.
pub fn print_space_report(r: &SpaceReport) {
    println!(
        "\n── {} / {}{} ──  n_states={} m(roles)={} n_fillers={} D={}",
        r.domain,
        r.space.label(),
        r.coalesce
            .as_ref()
            .map_or_else(String::new, |c| format!(" [coalesced T={}]", c.threshold)),
        r.n_states,
        r.n_roles,
        r.n_fillers,
        r.space.dim()
    );
    if let Some(c) = &r.coalesce {
        println!(
            "  coalesced: merged {:?} -> `{OTHER_ROLE}`  m'={} dropped={} identity={}",
            c.merged, c.m_after, c.dropped, c.identity
        );
    }
    println!(
        "  role census (sparsest first): {}",
        r.role_census
            .iter()
            .map(|(s, n)| format!("{s}={n}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    print_grid(r);
    print_probes(r);
}

fn print_grid(r: &SpaceReport) {
    println!(
        "  {:>3}  {:>5} {:>7} {:>7}  {:>9} {:>6}  verdict",
        "d", "K", "params", "obs", "resid", "ident"
    );
    for g in &r.grid {
        let a = &g.adm;
        println!(
            "  {:>3}  {:>5} {:>7} {:>7}  {:>9} {:>6}  {} — {}",
            a.d,
            a.k,
            a.params,
            a.observations,
            g.residual()
                .map_or_else(|| "—".to_string(), |v| format!("{v:.3e}")),
            g.double_fit_identical
                .map_or("—", |b| if b { "yes" } else { "NO" }),
            if a.admissible() {
                "ADMISSIBLE"
            } else {
                "INADMISSIBLE"
            },
            a.why()
        );
        if let Some(p) = g.params_shipped {
            assert_eq!(
                p, a.params,
                "param_count disagrees with the analytic bound at d={}",
                a.d
            );
        }
        if let Some(e) = &g.error {
            println!("       fit error: {e}");
        }
    }
}

fn print_probes(r: &SpaceReport) {
    match (&r.probe_status, r.chosen_d) {
        (ProbeStatus::Ran, Some(d)) => {
            println!("  BIC over admissible d {:?} → d={d}", r.bic_ds);
            if let Some(b) = &r.bic {
                println!(
                    "    scores: {}",
                    b.scores
                        .iter()
                        .map(|s| format!("{s:.1}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                println!("    label: {}", b.label);
            }
            if let Some(b) = &r.bow {
                println!(
                    "  bow_router: r_bow={:.4e} r_full={:.4e} ratio={:.4} structured={}",
                    b.r_bow, b.r_full, b.ratio, b.structured
                );
            }
            if let Some(s) = &r.shuffled {
                println!(
                    "  shuffled_role (upstream): r_true={:.4e} r_shuffled={:.4e} ratio={:.4} degraded={}{}",
                    s.r_true,
                    s.r_shuffled,
                    s.ratio,
                    s.degraded,
                    if r.shuffled_vacuous {
                        "   [VACUOUS — 1 binding/state, the per-state permutation is the identity]"
                    } else {
                        ""
                    }
                );
            }
            if let Some(x) = &r.cross_state {
                println!(
                    "  `cross_state_role`: r_true={:.4e} r_permuted={:.4e} ratio={:.4} degraded={} moved={}/{}",
                    x.r_true, x.r_permuted, x.ratio, x.degraded, x.moved, r.n_states
                );
            }
            println!(
                "  STRUCTURED (router AND cross-state): {:?}",
                r.structured()
            );
        }
        (status, _) => println!("  structure probes NOT RUN: {status:?}"),
    }
}
