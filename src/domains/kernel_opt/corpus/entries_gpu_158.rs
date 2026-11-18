//! Batch 370 — the `mlx-learn` certified-KNN + compensated-numerics
//! `kernel_opt` distill (Plan 383 / Research 415): four decision-content
//! laws from tabulai/mlx-learn's exactness machinery — the certified
//! approximate prefilter (shortlists + the next-omitted barrier + the
//! escalate-to-exact escape + refuse-safe −inf bias), the two-float
//! compensated accumulation for devices without f64, the escalating-precision
//! exact-ordering ladder, and the overflow-guard authoritative-sum law
//! (found in this batch's own unread-file re-read; the plan-383 task list
//! anticipated it — "new candidates go through the same dedup gate").
//!
//! Source: `tabulai/mlx-learn` @
//! `07054ce32b5b19db7b78b3d4c69e1d2446b3389b` (tag v0.1.3, Apache-2.0; T0
//! re-verified at the pin this batch: every quote byte-checked by two
//! reader subagents, six citation corrections carried — the device files
//! live under `src/mlxlearn_device/`, the carry-widening quotes live in
//! `_neighbors_integer.py:350-355` + `docs/knn-integer-certificate.md:23-26`
//! NOT the compact certificate doc, the participation probe lives in
//! `_neighbors.py:54-119` NOT `neighbors/_base.py`, R4's second cite is
//! `:260-262`, the precision header spans `:9-259`, F2's gate code is
//! `svm/_classes.py:248-273`). Per the transcription law the corpus consumes
//! DECISION CONTENT with ORIGINAL Rust fixtures
//! (`corpus/fixtures/b370_fixtures.rs`, standalone-compiled AND executed
//! before entry — 17/17; the Metal/MLX surface is modeled as a CPU mock;
//! the fixture never calls a GPU), never source verbatim. Renumber note:
//! this batch was staged as nominal Batch 369; the plan-384 sybil lane
//! allocated 369 first-push (commit 76811c50, 2026-11-18T07:28:54Z) —
//! first-allocation wins, this batch renumbered 370 pre-landing with zero
//! external citations under the old number. Dropped at authoring: NC2
//! `sorted-insert-shift-chain-expanded-to-registers` (C+/B− — the bounded
//! private-array→register expansion is layout engineering whose fix-space
//! is device-compiler-shaped; recorded in the research note) + NC3
//! `capability-refusal-before-lossy-conversion` (C+ — half (a) classifies
//! at the layer that can still serve, half (b) is the house refuse-loud
//! posture with no new corpus shape; recorded). Faces landed count-neutral
//! on their owners: F1 on B105 `cooperative-tensor-mma-capability-route`
//! (the probe VERIFIES OUTPUT), F2 on the dispatch family (the decisive
//! axis + both-counts AND + class exclusion). Batch 370 / Plan 383 /
//! Research 415.

use crate::domains::kernel_opt::corpus::KernelRule;

