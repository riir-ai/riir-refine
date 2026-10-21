//! Entries slice D - Batch 271 (2026-10-21, Plan 275 / Research 307): the
//! splash-wave second-tier serving pool, slice D (1 concurrency + 2
//! complexity rules) - incoai/splash @ 91d4ddb8d8125eb89aac12762b2c8bcf727fd7e3 +
//! ed1092f8631e579e09906805b9dd4544ebf327a4 +
//! 2f5fe8dd3d4cbd986812beee4652f4d4e0f2d765, Apache-2.0; every quote
//! re-verified at the .raw/b271/splash clone this batch. All three rules are
//! corpus-only `NotBenchable` (decision-content serving laws; the grades and
//! the source-reported figures ride each `context_notes`) - no proposer
//! files. Fixtures are ORIGINAL machinery (never splash code verbatim), each
//! compiled and executed standalone (rustc --edition 2021 --test) before
//! this table was assembled; the buggy/fixed arm pairs share contract tests
//! and differ only on the law.
use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "finished-command-copy-leaves-at-once-slot-returns-at-last-reader".to_string(),
            name: "A FINISHED operation's lingering read handle must not block eviction or quota return -- the entry leaves the index at once, the subordinate resource (the disk slot) is tied to the handle and returns at the LAST drop, and the no-lingering-reader invariant is asserted only in the QUIESCENT window between commands: the intermediate state (a finished command's copy still named while the block is gone) is legal, never a fatal throw -- an eager use_count check at erase (or at the quota's give-up-copy) turns a legal intermediate into a crash that restarts the engine or aborts the process"
                .to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `finished-command-copy-leaves-at-once-slot-returns-at-last-reader`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN arm.
//
// Re-authored shape from incoai/splash runtime/engine/{KvCache,KvCache}.{cpp,hpp}
// @ 91d4ddb8 (Apache-2.0): a block index whose entries carry disk-copy slots,
// reader handles a FINISHED command may still hold, and a unit quota. The
// anti-pattern: erase (and the quota's give-up-copy path) THROWS while any
// reader outside the cache still holds the copy -- a legal intermediate (the
// command finished; the engine applies its results while the copy lives)
// crashes the caller (endRequest / the reclaim loop) instead of completing.
//
// ORIGINAL machinery -- never splash code verbatim. A slot's `readers` count
// models the shared_ptr use_count (the cache's own registration included):
// 1 = only the cache; >1 = a request's (or a finished command's) copy lives.

use std::collections::HashMap;

pub const EAGER_ERASE_THROW: &str =
    "a KV cache block left while a reader still holds its copy";
pub const EAGER_GIVE_THROW: &str =
    "a KV cache block gave up a copy that a reader still holds";

pub struct Slot {
    pub readers: usize, // the cache's own registration + every live copy handle
    pub detached: bool, // the block left; the slot rides its readers now
    pub returned: bool, // back to the file (reusable)
}

#[derive(Clone, Copy)]
pub struct Handle {
    slot: usize,
    open: bool,
}

#[derive(Clone, Copy)]
pub struct Block {
    pub slot: Option<usize>,
    pub held: bool,     // a request holds the block
    pub transfer: bool, // a transfer moves it
}

pub struct Cache {
    pub blocks: HashMap<u64, Block>,
    pub slots: Vec<Slot>,
    pub free: Vec<usize>,  // returned slot ids, reusable
    pub used: usize,       // quota units: blocks carrying a slot
    pub quota: usize,
    pub registrations: usize, // slots the cache itself registers
    pub throws: usize,        // the eager-throw counter (the defect's face)
    pub returns: Vec<usize>,  // event log: slots returned at last drop
}

impl Cache {
    pub fn new(quota: usize, file_slots: usize) -> Self {
        Cache {
            blocks: HashMap::new(),
            slots: (0..file_slots)
                .map(|_| Slot { readers: 0, detached: false, returned: false })
                .collect(),
            free: Vec::new(),
            used: 0,
            quota,
            registrations: 0,
            throws: 0,
            returns: Vec::new(),
        }
    }

    // A block lands with a disk copy (the spill): one quota unit, the cache's
    // own registration on the slot.
    pub fn insert_with_copy(&mut self, id: u64) {
        let s = match self.free.pop() {
            Some(s) => s,
            None => self.slots.iter().position(|s| s.readers == 0 && !s.returned).expect("no file slot"),
        };
        self.slots[s] = Slot { readers: 1, detached: false, returned: false };
        self.blocks.insert(id, Block { slot: Some(s), held: false, transfer: false });
        self.used += 1;
        self.registrations += 1;
    }

    // A request reads the spilled K/V: it takes a copy (KvTier::copy).
    pub fn open_copy(&mut self, id: u64) -> Option<Handle> {
        let s = self.blocks.get(&id)?.slot?;
        self.slots[s].readers += 1;
        Some(Handle { slot: s, open: true })
    }

    // A reader's drop: the LAST drop returns a detached slot to the file.
    pub fn close_copy(&mut self, h: Handle) {
        if !h.open {
            return;
        }
        let s = h.slot;
        self.slots[s].readers -= 1;
        if self.slots[s].detached && self.slots[s].readers == 0 {
            self.return_slot(s);
        }
    }

    fn return_slot(&mut self, s: usize) {
        self.slots[s].returned = true;
        self.free.push(s);
        self.returns.push(s);
    }

    // THE ANTI-PATTERN: erase throws eagerly while a reader holds the copy --
    // the block never leaves, the quota unit never returns, and the caller
    // (a request end, a reclaim step) crashes on a legal intermediate.
    pub fn erase(&mut self, id: u64) -> Result<(), String> {
        let b = *self.blocks.get(&id).ok_or("no such block")?;
        if let Some(s) = b.slot {
            if self.slots[s].readers != 1 {
                self.throws += 1;
                return Err(EAGER_ERASE_THROW.to_string());
            }
        }
        self.blocks.remove(&id);
        self.take_slot(b);
        Ok(())
    }

    // The quota's give-up-copy path: same eager throw, second face.
    pub fn give_up_copy(&mut self, id: u64) -> Result<(), String> {
        let b = *self.blocks.get(&id).ok_or("no such block")?;
        if let Some(s) = b.slot {
            if self.slots[s].readers != 1 {
                self.throws += 1;
                return Err(EAGER_GIVE_THROW.to_string());
            }
        }
        self.blocks.remove(&id);
        self.take_slot(b);
        Ok(())
    }

    fn take_slot(&mut self, b: Block) {
        if let Some(s) = b.slot {
            self.used -= 1;
            self.registrations -= 1;
            self.slots[s].readers -= 1; // the cache lets go
            self.slots[s].detached = true;
            if self.slots[s].readers == 0 {
                self.return_slot(s);
            }
        }
    }

    // Quota pressure: give up `units` copies. A throw inside aborts the whole
    // reclaim (the caller restarts the engine in the source's own words).
    pub fn reclaim(&mut self, units: usize) -> Result<usize, String> {
        let ids: Vec<u64> = {
            let mut v: Vec<u64> = self.blocks.keys().copied().collect();
            v.sort_unstable();
            v
        };
        let mut got = 0;
        for id in ids {
            if got == units {
                break;
            }
            match self.give_up_copy(id) {
                Ok(()) => got += 1,
                Err(e) => return Err(e), // the whole reclaim aborts mid-loop
            }
        }
        Ok(got)
    }

    // checkInvariants: asserted ONLY between commands, in the tests (the
    // quiescent window). A live block's copy with no holder and no transfer
    // must have no other reader; the detached intermediate is not a block and
    // is not looked at here at all.
    pub fn check_invariants_between_commands(&self) -> Result<(), String> {
        for (id, b) in &self.blocks {
            if let Some(s) = b.slot {
                if !b.held && !b.transfer && self.slots[s].readers != 1 {
                    return Err(format!(
                        "block {id}: a copy no request holds and no transfer moves has no reader"
                    ));
                }
            }
        }
        Ok(())
    }
}

// -- shared contract tests (both arms) ----------------------------------------

#[test]
fn eq_a_quiet_block_erases_and_its_slot_returns_at_once() {
    let mut c = Cache::new(4, 4);
    c.insert_with_copy(7);
    assert_eq!(c.used, 1);
    assert!(c.erase(7).is_ok());
    assert_eq!(c.used, 0);
    assert_eq!(c.returns, vec![0]); // no reader: the slot returns at once
    assert!(c.check_invariants_between_commands().is_ok());
}

#[test]
fn eq_a_live_request_holds_its_copy_through_a_command() {
    let mut c = Cache::new(4, 4);
    c.insert_with_copy(1);
    c.insert_with_copy(2);
    let h = c.open_copy(1).unwrap();
    assert_eq!(c.slots[0].readers, 2); // the cache + the request's copy
    c.close_copy(h); // the command ends; the copy goes
    assert_eq!(c.slots[0].readers, 1);
    assert!(c.check_invariants_between_commands().is_ok());
}

// -- the anti-pattern's own shape ---------------------------------------------

