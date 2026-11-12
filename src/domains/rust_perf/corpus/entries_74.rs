//! Batch 358 — the splash lane-B ANE/host `rust_perf` transcription batch
//! (Plan 369 / Research 345 lane B): the five host-side serving laws staged
//! since 2026-10-28 whose pin-block (the release-set binary rebuild under the
//! 1.98.1 toolchain) lifted — the B355 precedent (zero clippy surface ⇒
//! freeze/score-bench not owed; the maintained-binary rebuild disclosed to
//! the i224 lane). ALL suggestion-only `NotBenchable-by-domain`: the decision
//! content is accelerator-residency / live-calibration / breaker-amnesia /
//! write-class starvation / checked-capability policies, priced on ANE-class
//! hardware no CPU Pareto validator hosts — the dial9/Batch-136 annotation,
//! not a demotion.
//!
//! Source: `incoai/splash` @ `cabbad2732b027540f5a141511544e0bb62224b7`
//! (main tip 2026-11-09, tag 1.5.0; Apache-2.0; re-cloned at T0 this batch,
//! every quote byte-verified at the pin — the Research-345 refs are dated
//! 2026-10-28 and the pin had not moved; corrections recorded in-rule where
//! the staged line paraphrased). ORIGINAL Rust fixtures
//! (`corpus/fixtures/b358_fixtures.rs`, 5/5 standalone-compiled AND executed
//! before entry), never pasted source — the accelerator service is a plain
//! struct with a compile ledger. ROUTING (research 345's routing line, set
//! per-row below): `rust_perf` — the wave's serving leads landed here (the
//! B270/271 serving family); `kernel_opt`'s "host-orchestration around GPU"
//! clause was the alternative home and loses because these are HOST policies
//! with no kernel we author (the kernels stay the source's own Metal/C++
//! bytes). Batch 358 / Plan 369 / Research 345 (lane B).