// The corpus entry IS the payload (data, not logic) — the kernel build fn
// convention (the entries_popcorn precedent); surfaced by the Plan-377
// lane's compile repair, disclosed in its commit.
#[allow(clippy::too_many_lines)]
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── K1 — the batch lead: a lossy fast screen emits per-partition
        // shortlists PLUS a next-omitted barrier; acceptance ONLY on a bound
        // certificate proving the screened answer IS the exact answer; per-ROW
        // escalation to the complete exact scan; refuse-safe −inf bias so the
        // cheap filter can never discard an uncertifiable row; widening is
        // perf-only while the acceptance criterion stays exact. ─────────────
        KernelRule {
            rule_id: "certified-prefilter-omission-bound-escalate-exact".to_string(),
            gpu_concept: "MECHANISM (certified approximate prefilter + per-row exact escalation; category B — correctness): a lossy fast pass (fp32 ranking, or int8 quantized codes) produces per-partition shortlists PLUS a *next-omitted barrier* — while the partition scan admits candidates onto a carry-sized shortlist, every candidate it does NOT admit folds its lower bound into `omitted = metal::min(omitted, metal::max(distance, distances[K - 1]))` (the scan emits the barrier; `_neighbors_registers.py:181` shows the identical update `omitted{q} = metal::min(omitted{q}, metal::max(sum{q}, {worst}))` — the barrier is emitted by EVERY scan variant), and the merge carries the barriers through (`_neighbors_screen.py:57-59` re-derives per row). The finish ADMITS the fast result only when a certificate proves it equals the exact result under conservative f32 interval arithmetic: each partition's omitted-candidate LOWER bound must clear the merged k-th widened UPPER (`docs/knn-compact-certificate.md:192-194`: 'real_lower(B) > real_upper(R).'); otherwise THAT ROW escalates — and the escalation granularity is PER ROW: only certificate-uncertain rows run the complete exact scan. Unsafe rows (nonfinite values or oversized quantization error) get a negative-infinity bias so the certificate can never discard them — verbatim (`_neighbors_integer.py:282-283`): 'Unsafe rows produce negative- / infinity biases so the certificate cannot discard them.' + the bias emission (`:84` train side / `:147` query side): 'biases[row]=safe?0.9841f*quant_norm-128.0f*measured:-INFINITY;' — the cheap filter defaults to ADMIT (the bias is −inf, the row simply cannot rank), the exact path still decides. Shortlist widening under centering is explicitly a perf knob while the acceptance criterion is untouched — verbatim (`_neighbors_integer.py:350-355`): 'A wider shortlist avoids needless complete scans; the / unchanged omitted-candidate bound remains the acceptance criterion.' + 'carry <= 32 is therefore a correctness requirement, not only a tuning knob.' (the finish refines at most one candidate per SIMD lane). DECISION CONTENT: (a) the barrier is EMITTED BY THE SCAN and CARRIED BY THE MERGE — a merge that drops the barriers (or a certificate never consulted) accepts shortlist-only answers and loses true neighbors (the fixture: the barrier-less accept emits id 8 where the oracle says id 15); (b) the certificate bound is CONSERVATIVE on both edges — the omission bound is a lower bound over every omitted candidate, compared against the k-th kept value's widened upper; inflating the bound (×1.5) flips the accept into the optimistic direction and the oracle catches it; (c) escalation is PER ROW, not per query batch — the certificate's failure mode is a targeted complete scan, never a wholesale fallback of certified rows; (d) refuse-safe marking: poison biases to −INF, it never drops — the row escalates and the exact answer survives; (e) widening the carry changes filtering EFFICIENCY ONLY; the omitted-point bound certifies any carry. CLASS SIGNATURE: a lossy screen pass whose per-partition order statistics yield a machine-checkable omission bound + an exact finish that can consume it per row. REGIME: any lossy prefilter (fp32 over f64, int8 codes, fp16 storage) feeding an exact consumer where the screen can compute a sound per-partition lower bound on what it omitted; workloads where the certificate clears on most rows (the fixture: certified rows skip the complete scan; uncertain rows still answer exactly). ANTI-REGIME: screens with no computable bound (a heuristic prefilter — the answer is a different rule, not this one); bounds computed from the DATA of the row at hand when a precomputed bound family exists (the tile-max class — different admission cost model, see kinship); consumers that need the exact VALUES of non-screened ids (the certificate covers the kept k only). ⛔ KINSHIP (adjudicated both directions): `int8-screen-error-bound-certifies-exact-rescore` (gpu_121, Batch 275) is the nearest sibling — THAT owns the matmul-route screen where the bound E is a WEIGHT property (per-group Amax/Bmax) computed once per head and the admission test reads the screen's own second value (tau <= i2 - 2E); THIS owns the PARTITION-SCAN screen where the bound is built incrementally DURING the scan from the candidates the shortlist refused (the barrier), carried through the merge as first-class data, with PER-ROW escalation granularity and the refuse-safe −inf bias as explicit content. Both are certified-screen/exact-finish laws; they compose (a route screen feeding a partition scan carries both certificates). `coarse-precision-first-exact-refine` (B111) owns the lossy-bin/refine contract (the lossy key may only DISCRIMINATE bins, never rank within them) — THIS adds the machine-checkable acceptance certificate + the escalation granularity. `refine-by-rescan-prefix-filter-vs-buffered-ties` (B110) owns tie refinement. `existential-witness-early-exit-scan` (B365) owns the ∃-arity early exit — different predicate. GREP EVIDENCE (re-run at the landing HEAD): 'certificate|certified' over the corpus -> gpu_121 (adjudicated above) + entries_49 (a revalidation identity token — unrelated sense) ONLY; 'escalat|acceptance.*bound|screen|shortlist|refuse.safe|sentinel.*bias' -> no other owner of the barrier/escalate/refuse-safe triple. NotBenchable-here (the entries_gpu_65 / dial9 / Batch-136 class — the law's home is the GPU screening domain; a rust_perf CPU-shaped sibling would fork the law across domains, the promote/demote law binds at authoring). The measurement instrument is the CPU fixture: BIT-IDENTITY of the certified screen's answer vs the complete exact f64 scan + the WORK CENSUS (certified rows never pay the full scan; uncertain rows escalate) — correctness-first, census arithmetic, never wall-time; no GOAT bench record owed (the plan's pre-registration, recorded 2026-11-16 BEFORE first corpus edit). Introduced in-repo at 07054ce (the pinned tip). Source: tabulai/mlx-learn @ 07054ce3 (Apache-2.0) — src/mlxlearn_device/_neighbors_screen.py:27-33 (the barrier injection) + :71-136 (the clear/membership/fallback ladder), _neighbors_integer.py:77-84/:147 (the −inf bias) + :276-302 (prepare_integer, the docstring at :282-283) + :350-355 (carry widening), _neighbors_compact.py:83-86 (the fp16-tier twin bias) + :107-110/:181-183/:214-222 (sentinel barriers + the accepted predicate), docs/knn-compact-certificate.md:173-194 (truncation bound + acceptance criterion), docs/knn-integer-certificate.md:23-26 (the carry prose). Batch 370 / Plan 383 / Research 415."
                .to_string(),
            ref_shape: r#"// ref_shape: the lossy screen's shortlist is trusted WITHOUT the
// omission barrier — whatever survived the carry-sized per-partition
// shortlists IS the answer, so a true neighbor omitted by the carry
// (a cast-collapse tie at the boundary) is silently lost.

const CARRY: usize = 6;
const TOP_K: usize = 3;
const PARTS: usize = 4;
const PER_PART: usize = 8;
const CANDS: usize = PARTS * PER_PART;

#[derive(Clone, Copy)]
struct Cand {
    idx: usize,
    cast: f32,
}

fn by_cast(a: &Cand, b: &Cand) -> std::cmp::Ordering {
    a.cast.partial_cmp(&b.cast).unwrap().then(a.idx.cmp(&b.idx))
}

fn screen_shortlists_only(d: &[f64]) -> Vec<(usize, f64)> {
    let mut all: Vec<Cand> = d
        .iter()
        .enumerate()
        .map(|(i, &v)| Cand { idx: i, cast: v as f32 })
        .collect();
    let mut kept: Vec<Cand> = Vec::with_capacity(PARTS * CARRY);
    for p in 0..PARTS {
        let mut part: Vec<Cand> = all[p * PER_PART..(p + 1) * PER_PART].to_vec();
        part.sort_by(by_cast);
        kept.extend(part.into_iter().take(CARRY)); // the barrier never forms
    }
    kept.sort_by(by_cast);
    kept.into_iter().take(TOP_K).map(|c| (c.idx, d[c.idx])).collect()
}

fn exact_top_k(d: &[f64]) -> Vec<(usize, f64)> {
    let mut v: Vec<(usize, f64)> =
        d.iter().enumerate().map(|(i, &x)| (i, x)).collect();
    v.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(a.0.cmp(&b.0)));
    v.truncate(TOP_K);
    v
}

