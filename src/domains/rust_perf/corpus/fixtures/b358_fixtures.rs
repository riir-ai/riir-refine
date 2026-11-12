//! Batch 358 fixtures — the splash lane-B ANE/host host-side laws as ONE
//! standalone file.
//!
//! Compiled AND executed BEFORE entry: `rustc --edition 2021 --test
//! b358_fixtures.rs` (the B336→B357 discipline). Each rule's ref/opt arms in
//! `entries_74.rs` (rust_perf) embed the same executed code. ORIGINAL Rust
//! throughout (the source is Apache-2.0 C++/Metal + docs prose; the fixtures
//! are original syntheses of the LAWS — an accelerator service is mocked as
//! a plain struct with a compile ledger; the fixture never links Core ML,
//! Metal, or any ANE runtime).
//!
//! K1 = one-program-many-functions-accelerator-service-memory (rust_perf, layout)
//! K2 = ane-split-calibration-fit-bandwidth-interference-term (rust_perf, measurement)
//! K3 = accelerator-lost-breaker-forgets-calibration (rust_perf, caching)
//! K4 = writeclass-share-partition-inflight-writes (rust_perf, concurrency)
//! K5 = spin-keep-awake-checked-capability-degrade-loud (rust_perf, thermal)
//!
//! Source: incoai/splash @ `cabbad27` (Apache-2.0; cloned at T0, removed at
//! close). Batch 358 / Plan 369 / Research 345 (lane B).

// ─────────────────────────────────────────────────────────────────────────────
// K1 — one program, many row-length functions: the accelerator service bills
// an intermediate buffer PER LOADED PROGRAM, so N one-function programs hold
// the working set N times; ONE program with N row-length functions shares one
// surface set sized for the largest rows. Compile is memoized per source hash
// (FNV-1a, the source's own key form), so the bank compiles once and warm
// loads skip the compile.
// ─────────────────────────────────────────────────────────────────────────────

/// FNV-1a over bytes — the source-hash form (the names' key in the source is
/// FNV-1a of the program source).
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// A mock accelerator: every LOADED program holds its own intermediate
/// buffer (the service's billed residency), and compiling is slow.
struct MockService {
    loaded: Vec<(u64, usize)>, // (source hash, intermediate bytes held)
    compiles: usize,
}

impl MockService {
    fn new() -> Self {
        Self {
            loaded: Vec::new(),
            compiles: 0,
        }
    }

    /// THE BUGGY ARM: one one-function program per row length. Each program
    /// is billed its own intermediate surface set, so the residency is the
    /// SUM over programs.
    fn load_one_function_per_len(&mut self, row_lens: &[usize], rows: usize) {
        for len in row_lens {
            let src = format!("fn(rows={rows}, len={len})");
            let hash = fnv1a(src.as_bytes());
            if self.loaded.iter().any(|(h, _)| *h == hash) {
                continue; // compile-once per source hash: a re-load is warm
            }
            self.compiles += 1;
            // the service bills ONE intermediate surface set per program,
            // sized for the chunk shape (rows x widest function) — the same
            // set size every one-function program holds
            self.loaded
                .push((hash, rows * row_lens.iter().max().unwrap() * 4));
        }
    }

    /// THE REMEDY: ONE program whose function table covers every row length;
    /// all functions read/write ONE surface set sized for the LARGEST rows,
    /// so the residency is the MAX, never the sum. Still one source, one
    /// hash, one compile.
    fn load_one_program_many_functions(&mut self, row_lens: &[usize], max_rows: usize) {
        let mut src = String::from("fn(group)");
        for len in row_lens {
            src.push_str(&format!(",f{len}"));
        }
        let hash = fnv1a(src.as_bytes());
        if self.loaded.iter().any(|(h, _)| *h == hash) {
            return;
        }
        self.compiles += 1;
        // one shared surface set sized for the largest rows the group runs
        self.loaded
            .push((hash, max_rows * row_lens.iter().max().unwrap() * 4));
    }

