//! Issue 232 T3 — the multiplicity-gated arming predicate (L1's ordering
//! law; source: dang2026t3 via `riir-train/.research/495`).
//!
//! **The law:** T³'s L1 found that holdout-flavored machinery pays only on
//! surfaces where the same unit is encountered MANY times — on a 1-pass
//! surface (each unit seen once) reserving data for validation starves the
//! training half and buys nothing. The extraction here: a holdout-flavored
//! arm fires only when the surface's per-unit encounter multiplicity has
//! passed a LOCALLY MEASURED knee — and the fence is the same law in its
//! negative direction: a surface that fails the reuse-dominance precondition
//! (chain/quorum validation shapes are 1-pass by construction) never arms,
//! at ANY multiplicity.
//!
//! **Nothing is imported.** T³'s 4-pass knee is a training-surface constant
//! (the issue's non-goal pins this); the knee here is MEASURED from the
//! surface's own encounter-multiplicity histogram, per surface, at
//! [`MultiplicityGate::measure`] time. The only constants in this module
//! are definitional: the sigmoid's slope (a shape knob, never a decision
//! threshold) and the readout version.
//!
//! **The measurement (integer-exact, the Issue-940 family):** given the
//! per-unit encounter counts of a surface,
//!
//! 1. zero units → [`KneeRefusal::Blind`] (never a green zero);
//! 2. any zero-count unit → [`KneeRefusal::MalformedZero`] (an encounter
//!    ledger never records zero — loud, never folded into blind);
//! 3. **the fence:** reuse-dominance — `repeat_encounters >=
//!    first_encounters` ⟺ `total >= 2·units` (exact integer comparison).
//!    Fail → [`KneeRefusal::OnePassSurface`]: 1-pass surfaces never arm;
//!    holdout machinery buys nothing there (the chain/quorum boundary
//!    result the issue's fixture carries);
//! 4. **the knee:** the ENCOUNTER-MASS MEDIAN multiplicity — the largest
//!    `m` whose holders (`multiplicity >= m`) still carry at least HALF
//!    the encounter mass (`2·mass_above(m) >= total`, exact integer).
//!    Arming at `multiplicity >= knee` therefore selects the surface's
//!    above-median-reused half. Exact-half ties resolve to the LARGER m
//!    (arm late — a holdout gate is conservative about arming).
//!
//! **The gate:** `σ(λ·(m − knee))` over the unit's OWN counter — sigmoid,
//! never softmax. Because the gate reads one counter against one knee, a
//! unit's gate value is independent of every OTHER unit's counter (the
//! softmax discriminator: a normalized distribution would move every unit
//! when one unit's count moves). `armed(m) ⟺ gate(m) >= 0.5 ⟺ m >= knee`
//! — the boolean IS the sigmoid's own half; there is no second threshold
//! anywhere. `gate(knee)` is exactly 0.5 (`exp(0)`).
//!
//! **Determinism reclassification (Issue-940 discipline):** the knee is
//! integer-exact and cross-machine stable. The gate VALUE is an f64 `exp`
//! render — a local report-only readout, never a sync/quorum crossing —
//! recorded local-only at this site per the determinism law's own
//! procedure. The fixture pins bit patterns on this box.
//!
//! **Report-only:** nothing here gates, refuses, or demotes any fix path;
//! the per-consult [`Consult`] verdict and [`MultiplicityGate::consult_line`]
//! exist so a future consumer (T4's source-case-first gate, T5's
//! two-signal composition) arms THROUGH this predicate instead of a bare
//! counter comparison. The correlation cell (this signal vs later measured
//! holdout deltas) is the issue's standing gate before any promotion talk.
//!
//! ## Feature posture
//!
//! `multiplicity_gate` (opt-in, default-OFF, no implies — std only; the
//! knee needs no hashing, so even `blake3` stays out). Feature-off is
//! byte-identical by construction (new gated code only, no shared-path
//! edit). The CLI lane `--multiplicity-report` measures the REAL surface
//! this repo already records — the fixer's per-rule outcome encounters in
//! `.refine/trajectories.kv` — under
//! `all(multiplicity_gate, latent_retrieval)`; a build without the
//! features refuses exit 2 (the corpus-gap shape). Report-only first per
//! the issue's proof-path ledger.

/// The readout's schema version — carried in the rendered header, so a law
/// change moves every consumer loud.
pub const READOUT_VERSION: u64 = 1;