/// the adversarial row: true top-3 = {10.0@0, 20.0@1, 500.0@15}; the
/// seven P1 fillers sit at 500.0 + 1.4e-5 — under half-ulp of 500.0 —
/// so they CAST to 500.0 and occupy P1's whole carry, evicting the
/// exact-500.0 twin @15 that the oracle ranks third.
fn adversarial_row() -> Vec<f64> {
    let mut d = vec![800.0f64; CANDS];
    d[0] = 10.0;
    d[1] = 20.0;
    for (i, v) in (2..8).zip([600.0, 650.0, 700.0, 750.0, 800.0, 850.0]) {
        d[i] = v;
    }
    for i in 8..15 {
        d[i] = 500.0 + 1.4e-5;
    }
    d[15] = 500.0;
    d[16] = 500.0;
    for (i, v) in (17..24).zip([510.0, 515.0, 520.0, 525.0, 530.0, 535.0, 540.0]) {
        d[i] = v;
    }
    for (i, v) in (24..32).zip([550.0, 560.0, 570.0, 580.0, 590.0, 600.0, 610.0, 620.0]) {
        d[i] = v;
    }
    d
}

fn ids(v: &[(usize, f64)]) -> Vec<usize> {
    v.iter().map(|w| w.0).collect()
}

#[test]
fn the_barrier_omitted_loses_a_true_neighbor() {
    let d = adversarial_row();
    let eids = ids(&screen_shortlists_only(&d));
    let oids = ids(&exact_top_k(&d));
    // the shortlist-only accept emits the wrong twin (8, a cast-collapse
    // filler) where the oracle says 15
    assert_eq!(oids, vec![0, 1, 15]);
    assert_ne!(eids, oids, "barrier-less accept must lose the true neighbor");
    assert_eq!(eids, vec![0, 1, 8]);
}"#.to_string(),
            opt_pattern: r#"// opt_pattern: the scan emits the next-omitted barrier, the merge
// carries it, the certificate (every partition's omission LOWER bound
// clears the merged k-th widened UPPER) admits per row, and every
// uncertain or poisoned row escalates to the complete exact scan —
// the screened answer, when certified, IS the exact answer bit for bit.

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

fn by_cast(a: &CandInterval, b: &CandInterval) -> std::cmp::Ordering {
    a.cast.partial_cmp(&b.cast).unwrap().then(a.idx.cmp(&b.idx))
}

fn screen_row(d: &[f64]) -> RowScreen {
    let mut bias = 1.0f32;
    let mut cands: Vec<Option<CandInterval>> = Vec::with_capacity(d.len());
    for (i, &v) in d.iter().enumerate() {
        let s = v as f32;
        let ok = s.is_finite() && (s as f64 - v).abs() <= QUANT_ERR_LIMIT;
        if !ok {
            bias = f32::NEG_INFINITY; // refuse-safe: escalate, never drop
        }
        cands.push(ok.then(|| CandInterval {
            idx: i,
            cast: s,
            lo: widen_lo(s),
            hi: widen_hi(s),
        }));
    }
    if !bias.is_finite() {
        return RowScreen { bias, parts: Vec::new() };
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
                // rejected: max(lower_bound, shortlist_kth) = the candidate's lo
                let kth = members[members.len() - 1];
                barrier = barrier.min(c.lo.max(kth.lo));
            }
        }
        parts.push(PartitionScreen { members, barrier });
    }
    RowScreen { bias, parts }
}

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
/// k-th widened upper — no omitted candidate can then displace the kept k.
/// `barrier_scale` is the tamper hook (×1.5 = the inflated-bound defect).
fn certified(parts: &[PartitionScreen], kth_hi: f32, barrier_scale: f32) -> bool {
    parts.iter().all(|p| p.barrier * barrier_scale > kth_hi)
}

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
        let emitted = decide_row(&d, &mut census);
        let oracle = exact_top_k(&d);
        for (w, o) in emitted.iter().zip(oracle.iter()) {
            assert_eq!(w.0, o.0, "row {r}: id mismatch");
            assert_eq!(w.1.to_bits(), o.1.to_bits(), "row {r}: not bit-identical");
        }
    }
    assert!(census.certified > 0, "no row certified — no work was saved");
    assert!(census.full_scans < 16, "every row escalated — nothing was saved");
}

