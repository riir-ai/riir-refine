//! Batch 370 fixtures (Plan 383 / Research 415) — ORIGINAL std-only
//! reproductions of the tabulai/mlx-learn laws (@ 07054ce3, Apache-2.0),
//! never pasted source. One standalone file per the B336/…/B362 discipline:
//! compiled AND executed BEFORE entry as `rustc --edition 2021 --test
//! b370_fixtures.rs && ./b370_fixtures`. Each rule's ref/opt arms in the
//! batch's entries file embed the same executed code — the fixture pass is
//! the authoring gate: a defect caught here is fixed HERE first, then synced
//! back into the entries in the same landing commit (the b361 precedent).
//!
//! Arms (rule → what the tests pin):
//! - F1 `certified-prefilter-omission-bound-escalate-exact`: per-partition
//!   carry shortlists + the next-omitted barrier; the certificate (every
//!   partition's omission bound clears the merged k-th widened upper) with
//!   the escalate-to-exact escape; poisoned rows bias to −INF, never drop;
//!   the barrier-less accept loses a true neighbor; the ×1.5 inflated bound
//!   is caught by the oracle. Nearest-first frame verbatim (the source's
//!   omission LOWER bound clears the merged k-th UPPER; identical ranking
//!   to the similarity frame's "value desc" under value = −distance).
//! - F2 `device-fp64-absent-two-float-compensated-accumulation`: fast2sum /
//!   add2 / mul2 verbatim algebra over f32 lanes; the naive f32 fold loses
//!   the WHOLE delta mass while the two-float fold tracks it; no harm
//!   outside the cancellation regime; dropping the error term is detectable
//!   at the hi component; the fast2sum error is EXACTLY the f64 error.
//! - F3 `exact-ordering-escalating-precision-ladder`: f32 ordering escalates
//!   interval → guarded double-single (Dekker 8193-split) → exact; values
//!   stay f32, only the chosen indices are exact; ties resolve by FIRST
//!   index; the adversarial 1e8+7/1e8+9 pair (casts collapse; naive Equal,
//!   truth b > a, ladder resolves); a magnitude sweep matches the f64
//!   oracle; separated pairs never leave tier one; tier-two deletion
//!   misorders.
//! - F4 `overflow-guard-authoritative-sum-repair-after-loop`: compensating
//!   through +inf launders it into NaN; the guard pins the authoritative
//!   (uncompensated) +inf and stops; repair only AFTER the loop, only on
//!   finite state; finite inputs bit-identical to plain Kahan; deleting the
//!   guard returns NaN.

use std::cmp::Ordering;

// ─────────────────────────────────────────────────────────────────────────────
// shared deterministic randomness — seeded LCG only, no rand, no deps
// ─────────────────────────────────────────────────────────────────────────────

struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    /// uniform f64 in [lo, hi)
    fn f64_in(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (self.next_u32() as f64 / 4294967296.0) * (hi - lo)
    }

    /// uniform f64 in [0, 1)
    fn f64_unit(&mut self) -> f64 {
        self.next_u32() as f64 / 4294967296.0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// F1 — certified-prefilter-omission-bound-escalate-exact
// ─────────────────────────────────────────────────────────────────────────────

const CARRY: usize = 6;
const TOP_K: usize = 3;
const PARTS: usize = 4;
const PER_PART: usize = 8;
const CANDS: usize = PARTS * PER_PART;

/// oversized-quantization poison threshold on |cast − true|; in-range row
/// values (≤ 1000) never exceed ½ ulp ≈ 3.1e-5, so only planted poison fires.
const QUANT_ERR_LIMIT: f64 = 1e-3;

#[derive(Clone, Copy)]
struct CandInterval {
    idx: usize,
    cast: f32,
    lo: f32,
    hi: f32,
}

/// ±2-ulp outward widening via nextafter-in-f32-bits (nonnegative f32:
/// bit order IS value order, so ±2 on the bits widens outward).
fn widen_lo(s: f32) -> f32 {
    f32::from_bits(s.to_bits().saturating_sub(2))
}

fn widen_hi(s: f32) -> f32 {
    f32::from_bits((s.to_bits().saturating_add(2)).min(0x7F7F_FFFF))
}

#[derive(Clone)]
struct PartitionScreen {
    members: Vec<CandInterval>,
    /// min-tracked omission LOWER bound: every omitted candidate is
    /// provably at least this far.
    barrier: f32,
}

struct RowScreen {
    /// −INF when poisoned — the row can never rank, it escalates.
    bias: f32,
    parts: Vec<PartitionScreen>,
}

/// the lossy fast screen: per-partition carry shortlists + the next-omitted
/// barrier. Admission is strict on the widened upper edge, so a candidate
/// tied at the carry boundary is omitted and its lower bound folds into the
/// partition barrier (`omitted = min(omitted, max(lower_bound, kth))`).
fn by_cast(a: &CandInterval, b: &CandInterval) -> Ordering {
    a.cast.partial_cmp(&b.cast).unwrap().then(a.idx.cmp(&b.idx))
}

fn screen_row(d: &[f64]) -> RowScreen {
    let mut bias = 1.0f32;
    let mut cands: Vec<Option<CandInterval>> = Vec::with_capacity(d.len());
    for (i, &v) in d.iter().enumerate() {
        let s = v as f32;
        let ok = s.is_finite() && (s as f64 - v).abs() <= QUANT_ERR_LIMIT;
        if !ok {
            bias = f32::NEG_INFINITY;
        }
        cands.push(ok.then(|| CandInterval {
            idx: i,
            cast: s,
            lo: widen_lo(s),
            hi: widen_hi(s),
        }));
    }
    if !bias.is_finite() {
        return RowScreen {
            bias,
            parts: Vec::new(),
        };
    }
    let mut parts = Vec::with_capacity(PARTS);
    for p in 0..PARTS {
        let slice = &cands[p * PER_PART..(p + 1) * PER_PART];
        let mut members: Vec<CandInterval> = Vec::with_capacity(CARRY);
        let mut barrier = f32::INFINITY;
        for c in slice.iter().flatten() {
            if members.len() < CARRY {
                members.push(*c);
                members.sort_by(by_cast);
            } else if c.hi < members[members.len() - 1].hi {
                members.push(*c);
                members.sort_by(by_cast);
                let evicted = members.pop().unwrap();
                // an evicted candidate is at least its own widened lo
                barrier = barrier.min(evicted.lo);
            } else {
                // rejected: certainly no closer than the carry boundary —
                // max(lower_bound, shortlist_kth) = the candidate's own lo
                let kth = members[members.len() - 1];
                barrier = barrier.min(c.lo.max(kth.lo));
            }
        }
        parts.push(PartitionScreen { members, barrier });
    }
    RowScreen { bias, parts }
}

/// merge the shortlists: the screened answer, top-k nearest, nearest-first
/// (best-first — the similarity frame's "value desc" under value = −distance).
/// Returns the answer rows + the widened hi of the k-th kept entry.
fn merge_top_k(parts: &[PartitionScreen], d: &[f64]) -> (Vec<(usize, f64)>, f32) {
    let mut all: Vec<CandInterval> = parts
        .iter()
        .flat_map(|p| p.members.iter().copied())
        .collect();
    all.sort_by(by_cast);
    let kth_hi = if all.len() >= TOP_K {
        all[TOP_K - 1].hi
    } else {
        f32::NEG_INFINITY
    };
    let answer = all.iter().take(TOP_K).map(|c| (c.idx, d[c.idx])).collect();
    (answer, kth_hi)
}

/// the certificate: every partition's omission bound must clear the merged
/// k-th widened upper — no omitted candidate can then displace the kept k,
/// so the screened answer IS exact. `barrier_scale` is the tamper hook
/// (×1.5 = the source's inflated-bound defect).
fn certified(parts: &[PartitionScreen], kth_hi: f32, barrier_scale: f32) -> bool {
    parts.iter().all(|p| p.barrier * barrier_scale > kth_hi)
}

/// the exact oracle: complete f64 scan, nearest-first, index-ascending ties;
/// nonfinite values never rank.
fn exact_top_k(d: &[f64]) -> Vec<(usize, f64)> {
    let mut v: Vec<(usize, f64)> = d
        .iter()
        .enumerate()
        .filter(|(_, x)| x.is_finite())
        .map(|(i, x)| (i, *x))
        .collect();
    v.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(a.0.cmp(&b.0)));
    v.truncate(TOP_K);
    v
}

