//! Issue 048 T2 — GP posterior-variance candidate ordering for the
//! [`CompileDriver::with_ordering`](super::driver::CompileDriver::with_ordering)
//! seam.
//!
//! The incumbent `order_candidates` is a static min-delta heuristic (stable
//! file/line/code key, shortest total replacement first). This module builds
//! the alternative the seam was shipped for: order candidates by GP σ² of
//! their trigger-`shape` embeddings — **high-uncertainty first** (`ActFlow`'s
//! acquisition signal: query the input the observed set explains least).
//!
//! Deterministic: σ² is a pure function of the observed set + the shapes;
//! exact ties fall through to the incumbent min-delta key, so two candidates
//! with identical σ² keep the stable production order.
//!
//! **Honest scope** (the T5 pre-registration): σ² over the local BLAKE3
//! 8-D embedder is dominated by ‖x‖² norm concentration — near-flat across
//! distinct shapes. The A/B gate measures whether this ordering beats
//! min-delta on the generated corpus; a null result keeps the seam + this
//! constructor shipped but unused by default (the `t48` verdict records it).

use crate::embedder::{EMBED_DIM, embed_into};
use crate::gp_acquisition::{DIM, GpAcquisition};

use super::driver::OrderingFn;
use super::playbooks::FixCandidate;

/// Build a GP-variance `OrderingFn`.
///
/// `observed` seeds the model with the trigger shapes already queried
/// (e.g. prior iteration shapes from the same run, or replayed shapes from
/// the trajectory store — embedded under their own error code as the key).
/// Each ordering call embeds every candidate's `shape` under its `code`
/// (the error code is the embedder's namespace key) and sorts by σ²
/// descending; exact σ² ties fall through to the incumbent stable key.
///
/// O(n²) per candidate against the observed set (n = `observed.len()` +
/// nothing — the model is NOT grown by ordering, only by explicit
/// [`GpAcquisition::observe`] calls; see [`gp_ordering_growing`] for the
/// growing variant).
#[must_use]
pub fn gp_ordering(observed: &[(String, String)]) -> OrderingFn {
    let mut gp = GpAcquisition::with_capacity(crate::gp_acquisition::DEFAULT_LAMBDA, observed.len());
    let mut buf = [0.0f32; EMBED_DIM];
    for (code, shape) in observed {
        embed_into(code, shape, &mut buf);
        gp.observe(&buf);
    }
    let gp = std::sync::Arc::new(gp);
    std::sync::Arc::new(move |cands: &mut [FixCandidate]| {
        let gp = gp.clone();
        // score each candidate; σ² ties resolved by the incumbent key.
        cands.sort_by(|a, b| {
            let sa = sigma_of(&gp, a);
            let sb = sigma_of(&gp, b);
            sb.total_cmp(&sa)
                .then_with(|| incumbent_key(a).cmp(&incumbent_key(b)))
        });
    })
}

/// The growing variant: each ordering call ALSO observes every candidate's
/// shape (so later iterations see earlier ones), approximating the
/// within-run acquisition loop without needing driver callbacks.
#[must_use]
pub fn gp_ordering_growing(observed: &[(String, String)]) -> OrderingFn {
    let mut gp = GpAcquisition::with_capacity(crate::gp_acquisition::DEFAULT_LAMBDA, observed.len());
    let mut buf = [0.0f32; EMBED_DIM];
    for (code, shape) in observed {
        embed_into(code, shape, &mut buf);
        gp.observe(&buf);
    }
    let gp = std::sync::Arc::new(std::sync::Mutex::new(gp));
    std::sync::Arc::new(move |cands: &mut [FixCandidate]| {
        let mut gp = gp.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        // Stack buffer per call — `OrderingFn` is `Fn`, so the shared
        // buffer lives inside the call frame (zero heap, still alloc-free).
        let mut buf = [0.0f32; EMBED_DIM];
        // Observe first (this call's candidates join the observed set)...
        for c in cands.iter() {
            embed_into(&c.code, &c.shape, &mut buf);
            gp.observe(&buf);
        }
        // ...then order by σ² with the freshly-grown model. Note: a shape
        // just observed has collapsed σ², so within one call this ordering
        // is by RESIDUAL variance (norm + cross-terms), not by prior
        // novelty — the honest semantics of a batch acquisition step.
        cands.sort_by(|a, b| {
            let sa = sigma_of(&gp, a);
            let sb = sigma_of(&gp, b);
            sb.total_cmp(&sa)
                .then_with(|| incumbent_key(a).cmp(&incumbent_key(b)))
        });
    })
}