#[test]
fn the_uncertified_row_escalates_and_stays_exact() {
    let mut census = Census::default();
    let mut d = vec![800.0f64; CANDS];
    d[0] = 10.0;
    d[1] = 20.0;
    for i in 8..15 {
        d[i] = 500.0 + 1.4e-5;
    }
    d[15] = 500.0;
    d[16] = 500.0;
    let screen = screen_row(&d);
    let before = census.full_scans;
    let emitted = decide_row(&d, &mut census);
    assert_eq!(census.full_scans, before + 1, "uncertain row must escalate");
    assert_eq!(ids(&emitted), ids(&exact_top_k(&d)), "drifted");
    let (_, kth_hi) = merge_top_k(&screen.parts, &d);
    assert!(!certified(&screen.parts, kth_hi, 1.0), "certified?");
    // the poison arm: a nonfinite screen value biases −INF and escalates
    let mut p = vec![500.0f64; CANDS];
    p[0] = f64::NAN;
    let ps = screen_row(&p);
    assert!(ps.bias.is_sign_negative() && !ps.bias.is_finite());
    assert_eq!(ids(&decide_row(&p, &mut census)), ids(&exact_top_k(&p)));
}

#[test]
fn an_inflated_barrier_is_detected_by_the_oracle() {
    let mut d = vec![800.0f64; CANDS];
    d[0] = 10.0;
    d[1] = 20.0;
    for i in 8..15 {
        d[i] = 500.0 + 1.4e-5;
    }
    d[15] = 500.0;
    d[16] = 500.0;
    let screen = screen_row(&d);
    let (emitted, kth_hi) = merge_top_k(&screen.parts, &d);
    assert!(
        certified(&screen.parts, kth_hi, 1.5),
        "tampered bound must wrongly certify"
    );
    assert_ne!(
        ids(&emitted),
        ids(&exact_top_k(&d)),
        "the oracle must catch the tampered acceptance"
    );
}