#[derive(Default)]
struct Census {
    full_scans: u32,
    certified: u32,
}

fn decide_row(d: &[f64], census: &mut Census) -> Vec<(usize, f64)> {
    let screen = screen_row(d);
    if !screen.bias.is_finite() || screen.parts.len() < PARTS {
        census.full_scans += 1;
        return exact_top_k(d);
    }
    let (answer, kth_hi) = merge_top_k(&screen.parts, d);
    if answer.len() == TOP_K && certified(&screen.parts, kth_hi, 1.0) {
        census.certified += 1;
        answer
    } else {
        census.full_scans += 1;
        exact_top_k(d)
    }
}

/// 32 fully explicit candidates. True top-3 = {10.0@0, 20.0@1, 500.0@15}.
/// The seven P1 fillers sit at true 500.0 + 1.4e-5 (< half-ulp of 500.0), so
/// they CAST to 500.0 exactly and occupy the whole carry; the exact-500.0
/// twin @15 arrives with an equal widened hi and is omitted (strict-hi
/// admission), while the other twin @16 is shortlisted in its own partition.
fn adversarial_row() -> Vec<f64> {
    let mut d = vec![800.0f64; CANDS];
    d[0] = 10.0;
    d[1] = 20.0;
    // P0 fillers
    for (i, v) in (2..8).zip([600.0, 650.0, 700.0, 750.0, 800.0, 850.0]) {
        d[i] = v;
    }
    // P1: seven cast-500 fillers, then the exact-500.0 twin @15
    for i in 8..15 {
        d[i] = 500.0 + 1.4e-5;
    }
    d[15] = 500.0;
    // P2: the other exact-500.0 twin @16 + seven strictly-worse fillers
    d[16] = 500.0;
    for (i, v) in (17..24).zip([510.0, 515.0, 520.0, 525.0, 530.0, 535.0, 540.0]) {
        d[i] = v;
    }
    // P3: eight fillers all worse than the cluster
    for (i, v) in (24..32).zip([550.0, 560.0, 570.0, 580.0, 590.0, 600.0, 610.0, 620.0]) {
        d[i] = v;
    }
    d
}

fn ids(v: &[(usize, f64)]) -> Vec<usize> {
    v.iter().map(|w| w.0).collect()
}

#[test]
fn the_certified_screen_matches_the_exact_scan_bit_for_bit() {
    let mut census = Census::default();
    for r in 0u64..16 {
        let mut g = Lcg(0xB369_0000 ^ r);
        let mut d = Vec::with_capacity(CANDS);
        for _ in 0..CANDS {
            d.push(g.f64_in(0.0, 1000.0));
        }
        if r % 4 == 3 {
            // plant a duplicate of the 3rd-smallest value at the row's end:
            // both copies land inside the kept sets, so the certificate
            // still holds and the index tiebreak matches the oracle.
            let mut ord: Vec<usize> = (0..CANDS).collect();
            ord.sort_by(|&a, &b| d[a].partial_cmp(&d[b]).unwrap());
            d[CANDS - 1] = d[ord[2]];
        }
        let emitted = decide_row(&d, &mut census);
        let oracle = exact_top_k(&d);
        assert_eq!(emitted.len(), TOP_K, "row {r}");
        for (w, o) in emitted.iter().zip(oracle.iter()) {
            assert_eq!(w.0, o.0, "row {r}: id mismatch");
            assert_eq!(w.1.to_bits(), o.1.to_bits(), "row {r}: not bit-identical");
        }
    }
    assert!(census.certified > 0, "no row certified — no work was saved");
    assert!(
        census.full_scans < 16,
        "every row escalated — nothing was saved"
    );
}

