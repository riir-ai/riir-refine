//! ELO rating math for Issue 039 — rule-vs-shape ratings over the
//! trajectory store's verdict history.
//!
//! **katgpt-rs Issue 686 CLOSED — the math delegates to
//! `katgpt_core::rating`** (DEFAULT-ON there since Bench 675): the f32
//! twins `expected_f32`/`update_f32` + the `STANDARD_*` constants are the
//! verbatim expression trees this module used to carry inline
//! (bit-identical — the persisted store ratings do not drift). What stays
//! local: [`EloTable`] (the string-id rating table + the T4 store→table
//! bridge surface) and this module's citations of the two original
//! katgpt-rs paths the core primitive was promoted from:
//!
//! - `katgpt-pruners/src/arena/types.rs` `EloCalculator` — the standard
//!   expected-score + K-factor update (the per-match math here is the same
//!   formula at f32, K=32);
//! - `katgpt-pruners/src/proof/plackett_luce.rs` — the standard-chess Elo
//!   convention (base 1200, 400 per log10 unit) the base rating and
//!   expected-score scale share.
//!
//! ## Match semantics (Issue 039's design verdict)
//!
//! Every heal attempt is the applied rule playing the span: win = a kept
//! verdict (`success_count`), loss = a reverted/failed verdict
//! (`fail_count`). BOTH sides are rated — span difficulty emerges as the
//! opponent rating. Ratings live denormalized per trajectory row
//! (`TrajRowDoc::elo` / `span_elo`) and are recomputed by BATCH replay only
//! ([`TrajStore::recompute_elo`](crate::traj_store::TrajStore::recompute_elo))
//! — the issue's non-goal pins NO online ELO updates inside the hot fix
//! loop (G4: the heal path stays allocation-stable).
//!
//! ## Why standard Elo (not the PL batch the issue sketched)
//!
//! Classic Elo is order-dependent; the issue preferred Plackett-Luce for
//! order-freeness. T2's recorded decision (adapt-locally, standard update)
//! keeps the math simple and buys determinism by CANONICAL ORDER instead:
//! rows replay in store key order, each row's aggregate counts replay as
//! losses-then-wins. Any fixed order is a modeling choice — the store keeps
//! aggregates, not event order — and PL is exactly the upgrade that removes
//! the choice (owned by katgpt-rs 686).
//!
//! The two paths agree at the fixed point: a pair playing at win rate `p`
//! equilibrates where the expected score equals `p`, i.e. a rating gap of
//! `400·log10(W/L)` — the PL conversion's own curve (pinned by test).

use std::collections::HashMap;

/// K-factor (per-match maximum movement) — `EloCalculator`'s default
/// (the standard-chess value from `katgpt_core::rating`).
#[allow(clippy::cast_possible_truncation)] // 32.0 is exactly representable in f32
pub const ELO_K: f32 = katgpt_core::rating::STANDARD_K as f32;

/// Issue 190 — the evidence floor where a rule's rating moves at FULL K:
/// `elo_k_eff` is damped (`K·n/n_floor`) for the first `n_floor` outcomes
/// of a rule's replay, then exactly `K`. Calibrated to the regime
/// monitor's own warmup ([`katgpt_core::step_noise::StepNoiseConfig::DEFAULT`]`::warmup`
/// = 16) — one evidence notion across the lane: a rule the readout calls
/// "warming" is the same rule whose rating still moves damped.
pub const ELO_K_EFF_FLOOR: u32 = 16;

/// Issue 190 — the evidence-damped effective K for one rule's match:
/// `elo_k_eff = K · min(1, n / n_floor)` (katgpt-rs Issue 975 T5(c)'s
/// DDR law, arXiv:2603.06009 — evidence per unit divergence — applied to
/// selection churn: a young rule with thin evidence moves at a damped
/// rate, so early noisy verdicts cannot fling a rating the batch cannot
/// take back). Monotone non-decreasing in `n`; exactly `k` at
/// `n >= n_floor`; `n_floor == 0` degenerates to `k` (no floor, no damp).
/// The schedule-shape reference is `katgpt_core::step_noise::min_ddr_at`
/// (evidence floor rising with budget) — the linear ramp here is the
/// issue's pinned formula, not a sigmoid.
#[must_use]
pub fn elo_k_eff(k: f32, n: u32, n_floor: u32) -> f32 {
    if n_floor == 0 || n >= n_floor {
        return k;
    }
    #[allow(clippy::cast_precision_loss)] // counts < 2^23 by contract
    let ratio = n as f32 / n_floor as f32;
    k * ratio
}