/// The sigmoid's slope, in nats per unit of multiplicity. Shape only: the
/// armed/unarmed split stays EXACTLY at the knee for every positive scale
/// (`σ(0) = 0.5`); this knob widens or narrows the transition band and is
/// never a decision threshold. Pinned at 1.0 — one multiplicity step on
/// either side of the knee moves the gate ~0.27/0.73.
pub const SIGMOID_SCALE_NATS: f64 = 1.0;

/// Why a knee could not be measured. Every variant renders a loud line;
/// none is ever folded into a green pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KneeRefusal {
    /// Zero units — a blind readout (never a green zero).
    Blind,
    /// A unit recorded with zero encounters — malformed, not blind (an
    /// encounter ledger never records zero; the measurement refuses loud
    /// naming the count rather than silently dropping rows).
    MalformedZero { units: usize },
    /// The fence: the surface fails reuse-dominance (`total < 2·units`) —
    /// it is 1-pass-shaped (the chain/quorum boundary result) and never
    /// arms, at any multiplicity.
    OnePassSurface { units: u64, total: u64 },
}

impl KneeRefusal {
    /// The loud one-line verdict. Rendered by the CLI lane on a refused
    /// measurement; the refusal vocabulary here is deliberate (the T1
    /// verdict law — a REFUSAL may say refuse; a REPORT may not).
    #[must_use]
    pub fn line(&self) -> String {
        match self {
            Self::Blind => {
                "multiplicity-gate: blind readout — zero units measured (never a green zero)"
                    .to_string()
            }
            Self::MalformedZero { units } => format!(
                "multiplicity-gate: malformed record — {units} unit(s) with zero encounters \
                 (an encounter ledger never records zero; the surface record is corrupt)"
            ),
            Self::OnePassSurface { units, total } => format!(
                "multiplicity-gate: fence holds — surface is ONE_PASS (units={units} total={total}, \
                 repeat < first): 1-pass surfaces (the chain/quorum shape) never arm; \
                 holdout machinery buys nothing there"
            ),
        }
    }
}

/// One unit's arming verdict under the measured gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consult {
    /// The unit's counter has passed the surface's knee — holdout-flavored
    /// machinery MAY arm on it.
    Armed,
    /// Below the knee (or the surface was refused) — the armed path must
    /// not fire; the consumer's behavior here is byte-identical to the
    /// unarmed posture.
    Unarmed,
}

/// The measured arming gate for one surface. Constructed ONLY through
/// [`MultiplicityGate::measure`] — there is no bypass constructor, so a
/// surface that failed the fence cannot yield a gate value at all.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiplicityGate {
    knee: u64,
    units: u64,
    total: u64,
    scale: f64,
}

impl MultiplicityGate {
    /// Measure the knee from the surface's per-unit encounter counts
    /// (one entry per DISTINCT unit, each ≥ 1). See the module docs for
    /// the four-step law. `O(max_m` · n) over the counts — fine for a
    /// report lane (n ~ 10³, `max_m` ~ 10²).
    ///
    /// # Panics
    ///
    /// Never: the only `expect` (over `iter().max()`) is guarded by the
    /// empty check above — recorded here because the lint counts the
    /// panicking site, and a silent "cannot panic" would be an
    /// unaudited claim.
    ///
    /// # Errors
    ///
    /// [`KneeRefusal::Blind`] on zero units, [`KneeRefusal::MalformedZero`]
    /// on any zero count, [`KneeRefusal::OnePassSurface`] when the surface
    /// fails reuse-dominance (the fence).
    pub fn measure(counts: &[u64]) -> Result<Self, KneeRefusal> {
        if counts.is_empty() {
            return Err(KneeRefusal::Blind);
        }
        let zero_units = counts.iter().filter(|&&c| c == 0).count();
        if zero_units > 0 {
            return Err(KneeRefusal::MalformedZero { units: zero_units });
        }
        let units = counts.len() as u64;
        // u64 fold: realistic report lanes (n ~ 10³–10⁴ units × u64
        // counters) stay far below 2^63, so the doubling below cannot
        // overflow.
        let total: u64 = counts.iter().sum();
        if total < 2 * units {
            return Err(KneeRefusal::OnePassSurface { units, total });
        }
        let max_m = *counts.iter().max().expect("non-empty, checked above");
        // The encounter-mass median: the largest m whose holders still
        // carry >= half the mass. m = 1 always qualifies (mass_above(1) =
        // total >= total/2), so `knee` always lands on a real multiplicity.
        let mut knee: u64 = 1;
        for m in 1..=max_m {
            let mass_above: u64 = counts.iter().map(|&c| if c >= m { c } else { 0 }).sum();
            if 2 * mass_above >= total {
                knee = m;
            }
        }
        Ok(Self {
            // knee ≤ max(counts), a u64 counter — the gate math widens to
            // f64; no narrowing anywhere.
            knee,
            units,
            total,
            scale: SIGMOID_SCALE_NATS,
        })
    }