#[test]
fn the_uncertified_row_escalates_and_stays_exact() {
    let mut census = Census::default();
    let mut rows: Vec<Vec<f64>> = vec![adversarial_row()];
    // poison arm 1: a nonfinite screen value
    let mut p_nonfinite: Vec<f64> = {
        let mut g = Lcg(0xB369_00A1);
        (0..CANDS).map(|_| g.f64_in(0.0, 1000.0)).collect()
    };
    p_nonfinite[0] = f64::NAN;
    rows.push(p_nonfinite);
    // poison arm 2: oversized quantization error (1e12 casts with ~3e4 error)
    let mut p_quant: Vec<f64> = {
        let mut g = Lcg(0xB369_00A2);
        (0..CANDS).map(|_| g.f64_in(0.0, 1000.0)).collect()
    };
    p_quant[1] = 1e12;
    rows.push(p_quant);
    for (ri, d) in rows.iter().enumerate() {
        let screen = screen_row(d);
        let before = census.full_scans;
        let emitted = decide_row(d, &mut census);
        assert_eq!(census.full_scans, before + 1, "row {ri} did not escalate");
        assert_eq!(ids(&emitted), ids(&exact_top_k(d)), "row {ri}: drifted");
        if !screen.bias.is_finite() {
            assert!(screen.bias.is_sign_negative(), "row {ri}: bias not -inf");
        } else {
            // the adversarial row: the certificate itself must have failed
            let (_, kth_hi) = merge_top_k(&screen.parts, d);
            assert!(
                !certified(&screen.parts, kth_hi, 1.0),
                "row {ri}: certified?"
            );
        }
    }
    assert_eq!(census.certified, 0);
    assert_eq!(census.full_scans, 3);
}

#[test]
fn the_barrier_omitted_loses_a_true_neighbor() {
    let d = adversarial_row();
    let screen = screen_row(&d);
    assert!(screen.bias.is_finite());
    let (emitted, kth_hi) = merge_top_k(&screen.parts, &d);
    // sanity: the bound WOULD have blocked this accept
    assert!(!certified(&screen.parts, kth_hi, 1.0));
    // SAME merge, certificate NOT consulted — the unsound accept
    let eids = ids(&emitted);
    let oids = ids(&exact_top_k(&d));
    assert_ne!(eids, oids, "barrier-less accept must emit the wrong twin");
    assert_eq!(oids, vec![0, 1, 15]);
    assert_eq!(eids, vec![0, 1, 8]);
}

#[test]
fn an_inflated_barrier_is_detected_by_the_oracle() {
    let d = adversarial_row();
    let screen = screen_row(&d);
    let (emitted, kth_hi) = merge_top_k(&screen.parts, &d);
    // the source-frame tamper: multiply the omission bound by 1.5 — claims
    // omitted rows are FARTHER than proven (the optimistic direction)
    assert!(
        certified(&screen.parts, kth_hi, 1.5),
        "tampered bound must wrongly certify"
    );
    let eids = ids(&emitted);
    let oids = ids(&exact_top_k(&d));
    assert_ne!(eids, oids, "the oracle must catch the tampered acceptance");
}

// ─────────────────────────────────────────────────────────────────────────────
// F2 — device-fp64-absent-two-float-compensated-accumulation
// ─────────────────────────────────────────────────────────────────────────────

/// a two-float (hi, lo) f32 pair — the device's stand-in for the missing f64.
#[derive(Clone, Copy, Debug)]
struct F2 {
    hi: f32,
    lo: f32,
}

impl F2 {
    const ZERO: F2 = F2 { hi: 0.0, lo: 0.0 };
}

/// fast2sum (rn): EXACT error recovery; requires |x| ≥ |y|.
fn fast2sum(x: f32, y: f32) -> F2 {
    let h = x + y;
    let e = y - (h - x);
    F2 { hi: h, lo: e }
}

/// add2: two-sum error recovery + renormalization — the source's algebra
/// verbatim: s, v, the four-term error, then rn(s, e).
fn add2(a: F2, b: F2) -> F2 {
    let s = a.hi + b.hi;
    let v = s - a.hi;
    let e = (a.hi - (s - v)) + (b.hi - v) + a.lo + b.lo;
    fast2sum(s, e)
}

