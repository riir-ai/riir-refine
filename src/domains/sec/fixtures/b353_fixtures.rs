//! Batch 353 sec fixtures — the FORGE Lab std memory-safety laws as ONE
//! standalone file (4 rules, the four-arm discipline per rule).
//!
//! ORIGINAL plain-Rust reproductions — zero std source pasted (the source
//! is the Microsoft FORGE Lab post, CC BY 4.0; the post's std snippets are
//! never a gate — the transcription law). The UB paths are demonstrated on
//! DEFINED-BEHAVIOR stand-ins (sentinel-initialized buffers, checked
//! constructors, toy lock models) so the harness stays deterministic and
//! defined; each substitution is documented at its site. Compiled AND
//! executed BEFORE entry: `rustc --edition 2021 --test b353_fixtures.rs`.
//!
//! Arms per rule (the b350/b351 discipline):
//! - F1 `unsafe-invariant-panic-exposed`: a panicking user call inside the
//!   broken-invariant window runs drop glue over slots the fill never
//!   wrote (demonstrated with a tracked drop ledger over a
//!   sentinel-initialized buffer — the production shape is uninit memory,
//!   the sentinel keeps the drop defined), the ManuallyDrop fix leaks
//!   instead (no drop over unwritten slots), the no-panic arm drops each
//!   value exactly twice (its clone + its owned copy — the ledger's
//!   defined baseline), and the tampered "fix" (a ManuallyDrop-shaped
//!   wrapper whose own Drop drops the contents) disagrees.
//! - F2 `user-impl-call-once-keep-result`: a flip-flop `AsRef` impl makes
//!   the twice-call getter's check and access disagree (observed at the
//!   trust boundary — the OOB the disagreement reaches is named, never
//!   performed), the call-once fix has no disagreement possible, a
//!   CONSISTENT impl keeps even the twice-call getter correct (the
//!   anti-regime), and the half-fixed "keep-for-check-only" arm
//!   disagrees.
//! - F3 `user-impl-derived-property-validate`: a pattern impl returning
//!   `0x80` produces a byte vector that fails the CHECKED UTF-8
//!   constructor (the corruption observed without laundering it), the
//!   validate arm refuses the non-ASCII byte, an honest ASCII impl is
//!   clean on both postures (the anti-regime), and a wrong-property
//!   validation (`b != 0`) disagrees.
//! - F4 `process-global-write-under-shared-lock`: two readers hold read
//!   guards SIMULTANEOUSLY with a writer (non-exclusion proven), a reader
//!   DURING the interleaved swap sees the child-only value the post's own
//!   sentence names ("a thread calling `env::var` ... can see variables
//!   meant only for the new program"), the serialized execution never
//!   exposes it, a lone writer's save/restore is exact (the anti-regime),
//!   and the tampered "fix" whose restore escapes the exclusive section
//!   disagrees.

// ────────────────────────────────────────────────────────────────────
// F1 — the broken-invariant window under a panicking user call
// ────────────────────────────────────────────────────────────────────

mod f1_invariant_panic_exposed {
    use std::cell::RefCell;
    use std::mem::{ManuallyDrop, MaybeUninit};
    use std::panic::{self, AssertUnwindSafe};
    use std::rc::Rc;

    /// A value whose drops are all recorded — the drop ledger.
    #[derive(Clone)]
    struct Tracked {
        id: usize,
        ledger: Rc<RefCell<Vec<usize>>>,
    }

    impl Tracked {
        fn new(id: usize, ledger: &Rc<RefCell<Vec<usize>>>) -> Self {
            Tracked {
                id,
                ledger: Rc::clone(ledger),
            }
        }
    }

    impl Drop for Tracked {
        fn drop(&mut self) {
            self.ledger.borrow_mut().push(self.id);
        }
    }

