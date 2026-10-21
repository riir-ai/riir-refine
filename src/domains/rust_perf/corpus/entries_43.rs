//! Entries slice A - Batch 271 (2026-10-21, Plan 275 / Research 307): the
//! splash-wave second-tier serving pool, slice A (3 allocation rules) -
//! incoai/splash @ fa63d97f8956113b067dfdce46caf1fcd22213e1 (+ the
//! count-neutral rider 593f2a6d7427382a88a474a6a27c6dcc61b7415e),
//! 88d0e57ee3a37305a895197538554ba9f3ea83d8,
//! 81ae003e18d9362f6adf4131cf88a17a17803f65, Apache-2.0; every quote
//! re-verified at the .raw/b271/splash clone this batch. All three rules are
//! corpus-only `NotBenchable` (decision-content serving laws; the
//! source-reported figures ride each `context_notes`) - no proposer files.
//! Fixtures are ORIGINAL machinery (never splash code verbatim), each
//! compiled and executed standalone (rustc --edition 2021 --test) before
//! this table was assembled; the buggy/fixed arm pairs share contract tests
//! and differ only on the law.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "resume-admission-sized-to-per-command-window-not-suspended-peak".to_string(),
            name: "a suspended lane's resume admission sizes from the KV target its suspension recorded -- the PRE-SUSPENSION PEAK -- so a Spill lane's replay (which need only hold its dense history, its tail window and one command page: the replay's other pages spill as their copies land) asks for every missing page as Full at once and is refused with capacity_exhausted every time -- alone it fails after streaming its output, beside other lanes it holds back the suspended requests after it, and a Resident lane that converts to Spill mid-allocation retries the same Full ask and fails the same way → size the resume ask by the lane's MODE AT EACH ATTEMPT: a Resident resume takes its held target at once; a spill-capable resume (a converting Resident included) takes the first work unit's page past the restored chain and its replay takes Full pages a command at a time (prepare), which spill as their copies land -- the ask follows the mode, so a conversion retries with the smaller one".to_string(),
            category: "allocation".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `resume-admission-sized-to-per-command-window-not-suspended-peak`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/engine/Engine.cpp @ fa63d97
// (Apache-2.0): a paged lane KV over a RAM page pool, where a lane suspended
// for a higher priority or for growth resumes with the KV target its
// suspension recorded. THE ANTI-PATTERN: the resume took that target in one
// ensure grant, WHICH ADMITS EVERY MISSING PAGE AS A FULL PAGE, whatever the
// lane's mode -- a Spill lane keeps only its dense history and tail window
// Full (its other pages spill as their copies land), so its pre-suspension
// peak is memory it need never hold again; the ask fails with
// capacity_exhausted every time, and a Resident lane that converts to Spill
// mid-allocation retries the same Full ask and fails the same way. The LAW
// arm keys the ask on the lane's MODE AT EACH ATTEMPT.
//
// ORIGINAL machinery -- never splash code verbatim. Geometry: PAGE_TOKENS
// tokens per page; the pool counts Full pages; a lane's restored chain sits
// in the pool at resume time (the dense pages a Spill lane keeps), the
// replay runs `replay_pages` of work one command (one page) at a time; a
// Spill replay's page spills as its copy lands (the RAM page is freed the
// same step), a Resident replay writes into pages the admission granted.

pub const PAGE_TOKENS: u64 = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KvMode {
    Resident,
    Spill,
}

pub struct Pool {
    pub pages: usize,
    pub used: usize,
    pub peak: usize,
    pub granted: usize,        // pages granted since the resume began
    pub cleared: usize,        // granted pages that came cleared (past the chain)
    pub capacity_exhausted: usize,
}

impl Pool {
    pub fn new(pages: usize, used: usize) -> Self {
        Pool {
            pages,
            used,
            peak: used,
            granted: 0,
            cleared: 0,
            capacity_exhausted: 0,
        }
    }

    pub fn free(&self) -> usize {
        self.pages - self.used
    }

    // the ensure grant: admit every missing page as a Full page at once.
    // Pages past the restored chain come cleared (a replay's chunks may end
    // inside any of them).
    pub fn ensure_full_pages(&mut self, missing: usize) -> bool {
        if self.used + missing > self.pages {
            self.capacity_exhausted += 1;
            return false;
        }
        self.used += missing;
        self.peak = self.peak.max(self.used);
        self.granted += missing;
        self.cleared += missing;
        true
    }

    // prepare: one command page taken for the replay; it spills as its copy
    // lands (the RAM page is freed the same step).
    pub fn take_and_spill(&mut self) -> bool {
        if self.used + 1 > self.pages {
            self.capacity_exhausted += 1;
            return false;
        }
        self.used += 1;
        self.peak = self.peak.max(self.used);
        self.used -= 1;
        true
    }
}

pub struct Lane {
    pub mode: KvMode,
    pub dense_full: usize,             // restored chain pages held Full now
    pub suspended_target_pages: usize, // the KV target the suspension recorded
    pub boundary_page: usize,          // the first work unit's page past the chain
    pub replay_pages: usize,           // the replay's work, in command pages
    pub full: usize,                   // pages the lane holds Full right now
    pub prepared: usize,
    pub completed: bool,
}

// the replay: a Resident lane writes into the pages the admission granted; a
// Spill lane's replay takes Full pages A COMMAND AT A TIME (prepare), which
// spill as their copies land -- the granted window's page serves the first
// command.
fn replay(pool: &mut Pool, lane: &mut Lane) -> bool {
    let mut window_served = false;
    for _ in 0..lane.replay_pages {
        if lane.mode == KvMode::Spill {
            if window_served {
                if !pool.take_and_spill() {
                    return false;
                }
            } else {
                window_served = true; // the first work's page, granted at the ask
            }
        } // a Resident replay writes into granted pages
        lane.prepared += 1;
    }
    lane.completed = true;
    true
}

// THE ANTI-PATTERN: the ask is the suspended target, whatever the mode; a
// conversion retries the same ask.
pub fn resume_buggy(pool: &mut Pool, lane: &mut Lane) -> bool {
    loop {
        let missing = lane.suspended_target_pages.saturating_sub(lane.full);
        if pool.ensure_full_pages(missing) {
            lane.full += missing;
            return replay(pool, lane);
        }
        if lane.mode == KvMode::Resident {
            lane.mode = KvMode::Spill; // the spill fallback converts -- the ask is unchanged
            continue;
        }
        return false;
    }
}