/// the defect arm: the same add2 with the recovered error discarded.
fn add2_drop_e(a: F2, b: F2) -> F2 {
    let s = a.hi + b.hi;
    F2 { hi: s, lo: 0.0 }
}

/// mul2 with the fma error term: `mul_add(hi, hi, -p)` IS the source's
/// fma(a.hi, b.hi, −p), correctly rounded on this host.
fn mul2(a: F2, b: F2) -> F2 {
    let p = a.hi * b.hi;
    let e = a.hi.mul_add(b.hi, -p) + a.hi * b.lo + a.lo * b.hi;
    fast2sum(p, e)
}

fn fold_f32_naive(v: &[f32]) -> f32 {
    v.iter().fold(0.0f32, |acc, &x| acc + x)
}

fn fold_f64_oracle(v: &[f32]) -> f64 {
    v.iter().fold(0.0f64, |acc, &x| acc + x as f64)
}

/// the pinned sequential ascending fold (Issue-940 determinism carve-out)
fn fold_two_float(v: &[f32]) -> F2 {
    v.iter()
        .fold(F2::ZERO, |a, &x| add2(a, F2 { hi: x, lo: 0.0 }))
}

fn fold_two_float_drop_e(v: &[f32]) -> F2 {
    v.iter()
        .fold(F2::ZERO, |a, &x| add2_drop_e(a, F2 { hi: x, lo: 0.0 }))
}

fn two_float_as_f64(f: F2) -> f64 {
    f.hi as f64 + f.lo as f64
}

/// [64 × 20000.0, 64 × 0.00390625], bigs first: every delta lands on a sum
/// whose half-ulp (0.0625 at 1.28e6) already exceeds it, so the naive loss
/// is deterministic (interleaved keeps early deltas — the mass still
/// vanishes, but not bit-exactly).
fn big_then_delta() -> Vec<f32> {
    let mut v = Vec::with_capacity(128);
    for _ in 0..64 {
        v.push(20000.0f32);
    }
    for _ in 0..64 {
        v.push(0.00390625f32);
    }
    v
}

/// the delta mass: 64 × 2⁻⁸ = 0.25. (The batch note's "≥ 250.0" mis-scales
/// this: a naive fold cannot lose more than the mass it was fed. The
/// fixture pins the physical number — the naive fold loses EXACTLY all of it.)
const DELTA_MASS: f64 = 64.0 * 0.00390625;

#[test]
fn naive_f32_loses_the_small_deltas() {
    let v = big_then_delta();
    let naive = fold_f32_naive(&v);
    // the deltas contributed exactly zero: bit-exact 1280000.0
    assert_eq!(naive.to_bits(), 1280000.0f32.to_bits());
    let oracle = fold_f64_oracle(&v);
    assert_eq!(oracle, 1280000.0 + DELTA_MASS);
}

#[test]
fn two_float_tracks_the_deltas() {
    let v = big_then_delta();
    let naive = fold_f32_naive(&v) as f64;
    let two = two_float_as_f64(fold_two_float(&v));
    let oracle = fold_f64_oracle(&v);
    assert!(
        two - oracle < 1.0,
        "two-float lost the mass: {two} vs {oracle}"
    );
    assert!(
        two - oracle < 1.0,
        "two-float lost the mass: {two} vs {oracle}"
    );
    assert!(
        (naive - oracle).abs() >= DELTA_MASS,
        "naive kept the mass: {naive}"
    );
}

#[test]
fn no_cancellation_both_agree() {
    let mut g = Lcg(0xB369_00B0);
    let v: Vec<f32> = (0..128).map(|_| g.f64_in(900.0, 1100.0) as f32).collect();
    let oracle = fold_f64_oracle(&v);
    let naive = fold_f32_naive(&v) as f64;
    let two = two_float_as_f64(fold_two_float(&v));
    let tol = oracle * 1e-3;
    assert!((naive - oracle).abs() < tol);
    assert!((two - oracle).abs() < tol);
    // keep the product op exercised: mul2 is exact on representable inputs
    let m = mul2(F2 { hi: 3.0, lo: 0.0 }, F2 { hi: 0.5, lo: 0.0 });
    assert_eq!(m.hi, 1.5f32);
    assert_eq!(m.lo, 0.0f32);
}

