//! Issue 093 T2 — the per-run strategy-posterior ordering at the
//! [`CompileDriver`](super::driver::CompileDriver) ordering seam.
//!
//! The T1 `PoC` ([Bench 084](../../../.benchmarks/084_nonergodic_driver_poc.md),
//! `examples/nonergodic_driver_poc.rs`) measured the per-run
//! `NonergodicFilter` (katgpt-rs Plan 592) beating both the static min-delta
//! ordering and the shipped shape-conditional `BetaPosterior` ordering on a
//! toy multi-error world (resolved 41.7% vs 35.2/36.7, `McNemar` at `p=1e-4`).
//! This module is the production-shaped wiring of that winner: the T4.2
//! ordering seam, the Issue-91-T1.5 attempt ring (consumed in-process via
//! the driver observer), and the context-scoped revive policy — composed
//! behind the opt-in `nonergodic_driver` feature.
//!
//! ## Measured verdict (Issue 093 closed 2026-09-13)
//!
//! The `PoC` win did not transfer to production surfaces: no gain on any
//! real corpus ([Bench 088](../../../.benchmarks/088_nonergodic_ordering_headroom_predicate.md)
//! — K=1 unmeasurable, chunked/strided parity, shared-handle regression)
//! and on the interacting corpus every learned ordering REGRESSED
//! ([Bench 089](../../../.benchmarks/089_interacting_error_corpus.md))
//! until the keep SIGNAL was fixed — Issue 100's lexicographic keep rule
//! repaired 10/34 to 34/46 parity for ALL arms
//! ([Bench 090](../../../.benchmarks/090_e0614_unmasking_trade.md)).
//! Static min-delta stays shipped; this module stays opt-in, out of the
//! release set. Reopen triggers: Research 166 §B.
//!
//! ## The component family (the honest port)
//!
//! The `PoC`'s components were the world's own regime generators
//! (well-specified by construction — its recorded bound). Real rustc
//! "regimes" are latent and unnamed, so this port models a **rank-rotation
//! family**: the menu's strategies are ranked once per run (store
//! shape-conditional score when a store is present, incumbent min-delta
//! rank otherwise), and component `k` asserts "the strategy ranked `k` is
//! the hot one" — success probability `P_HOT · ρ^|rank − k|` decaying by
//! rank distance. The filter's job within the run is to identify which
//! rotation explains the observed strategy/outcome tokens. This is a
//! *weaker, honest* family: it captures "which strategy wins here" (the
//! ordering-relevant signal) without claiming the `PoC`'s per-regime semantic
//! structure. The two-state inner chain (tractable/messy, sticky 0.97)
//! keeps the `PoC`'s difficulty model.
//!
//! ## Determinism
//!
//! Every input is deterministic for a given (candidates, attempts) history:
//! the menu is sorted, scores are `f32` compared by `total_cmp`, exact ties
//! fall through to the incumbent min-delta key, and the context hash is
//! blake3 over the sorted per-file diagnostic set. Two runs over the same
//! repo state order identically.
//!
//! ## The bounded leak
//!
//! `NonergodicFilter` borrows its component models (`&'a dyn
//! ComponentModel`) while both the filter and the tables must live in the
//! shared policy state across closures — a self-referential shape. The
//! models are therefore leaked to `'static` at construction: **K=4 models,
//! a few hundred bytes, once per driver run** (a CLI process performs
//! exactly one run; the A/B harness performs a bounded number). Documented,
//! bounded, never grows with iterations.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use katgpt_micro_belief::nonergodic::{ComponentModel, NonergodicFilter};

use super::driver::{span_hash, AttemptOutcome, AttemptRecord, ObserverFn, OrderingFn, ReviveFn};
use super::playbooks::FixCandidate;
use crate::traj_store::{shape_conditional_score, TrajStore};