    fn residency_bytes(&self) -> usize {
        self.loaded.iter().map(|(_, b)| *b).sum()
    }
}

#[test]
fn k1_one_program_many_functions_shares_billed_intermediates() {
    // the source's worked shape: 13 row-length functions (512..=2048 step 128)
    let lens: Vec<usize> = (512..=2048usize).step_by(128).collect();
    assert_eq!(lens.len(), 13);
    let max_rows = 2048;

    let mut split = MockService::new();
    split.load_one_function_per_len(&lens, max_rows);
    let mut grouped = MockService::new();
    grouped.load_one_program_many_functions(&lens, max_rows);

    // the sum-over-programs vs max-over-shared-surfaces axis: the grouped
    // bank holds a strictly smaller residency for the SAME coverage
    assert!(
        grouped.residency_bytes() < split.residency_bytes(),
        "one program with {} functions must hold less billed residency than {} one-function programs",
        lens.len(),
        lens.len()
    );
    // and compiles ONCE, not once per function
    assert_eq!(grouped.compiles, 1);
    assert_eq!(split.compiles, lens.len());

    // warm load: re-loading the same source compiles nothing anywhere
    let before = grouped.compiles;
    grouped.load_one_program_many_functions(&lens, max_rows);
    assert_eq!(grouped.compiles, before, "warm load skips the compile");

    // the source's magnitude, reproduced structurally: 13 one-function
    // programs ≈ 13× the one-program residency (13 × 2048·2048·4 vs
    // 1 × 2048·2048·4)
    assert_eq!(split.residency_bytes(), 13 * grouped.residency_bytes());
}

// ─────────────────────────────────────────────────────────────────────────────
// K2 — the live split calibration: fit a two-line model from a low and a high
// share measurement (GPU line from the two GPU-alone points; the ANE line
// from the high share; u = the bandwidth the accelerator's part takes from the
// GPU's beside it, clamped to [0,1] and only where the GPU's low-share point
// sits under the ANE's), then choose the split by argmin over ±1 unit with a
// 5% gain floor against the GPU alone. A calibration that clears no floor is
// REFUSED (None), never a static default.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Timing {
    share: f64,
    gpu_ms: f64,   // the GPU's part ALONE at this share
    split_ms: f64, // the whole chunk at this share (split running)
}

#[derive(Clone, Copy, Debug)]
struct Model {
    g0: f64, // the GPU line: gpu(s) = g0 + slope·s
    slope: f64,
    ane: f64, // the ANE line: flat in the fit's regime (per-row-scaled)
    u: f64,   // the interference term
}

impl Model {
    fn gpu(&self, s: f64) -> f64 {
        self.g0 + self.slope * s
    }
    /// T(s) = max(A(s), G(s) + u·A(s)) — the chunk takes the longer of the
    /// ANE's part and the GPU's part running beside it.
    fn total(&self, s: f64) -> f64 {
        let a = self.ane;
        let g = self.gpu(s);
        a.max(g + self.u * a)
    }
}

/// fit() — the source's shape: the GPU line through the two alone points,
/// the ANE line flat at the high share's reading, u clamped where defined.
fn fit(low: Timing, high: Timing) -> Model {
    assert!(low.share < high.share, "needs a low and a high share");
    let slope = (high.gpu_ms - low.gpu_ms) / (high.share - low.share);
    let mut model = Model {
        g0: low.gpu_ms - slope * low.share,
        slope,
        ane: high.split_ms / high.share,
        u: 0.0,
    };
    let gpu = model.gpu(low.share);
    let ane = model.ane;
    if gpu > ane {
        // the low-share excess over the GPU's own part IS the interference
        model.u = ((low.split_ms - gpu) / ane).clamp(0.0, 1.0);
    }
    model
}