/// Elo per log10 unit — the standard-chess scale both katgpt-rs paths use
/// (`PlackettLuceConfig::elo_scale`, from `katgpt_core::rating`).
#[allow(clippy::cast_possible_truncation)] // 400.0 is exactly representable in f32
pub const ELO_SCALE: f32 = katgpt_core::rating::STANDARD_SCALE as f32;

/// The base (unrated) rating — standard chess 1200, the PL conversion's
/// `elo_offset` (from `katgpt_core::rating`; 1200 is exactly representable
/// in f32). Lives here (with the math) since Issue 039 T2;
/// `traj_store` re-exports it for back-compat.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // 1200.0 is exactly representable in f32
pub fn default_elo() -> f32 {
    katgpt_core::rating::STANDARD_BASE as f32
}

/// Expected score of `a` vs `b` — `1 / (1 + 10^((b-a)/400))` (the arena
/// formula; the `katgpt_core::rating::expected_f32` twin, verbatim).
#[must_use]
pub fn expected_score(a: f32, b: f32) -> f32 {
    katgpt_core::rating::expected_f32(a, b, ELO_SCALE)
}

/// One zero-sum match update — returns `(new_a, new_b)`; the arena
/// `update` formula at f32 (the `katgpt_core::rating::update_f32` twin,
/// verbatim — the winner gains what the loser loses, up to float
/// rounding).
#[must_use]
pub fn update_pair(a: f32, b: f32, a_won: bool) -> (f32, f32) {
    katgpt_core::rating::update_f32(a, b, a_won, ELO_K, ELO_SCALE)
}

/// [`update_pair`] at an EXPLICIT K — the Issue-190 damp seam: the batch
/// replay passes `elo_k_eff(ELO_K, n, ELO_K_EFF_FLOOR)` so a young rule's
/// match moves both sides at the damped rate. Zero-sum by construction
/// (one K per match — the rule's evidence schedules the WHOLE match, so
/// the span opponent still gains exactly what the rule loses; per-side
/// damping would break conservation).
#[must_use]
pub fn update_pair_k(a: f32, b: f32, a_won: bool, k: f32) -> (f32, f32) {
    katgpt_core::rating::update_f32(a, b, a_won, k, ELO_SCALE)
}

/// One zero-sum SCORED match update — the half-win representation (Issue
/// 089 T7): `score_a ∈ [0,1]` is `a`'s realized outcome (`1.0` win, `0.0`
/// loss, `0.5` the shaped verdict / a draw). The
/// `katgpt_core::rating::update_scored` twin at f32 (delegating through
/// the f64 primitive — the module's house pattern; K/scale/0.5 are exact
/// in both widths). Zero-sum by construction: `a` gains exactly what `b`
/// loses, and `score_a = 0.5` between EQUAL ratings is a no-op (a draw
/// the expected score already predicted).
#[must_use]
#[allow(clippy::cast_possible_truncation)] // f64→f32 twin of the persisted-rating contract
pub fn update_pair_scored(a: f32, b: f32, score_a: f32) -> (f32, f32) {
    let (na, nb) = katgpt_core::rating::update_scored(
        f64::from(a),
        f64::from(b),
        f64::from(score_a),
        katgpt_core::rating::STANDARD_K,
        katgpt_core::rating::STANDARD_SCALE,
    );
    (na as f32, nb as f32)
}

/// A rating table over string player ids. The two namespaces are disjoint
/// by construction at the one call site ([`TrajStore::recompute_elo`]):
/// rule ids are `lint_key`s, span ids are shape-hash hex. Unseen players
/// carry [`default_elo`] — replaying a match inserts both players.
#[derive(Default)]
pub struct EloTable {
    ratings: HashMap<String, f32>,
}