/// Components (rank rotations) — the `PoC`'s K.
pub const K_COMPONENTS: usize = 4;
/// Inner states per component: {tractable, messy}.
const D_STATES: usize = 2;
/// Menu cap: strategies beyond this score the neutral prior (the menu is
/// frozen at the first ordering call; real playbook menus are ~10).
const MENU_CAP: usize = 16;
/// Hot-strategy success probability (component `k` at rank distance 0).
const P_HOT: f32 = 0.85;
/// Rank-decay ratio per unit distance from the component's hot rank.
const RANK_DECAY: f32 = 0.55;
/// Success-probability floor (a 0 would permanently exclude a component —
/// the `PoC`'s misspec lesson).
const P_FLOOR: f32 = 0.15;
/// Flat P(new errors) per attempt (the wasted-attempt amplifier; the rank
/// family carries no per-strategy hypothesis for it).
const P_NEW: f32 = 0.10;
/// Messy-state success scale (the `PoC`'s hard-file transform, uniform here —
/// the family's recorded misspec vs the `PoC`'s blunt-strategy rank shift).
const MESSY_SCALE: f32 = 0.35;
/// Sticky self-transition probability for the inner chain.
const STICKY: f32 = 0.97;
/// Initial P(messy) — the `PoC`'s `1 − P_HARD` with `P_HARD = 0.3`.
const INITIAL_MESSY: f32 = 0.30;
/// Revive threshold: `revive_gate()` must clear this AND the committed
/// component AND the file context must both have moved (`PoC` constant).
const REVIVE_GATE: f32 = 0.35;
/// The neutral score for un-menued strategies (the store's own unseen
/// prior, `best_belief_score(0, 0, 0.32)`).
const NEUTRAL: f32 = 0.32;
/// Stale-store gate: once the filter commits a NON-base rotation with
/// majority weight after ≥ this many observations, the store term is
/// dropped (λ forced to 1) — the `PoC`'s "D under a lying store paid 3.5pp
/// vs C" defense.
const STALE_MIN_OBS: usize = 2;

// ─────────────────────────────────────────────────────────────────────────────
// The rank-rotation component model
// ─────────────────────────────────────────────────────────────────────────────

/// One component: "the strategy at rank `hot` is the hot one" over the
/// frozen menu, with a two-state {tractable, messy} sticky inner chain.
/// `q[state][strat] = [P(resolved), P(no progress), P(new errors)]`.
struct RotationModel {
    q: Vec<Vec<[f32; 3]>>,
    trans: [[f32; D_STATES]; D_STATES],
}

impl RotationModel {
    /// Build the component favoring base rank `hot_rank`; `rank_of_menu`
    /// maps each menu position to its base rank (store-seeded or incumbent
    /// — component `k` asserts "the strategy RANKED `k` is the hot one",
    /// with success `P_HOT · RANK_DECAY^|rank − hot_rank|`).
    fn new(hot_rank: usize, rank_of_menu: &[usize]) -> Self {
        let menu_len = rank_of_menu.len();
        let hot_rank = if menu_len == 0 {
            0
        } else {
            hot_rank % menu_len
        };
        let mut q = Vec::with_capacity(D_STATES);
        for &state_scale in &[1.0f32, MESSY_SCALE] {
            let mut row = Vec::with_capacity(menu_len);
            for &rank in rank_of_menu {
                // Rank distance ≤ menu_len ≤ MENU_CAP — lossless in practice.
                #[allow(clippy::cast_precision_loss)]
                let dist = rank.abs_diff(hot_rank) as f32;
                let mut s = (P_HOT * RANK_DECAY.powf(dist)).max(P_FLOOR);
                let mut new_err = P_NEW;
                if state_scale < 1.0 {
                    // The messy state scales success down and new-errors up
                    // (the `PoC`'s hard-file transform), floored/renormalized.
                    s = (s * MESSY_SCALE).max(0.03);
                    new_err = (new_err * 1.5).min(0.5);
                }
                let no = (1.0 - s - new_err).max(0.02);
                let tot = s + new_err + no;
                row.push([s / tot, no / tot, new_err / tot]);
            }
            q.push(row);
        }
        Self {
            q,
            trans: [[STICKY, 1.0 - STICKY], [1.0 - STICKY, STICKY]],
        }
    }
}

impl ComponentModel for RotationModel {
    fn likelihood(&self, eta: &[f32], token: u8) -> f32 {
        let s = usize::from(token / 3);
        let o = usize::from(token % 3);
        let Some(first) = self.q.first() else {
            return 0.0;
        };
        if s >= first.len() {
            return 0.0;
        }
        let mut m = 0.0f32;
        for (i, ei) in eta.iter().enumerate().take(D_STATES) {
            m += ei * self.q[i][s][o];
        }
        // Uniform strategy prior — a constant that cancels in Bayes (the
        // `PoC`'s precedent). menu_len ≤ MENU_CAP — lossless in practice.
        #[allow(clippy::cast_precision_loss)] // bounded by MENU_CAP
        let menu_len_f = first.len() as f32;
        m / menu_len_f
    }