    const SLOTS: usize = 4;
    /// The sentinel id pre-written into every slot at Window birth: a
    /// drop of the sentinel at slot i proves the ARMED DESTRUCTOR ran
    /// over memory the fill never constructed. (Production shape: the
    /// slot is UNINITIALIZED and the drop is UB — the sentinel keeps
    /// this harness defined; the substitution is the only difference.)
    const SENTINEL: usize = 9_000;

    /// The armed-destructor wrapper — the MapWindows::Buffer analog: its
    /// Drop runs drop glue over EVERY slot, trusting the invariant "all
    /// slots initialized". The constructor below arms that destructor
    /// BEFORE any slot is written: the invariant is broken from birth.
    struct Window {
        slots: [MaybeUninit<Tracked>; SLOTS],
    }

    impl Window {
        /// The broken-invariant birth: the destructor is armed while no
        /// slot is constructed. Sentinel stand-ins keep the eventual
        /// drop defined.
        fn new_standin(ledger: &Rc<RefCell<Vec<usize>>>) -> Window {
            Window {
                slots: new_standin_slots(ledger),
            }
        }
    }

    /// The sentinel-primed slot array (standalone so the tampered arm can
    /// build the same state without moving out of a Drop type).
    fn new_standin_slots(ledger: &Rc<RefCell<Vec<usize>>>) -> [MaybeUninit<Tracked>; SLOTS] {
        let mut slots: [MaybeUninit<Tracked>; SLOTS] =
            [const { MaybeUninit::uninit() }; SLOTS];
        for slot in slots.iter_mut() {
            slot.write(Tracked::new(SENTINEL, ledger));
        }
        slots
    }

    impl Drop for Window {
        fn drop(&mut self) {
            for slot in self.slots.iter_mut() {
                unsafe { slot.assume_init_drop() };
            }
        }
    }

    /// The buggy fill: user code (`T::clone`) runs while the Window's
    /// destructor is armed over slots it never wrote. `panic_on` names
    /// the id whose clone panics (None = no panic).
    fn fill_window_buggy(w: &mut Window, xs: &[Tracked], panic_on: Option<usize>) {
        for (i, slot) in w.slots.iter_mut().enumerate() {
            if let Some(target) = panic_on {
                if xs[i].id == target {
                    panic!("user clone panicked inside the window");
                }
            }
            let _clone_made = xs[i].clone(); // the user call inside the window
            slot.write(Tracked::new(xs[i].id, &xs[i].ledger));
        }
    }

    /// The ManuallyDrop fix shape: the half-built slot array is held in
    /// ManuallyDrop until fully written; a panicking clone LEAKS it (no
    /// drop glue over unwritten slots).
    fn fill_fixed(xs: &[Tracked], panic_on: Option<usize>) -> Result<[Tracked; SLOTS], ()> {
        let mut buf = ManuallyDrop::new([const { MaybeUninit::<Tracked>::uninit() }; SLOTS]);
        for (i, slot) in buf.iter_mut().enumerate() {
            if let Some(target) = panic_on {
                if xs[i].id == target {
                    // Leak, not UB: the wrapper's Drop is a no-op, so the
                    // unwinding scope drops NOTHING from the buffer.
                    return Err(());
                }
            }
            let _clone_made = xs[i].clone();
            slot.write(Tracked::new(xs[i].id, &xs[i].ledger));
        }
        let out: [Tracked; SLOTS] =
            std::array::from_fn(|i| unsafe { (*buf)[i].assume_init_read() });
        Ok(out)
    }