/// shared deterministic randomness — seeded LCG only, no rand, no deps
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
}"#.to_string(),
        },
        // ── K2 — two-float compensated accumulation for devices whose
        // language has no f64: fast2sum / add2 / mul2, the honest
        // "numerical check, not a proof" posture, the pinned fold order. ──
        KernelRule {
            rule_id: "device-fp64-absent-two-float-compensated-accumulation".to_string(),
            gpu_concept: "MECHANISM (compensated arithmetic for f64-less devices; category B — numerics): a cancellation-heavy reduction on a device whose language has no f64 (Metal/WGSL) accumulates in hi/lo TWO-FLOAT arithmetic instead of trusting the bare f32 lane — fast2sum rn(x,y) = (h = x+y, err = y-(h-x)) with EXACT error recovery (requires |x| ≥ |y|), add2 the two-sum error recovery (s = a.x+b.x; v = s-a.x; e = (a.x-(s-v)) + (b.x-v) + a.y + b.y; renormalize rn(s,e)), mul2 with the FMA error term (p = a.x*b.x; e = fma(a.x,b.x,-p) + a.x*b.y + a.y*b.x), div2, and a pairwise two-float tree via simd_shuffle_down for the reduction itself. Verbatim algebra (`_compensated.py:13-18`): 'inline float2 rn(float x,float y){float h=x+y;return float2(h,y-(h-x));}' + 'inline float2 add2(float2 a,float2 b){float s=a.x+b.x;float v=s-a.x;float e=(a.x-(s-v))+(b.x-v)+a.y+b.y;return rn(s,e);}' + 'inline float2 mul2(float2 a,float2 b){float p=a.x*b.x;float e=fma(a.x,b.x,-p)+a.x*b.y+a.y*b.x;return rn(p,e);}' + 'inline float2 sum2(float2 a){for(uint k=16;k>0;k>>=1){float2 b=float2(simd_shuffle_down(a.x,k),simd_shuffle_down(a.y,k));a=add2(a,b);}return a;}'. THE HONEST POSTURE is the content, not ceremony — the module docstring verbatim (`_compensated.py:4-5`): 'The transcendental approximation and arithmetic do not provide a formal error / bound; callers must treat the output as a numerical check, not a proof.' — the two-float result is a CHECK that tracks cancellation, never a proof of correctness; consumers treat it as such (the source's own logistic convergence gate consumes norm+tail under a pinned 0.95 margin, `_logistic.py:187`, and its predict tie-break reads BOTH components `(hi > 0) | ((hi == 0) & (lo >= 0))`, `_logistic.py:93` — the lo component is decision-bearing at the boundary). ⛔ THE FOLD ORDER IS PINNED (the Issue-940 determinism carve-out, carried in-rule): two-float arithmetic is ORDER-SENSITIVE (each add2's error term depends on the running pair) — a two-float reduction must pin its fold order (sequential ascending in the fixture; a pairwise tree in the source's sum2) and never present compensated accumulation as order-free or reassociate it across lanes. REGIME: cancellation-heavy reductions (large feature offsets, mean-then-subtract patterns, small deltas on big running sums) on devices whose shader language lacks f64. ANTI-REGIME: well-separated magnitudes (naive f32 tracks within tolerance — the fixture's no-harm arm; the extra ops are waste); hosts WITH f64 (use it); outputs that need a FORMAL bound (two-float is a check, not a proof — the source says so and so does this rule). ⛔ KINSHIP (adjudicated): `kahan-endpoint-peel-dead-flops-exact` (B276) owns endpoint-peel DEAD-OP elimination on a Kahan loop — different axis (this rule ADDS compensation algebra, that one removes dead flops from existing compensation); `hi-lo-split-pow2-prescale-clears-subnormal-floor` (gpu_102) owns the pow2 PRESCALE axis for subnormal floors — adjacent remedy class, composes; sec `wide-reduction-precision-split` (B352) owns the CPU wider-ACCUMULATOR remedy (f64 lanes exist) — THIS is the no-f64-learner's two-lane emulation of the same idea, the axis distinct in-rule; sec `variance-cancellation-two-pass` (B366) owns the two-pass variance spelling — value-collapse avoidance at the ALGORITHM level, not an accumulation substrate. GREP EVIDENCE (re-run at the landing HEAD): 'two_sum|fast2sum|double-double|double-float' -> ZERO owners; 'compensated' -> B276 + gpu_102 + sec wide-reduction only (all adjudicated above). NotBenchable-here (the dial9/entries_gpu_65 class — same posture as K1: the CPU fixture proves error REDUCTION on a cancellation vector + the fast2sum exactness identity, never wall-time). Source: tabulai/mlx-learn @ 07054ce3 (Apache-2.0) — src/mlxlearn_device/_compensated.py:12-38 (the header algebra), :40-72 (the forward), :1-6 (the docstring posture). Batch 370 / Plan 383 / Research 415."
                .to_string(),
            ref_shape: r"// ref_shape: the reduction trusts the bare f32 lane on a device
// with no f64 — a large running sum's half-ulp exceeds each small
// delta, so the naive ascending fold loses the delta mass silently
// (deterministically here: bigs first puts every delta on a sum of
// 1.28e6 whose half-ulp 0.0625 already exceeds 0.00390625).

fn fold_f32_naive(v: &[f32]) -> f32 {
    v.iter().fold(0.0f32, |acc, &x| acc + x)
}

fn fold_f64_oracle(v: &[f32]) -> f64 {
    v.iter().fold(0.0f64, |acc, &x| acc + x as f64)
}

/// [64 × 20000.0, 64 × 0.00390625], bigs first: every delta lands on a
/// sum whose half-ulp already exceeds it.
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

/// the delta mass: 64 × 2⁻⁸ = 0.25 — a naive fold cannot lose more
/// than the mass it was fed, and this one loses EXACTLY all of it.
const DELTA_MASS: f64 = 64.0 * 0.00390625;

#[test]
fn naive_f32_loses_the_small_deltas() {
    let v = big_then_delta();
    let naive = fold_f32_naive(&v);
    // the deltas contributed exactly zero: bit-exact 1280000.0
    assert_eq!(naive.to_bits(), 1280000.0f32.to_bits());
    let oracle = fold_f64_oracle(&v);
    assert_eq!(oracle, 1280000.0 + DELTA_MASS);
}".to_string(),
            opt_pattern: r#"// opt_pattern: the same fold in hi/lo two-float arithmetic —
// fast2sum/add2/mul2, the pinned sequential-ascending order, the lo
// lane carrying what the hi lane's f32 rounding dropped.

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

/// add2: two-sum error recovery + renormalization — the source's
/// algebra verbatim: s, v, the four-term error, then rn(s, e).
fn add2(a: F2, b: F2) -> F2 {
    let s = a.hi + b.hi;
    let v = s - a.hi;
    let e = (a.hi - (s - v)) + (b.hi - v) + a.lo + b.lo;
    fast2sum(s, e)
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

fn two_float_as_f64(f: F2) -> f64 {
    f.hi as f64 + f.lo as f64
}

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

const DELTA_MASS: f64 = 64.0 * 0.00390625;

/// shared deterministic randomness — seeded LCG only, no rand, no deps
struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    fn f64_in(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (self.next_u32() as f64 / 4294967296.0) * (hi - lo)
    }
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
}"#.to_string(),
        },
        // ── K3 — exact ordering escalates through precision tiers: f32
        // interval compare → guarded double-single → exact; values stay
        // f32, only the chosen indices are exact; ties by FIRST index. ──
        KernelRule {
            rule_id: "exact-ordering-escalating-precision-ladder".to_string(),
            gpu_concept: "MECHANISM (escalating-precision exact ordering; category B — numerics): f32 distance ORDERING is decided through a three-tier comparison ladder — tier 1 a conservative f32 interval compare (each value widened ±2 ulp outward; a verdict ONLY when the intervals are disjoint), tier 2 a guarded double-single compare (Dekker split via the 2^13+1 multiplier; d = ah-bh; e = ((ah-d)-bh) + (al-bl); sign(d+e) decisive, tiny/uncertain magnitudes escalate), tier 3 EXACT arithmetic — in the source a dyadic INTEGER SUPERACCUMULATOR over bit-decoded mantissas: 'Add/subtract a shifted 48-bit product in a signed two's-complement / superaccumulator. Nineteen limbs cover every intermediate for positive / int32 D' (`_neighbors_precision.py:26-28`), the bit-decode because FP ops are not trusted: 'Decode the bits rather than performing floating / operations, which are allowed to flush subnormal operands/results to zero.' (`:15-17`), with two PROVED exact sub-tier shortcuts ahead of the general 19-limb form (common-exponent int64 `:144-166`, rebased 4-limb `:170-187` — 'this is the same exact arithmetic, with a proved smaller accumulator, not a reduced-precision approximation', `:167-169`). THE CONTRACT is indices-only exactness — verbatim (`:6`): 'remain FP32; these helpers only choose their corresponding training indices.' — and ties resolve by INDEX ORDER only: `comparison < 0 || (comparison == 0 && index < other_index)` (`:227`) — first index wins (the Issue-940 first-index tie rule). REGIME: orderings where f32 rounding can misorder (cast-collapse near-ties like 1e8+7 vs 1e8+9 — both collapse onto one f32; the f64 truths differ but the lane cannot see it) and the ORDER of indices is the output (top-k selection, tournament/scatter argmin). ANTI-REGIME: consumers that need exact VALUES (returned distances stay f32 — the ladder never promises f64 values); well-separated inputs (tier 1 decides; the ladder's extra tiers are dead cost — the fixture's no-escalation arm); exact-equal values (they are NOT misorderings — the tie rule owns them). ⛔ KINSHIP (adjudicated): the exact-tie family (topk-sampler-specialization-carries-general-tie-rule, B343's faces; chunked-argmax-strict-gt-tie-identical B361) owns the DETERMINISM of ties (which equal value wins) — THIS owns the EXACTNESS of ordering (whether two f32-representable values are truly ordered); a chunked argmax over ladder-compared candidates composes both. `qmc-fixed-dims-hash-for-unbounded-draws` (gpu_157's K7) — sampler posture, unrelated axis. `coarse-precision-first-exact-refine` (B111) refines VALUES within bins; THIS refines only ORDER/indices. GREP EVIDENCE (re-run at the landing HEAD): 'superaccumulator|dyadic' -> ZERO owners; 'exact.*(comparison|ordering|tie)' -> the bit-exact emulation/dispatch family (different axis — exact REPLICATION of an engine, not exact ORDER of lane values), no owner of the escalation ladder. NotBenchable-here (the dial9 class, K1's posture): the CPU fixture proves the ladder matches the f64 oracle EXACTLY across a magnitude sweep (subnormals, near-ties, duplicates) with escalation census — correctness, never wall-time; on-device the real tier 3 is the integer superaccumulator, which the fixture names but stands in with the host's exact f64 compare (two f32-born quantities compared in f64 are exact). Source: tabulai/mlx-learn @ 07054ce3 (Apache-2.0) — src/mlxlearn_device/_neighbors_precision.py:1-7 (docstring) + :9-259 (PRECISION_HEADER; tiers at :100-129/:144-187/:188-259) + :15-24 (bit-decode) + :26-29 (the 19-limb bound) + :227 (tie rule), docs/knn-precision-ordering.md (the telescoping-induction Kahan bound for tier 1, the Ogita-Rump-Oishi two-sum bound for tier 2 — both explicitly BOUNDED, unresolved differences escalate rather than being declared ties; exhaustive f64/integer oracle tests over the full finite f32 exponent range). Batch 370 / Plan 383 / Research 415."
                .to_string(),
            ref_shape: r"// ref_shape: the lane trusts the f32 compare — two true values
// whose casts collapse (ulp(1e8) = 8) read EQUAL and the index
// tiebreak picks whichever carries the smaller index, even when the
// f64 truth orders them the other way.

fn naive_cmp(a: f64, b: f64, ia: usize, ib: usize) -> std::cmp::Ordering {
    match (a as f32).partial_cmp(&(b as f32)) {
        Some(std::cmp::Ordering::Equal) | None => ia.cmp(&ib),
        Some(o) => o,
    }
}

fn oracle_cmp(a: f64, b: f64, ia: usize, ib: usize) -> std::cmp::Ordering {
    match a.partial_cmp(&b) {
        Some(std::cmp::Ordering::Equal) | None => ia.cmp(&ib),
        Some(o) => o,
    }
}

#[test]
fn naive_compare_misorders_the_adversarial_pair() {
    let a = 1.0e8f64 + 7.0;
    let b = 1.0e8f64 + 9.0;
    // both collapse onto the same f32 (ulp(1e8) = 8)
    assert_eq!((a as f32).to_bits(), (b as f32).to_bits());
    assert_eq!((a as f32).partial_cmp(&(b as f32)), Some(std::cmp::Ordering::Equal));
    // f64 truth: a < b — but the indices CONFLICT with the truth order,
    // so the naive index tiebreak hands the win to the wrong side
    assert_eq!(oracle_cmp(a, b, 1, 0), std::cmp::Ordering::Less);
    assert_eq!(naive_cmp(a, b, 1, 0), std::cmp::Ordering::Greater);
}".to_string(),
            opt_pattern: r#"// opt_pattern: the three-tier ladder — interval compare → guarded
// double-single (Dekker 8193-split) → exact (here the host's f64
// compare, the stand-in for the dyadic integer superaccumulator the
// device runs). Returned values stay f32; only the chosen indices are
// exact; true ties resolve by FIRST index.

use std::cmp::Ordering;

/// tier 1: the cheap conservative interval compare — each f32 cast
/// widened ±2 ulp outward; a verdict only when the intervals are disjoint.
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

/// Dekker's split: ~12-bit halves via the 2¹³+1 multiplier.
fn dekker_split(x: f32) -> (f32, f32) {
    let t = 8193.0f32 * x;
    let hi = t - (t - x);
    let lo = x - hi;
    (hi, lo)
}

/// tier 2: guarded double-single subtract — sign(d + e) decisive; tiny
/// or uncertain magnitudes escalate.
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
    Some(if s < 0.0 { Ordering::Less } else { Ordering::Greater })
}