    fn update_into(&self, eta: &[f32], token: u8, out: &mut [f32]) {
        let s = usize::from(token / 3);
        let o = usize::from(token % 3);
        let Some(first) = self.q.first() else {
            out[0] = 1.0;
            out[1] = 0.0;
            return;
        };
        if s >= first.len() {
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
// The shared per-run policy state
// ─────────────────────────────────────────────────────────────────────────────

/// A suppression record — the revive policy's bookkeeping for one
/// `(site, strategy)` that failed (the context the driver's own `tried`
/// set does not keep). Keyed coarser than the driver's `TriedKey`
/// (span+strategy, not span+strategy+replacement): revive asks "may I
/// retry strategy S at this site" — a replacement variant under the same
/// site+strategy is the same strategy decision.
struct Suppression {
    sup_argmax: usize,
    sup_ctx: u64,
    revive_used: bool,
}

type Filter = NonergodicFilter<'static, K_COMPONENTS, D_STATES>;

struct Inner {
    /// Frozen strategy menu (sorted, capped) — set at the first ordering
    /// call; empty until then.
    menu: Vec<String>,
    /// The leaked component models (kept for the emission-table readout the
    /// borrowed filter does not expose).
    models: Vec<&'static RotationModel>,
    /// The per-run filter — built with the menu. `None` until freeze.
    filter: Option<Filter>,
    /// Observed attempts (the λ-schedule input).
    t_observed: usize,
    /// Suppression bookkeeping keyed by `(span_hash, strategy_id)`.
    suppressions: HashMap<(String, String), Suppression>,
    /// Per-file context hash from the last ordering pass.
    ctx: HashMap<PathBuf, u64>,
    /// `span_hash` → file, refreshed every ordering pass — the ring record
    /// carries only the hash, the suppression bookkeeping needs the file
    /// (the ctx lookup key).
    span_file: HashMap<String, PathBuf>,
}

impl Inner {
    fn menu_index(&self, strategy_id: &str) -> Option<usize> {
        self.menu.iter().position(|s| s == strategy_id)
    }

    /// The filter's marginal P(resolved) for menu strategy `mi` — the
    /// ordering readout (the `PoC`'s `pred_success`).
    fn filter_score(&self, mi: usize) -> f32 {
        let Some(f) = self.filter.as_ref() else {
            return NEUTRAL;
        };
        let mut score = 0.0f32;
        for (k, model) in self.models.iter().enumerate() {
            let eta = f.component(k);
            let m = &model.q;
            score += f.weights()[k] * (eta[0] * m[0][mi][0] + eta[1] * m[1][mi][0]);
        }
        score
    }

    /// The stale-store gate: the store term drops out once the filter
    /// commits a non-base rotation with majority weight.
    fn store_stale(&self) -> bool {
        match self.filter.as_ref() {
            Some(f) if self.t_observed >= STALE_MIN_OBS => {
                let (argmax, w) = f.committed();
                argmax != 0 && w > 0.5
            }
            _ => false,
        }
    }

    /// blake3 context hash for one file's candidate set — the diagnostic
    /// set the file presents THIS iteration (the `PoC`'s remaining-site hash).
    fn ctx_of(cands_of_file: &[(String, usize, String)]) -> u64 {
        let mut h = blake3::Hasher::new();
        for (code, line, strat) in cands_of_file {
            h.update(code.as_bytes());
            h.update(&line.to_le_bytes());
            h.update(strat.as_bytes());
        }
        let mut b = [0u8; 8];
        b.copy_from_slice(&h.finalize().as_bytes()[..8]);
        u64::from_le_bytes(b)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The public policy
// ─────────────────────────────────────────────────────────────────────────────

/// The per-run nonergodic ordering policy (Issue 093 T2). Build one per
/// driver run; install its three closures on the SAME `CompileDriver`:
/// [`ordering`](Self::ordering) at the T4.2 seam,
/// [`observer`](Self::observer) for the attempt feed,
/// [`revive`](Self::revive) for the context-scoped retry policy. All three
/// share one state; installing a subset degrades gracefully (no observer →
/// the filter never learns; no revive → suppression stays permanent).
pub struct NonergodicOrdering {
    inner: Arc<Mutex<Inner>>,
    store: Option<Arc<TrajStore>>,
}

/// Whether the env kill-switch left the ordering armed (the feature is the
/// opt-in; `RIIR_REFINE_NONERGODIC=0|false|off` disarms at runtime — the
/// workspace's kill-switch polarity).
#[must_use]
pub fn armed_by_env() -> bool {
    match std::env::var("RIIR_REFINE_NONERGODIC") {
        Ok(v) => !matches!(v.as_str(), "0" | "false" | "off"),
        Err(_) => true,
    }
}

/// The policy's health readout (introspection for tests + run tails).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolicySnapshot {
    /// Frozen menu size (0 until the first ordering call).
    pub menu_len: usize,
    /// Observed strategy-informative attempts so far.
    pub t_observed: usize,
    /// The filter's committed component `(argmax, weight)` — `None`
    /// before the menu freezes.
    pub committed: Option<(usize, f32)>,
    /// Whether the stale-store gate has fired (store term dropped).
    pub store_stale: bool,
}

/// The incumbent min-delta tiebreak key (the driver's own
/// `order_candidates` order — file → line → total length → strategy id;
/// the `PoC` + `gp_ordering` precedent for a local key at this seam).
fn incumbent_key(c: &FixCandidate) -> (PathBuf, usize, usize, String) {
    (c.file.clone(), c.line, c.total_len(), c.strategy_id.clone())
}

impl NonergodicOrdering {
    /// Build the policy; `store` (when present) seeds the base ranking and
    /// composes the arm-D score blend with the stale-detection gate.
    #[must_use]
    pub fn new(store: Option<Arc<TrajStore>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                menu: Vec::new(),
                models: Vec::new(),
                filter: None,
                t_observed: 0,
                suppressions: HashMap::new(),
                ctx: HashMap::new(),
                span_file: HashMap::new(),
            })),
            store,
        }
    }