#[test]
fn bug_a_finished_command_s_copy_makes_erase_throw_and_the_quota_sticks() {
    // The command FINISHED (its results are being applied); its lane still
    // holds the copy of a spilled block. Quota pressure must reclaim that
    // block now -- the eager throw turns the legal intermediate into a
    // crash, and the block (and its quota unit) stay stuck.
    let mut c = Cache::new(4, 4);
    c.insert_with_copy(1);
    c.insert_with_copy(2);
    let h = c.open_copy(2).unwrap(); // the finished command's copy, still named
    let err = c.erase(2).unwrap_err();
    assert_eq!(err, EAGER_ERASE_THROW);
    assert_eq!(c.throws, 1);
    assert_eq!(c.used, 2, "the quota unit never returned");
    assert!(c.blocks.contains_key(&2), "the block never left the index");
    c.close_copy(h); // only a manual drop heals it -- too late for the caller
    assert!(c.erase(2).is_ok());
}

#[test]
fn bug_the_reclaim_aborts_mid_loop_on_the_lingering_copy() {
    // The reclaim walks blocks oldest-first; the SECOND one is a finished
    // command's block. The eager give-up throw kills the whole pass: the
    // first reclamation happened, the rest never did, and the caller
    // restarts the engine (or aborts on the last spilled page).
    let mut c = Cache::new(8, 8);
    for id in 1..=4 {
        c.insert_with_copy(id);
    }
    let h = c.open_copy(2).unwrap(); // block 2's copy outlives its command
    let err = c.reclaim(3).unwrap_err();
    assert_eq!(err, EAGER_GIVE_THROW);
    assert_eq!(c.throws, 1);
    assert_eq!(c.used, 3, "only block 1 was reclaimed before the abort");
    assert!(c.blocks.contains_key(&2) && c.blocks.contains_key(&3) && c.blocks.contains_key(&4));
    c.close_copy(h);
    assert_eq!(c.reclaim(3).unwrap(), 3);
    assert_eq!(c.used, 0);
}

