//! Batch 380 fixtures (Plan 397 / Research 429 §Delta) — ORIGINAL std-only
//! reproductions of the alesha-pro/maya-inkling-mini 8-GB-tier laws (@
//! e819852f, MIT), never pasted source. One standalone file per the
//! B336/…/B379 discipline: compiled AND executed BEFORE entry as
//! `rustc --edition 2021 --test b380_fixtures.rs && ./b380_fixtures`, at
//! debug AND -O. The heterogeneous CPU+GPU surface is modeled as a CPU
//! mock (the b370/b379 precedent); the fixture never calls a GPU. Each
//! arm of the batch's entries file embeds the same executed code — the
//! fixture pass is the authoring gate: a defect caught here is fixed HERE
//! first, then synced back into the entries in the same landing commit.
//!
//! Arms (the rule → what the tests pin):
//! - HIT: a cached id dispatches `Hit`, touches no lane accumulator, and
//!   moves no price.
//! - MISS-COMPUTED-OVERLAPPED (the law's arm): the free-first dispatcher
//!   hands the miss to the lane that frees first; the token wall is
//!   max(gpu, cpu) — strictly under the serialized sum; the first computed
//!   miss takes its lazy copy (the next visit HITS — the miss is paid
//!   once); a second computed miss inside the same call is COMPUTE-ONLY
//!   (per-call bound) and evicts nothing.
//! - MISS-COMPUTED-SERIALIZED (the negative, the ref shape): on a narrow
//!   link (the copy priced high, IK2_UP_MS) the free-first comparison
//!   degenerates to compute-every-miss — never a stall; the ref shape's
//!   static copy-first precedence is what the free-first law replaces.
//! - TAMPERED: a pool that posts but never completes refuses LOUD (typed
//!   error), never a silent stall; the EMA re-price converges and FLIPS
//!   the lane choice when the measured prices cross (the online law).
//! - CACHE SHAPE: the S3-FIFO second-chance rotation (a probation front
//!   with freq > 0 moves to main at freq-1, the scan finds the freq-0
//!   victim); a ghost re-admission inside one generation carries freq 1;
//!   per-size-class pools isolate evictions.

use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// The two-lane miss dispatcher (the law's home).
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lane {
    Copy,
    Compute,
}

#[derive(Debug)]
enum Dispatch {
    Hit,
    Served(Lane),
}

struct Tier {
    cached: std::collections::HashSet<u32>,
    acc_cpu: f64,
    acc_up: f64,
    t_cpu: f64,
    t_up: f64,
    lazy_in_flight: usize,
    lazy_max: usize,
    lazy_per: usize,
    lazy_this_call: usize,
    pool_alive: bool,
    compute_only: usize,
}

impl Tier {
    fn new() -> Self {
        Self {
            cached: Default::default(),
            acc_cpu: 0.0,
            acc_up: 0.0,
            t_cpu: 0.50e-3,
            t_up: 0.36e-3,
            lazy_in_flight: 0,
            lazy_max: 2,
            lazy_per: 1,
            lazy_this_call: 0,
            pool_alive: true,
            compute_only: 0,
        }
    }

