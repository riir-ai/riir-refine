//! Perf-bench Pareto validator — runs generated microbenchmarks across multiple
//! input sizes and reports a Pareto verdict (faster on N of M sizes).
//!
//! This is the **promotion gate** for the `rust_perf` domain (Proposal 001
//! caveat 3). The existing `rust_perf_verify` runs `cargo test` (compilation +
//! test pass) — a *degraded* validator with no perf claim. This module adds
//! the real perf ground truth: does the fixed code actually run faster?
//!
//! Behind the `rust_perf_bench` feature gate (opt-in, default-off) because:
//! - **Slow**: each rule spawns a cargo build + bench subprocess (seconds)
//! - **Environment-dependent**: requires cargo on PATH
//! - **Noise**: microbenchmarks are inherently noisy (mitigated via warmup +
//!   dynamic calibration + median-of-samples, but still a measurement, not a proof)
//!
//! ## Methodology
//!
//! 1. Generate a temp **bin** crate embedding the rule's `BenchSpec.harness`
//!    + a timing loop (dynamic calibration → warmup → median-of-samples)
//!    + a counting `#[global_allocator]` (Issue 014 T1).
//! 2. Run `cargo run --release` on the temp crate.
//! 3. Parse JSON lines:
//!    `{"size":N,"buggy_ns":X,"fixed_ns":Y,"buggy_allocs":A,"buggy_bytes":B,"fixed_allocs":C,"fixed_bytes":D}`.
//! 4. Compute per-size outcome (Faster/Slower/Tie at ±5% threshold).
//! 5. Aggregate into a [`ParetoVerdict`] (ParetoDominant if faster on majority).
//!
//! ## The deterministic alloc axis (Issue 014 T1, Research 032)
//!
//! Wall-clock timing is noisy (±5% Tie band; documented regime-fragility in
//! Benches 015/016). Allocation count + bytes are **exact integers that are
//! identical on every run** — the same code path allocates the same bytes
//! deterministically. The generated crate installs a counting
//! `#[global_allocator]` (alloc + realloc + alloc_zeroed counted; dealloc
//! is free) and, after the timing loops, measures one steady-state call of
//! each side with the counters reset — warmup has already settled one-time
//! init paths (LazyLock compilation, first-use lazies).
//!
//! **Verdict axis unchanged**: timing stays the Pareto verdict axis; the alloc
//! integers are reported + recorded, and gate rules whose timing is Tie-class
//! (e.g. `arc-str-payload`, Bench 020). Determinism is assertable across
//! repeat runs via [`PerfBenchValidator::run_bench_with_determinism`] — any
//! variance there is a harness bug, not noise. Precedent shapes: dhat-rs
//! heap-usage testing; riir-mmorpg-examples G4 `CountingAllocator` gates.
//!
//! ## Modelless-first
//!
//! `cargo run` + `std::time::Instant` are deterministic runners — no ML, no
//! training. The timing *measurement* is noisy, but the verdict logic
//! (median + threshold comparison) is deterministic given the measurements.
//! This satisfies the modelless-first mandate.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// A benchmark specification for a perf rule.
///
/// Rules without a bench spec are [`BenchVerdict::NotBenchable`] — honest
/// marking for rules that need external runtimes (Bevy, WASM) a temp crate
/// cannot provide.
///
/// The `harness` is a **self-contained Rust source fragment** that defines:
/// - `pub fn make_input(n: usize) -> <Input>` — builds the input for size `n`
/// - `pub fn run_buggy(input: &<Input>)` — exercises the buggy pattern
/// - `pub fn run_fixed(input: &<Input>)` — exercises the fixed pattern
///
/// The harness embeds the buggy + fixed code inline (with any needed `pub`
/// modifiers). The validator wraps this fragment with a timing loop + compiles
/// it as a bin crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchSpec {
    /// Input sizes to sweep (e.g., `[10, 100, 1000, 10000]`).
    pub sizes: Vec<usize>,
    /// Self-contained Rust source defining `make_input` + `run_buggy` + `run_fixed`.
    pub harness: String,
}

/// Rules whose alloc axis is EXEMPT from the G-det determinism gate —
/// a measured platform class, not a harness bug: `std::sync::Mutex`
/// contention on macOS allocates nondeterministically in the pthread
/// slow path (probed 2026-08-27, Batch 78: global-mutex 12t×20K bumps
/// measured 41/44/42/41/41 allocs across runs while UNCONTENDED mutex
/// construction is exactly stable at 513 for 512 locks — the variance
/// is the contended wait path, which the harness cannot exclude without
/// removing the contention it exists to measure). Timing verdicts for
/// these rules are unaffected (median-of-samples); only the alloc-count
/// comparison is skipped. Add a rule here ONLY with probe evidence.
pub const ALLOC_AXIS_EXEMPT: &[&str] = &[
    "cache-line-block-partitioned-lock",
    // Batch 135 (partitioned-mut-slices-zero-sync): the buggy arm locks a
    // single std Mutex PER BUMP across all workers — the same contended
    // macOS pthread slow path the Batch-78 probe measured allocating
    // nondeterministically (41/44/42/41/41 allocs across 12t runs). Same
    // platform class, no fresh probe; the partition arm allocates
    // deterministically. Timing verdicts unaffected (median-of-samples).
    "partitioned-mut-slices-zero-sync",
];

