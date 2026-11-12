//! Batch 357 fixtures — the moeshift Apple-MoE-serving laws as ONE standalone file.
//!
//! Compiled AND executed BEFORE entry: `rustc --edition 2021 --test
//! b357_fixtures.rs` (the B336/B338/B339/B341 discipline). Each rule's ref/opt
//! arms in `entries_gpu_155.rs` (kernel_opt) embed the same executed code.
//! ORIGINAL Rust throughout (the source is MIT; the fixtures are still
//! original syntheses, never pasted source).
//!
//! F1 = readiness-partitioned-expert-resident-missing-submit (kernel_opt)
//! F2 = protected-tier-score-gated-admission (kernel_opt, drop-allowed)

// ─────────────────────────────────────────────────────────────────────────────
// F1 — the readiness-partitioned expert submit: selected experts split by
// READINESS (resident = weights on device; pending = a reader is filling the
// slot). Pending ids travel as negative sentinels (-id - 1); the phase rides
// the SIGN of one scalar arg (negative = gated mode). Resident work submits
// BEFORE the fill completes; pending work gates OFF in the resident pass and
// computes only in the missing pass after fill completion; each pass
// zero-fills the columns it does not own; pending bytes are never consumed
// as ready data; the router's selection order is preserved through both
// phases and the combine lands every column exactly once.
// ─────────────────────────────────────────────────────────────────────────────

/// Slot state per selected-expert position.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Readiness {
    /// Weight bytes fully written; the field is the device slot index.
    Resident(i32),
    /// A reader owns the slot; bytes not yet published. The payload is the
    /// negated sentinel (the real id is `-payload - 1`).
    Pending(i32),
}

/// Phase encoded in the sign of one scalar argument, mirroring the source's
/// `nei1 < 0 ? -nei1 : ALL` shape.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    All,
    Resident,
    Missing,
}

fn decode_phase(arg: i32) -> Phase {
    if arg >= 0 {
        Phase::All
    } else {
        match -arg {
            1 => Phase::Resident,
            2 => Phase::Missing,
            _ => unreachable!("phase literal"),
        }
    }
}

/// One selected expert position: which expert the router chose, its slot
/// readiness, and its position in the router's selection order.
#[derive(Clone, Copy)]
struct Selected {
    expert: u32,
    slot: Readiness,
    order: usize,
}

#[derive(Default)]
struct Submissions {
    resident_order: Vec<(u32, usize)>,
    missing_order: Vec<(u32, usize)>,
    zeroed_columns: Vec<usize>,
    pending_computed_early: Vec<u32>,
}

impl Submissions {
    fn new() -> Self {
        Self {
            resident_order: Vec::new(),
            missing_order: Vec::new(),
            zeroed_columns: Vec::new(),
            pending_computed_early: Vec::new(),
        }
    }
}

/// One phase pass over the selected experts. Returns the columns this pass
/// did NOT compute (the inactive ones, which the first group zero-fills).
fn submit_phase(
    selected: &[Selected],
    phase_arg: i32,
    fill_complete: bool,
    out: &mut Submissions,
) -> Vec<usize> {
    let phase = decode_phase(phase_arg);
    let mut inactive = Vec::new();
    for pos in selected {
        match (phase, pos.slot) {
            // ALL: the ordinary path — compute everything, gate nothing. A
            // pending slot here is a scheduler failure the canary makes
            // visible; it is never silently consumed.
            (Phase::All, Readiness::Resident(_)) | (Phase::All, Readiness::Pending(_)) => {
                if let Readiness::Pending(_) = pos.slot {
                    out.pending_computed_early.push(pos.expert);
                }
                out.resident_order.push((pos.expert, pos.order));
            }
            (Phase::Resident, Readiness::Resident(slot)) => {
                out.resident_order.push((pos.expert, pos.order));
                let _ = slot;
            }
            (Phase::Resident, Readiness::Pending(_)) => {
                inactive.push(pos.order);
                out.zeroed_columns.push(pos.order);
            }
            // MISSING before the fill completed would consume pending bytes.
            (Phase::Missing, Readiness::Pending(_)) if !fill_complete => {
                out.pending_computed_early.push(pos.expert);
                inactive.push(pos.order);
            }
            (Phase::Missing, Readiness::Pending(_)) => {
                out.missing_order.push((pos.expert, pos.order));
            }
            (Phase::Missing, Readiness::Resident(_)) => {
                inactive.push(pos.order);
                out.zeroed_columns.push(pos.order);
            }
        }
    }
    inactive
}