fn sigma_of(gp: &GpAcquisition, c: &FixCandidate) -> f32 {
    let mut buf = [0.0f32; EMBED_DIM];
    embed_into(&c.code, &c.shape, &mut buf);
    gp.sigma_sq(&buf)
}

/// The incumbent `order_candidates` key (stable, total) — the tie-break so
/// identical σ² never shuffles relative order.
fn incumbent_key(c: &FixCandidate) -> (String, usize, String, usize, String) {
    (
        c.file.to_string_lossy().into_owned(),
        c.line,
        c.code.clone(),
        c.total_len(),
        c.strategy_id.clone(),
    )
}

const _: () = assert!(DIM == EMBED_DIM, "gp_acquisition + embedder dims must agree");

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cand(code: &str, shape: &str, line: usize, strat: &str) -> FixCandidate {
        FixCandidate {
            strategy_id: strat.to_string(),
            code: code.to_string(),
            file: PathBuf::from("/tmp/x.rs"),
            line,
            byte_span: (0, shape.len()),
            replacement: shape.to_string(),
            shape: shape.to_string(),
            semantic_note: None,
            extra_edits: Vec::new(),
        }
    }

    #[test]
    fn orders_high_sigma_first_with_stable_ties() {
        // Two observed shapes + three candidates: one far (high σ²), one
        // exact-match observed (low σ²), one duplicate of the first.
        let observed = vec![
            ("E0502".to_string(), "let a = b.clone();".to_string()),
            ("E0502".to_string(), "let c = d.clone();".to_string()),
        ];
        let f = gp_ordering(&observed);
        let mut cands = vec![
            cand("E0502", "let a = b.clone();", 10, "s1"), // observed → low σ²
            cand("E0502", "fn totally_different_shape() -> X {", 20, "s2"), // novel
            cand("E0502", "fn totally_different_shape() -> X {", 20, "s3"), // tie w/ s2
        ];
        f(&mut cands);
        // Novel shapes first (σ² ties between s2/s3 resolved by incumbent
        // key: strategy_id s2 < s3), observed last.
        assert_eq!(cands[0].strategy_id, "s2");
        assert_eq!(cands[1].strategy_id, "s3");
        assert_eq!(cands[2].strategy_id, "s1");
    }

    #[test]
    fn empty_observed_still_orders_deterministically() {
        let f = gp_ordering(&[]);
        let mut a = vec![
            cand("E0499", "shape one", 1, "x"),
            cand("E0499", "shape two", 2, "y"),
        ];
        let mut b = a.clone();
        f(&mut a);
        f(&mut b);
        let ids: Vec<_> = a.iter().map(|c| c.strategy_id.clone()).collect();
        let ids_b: Vec<_> = b.iter().map(|c| c.strategy_id.clone()).collect();
        assert_eq!(ids, ids_b, "deterministic");
        // With no observations σ² = ‖x‖² (norms differ by content length)
        // — ordering exists but is semantic-free; just pin determinism.
    }

    #[test]
    fn growing_variant_never_panics_and_orders() {
        let f = gp_ordering_growing(&[]);
        let mut cands = vec![
            cand("E0505", "let x = self.f();", 5, "m1"),
            cand("E0505", "let y = self.g();", 6, "m2"),
            cand("E0505", "let x = self.f();", 5, "m1b"),
        ];
        f(&mut cands);
        assert_eq!(cands.len(), 3);
        // After observing all three, near-duplicates share σ²; ties resolve
        // by incumbent key → m1 before m1b.
        let pos = |id: &str| cands.iter().position(|c| c.strategy_id == id).unwrap();
        assert!(pos("m1") < pos("m1b"));
    }
}