    /// The measured knee (encounter-mass median; exact-half ties arm late).
    #[must_use]
    pub fn knee(&self) -> u64 {
        self.knee
    }

    /// Distinct units the surface recorded.
    #[must_use]
    pub fn units(&self) -> u64 {
        self.units
    }

    /// Total encounters across the surface.
    #[must_use]
    pub fn total_encounters(&self) -> u64 {
        self.total
    }

    /// The gate value `σ(λ·(m − knee))` over the unit's OWN counter.
    /// Sigmoid, never softmax: the value depends on (m, knee, λ) and
    /// nothing else — no other unit's counter can move it.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // surface counters are report-lane scale
    pub fn gate(&self, multiplicity: u64) -> f64 {
        let z = self.scale * (multiplicity as f64 - self.knee as f64);
        1.0 / (1.0 + (-z).exp())
    }

    /// The arming verdict: `m >= knee` — which IS `gate(m) >= 0.5` (the
    /// fixture pins the equivalence across a sweep; there is no second
    /// threshold). `gate(knee)` is exactly 0.5, so the knee itself arms.
    #[must_use]
    pub fn consult(&self, multiplicity: u64) -> Consult {
        if multiplicity >= self.knee {
            Consult::Armed
        } else {
            Consult::Unarmed
        }
    }

    /// The one-line per-unit consult render. Deterministic formatting
    /// (gate at 5 decimals; the bit-exact value is the fixture's business,
    /// not the wire's).
    #[must_use]
    pub fn consult_line(&self, surface: &str, unit: &str, multiplicity: u64) -> String {
        let verdict = match self.consult(multiplicity) {
            Consult::Armed => "ARMED",
            Consult::Unarmed => "UNARMED",
        };
        format!(
            "[multiplicity] surface={surface} unit={unit} multiplicity={multiplicity} \
             knee={} consult={verdict} gate={:.5}",
            self.knee,
            self.gate(multiplicity)
        )
    }

    /// The surface summary render (the `--multiplicity-report` body).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // the repeat share is a report figure
    pub fn render(&self, surface: &str) -> String {
        use std::fmt::Write as _;
        let repeat_share = (self.total - self.units) as f64 / self.total as f64 * 100.0;
        let mut out = String::new();
        let _ = write!(
            out,
            "multiplicity-gate readout (i232 T3 — report-only; L1's ordering law; \
             the knee is measured, never imported) v{READOUT_VERSION}\n\
             surface: {surface} (per-unit encounter multiplicity)\n\
             units: {} — encounters: {} (repeat share {repeat_share:.1}%)\n\
             class: REUSE_DOMINANT — knee measured {} (encounter-mass median; \
             exact-half ties arm late)\n\
             arming: consult=ARMED at multiplicity >= {} (gate = sigmoid({}·(m − knee)); \
             gate(knee) = {:.5})\n\
             fence: ONE_PASS surfaces (1-pass — the chain/quorum shape) never arm, \
             at any multiplicity\n",
            self.units,
            self.total,
            self.knee,
            self.knee,
            self.scale,
            self.gate(self.knee)
        );
        out
    }
}

#[cfg(feature = "latent_retrieval")]
mod store_surface {
    //! The real-surface feeding half: the fixer's per-rule outcome
    //! encounters in `.refine/trajectories.kv`, folded per `lint_key`.
    //! Composes `all(multiplicity_gate, latent_retrieval)` — the gate
    //! module itself stays std-only (the crate builds at every posture).

    use super::{KneeRefusal, MultiplicityGate};
    use std::collections::BTreeMap;
    use std::path::Path;

    /// The surface name the CLI lane renders.
    pub const SURFACE_NAME: &str = "traj-store";

    /// One store-surface measurement: the gate verdict + the count of
    /// rows excluded for having zero recorded outcomes.
    #[derive(Debug, Clone, PartialEq)]
    pub struct StoreSurfaceMeasure {
        /// [`Ok`] = measured gate; [`Err`] = the loud knee refusal (blind
        /// / malformed / the `OnePass` fence).
        pub verdict: Result<MultiplicityGate, KneeRefusal>,
        /// Rows whose success+fail counters sum to zero — the store's
        /// legitimate seed/pending shape (a row exists before any verdict
        /// lands). They are NOT encounter units (an encounter reads a
        /// verdict, not row existence), so the feeder excludes them and
        /// DISCLOSES the count — never a silent fold: the CLI prints one
        /// line naming this number before the render.
        pub excluded_zero_outcome_rows: usize,
    }

