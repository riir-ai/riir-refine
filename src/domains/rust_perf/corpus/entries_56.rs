//! Entries slice - Batch 318 (2026-10-30): the impeccable io stretch-tier
//! remainder (Plan 318, completing Plan 316's honest partial). TWO
//! `rust_perf` rules, both ANNOTATED `NotBenchable` `bench_spec` None
//! (concurrency x1 + io x1):
//!
//! - `generation-counter-stale-timer-invalidation` (concurrency,
//!   pbakaus/impeccable @ `02523aec` Apache-2.0) - a detached timer
//!   thread cannot be cancelled: a reschedule that leaves it armed lets
//!   the superseded timer fire and act on a dead schedule, and a strong
//!   capture holds the owner alive past its drop. The three-part guard:
//!   the generation bumped under the lock on every schedule (the
//!   clearTimeout), captured at arm time, re-checked after the wake
//!   (`timer_active && gen == captured`), the state held through a
//!   `Weak`.
//! - `exclusive-mkdir-temp-resource-reservation` (io) - a timestamp is
//!   not a reservation: concurrent calls observe the same tick, and a
//!   permissive create shares (or a bare create spuriously fails).
//!   Reserve by an exclusive side effect: `create_dir` in a bounded
//!   `AlreadyExists` loop, the name carrying pid + stamp + atomic serial;
//!   a previous process's leftover is skipped, never reused.
//!
//! Every quote is verbatim at the pin (the .raw clone HEAD-equal to
//! `02523aec3e4c4580102d996d1b6a71fbe062e03f` this session). Fixtures are
//! ORIGINAL machinery (never the source pasted), each compiled and
//! executed standalone (rustc --edition 2021 --test) before this table
//! was assembled; the buggy/fixed arm pair shares the surrounding harness
//! and differs only on the law.
use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "generation-counter-stale-timer-invalidation".to_string(),
            name: "a detached timer thread cannot be cancelled, so a reschedule that leaves it armed lets the superseded timer fire anyway and act on a schedule that no longer exists, and a timer that captures its state strongly holds the owner alive until its sleep ends; invalidate in-flight timers with a generation counter bumped under the state lock on every reschedule and cancel (the detached thread's clearTimeout), captured by the timer at arm time, re-checked under the lock after it wakes -- act only if timer_active && generation == captured; hold the shared state through a Weak so a timer outliving its owner fails the upgrade and exits instead of delaying shutdown".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `generation-counter-stale-timer-invalidation`
// (riir-refine rust_perf corpus, Batch 318) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's lease-flush scheduler
// (a live-reload server porting JS timer semantics to threads;
// impeccable crates/live/src/server_state.rs `schedule_lease_flush` is
// cited in context_notes).
//
// The scheduler arms a detached thread per schedule and reschedules by
// spawning another one: nothing invalidates the timers already in
// flight. Two defects, both shown:
// (1) a superseded timer still fires -- the stale schedule acts;
// (2) the timer captures its state STRONGLY, so a board dropped by its
//     owner stays alive until the timer's sleep ends -- shutdown waits
//     on a timer nobody wants any more.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The shared schedule: the lease deadline a flush is armed against.
/// The witness counts the state's Drop runs (the owner-facing fact).
struct BoardState {
    #[allow(dead_code)]
    deadline_ms: u64,
    drop_witness: Arc<AtomicUsize>,
}
impl Drop for BoardState {
    fn drop(&mut self) {
        self.drop_witness.fetch_add(1, Ordering::SeqCst);
    }
}

struct LeaseBoard {
    state: Arc<Mutex<BoardState>>,
    /// Which timers fired, in fire order, tagged by the schedule that
    /// armed them.
    fired: Arc<Mutex<Vec<&'static str>>>,
}

impl LeaseBoard {
    fn new() -> (Self, Arc<AtomicUsize>) {
        let witness = Arc::new(AtomicUsize::new(0));
        let board = Self {
            state: Arc::new(Mutex::new(BoardState {
                deadline_ms: 100,
                drop_witness: Arc::clone(&witness),
            })),
            fired: Arc::new(Mutex::new(Vec::new())),
        };
        (board, witness)
    }