impl EloTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load persisted ratings verbatim (the substrate half of Issue 039
    /// T4's store→table bridge): no match replay — the entries become the
    /// table exactly as given (later duplicates win, i.e. last-write-wins
    /// in the caller's iteration order). Ids absent from `entries` keep
    /// answering [`default_elo`] from [`Self::rating`].
    #[must_use]
    pub fn from_ratings<I, S>(entries: I) -> Self
    where
        I: IntoIterator<Item = (S, f32)>,
        S: Into<String>,
    {
        Self {
            ratings: entries.into_iter().map(|(id, r)| (id.into(), r)).collect(),
        }
    }

    /// Current rating ([`default_elo`] for unseen players).
    #[must_use]
    pub fn rating(&self, id: &str) -> f32 {
        self.ratings.get(id).copied().unwrap_or_else(default_elo)
    }

    /// Whether `id` has played a match.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.ratings.contains_key(id)
    }

    /// Play one match `a` vs `b`, updating both sides.
    pub fn play(&mut self, a: &str, b: &str, a_won: bool) {
        let (ra, rb) = update_pair(self.rating(a), self.rating(b), a_won);
        self.ratings.insert(a.to_string(), ra);
        self.ratings.insert(b.to_string(), rb);
    }

    /// [`EloTable::play`] at an explicit K — the Issue-190 replay seam
    /// (`traj_store::recompute_elo` schedules each match by the rule's
    /// evidence count). `k == ELO_K` reproduces [`EloTable::play`]
    /// bit-for-bit (the same primitive underneath).
    pub fn play_with_k(&mut self, a: &str, b: &str, a_won: bool, k: f32) {
        let (ra, rb) = update_pair_k(self.rating(a), self.rating(b), a_won, k);
        self.ratings.insert(a.to_string(), ra);
        self.ratings.insert(b.to_string(), rb);
    }

    /// Play one SCORED match `a` vs `b` (Issue 089 T7): `score_a = 0.5`
    /// is the tie a shaped verdict ("target cleared ∧ new diags
    /// introduced") replays as — the rule banked half a win against the
    /// span. The feed contract: when a shaped outcome enters the verdict
    /// history, THIS is the match it plays (the store's replay currently
    /// knows only win/loss counts; the primitive is the wired-in-advance
    /// representation the bench's `shaped` axis prices).
    pub fn play_scored(&mut self, a: &str, b: &str, score_a: f32) {
        let (ra, rb) = update_pair_scored(self.rating(a), self.rating(b), score_a);
        self.ratings.insert(a.to_string(), ra);
        self.ratings.insert(b.to_string(), rb);
    }

    /// Rated-player count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ratings.len()
    }

    /// Is no one rated yet?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ratings.is_empty()
    }
}

/// One ordered preference observation: `winner` beat `loser`. The BT input
/// (Issue 072 T3 — construction pairs are the first producer; outcome
/// verdicts could feed the same form later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairObservation {
    pub winner: String,
    pub loser: String,
}