    /// Fold the store's trajectory rows into per-rule encounter counts
    /// (success + fail per `lint_key`, canonical key order via the
    /// `BTreeMap`), excluding zero-outcome rows with a disclosed count.
    /// Corrupt rows are refused outright — the knee reads counters, and a
    /// skipped row is an uncounted encounter, so a partial measure would
    /// be a confident wrong number.
    ///
    /// # Errors
    ///
    /// The store open failure, verbatim; the corrupt-row refusal.
    pub fn measure_store_surface(root: &Path) -> Result<StoreSurfaceMeasure, String> {
        let (store, bad) = crate::traj_store::TrajStore::open(root)?;
        if bad > 0 {
            return Err(format!(
                "multiplicity-gate: {bad} corrupt trajectory row(s) — \
                 the surface record is incomplete; refusing a partial measure"
            ));
        }
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        let mut excluded = 0usize;
        for row in store.dump() {
            let encounters = u64::from(row.success_count) + u64::from(row.fail_count);
            if encounters == 0 {
                excluded += 1;
                continue;
            }
            *counts.entry(row.lint_key).or_default() += encounters;
        }
        let verdict = if counts.is_empty() {
            Err(KneeRefusal::Blind)
        } else {
            MultiplicityGate::measure(&counts.into_values().collect::<Vec<u64>>())
        };
        Ok(StoreSurfaceMeasure {
            verdict,
            excluded_zero_outcome_rows: excluded,
        })
    }
}

#[cfg(feature = "latent_retrieval")]
pub use store_surface::{SURFACE_NAME, StoreSurfaceMeasure, measure_store_surface};

#[cfg(all(test, feature = "multiplicity_gate"))]
mod tests {
    use super::*;

    #[test]
    fn knee_lands_on_the_encounter_mass_median() {
        // [1,1,1,1,10]: the 10-count unit alone carries 10/14 ≥ half —
        // knee 10 (only it arms).
        let g = MultiplicityGate::measure(&[1, 1, 1, 1, 10]).expect("reuse-dominant");
        assert_eq!(g.knee(), 10);
        assert_eq!(g.units(), 5);
        assert_eq!(g.total_encounters(), 14);
        // All-uniform 2×: every unit is at the median — knee 2 arms
        // surface-wide (the T³ 4-pass shape's cousin).
        let g = MultiplicityGate::measure(&[2, 2, 2]).expect("reuse-dominant");
        assert_eq!(g.knee(), 2);
    }

    #[test]
    fn exact_half_ties_arm_late() {
        // [2,2,4]: total 8; mass_above(3) = 4 and mass_above(4) = 4 both
        // satisfy 2·mass >= 8 — the tie resolves to the LARGER m.
        let g = MultiplicityGate::measure(&[2, 2, 4]).expect("reuse-dominant");
        assert_eq!(g.knee(), 4);
        assert_eq!(g.consult(4), Consult::Armed);
        assert_eq!(g.consult(3), Consult::Unarmed);
    }

    #[test]
    fn knee_is_order_independent() {
        let a = MultiplicityGate::measure(&[6, 2, 1, 1]).expect("reuse-dominant");
        let b = MultiplicityGate::measure(&[1, 6, 1, 2]).expect("reuse-dominant");
        assert_eq!(a.knee(), 6);
        assert_eq!(a.knee(), b.knee());
    }

    #[test]
    fn gate_is_a_counter_sigmoid_never_softmax() {
        // Two surfaces with the SAME knee but different shapes: a unit at
        // multiplicity 7 reads the same gate bits under both — the gate
        // reads its own counter against the knee, never the surrounding
        // distribution (the softmax discriminator).
        let g1 = MultiplicityGate::measure(&[4, 4, 1, 1, 1]).expect("reuse-dominant");
        let g2 = MultiplicityGate::measure(&[4, 4, 1]).expect("reuse-dominant");
        assert_eq!(g1.knee(), 4);
        assert_eq!(g2.knee(), 4);
        assert_eq!(g1.gate(7).to_bits(), g2.gate(7).to_bits());
        // Monotone non-decreasing in the counter.
        let mut prev = 0.0;
        for m in 0..=100u64 {
            let v = g1.gate(m);
            assert!(v >= prev, "gate must be monotone at m={m}");
            prev = v;
        }
    }