    /// THE DEFECT: arming is bare spawn. No generation, no active flag,
    /// no Weak -- a prior timer is neither cancelled nor invalidated,
    /// and this one pins the state for as long as it sleeps.
    fn schedule_flush(&self, tag: &'static str, delay_ms: u64) {
        let state = Arc::clone(&self.state);
        let fired = Arc::clone(&self.fired);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(delay_ms));
            // The stale timer acts exactly like a current one: it cannot
            // tell that a later schedule replaced it.
            if let Ok(mut st) = state.lock() {
                st.deadline_ms = st.deadline_ms.saturating_sub(1); // the flush work
            }
            fired.lock().unwrap().push(tag);
        });
    }
}

#[test]
fn the_superseded_timer_still_fires_and_acts() {
    let (board, _witness) = LeaseBoard::new();
    // t=0: the scheduler arms a flush for the current schedule.
    board.schedule_flush("armed-first", 300);
    std::thread::sleep(Duration::from_millis(20));
    // t=20: a new lease arrives; the reschedule SUPERSEDES the first
    // timer (its fire-time no longer matches the schedule). The lawful
    // scheduler invalidates the old one; this one just arms again.
    board.schedule_flush("armed-second", 60);
    // t=80: the fresh timer has fired; t=320: the stale one fires too.
    std::thread::sleep(Duration::from_millis(320));
    let fired = board.fired.lock().unwrap().clone();
    assert_eq!(
        fired,
        vec!["armed-second", "armed-first"],
        "the superseded timer fired and acted: two flushes where the \
         schedule owed one, the stale tag acting on a superseded schedule"
    );
}

#[test]
fn the_strong_capture_holds_the_state_past_its_owner() {
    let drop_witness;
    {
        let (board, witness) = LeaseBoard::new();
        drop_witness = witness;
        board.schedule_flush("late-timer", 150);
        // The owner drops the whole board here.
    }
    std::thread::sleep(Duration::from_millis(40));
    // THE DEFECT: the timer's STRONG Arc keeps the state alive -- the
    // Drop has not run 40 ms after the owner let go, so a server's
    // shutdown waits for a timer it no longer wants.
    assert_eq!(
        drop_witness.load(Ordering::SeqCst),
        0,
        "the state survived its owner: the detached timer's strong \
         capture delays the Drop until the timer's sleep ends"
    );
    std::thread::sleep(Duration::from_millis(160));
    // Only after the timer fires and its Arcs die does the Drop run.
    assert_eq!(drop_witness.load(Ordering::SeqCst), 1);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `generation-counter-stale-timer-invalidation`
// (riir-refine rust_perf corpus, Batch 318) -- the REMEDY arm.
//
// Same LeaseBoard, same scenarios -- but the timer carries the source's
// three-part guard:
// (1) INVALIDATION: every schedule bumps a generation counter under the
//     state lock (a spawned thread cannot be cancelled, so the counter
//     IS the clearTimeout);
// (2) RE-CHECK: the timer captures its generation at arm time and after
//     waking re-locks and acts only if the schedule is still current
//     (active && generation == captured);
// (3) WEAK: the timer holds the state through a Weak -- an owner that
//     drops first makes the upgrade fail and the timer exit, so nothing
//     is kept alive by a timer and no shutdown waits on a sleep.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct BoardState {
    #[allow(dead_code)]
    deadline_ms: u64,
    /// Bumped under the lock on EVERY schedule -- any timer armed for an
    /// earlier generation is stale by construction.
    generation: u64,
    /// Cleared at the top of every schedule (the clearTimeout), set for
    /// the timer this schedule arms.
    timer_active: bool,
    drop_witness: Arc<AtomicUsize>,
}
impl Drop for BoardState {
    fn drop(&mut self) {
        self.drop_witness.fetch_add(1, Ordering::SeqCst);
    }
}

struct LeaseBoard {
    state: Arc<Mutex<BoardState>>,
    fired: Arc<Mutex<Vec<&'static str>>>,
}