/// the ladder. Tier 3 is the exact stand-in (the entry names the dyadic
/// integer superaccumulator the device uses). Exact ties resolve by
/// FIRST index; returned values stay f32.
fn ladder_cmp(a: f64, b: f64, ia: usize, ib: usize) -> (Ordering, &'static str) {
    if let Some(o) = tier1_interval(a, b) {
        return (o, "interval");
    }
    if let Some(o) = tier2_ds_compare(a as f32, b as f32) {
        return (o, "double-single");
    }
    let o = match a.partial_cmp(&b) {
        Some(Ordering::Equal) | None => ia.cmp(&ib),
        Some(o) => o,
    };
    (o, "exact")
}

fn oracle_cmp(a: f64, b: f64, ia: usize, ib: usize) -> Ordering {
    match a.partial_cmp(&b) {
        Some(Ordering::Equal) | None => ia.cmp(&ib),
        Some(o) => o,
    }
}

fn naive_cmp(a: f64, b: f64, ia: usize, ib: usize) -> Ordering {
    match (a as f32).partial_cmp(&(b as f32)) {
        Some(Ordering::Equal) | None => ia.cmp(&ib),
        Some(o) => o,
    }
}

/// shared deterministic randomness — seeded LCG only, no rand, no deps
struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    fn f64_in(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (self.next_u32() as f64 / 4294967296.0) * (hi - lo)
    }

    fn f64_unit(&mut self) -> f64 {
        self.next_u32() as f64 / 4294967296.0
    }
}