    /// the free-first law: the lane that frees first takes the miss.
    fn dispatch(&mut self, id: u32) -> Result<Dispatch, &'static str> {
        if self.cached.contains(&id) {
            return Ok(Dispatch::Hit);
        }
        if !self.pool_alive {
            // a pool that posts but never completes must refuse LOUD,
            // never silently stall the tier on a dead lane.
            const POOL_DEAD: &str = "cpu pool dead: lane prices unmeasurable, refuse";
            return Err(POOL_DEAD);
        }
        let lane = if self.acc_cpu + self.t_cpu <= self.acc_up + self.t_up {
            self.acc_cpu += self.t_cpu;
            Lane::Compute
        } else {
            self.acc_up += self.t_up;
            Lane::Copy
        };
        if lane == Lane::Compute {
            // lazy: blocks follow on the copy stream, bounded per call
            // and in flight; past either bound: compute-only, NO slot —
            // nothing in VRAM is evicted for bytes we will not wait for.
            let ok = self.lazy_in_flight < self.lazy_max && self.lazy_this_call < self.lazy_per;
            if ok {
                self.cached.insert(id);
                self.lazy_in_flight += 1;
                self.lazy_this_call += 1;
            } else {
                self.compute_only += 1;
            }
        } else {
            self.cached.insert(id);
        }
        Ok(Dispatch::Served(lane))
    }

    /// the online re-price: every completed CPU batch moves the EMA.
    fn reprice_cpu(&mut self, batch_s: f64, n_jobs: f64) {
        self.t_cpu = 0.9 * self.t_cpu + 0.1 * (batch_s / n_jobs);
    }

    fn begin_call(&mut self) {
        self.lazy_this_call = 0;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The cache shape: O(1) S3-FIFO with ghost history (per-size-class pools wrap
// one of these each — `Pools` below).
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Seg {
    Probation,
    Main,
}

struct S3Fifo {
    /// slot -> (id, segment, freq) — the intrusive lists modeled as order.
    slots: Vec<Option<(u32, Seg, u8)>>,
    prob: Vec<usize>,
    main: Vec<usize>,
    max_freq: u8,
    ghost: HashMap<u32, u64>,
    ghost_seq: u64,
}

impl S3Fifo {
    fn new(n_slots: usize) -> Self {
        Self {
            slots: vec![None; n_slots],
            prob: Vec::new(),
            main: Vec::new(),
            max_freq: 7,
            ghost: HashMap::new(),
            ghost_seq: 1,
        }
    }

    /// O(1) admission: a free slot first; otherwise the probation front
    /// with freq > 0 gets its SECOND CHANCE (freq-1, moved to main back);
    /// the freq-0 front is the victim; probation exhausted → main's front.
    fn insert(&mut self, id: u32) -> usize {
        let slot = if let Some(s) = self.slots.iter().position(|s| s.is_none()) {
            s
        } else {
            loop {
                if let Some(&s) = self.prob.first() {
                    self.prob.remove(0);
                    let (old, _, freq) = self.slots[s].expect("prob slot filled");
                    if freq > 0 {
                        self.slots[s] = Some((old, Seg::Main, freq - 1));
                        self.main.push(s);
                        continue; // the scan moves on: one more probation front
                    }
                    // the victim: stamp the ghost history, reuse its slot
                    self.ghost.insert(old, self.ghost_seq);
                    self.ghost_seq += 1;
                    break s;
                }
                // probation exhausted: the main FIFO's front is the victim.
                let &s = self.main.first().expect("cache full beyond its lists");
                self.main.remove(0);
                let (old, _, _) = self.slots[s].expect("main slot filled");
                self.ghost.insert(old, self.ghost_seq);
                self.ghost_seq += 1;
                break s;
            }
        };
        // the ghost window: a re-admission inside one cache generation is
        // recognized and re-enters PROBATION at freq 1 (the second chance
        // is the FREQ, not a private segment queue in this mock).
        let in_window = self
            .ghost
            .get(&id)
            .is_some_and(|g| self.ghost_seq - *g <= self.slots.len() as u64);
        let freq = u8::from(in_window);
        self.slots[slot] = Some((id, Seg::Probation, freq));
        self.prob.push(slot);
        slot
    }

    fn hit(&mut self, id: u32) -> bool {
        for s in 0..self.slots.len() {
            if let Some((sid, seg, freq)) = self.slots[s] {
                if sid == id {
                    let nf = (freq + 1).min(self.max_freq);
                    if seg == Seg::Probation && nf >= 2 {
                        self.prob.retain(|&p| p != s);
                        self.main.push(s);
                        self.slots[s] = Some((sid, Seg::Main, nf));
                    } else {
                        self.slots[s] = Some((sid, seg, nf));
                    }
                    return true;
                }
            }
        }
        false
    }

    fn slot_of(&self, id: u32) -> Option<usize> {
        self.slots
            .iter()
            .position(|s| matches!(s, Some((sid, _, _)) if *sid == id))
    }
}

/// per-size-class pools: a small expert's pool never evicts a big one's.
struct Pools {
    classes: Vec<S3Fifo>,
}

impl Pools {
    fn new() -> Self {
        Self {
            classes: vec![S3Fifo::new(4), S3Fifo::new(2)],
        }
    }
    fn pool_for(&mut self, big: bool) -> &mut S3Fifo {
        if big {
            &mut self.classes[1]
        } else {
            &mut self.classes[0]
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Arms.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn arm1_hit_touches_no_lane() {
    let mut tier = Tier::new();
    tier.cached.insert(7);
    let before = (tier.acc_cpu, tier.acc_up, tier.t_cpu);
    assert!(matches!(tier.dispatch(7), Ok(Dispatch::Hit)));
    assert_eq!((tier.acc_cpu, tier.acc_up, tier.t_cpu), before);
}

#[test]
fn arm2_overlapped_miss_beats_serialized() {
    let mut tier = Tier::new();
    tier.begin_call();
    let gpu_s = 15.0e-3_f64; // the token's own GPU kernels (their datum)
    let d1 = tier.dispatch(1).expect("pool alive");
    // both accumulators zero: 0 + t_cpu <= 0 + t_up reads 0.50 <= 0.36 —
    // FALSE, the COPY lane frees first. The honest pick; the law's arm is
    // the WALL, not the lane identity.
    assert!(matches!(d1, Dispatch::Served(Lane::Copy)));
    let lane_s = 0.36e-3_f64;
    let overlapped = gpu_s.max(lane_s);
    let serialized = gpu_s + lane_s;
    assert!(overlapped < serialized, "the overlap is the win");

    // the second miss prices to the CPU lane; its lazy copy admits (the
    // call's per-call bound had room) → the next visit HITS.
    let d2 = tier.dispatch(2).expect("pool alive");
    assert!(matches!(d2, Dispatch::Served(Lane::Compute)));
    assert!(tier.cached.contains(&2), "the lazy copy takes its slot");
    assert!(matches!(tier.dispatch(2), Ok(Dispatch::Hit)), "paid once");

    // the third miss prices back to the copy lane (0.50+0.50 > 0.72+0.36
    // cumulative — the accumulators keep the honest ledger); the FOURTH
    // prices to the CPU again and trips the per-call bound: compute-only —
    // no slot, nothing in VRAM evicted for it.
    let d3 = tier.dispatch(3).expect("pool alive");
    assert!(matches!(d3, Dispatch::Served(Lane::Copy)));
    let d4 = tier.dispatch(4).expect("pool alive");
    assert!(matches!(d4, Dispatch::Served(Lane::Compute)));
    assert!(!tier.cached.contains(&4), "compute-only miss took no slot");
    assert_eq!(tier.compute_only, 1);
}

#[test]
fn arm3_narrow_link_degenerates_to_compute_not_stall() {
    let mut tier = Tier::new();
    tier.begin_call();
    // IK2_UP_MS: the copy priced so high the CPU computes EVERY miss —
    // the free-first comparison degrades gracefully, never a stall.
    tier.t_up = 5.0e-3; // the knob's 5 ms posture
    for id in 0..8u32 {
        let d = tier.dispatch(id).expect("pool alive");
        assert!(
            matches!(d, Dispatch::Served(Lane::Compute)),
            "narrow link: every miss goes to the CPU lane (got {d:?})"
        );
    }
}

#[test]
fn arm4_tampered_pool_refuses_loud_and_ema_flips_lanes() {
    let mut tier = Tier::new();
    tier.pool_alive = false;
    let err = tier.dispatch(9).expect_err("dead pool refuses loud");
    assert!(err.contains("refuse"), "typed refusal: {err}");

    // the online re-price: a slow pool starts on the copy lane; the EMA
    // over completed batches converges to the true fast price and the
    // free-first pick FLIPS to the CPU lane.
    let mut tier = Tier::new();
    tier.begin_call();
    tier.t_cpu = 2.0e-3; // slow-pool prior: copy wins (0.36 < 2.0)
    let d = tier.dispatch(11).expect("alive");
    assert!(matches!(d, Dispatch::Served(Lane::Copy)));
    // 100 jobs a 4 ms batch → 0.04 ms/job measured; the 0.9/0.1 EMA
    // converges from the slow prior within a few dozen batches.
    for _ in 0..40 {
        tier.reprice_cpu(4.0e-3, 100.0);
    }
    assert!(tier.t_cpu < 0.25e-3, "EMA converged: {}", tier.t_cpu);
    tier.begin_call();
    let d = tier.dispatch(12).expect("alive");
    assert!(matches!(d, Dispatch::Served(Lane::Compute)), "lane flipped");
}

#[test]
fn arm5_cache_shape_s3fifo_ghost_and_per_class() {
    // the second-chance rotation: with the cache FULL, the admission scan
    // moves a freq-1 probation front to main (freq-1) before evicting, and
    // takes the freq-0 front as the victim.
    let mut c = S3Fifo::new(2);
    let a = c.insert(100); // slot 0, probation
    let _ = c.hit(100); // freq 1
    let b = c.insert(101); // slot 1 (free), probation — no eviction yet
    assert_eq!(c.slot_of(100), Some(a));
    assert_eq!(c.slot_of(101), Some(b));
    // FULL: the scan second-chances 100 (freq 1 → main at freq 0), then
    // evicts the freq-0 front (101).
    let d = c.insert(102);
    assert_eq!(c.slot_of(100), Some(a), "second chance: 100 moved to main");
    assert!(c.slot_of(101).is_none(), "the freq-0 front is the victim");
    assert!(
        c.ghost.contains_key(&101),
        "the victim is stamped in history"
    );
    // ghost re-admission inside one generation: freq 1 on re-entry.
    let s = c.insert(101);
    assert_eq!(c.slot_of(101), Some(s));
    let f1 = matches!(c.slots[s], Some((_, _, f)) if f == 1);
    assert!(f1, "ghost re-admission re-enters at freq 1");
    assert_ne!(a, d, "the victim's slot is distinct from the survivor's");

    // per-size-class pools: flooding the BIG class never evicts the SMALL.
    let mut pools = Pools::new();
    let small = pools.pool_for(false);
    let _ = small.insert(1);
    let big = pools.pool_for(true);
    for id in 10..16u32 {
        let _ = big.insert(id);
    }
    assert!(
        pools.pool_for(false).slot_of(1).is_some(),
        "a big-class flood never evicts the small class"
    );
}

#[test]
fn arm6_lane_accumulators_keep_free_first_order() {
    // within one call the accumulators reproduce the source's order:
    // exact prices tie → the CPU lane (the <= tie, the source's own
    // comparison), and the chosen lane's accumulator grows so the NEXT
    // miss prices to the other lane.
    let mut tier = Tier::new();
    tier.begin_call();
    tier.t_cpu = tier.t_up; // exact tie
    let d = tier.dispatch(21).expect("alive");
    assert!(matches!(d, Dispatch::Served(Lane::Compute)), "tie → CPU");
    assert!(tier.acc_cpu > 0.0);
    let d = tier.dispatch(22).expect("alive");
    assert!(matches!(d, Dispatch::Served(Lane::Copy)), "the other lane");
}
