//! ELO rating math for Issue 039 — rule-vs-shape ratings over the
//! trajectory store's verdict history.
//!
//! Adapted locally (with citation) from the two in-tree katgpt-rs paths;
//! promotion to `katgpt-core` is filed as katgpt-rs Issue 686 — this module
//! switches to the core primitive when 686 lands:
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

/// K-factor (per-match maximum movement) — `EloCalculator`'s default.
pub const ELO_K: f32 = 32.0;

/// Elo per log10 unit — the standard-chess scale both katgpt-rs paths use
/// (`PlackettLuceConfig::elo_scale`).
pub const ELO_SCALE: f32 = 400.0;

/// The base (unrated) rating — standard chess 1200, the PL conversion's
/// `elo_offset`. Lives here (with the math) since Issue 039 T2;
/// `traj_store` re-exports it for back-compat.
#[must_use]
pub fn default_elo() -> f32 {
    1200.0
}

/// Expected score of `a` vs `b` — `1 / (1 + 10^((b-a)/400))` (the arena
/// formula at f32).
#[must_use]
pub fn expected_score(a: f32, b: f32) -> f32 {
    1.0 / (1.0 + 10.0_f32.powf((b - a) / ELO_SCALE))
}

/// One zero-sum match update — returns `(new_a, new_b)`; the arena
/// `update` formula at f32 (the winner gains what the loser loses, up to
/// float rounding).
#[must_use]
pub fn update_pair(a: f32, b: f32, a_won: bool) -> (f32, f32) {
    let e_a = expected_score(a, b);
    let (s_a, s_b) = if a_won { (1.0, 0.0) } else { (0.0, 1.0) };
    (a + ELO_K * (s_a - e_a), b + ELO_K * (s_b - (1.0 - e_a)))
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

#[cfg(test)]
mod tests {
    use super::*;

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
            assert!((f64::from(t.rating("a")) - ra).abs() < 0.01, "a: {} vs {ra}", t.rating("a"));
            assert!((f64::from(t.rating("b")) - rb).abs() < 0.01, "b: {} vs {rb}", t.rating("b"));
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
}