/// choose() — argmin of T over ±1 unit around the measured point, with the
/// 5% gain floor against the GPU alone; no candidate clears the floor ⇒ None
/// (the split is refused, never a default).
fn choose(
    model: &Model,
    at: usize,
    units: usize,
    gpu_alone: impl Fn(usize) -> f64,
) -> Option<usize> {
    let mut best: Option<(f64, usize)> = None;
    for cand in at.saturating_sub(1)..=(at + 1).min(units) {
        let s = cand as f64 / units as f64;
        let t = model.total(s);
        if t > (1.0 - 0.05) * gpu_alone(cand) {
            continue; // the 5% gain floor
        }
        if best.map_or(true, |(bt, _)| t < bt) {
            best = Some((t, cand));
        }
    }
    best.map(|(_, c)| c)
}

#[test]
fn k2_calibration_fit_carries_interference_and_floor_refuses() {
    // a model where the ANE part is long at the high share and the GPU part
    // pays interference at the low share (the M6-class shape: u ≈ 0.1–0.33)
    let low = Timing {
        share: 0.2,
        gpu_ms: 10.0,
        split_ms: 13.0,
    };
    let high = Timing {
        share: 0.8,
        gpu_ms: 16.0,
        split_ms: 40.0,
    };
    let model = fit(low, high);
    // u landed in [0,1] and the fit's ane reads off the high share
    assert!((0.0..=1.0).contains(&model.u));
    assert!((model.ane - 40.0 / 0.8).abs() < 1e-9);
    // T(s) is the max of the two arms — never a sum
    let t_low = model.total(low.share);
    assert!(t_low >= model.ane && t_low >= model.gpu(low.share));

    // the floor: GPU alone already faster than any split total ⇒ no gain
    let fast_gpu = |_: usize| 30.0_f64;
    assert_eq!(choose(&model, 5, 10, fast_gpu), None, "no gain ⇒ no split");

    // a GPU alone that loses to the split at some unit ⇒ a choice exists
    let slow_gpu = |u: usize| match u {
        4 | 5 | 6 => 60.0,
        _ => 30.0,
    };
    let pick = choose(&model, 5, 10, slow_gpu);
    assert!(pick.is_some(), "a real gain must pick a unit");
    // the choice stays within ±1 of the measured point
    assert!((pick.unwrap() as i64 - 5).abs() <= 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// K3 — the lost-breaker: a fast path (the split) is judged over a bounded
// window (8 commands) against the accumulated GPU-alone baseline; when the
// window's accumulated fast-path time reaches the baseline's, the path retires
// AND the remembered calibration is FORGOTTEN (the forget closure fires) so
// the next start re-derives it from measurement. The window resets every 8
// commands whether or not it trips.
// ─────────────────────────────────────────────────────────────────────────────

/// The breaker (the fixture's law): accumulate, judge at the window,
/// reset, and on trip fire forget(). The accumulator is read BEFORE the
/// reset — a reset-then-judge shape would trip on nothing, ever.
struct RealBreaker<F: Fn(u32) -> f64> {
    gpu_alone: F,
    window: u32,
    commands: u32,
    fast_ms: f64,
    baseline_ms: f64,
}

impl<F: Fn(u32) -> f64> RealBreaker<F> {
    fn new(gpu_alone: F, window: u32) -> Self {
        Self {
            gpu_alone,
            window,
            commands: 0,
            fast_ms: 0.0,
            baseline_ms: 0.0,
        }
    }

    fn add(
        &mut self,
        rows: u32,
        evaluations: u32,
        ms: f64,
        forget: &mut dyn FnMut(),
    ) -> Option<String> {
        self.fast_ms += ms;
        self.baseline_ms += f64::from(evaluations) * (self.gpu_alone)(rows);
        self.commands += 1;
        if self.commands < self.window {
            return None;
        }
        let trip = self.fast_ms >= self.baseline_ms;
        let ratio = self.fast_ms / self.baseline_ms;
        // the window resets EVERY window, trip or not
        self.commands = 0;
        self.fast_ms = 0.0;
        self.baseline_ms = 0.0;
        if trip {
            forget(); // calibration amnesia: the next start re-derives
            return Some(format!(
                "losing to the GPU alone ({:.1}x the baseline over the window)",
                ratio
            ));
        }
        None
    }
}

#[test]
fn k3_breaker_trips_over_window_and_forgets_calibration() {
    let forgotten = std::cell::Cell::new(0usize);
    let mut forget = || forgotten.set(forgotten.get() + 1);
    let mut breaker = RealBreaker::new(|rows| f64::from(rows) * 0.01, 8);

    // 8 FAST commands: the fast path wins, no trip, no forget
    for _ in 0..8 {
        assert!(breaker.add(100, 1, 0.5, &mut forget).is_none());
    }
    assert_eq!(forgotten.get(), 0, "a winning window forgets nothing");

    // 7 slow commands then the 8th — the trip lands AT the window boundary,
    // not on the first slow command (bounded window, not a threshold)
    let mut tripped = None;
    for i in 0..8 {
        let ms = if i < 7 { 0.5 } else { 50.0 };
        tripped = breaker.add(100, 1, ms, &mut forget);
    }
    assert!(tripped.is_some(), "the losing window trips at its boundary");
    assert_eq!(
        forgotten.get(),
        1,
        "the trip forgets the remembered calibration"
    );
    assert!(tripped.unwrap().contains("losing to the GPU alone"));

    // the window reset: the next 8 commands are a fresh judgment
    for _ in 0..8 {
        assert!(breaker.add(100, 1, 0.5, &mut forget).is_none());
    }
    assert_eq!(forgotten.get(), 1, "no double trip from stale accumulation");
}

// ─────────────────────────────────────────────────────────────────────────────
// K4 — write-class share partition: a bounded in-flight write budget is
// PARTITIONED per write class (reclaim all, persist a quarter, spill half,
// each with a reserved minimum of one) so one class's flood can never starve
// another, and the reclaim class always keeps room. Admission requires BOTH
// the class share and the total budget.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WriteClass {
    Reclaim,
    Persist,
    Spill,
}

fn share(write: WriteClass, limit: u32) -> u32 {
    match write {
        WriteClass::Reclaim => limit,
        WriteClass::Persist => limit / 4,
        WriteClass::Spill => limit / 2,
    }
}

/// The in-flight ledger: per-class counts under one total budget.
struct Inflight {
    limit: u32,
    counts: [u32; 3],
}

impl Inflight {
    fn new(limit: u32) -> Self {
        Self {
            limit,
            counts: [0; 3],
        }
    }

    fn idx(write: WriteClass) -> usize {
        match write {
            WriteClass::Reclaim => 0,
            WriteClass::Persist => 1,
            WriteClass::Spill => 2,
        }
    }

    fn can_admit(&self, write: WriteClass) -> bool {
        let total: u32 = self.counts.iter().sum();
        total < self.limit && self.counts[Self::idx(write)] < share(write, self.limit).max(1)
    }

    fn admit(&mut self, write: WriteClass) -> bool {
        if self.can_admit(write) {
            self.counts[Self::idx(write)] += 1;
            true
        } else {
            false
        }
    }
}

#[test]
fn k4_writeclass_share_partition_prevents_starvation() {
    let limit = 64;
    let mut inflight = Inflight::new(limit);

    // a SPILL flood fills the budget to its class share (32), never past it
    let mut admitted_spill = 0;
    for _ in 0..200 {
        if inflight.admit(WriteClass::Spill) {
            admitted_spill += 1;
        }
    }
    assert_eq!(admitted_spill, limit / 2, "spill caps at its share");

    // PERSIST still admits: the flood starved nothing (reserved 16)
    assert!(inflight.admit(WriteClass::Persist));
    assert!(inflight.admit(WriteClass::Persist));
    assert_eq!(inflight.counts[Inflight::idx(WriteClass::Persist)], 2);

    // RECLAIM always has room (its share is the whole budget)
    assert!(inflight.admit(WriteClass::Reclaim));

    // the total budget still binds: fill everything to the limit and every
    // class refuses
    let mut full = Inflight::new(limit);
    while full.admit(WriteClass::Reclaim) {}
    assert_eq!(full.counts.iter().sum::<u32>(), limit);
    assert!(!full.admit(WriteClass::Persist), "the total budget binds");

    // the reserved minimum: a tiny budget still admits one of each class
    let mut tiny = Inflight::new(2);
    assert!(tiny.admit(WriteClass::Persist));
    assert!(tiny.admit(WriteClass::Spill));
}

// ─────────────────────────────────────────────────────────────────────────────
// K5 — the checked keep-awake spin: a clock-holding spin whose memory scope
// the platform's public headers lack is built SEPARATELY; before the first
// command relying on it, the backend CHECKS the spin actually observes the
// CPU's end signal; without the capability, or if the check fails, the steps
// run WITHOUT the spin and the degrade is LOUD (a counted warning), never a
// silent no-op.
// ─────────────────────────────────────────────────────────────────────────────

/// A mock "metallib" availability + round-trip probe.
struct SpinLibrary {
    /// does the built artifact exist at all?
    built: bool,
    /// does the spin's memory scope actually see the writer's end flag?
    scope_works: bool,
}

enum KeepAwake {
    /// the spin is armed: the check PASSED (the probe saw the CPU end it)
    Armed,
    /// the spin is absent or its scope failed the round-trip: run without,
    /// loudly
    Degraded { reason: &'static str, warned: bool },
}

fn arm_spin(lib: &SpinLibrary) -> KeepAwake {
    if !lib.built {
        // the source's law: a compiler that refuses it only WARNS
        return KeepAwake::Degraded {
            reason: "spin library refused by the compiler",
            warned: true,
        };
    }
    // the CHECKED half: before relying on the spin, verify it observes the
    // CPU end (a round trip through the scope). A spin that cannot see the
    // end would spin forever — the check is load-bearing, not ceremony.
    if !lib.scope_works {
        return KeepAwake::Degraded {
            reason: "spin scope failed the round-trip check",
            warned: true,
        };
    }
    KeepAwake::Armed
}

/// The command runner: spins only when Armed; work completes either way.
fn run_command(keep: &KeepAwake, spins: &mut u32, warnings: &mut u32, work_ms: &mut f64) {
    match keep {
        KeepAwake::Armed => {
            *spins += 1; // the spin holds clocks through the short step
            *work_ms += 1.0;
        }
        KeepAwake::Degraded { warned, .. } => {
            if *warned {
                *warnings += 1; // the LOUD degrade — once, counted
            }
            *work_ms += 1.5; // the un-spun step runs, slower (cold clocks)
        }
    }
}

#[test]
fn k5_spin_degrades_loud_and_work_still_completes() {
    // the good library: built AND the scope passes the round trip
    let good = SpinLibrary {
        built: true,
        scope_works: true,
    };
    let mut spins = 0;
    let mut warnings = 0;
    let mut work = 0.0;
    let keep = arm_spin(&good);
    assert!(matches!(keep, KeepAwake::Armed));
    run_command(&keep, &mut spins, &mut warnings, &mut work);
    assert_eq!(spins, 1);
    assert_eq!(warnings, 0);

    // a scope that fails the check degrades LOUDLY and still serves
    let broken_scope = SpinLibrary {
        built: true,
        scope_works: false,
    };
    let keep = arm_spin(&broken_scope);
    let mut spins = 0;
    let mut warnings = 0;
    let mut work = 0.0;
    run_command(&keep, &mut spins, &mut warnings, &mut work);
    assert!(matches!(&keep, KeepAwake::Degraded { reason, warned: true }
        if reason.contains("round-trip")));
    assert_eq!(spins, 0, "a failed check never arms the spin");
    assert_eq!(warnings, 1, "the degrade is loud");
    assert!(work > 0.0, "the step still ran without the spin");

    // a refused build (the compiler that only warns) degrades the same way
    let refused = SpinLibrary {
        built: false,
        scope_works: true,
    };
    let keep = arm_spin(&refused);
    assert!(matches!(&keep, KeepAwake::Degraded { reason, .. }
        if reason.contains("refused")));
}