    /// The policy's health readout — see [`PolicySnapshot`].
    ///
    /// # Panics
    ///
    /// Panics if the policy mutex is poisoned (a closure panicked mid-call).
    #[must_use]
    pub fn snapshot(&self) -> PolicySnapshot {
        let g = self.inner.lock().expect("nonergodic policy mutex");
        PolicySnapshot {
            menu_len: g.menu.len(),
            t_observed: g.t_observed,
            committed: g.filter.as_ref().map(NonergodicFilter::committed),
            store_stale: g.store_stale(),
        }
    }

    /// The T4.2 ordering closure: score DESC by the (composed) posterior,
    /// exact ties by the incumbent min-delta key.
    ///
    /// # Panics
    ///
    /// Panics if the policy mutex is poisoned (a closure panicked mid-call).
    #[must_use]
    pub fn ordering(self: &Arc<Self>) -> OrderingFn {
        let inner = Arc::clone(&self.inner);
        let store = self.store.clone();
        Arc::new(move |cands: &mut [FixCandidate]| {
            if cands.is_empty() {
                return;
            }
            let mut g = inner.lock().expect("nonergodic policy mutex");
            if g.menu.is_empty() {
                freeze_menu(&mut g, cands, store.as_deref());
            }
            // Refresh per-file context hashes from THIS pass's candidates
            // (the revive policy's "has the question changed" input) + the
            // span_hash→file reverse map the observer needs (the ring record
            // carries only the hash).
            g.ctx.clear();
            g.span_file.clear();
            let mut per_file: HashMap<&Path, Vec<(String, usize, String)>> = HashMap::new();
            for c in cands.iter() {
                per_file.entry(&c.file).or_default().push((
                    c.code.clone(),
                    c.line,
                    c.strategy_id.clone(),
                ));
                g.span_file
                    .insert(span_hash(&c.file, c.line), c.file.clone());
            }
            for (f, items) in per_file {
                g.ctx.insert(f.to_path_buf(), Inner::ctx_of(&items));
            }

            // Precompute scores (the store read must not re-retrieve per
            // sort comparison — the `PoC`'s measured lesson).
            // t_observed ≤ max_iters — lossless in practice.
            #[allow(clippy::cast_precision_loss)] // bounded by max_iters
            let t = g.t_observed as f32;
            let lambda = match (&store, g.store_stale()) {
                // Stale gate + filter-only both read λ = 1: the first drops
                // the store term by contradiction, the second never had one.
                (Some(_), true) | (None, _) => 1.0f32,
                (Some(_), false) => t / (t + 1.0),
            };
            let mut scored: Vec<(f32, FixCandidate)> = cands
                .iter()
                .map(|c| {
                    let fs = match g.menu_index(&c.strategy_id) {
                        Some(mi) => g.filter_score(mi),
                        None => NEUTRAL,
                    };
                    let final_score = match &store {
                        Some(s) => {
                            let ss = shape_conditional_score(s, &c.code, &c.strategy_id, &c.shape);
                            (1.0 - lambda) * ss + lambda * fs
                        }
                        None => fs,
                    };
                    (final_score, c.clone())
                })
                .collect();
            scored.sort_by(|a, b| {
                b.0.total_cmp(&a.0)
                    .then_with(|| incumbent_key(&a.1).cmp(&incumbent_key(&b.1)))
            });
            for (slot, (_, c)) in cands.iter_mut().zip(scored) {
                *slot = c;
            }
        })
    }