#[allow(clippy::too_many_lines)] // the corpus entries ARE the payload
pub fn entries() -> Vec<crate::domains::rust_perf::PerfRule> {
    vec![
        // ── R1 — the GOAT lead of the residue: the accelerator service bills
        // intermediate memory PER LOADED PROGRAM, so one-function programs
        // multiply the residency by the function count; ONE program with N
        // row-length functions shares one surface set. Compile once per
        // source hash. ─────────────────────────────────────────────────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "one-program-many-functions-accelerator-service-memory".to_string(),
            name: "an accelerator service that bills intermediate memory per LOADED program multiplies the working set by the program count — group the row-length variants into ONE program whose functions share one surface set, compiled once per source hash".to_string(),
            category: "layout".to_string(),
            buggy_code: r#"// NAIVE: one compiled program per row-length variant. The service
// holds intermediate memory for EACH loaded program, so 13 variants
// hold the surface set 13 times; a per-variant compile also pays the
// cold compile every time a new length appears.
const ROWS: usize = 2048;
const MAX_LEN: usize = 2048;

struct Service {
    programs: Vec<(u64, usize)>, // (source hash, billed bytes)
    compiles: usize,
}

impl Service {
    fn load_one_function_per_len(&mut self, lens: &[usize]) {
        for len in lens {
            let hash = fnv1a(format!("fn(rows={ROWS}, len={len})").as_bytes());
            if self.programs.iter().any(|(h, _)| *h == hash) {
                continue; // warm: compile-once per source hash
            }
            self.compiles += 1;
            // each program holds its own full surface set
            self.programs.push((hash, ROWS * MAX_LEN * 4));
        }
    }
    fn residency(&self) -> usize {
        self.programs.iter().map(|(_, b)| b).sum()
    }
}"#.to_string(),
            fixed_code: r#"// REMEDY: ONE program with N row-length functions sharing ONE surface
// set sized for the largest rows — residency is the MAX, never the
// sum; the whole group is one source, one hash, one compile.
// REGIME: any accelerator runtime (NPU/Core ML/ANE-class) whose
// service bills intermediate memory per LOADED program, with a family
// of row-length or shape variants of one operator.
// ANTI-REGIME: runtimes billing memory per OPERAND or per EXECUTION
// (grouping buys nothing); variants with genuinely disjoint surface
// shapes (a shared set would be sized for the union and waste);
// variants compiled for different targets where one program cannot
// hold them.
impl Service {
    fn load_one_program_many_functions(&mut self, lens: &[usize]) {
        let mut src = String::from("fn(group)");
        for len in lens {
            src.push_str(&format!(",f{len}"));
        }
        let hash = fnv1a(src.as_bytes());
        if self.programs.iter().any(|(h, _)| *h == hash) {
            return;
        }
        self.compiles += 1; // ONE compile for the whole group
        // one shared surface set, sized for the largest rows
        self.programs.push((hash, ROWS * MAX_LEN * 4));
    }
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (residency is billed by the accelerator's own service at load — no CPU validator hosts an ANE-class runtime; the dial9/Batch-136 annotation). ROUTING: rust_perf — a HOST residency/compile policy (how many programs a service loads); kernel_opt's host-orchestration clause was the alternative and loses because no kernel we author is involved (the source's kernels stay theirs). SOURCE (incoai/splash @ cabbad27, Apache-2.0, tag 1.5.0; T0 re-verified this batch). THE LAW IN-SOURCE (docs/internals.md, the W8A8 ANE program section, byte-verified at the pin): 'a chunk takes the smallest of its functions that holds it, one every 128 rows from 512 to 2048, all reading and writing one set of surfaces sized for 2048 rows. The ANE service holds memory for a loaded program's intermediate values, which its functions share: 13 programs of one function each held it 13 times (3.25 GB at the M6's share, against 0.67 GB for the one program).' — the 13x/3.25-GB-vs-0.67-GB axis. COMPILE-ONCE: runtime/ane/Program.mm:629 'The source's hash names its directory and the service's key' over the FNV-1a of the source bytes (:266-271); warm load AneFfn.cpp:62 'loads one it has compiled in 50-150 ms on an M5 Max' (the staged '~0.1 s' paraphrase corrected to the verbatim range; the cold compile budget comment :54 'four times the cold compile of the largest the split compiles'). REGIME + ANTI-REGIME above. ADJUDICATIONS (do not double-count): reserve-then-commit-capacity-accounting (entries_58, B324) owns the reserve/commit LEDGER inside ONE store — this owns the PROGRAM-COUNT multiplier on billed residency; command-buffer-declared-input-pacing (kernel_opt entries_gpu_152, B337) owns per-call DECLARED-INPUT commit pacing at submission — this owns load-time program grouping; launch-time-code-as-data-l2-warm (entries_gpu_156, B354) owns making kernel TEXT L2-hot at launch — neither touches how many programs hold the intermediates. FIXTURE (b358_fixtures K1): 13 one-function programs hold exactly 13x the one-program residency at the same coverage; warm load compiles nothing. Cite: @ cabbad27 — docs/internals.md (the W8A8 program section); runtime/ane/Program.mm:266-271,629; runtime/ops/AneFfn.cpp:54-62. Batch 358 / Plan 369 / Research 345 (lane B #1).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R2 — the live split calibration: fit from a low + high share
        // measurement, carry the bandwidth-interference term, choose by
        // argmin ±1 unit with a 5% gain floor — never a static knob. ──────
        crate::domains::rust_perf::PerfRule {
            rule_id: "ane-split-calibration-fit-bandwidth-interference-term".to_string(),
            name: "a live split fraction between a host device and an accelerator is FIT per device from two share measurements — the GPU line through both alone points, the accelerator line off the high share, and u the bandwidth the accelerator's part takes from the GPU's beside it — then chosen by argmin over ±1 unit with a 5% gain floor; never a static config knob".to_string(),
            category: "measurement".to_string(),
            buggy_code: r#"// NAIVE: the split is a compile-time constant — one number for every
// device, tuned once on one box. On the boxes where the accelerator's
// part behaves differently (its bandwidth steals from the GPU's, or
// its share runs longer), the constant mis-splits every chunk.
const SPLIT_AT: usize = 8; // tuned on one M6, shipped everywhere

fn run_chunk(rows: usize, gpu: &mut f64, ane: &mut f64) {
    // the constant never sees the device it lands on
    if rows >= SPLIT_AT {
        *ane += rows as f64 * 0.02;
    } else {
        *gpu += rows as f64 * 0.01;
    }
}"#.to_string(),
            fixed_code: r#"// REMEDY: fit T(s) = max(A(s), G(s) + u·A(s)) from a low and a high
// share measurement per device; choose the split by argmin over ±1
// unit with a 5% gain floor vs GPU alone; no candidate clears the
// floor ⇒ the split is REFUSED (None), never a default. Remember the
// calibration, forget it on a trip (see the breaker rule).
#[derive(Clone, Copy)]
struct Timing { share: f64, gpu_ms: f64, split_ms: f64 }

struct Model { g0: f64, slope: f64, ane: f64, u: f64 }

fn fit(low: Timing, high: Timing) -> Model {
    assert!(low.share < high.share);
    let slope = (high.gpu_ms - low.gpu_ms) / (high.share - low.share);
    let mut m = Model { g0: low.gpu_ms - slope * low.share, slope,
                        ane: high.split_ms / high.share, u: 0.0 };
    let gpu = m.gpu(low.share);
    if gpu > m.ane {
        // the low-share excess over the GPU's own part IS the interference
        m.u = ((low.split_ms - gpu) / m.ane).clamp(0.0, 1.0);
    }
    m
}

impl Model {
    fn gpu(&self, s: f64) -> f64 { self.g0 + self.slope * s }
    fn total(&self, s: f64) -> f64 {
        let a = self.ane;
        a.max(self.gpu(s) + self.u * a) // max, never a sum
    }
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (the fit's quality is priced per-device on accelerator hardware — the M6/M5-class timings are the source's own measurements, provenance never bars). ROUTING: rust_perf — a host-side measurement/admission policy. SOURCE (incoai/splash @ cabbad27, Apache-2.0, T0 byte-verified). THE LAW IN-SOURCE (runtime/ops/AneFfnCalibration.cpp): kMinimumGain = 0.05 (:16); the fit comment :55-69 verbatim on u — 'exceeds it by the bandwidth the ANE's part takes beside it, which gives u: on an M6 the GPU's part runs a tenth to a third longer beside the ANE's, on an M5 Pro 3-7% near its fastest share. Where the ANE's part takes the longer there, as on an M5 Max, the split layer's excess over A is the ANE's own' — the per-device spread IS the law; fit() :70-77 'model.u = std::clamp((low.split - gpu) / ane, 0.0, 1.0)'; the 5% floor at :100 'if (!(at[chosen] <= (1.0 - kMinimumGain) * gpuAlone)) return std::nullopt' — no gain ⇒ the split REFUSED; minimum-rows chain :104-123 'A chunk splits while its function's split layer takes at most kChunkNoise longer than the GPU's layer alone' with kChunkNoise = 0.02 (:22) and the per-function row economics 'the ANE's part runs them and takes the longer. The functions do not cost the ANE in proportion to their rows (on an M5 Max 640 rows took 93% of 768's, 512 rows 73%), so each is timed'; median-of-3 AneFfnMeasurement.cpp:22-27 (kRuns = 3, 'which passes over a first run') with the GPU-alone FFN stand-in subtraction (:86 'usable(median(command) - standIn)'). REGIME: any host+accelerator split calibrated at startup from per-device timings. ANTI-REGIME: single-device firmware where one measured constant IS the calibration (there is no per-device spread to fit); splits with no interference coupling (u ≡ 0 degenerates to two independent lines). ADJUDICATIONS: calibrate-verify-window-on-text-not-proposer-chain (entries_41, B270/271) owns calibrating a verify WINDOW on the RIGHT SIGNAL (text, not the proposer chain) — this owns the per-device FIT SHAPE (two-line model + interference term) of a split fraction; launch-heuristic-pinned-to-calibration-arch (kernel_opt entries_gpu_23, B111) owns never extrapolating a tuned ladder off its calibration ARCH — this rule is that law's POSITIVE INSTANCE (u measured per device: M6 0.1-0.33 / M5 Pro 3-7% / M5 Max 0, cited in-rule); entries_19 (B219) owns capture-time NOISE REPAIR of a persisted table — this owns the fit itself. FIXTURE (b358_fixtures K2): the max-never-sum total, the clamped u, the floor refusing a no-gain split, the ±1-unit choice. Cite: @ cabbad27 — runtime/ops/AneFfnCalibration.cpp:16,22,55-77,100-123; runtime/ops/AneFfnMeasurement.cpp:22-27,86. Batch 358 / Plan 369 / Research 345 (lane B #2).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R3 — the lost-breaker: a fast path judged over a bounded window
        // against the accumulated baseline trips at the window, retires, and
        // FORGETS the calibration so the next start re-derives. ────────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "accelerator-lost-breaker-forgets-calibration".to_string(),
            name: "an accelerator fast path that can silently lose to the serial baseline is judged over a BOUNDED window of commands against the ACCUMULATED baseline line — at the window the loser retires AND the remembered calibration is deleted so the next start re-derives it; the window resets whether or not it trips".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// NAIVE: the fast path is armed from a remembered calibration and
// trusted forever — no comparison against the baseline, or a bare
// threshold that trips on the FIRST slow command (one cold start or a
// busy neighbour kills the path), or a breaker that retires the path
// but KEEPS the calibration (every restart re-loses until someone
// deletes the file by hand).
struct Split {
    calibration: Calibration, // remembered from the last run
}

impl Split {
    fn run(&mut self, rows: u32, ms: f64) {
        // trusted forever: whatever the calibration says, goes
        let _ = (rows, ms);
    }
}"#.to_string(),
            fixed_code: r#"// REMEDY: accumulate fast-path ms and the baseline line (evaluations x
// gpu_alone(rows)) over a bounded window; at the window boundary judge
// fast >= baseline (the ratio construction reads BOTH sums BEFORE the
// reset), retire on trip, fire forget() — calibration amnesia — and
// reset every window. The bounded window weighs ONE slow evaluation
// (a first-after-reload command) among the whole window's commands.
struct Breaker<F: Fn(u32) -> f64> {
    gpu_alone: F,
    window: u32,
    commands: u32,
    fast_ms: f64,
    baseline_ms: f64,
}

impl<F: Fn(u32) -> f64> Breaker<F> {
    fn add(&mut self, rows: u32, evaluations: u32, ms: f64,
           forget: &mut dyn FnMut()) -> Option<String> {
        self.fast_ms += ms;
        self.baseline_ms += f64::from(evaluations) * (self.gpu_alone)(rows);
        self.commands += 1;
        if self.commands < self.window {
            return None;
        }
        let trip = self.fast_ms >= self.baseline_ms;
        let ratio = self.fast_ms / self.baseline_ms;
        self.commands = 0; // the window resets EVERY window, trip or not
        self.fast_ms = 0.0;
        self.baseline_ms = 0.0;
        if trip {
            forget(); // amnesia: the next start re-derives from measurement
            return Some(format!("losing to the GPU alone ({ratio:.1}x)"));
        }
        None
    }
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (the trip economics are accelerator-visited — no validator hosts the split; the dial9/Batch-136 annotation). ROUTING: rust_perf — a host-side admission/invalidation policy over a measured fast path. SOURCE (incoai/splash @ cabbad27, Apache-2.0, T0 byte-verified). THE LAW IN-SOURCE (runtime/ops/AneFfnCalibration.hpp:100-104 'over each kWindow commands it finishes, the Neural Engine's milliseconds' with kWindow = 8; .cpp:182-194 Breaker::add — 'ane_ += milliseconds; alone_ += evaluations * (*gpu_)(rows); if (++commands_ < kWindow) return {}; ... if (!(ratio >= 1.0)) return {}; reason << ... \"losing to the GPU alone (\" << ratio * most << \" ms on the Neural Engine per \" << mostRows << \"-row layer against \" << most << \" ms on the GPU alone)\"' — the sums read BEFORE the reset (:188-189 reset happens after ratio; the fixture pins the read-before-reset shape); the measured headroom comment :165-171 'another process keeping the ANE busy beside the split costs it 2-8% on an M5 Max, an M5 Pro and an M6, and a minute of prefill slows the M6's ANE by a third: far short of the trip point. Judging kWindow commands at once, it weighs one slow evaluation ... among all of theirs'; THE AMNESIA (runtime/engine/AneFfnStartup.cpp:188-195, byte-verified) — 'The split takes the calibration's least chunk, and judges itself against the GPU alone as calibrated, forgetting the calibration once it' [loses: the breaker's trip callback] 'prepared.split->setBreaker(ane_ffn::Breaker(calibration.gpu), [forget = model.forget, maxAneUnits] { forget(maxAneUnits); })' — the trip fires the model's forget, deleting the remembered calibration. REGIME: any accelerator/DMA/fast-path admitted from a remembered calibration that can silently lose (a busier neighbour, a thermally slower part). ANTI-REGIME: paths whose loss is IMPOSSIBLE by construction (a correctness guard already refuses); one-shot batch jobs with no steady state to window. ADJUDICATIONS: bounded-peer-handoff-wait (entries_22/59, B270/271) owns bounded WAIT on a peer's PROGRESS (give-up on silence) — this owns give-up on being SLOWER THAN THE BASELINE (a winning-but-slower-than-serial path); entries_19 (B219, the TensorFold cost-rule law) owns CAPTURE-time noise repair + persistence per build identity — this owns RUNTIME ADMISSION of the calibrated path + the amnesia half B219 deliberately does not have (its table is re-derivable per build; splash's calibration must be FORGOTTEN on evidence it lies); the load-admission rule (entries_21, the real-weight probe with two failure classes) admits at LOAD — this is the steady-state WATCH over the admitted path; the calibration-amnesia axis is ZERO-OWNER before this rule (grep 'amnesia|forget.*calibrat|breaker' over rust_perf/sec corpora: nothing). FIXTURE (b358_fixtures K3): trip lands at the window boundary (7 fast + 1 slow), forget fires exactly once, the reset window does not double-trip. Cite: @ cabbad27 — runtime/ops/AneFfnCalibration.hpp:100-104; runtime/ops/AneFfnCalibration.cpp:165-194; runtime/engine/AneFfnStartup.cpp:188-195,205-207. Batch 358 / Plan 369 / Research 345 (lane B #3).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R4 — the write-class share partition: one in-flight budget,
        // partitioned per write class with reserved minimums, so a flood in
        // one class never starves another and reclaim always keeps room. ──
        crate::domains::rust_perf::PerfRule {
            rule_id: "writeclass-share-partition-inflight-writes".to_string(),
            name: "a bounded in-flight write budget shared by write classes with different urgency is PARTITIONED per class — reclaim all, persist a quarter, spill half, each with a reserved minimum of one — so one class's flood never starves another and the reclaim class always keeps room; admission requires BOTH the class share and the total budget".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// NAIVE: one FIFO budget for every write class. A spill flood (bulk
// prefetch to disk) fills ALL 64 in-flight slots; the persist writer
// (durability) and the reclaim writer (eviction) starve until the
// flood drains — the durability write waits behind bulk traffic.
const LIMIT: usize = 64;

struct Tier {
    in_flight: Vec<Write>,
}

impl Tier {
    fn can_write(&self) -> bool {
        self.in_flight.len() < LIMIT // one class's flood owns the budget
    }
}"#.to_string(),
            fixed_code: r#"// REMEDY: partition the budget per class — reclaim the whole limit,
// persist limit/4, spill limit/2, each with a reserved minimum of one;
// a class admits only under its share AND the total. One kind never
// starves another, and reclaim always keeps room.
#[derive(Clone, Copy)]
enum WriteClass { Reclaim, Persist, Spill }

fn share(write: WriteClass, limit: u32) -> u32 {
    match write {
        WriteClass::Reclaim => limit,
        WriteClass::Persist => (limit / 4).max(1),
        WriteClass::Spill => (limit / 2).max(1),
    }
}

struct Tier {
    limit: u32,
    counts: [u32; 3],
}

impl Tier {
    fn can_admit(&self, write: WriteClass) -> bool {
        let total: u32 = self.counts.iter().sum();
        let i = write as usize;
        total < self.limit && self.counts[i] < share(write, self.limit).max(1)
    }
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (starvation-freedom under a mixed write load is a serving-runtime property — the dial9/Batch-136 annotation). ROUTING: rust_perf — a host-side concurrency/budget law. SOURCE (incoai/splash @ cabbad27, Apache-2.0, T0 byte-verified). THE LAW IN-SOURCE (runtime/engine/KvPageTier.hpp:32, verbatim): 'that one kind never starves another and reclaim always keeps a quarter' — note the staged line said 'reclaim always keeps a quarter' and the source comment's reclaim share IS the whole limit (share(): Reclaim → writeLimit_, Persist → max(1, writeLimit_/4), Spill → max(1, writeLimit_/2), :217-225 byte-verified; the comment's 'quarter' binds the PERSIST share and the reclaim-keeps-room posture, the fixture pins the code shape); kWrites = 64 (:67 'static constexpr uint32_t kWrites = 64'); canDemote checks 'writes_.size() < writeLimit_ && classWrites_[write] < share(write) && writable()' (:227-231) — BOTH the class share and the total; the tier-stacking sibling KvTier.hpp:37 'flight (KvPageTier), so that one kind never starves another'. REGIME: a bounded in-flight write budget shared by classes of different urgency (reclaim/persist/spill, or compaction/wal/cold-tier). ANTI-REGIME: single-class write paths (no flood to partition against); unbounded budgets (starvation needs a shared finite budget); classes whose correctness REQUIRES strict FIFO across classes. ADJUDICATIONS: reserve-then-commit-capacity-accounting (entries_58, B324) owns the ADVERTISE-THEN-MEASURE capacity ledger for in-flight writes of approximately-known size — this owns the CLASS-SHARE partition of the budget (who may hold how many slots), the ledger rule's orthogonality both directions (reserve/commit without shares still starves; shares without a ledger still over-advertises); slow-consumer-latest-value-single-slot (entries_70, B345) owns latest-wins COALESCING for a slow consumer — this owns multi-writer slot quotas (coalescing shrinks demand, quotas partition capacity); fanout-first-error-leaves-siblings-running (entries_64, B338) owns cancellation on first error — unrelated axis, ruled out. FIXTURE (b358_fixtures K4): at limit 64 a 200-write spill flood admits exactly 32, persist still admits, reclaim always admits, the total still binds at 64, and a limit-2 budget admits one of each class (the reserved minimum). Cite: @ cabbad27 — runtime/engine/KvPageTier.hpp:32,67,217-231; runtime/engine/KvTier.hpp:37. Batch 358 / Plan 369 / Research 345 (lane B #4).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R5 — the checked keep-awake spin: a capability whose memory
        // scope the public headers lack is built separately, CHECKED at
        // runtime (the spin must observe the CPU's end), and degrades LOUD —
        // warn + run without — never silently skipped. ─────────────────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "spin-keep-awake-checked-capability-degrade-loud".to_string(),
            name: "a clock-holding spin whose memory scope the platform's public headers lack is built as a SEPARATE artifact, and before the first command relying on it the backend CHECKS the spin actually observes the CPU's end signal — without the artifact, or on a failed check, the steps run without the spin behind a loud warning; never silently skipped".to_string(),
            category: "thermal".to_string(),
            buggy_code: r#"// NAIVE A: the spin is wired unconditionally from a header constant —
// on the platforms where the scope does not compile or does not see
// the writer's end flag, the GPU spins PAST the CPU's end (a hang at
// best). NAIVE B: the spin is silently #ifdef'd out where the header
// lacks the scope — the clock residency quietly disappears on those
// boxes and nobody knows why the short steps run cold.
fn arm_spin(built: bool) -> Spin {
    // trusted: if it compiled, it works (NAIVE A)
    // or: silently absent when it did not (NAIVE B)
    if built { Spin::Spinning } else { Spin::Absent }
}"#.to_string(),
            fixed_code: r#"// REMEDY: the spin builds as its OWN artifact; before the first
// command relying on it, run a round-trip probe — does the spin
// OBSERVE the CPU's end flag through its scope? Armed only on a
// passing probe; otherwise degrade LOUD (a counted warning) and run
// the steps without the spin. The check is load-bearing: a spin that
// cannot see the end would spin forever.
enum KeepAwake {
    Armed,
    Degraded { reason: &'static str, warned: bool },
}

fn arm_spin(lib: &SpinLibrary) -> KeepAwake {
    if !lib.built {
        // a compiler that refuses the artifact only WARNS
        return KeepAwake::Degraded {
            reason: "spin library refused by the compiler", warned: true,
        };
    }
    if !lib.scope_works {
        // the round-trip check BEFORE the first command that needs it
        return KeepAwake::Degraded {
            reason: "spin scope failed the round-trip check", warned: true,
        };
    }
    KeepAwake::Armed
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (clock residency is priced on the accelerator's clocks — the source's own numbers are the M-class wake-up figures, provenance never bars). ROUTING: rust_perf — a host-side capability/degrade law. SOURCE (incoai/splash @ cabbad27, Apache-2.0, T0 byte-verified). THE LAW IN-SOURCE (docs/internals.md, the host-step section, byte-verified): 'A step about as short as the GPU's wake-up can keep the GPU awake: one GPU thread spins through it until the CPU ends it, so the work after it starts at once and at full clocks. The spin sees the CPU end it only through a memory scope Metal's public headers leave out (runtime/metal/kernels/host_step.metal), so it builds apart from the metallib, into build/host-step.metallib, and a Metal compiler that refuses it only warns. Before the first command with such a step, the backend checks that the spin sees the CPU end it; without the library, or if it does not, the steps run without the spin, with a warning.' — the THREE atoms: separate build artifact; the round-trip CHECK before reliance; loud run-without. The spin's own kernel lives at runtime/metal/kernels/host_step.metal (the built-apart artifact). REGIME: an optimization whose memory-ordering/capability precondition the platform's public ABI cannot express (custom scopes, non-portable intrinsics, experimental extensions). ANTI-REGIME: capabilities the platform TYPES (the compiler refuses loudly at build — no runtime check to add); optimizations with a SAFE silent fallback cheaper than the check (the check must be load-bearing: here a spin that cannot see the end spins forever, so the probe is the only sound admission). ADJUDICATIONS: metal-keepalive-residency-unwire + its faces (entries_21 — the B357 F1 FREQUENCY face owns the idle window's frequency/residency/yield OBLIGATIONS for work that is NOT running; this owns the ACTIVE-side clock-hold spin AROUND a short step, plus the checked-admission axis); the idle-server park law (entries_02's cubecl rider — park with producer unpark, never fixed-sleep) is the OPPOSITE pole and the ANTI-REGIME: that law governs an IDLE server (a spin there burns CPU for nothing); THIS governs a bounded window where the work after the step starts at full clocks — the two rules compose (idle: park; active short-step: checked spin), never merge; the load-admission rule (entries_21, real-weight probe) admits at LOAD by numerics — this admits at BOOT by an OBSERVATION probe of a concurrency capability, and degrades at runtime rather than disabling the load. FIXTURE (b358_fixtures K5): armed passes the round trip; a failed scope or a refused build degrades loudly (counted warning, once) and the step still runs without the spin (slower, never hung, never silent). Cite: @ cabbad27 — docs/internals.md (the host-step section, :968-979 at the T0 read); runtime/metal/kernels/host_step.metal (the artifact's home). Batch 358 / Plan 369 / Research 345 (lane B #5).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