// THE LAW: the ask follows the lane's MODE at each attempt -- a Resident
// resume takes its held target at once; a spill-capable resume takes the
// first work unit's page past the restored chain (resumeBoundary + 1).
pub fn resume_fixed(pool: &mut Pool, lane: &mut Lane) -> bool {
    loop {
        let work_end_tokens = if lane.mode == KvMode::Resident {
            lane.suspended_target_pages as u64 * PAGE_TOKENS
        } else {
            lane.boundary_page as u64 * PAGE_TOKENS + 1
        };
        let target_pages = ((work_end_tokens + PAGE_TOKENS - 1) / PAGE_TOKENS) as usize;
        let missing = target_pages.saturating_sub(lane.full);
        if pool.ensure_full_pages(missing) {
            lane.full += missing;
            return replay(pool, lane);
        }
        if lane.mode == KvMode::Resident {
            lane.mode = KvMode::Spill; // a Resident one that converts retries smaller
            continue;
        }
        return false;
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_a_resident_resume_with_room_takes_its_held_target() {
    // a Resident resume whose target still fits takes it at once -- both
    // arms agree (the law keeps the Resident form)
    let mut pool = Pool::new(128, 64);
    let mut lane = Lane {
        mode: KvMode::Resident,
        dense_full: 64,
        suspended_target_pages: 80,
        boundary_page: 64,
        replay_pages: 16,
        full: 64,
        prepared: 0,
        completed: false,
    };
    assert!(resume_buggy(&mut pool, &mut lane));
    assert!(lane.completed);
    assert_eq!(lane.full, 80);
    assert_eq!(lane.prepared, 16);
    assert_eq!(pool.capacity_exhausted, 0);
    assert_eq!(pool.used, 80);
}

#[test]
fn eq_a_start_waits_on_one_cleared_page_past_its_restored_chain() {
    // the START posture both arms share: a start that waits for its restore
    // holds the chain and one page past it, and the past-chain pages come
    // cleared (a replay's chunks may end inside any of them)
    let mut pool = Pool::new(12, 10);
    assert!(pool.ensure_full_pages(1));
    assert_eq!(pool.cleared, 1); // the past-chain page came cleared
    assert_eq!(pool.used, 11);
    assert_eq!(pool.free(), 1);
}

// -- the anti-pattern's own shape --------------------------------------------

#[test]
fn bug_a_spill_resume_fails_capacity_exhausted_every_time() {
    // the measured face: a Spill lane suspended at 125 pages exceeds the 100
    // the memory holds Full -- the resume asks for every missing page as
    // Full at once and is refused, though its replay needs only the dense
    // chain plus one command page at a time
    let mut pool = Pool::new(100, 96);
    let mut lane = Lane {
        mode: KvMode::Spill,
        dense_full: 96,
        suspended_target_pages: 125,
        boundary_page: 96,
        replay_pages: 32,
        full: 96,
        prepared: 0,
        completed: false,
    };
    assert!(!resume_buggy(&mut pool, &mut lane));
    assert!(!lane.completed);
    assert_eq!(lane.prepared, 0);
    assert_eq!(pool.capacity_exhausted, 1);
    assert_eq!(pool.used, 96); // nothing was granted
}

#[test]
fn bug_a_converting_resident_retries_the_same_full_ask() {
    // a Resident lane suspended with 80 pages resumes after memory shrank to
    // 72: the Resident ask is refused, the lane converts to Spill -- and the
    // unchanged ask asks for the same Full pages and fails again
    let mut pool = Pool::new(72, 64);
    let mut lane = Lane {
        mode: KvMode::Resident,
        dense_full: 64,
        suspended_target_pages: 80,
        boundary_page: 64,
        replay_pages: 16,
        full: 64,
        prepared: 0,
        completed: false,
    };
    assert!(!resume_buggy(&mut pool, &mut lane));
    assert!(!lane.completed);
    assert_eq!(lane.mode, KvMode::Spill); // it converted -- and failed anyway
    assert_eq!(pool.capacity_exhausted, 2);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `resume-admission-sized-to-per-command-window-not-suspended-peak`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/Engine.cpp @ fa63d97
// (Apache-2.0): the resume's ask follows the lane's MODE AT EACH ATTEMPT --
// a Resident lane that resumes takes the KV target it was suspended with at
// once; a Spill lane, a Resident one that converts here included, takes the
// first work's page, as a start that waits for its restore does: its replay
// takes Full pages a command at a time (prepare), which spill as their
// copies land. The target follows the mode, so a conversion retries with the
// smaller one. Pages past the restored chain still come cleared.
//
// ORIGINAL machinery -- never splash code verbatim. Geometry: PAGE_TOKENS
// tokens per page; the pool counts Full pages; a lane's restored chain sits
// in the pool at resume time (the dense pages a Spill lane keeps), the
// replay runs `replay_pages` of work one command (one page) at a time; a
// Spill replay's page spills as its copy lands (the RAM page is freed the
// same step), a Resident replay writes into pages the admission granted.

pub const PAGE_TOKENS: u64 = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KvMode {
    Resident,
    Spill,
}

pub struct Pool {
    pub pages: usize,
    pub used: usize,
    pub peak: usize,
    pub granted: usize,        // pages granted since the resume began
    pub cleared: usize,        // granted pages that came cleared (past the chain)
    pub capacity_exhausted: usize,
}

impl Pool {
    pub fn new(pages: usize, used: usize) -> Self {
        Pool {
            pages,
            used,
            peak: used,
            granted: 0,
            cleared: 0,
            capacity_exhausted: 0,
        }
    }

    pub fn free(&self) -> usize {
        self.pages - self.used
    }

    // the ensure grant: admit every missing page as a Full page at once.
    // Pages past the restored chain come cleared (a replay's chunks may end
    // inside any of them).
    pub fn ensure_full_pages(&mut self, missing: usize) -> bool {
        if self.used + missing > self.pages {
            self.capacity_exhausted += 1;
            return false;
        }
        self.used += missing;
        self.peak = self.peak.max(self.used);
        self.granted += missing;
        self.cleared += missing;
        true
    }

    // prepare: one command page taken for the replay; it spills as its copy
    // lands (the RAM page is freed the same step).
    pub fn take_and_spill(&mut self) -> bool {
        if self.used + 1 > self.pages {
            self.capacity_exhausted += 1;
            return false;
        }
        self.used += 1;
        self.peak = self.peak.max(self.used);
        self.used -= 1;
        true
    }
}

pub struct Lane {
    pub mode: KvMode,
    pub dense_full: usize,             // restored chain pages held Full now
    pub suspended_target_pages: usize, // the KV target the suspension recorded
    pub boundary_page: usize,          // the first work unit's page past the chain
    pub replay_pages: usize,           // the replay's work, in command pages
    pub full: usize,                   // pages the lane holds Full right now
    pub prepared: usize,
    pub completed: bool,
}

// the replay: a Resident lane writes into the pages the admission granted; a
// Spill lane's replay takes Full pages A COMMAND AT A TIME (prepare), which
// spill as their copies land -- the granted window's page serves the first
// command.
fn replay(pool: &mut Pool, lane: &mut Lane) -> bool {
    let mut window_served = false;
    for _ in 0..lane.replay_pages {
        if lane.mode == KvMode::Spill {
            if window_served {
                if !pool.take_and_spill() {
                    return false;
                }
            } else {
                window_served = true; // the first work's page, granted at the ask
            }
        } // a Resident replay writes into granted pages
        lane.prepared += 1;
    }
    lane.completed = true;
    true
}

// THE ANTI-PATTERN: the ask is the suspended target, whatever the mode; a
// conversion retries the same ask.
pub fn resume_buggy(pool: &mut Pool, lane: &mut Lane) -> bool {
    loop {
        let missing = lane.suspended_target_pages.saturating_sub(lane.full);
        if pool.ensure_full_pages(missing) {
            lane.full += missing;
            return replay(pool, lane);
        }
        if lane.mode == KvMode::Resident {
            lane.mode = KvMode::Spill; // the spill fallback converts -- the ask is unchanged
            continue;
        }
        return false;
    }
}

// THE LAW: the ask follows the lane's MODE at each attempt -- a Resident
// resume takes its held target at once; a spill-capable resume takes the
// first work unit's page past the restored chain (resumeBoundary + 1).
pub fn resume_fixed(pool: &mut Pool, lane: &mut Lane) -> bool {
    loop {
        let work_end_tokens = if lane.mode == KvMode::Resident {
            lane.suspended_target_pages as u64 * PAGE_TOKENS
        } else {
            lane.boundary_page as u64 * PAGE_TOKENS + 1
        };
        let target_pages = ((work_end_tokens + PAGE_TOKENS - 1) / PAGE_TOKENS) as usize;
        let missing = target_pages.saturating_sub(lane.full);
        if pool.ensure_full_pages(missing) {
            lane.full += missing;
            return replay(pool, lane);
        }
        if lane.mode == KvMode::Resident {
            lane.mode = KvMode::Spill; // a Resident one that converts retries smaller
            continue;
        }
        return false;
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_a_resident_resume_with_room_takes_its_held_target() {
    // a Resident resume whose target still fits takes it at once -- both
    // arms agree (the law keeps the Resident form)
    let mut pool = Pool::new(128, 64);
    let mut lane = Lane {
        mode: KvMode::Resident,
        dense_full: 64,
        suspended_target_pages: 80,
        boundary_page: 64,
        replay_pages: 16,
        full: 64,
        prepared: 0,
        completed: false,
    };
    assert!(resume_fixed(&mut pool, &mut lane));
    assert!(lane.completed);
    assert_eq!(lane.full, 80);
    assert_eq!(lane.prepared, 16);
    assert_eq!(pool.capacity_exhausted, 0);
    assert_eq!(pool.used, 80);
}

#[test]
fn eq_a_start_waits_on_one_cleared_page_past_its_restored_chain() {
    // the START posture both arms share: a start that waits for its restore
    // holds the chain and one page past it, and the past-chain pages come
    // cleared (a replay's chunks may end inside any of them)
    let mut pool = Pool::new(12, 10);
    assert!(pool.ensure_full_pages(1));
    assert_eq!(pool.cleared, 1); // the past-chain page came cleared
    assert_eq!(pool.used, 11);
    assert_eq!(pool.free(), 1);
}

// -- the law's own shape -----------------------------------------------------

#[test]
fn law_a_spill_resume_replays_a_command_at_a_time() {
    // the same soak the anti-pattern arm dies in: a Spill lane suspended at
    // 125 pages over a 100-page pool -- the ask is the first work's page
    // past the restored chain, and the replay admits Full pages a command at
    // a time, which spill as their copies land
    let mut pool = Pool::new(100, 96);
    let mut lane = Lane {
        mode: KvMode::Spill,
        dense_full: 96,
        suspended_target_pages: 125,
        boundary_page: 96,
        replay_pages: 32,
        full: 96,
        prepared: 0,
        completed: false,
    };
    assert!(resume_fixed(&mut pool, &mut lane));
    assert!(lane.completed);
    assert_eq!(lane.prepared, 32);
    assert_eq!(pool.capacity_exhausted, 0);
    assert_eq!(lane.full, 97); // the chain + the first work's page
    assert!(pool.peak <= pool.pages);
}

#[test]
fn law_a_conversion_retries_with_the_smaller_ask() {
    // a Resident lane suspended with 80 pages resumes after memory shrank to
    // 72: the Resident ask is refused once, the lane converts to Spill, and
    // the SMALLER ask (the first work's page) is admitted -- it completes
    let mut pool = Pool::new(72, 64);
    let mut lane = Lane {
        mode: KvMode::Resident,
        dense_full: 64,
        suspended_target_pages: 80,
        boundary_page: 64,
        replay_pages: 16,
        full: 64,
        prepared: 0,
        completed: false,
    };
    assert!(resume_fixed(&mut pool, &mut lane));
    assert!(lane.completed);
    assert_eq!(lane.mode, KvMode::Spill);
    assert_eq!(pool.capacity_exhausted, 1); // the Resident ask, refused once
    assert_eq!(lane.full, 65); // the chain + the first work's page
    assert_eq!(lane.prepared, 16);
    assert!(pool.peak <= pool.pages);
}

#[test]
fn law_a_resident_resume_still_takes_its_held_target() {
    // a Resident lane still resumes with its target, the memory it held --
    // with room, the law keeps the Resident form at once, no spill
    let mut pool = Pool::new(128, 64);
    let mut lane = Lane {
        mode: KvMode::Resident,
        dense_full: 64,
        suspended_target_pages: 80,
        boundary_page: 64,
        replay_pages: 16,
        full: 64,
        prepared: 0,
        completed: false,
    };
    assert!(resume_fixed(&mut pool, &mut lane));
    assert_eq!(lane.mode, KvMode::Resident);
    assert_eq!(lane.full, 80);
    assert_eq!(pool.capacity_exhausted, 0);
}
"#.to_string(),
            context_notes: Some("Grade B -- NotBenchable ON THIS BOX (the decision content is the RESUME-ADMISSION ECONOMICS of a paged serving lane -- which pages a suspended lane's re-admission takes at each attempt, keyed on its KV mode -- and a Rust microbench of the stand-in cannot measure it; source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ fa63d97f8956113b067dfdce46caf1fcd22213e1 'Take a Spill resume's replay pages a command at a time', Apache-2.0, Jian Chen, 2026-10-19; every quote re-verified verbatim at the .raw/b271/splash clone this batch at those pins; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (commit message, verbatim): 'A lane suspended for a higher priority or for growth resumes with the KV target suspendLane recorded ... admit() took that target in one ensureTokens call, which admits every missing page as a Full page.' -- and the failure (verbatim): 'Such a resume was refused every time: alone, it failed with capacity_exhausted (HTTP 400, 'restart the server with a larger --max-memory') after streaming its output, and with a restore pending it first gave up its cached prefix and asked for its whole history; beside other lanes it waited, holding back the suspended requests after it, until they ended, then failed the same way. A Resident lane that converts to Spill in that allocation, as memory refuses its target, asked again for the same Full pages and failed the same way.' MEASURED (message, source-reported): 'Tests: engine: a Spill request with 4,096 tokens of output, suspended for a Normal request after 3,840, when its 125 pages exceed the 100 memory holds Full, resumes and completes; a Resident request suspended with 80 Full pages resumes after memory shrank to 72, converts and completes. Before this change both failed with capacity_exhausted (additional_pages=121, free_pages=95; additional_pages=80, free_pages=72).' THE LAW (runtime/engine/Engine.cpp:862-874 @ fa63d97, verbatim): 'Its new pages past the chain it restored come cleared: a resume's replay writes them in chunks that may end inside any of them. A Resident lane that resumes takes the KV target it was suspended with at once. A Spill lane, a Resident one that converts here included, takes the first work's page, as a start that waits for its restore does: its replay takes Full pages a command at a time (prepare), which spill as their copies land, as its output's did.' over 'const auto workEnd = [&] { return resuming && active.kvMode == KvMode::Resident ? active.resumeKvTargetTokens : uint64_t{resumeBoundary} + 1; };' -- the ask follows the lane's MODE AT EACH ATTEMPT ('The target follows the lane's mode at each attempt, so a conversion retries with the smaller one.' -- message, verbatim), and the Resident form is KEPT: 'A Resident lane still resumes with its target, the memory it held.' (message). THE REPAIR: the resume's ensureTokens ask is the workEnd lambda above -- a Resident resume takes resumeKvTargetTokens (the memory it held) at once; a spill-capable resume takes resumeBoundary + 1 (the first work unit's page past the restored chain) and the replay admits Full pages a command at a time (prepare), which spill as their copies land; the pages past the restored chain still come cleared, 'a restoring start's one page as before' (message). REGIME: paged/suspended serving lanes whose re-admission sizes from a recorded peak (a suspend-time KV target) while the lane's steady-state residency is smaller by its mode (spilled history, per-command staging); any resume/retry path that replays a recorded allocation target without re-deriving it from the lane's CURRENT form. ANTI-REGIMES: lanes whose peak IS their residency (the Resident arm of the law keeps the held target); resumes whose replay publishes states so every replayed page must stay Full (the source's premise is the opposite: 'decode publishes none', which is what lets the Spill replay spill); non-paged engines. KINSHIP/DISTINCT-FROM (read + disclosed): first-message-sized-to-smallest-job (B257, entries_37) is the WIRE-PLANE twin -- a decode step's first message sized to the smallest job (one message, not 16 rows of padding) -- THIS is the STORAGE-plane twin: the resume's first grant sized to the first work unit's PAGE, not the suspended peak; compose in the grant-shaping family. work-buffer-sized-by-execution-width-not-plan-tasks (B237, entries_25) owns sizing a work buffer by the width actually served rather than the plan's task count -- the same size-from-the-serving-shape family, different surface (a scratch buffer vs a resume admission). prefix-cache-entry-one-token-early (B200, entries_13) is the resume-BOUNDARY twin for the RIDER face below -- it keeps a cache entry one token short so a strictly-extending resume always has somewhere to go; the rider bounds a resume's OUTPUT by the anchor the model holds at the boundary. RIDER (Batch 271 / Plan 275, incoai/splash @ 593f2a6d7427382a88a474a6a27c6dcc61b7415e 'Bound a Spill resume's output only past the anchor it holds', Apache-2.0, 2026-10-19 - the OUTPUT-bound face of the same resume path): when the SSD cache's quota bounds a resumed Spill lane's output to the pages it holds, the bound must count the anchor the model already selected but has not emitted -- 'const uint64_t held = active.exactTokens.size() + (active.suspendedWithAnchor ? 1 : 0);' (Engine.cpp:1007 @ 593f2a6) under the comment 'A resume counts among them the anchor its model holds once it selected the request's first token (Request::suspendedWithAnchor), which the model decodes only with output left past it: it emits a last budgeted anchor as it selects it.' (Engine.cpp:1001-1006); the flag is recorded at suspend (Engine.cpp:2149-2158 @ fa63d97, verbatim): 'A decoding lane holds an anchor unless its first mask has yet to select one; a lane in prefill holds what it held when its replay began.' -- the task-staged cite :2149-2153 is the suspendLane SIGNATURE range; the comment sits at :2154-2155, the assignment :2157-2158 (drift disclosed). THE RIDER DEFECT (message, verbatim): 'With prompt and output 32q + 31 tokens long and room for exactly the pages they need, the bound was the anchor itself, one token past the output. ... the resumed lane's first decode threw out of Engine::tick: engine_execution_failed for every request in service, a relaunch, and within probation an emptied cache.' -- a bound from committed output alone leaves the lane below its minimum progress unit and fails it; the repair counts the anchor among the tokens the request holds ('A resume now counts the anchor among the tokens it holds where its request holds one, which suspendLane records from what the lane ran (Request::suspendedWithAnchor)'), leaving the resume 'at least two tokens past its output, else it waits for that room'. Count-neutral rider; the fixture does not model the bound (the admission law is rule-sized). DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'resume admission|resumeKvTarget|suspended peak|suspendedWithAnchor|resumeBoundary|replay pages|admits every missing page|pre-suspension peak' -- zero owners. FIXTURE PROVENANCE: original machinery (never splash code) -- a paged lane over a RAM page pool (PAGE_TOKENS = 32): a lane carries its mode, the restored chain (its dense Full pages), the suspend-time KV target, the resume boundary (the first work unit's page past the chain), and a replay workload; the buggy arm's resume asks the SUSPENDED TARGET in one grant whatever the mode and a conversion retries the same ask; the fixed arm's ask follows the mode at each attempt and the Spill replay takes one command page at a time (prepare + spill-as-copies-land, the granted window's page serving the first command); the shared-contract tests (a Resident resume with room takes its target; a start waits on one cleared page past its restored chain) pass in BOTH arms; the discriminating tests pin the two capacity_exhausted failures (buggy: the Spill resume refused at 125 over a 100-page pool, the converted Resident refused twice) and the three law outcomes (the Spill resume completes a command at a time at peak 97 <= 100, the conversion retries smaller and completes at peak 66 <= 72, the roomy Resident still takes its held target). Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: @ fa63d97f8956113b067dfdce46caf1fcd22213e1 -- runtime/engine/Engine.cpp:862-874 (workEnd) + :2149-2158 (suspendLane anchor, rider) and @ 593f2a6d7427382a88a474a6a27c6dcc61b7415e -- Engine.cpp:1001-1009 (the held bound, rider), commit messages (all quotes); Batch 271 / Plan 275.".to_string()),
            // Not benchable: resume-admission decision content -- the win is
            // a serving-lane completion that no Rust-side A/B axis measures.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "phase-scoped-staging-released-at-successor-reservation-when-plan-budgets-max".to_string(),
            name: "a phase-scoped prompt staging that holds nothing between commands stays allocated through the decode it precedes -- idle, beside the hot floor its decode then takes -- while the memory plan budgets max(phase buffers) on the premise a lane never holds both, so at 256K on the M5 Pro the 528 MB staging sat through every decode, the largest part of what kept the decode 1.67 GiB below resident instead of the 2.3 GiB it should save → release the idle staging at the SUCCESSOR PHASE's reservation: reserveSpilledReads gives it back, when no prompt holds it, BEFORE it takes the lane's floor, so the floor's growth reuses its memory -- priced against the next prompt's re-allocation (about 60 staging extents at 256K, a few ms each), while the hot block cache keeps its blocks across turns".to_string(),
            category: "allocation".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `phase-scoped-staging-released-at-successor-reservation-when-plan-budgets-max`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/model/Runtime.cpp @ 88d0e57
// (Apache-2.0): a prompt reads its spilled history through the STAGING, a
// decode through the hot block cache FLOOR, and the memory plan budgets
// MAX(staging, floor) -- a request prefills or decodes, never both, so a
// lane never holds both. THE ANTI-PATTERN: the staging stayed allocated as
// an idle buffer after the prompt until a reclaim, beside the floor its
// decode then took -- the process held staging + floor under a max plan
// (the source: 528 MB through every decode at 256K on the M5 Pro, the
// largest part of what kept the decode 1.67 GiB below resident instead of
// the 2.3 GiB it should save). The LAW arm releases the idle staging at the
// SUCCESSOR PHASE's reservation: reserve_spilled_reads gives it back, when
// no prompt holds it, BEFORE it takes the lane's floor, so the floor's
// growth reuses its memory.
//
// ORIGINAL machinery -- never splash code verbatim. The staging is a
// HistoryStream of extents; the hot block cache keeps its blocks across
// turns; a next prompt allocates the staging again as it streams (the
// re-allocation accounting is the release's price: ~60 extents at 256K,
// a few ms each).

pub const EXTENT_BYTES: usize = 8 * 1024 * 1024; // 8 MiB extents; 66 = the 528 MB shape
pub const STAGING_EXTENTS: usize = 66;
pub const FLOOR_EXTENTS: usize = 48; // 384 MB of hot block cache floor

pub struct HistoryStream {
    pub allocated: bool,
    pub extents: usize,
    pub extents_built: usize, // the re-allocation accounting (the release's price)
}

impl HistoryStream {
    pub fn bytes(&self) -> usize {
        if self.allocated {
            self.extents * EXTENT_BYTES
        } else {
            0
        }
    }

    // a next prompt allocates the staging again as it streams
    pub fn allocate(&mut self, extents: usize) {
        self.allocated = true;
        self.extents = extents;
        self.extents_built += extents;
    }

    pub fn release_idle(&mut self) {
        self.allocated = false;
    }
}

pub struct BlockCache {
    pub lane_floor_bytes: usize,
    pub floor_taken: bool,
    pub blocks: usize, // the hot block cache keeps its blocks across turns
}

pub struct RequestState {
    pub resident: bool,
    pub prompt_complete: bool,
}

pub struct Runtime {
    pub stream: HistoryStream,
    pub cache: BlockCache,
    pub plan_bytes: usize, // max(staging, floor): a request prefills or decodes
    staging_request: Option<u64>,
    pub requests: Vec<(u64, RequestState)>,
    pub reserves_refused: usize,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            stream: HistoryStream {
                allocated: false,
                extents: 0,
                extents_built: 0,
            },
            cache: BlockCache {
                lane_floor_bytes: FLOOR_EXTENTS * EXTENT_BYTES,
                floor_taken: false,
                blocks: 0,
            },
            plan_bytes: STAGING_EXTENTS * EXTENT_BYTES, // max(528 MB, 384 MB)
            staging_request: None,
            requests: Vec::new(),
            reserves_refused: 0,
        }
    }

    pub fn admit(&mut self, id: u64) {
        self.requests
            .push((id, RequestState { resident: true, prompt_complete: false }));
    }

    // a prompt reserves the staging for its history and streams through it;
    // a next turn's prompt re-opens the prompt phase
    pub fn reserve_history_staging(&mut self, id: u64, extents: usize) {
        self.stream.allocate(extents);
        self.staging_request = Some(id);
        if let Some((_, s)) = self.requests.iter_mut().find(|(r, _)| *r == id) {
            s.prompt_complete = false;
        }
    }

    pub fn complete_prompt(&mut self, id: u64) {
        if let Some((_, s)) = self.requests.iter_mut().find(|(r, _)| *r == id) {
            s.prompt_complete = true;
        }
    }

    // Whether the history's staging is still a prompt's: that of the request
    // it was last reserved for, while the request is resident and its prompt
    // goes on. Releasing it would only make its next command allocate it
    // again, and could never fund its own growth.
    pub fn staging_held(&self) -> bool {
        match self.staging_request {
            Some(id) => self
                .requests
                .iter()
                .find(|(r, _)| *r == id)
                .map(|(_, s)| s.resident && !s.prompt_complete)
                .unwrap_or(false),
            None => false,
        }
    }

    pub fn footprint(&self) -> usize {
        self.stream.bytes()
            + if self.cache.floor_taken {
                self.cache.lane_floor_bytes
            } else {
                0
            }
    }

    pub fn plan_room_ok(&self) -> bool {
        self.footprint() <= self.plan_bytes
    }

    // the idle reclaim: the only thing that ever gives the staging back in
    // this arm -- it finds the staging AFTER the decode has taken its floor
    pub fn idle_reclaim(&mut self) {
        if !self.staging_held() {
            self.stream.release_idle();
        }
    }

    // THE ANTI-PATTERN: the floor's reservation takes no idle release -- the
    // staging is invisible to the gate (the plan budgets max, and the FLOOR
    // alone fits it), so the staging stays beside the floor until a reclaim
    // and the process holds staging + floor through every decode.
    pub fn reserve_spilled_reads(&mut self) -> bool {
        if self.cache.lane_floor_bytes > self.plan_bytes {
            self.reserves_refused += 1;
            return false;
        }
        self.cache.floor_taken = true;
        true
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_a_prompt_reads_its_history_through_the_staging() {
    let mut rt = Runtime::new();
    rt.admit(1);
    rt.reserve_history_staging(1, STAGING_EXTENTS);
    assert!(rt.staging_held());
    assert_eq!(rt.stream.bytes(), 528 * 1024 * 1024);
    assert_eq!(rt.stream.extents_built, STAGING_EXTENTS);
}

#[test]
fn eq_the_plan_counts_the_larger_of_the_two() {
    // the plan budgets max(staging, floor) -- a request prefills or decodes
    let staging = STAGING_EXTENTS * EXTENT_BYTES;
    let floor = FLOOR_EXTENTS * EXTENT_BYTES;
    assert_eq!(staging.max(floor), staging);
    let rt = Runtime::new();
    assert_eq!(rt.plan_bytes, staging);
}

#[test]
fn eq_a_staging_a_prompt_still_holds_is_not_released() {
    // while the holder's prompt goes on the staging stays, in BOTH arms --
    // the held buffer is held, and the decode that runs anyway honestly
    // breaches the max plan (the residual cost neither arm can avoid)
    let mut rt = Runtime::new();
    rt.admit(1);
    rt.reserve_history_staging(1, STAGING_EXTENTS);
    assert!(rt.staging_held());
    assert!(rt.reserve_spilled_reads()); // the floor fits the budget alone
    assert_eq!(rt.stream.bytes(), 528 * 1024 * 1024); // still held
    assert!(rt.cache.floor_taken);
    assert!(!rt.plan_room_ok());
}

// -- the anti-pattern's own shape --------------------------------------------

#[test]
fn bug_the_idle_staging_is_held_through_every_decode() {
    // the prompt completed; the decode takes its floor -- and the staging
    // stays beside it: the process holds 912 MB under a 528 MB max plan, the
    // source's 528 MB through every decode at 256K
    let mut rt = Runtime::new();
    rt.admit(1);
    rt.reserve_history_staging(1, STAGING_EXTENTS);
    rt.complete_prompt(1);
    assert!(!rt.staging_held()); // the staging is idle now
    assert!(rt.reserve_spilled_reads()); // the floor fits the budget alone
    assert!(rt.cache.floor_taken);
    assert_eq!(rt.footprint(), 912 * 1024 * 1024); // staging + floor held together
    assert!(!rt.plan_room_ok()); // the max-plan premise is breached
}

#[test]
fn bug_the_staging_survives_turn_after_turn_until_a_reclaim() {
    // the idle staging rides through EVERY decode turn; the idle reclaim is
    // the only release and runs after every decode has held both
    let mut rt = Runtime::new();
    rt.admit(1);
    rt.reserve_history_staging(1, STAGING_EXTENTS);
    rt.complete_prompt(1);
    for _ in 0..4 {
        assert!(rt.reserve_spilled_reads());
        assert_eq!(rt.stream.bytes(), 528 * 1024 * 1024); // through every decode
        assert!(!rt.plan_room_ok());
    }
    rt.idle_reclaim(); // too late for this decode's headroom
    assert_eq!(rt.stream.bytes(), 0);
    assert_eq!(rt.footprint(), 384 * 1024 * 1024);
    assert!(rt.plan_room_ok());
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `phase-scoped-staging-released-at-successor-reservation-when-plan-budgets-max`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/model/Runtime.cpp @ 88d0e57
// (Apache-2.0): reserveSpilledReads now gives the staging back, when no
// prompt holds it, before it takes the lane's floor, so the floor's growth
// reuses its memory. The staging holds nothing between commands; the next
// prompt that streams allocates it again as it grows (about 60 staging
// extents at 256K, a few ms each), while the hot block cache keeps its
// blocks across turns. The release is guarded by stagingHeld() -- the
// holder exists, is resident, and its prompt goes on -- so a prompt that
// still holds the staging keeps it.
//
// ORIGINAL machinery -- never splash code verbatim. The staging is a
// HistoryStream of extents; the re-allocation accounting is the release's
// price.

pub const EXTENT_BYTES: usize = 8 * 1024 * 1024; // 8 MiB extents; 66 = the 528 MB shape
pub const STAGING_EXTENTS: usize = 66;
pub const FLOOR_EXTENTS: usize = 48; // 384 MB of hot block cache floor

pub struct HistoryStream {
    pub allocated: bool,
    pub extents: usize,
    pub extents_built: usize, // the re-allocation accounting (the release's price)
}

impl HistoryStream {
    pub fn bytes(&self) -> usize {
        if self.allocated {
            self.extents * EXTENT_BYTES
        } else {
            0
        }
    }

    // a next prompt allocates the staging again as it streams
    pub fn allocate(&mut self, extents: usize) {
        self.allocated = true;
        self.extents = extents;
        self.extents_built += extents;
    }

    pub fn release_idle(&mut self) {
        self.allocated = false;
    }
}

pub struct BlockCache {
    pub lane_floor_bytes: usize,
    pub floor_taken: bool,
    pub blocks: usize, // the hot block cache keeps its blocks across turns
}

pub struct RequestState {
    pub resident: bool,
    pub prompt_complete: bool,
}

pub struct Runtime {
    pub stream: HistoryStream,
    pub cache: BlockCache,
    pub plan_bytes: usize, // max(staging, floor): a request prefills or decodes
    staging_request: Option<u64>,
    pub requests: Vec<(u64, RequestState)>,
    pub reserves_refused: usize,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            stream: HistoryStream {
                allocated: false,
                extents: 0,
                extents_built: 0,
            },
            cache: BlockCache {
                lane_floor_bytes: FLOOR_EXTENTS * EXTENT_BYTES,
                floor_taken: false,
                blocks: 0,
            },
            plan_bytes: STAGING_EXTENTS * EXTENT_BYTES, // max(528 MB, 384 MB)
            staging_request: None,
            requests: Vec::new(),
            reserves_refused: 0,
        }
    }

    pub fn admit(&mut self, id: u64) {
        self.requests
            .push((id, RequestState { resident: true, prompt_complete: false }));
    }

    // a prompt reserves the staging for its history and streams through it;
    // a next turn's prompt re-opens the prompt phase
    pub fn reserve_history_staging(&mut self, id: u64, extents: usize) {
        self.stream.allocate(extents);
        self.staging_request = Some(id);
        if let Some((_, s)) = self.requests.iter_mut().find(|(r, _)| *r == id) {
            s.prompt_complete = false;
        }
    }

    pub fn complete_prompt(&mut self, id: u64) {
        if let Some((_, s)) = self.requests.iter_mut().find(|(r, _)| *r == id) {
            s.prompt_complete = true;
        }
    }

    // Whether the history's staging is still a prompt's: that of the request
    // it was last reserved for, while the request is resident and its prompt
    // goes on. Releasing it would only make its next command allocate it
    // again, and could never fund its own growth.
    pub fn staging_held(&self) -> bool {
        match self.staging_request {
            Some(id) => self
                .requests
                .iter()
                .find(|(r, _)| *r == id)
                .map(|(_, s)| s.resident && !s.prompt_complete)
                .unwrap_or(false),
            None => false,
        }
    }

    pub fn footprint(&self) -> usize {
        self.stream.bytes()
            + if self.cache.floor_taken {
                self.cache.lane_floor_bytes
            } else {
                0
            }
    }

    pub fn plan_room_ok(&self) -> bool {
        self.footprint() <= self.plan_bytes
    }

    // THE LAW: the floor takes the place of the staging once no prompt holds
    // it -- the release runs BEFORE the floor's reservation, so the floor's
    // growth reuses its memory; the hot block cache keeps its blocks.
    pub fn reserve_spilled_reads(&mut self) -> bool {
        if self.stream.allocated && !self.staging_held() {
            self.stream.release_idle();
        }
        if self.cache.lane_floor_bytes > self.plan_bytes {
            self.reserves_refused += 1;
            return false;
        }
        self.cache.floor_taken = true;
        true
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_a_prompt_reads_its_history_through_the_staging() {
    let mut rt = Runtime::new();
    rt.admit(1);
    rt.reserve_history_staging(1, STAGING_EXTENTS);
    assert!(rt.staging_held());
    assert_eq!(rt.stream.bytes(), 528 * 1024 * 1024);
    assert_eq!(rt.stream.extents_built, STAGING_EXTENTS);
}

#[test]
fn eq_the_plan_counts_the_larger_of_the_two() {
    // the plan budgets max(staging, floor) -- a request prefills or decodes
    let staging = STAGING_EXTENTS * EXTENT_BYTES;
    let floor = FLOOR_EXTENTS * EXTENT_BYTES;
    assert_eq!(staging.max(floor), staging);
    let rt = Runtime::new();
    assert_eq!(rt.plan_bytes, staging);
}

#[test]
fn eq_a_staging_a_prompt_still_holds_is_not_released() {
    // while the holder's prompt goes on the staging stays, in BOTH arms --
    // the held buffer is held, and the decode that runs anyway honestly
    // breaches the max plan (the residual cost neither arm can avoid)
    let mut rt = Runtime::new();
    rt.admit(1);
    rt.reserve_history_staging(1, STAGING_EXTENTS);
    assert!(rt.staging_held());
    assert!(rt.reserve_spilled_reads()); // the floor fits the budget alone
    assert_eq!(rt.stream.bytes(), 528 * 1024 * 1024); // still held
    assert!(rt.cache.floor_taken);
    assert!(!rt.plan_room_ok());
}

// -- the law's own shape -----------------------------------------------------

#[test]
fn law_the_floor_takes_the_place_of_the_idle_staging() {
    // the prompt completed; the decode's release runs BEFORE the floor's
    // reservation: the staging gives its memory back and the floor's growth
    // reuses it -- a lane never holds both
    let mut rt = Runtime::new();
    rt.admit(1);
    rt.reserve_history_staging(1, STAGING_EXTENTS);
    rt.complete_prompt(1);
    assert!(!rt.staging_held());
    assert!(rt.reserve_spilled_reads());
    assert_eq!(rt.stream.bytes(), 0); // the staging gave its memory back
    assert!(rt.cache.floor_taken);
    assert_eq!(rt.footprint(), 384 * 1024 * 1024);
    assert!(rt.plan_room_ok());
}

#[test]
fn law_the_next_prompt_reallocates_the_staging_and_the_cache_keeps_its_blocks() {
    // the next turn prompts: the staging allocates again as it streams (the
    // release's price, the extents-built accounting); the decode then takes
    // its place again, and the hot block cache keeps its blocks across turns
    let mut rt = Runtime::new();
    rt.admit(1);
    rt.reserve_history_staging(1, STAGING_EXTENTS);
    rt.complete_prompt(1);
    assert!(rt.reserve_spilled_reads());
    rt.cache.blocks = 12; // the hot block cache filled across the decode
    rt.reserve_history_staging(1, STAGING_EXTENTS); // the next prompt streams
    assert!(rt.staging_held());
    assert_eq!(rt.stream.extents_built, 2 * STAGING_EXTENTS);
    rt.complete_prompt(1);
    assert!(rt.reserve_spilled_reads()); // the release runs, the floor stays
    assert_eq!(rt.stream.bytes(), 0);
    assert_eq!(rt.cache.blocks, 12); // the hot block cache keeps its blocks
    assert_eq!(rt.footprint(), 384 * 1024 * 1024);
    assert!(rt.plan_room_ok());
}
"#.to_string(),
            context_notes: Some("Grade B -- NotBenchable ON THIS BOX (the decision content is the RESIDENCY ACCOUNTING of a two-phase serving lane -- whether an idle phase-scoped buffer is released at the successor phase's reservation under a memory plan that budgets max(phase buffers) -- and a Rust microbench of the stand-in cannot measure it; source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ 88d0e57ee3a37305a895197538554ba9f3ea83d8 'Let a decoding lane's hot floor take the place of the idle staging', Apache-2.0, Jian Chen, 2026-10-18; every quote re-verified verbatim at the .raw/b271/splash clone this batch at those pins; inner double quotes rendered as single quotes per house string-literal safety; the file is runtime/model/Runtime.cpp -- the task-staged bare 'Runtime.cpp' path resolves to runtime/model/, disclosed). THE DEFECT (commit message, verbatim): 'A prompt reads its spilled history through the staging, a decode through the hot block cache, and the memory plan counts the larger of the two, since a request prefills or decodes. But the staging stayed allocated as an idle buffer after the prompt until a reclaim, beside the floor its decode then took: at 256K on the M5 Pro 528 MB through every decode, the largest part of what kept the decode 1.67 GiB below resident, not the 2.3 GiB it should save.' THE LAW (runtime/model/Runtime.cpp:3001-3006 @ 88d0e57, verbatim): 'A prompt reads its spilled history through the staging, a decode through the hot block cache: the floor a decoding lane takes replaces the staging once no prompt holds it, so a lane never holds both (the memory plan counts the larger of the two). The staging holds nothing between commands, and a next prompt allocates it again as it streams; the hot block cache keeps its blocks.' -- the release runs at the SUCCESSOR PHASE's reservation, before the floor's reservation (Runtime.cpp:3007-3011, verbatim code): 'if (impl_->historyStream && !impl_->stagingHeld()) static_cast<void>(impl_->historyStream->releaseIdle());' inside reserveSpilledReads ahead of 'return impl_->blockCache->reserveLane(lane);' (:3011). THE GUARD (Runtime.cpp:635-644, verbatim): 'Whether the history's staging is still a prompt's: that of the request it was last reserved for, while the request is resident and its prompt goes on. Releasing it would only make its next command allocate it again, and could never fund its own growth. Once it is not, a decoding lane's floor replaces it (reserveSpilledReads), or an idle reclaim gives it back.' over 'bool stagingHeld() const noexcept { const auto holder = requests.find(stagingRequest); return holder != requests.end() && holder->second.resident && !holder->second.promptComplete; }'. THE REPAIR: 'reserveSpilledReads now gives the staging back, when no prompt holds it, before it takes the lane's floor, so the floor's growth reuses its memory.' -- and the release is PRICED against the successor's re-allocation (message, verbatim): 'The staging holds nothing between commands; the next prompt that streams allocates it again as it grows (about 60 staging extents at 256K, a few ms each), while the hot block cache keeps its blocks across turns. Keeping the staging for the next turn saved those allocations but held it through every decode.' REGIME: phase-scoped scratch (a prompt's history staging, an encode's work arena, a batch's staging tables) under a memory plan that budgets max(phase buffers) because a request runs one phase at a time; the idle buffer is held across the phase boundary while the successor phase's floor is taken. ANTI-REGIMES: buffers holding state ACROSS commands (the hot block cache itself -- 'the hot block cache keeps its blocks'; a prefix cache; anything a next phase reads instead of rebuilding); plans that budget the SUM (then holding both is honest, just over budget); phases so cheap to re-enter that the release thrashes (the source prices it: ~60 extents, a few ms -- and the stagingHeld guard keys the release on prompt completion, never on a timer, so an in-prompt staging never churns). KINSHIP/DISTINCT-FROM (read + disclosed): one-shot-staging-release-to-system (B200, entries_15 + its B214 evict-to-reuse rider) owns the disposition family's release-to-SYSTEM pole -- a one-shot load seam's pinned staging returned to the host allocator at the seam's end, copies synchronized first -- and the handoff pole (evict-to-reuse); THIS owns the TIMING pole: the release at the SUCCESSOR's reservation, not at the seam's end and not only at an idle reclaim, because a plan that budgets max(phase buffers) makes the idle hold a standing lie between the phases -- new pole, cite and distinguish. reserve-stream-caches-at-admission-warm-request-paths-at-load (B257, entries_37) owns reserving and warming AT LOAD -- a boot-time axis, unrelated. persistent-digest-memo-stat-identity-keyed (B256, entries_36) owns the same release-because-rebuildable instinct for a disk memo keyed on stat identity -- the memo face, not the phase-boundary face. teardown-takes-list-whole (kernel_opt B257) owns taking a doomed list whole once teardown is warranted -- an unrelated granularity axis. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'stagingHeld|releaseIdle|hot floor|idle staging|block cache floor|floor replaces the staging|phase buffer|phase-scoped|prompt staging' -- zero owners. FIXTURE PROVENANCE: original machinery (never splash code) -- a Runtime stand-in with a HistoryStream staging (66 x 8 MiB extents = the 528 MB shape), a per-lane hot-block-cache floor (48 extents = 384 MB), a memory plan budgeting max(staging, floor), the stagingHeld() holder predicate (holder exists && resident && !promptComplete), and reserve_spilled_reads; the buggy arm takes the floor with NO idle release (the staging is invisible to the floor-fits-the-budget gate, so staging + floor = 912 MB sit under the 528 MB max plan through every decode until an idle reclaim); the fixed arm releases the idle staging BEFORE the floor reservation (footprint 384 MB <= plan), re-allocates the staging on the next prompt (the extents-built accounting -- the release's price), and skips the release while a prompt still holds the staging (the guard); the shared-contract tests (a prompt reads through the staging; the plan counts max; a held staging is not released and the decode then honestly breaches the max plan) pass in BOTH arms; the discriminating tests pin the 912-over-528 breach and the turn-after-turn hold (buggy) and the replacement + re-allocation + guard outcomes (fixed). Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: @ 88d0e57ee3a37305a895197538554ba9f3ea83d8 -- runtime/model/Runtime.cpp:3001-3006 + :3007-3011 + :635-644, commit message (all quotes); Batch 271 / Plan 275.".to_string()),
            // Not benchable: two-phase residency accounting -- the win is
            // resident headroom under a shared plan, not a Rust-side A/B.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "derived-reservation-taken-once-after-admission-settles".to_string(),
            name: "a multi-lane command's read reservation over the KV only the SSD cache holds -- a decoding lane's hot-block-cache floor, a prefill's history staging -- was taken PER LANE right after that lane's own KV, if the lane held Index pages then, but a LATER lane refused its KV converts to Spill and Cache::convertToSpill moves the landed pages of every earlier Spill lane into Index form, so the earlier lane reads Index pages it holds no floor for; the runtime's readers do not check -- KvBlockCache's resolve throws and the command's failure fails every request in service → take the derived reservation ONCE, after the admission loop settles, for every lane the command keeps in the plan's order -- the command's final shape decided in the same pass -- because a later admission can move earlier lanes' pages to another form and void an earlier reservation's basis; a lane refused its reservation leaves the command and the lanes after it take that shape without it".to_string(),
            category: "allocation".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `derived-reservation-taken-once-after-admission-settles`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/engine/Engine.cpp @ 81ae003
// (Apache-2.0): a multi-lane command over a two-form page pool -- Full RAM
// pages, and the SSD-side Index quota a Spill lane reads through. A Spill
// lane's pages land as Full when no Index page is free at its admission; a
// Resident lane refused its KV converts to Spill, and the conversion moves
// the LANDED pages of every Spill lane -- its own and the lanes admitted
// before it -- into the now-free Index pages. THE ANTI-PATTERN: each lane's
// read floor (the reservation for reading Index-form pages) was taken right
// after THAT lane's own KV, if the lane held Index pages then -- lane 1
// reserved 0, lane 2's conversion moved lane 1's pages to Index after it,
// and lane 1 read Index pages it held no floor for; the readers throw and
// the command's failure fails every request in service. The LAW arm takes
// the derived reservation ONCE, after the admission loop settles, for every
// lane the command keeps, in plan order.
//
// ORIGINAL machinery -- never splash code verbatim. The Index quota frees
// between admissions via a write-behind-lands step (the source's own
// copies-land mechanics, simplified). The streaming carve-out (a
// history-streaming prefill runs alone and whole) is NOT modeled -- the
// ordering law is rule-sized.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Resident,
    Spill,
}

pub struct Lane {
    pub id: usize,
    pub pages: usize,
    pub mode: Mode,
    pub index_pages: usize, // pages the lane reads through the SSD cache
    pub landed_full: usize, // pages that landed as Full (no Index page was free)
    pub floor: usize,       // the read reservation taken for its Index pages
    pub floored: bool,
}

pub struct Engine {
    pub full_cap: usize,
    pub full_used: usize,
    pub index_cap: usize,
    pub index_used: usize,
    pub floor_cap: usize,
    pub floor_used: usize,
    pub lanes: Vec<Lane>,
    pub reader_faults: usize,
    pub command_failed: bool,
}

impl Engine {
    pub fn new(full_cap: usize, index_used: usize, floor_cap: usize) -> Self {
        Engine {
            full_cap,
            full_used: 0,
            index_cap: 6,
            index_used,
            floor_cap,
            floor_used: 0,
            lanes: Vec::new(),
            reader_faults: 0,
            command_failed: false,
        }
    }

    pub fn index_free(&self) -> usize {
        self.index_cap - self.index_used
    }

    // write-behind lands the copies and frees Index pages between admissions
    // (the source's own copies-land mechanics, simplified)
    pub fn write_behind_lands(&mut self, n: usize) {
        self.index_used -= n.min(self.index_used);
    }

    // admit a lane's KV. A Spill lane's pages land as Index pages when the
    // quota has room, else they land as Full pages beside (no Index page was
    // free at the command's start). A Resident lane's pages are Full.
    pub fn admit_kv(&mut self, id: usize, pages: usize, mode: Mode) -> bool {
        let as_index = match mode {
            Mode::Spill => pages.min(self.index_free()),
            Mode::Resident => 0,
        };
        let as_full = pages - as_index;
        if self.full_used + as_full > self.full_cap {
            return false;
        }
        self.index_used += as_index;
        self.full_used += as_full;
        self.lanes.push(Lane {
            id,
            pages,
            mode,
            index_pages: as_index,
            landed_full: as_full,
            floor: 0,
            floored: false,
        });
        true
    }

    // the spill fallback: a Resident lane refused its KV converts, and the
    // conversion moves the landed pages of every Spill lane -- its own and
    // the lanes admitted before it -- into free Index pages.
    pub fn convert_to_spill(&mut self) {
        let mut free = self.index_free();
        let mut moved_total = 0;
        for lane in self.lanes.iter_mut() {
            if lane.mode != Mode::Spill || lane.landed_full == 0 || free == 0 {
                continue;
            }
            let moved = lane.landed_full.min(free);
            lane.landed_full -= moved;
            lane.index_pages += moved;
            free -= moved;
            moved_total += moved;
        }
        self.index_used += moved_total;
        self.full_used -= moved_total;
    }

    // THE ANTI-PATTERN: each lane's floor taken right after ITS OWN KV, if
    // the lane held Index pages then -- the derived reservation reads the
    // page forms mid-loop, before a later conversion can move them.
    fn take_floor_now(&mut self, id: usize) {
        let need = self
            .lanes
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.index_pages)
            .unwrap_or(0);
        if need == 0 {
            return; // no Index pages THEN: no floor taken
        }
        if self.floor_used + need <= self.floor_cap {
            if let Some(l) = self.lanes.iter_mut().find(|l| l.id == id) {
                l.floor = need;
                l.floored = true;
            }
            self.floor_used += need;
        }
        // short of the floor the lane keeps it at 0 -- the readers throw
    }

    fn convert_then_admit(&mut self, id: usize, pages: usize) -> bool {
        self.convert_to_spill(); // the spill fallback
        self.admit_kv(id, pages, Mode::Spill)
    }

    pub fn prepare_interleaved(
        &mut self,
        specs: &[(usize, usize, Mode)],
        interleave: &mut dyn FnMut(&mut Engine, usize),
    ) {
        for (i, (id, pages, mode)) in specs.iter().enumerate() {
            if !self.admit_kv(*id, *pages, *mode) {
                if !(*mode == Mode::Resident && self.convert_then_admit(*id, *pages)) {
                    interleave(self, i);
                    continue; // a lane refused its KV leaves the command
                }
            }
            let lane = self.lanes.last().unwrap();
            self.take_floor_now(lane.id); // if the lane held Index pages THEN
            interleave(self, i);
        }
        self.run_readers();
    }

    // the readers: a lane reading Index pages needs its floor to cover them
    // -- the resolve throw when a command misses more blocks than the slots
    // free, which the lanes' floors provide
    fn run_readers(&mut self) {
        for lane in &self.lanes {
            if lane.index_pages > lane.floor {
                self.reader_faults += 1;
                self.command_failed = true;
            }
        }
    }
}

// the defect scenario: lane 1 (Spill) lands Full -- no Index page was free
// at the command's start; the copies land (write-behind) BETWEEN the
// admissions, so 4 Index pages are free by the time lane 2 (Resident) is
// refused and converts, moving lane 1's pages to Index after lane 1's own
// reservation pass.
pub fn scenario() -> Engine {
    Engine::new(10, 6, 12) // full 10; Index quota 6 all held at the start
}

pub fn command_specs() -> Vec<(usize, usize, Mode)> {
    vec![
        (1, 4, Mode::Spill),
        (2, 8, Mode::Resident), // refused as Full: the pool cannot hold it
    ]
}

// the between-admissions write-behind hook both arms share
pub fn write_behind_after_first(e: &mut Engine, i: usize) {
    if i == 0 {
        e.write_behind_lands(4); // the copies land between the admissions
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_a_plain_all_resident_command_runs() {
    let mut e = Engine::new(10, 6, 12);
    e.prepare_interleaved(
        &[(1, 3, Mode::Resident), (2, 3, Mode::Resident)],
        &mut |_, _| {},
    );
    assert!(!e.command_failed);
    assert_eq!(e.reader_faults, 0);
    assert_eq!(e.lanes.len(), 2);
    assert_eq!(e.full_used, 6);
}

#[test]
fn eq_a_spill_lane_landing_index_gets_its_floor() {
    // with Index pages free at its admission, a Spill lane lands Index and
    // its floor is taken -- both arms agree when nothing moves later
    let mut e = Engine::new(10, 0, 12); // the Index quota fully free
    e.prepare_interleaved(&[(1, 4, Mode::Spill)], &mut |_, _| {});
    assert!(!e.command_failed);
    let lane = &e.lanes[0];
    assert_eq!(lane.index_pages, 4);
    assert!(lane.floored);
    assert_eq!(lane.floor, 4);
    assert_eq!(e.floor_used, 4);
}

#[test]
fn eq_a_lane_refused_even_as_spill_leaves_and_the_rest_run() {
    let mut e = Engine::new(10, 4, 12); // 2 Index pages free
    e.prepare_interleaved(
        &[(1, 3, Mode::Resident), (2, 12, Mode::Resident)],
        &mut |_, _| {},
    );
    assert!(!e.command_failed);
    assert_eq!(e.lanes.len(), 1); // lane 2 never joined
    assert_eq!(e.lanes[0].id, 1);
}

// -- the anti-pattern's own shape --------------------------------------------

#[test]
fn bug_a_later_conversion_voids_an_earlier_lanes_floor() {
    // lane 1 reserved 0 (it held no Index pages THEN); lane 2's conversion
    // moves lane 1's pages to Index after it -- lane 1 reads Index pages it
    // holds no floor for, the readers throw, and the command's failure fails
    // every request in service
    let mut e = scenario();
    e.prepare_interleaved(&command_specs(), &mut write_behind_after_first);
    let lane1 = e.lanes.iter().find(|l| l.id == 1).unwrap();
    assert_eq!(lane1.index_pages, 4); // the conversion moved them
    assert_eq!(lane1.floor, 0); // ...after the reservation pass
    assert!(e.command_failed);
    assert_eq!(e.reader_faults, 1);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `derived-reservation-taken-once-after-admission-settles`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/Engine.cpp @ 81ae003
// (Apache-2.0): the reservations follow the admission loop -- for every
// admitted lane that holds Index pages ONCE ALL OF THEM HAVE THEIR KV, since
// a lane that converts to Spill moves the landed pages of lanes admitted
// before it to the Index form too, and only for the lanes the command keeps,
// in the plan's order. A lane refused its memory leaves the command, and the
// lanes after it take that shape without it. The command's final shape is
// decided in the same pass as the reservations.
//
// ORIGINAL machinery -- never splash code verbatim. The Index quota frees
// between admissions via a write-behind-lands step (the source's own
// copies-land mechanics, simplified). The streaming carve-out (a
// history-streaming prefill runs alone and whole) is NOT modeled -- the
// ordering law is rule-sized.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Resident,
    Spill,
}

pub struct Lane {
    pub id: usize,
    pub pages: usize,
    pub mode: Mode,
    pub index_pages: usize, // pages the lane reads through the SSD cache
    pub landed_full: usize, // pages that landed as Full (no Index page was free)
    pub floor: usize,       // the read reservation taken for its Index pages
    pub floored: bool,
}

pub struct Engine {
    pub full_cap: usize,
    pub full_used: usize,
    pub index_cap: usize,
    pub index_used: usize,
    pub floor_cap: usize,
    pub floor_used: usize,
    pub lanes: Vec<Lane>,
    pub reader_faults: usize,
    pub command_failed: bool,
}

impl Engine {
    pub fn new(full_cap: usize, index_used: usize, floor_cap: usize) -> Self {
        Engine {
            full_cap,
            full_used: 0,
            index_cap: 6,
            index_used,
            floor_cap,
            floor_used: 0,
            lanes: Vec::new(),
            reader_faults: 0,
            command_failed: false,
        }
    }

    pub fn index_free(&self) -> usize {
        self.index_cap - self.index_used
    }

    // write-behind lands the copies and frees Index pages between admissions
    // (the source's own copies-land mechanics, simplified)
    pub fn write_behind_lands(&mut self, n: usize) {
        self.index_used -= n.min(self.index_used);
    }

    // admit a lane's KV. A Spill lane's pages land as Index pages when the
    // quota has room, else they land as Full pages beside (no Index page was
    // free at the command's start). A Resident lane's pages are Full.
    pub fn admit_kv(&mut self, id: usize, pages: usize, mode: Mode) -> bool {
        let as_index = match mode {
            Mode::Spill => pages.min(self.index_free()),
            Mode::Resident => 0,
        };
        let as_full = pages - as_index;
        if self.full_used + as_full > self.full_cap {
            return false;
        }
        self.index_used += as_index;
        self.full_used += as_full;
        self.lanes.push(Lane {
            id,
            pages,
            mode,
            index_pages: as_index,
            landed_full: as_full,
            floor: 0,
            floored: false,
        });
        true
    }

    // the spill fallback: a Resident lane refused its KV converts, and the
    // conversion moves the landed pages of every Spill lane -- its own and
    // the lanes admitted before it -- into free Index pages.
    pub fn convert_to_spill(&mut self) {
        let mut free = self.index_free();
        let mut moved_total = 0;
        for lane in self.lanes.iter_mut() {
            if lane.mode != Mode::Spill || lane.landed_full == 0 || free == 0 {
                continue;
            }
            let moved = lane.landed_full.min(free);
            lane.landed_full -= moved;
            lane.index_pages += moved;
            free -= moved;
            moved_total += moved;
        }
        self.index_used += moved_total;
        self.full_used -= moved_total;
    }

    pub fn prepare_after_settle(
        &mut self,
        specs: &[(usize, usize, Mode)],
        interleave: &mut dyn FnMut(&mut Engine, usize),
    ) -> usize {
        // phase 1: the admission loop runs to settlement -- every lane has
        // its KV (or leaves the command), every page in its final form
        for (i, (id, pages, mode)) in specs.iter().enumerate() {
            if !self.admit_kv(*id, *pages, *mode)
                && !(*mode == Mode::Resident && self.convert_then_admit(*id, *pages))
            {
                interleave(self, i);
                continue; // a lane refused its KV leaves the command
            }
            interleave(self, i);
        }
        // phase 2: the loop has settled -- NOW derive the reservations, for
        // every lane the command keeps, in plan order; a lane refused its
        // floor leaves the command and the lanes after it take that shape
        // without it
        let mut i = 0;
        while i < self.lanes.len() {
            let need = self.lanes[i].index_pages;
            let id = self.lanes[i].id;
            if need == 0 {
                i += 1;
                continue;
            }
            if self.floor_used + need <= self.floor_cap {
                self.lanes[i].floor = need;
                self.lanes[i].floored = true;
                self.floor_used += need;
                i += 1;
            } else {
                self.eject(id); // do not advance: the lanes shifted left
            }
        }
        self.run_readers();
        self.lanes.len()
    }

    fn convert_then_admit(&mut self, id: usize, pages: usize) -> bool {
        self.convert_to_spill(); // the spill fallback
        self.admit_kv(id, pages, Mode::Spill)
    }

    // a lane refused its memory leaves the command: its pages go back
    fn eject(&mut self, id: usize) {
        if let Some(i) = self.lanes.iter().position(|l| l.id == id) {
            let lane = &self.lanes[i];
            self.full_used -= lane.landed_full;
            self.index_used -= lane.index_pages;
            self.floor_used -= lane.floor;
            self.lanes.remove(i);
        }
    }

    // the readers: a lane reading Index pages needs its floor to cover them
    // -- the resolve throw when a command misses more blocks than the slots
    // free, which the lanes' floors provide
    fn run_readers(&mut self) {
        for lane in &self.lanes {
            if lane.index_pages > lane.floor {
                self.reader_faults += 1;
                self.command_failed = true;
            }
        }
    }
}

// the defect scenario: lane 1 (Spill) lands Full -- no Index page was free
// at the command's start; the copies land (write-behind) BETWEEN the
// admissions, so 4 Index pages are free by the time lane 2 (Resident) is
// refused and converts, moving lane 1's pages to Index before the settled
// reservation pass.
pub fn scenario() -> Engine {
    Engine::new(10, 6, 12) // full 10; Index quota 6 all held at the start
}

pub fn command_specs() -> Vec<(usize, usize, Mode)> {
    vec![
        (1, 4, Mode::Spill),
        (2, 8, Mode::Resident), // refused as Full: the pool cannot hold it
    ]
}

// the between-admissions write-behind hook both arms share
pub fn write_behind_after_first(e: &mut Engine, i: usize) {
    if i == 0 {
        e.write_behind_lands(4); // the copies land between the admissions
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_a_plain_all_resident_command_runs() {
    let mut e = Engine::new(10, 6, 12);
    let kept = e.prepare_after_settle(
        &[(1, 3, Mode::Resident), (2, 3, Mode::Resident)],
        &mut |_, _| {},
    );
    assert!(!e.command_failed);
    assert_eq!(e.reader_faults, 0);
    assert_eq!(kept, 2);
    assert_eq!(e.full_used, 6);
}

#[test]
fn eq_a_spill_lane_landing_index_gets_its_floor() {
    // with Index pages free at its admission, a Spill lane lands Index and
    // its floor is taken -- both arms agree when nothing moves later
    let mut e = Engine::new(10, 0, 12); // the Index quota fully free
    e.prepare_after_settle(&[(1, 4, Mode::Spill)], &mut |_, _| {});
    assert!(!e.command_failed);
    let lane = &e.lanes[0];
    assert_eq!(lane.index_pages, 4);
    assert!(lane.floored);
    assert_eq!(lane.floor, 4);
    assert_eq!(e.floor_used, 4);
}

#[test]
fn eq_a_lane_refused_even_as_spill_leaves_and_the_rest_run() {
    let mut e = Engine::new(10, 4, 12); // 2 Index pages free
    let kept = e.prepare_after_settle(
        &[(1, 3, Mode::Resident), (2, 12, Mode::Resident)],
        &mut |_, _| {},
    );
    assert!(!e.command_failed);
    assert_eq!(kept, 1); // lane 2 never joined
    assert_eq!(e.lanes[0].id, 1);
}

// -- the law's own shape -----------------------------------------------------

#[test]
fn law_the_settled_command_floors_every_lane_holding_index_pages() {
    // the same scenario the anti-pattern arm faults on: the reservations
    // follow the admission loop -- lane 1's floor is derived from its FINAL
    // form (4 Index pages), so the command runs
    let mut e = scenario();
    let kept = e.prepare_after_settle(&command_specs(), &mut write_behind_after_first);
    assert_eq!(kept, 2);
    let lane1 = e.lanes.iter().find(|l| l.id == 1).unwrap();
    assert_eq!(lane1.index_pages, 4);
    assert!(lane1.floored);
    assert_eq!(lane1.floor, 4);
    assert!(!e.command_failed);
    assert_eq!(e.reader_faults, 0);
    assert_eq!(e.floor_used, 4);
}

#[test]
fn law_a_lane_refused_its_floor_leaves_the_command() {
    // the floor memory cannot fund lane 1's 4 Index pages: the lane leaves
    // the command and the lanes after it take that shape without it -- the
    // command runs with what is kept, the readers never throw
    let mut e = scenario();
    e.floor_cap = 2; // the read memory the command may fund
    let kept = e.prepare_after_settle(&command_specs(), &mut write_behind_after_first);
    assert_eq!(kept, 1); // lane 1 left
    assert!(e.lanes.iter().all(|l| l.id != 1));
    assert!(!e.command_failed);
    assert_eq!(e.reader_faults, 0);
    assert_eq!(e.index_used, 2); // lane 1's Index pages went back
    assert_eq!(e.floor_used, 0);
}
"#.to_string(),
            context_notes: Some("Grade B -- NotBenchable ON THIS BOX (the decision content is the RESERVATION ORDERING of a multi-lane serving command -- when a derived read reservation over currently-owned pages may be taken relative to the admission loop that can still move those pages to another form -- and a Rust microbench of the stand-in cannot measure it; no source-reported perf figures exist, the failure mode is a correctness throw). SOURCE (incoai/splash @ 81ae003e18d9362f6adf4131cf88a17a17803f65 'Reserve the memory for spilled reads once every lane has its KV', Apache-2.0, Jian Chen, 2026-10-19; every quote re-verified verbatim at the .raw/b271/splash clone this batch at those pins; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (commit message, verbatim): 'Engine::prepare reserved a lane's memory for reading the KV only the SSD cache holds, a decoding lane's floor of the hot block cache or a prefill's history staging, right after the lane's own KV, if the lane held Index pages then. A later lane of the same command whose KV was refused may convert to Spill (spillFallback), and Cache::convertToSpill moves the landed pages of every Spill request into free Index pages: an earlier lane's own pages, which no Index page was free for at the command's start, or the blocks it shares with the converting lane, which stayed Full only while that lane was Resident. The earlier lane then read Index pages it held no floor for.' -- the blast radius (verbatim): 'The runtime's readers do not check: KvBlockCache's resolve throws once a command misses more blocks than the cache has slots free, which the lanes' floors provide, and KvHistoryStream::begin when a command's pages exceed the staging; the command's failure fails every request in service (engine_execution_failed).' THE LAW (runtime/engine/Engine.cpp:1605-1610 @ 81ae003, verbatim): 'The model's memory for reads of the KV only the SSD cache holds: a decoding lane's floor of the hot block cache, or the staging of a prefill's history up to the command's end. It is reserved once every lane has its KV, since a lane that converts to Spill moves the landed pages of lanes admitted before it to the index form too (Cache::convertToSpill), and only for the lanes the command keeps, in the plan's order.' -- and the shape clause (:1614-1618, verbatim): 'after a lane kept, it leaves for a command of its own. Admission may have made it stream, converting it at once where its restored chain's copies had landed, which the plan, made before, did not know. A lane refused its memory leaves the command, and the lanes after it take that shape without it.' THE REPAIR: 'The reservations now follow the admission loop, for every admitted lane that holds Index pages once all of them have their KV; a lane refused its reservation leaves the command as before.' (message) -- and the command's final shape is decided in the SAME pass as the reservations, lane by lane in the plan's order, so only the lanes the command keeps reserve; a leading lane refused its staging leaves and the next may lead, 'where the command cut to the lane first was left empty and a lane suspended, its prefill to replay, though the others fit' (message). THE NARROWING ALTERNATIVE, REJECTED (message, verbatim): 'Making convertToSpill move only the converting request's pages would not do: the blocks it shares with Spill lanes admitted before it still move under them, and the reservations would still rest on no later allocation of the command changing a page's form.' -- the law is the ORDERING (derive the reservations from the command's settled shape), not a narrower conversion. REGIME: multi-lane commands whose per-lane memory reservations are DERIVED from mutable page/state ownership (page forms, block sharing, streaming shape) while the admission loop can still change that ownership for EARLIER lanes (a later conversion, a spill fallback, a demotion); any reservation taken per-item inside the loop that produces the mutations. ANTI-REGIMES: reservations independent of later admissions (fixed-size per-lane floors no conversion can move); single-lane commands (nothing earlier to void); allocations already ordered after all mutations by construction; reserve-once-at-load budgets (a lifecycle axis, not an ordering axis). KINSHIP/DISTINCT-FROM (read + disclosed): lazy-admission-reservation-atomic-register (kernel_opt B195, entries_gpu_63) owns reservation ATOMICITY -- the capacity check and the reservation write in one critical section, max(actual, estimate) charged, so N same-step admissions cannot collectively overshoot -- THIS owns reservation ORDERING: the reservation must be derived from the command's SETTLED form because a later ADMISSION (a conversion) changes EARLIER lanes' page forms and voids the earlier reservation's BASIS; compose (atomicity within one reservation, ordering across the loop). reserve-stream-caches-at-admission-warm-request-paths-at-load (B257, entries_37) owns WHEN a stream cache is reserved and warmed (at admission/load) -- a lifecycle axis, not an ordering-vs-loop axis. launch-cost-scales-with-allocation-count-pack-one-arena (B256, entries_36) carries a 'per-stream caches reserved once their count is known' parenthetical -- the same once-known instinct for a COUNT, not for ownership MOVES inside an admission loop; distinct. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'convertToSpill|reservation ordering|reserve once|reserved once every|admission settles|per-item reservation|admission loop' -- zero owners (the entries_36 parenthetical and the B195 atomicity rule are the nearest adjacencies, both disclosed above). FIXTURE PROVENANCE: original machinery (never splash code) -- a two-form page pool (10 Full RAM pages + a 6-slot SSD-side Index quota) and a two-lane command: lane 1 (Spill) lands Full because no Index page is free at its admission ('which no Index page was free for at the command's start'), a write-behind-lands step frees 4 Index pages BETWEEN the admissions (the source's own copies-land mechanics, simplified), lane 2 (Resident) is refused and converts, and the conversion moves lane 1's landed pages to Index; each lane holding Index pages needs a read floor sized to them; the buggy arm reserves each lane's floor INSIDE the loop at its own admission time (lane 1 reserves 0; the conversion moves its pages after it) so the readers fault (the resolve-throw face); the fixed arm runs the admissions to settlement, then reserves once for every kept lane holding Index pages, in plan order, ejecting a lane whose floor does not fund (it leaves the command, the rest run without it); the shared-contract tests (a plain all-Resident command; a Spill lane landing Index gets its floor; a lane refused even as Spill leaves and the rest run) pass in BOTH arms; the discriminating tests pin the reader fault (buggy) and the settled reservation + the floor-refusal ejection (fixed). The source's streaming carve-out (a history-streaming prefill runs alone and whole) is NOT modeled -- the ordering law is rule-sized; disclosed. Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: @ 81ae003e18d9362f6adf4131cf88a17a17803f65 -- runtime/engine/Engine.cpp:1605-1610 + :1614-1618, commit message (all quotes); Batch 271 / Plan 275.".to_string()),
            // Not benchable: reservation-ordering decision content -- the
            // failure is a correctness throw, never a Rust-side A/B axis.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