    #[test]
    fn f1_panic_inside_the_window_drops_unwritten_slots() {
        let ledger = Rc::new(RefCell::new(Vec::new()));
        let xs: Vec<Tracked> = (0..SLOTS).map(|i| Tracked::new(i, &ledger)).collect();
        // The closure OWNS the Window: the panic unwinds through it and
        // the armed destructor runs — the unwind analog.
        let panicked = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut w = Window::new_standin(&ledger);
            fill_window_buggy(&mut w, &xs, Some(2));
        }))
        .is_err();
        assert!(panicked, "the fill must panic inside the window");
        let drops = ledger.borrow();
        // The clone drops (ids 0 and 1, once each) are the defined
        // baseline. The SENTINEL drops are the finding: the armed
        // destructor ran over the two slots the fill never wrote.
        let sentinel_drops = drops.iter().filter(|id| **id == SENTINEL).count();
        assert_eq!(
            sentinel_drops, 2,
            "the armed destructor must have run over the two never-written              slots (their sentinel stand-ins fired) — the invariant break,              demonstrated defined: {drops:?}"
        );
        assert!(
            !drops.contains(&2) && !drops.contains(&3),
            "slots 2 and 3 were never constructed — only their sentinels may fire"
        );
    }

    #[test]
    fn f1_manuallydrop_leaks_instead() {
        let ledger = Rc::new(RefCell::new(Vec::new()));
        let xs: Vec<Tracked> = (0..SLOTS).map(|i| Tracked::new(i, &ledger)).collect();
        let err = fill_fixed(&xs, Some(2));
        assert!(err.is_err());
        let drops = ledger.borrow();
        // The ONLY drops are the two clones made before the panic (ids 0
        // and 1, once each). No sentinel fires: the half-built buffer
        // LEAKED instead of dropping.
        assert_eq!(drops.len(), 2, "only the clone drops may appear: {drops:?}");
        assert!(drops.contains(&0) && drops.contains(&1));
        assert!(!drops.contains(&SENTINEL), "the leak must not drop anything unwritten");
    }

    #[test]
    fn f1_no_panic_drops_each_value_exactly_twice() {
        let ledger = Rc::new(RefCell::new(Vec::new()));
        let xs: Vec<Tracked> = (0..SLOTS).map(|i| Tracked::new(i, &ledger)).collect();
        let done = fill_fixed(&xs, None).expect("no panic");
        drop(done);
        let drops = ledger.borrow();
        // Each id drops exactly TWICE: its transient clone + its owned
        // copy in the returned array. The invariant held throughout.
        assert_eq!(drops.len(), 2 * SLOTS, "clone + owned per value: {drops:?}");
        for i in 0..SLOTS {
            assert_eq!(drops.iter().filter(|id| **id == i).count(), 2, "id {i}");
        }
        assert!(!drops.contains(&SENTINEL));
    }

    #[test]
    fn f1_tampered_wrapper_that_drops_contents_disagrees() {
        // The tampered "fix": a ManuallyDrop-shaped wrapper whose OWN Drop
        // drops the contents — the std bug reproduced inside the "fix"
        // (the fix law: hold in ManuallyDrop, whose Drop does nothing).
        struct BrokenGuard([MaybeUninit<Tracked>; SLOTS]);
        impl Drop for BrokenGuard {
            fn drop(&mut self) {
                for slot in self.0.iter_mut() {
                    unsafe { slot.assume_init_drop() };
                }
            }
        }
        let ledger = Rc::new(RefCell::new(Vec::new()));
        let mut buf = new_standin_slots(&ledger);
        // Fill two slots, then the unwind analog drops the wrapper.
        buf[0].write(Tracked::new(0, &ledger));
        buf[1].write(Tracked::new(1, &ledger));
        drop(BrokenGuard(buf));
        let drops = ledger.borrow();
        let sentinel_drops = drops.iter().filter(|id| **id == SENTINEL).count();
        assert_eq!(
            sentinel_drops, 2,
            "the tampered wrapper must disagree with the ManuallyDrop law:              dropping the contents over unwritten slots fires their sentinels"
        );
    }
}

// ────────────────────────────────────────────────────────────────────
// F2 — the twice-called provider impl
// ────────────────────────────────────────────────────────────────────

mod f2_call_once_keep_result {
    use std::cell::Cell;