#[test]
fn naive_compare_misorders_the_adversarial_pair() {
    let a = 1.0e8f64 + 7.0;
    let b = 1.0e8f64 + 9.0;
    // the casts collapsed, so even the double-single tier sees equal
    // values and the exact tier owns the verdict
    assert_eq!(ladder_cmp(b, a, 1, 0), (Ordering::Greater, "exact"));
    assert_eq!(ladder_cmp(a, b, 1, 0).0, Ordering::Less);
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
    // planted near-ties: 1-ulp f64 neighbors, sub-cast wobbles, duplicates
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
            if tier != "interval" {
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
        assert_eq!(tier, "interval", "pair {i} escalated");
        assert_eq!(o, Ordering::Less);
        assert_eq!(o, oracle_cmp(a, b, 0, 1));
        assert_eq!(o, naive_cmp(a, b, 0, 1));
    }
}

#[test]
fn removing_tier_two_keeps_the_misorder() {
    let a = 1.0e8f64 + 7.0;
    let b = 1.0e8f64 + 9.0;
    // the tampered ladder: tier-one escalation jumps straight to the
    // naive f32 compare (tier two deleted) — indices conflict with the
    // truth order, else the tiebreak masks the defect
    let tampered = |x: f64, y: f64, ix: usize, iy: usize| -> Ordering {
        match tier1_interval(x, y) {
            Some(o) => o,
            None => naive_cmp(x, y, ix, iy),
        }
    };
    assert_eq!(oracle_cmp(a, b, 1, 0), Ordering::Less);
    assert_eq!(tampered(a, b, 1, 0), Ordering::Greater);
    assert_ne!(tampered(a, b, 1, 0), oracle_cmp(a, b, 1, 0));
    // the full ladder is unfooled
    assert_eq!(ladder_cmp(a, b, 1, 0).0, Ordering::Less);
}"#.to_string(),
        },
        // ── K4 — the overflow-guard law (found in this batch's own
        // unread-file re-read): a compensated accumulator must not launder
        // +inf into NaN — the guarded sum stays the overflow authority;
        // repair only after the loop, only on finite state. ───────────────
        KernelRule {
            rule_id: "overflow-guard-authoritative-sum-repair-after-loop".to_string(),
            gpu_concept: "MECHANISM (overflow-authoritative compensation; category B — numerics/safety): a compensated accumulator (Kahan/two-float) over nonnegative, overflow-capable terms must not let its compensation ALGEBRA run through a nonfinite running state — once the running sum overflows to +inf, the GUARDED (uncompensated) sum is the overflow AUTHORITY: pin it, stop compensating (break or skip the update), and run any post-loop repair (sum + c) only on finite state; overflow stays +inf, never NaN. Verbatim source law (`_neighbors_registers.py:69-76`): 'Once nonnegative squared terms overflow, the original guarded sum remains / +inf. Unguarded compensation can become NaN; repair only after the loop.' + the guard itself `sum{q} = metal::isfinite(sum{q}) ? sum{q} : INFINITY;` — the isfinite test INSIDE the loop pins the authority every iteration. THE DEFECT IS DETERMINISTIC: Kahan's correction term is (t - sum) - y with t = sum + y; once sum = +inf, t = +inf, so the correction is (inf - inf) - y = NaN, which feeds the next y - c and poisons the accumulator permanently — a compensated loop is strictly WORSE than a naive one under overflow (naive +inf stays +inf; compensated +inf becomes NaN). REGIME: compensated reductions over nonnegative terms that can overflow the lane's float range (squared-distance accumulations in kernels — the source's own squared-term context; large-batch reductions over growing magnitudes). ANTI-REGIME: terms of bounded range (no overflow path — the guard is dead cost, though the finite-input arm shows it is HARMLESS there: bit-identical to plain Kahan); signed terms with cancellation (an inf can arise from a genuine +inf−+inf indeterminacy the guard's pinning also covers — the pin keeps the first nonfinite state authoritative either way); consumers that want saturating behavior (they should clamp INPUTS — this rule only preserves the honest +inf). ⛔ KINSHIP (adjudicated): K1's sibling `device-fp64-absent-two-float-compensated-accumulation` (this batch, K2) owns the compensated ACCUMULATION SUBSTRATE (the algebra); THIS owns its OVERFLOW interface (when the substrate's state goes nonfinite, the algebra stops — the two compose: a two-float fold carries the K4 guard at its hi lane); `maddubs-overflow-two-exact-mac-selector` (gpu_53) owns an int8 saturating-MAC selector — input-range admission, not accumulation-state authority; sec `variance-cancellation-two-pass` (B366) — value collapse, not overflow laundering. GREP EVIDENCE (re-run at the landing HEAD): 'overflow' -> gpu_53 + entries_popcorn block-local-reference-frame (both admission/coverage selectors, no accumulation-state owner) + entries_05 renderer-error (unrelated sense); 'isfinite.*repair|launder|authoritative.*sum' -> ZERO owners. NotBenchable-here (the dial9 class, K1's posture): the CPU fixture pins the DETERMINISTIC NaN arm vs the +inf-pinning arm + bit-identity with plain Kahan on finite inputs — a correctness census, never wall-time. Source: tabulai/mlx-learn @ 07054ce3 (Apache-2.0) — src/mlxlearn_device/_neighbors_registers.py:69-76 (the candidate guard + its comment), found by this batch's unread-file re-read (the plan-383 task anticipated new candidates through the same dedup gate). Batch 370 / Plan 383 / Research 415."
                .to_string(),
            ref_shape: r#"// ref_shape: Kahan compensation with no overflow guard — once the
// running sum reaches +inf, the correction algebra (inf - inf) - y
// produces NaN and poisons the accumulator permanently. A compensated
// loop is strictly WORSE than a naive one here: naive +inf stays +inf.

fn kahan_unguarded(terms: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let mut c = 0.0f32;
    for &term in terms {
        let y = term - c;
        let t = sum + y;
        c = (t - sum) - y; // (inf - inf) - y => NaN once sum overflows
        sum = t;
    }
    sum + c
}

const OVERFLOW_TERMS: [f32; 5] = [3.2e38, 3.2e38, 1e30, 1e30, 7e35];

#[test]
fn compensation_inside_the_loop_lauanders_inf_to_nan() {
    let s = kahan_unguarded(&OVERFLOW_TERMS);
    assert!(s.is_nan(), "compensating through +inf must produce NaN");
}"#.to_string(),
            opt_pattern: r"// opt_pattern: the guard — the first nonfinite running sum is
// pinned as the overflow AUTHORITY, compensation stops, and the
// post-loop repair (sum + c) runs only on finite state. Overflow
// stays +inf, never NaN; finite inputs stay bit-identical to plain
// Kahan (the guard is dead cost outside the regime).

fn kahan_sum(terms: &[f32]) -> (f32, bool) {
    let mut sum = 0.0f32;
    let mut c = 0.0f32;
    let mut overflowed = false;
    for &term in terms {
        if overflowed {
            break; // the authoritative +inf is pinned; algebra stops
        }
        let y = term - c;
        let t = sum + y;
        if !t.is_finite() {
            sum = t; // the authoritative +inf
            overflowed = true;
            break;
        }
        c = (t - sum) - y;
        sum = t;
    }
    if overflowed {
        return (sum, true);
    }
    (sum + c, false) // repair AFTER the loop, only on finite state
}

const OVERFLOW_TERMS: [f32; 5] = [3.2e38, 3.2e38, 1e30, 1e30, 7e35];
const FINITE_TERMS: [f32; 5] = [1e30, 2e30, 3e30, 4e28, 1.5e20];

#[test]
fn the_guard_keeps_the_authoritative_inf() {
    let (s, ovf) = kahan_sum(&OVERFLOW_TERMS);
    assert!(ovf);
    assert!(s.is_infinite() && s.is_sign_positive() && !s.is_nan());
}

#[test]
fn finite_values_identical_to_plain_kahan() {
    // plain (unguarded) Kahan is the finite-path reference: identical bits
    let mut sum = 0.0f32;
    let mut c = 0.0f32;
    for &term in FINITE_TERMS.iter() {
        let y = term - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    let (fixed, ovf) = kahan_sum(&FINITE_TERMS);
    assert!(!ovf);
    assert_eq!(fixed.to_bits(), (sum + c).to_bits());
}

#[test]
fn removing_the_guard_returns_nan() {
    // the landed form with the guard deleted IS the defect arm: the same
    // loop, compensation allowed through the nonfinite state
    let mut sum = 0.0f32;
    let mut c = 0.0f32;
    for &term in OVERFLOW_TERMS.iter() {
        let y = term - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    assert!((sum + c).is_nan());
}".to_string(),
        },
    ]
}