/// The per-size outcome of a bench comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeOutcome {
    /// Fixed code is ≥5% faster than buggy at this size.
    Faster,
    /// Fixed code is ≥5% slower than buggy at this size.
    Slower,
    /// Within ±5% (noise band).
    Tie,
}

/// A single size's measurement result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeResult {
    pub size: usize,
    /// Median ns/iter for the buggy code.
    pub buggy_ns: u64,
    /// Median ns/iter for the fixed code.
    pub fixed_ns: u64,
    /// `buggy_ns / fixed_ns` (>1.0 = fixed is faster).
    pub speedup: f64,
    pub outcome: SizeOutcome,
    /// Steady-state allocation count for ONE `run_buggy` call (deterministic
    /// second axis, Issue 014 T1). Measured after warmup so one-time init
    /// paths (LazyLock compile, first-use lazies) are excluded.
    #[serde(default)]
    pub buggy_allocs: u64,
    /// Steady-state allocated bytes for ONE `run_buggy` call.
    #[serde(default)]
    pub buggy_alloc_bytes: u64,
    /// Steady-state allocation count for ONE `run_fixed` call.
    #[serde(default)]
    pub fixed_allocs: u64,
    /// Steady-state allocated bytes for ONE `run_fixed` call.
    #[serde(default)]
    pub fixed_alloc_bytes: u64,
}

/// The overall Pareto verdict across all sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BenchOverall {
    /// Fixed code is faster on the majority of sizes (the promotion bar).
    ParetoDominant,
    /// Faster on some sizes, slower on others.
    Mixed,
    /// Fixed code is slower on the majority (a regression).
    ParetoDominated,
}

/// The full verdict from a perf-bench Pareto comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParetoVerdict {
    pub rule_id: String,
    pub results: Vec<SizeResult>,
    pub faster_count: usize,
    pub slower_count: usize,
    pub tie_count: usize,
    pub overall: BenchOverall,
    /// Whether `cargo run` executed successfully.
    pub ran: bool,
    /// Error message if `ran` is false.
    pub error: Option<String>,
}

/// The verdict for a rule that has no `BenchSpec`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotBenchable {
    pub rule_id: String,
    pub reason: String,
}

/// The union verdict from the bench validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchVerdict {
    Pareto(ParetoVerdict),
    NotBenchable(NotBenchable),
}

impl BenchVerdict {
    /// `true` if the rule passed its bench gate (Pareto-dominant).
    #[must_use]
    pub fn is_pareto_dominant(&self) -> bool {
        matches!(
            self,
            BenchVerdict::Pareto(ParetoVerdict {
                overall: BenchOverall::ParetoDominant,
                ..
            })
        )
    }

    /// `true` if the rule was honestly skipped (no bench spec).
    #[must_use]
    pub fn is_not_benchable(&self) -> bool {
        matches!(self, BenchVerdict::NotBenchable(_))
    }
}

/// Classify a speedup ratio into a per-size outcome.
///
/// - `speedup > 1.05` → Faster (fixed is ≥5% faster)
/// - `speedup < 0.95` → Slower (fixed is ≥5% slower)
/// - otherwise → Tie (within the ±5% noise band)
#[inline]
fn classify_speedup(speedup: f64) -> SizeOutcome {
    const FASTER_THRESHOLD: f64 = 1.05;
    const SLOWER_THRESHOLD: f64 = 0.95;
    if speedup > FASTER_THRESHOLD {
        SizeOutcome::Faster
    } else if speedup < SLOWER_THRESHOLD {
        SizeOutcome::Slower
    } else {
        SizeOutcome::Tie
    }
}

/// Aggregate per-size outcomes into an overall verdict.
fn aggregate_outcomes(results: &[SizeResult]) -> BenchOverall {
    let total = results.len();
    let faster = results
        .iter()
        .filter(|r| r.outcome == SizeOutcome::Faster)
        .count();
    let slower = results
        .iter()
        .filter(|r| r.outcome == SizeOutcome::Slower)
        .count();
    if total == 0 {
        return BenchOverall::Mixed;
    }
    if faster > slower && faster * 2 > total {
        BenchOverall::ParetoDominant
    } else if slower > faster && slower * 2 > total {
        BenchOverall::ParetoDominated
    } else {
        BenchOverall::Mixed
    }
}

/// The deterministic alloc axis of a bench run (Issue 014 T1.4): the
/// `(size, buggy_allocs, buggy_alloc_bytes, fixed_allocs, fixed_alloc_bytes)`
/// tuple per size. Two runs with equal axes allocated identically — the
/// determinism proof behind [`PerfBenchValidator::run_bench_with_determinism`].
/// Timing medians are deliberately excluded (wall-clock noise is the axis's
/// whole reason to exist).
fn alloc_axis(results: &[SizeResult]) -> Vec<(usize, u64, u64, u64, u64)> {
    results
        .iter()
        .map(|r| {
            (
                r.size,
                r.buggy_allocs,
                r.buggy_alloc_bytes,
                r.fixed_allocs,
                r.fixed_alloc_bytes,
            )
        })
        .collect()
}

/// The bench validator — generates + runs microbenchmarks for perf rules.
pub struct PerfBenchValidator {
    cargo_bin: String,
    work_root: PathBuf,
}