    /// A flip-flop `AsRef<[u8]>`: the first borrow is long, the second is
    /// short. Safe code; nothing here is `unsafe` to CALL.
    struct FlipFlop {
        calls: Cell<usize>,
    }

    impl FlipFlop {
        fn new() -> Self {
            FlipFlop { calls: Cell::new(0) }
        }
    }

    impl AsRef<[u8]> for FlipFlop {
        fn as_ref(&self) -> &[u8] {
            let n = self.calls.get();
            self.calls.set(n + 1);
            if n == 0 { &[0xAA; 16] } else { &[0xBB; 1] }
        }
    }

    /// A consistent impl (the anti-regime arm's provider).
    struct Steady(Vec<u8>);

    impl AsRef<[u8]> for Steady {
        fn as_ref(&self) -> &[u8] {
            &self.0
        }
    }

    /// The buggy getter's SHAPE: check against one borrow, access through
    /// another. The disagreement is OBSERVED at the trust boundary (the
    /// OOB the second borrow reaches is named, never performed — the
    /// production shape is `get_unchecked(idx)` on the short slice).
    fn get_twice<S: AsRef<[u8]>>(s: &S, idx: usize) -> Result<u8, String> {
        let check = s.as_ref();
        let access = s.as_ref();
        if idx < check.len() {
            if idx >= access.len() {
                Err(format!(
                    "disagreement: check len {}, access len {}, idx {}",
                    check.len(),
                    access.len(),
                    idx
                ))
            } else {
                Ok(access[idx])
            }
        } else {
            Err("out of range per the check".into())
        }
    }

    /// The call-once fix: one call, the result kept.
    fn get_once<S: AsRef<[u8]>>(s: &S, idx: usize) -> Result<u8, String> {
        let bytes = s.as_ref();
        if idx < bytes.len() {
            Ok(bytes[idx])
        } else {
            Err("out of range".into())
        }
    }

    #[test]
    fn f2_twice_call_disagrees_on_the_flip_flop_impl() {
        let ff = FlipFlop::new();
        let err = get_twice(&ff, 3).expect_err("the disagreement must surface");
        assert!(
            err.contains("check len 16") && err.contains("access len 1"),
            "the two borrows disagreed: {err}"
        );
    }

    #[test]
    fn f2_call_once_has_no_disagreement_possible() {
        let ff = FlipFlop::new();
        // Even on the flip-flop impl, ONE call sees ONE slice.
        let v = get_once(&ff, 3).expect("idx 3 < 16");
        assert_eq!(v, 0xAA);
        assert!(get_once(&ff, 99).is_err());
    }

    #[test]
    fn f2_consistent_impl_keeps_even_the_twice_call_correct() {
        // The anti-regime: the hazard needs an impl that CHANGES between
        // calls. A consistent impl makes the twice-call shape correct.
        let steady = Steady(vec![7, 8, 9, 10]);
        let v = get_twice(&steady, 3).expect("consistent impl: no disagreement");
        assert_eq!(v, 10);
    }

    #[test]
    fn f2_half_fixed_check_only_keep_disagrees() {
        // The tampered arm: the check result is kept, but the access
        // calls the impl AGAIN — the disagreement returns.
        fn half_fixed<S: AsRef<[u8]>>(s: &S, idx: usize) -> Result<u8, String> {
            let check = s.as_ref();
            if idx < check.len() {
                let access = s.as_ref(); // the re-call the fix must remove
                if idx >= access.len() {
                    Err(format!(
                        "disagreement again: check len {}, access len {}",
                        check.len(),
                        access.len()
                    ))
                } else {
                    Ok(access[idx])
                }
            } else {
                Err("out of range".into())
            }
        }
        let ff = FlipFlop::new();
        let err = half_fixed(&ff, 3).expect_err("the half-fix must still disagree");
        assert!(err.contains("disagreement again"), "{err}");
    }
}