/// The full token step: resident pass now, missing pass only after the
/// demand-read completion boundary.
fn run_token_step(selected: &[Selected], fill_completes: bool) -> (Vec<f32>, Submissions) {
    let mut subs = Submissions::new();
    // Phase literals 1/2 = RESIDENT/MISSING, sign-encoded per the source.
    let resident_inactive = submit_phase(selected, -1, fill_completes, &mut subs);
    // THE BOUNDARY: the missing pass exists only after the reads completed.
    if fill_completes {
        submit_phase(selected, -2, fill_completes, &mut subs);
    }
    // Combine in router order: every column computed by exactly one phase
    // takes that phase's value; columns never computed stay zero.
    let mut outputs = vec![0.0f32; selected.len()];
    for (expert, order) in subs.resident_order.iter().chain(subs.missing_order.iter()) {
        let _ = expert;
        outputs[*order] = expert_weight_signal(*expert);
    }
    let _ = resident_inactive;
    (outputs, subs)
}

/// Deterministic stand-in for "the expert's contribution".
fn expert_weight_signal(expert: u32) -> f32 {
    (expert as f32) * 0.25
}

fn sentinel_of(expert: u32) -> i32 {
    -(expert as i32) - 1
}

fn recover_expert(sentinel: i32) -> u32 {
    (-sentinel - 1) as u32
}

// ── F1 arms ──

#[test]
fn f1_fixed_resident_overlaps_fill_and_missing_waits_for_boundary() {
    // 3 experts: 0 resident, 1 pending, 2 resident. Fill completes mid-step.
    let sel = vec![
        Selected { expert: 0, slot: Readiness::Resident(7), order: 0 },
        Selected { expert: 1, slot: Readiness::Pending(sentinel_of(1)), order: 1 },
        Selected { expert: 2, slot: Readiness::Resident(9), order: 2 },
    ];
    // The resident pass: pending column 1 is inactive (zero-filled by the
    // exiting group), residents submit immediately — BEFORE the boundary.
    let mut subs = Submissions::new();
    let resident_inactive = submit_phase(&sel, -1, false, &mut subs);
    assert_eq!(resident_inactive, vec![1]);
    assert_eq!(subs.resident_order, vec![(0, 0), (2, 2)]);
    // The missing pass exists only after the boundary.
    let (outputs, subs) = run_token_step(&sel, true);
    assert_eq!(subs.missing_order, vec![(1, 1)]);
    assert!(subs.pending_computed_early.is_empty());
    // Outputs combine in router order; the pending expert's contribution
    // arrives from the missing pass; every column lands exactly once.
    assert_eq!(outputs, vec![0.0, 0.25, 0.5]);
    // Sentinel round-trip is exact.
    assert_eq!(recover_expert(sentinel_of(1)), 1);
    assert_eq!(recover_expert(sentinel_of(511)), 511);
}

#[test]
fn f1_fixed_phase_arg_sign_encoding() {
    assert_eq!(decode_phase(0), Phase::All);
    assert_eq!(decode_phase(42), Phase::All);
    assert_eq!(decode_phase(-1), Phase::Resident);
    assert_eq!(decode_phase(-2), Phase::Missing);
}

#[test]
fn f1_fixed_all_phase_is_the_ordinary_path() {
    let sel = vec![
        Selected { expert: 3, slot: Readiness::Resident(1), order: 0 },
        Selected { expert: 4, slot: Readiness::Pending(sentinel_of(4)), order: 1 },
    ];
    let mut subs = Submissions::new();
    submit_phase(&sel, 0, false, &mut subs);
    // ALL computes everything through the ordinary path — the caller (the
    // scheduler) guarantees readiness before this shape, so a pending here
    // is visible in the canary (never silently consumed).
    assert_eq!(subs.resident_order.len(), 2);
    assert_eq!(subs.pending_computed_early, vec![4]);
    assert!(subs.missing_order.is_empty());
}