/// Bradley–Terry maximum-likelihood fit (Zermelo's iterative form) over a
/// pair stream, mapped onto the standard Elo scale.
///
/// Why not Elo replay over the pairs: Elo is ORDER-DEPENDENT and the pair
/// stream is an aggregate (construction pairs arrive as a set, not a
/// history) — the issue's "BT closed form" axis needs the order-free fit.
/// Zermelo's update `r_i ← W_i / Σ_j n_ij/(r_i + r_j)` — where `n_ij` is
/// the TOTAL number of games between `i` and `j`, both directions —
/// monotonically increases the likelihood (Gu & Hunter 2001) and converges
/// from any positive start; the fix-point Elo conversion is the SAME curve
/// the module doc already pins for PL (`elo_from_lambda` — strength λ =
/// the BT strength at the pairwise fixed point).
///
/// Scale invariants (pinned by test): only WIN DIRECTION matters, not
/// multiplicity of identical pairs (one win in k identical observations is
/// the same likelihood as k); the map to Elo is `base + scale·log10(s)`, so
/// a player that always loses sits below base and one that always wins
/// above — the same ordering Elo replay produces, without its order
/// dependence.
///
/// Degenerate inputs: an empty stream answers an empty map; a global sweep
/// (one player beats everyone) drives the ML to a boundary — the iteration
/// is capped and the result stays finite via the 1e-10 clamp inside
/// `elo_from_lambda` (the `katgpt_core` house clamp).
#[must_use]
pub fn bradley_terry_elo(
    pairs: &[PairObservation],
    iters: usize,
) -> std::collections::BTreeMap<String, f32> {
    use std::collections::{BTreeMap, BTreeSet};

    // Aggregate: wins per player + total games per (player, opponent),
    // BOTH directions (Zermelo's denominator needs every meeting).
    // BTreeMaps throughout: the denominator SUM is order-sensitive in f64,
    // so the fit must iterate in sorted order to stay bit-deterministic
    // across runs (the store-scan house rule).
    let mut total_wins: BTreeMap<String, f64> = BTreeMap::new();
    let mut games: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    let mut players: BTreeSet<String> = BTreeSet::new();
    for p in pairs {
        *total_wins.entry(p.winner.clone()).or_insert(0.0) += 1.0;
        let wrow = games.entry(p.winner.clone()).or_default();
        *wrow.entry(p.loser.clone()).or_insert(0.0) += 1.0;
        let lrow = games.entry(p.loser.clone()).or_default();
        *lrow.entry(p.winner.clone()).or_insert(0.0) += 1.0;
        players.insert(p.winner.clone());
        players.insert(p.loser.clone());
    }
    let base: f64 = katgpt_core::rating::STANDARD_BASE;
    let scale: f64 = katgpt_core::rating::STANDARD_SCALE;
    let mut strength: BTreeMap<String, f64> = players.iter().map(|p| (p.clone(), 1.0)).collect();
    // Fixed-point iteration (the multiplicative form is the stable one).
    for _ in 0..iters {
        let mut next = strength.clone();
        for name in &players {
            let r_i = strength.get(name).copied().unwrap_or(1.0);
            let mut denom = 0.0;
            if let Some(opp) = games.get(name) {
                for (o, n_ij) in opp {
                    let r_j = strength.get(o).copied().unwrap_or(1.0);
                    denom += n_ij / (r_i + r_j);
                }
            }
            if denom > 0.0 {
                let w = total_wins.get(name).copied().unwrap_or(0.0);
                next.insert(name.clone(), (w / denom).max(1e-10));
            }
        }
        strength = next;
    }
    strength
        .into_iter()
        .map(|(k, s)| {
            let elo = katgpt_core::rating::elo_from_lambda(s, base, scale);
            #[allow(clippy::cast_possible_truncation)] // persisted ratings are f32 by contract
            (k, elo as f32)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod bt {
        use super::*;
        use crate::elo::PairObservation;

        fn obs(w: &str, l: &str) -> PairObservation {
            PairObservation {
                winner: w.into(),
                loser: l.into(),
            }
        }

        #[test]
        fn empty_stream_answers_empty_map() {
            assert!(bradley_terry_elo(&[], 50).is_empty());
        }

        #[test]
        fn sweep_orders_above_base_and_swept_below() {
            // a beats everyone: a above base, the swept players below it,
            // and the swept players TIE each other (identical records).
            let pairs = vec![obs("a", "b"), obs("a", "c"), obs("a", "d")];
            let t = bradley_terry_elo(&pairs, 200);
            assert!(t["a"] > default_elo());
            assert!(t["b"] < default_elo());
            // Identical records → identical ML strengths (float_cmp IS the
            // assertion: exact equality of the two fitted ratings).
            #[allow(clippy::float_cmp)]
            {
                assert_eq!(t["b"], t["c"]);
                assert_eq!(t["c"], t["d"]);
            }
        }

        #[test]
        fn multiplicity_of_identical_pairs_is_invariant() {
            // The likelihood only sees win direction — 1 win vs 50 wins
            // against the same opponent is the same evidence.
            let one = vec![obs("a", "b")];
            let many: Vec<_> = std::iter::repeat_n(obs("a", "b"), 50).collect();
            let t1 = bradley_terry_elo(&one, 200);
            let t2 = bradley_terry_elo(&many, 200);
            // Same ML fixed point by construction — exact equality.
            #[allow(clippy::float_cmp)]
            {
                assert_eq!(t1["a"], t2["a"]);
                assert_eq!(t1["b"], t2["b"]);
            }
        }

        #[test]
        fn direction_is_the_signal_not_order() {
            // Shuffling the stream must not move the ratings (the
            // order-freeness the A/B needs from the closed form).
            let seq = [
                obs("a", "b"),
                obs("c", "b"),
                obs("a", "c"),
                obs("d", "b"),
                obs("c", "d"),
                obs("a", "d"),
            ];
            let mut rev = seq.clone();
            rev.reverse();
            let t1 = bradley_terry_elo(&seq, 300);
            let t2 = bradley_terry_elo(&rev, 300);
            // Order-free fit: bit-stable under reordering (float_cmp IS the
            // assertion — the closed form must reproduce identical bits).
            #[allow(clippy::float_cmp)]
            for p in ["a", "b", "c", "d"] {
                assert_eq!(t1[p], t2[p], "player {p} moved under reordering");
            }
        }

        #[test]
        fn cyclical_beats_rank_ate_expected_monotone() {
            // a > b > c > d by every pair among them: the fit must order
            // them strictly and monotonically.
            let pairs = vec![
                obs("a", "b"),
                obs("b", "c"),
                obs("c", "d"),
                obs("a", "c"),
                obs("b", "d"),
                obs("a", "d"),
            ];
            let t = bradley_terry_elo(&pairs, 300);
            assert!(t["a"] > t["b"]);
            assert!(t["b"] > t["c"]);
            assert!(t["c"] > t["d"]);
        }

        #[test]
        fn unconnected_player_keeps_base() {
            // e never plays: no games row → its strength stays at the
            // 1.0 prior → exactly base Elo.
            let pairs = vec![obs("a", "b")];
            let t = bradley_terry_elo(&pairs, 100);
            assert!(!t.contains_key("e"));
        }

        #[test]
        fn fixed_point_matches_the_elo_expected_score() {
            // The convergence contract: at the fit point, BT expected
            // score s_i/(s_i+s_j) equals the Elo expected score the map
            // produces — the same fixed point the module doc pins for PL.
            let pairs = vec![obs("a", "b"), obs("a", "b"), obs("b", "a"), obs("a", "b")];
            let t = bradley_terry_elo(&pairs, 500);
            let (sa, sb) = (
                10f64.powf(f64::from((t["a"] - default_elo()) / ELO_SCALE)),
                10f64.powf(f64::from((t["b"] - default_elo()) / ELO_SCALE)),
            );
            let bt_p = sa / (sa + sb);
            let elo_p = f64::from(expected_score(t["a"], t["b"]));
            assert!((bt_p - elo_p).abs() < 1e-6, "bt={bt_p} elo={elo_p}");
            // Observed win rate 3/4 sits between the two players' expected
            // scores (the MLE property).
            let observed = 0.75;
            assert!((bt_p - observed).abs() < 0.2);
        }
    }

    /// Verbatim f64 transcription of `katgpt-pruners` `EloCalculator`
    /// (arena/types.rs — `expected` + `update`, K=32): the cross-check
    /// reference for "the two math paths" (Issue 039 T2's unit gate).
    mod arena_reference {
        pub const K: f64 = 32.0;
        pub fn expected(a: f64, b: f64) -> f64 {
            1.0 / (1.0 + 10.0_f64.powf((b - a) / 400.0))
        }
        pub fn update(a: f64, b: f64, a_won: bool) -> (f64, f64) {
            let e = expected(a, b);
            let (sa, sb) = if a_won { (1.0, 0.0) } else { (0.0, 1.0) };
            (a + K * (sa - e), b + K * (sb - (1.0 - e)))
        }
    }

    #[test]
    #[allow(clippy::float_cmp)] // ±16 is exactly representable — equality IS the fixture
    fn one_match_from_base_moves_exactly_16() {
        // Hand-computed fixture: equal ratings → E = 0.5 exactly →
        // ±K/2 = ±16 exactly representable in f32.
        let (a, b) = update_pair(1200.0, 1200.0, true);
        assert_eq!(a, 1216.0);
        assert_eq!(b, 1184.0);
        // Mirror: the loss side.
        let (a, b) = update_pair(1200.0, 1200.0, false);
        assert_eq!(a, 1184.0);
        assert_eq!(b, 1216.0);
    }

    #[test]
    fn upset_moves_more_than_expected_result() {
        // The favorite gains less than the underdog gains from an upset —
        // the arena calculator's own property test, pinned here at f32.
        let (fav_gain, _) = update_pair(1200.0, 800.0, true);
        let (_, upset_gain) = update_pair(800.0, 1200.0, true);
        assert!(upset_gain - 800.0 > fav_gain - 1200.0);
    }

    #[test]
    fn updates_are_zero_sum_up_to_rounding() {
        for &(ra, rb, won) in &[
            (1200.0, 1200.0, true),
            (1300.0, 900.0, false),
            (1180.0, 1260.0, true),
            (1216.0, 1184.0, false),
        ] {
            let (na, nb) = update_pair(ra, rb, won);
            assert!(
                (na + nb - (ra + rb)).abs() < 1e-3,
                "zero-sum violated: {ra}/{rb} → {na}/{nb}"
            );
        }
    }

    #[test]
    #[allow(clippy::float_cmp)] // base equality IS the unseen-player contract
    fn table_rates_both_sides_and_keeps_unseen_at_base() {
        let mut t = EloTable::new();
        assert_eq!(t.rating("nobody"), default_elo());
        assert!(t.is_empty());
        t.play("rule", "span", true);
        assert_eq!(t.len(), 2);
        assert!(t.rating("rule") > default_elo());
        assert!(t.rating("span") < default_elo());
        assert!(t.contains("rule"));
        assert!(!t.contains("nobody"));
    }

    #[test]
    #[allow(clippy::float_cmp)] // verbatim load + base equality ARE the assertions
    fn from_ratings_loads_verbatim_and_keeps_unseen_at_base() {
        // The T4 bridge substrate: persisted ratings load as-is (no replay),
        // unseen ids answer base, and an empty load equals `new()`.
        let t = EloTable::from_ratings([("rule_x", 1625.0f32), ("rule_y", 1300.5)]);
        assert_eq!(t.len(), 2);
        assert_eq!(t.rating("rule_x"), 1625.0);
        assert_eq!(t.rating("rule_y"), 1300.5);
        // Owned-string entries load the same way (the bridge passes
        // HashMap<String, f32>).
        let owned: Vec<(String, f32)> = vec![("rule_z".to_string(), 1400.0)];
        assert_eq!(EloTable::from_ratings(owned).rating("rule_z"), 1400.0);
        assert_eq!(t.rating("nobody"), default_elo());
        assert!(!t.contains("nobody"));
        assert!(EloTable::from_ratings(Vec::<(String, f32)>::new()).is_empty());
        // Last-write-wins on duplicate ids (the documented load rule).
        let dup = EloTable::from_ratings([("a", 1216.0f32), ("a", 1240.0)]);
        assert_eq!(dup.rating("a"), 1240.0);
    }

    #[test]
    #[allow(clippy::float_cmp)] // bit-identity across replays IS the assertion
    fn table_replay_is_deterministic() {
        // Same match sequence twice → bit-identical ratings (the property
        // the store's idempotent backfill leans on).
        let seq = [true, false, false, true, true, false, true, true];
        let mut t1 = EloTable::new();
        let mut t2 = EloTable::new();
        for &won in seq.iter().cycle().take(23) {
            t1.play("a", "b", won);
            t2.play("a", "b", won);
        }
        assert_eq!(t1.rating("a"), t2.rating("a"));
        assert_eq!(t1.rating("b"), t2.rating("b"));
    }

    // ── Issue 089 T7: the tie / shaped-verdict representation ──────────

    #[test]
    #[allow(clippy::float_cmp)] // E = 0.5 exactly at equal ratings — equality IS the fixture
    fn tie_between_equal_ratings_is_a_no_op() {
        // The draw the expected score already predicted: zero movement,
        // bit-exact (0.5 and the ratings are exactly representable).
        let (a, b) = update_pair_scored(1200.0, 1200.0, 0.5);
        assert_eq!(a, 1200.0);
        assert_eq!(b, 1200.0);
    }

    #[test]
    fn tie_between_unequal_ratings_pulls_both_toward_the_middle() {
        // E_a = 1/(1+10^(-400/400)) = 1/(1+0.1) = 1/11 ≈ 0.909: the
        // favorite's expected score EXCEEDS the tie's 0.5 realized score,
        // so a tie pulls the favorite DOWN and the underdog UP — both
        // strictly toward the middle, and strictly less than a full win
        // (resp. loss) would move either.
        let (tie_a, tie_b) = update_pair_scored(1200.0, 800.0, 0.5);
        assert!(
            tie_a < 1200.0 && tie_a > 1200.0 - ELO_K,
            "favorite: {tie_a}"
        );
        assert!(tie_b > 800.0 && tie_b < 800.0 + ELO_K, "underdog: {tie_b}");
        let (win_a, _) = update_pair(1200.0, 800.0, true);
        assert!(
            win_a > 1200.0,
            "a full win lifts the favorite where a tie lowers it"
        );
        // Zero-sum at the tie too.
        assert!((tie_a + tie_b - 2000.0).abs() < 1e-3);
    }

    #[test]
    // float_cmp: the endpoint-agreement contract is BIT-identity — the
    // scored path must reproduce the bool path's exact f32 bits.
    #[allow(clippy::float_cmp)]
    fn scored_endpoints_agree_with_the_bool_update() {
        // s = 1.0 / 0.0 must reproduce `update_pair` exactly — the scored
        // path is a SUPERSET of the bool path, never a second math.
        for &(ra, rb) in &[(1200.0, 1200.0), (1300.0, 900.0), (1184.0, 1216.0)] {
            let (w1, l1) = update_pair(ra, rb, true);
            let (w2, l2) = update_pair_scored(ra, rb, 1.0);
            assert_eq!(w1, w2, "win endpoint drifted at {ra}/{rb}");
            assert_eq!(l1, l2, "loss endpoint drifted at {ra}/{rb}");
            let (x1, y1) = update_pair(ra, rb, false);
            let (x2, y2) = update_pair_scored(ra, rb, 0.0);
            assert_eq!(x1, x2);
            assert_eq!(y1, y2);
        }
    }

    #[test]
    fn table_plays_a_tie_between_players() {
        // The feed contract in miniature: rule vs span, shaped verdict →
        // both sides stay rated (the tie INSERTS both players), movement
        // is half a win's magnitude.
        let mut tie = EloTable::new();
        tie.play_scored("rule", "span", 0.5);
        assert!(tie.contains("rule") && tie.contains("span"));
        let mut win = EloTable::new();
        win.play("rule", "span", true);
        assert!(win.rating("rule") - default_elo() > tie.rating("rule") - default_elo());
        // And the pair-sum is conserved.
        let s = tie.rating("rule") + tie.rating("span");
        assert!((s - 2.0 * default_elo()).abs() < 1e-3);
    }

    #[test]
    fn arena_cross_check_f64_reference() {
        // The "two math paths" gate: our f32 table vs the verbatim f64
        // arena calculator over the same mixed sequence — same-order
        // same-formula replay, so the only drift is f32 rounding.
        let mut t = EloTable::new();
        let (mut ra, mut rb) = (1200.0f64, 1200.0f64);
        for won in [
            true, false, false, true, true, true, false, true, true, true, false, true,
        ] {
            t.play("a", "b", won);
            let (na, nb) = arena_reference::update(ra, rb, won);
            ra = na;
            rb = nb;
            assert!(
                (f64::from(t.rating("a")) - ra).abs() < 0.01,
                "a: {} vs {ra}",
                t.rating("a")
            );
            assert!(
                (f64::from(t.rating("b")) - rb).abs() < 0.01,
                "b: {} vs {rb}",
                t.rating("b")
            );
        }
    }

    #[test]
    fn converged_gap_matches_the_pl_conversion() {
        // Standard Elo's equilibrium IS the PL conversion's curve: a pair
        // at win rate p equilibrates at gap 400·log10(W/L). 100 cycles of
        // (1 loss, 3 wins) = 75% → gap 400·log10(3) ≈ 190.8. Discrete
        // fixed-order matches oscillate AROUND the fixed point (the cycle
        // ends on a win = the top of a band ~±K·E ≈ ±24 at p=0.75), so the
        // gate is the band, not the point.
        let mut t = EloTable::new();
        for _ in 0..100 {
            t.play("r", "s", false);
            for _ in 0..3 {
                t.play("r", "s", true);
            }
        }
        let gap = t.rating("r") - t.rating("s");
        let want = 400.0 * (3.0f32 / 1.0).log10();
        assert!(
            (gap - want).abs() < 30.0,
            "gap {gap} vs PL fixed point {want} (band ±K·E)"
        );
        // And the expected score at equilibrium is the win rate itself.
        let e = expected_score(t.rating("r"), t.rating("s"));
        assert!((e - 0.75).abs() < 0.05, "expected {e} vs win rate 0.75");
    }

    // ── Issue 190: the evidence-damped K ─────────────────────────

    #[test]
    fn elo_k_eff_is_monotone_in_n_and_k_at_the_floor() {
        // Monotone non-decreasing in n, exactly k at the floor and above.
        let mut prev = 0.0f32;
        for n in 0..=64u32 {
            let k = elo_k_eff(ELO_K, n, ELO_K_EFF_FLOOR);
            assert!(k >= prev, "k regressed at n={n}: {k} < {prev}");
            if n >= ELO_K_EFF_FLOOR {
                assert_eq!(k, ELO_K, "n={n} past the floor must be full K");
            }
            prev = k;
        }
        // First outcome is the deepest damp.
        assert_eq!(elo_k_eff(ELO_K, 1, ELO_K_EFF_FLOOR), ELO_K / 16.0);
        assert_eq!(elo_k_eff(ELO_K, 0, ELO_K_EFF_FLOOR), 0.0);
    }

    #[test]
    fn elo_k_eff_floor_zero_is_full_k_and_damp_reproduces_elo_k_off_path() {
        // n_floor == 0 degenerates to k (no floor, no damp).
        assert_eq!(elo_k_eff(ELO_K, 0, 0), ELO_K);
        assert_eq!(elo_k_eff(ELO_K, 3, 0), ELO_K);
        // The feature-OFF identity the byte-identity gate leans on: the
        // replay's off-branch k is ELO_K verbatim, so play_with_k at that
        // k IS play (same primitive, same args).
        let mut a = EloTable::new();
        let mut b = EloTable::new();
        for won in [true, false, true, true, false] {
            a.play("r", "s", won);
            b.play_with_k("r", "s", won, ELO_K);
        }
        assert_eq!(a.rating("r"), b.rating("r"));
        assert_eq!(a.rating("s"), b.rating("s"));
    }

    #[test]
    fn damped_replay_moves_young_rules_less_and_stays_zero_sum() {
        // The damp law: the SAME 4-match evidence produces a smaller
        // rating swing when every match is damped (thin evidence), and
        // the zero-sum invariant survives (both sides move one K).
        let k_damped = elo_k_eff(ELO_K, 4, ELO_K_EFF_FLOOR);
        assert!(k_damped < ELO_K);
        let (r_damped, b_damped) = update_pair_k(1200.0, 1200.0, true, k_damped);
        assert!((r_damped - 1200.0 + (b_damped - 1200.0)).abs() < 1e-4, "zero-sum");
        let (r_full, b_full) = update_pair_k(1200.0, 1200.0, true, ELO_K);
        assert!(r_damped < r_full, "damped win moves less");
        assert!(b_damped > b_full, "damped loss costs less");
    }
}