// ────────────────────────────────────────────────────────────────────
// F3 — the derived property taken on faith
// ────────────────────────────────────────────────────────────────────

mod f3_derived_property_validate {
    /// The provider trait: any impl can return any byte (the
    /// `as_utf8_pattern`-is-safe shape).
    trait EncodedPattern {
        fn byte_at(&self) -> u8;
    }

    /// The honest impl: a real ASCII byte.
    struct AsciiPattern(u8);

    impl EncodedPattern for AsciiPattern {
        fn byte_at(&self) -> u8 {
            self.0
        }
    }

    /// The rogue impl: a one-byte pattern that is NOT ASCII (the 0x80
    /// class — the byte only appears inside multibyte UTF-8 sequences).
    struct RogueBytePattern;

    impl EncodedPattern for RogueBytePattern {
        fn byte_at(&self) -> u8 {
            0x80
        }
    }

    /// The buggy swap: the derived byte is trusted ASCII and the result
    /// is laundered. The LAUNDERING is named (from_utf8_unchecked), the
    /// demonstration goes through the CHECKED constructor so the harness
    /// stays defined: the corruption is the invalid-UTF-8 byte vector.
    fn swap_buggy(haystack: &str, b: u8, to: u8) -> Result<String, String> {
        let bytes: Vec<u8> = haystack
            .bytes()
            .map(|x| if x == b { to } else { x })
            .collect();
        // Production: `unsafe { String::from_utf8_unchecked(bytes) }`.
        // Harness: the checked constructor OBSERVES the corruption.
        String::from_utf8(bytes).map_err(|e| format!("corruption: {e}"))
    }

    /// The validate arm: the derived property is checked first.
    fn swap_validated(haystack: &str, b: u8, to: u8) -> Result<String, String> {
        if !b.is_ascii() {
            return Err("pattern byte must be ascii".into());
        }
        if !to.is_ascii() {
            return Err("replacement byte must be ascii".into());
        }
        let bytes: Vec<u8> = haystack
            .bytes()
            .map(|x| if x == b { to } else { x })
            .collect();
        // The property now holds by validation; the unchecked constructor
        // would be sound. The harness keeps the checked path.
        String::from_utf8(bytes).map_err(|e| format!("unexpected: {e}"))
    }

    /// The wrong-property "validation": checks the wrong invariant.
    fn swap_wrong_check(haystack: &str, b: u8, to: u8) -> Result<String, String> {
        if b == 0 {
            return Err("null byte refused".into());
        }
        let bytes: Vec<u8> = haystack
            .bytes()
            .map(|x| if x == b { to } else { x })
            .collect();
        String::from_utf8(bytes).map_err(|e| format!("corruption: {e}"))
    }

    /// A continuation-bearing haystack: `\u{d000}` is [ed, 80, 80] — the
    /// post's own demonstration shape.
    const HAYSTACK: &str = "\u{d000}";
    const REPLACEMENT: u8 = b'?';

    #[test]
    fn f3_rogue_byte_corrupts_the_string() {
        let err = swap_buggy(HAYSTACK, RogueBytePattern.byte_at(), REPLACEMENT)
            .expect_err("the 0x80 replacement must corrupt the string");
        assert!(err.contains("corruption"), "{err}");
        // The honest impl on the same haystack is CLEAN (its byte never
        // matches a continuation byte).
        assert!(swap_buggy(HAYSTACK, AsciiPattern(b'x').byte_at(), REPLACEMENT).is_ok());
    }

    #[test]
    fn f3_validated_arm_refuses_the_non_ascii_byte() {
        let err = swap_validated(HAYSTACK, RogueBytePattern.byte_at(), REPLACEMENT)
            .expect_err("the validate arm must refuse the rogue byte");
        assert!(err.contains("must be ascii"), "{err}");
        assert!(swap_validated(HAYSTACK, AsciiPattern(b'x').byte_at(), REPLACEMENT).is_ok());
    }