impl Default for PerfBenchValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PerfBenchValidator {
    /// Create a validator using system cargo + default temp root.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cargo_bin: "cargo".to_string(),
            work_root: std::env::temp_dir().join("riir_perf_bench"),
        }
    }

    /// Override the cargo binary path.
    #[must_use]
    pub fn with_cargo_bin(cargo: impl Into<String>) -> Self {
        Self {
            cargo_bin: cargo.into(),
            work_root: std::env::temp_dir().join("riir_perf_bench"),
        }
    }

    /// Override the temp crate root directory.
    #[must_use]
    pub fn with_work_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.work_root = root.into();
        self
    }

    /// Check if cargo is available.
    #[must_use]
    pub fn is_cargo_available(&self) -> bool {
        Command::new(&self.cargo_bin)
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Run the bench validator for one rule.
    ///
    /// If the rule has a `BenchSpec`, generates a temp bin crate, runs the
    /// timing loop, and returns a [`ParetoVerdict`]. If not, returns
    /// [`BenchVerdict::NotBenchable`].
    #[must_use]
    pub fn validate(&self, rule_id: &str, spec: Option<&BenchSpec>) -> BenchVerdict {
        let Some(spec) = spec else {
            return BenchVerdict::NotBenchable(NotBenchable {
                rule_id: rule_id.to_string(),
                reason: "no BenchSpec (rule needs an external runtime or is structural)"
                    .to_string(),
            });
        };

        match self.run_bench(rule_id, spec) {
            Ok(results) => {
                let faster_count = results
                    .iter()
                    .filter(|r| r.outcome == SizeOutcome::Faster)
                    .count();
                let slower_count = results
                    .iter()
                    .filter(|r| r.outcome == SizeOutcome::Slower)
                    .count();
                let tie_count = results
                    .iter()
                    .filter(|r| r.outcome == SizeOutcome::Tie)
                    .count();
                let overall = aggregate_outcomes(&results);
                BenchVerdict::Pareto(ParetoVerdict {
                    rule_id: rule_id.to_string(),
                    results,
                    faster_count,
                    slower_count,
                    tie_count,
                    overall,
                    ran: true,
                    error: None,
                })
            }
            Err(e) => BenchVerdict::Pareto(ParetoVerdict {
                rule_id: rule_id.to_string(),
                results: vec![],
                faster_count: 0,
                slower_count: 0,
                tie_count: 0,
                overall: BenchOverall::Mixed,
                ran: false,
                error: Some(e),
            }),
        }
    }

    /// Generate the temp bin crate, compile, run, and parse results.
    fn run_bench(&self, rule_id: &str, spec: &BenchSpec) -> Result<Vec<SizeResult>, String> {
        let (crate_dir, stem) = self.prepare_crate(rule_id, spec)?;
        let stdout = self.run_cargo(&crate_dir)?;
        // ⛔ Issue 112: the binary that ran must be the one we generated.
        // The Issue 110 collision here is not a wrong verdict but a
        // MEASUREMENT THAT NEVER HAPPENED, reported as one — timings from
        // another rule's code, attributed to this rule_id. An absent line is
        // not agreement: a stale binary predating this echo yields None.
        match crate::domains::probe_witness::parse_probe_id(&stdout) {
            Some(id) if id == stem => {}
            other => {
                let saw = other.as_deref().unwrap_or("<no probe line>");
                return Err(format!(
                    "probe identity mismatch for `{rule_id}`: generated `{stem}`, the binary that ran \
                     said `{saw}` — these nanoseconds describe other code (Issue 110 scratch-crate \
                     unit collision); check for a shared CARGO_TARGET_DIR or a de-keyed [[bin]] name"
                ));
            }
        }
        Ok(Self::parse_bench_output(&stdout))
    }

    /// Run the bench `repeats` times and verify the alloc axis is
    /// bit-identical across runs (Issue 014 T1.4 — the determinism proof).
    ///
    /// Timing medians are NOT expected to match (wall-clock noise — that is
    /// the whole point of the axis); only the alloc integers must. Returns
    /// the FIRST run's full results plus whether every subsequent run's alloc
    /// axis agreed. A `false` here means a harness bug (background-thread
    /// allocation, timing-dependent lazy init), not measurement noise.
    ///
    /// The first run pays the cargo build; repeats reuse the cached build
    /// (BLAKE3-keyed crate dir) and only re-execute the binary.
    ///
    /// # Errors
    ///
    /// Returns `Err` when any run's crate preparation or `cargo run` fails
    /// (propagated from [`PerfBenchValidator::run_bench`]); the error string
    /// carries the truncated cargo stderr.
    ///
    /// # Panics
    ///
    /// Panics only on an internal invariant violation (`first_results` is
    /// always populated before the comparison loop, so the trailing
    /// `unwrap_or_default` is unreachable in practice).
    #[allow(clippy::too_many_lines)] // linear template assembler; splitting hurts readability
    pub fn run_bench_with_determinism(
        &self,
        rule_id: &str,
        spec: &BenchSpec,
        repeats: usize,
    ) -> Result<(Vec<SizeResult>, bool), String> {
        // Platform-exempt rules (see `ALLOC_AXIS_EXEMPT`): the alloc axis
        // measures OS slow-path allocation, not the code — report
        // deterministic without the exact-equality comparison.
        let exempt = ALLOC_AXIS_EXEMPT.contains(&rule_id);
        let mut first_results: Option<Vec<SizeResult>> = None;
        let mut deterministic = true;
        for _ in 0..repeats.max(1) {
            let results = self.run_bench(rule_id, spec)?;
            if first_results.is_none() {
                first_results = Some(results);
                continue;
            }
            if exempt {
                continue;
            }
            let first = first_results.as_ref().expect("set on first iteration");
            if alloc_axis(first) != alloc_axis(&results) {
                deterministic = false;
                break;
            }
        }
        Ok((first_results.unwrap_or_default(), deterministic))
    }

    /// Prepare the temp bin crate.
    ///
    /// BLAKE3-keyed dir name → cargo reuses build cache on repeat calls.
    fn prepare_crate(&self, rule_id: &str, spec: &BenchSpec) -> Result<(PathBuf, String), String> {
        // Hash rule_id + harness + sizes for a stable dir name.
        let mut hasher_input = String::new();
        hasher_input.push_str(rule_id);
        hasher_input.push('|');
        hasher_input.push_str(&spec.harness);
        hasher_input.push('|');
        for &s in &spec.sizes {
            hasher_input.push_str(&s.to_string());
            hasher_input.push(',');
        }
        let hash = blake3::hash(hasher_input.as_bytes());
        let hex = hash.to_hex();
        let stem = &hex[..16];
        let dir = self.work_root.join(stem);

        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir).map_err(|e| format!("create_dir_all failed: {e}"))?;

        // ⛔ The package AND `[[bin]]` names carry the key, not just the
        // directory (Issue 110). Two identically-named root packages sharing
        // a target dir are ONE cargo unit — and for a BIN that is worse than
        // a wrong verdict: `cargo run` re-executes the FIRST crate's binary.
        // Measured with a known answer — two `perf_bench` bins printing
        // different strings, one CARGO_TARGET_DIR: the second run printed
        // "I AM CRATE A", exit 0. This harness would have timed one rule's
        // code and reported the nanoseconds under another rule's id.
        let cargo_toml = format!(
            "[package]\n\
            name = \"perf_bench_{stem}\"\n\
            version = \"0.0.0\"\n\
            edition = \"2021\"\n\
            \n\
            [[bin]]\n\
            name = \"perf_bench_{stem}\"\n\
            path = \"src/main.rs\"\n"
        );

        let deps = Self::extract_deps(&spec.harness);
        let cargo_toml: String = if deps.is_empty() {
            cargo_toml
        } else {
            format!("{cargo_toml}\n[dependencies]\n{}", deps.join("\n"))
        };

        std::fs::write(dir.join("Cargo.toml"), cargo_toml)
            .map_err(|e| format!("write Cargo.toml failed: {e}"))?;

        // Generate src/main.rs: harness code + timing loop, carrying this
        // crate's key so the program states which probe it is (Issue 112).
        let main_rs = Self::generate_main_rs(spec, stem);
        std::fs::write(src_dir.join("main.rs"), main_rs)
            .map_err(|e| format!("write main.rs failed: {e}"))?;

        Ok((dir, stem.to_string()))
    }

    /// Extract [dep:...] markers from the harness to build Cargo.toml [dependencies].
    ///
    /// Convention: harness may include lines like `// [dep: papaya = "0.2"]` to
    /// declare crates.io deps. These are stripped from the Rust source + added
    /// to Cargo.toml as-is (the marker content IS the TOML dependency line).
    fn extract_deps(harness: &str) -> Vec<String> {
        let mut deps = Vec::new();
        for line in harness.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("// [dep:") {
                // Extract everything between `[dep:` and `]` as the TOML dep line.
                let dep = rest.trim_end_matches(']').trim();
                if !dep.is_empty() {
                    deps.push(dep.to_string());
                }
            }
        }
        deps.shrink_to_fit();
        deps
    }

    /// Generate the complete `src/main.rs`: harness (with dep markers stripped)
    /// + the counting allocator + the timing loop.
    ///
    /// Uses raw-string templates with `__SIZES__` placeholder + `.replace()`
    /// instead of nested `format!` — avoids the brace-escaping hell that
    /// comes from generating Rust source containing `println!` format strings.
    #[allow(clippy::too_many_lines)] // linear template assembler (harness + allocator + timing loop); splitting hurts readability
    fn generate_main_rs(spec: &BenchSpec, stem: &str) -> String {
        // Raw-string template — `{{` and `}}` here are LITERAL characters in
        // the generated Rust source (the runtime `println!` format string uses
        // them as escape sequences). No format! processing on this template.
        const TIMING_LOOP: &str = r#"
fn main() {
    // ⛔ Issue 112: state which probe this binary IS, before anything else.
    // `cargo run` has no --message-format stream to read a freshness flag
    // out of, so the executed PROGRAM is the only thing that can witness
    // its own identity here — and that is the stronger assertion anyway.
    // {{ and }} are escape sequences in the runtime println! format string.
    println!("{{\"probe\":\"__PROBE_ID__\"}}");
    let sizes: &[usize] = &[__SIZES__];
    let warmup_iters: u32 = 100;
    let sample_count: usize = 21; // odd → clean median
    let target_sample_ns: u64 = 1_000_000; // ~1ms per sample

    for &n in sizes {
        let input = make_input(n);

        // Warmup both functions (instruction cache + branch predictor).
        for _ in 0..warmup_iters {
            run_buggy(&input);
            run_fixed(&input);
        }

        // Dynamically calibrate iterations per sample for each function
        // so each sample targets ~1ms (avoids timer-noise floor for fast fns).
        let buggy_iters = calibrate(|| run_buggy(&input), target_sample_ns);
        let fixed_iters = calibrate(|| run_fixed(&input), target_sample_ns);

        // Measure buggy: take median of `sample_count` samples.
        let mut buggy_samples: Vec<u64> = Vec::with_capacity(sample_count);
        for _ in 0..sample_count {
            let t0 = std::time::Instant::now();
            for _ in 0..buggy_iters {
                run_buggy(&input);
            }
            let total_ns = t0.elapsed().as_nanos() as u64;
            buggy_samples.push(total_ns / buggy_iters.max(1));
        }
        buggy_samples.sort_unstable();
        let buggy_median = buggy_samples[sample_count / 2];

        // Measure fixed (interleave to reduce thermal/frequency bias).
        let mut fixed_samples: Vec<u64> = Vec::with_capacity(sample_count);
        for _ in 0..sample_count {
            let t0 = std::time::Instant::now();
            for _ in 0..fixed_iters {
                run_fixed(&input);
            }
            let total_ns = t0.elapsed().as_nanos() as u64;
            fixed_samples.push(total_ns / fixed_iters.max(1));
        }
        fixed_samples.sort_unstable();
        let fixed_median = fixed_samples[sample_count / 2];

        // Alloc-count pass (deterministic second axis, Issue 014 T1).
        // Warmup + timing above already settled one-time init paths
        // (LazyLock compilation, first-use lazies), so ONE call measures
        // the steady-state per-call allocation traffic exactly. Counters
        // are read into locals BEFORE println's own allocations.
        ALLOC_COUNT.store(0, std::sync::atomic::Ordering::Relaxed);
        ALLOC_BYTES.store(0, std::sync::atomic::Ordering::Relaxed);
        run_buggy(&input);
        let buggy_allocs = ALLOC_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let buggy_alloc_bytes = ALLOC_BYTES.load(std::sync::atomic::Ordering::Relaxed);

        ALLOC_COUNT.store(0, std::sync::atomic::Ordering::Relaxed);
        ALLOC_BYTES.store(0, std::sync::atomic::Ordering::Relaxed);
        run_fixed(&input);
        let fixed_allocs = ALLOC_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let fixed_alloc_bytes = ALLOC_BYTES.load(std::sync::atomic::Ordering::Relaxed);

        // Emit JSON line for the validator to parse.
        // {{ and }} are escape sequences in the runtime println! format string.
        println!("{{\"size\":{},\"buggy_ns\":{},\"fixed_ns\":{},\"buggy_allocs\":{},\"buggy_bytes\":{},\"fixed_allocs\":{},\"fixed_bytes\":{}}}",
            n, buggy_median, fixed_median,
            buggy_allocs, buggy_alloc_bytes, fixed_allocs, fixed_alloc_bytes);
    }
}

/// Calibrate iterations so one sample batch ≈ `target_ns`.
fn calibrate<F: Fn()>(f: F, target_ns: u64) -> u64 {
    let test_iters: u64 = 50;
    let t0 = std::time::Instant::now();
    for _ in 0..test_iters {
        f();
    }
    let per_iter_ns = t0.elapsed().as_nanos() as u64 / test_iters.max(1);
    let iters = target_ns / per_iter_ns.max(1);
    iters.clamp(1, 10_000_000)
}
"#;

        // Counting global allocator injected into every generated bench crate
        // (Issue 014 T1 — the deterministic alloc-count axis). Counts alloc +
        // realloc + alloc_zeroed (alloc_zeroed is overridden so it keeps
        // System's fast calloc path instead of the trait default that routes
        // through our alloc); dealloc is not counted (traffic axis, not heap-
        // residency axis). Overhead: two relaxed atomic RMWs per allocation —
        // amortized against the underlying malloc cost (~tens of ns).
        //
        // Fully-qualified paths, NO `use` statements: the injected block sits
        // at module scope next to the harness's own imports — a `use` here
        // collides with any harness importing the same name (E0252 on
        // `Ordering`, caught by the mutex-to-atomic sweep in Bench 020).
        const ALLOCATOR_BLOCK: &str = r#"
// ── Counting allocator (deterministic alloc axis — Issue 014 T1) ────────
// Exact integers, identical on every run; variance here is a harness bug.
// Fully-qualified paths only — no `use` (see generate_main_rs doc above).
static ALLOC_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static ALLOC_BYTES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct CountingAlloc;

#[inline]
fn bump_counting_alloc(size: usize) {
    ALLOC_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    ALLOC_BYTES.fetch_add(size as u64, std::sync::atomic::Ordering::Relaxed);
}

unsafe impl std::alloc::GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        bump_counting_alloc(layout.size());
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, _layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        bump_counting_alloc(new_size);
        unsafe { std::alloc::System.realloc(ptr, layout, new_size) }
    }
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        bump_counting_alloc(layout.size());
        unsafe { std::alloc::System.alloc_zeroed(layout) }
    }
}