impl LeaseBoard {
    fn new() -> (Self, Arc<AtomicUsize>, Arc<Mutex<Vec<&'static str>>>) {
        let witness = Arc::new(AtomicUsize::new(0));
        let fired = Arc::new(Mutex::new(Vec::new()));
        let board = Self {
            state: Arc::new(Mutex::new(BoardState {
                deadline_ms: 100,
                generation: 0,
                timer_active: false,
                drop_witness: Arc::clone(&witness),
            })),
            fired: Arc::clone(&fired),
        };
        (board, witness, fired)
    }

    fn schedule_flush(&self, tag: &'static str, delay_ms: u64) {
        let captured = {
            let mut st = self.state.lock().unwrap();
            // (1) the clearTimeout: invalidate every timer armed before
            // this line, then mark the schedule live.
            st.generation += 1;
            st.timer_active = true;
            st.deadline_ms = 100; // the recomputed schedule
            st.generation // (2) capture AFTER the bump, under the lock
        };
        // (3) the timer holds the state WEAKLY.
        let state = Arc::downgrade(&self.state);
        let fired = Arc::clone(&self.fired);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(delay_ms));
            let Some(state) = state.upgrade() else {
                return; // the owner is gone: exit, never resurrect it
            };
            let mut st = state.lock().unwrap();
            if st.timer_active && st.generation == captured {
                st.timer_active = false;
                st.deadline_ms = st.deadline_ms.saturating_sub(1); // the flush work
                fired.lock().unwrap().push(tag);
            }
            // A stale timer (the generation moved on) wakes, reads the
            // mismatch, and exits WITHOUT acting.
        });
    }
}

#[test]
fn the_superseded_timer_wakes_and_stays_inert() {
    let (board, _witness, _fired_probe) = LeaseBoard::new();
    board.schedule_flush("armed-first", 300);
    std::thread::sleep(Duration::from_millis(20));
    board.schedule_flush("armed-second", 60);
    std::thread::sleep(Duration::from_millis(320));
    let fired = board.fired.lock().unwrap().clone();
    assert_eq!(
        fired,
        vec!["armed-second"],
        "the stale timer woke, read generation 1 != current 2, and did \
         not act: exactly the fresh schedule's flush happened"
    );
}

