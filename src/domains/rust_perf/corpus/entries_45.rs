//! Entries slice C - Batch 271 (2026-10-21, Plan 275 / Research 307): the
//! splash-wave second-tier serving pool, slice C (2 dispatch + 1 concurrency
//! rules) - incoai/splash @ 5ea4043fbecf7612a9b0066e7f9c10481eae4e1b +
//! 82b93c5f32a39865ac945a507134f3ab01b59d92, b7844dcf745909fb5caf79f82dd985d64041393b,
//! e2c0af27e8cb31ffd5990fac001b111e0894da93, Apache-2.0; every quote re-verified
//! at the .raw/b271/splash clone this batch. All three rules are corpus-only
//! `NotBenchable` (decision-content serving laws: an admission-bound selection,
//! an admission-posture retry and an event-ownership ordering -- the staged
//! grades and source-reported figures ride each `context_notes`) - no proposer
//! files. Fixtures are ORIGINAL machinery (never splash code verbatim), each
//! compiled and executed standalone (rustc --edition 2021 --test) before this
//! table was assembled; the buggy/fixed arm pairs share contract tests and
//! differ only on the law.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "admission-tests-the-bound-the-admitted-mode-serves".to_string(),
            name: "A start that cannot spill (the tier closed, its write budget spent, its quota short of the projection) is admitted Resident against the bound the admitted mode serves -- the SOLO bound, the tokens one request's KV may take in memory ALONE (the resident context, as without the SSD cache) -- never against the shared resident-KV budget minus the other lanes' floors (the lane state, the Index runway, the readers' staging or hot cache) that a lone Resident request never uses; the shared bound keeps its own job, the mode decision BESIDE other lanes where every lane's footprint sums into it; a refusal names the solo bound that refused it".to_string(),
            category: "dispatch".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `admission-tests-the-bound-the-admitted-mode-serves`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/engine/{Engine,MemoryPlan}
// @ 5ea4043 + 82b93c5 (Apache-2.0): an admission gate that decides, for a
// start that CANNOT spill (the spill tier closed, its write budget spent or
// its quota short of the projection), whether the start runs Resident anyway
// -- as it would without the SSD cache. Two candidate bounds exist:
//   (a) the SOLO bound -- the tokens one request's KV may take in memory
//       ALONE (the resident context the planner advertises);
//   (b) the SHARED-bytes budget -- resident KV after the other lanes'
//       floors (the lane state, the Index runway, the readers' staging or
//       hot cache), memory a lone Resident request never uses.
// THE ANTI-PATTERN: the cannot-spill arm tests (b) -- so a lone request
// whose projection sits between the two bounds is refused (or bounded)
// though memory holds it alone, while the same start with the tier
// disabled is served. The fixed arm tests the bound the admitted mode
// serves: (a) for the cannot-spill arm, (b) kept for the mode decision
// BESIDE other lanes.

pub const PAGE_TOKENS: usize = 1024;
pub const FULL_PAGE_BYTES: u64 = 4096;

/// The planner's own accounting: the dynamic budget and every floor under
/// it, plus the resident context (one request's KV alone, in tokens).
pub struct MemoryPlan {
    pub dynamic_budget_bytes: u64,
    pub lane_state_bytes: u64,
    pub index_runway_bytes: u64,
    pub staging_bytes: u64,
    pub hot_target_bytes: u64,
    /// The resident context: the tokens one request's KV may take in
    /// memory alone, as without the SSD cache.
    pub resident_context_tokens: u32,
}

impl MemoryPlan {
    /// Resident KV: the dynamic budget less every floor -- the SHARED
    /// budget the plan builds for the mode decision beside other lanes.
    pub fn resident_kv_bytes(&self) -> u64 {
        self.dynamic_budget_bytes
            - self.lane_state_bytes
            - self.index_runway_bytes
            - self.staging_bytes.max(self.hot_target_bytes)
    }
}

/// What admission needs of the plan, as data.
pub struct SpillGeometry {
    pub resident_kv_bytes: u64,
    pub resident_tokens: u32,
}