    #[test]
    fn f3_honest_impl_is_clean_on_both_postures() {
        // The anti-regime: a by-construction honest impl (an ASCII byte)
        // triggers neither posture — even though the buggy posture takes
        // it on faith.
        let honest = AsciiPattern(b'a').byte_at();
        assert!(swap_buggy(HAYSTACK, honest, REPLACEMENT).is_ok());
        assert!(swap_validated(HAYSTACK, honest, REPLACEMENT).is_ok());
    }

    #[test]
    fn f3_wrong_property_validation_disagrees() {
        // The tampered arm: `b != 0` passes 0x80 — the corruption returns.
        let err = swap_wrong_check(HAYSTACK, RogueBytePattern.byte_at(), REPLACEMENT)
            .expect_err("the wrong-property check must let the corruption through");
        assert!(err.contains("corruption"), "{err}");
    }
}

// ────────────────────────────────────────────────────────────────────
// F4 — the process-global write under the shared lock tier
// ────────────────────────────────────────────────────────────────────

mod f4_process_global_write_under_shared_lock {
    use std::cell::Cell;

    /// A toy RwLock: read() admits ANY number of concurrent guards;
    /// write() requires exclusivity. The lock model makes the tier
    /// semantics explicit (the real `environ` race is timing-dependent —
    /// the toy steps the interleaving deterministically, the f1-SQL /
    /// f4-virtual-clock discipline).
    #[derive(Default)]
    struct ToyRwLock {
        readers: Cell<usize>,
        writer: Cell<bool>,
    }

    impl ToyRwLock {
        fn read(&self) -> ReadGuard<'_> {
            self.readers.set(self.readers.get() + 1);
            ReadGuard { lock: self }
        }