    /// The per-attempt observer: ticks the filter on strategy-informative
    /// outcomes (io failures are environment noise — no tick, but the
    /// suppression is still recorded for revive) and keeps the suppression
    /// bookkeeping.
    ///
    /// # Panics
    ///
    /// Panics if the policy mutex is poisoned (a closure panicked mid-call).
    #[must_use]
    pub fn observer(self: &Arc<Self>) -> ObserverFn {
        let inner = Arc::clone(&self.inner);
        Arc::new(move |rec: &AttemptRecord| {
            let mut g = inner.lock().expect("nonergodic policy mutex");
            let outcome_idx = match rec.outcome {
                AttemptOutcome::Kept => 0,
                // Both revert classes map to the no-progress token — the
                // driver's keep rule is strictly-fewer-errors, so a
                // new-errors revert reports as no-progress at this seam;
                // io reverts are skipped for the tick below but keep the
                // same mapping for the suppression bookkeeping.
                AttemptOutcome::RevertedNoProgress | AttemptOutcome::RevertedIo => 1,
            };
            if rec.outcome != AttemptOutcome::RevertedIo {
                if let Some(mi) = g.menu_index(&rec.strategy_id) {
                    if let Some(f) = g.filter.as_mut() {
                        // mi ≤ MENU_CAP−1, outcome_idx ≤ 1 → max 46 — a
                        // bounded menu token, never truncating.
                        #[allow(clippy::cast_possible_truncation)]
                        f.tick((mi * 3 + outcome_idx) as u8);
                        g.t_observed += 1;
                    }
                }
            }
            let key = (rec.span_hash.clone(), rec.strategy_id.clone());
            if let AttemptOutcome::Kept = rec.outcome {
                // The strategy worked — the site is gone; a future
                // proposal here is a new question (the `PoC`'s
                // `tried.remove` on resolve).
                g.suppressions.remove(&key);
            } else {
                let sup_argmax = g.filter.as_ref().map_or(0, |f| f.committed().0);
                // The context this attempt was proposed under = the
                // last ordering pass's hash for its file.
                let sup_ctx = g
                    .span_file
                    .get(&rec.span_hash)
                    .and_then(|f| g.ctx.get(f).copied())
                    .unwrap_or_default();
                match g.suppressions.entry(key) {
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        let e = e.get_mut();
                        e.sup_argmax = sup_argmax;
                        e.sup_ctx = sup_ctx;
                    }
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(Suppression {
                            sup_argmax,
                            sup_ctx,
                            revive_used: false,
                        });
                    }
                }
            }
        })
    }

    /// The context-scoped one-shot revive policy: a suppressed
    /// `(site, strategy)` retries only when the filter's committed
    /// component has MOVED, the file's diagnostic context has MOVED, and
    /// `revive_gate()` clears the threshold — one revive per suppression.
    ///
    /// # Panics
    ///
    /// Panics if the policy mutex is poisoned (a closure panicked mid-call).
    #[must_use]
    pub fn revive(self: &Arc<Self>) -> ReviveFn {
        let inner = Arc::clone(&self.inner);
        Arc::new(move |c: &FixCandidate| {
            let mut g = inner.lock().expect("nonergodic policy mutex");
            let key = (span_hash(&c.file, c.line), c.strategy_id.clone());
            // Read the suppression state first (immutable), decide, then
            // write back the one-shot flag — keeps the borrows disjoint.
            let Some((sup_argmax, sup_ctx, revive_used)) = g
                .suppressions
                .get(&key)
                .map(|e| (e.sup_argmax, e.sup_ctx, e.revive_used))
            else {
                return false;
            };
            if revive_used {
                return false;
            }
            let Some(f) = g.filter.as_ref() else {
                return false;
            };
            let (cur_argmax, _w) = f.committed();
            let cur_ctx = g.ctx.get(&c.file).copied().unwrap_or_default();
            let revived =
                sup_argmax != cur_argmax && sup_ctx != cur_ctx && f.revive_gate() >= REVIVE_GATE;
            if revived {
                g.suppressions
                    .get_mut(&key)
                    .expect("key checked above")
                    .revive_used = true;
            }
            revived
        })
    }
}

