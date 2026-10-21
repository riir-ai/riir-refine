//! Entries slice A - Batch 270 (2026-10-21, Plan 274 / Research 306): the
//! splash-wave serving pool, slice A (3 caching rules) - incoai/splash @
//! a20f344 + 65f9713, 6e21cc9, a7e5347 + f3eb1a0, Apache-2.0; every quote
//! re-verified at the .raw/b270/splash clone this batch. All three rules are
//! corpus-only `NotBenchable` (decision-content cache laws; the staged grades
//! and source-reported figures ride each `context_notes`) - no proposer files.
//! The staged leads land: 'disk-state-keyed redundancy', 'second-witness
//! teardown' and 'flat-table slot recycler' (the `kernel_opt` Batch 268 wave
//! header's staging list, `entries_gpu_117`). Fixtures are ORIGINAL machinery
//! (never splash code verbatim), each compiled and executed standalone
//! (rustc --edition 2021 --test) before this table was assembled; the
//! buggy/fixed arm pairs share contract tests and differ only on the law.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "restore-point-chain-copies-keyed-on-disk-state-count".to_string(),
            name: "A persistent KV tier's reclaim keys its 'unneeded copy' predicate on the RAM-resident state counts, so the redundant chain copies of states that stay in RAM are never reclaimable and quota pressure spends the durable restore points instead (the only thing dropOldestPoint can take) -- the tier's crash-recovery value collapses while the copies hold the quota → key the retention of a restore point's chain copies on the count of states WITH A DISK COPY at the block or below it (countDiskState), kept from the FIRST ATTEMPT of a state's write through refusal-rollback and erase, so making room for it never takes its own chain; a redundant copy (a Full page's) goes once no state on disk restores through it; and a disk-only block a request computes again KEEPS its copy (foreignCopy) instead of dropping it on page adoption -- the chain on disk keeps no hole".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `restore-point-chain-copies-keyed-on-disk-state-count`
// (riir-refine rust_perf corpus, Batch 270 / Plan 274) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/engine/{KvCache,StateCache,
// Cache}.{hpp,cpp} @ a20f344 + 65f9713 (Apache-2.0): a persistent KV tier of
// block chains with per-block disk copies and checkpoint states, under a
// unit quota, with a two-step reclaim (unneeded copies first, then the
// oldest restore point). The anti-pattern keys the "unneeded" copy predicate
// on the RAM-RESIDENT state counts (statesHere/statesBelow): the redundant
// chain copies of states that stay in RAM are never reclaimable, so quota
// pressure spends the durable restore points instead (the only thing
// dropOldestPoint can take) -- the tier's crash-recovery value collapses
// while the copies hold the quota. The fixed arm keys the same predicate on
// the count of states WITH A DISK COPY at the block or below it
// (countDiskState), kept from the FIRST ATTEMPT of a state's write through
// refusal-rollback and erase.
//
// ORIGINAL machinery -- never splash code verbatim. Two conversations share
// the tier (the minimal fork model: one conversation's exclusive blocks are
// not another's ancestors). The recompute face (65f9713) is carried too: a
// disk-only block a request computes again DROPS its copy on page adoption,
// holing the chain of a disk state.
//
// Geometry: block ids 1..=DEPTH per chain; parent(b) = b - 1 within the
// chain. A state at depth d restores through the copies of blocks 1..=d of
// its chain. A block's copy is REDUNDANT while a page also holds its K/V (a
// resident state's chain, or a transient request page), else it is the ONLY
// copy. Each resident copy and each state's restore point is one quota unit.

pub const TOKENS_PER_BLOCK: usize = 1024;
pub const CHAINS: usize = 2;

#[derive(Clone, Copy)]
pub struct State {
    pub chain: usize,
    pub at: usize,
    pub ram: bool,
    pub disk: bool,
}

pub struct Tier {
    quota: usize,
    used: usize,
    depth: usize, // per-chain max depth
    frontier: Vec<usize>, // per chain: the deepest block the chain reaches
    copy_of: Vec<Vec<bool>>,
    pages: Vec<Vec<u32>>,     // RAM K/V holders per block
    transient: Vec<Vec<u32>>, // request pages (no state behind them)
    ram_below: Vec<Vec<u32>>,
    states_at: Vec<Vec<u32>>,
    pub states: Vec<State>,
    // instrumentation (the recovery economics)
    pub points_dropped: usize,
    pub copies_dropped: usize,
    pub writes_refused: usize,
}

impl Tier {
    pub fn new(quota: usize, depth: usize) -> Self {
        Tier {
            quota,
            used: 0,
            depth,
            frontier: vec![0; CHAINS],
            copy_of: vec![vec![false; depth + 1]; CHAINS],
            pages: vec![vec![0; depth + 1]; CHAINS],
            transient: vec![vec![0; depth + 1]; CHAINS],
            ram_below: vec![vec![0; depth + 1]; CHAINS],
            states_at: vec![vec![0; depth + 1]; CHAINS],
            states: Vec::new(),
            points_dropped: 0,
            copies_dropped: 0,
            writes_refused: 0,
        }
    }

    pub fn used(&self) -> usize {
        self.used
    }

    pub fn durable_points(&self) -> usize {
        self.states.iter().filter(|s| s.disk).count()
    }

    pub fn block_copied(&self, chain: usize, b: usize) -> bool {
        self.copy_of[chain][b]
    }

    // The deepest disk state whose whole chain of copies is resident, across
    // all conversations: what the next process takes back after a kill.
    pub fn restart_tokens(&self) -> usize {
        let mut best = 0usize;
        for s in self.states.iter().filter(|s| s.disk) {
            if (1..=s.at).all(|b| self.copy_of[s.chain][b]) {
                best = best.max(s.at * TOKENS_PER_BLOCK);
            }
        }
        best
    }

    fn add_range(v: &mut [u32], lo: usize, hi: usize, d: i32) {
        for b in lo..=hi {
            v[b] = (v[b] as i32 + d) as u32;
        }
    }

    // A checkpoint published at depth `at` of `chain`, RAM-resident, written
    // behind. (This arm counts RAM states only; the in-flight write has no
    // protection of its own.)
    pub fn publish(&mut self, chain: usize, at: usize) -> bool {
        let new_blocks = at.saturating_sub(self.frontier[chain]);
        let first_new = self.frontier[chain] + 1;
        self.frontier[chain] = self.frontier[chain].max(at);
        self.ram_below_add(chain, 1, at, 1);
        let needed = new_blocks + 1; // new blocks + the point
        while self.used + needed > self.quota && self.reclaim_step() {}
        if self.used + needed > self.quota {
            self.ram_below_add(chain, 1, at, -1); // a refused write counts for nothing
            self.writes_refused += 1;
            return false;
        }
        for b in first_new..=at {
            self.copy_of[chain][b] = true;
            self.used += 1;
        }
        self.states.push(State {
            chain,
            at,
            ram: true,
            disk: true,
        });
        self.used += 1;
        self.states_at[chain][at] += 1;
        self.pages_add(chain, 1, at, 1);
        true
    }

    // The state's RAM K/V pages go (memory pressure); the state becomes
    // disk-only.
    pub fn drop_ram(&mut self, chain: usize, at: usize) {
        let i = self
            .states
            .iter()
            .rposition(|s| s.chain == chain && s.at == at && s.ram)
            .expect("no RAM state there");
        self.states[i].ram = false;
        self.pages_add(chain, 1, at, -1);
        self.ram_below_add(chain, 1, at, -1);
    }

    // The state is erased entirely: an erased state keeps no count.
    pub fn erase(&mut self, chain: usize, at: usize) {
        let i = self
            .states
            .iter()
            .rposition(|s| s.chain == chain && s.at == at)
            .expect("no state there");
        let s = self.states.remove(i);
        self.states_at[chain][at] -= 1;
        if s.disk {
            self.used -= 1; // the restore point goes
        }
        if s.ram {
            self.pages_add(chain, 1, at, -1);
            self.ram_below_add(chain, 1, at, -1);
        }
    }

    // A request computes block `at` again and a page lands there. In this
    // arm the adoption DROPS the disk copy: the chain on disk keeps a hole.
    pub fn recompute(&mut self, chain: usize, at: usize) {
        self.pages[chain][at] += 1;
        self.transient[chain][at] += 1;
        if self.copy_of[chain][at] {
            self.copy_of[chain][at] = false;
            self.used -= 1;
            self.copies_dropped += 1;
        }
    }

    // The recomputing request ends; its page goes.
    pub fn end_recompute(&mut self, chain: usize, at: usize) {
        self.pages[chain][at] -= 1;
        self.transient[chain][at] -= 1;
    }

    // One reclaim step: an unneeded copy first, then the oldest point.
    fn reclaim_step(&mut self) -> bool {
        if self.drop_unneeded_copy() {
            return true;
        }
        self.drop_oldest_point()
    }

    // THE ANTI-PATTERN: unneeded is keyed on the RAM state counts.
    fn unneeded(&self, chain: usize, b: usize) -> bool {
        if self.pages[chain][b] > 0 {
            // a redundant copy (a Full page's): stuck while any RAM state
            // sits at or below the block
            self.ram_below[chain][b] == 0 && self.states_at[chain][b] == 0
        } else {
            // the only copy: also stuck while a RAM state sits below, and a
            // leaf only (nothing below depends on it)
            self.ram_below[chain][b] == 0
                && self.states_at[chain][b] == 0
                && b == self.frontier[chain]
        }
    }

    fn drop_unneeded_copy(&mut self) -> bool {
        for chain in 0..CHAINS {
            for b in 1..=self.depth {
                if self.copy_of[chain][b] && self.unneeded(chain, b) {
                    self.copy_of[chain][b] = false;
                    self.used -= 1;
                    self.copies_dropped += 1;
                    return true;
                }
            }
        }
        false
    }

    // The oldest restore point of a state also in RAM: the durable copies
    // this arm spends under pressure.
    fn drop_oldest_point(&mut self) -> bool {
        let victim = self.states.iter().position(|s| s.disk && s.ram);
        match victim {
            Some(i) => {
                self.states[i].disk = false;
                self.used -= 1;
                self.points_dropped += 1;
                true
            }
            None => false,
        }
    }

    fn pages_add(&mut self, chain: usize, lo: usize, hi: usize, d: i32) {
        Self::add_range(&mut self.pages[chain], lo, hi, d);
    }

    fn ram_below_add(&mut self, chain: usize, lo: usize, hi: usize, d: i32) {
        Self::add_range(&mut self.ram_below[chain], lo, hi, d);
    }

    // A RAM-resident state always restores (through its pages).
    pub fn restore_ram(&self, chain: usize, at: usize) -> bool {
        (1..=at).all(|b| self.pages[chain][b] > 0)
    }