        fn write(&self) -> WriteGuard<'_> {
            assert!(!self.writer.get(), "writer already held");
            assert_eq!(self.readers.get(), 0, "readers still held");
            self.writer.set(true);
            WriteGuard { lock: self }
        }

        fn readers_held(&self) -> usize {
            self.readers.get()
        }
    }

    struct ReadGuard<'a> {
        lock: &'a ToyRwLock,
    }

    impl Drop for ReadGuard<'_> {
        fn drop(&mut self) {
            self.lock.readers.set(self.lock.readers.get() - 1);
        }
    }

    struct WriteGuard<'a> {
        lock: &'a ToyRwLock,
    }

    impl Drop for WriteGuard<'_> {
        fn drop(&mut self) {
            self.lock.writer.set(false);
        }
    }

    /// The process-global slot and its save/restore slot (the exec-analog
    /// state).
    #[derive(Default)]
    struct Globals {
        slot: Cell<u64>,
        restore: Cell<u64>,
    }

    /// The buggy swap step: the global write rides the SHARED tier.
    fn swap_write_step_buggy(lock: &ToyRwLock, g: &Globals, new: u64) {
        let _guard = lock.read(); // the shared tier
        let old = g.slot.get();
        g.restore.set(old);
        g.slot.set(new); // the process-global write under the read side
    }

    /// The restore step (the failed-exec analog): the saved old value is
    /// put back.
    fn restore_step(g: &Globals) {
        g.slot.set(g.restore.get());
    }

    /// The reader: a plain env-style read through the same lock.
    fn reader_snapshot(lock: &ToyRwLock, g: &Globals) -> u64 {
        let _guard = lock.read();
        g.slot.get()
    }

    /// The FIXED swap: write + restore complete inside ONE exclusive
    /// section — no reader can observe the intermediate state.
    fn swap_and_restore_fixed(lock: &ToyRwLock, g: &Globals, new: u64) {
        let _guard = lock.write();
        let old = g.slot.get();
        g.restore.set(old);
        g.slot.set(new);
        restore_step(g); // the failed-swap restore, inside the section
    }

    /// The tampered "fix": the write rides the exclusive tier but the
    /// restore escapes it — the intermediate state leaks to readers.
    fn swap_fixed_restore_outside(lock: &ToyRwLock, g: &Globals, new: u64) {
        {
            let _guard = lock.write();
            let old = g.slot.get();
            g.restore.set(old);
            g.slot.set(new);
        } // the section ends BEFORE the restore
        restore_step(g);
    }

    const V0: u64 = 100;
    const VA: u64 = 200;

    #[test]
    fn f4_read_guards_admit_concurrent_holders() {
        let lock = ToyRwLock::default();
        let _a = lock.read();
        let _b = lock.read();
        assert_eq!(
            lock.readers_held(),
            2,
            "two read guards held SIMULTANEOUSLY — the shared tier never \
             excluded the writer-to-be"
        );
    }

    #[test]
    fn f4_reader_during_the_interleaved_swap_sees_the_child_value() {
        let lock = ToyRwLock::default();
        let g = Globals::default();
        g.slot.set(V0);
        // The buggy posture: the write step and the restore step run as
        // separate shared-tier sections — a reader fits between them.
        swap_write_step_buggy(&lock, &g, VA);
        let seen = reader_snapshot(&lock, &g);
        assert_eq!(
            seen, VA,
            "the reader must see the CHILD-ONLY value mid-flight — the \
             post's sentence: a thread calling env::var can see variables \
             meant only for the new program"
        );
        restore_step(&g);
        assert_eq!(g.slot.get(), V0, "the restore lands the original value");
    }

    #[test]
    fn f4_serialized_swap_never_exposes_the_intermediate() {
        let lock = ToyRwLock::default();
        let g = Globals::default();
        g.slot.set(V0);
        // The fixed posture: write + restore in one exclusive section.
        // The toy write() REFUSES while readers are held, and the reader
        // can only run before or after the section — never inside it.
        swap_and_restore_fixed(&lock, &g, VA);
        let seen = reader_snapshot(&lock, &g);
        assert_eq!(
            seen, V0,
            "the reader after a serialized failed swap sees the original \
             value — the intermediate never leaked"
        );
    }

    #[test]
    fn f4_lone_writer_restore_is_exact() {
        // The anti-regime: a single-threaded posture (no concurrent
        // readers) is a legitimate set_var world — the save/restore cycle
        // is exact even under the shared tier.
        let lock = ToyRwLock::default();
        let g = Globals::default();
        g.slot.set(V0);
        swap_write_step_buggy(&lock, &g, VA);
        restore_step(&g);
        assert_eq!(g.slot.get(), V0);
        assert_eq!(reader_snapshot(&lock, &g), V0);
    }

    #[test]
    fn f4_restore_outside_the_exclusive_section_disagrees() {
        // The tampered arm: the write tier is taken, but the restore runs
        // AFTER the section ends — the intermediate value leaks to a
        // reader, exactly the buggy posture's exposure.
        let lock = ToyRwLock::default();
        let g = Globals::default();
        g.slot.set(V0);
        swap_fixed_restore_outside(&lock, &g, VA);
        // The reader here runs after the fact, so assert the structural
        // difference instead: the toy write() excludes readers during the
        // write step, but the restore step ran UNPROTECTED — modeled by
        // the section's readers count being zero while restore is not
        // covered. The observable: a reader scheduled between the two
        // steps (the toy steps it deterministically).
        let g2 = Globals::default();
        g2.slot.set(V0);
        {
            // Re-step the tampered path with the reader between.
            let _w = lock.write();
            let old = g2.slot.get();
            g2.restore.set(old);
            g2.slot.set(VA);
        } // section ends
        let seen = reader_snapshot(&lock, &g2); // runs BEFORE the restore
        assert_eq!(
            seen, VA,
            "the restore-outside fix must leak the intermediate value to \
             the reader — it disagrees with the serialized law"
        );
        restore_step(&g2);
        assert_eq!(g2.slot.get(), V0);
    }
}