/// Freeze the menu + build the filter at the first ordering call.
///
/// Base ranking: store shape-conditional score (max over the strategy's
/// candidates in this pass) when a store is present; otherwise the
/// incumbent min-delta first-seen rank. Component `k` favors base rank
/// `k` (mod menu len) — component 0 IS the base ranking.
fn freeze_menu(g: &mut Inner, cands: &[FixCandidate], store: Option<&TrajStore>) {
    let strategies: BTreeSet<&str> = cands.iter().map(|c| c.strategy_id.as_str()).collect();
    let menu: Vec<String> = strategies
        .into_iter()
        .take(MENU_CAP)
        .map(str::to_string)
        .collect();
    let n = menu.len();
    if n == 0 {
        return;
    }

    // Base rank per strategy: incumbent first-seen order.
    let mut by_incumbent: Vec<&FixCandidate> = cands.iter().collect();
    by_incumbent.sort_by_key(|a| incumbent_key(a));
    let mut incumbent_rank: HashMap<&str, usize> = HashMap::with_capacity(by_incumbent.len());
    for c in &by_incumbent {
        let sid = c.strategy_id.as_str();
        if !incumbent_rank.contains_key(sid) {
            let r = incumbent_rank.len();
            incumbent_rank.insert(sid, r);
        }
    }
    // Store score per strategy (max over its candidates), when present.
    let mut store_score: HashMap<&str, f32> = HashMap::new();
    if let Some(s) = store {
        for c in cands {
            let sc = shape_conditional_score(s, &c.code, &c.strategy_id, &c.shape);
            let e = store_score.entry(c.strategy_id.as_str());
            e.and_modify(|v| *v = v.max(sc)).or_insert(sc);
        }
    }
    // Rank the menu: store score desc, then incumbent rank asc.
    let mut ranked: Vec<&String> = menu.iter().collect();
    ranked.sort_by(|a, b| {
        let sa = store_score.get(a.as_str()).copied().unwrap_or(0.0);
        let sb = store_score.get(b.as_str()).copied().unwrap_or(0.0);
        sb.total_cmp(&sa).then_with(|| {
            incumbent_rank
                .get(a.as_str())
                .copied()
                .unwrap_or(usize::MAX)
                .cmp(
                    &incumbent_rank
                        .get(b.as_str())
                        .copied()
                        .unwrap_or(usize::MAX),
                )
        })
    });
    let rank_of: HashMap<&str, usize> = ranked
        .iter()
        .enumerate()
        .map(|(r, s)| (s.as_str(), r))
        .collect();

    // The permutation the models consume: `rank_of_menu[m]` = base rank of
    // the strategy at menu position `m` (the menu is alphabetical; the
    // models index q by menu position but decay by RANK distance).
    let rank_of_menu: Vec<usize> = menu
        .iter()
        .map(|s| rank_of.get(s.as_str()).copied().unwrap_or(usize::MAX))
        .collect();

    // Build + leak the K rotation models over the ranked menu: component
    // `k` favors base rank `k % n` — component 0 IS the base ranking.
    // (`Box::leak` yields `&'static mut`; the shared readout takes `&`.)
    let mut models: Vec<&'static RotationModel> = Vec::with_capacity(K_COMPONENTS);
    for k in 0..K_COMPONENTS {
        let leaked: &'static mut RotationModel =
            Box::leak(Box::new(RotationModel::new(k % n, &rank_of_menu)));
        models.push(&*leaked);
    }
    let refs: [&'static dyn ComponentModel; K_COMPONENTS] =
        [models[0], models[1], models[2], models[3]];
    #[allow(clippy::cast_precision_loss)] // K_COMPONENTS = 4 — lossless.
    let filter = NonergodicFilter::new(refs, [1.0 / K_COMPONENTS as f32; K_COMPONENTS])
        .with_initial_eta([[1.0 - INITIAL_MESSY, INITIAL_MESSY]; K_COMPONENTS]);

    g.menu = menu;
    g.models = models;
    g.filter = Some(filter);
}