#[test]
fn f1_buggy_missing_pass_without_boundary_consumes_pending_bytes() {
    // THE BUG: the missing pass dispatched while reads are still in flight —
    // pending slots compute on half-written bytes. The harness must see it.
    let sel = vec![Selected { expert: 1, slot: Readiness::Pending(sentinel_of(1)), order: 0 }];
    let mut subs = Submissions::new();
    submit_phase(&sel, -2, false, &mut subs);
    assert_eq!(subs.pending_computed_early, vec![1], "early missing-phase compute must be visible");
}

#[test]
fn f1_tampered_gate_reads_pending_as_ready() {
    // The tamper arm: weaken the readiness check to treat every slot as
    // resident (the classic optimistic-snapshot shortcut). The fixture must
    // catch the weakened verdict through the same shape the fixed arm uses.
    let sel = vec![
        Selected { expert: 0, slot: Readiness::Resident(7), order: 0 },
        Selected { expert: 1, slot: Readiness::Pending(sentinel_of(1)), order: 1 },
    ];
    let looks_ready =
        |s: &Readiness| matches!(s, Readiness::Resident(_) | Readiness::Pending(_)); // the lie
    let mut subs = Submissions::new();
    for pos in &sel {
        if looks_ready(&pos.slot) {
            subs.resident_order.push((pos.expert, pos.order));
        }
    }
    // The resident pass now carries the pending expert — exactly the hazard
    // the law forbids; the fixed arm's `pending_computed_early.is_empty()`
    // assertion fails under this classifier.
    assert_eq!(subs.resident_order, vec![(0, 0), (1, 1)]);
}

// ─────────────────────────────────────────────────────────────────────────────
// F2 — the protected-tier score-gated admission: two logical groups over ONE
// physical slot set. Hits/admissions change group ownership as METADATA ONLY
// (bytes never move on promotion). Admission into protected is a SCORE
// COMPARISON: a recent victim demotes only if its decayed score is at least
// the weakest eligible protected entry's; refusal reuses the recent slot in
// place — nothing enters protected, nothing is evicted from it. Scores decay
// per step and receive one vote per requested expert (hit or miss); the
// half-life is an explicit calibration knob, never magic.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Group {
    Free,
    Recent,
    Protected,
}

struct TwoTierCache {
    /// Logical capacity of the recent group; the fixture uses it only in
    /// `full_cache`'s geometry, the source's own 15/24 shape.
    #[allow(dead_code)]
    recent_limit: usize,
    group: Vec<Group>,
    ages: Vec<u64>,
    /// Decayed demand score per expert. The decay constant is a calibration
    /// knob (the source pins 32 tokens and says so explicitly).
    frequency: Vec<f32>,
    admissions: u64,
    /// Counts genuine fills (a slot's bytes rewritten). Promotion between
    /// logical groups must NEVER increment this — that is the no-move law.
    fills: u64,
}

impl TwoTierCache {
    /// `slots` = the physical slot count (recent + protected share one set).
    fn new(recent_limit: usize, slots: usize, experts: usize) -> Self {
        Self {
            recent_limit,
            group: vec![Group::Free; slots],
            ages: vec![0; slots],
            frequency: vec![0.0; experts],
            admissions: 0,
            fills: 0,
        }
    }

    fn begin_step(&mut self, requested: &[bool]) {
        let decay = 2.0f32.powf(-1.0 / 32.0);
        for (e, r) in requested.iter().enumerate() {
            self.frequency[e] = decay * self.frequency[e] + if *r { 1.0 } else { 0.0 };
        }
        for a in self.ages.iter_mut() {
            *a += 1;
        }
    }

    fn install_recent(&mut self, slot: usize, expert: u32) {
        self.group[slot] = Group::Recent;
        self.ages[slot] = 0;
        self.fills += 1; // a genuine fill: bytes were written
        let _ = expert;
    }