impl MemoryPlan {
    pub fn geometry(&self) -> SpillGeometry {
        SpillGeometry {
            resident_kv_bytes: self.resident_kv_bytes(),
            resident_tokens: self.resident_context_tokens,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gate {
    Open,
    /// The tier takes no writes: the write budget is spent.
    WriteBudgetSpent,
    /// The tier takes no writes: the quota is short.
    TierClosed,
}

impl Gate {
    pub fn takes_writes(&self) -> bool {
        matches!(self, Gate::Open)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LaneOutcome {
    Resident,
    Spill,
    /// Refused: the tier's quota (capacity exhausted).
    CapacityExhausted,
    /// Refused: the tier's daily write budget is spent.
    KvWriteBudget,
}

/// The one request admission judges.
pub struct Request {
    pub projected_tokens: usize,
}

pub fn pages(tokens: usize) -> u64 {
    (tokens / PAGE_TOKENS) as u64
}

pub fn resident_bytes(tokens: usize) -> u64 {
    pages(tokens) * FULL_PAGE_BYTES
}

pub struct Engine {
    pub geometry: SpillGeometry,
}

impl Engine {
    // THE ANTI-PATTERN: the cannot-spill arm judged against the SHARED
    // bytes -- `residentBytes(projectedTokens) <= residentKvBytes` -- the
    // dynamic budget less the lane state, the Index runway and the
    // readers' floor, memory a Resident request never uses.
    fn fits_alone_bytes(&self, active: &Request) -> bool {
        resident_bytes(active.projected_tokens) <= self.geometry.resident_kv_bytes
    }

    /// The cannot-spill arm: the tier takes no writes, so the start runs
    /// Resident if it fits, else it is refused.
    pub fn admit_start_cannot_spill(&self, active: &Request, gate: Gate) -> LaneOutcome {
        debug_assert!(!gate.takes_writes());
        if self.fits_alone_bytes(active) {
            return LaneOutcome::Resident;
        }
        match gate {
            Gate::WriteBudgetSpent => LaneOutcome::KvWriteBudget,
            _ => LaneOutcome::CapacityExhausted,
        }
    }

    /// The mode decision BESIDE other lanes: the shared-bytes bound is the
    /// CORRECT bound here -- every lane's footprint sums into it.
    pub fn kv_mode_beside_peers(&self, active: &Request, peer_pages: &[u64]) -> LaneOutcome {
        let mut bytes = resident_bytes(active.projected_tokens);
        for &p in peer_pages {
            bytes += p * FULL_PAGE_BYTES;
        }
        if bytes <= self.geometry.resident_kv_bytes {
            LaneOutcome::Resident
        } else {
            LaneOutcome::Spill
        }
    }
}

const PLAN: MemoryPlan = MemoryPlan {
    dynamic_budget_bytes: 110 * FULL_PAGE_BYTES,
    lane_state_bytes: 40 * FULL_PAGE_BYTES,
    index_runway_bytes: 30 * FULL_PAGE_BYTES,
    staging_bytes: 20 * FULL_PAGE_BYTES,
    hot_target_bytes: 10 * FULL_PAGE_BYTES,
    resident_context_tokens: (50 * PAGE_TOKENS) as u32, // 50 pages
};

fn engine() -> Engine {
    Engine {
        geometry: PLAN.geometry(),
    }
}

// -- shared contract tests (pass in BOTH arms) --------------------------------

#[test]
fn eq_the_shared_budget_is_the_dynamic_budget_less_the_floors() {
    // 110 - 40 - 30 - max(20, 10) = 20 pages: the floors stay out of the
    // resident-KV budget -- which is exactly why it is the WRONG bound for
    // a request that uses none of them.
    assert_eq!(PLAN.resident_kv_bytes(), 20 * FULL_PAGE_BYTES);
}

#[test]
fn eq_the_geometry_carries_the_planners_resident_context() {
    // 82b93c5's enabler face: admission sees the resident context as a
    // token bound, as data.
    assert_eq!(engine().geometry.resident_tokens, (50 * PAGE_TOKENS) as u32);
}

#[test]
fn eq_a_start_under_both_bounds_runs_resident_with_the_tier_closed() {
    let e = engine();
    let small = Request {
        projected_tokens: 10 * PAGE_TOKENS,
    };
    assert_eq!(
        e.admit_start_cannot_spill(&small, Gate::TierClosed),
        LaneOutcome::Resident
    );
    assert_eq!(
        e.admit_start_cannot_spill(&small, Gate::WriteBudgetSpent),
        LaneOutcome::Resident
    );
}

#[test]
fn eq_a_start_past_both_bounds_is_refused() {
    let e = engine();
    let huge = Request {
        projected_tokens: 80 * PAGE_TOKENS, // past the 50-page context AND
                                            // past the 20-page shared budget
    };
    assert_eq!(
        e.admit_start_cannot_spill(&huge, Gate::TierClosed),
        LaneOutcome::CapacityExhausted
    );
}

#[test]
fn eq_beside_peers_the_mode_still_keys_on_the_shared_budget() {
    // The shared-bytes bound is CORRECT for the beside-peers decision:
    // every lane's footprint sums into it. Both arms keep this.
    let e = engine();
    let mid = Request {
        projected_tokens: 10 * PAGE_TOKENS,
    };
    // 10 + 5 = 15 pages <= 20: Resident beside the peer.
    assert_eq!(e.kv_mode_beside_peers(&mid, &[5]), LaneOutcome::Resident);
    // 10 + 12 = 22 pages > 20: Spill beside the peer.
    assert_eq!(e.kv_mode_beside_peers(&mid, &[12]), LaneOutcome::Spill);
}

// -- the anti-pattern's own shape ---------------------------------------------

#[test]
fn bug_a_lone_start_between_the_bounds_is_refused() {
    // The measured shape: resident KV of 20 pages, resident context of 50,
    // a lone start projecting 43 pages. Memory holds it ALONE (43 <= 50),
    // the shared budget does not (43 > 20) -- and the same start with the
    // SSD cache disabled is served.
    let e = engine();
    let lone = Request {
        projected_tokens: 43 * PAGE_TOKENS,
    };
    assert!(pages(lone.projected_tokens) <= 50); // fits ALONE
    assert!(resident_bytes(lone.projected_tokens) > PLAN.resident_kv_bytes()); // not beside the floors
    assert_eq!(
        e.admit_start_cannot_spill(&lone, Gate::TierClosed),
        LaneOutcome::CapacityExhausted
    );
    assert_eq!(
        e.admit_start_cannot_spill(&lone, Gate::WriteBudgetSpent),
        LaneOutcome::KvWriteBudget
    );
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `admission-tests-the-bound-the-admitted-mode-serves`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/{Engine,MemoryPlan}
// @ 5ea4043 + 82b93c5 (Apache-2.0): the law -- admission tests the bound
// the admitted mode serves. A start that cannot spill is judged against
// the SOLO bound, the tokens one request's KV may take in memory alone
// (the resident context), as it would without the SSD cache; the
// shared-bytes budget -- resident KV after the other lanes' floors -- is
// kept for the mode decision BESIDE other lanes, the only decision that
// spends it. A refusal names that solo bound.

pub const PAGE_TOKENS: usize = 1024;
pub const FULL_PAGE_BYTES: u64 = 4096;

/// The planner's own accounting: the dynamic budget and every floor under
/// it, plus the resident context (one request's KV alone, in tokens).
pub struct MemoryPlan {
    pub dynamic_budget_bytes: u64,
    pub lane_state_bytes: u64,
    pub index_runway_bytes: u64,
    pub staging_bytes: u64,
    pub hot_target_bytes: u64,
    /// The resident context: the tokens one request's KV may take in
    /// memory alone, as without the SSD cache.
    pub resident_context_tokens: u32,
}

impl MemoryPlan {
    /// Resident KV: the dynamic budget less every floor -- the SHARED
    /// budget the plan builds for the mode decision beside other lanes.
    pub fn resident_kv_bytes(&self) -> u64 {
        self.dynamic_budget_bytes
            - self.lane_state_bytes
            - self.index_runway_bytes
            - self.staging_bytes.max(self.hot_target_bytes)
    }
}

/// What admission needs of the plan, as data.
pub struct SpillGeometry {
    pub resident_kv_bytes: u64,
    pub resident_tokens: u32,
}

impl MemoryPlan {
    pub fn geometry(&self) -> SpillGeometry {
        SpillGeometry {
            resident_kv_bytes: self.resident_kv_bytes(),
            resident_tokens: self.resident_context_tokens,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gate {
    Open,
    /// The tier takes no writes: the write budget is spent.
    WriteBudgetSpent,
    /// The tier takes no writes: the quota is short.
    TierClosed,
}

impl Gate {
    pub fn takes_writes(&self) -> bool {
        matches!(self, Gate::Open)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LaneOutcome {
    Resident,
    Spill,
    /// Refused: past the solo bound, the tier keeps its KV on the cache.
    CapacityExhausted,
    /// Refused: past the solo bound, the write budget is spent.
    KvWriteBudget,
}

/// The one request admission judges.
pub struct Request {
    pub projected_tokens: usize,
}

pub fn pages(tokens: usize) -> u64 {
    (tokens / PAGE_TOKENS) as u64
}

pub fn resident_bytes(tokens: usize) -> u64 {
    pages(tokens) * FULL_PAGE_BYTES
}

pub struct Engine {
    pub geometry: SpillGeometry,
}

impl Engine {
    // THE LAW: the SOLO bound -- the tokens one request's KV may take in
    // memory ALONE, as without the SSD cache. This is the bound the
    // Resident mode actually serves; the shared-bytes budget is the bound
    // of a DIFFERENT decision (the mode beside other lanes).
    pub fn fits_resident_alone(&self, active: &Request) -> bool {
        active.projected_tokens as u32 <= self.geometry.resident_tokens
    }

    /// The cannot-spill arm: the tier takes no writes, so the start runs
    /// Resident if it fits in memory alone, else it is refused -- and the
    /// refusal NAMES that bound.
    pub fn admit_start_cannot_spill(&self, active: &Request, gate: Gate) -> (LaneOutcome, String) {
        debug_assert!(!gate.takes_writes());
        if self.fits_resident_alone(active) {
            return (LaneOutcome::Resident, String::new());
        }
        let reason = format!(
            "requests past {} tokens keep their KV on the SSD cache, ",
            self.geometry.resident_tokens
        );
        match gate {
            Gate::WriteBudgetSpent => (
                LaneOutcome::KvWriteBudget,
                reason + "whose daily write budget is spent",
            ),
            _ => (LaneOutcome::CapacityExhausted, reason + "which takes no writes"),
        }
    }

    /// The mode decision BESIDE other lanes: the shared-bytes bound is the
    /// CORRECT bound here -- every lane's footprint sums into it.
    pub fn kv_mode_beside_peers(&self, active: &Request, peer_pages: &[u64]) -> LaneOutcome {
        let mut bytes = resident_bytes(active.projected_tokens);
        for &p in peer_pages {
            bytes += p * FULL_PAGE_BYTES;
        }
        if bytes <= self.geometry.resident_kv_bytes {
            LaneOutcome::Resident
        } else {
            LaneOutcome::Spill
        }
    }
}

const PLAN: MemoryPlan = MemoryPlan {
    dynamic_budget_bytes: 110 * FULL_PAGE_BYTES,
    lane_state_bytes: 40 * FULL_PAGE_BYTES,
    index_runway_bytes: 30 * FULL_PAGE_BYTES,
    staging_bytes: 20 * FULL_PAGE_BYTES,
    hot_target_bytes: 10 * FULL_PAGE_BYTES,
    resident_context_tokens: (50 * PAGE_TOKENS) as u32, // 50 pages
};

fn engine() -> Engine {
    Engine {
        geometry: PLAN.geometry(),
    }
}

// -- shared contract tests (pass in BOTH arms) --------------------------------

#[test]
fn eq_the_shared_budget_is_the_dynamic_budget_less_the_floors() {
    // 110 - 40 - 30 - max(20, 10) = 20 pages: the floors stay out of the
    // resident-KV budget -- which is exactly why it is the WRONG bound for
    // a request that uses none of them.
    assert_eq!(PLAN.resident_kv_bytes(), 20 * FULL_PAGE_BYTES);
}

#[test]
fn eq_the_geometry_carries_the_planners_resident_context() {
    // 82b93c5's enabler face: admission sees the resident context as a
    // token bound, as data.
    assert_eq!(engine().geometry.resident_tokens, (50 * PAGE_TOKENS) as u32);
}

#[test]
fn eq_a_start_under_both_bounds_runs_resident_with_the_tier_closed() {
    let e = engine();
    let small = Request {
        projected_tokens: 10 * PAGE_TOKENS,
    };
    assert_eq!(
        e.admit_start_cannot_spill(&small, Gate::TierClosed).0,
        LaneOutcome::Resident
    );
    assert_eq!(
        e.admit_start_cannot_spill(&small, Gate::WriteBudgetSpent).0,
        LaneOutcome::Resident
    );
}

#[test]
fn eq_a_start_past_both_bounds_is_refused() {
    let e = engine();
    let huge = Request {
        projected_tokens: 80 * PAGE_TOKENS, // past the 50-page context AND
                                            // past the 20-page shared budget
    };
    assert_eq!(
        e.admit_start_cannot_spill(&huge, Gate::TierClosed).0,
        LaneOutcome::CapacityExhausted
    );
}

#[test]
fn eq_beside_peers_the_mode_still_keys_on_the_shared_budget() {
    // The shared-bytes bound is CORRECT for the beside-peers decision:
    // every lane's footprint sums into it. Both arms keep this.
    let e = engine();
    let mid = Request {
        projected_tokens: 10 * PAGE_TOKENS,
    };
    // 10 + 5 = 15 pages <= 20: Resident beside the peer.
    assert_eq!(e.kv_mode_beside_peers(&mid, &[5]), LaneOutcome::Resident);
    // 10 + 12 = 22 pages > 20: Spill beside the peer.
    assert_eq!(e.kv_mode_beside_peers(&mid, &[12]), LaneOutcome::Spill);
}

// -- the law's own shape -------------------------------------------------------

#[test]
fn law_a_lone_start_between_the_bounds_runs_resident_alone() {
    // The measured shape: resident KV of 20 pages, resident context of 50,
    // a lone start projecting 43 pages. Memory holds it ALONE (43 <= 50),
    // the shared budget does not (43 > 20) -- the law admits it with the
    // tier closed and with the write budget spent, as without the SSD
    // cache; the old byte test refused it.
    let e = engine();
    let lone = Request {
        projected_tokens: 43 * PAGE_TOKENS,
    };
    assert!(pages(lone.projected_tokens) <= 50); // fits ALONE
    assert!(resident_bytes(lone.projected_tokens) > PLAN.resident_kv_bytes()); // not beside the floors
    assert_eq!(
        e.admit_start_cannot_spill(&lone, Gate::TierClosed).0,
        LaneOutcome::Resident
    );
    assert_eq!(
        e.admit_start_cannot_spill(&lone, Gate::WriteBudgetSpent).0,
        LaneOutcome::Resident
    );
}

#[test]
fn law_the_refusal_names_the_solo_bound() {
    // A closed tier's refusal names the bound that refused it -- the
    // resident context, not a byte quota the request never shared.
    let e = engine();
    let huge = Request {
        projected_tokens: 80 * PAGE_TOKENS,
    };
    let (outcome, reason) = e.admit_start_cannot_spill(&huge, Gate::TierClosed);
    assert_eq!(outcome, LaneOutcome::CapacityExhausted);
    assert!(
        reason.contains("past 51200 tokens"),
        "the refusal names the resident context: {reason}"
    );
}
"#.to_string(),
            context_notes: Some("Grade B+ -- NotBenchable ON THIS BOX (the decision content is an ADMISSION-BOUND law -- which of two bounds a cannot-spill start is judged against -- priced by served requests in a token band; the source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ 5ea4043fbecf7612a9b0066e7f9c10481eae4e1b 'Start a request that cannot spill Resident within the resident context' + 82b93c5f32a39865ac945a507134f3ab01b59d92 'Give admission the resident context as the bound of a request alone' (the enabler, folded: the plan's ContextBounds::resident becomes SpillGeometry::residentTokens), Apache-2.0, Jian Chen, 2026-10-18; every quote re-verified verbatim at the .raw/b271/splash clone this batch at those pins; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (5ea4043 message, verbatim): 'A start whose projection memory does not hold beside the other lanes goes to Spill; when it cannot spill, the tier closed, the write budget spent or the quota short of it, admitSpill starts it Resident if it 'fits in memory alone', as it would without the SSD cache. That test compared it with the plan's resident KV, the dynamic budget less the lane state, the Index runway and the readers' floor (staging or hot cache), memory a Resident request never uses. So a lone request between that and the resident context the server advertises was refused (kv_disk_quota, kv_write_budget, capacity_exhausted) or bounded, though memory held it alone and the same start with --max-cache-disk 0 is served: on a 24 GB M6 with a 1 GiB quota, every request projecting 73,760 to 98,297 tokens.' THE LAW (runtime/engine/Engine.cpp:938-939 @ 5ea4043, verbatim code): 'bool Engine::fitsResidentAlone(const Request &active) const { return projectedTokens(active) <= config_.spill.geometry.residentTokens; }' -- replacing the parent commit's 'geometry.residentBytes(projectedTokens(active)) <= geometry.residentKvBytes' (:938-940 at 5ea4043^), memory a Resident request never uses. THE ARM'S OWN COMMENT (Engine.cpp:962-963 @ 5ea4043, verbatim): 'What it would do without the SSD cache: start Resident if its projection fits in memory alone, and wait for the other lanes there.' THE REFUSAL NAMES THE BOUND (Engine.cpp:977-978, verbatim): 'requests past ' + std::to_string(geometry.residentTokens) + ' tokens keep their KV on the SSD cache, ' -- a closed tier's refusal names the bound that refused it, never a byte quota the request never shared. THE ENABLER (82b93c5, runtime/engine/MemoryPlan.hpp:352-361, verbatim): 'floors; the tokens one request's KV may take in memory alone, as without the SSD cache (ContextBounds::resident), for a request that cannot spill;' over 'uint32_t residentTokens = 0;' -- filled at MemoryPlan.cpp:554 '.residentTokens = breakdown_.contextBounds.resident,' so admission sees the planner's resident context as a token bound, as data. THE SHARED BOUND KEEPS ITS OWN JOB: residentKvBytes still decides the mode beside other lanes -- kvModeFor (Engine.cpp:925-935 @ 5ea4043) sums the active request's resident bytes with every peer lane's spill-floor/resident footprint and compares against residentKvBytes. The upstream test names the shape (message, verbatim): 'with resident KV of 20 pages and a resident context of 50, a lone start projecting 43 pages runs Resident with the tier closed and with a quota of 8 slots; with the old test it is refused.' REGIME: two-level admission where a fallback posture (a start that cannot take the spill path) is judged against a SHARED budget computed for a different decision (the mode beside peers, floors subtracted for lanes a lone request never carries); any admission gate holding a solo bound and a shared budget where the fallback can test the wrong one. ANTI-REGIMES: admissions where the fallback genuinely shares the budget (the floors ARE its footprint -- subtracting them is correct there); single-bound gates (nothing to confuse); bounds that name the same quantity in different units (token bound == byte bound -- no divergence to catch). KINSHIP/DISTINCT-FROM (read + disclosed): concurrent-round-parity-solo-equal (entries_14, B200) owns N-stream admission BEFORE LOAD counting every stream's states+caches into ONE budget and refusing with the fitting window -- a different decision (concurrent N-stream capacity vs the cannot-spill fallback bound) and a different bound consumer; THIS owns which of two existing bounds the cannot-spill arm may test. reserve-stream-caches-at-admission-warm-request-paths-at-load (entries_37, B257) owns admission-time RESERVATION + warm of per-stream caches -- an allocation-timing law, not a bound-selection law; compose (the admission decision precedes the reserve). discretionary-cache-growth-keys-on-spare-not-available (entries_26, B237, same repo) is the SAME two-estimates shape on the growth axis (spare vs available, the OS-warning stop) -- THIS is the admission-bound axis of the neighboring decision; kin shape, distinct law. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'residentTokens|fitsResidentAlone|residentKvBytes|resident context|fits resident alone' -- zero owners; 'admission-tests-the-bound|admitted mode|solo bound' -- zero owners. FIXTURE PROVENANCE: original machinery (never splash code) -- a MemoryPlan deriving the shared budget from the dynamic budget less the floors (lane state, Index runway, max(staging, hot)), a SpillGeometry carrying BOTH bounds, a closed/write-budget-spent tier gate, and the beside-peers mode decision kept on the shared budget; the shared-contract tests (the floor derivation pinned at 20 pages, the geometry carries the planner's context, under-both-bounds admits, past-both-bounds refuses, beside-peers keys on the shared budget) pass in BOTH arms; the discriminating test pins the measured shape (resident KV 20 pages, context 50, a lone start at 43 pages: buggy refuses -- capacity_exhausted / kv_write_budget vs fixed admits Resident under both closed-tier gates, and the refusal names the solo bound). Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: incoai/splash @ 5ea4043fbecf7612a9b0066e7f9c10481eae4e1b + 82b93c5f32a39865ac945a507134f3ab01b59d92 -- runtime/engine/Engine.cpp:938-939 + :962-963 + :977-978 + :925-935 (@ 5ea4043), runtime/engine/MemoryPlan.hpp:352-361 + MemoryPlan.cpp:554 (@ 82b93c5), commit messages (all quotes); Batch 271 / Plan 275.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "in-use-state-retry-under-serving-mark-keyed-on-refusal-cause".to_string(),
            name: "The prompt's replay point is IN-USE work of the request in service (the point its follow-up resumes from): when the host refuses its buffers AND the refusal cause is host pressure (HostPressure) AND its class's reclaim room is exhausted, ONE retry under the Serving mark -- the same posture the request's own KV is admitted under (down to the margin, not one state above it); junctions and checkpoints stay optional (never retried); a Budget cause is never retried; a refusal judged OUTSIDE the serving mark permanently refuses in the headroom band -- every turn replays its whole prompt (source-reported: cached-token ratio 0.103 -> 0.755 on an M6)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `in-use-state-retry-under-serving-mark-keyed-on-refusal-cause`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/engine/Engine.cpp +
// runtime/model/Model.hpp @ b7844dc (Apache-2.0): a host-side allocator of
// state buffers for a serving engine's capture points. Two classes:
//   InUse    -- the prompt's replay point: work of the request in service,
//               the point its follow-up resumes from;
//   Optional -- a junction or a checkpoint: nice-to-have, never retried.
// THE HOST refuses a reservation while its headroom sits in the band
// [warning margin, margin + one state): the optional posture demands one
// state (160 MB on the 27B) of headroom above the 1 GiB warning margin,
// while a paced pass restores only the margin. THE ANTI-PATTERN: every
// capture is taken as an optional allocation OUTSIDE the Serving mark and
// nothing retries -- an in-use replay point refused in that band stays
// refused for good, so every turn replays its whole prompt and the
// cached-token ratio collapses (source-reported 0.103 on an M6). The fixed
// arm keys ONE retry under the Serving mark on the refusal CAUSE
// (HostPressure) and the CLASS (InUse), after the class's reclaim room is
// exhausted -- the same posture the request's own KV is admitted under.
// Optional classes stay optional in both arms.

pub const GIB: u64 = 1 << 30;
pub const WARNING_MARGIN: u64 = GIB; // the 1 GiB warning margin
pub const ONE_STATE: u64 = 160 << 20; // one state's buffers, the 27B

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StateClass {
    /// The prompt's replay point: in-service work.
    InUse,
    /// A junction or a checkpoint: optional.
    Optional,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocationFailure {
    None,
    /// The host's own headroom refused the posture.
    HostPressure,
    /// The tier's own budget (no slot) refused.
    Budget,
}

/// The host stand-in: it refuses a reservation below the posture's floor.
pub struct Host {
    pub headroom_bytes: u64,
}

impl Host {
    /// A request in service (under the Serving mark) may spend down to the
    /// margin; any other posture must keep one state of headroom above it.
    /// The reservation IS one state: the floor check is the whole check.
    pub fn floor(&self, serving: bool) -> u64 {
        if serving {
            WARNING_MARGIN
        } else {
            WARNING_MARGIN + ONE_STATE
        }
    }

    pub fn admits(&self, serving: bool) -> AllocationFailure {
        if self.headroom_bytes >= self.floor(serving) {
            AllocationFailure::None
        } else {
            AllocationFailure::HostPressure
        }
    }
}

/// The model's reservation: the tier's slot budget first, then the host.
pub struct Model {
    pub host: Host,
    pub slots_free: usize,
}

impl Model {
    pub fn reserve_state(&self, serving: bool) -> (bool, AllocationFailure) {
        if self.slots_free == 0 {
            return (false, AllocationFailure::Budget);
        }
        match self.host.admits(serving) {
            AllocationFailure::None => (true, AllocationFailure::None),
            cause => (false, cause),
        }
    }
}

pub struct Engine {
    pub model: Model,
    pub cached_states_free: usize, // what the InUse class's room may reclaim
    pub serving: bool,             // the Serving mark, raised inside the retry only
    pub retries_under_mark: usize,
    pub make_room_attempts: usize,
}

pub struct Capture {
    pub buffers: bool,
    pub cause: AllocationFailure,
}

impl Engine {
    /// makeStateRoom: reclaim one cached state at a time -- the class's own
    /// room -- until the take succeeds or nothing of the class is left.
    fn make_state_room(&mut self, take: &mut dyn FnMut(&mut Self) -> bool) -> bool {
        while self.cached_states_free > 0 {
            self.make_room_attempts += 1;
            self.cached_states_free -= 1;
            self.model.host.headroom_bytes += ONE_STATE; // the bytes come back
            if take(self) {
                return true;
            }
        }
        false
    }

    /// ReserveCapture for one capture point.
    pub fn reserve_capture(&mut self, kind: StateClass) -> Capture {
        // the class arrives and is never read: every capture is optional
        // here -- the anti-pattern
        let _ = matches!(kind, StateClass::InUse);
        // the capture's posture: optional everywhere in THIS arm
        let (ok, mut cause) = self.model.reserve_state(self.serving);
        let mut buffers = ok;
        if !buffers {
            // room from what the class may take, before the refusal is judged
            let got = self.make_state_room(&mut |e: &mut Self| {
                let (ok, _) = e.model.reserve_state(e.serving);
                ok
            });
            if got {
                let (ok, c) = self.model.reserve_state(self.serving);
                buffers = ok;
                cause = c;
            }
        }
        if buffers {
            self.model.slots_free -= 1;
            return Capture {
                buffers: true,
                cause: AllocationFailure::None,
            };
        }
        // THE ANTI-PATTERN: no retry under the Serving mark for ANY class
        // -- an in-use replay point refused in the headroom band stays
        // refused for good.
        Capture { buffers, cause }
    }
}

/// The turn economy: a turn either publishes its replay point (the next
/// turn resumes from it) or replays its whole prompt.
pub struct Transcript {
    pub prompt_tokens: usize,
    pub replay_at: usize,
    pub turns: usize,
    pub cached_tokens: usize,
}

impl Transcript {
    /// Run `turns` turns over the static host posture `headroom`.
    pub fn run(mut self, headroom: u64, slots: usize, cache: usize) -> usize {
        let mut eng = Engine {
            model: Model {
                host: Host { headroom_bytes: headroom },
                slots_free: slots,
            },
            cached_states_free: cache,
            serving: false,
            retries_under_mark: 0,
            make_room_attempts: 0,
        };
        let mut published = false; // the prior turn's replay point, if any
        for _ in 0..self.turns {
            if published {
                // the prior turn published its replay point: this turn
                // resumes from it instead of replaying the whole prompt
                self.cached_tokens += self.replay_at;
            }
            let c = eng.reserve_capture(StateClass::InUse);
            published = c.buffers; // this turn's point serves the next one
        }
        self.cached_tokens
    }
}

pub fn cached_ratio(t: &Transcript) -> f64 {
    t.cached_tokens as f64 / ((t.turns - 1).max(1) * t.prompt_tokens) as f64
}

fn eng_at(headroom: u64, slots: usize, cache: usize) -> Engine {
    Engine {
        model: Model {
            host: Host { headroom_bytes: headroom },
            slots_free: slots,
        },
        cached_states_free: cache,
        serving: false,
        retries_under_mark: 0,
        make_room_attempts: 0,
    }
}

// -- shared contract tests (pass in BOTH arms) --------------------------------

const BAND: u64 = WARNING_MARGIN + (100 << 20); // inside the refusal band
const OUT: u64 = WARNING_MARGIN + (300 << 20); // past the optional floor

#[test]
fn eq_the_serving_floor_is_the_margin_the_optional_floor_adds_one_state() {
    let h = Host { headroom_bytes: 0 };
    assert_eq!(h.floor(false), WARNING_MARGIN + ONE_STATE);
    assert_eq!(h.floor(true), WARNING_MARGIN);
}

#[test]
fn eq_an_optional_class_is_never_retried_under_the_mark() {
    // A junction in the band: refused in both arms, and nothing retried --
    // junctions and checkpoints stay optional.
    let mut eng = eng_at(BAND, 4, 0);
    let c = eng.reserve_capture(StateClass::Optional);
    assert!(!c.buffers);
    assert_eq!(c.cause, AllocationFailure::HostPressure);
    assert_eq!(eng.retries_under_mark, 0);
}

#[test]
fn eq_a_budget_refusal_is_never_retried() {
    // The tier has no slot: the cause is Budget, and the host posture is
    // never the problem -- no retry in either arm.
    let mut eng = eng_at(OUT, 0, 0);
    let c = eng.reserve_capture(StateClass::InUse);
    assert!(!c.buffers);
    assert_eq!(c.cause, AllocationFailure::Budget);
    assert_eq!(eng.retries_under_mark, 0);
}

#[test]
fn eq_room_from_the_class_runs_before_any_refusal_is_judged() {
    // With a cached state to recycle, the reclaim frees the room and the
    // reservation lands plainly, outside the mark.
    let mut eng = eng_at(BAND, 4, 1);
    let c = eng.reserve_capture(StateClass::InUse);
    assert!(c.buffers);
    assert_eq!(eng.make_room_attempts, 1);
    assert_eq!(eng.model.host.headroom_bytes, BAND + ONE_STATE);
}

#[test]
fn eq_out_of_band_the_replay_point_publishes_plainly() {
    let mut eng = eng_at(OUT, 4, 0);
    let c = eng.reserve_capture(StateClass::InUse);
    assert!(c.buffers);
}

// -- the anti-pattern's own shape ---------------------------------------------

#[test]
fn bug_the_band_refuses_the_replay_point_and_the_transcript_replays_cold() {
    // The band with no cached state to recycle: the replay point is
    // refused for good -- every turn replays its whole prompt, the
    // cached-token ratio collapses to 0 (source-reported 0.103 on main).
    let mk = || Transcript {
        prompt_tokens: 24_000,
        replay_at: 18_000,
        turns: 8,
        cached_tokens: 0,
    };
    let cached = mk().run(BAND, 8, 0);
    assert_eq!(cached, 0);
    let t = Transcript { cached_tokens: cached, ..mk() };
    assert!(cached_ratio(&t) < 0.05);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `in-use-state-retry-under-serving-mark-keyed-on-refusal-cause`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/Engine.cpp +
// runtime/model/Model.hpp @ b7844dc (Apache-2.0): the law -- the prompt's
// replay point is work of the request in service. When the host refuses its
// buffers AND the refusal cause is HostPressure AND its class's reclaim
// room is exhausted, Engine::reserveCapture retries ONCE under the Serving
// mark -- the same posture the request's own KV is admitted under (down to
// the margin, not one state above it). Junctions and checkpoints stay
// optional: never retried. A Budget cause is never retried. A refusal
// judged outside the serving mark permanently refuses in the headroom band
// -- every turn replays its whole prompt (source-reported: cached-token
// ratio 0.103 -> 0.755 on an M6).

pub const GIB: u64 = 1 << 30;
pub const WARNING_MARGIN: u64 = GIB; // the 1 GiB warning margin
pub const ONE_STATE: u64 = 160 << 20; // one state's buffers, the 27B

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StateClass {
    /// The prompt's replay point: in-service work.
    InUse,
    /// A junction or a checkpoint: optional.
    Optional,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocationFailure {
    None,
    /// The host's own headroom refused the posture.
    HostPressure,
    /// The tier's own budget (no slot) refused.
    Budget,
}

/// The host stand-in: it refuses a reservation below the posture's floor.
pub struct Host {
    pub headroom_bytes: u64,
}

impl Host {
    /// A request in service (under the Serving mark) may spend down to the
    /// margin; any other posture must keep one state of headroom above it.
    /// The reservation IS one state: the floor check is the whole check.
    pub fn floor(&self, serving: bool) -> u64 {
        if serving {
            WARNING_MARGIN
        } else {
            WARNING_MARGIN + ONE_STATE
        }
    }

    pub fn admits(&self, serving: bool) -> AllocationFailure {
        if self.headroom_bytes >= self.floor(serving) {
            AllocationFailure::None
        } else {
            AllocationFailure::HostPressure
        }
    }
}

/// The model's reservation: the tier's slot budget first, then the host.
/// The refusal CAUSE is reported -- `reserveState` sets it, the caller may
/// release a cached state and retry.
pub struct Model {
    pub host: Host,
    pub slots_free: usize,
}

impl Model {
    pub fn reserve_state(&self, serving: bool) -> (bool, AllocationFailure) {
        if self.slots_free == 0 {
            return (false, AllocationFailure::Budget);
        }
        match self.host.admits(serving) {
            AllocationFailure::None => (true, AllocationFailure::None),
            cause => (false, cause),
        }
    }
}

pub struct Engine {
    pub model: Model,
    pub cached_states_free: usize, // what the InUse class's room may reclaim
    pub serving: bool,             // the Serving mark, raised inside the retry only
    pub retries_under_mark: usize,
    pub make_room_attempts: usize,
}

pub struct Capture {
    pub buffers: bool,
    pub cause: AllocationFailure,
}

/// RAII stand-in for `const Serving mark(config_.serving, true)`: the mark
/// is held for the retry's scope alone.
struct ServingMark<'a> {
    eng: &'a mut Engine,
}

impl<'a> ServingMark<'a> {
    fn raise(eng: &'a mut Engine) -> Self {
        eng.serving = true;
        ServingMark { eng }
    }
}

impl<'a> Drop for ServingMark<'a> {
    fn drop(&mut self) {
        self.eng.serving = false;
    }
}

impl Engine {
    /// makeStateRoom: reclaim one cached state at a time -- the class's own
    /// room -- until the take succeeds or nothing of the class is left.
    fn make_state_room(&mut self, take: &mut dyn FnMut(&mut Self) -> bool) -> bool {
        while self.cached_states_free > 0 {
            self.make_room_attempts += 1;
            self.cached_states_free -= 1;
            self.model.host.headroom_bytes += ONE_STATE; // the bytes come back
            if take(self) {
                return true;
            }
        }
        false
    }

    /// ReserveCapture for one capture point.
    pub fn reserve_capture(&mut self, kind: StateClass) -> Capture {
        let in_use = matches!(kind, StateClass::InUse);
        let (ok, mut cause) = self.model.reserve_state(self.serving);
        let mut buffers = ok;
        if !buffers {
            // room from what the class may take, before the refusal is judged
            let got = self.make_state_room(&mut |e: &mut Self| {
                let (ok, _) = e.model.reserve_state(e.serving);
                ok
            });
            if got {
                let (ok, c) = self.model.reserve_state(self.serving);
                buffers = ok;
                cause = c;
            }
        }
        if buffers {
            self.model.slots_free -= 1;
            return Capture {
                buffers: true,
                cause: AllocationFailure::None,
            };
        }
        // THE LAW: one retry under the Serving mark, keyed on the refusal
        // CAUSE (HostPressure) and the CLASS (InUse), after the class's
        // reclaim room is exhausted -- the rule the request's own KV runs
        // under. A junction or a checkpoint stays optional.
        if !buffers && in_use && cause == AllocationFailure::HostPressure {
            self.retries_under_mark += 1;
            let mark = ServingMark::raise(self);
            let (ok, c) = mark.eng.model.reserve_state(mark.eng.serving);
            drop(mark);
            if ok {
                self.model.slots_free -= 1;
                return Capture {
                    buffers: true,
                    cause: AllocationFailure::None,
                };
            }
            cause = c;
        }
        Capture { buffers, cause }
    }
}

/// The turn economy: a turn either publishes its replay point (the next
/// turn resumes from it) or replays its whole prompt.
pub struct Transcript {
    pub prompt_tokens: usize,
    pub replay_at: usize,
    pub turns: usize,
    pub cached_tokens: usize,
}

impl Transcript {
    /// Run `turns` turns over the static host posture `headroom`.
    pub fn run(mut self, headroom: u64, slots: usize, cache: usize) -> usize {
        let mut eng = Engine {
            model: Model {
                host: Host { headroom_bytes: headroom },
                slots_free: slots,
            },
            cached_states_free: cache,
            serving: false,
            retries_under_mark: 0,
            make_room_attempts: 0,
        };
        let mut published = false; // the prior turn's replay point, if any
        for _ in 0..self.turns {
            if published {
                // the prior turn published its replay point: this turn
                // resumes from it instead of replaying the whole prompt
                self.cached_tokens += self.replay_at;
            }
            let c = eng.reserve_capture(StateClass::InUse);
            published = c.buffers; // this turn's point serves the next one
        }
        self.cached_tokens
    }
}

pub fn cached_ratio(t: &Transcript) -> f64 {
    t.cached_tokens as f64 / ((t.turns - 1).max(1) * t.prompt_tokens) as f64
}

fn eng_at(headroom: u64, slots: usize, cache: usize) -> Engine {
    Engine {
        model: Model {
            host: Host { headroom_bytes: headroom },
            slots_free: slots,
        },
        cached_states_free: cache,
        serving: false,
        retries_under_mark: 0,
        make_room_attempts: 0,
    }
}

// -- shared contract tests (pass in BOTH arms) --------------------------------

const BAND: u64 = WARNING_MARGIN + (100 << 20); // inside the refusal band
const OUT: u64 = WARNING_MARGIN + (300 << 20); // past the optional floor

#[test]
fn eq_the_serving_floor_is_the_margin_the_optional_floor_adds_one_state() {
    let h = Host { headroom_bytes: 0 };
    assert_eq!(h.floor(false), WARNING_MARGIN + ONE_STATE);
    assert_eq!(h.floor(true), WARNING_MARGIN);
}

#[test]
fn eq_an_optional_class_is_never_retried_under_the_mark() {
    // A junction in the band: refused in both arms, and nothing retried --
    // junctions and checkpoints stay optional.
    let mut eng = eng_at(BAND, 4, 0);
    let c = eng.reserve_capture(StateClass::Optional);
    assert!(!c.buffers);
    assert_eq!(c.cause, AllocationFailure::HostPressure);
    assert_eq!(eng.retries_under_mark, 0);
    assert!(!eng.serving); // the mark never rose
}

#[test]
fn eq_a_budget_refusal_is_never_retried() {
    // The tier has no slot: the cause is Budget, and the host posture is
    // never the problem -- no retry in either arm.
    let mut eng = eng_at(OUT, 0, 0);
    let c = eng.reserve_capture(StateClass::InUse);
    assert!(!c.buffers);
    assert_eq!(c.cause, AllocationFailure::Budget);
    assert_eq!(eng.retries_under_mark, 0);
}

#[test]
fn eq_room_from_the_class_runs_before_any_refusal_is_judged() {
    // With a cached state to recycle, the reclaim frees the room and the
    // reservation lands plainly, outside the mark.
    let mut eng = eng_at(BAND, 4, 1);
    let c = eng.reserve_capture(StateClass::InUse);
    assert!(c.buffers);
    assert_eq!(eng.make_room_attempts, 1);
    assert_eq!(eng.retries_under_mark, 0); // room, not the mark, got them
    assert_eq!(eng.model.host.headroom_bytes, BAND + ONE_STATE);
}

#[test]
fn eq_out_of_band_the_replay_point_publishes_plainly() {
    let mut eng = eng_at(OUT, 4, 0);
    let c = eng.reserve_capture(StateClass::InUse);
    assert!(c.buffers);
}

// -- the law's own shape -------------------------------------------------------

#[test]
fn law_the_band_retries_once_under_the_mark_and_the_next_turn_resumes() {
    // The band with no cached state to recycle: the FIRST attempt is
    // refused, the class's room is exhausted (nothing reclaimed), and ONE
    // retry under the Serving mark -- the request's own KV posture --
    // admits it. The next turn resumes at the replay point (source
    // -reported ratio 0.755 on the M6; the fixture's shape reads 0.75).
    let mut eng = eng_at(BAND, 8, 0);
    let c = eng.reserve_capture(StateClass::InUse);
    assert!(c.buffers);
    assert_eq!(eng.retries_under_mark, 1); // ONE retry, no more
    assert_eq!(eng.make_room_attempts, 0); // the room was already exhausted
    assert!(!eng.serving); // the mark held for the retry's scope alone

    let mk = || Transcript {
        prompt_tokens: 24_000,
        replay_at: 18_000,
        turns: 8,
        cached_tokens: 0,
    };
    let cached = mk().run(BAND, 8, 0);
    assert_eq!(cached, 7 * 18_000); // every turn after the first resumes
    let t = Transcript { cached_tokens: cached, ..mk() };
    assert!((cached_ratio(&t) - 0.75).abs() < 1e-9);
}
"#.to_string(),
            context_notes: Some("Grade B+ -- NotBenchable ON THIS BOX (the decision content is an ADMISSION-POSTURE law -- which refusal causes and which state classes earn ONE retry under the Serving mark -- priced by the cached-token ratio of a real transcript; the source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ b7844dcf745909fb5caf79f82dd985d64041393b 'Admit the prompt's replay point as memory its request needs', Apache-2.0, Jian Chen, 2026-10-18; every quote re-verified verbatim at the .raw/b271/splash clone this batch at that pin; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (message, verbatim): 'The replay point's buffers were an optional allocation outside the Serving mark, so the host refused them unless its headroom stayed one state (160 MB on the 27B) above the 1 GiB warning margin, while a paced pass restores only the margin. With no cached state left to recycle, a headroom between the two refused every replay point, junction and checkpoint for good: each turn replayed its whole prompt, grew through the margin as a request in service, and left the host in the same band.' MEASURED (message, source-reported, the M6 on the 27B UD-IQ3_XXS agentic chat transcript, 8 requests): 'the two commits, ready 1.58 GB above the spare reserve, cached 0/448/5888/6048/17440/17600/21664/21824 (ratio 0.755, every replay point published, no state in use evicted). Main cached 0.661 at 1.78 GB in an earlier run and nothing at 1.18 GB just before this one; the #449-#470 stack, which behaves as main here, cached 0.103 at 1.53 GB.' THE LAW (runtime/engine/Engine.cpp:1879-1887 @ b7844dc, verbatim): 'The prompt's replay point is work of the request in service, the point its follow-up resumes from: once nothing its class may take is left, the host refuses it no more than that request's KV (allocate()). A junction or a checkpoint stays optional.' over 'if (!result.buffers && kind == StateClass::InUse && refusal == metal::AllocationFailure::HostPressure) { const Serving mark(config_.serving, true); result.buffers = model_.reserveState(id, tokens, refusal); }' -- ONE retry, keyed on the CLASS (InUse) and the CAUSE (HostPressure), after makeStateRoom (the class's reclaim room) already ran. THE ACCESSOR DOC (runtime/engine/Engine.hpp:651-653 @ b7844dc, verbatim): 'only if the pin has gone by then. The prompt's replay point that only the host refuses once room is made takes its buffers as memory the request in service needs (Serving). Null buffers when refused.' [FILE DRIFT, disclosed: the authoring brief attributed this text to Model.hpp:651-653 -- the line numbers are exact, the file is Engine.hpp; Model.hpp at this pin carries no such text.] THE CAUSE REPORTING the repair needs (runtime/model/Model.hpp:756-759 @ b7844dc, verbatim): 'Null when the governor refuses, which sets 'refusal' to the cause; the caller may release a cached state and retry.' over 'reserveState(uint64_t requestId, uint32_t tokens, metal::AllocationFailure &refusal) = 0;' -- 'Model::reserveState now reports the cause of a refusal, which QwenStateStorage::acquire already returned and reserve() dropped' (message). THE REPAIR (message, verbatim): 'Engine::reserveCapture retries a replay point's buffers once under the Serving mark when the host refused them and room from what its class may take did not get them, the rule allocate() applies to the same request's KV. A later paced pass takes the bytes back from older entries; the replay point is in use until its request ends and then the resume point, which that pass keeps. Junctions and checkpoints stay optional.' The upstream test names the posture: 'testReplayPointAdmittedInMarginBand starts the host in that band with no cached state; the first turn publishes its replay point and the next resumes from it (on main the point is refused and the next turn starts cold).' REGIME: state-buffer allocators on a host governor with a two-floor posture (a serving floor and a margin-plus-one-state floor) where a capture class is genuinely in-service work; any admission whose retry is refused permanently because BOTH the cause axis (host pressure vs a hard budget) and the class axis (in use vs optional) are ignored. ANTI-REGIMES: a Budget refusal (the retry cannot conjure a tier slot -- never retried); optional classes (the retry would turn a nice-to-have into a must-have -- never retried); hosts with ONE posture floor (both classes already share it -- nothing to retry differently); causes that never distinguish pressure from hard refusal. KINSHIP/DISTINCT-FROM (read + disclosed): reclaim-progress-keyed-on-refused-resource-returned (entries_41, B270) + refusal-wait-keyed-on-shortfall-returning-transfer (entries_41, B270) -- the same refusal family, distinct decisions: THOSE key the retry's WAIT / the wait-or-act call on WHICH resource class RETURNS (the release schedule); THIS keys the ADMISSION POSTURE of one immediate retry on the refusal CAUSE + the state CLASS -- what happens before any wait (the retry that gets the buffers NOW under the posture the request's own KV runs under); compose (this posture retry feeds the same refusal stream those key). mid-prefill-recurrent-state-capture-boundary-unsplit (kernel_opt entries_gpu_95, B236, same source repo) owns the GPU-COMMAND face of the same replay point (capture inside the command, never split the prefill at a boundary; buffers reserved before the command via Model::reserveState) -- THIS owns the HOST-side buffer admission of that reservation when the governor refuses it (they compose: the boundary-unsplit law created the reserve-before-command seam this retry lives in); the dedup grep hits on 'replay point|reserveState' are that rule, disclosed. discretionary-cache-growth-keys-on-spare-not-available (entries_26, B237, same repo) owns the GROWTH axis of the same host governor (spare vs available, the OS-warning stop) and its prose cites the upstream test name testRequestInServiceGrowsThroughHostPressure -- a different decision (discretionary growth), not the capture admission; disclosed as the one 'HostPressure' grep hit. pinned-entry-tombstone-freed-at-last-unpin (entries_36, B256) owns shared-entry HOLDER lifetime -- unrelated to admission posture. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'Serving mark|serving mark' -- zero owners; 'reserveCapture' -- zero owners; 'StateClass' -- zero owners; 'in-use-state|refusal-cause retry|serving-mark retry' -- zero owners. FIXTURE PROVENANCE: original machinery (never splash code) -- a two-floor host governor (serving floor = the 1 GiB warning margin, optional floor = margin + one 160 MB state) over a state-buffer tier with a slot budget (the Budget cause), a class-keyed reclaim room (make_state_room returns each reclaimed state's bytes), an RAII ServingMark held for the retry's scope alone, and a turn economy (a turn resumes from the PRIOR turn's published replay point or replays its whole prompt); the shared-contract tests (the floor arithmetic, optional never retried, Budget never retried, room before the mark, out-of-band plain publish) pass in BOTH arms; the discriminating tests pin the band (buggy: nothing published, cached 0, ratio < 0.05 vs fixed: ONE retry under the mark, the mark held for its scope alone, 7 resumes, ratio 0.75). Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: incoai/splash @ b7844dcf745909fb5caf79f82dd985d64041393b -- runtime/engine/Engine.cpp:1879-1887, runtime/engine/Engine.hpp:651-653, runtime/model/Model.hpp:756-759, commit message (all quotes); Batch 271 / Plan 275.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "one-event-per-writer-consumer-waits-on-the-side-it-needs".to_string(),
            name: "A progress event shared by two writers (the GPU signals ready, the CPU signals ran) is last-write-wins: a late signal from one side REGRESSES the other's newer value and the waiter stalls to its timeout (source-reported: about once in a million steps on an M5 Max, macOS 27 -- a 5 s Metal GPU timeout, seven buffers aborted at once); ONE event per side, values monotone per side, and the consumer waits on the event of the side whose completion it needs -- no write of one side puts back a value of the other".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `one-event-per-writer-consumer-waits-on-the-side-it-needs`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/metal/MetalBackend.mm @
// e2c0af2 (Apache-2.0): the host steps of one command (CPU work between GPU
// buffers) ordered by shared events. THE ANTI-PATTERN writes BOTH sides to
// ONE shared event: the GPU signals 2*i+1 once the work before step i has
// completed, the runner signals 2*i+2 once step i ran, and the buffer after
// the step waits for 2*i+2. A shared event holds the value written LAST,
// and a GPU signal can land after the runner's later value -- the event
// REGRESSES, and the waiter stalls to its timeout (source-reported: about
// once in a million steps on an M5 Max, macOS 27; a 5 s Metal GPU timeout,
// seven buffers aborted at once). The fixed arm gives each side its OWN
// event, values monotone per side, and the consumer waits on the event of
// the side whose completion it needs. The interleaving is simulated
// DETERMINISTICALLY: the hostile reorder (the GPU signal landing after the
// runner's later value) is applied explicitly, no threads.

pub const GPU_TIMEOUT_TICKS: u64 = 5; // the 5 s Metal timeout, on a virtual clock

/// ONE shared event for both sides: last write wins.
pub struct SharedEvent {
    pub value: u64,
}

impl SharedEvent {
    pub fn signal(&mut self, v: u64) {
        self.value = v; // holds the value written LAST
    }

    /// A buffer's encoded wait: does the event hold the value?
    pub fn holds(&self, v: u64) -> bool {
        self.value >= v
    }
}

#[derive(Default)]
pub struct Clock {
    pub now: u64,
}

impl Clock {
    /// The waiter stalls to the GPU timeout: five ticks burn, the buffer
    /// aborts.
    pub fn stall_to_timeout(&mut self) {
        self.now += GPU_TIMEOUT_TICKS;
    }
}

/// The command's host steps, encoded on the shared event.
pub struct Command {
    pub steps: usize,
    pub event: SharedEvent,
    pub timeouts: usize,
    pub aborted_buffers: usize,
    pub stalled_ticks: u64,
}

pub const BUFFERS_PER_TIMEOUT: usize = 7; // the measured abort fan-out

impl Command {
    pub fn new(steps: usize) -> Self {
        Command {
            steps,
            event: SharedEvent { value: 0 },
            timeouts: 0,
            aborted_buffers: 0,
            stalled_ticks: 0,
        }
    }

    /// One step, GOOD order: the GPU signals 2i+1 (the work before step i
    /// completed), the runner signals 2i+2 (step i ran), the buffer after
    /// the step waits for 2i+2.
    /// Returns false when the wait failed (the caller prices the stall).
    pub fn run_step_in_order(&mut self, clock: &mut Clock, i: usize) -> bool {
        let gpu = 2 * i as u64 + 1;
        let ran = 2 * i as u64 + 2;
        self.event.signal(gpu); // GPU: work before step i completed
        self.event.signal(ran); // runner: step i ran
        if !self.event.holds(ran) {
            // unreachable in the good order -- the waiter's guard
            clock.stall_to_timeout();
            self.timeouts += 1;
            self.aborted_buffers += BUFFERS_PER_TIMEOUT;
            self.stalled_ticks += GPU_TIMEOUT_TICKS;
            return false;
        }
        true
    }

    /// The HOSTILE reorder: the GPU's signal for step i lands AFTER the
    /// runner's later value for the same step. A shared event holds the
    /// value written LAST -- the event regresses to 2i+1.
    pub fn apply_late_gpu_signal(&mut self, i: usize) {
        self.event.signal(2 * i as u64 + 1);
    }

    /// The buffer after step i takes its encoded wait: 2i+2.
    pub fn buffer_after_waits(&mut self, clock: &mut Clock, i: usize) -> bool {
        let ran = 2 * i as u64 + 2;
        if self.event.holds(ran) {
            return true;
        }
        clock.stall_to_timeout();
        self.timeouts += 1;
        self.aborted_buffers += BUFFERS_PER_TIMEOUT;
        self.stalled_ticks += GPU_TIMEOUT_TICKS;
        false
    }
}

// -- shared contract tests (pass in BOTH arms) --------------------------------

#[test]
fn eq_a_command_in_order_completes_with_no_timeouts() {
    // 200 steps in the good order: every buffer's wait is satisfied, the
    // clock never burns a timeout tick.
    let mut cmd = Command::new(200);
    let mut clock = Clock::default();
    for i in 0..200 {
        assert!(cmd.run_step_in_order(&mut clock, i));
        assert!(cmd.buffer_after_waits(&mut clock, i));
    }
    assert_eq!(cmd.timeouts, 0);
    assert_eq!(cmd.aborted_buffers, 0);
    assert_eq!(clock.now, 0);
    assert_eq!(cmd.event.value, 400); // the last written value: 2*199+2
}

#[test]
fn eq_a_skipped_step_still_advances_the_waiter() {
    // A step SKIPPED still raises the runner's side (ran or was skipped),
    // so the buffer after it never waits on a step that did not run.
    let mut cmd = Command::new(3);
    let mut clock = Clock::default();
    // steps 0 and 2 run; step 1 is skipped -- its runner signal still lands
    assert!(cmd.run_step_in_order(&mut clock, 0));
    cmd.event.signal(2 * 1 + 2); // the skip's runner signal
    assert!(cmd.run_step_in_order(&mut clock, 2));
    for i in 0..3 {
        assert!(cmd.buffer_after_waits(&mut clock, i));
    }
    assert_eq!(cmd.timeouts, 0);
}

// -- the anti-pattern's own shape ---------------------------------------------

#[test]
fn bug_the_late_gpu_signal_regresses_the_shared_event_and_stalls_the_waiter() {
    // The measured shape: about once in a million steps the GPU's signal
    // for step 191 lands AFTER the runner's 384. The shared event holds
    // the value written LAST -- 383 -- and the buffer waiting for 384
    // stalls until Metal fails it after 5 s, seven buffers aborted at
    // once.
    let mut cmd = Command::new(200);
    let mut clock = Clock::default();
    for i in 0..191 {
        assert!(cmd.run_step_in_order(&mut clock, i));
        assert!(cmd.buffer_after_waits(&mut clock, i));
    }
    // step 191: the runner's value lands first, then the GPU's signal
    // arrives late -- the hostile interleaving, applied deterministically.
    assert!(cmd.run_step_in_order(&mut clock, 191));
    cmd.apply_late_gpu_signal(191);
    assert_eq!(cmd.event.value, 383); // REGRESSED from 384
    assert!(!cmd.buffer_after_waits(&mut clock, 191)); // the buffer after step 191
    assert_eq!(cmd.timeouts, 1);
    assert_eq!(cmd.aborted_buffers, BUFFERS_PER_TIMEOUT);
    assert_eq!(clock.now, GPU_TIMEOUT_TICKS); // five seconds burned
    assert_eq!(cmd.stalled_ticks, GPU_TIMEOUT_TICKS);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `one-event-per-writer-consumer-waits-on-the-side-it-needs`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/metal/MetalBackend.mm @
// e2c0af2 (Apache-2.0): the law -- each side of the host-step ordering
// writes its OWN event (the GPU raises readyEvent to i + 1 once the work
// before step i has completed; the runner raises ranEvent to i + 1 once
// step i ran or was skipped), values monotone per side, and the buffer
// after the step waits on the event of the side whose completion it needs
// (ranEvent). No write of one side can put back a value of the other: the
// hostile interleaving (the GPU signal landing after the runner's later
// value) lands on readyEvent and cannot touch what the waiter reads. The
// interleaving is simulated DETERMINISTICALLY, no threads.

pub const GPU_TIMEOUT_TICKS: u64 = 5; // the 5 s Metal timeout, on a virtual clock

/// ONE side's event: one writer, values monotone.
pub struct SideEvent {
    value: u64,
}

impl SideEvent {
    pub fn raise(&mut self, v: u64) {
        // monotone: a late signal of an older value moves nothing
        self.value = self.value.max(v);
    }

    pub fn holds(&self, v: u64) -> bool {
        self.value >= v
    }
}

/// The command's host steps: TWO events, one per side.
pub struct Command {
    pub steps: usize,
    pub ready: SideEvent, // the GPU's side: the work before step i completed
    pub ran: SideEvent,   // the runner's side: step i ran or was skipped
    pub timeouts: usize,
    pub aborted_buffers: usize,
    pub stalled_ticks: u64,
}

#[derive(Default)]
pub struct Clock {
    pub now: u64,
}

impl Clock {
    pub fn stall_to_timeout(&mut self) {
        self.now += GPU_TIMEOUT_TICKS;
    }
}

pub const BUFFERS_PER_TIMEOUT: usize = 7; // the measured abort fan-out

impl Command {
    pub fn new(steps: usize) -> Self {
        Command {
            steps,
            ready: SideEvent { value: 0 },
            ran: SideEvent { value: 0 },
            timeouts: 0,
            aborted_buffers: 0,
            stalled_ticks: 0,
        }
    }

    /// One step, GOOD order: the GPU raises readyEvent to i+1, the runner
    /// raises ranEvent to i+1, the buffer after the step waits for ranEvent
    /// at i+1 -- the event of the side whose completion it needs.
    pub fn run_step_in_order(&mut self, clock: &mut Clock, i: usize) -> bool {
        let target = i as u64 + 1;
        self.ready.raise(target); // GPU: the work before step i completed
        self.ran.raise(target); // runner: step i ran
        if !self.ran.holds(target) {
            // unreachable in the good order -- the waiter's guard
            clock.stall_to_timeout();
            self.timeouts += 1;
            self.aborted_buffers += BUFFERS_PER_TIMEOUT;
            self.stalled_ticks += GPU_TIMEOUT_TICKS;
            return false;
        }
        true
    }

    /// The HOSTILE reorder: the GPU's signal for step i lands AFTER the
    /// runner's value for the same step. It raises readyEvent -- ONE side,
    /// its own event -- and cannot touch ranEvent.
    pub fn apply_late_gpu_signal(&mut self, i: usize) {
        self.ready.raise(i as u64 + 1);
    }

    /// The buffer after step i takes its encoded wait: ranEvent at i+1.
    pub fn buffer_after_waits(&mut self, clock: &mut Clock, i: usize) -> bool {
        let target = i as u64 + 1;
        if self.ran.holds(target) {
            return true;
        }
        clock.stall_to_timeout();
        self.timeouts += 1;
        self.aborted_buffers += BUFFERS_PER_TIMEOUT;
        self.stalled_ticks += GPU_TIMEOUT_TICKS;
        false
    }
}

// -- shared contract tests (pass in BOTH arms) --------------------------------

#[test]
fn eq_a_command_in_order_completes_with_no_timeouts() {
    // 200 steps in the good order: every buffer's wait is satisfied, the
    // clock never burns a timeout tick.
    let mut cmd = Command::new(200);
    let mut clock = Clock::default();
    for i in 0..200 {
        assert!(cmd.run_step_in_order(&mut clock, i));
        assert!(cmd.buffer_after_waits(&mut clock, i));
    }
    assert_eq!(cmd.timeouts, 0);
    assert_eq!(cmd.aborted_buffers, 0);
    assert_eq!(clock.now, 0);
    assert!(cmd.ready.holds(200));
    assert!(cmd.ran.holds(200));
}

#[test]
fn eq_a_skipped_step_still_advances_the_waiter() {
    // A step SKIPPED still raises the runner's side (ran or was skipped),
    // so the buffer after it never waits on a step that did not run.
    let mut cmd = Command::new(3);
    let mut clock = Clock::default();
    assert!(cmd.run_step_in_order(&mut clock, 0));
    cmd.ran.raise(2); // the skip's runner signal
    assert!(cmd.run_step_in_order(&mut clock, 2));
    for i in 0..3 {
        assert!(cmd.buffer_after_waits(&mut clock, i));
    }
    assert_eq!(cmd.timeouts, 0);
}

// -- the law's own shape -------------------------------------------------------

#[test]
fn law_the_late_gpu_signal_lands_on_its_own_event_and_the_waiter_proceeds() {
    // The same hostile interleaving that regressed the shared event: the
    // GPU's signal for step 191 lands after the runner's value. Each side
    // owns its event, so the signal cannot put the waiter's value back --
    // the buffer after step 191 proceeds.
    let mut cmd = Command::new(200);
    let mut clock = Clock::default();
    for i in 0..191 {
        assert!(cmd.run_step_in_order(&mut clock, i));
        assert!(cmd.buffer_after_waits(&mut clock, i));
    }
    // step 191: the runner's value lands first, then the GPU's signal
    // arrives late.
    assert!(cmd.run_step_in_order(&mut clock, 191));
    cmd.apply_late_gpu_signal(191);
    assert!(cmd.ready.holds(192)); // the late signal landed on ITS side
    assert!(cmd.ran.holds(192)); // and moved nothing on the waiter's side
    assert!(cmd.buffer_after_waits(&mut clock, 191));
    assert_eq!(cmd.timeouts, 0);
    assert_eq!(cmd.aborted_buffers, 0);
    assert_eq!(clock.now, 0);
    // and the command runs on to completion
    for i in 192..200 {
        assert!(cmd.run_step_in_order(&mut clock, i));
        assert!(cmd.buffer_after_waits(&mut clock, i));
    }
    assert_eq!(cmd.timeouts, 0);
}

#[test]
fn law_each_side_is_monotone_under_any_interleaving() {
    // Under ANY interleaving -- the runner ahead, GPU signals late, the
    // whole hostile pattern applied at every step -- each side's value
    // never regresses, so every waiter observes a non-decreasing side.
    let mut cmd = Command::new(50);
    let mut clock = Clock::default();
    for i in 0..50 {
        // the hostile order, every step: runner first, GPU late
        cmd.ran.raise(i as u64 + 1);
        let before_ran = cmd.ran.holds(i as u64 + 1);
        cmd.apply_late_gpu_signal(i);
        assert!(cmd.ran.holds(i as u64 + 1) == before_ran);
        cmd.ready.raise(i as u64 + 1);
        assert!(cmd.buffer_after_waits(&mut clock, i));
    }
    assert_eq!(cmd.timeouts, 0);
    assert_eq!(clock.now, 0);
}
"#.to_string(),
            context_notes: Some("Grade B+ -- NotBenchable ON THIS BOX (the decision content is an EVENT-OWNERSHIP law -- which writer may signal which event and which side a consumer waits on -- priced by a ~1-in-a-million liveness failure; the source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ e2c0af27e8cb31ffd5990fac001b111e0894da93 'Write each host-step event from one side only', Apache-2.0, Jian Chen, 2026-10-18; every quote re-verified verbatim at the .raw/b271/splash clone this batch at that pin; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (message, verbatim): 'A command's host steps shared one event: the GPU raised it to 2i + 1 once the work before step i had completed, and the runner raised it to 2i + 2 once the step ran, which the buffer after the step waited for. A shared event holds the value written last, and on the Studio's M5 Max (macOS 27) a GPU signal can land after the runner's later value: the runner raised 192 after step 191, then read 191 again for 5 s while the buffer waiting for 192 stalled until Metal failed it ('Caused GPU Timeout Error', seven buffers aborted at once), about once in a million empty steps followed by an empty dispatch, as the last rounds of a short prefill's expert loads are. That is the 4-lane Flash-Next failure behind 'an expert load's round follows another block's load'.' THE LAW (message, verbatim): 'The GPU now raises a readyEvent to i + 1 and the runner a ranEvent to i + 1, which the buffer after the step waits for, so no write of one side puts back a value of the other.' THE IN-SOURCE COMMENT (runtime/metal/MetalBackend.mm:497-505 @ e2c0af2, verbatim): 'The host steps of one command (EventStep::Kind::Host), which the backend raises through two events of their own, each written by one side: the GPU raises readyEvent to i + 1 once the work before step i has completed, and the runner raises ranEvent to i + 1 once step i ran or was skipped, which the work after it waits for. A shared event holds the value written last, and a GPU signal can land after a later value the CPU wrote to the same event (M5 Max, macOS 27: about once in a million steps), which puts it back and holds the work waiting for the CPU's value until Metal fails it after 5 s ('Caused GPU Timeout Error').' [LINE DRIFT, disclosed: the authoring brief cited :498-502 -- the quoted text spans :497-505 at the pin.] THE TWO EVENTS (MetalBackend.mm:530 @ e2c0af2, verbatim code): '__strong id<MTLSharedEvent> readyEvent = nil, ranEvent = nil;' -- TWO events replacing one. THE SIGNAL AND THE WAIT (MetalBackend.mm:1173 + :1185 @ e2c0af2, verbatim code): '[command encodeSignalEvent:steps->readyEvent value:ready];' / '[next encodeWaitForEvent:steps->ranEvent value:ready];' -- the GPU's signal rides readyEvent, the buffer after the step waits on ranEvent: the consumer waits on the event of the side whose completion it needs, never on the other side's. REGIME: cross-side progress signaling between two writers (GPU/CPU, device/host, producer threads) through value-carrying monotone events where consumers wait on ordered values; any shared signal counter both sides write. ANTI-REGIMES: single-writer events (no second side to regress); boolean edge events (an edge cannot regress -- the value-carrying monotone event is the hazard); waits that recompute freshness from scratch instead of reading a stored last-write. KINSHIP/DISTINCT-FROM (read + disclosed): two-pass-partial-reduction-inter-dispatch-barrier (kernel_opt B269, same repo) owns the GPU-GPU face -- stale partials BETWEEN dispatches fixed by an inter-dispatch MEMORY BARRIER; THIS owns the GPU-CPU face -- a shared EVENT's value regressing, fixed by OWNERSHIP (one event per side, monotone per side), no barrier involved: different side pair, different medium (barrier semantics vs event monotonicity), the same 'a late write from the wrong side unorders a consumer' instinct. watchdog-deadline-moves-on-command-progress (entries_41, B270, same repo) CONSUMES these host-step events (the deadline moves at each host step 'as the GPU work before it ends and as it returns') -- a consumer of the progress signal, not the event-ownership law; compose (the regressed event starves the BUFFER wait, not the watchdog). staging-packed-uniforms-encoder-ordered-copies + identical-uniforms-share-single-slot (kernel_opt entries_gpu_02, B27) carry the 'last-write-wins' vocabulary for wgpu uniform-buffer flush order -- buffer writes on an encoder timeline, not shared-event signal values; the dedup grep hits land there, disclosed (different medium, different law: packed staging copies vs per-side monotone events). pinned-entry-tombstone-freed-at-last-unpin (entries_36, B256) owns holder LIFETIME -- unrelated. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'readyEvent|ranEvent|MTLSharedEvent|encodeSignalEvent|encodeWaitForEvent' -- zero owners; 'host step' -- entries_41's watchdog only (the consumer, disclosed above); 'last-write-wins' -- the wgpu uniform-buffer family only (disclosed above); 'one event per writer|per-side monotone' -- zero owners. FIXTURE PROVENANCE: original machinery (never splash code) -- a command of 200 host steps over a virtual clock with the hostile interleaving applied DETERMINISTICALLY (no threads): the buggy arm is ONE SharedEvent -- the GPU signals 2i+1, the runner 2i+2, the buffer after the step waits for 2i+2, last write wins, and the late GPU signal regresses 384 -> 383; the fixed arm is TWO SideEvents (ready/ran), each raised by one side with monotone values, the waiter reading ranEvent only. Shared-contract tests (200 in-order steps complete with zero timeouts; a skipped step still advances the waiter) pass in BOTH arms; the discriminating tests pin the reorder (buggy: the wait fails, 1 timeout, 5 clock ticks burned, 7 buffers aborted vs fixed: the late signal lands on its own side, the waiter proceeds, the command completes) and the monotonicity law under a per-step hostile pattern. Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: incoai/splash @ e2c0af27e8cb31ffd5990fac001b111e0894da93 -- runtime/metal/MetalBackend.mm:497-505 + :530 + :1173 + :1185, commit message (all quotes); Batch 271 / Plan 275.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