#[test]
fn the_weak_capture_lets_the_state_drop_with_its_owner() {
    let drop_witness;
    let fired_probe;
    {
        let (board, witness, fired) = LeaseBoard::new();
        drop_witness = witness;
        fired_probe = fired;
        board.schedule_flush("late-timer", 150);
        // The owner drops the whole board here.
    }
    std::thread::sleep(Duration::from_millis(40));
    // THE REMEDY: the Weak fails the upgrade -- the state dropped WITH
    // its owner, nothing waits on the timer's sleep.
    assert_eq!(
        drop_witness.load(Ordering::SeqCst),
        1,
        "the state dropped with its owner: the timer's Weak capture \
         cannot hold it past the drop"
    );
    std::thread::sleep(Duration::from_millis(160));
    // The timer woke past the owner's death, upgraded to nothing, and
    // exited: no flush, no resurrection, no panic.
    assert!(
        fired_probe.lock().unwrap().is_empty(),
        "the orphaned timer acted on nothing: no upgrade, no flush"
    );
}
"#.to_string(),
            context_notes: Some("Grade B- -- NotBenchable ON THIS BOX (the decision content is a timer-staleness LIVENESS law -- which in-flight timer may still act after a reschedule supersedes it, and which reference keeps the owner alive -- priced in a long-lived server's behavior under reschedules and shutdown, not in any A/B microbench; the failure is a stale double-act or a delayed shutdown, binary not us; a stand-in benchmark cannot measure staleness at all). SOURCE (pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f, Apache-2.0; quotes verified verbatim in the .raw clone HEAD-equal to the pin this session; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT -- the source ports JS timer semantics to detached threads, and JS rescheduling CANCELS the prior timer; a spawned std::thread has no cancellation, so the port must supply the clearTimeout itself. THE LAW, three parts (crates/live/src/server_state.rs @ 02523aec, schedule_lease_flush): (1) THE INVALIDATION -- every schedule bumps the generation under the state lock BEFORE arming, body verbatim: '// clearTimeout' + 'self.lease_timer_gen += 1;' + 'self.lease_timer_active = false;' -- every timer armed for an earlier generation is stale by construction; (2) THE RE-CHECK -- the timer captures its generation at arm time AFTER the bump ('let gen = self.lease_timer_gen;'), and after waking re-locks and acts only if the schedule is still current, verbatim: 'if st.lease_timer_active && st.lease_timer_gen == gen {' -- a superseded timer wakes, reads the mismatch, and exits WITHOUT acting; (3) THE WEAK -- the timer holds the shared state through 'self.self_ref.clone()' where the field is 'self_ref: Weak<Mutex<ServerState>>', so a timer outliving its owner fails the upgrade and exits: nothing is resurrected, and no shutdown is delayed until a timer's sleep ends. The pattern is the file's timer DISCIPLINE, not a one-off: four generation counters (exit_timer_gen, lease_timer_gen, next_agent_target_timer_gen, next_apply_timer_gen) guard the same file's detached-timer sites. HOME ADJUDICATION (this session): no detached-timer reschedule site in this repo's src/ (grep 'timer_gen|generation.*timer|stale.*timer' over src/ returns corpus text only); the class lives in servers that port event-loop timer semantics to threads -- live-reload servers, lease/timeout schedulers, debounced saves, retry re-arms. REGIME: any detached timer/closure armed against shared mutable state a LATER schedule can supersede (lease flush, debounce, idle commit, backoff re-arm); every thread::spawn-with-sleep whose body locks and acts. ANTI-REGIMES: timers with real cancellation (an AbortHandle, a Condvar-woken worker, a timer wheel that owns the handle -- cancel directly, the counter is redundant); single-shot timers that can never be superseded; acts that are idempotent AND order-free (a stale fire is harmless -- say so at the site); timers whose state the timer exclusively owns (nothing shared to go stale). KINSHIP/DISTINCT-FROM (read + disclosed): certifying-identity-token-revalidation-skip (B283, entries_49) owns the input-unchanged skip -- a token certifies the INPUT is unchanged, a generation certifies the SCHEDULE is still current; both are prove-current-before-act guards over different staleness axes and compose; arrival-order-turnstile-lane (B316, entries_55, the same source repo) owns ARRIVAL-ORDER serialization of state mutations -- THIS owns timer-staleness cancellation, a different concurrency axis from the same source; watchdog-deadline-moves-on-command-progress (B270, entries_41) owns a progress deadline on commands, not timer invalidation; the kernel_opt generation vocabulary (per-generation-plan-table-unmeasured-generation-fallback, gpu_98) is GPU-family calibration -- unrelated. NOVEL: grep 'timer_gen|generation.*timer|stale.*timer' over src/domains/ at the landing HEAD: zero corpus rule owns timer invalidation; 'Weak' appears in zero rust_perf rules. Suggestion-only (the defect is a MISSING guard on a spawned closure -- arm-time capture vs wake-time check -- invisible to a parse-only proposer). FIXTURE PROVENANCE: original machinery (never impeccable code) -- a LeaseBoard stand-in (a lease deadline + a flush log tagged by the arming schedule + a Drop witness), the buggy arm's schedule (bare spawn, strong capture: test 1 the superseded timer still fires and the flush log carries the stale tag; test 2 the state survives its owner until the timer's sleep ends), the fixed arm's schedule (gen bump under the lock + capture + active-flag re-check + Weak: the same supersede leaves exactly the fresh flush; the owner's drop runs the Drop immediately and the orphaned timer exits without acting); both arms share the harness and differ only on the law; every arm standalone-compiled AND executed (rustc --edition 2021 --test, 2 passed per arm, timing-stable across repeated runs) before this table was assembled. Cite: pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f -- crates/live/src/server_state.rs (schedule_lease_flush: the clearTimeout bump, the captured gen, the 'if st.lease_timer_active && st.lease_timer_gen == gen' re-check, self_ref: Weak<Mutex<ServerState>>); Batch 318 / Plan 318 / Research 356.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "exclusive-mkdir-temp-resource-reservation".to_string(),
            name: "a timestamp is not a reservation: concurrent calls can observe the same tick, so a temp-workdir name derived from the clock alone is shared by every caller in that tick -- create_dir_all silently gives both callers the same directory and their files clobber, create_dir makes one caller fail with a spurious AlreadyExists; reserve by an EXCLUSIVE side effect instead: create_dir in a bounded loop, on AlreadyExists retry with the next atomic serial, the name carrying pid + stamp + serial so two callers can never derive the same path -- and a name a previous process left behind is skipped, never reused with its stale contents".to_string(),
            category: "io".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `exclusive-mkdir-temp-resource-reservation`
// (riir-refine rust_perf corpus, Batch 318) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's browser-profile
// reservation (impeccable crates/browser/src/cdp.rs `reserve_profile` is
// cited in context_notes). A fixed injected clock stands in for "the
// same tick": both callers observe the identical stamp.
//
// The timestamp-only reservation derives the whole name from the clock
// and creates it permissively (create_dir_all): two callers in one tick
// get ONE directory, both own it, their files clobber each other -- and
// a directory a previous process left behind is silently reused with
// all its stale contents.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Scratch root unique to this process (the house law: a fixed temp
/// path is shared by any other process running the same test).
fn scratch_root(tag: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("b318_stamp_only_{}_{n}_{tag}", std::process::id()))
}