#[test]
fn bug_the_intermediate_state_is_invisible_to_the_law_the_invariant_cannot_express() {
    // The legal intermediate (the block gone from the index while its copy's
    // slot still rides a reader) is exactly the state the eager throw
    // refuses to produce: the buggy arm never reaches it, so the cache
    // cannot serve quota-and-reclaim "while the command that read them
    // holds its sources" -- the source's own property test scenario.
    let mut c = Cache::new(4, 4);
    c.insert_with_copy(9);
    let h = c.open_copy(9).unwrap();
    assert!(c.erase(9).is_err()); // the intermediate is unreachable
    assert_eq!(c.returns.len(), 0);
    c.close_copy(h);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `finished-command-copy-leaves-at-once-slot-returns-at-last-reader`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/{KvCache,KvCache}.{cpp,hpp}
// @ 91d4ddb8 (Apache-2.0): the law -- a FINISHED operation's lingering read
// handle does not block eviction or quota return. The entry leaves the index
// AT ONCE; the subordinate resource (the disk slot) is tied to the handle and
// returns at the LAST drop; and the no-lingering-reader invariant is asserted
// only in the QUIESCENT window between commands -- the intermediate state (a
// finished command's copy still named while the block is gone) is legal,
// never a fatal throw.
//
// ORIGINAL machinery -- never splash code verbatim. A slot's `readers` count
// models the shared_ptr use_count (the cache's own registration included):
// 1 = only the cache; >1 = a request's (or a finished command's) copy lives.

use std::collections::HashMap;

pub struct Slot {
    pub readers: usize, // the cache's own registration + every live copy handle
    pub detached: bool, // the block left; the slot rides its readers now
    pub returned: bool, // back to the file (reusable)
}

#[derive(Clone, Copy)]
pub struct Handle {
    slot: usize,
    open: bool,
}

#[derive(Clone, Copy)]
pub struct Block {
    pub slot: Option<usize>,
    pub held: bool,     // a request holds the block
    pub transfer: bool, // a transfer moves it
}

pub struct Cache {
    pub blocks: HashMap<u64, Block>,
    pub slots: Vec<Slot>,
    pub free: Vec<usize>,     // returned slot ids, reusable
    pub used: usize,          // quota units: blocks carrying a slot
    pub quota: usize,
    pub registrations: usize, // slots the cache itself registers
    pub returns: Vec<usize>,  // event log: slots returned at last drop
}

impl Cache {
    pub fn new(quota: usize, file_slots: usize) -> Self {
        Cache {
            blocks: HashMap::new(),
            slots: (0..file_slots)
                .map(|_| Slot { readers: 0, detached: false, returned: false })
                .collect(),
            free: Vec::new(),
            used: 0,
            quota,
            registrations: 0,
            returns: Vec::new(),
        }
    }

    // A block lands with a disk copy (the spill): one quota unit, the cache's
    // own registration on the slot.
    pub fn insert_with_copy(&mut self, id: u64) {
        let s = match self.free.pop() {
            Some(s) => s,
            None => self
                .slots
                .iter()
                .position(|s| s.readers == 0 && !s.returned)
                .expect("no file slot"),
        };
        self.slots[s] = Slot { readers: 1, detached: false, returned: false };
        self.blocks.insert(id, Block { slot: Some(s), held: false, transfer: false });
        self.used += 1;
        self.registrations += 1;
    }

    // A request reads the spilled K/V: it takes a copy (KvTier::copy).
    pub fn open_copy(&mut self, id: u64) -> Option<Handle> {
        let s = self.blocks.get(&id)?.slot?;
        self.slots[s].readers += 1;
        Some(Handle { slot: s, open: true })
    }

    // A reader's drop: the LAST drop returns a detached slot to the file.
    pub fn close_copy(&mut self, h: Handle) {
        if !h.open {
            return;
        }
        let s = h.slot;
        self.slots[s].readers -= 1;
        if self.slots[s].detached && self.slots[s].readers == 0 {
            self.return_slot(s);
        }
    }

    fn return_slot(&mut self, s: usize) {
        self.slots[s].returned = true;
        self.free.push(s);
        self.returns.push(s);
    }

    // THE LAW: the entry leaves the index at once; the copy's quota unit goes
    // with it; the slot detaches and rides its readers, returning to the file
    // at the LAST drop. No reader check, no throw -- the intermediate is a
    // legal state the cache serves.
    pub fn erase(&mut self, id: u64) -> Result<(), String> {
        let b = *self.blocks.get(&id).ok_or("no such block")?;
        self.blocks.remove(&id);
        self.take_slot(b);
        Ok(())
    }

    // The quota's give-up-copy path: the same law, no throw.
    pub fn give_up_copy(&mut self, id: u64) -> Result<(), String> {
        self.erase(id)
    }

    fn take_slot(&mut self, b: Block) {
        if let Some(s) = b.slot {
            self.used -= 1;
            self.registrations -= 1;
            self.slots[s].readers -= 1; // the cache lets go
            self.slots[s].detached = true;
            if self.slots[s].readers == 0 {
                self.return_slot(s);
            }
        }
    }

    // Quota pressure: give up `units` copies. The pass completes under a
    // finished command's copy; the slot returns when its last reader drops.
    pub fn reclaim(&mut self, units: usize) -> Result<usize, String> {
        let ids: Vec<u64> = {
            let mut v: Vec<u64> = self.blocks.keys().copied().collect();
            v.sort_unstable();
            v
        };
        let mut got = 0;
        for id in ids {
            if got == units {
                break;
            }
            self.give_up_copy(id)?;
            got += 1;
        }
        Ok(got)
    }

    // checkInvariants: asserted ONLY between commands, in the tests (the
    // quiescent window). A live block's copy with no holder and no transfer
    // must have no other reader. The DETACHED intermediate (a finished
    // command's copy still naming its slot, the block gone) is legal and is
    // not a violation -- it is not a block in the index at all.
    pub fn check_invariants_between_commands(&self) -> Result<(), String> {
        for (id, b) in &self.blocks {
            if let Some(s) = b.slot {
                if !b.held && !b.transfer && self.slots[s].readers != 1 {
                    return Err(format!(
                        "block {id}: a copy no request holds and no transfer moves has no reader"
                    ));
                }
            }
        }
        Ok(())
    }
}

// -- shared contract tests (both arms) ----------------------------------------

#[test]
fn eq_a_quiet_block_erases_and_its_slot_returns_at_once() {
    let mut c = Cache::new(4, 4);
    c.insert_with_copy(7);
    assert_eq!(c.used, 1);
    assert!(c.erase(7).is_ok());
    assert_eq!(c.used, 0);
    assert_eq!(c.returns, vec![0]); // no reader: the slot returns at once
    assert!(c.check_invariants_between_commands().is_ok());
}

#[test]
fn eq_a_live_request_holds_its_copy_through_a_command() {
    let mut c = Cache::new(4, 4);
    c.insert_with_copy(1);
    c.insert_with_copy(2);
    let h = c.open_copy(1).unwrap();
    assert_eq!(c.slots[0].readers, 2); // the cache + the request's copy
    c.close_copy(h); // the command ends; the copy goes
    assert_eq!(c.slots[0].readers, 1);
    assert!(c.check_invariants_between_commands().is_ok());
}

// -- the law's own shape -------------------------------------------------------

#[test]
fn fixed_a_finished_command_s_copy_lets_the_block_leave_and_the_quota_return() {
    // The command FINISHED; its lane still names the copy. The block leaves
    // the index at once, the quota unit returns at once, the quiescent-window
    // invariant passes over the detached slot -- and the slot itself returns
    // to the file only at the LAST drop.
    let mut c = Cache::new(4, 4);
    c.insert_with_copy(2);
    let h = c.open_copy(2).unwrap();
    assert!(c.erase(2).is_ok());
    assert_eq!(c.used, 0, "the copy's quota goes at once");
    assert!(!c.blocks.contains_key(&2), "the entry leaves at once");
    assert_eq!(c.returns.len(), 0, "the slot rides its reader still");
    assert!(c.slots[0].detached);
    assert_eq!(c.slots[0].readers, 1); // the finished command's copy
    assert!(
        c.check_invariants_between_commands().is_ok(),
        "the intermediate state is legal between commands"
    );
    c.close_copy(h); // the last drop
    assert_eq!(c.returns, vec![0], "the slot returns at the last drop");
    assert!(c.slots[0].returned);
}

#[test]
fn fixed_the_reclaim_completes_under_a_finished_command_s_copy() {
    let mut c = Cache::new(8, 8);
    for id in 1..=4 {
        c.insert_with_copy(id);
    }
    let h = c.open_copy(2).unwrap(); // block 2's copy outlives its command
    assert_eq!(c.reclaim(3).unwrap(), 3, "the whole pass completes");
    assert_eq!(c.used, 1, "only block 4 remains");
    assert!(!c.blocks.contains_key(&2), "block 2 left with the rest");
    assert_eq!(c.returns, vec![0, 2], "quiet slots returned at once; only the detached one rides");
    assert!(!c.slots[1].returned, "the detached slot still rides its reader");
    assert!(c.check_invariants_between_commands().is_ok());
    c.close_copy(h); // the last reader lets go
    assert_eq!(c.returns, vec![0, 2, 1], "its slot returns then");
    assert_eq!(c.used, 1);
}

#[test]
fn fixed_the_returned_slot_is_reusable_and_the_invariant_keeps_its_teeth() {
    let mut c = Cache::new(4, 4);
    c.insert_with_copy(9);
    let h = c.open_copy(9).unwrap();
    assert!(c.erase(9).is_ok()); // the detached intermediate
    c.close_copy(h); // the slot returns
    c.insert_with_copy(10); // the file slot is reusable at once
    assert_eq!(c.blocks[&10].slot, Some(0));
    // the invariant still fires on a REAL violation: a live block whose copy
    // a non-request reader keeps, with no holder and no transfer.
    let mut d = Cache::new(4, 4);
    d.insert_with_copy(3);
    d.slots[0].readers += 1; // a lingering reader with no excuse
    let err = d.check_invariants_between_commands().unwrap_err();
    assert!(err.contains("a copy no request holds and no transfer moves has no reader"));
}
"#.to_string(),
            context_notes: Some("Grade B+ -- NotBenchable ON THIS BOX (the decision content is the SERVING-LEVEL lifetime contract for a finished operation's read handle -- which intermediate states are legal BETWEEN commands and which subordinate resource returns at WHICH drop; a Rust microbench of the stand-in cannot measure it and the source ships no perf figure for the repair -- its win is reclaim COMPLETING where it used to throw, evidenced upstream by the commit's own test battery). SOURCE (incoai/splash @ 91d4ddb8d8125eb89aac12762b2c8bcf727fd7e3 'Let a block leave while a finished command still names its copy', Apache-2.0, Jian Chen, 2026-10-17; every quote re-verified verbatim at the .raw/b271/splash clone this batch at that pin; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (commit message, verbatim): 'KvCache::erase and dropping a copy threw while anything outside the cache held the copy (use_count), and erasePoisonedLeaf turned that into std::terminate.' + 'Each of those threw out of endRequest or poisonCopy and restarted the engine, or aborted the process when the poisoned block was the last spilled one.' THE REMOVED THROWS (91d4ddb^ runtime/engine/KvCache.cpp:600-602 at erase + :433-437 at giveDiskCopy, verbatim modulo the house quote rendering): 'if (candidate.slot && candidate.slot.use_count() != 1) throw std::logic_error('a KV cache block left while a reader still holds its copy');' and 'if (entry.slot && !slot && entry.slot.use_count() != 1) throw std::logic_error('a KV cache block gave up a copy that a reader still holds');' -- the eager check made the engine's apply window a crash window (Engine::step keeps the command, and so its cut's sources, alive across apply). THE LAW (commit message, verbatim): 'Nothing reads such a copy any more: the command has completed. The block now leaves; the copy's slot returns to the file when the last reader lets go. That the copies of blocks nothing holds or moves have no reader is what checkInvariants asserts, between commands, in the tests; the repository has no test-build assertions other than these checks.' THE QUARANTINE-WINDOW INVARIANT (runtime/engine/KvCache.cpp:690-691 @ 91d4ddb, verbatim code): 'check(!entry.slot || entry.held || entry.transfer != Transfer::None || entry.slot.use_count() == 1, 'a copy no request holds and no transfer moves has no reader');' -- inside checkInvariants, i.e. the QUIESCENT window, never the apply path; its own comment (KvCache.cpp:686-689, verbatim): 'Reads of spilled K/V hold a copy only while a request holds its block or a transfer moves it (Cache::cutPageTable), so a copy quota gives up returns its slot at once. A command's cut names blocks its lanes held, and outlives them only while the engine applies its results.' THE SUBORDINATE-RESOURCE LAW (runtime/engine/KvCache.hpp:212-215, the erase doc @ 91d4ddb, verbatim): 'Only a block without children, users, holders or transfer can be removed. Its copy goes with it, and the copy's slot once a reader that still holds it (KvTier::copy) lets go. The caller handles any composite state attached to it first.' THE REPAIR: erase and the give-up-copy path complete unconditionally -- the entry leaves the index at once (the quota unit goes with it), the slot detaches and rides its readers, and the file slot returns at the LAST drop; the invariant's teeth live entirely in the between-commands check. THE COMMIT'S OWN BATTERY names the shapes (message, verbatim): 'quota and reclaim give up the blocks of a request that ended while its command's cut lives, and their slots return once it goes; a poisoned copy, at the first or the last spilled page, before or after its lane ends, leaves with the blocks below it while the cut lives; the KvCache invariants find a copy whose block nothing holds that a reader kept; and the property runs end and poison lanes while the command that read them holds its sources, with quota and reclaim in between.' REGIME: serving caches whose entries carry a subordinate pooled resource (disk slots, staging buffers) where an operation FINISHES before its results are applied and its read handles linger across that window; any eviction/quota path that can run while a finished operation's handle is alive. ANTI-REGIMES: IN-FLIGHT readers whose data is still needed (that is the pin/tombstone family below -- a finished command needs nothing more); caches without a pooled subordinate (the entry can free outright); invariant checks that must run on the live path (they cannot tolerate the legal intermediate by definition). KINSHIP/DISTINCT-FROM (read + disclosed): pinned-entry-tombstone-freed-at-last-unpin (B256, entries_36) is the OPPOSITE posture -- IN-FLIGHT jobs where eviction SKIPS pinned entries and the entry survives as a tombstone until its last unpin, because the job still NEEDS the data; HERE the command FINISHED, nothing needs the copy, the entry leaves AT ONCE and only the subordinate slot defers to the last drop -- compose at the lifecycle boundary (in-flight = skip + tombstone; finished = leave-now + deferred slot return). publication-takes-over-condemned-cache-entry (B270, entries_40) owns the reader-keeps-its-handle half on the REPUBLICATION axis (a new write taking over a condemned entry a reader still reads) -- not quota/eviction return; compose. prompt-cache-stale-invalidation-push-borrower-boundary (kernel_opt B249 slice 2) is the quiescent-window kin -- collect only at safe host boundaries with borrowers zero; HERE the boundary is BETWEEN COMMANDS and it is the ASSERT (not the collect) that lives there. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'use_count|lingering reader|finished command|slot return|quiescent|giveDiskCopy|erasePoisonedLeaf|copy quota' -- zero owners; the adjacent hits disclosed: functional-cache-versioning-loses-donation-under-kernel-read (gpu_14) reads use_count()==1 as the DONATION pin on a write path (donation eligibility, not a finished-command lifetime); copy-source-quarantine-unproven-free-pinned (gpu_111) names 'refcounted buffers where the last reader frees' only as its own anti-regime (a failed-wait custody plane); smem-budget-single-buffer-refill-at-kill-points (gpu_40) and dead-line-l2-discard-after-last-consumer (gpu_48) carry the 'last reader/last consumer' vocabulary in GPU staging and L2 domains -- different objects, no compose. FIXTURE PROVENANCE: original machinery (never splash code) -- a block index whose entries carry reader-counted disk slots (the cache's own registration = 1, the use_count model), erase + give-up-copy + a reclaim loop, and a quiescent-window invariant; the shared-contract tests (a quiet block erases and its slot returns at once; a live request holds its copy through a command) pass in BOTH arms; the discriminating tests pin the eager throw on the legal intermediate (throw, quota stuck, reclaim aborted mid-loop, the intermediate unreachable) against the law (leaves at once, quota at once, the invariant passes over the detached slot, the slot returns at the last drop, the returned slot is reusable, the invariant keeps its teeth on a real violation). Arms 5/5 + 5/5 standalone (rustc --edition 2021 --test). Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: @ 91d4ddb8d8125eb89aac12762b2c8bcf727fd7e3 -- runtime/engine/KvCache.cpp:686-691 + :600-602@^ + :433-437@^, runtime/engine/KvCache.hpp:212-215, commit message (all quotes); Batch 271 / Plan 275.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "part-cost-measured-by-planning-ablation-not-component-sum".to_string(),
            name: "A part's memory cost is measured as the PLANNING DELTA with-and-without it at the site it loads from -- plan the images with it and without it, take the difference, and add the side table it loads with -- never as a sum of its primary components: a component-sum budget one byte short flips the part to its streaming floor and the plan made with what actually loaded then fails the start; and an automatic part whose smallest store takes all of it loads WHOLE (no floor-first double load)"
                .to_string(),
            category: "complexity".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `part-cost-measured-by-planning-ablation-not-component-sum`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/model/ModelSizing.cpp @
// ed1092f8 (Apache-2.0): a start sizes a MoE target's streamed stores. The
// anti-pattern sizes the routed experts' part by the bytes of their PLANES
// (a component sum of the part's primary), which leaves out what their
// IMAGES take beyond the planes and the slot table they load with -- so the
// whole-store budget the sizing hands the start is short, a posture the
// budget cannot hold is picked, and the plan -- made with what actually
// loaded -- holds no lane's state: the start fails. The automatic part is
// also probed at its FLOOR first, so a part whose smallest store takes all
// of it loads all its weights twice.
//
// ORIGINAL machinery -- never splash code verbatim. `plan_loaded` is the
// planning source of truth BOTH arms share (what loads under a posture);
// only the SIZING of the expert part differs.

pub const GIB: u64 = 1 << 30;

pub struct Layout {
    pub dense_bytes: u64,           // the dense weights (always load)
    pub plane_bytes: u64,           // the routed experts' planes
    pub image_overhead_bytes: u64,  // what their images take beyond the planes
    pub slot_table_bytes: u64,      // the routed experts' slot table, with them
    pub expert_floor_bytes: u64,    // the experts' smallest (streamed) store
    pub table_row_cache_bytes: u64, // the PLE table at its planned row cache
    pub table_floor_bytes: u64,     // the PLE table at its smallest row cache
    pub lanes: u64,
    pub lane_state_bytes: u64,      // per-lane state the plan must hold
}

pub struct Streamable {
    pub loaded: u64,             // what loads while every part streams
    pub table_bytes: u64,        // what the table adds where it loads instead
    pub expert_bytes: u64,       // what the experts add where they load instead
    pub expert_floor_bytes: u64, // the experts' smallest store
}

// What loads under a posture: the plan both arms consume.
pub fn plan_loaded(l: &Layout, stream_experts: bool, stream_table: bool) -> u64 {
    let experts = if stream_experts {
        l.expert_floor_bytes
    } else {
        l.plane_bytes + l.image_overhead_bytes
    };
    let table = if stream_table { l.table_floor_bytes } else { l.table_row_cache_bytes };
    l.dense_bytes + experts + table
}

// THE ANTI-PATTERN: the routed experts' cost is the bytes of their planes --
// a component sum of the part's primary. The images beyond the planes and
// the slot table they load with are left out, so the whole-store budget the
// sizing computes is short by exactly what it left out.
pub fn target_weights_component_sum(l: &Layout, moe: bool) -> Streamable {
    let streamed = plan_loaded(l, moe, true);
    let table_bytes = plan_loaded(l, moe, false) - streamed; // the ablation, held
    let expert_bytes = if moe { l.plane_bytes } else { 0 };
    Streamable {
        loaded: streamed,
        table_bytes,
        expert_bytes,
        expert_floor_bytes: l.expert_floor_bytes,
    }
}

#[derive(Debug, PartialEq)]
pub enum Start {
    Ran { loads: usize, arena_bytes: u64 },
    Failed(String),
}

// The start: the sizing's whole-store budget decides the posture, the loader
// loads what the posture says, and the plan -- made with what loaded -- must
// hold every lane's state. The automatic part probes its floor first.
pub fn start(l: &Layout, budget: u64, sizing: &Streamable, moe: bool) -> Start {
    let whole_cost = sizing.loaded + sizing.table_bytes + sizing.expert_bytes;
    let mut loads = 0;
    let posture = if whole_cost <= budget {
        (false, false)
    } else if moe && sizing.expert_floor_bytes >= sizing.expert_bytes {
        // the probe loaded the smallest store -- all of the part; the real
        // load then loads all of it AGAIN
        loads += 1;
        (true, true)
    } else {
        (true, true)
    };
    loads += 1;
    let loaded = plan_loaded(l, posture.0, posture.1);
    let arena = budget.saturating_sub(loaded);
    let need = l.lanes * l.lane_state_bytes;
    if arena < need {
        return Start::Failed(format!(
            "the plan made with what loaded holds no lane's state: arena {arena} < {need}"
        ));
    }
    Start::Ran { loads, arena_bytes: arena }
}

pub fn fixture_layout() -> Layout {
    Layout {
        dense_bytes: 8 * GIB,
        plane_bytes: 20 * GIB,
        image_overhead_bytes: 2 * GIB,  // what the images take beyond the planes
        slot_table_bytes: GIB / 2,      // the routed experts' slot table
        expert_floor_bytes: GIB / 2,    // the streamed floor (its own store)
        table_row_cache_bytes: GIB,
        table_floor_bytes: GIB / 4,
        lanes: 4,
        lane_state_bytes: GIB / 2,      // 2 GiB of lane state the plan must hold
    }
}

// The TRUE whole-store cost: what the whole posture really loads.
pub fn true_whole_cost(l: &Layout) -> u64 {
    plan_loaded(l, false, false)
}

// -- shared contract tests (both arms) ----------------------------------------

#[test]
fn eq_a_generous_budget_loads_whole_once_and_holds_the_lanes() {
    let l = fixture_layout();
    let s = target_weights_component_sum(&l, true);
    let budget = true_whole_cost(&l) + 4 * GIB;
    match start(&l, budget, &s, true) {
        Start::Ran { loads, arena_bytes } => {
            assert_eq!(loads, 1);
            assert!(arena_bytes >= 4 * l.lane_state_bytes + 2 * GIB);
        }
        other => panic!("expected a whole-load start, got {other:?}"),
    }
}

#[test]
fn eq_a_short_budget_streams_experts_in_both_arms() {
    let l = fixture_layout();
    let s = target_weights_component_sum(&l, true);
    // far below any whole store: streaming is the honest posture here
    let budget = l.dense_bytes + l.expert_floor_bytes + l.table_floor_bytes + 3 * GIB;
    match start(&l, budget, &s, true) {
        Start::Ran { loads, arena_bytes } => {
            assert_eq!(loads, 1);
            assert!(arena_bytes >= 4 * l.lane_state_bytes);
        }
        other => panic!("expected a streaming start, got {other:?}"),
    }
}

#[test]
fn eq_the_table_part_is_measured_by_the_plan_delta() {
    // the table's ablation is the one measurement the component sum got right
    let l = fixture_layout();
    let s = target_weights_component_sum(&l, true);
    assert_eq!(s.table_bytes, l.table_row_cache_bytes - l.table_floor_bytes);
    assert_eq!(s.loaded, plan_loaded(&l, true, true));
}

// -- the anti-pattern's own shape ---------------------------------------------

#[test]
fn bug_the_component_sum_hides_the_images_and_the_slot_table() {
    let l = fixture_layout();
    let s = target_weights_component_sum(&l, true);
    assert_eq!(s.expert_bytes, l.plane_bytes);
    // the truth, by ablation at the site the part loads from, slot table with it
    let truth = plan_loaded(&l, false, true) - plan_loaded(&l, true, true) + l.slot_table_bytes;
    assert_eq!(truth, l.image_overhead_bytes + l.slot_table_bytes + l.plane_bytes
        - l.expert_floor_bytes);
    assert!(s.expert_bytes < truth, "the sizing's whole-store budget is short");
}

#[test]
fn bug_a_budget_a_byte_short_of_the_whole_store_fails_the_start() {
    // the budget holds the whole store EXCEPT what the sizing left out (the
    // images beyond the planes and the slot table, less the floor). The
    // under-counted whole-store budget fits, the whole posture is picked,
    // what loads eats the budget -- and the plan made with what loaded holds
    // no lane's state: the start fails.
    let l = fixture_layout();
    let s = target_weights_component_sum(&l, true);
    let budget = true_whole_cost(&l) - 1;
    match start(&l, budget, &s, true) {
        Start::Failed(msg) => {
            assert!(msg.contains("no lane's state"), "{msg}");
        }
        other => panic!("the byte-short start should fail, got {other:?}"),
    }
}

#[test]
fn bug_an_automatic_part_whose_floor_takes_all_loads_twice() {
    // the smallest store takes ALL of the experts (floor == the whole store):
    // the floor probe pre-loads every weight, then the real load loads every
    // weight again -- two loads where the law loads once.
    let mut l = fixture_layout();
    l.expert_floor_bytes = l.plane_bytes + l.image_overhead_bytes; // the floor takes all
    let s = target_weights_component_sum(&l, true);
    let budget = true_whole_cost(&l) + 4 * GIB;
    match start(&l, budget, &s, true) {
        Start::Ran { loads, .. } => assert_eq!(loads, 2, "all the weights, twice"),
        other => panic!("expected a start, got {other:?}"),
    }
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `part-cost-measured-by-planning-ablation-not-component-sum`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/model/ModelSizing.cpp @
// ed1092f8 (Apache-2.0): the law -- a part's memory cost is measured as the
// PLANNING DELTA with-and-without it at the site it loads from (plan the
// images with it and without it, take the difference; the side table it
// loads with rides along), never as a sum of its primary components. A
// component-sum budget one byte short flips the posture and the plan made
// with what actually loaded then fails the start; and an automatic part
// whose smallest store takes all of it loads WHOLE -- no floor-first double
// load.
//
// ORIGINAL machinery -- never splash code verbatim. `plan_loaded` is the
// planning source of truth BOTH arms share (what loads under a posture);
// only the SIZING of the expert part differs.

pub const GIB: u64 = 1 << 30;

pub struct Layout {
    pub dense_bytes: u64,           // the dense weights (always load)
    pub plane_bytes: u64,           // the routed experts' planes
    pub image_overhead_bytes: u64,  // what their images take beyond the planes
    pub slot_table_bytes: u64,      // the routed experts' slot table, with them
    pub expert_floor_bytes: u64,    // the experts' smallest (streamed) store
    pub table_row_cache_bytes: u64, // the PLE table at its planned row cache
    pub table_floor_bytes: u64,     // the PLE table at its smallest row cache
    pub lanes: u64,
    pub lane_state_bytes: u64,      // per-lane state the plan must hold
}

pub struct Streamable {
    pub loaded: u64,             // what loads while every part streams
    pub table_bytes: u64,        // what the table adds where it loads instead
    pub expert_bytes: u64,       // what the experts add where they load instead
    pub expert_floor_bytes: u64, // the experts' smallest store
}

// What loads under a posture: the plan both arms consume.
pub fn plan_loaded(l: &Layout, stream_experts: bool, stream_table: bool) -> u64 {
    let experts = if stream_experts {
        l.expert_floor_bytes
    } else {
        l.plane_bytes + l.image_overhead_bytes
    };
    let table = if stream_table { l.table_floor_bytes } else { l.table_row_cache_bytes };
    l.dense_bytes + experts + table
}

// THE LAW: each part's cost where it loads from is planning the images WITH
// it and WITHOUT it -- the plan twice, the difference -- and the routed
// experts' slot table rides with them. The table's delta is the same
// ablation (plan the table on, then off, over the same streamed base).
pub fn target_weights(l: &Layout, moe: bool) -> Streamable {
    let streamed = plan_loaded(l, moe, true);
    let table_bytes = plan_loaded(l, moe, false) - streamed;
    let expert_bytes = if moe {
        plan_loaded(l, false, true) - streamed + l.slot_table_bytes
    } else {
        0
    };
    Streamable {
        loaded: streamed,
        table_bytes,
        expert_bytes,
        expert_floor_bytes: l.expert_floor_bytes,
    }
}

#[derive(Debug, PartialEq)]
pub enum Start {
    Ran { loads: usize, arena_bytes: u64 },
    Failed(String),
}

// The start: the sizing's whole-store budget decides the posture, the loader
// loads what the posture says, and the plan -- made with what loaded -- must
// hold every lane's state. An automatic part whose smallest store takes all
// of it loads whole -- there is no floor-first probe to double-load.
pub fn start(l: &Layout, budget: u64, sizing: &Streamable, moe: bool) -> Start {
    let whole_cost = sizing.loaded + sizing.table_bytes + sizing.expert_bytes;
    let mut loads = 0;
    let posture = if whole_cost <= budget {
        (false, false)
    } else if moe && sizing.expert_floor_bytes >= sizing.expert_bytes {
        // the smallest store takes all of the part: loading the floor IS
        // loading the whole store -- one load, never a probe and a repeat
        (false, false)
    } else {
        (true, true)
    };
    loads += 1;
    let loaded = plan_loaded(l, posture.0, posture.1);
    let arena = budget.saturating_sub(loaded);
    let need = l.lanes * l.lane_state_bytes;
    if arena < need {
        return Start::Failed(format!(
            "the plan made with what loaded holds no lane's state: arena {arena} < {need}"
        ));
    }
    Start::Ran { loads, arena_bytes: arena }
}

pub fn fixture_layout() -> Layout {
    Layout {
        dense_bytes: 8 * GIB,
        plane_bytes: 20 * GIB,
        image_overhead_bytes: 2 * GIB,  // what the images take beyond the planes
        slot_table_bytes: GIB / 2,      // the routed experts' slot table
        expert_floor_bytes: GIB / 2,    // the streamed floor (its own store)
        table_row_cache_bytes: GIB,
        table_floor_bytes: GIB / 4,
        lanes: 4,
        lane_state_bytes: GIB / 2,      // 2 GiB of lane state the plan must hold
    }
}

// The TRUE whole-store cost: what the whole posture really loads.
pub fn true_whole_cost(l: &Layout) -> u64 {
    plan_loaded(l, false, false)
}

// -- shared contract tests (both arms) ----------------------------------------

#[test]
fn eq_a_generous_budget_loads_whole_once_and_holds_the_lanes() {
    let l = fixture_layout();
    let s = target_weights(&l, true);
    let budget = true_whole_cost(&l) + 4 * GIB;
    match start(&l, budget, &s, true) {
        Start::Ran { loads, arena_bytes } => {
            assert_eq!(loads, 1);
            assert!(arena_bytes >= 4 * l.lane_state_bytes + 2 * GIB);
        }
        other => panic!("expected a whole-load start, got {other:?}"),
    }
}

#[test]
fn eq_a_short_budget_streams_experts_in_both_arms() {
    let l = fixture_layout();
    let s = target_weights(&l, true);
    let budget = l.dense_bytes + l.expert_floor_bytes + l.table_floor_bytes + 3 * GIB;
    match start(&l, budget, &s, true) {
        Start::Ran { loads, arena_bytes } => {
            assert_eq!(loads, 1);
            assert!(arena_bytes >= 4 * l.lane_state_bytes);
        }
        other => panic!("expected a streaming start, got {other:?}"),
    }
}

#[test]
fn eq_the_table_part_is_measured_by_the_plan_delta() {
    let l = fixture_layout();
    let s = target_weights(&l, true);
    assert_eq!(s.table_bytes, l.table_row_cache_bytes - l.table_floor_bytes);
    assert_eq!(s.loaded, plan_loaded(&l, true, true));
}

// -- the law's own shape -------------------------------------------------------

#[test]
fn fixed_the_expert_part_costs_the_ablation_delta_plus_its_slot_table() {
    let l = fixture_layout();
    let s = target_weights(&l, true);
    let expected = plan_loaded(&l, false, true) - plan_loaded(&l, true, true)
        + l.slot_table_bytes;
    assert_eq!(s.expert_bytes, expected);
    assert_eq!(s.expert_bytes, l.plane_bytes + l.image_overhead_bytes
        - l.expert_floor_bytes + l.slot_table_bytes);
}

#[test]
fn fixed_a_byte_short_of_the_true_whole_store_streams_and_the_start_lives() {
    // the same budget that killed the component-sum arm: the honest ablation
    // sees the whole store does not fit, streams the experts, and the plan
    // -- made with what actually loaded -- holds every lane's state.
    let l = fixture_layout();
    let s = target_weights(&l, true);
    let budget = true_whole_cost(&l) - 1;
    match start(&l, budget, &s, true) {
        Start::Ran { loads, arena_bytes } => {
            assert_eq!(loads, 1);
            assert!(arena_bytes >= 4 * l.lane_state_bytes, "{arena_bytes}");
        }
        other => panic!("the byte-short start should stream and live, got {other:?}"),
    }
}

#[test]
fn fixed_an_automatic_part_whose_floor_takes_all_loads_whole_once() {
    // the smallest store takes ALL of the experts: the ablation's delta is
    // the slot table alone, the part loads whole, and there is exactly ONE
    // load -- never the floor probe and the repeat.
    let mut l = fixture_layout();
    l.expert_floor_bytes = l.plane_bytes + l.image_overhead_bytes; // the floor takes all
    let s = target_weights(&l, true);
    assert_eq!(s.expert_bytes, l.slot_table_bytes, "the delta left is the slot table");
    let budget = true_whole_cost(&l) + 4 * GIB;
    match start(&l, budget, &s, true) {
        Start::Ran { loads, .. } => assert_eq!(loads, 1),
        other => panic!("expected a start, got {other:?}"),
    }
}
"#.to_string(),
            context_notes: Some("Grade B+ -- NotBenchable ON THIS BOX (the decision content is the SIZING ACCOUNTING of a streamed-store planner -- which budget arithmetic decides a part's posture at start; a Rust microbench of the stand-in cannot measure it; the source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ ed1092f8631e579e09906805b9dd4544ebf327a4 'Size the streamed stores by what the experts take whole, in at most three loads', Apache-2.0, Jian Chen, 2026-10-17; every quote re-verified verbatim at the .raw/b271/splash clone this batch at that pin; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (commit message, verbatim): 'The plan took the routed experts' cost where they load with their layers for the bytes of their planes, what a store of all of them takes, which leaves out what their images take beyond the planes and the slot table they load with. A budget a byte short of the one that holds every expert loaded them whole, and the engine's plan, made with what loaded, then held no lane's state: the start failed. It also loaded the PLE table at its smallest row cache first, so a model whose experts load whole, as Qwen3.8-Flash-Next does on a 64 GB Mac, loaded all its weights twice.' THE LAW (runtime/model/ModelSizing.cpp:30-33 @ ed1092f, verbatim): 'What a target of `layout` whose images `plan` plans for what streams takes, while their sources are open: what loads while every part streams, and what each part adds where it loads instead, the routed experts' slot table with them.' THE ABLATION ITSELF (ModelSizing.cpp:36-38 + :47-48, verbatim code): 'const std::vector<gguf::Image> streamed = plan({.streamExperts = moe, .streamTable = true}); StreamableWeights result{.loadedBytes = loadedBytes(streamed)}; result.tableBytes = loadedBytes(plan({.streamExperts = moe, .streamTable = false})) - result.loadedBytes;' + 'result.expertBytes = loadedBytes(plan({.streamExperts = false, .streamTable = true})) - result.loadedBytes + residentExpertSlotsBytes(layout.experts);' -- the plan runs TWICE per part (with it, without it) and the part's cost is the DIFFERENCE at the site it loads from, the side table it loads with added on; never the sum of the part's primary components. THE AUTOMATIC-PART CLAUSE (commit message, verbatim): 'An automatic part whose smallest store takes all that loads whole.' -- no floor-first probe-and-repeat for a part whose streaming floor is the whole store (the double-load face of the defect). THE COMMIT'S OWN TESTS name the arithmetic (message, verbatim): 'the starts count their loads: one where the experts load whole and with an explicit store, three a byte short of the budget that holds them, two short of what the plan without the arenas holds whole, each streaming through more than the floor and keeping 64K tokens of context.' REGIME: streamed-store sizers and start planners that price a PART's marginal cost to decide whole-vs-stream; budgets that can sit within a few percent of a whole-store boundary (the only place the accounting error is load-bearing). ANTI-REGIMES: parts with no streaming variant (one store, nothing to ablate); budgets orders of magnitude from the boundary (any rounding lives); reporting-only costing that decides nothing. KINSHIP/DISTINCT-FROM (read + disclosed): startup-estimate-derived-from-allocation-shapes (B196, entries_11) is the cost-accounting-honesty family -- THAT derives a STARTUP ADMISSION estimate from the allocator's own allocation shapes and proves it with a fake-allocator reconciliation; THIS prices a part's MARGINAL cost by planning ABLATION at the site it loads from (plan twice, take the difference, add the side table) -- the ablation is the decision-bearing form of the same honesty; compose. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'planning ablation|component sum|plan delta|expertBytes|slot table|residentExpertSlots|budget a byte|streamed store' -- zero owners; the expertBytes/streamed-store hits are moe-unverified-rows-route-zero-experts-dense-shape (gpu_117, B268) -- MoE verify-row ROUTING, not sizing. FIXTURE PROVENANCE: original machinery (never splash code) -- a two-part planner over a shared plan_loaded truth (what loads under a posture), the experts sized by component sum (the planes only) against the ablation delta plus the slot table, a byte-short budget that flips the posture, and the floor-takes-all double load; the shared-contract tests (a generous budget loads whole once; a short budget streams in both arms; the table part measured by the plan delta) pass in BOTH arms; the discriminating tests pin the component-sum hiding (the miss is exactly the images beyond the planes plus the slot table less the floor), the byte-short start failure (the whole store loads, the plan holds no lane's state) against the law arm's streaming start that lives, and the automatic part whose floor takes all (two loads vs one). Arms 6/6 + 6/6 standalone (rustc --edition 2021 --test). Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: @ ed1092f8631e579e09906805b9dd4544ebf327a4 -- runtime/model/ModelSizing.cpp:30-33 + :36-38 + :47-48, commit message (all quotes); Batch 271 / Plan 275.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "plan-input-precomputed-per-output-variant-bounded-convergence".to_string(),
            name: "When a plan's INPUT depends on the plan's own OUTPUT (the arena sizes depend on what streams; the plan decides what streams), precompute the input ONCE for each output variant -- a bounded 2x2 table keyed by what can stream -- and let each iteration read the input indexed by its own previous output: a bounded at-most-three-evaluation convergence replaces load-plan-reload re-measurement (a model load per posture change, and a fatal logic_error where the cycle widened the input); streaming is monotone in the arenas (a part that streams with smaller arenas streams with larger ones), so the flags only ever turn on and the plan streams no more than the arenas it was made with"
                .to_string(),
            category: "complexity".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `plan-input-precomputed-per-output-variant-bounded-convergence`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/engine/{MemoryPlan.cpp,
// RuntimeResources.mm} @ 2f5fe8dd (Apache-2.0): a plan's INPUT depends on the
// plan's own OUTPUT (the arena sizes depend on what streams; the plan decides
// what streams). The anti-pattern: plan with the arenas AS LAST LOADED, and
// where the plan changed what streams, LOAD AGAIN and plan again -- up to
// three loads -- and fail with a fatal logic_error where the plan streamed
// the table, whose staging then grew the arenas the plan was made with.
//
// ORIGINAL machinery -- never splash code verbatim. `arenas_for` models the
// measured shared arenas (they grow with the staging of what streams);
// `plan_parts` models the store decision given a budget and the arenas.

pub struct Model {
    pub budget: u64,
    pub dense: u64,
    pub lanes_bytes: u64,    // every lane's state, held beside the stores
    pub expert_whole: u64,   // the experts' whole store
    pub expert_floor: u64,   // the experts' streaming floor
    pub table_store: u64,    // the table's whole store
    pub expert_staging: u64, // the arenas' staging while the experts stream
    pub table_staging: u64,  // the arenas' staging while the table streams
    pub base_arenas: u64,
    pub cfg_experts: bool, // the posture the stores are configured with
    pub cfg_table: bool,
}

// The stores the plan streams that the posture does not stream yet.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Plan {
    pub expert_bytes: bool,
    pub table_bytes: bool,
}

pub fn arenas_for(m: &Model, experts: bool, table: bool) -> u64 {
    m.base_arenas
        + if experts { m.expert_staging } else { 0 }
        + if table { m.table_staging } else { 0 }
}

// The store decision given the arenas: a part streams when the budget cannot
// hold it whole beside them. The experts decide first; the table takes what
// the experts leave.
pub fn plan_parts(m: &Model, arenas: u64) -> Plan {
    let free = m.budget.saturating_sub(m.dense + arenas + m.lanes_bytes);
    let expert_bytes = free < m.expert_whole;
    let after_expert = free - if expert_bytes { m.expert_floor } else { m.expert_whole };
    let table_bytes = after_expert < m.table_store;
    Plan { expert_bytes, table_bytes }
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Planned { experts: bool, table: bool, loads: usize },
    Fatal(String),
}

// THE ANTI-PATTERN: plan with the arenas of the model AS LAST LOADED; where
// the plan changed what streams, load again and plan again (up to three
// loads); and where the plan streams the table -- whose staging the arenas
// then grow -- the plan's premise broke: a fatal logic_error.
pub fn plan_stores(m: &Model) -> Outcome {
    let mut experts = m.cfg_experts;
    let mut table = m.cfg_table;
    let mut loads = 0;
    loop {
        loads += 1; // the reload: the model loads (again) to re-measure
        let arenas = arenas_for(m, experts, table);
        let plan = plan_parts(m, arenas);
        if (experts || !plan.expert_bytes) && (table || !plan.table_bytes) {
            return Outcome::Planned { experts, table, loads };
        }
        if plan.table_bytes && !table {
            return Outcome::Fatal(
                "the plan streamed the table, whose staging the arenas then grew".to_string(),
            );
        }
        if loads >= 3 {
            return Outcome::Fatal("planned past the three loads".to_string());
        }
        experts = experts || plan.expert_bytes;
        table = table || plan.table_bytes;
    }
}

pub fn fixture() -> Model {
    Model {
        budget: 100,
        dense: 40,
        lanes_bytes: 20,
        expert_whole: 30,
        expert_floor: 2,
        table_store: 29,
        expert_staging: 2,
        table_staging: 6,
        base_arenas: 10,
        cfg_experts: false,
        cfg_table: false,
    }
}

// -- shared contract tests (both arms) ----------------------------------------

#[test]
fn eq_a_plan_that_flips_nothing_loads_once() {
    let mut m = fixture();
    m.budget = 200; // everything fits beside the arenas: no flips at all
    match plan_stores(&m) {
        Outcome::Planned { experts, table, loads } => {
            assert!(!experts && !table);
            assert_eq!(loads, 1);
        }
        other => panic!("expected a plan, got {other:?}"),
    }
}

#[test]
fn eq_a_table_configured_from_the_start_never_widens() {
    // the table streams by configuration: its staging was in the arenas the
    // plan was made with from the first pass -- no widening, no fatal
    let mut m = fixture();
    m.cfg_table = true;
    match plan_stores(&m) {
        Outcome::Planned { experts, table, loads } => {
            assert!(experts && table);
            assert_eq!(loads, 2); // the expert flip re-measured once
        }
        other => panic!("expected a plan, got {other:?}"),
    }
}

// -- the anti-pattern's own shape ---------------------------------------------

#[test]
fn bug_the_widening_input_is_a_fatal_logic_error() {
    // the configured posture streams nothing; the plan (made with the small
    // arenas) streams the table -- whose staging then grows the arenas: the
    // premise broke and the start died before it began.
    let m = fixture();
    match plan_stores(&m) {
        Outcome::Fatal(msg) => {
            assert!(msg.contains("whose staging the arenas then grew"), "{msg}");
        }
        other => panic!("the widening input should be fatal here, got {other:?}"),
    }
}

#[test]
fn bug_every_expert_only_flip_still_buys_a_whole_reload() {
    // even where the cycle never touches the table, every flip pays a whole
    // model load to re-measure the arenas -- two loads where the law loads
    // once.
    let mut m = fixture();
    m.budget = 99; // the experts stream, the table just fits beside the growth
    m.table_store = 25;
    match plan_stores(&m) {
        Outcome::Planned { experts, table, loads } => {
            assert!(experts && !table);
            assert_eq!(loads, 2, "the flip reloaded the model");
        }
        other => panic!("expected a plan, got {other:?}"),
    }
}

#[test]
fn bug_a_second_widening_flip_dies_inside_the_load_budget() {
    // the expert flip re-measures; the grown arenas then stream the table
    // too -- the fatal fires on the SECOND pass, two loads spent.
    let mut m = fixture();
    m.budget = 99;
    m.table_store = 27;
    match plan_stores(&m) {
        Outcome::Fatal(msg) => {
            assert!(msg.contains("whose staging the arenas then grew"), "{msg}");
        }
        other => panic!("expected the widening fatal, got {other:?}"),
    }
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `plan-input-precomputed-per-output-variant-bounded-convergence`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/{MemoryPlan.cpp,
// RuntimeResources.mm} @ 2f5fe8dd (Apache-2.0): the law -- when a plan's
// INPUT depends on the plan's own OUTPUT (the arena sizes depend on what
// streams; the plan decides what streams), precompute the input ONCE for
// each output variant (a bounded 2x2 table keyed by what can stream) and let
// each iteration read the input indexed by its own previous output: a
// bounded, at-most-three-evaluation convergence replaces load-plan-reload
// re-measurement, and a widening cycle is a plan answer, never a fatal.
//
// ORIGINAL machinery -- never splash code verbatim. `arenas_for` models the
// measured shared arenas (they grow with the staging of what streams);
// `plan_parts` models the store decision given a budget and the arenas. The
// two companion clauses of the law ride at the bottom: the KV allowance's
// consumer-side capacity rounding, and the optional cache's surrendered
// reserve.

pub const EXTENT_PAGES: u64 = 4; // the capacity granularity (whole extents)
pub const PAGE_TOKENS: u64 = 16; // tokens per page

pub struct Model {
    pub budget: u64,
    pub dense: u64,
    pub lanes_bytes: u64,    // every lane's state, held beside the stores
    pub expert_whole: u64,   // the experts' whole store
    pub expert_floor: u64,   // the experts' streaming floor
    pub table_store: u64,    // the table's whole store
    pub expert_staging: u64, // the arenas' staging while the experts stream
    pub table_staging: u64,  // the arenas' staging while the table streams
    pub base_arenas: u64,
    pub cfg_experts: bool, // the posture the stores are configured with
    pub cfg_table: bool,
    pub spill: bool,         // the plan spills (the cache's staging is possible)
    pub cache_reserve: u64,  // the optional cache's reserve
    pub keep_cache: bool,    // the tier keeps the cache at all
}

// The stores the plan streams that the posture does not stream yet.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Plan {
    pub expert_bytes: bool,
    pub table_bytes: bool,
}

pub fn arenas_for(m: &Model, experts: bool, table: bool) -> u64 {
    m.base_arenas
        + if experts { m.expert_staging } else { 0 }
        + if table { m.table_staging } else { 0 }
}

// The store decision given the arenas: a part streams when the budget cannot
// hold it whole beside them. The experts decide first; the table takes what
// the experts leave.
pub fn plan_parts(m: &Model, arenas: u64, budget: u64) -> Plan {
    let free = budget.saturating_sub(m.dense + arenas + m.lanes_bytes);
    let expert_bytes = free < m.expert_whole;
    let after_expert = free.saturating_sub(if expert_bytes { m.expert_floor } else { m.expert_whole });
    let table_bytes = after_expert < m.table_store;
    Plan { expert_bytes, table_bytes }
}

// COMPANION (b): the optional cache's reserve is surrendered where it alone
// would stream a part that loads whole without it, unless the plan spills --
// a tier that only caches costs the weights nothing.
pub fn effective_budget(m: &Model) -> u64 {
    let charged = m.budget - m.cache_reserve;
    if !m.keep_cache || m.spill {
        return charged;
    }
    let arenas = arenas_for(m, m.cfg_experts, m.cfg_table);
    let held = plan_parts(m, arenas, charged);
    let freed = plan_parts(m, arenas, m.budget);
    let streams = |p: &Plan| p.expert_bytes || p.table_bytes;
    if streams(&held) && !streams(&freed) {
        m.budget // the reserve gives way: the part loads whole
    } else {
        charged
    }
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Planned { experts: bool, table: bool, loads: usize, evals: usize },
    Fatal(String),
}

// THE LAW: the arenas are precomputed ONCE for each output variant (the 2x2
// table, one model load), and the loop reads the input indexed by its own
// previous output. A part that streams with smaller arenas streams with
// larger ones, so the flags only ever turn on and the plan converges after
// at most three evaluations -- no reload, no fatal on widening.
pub fn plan_stores(m: &Model) -> Outcome {
    // one load; the four arena variants are planned over it
    let mut arena = [[0u64; 2]; 2];
    for experts in [false, true] {
        for table in [false, true] {
            arena[experts as usize][table as usize] = arenas_for(m, experts, table);
        }
    }
    let budget = effective_budget(m);
    let mut experts = m.cfg_experts;
    let mut table = m.cfg_table;
    let mut evals = 0;
    loop {
        evals += 1;
        let plan = plan_parts(m, arena[experts as usize][table as usize], budget);
        if (experts || !plan.expert_bytes) && (table || !plan.table_bytes) {
            return Outcome::Planned { experts, table, loads: 1, evals };
        }
        if evals >= 3 {
            return Outcome::Fatal("the cycle did not settle in three".to_string());
        }
        experts = experts || plan.expert_bytes;
        table = table || plan.table_bytes;
    }
}

pub fn fixture() -> Model {
    Model {
        budget: 100,
        dense: 40,
        lanes_bytes: 20,
        expert_whole: 30,
        expert_floor: 2,
        table_store: 29,
        expert_staging: 2,
        table_staging: 6,
        base_arenas: 10,
        cfg_experts: false,
        cfg_table: false,
        spill: false,
        cache_reserve: 0,
        keep_cache: false,
    }
}

// -- shared contract tests (both arms) ----------------------------------------

#[test]
fn eq_a_plan_that_flips_nothing_loads_once() {
    let mut m = fixture();
    m.budget = 200;
    match plan_stores(&m) {
        Outcome::Planned { experts, table, loads, evals } => {
            assert!(!experts && !table);
            assert_eq!(loads, 1);
            assert_eq!(evals, 1);
        }
        other => panic!("expected a plan, got {other:?}"),
    }
}

#[test]
fn eq_a_table_configured_from_the_start_never_widens() {
    let mut m = fixture();
    m.cfg_table = true;
    match plan_stores(&m) {
        Outcome::Planned { experts, table, loads, evals } => {
            assert!(experts && table);
            assert_eq!(loads, 1); // no reload: the variants were precomputed
            assert_eq!(evals, 2);
        }
        other => panic!("expected a plan, got {other:?}"),
    }
}

// -- the law's own shape -------------------------------------------------------

#[test]
fn fixed_the_same_widening_input_converges_in_three_evaluations_one_load() {
    // the input that killed the reload arm: the plan streams the table (whose
    // staging grows the arenas), then the experts -- and the precomputed
    // table answers both flips from ONE load, in three evaluations, with a
    // plan, not a fatal.
    let m = fixture();
    match plan_stores(&m) {
        Outcome::Planned { experts, table, loads, evals } => {
            assert!(experts && table, "the widening settles where the arenas hold it");
            assert_eq!(loads, 1);
            assert_eq!(evals, 3);
        }
        other => panic!("the widening input should converge, got {other:?}"),
    }
}

#[test]
fn fixed_a_plan_streams_no_more_than_the_arenas_it_was_made_with() {
    // the monotone law, swept: a part that streams with smaller arenas
    // streams with larger ones, and the converged posture re-plans clean.
    for budget in 60..=200 {
        let mut m = fixture();
        m.budget = budget;
        match plan_stores(&m) {
            Outcome::Planned { experts, table, loads, evals } => {
                assert_eq!(loads, 1);
                assert!(evals <= 3);
                let arenas = arenas_for(&m, experts, table);
                let again = plan_parts(&m, arenas, m.budget);
                assert!(
                    (experts || !again.expert_bytes) && (table || !again.table_bytes),
                    "budget {budget}: the plan was not made with its own arenas"
                );
                // monotone in the arenas: streaming at the small variant
                // holds at the largest one
                let small = plan_parts(&m, arena_small(&m), m.budget);
                let large = plan_parts(&m, arenas_for(&m, true, true), m.budget);
                assert!(!small.expert_bytes || large.expert_bytes);
                assert!(!small.table_bytes || large.table_bytes);
            }
            other => panic!("budget {budget}: expected a plan, got {other:?}"),
        }
    }
}

fn arena_small(m: &Model) -> u64 {
    arenas_for(m, false, false)
}

// -- companion (a): the allowance uses the consumer's own capacity rounding ----

pub fn pages_of(tokens: u64) -> u64 {
    tokens.div_ceil(PAGE_TOKENS)
}

// the consumer's own capacity function: capacity is granted in whole extents
pub fn kv_capacity_pages_for(budget: u64) -> u64 {
    budget / EXTENT_PAGES * EXTENT_PAGES
}

// the fewest pages whose extents hold the context: the search runs through
// the consumer's own rounding, never a padded guess
pub fn allowance_pages(tokens: u64) -> u64 {
    let pages = pages_of(tokens);
    let mut budget = pages;
    while kv_capacity_pages_for(budget) < pages {
        budget += 1;
    }
    budget
}

// THE ANTI-PATTERN: a ceil-by-max-extent fudge pads one largest extent onto
// every request -- over-reserving one extent, and a marginal whole-store fit
// flips to streaming.
pub fn allowance_fudged(tokens: u64) -> u64 {
    pages_of(tokens) + EXTENT_PAGES - 1
}

#[test]
fn companion_the_honest_allowance_holds_a_marginal_fit_the_fudge_flips() {
    // 1024 tokens = 64 pages, granted in extents of 4: the consumer's own
    // rounding admits 64; the fudge reserves 67 -- a whole extent more.
    assert_eq!(pages_of(1024), 64);
    assert_eq!(allowance_pages(1024), 64);
    assert_eq!(allowance_fudged(1024), 67);
    // the KV allowance rounds through its own function when it is not whole
    assert_eq!(allowance_pages(1025), 68); // 65 pages -> the extents that hold them
    // the marginal store: 16 units free after dense + lanes + KV
    let free_after = |kv: u64| 140 - 40 - 20 - kv;
    let expert_whole = 16;
    assert!(free_after(allowance_pages(1024)) >= expert_whole, "the honest allowance holds it");
    assert!(free_after(allowance_fudged(1024)) < expert_whole, "the fudge flips it to streaming");
}

// -- companion (b): the optional cache's reserve gives way ---------------------

#[test]
fn companion_the_cache_reserve_surrenders_where_it_alone_streams_a_part() {
    let mut m = fixture();
    m.budget = 129;
    m.cache_reserve = 10;
    m.keep_cache = true;
    // keeping the reserve streams the table (119 charges it, 29 short of the
    // store); freeing it loads everything whole: the reserve gives way (the
    // tier spills nothing).
    assert_eq!(effective_budget(&m), 129);
    let p = plan_stores(&m);
    assert_eq!(p, Outcome::Planned { experts: false, table: false, loads: 1, evals: 1 });
    // where the plan spills, the tier keeps its staging and the reserve
    // stays charged: the streaming it makes possible is the tier's own.
    m.spill = true;
    assert_eq!(effective_budget(&m), 119);
    // a budget that streams either way keeps the reserve charged: giving it
    // up buys nothing.
    let mut m2 = fixture();
    m2.budget = 99;
    m2.cache_reserve = 10;
    m2.keep_cache = true;
    assert_eq!(effective_budget(&m2), 89);
}
"#.to_string(),
            context_notes: Some("Grade B+ -- NotBenchable ON THIS BOX (the decision content is the CONVERGENCE SHAPE of a plan whose input depends on its own output -- how many model loads a start pays and whether a widening cycle is a plan answer or a fatal; a Rust microbench of the stand-in cannot measure it; the source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ 2f5fe8dd3d4cbd986812beee4652f4d4e0f2d765 'Plan stores with the arenas of what streams and main's KV, in two loads at most', Apache-2.0, Jian Chen, 2026-10-18; every quote re-verified verbatim at the .raw/b271/splash clone this batch at that pin; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (commit message, verbatim): 'The start planned the stores with the arenas of the model as it last loaded, so where the plan changed what streams it loaded again and planned again, up to three loads, and failed with a logic_error where the plan streamed the PLE table, whose staging the arenas then grew.' THE LAW (runtime/engine/MemoryPlan.cpp:363-366 @ 2f5fe8d, verbatim): 'The arenas of the parts set to stream, and where that plan streams more, those of the parts it streams: a part that streams with smaller arenas streams with larger ones, so a plan streams no more than the arenas it was made with after three at most.' THE CONVERGENCE LOOP (MemoryPlan.cpp:367-373, verbatim code): 'bool expertsStream = experts->streams, tableStreams = table->streams; for (;;) { StreamedMemoryResult result = planParts(in, *experts, *table, in.arenaBytes[expertsStream][tableStreams]); if (!result.plan || ((expertsStream || !result.plan->expertBytes) && (tableStreams || !result.plan->tableBytes))) return result; expertsStream = expertsStream || result.plan->expertBytes; tableStreams = tableStreams || result.plan->tableBytes; }' -- each iteration reads the arena input indexed by its own previous output; the flags only ever turn on. THE PRECOMPUTE (runtime/engine/RuntimeResources.mm:817-823 @ 2f5fe8d, verbatim code): 'for (const bool experts : {false, true}) for (const bool table : {false, true}) { const model::ModelMemoryPlan streamed = model::plannedRuntimeMemory(loaded, operators, config.kvFormat, config.stateFormat, {.experts = experts && streamable.expertBytes, .tableRowBytes = table ? streamable.tableRowBytes : 0}); arenaBytes[experts][table] = streamed.sharedPrefillPlannedAllocatedBytes + streamed.sharedDecodePlannedAllocatedBytes; }' -- the 2x2 input table is built ONCE over the one model load, keyed by what can stream. THE MONOTONE PREMISE is why three evaluations bound it: streaming is monotone in the arenas, so the visited postures form a chain ((0,0) -> (0,1) -> (1,1), or (0,0) -> (1,0) -> (1,1)) and no flag ever un-flips. COMPANION (a) -- THE CONSUMER'S OWN CAPACITY ROUNDING (MemoryPlan.cpp:201-204 @ 2f5fe8d, verbatim code): 'const uint64_t pages = pagesOf(kvTokens(tokens)); uint64_t budget = pages; while (kvCapacityPagesFor(layout, budget) < pages) ++budget;' -- a budget's allowance for a resource must round through the CONSUMER'S OWN capacity function (the fewest pages whose extents hold the context), never a padded guess; the defect face (message, verbatim): 'The planner held one largest extent more KV than main's capacity rounds to' -- a ceil-by-max-extent fudge over-reserves one extent and flips a marginal whole-store fit to streaming. COMPANION (b) -- THE OPTIONAL CACHE'S SURRENDERED RESERVE (runtime/engine/RuntimeResources.mm:714-719 @ 2f5fe8d, verbatim): 'The sizing plan with the staging the tier keeps. The automatic tier gives it up where the plan cannot hold the model beside it, and, once the model `loaded` gave the arenas, unless the plan spills, which the tier makes possible, where the staging would stream a part that loads whole without it: a tier that only caches costs the weights nothing, as it costs the context nothing (below).' REGIME: any planner whose input (arena/reserve/cache sizing) depends on the planner's own output (what streams/loads), iterated to a fixed posture; start paths where each re-measurement costs a whole load. ANTI-REGIMES: plans whose input is output-independent (one pass); cycles that widen the input unboundedly (the bounded table needs a finite variant set -- a genuinely divergent cycle is a design bug, not a convergence shape); inputs measurable without a load (re-measuring is only a defect where it is expensive). KINSHIP/DISTINCT-FROM (read + disclosed): dataflow-fixpoint-convergence-traversal-order (B202, entries_16) owns the WORKLIST ORDER for convergence over a CFG (min-rank re-processing so loop-free code converges in a pass); THIS enumerates the input/output cycle's VARIANTS once (a bounded 2x2 keyed table) and reads each iteration's input indexed by its previous output -- no worklist, no ordering, the variants themselves are the input; cite and distinguish (both replace re-measurement with structure). discretionary-cache-growth-keys-on-spare-not-available (B237, entries_26) is the same repo family's growth gate; COMPANION (b) is its drop-when-it-flips-a-load sibling face -- the cache's own reserve yields where keeping it would stream a part that loads whole without it (a tier that only caches costs the weights nothing), unless the plan spills, which the tier makes possible. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'load-plan-reload|variant table|arenaBytes|capacity rounding|extent fudge|kvCapacityPages|re-plan|precompute per variant' -- zero owners; the 're-plan' hits are the graph-reallocation reservation face (kv-dequant-materialization-batch-threshold-gate's companion, gpu_34 -- always reserve the temp buffer so a graph allocator never re-plans; a different re-plan) and 'fixpoint' is entries_16's worklist rule (disclosed above). FIXTURE PROVENANCE: original machinery (never splash code) -- a model whose shared arenas grow with the staging of what streams, a plan that decides whole-vs-stream per part from the budget beside the arenas, the 2x2 precomputed arena table over ONE load, and the at-most-three-evaluation loop; the shared-contract tests (a no-flip plan loads once; a table configured from the start never widens) pass in BOTH arms; the discriminating tests pin the widening input (the fatal logic_error) against the law arm's convergence (three evaluations, one load, a plan not a fatal), the expert-only flip's wasted reload (two loads vs one), the monotone sweep (every converged posture re-plans clean and streaming is monotone in the arenas), and the two companions (the honest allowance holds a marginal fit the fudge flips; the reserve surrenders where it alone streams a part, stays charged under spill or where freeing buys nothing). Arms 5/5 + 6/6 standalone (rustc --edition 2021 --test). Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: @ 2f5fe8dd3d4cbd986812beee4652f4d4e0f2d765 -- runtime/engine/MemoryPlan.cpp:363-373 + :201-204, runtime/engine/RuntimeResources.mm:817-823 + :714-719, commit message (all quotes); Batch 271 / Plan 275.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