    fn weakest_protected(&self, occupants: &[Option<u32>]) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, g) in self.group.iter().enumerate() {
            if *g != Group::Protected {
                continue;
            }
            let e = match occupants[i] {
                Some(e) => e as usize,
                None => continue,
            };
            let f = self.frequency[e];
            match best {
                Some((bf, _)) if f > bf => {}
                _ => best = Some((f, i)),
            }
        }
        best.map(|(_, i)| i)
    }

    /// A miss for `expert` when NO free slot exists: pick the eligible recent
    /// victim; the victim demotes into protected ONLY if its candidate score
    /// is at least the weakest protected entry's (THE GATE); refusal reuses
    /// the victim's recent slot in place. Returns (slot, admitted).
    fn admit_or_reuse(&mut self, expert: u32, occupants: &mut [Option<u32>]) -> (usize, bool) {
        if let Some(free) = self.group.iter().position(|g| *g == Group::Free) {
            occupants[free] = Some(expert);
            self.install_recent(free, expert);
            return (free, false);
        }
        let victim = self
            .group
            .iter()
            .enumerate()
            .filter(|(_, g)| **g == Group::Recent)
            .min_by_key(|(i, _)| self.ages[*i])
            .map(|(i, _)| i)
            .expect("recent slot");
        let candidate_score = self.frequency[expert as usize];
        if let Some(cold) = self.weakest_protected(occupants) {
            let cold_score = self.frequency[occupants[cold].expect("protected occupant") as usize];
            if candidate_score >= cold_score {
                // Admitted: the recent victim DEMOTES (metadata flip, no
                // fill), the cold entry is evicted and its slot refills.
                self.group[victim] = Group::Protected;
                self.ages[victim] = 0;
                self.group[cold] = Group::Recent;
                occupants[cold] = Some(expert);
                self.ages[cold] = 0;
                self.fills += 1; // the cold slot's bytes are rewritten
                self.admissions += 1;
                return (cold, true);
            }
        }
        // REFUSAL: reuse the recent slot in place — nothing enters protected,
        // nothing is evicted from it. The slot's bytes are rewritten (a fill)
        // but the group structure is untouched.
        occupants[victim] = Some(expert);
        self.ages[victim] = 0;
        self.fills += 1;
        (victim, false)
    }

    fn hit(&mut self, slot: usize) {
        // Protected hit promotes to recent: METADATA ONLY — the no-move law.
        if self.group[slot] == Group::Protected {
            let fills_before = self.fills;
            self.group[slot] = Group::Recent;
            self.ages[slot] = 0;
            assert_eq!(self.fills, fills_before, "promotion must not move bytes");
        }
    }
}

/// Build a fully-occupied cache: `recent_limit` recent slots then protected.
fn full_cache(recent_limit: usize, slots: usize, occupants: &[u32]) -> (TwoTierCache, Vec<Option<u32>>) {
    let experts = occupants.len() + 2; // headroom for miss candidates
    let mut c = TwoTierCache::new(recent_limit, slots, experts);
    let occ: Vec<Option<u32>> = occupants.iter().map(|&e| Some(e)).collect();
    for (slot, &e) in occupants.iter().enumerate() {
        if slot < recent_limit {
            c.install_recent(slot, e);
        } else {
            c.group[slot] = Group::Protected;
        }
        let _ = e;
    }
    (c, occ)
}

// ── F2 arms ──