/// THE DEFECT: the name is the clock's, the create is permissive.
fn reserve_work_area(root: &Path, stamp: u64) -> std::io::Result<PathBuf> {
    let dir = root.join(format!("work-{stamp}"));
    fs::create_dir_all(&dir)?; // succeeds on an EXISTING dir too
    Ok(dir)
}

/// A fixed clock: every caller in this "tick" observes the same stamp --
/// exactly the concurrency the source names.
fn the_same_tick() -> u64 {
    1_733_000_000_000
}

#[test]
fn two_callers_in_one_tick_share_one_directory_and_clobber() {
    let root = scratch_root("clobber");
    let a = reserve_work_area(&root, the_same_tick()).unwrap();
    let b = reserve_work_area(&root, the_same_tick()).unwrap();
    // THE COLLISION: both calls "succeeded" -- into the SAME directory.
    assert_eq!(
        a, b,
        "the timestamp is not a reservation: both callers observed the \
         same tick and reserved the same directory"
    );
    // Two owners, one dir: the second write destroys the first.
    fs::write(a.join("result.txt"), "owner A's output").unwrap();
    fs::write(b.join("result.txt"), "owner B's output").unwrap();
    let left = fs::read_to_string(a.join("result.txt")).unwrap();
    assert_eq!(
        left, "owner B's output",
        "owner A's file was clobbered: a shared reservation is no \
         reservation"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_leftover_directory_from_a_previous_run_is_silently_reused() {
    let root = scratch_root("reuse");
    // A previous process died here and left its stamp-named dir behind.
    let stale = root.join(format!("work-{}", the_same_tick()));
    fs::create_dir_all(&stale).unwrap();
    fs::write(stale.join("stale-cache.bin"), b"previous run's bytes").unwrap();
    // This process reserves: same clock reading, permissive create --
    // it gets the STALE directory and inherits everything in it.
    let reserved = reserve_work_area(&root, the_same_tick()).unwrap();
    assert_eq!(
        reserved, stale,
        "the leftover dir was handed out again"
    );
    assert!(
        stale.join("stale-cache.bin").exists(),
        "the stale contents were inherited: exclusive create would have \
         skipped this name"
    );
    let _ = fs::remove_dir_all(&root);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `exclusive-mkdir-temp-resource-reservation`
// (riir-refine rust_perf corpus, Batch 318) -- the REMEDY arm.
//
// Same WorkArea reservation, same scenarios -- but the claim is made by
// an EXCLUSIVE side effect (create_dir fails if the name exists), the
// name carries all three uniqueness components (pid + stamp + an atomic
// serial), and a collision retries with the NEXT serial instead of
// failing or sharing. A name a previous process left behind is skipped,
// never reused.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

fn scratch_root(tag: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("b318_exclusive_{}_{n}_{tag}", std::process::id()))
}

/// The process-lifetime serial: never repeats within a process, so two
/// callers can never derive the same name even at one tick.
static NEXT_SERIAL: AtomicU64 = AtomicU64::new(0);

fn reserve_work_area(root: &Path, stamp: u64) -> std::io::Result<PathBuf> {
    for _ in 0..128 {
        let serial = NEXT_SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = root.join(format!("work-{}-{stamp}-{serial}", std::process::id()));
        match fs::create_dir(&path) {
            // The exclusive claim: this caller OWNED the create.
            Ok(()) => return Ok(path),
            // The name was taken (a concurrent caller or a leftover):
            // try the next serial -- never share, never reuse.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::other("reservation exhausted"))
}

fn the_same_tick() -> u64 {
    1_733_000_000_000
}

#[test]
fn eight_concurrent_callers_at_one_tick_get_eight_distinct_dirs() {
    let root = scratch_root("distinct");
    // The caller precondition (the source's parent is an existing temp
    // dir): the reservation creates the LEAF, never the chain.
    fs::create_dir_all(&root).unwrap();
    let mut joins = Vec::new();
    for t in 0..8u32 {
        let root = root.clone();
        joins.push(std::thread::spawn(move || {
            let dir = reserve_work_area(&root, the_same_tick()).unwrap();
            // Each owner's marker: with a shared dir these would clobber.
            fs::write(dir.join(format!("owner-{t}.txt")), t.to_string()).unwrap();
            (t, dir)
        }));
    }
    let mut results = Vec::new();
    for j in joins {
        results.push(j.join().unwrap());
    }
    for (t, dir) in &results {
        assert!(
            dir.join(format!("owner-{t}.txt")).exists(),
            "owner {t}'s marker survived: the dirs are distinct"
        );
    }
    let mut dirs: Vec<PathBuf> = results.into_iter().map(|(_, d)| d).collect();
    dirs.sort();
    let before = dirs.len();
    dirs.dedup();
    assert_eq!(
        before,
        dirs.len(),
        "every caller reserved a DISTINCT dir at the SAME stamp"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_leftover_directory_is_skipped_never_reused() {
    let root = scratch_root("skip");
    fs::create_dir_all(&root).unwrap();
    // What a previous process would have left behind: the exact name
    // this process's NEXT serial would derive (the leftover's own
    // serial space is gone; the collision is made deterministic by
    // pre-creating the name the next fetch_add will produce).
    let next = NEXT_SERIAL.load(Ordering::Relaxed);
    let stale = root.join(format!(
        "work-{}-{}-{next}",
        std::process::id(),
        the_same_tick()
    ));
    fs::create_dir_all(&stale).unwrap();
    fs::write(stale.join("stale-cache.bin"), b"previous run's bytes").unwrap();
    // This process reserves: the serial collides with the leftover --
    // the loop takes the NEXT serial instead of handing out the stale
    // dir.
    let reserved = reserve_work_area(&root, the_same_tick()).unwrap();
    assert_ne!(
        reserved, stale,
        "the leftover dir was skipped, not reused"
    );
    assert!(
        stale.join("stale-cache.bin").exists(),
        "the stale contents are untouched"
    );
    assert!(
        !reserved.join("stale-cache.bin").exists(),
        "the new dir is empty"
    );
    let _ = fs::remove_dir_all(&root);
}
"#.to_string(),
            context_notes: Some("Grade B- -- NotBenchable ON THIS BOX (the decision content is a RESERVATION law -- what makes a claimed name exclusive under concurrency and across processes -- priced in a multi-caller work-dir's behavior, not in any A/B microbench; the failure is a collision or a clobber, binary not us). SOURCE (pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f, Apache-2.0; quotes verified verbatim in the .raw clone HEAD-equal to the pin this session; inner double quotes rendered as single quotes per house string-literal safety). THE LAW is the comment itself, verbatim, two sentences (crates/browser/src/cdp.rs @ 02523aec, reserve_profile): 'A timestamp is not a reservation: concurrent calls can observe the same tick.' + 'Exclusive mkdir also avoids reusing a stale profile from a previous process.' THE SHAPE (the body, facts): a module-level 'static NEXT: AtomicU64', a bounded 128-attempt loop, the serial from 'NEXT.fetch_add(1, Ordering::Relaxed)', the name carrying all three uniqueness components (prefix + std::process::id() + the stamp + the serial), and std::fs::create_dir as the EXCLUSIVE claim: Ok = this caller owned the create; 'Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue' (try the next serial -- never share, never reuse); any other error is real and returned; exhaustion returns an error. The second sentence's face: exclusive create means a name a PREVIOUS process left behind is skipped, never handed out -- a permissive create (create_dir_all) succeeds onto the stale directory and inherits its contents. THE DEFECT -- the timestamp-only posture: a name derived from the clock alone is identical for every caller in the same tick; with create_dir_all the second caller succeeds INTO the same directory (two owners, files clobber silently); with create_dir it fails with a spurious AlreadyExists (an availability bug that only fires under concurrent or same-tick callers -- it reads as 'works on my machine'). HOME ADJUDICATION (this session): no product site in this repo derives temp names from a bare stamp (grep 'AlreadyExists|create_dir|mkdir' over src/: test infrastructure only); this repo's own fixture discipline already carries the pid-suffix law (the shared-temp-path gate class -- scratch named with process::id()), which is this rule's single-caller face; the CONCURRENCY face (two callers, one tick) is what the corpus adds. REGIME: temp workdirs/profiles/session dirs reserved by servers, test harnesses, or tools whose callers can be concurrent, repeat within one tick, or follow an earlier crashed process. ANTI-REGIMES: single-caller once-per-process names (the clock cannot collide with itself -- the pid suffix suffices, say so at the site); uniqueness from a real randomness source with collision odds priced (a v7-uuid name -- a different mechanism, the same law; the exclusive create is what makes ANY id a reservation, entropy only prices the retries); names claimed through a higher-level exclusive API (an O_EXCL lock file -- the same side-effect shape on a file); paths the caller owns exclusively. KINSHIP/DISTINCT-FROM (read + disclosed): no corpus rule owns temp-name reservation (grep 'AlreadyExists|create_dir|mkdir|reserve.*dir' over src/domains/: zero owners -- the two create_dir_all hits are test infrastructure); the house Uuid::now_v7 rule is the id-entropy face -- compose (v7 reduces retries, the exclusive mkdir makes the claim exclusive); the seal-game-editor shared_temp_path gate is the process-isolation face of the same hygiene. NOVEL: grep at the landing HEAD: zero owners. Suggestion-only (needs the name derivation AND the create call together -- a parse-only proposer cannot see the reservation contract). FIXTURE PROVENANCE: original machinery (never impeccable code) -- a WorkArea reservation over a pid-unique scratch root with an injected fixed clock (the same-tick stand-in), the buggy arm's stamp-only reserve with create_dir_all (test 1: two callers in one tick both succeed to the SAME path and the second write clobbers the first; test 2: a previous run's leftover dir is handed out again with its stale contents inherited), the fixed arm's exclusive reserve (test 1: eight concurrent callers at one stamp get eight DISTINCT dirs, every marker survives; test 2: the leftover name is deterministically collided -- pre-created at the serial the next fetch_add will produce -- and skipped, contents untouched); both arms share the harness and differ only on the law; every arm standalone-compiled AND executed (rustc --edition 2021 --test, 2 passed per arm) before this table was assembled. Cite: pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f -- crates/browser/src/cdp.rs (reserve_profile: the two-sentence law comment, static NEXT AtomicU64, the 128-attempt AlreadyExists loop, the pid+stamp+serial name); Batch 318 / Plan 318 / Research 356.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