#[global_allocator]
static GLOBAL_COUNTING_ALLOCATOR: CountingAlloc = CountingAlloc;
"#;

        // Strip dep-marker lines from the harness source.
        let harness_clean: String = spec
            .harness
            .lines()
            .filter(|l| !l.trim_start().starts_with("// [dep:"))
            .collect::<Vec<_>>()
            .join("\n");

        let sizes_array: String = spec
            .sizes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");

        let timing_loop = TIMING_LOOP
            .replace("__SIZES__", &sizes_array)
            .replace("__PROBE_ID__", stem);

        format!(
            "// Auto-generated by PerfBenchValidator — do not hand-edit.\n{harness_clean}\n{ALLOCATOR_BLOCK}\n{timing_loop}"
        )
    }

    /// Run `cargo run --release` on the temp crate, return stdout.
    fn run_cargo(&self, crate_dir: &std::path::Path) -> Result<String, String> {
        let manifest_path = crate_dir.join("Cargo.toml");
        let output = Command::new(&self.cargo_bin)
            // ⛔ PIN the probe's target dir, do not inherit one (Issue 110).
            // With a shared one, `cargo run` re-executes another scratch
            // crate's binary and this harness times the wrong code.
            .env("CARGO_TARGET_DIR", crate_dir.join("target"))
            .arg("run")
            .arg("--release")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .output()
            .map_err(|e| format!("cargo spawn failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Truncate long error output.
            let snippet = if stderr.len() > 2000 {
                &stderr[stderr.len() - 2000..]
            } else {
                &stderr
            };
            return Err(format!("cargo run failed: {snippet}"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// Parse JSON lines from the bench subprocess into SizeResults.
    #[allow(clippy::cast_precision_loss)]
    fn parse_bench_output(stdout: &str) -> Vec<SizeResult> {
        let mut results = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let Some(size) = v.get("size").and_then(serde_json::Value::as_u64) else {
                continue;
            };
            let Some(buggy_ns) = v.get("buggy_ns").and_then(serde_json::Value::as_u64) else {
                continue;
            };
            let Some(fixed_ns) = v.get("fixed_ns").and_then(serde_json::Value::as_u64) else {
                continue;
            };
            // Alloc axis (Issue 014 T1): fields default to 0 when absent
            // (old-format output / harness without the counting pass).
            let buggy_allocs = v
                .get("buggy_allocs")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let buggy_alloc_bytes = v
                .get("buggy_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let fixed_allocs = v
                .get("fixed_allocs")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let fixed_alloc_bytes = v
                .get("fixed_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let speedup = if fixed_ns == 0 {
                1.0
            } else {
                buggy_ns as f64 / fixed_ns as f64
            };
            let outcome = classify_speedup(speedup);
            results.push(SizeResult {
                size: usize::try_from(size).unwrap_or(usize::MAX),
                buggy_ns,
                fixed_ns,
                speedup,
                outcome,
                buggy_allocs,
                buggy_alloc_bytes,
                fixed_allocs,
                fixed_alloc_bytes,
            });
        }
        results.shrink_to_fit();
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔ Issue 110, and this is the SHARPEST instance of the class because
    /// the artifact is a BINARY. Two scratch crates with one package/bin
    /// name, sharing a target dir, are one cargo unit — and `cargo run`
    /// then re-executes the FIRST crate's binary. Measured with a known
    /// answer: two `perf_bench` bins printing different strings, one
    /// `CARGO_TARGET_DIR`, and the second run printed "I AM CRATE A" with
    /// exit 0. This harness would have timed one rule's code and reported
    /// the nanoseconds under another rule's id — not a wrong verdict but a
    /// measurement that never happened, reported as one.
    #[test]
    fn prepare_crate_keys_the_package_and_bin_name() {
        let root =
            std::env::temp_dir().join(format!("riir_perf_bench_unit_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let v = PerfBenchValidator::new().with_work_root(&root);
        let spec = |h: &str| BenchSpec {
            sizes: vec![8],
            harness: h.to_string(),
        };
        let name_of = |dir: &std::path::Path| {
            std::fs::read_to_string(dir.join("Cargo.toml"))
                .unwrap()
                .lines()
                .filter_map(|l| l.trim().strip_prefix("name = "))
                .map(|n| n.trim_matches('"').to_string())
                .collect::<Vec<_>>()
        };

        let (d1, stem1) = v.prepare_crate("rule_a", &spec("fn a() {}")).unwrap();
        let (d2, stem2) = v.prepare_crate("rule_b", &spec("fn b() {}")).unwrap();
        let (n1, n2) = (name_of(&d1), name_of(&d2));

        // BOTH names — package and [[bin]] — must be keyed. The bin name is
        // the one that decides which executable `cargo run` launches, so a
        // keyed package with an unkeyed bin still times the wrong code.
        assert_eq!(n1.len(), 2, "expected a package name and a [[bin]] name");
        assert_eq!(n2.len(), 2);
        for (a, b) in n1.iter().zip(n2.iter()) {
            assert_ne!(a, b, "two bench crates share a cargo unit");
        }
        for (dir, names) in [(&d1, &n1), (&d2, &n2)] {
            let stem = dir.file_name().unwrap().to_string_lossy().to_string();
            for n in names {
                assert!(
                    n.ends_with(&stem),
                    "name must carry the dir key: {n} vs {stem}"
                );
            }
        }
        // Identical (rule, spec) must still reuse one crate — the cached
        // build is what `run_bench_with_determinism`'s repeats ride on.
        assert_eq!(d1, v.prepare_crate("rule_a", &spec("fn a() {}")).unwrap().0);

        // ⛔ Issue 112: the generated program must STATE which probe it is,
        // and `cargo run` gives no stream to read a freshness flag from — so
        // this echo is the whole witness at this site. Assert it is present,
        // distinct per crate, and readable by the parser that consumes it.
        assert_ne!(stem1, stem2);
        for (dir, stem) in [(&d1, &stem1), (&d2, &stem2)] {
            let main_rs = std::fs::read_to_string(dir.join("src/main.rs")).unwrap();
            assert!(
                !main_rs.contains("__PROBE_ID__"),
                "the placeholder must be substituted, not shipped"
            );
            assert!(
                main_rs.contains(&format!(r#"\"probe\":\"{stem}\""#)),
                "generated main must print its own key"
            );
        }

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn classify_speedup_thresholds() {
        assert_eq!(classify_speedup(1.10), SizeOutcome::Faster);
        assert_eq!(classify_speedup(1.06), SizeOutcome::Faster);
        assert_eq!(classify_speedup(1.05), SizeOutcome::Tie); // not > 1.05
        assert_eq!(classify_speedup(1.00), SizeOutcome::Tie);
        assert_eq!(classify_speedup(0.96), SizeOutcome::Tie); // not < 0.95
        assert_eq!(classify_speedup(0.94), SizeOutcome::Slower);
        assert_eq!(classify_speedup(0.50), SizeOutcome::Slower);
    }

    #[test]
    fn aggregate_all_faster_is_dominant() {
        let results = vec![
            SizeResult {
                size: 10,
                buggy_ns: 100,
                fixed_ns: 50,
                speedup: 2.0,
                outcome: SizeOutcome::Faster,
                buggy_allocs: 0,
                buggy_alloc_bytes: 0,
                fixed_allocs: 0,
                fixed_alloc_bytes: 0,
            },
            SizeResult {
                size: 100,
                buggy_ns: 200,
                fixed_ns: 80,
                speedup: 2.5,
                outcome: SizeOutcome::Faster,
                buggy_allocs: 0,
                buggy_alloc_bytes: 0,
                fixed_allocs: 0,
                fixed_alloc_bytes: 0,
            },
        ];
        assert_eq!(aggregate_outcomes(&results), BenchOverall::ParetoDominant);
    }

    #[test]
    fn aggregate_majority_faster_is_dominant() {
        // 3 faster, 1 slower out of 4 → faster*2 (6) > total (4) → dominant.
        let results = vec![
            SizeResult {
                size: 10,
                buggy_ns: 100,
                fixed_ns: 50,
                speedup: 2.0,
                outcome: SizeOutcome::Faster,
                buggy_allocs: 0,
                buggy_alloc_bytes: 0,
                fixed_allocs: 0,
                fixed_alloc_bytes: 0,
            },
            SizeResult {
                size: 100,
                buggy_ns: 200,
                fixed_ns: 80,
                speedup: 2.5,
                outcome: SizeOutcome::Faster,
                buggy_allocs: 0,
                buggy_alloc_bytes: 0,
                fixed_allocs: 0,
                fixed_alloc_bytes: 0,
            },
            SizeResult {
                size: 1000,
                buggy_ns: 300,
                fixed_ns: 100,
                speedup: 3.0,
                outcome: SizeOutcome::Faster,
                buggy_allocs: 0,
                buggy_alloc_bytes: 0,
                fixed_allocs: 0,
                fixed_alloc_bytes: 0,
            },
            SizeResult {
                size: 10000,
                buggy_ns: 100,
                fixed_ns: 200,
                speedup: 0.5,
                outcome: SizeOutcome::Slower,
                buggy_allocs: 0,
                buggy_alloc_bytes: 0,
                fixed_allocs: 0,
                fixed_alloc_bytes: 0,
            },
        ];
        assert_eq!(aggregate_outcomes(&results), BenchOverall::ParetoDominant);
    }

    #[test]
    fn aggregate_half_and_half_is_mixed() {
        // 2 faster, 2 slower → neither dominant.
        let results = vec![
            SizeResult {
                size: 10,
                buggy_ns: 100,
                fixed_ns: 50,
                speedup: 2.0,
                outcome: SizeOutcome::Faster,
                buggy_allocs: 0,
                buggy_alloc_bytes: 0,
                fixed_allocs: 0,
                fixed_alloc_bytes: 0,
            },
            SizeResult {
                size: 100,
                buggy_ns: 100,
                fixed_ns: 200,
                speedup: 0.5,
                outcome: SizeOutcome::Slower,
                buggy_allocs: 0,
                buggy_alloc_bytes: 0,
                fixed_allocs: 0,
                fixed_alloc_bytes: 0,
            },
        ];
        assert_eq!(aggregate_outcomes(&results), BenchOverall::Mixed);
    }

    #[test]
    fn aggregate_all_slower_is_dominated() {
        let results = vec![
            SizeResult {
                size: 10,
                buggy_ns: 50,
                fixed_ns: 100,
                speedup: 0.5,
                outcome: SizeOutcome::Slower,
                buggy_allocs: 0,
                buggy_alloc_bytes: 0,
                fixed_allocs: 0,
                fixed_alloc_bytes: 0,
            },
            SizeResult {
                size: 100,
                buggy_ns: 80,
                fixed_ns: 200,
                speedup: 0.4,
                outcome: SizeOutcome::Slower,
                buggy_allocs: 0,
                buggy_alloc_bytes: 0,
                fixed_allocs: 0,
                fixed_alloc_bytes: 0,
            },
        ];
        assert_eq!(aggregate_outcomes(&results), BenchOverall::ParetoDominated);
    }

    #[test]
    fn aggregate_empty_is_mixed() {
        assert_eq!(aggregate_outcomes(&[]), BenchOverall::Mixed);
    }

    #[test]
    fn alloc_axis_equal_for_identical_results() {
        let mk = |allocs: u64, bytes: u64| SizeResult {
            size: 100,
            buggy_ns: 10,
            fixed_ns: 5,
            speedup: 2.0,
            outcome: SizeOutcome::Faster,
            buggy_allocs: allocs,
            buggy_alloc_bytes: bytes,
            fixed_allocs: 1,
            fixed_alloc_bytes: 16,
        };
        // Identical alloc integers → identical axes (the determinism proof).
        assert_eq!(alloc_axis(&[mk(7, 256)]), alloc_axis(&[mk(7, 256)]));
        // Different timing is IRRELEVANT to the axis (noise is excluded).
        let mut noisy = mk(7, 256);
        noisy.buggy_ns = 999;
        noisy.fixed_ns = 998;
        assert_eq!(alloc_axis(&[mk(7, 256)]), alloc_axis(&[noisy]));
        // Any alloc-integer difference → different axes.
        assert_ne!(alloc_axis(&[mk(7, 256)]), alloc_axis(&[mk(8, 256)]));
        assert_ne!(alloc_axis(&[mk(7, 256)]), alloc_axis(&[mk(7, 512)]));
    }

    #[test]
    fn verdict_is_pareto_dominant_helper() {
        let v = BenchVerdict::Pareto(ParetoVerdict {
            rule_id: "test".to_string(),
            results: vec![],
            faster_count: 2,
            slower_count: 0,
            tie_count: 0,
            overall: BenchOverall::ParetoDominant,
            ran: true,
            error: None,
        });
        assert!(v.is_pareto_dominant());
        assert!(!v.is_not_benchable());
    }

    #[test]
    fn verdict_not_benchable_helper() {
        let v = BenchVerdict::NotBenchable(NotBenchable {
            rule_id: "bevy".to_string(),
            reason: "needs bevy runtime".to_string(),
        });
        assert!(!v.is_pareto_dominant());
        assert!(v.is_not_benchable());
    }

    #[test]
    fn validate_none_spec_returns_not_benchable() {
        let v = PerfBenchValidator::new();
        let verdict = v.validate("test", None);
        assert!(verdict.is_not_benchable());
    }

    #[test]
    fn extract_deps_parses_dep_markers() {
        let harness = "// [dep: papaya = \"0.2\"]\n// [dep: blake3 = \"1\"]\npub fn foo() {}";
        let deps = PerfBenchValidator::extract_deps(harness);
        assert_eq!(deps, vec!["papaya = \"0.2\"", "blake3 = \"1\""]);
    }

    #[test]
    fn extract_deps_no_markers_returns_empty() {
        let harness = "pub fn foo() {}";
        let deps = PerfBenchValidator::extract_deps(harness);
        assert!(deps.is_empty());
    }

    #[test]
    fn generate_main_rs_strips_dep_markers() {
        let spec = BenchSpec {
            sizes: vec![10],
            harness: "// [dep: papaya]\npub fn make_input(_n: usize) -> u64 { 42 }\npub fn run_buggy(_: &u64) {}\npub fn run_fixed(_: &u64) {}".to_string(),
        };
        let main_rs = PerfBenchValidator::generate_main_rs(&spec, "deadbeefdeadbeef");
        assert!(!main_rs.contains("// [dep: papaya]"));
        assert!(
            main_rs.contains("deadbeefdeadbeef"),
            "the probe id must reach the source"
        );
        assert!(main_rs.contains("fn main()"));
        assert!(main_rs.contains("pub fn make_input"));
        assert!(main_rs.contains("&[10]"));
    }

    #[test]
    fn parse_bench_output_valid_json() {
        let stdout = r#"{"size":10,"buggy_ns":100,"fixed_ns":50}
{"size":100,"buggy_ns":200,"fixed_ns":80}
some garbage line
{"size":1000,"buggy_ns":300,"fixed_ns":600}"#;
        let results = PerfBenchValidator::parse_bench_output(stdout);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].size, 10);
        assert_eq!(results[0].buggy_ns, 100);
        assert_eq!(results[0].fixed_ns, 50);
        assert!((results[0].speedup - 2.0).abs() < 0.001);
        assert_eq!(results[0].outcome, SizeOutcome::Faster);
        assert_eq!(results[2].outcome, SizeOutcome::Slower);
    }

    #[test]
    fn parse_bench_output_empty_returns_empty() {
        let results = PerfBenchValidator::parse_bench_output("");
        assert!(results.is_empty());
    }

    #[test]
    fn parse_bench_output_handles_zero_fixed_ns() {
        let stdout = r#"{"size":10,"buggy_ns":100,"fixed_ns":0}"#;
        let results = PerfBenchValidator::parse_bench_output(stdout);
        assert_eq!(results.len(), 1);
        assert!((results[0].speedup - 1.0).abs() < 0.001); // guard against div-by-zero
    }
}