    #[test]
    fn armed_is_the_sigmoids_own_half() {
        let g = MultiplicityGate::measure(&[6, 2, 1, 1]).expect("reuse-dominant");
        let k = g.knee();
        assert_eq!(k, 6);
        for m in 0..=(k + 3) {
            let gate = g.gate(m);
            let armed = g.consult(m) == Consult::Armed;
            assert_eq!(armed, gate >= 0.5, "armed ⟺ gate ≥ 0.5 at m={m}");
        }
        // The knee itself is exactly 0.5.
        assert_eq!(g.gate(k).to_bits(), 0.5f64.to_bits());
    }

    #[test]
    fn below_knee_is_unarmed_with_clean_vocabulary() {
        let g = MultiplicityGate::measure(&[6, 2, 1, 1]).expect("reuse-dominant");
        for m in 0..g.knee() {
            assert_eq!(g.consult(m), Consult::Unarmed);
            let line = g.consult_line("s", "u", m);
            assert!(
                !line.contains("consult=ARMED"),
                "an unarmed consult must never carry the armed vocabulary: {line}"
            );
            assert!(line.contains("consult=UNARMED"));
        }
        let armed_line = g.consult_line("s", "u", g.knee());
        assert!(armed_line.contains("consult=ARMED"));
    }

    #[test]
    fn refusals_are_loud_and_discriminated() {
        assert_eq!(MultiplicityGate::measure(&[]), Err(KneeRefusal::Blind));
        assert_eq!(
            MultiplicityGate::measure(&[3, 0, 5]),
            Err(KneeRefusal::MalformedZero { units: 1 })
        );
        // The fence: 1-pass shapes refuse at any size.
        let big_one_pass: Vec<u64> = vec![1; 500];
        assert_eq!(
            MultiplicityGate::measure(&big_one_pass),
            Err(KneeRefusal::OnePassSurface {
                units: 500,
                total: 500
            })
        );
        assert_eq!(
            MultiplicityGate::measure(&[1, 1]),
            Err(KneeRefusal::OnePassSurface { units: 2, total: 2 })
        );
        // Near-boundary mixed shape: repeats < first still fences.
        assert_eq!(
            MultiplicityGate::measure(&[1, 1, 1, 2]),
            Err(KneeRefusal::OnePassSurface { units: 4, total: 5 })
        );
        // Each refusal renders its own loud line.
        for r in [
            KneeRefusal::Blind,
            KneeRefusal::MalformedZero { units: 2 },
            KneeRefusal::OnePassSurface { units: 9, total: 9 },
        ] {
            assert!(!r.line().is_empty());
        }
    }

    #[cfg(feature = "latent_retrieval")]
    #[test]
    fn store_surface_measures_the_fixer_encounters() {
        let root = std::env::temp_dir().join(format!("mult_gate_g1_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp root");
        // Fresh store, no rows: blind, never a green zero.
        let res = measure_store_surface(&root).expect("store opens");
        assert_eq!(res.verdict, Err(KneeRefusal::Blind));
        assert_eq!(res.excluded_zero_outcome_rows, 0);
        // Seed [6,2,1,1] via outcomes: rule_a ×6, rule_b ×2, rule_c ×1,
        // rule_d ×1 (total 10, units 4 — reuse-dominant; knee 6).
        let (store, bad) = crate::traj_store::TrajStore::open(&root).expect("open store");
        assert_eq!(bad, 0);
        let plan: [(&str, usize); 4] = [("rule_a", 6), ("rule_b", 2), ("rule_c", 1), ("rule_d", 1)];
        for (i, (rule, n)) in plan.iter().enumerate() {
            for k in 0..*n {
                let win = (k + i) % 2 == 0;
                store
                    .record(rule, &format!("shape_{i}"), "fix()", win, win, "g1-seed")
                    .expect("record");
            }
        }
        drop(store);
        let res = measure_store_surface(&root).expect("store opens");
        let g = res.verdict.expect("reuse-dominant");
        assert_eq!(res.excluded_zero_outcome_rows, 0);
        assert_eq!(g.knee(), 6);
        let text = g.render(SURFACE_NAME);
        assert!(text.contains("REUSE_DOMINANT"));
        assert!(text.contains("knee measured 6"));
        assert!(!text.to_ascii_lowercase().contains("refus"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