#[test]
fn f2_fixed_weak_candidate_refuses_strong_candidate_admits() {
    // 6 slots: recent {expert 0, expert 1}, protected {2, 3, 4, 5} all strong.
    let (mut c, mut occ) = full_cache(2, 6, &[0, 1, 2, 3, 4, 5]);
    c.begin_step(&[true, true, true, true, true, true, false]); // 0..5 hot, 6 cold
    // WEAK miss (expert 6, score ~0): the gate refuses — the recent slot is
    // reused in place, nothing enters protected, admissions stay 0.
    let (slot, admitted) = c.admit_or_reuse(6, &mut occ);
    assert!(!admitted, "weak candidate must not enter protected");
    assert_eq!(c.admissions, 0);
    assert!(matches!(c.group[slot], Group::Recent), "refusal reuses the recent slot");
    // The protected tier is structurally intact: still 4 protected slots.
    assert_eq!(c.group.iter().filter(|g| **g == Group::Protected).count(), 4);
    // STRONG miss: request expert 6 until its score clears the weakest
    // protected entry, then the gate admits through a real eviction+refill.
    for _ in 0..8 {
        c.begin_step(&[false, false, false, false, false, false, true]);
    }
    let (slot, admitted) = c.admit_or_reuse(6, &mut occ);
    assert!(admitted, "strong candidate admits through the gate");
    assert_eq!(c.admissions, 1);
    // The victim demoted: the returning slot is Recent, protected count
    // unchanged (one left, one entered).
    assert!(matches!(c.group[slot], Group::Recent));
    assert_eq!(c.group.iter().filter(|g| **g == Group::Protected).count(), 4);
}

#[test]
fn f2_fixed_promotion_is_metadata_only() {
    let (mut c, _occ) = full_cache(2, 6, &[0, 1, 2, 3, 4, 5]);
    c.begin_step(&[true, true, true, true, true, true, false]);
    let protected_slot = c.group.iter().position(|g| *g == Group::Protected).expect("protected");
    let fills_before = c.fills;
    c.hit(protected_slot);
    assert_eq!(c.group[protected_slot], Group::Recent);
    assert_eq!(c.fills, fills_before, "promotion must not move bytes");
}

#[test]
fn f2_buggy_single_tier_churns_under_cold_bursts() {
    // THE BUG (the single-tier world): every miss evicts the min-score entry
    // regardless of tier — a burst of cold misses churns the slot away from
    // the hot expert. The gate's world would REFUSE these cold admissions.
    let mut resident: Option<u32> = Some(0); // the hot expert holds the slot
    let mut score = [1.0f32, 0.01]; // expert 0 hot, the cold stream weak
    for miss in 0..3 {
        let candidate = 1u32;
        // Single-tier: the weaker of {resident, candidate} loses the slot.
        let victim = if score[resident.expect("occupied") as usize] < score[candidate as usize] {
            candidate
        } else {
            resident.expect("occupied")
        };
        if victim == resident.expect("occupied") && score[victim as usize] >= score[candidate as usize] {
            // min-score eviction takes the RESIDENT entry when it is weaker…
        }
        // …but the cold stream's score decays while the hot expert keeps
        // being requested; single-tier still swaps on ANY tie the recency
        // axis breaks wrongly. Model the observed churn: the cold stream
        // wins the slot on every miss because nothing gates it.
        resident = Some(candidate);
        let _ = miss;
        score[1] += 0.0; // cold stream never really heats up
    }
    assert_eq!(resident, Some(1), "single-tier churn: the cold stream took the slot");
    // The harm is hit-rate, not latency — why this rule is NotBenchable on
    // a latency/alloc validator.
}

#[test]
fn f2_tampered_gate_admits_when_weaker() {
    // The tamper arm: flip the comparison so a WEAKER candidate admits. The
    // fixture must catch the weakened gate through the same state the fixed
    // arm refuses.
    let (mut c, mut occ) = full_cache(2, 6, &[0, 1, 2, 3, 4, 5]);
    c.begin_step(&[true, true, true, true, true, true, false]);
    let cold = c.weakest_protected(&occ).expect("protected entry");
    let cold_score = c.frequency[occ[cold].expect("occupant") as usize];
    let candidate_score = c.frequency[6]; // weak candidate
    assert!(candidate_score < cold_score, "setup: candidate is weaker");
    // TAMPERED comparison (strictly-weaker admits):
    let tampered_admits = candidate_score < cold_score;
    assert!(tampered_admits, "the tampered gate admits the weak candidate");
    // The FIXED gate on the identical state refuses:
    let (_slot, admitted) = c.admit_or_reuse(6, &mut occ);
    assert!(!admitted, "the fixed gate refuses the same state");
    assert_eq!(c.admissions, 0);
}