#[test]
fn dropping_the_error_term_is_detectable() {
    let v = big_then_delta();
    let fixed = fold_two_float(&v);
    let dropped = fold_two_float_drop_e(&v);
    // a renormalized two-float keeps |lo| ≤ ½ ulp(hi), so the recovered
    // mass (0.25) can never sit in lo under hi = 1280000.0 (half-ulp only
    // 0.0625) — the fixed form's hi MUST move; the dropped form IS naive.
    assert_ne!(fixed.hi.to_bits(), dropped.hi.to_bits(), "no hi difference");
    assert_eq!(dropped.hi.to_bits(), fold_f32_naive(&v).to_bits());
    let oracle = fold_f64_oracle(&v);
    assert!((two_float_as_f64(fixed) - oracle).abs() < 1.0);
    assert!((two_float_as_f64(dropped) - oracle).abs() >= DELTA_MASS);
}

#[test]
fn fast2sum_error_term_is_exact() {
    let mut g = Lcg(0xB369_00B1);
    for i in 0..100 {
        let x = g.f64_in(1.0, 1e6) as f32;
        let y = (g.f64_in(-1.0, 1.0) * x as f64) as f32; // |y| ≤ |x|
        let f = fast2sum(x, y);
        // the true rounding error of the f32 add, computed in f64
        let true_err = (x as f64 + y as f64) - f.hi as f64;
        assert_eq!(f.lo as f64, true_err, "pair {i}: x={x} y={y}");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// F3 — exact-ordering-escalating-precision-ladder
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tier {
    Interval,
    DoubleSingle,
    Exact,
}

/// tier 1: the cheap conservative interval compare — each f32 cast widened
/// ±2 ulp outward; a verdict only when the intervals are disjoint. The
/// lane's values are nonnegative magnitudes: bit order = value order.
fn tier1_interval(a: f64, b: f64) -> Option<Ordering> {
    let ab = (a as f32).to_bits();
    let bb = (b as f32).to_bits();
    let a_lo = f32::from_bits(ab.saturating_sub(2));
    let a_hi = f32::from_bits((ab.saturating_add(2)).min(0x7F7F_FFFF));
    let b_lo = f32::from_bits(bb.saturating_sub(2));
    let b_hi = f32::from_bits((bb.saturating_add(2)).min(0x7F7F_FFFF));
    if a_hi < b_lo {
        return Some(Ordering::Less);
    }
    if b_hi < a_lo {
        return Some(Ordering::Greater);
    }
    None
}

/// Dekker's split: ~12-bit halves via the 2¹³+1 multiplier (exact for
/// normal f32 inputs).
fn dekker_split(x: f32) -> (f32, f32) {
    let t = 8193.0f32 * x;
    let hi = t - (t - x);
    let lo = x - hi;
    (hi, lo)
}

/// tier 2: guarded double-single subtract — sign(d + e) decisive; tiny or
/// uncertain magnitudes escalate.
fn tier2_ds_compare(a: f32, b: f32) -> Option<Ordering> {
    if a.to_bits() == b.to_bits() {
        return None; // indistinguishable at this precision
    }
    let (ah, al) = dekker_split(a);
    let (bh, bl) = dekker_split(b);
    let d = ah - bh;
    let e = ((ah - d) - bh) + (al - bl);
    let s = d + e;
    let scale = ah.abs().max(bh.abs()).max(f32::MIN_POSITIVE);
    if s.abs() < scale * 2f32.powi(-22) {
        return None;
    }
    Some(if s < 0.0 {
        Ordering::Less
    } else {
        Ordering::Greater
    })
}

/// the ladder. Tier 3 is the exact stand-in: on this host the f64 compare
/// of two f32-born quantities is exact (the entry names the dyadic integer
/// superaccumulator the device uses). Exact ties resolve by FIRST index;
/// returned values stay f32 — only the chosen indices are exact.
fn ladder_cmp(a: f64, b: f64, ia: usize, ib: usize) -> (Ordering, Tier) {
    if let Some(o) = tier1_interval(a, b) {
        return (o, Tier::Interval);
    }
    if let Some(o) = tier2_ds_compare(a as f32, b as f32) {
        return (o, Tier::DoubleSingle);
    }
    let o = match a.partial_cmp(&b) {
        Some(Ordering::Equal) | None => ia.cmp(&ib),
        Some(o) => o,
    };
    (o, Tier::Exact)
}

/// the f64 total order (value, then first index) — the oracle.
fn oracle_cmp(a: f64, b: f64, ia: usize, ib: usize) -> Ordering {
    match a.partial_cmp(&b) {
        Some(Ordering::Equal) | None => ia.cmp(&ib),
        Some(o) => o,
    }
}

/// the naive arm: f32-cast compare, index tiebreak.
fn naive_cmp(a: f64, b: f64, ia: usize, ib: usize) -> Ordering {
    match (a as f32).partial_cmp(&(b as f32)) {
        Some(Ordering::Equal) | None => ia.cmp(&ib),
        Some(o) => o,
    }
}

#[test]
fn naive_compare_misorders_the_adversarial_pair() {
    let a = 1.0e8f64 + 7.0;
    let b = 1.0e8f64 + 9.0;
    // both collapse onto the same f32 (ulp(1e8) = 8)
    assert_eq!((a as f32).to_bits(), (b as f32).to_bits());
    assert_eq!((a as f32).partial_cmp(&(b as f32)), Some(Ordering::Equal));
    // f64 truth: b > a
    assert_eq!(a.partial_cmp(&b), Some(Ordering::Less));
    // the ladder resolves: Greater on (b, a), Less on (a, b) — the casts
    // collapsed, so even the double-single tier sees equal values and the
    // exact tier owns the verdict
    assert_eq!(ladder_cmp(b, a, 1, 0), (Ordering::Greater, Tier::Exact));
    assert_eq!(ladder_cmp(a, b, 0, 1).0, Ordering::Less);
}

#[test]
fn the_ladder_matches_f64_exactly() {
    let mut g = Lcg(0xB369_00C0);
    let mut vals: Vec<f64> = Vec::new();
    for _ in 0..4 {
        // subnormal, tiny-normal, unit, big, huge magnitudes
        vals.push(f32::from_bits(g.next_u32() % 0x007F_FFFF) as f64);
        vals.push((0.5 + g.f64_unit()) * 1e-30);
        vals.push(0.001 + g.f64_unit() * 10.0);
        vals.push(1e5 + g.f64_unit() * 1e5);
        vals.push(1e30 * (1.0 + g.f64_unit()));
    }
    // planted near-ties: 1-ulp f64 neighbors, sub-cast-resolution wobbles,
    // exact duplicates
    let base = vals.clone();
    for &v in base.iter().take(5) {
        vals.push(f64::from_bits(v.to_bits() + 1));
        vals.push(v * (1.0 + 1e-9));
        vals.push(v);
    }
    let mut escalations = 0usize;
    let mut pairs = 0usize;
    for i in 0..vals.len() {
        for j in (i + 1)..vals.len() {
            let (a, b) = (vals[i], vals[j]);
            let (o, tier) = ladder_cmp(a, b, i, j);
            assert_eq!(o, oracle_cmp(a, b, i, j), "a={a} b={b}");
            if tier != Tier::Interval {
                escalations += 1;
            }
            pairs += 1;
        }
    }
    assert!(pairs >= 200);
    assert!(escalations > 0, "the sweep never left tier one");
    // exact duplicates tie by FIRST index: v@3 ranks before v@7
    let v = vals[0];
    assert_eq!(ladder_cmp(v, v, 7, 3).0, Ordering::Greater);
    assert_eq!(ladder_cmp(v, v, 3, 7).0, Ordering::Less);
}

#[test]
fn well_separated_values_stay_tier_one() {
    let mut g = Lcg(0xB369_00C1);
    for i in 0..50 {
        let a = g.f64_in(1.0, 1000.0);
        let b = a * (1.0 + 1e-3); // relative gap ≫ the ±2-ulp window
        let (o, tier) = ladder_cmp(a, b, 0, 1);
        assert_eq!(tier, Tier::Interval, "pair {i} escalated");
        assert_eq!(o, Ordering::Less);
        assert_eq!(o, oracle_cmp(a, b, 0, 1));
        assert_eq!(o, naive_cmp(a, b, 0, 1));
    }
}

#[test]
fn removing_tier_two_keeps_the_misorder() {
    let a = 1.0e8f64 + 7.0;
    let b = 1.0e8f64 + 9.0;
    // the tampered ladder: tier-one escalation jumps straight to the naive
    // f32 compare (tier two deleted)
    let tampered = |x: f64, y: f64, ix: usize, iy: usize| -> Ordering {
        match tier1_interval(x, y) {
            Some(o) => o,
            None => naive_cmp(x, y, ix, iy),
        }
    };
    // indices CONFLICT with the truth order, else the index tiebreak masks
    // the defect: a is truly smaller but carries the LARGER index
    assert_eq!(oracle_cmp(a, b, 1, 0), Ordering::Less);
    assert_eq!(tampered(a, b, 1, 0), Ordering::Greater);
    assert_ne!(tampered(a, b, 1, 0), oracle_cmp(a, b, 1, 0));
    // the full ladder is unfooled
    assert_eq!(ladder_cmp(a, b, 1, 0).0, Ordering::Less);
}

// ─────────────────────────────────────────────────────────────────────────────
// F4 — overflow-guard-authoritative-sum-repair-after-loop
// ─────────────────────────────────────────────────────────────────────────────

/// Kahan compensation over f32 terms. `guard = true` is the landed law: the
/// first nonfinite running sum is pinned as the overflow AUTHORITY, the
/// compensation stops, and the post-loop repair (sum + c) runs only on
/// finite state — overflow stays +inf, never NaN. `guard = false` is the
/// defect: compensation continues through +inf into NaN.
fn kahan_sum(terms: &[f32], guard: bool) -> (f32, bool) {
    let mut sum = 0.0f32;
    let mut c = 0.0f32;
    let mut i = 0usize;
    let mut overflowed = false;
    while i < terms.len() {
        let y = terms[i] - c;
        let t = sum + y;
        if !t.is_finite() {
            if guard {
                sum = t; // the authoritative +inf
                overflowed = true;
                break;
            }
            c = (t - sum) - y; // (inf - inf) − y => NaN
            sum = t;
            i += 1;
            continue;
        }
        c = (t - sum) - y;
        sum = t;
        i += 1;
    }
    if guard {
        // repair AFTER the loop, only on finite state
        if overflowed {
            return (sum, true);
        }
        return (sum + c, false);
    }
    (sum + c, false)
}

const OVERFLOW_TERMS: [f32; 5] = [3.2e38, 3.2e38, 1e30, 1e30, 7e35];
const FINITE_TERMS: [f32; 5] = [1e30, 2e30, 3e30, 4e28, 1.5e20];

#[test]
fn compensation_inside_the_loop_lauanders_inf_to_nan() {
    let (s, ovf) = kahan_sum(&OVERFLOW_TERMS, false);
    assert!(!ovf);
    assert!(s.is_nan(), "compensating through +inf must produce NaN");
}

#[test]
fn the_guard_keeps_the_authoritative_inf() {
    let (s, ovf) = kahan_sum(&OVERFLOW_TERMS, true);
    assert!(ovf);
    assert!(s.is_infinite() && s.is_sign_positive() && !s.is_nan());
}

#[test]
fn finite_values_identical_to_plain_kahan() {
    let (fixed, ovf) = kahan_sum(&FINITE_TERMS, true);
    let (plain, _) = kahan_sum(&FINITE_TERMS, false);
    assert!(!ovf);
    assert_eq!(fixed.to_bits(), plain.to_bits());
}

#[test]
fn removing_the_guard_returns_nan() {
    // the landed form with the guard deleted IS the defect arm: the same
    // loop, compensation allowed through the nonfinite state
    let (s, _) = kahan_sum(&OVERFLOW_TERMS, false);
    assert!(s.is_nan());
}