    // The counters agree with the state vector (the checkInvariants echo).
    pub fn check_invariants(&self) {
        let mut pages = vec![vec![0u32; self.depth + 1]; CHAINS];
        let mut ram_below = vec![vec![0u32; self.depth + 1]; CHAINS];
        let mut states_at = vec![vec![0u32; self.depth + 1]; CHAINS];
        for s in &self.states {
            if s.ram {
                Self::add_range(&mut pages[s.chain], 1, s.at, 1);
                Self::add_range(&mut ram_below[s.chain], 1, s.at, 1);
            }
            states_at[s.chain][s.at] += 1;
        }
        for chain in 0..CHAINS {
            for b in 1..=self.depth {
                pages[chain][b] += self.transient[chain][b];
            }
            assert_eq!(pages[chain], self.pages[chain], "pages {chain}");
            assert_eq!(ram_below[chain], self.ram_below[chain], "ram_below {chain}");
            assert_eq!(states_at[chain], self.states_at[chain], "states_at {chain}");
        }
        let copies: usize = self
            .copy_of
            .iter()
            .map(|c| c.iter().filter(|&&x| x).count())
            .sum();
        let points = self.states.iter().filter(|s| s.disk).count();
        assert_eq!(self.used, copies + points, "used == copies + points");
        assert!(self.used <= self.quota, "quota holds");
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_ram_states_always_restore_and_the_quota_holds() {
    let mut t = Tier::new(64, 32);
    for d in [2usize, 4, 6, 8, 10, 12] {
        assert!(t.publish(0, d));
        t.check_invariants();
        assert!(t.restore_ram(0, d));
    }
    assert_eq!(t.durable_points(), 6);
    assert_eq!(t.restart_tokens(), 12 * TOKENS_PER_BLOCK);
}

#[test]
fn eq_a_refused_write_leaves_no_state_and_no_count() {
    let mut t = Tier::new(2, 8);
    assert!(!t.publish(0, 4)); // needs 5 units under a 2-unit quota
    assert_eq!(t.writes_refused, 1);
    assert_eq!(t.durable_points(), 0);
    t.check_invariants(); // the counts rolled back cleanly
}

#[test]
fn eq_an_erased_state_keeps_no_count() {
    let mut t = Tier::new(16, 16);
    assert!(t.publish(0, 4));
    assert!(t.publish(0, 6));
    t.erase(0, 4);
    t.check_invariants();
    assert_eq!(t.durable_points(), 1);
    assert_eq!(t.restart_tokens(), 6 * TOKENS_PER_BLOCK);
}

// -- the anti-pattern's own shape --------------------------------------------

#[test]
fn bug_the_soak_spends_its_restore_points_under_ram_pressure() {
    // Two conversations share the tier. The short one's checkpoints die
    // first (they are the oldest), its stale chain copies are never
    // reclaimable (a RAM state still sits below every one of them), so the
    // growing conversation's restore points are spent instead -- the
    // measured "restore points fell from 51 to 0-1 while KV copies held 96%
    // of the quota".
    let mut t = Tier::new(30, 32);
    // the short conversation checkpoints first, so its points are the oldest
    assert!(t.publish(0, 4));
    assert!(t.publish(0, 8));
    for d in [4usize, 8, 12, 16] {
        assert!(t.publish(1, d));
        t.check_invariants();
    }
    // the next checkpoint of the long conversation must make room
    assert!(t.publish(1, 20));
    t.check_invariants();

    assert_eq!(t.writes_refused, 0);
    assert!(
        t.points_dropped >= 4,
        "points_dropped={}",
        t.points_dropped
    );
    assert!(
        t.durable_points() <= 3,
        "durable_points={}",
        t.durable_points()
    );
    assert_eq!(
        t.copies_dropped, 0,
        "the RAM-keyed order never frees a copy while a RAM state sits below it"
    );
}

#[test]
fn bug_a_recompute_holes_the_chain_of_a_disk_state() {
    // 65f9713's face: a request computes a disk-only chain block again and
    // adopts it -- the copy goes with the adoption, and after a kill the
    // restore point's chain has a hole where the request wrote.
    let mut t = Tier::new(32, 16);
    assert!(t.publish(0, 6));
    t.drop_ram(0, 6); // the state's RAM pages go; the restore point remains
    assert_eq!(t.restart_tokens(), 6 * TOKENS_PER_BLOCK);

    t.recompute(0, 3); // a later request computes block 3 again
    t.end_recompute(0, 3);
    t.check_invariants();
    // the copy went with the adoption: the chain on disk has a hole at 3
    assert!(!t.block_copied(0, 3));
    assert_eq!(t.restart_tokens(), 0); // the restore point is dead
}

#[test]
fn bug_publishes_over_a_holed_in_flight_chain() {
    // The write's own chain has no protection in this arm: the adoption of
    // the recomputed block already holed it, and the write publishes its
    // restore point over the hole -- a point that is broken at birth. The
    // fixed arm refuses this write instead.
    let mut t = Tier::new(8, 16);
    assert!(t.publish(0, 4));
    assert!(t.publish(0, 6));
    t.drop_ram(0, 6); // both states' pages decay
    t.drop_ram(0, 4);
    t.erase(0, 6); // the states go; the chain copies remain
    t.erase(0, 4);
    t.check_invariants();
    t.recompute(0, 3); // a Resident request is computing block 3 again
    t.end_recompute(0, 3);

    assert!(t.publish(0, 8)); // publishes -- over the hole
    t.check_invariants();
    assert!(!t.block_copied(0, 3));
    assert_eq!(t.durable_points(), 1);
    assert_eq!(t.restart_tokens(), 0); // the fresh point cannot restore
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `restore-point-chain-copies-keyed-on-disk-state-count`
// (riir-refine rust_perf corpus, Batch 270 / Plan 274) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/{KvCache,StateCache,
// Cache}.{hpp,cpp} @ a20f344 + 65f9713 (Apache-2.0): a persistent KV tier of
// block chains with per-block disk copies and checkpoint states, under a
// unit quota, with a two-step reclaim (unneeded copies first, then the
// oldest restore point). The law: the "unneeded" copy predicate keys on the
// count of states WITH A DISK COPY at the block or below it (countDiskState),
// kept by the state cache from the FIRST ATTEMPT of a state's write through
// refusal-rollback and erase -- so making room for a write never takes its
// own chain, a redundant copy (a Full page's) goes once no state on disk
// restores through it, and a recomputed disk-only block KEEPS its copy
// (foreignCopy) instead of holing the chain on page adoption.
//
// ORIGINAL machinery -- never splash code verbatim. Two conversations share
// the tier (the minimal fork model: one conversation's exclusive blocks are
// not another's ancestors).
//
// Geometry: block ids 1..=DEPTH per chain; parent(b) = b - 1 within the
// chain. A state at depth d restores through the copies of blocks 1..=d of
// its chain. A block's copy is REDUNDANT while a page also holds its K/V (a
// resident state's chain, or a transient request page), else it is the ONLY
// copy. The chain's structural frontier covers the blocks an in-flight write
// will need, so the leaf rule never strips a chain a write is extending.
// Each resident copy and each state's restore point is one quota unit.

pub const TOKENS_PER_BLOCK: usize = 1024;
pub const CHAINS: usize = 2;

#[derive(Clone, Copy)]
pub struct State {
    pub chain: usize,
    pub at: usize,
    pub ram: bool,
    pub disk: bool,
}

pub struct Tier {
    quota: usize,
    used: usize,
    depth: usize, // per-chain max depth
    frontier: Vec<usize>, // per chain: the deepest block the chain reaches
    copy_of: Vec<Vec<bool>>,
    foreign: Vec<Vec<bool>>,  // the copy holds what another command computed
    pages: Vec<Vec<u32>>,     // RAM K/V holders per block
    transient: Vec<Vec<u32>>, // request pages (no state behind them)
    ram_below: Vec<Vec<u32>>,
    disk_below: Vec<Vec<u32>>,
    states_at: Vec<Vec<u32>>,
    pub states: Vec<State>,
    // instrumentation (the recovery economics)
    pub points_dropped: usize,
    pub copies_dropped: usize,
    pub writes_refused: usize,
}

impl Tier {
    pub fn new(quota: usize, depth: usize) -> Self {
        Tier {
            quota,
            used: 0,
            depth,
            frontier: vec![0; CHAINS],
            copy_of: vec![vec![false; depth + 1]; CHAINS],
            foreign: vec![vec![false; depth + 1]; CHAINS],
            pages: vec![vec![0; depth + 1]; CHAINS],
            transient: vec![vec![0; depth + 1]; CHAINS],
            ram_below: vec![vec![0; depth + 1]; CHAINS],
            disk_below: vec![vec![0; depth + 1]; CHAINS],
            states_at: vec![vec![0; depth + 1]; CHAINS],
            states: Vec::new(),
            points_dropped: 0,
            copies_dropped: 0,
            writes_refused: 0,
        }
    }

    pub fn used(&self) -> usize {
        self.used
    }

    pub fn durable_points(&self) -> usize {
        self.states.iter().filter(|s| s.disk).count()
    }

    pub fn block_copied(&self, chain: usize, b: usize) -> bool {
        self.copy_of[chain][b]
    }

    // The deepest disk state whose whole chain of copies is resident, across
    // all conversations: what the next process takes back after a kill.
    pub fn restart_tokens(&self) -> usize {
        let mut best = 0usize;
        for s in self.states.iter().filter(|s| s.disk) {
            if (1..=s.at).all(|b| self.copy_of[s.chain][b]) {
                best = best.max(s.at * TOKENS_PER_BLOCK);
            }
        }
        best
    }

    fn add_range(v: &mut [u32], lo: usize, hi: usize, d: i32) {
        for b in lo..=hi {
            v[b] = (v[b] as i32 + d) as u32;
        }
    }

    // A checkpoint published at depth `at` of `chain`, RAM-resident, written
    // behind. The disk count runs from the FIRST ATTEMPT of the write, so
    // making room for it never takes its own chain; a refused write counts
    // for no chain.
    pub fn publish(&mut self, chain: usize, at: usize) -> bool {
        let new_blocks = at.saturating_sub(self.frontier[chain]);
        let first_new = self.frontier[chain] + 1;
        self.frontier[chain] = self.frontier[chain].max(at);
        self.disk_below_add(chain, 1, at, 1); // from the first attempt
        let needed = new_blocks + 1; // new blocks + the point
        while self.used + needed > self.quota && self.reclaim_step() {}
        if self.used + needed > self.quota {
            self.disk_below_add(chain, 1, at, -1); // counts for no chain
            self.writes_refused += 1;
            return false;
        }
        for b in first_new..=at {
            self.copy_of[chain][b] = true;
            self.used += 1;
        }
        self.states.push(State {
            chain,
            at,
            ram: true,
            disk: true,
        });
        self.used += 1;
        self.states_at[chain][at] += 1;
        self.pages_add(chain, 1, at, 1);
        self.ram_below_add(chain, 1, at, 1);
        true
    }

    // The state's RAM K/V pages go (memory pressure); the state becomes
    // disk-only.
    pub fn drop_ram(&mut self, chain: usize, at: usize) {
        let i = self
            .states
            .iter()
            .rposition(|s| s.chain == chain && s.at == at && s.ram)
            .expect("no RAM state there");
        self.states[i].ram = false;
        self.pages_add(chain, 1, at, -1);
        self.ram_below_add(chain, 1, at, -1);
    }

    // The state is erased entirely: an erased state keeps no count.
    pub fn erase(&mut self, chain: usize, at: usize) {
        let i = self
            .states
            .iter()
            .rposition(|s| s.chain == chain && s.at == at)
            .expect("no state there");
        let s = self.states.remove(i);
        self.states_at[chain][at] -= 1;
        if s.disk {
            self.disk_below_add(chain, 1, at, -1); // the disk copy went with it
            self.used -= 1;
        }
        if s.ram {
            self.pages_add(chain, 1, at, -1);
            self.ram_below_add(chain, 1, at, -1);
        }
    }

    // A request computes block `at` again and a page lands there. The copy,
    // which holds what another command computed, STAYS: a restore point's
    // chain on disk may need it (foreignCopy, 65f9713).
    pub fn recompute(&mut self, chain: usize, at: usize) {
        self.pages[chain][at] += 1;
        self.transient[chain][at] += 1;
        if self.copy_of[chain][at] {
            self.foreign[chain][at] = true;
        }
    }

    // The recomputing request ends; its page goes. The copy remains: it
    // holds the block's only K/V now, whoever computed it.
    pub fn end_recompute(&mut self, chain: usize, at: usize) {
        self.pages[chain][at] -= 1;
        self.transient[chain][at] -= 1;
        if self.pages[chain][at] == 0 {
            self.foreign[chain][at] = false;
        }
    }

    // One reclaim step: an unneeded copy first, then the oldest point.
    fn reclaim_step(&mut self) -> bool {
        if self.drop_unneeded_copy() {
            return true;
        }
        self.drop_oldest_point()
    }

    // THE LAW: unneeded is keyed on the disk-state counts.
    fn unneeded(&self, chain: usize, b: usize) -> bool {
        if self.pages[chain][b] > 0 {
            // a redundant copy (a Full page's): a state in RAM restores
            // through the page, so it is unneeded once no state on disk
            // sits at or below the block
            self.disk_below[chain][b] == 0
        } else {
            // the only copy of a leaf: once no state sits on it
            self.states_at[chain][b] == 0 && b == self.frontier[chain]
        }
    }

    fn drop_unneeded_copy(&mut self) -> bool {
        for chain in 0..CHAINS {
            for b in 1..=self.depth {
                if self.copy_of[chain][b] && self.unneeded(chain, b) {
                    self.copy_of[chain][b] = false;
                    self.used -= 1;
                    self.copies_dropped += 1;
                    return true;
                }
            }
        }
        false
    }

    // The oldest restore point of a state also in RAM: the durable copies
    // this order spends only when no copy is unneeded.
    fn drop_oldest_point(&mut self) -> bool {
        let victim = self.states.iter().position(|s| s.disk && s.ram);
        match victim {
            Some(i) => {
                self.states[i].disk = false;
                self.disk_below_add(self.states[i].chain, 1, self.states[i].at, -1);
                self.used -= 1;
                self.points_dropped += 1;
                true
            }
            None => false,
        }
    }

    fn pages_add(&mut self, chain: usize, lo: usize, hi: usize, d: i32) {
        Self::add_range(&mut self.pages[chain], lo, hi, d);
    }

    fn ram_below_add(&mut self, chain: usize, lo: usize, hi: usize, d: i32) {
        Self::add_range(&mut self.ram_below[chain], lo, hi, d);
    }

    fn disk_below_add(&mut self, chain: usize, lo: usize, hi: usize, d: i32) {
        Self::add_range(&mut self.disk_below[chain], lo, hi, d);
    }

    // A RAM-resident state always restores (through its pages).
    pub fn restore_ram(&self, chain: usize, at: usize) -> bool {
        (1..=at).all(|b| self.pages[chain][b] > 0)
    }

    // The counters agree with the state vector (the checkInvariants echo).
    pub fn check_invariants(&self) {
        let mut pages = vec![vec![0u32; self.depth + 1]; CHAINS];
        let mut ram_below = vec![vec![0u32; self.depth + 1]; CHAINS];
        let mut disk_below = vec![vec![0u32; self.depth + 1]; CHAINS];
        let mut states_at = vec![vec![0u32; self.depth + 1]; CHAINS];
        for s in &self.states {
            if s.ram {
                Self::add_range(&mut pages[s.chain], 1, s.at, 1);
                Self::add_range(&mut ram_below[s.chain], 1, s.at, 1);
            }
            if s.disk {
                Self::add_range(&mut disk_below[s.chain], 1, s.at, 1);
            }
            states_at[s.chain][s.at] += 1;
        }
        for chain in 0..CHAINS {
            for b in 1..=self.depth {
                pages[chain][b] += self.transient[chain][b];
                // a copy of another command's K/V sits beside a full page
                assert!(
                    !self.foreign[chain][b] || self.pages[chain][b] > 0,
                    "foreign at {chain}:{b}"
                );
            }
            assert_eq!(pages[chain], self.pages[chain], "pages {chain}");
            assert_eq!(ram_below[chain], self.ram_below[chain], "ram_below {chain}");
            assert_eq!(disk_below[chain], self.disk_below[chain], "disk_below {chain}");
            assert_eq!(states_at[chain], self.states_at[chain], "states_at {chain}");
        }
        let copies: usize = self
            .copy_of
            .iter()
            .map(|c| c.iter().filter(|&&x| x).count())
            .sum();
        let points = self.states.iter().filter(|s| s.disk).count();
        assert_eq!(self.used, copies + points, "used == copies + points");
        assert!(self.used <= self.quota, "quota holds");
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_ram_states_always_restore_and_the_quota_holds() {
    let mut t = Tier::new(64, 32);
    for d in [2usize, 4, 6, 8, 10, 12] {
        assert!(t.publish(0, d));
        t.check_invariants();
        assert!(t.restore_ram(0, d));
    }
    assert_eq!(t.durable_points(), 6);
    assert_eq!(t.restart_tokens(), 12 * TOKENS_PER_BLOCK);
}

#[test]
fn eq_a_refused_write_leaves_no_state_and_no_count() {
    let mut t = Tier::new(2, 8);
    assert!(!t.publish(0, 4)); // needs 5 units under a 2-unit quota
    assert_eq!(t.writes_refused, 1);
    assert_eq!(t.durable_points(), 0);
    t.check_invariants(); // the counts rolled back cleanly
}

#[test]
fn eq_an_erased_state_keeps_no_count() {
    let mut t = Tier::new(16, 16);
    assert!(t.publish(0, 4));
    assert!(t.publish(0, 6));
    t.erase(0, 4);
    t.check_invariants();
    assert_eq!(t.durable_points(), 1);
    assert_eq!(t.restart_tokens(), 6 * TOKENS_PER_BLOCK);
}

// -- the law's own shape -----------------------------------------------------

#[test]
fn law_the_soak_keeps_its_restore_points_under_ram_pressure() {
    // The same soak the anti-pattern arm dies in: two conversations share
    // the tier, the short one's checkpoints die first -- but keyed on the
    // disk counts, its stale chain copies (still paged by its RAM state)
    // become unneeded the moment no disk state restores through them, so
    // the reclaim sheds COPIES and the growing conversation's restore
    // points stay.
    let mut t = Tier::new(30, 32);
    // the short conversation checkpoints first, so its points are the oldest
    assert!(t.publish(0, 4));
    assert!(t.publish(0, 8));
    for d in [4usize, 8, 12, 16] {
        assert!(t.publish(1, d));
        t.check_invariants();
    }
    // the next checkpoint of the long conversation must make room
    assert!(t.publish(1, 20));
    t.check_invariants();

    assert_eq!(t.writes_refused, 0);
    assert_eq!(t.points_dropped, 2, "only the dead conversation's points");
    assert_eq!(t.durable_points(), 5, "the long conversation keeps every point");
    assert!(
        t.copies_dropped >= 3,
        "the disk-keyed order sheds the dead chain instead: {}",
        t.copies_dropped
    );
    assert_eq!(t.restart_tokens(), 20 * TOKENS_PER_BLOCK);
}

#[test]
fn law_a_recompute_keeps_the_chain_whole() {
    // 65f9713's face: a request computes a disk-only chain block again and
    // adopts it -- the copy stays (foreign), so after a kill the restore
    // point's chain has no hole.
    let mut t = Tier::new(32, 16);
    assert!(t.publish(0, 6));
    t.drop_ram(0, 6); // the state's RAM pages go; the restore point remains
    assert_eq!(t.restart_tokens(), 6 * TOKENS_PER_BLOCK);

    t.recompute(0, 3); // a later request computes block 3 again
    t.check_invariants();
    assert!(t.block_copied(0, 3)); // the copy stayed
    t.end_recompute(0, 3); // the request ends; the copy remains, alone now
    t.check_invariants();
    assert!(t.block_copied(0, 3));
    assert_eq!(t.restart_tokens(), 6 * TOKENS_PER_BLOCK); // the point is whole
}

#[test]
fn law_making_room_never_takes_the_in_flight_chain() {
    // The write's own chain is counted from the first attempt: with a
    // transient request page the only reclaimable-looking copy is the block
    // the in-flight state will restore through -- and the tier refuses the
    // write rather than take it. The anti-pattern arm publishes the same
    // write over a hole.
    let mut t = Tier::new(8, 16);
    assert!(t.publish(0, 4));
    assert!(t.publish(0, 6));
    t.drop_ram(0, 6); // both states' pages decay
    t.drop_ram(0, 4);
    t.erase(0, 6); // the states go; the chain copies remain
    t.erase(0, 4);
    t.check_invariants();
    t.recompute(0, 3); // a Resident request is computing block 3 again

    // the in-flight chain is protected: no unneeded copy, no point to drop
    assert!(!t.publish(0, 8));
    assert_eq!(t.writes_refused, 1);
    assert!(t.block_copied(0, 3)); // the chain is intact
    assert_eq!(t.durable_points(), 0);
    t.end_recompute(0, 3); // the request ends
    t.check_invariants();
    assert!(t.block_copied(0, 3)); // the copy remains: the chain has no hole
}
"#.to_string(),
            context_notes: Some("Grade B+ -- NotBenchable ON THIS BOX (the decision content is the SERVING-LEVEL recovery economics of a persistent KV tier under quota -- which copies the reclaim may take, keyed on which liveness counter -- and a Rust microbench of the stand-in cannot measure it; the source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ a20f344e25a37ef4a58a7f4d1c78ef3d32d69cd8 'Free a dropped point's chain copies before the next point goes' + 65f971346ecad4b01362767daa394393040ab3c6 'Keep the copies of a restore point's chain that a request computes again', Apache-2.0, Jian Chen, 2026-10-20; every quote re-verified verbatim at the .raw/b270/splash clone this batch at those pins; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (a20f344 message, verbatim): 'A persistent tier's quota kept the KV copies of states that stay in RAM and gave up the durable restore points instead. dropOldestPoint only drops the disk copy of a state also in RAM, and a copy was unneeded only with no state at all at its block or below, so the chain's copies stayed and the next call took the next point, contrary to Cache.hpp.' MEASURED (a20f344 message, source-reported, the 64 GB M5 Pro agentic soak on a 35B GGUF): 'the disk's restore points fell from 51 to 0-1 while KV copies held 96% of the 7.8 GiB quota, write-behind refused 25-31 points, and after a SIGKILL a conversation cached at 104,928 tokens restarted from 0 (TTFT 588 s).' THE LAW (runtime/engine/KvCache.hpp:155-158 @ a20f344, verbatim): 'In a persistent cache, StateCache counts each state's disk copy in from the first attempt of its write and out when it goes: the redundant copies of a chain are what the next process takes back for the states on disk at their blocks or below.' -- the new per-block 'uint32_t diskStates = 0;' (KvCache.hpp:264) fed by 'void KvCache::countDiskState(uint64_t blockId, bool added) noexcept' (KvCache.cpp:348), whose reindex trigger is the comment 'Its redundant copy is needed again, or no longer.' (KvCache.cpp:357) over 'if (entry.diskStates == (added ? 1u : 0u)) reindex(entry);'. THE ORDER PREDICATE (KvCache.cpp:683 @ a20f344, verbatim code): 'result.unneeded = persistent_ && (full ? !entry.diskStates : !entry.statesHere && !entry.children);' -- a redundant copy (a Full page's) 'is unneeded once no state on disk sits at or below it, as a state in RAM restores through the page; the only copy of a spilled or disk-only leaf is unneeded, as before, once no state sits on it' (message). THE WRITE-SITE LIFECYCLE (runtime/engine/StateCache.cpp @ a20f344): the count is taken BEFORE the room-making loop -- 'The copy counts from the first attempt: making room for it gives up none of the KV copies it restores through.' (StateCache.cpp:617) over 'kv_.countDiskState(kvBlock, true);' (:619), rolled back on refusal at :624 ('a point refused for want of room counts for no chain' -- the new test 'fails when the count starts only once the write does, or when an erased state keeps its count'), counted out at erase (:521). THE RETENTION ORDER ITSELF (runtime/engine/Cache.hpp:871-881 @ a20f344, verbatim): 'In a persistent tier a redundant copy is what the next process takes back, and the unit worth keeping is a restore point, a state on disk and its chain. A copy no state needs goes first (dropUnneededCopy), then the oldest state copy, redundant or not: the copies of its chain that no other state needs become unneeded and go on later calls, so a chain stays whole while a state on disk needs it.' THE RECOMPUTE FACE (65f9713 message, verbatim): 'a disk-only block a request computed again gave its copy up when it adopted the writer's page (KvCache::insert, since 214a9cd3), even on the chain of a state on disk. A Resident request owes no copy, so the chain on disk kept a hole: after a kill, or a clean stop that does not write the point again, the next start lost the point and all below the hole.' THE REPAIR: 'The block now keeps the copy, marked as another computation than its page (foreignCopy), until a copy of the page replaces it or the page goes.' -- 'Only a copy of what the page holds is active KV.' (KvCache.cpp:467), the accessor doc 'The block's K/V has a copy: a slot that holds what its page holds, or its only K/V. The copy a disk-only block keeps beside the page a writer filled it with (insert) is none: a restore point's chain keeps it, until a copy of the page replaces it or the page goes.' (KvCache.hpp:152-155), pinned by the invariant 'a copy of another command's K/V sits beside a Full page' (KvCache.cpp:752). The insertion-site comment (KvCache.cpp:117-119 @ 65f9713, verbatim): 'The copy, which holds what another command computed, stays: a restore point's chain on disk may need it. The block spills only once a copy of the writer's page replaced it.' REGIME: persistent/spilling KV caches with restore points (checkpoint states whose restore reads the chain's disk copies) under a byte quota shared by RAM-resident K/V and durable copies; any eviction order whose 'reclaimable' predicate is keyed on a proxy liveness counter (RAM residency) while the durable value is keyed on another (disk states). ANTI-REGIMES: tiers without restore points (an LRU copy is worthless the moment its page is gone -- the ordinary eviction orders apply); non-persistent caches ('Only a persistent cache keeps the unneeded order, its only user'); copies whose redundancy cannot be distinguished from their only-copy role. KINSHIP/DISTINCT-FROM (read + disclosed): partial-state-serialization-honors-rollback-contract-at-leaf-cache (B239, entries_27) owns WHAT a partial write owes at the leaf cache (the size-monotonicity invariant) -- THIS owns which already-resident copies the quota's eviction may take; compose. cache-write-admission-requires-restoring-consumer (B239, entries_27) owns WHICH states may enter the cache at all (write-path admission) -- THIS owns retention after admission; compose. reuse-admission-threshold-must-equal-window-semantics (B241, entries_28) owns the reuse ACCEPTANCE bound -- distinct axis (admission vs eviction-keying). seq-fork-carries-pending-rollback-record (B246) owns fork-time mutation-record travel -- unrelated. prompt-cache-rewind-states-min-gap-eviction (B219) owns the eviction GAP policy for rewind states -- THIS owns the liveness counter that keys copy retention. prompt-cache-stale-invalidation-push-borrower-boundary (kernel_opt B249 slice 2) owns the push-invalidation protocol -- unrelated axis. pinned-entry-tombstone-freed-at-last-unpin (B256, entries_36) owns in-flight HOLDER lifetime -- THIS owns durability-keyed retention, no holder class. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'restore point|chain copy|countDiskState|disk state|unneeded order|retention predicate' -- zero owners (the kernel_opt wave notes stage this lead as 'disk-state-keyed redundancy' for a rust_perf batch -- this IS that landing); 'disk copy' hits are unrelated prose (entries_gpu_95 coherence rule). The kernel_opt Batch 268 splash-wave header (entries_gpu_117) explicitly stages this lead for a rust_perf batch. FIXTURE PROVENANCE: original machinery (never splash code) -- a two-chain persistent tier (the minimal fork model: one conversation's exclusive blocks are not another's ancestors) with per-block disk copies, checkpoint states (RAM-resident or disk-only), a unit quota, the two-step reclaim, transient request pages, and the recompute/adoption face; the shared-contract tests (quota holds, RAM states restore, refusals leave no count, erased states keep no count, a checkInvariants echo) pass in BOTH arms; the discriminating tests pin the soak (buggy spends points: 4 dropped/3 durable/0 copies vs fixed: 2 dropped/5 durable/3+ copies), the recompute hole (restart 0 vs full), and the in-flight write (publishes over a hole vs refuses). Wave: Batch 270 / Plan 274 / Research 306 (the splash-wave serving pool staged by the kernel_opt Batch 268 header). Cite: incoai/splash @ a20f344e25a37ef4a58a7f4d1c78ef3d32d69cd8 + 65f971346ecad4b01362767daa394393040ab3c6 -- runtime/engine/KvCache.hpp:155-158 + :264, runtime/engine/KvCache.cpp:348 + :357 + :683 (@ a20f344) and KvCache.cpp:117-119 + :467 + :752 (@ 65f9713), runtime/engine/StateCache.cpp:617-624 + :521, runtime/engine/Cache.hpp:871-881, commit messages (all quotes); Batch 270 / Plan 274. RIDER (Batch 271 / Plan 275, incoai/splash @ 253506f393b17c9253e0a105f479908c7a3d15d7 'Give the spilled leaf of a state in RAM only to a Spill request's page', Apache-2.0, Jian Chen, 2026-10-19 -- the SPILL-TIER face of the same liveness-keyed exclusion, runtime/engine/Cache.cpp): the host rule owns which copies the reclaim may take in the PERSISTENT tier, keyed on the disk-state counts; this face is the same exclusion arriving in the TEMPORARY/spill tier, triggered by a new holder shape -- a Spill request's replay point, whose pages below were dropped after the request ended, leaves the state sitting on a spilled leaf, and the leaf's victim scan must skip it. The widened victim predicate (Cache.cpp:1464-1471, verbatim): '// A leaf holding a state in use stays with that state, and one holding a state in RAM, as a spilled leaf can, stays until reclaim takes the state or a committed page evicts it (evictStateOnDiskLeaf): a reclaim makes room here for that state's own write, or for a demotion beside the state it picked.' over 'while (kvLeaf && (kvNeededByStateInUse(kvLeaf->id) || states_.stateResident(kvLeaf->id))) kvLeaf = kv_.diskCandidate(false, kvLeaf->id);'. The exclusion is also a UAF guard: freeDiskSpace runs as makeRoom UNDER StateCache::reclaim, which holds the entry it is writing -- the scan must never take the very state whose room it is making. The ONLY arm that may take a RAM-resident state is the committed writer's draw, once nothing else is left (Cache.cpp:1441-1448, verbatim): 'bool Cache::evictStateOnDiskLeaf() { for (auto leaf = kv_.diskCandidate(false); leaf; leaf = kv_.diskCandidate(false, leaf->id)) { if (states_.stateResident(leaf->id) && !kvNeededByStateInUse(leaf->id)) { states_.evict(leaf->id); return true; } } return false; }', gated in acquireDiskSlot (Cache.cpp:1432-1437) on 'if (!freeDiskSpace(inUse) && !(committed && evictStateOnDiskLeaf())) return {};'. Same law (reclaimable keyed on which liveness counter), new tier + new holder shape + the committed-draw exception arm. Count-neutral rider, Batch 271 / Plan 275. RIDER (Batch 271 / Plan 275, incoai/splash @ 4fc7465e85648402bbe8d2db5d8da8e2bb443841 'Relabel a state's disk copy when its checkpoint becomes ordinary', Apache-2.0, Jian Chen, 2026-10-20 -- the DURABLE-LABEL-AT-RECLASSIFICATION face, runtime/engine/StateCache.cpp): the host rule owns WHICH copies the reclaim takes; this face owns WHEN a state's durable record learns its class changed. The defect (message, verbatim): 'A checkpoint on disk that a request reused as an ordinary state (reuseStoredState, reuseCompositeState) or resumed from (useState) kept checkpoint=1 in its record until relabel() at a clean stop: StateCache::makeOrdinary changed the entry's class, not its copy's label. After a SIGKILL the next process took it back as a disposable checkpoint, which a resuming lane retires.' -- the crash window reads the STALE class and its eviction policy: a state a lane resumed from/reused gets retired as disposable, the cached tokens lost. The repair (message): 'makeOrdinary now labels the disk copy with its new class at once in a persistent tier (labelDisk, as relabel() does).', in-source (StateCache.cpp:242-246, verbatim): '// A crash before the next clean stop must not leave it a checkpoint the next process retires.' over 'if (entry.disk && !entry.invalid) labelDisk(kvBlock, entry);'. Distinct decision from the host's retention counts (label TIMING vs retention COUNTING), same crash-recovery family: a durable record must carry its class at the moment the class changes, not at the next flush. Count-neutral rider, Batch 271 / Plan 275.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "persistent-cache-teardown-needs-second-witness".to_string(),
            name: "A persistent cache's crash-recovery teardown fires on ONE unattributable failure -- a start that failed within probation empties the whole warm tier at the next start, though the rule cannot tell a cached point that breaks the engine from a kill, jetsam or an engine bug of another cause → destructive teardown needs a SECOND witness: every start serves on probation; the start that finds the last start's probation mark marks the failure (failed-probation), and the start that finds BOTH marks empties the cache; an engine-REPORTED failure within probation (an exception at the native loop's failure boundary) marks the failure itself and the next start empties as before; a start that serves its probation out clears both -- so the cache goes after two fast failures in a row, or one fast engine-reported failure, while one kill or crash of another cause costs nothing".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `persistent-cache-teardown-needs-second-witness` (riir-refine
// rust_perf corpus, Batch 270 / Plan 274) -- the ANTI-PATTERN arm.
//
// Re-authored shape from incoai/splash runtime/engine/CacheDirectory.{hpp,cpp}
// @ 6e21cc9 (Apache-2.0): the cross-start mark machine that decides whether a
// persistent cache of restore points is EMPTIED at boot. The anti-pattern
// fires destructive teardown on ONE unattributable failure: a start that
// failed within its probation leaves the probation mark, and the next start
// that finds it empties the warm tier -- though the rule cannot tell a
// cached point that breaks the engine from a kill, jetsam or an engine bug
// of another cause. The fixed arm adds the failed-probation mark (the
// second witness) and the engine-reported failure path.
//
// ORIGINAL machinery -- never splash code verbatim. Probation is modeled as
// a logical phase (served out vs failed within), never a clock.

use std::cell::RefCell;
use std::rc::Rc;

// The on-"disk" state carried between starts (the mark set).
#[derive(Default)]
pub struct Marks {
    pub serving: bool,
    pub probation: bool,
}

#[derive(Default)]
pub struct Tier {
    pub points: usize,
    pub bytes_mib: u64,
}

pub struct World {
    pub marks: Marks,
    pub tier: Tier,
    // instrumentation (the perf axis: each empty destroys the warm tier)
    pub empties: u64,
    pub bytes_destroyed_mib: u64,
    pub starts: u64,
    pub cold_reason: Option<&'static str>,
}

pub struct Start {
    world: Rc<RefCell<World>>,
    open: bool,
}

pub fn world() -> Rc<RefCell<World>> {
    Rc::new(RefCell::new(World {
        marks: Marks::default(),
        tier: Tier::default(),
        empties: 0,
        bytes_destroyed_mib: 0,
        starts: 0,
        cold_reason: None,
    }))
}

// The boot: read the marks the last process left, maybe empty the cache,
// then begin serving.
pub fn start(w: &Rc<RefCell<World>>) -> Start {
    let mut b = w.borrow_mut();
    b.starts += 1;
    if b.marks.probation {
        // ONE failure within probation: empty the cache now.
        b.empties += 1;
        b.bytes_destroyed_mib += b.tier.bytes_mib;
        b.tier = Tier::default();
        b.marks.probation = false;
        b.cold_reason = Some("the last start failed within its probation");
    }
    // beginServing: a start after an unclean end serves on probation.
    let after_unclean = b.marks.serving;
    b.marks.serving = true;
    if after_unclean {
        b.marks.probation = true;
    }
    drop(b);
    Start {
        world: Rc::clone(w),
        open: true,
    }
}

impl Start {
    // The warm start takes the tier's restore points back.
    pub fn took_back(&mut self, points: usize, bytes_mib: u64) {
        assert!(self.open);
        let mut w = self.world.borrow_mut();
        w.tier = Tier { points, bytes_mib };
    }

    // The start cached new points while serving.
    pub fn cached(&mut self, points: usize, bytes_mib: u64) {
        assert!(self.open);
        let mut w = self.world.borrow_mut();
        w.tier.points += points;
        w.tier.bytes_mib += bytes_mib;
    }

    // The process died inside its probation (a kill, a crash): the marks
    // stand.
    pub fn fail_within_probation(self) {
        assert!(self.open);
        // nothing cleared: serving + probation remain
    }

    // A failure after probation: it counts for nothing.
    pub fn fail_after_probation(self) {
        assert!(self.open);
        let mut w = self.world.borrow_mut();
        w.marks.probation = false;
        w.marks.serving = true; // the end was still unclean
    }

    // The process has served long enough.
    pub fn serve_probation_out(&mut self) {
        assert!(self.open);
        self.world.borrow_mut().marks.probation = false;
    }

    // A clean end.
    pub fn close_cleanly(mut self) {
        assert!(self.open);
        let mut w = self.world.borrow_mut();
        w.marks.probation = false;
        w.marks.serving = false;
        self.open = false;
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_a_bad_point_still_ends_the_run_within_the_limit() {
    // A point that breaks EVERY start that takes it back: the next start
    // after a probation failure begins empty. The containment contract both
    // arms satisfy (the arms differ only in WHICH boot empties -- and in
    // that the fixed arm puts every start on probation, where this arm
    // needed a prior unclean end to arm one).
    let w = world();
    let s = start(&w);
    s.fail_within_probation(); // a prior run's unclean end

    let mut s = start(&w); // takes the cache back after an unclean end: ON probation
    assert!(w.borrow().marks.probation);
    s.took_back(31, 4264);
    s.fail_within_probation(); // failure 1, within probation

    let s = start(&w); // found the probation mark: EMPTIES
    assert_eq!(w.borrow().empties, 1);
    assert_eq!(w.borrow().tier.points, 0);
    drop(s);
}

#[test]
fn eq_a_failure_after_probation_counts_for_nothing() {
    let w = world();
    let s = start(&w);
    s.fail_within_probation(); // a prior run's unclean end
    let mut s = start(&w); // on probation
    s.took_back(31, 4264);
    s.serve_probation_out();
    s.fail_within_probation(); // died after probation: an ordinary unclean end
    let mut s = start(&w); // probation again (unclean end) but NO failure
    assert_eq!(w.borrow().empties, 0);
    assert_eq!(w.borrow().tier.points, 31);
    s.serve_probation_out();
    s.close_cleanly();
    assert_eq!(w.borrow().empties, 0);
}

#[test]
fn eq_a_clean_end_then_a_start_leaves_the_marks_clear() {
    let w = world();
    let mut s = start(&w);
    s.took_back(31, 4264);
    s.serve_probation_out();
    s.close_cleanly();
    let s = start(&w);
    assert_eq!(w.borrow().empties, 0);
    assert!(!w.borrow().marks.probation); // a clean end: no probation
    assert_eq!(w.borrow().tier.points, 31);
    s.close_cleanly();
}

#[test]
fn eq_an_engine_reported_failure_empties_too() {
    // The engine's own failure boundary: in the anti-pattern arm the report
    // goes nowhere -- the rule saw only an unclean end within probation, and
    // the next start empties all the same.
    let w = world();
    let s = start(&w);
    s.fail_within_probation(); // a prior run's unclean end
    let mut s = start(&w); // on probation, takes the cache back
    s.took_back(31, 4264);
    s.fail_within_probation(); // the engine crashed at its failure boundary
    let s = start(&w);
    assert_eq!(w.borrow().empties, 1);
    assert_eq!(
        w.borrow().cold_reason,
        Some("the last start failed within its probation")
    );
    drop(s);
}

// -- the anti-pattern's own shape --------------------------------------------

#[test]
fn bug_one_spurious_kill_destroys_the_warm_tier() {
    // A start takes 31 restore points (4,264 MiB) back, serves 38 s, and a
    // SIGKILL lands. The next start finds the probation mark and EMPTIES:
    // 4,264 MiB of warm start destroyed for a failure the rule cannot
    // attribute to any cached point.
    let w = world();
    let s = start(&w);
    s.fail_within_probation(); // a prior run's unclean end (the tier exists)
    let mut s = start(&w); // takes the cache back after an unclean end
    s.took_back(31, 4264);
    s.fail_within_probation(); // the SIGKILL, 38 s after Ready

    let s = start(&w);
    assert_eq!(w.borrow().empties, 1);
    assert_eq!(w.borrow().bytes_destroyed_mib, 4264);
    assert_eq!(w.borrow().tier.points, 0); // the warm tier is gone
    drop(s);
}

#[test]
fn bug_the_smoke_emptied_twice_in_44_seconds() {
    // The measured shape: an engine-reported prefill crash 38 s after Ready
    // on a start that had taken 31 points back; then the harness's own
    // SIGKILL 34 s into the next start, which had taken back NOTHING and
    // cached for that long. Both empties fire; the second destroys a tier
    // the smoke itself had just built.
    let w = world();
    let s = start(&w);
    s.fail_within_probation(); // a prior run's unclean end
    let mut s = start(&w); // on probation, takes 31 points back
    s.took_back(31, 4264);
    s.fail_within_probation(); // the prefill-passes crash

    let mut s = start(&w); // EMPTIES (#1: 4,264 MiB)
    assert_eq!(w.borrow().empties, 1);
    s.cached(9, 1160); // the smoke caches 34 s worth
    s.fail_within_probation(); // the smoke's own SIGKILL

    let s = start(&w); // EMPTIES (#2: 1,160 MiB of fresh cache)
    assert_eq!(w.borrow().empties, 2);
    assert_eq!(w.borrow().bytes_destroyed_mib, 4264 + 1160);
    drop(s);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `persistent-cache-teardown-needs-second-witness` (riir-refine
// rust_perf corpus, Batch 270 / Plan 274) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/CacheDirectory.{hpp,cpp}
// @ 6e21cc9 (Apache-2.0): destructive teardown of a persistent cache needs a
// SECOND witness. Every start serves on probation; the start that finds the
// last start's probation mark marks the failure (failed-probation), and the
// start that finds BOTH marks empties the cache; an engine-REPORTED failure
// within probation (an exception at the native loop's failure boundary)
// marks the failure itself and the next start empties as before; a start
// that serves its probation out clears both. So the cache goes after two
// fast failures in a row, or one fast engine-reported failure, while one
// kill or crash of another cause costs nothing.
//
// ORIGINAL machinery -- never splash code verbatim. Probation is modeled as
// a logical phase (served out vs failed within), never a clock.

use std::cell::RefCell;
use std::rc::Rc;

// The on-"disk" state carried between starts (the mark set).
#[derive(Default)]
pub struct Marks {
    pub serving: bool,
    pub probation: bool,
    pub failed_probation: bool,
}

#[derive(Default)]
pub struct Tier {
    pub points: usize,
    pub bytes_mib: u64,
}

pub struct World {
    pub marks: Marks,
    pub tier: Tier,
    // instrumentation (the perf axis: each empty destroys the warm tier)
    pub empties: u64,
    pub bytes_destroyed_mib: u64,
    pub starts: u64,
    pub cold_reason: Option<&'static str>,
}

pub struct Start {
    world: Rc<RefCell<World>>,
    open: bool,
    begins_empty: bool,
}

pub fn world() -> Rc<RefCell<World>> {
    Rc::new(RefCell::new(World {
        marks: Marks::default(),
        tier: Tier::default(),
        empties: 0,
        bytes_destroyed_mib: 0,
        starts: 0,
        cold_reason: None,
    }))
}

// The boot: read the marks the last process left, maybe empty the cache,
// then begin serving (EVERY start on probation).
pub fn start(w: &Rc<RefCell<World>>) -> Start {
    let mut b = w.borrow_mut();
    b.starts += 1;
    let mut begins_empty = false;
    // The last start failed within its probation: the cache goes when the
    // one before it did too, or when its engine reported the failure.
    if b.marks.probation {
        if b.marks.failed_probation {
            b.empties += 1;
            b.bytes_destroyed_mib += b.tier.bytes_mib;
            b.tier = Tier::default();
            b.marks.failed_probation = false;
            begins_empty = true;
            b.cold_reason = Some(
                "the last two starts failed within their probation, or the last one's engine did",
            );
        } else {
            // the previous start failed within its probation: mark the
            // failure; this start may yet serve it out
            b.marks.failed_probation = true;
        }
        b.marks.probation = false;
    }
    // beginServing: mark serving + probation.
    b.marks.serving = true;
    b.marks.probation = true;
    drop(b);
    Start {
        world: Rc::clone(w),
        open: true,
        begins_empty,
    }
}

impl Start {
    pub fn begins_empty(&self) -> bool {
        self.begins_empty
    }

    // The warm start takes the tier's restore points back.
    pub fn took_back(&mut self, points: usize, bytes_mib: u64) {
        assert!(self.open);
        let mut w = self.world.borrow_mut();
        w.tier = Tier { points, bytes_mib };
    }

    // The start cached new points while serving.
    pub fn cached(&mut self, points: usize, bytes_mib: u64) {
        assert!(self.open);
        let mut w = self.world.borrow_mut();
        w.tier.points += points;
        w.tier.bytes_mib += bytes_mib;
    }

    // The process died inside its probation (a kill, a crash): the marks
    // stand.
    pub fn fail_within_probation(self) {
        assert!(self.open);
        // nothing cleared: serving + probation [+ failed] remain
    }

    // A failure after probation: it counts for nothing.
    pub fn fail_after_probation(self) {
        assert!(self.open);
        self.end_probation(&self.world);
        // the end was still unclean: serving remains
    }

    // The engine reported its own failure at the native loop's failure
    // boundary: within probation, the next start empties the cache. The
    // start dies with the exception.
    pub fn engine_reported_failure(&mut self) {
        assert!(self.open);
        let mut w = self.world.borrow_mut();
        if w.marks.probation {
            w.marks.failed_probation = true;
        }
    }

    // The process has served long enough: neither a failure of it nor one
    // of the start before it counts toward emptying the cache.
    pub fn serve_probation_out(&mut self) {
        assert!(self.open);
        self.end_probation(&self.world);
    }

    fn end_probation(&self, w: &Rc<RefCell<World>>) {
        let mut w = w.borrow_mut();
        w.marks.failed_probation = false;
        w.marks.probation = false;
    }

    // A clean end.
    pub fn close_cleanly(mut self) {
        assert!(self.open);
        self.end_probation(&self.world);
        self.world.borrow_mut().marks.serving = false;
        self.open = false;
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_a_bad_point_still_ends_the_run_within_the_limit() {
    // A point that breaks EVERY start that takes it back: the third start of
    // the run begins empty, within a three-failures crash-loop limit. The
    // containment contract both arms satisfy (the arms differ only in WHICH
    // boot empties).
    let w = world();
    let mut s = start(&w);
    assert!(!s.begins_empty());
    s.took_back(31, 4264);
    s.fail_within_probation(); // failure 1

    let mut s = start(&w); // found probation: marks the failure, serves on
    assert!(!s.begins_empty());
    assert_eq!(w.borrow().empties, 0);
    assert_eq!(w.borrow().tier.points, 31); // the warm tier survived failure 1
    s.took_back(31, 4264);
    s.fail_within_probation(); // failure 2

    let mut s = start(&w); // found BOTH marks: begins EMPTY
    assert!(s.begins_empty());
    assert_eq!(w.borrow().empties, 1);
    assert_eq!(w.borrow().tier.points, 0);
    assert_eq!(
        w.borrow().cold_reason,
        Some("the last two starts failed within their probation, or the last one's engine did")
    );
    s.serve_probation_out(); // the bad point is gone: this start survives
    s.close_cleanly();
    assert_eq!(w.borrow().tier.points, 0);
}

#[test]
fn eq_a_failure_after_probation_counts_for_nothing() {
    let w = world();
    let mut s = start(&w);
    s.took_back(31, 4264);
    s.serve_probation_out();
    s.fail_within_probation(); // died after probation: an ordinary unclean end
    let mut s = start(&w); // probation again (unclean end) but NO failure marked
    assert_eq!(w.borrow().empties, 0);
    assert_eq!(w.borrow().tier.points, 31);
    assert!(!w.borrow().marks.failed_probation);
    s.serve_probation_out();
    s.close_cleanly();
    assert_eq!(w.borrow().empties, 0);
}

#[test]
fn eq_a_clean_end_then_a_start_leaves_the_marks_clear() {
    let w = world();
    let mut s = start(&w);
    s.took_back(31, 4264);
    s.serve_probation_out();
    s.close_cleanly();
    let mut s = start(&w);
    assert_eq!(w.borrow().empties, 0);
    assert!(!w.borrow().marks.failed_probation);
    assert_eq!(w.borrow().tier.points, 31);
    s.close_cleanly();
}

#[test]
fn eq_an_engine_reported_failure_empties_for_the_next_start() {
    // The engine's exception at the native loop's failure boundary marks
    // the failure itself: one fast engine-reported failure is enough.
    let w = world();
    let mut s = start(&w);
    s.took_back(31, 4264);
    s.engine_reported_failure(); // the prefill-passes crash
    s.fail_within_probation(); // the start dies with it

    let s = start(&w);
    assert_eq!(w.borrow().empties, 1);
    assert_eq!(w.borrow().bytes_destroyed_mib, 4264);
    assert_eq!(w.borrow().tier.points, 0);
    drop(s);
}

#[test]
fn eq_an_engine_reported_failure_after_probation_does_not_empty() {
    let w = world();
    let mut s = start(&w);
    s.took_back(31, 4264);
    s.serve_probation_out();
    s.engine_reported_failure(); // AFTER probation: counts for nothing
    let mut s = start(&w);
    assert_eq!(w.borrow().empties, 0);
    assert_eq!(w.borrow().tier.points, 31);
    s.serve_probation_out();
    s.close_cleanly();
}

// -- the law's own shape -----------------------------------------------------

#[test]
fn law_one_spurious_kill_no_longer_destroys_the_warm_tier() {
    // A start takes 31 restore points (4,264 MiB) back, serves 38 s, and a
    // SIGKILL lands. The next start marks the failure and KEEPS the tier;
    // once it serves its probation out, neither failure counts any more.
    let w = world();
    let mut s = start(&w);
    s.took_back(31, 4264);
    s.fail_within_probation(); // the SIGKILL, 38 s after Ready

    let mut s = start(&w);
    assert_eq!(w.borrow().empties, 0);
    assert_eq!(w.borrow().bytes_destroyed_mib, 0);
    assert_eq!(w.borrow().tier.points, 31); // the warm tier survived
    assert!(w.borrow().marks.failed_probation); // the failure is marked
    s.serve_probation_out();
    s.close_cleanly();

    let s = start(&w);
    assert_eq!(w.borrow().empties, 0);
    assert_eq!(w.borrow().tier.points, 31); // and it is still there
    s.close_cleanly();
}

#[test]
fn law_the_smoke_empties_once_not_twice() {
    // The measured shape: an engine-reported prefill crash 38 s after Ready
    // on a start that had taken 31 points back; then the harness's own
    // SIGKILL 34 s into the next start, which had taken back NOTHING and
    // cached for that long. The engine-reported empty STAYS; the SIGKILL
    // empty GOES.
    let w = world();
    let mut s = start(&w);
    s.took_back(31, 4264);
    s.engine_reported_failure(); // the prefill-passes crash
    s.fail_within_probation();

    let mut s = start(&w); // EMPTIES (#1: the engine reported it)
    assert_eq!(w.borrow().empties, 1);
    assert_eq!(w.borrow().bytes_destroyed_mib, 4264);
    s.cached(9, 1160); // the smoke caches 34 s worth
    s.fail_within_probation(); // the smoke's own SIGKILL

    let mut s = start(&w); // the SIGKILL alone: the fresh tier STAYS
    assert_eq!(w.borrow().empties, 1); // not twice
    assert_eq!(w.borrow().bytes_destroyed_mib, 4264);
    assert_eq!(w.borrow().tier.points, 9);
    s.serve_probation_out();
    s.close_cleanly();
}
"#.to_string(),
            context_notes: Some("Grade B+ -- NotBenchable ON THIS BOX (the decision content is a destructive-teardown EVIDENCE policy across process starts -- the win metric is warm-start capacity surviving spurious failures -- which a Rust microbench of the stand-in cannot measure; the source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ 6e21cc90c7848976fe1e34ae4563529c5f345243 'Empty the persistent cache only for a repeated or engine-reported failure', Apache-2.0, Jian Chen, 2026-10-19; every quote re-verified verbatim at the .raw/b270/splash clone this batch at that pin; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (message, verbatim): 'A start that took back a cache after an unclean end served its first minute on probation, and the start after one that failed within it emptied the cache, so that a cached point that breaks the engine cannot break every start. The rule cannot tell such a point from a kill, jetsam or an engine bug of another cause.' MEASURED (message, source-reported, the soak's smoke run on the M5 Max): 'the cache was emptied twice in 44 s: after a start that took back 31 restore points (4,264 MiB) failed 38 s after Ready with the prefill-passes crash, and again after the smoke's own SIGKILL landed 34 s into the next start, which had taken back nothing and cached for that long.' THE LAW (message, verbatim): 'A bad point breaks every start that takes it back, so one failure within probation now keeps the cache: the start that finds the last start's probation mark marks the failure, and the start that finds both marks empties the cache. A start that serves its probation out clears both.' THE MARK (runtime/engine/CacheDirectory.cpp:23-25 @ 6e21cc9, verbatim): 'A start failed within its probation: the next one that does empties the cache.' over 'constexpr char kFailedProbation[] = \"failed-probation\";'. THE BOOT DECISION (CacheDirectory.cpp:191-208, verbatim): 'The last start failed within its probation: the cache goes when the one before it did too, or when its engine reported the failure.' -- both marks found: the cache files are unlinked and the cold reason is 'the last two starts failed within their probation, or the last one's engine did' (:201); one mark: 'result->mark(kFailedProbation);' (the failure is recorded, the cache KEPT). THE ENGINE-REPORTED PATH (CacheDirectory.cpp:276-279, verbatim): 'void CacheDirectory::engineFailed() { if (present(directory_, kProbation)) mark(kFailedProbation); }' -- an exception at the native loop's failure boundary (engine_execution_failed or metal_execution_failed) marks the failure ITSELF, and the next start empties as before. THE HEADER (CacheDirectory.hpp:25-35 @ 6e21cc9, verbatim): 'Every start serves on probation, marked until it has served long enough. A start that finds that mark follows one that failed within its probation and marks the failure; one that finds the failure marked as well empties the cache, as does the start after an engine that reported its own failure within probation (engineFailed). A cached point that breaks the engine breaks every start that takes it back, so the third start in a row begins empty, within the server's limit on failed starts, while one kill or crash of another cause costs nothing.' THE CRASH-LOOP ARITHMETIC (message, verbatim): 'The server stops restarting the engine after three failures in a row, each within 60 s of starting (server/backend.py CRASH_LOOP_LIMIT and CRASH_LOOP_WINDOW_SECONDS). ... With every start on probation the third start of any run of fast failures begins empty, within the server's limit. So the cache now goes after two fast failures in a row, or one fast engine-reported failure, where it went after any failure followed by a fast one.' WHY IT IS A PERF LAW: each emptying destroys the whole warm tier (the smoke lost 4,264 MiB of taken-back points AND a freshly built tier); a persistent cache exists to make restarts warm, so a teardown rule that fires on ONE unattributable failure converts every crash into a cold start -- the policy is the hit rate of the persistent tier across process lifetimes. The instrument class is the source's own: 'one failure within probation keeps the cache, a second in a row empties it, a probation served out ends the run, a failure after probation counts for nothing, and an engine failure within probation empties the cache for the next start while one after probation does not'. REGIME: persistent caches whose warm contents are expensive to rebuild and whose consumers crash for causes the cache cannot attribute (client kills, OOM jetsam, unrelated engine bugs); crash-loop-limited restart supervisors that bound how many fast failures a process may burn. ANTI-REGIMES: caches that are cheap to rebuild (empty eagerly, keep no marks); single-shot processes (no next start to protect); attributed failure channels only (if every failure names its cause, mark on the bad-cause directly). KINSHIP/DISTINCT-FROM (read + disclosed): copy-source-quarantine-unproven-free-pinned (kernel_opt B257) owns free-SAFETY of async device memory (a free needs WAIT PROOF) -- destructive action needs evidence is the shared instinct, but THAT proof is a completion wait within one process; THIS witness is a cross-start attribution chain (marks on 'disk') for a CACHE tier, and the evidence is repetition (two) or an authoritative report (engineFailed) -- distinct axis, kin shape. drain-device-work-before-free (kernel_opt B257) owns the completion axis of frees -- unrelated. teardown-takes-list-whole (kernel_opt B257) owns tearing a list down WHOLE once teardown is warranted -- THIS owns WHEN teardown is warranted at all (evidence), not the sweep's width; compose. slru-probation-protected-promote-on-second-touch (B256, entries_35) is a NAME collision only: its 'probation' is an SLRU segment promoting cache ENTRIES on a second touch; THIS 'probation' is a process-lifetime mark attributing FAILURES across restarts -- different objects, different lifecycle, no compose. empty-slot-first-victim-order-save-before-overwrite (B241) owns victim order within a live cache -- unrelated. seq-fork-carries-pending-rollback-record (B246) -- unrelated. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'crash loop|crash_loop|cold reason|second witness|cache emptied|warm tier' -- zero owners; 'probation' hits only the SLRU rule + its tests (disclosed above). The kernel_opt Batch 268 splash-wave header (entries_gpu_117) stages this lead as 'second-witness teardown' for a rust_perf batch -- this IS that landing. FIXTURE PROVENANCE: original machinery (never splash code) -- the cross-start mark machine (serving/probation[/failed-probation] marks, the boot that reads them, probation as a LOGICAL phase served-out-vs-failed-within, never a clock); the shared-contract tests (a bad point ends the run within the limit, a failure after probation counts for nothing, a clean end leaves the marks clear, an engine-reported failure empties) pass in BOTH arms; the discriminating tests pin the spurious SIGKILL (buggy destroys the 4,264 MiB tier vs fixed keeps it) and the measured smoke (emptied twice in 44 s vs once). Wave: Batch 270 / Plan 274 / Research 306. Cite: incoai/splash @ 6e21cc90c7848976fe1e34ae4563529c5f345243 -- runtime/engine/CacheDirectory.cpp:23-25 + :191-208 + :276-279, runtime/engine/CacheDirectory.hpp:25-35 + :90-98, commit message (all quotes); Batch 270 / Plan 274.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "hot-slot-recycler-flat-tables-zero-alloc-claims".to_string(),
            name: "A hot adaptive-replacement slot recycler (~2,000 claims a layer and step, 24,000 a decode step) keeps its ghosts in node containers (a std::list + an unordered_map -- a node allocated for every slot it gave up and freed for every ghost it dropped) and leaves the key-to-slot map to its store → keep the keys of the slots and of the ghosts in flat tables (open addressing, linear probing over at least twice as many buckets as keys, deletion by backward shift) and the ghosts in lists over arrays as many as the slots, so a claim allocates nothing; only resize() does -- and a shrink keeps the newest ghosts within ARC's bounds at the new size (moving those past it into the free places below) instead of forgetting every ghost".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `hot-slot-recycler-flat-tables-zero-alloc-claims` (riir-refine
// rust_perf corpus, Batch 270 / Plan 274) -- the ANTI-PATTERN arm.
//
// Re-authored shape from incoai/splash runtime/model/SlotRecycler.{hpp,cpp}
// @ a7e5347 + f3eb1a0 (Apache-2.0): an adaptive-replacement (ARC) slot
// recycler whose ghosts live in NODE containers -- a heap Box allocated for
// every slot it gives up and freed for every ghost it drops -- plus a
// growable hash map for key -> slot (the map the store used to keep for
// itself) and a second one for ghost membership. A claim allocates. The
// fixed arm keeps the SAME policy decisions in flat tables (open addressing,
// linear probing, backward-shift deletion) and lists over arrays sized to
// the slot count, so a claim allocates nothing; only resize() does.
//
// ORIGINAL machinery -- never splash code verbatim. Policy (ARC with a +/-
// 1 ghost delta; scan claims leave no ghosts and their hits move nothing;
// a same-load pin is never recycled) is identical in both arms; only the
// storage differs.

use std::collections::HashMap;

pub const FREE: u8 = 0;
pub const RECENT: u8 = 1; // T1
pub const FREQUENT: u8 = 2; // T2

// -- counting allocator (the perf axis: a claim allocates) ------------------
//
// Thread-local with const init: the test harness runs tests on parallel
// threads, and a process-global counter would let a sibling test's
// allocations land in this test's measurement window.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        System.alloc_zeroed(l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        System.realloc(p, l, n)
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

// -- the node-container ARC recycler (the anti-pattern) ----------------------

pub struct GhostNode {
    key: u64,
    frequent: bool,
    next: Option<Box<GhostNode>>,
}

pub struct Recycler {
    slots: usize,
    key_of: Vec<Option<u64>>,
    list: Vec<u8>,
    prev: Vec<u32>,
    next: Vec<u32>,
    head: [u32; 2], // LRU end of T1, T2
    tail: [u32; 2],
    count: [usize; 2],
    last_load: Vec<u64>,
    free: Vec<u32>,
    // the key -> slot map, grown per claim (the store kept its own)
    slot_of: HashMap<u64, u32>,
    // the ghost list: a heap Box per ghost, LRU at head
    ghosts: Option<Box<GhostNode>>,
    ghost_len: usize,
    ghost_of: HashMap<u64, ()>,
    target: i64, // ARC's p: T1's target share
    pub ghost_hits: u64,
}

impl Recycler {
    pub fn new(slots: usize) -> Self {
        Recycler {
            slots,
            key_of: vec![None; slots],
            list: vec![FREE; slots],
            prev: vec![u32::MAX; slots],
            next: vec![u32::MAX; slots],
            head: [u32::MAX; 2],
            tail: [u32::MAX; 2],
            count: [0; 2],
            last_load: vec![0; slots],
            free: (0..slots as u32).collect(),
            slot_of: HashMap::new(),
            ghosts: None,
            ghost_len: 0,
            ghost_of: HashMap::new(),
            target: 0,
            ghost_hits: 0,
        }
    }

    pub fn size(&self) -> usize {
        self.slots
    }

    pub fn ghost_len(&self) -> usize {
        self.ghost_len
    }

    // The slot that holds `key`, if any.
    pub fn find(&self, key: u64) -> Option<u32> {
        self.slot_of.get(&key).copied()
    }

    // Whether load `load` touched or claimed `slot`.
    pub fn pinned(&self, slot: u32, load: u64) -> bool {
        self.last_load[slot as usize] == load
    }

    pub fn touch(&mut self, slot: u32, load: u64, chunk: bool) {
        self.last_load[slot as usize] = load;
        if chunk {
            return; // a scan's hits move nothing
        }
        match self.list[slot as usize] {
            RECENT => {
                self.unlink(slot);
                self.append(slot, FREQUENT);
            }
            FREQUENT => {
                self.unlink(slot);
                self.append(slot, FREQUENT);
            }
            _ => {}
        }
    }

    // The slot load `load`'s missing `key` takes.
    pub fn claim(&mut self, key: u64, load: u64, chunk: bool) -> u32 {
        if let Some(&s) = self.slot_of.get(&key) {
            self.touch(s, load, chunk);
            return s;
        }
        // A ghost's claim adapts the target and drops the ghost, before a
        // slot is recycled. A scan's misses leave no ghost and adapt nothing.
        if !chunk && self.ghost_of.contains_key(&key) {
            let frequent = self.ghost_frequent(key);
            self.target += if frequent { -1 } else { 1 };
            if self.target < 0 {
                self.target = 0;
            }
            self.drop_ghost(key);
            self.ghost_hits += 1;
        }
        let slot = match self.take_free() {
            Some(s) => s,
            None => {
                let from_t1 = self.count[0] as i64 > self.target;
                let list = if from_t1 { RECENT } else { FREQUENT };
                let victim = self.oldest_not_pinned(list, load);
                self.evict(victim, !chunk);
                victim
            }
        };
        self.install(slot, key, load, chunk);
        slot
    }

    // `slot` holds no key (a failed load claimed it).
    pub fn free_slot(&mut self, slot: u32) {
        if self.list[slot as usize] != FREE {
            self.evict(slot, false);
            self.free.push(slot);
        }
    }

    pub fn reset(&mut self) {
        *self = Recycler::new(self.slots);
    }

    // Fewer: the slots past it are gone with their keys and EVERY ghost is
    // forgotten (the a7e5347 first cut, repaired by f3eb1a0). More: free.
    pub fn resize(&mut self, slots: usize) {
        let held = self.slots;
        for slot in slots..held {
            if self.list[slot] != FREE {
                self.evict(slot as u32, false);
            }
        }
        self.forget_ghosts();
        self.key_of.resize(slots, None);
        self.list.resize(slots, FREE);
        self.prev.resize(slots, u32::MAX);
        self.next.resize(slots, u32::MAX);
        self.last_load.resize(slots, 0);
        self.slots = slots;
        if self.target > slots as i64 {
            self.target = slots as i64;
        }
        self.free.clear();
        self.free
            .extend((0..slots).filter(|&s| self.list[s] == FREE).map(|s| s as u32));
    }

    fn forget_ghosts(&mut self) {
        self.ghosts = None;
        self.ghost_of.clear();
        self.ghost_len = 0;
    }

    fn ghost_frequent(&self, key: u64) -> bool {
        let mut cur = self.ghosts.as_deref();
        while let Some(g) = cur {
            if g.key == key {
                return g.frequent;
            }
            cur = g.next.as_deref();
        }
        false
    }

    fn drop_ghost(&mut self, key: u64) {
        // unlink by key in the Box chain (the node-container tax: a walk
        // plus a free for every ghost dropped)
        let mut cur = self.ghosts.as_deref();
        while let Some(g) = cur {
            if g.key == key {
                break;
            }
            cur = g.next.as_deref();
        }
        fn remove(chain: &mut Option<Box<GhostNode>>, key: u64) -> bool {
            if let Some(node) = chain {
                if node.key == key {
                    *chain = node.next.take();
                    return true;
                }
                return remove(&mut node.next, key);
            }
            false
        }
        if remove(&mut self.ghosts, key) {
            self.ghost_of.remove(&key);
            self.ghost_len -= 1;
        }
    }

    fn ghost_add(&mut self, key: u64, frequent: bool) {
        self.ghosts = Some(Box::new(GhostNode {
            key,
            frequent,
            next: self.ghosts.take(),
        }));
        self.ghost_of.insert(key, ());
        self.ghost_len += 1;
        // ARC holds |B1| + |B2| to the slots.
        while self.ghost_len > self.slots {
            let lru_key = self.ghosts.as_ref().unwrap().key;
            self.drop_ghost(lru_key);
        }
    }

    fn take_free(&mut self) -> Option<u32> {
        if self.free.is_empty() {
            return None;
        }
        // lowest free slot, deterministic
        let mut best = 0usize;
        for i in 1..self.free.len() {
            if self.free[i] < self.free[best] {
                best = i;
            }
        }
        Some(self.free.remove(best))
    }

    fn oldest_not_pinned(&self, list: u8, load: u64) -> u32 {
        let idx = if list == RECENT { 0 } else { 1 };
        let mut cur = self.head[idx];
        while cur != u32::MAX {
            if self.last_load[cur as usize] != load {
                return cur;
            }
            cur = self.next[cur as usize];
        }
        panic!("every slot holds a key the load uses");
    }

    fn evict(&mut self, slot: u32, ghost: bool) {
        // a recycled slot goes straight to its next key: it never enters the
        // free list here (only free_slot, the failed-load path, frees)
        let key = self.key_of[slot as usize].take().unwrap();
        let was = self.list[slot as usize];
        self.unlink(slot);
        self.slot_of.remove(&key);
        self.list[slot as usize] = FREE;
        if ghost {
            self.ghost_add(key, was == FREQUENT);
        }
    }

    fn install(&mut self, slot: u32, key: u64, load: u64, chunk: bool) {
        self.key_of[slot as usize] = Some(key);
        self.last_load[slot as usize] = load;
        self.slot_of.insert(key, slot);
        self.list[slot as usize] = RECENT;
        if chunk {
            self.prepend(slot, RECENT); // a scan's misses enter at the LRU end
        } else {
            self.append(slot, RECENT);
        }
    }

    fn unlink(&mut self, slot: u32) {
        let idx = match self.list[slot as usize] {
            RECENT => 0,
            FREQUENT => 1,
            _ => return,
        };
        let s = slot as usize;
        let p = self.prev[s];
        let n = self.next[s];
        if p != u32::MAX {
            self.next[p as usize] = n;
        } else {
            self.head[idx] = n;
        }
        if n != u32::MAX {
            self.prev[n as usize] = p;
        } else {
            self.tail[idx] = p;
        }
        self.count[idx] -= 1;
        self.prev[s] = u32::MAX;
        self.next[s] = u32::MAX;
    }

    fn append(&mut self, slot: u32, list: u8) {
        let idx = if list == RECENT { 0 } else { 1 };
        let s = slot as usize;
        self.list[s] = list;
        self.prev[s] = self.tail[idx];
        self.next[s] = u32::MAX;
        if self.tail[idx] != u32::MAX {
            self.next[self.tail[idx] as usize] = slot;
        } else {
            self.head[idx] = slot;
        }
        self.tail[idx] = slot;
        self.count[idx] += 1;
    }

    fn prepend(&mut self, slot: u32, list: u8) {
        let idx = if list == RECENT { 0 } else { 1 };
        let s = slot as usize;
        self.list[s] = list;
        self.next[s] = self.head[idx];
        self.prev[s] = u32::MAX;
        if self.head[idx] != u32::MAX {
            self.prev[self.head[idx] as usize] = slot;
        } else {
            self.tail[idx] = slot;
        }
        self.head[idx] = slot;
        self.count[idx] += 1;
    }
}

// -- deterministic driver ----------------------------------------------------

pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// The mixed decode-shaped trace: claims over a working set wider than the
/// slots (with scan chunks), touches, finds. Identical in both arms.
pub fn workload(r: &mut Recycler, ops: u64) {
    let mut rng = Lcg(0x5107_B270);
    for i in 0..ops {
        let key = 1 + rng.next() % 96;
        let roll = rng.next() % 10;
        if roll < 6 {
            let chunk = rng.next() % 4 == 0;
            r.claim(key, i + 1, chunk);
        } else if roll < 8 {
            if let Some(slot) = r.find(key) {
                r.touch(slot, i + 1, false);
            }
        } else {
            let _ = r.find(key);
        }
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn the_arc_decision_trace_matches_the_spec() {
    // Hand-computed ARC decisions (slots = 4, +/1 ghost delta): the policy
    // both arms must reproduce. Slots are taken lowest-free-first.
    let mut r = Recycler::new(4);
    assert_eq!(r.claim(1, 1, false), 0);
    assert_eq!(r.claim(2, 2, false), 1);
    assert_eq!(r.claim(3, 3, false), 2);
    r.touch(r.find(1).unwrap(), 4, false); // k1: T1 -> T2
    assert_eq!(r.claim(4, 5, false), 3);
    // k5: no ghost, |T1|=3 > p=0 -> evict LRU(T1) = k2 (ghost B1)
    assert_eq!(r.claim(5, 6, false), 1);
    assert_eq!(r.find(2), None);
    // k2: B1 hit -> p=1, drop ghost; |T1|=3 > 1 -> evict k3 (ghost B1)
    assert_eq!(r.claim(2, 7, false), 2);
    assert_eq!(r.ghost_hits, 1);
    // k3: B1 hit -> p=2; |T1|=3 > 2 -> evict k4 (ghost B1)
    assert_eq!(r.claim(3, 8, false), 3);
    assert_eq!(r.ghost_hits, 2);
    // k4: B1 hit -> p=3; |T1|=3 > 3 false -> evict LRU(T2) = k1 (ghost B2)
    assert_eq!(r.claim(4, 9, false), 0);
    assert_eq!(r.ghost_hits, 3);
    // k1: B2 hit -> p=2; |T1|=4 > 2 -> evict k5 (ghost B1)
    assert_eq!(r.claim(1, 10, false), 1);
    assert_eq!(r.ghost_hits, 4);
    assert_eq!(r.find(1), Some(1));
    assert_eq!(r.find(2), Some(2));
    assert_eq!(r.find(3), Some(3));
    assert_eq!(r.find(4), Some(0));
    assert_eq!(r.find(5), None);
}

#[test]
fn scan_claims_leave_no_ghosts_and_enter_at_the_lru_end() {
    let mut r = Recycler::new(4);
    for k in 1..=4u64 {
        r.claim(k, k, false);
    }
    // four chunk claims sweep the cache: evictions leave NO ghost
    for k in 10..=13u64 {
        r.claim(k, 100, true);
    }
    assert_eq!(r.ghost_len(), 0);
    // the chunk keys entered T1 at the LRU end: each new scan key became
    // the LRU, so the next non-chunk eviction takes the LAST chunk key
    r.claim(20, 101, false);
    assert_eq!(r.find(13), None); // k13 was LRU
    assert_eq!(r.find(10), Some(0));
    assert_eq!(r.find(11), Some(1));
    assert_eq!(r.find(12), Some(2));
    assert_eq!(r.find(20), Some(3));
}

#[test]
fn a_same_load_pin_is_never_recycled() {
    let mut r = Recycler::new(4);
    for k in 1..=4u64 {
        r.claim(k, 1, false);
    }
    // two chunk claims at load 7 evict k1, k2 and sit at the LRU end pinned
    // to load 7
    r.claim(5, 7, true);
    r.claim(6, 7, true);
    assert_eq!(r.find(5), Some(0));
    assert_eq!(r.find(6), Some(1));
    // load 7 claims again: the pinned k5, k6 survive the LRU walk; the
    // unpinned k3 goes instead
    r.claim(7, 7, false);
    assert_eq!(r.find(5), Some(0));
    assert_eq!(r.find(6), Some(1));
    assert_eq!(r.find(3), None);
}

#[test]
fn resize_gives_up_slots_past_the_new_size_and_forgets_ghosts() {
    let mut r = Recycler::new(8);
    for k in 1..=8u64 {
        r.claim(k, k, false);
    }
    r.claim(9, 9, false); // evicts k1 -> ghost B1
    assert_eq!(r.ghost_len(), 1);
    r.resize(4);
    assert_eq!(r.find(5), None); // past the new size: gone
    assert_eq!(r.find(1), None);
    assert_eq!(r.ghost_len(), 0); // every ghost forgotten (the first cut)
    // the ghost is gone: k1's next claim is NOT a ghost hit
    r.claim(1, 50, false);
    assert_eq!(r.ghost_hits, 0);
    assert!(r.find(1).is_some());
}

#[test]
fn the_node_storage_allocates_on_every_ghost_lifecycle_and_claim() {
    let mut r = Recycler::new(64);
    ALLOCS.with(|c| c.set(0));
    workload(&mut r, 3_000);
    let allocs = ALLOCS.with(|c| c.get());
    // a Box per ghost add, a free per drop, map growth on the claims: the
    // node containers allocate throughout the hot path
    assert!(allocs > 100, "allocs={}", allocs);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `hot-slot-recycler-flat-tables-zero-alloc-claims` (riir-refine
// rust_perf corpus, Batch 270 / Plan 274) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/model/SlotRecycler.{hpp,cpp}
// @ a7e5347 + f3eb1a0 (Apache-2.0): the keys of the slots and of the ghosts
// live in FLAT TABLES (open addressing, linear probing over at least twice
// as many buckets as keys, deletion by backward shift) and the ghosts in
// lists over arrays as many as the slots: a claim allocates nothing; only
// resize() does. A shrink keeps the newest ghosts within ARC's bounds at the
// new size (f3eb1a0), moving those past the new size into the free places
// below it -- it does not forget them.
//
// ORIGINAL machinery -- never splash code verbatim. Policy (ARC with a +/-
// 1 ghost delta; scan claims leave no ghosts and their hits move nothing; a
// same-load pin is never recycled) is identical in both arms; only the
// storage differs.

// -- counting allocator (the perf axis: a claim allocates nothing) -----------
//
// Thread-local with const init: the test harness runs tests on parallel
// threads, and a process-global counter would let a sibling test's resize
// allocations land in this test's measurement window.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        System.alloc_zeroed(l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        System.realloc(p, l, n)
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

// -- the flat key index: open addressing, linear probing, backward shift -----

/// A flat map from u64 keys to u32 values (0 = empty): open addressing with
/// linear probing over at least twice as many buckets as it holds keys at
/// most (reserve), deletion by backward shift. Allocated only by `reserve`.
pub struct KeyIndex {
    keys: Vec<u64>,
    vals: Vec<u32>, // 0 = empty, else value (a slot or ghost index + 1)
    shift: u32,
    len: usize,
}

const EMPTY: u32 = 0;

impl KeyIndex {
    pub fn new() -> Self {
        KeyIndex {
            keys: Vec::new(),
            vals: Vec::new(),
            shift: 63,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Holds up to `keys_max` keys from now on, with those it holds.
    pub fn reserve(&mut self, keys_max: usize) {
        let want = if keys_max == 0 {
            16
        } else {
            (keys_max * 2).next_power_of_two().max(16)
        };
        let old_keys = std::mem::take(&mut self.keys);
        let old_vals = std::mem::take(&mut self.vals);
        self.keys = vec![0; want];
        self.vals = vec![EMPTY; want];
        self.shift = 64 - want.trailing_zeros();
        self.len = 0;
        if !old_vals.is_empty() {
            for (i, &v) in old_vals.iter().enumerate() {
                if v != EMPTY {
                    self.insert_raw(old_keys[i], v);
                }
            }
        }
    }

    #[inline]
    fn home(&self, key: u64) -> usize {
        ((key.wrapping_mul(0x9e37_79b9_7f4a_7c15)) >> self.shift) as usize
    }

    #[inline]
    fn step(&self, bucket: usize) -> usize {
        (bucket + 1) & (self.vals.len() - 1)
    }

    pub fn find(&self, key: u64) -> Option<u32> {
        if self.vals.is_empty() {
            return None;
        }
        let mut b = self.home(key);
        loop {
            let v = self.vals[b];
            if v == EMPTY {
                return None;
            }
            if self.keys[b] == key {
                return Some(v);
            }
            b = self.step(b);
        }
    }

    /// `key` is not held.
    pub fn insert(&mut self, key: u64, value: u32) {
        self.insert_raw(key, value);
    }

    fn insert_raw(&mut self, key: u64, value: u32) {
        let mut b = self.home(key);
        loop {
            if self.vals[b] == EMPTY {
                self.keys[b] = key;
                self.vals[b] = value;
                self.len += 1;
                return;
            }
            if self.keys[b] == key {
                self.vals[b] = value;
                return;
            }
            b = self.step(b);
        }
    }

    pub fn erase(&mut self, key: u64) {
        if self.vals.is_empty() {
            return;
        }
        let mut b = self.home(key);
        loop {
            let v = self.vals[b];
            if v == EMPTY {
                return;
            }
            if self.keys[b] == key {
                self.remove_at(b);
                return;
            }
            b = self.step(b);
        }
    }

    // Deletion by backward shift: the cluster stays probe-closed.
    fn remove_at(&mut self, mut hole: usize) {
        let n = self.vals.len();
        let mut j = self.step(hole);
        while self.vals[j] != EMPTY {
            let home = self.home(self.keys[j]);
            // the key at j moves into the hole iff its home reaches the hole
            // no later than it reaches j, on the probe ring
            let d_hole = (hole + n - home) % n;
            let d_j = (j + n - home) % n;
            if d_hole <= d_j {
                self.keys[hole] = self.keys[j];
                self.vals[hole] = self.vals[j];
                hole = j;
            }
            j = self.step(j);
        }
        self.vals[hole] = EMPTY;
        self.len -= 1;
    }

    pub fn clear(&mut self) {
        for v in self.vals.iter_mut() {
            *v = EMPTY;
        }
        self.len = 0;
    }
}

fn cyclic_in(x: usize, lo: usize, hi: usize) -> bool {
    // is x in the cyclic (exclusive lo, inclusive hi] over the ring?
    if lo < hi {
        x > lo && x <= hi
    } else {
        x > lo || x <= hi
    }
}

// -- the flat ARC recycler ----------------------------------------------------

pub const FREE: u8 = 0;
pub const RECENT: u8 = 1; // T1
pub const FREQUENT: u8 = 2; // T2

pub struct Recycler {
    slots: usize,
    key_of: Vec<u64>,
    list: Vec<u8>,
    prev: Vec<u32>,
    next: Vec<u32>,
    head: [u32; 2],
    tail: [u32; 2],
    count: [usize; 2],
    last_load: Vec<u64>,
    free: Vec<u32>,
    slot_of: KeyIndex, // key -> slot + 1
    // ghosts in lists over arrays as many as the slots
    ghost_key: Vec<u64>,
    ghost_frequent: Vec<bool>,
    gprev: Vec<u32>,
    gnext: Vec<u32>,
    ghead: [u32; 2],
    gtail: [u32; 2],
    gcount: [usize; 2],
    ghost_of: KeyIndex, // key -> ghost index + 1
    free_ghosts: Vec<u32>,
    target: i64,
    pub ghost_hits: u64,
}

impl Recycler {
    pub fn new(slots: usize) -> Self {
        let mut r = Recycler {
            slots,
            key_of: vec![0; slots],
            list: vec![FREE; slots],
            prev: vec![u32::MAX; slots],
            next: vec![u32::MAX; slots],
            head: [u32::MAX; 2],
            tail: [u32::MAX; 2],
            count: [0; 2],
            last_load: vec![0; slots],
            free: Vec::with_capacity(slots),
            slot_of: KeyIndex::new(),
            ghost_key: vec![0; slots],
            ghost_frequent: vec![false; slots],
            gprev: vec![u32::MAX; slots],
            gnext: vec![u32::MAX; slots],
            ghead: [u32::MAX; 2],
            gtail: [u32::MAX; 2],
            gcount: [0; 2],
            ghost_of: KeyIndex::new(),
            free_ghosts: Vec::with_capacity(slots),
            target: 0,
            ghost_hits: 0,
        };
        r.free.extend(0..slots as u32);
        r.free_ghosts.extend(0..slots as u32);
        r.slot_of.reserve(slots);
        r.ghost_of.reserve(slots);
        r
    }

    pub fn size(&self) -> usize {
        self.slots
    }

    pub fn ghost_len(&self) -> usize {
        self.gcount[0] + self.gcount[1]
    }

    pub fn find(&self, key: u64) -> Option<u32> {
        self.slot_of.find(key).map(|v| v - 1)
    }

    pub fn pinned(&self, slot: u32, load: u64) -> bool {
        self.last_load[slot as usize] == load
    }

    pub fn touch(&mut self, slot: u32, load: u64, chunk: bool) {
        self.last_load[slot as usize] = load;
        if chunk {
            return; // a scan's hits move nothing
        }
        match self.list[slot as usize] {
            RECENT => {
                self.unlink(slot);
                self.append(slot, FREQUENT);
            }
            FREQUENT => {
                self.unlink(slot);
                self.append(slot, FREQUENT);
            }
            _ => {}
        }
    }

    pub fn claim(&mut self, key: u64, load: u64, chunk: bool) -> u32 {
        if let Some(s) = self.find(key) {
            self.touch(s, load, chunk);
            return s;
        }
        // A ghost's claim adapts the target and drops the ghost, before a
        // slot is recycled. A scan's misses leave no ghost and adapt nothing.
        if !chunk {
            if let Some(g) = self.ghost_of.find(key) {
                let frequent = self.ghost_frequent[(g - 1) as usize];
                self.target += if frequent { -1 } else { 1 };
                if self.target < 0 {
                    self.target = 0;
                }
                self.drop_ghost(g - 1);
                self.ghost_hits += 1;
            }
        }
        let slot = match self.take_free() {
            Some(s) => s,
            None => {
                let from_t1 = self.count[0] as i64 > self.target;
                let list = if from_t1 { RECENT } else { FREQUENT };
                let victim = self.oldest_not_pinned(list, load);
                self.evict(victim, !chunk);
                victim
            }
        };
        self.install(slot, key, load, chunk);
        slot
    }

    // `slot` holds no key (a failed load claimed it).
    pub fn free_slot(&mut self, slot: u32) {
        if self.list[slot as usize] != FREE {
            self.evict(slot, false);
            self.free.push(slot);
        }
    }

    pub fn reset(&mut self) {
        *self = Recycler::new(self.slots);
    }

    // `slots` slots: those added are free; those past it are gone with their
    // keys; a shrink KEEPS the newest ghosts within ARC's bounds at the new
    // size and moves those past it into the free places below it (f3eb1a0).
    pub fn resize(&mut self, slots: usize) {
        let held = self.slots;
        let fewer = slots < held;
        if fewer {
            for slot in slots..held {
                if self.list[slot] != FREE {
                    self.evict(slot as u32, false);
                }
            }
            // ARC's bound at the new size: T1's ghosts while T1 and its
            // ghosts fit the slots, then either list up to the slots.
            while self.gcount[0] > 0 && self.count[0] + self.gcount[0] > slots {
                self.drop_ghost(self.ghead[0]);
            }
            while self.gcount[0] + self.gcount[1] > slots {
                let victim = if self.gcount[1] > 0 {
                    self.ghead[1]
                } else {
                    self.ghead[0]
                };
                self.drop_ghost(victim);
            }
            self.target = self.target.min(slots as i64);
            // As many places below `slots` are free as ghosts lie past it:
            // move each ghost at or past the new size into a free place.
            // (Runs while the ghost arrays are still `held`-sized.)
            self.free_ghosts.retain(|&g| (g as usize) < slots);
            let mut displaced: Vec<(u64, u8, bool)> = Vec::new(); // resize-only allocation
            for list in [0usize, 1] {
                let mut cur = self.ghead[list];
                while cur != u32::MAX {
                    let n = self.gnext[cur as usize];
                    if cur as usize >= slots {
                        let key = self.ghost_key[cur as usize];
                        let frequent = self.ghost_frequent[cur as usize];
                        self.gunlink(cur);
                        self.ghost_of.erase(key);
                        displaced.push((key, list as u8, frequent));
                    }
                    cur = n;
                }
            }
            for (key, list, frequent) in displaced {
                let place = self.free_ghosts.pop().expect("a ghost place below the new size");
                self.ghost_key[place as usize] = key;
                self.ghost_frequent[place as usize] = frequent;
                self.gappend(place, if list == FREQUENT { FREQUENT } else { RECENT });
                self.ghost_of.insert(key, place + 1);
            }
        }
        // the arrays shrink (or grow) only now
        self.key_of.resize(slots, 0);
        self.list.resize(slots, FREE);
        self.prev.resize(slots, u32::MAX);
        self.next.resize(slots, u32::MAX);
        self.last_load.resize(slots, 0);
        self.ghost_key.resize(slots, 0);
        self.ghost_frequent.resize(slots, false);
        self.gprev.resize(slots, u32::MAX);
        self.gnext.resize(slots, u32::MAX);
        self.slots = slots;
        self.slot_of.reserve(slots);
        self.ghost_of.reserve(slots);
        // rebuild the free places over the surviving slots
        self.free.clear();
        self.free
            .extend((0..slots).filter(|&s| self.list[s] == FREE).map(|s| s as u32));
        if slots > held {
            self.free_ghosts.extend(held as u32..slots as u32);
        }
    }

    fn drop_ghost(&mut self, ghost: u32) {
        let key = self.ghost_key[ghost as usize];
        self.gunlink(ghost);
        self.ghost_of.erase(key);
        self.ghost_frequent[ghost as usize] = false;
        self.free_ghosts.push(ghost);
    }

    fn gunlink(&mut self, ghost: u32) {
        let idx = match self.ghost_list_of(ghost) {
            Some(i) => i,
            None => return,
        };
        let g = ghost as usize;
        let p = self.gprev[g];
        let n = self.gnext[g];
        if p != u32::MAX {
            self.gnext[p as usize] = n;
        } else {
            self.ghead[idx] = n;
        }
        if n != u32::MAX {
            self.gprev[n as usize] = p;
        } else {
            self.gtail[idx] = p;
        }
        self.gcount[idx] -= 1;
        self.gprev[g] = u32::MAX;
        self.gnext[g] = u32::MAX;
    }

    fn ghost_list_of(&self, ghost: u32) -> Option<usize> {
        for (i, &h) in self.ghead.iter().enumerate() {
            let mut cur = h;
            while cur != u32::MAX {
                if cur == ghost {
                    return Some(i);
                }
                cur = self.gnext[cur as usize];
            }
        }
        None
    }

    fn gappend(&mut self, ghost: u32, list: u8) {
        let idx = if list == RECENT { 0 } else { 1 };
        let g = ghost as usize;
        self.gprev[g] = self.gtail[idx];
        self.gnext[g] = u32::MAX;
        if self.gtail[idx] != u32::MAX {
            self.gnext[self.gtail[idx] as usize] = ghost;
        } else {
            self.ghead[idx] = ghost;
        }
        self.gtail[idx] = ghost;
        self.gcount[idx] += 1;
    }

    fn ghost_add(&mut self, key: u64, frequent: bool) {
        // As many ghosts as slots: more would need a free place that is not
        // there -- drop the LRU first (ARC holds |B1| + |B2| to the slots).
        while self.gcount[0] + self.gcount[1] >= self.slots {
            let victim = if self.gcount[0] > 0 {
                self.ghead[0]
            } else {
                self.ghead[1]
            };
            self.drop_ghost(victim);
        }
        let ghost = self.free_ghosts.pop().unwrap();
        self.ghost_key[ghost as usize] = key;
        self.ghost_frequent[ghost as usize] = frequent;
        self.gappend(ghost, if frequent { FREQUENT } else { RECENT });
        self.ghost_of.insert(key, ghost + 1);
    }

    fn take_free(&mut self) -> Option<u32> {
        if self.free.is_empty() {
            return None;
        }
        let mut best = 0usize;
        for i in 1..self.free.len() {
            if self.free[i] < self.free[best] {
                best = i;
            }
        }
        Some(self.free.swap_remove(best))
    }

    fn oldest_not_pinned(&self, list: u8, load: u64) -> u32 {
        let idx = if list == RECENT { 0 } else { 1 };
        let mut cur = self.head[idx];
        while cur != u32::MAX {
            if self.last_load[cur as usize] != load {
                return cur;
            }
            cur = self.next[cur as usize];
        }
        panic!("every slot holds a key the load uses");
    }

    fn evict(&mut self, slot: u32, ghost: bool) {
        // a recycled slot goes straight to its next key: it never enters the
        // free list here (only free_slot, the failed-load path, frees)
        let key = self.key_of[slot as usize];
        let was = self.list[slot as usize];
        self.unlink(slot);
        self.slot_of.erase(key);
        self.list[slot as usize] = FREE;
        if ghost {
            self.ghost_add(key, was == FREQUENT);
        }
    }

    fn install(&mut self, slot: u32, key: u64, load: u64, chunk: bool) {
        self.key_of[slot as usize] = key;
        self.last_load[slot as usize] = load;
        self.slot_of.insert(key, slot + 1);
        self.list[slot as usize] = RECENT;
        if chunk {
            self.prepend(slot, RECENT); // a scan's misses enter at the LRU end
        } else {
            self.append(slot, RECENT);
        }
    }

    fn unlink(&mut self, slot: u32) {
        let idx = match self.list[slot as usize] {
            RECENT => 0,
            FREQUENT => 1,
            _ => return,
        };
        let s = slot as usize;
        let p = self.prev[s];
        let n = self.next[s];
        if p != u32::MAX {
            self.next[p as usize] = n;
        } else {
            self.head[idx] = n;
        }
        if n != u32::MAX {
            self.prev[n as usize] = p;
        } else {
            self.tail[idx] = p;
        }
        self.count[idx] -= 1;
        self.prev[s] = u32::MAX;
        self.next[s] = u32::MAX;
    }

    fn append(&mut self, slot: u32, list: u8) {
        let idx = if list == RECENT { 0 } else { 1 };
        let s = slot as usize;
        self.list[s] = list;
        self.prev[s] = self.tail[idx];
        self.next[s] = u32::MAX;
        if self.tail[idx] != u32::MAX {
            self.next[self.tail[idx] as usize] = slot;
        } else {
            self.head[idx] = slot;
        }
        self.tail[idx] = slot;
        self.count[idx] += 1;
    }

    fn prepend(&mut self, slot: u32, list: u8) {
        let idx = if list == RECENT { 0 } else { 1 };
        let s = slot as usize;
        self.list[s] = list;
        self.next[s] = self.head[idx];
        self.prev[s] = u32::MAX;
        if self.head[idx] != u32::MAX {
            self.prev[self.head[idx] as usize] = slot;
        } else {
            self.tail[idx] = slot;
        }
        self.head[idx] = slot;
        self.count[idx] += 1;
    }
}

// -- deterministic driver (identical in both arms) ---------------------------

pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

pub fn workload(r: &mut Recycler, ops: u64) {
    let mut rng = Lcg(0x5107_B270);
    for i in 0..ops {
        let key = 1 + rng.next() % 96;
        let roll = rng.next() % 10;
        if roll < 6 {
            let chunk = rng.next() % 4 == 0;
            r.claim(key, i + 1, chunk);
        } else if roll < 8 {
            if let Some(slot) = r.find(key) {
                r.touch(slot, i + 1, false);
            }
        } else {
            let _ = r.find(key);
        }
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn the_arc_decision_trace_matches_the_spec() {
    // Hand-computed ARC decisions (slots = 4, +/-1 ghost delta): the policy
    // both arms must reproduce. Slots are taken lowest-free-first.
    let mut r = Recycler::new(4);
    assert_eq!(r.claim(1, 1, false), 0);
    assert_eq!(r.claim(2, 2, false), 1);
    assert_eq!(r.claim(3, 3, false), 2);
    r.touch(r.find(1).unwrap(), 4, false); // k1: T1 -> T2
    assert_eq!(r.claim(4, 5, false), 3);
    // k5: no ghost, |T1|=3 > p=0 -> evict LRU(T1) = k2 (ghost B1)
    assert_eq!(r.claim(5, 6, false), 1);
    assert_eq!(r.find(2), None);
    // k2: B1 hit -> p=1, drop ghost; |T1|=3 > 1 -> evict k3 (ghost B1)
    assert_eq!(r.claim(2, 7, false), 2);
    assert_eq!(r.ghost_hits, 1);
    // k3: B1 hit -> p=2; |T1|=3 > 2 -> evict k4 (ghost B1)
    assert_eq!(r.claim(3, 8, false), 3);
    assert_eq!(r.ghost_hits, 2);
    // k4: B1 hit -> p=3; |T1|=3 > 3 false -> evict LRU(T2) = k1 (ghost B2)
    assert_eq!(r.claim(4, 9, false), 0);
    assert_eq!(r.ghost_hits, 3);
    // k1: B2 hit -> p=2; |T1|=4 > 2 -> evict k5 (ghost B1)
    assert_eq!(r.claim(1, 10, false), 1);
    assert_eq!(r.ghost_hits, 4);
    assert_eq!(r.find(1), Some(1));
    assert_eq!(r.find(2), Some(2));
    assert_eq!(r.find(3), Some(3));
    assert_eq!(r.find(4), Some(0));
    assert_eq!(r.find(5), None);
}

#[test]
fn scan_claims_leave_no_ghosts_and_enter_at_the_lru_end() {
    let mut r = Recycler::new(4);
    for k in 1..=4u64 {
        r.claim(k, k, false);
    }
    // four chunk claims sweep the cache: evictions leave NO ghost
    for k in 10..=13u64 {
        r.claim(k, 100, true);
    }
    assert_eq!(r.ghost_len(), 0);
    // the chunk keys entered T1 at the LRU end: each new scan key became
    // the LRU, so the next non-chunk eviction takes the LAST chunk key
    r.claim(20, 101, false);
    assert_eq!(r.find(13), None); // k13 was LRU
    assert_eq!(r.find(10), Some(0));
    assert_eq!(r.find(11), Some(1));
    assert_eq!(r.find(12), Some(2));
    assert_eq!(r.find(20), Some(3));
}

#[test]
fn a_same_load_pin_is_never_recycled() {
    let mut r = Recycler::new(4);
    for k in 1..=4u64 {
        r.claim(k, 1, false);
    }
    // two chunk claims at load 7 evict k1, k2 and sit at the LRU end pinned
    // to load 7
    r.claim(5, 7, true);
    r.claim(6, 7, true);
    assert_eq!(r.find(5), Some(0));
    assert_eq!(r.find(6), Some(1));
    // load 7 claims again: the pinned k5, k6 survive the LRU walk; the
    // unpinned k3 goes instead
    r.claim(7, 7, false);
    assert_eq!(r.find(5), Some(0));
    assert_eq!(r.find(6), Some(1));
    assert_eq!(r.find(3), None);
}

#[test]
fn the_key_index_survives_backward_shift_deletion() {
    let mut ix = KeyIndex::new();
    ix.reserve(64);
    for k in 0..50u64 {
        ix.insert(k * 7 + 1, (k + 10) as u32);
    }
    assert_eq!(ix.len(), 50);
    // erase a cluster-straddling sample, then re-find everything
    for k in [3u64, 4, 17, 18, 19, 31, 49] {
        ix.erase(k * 7 + 1);
    }
    assert_eq!(ix.len(), 43);
    for k in 0..50u64 {
        let hit = ix.find(k * 7 + 1);
        if [3u64, 4, 17, 18, 19, 31, 49].contains(&k) {
            assert_eq!(hit, None);
        } else {
            assert_eq!(hit, Some((k + 10) as u32));
        }
    }
}

#[test]
fn a_shrink_keeps_the_newest_ghosts_within_arcs_bounds() {
    let mut r = Recycler::new(8);
    for k in 1..=4u64 {
        r.claim(k, k, false);
    }
    r.touch(r.find(1).unwrap(), 10, false); // k1 -> T2 (low slots)
    r.touch(r.find(2).unwrap(), 11, false); // k2 -> T2
    for k in 5..=8u64 {
        r.claim(k, k, false); // T1 fills the high slots
    }
    r.claim(9, 9, false); // evicts k3 -> ghost B1; T1 = k4, k5..k9
    assert_eq!(r.ghost_len(), 1);
    // the shrink evicts the T1 members on the high slots; T1 = 2, so
    // |T1| + |B1| = 3 <= 4: the ghost stays within ARC's bound at the new
    // size (and at a ghost place past the new size, it moves below it)
    r.resize(4);
    assert_eq!(r.find(5), None); // past the new size: gone
    assert_eq!(r.find(8), None);
    assert_eq!(r.find(3), None); // its SLOT is gone...
    // ...but the ghost survived the shrink: k3's claim is still a ghost hit
    r.claim(3, 50, false);
    assert_eq!(r.ghost_hits, 1);
    assert!(r.find(3).is_some());
}

#[test]
fn the_flat_storage_allocates_nothing_over_the_workload() {
    let mut r = Recycler::new(64);
    ALLOCS.with(|c| c.set(0));
    workload(&mut r, 3_000);
    let allocs = ALLOCS.with(|c| c.get());
    // a claim allocates nothing; only resize() does
    assert_eq!(allocs, 0, "allocs={}", allocs);
}
"#.to_string(),
            context_notes: Some("Grade A- -- NotBenchable ON THIS BOX (the decision content is the zero-alloc CLAIM STRUCTURE of a hot-path recycler -- 13 ns/entry source-reported at serving scale -- and the fixture's counting-allocator test proves the SHAPE locally, while the source-reported figure is PROVENANCE, never a bar; a local ns A/B of the stand-in would measure the fixture, not splash's 93M-entry replay). SOURCE (incoai/splash @ a7e53472cb091291a72a181640076b0b271b7bbd 'Keep slot recycler keys and ghosts in flat tables; let it find, resize and count' + f3eb1a036734586851ba244e489547b1c6262a96 'Keep the ghosts a shrinking slot recycler still has room for', Apache-2.0, Jian Chen, 2026-10-17/18; every quote re-verified verbatim at the .raw/b270/splash clone this batch at those pins; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (a7e5347 message, verbatim): 'ExpertSlots kept its ghosts in a std::list and an unordered_map, a node allocated for every slot it gave up and freed for every ghost it dropped, and left the map from a key to its slot to its store.' THE SCALE (message, verbatim): the hot block cache of spilled K/V recycles 'each attention layer's slots ... by adaptive replacement (SlotRecycler), at about 2,000 claims a layer and step: 24,000 a decode step of Qwen3.8-Flash-Next's 12 attention layers.' THE LAW (a7e5347 message, verbatim): 'SlotRecycler now keeps the keys of its slots and of its ghosts in flat tables (open addressing, linear probing over at least twice as many buckets as keys, deletion by backward shift) and its ghosts in lists over arrays as many as the slots, so a claim allocates nothing; only resize() does.' The header carries it in-source (runtime/model/SlotRecycler.hpp:24-27 @ a7e5347, verbatim): 'The keys of the slots and of the ghosts are in flat tables, and the ghosts in lists over arrays as many as the slots: a claim allocates nothing, and only resize() does.' THE FLAT MAP (SlotRecycler.hpp:68-71, verbatim): 'A flat map from keys to values below UINT32_MAX: open addressing with linear probing over at least twice as many buckets as it holds keys at most (reserve), deletion by backward shift.' THE API GAINS the rewrite bought (message, verbatim): 'find(key): the slot that holds a key, so a store keeps no map of its own; pinned(slot, load): whether a load touched or claimed a slot, which it then never recycles; resize(slots): more slots are free; fewer give up the keys of the slots past them and forget every ghost (an idle reclaim shrinks the hot block cache); ghostHits(): the claims, not a scan's, of a key T1 or T2 gave up lately.' MEASURED (a7e5347 message, source-reported): 'Replaying the decode traces of the next commit as a resolve does (a touch for an entry it holds, a claim for one it misses, 93M entries), an entry takes 13 ns on an M5 Pro, against 22 ns with ExpertSlots and an unordered_map from keys to slots.' (the staged '13 ns/entry vs 22'). THE DECISIONS ARE PINNED (message, verbatim): 'the decisions are the ones it made (replayed on the decode traces of the next commit, every request's misses are those of ExpertSlots)' -- the storage rewrite is policy-IDENTICAL, which is exactly what the fixture mirrors (both arms reproduce the same hand-computed ARC trace). THE POLICY BEING SERVED (SlotRecycler.hpp:17-25 @ a7e5347, verbatim): 'Adaptive replacement (ARC, Megiddo and Modha, FAST 2003): the slots of keys used once lately (T1) and of those used again since (T2), each recycled least recently used first, and the keys each recently gave up (ghosts, B1 and B2, as many as the slots). A miss of a key T1 gave up lately grows T1's target share, one T2 gave up shrinks it' -- and the scan law: 'A long chunk's routes are a scan: its misses enter T1 at its least recently used end and leave no ghost, and its hits move nothing, so a prompt does not flush what decode steps route.' THE SHRINK FACE (f3eb1a0 message, verbatim): 'SlotRecycler::resize() to fewer slots forgot every ghost (ARC's B1 and B2), so each shrink of the hot block cache lost what adapts its replacement, the record of the blocks it gave up lately. A shrink now keeps the newest ghosts within ARC's bounds at the new size ... and moves those past the new size into the free places below it; ExpertStore never shrinks its arenas.' (keepGhosts, SlotRecycler.cpp:129 @ f3eb1a0; the test 'the ghost T1 gave up before a shrink still counts as a ghost's claim after it'). THE UPSTREAM TEST NAMES THE LAW (a7e5347 message, verbatim): 'allocates nothing over 300 loads of a working set, others in turn, scans and frees (a counting operator new)'. REGIME: hot-path membership+order structures recycled per request/step (slot recyclers, connection/arena recyclers, admission caches) where the hot ops are claim/touch/find and the working set is bounded by the slot count -- flat open-addressing tables sized at reserve + intrusive array-linked lists make the hot path allocation-free; node containers (std::list, unordered_map, Box chains) allocate per transition. ANTI-REGIMES: cold paths (resize/setup -- the allocation belongs there by design); unbounded key populations (a flat table needs a bounded key universe -- 'as many as the slots'); single-threaded rarely-called structures; keys needing stable node addresses (intrusive-element invalidation on rehash -- the source's index-based lists exist precisely for this). KINSHIP/DISTINCT-FROM (read + disclosed): flat-pool-start-len-static-rows (B248, entries_33-family) owns the LAYOUT of a static row pool (start/len rows over one flat buffer) -- THIS owns the dynamic membership+order STRUCTURE of a recycler (open addressing with backward-shift deletion + intrusive lists) and its zero-alloc CLAIM invariant; kin storage philosophy, distinct subject. minimal-perfect-hash-static-lookup (B248) owns the STATIC build-once key universe -- THIS is the DYNAMIC boundary (insert/erase/resize with backward-shift deletion); compose at the static/dynamic boundary. empty-slot-first-victim-order-save-before-overwrite (B241) owns victim ORDER within one list + overwrite persistence -- THIS keeps the ARC policy decisions bit-identical and changes only the storage; compose. slru-probation-protected-promote-on-second-touch (B256, entries_35) owns the eviction POLICY (two-segment promotion); THIS owns the data structure executing an ARC policy -- the source's own 'the decisions are the ones it made' pins the split; compose. pinned-entry-tombstone-freed-at-last-unpin (B256) owns pinned LIFETIMES -- THIS 'pinned(slot, load)' is a same-load guard, not a cross-async holder; distinct. persistent-digest-memo-stat-identity-keyed (B256) owns an on-disk digest memo -- unrelated. autotuner-key-memoized-per-launch-not-rebuilt (kernel_opt B265) -- memoization per launch key, unrelated. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'slot recycler' -- the only hit is the kernel_opt Batch 268 splash-wave header staging THIS lead ('flat-table slot recycler') for a rust_perf batch -- this IS that landing; 'linear probing|backward shift|open addressing|ghost list' -- zero owners. FIXTURE PROVENANCE: original machinery (never splash code) -- an ARC slot recycler with a +/-1 ghost delta, lowest-free-first slots, scan claims that leave no ghosts and enter at the LRU end, and same-load pins; the buggy arm is the node-container shape (Box-linked ghost list + HashMap key index + HashMap ghost membership, a Box allocated per ghost add and freed per drop); the fixed arm is the flat shape (KeyIndex: open addressing, linear probing over 2x buckets, deletion by backward shift, hash-displaced multiplicative home; ghosts in arrays sized to the slots with index-linked lists; resize-only allocation, including f3eb1a0's keepGhosts remap). Both arms reproduce the same hand-computed 10-step ARC decision trace (slot ids, ghost hits, target) -- the policy-identity premise -- plus scan/pin/resize behaviors; the alloc law is a thread-local counting GlobalAlloc over a 3,000-op mixed workload: buggy > 100 allocations, fixed == 0. Wave: Batch 270 / Plan 274 / Research 306. Cite: incoai/splash @ a7e53472cb091291a72a181640076b0b271b7bbd + f3eb1a036734586851ba244e489547b1c6262a96 -- runtime/model/SlotRecycler.hpp:24-27 + :44 + :49 + :68-71 (@ a7e5347), runtime/model/SlotRecycler.cpp:129 (@ f3eb1a0 keepGhosts), commit messages (all quotes); Batch 270 / Plan 274.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
