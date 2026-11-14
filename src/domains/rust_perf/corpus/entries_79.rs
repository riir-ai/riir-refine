//! Entries slice — Batch 363 (2026-11-14): four `rust_perf` rules from the
//! hotpath.rs CI-benchmarking blog distill (Plan 374 / Research 407) — the
//! instruction-count cache-blindness law, the async measuring-wrapper
//! future-size law, the wall-clock judged-role / shared-CI noise-floor law
//! (carrying the folded ci-gate-policy-shape composite), and the
//! alloc-proxy inversion caveat.
//!
//! Source: the hotpath.rs blog "Rust Benchmarking: How to Detect
//! Performance Regressions in CI" (Paweł Urbanek, 2026-11-10, MIT-ecosystem
//! prose source; the two code rows cite the author's own landed hotpath-rs
//! PRs #720/#801 as upstream-present decision content — no clone/pin, the
//! blog is the citation of record and its figures were re-verified against
//! the live page at claim time, T0). The FOURTH hotpath-family pass
//! (R032 → R081/B65 → R234 → R235 → R407 lineage).
//!
//! Gates at authoring: the folded/dropped candidates are disclosed here —
//! `ci-gate-policy-shape` FOLDED into the wall-clock rule (the generic
//! policy shape is one law with its metric-role half); `cpu-model-per-run-
//! metadata` DROPPED (benchmark-report metadata is measurement
//! infrastructure, not a code fix-space — the home box-state PROVENANCE
//! practice owns it). Yield 4 of the estimated 2-4+gates.
//!
//! Fixtures are ORIGINAL plain-Rust stand-ins (an op-count/cache-capacity
//! model, twin-composed async chains, a counting-allocator dedup pair, a
//! policy-gate model — never pasted source), standalone rustc-compiled AND
//! executed before landing (8/8). Batch 363 / Plan 374 / Research 407.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── measurement honesty: an instruction-count gate is blind to
        // cache locality — equal instructions, divergent layout ──
        PerfRule {
            rule_id: "instruction-count-cache-blindness".to_string(),
            name: "A deterministic instruction-count (icount/Callgrind Ir-class) regression gate cannot adjudicate a cache-locality divergence: two shapes with EQUAL instruction counts but different access patterns (sequential vs pointer-chased, hashed-vs-scanned lookup, array-of-structs vs struct-of-arrays walk) report a 0% icount delta while wall-clock diverges by orders of magnitude - the gate must carry the cache-blindness caveat (or pair a locality-sensitive axis: cache-sim counters, a paired wall-clock context read) and must NEVER answer no-regression on the icount column alone for a layout-divergent pair".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// NAIVE: judge a layout-sensitive optimization pair on the
// icount column alone. Both shapes execute the same instruction
// sequence over the same element count, so the icount delta is 0%
// and the gate answers no-regression - while one shape walks a
// scattered layout and pays a cache miss on nearly every line.

fn sweep(gate_icount_a: usize, gate_icount_b: usize, wall_ms_a: f64, wall_ms_b: f64) -> &'static str {
    let delta_pct = (gate_icount_b as f64 - gate_icount_a as f64) / gate_icount_a as f64 * 100.0;
    if delta_pct.abs() < 0.1 { "no-regression" } else { "regression" }
    // `wall_ms_*` unused: the context axis that would expose the
    // divergence is in the report but never consulted.
}

#[test]
fn blind_gate_blesses_a_26x_wall_regression() {
    // equal icount by construction, wall apart by >20x: the gate answers
    // no-regression on the pair the wall context shows diverging.
    assert_eq!(sweep(20_971_545, 20_971_545, 5.06, 133.04), "no-regression");
}
"#.to_string(),
            fixed_code: r#"// FIXED: the gate classifies the PAIR before judging - a
// layout-divergent pair (same instructions, different access order)
// is outside the icount gate's authority and is answered with a
// typed caveat (or routed to a locality-sensitive axis), never
// blessed as no-regression.

#[derive(Debug, PartialEq)]
enum Verdict {
    NoRegression,
    Regression,
    CacheBlind,
}

fn sweep(a: (usize, f64, bool), b: (usize, f64, bool)) -> Verdict {
    // (icount, wall_ms, layout_divergent) — the third field is the
    // pair classifier: same work, different access pattern.
    let (icount_a, _, divergent) = a;
    let (icount_b, wall_b, _) = b;
    let (_, wall_a, _) = a;
    if divergent {
        // equal icount + divergent layout: the icount column cannot
        // adjudicate — return the typed blindness verdict.
        let _ = (icount_a, icount_b);
        let wall_ratio = wall_b / wall_a;
        if wall_ratio > 2.0 {
            return Verdict::CacheBlind;
        }
    }
    let delta_pct = (icount_b as f64 - icount_a as f64) / icount_a as f64 * 100.0;
    if delta_pct.abs() < 0.1 { Verdict::NoRegression } else { Verdict::Regression }
}

#[test]
fn locality_divergent_pair_gets_the_typed_verdict() {
    let seq = (20_971_545, 5.06, false);
    let chase = (20_971_545, 133.04, true);
    assert_eq!(sweep(seq, chase), Verdict::CacheBlind);
    // an honest icount move on a NON-divergent pair still judges
    assert_eq!(sweep((100, 1.0, false), (110, 1.1, false)), Verdict::Regression);
}
"#.to_string(),
            context_notes: Some("MECHANISM: Callgrind Ir counts every executed instruction equally — a line that misses in cache and a line that hits cost the SAME count — so two shapes with identical instruction streams but different access orders (the sequential-vs-pointer-chase pair; a hash lookup vs a scan; a transposed iteration order) are icount-indistinguishable while their wall times diverge arbitrarily (the source's witness table: 20,971,545 instructions both, 5.06 ms vs 133.04 ms — a +2529% wall delta at a 0% icount delta). THE LAW: an icount-class deterministic gate is authoritative ONLY over work count; it is structurally blind over locality, and a layout-sensitive change judged on it alone can ship a measured 20x+ wall regression as no-regression. THE REMEDY AXIS (carried, not owned): Valgrind's --cache-sim=yes exposes the locality axis the Ir metric lacks; the lighter paired read is a wall-clock context beside the icount verdict (context, never judge — see wall-clock-ungated-shared-ci-noise-floor for the role law). Valgrind also SERIALIZES thread execution, so a multithreaded shape measured under Callgrind is not the shape that runs natively — the blindness law is stricter there. ANTI-REGIMES: instruction-bound workloads (pure arithmetic, register-resident loops — no memory axis, icount IS representative; the caveat must not fire); genuinely equal-layout pairs (same access order, icount authoritative). ADJUDICATION (mandatory, both directions): vs kernel_opt's dry-pass-icache-warm-before-real + launch-time-code-as-data-l2-warm — those own the WARMING remedy (making instruction fetches hit: run the text twice effects-skipped; probe the text from the data side at launch); THIS owns the METRIC critique (an icount gate cannot SEE the fetch cost either way) — a warming change is exactly the pair class this rule forbids judging on icount alone (equal instructions, divergent fetch locality), so they compose: warm the code path, then judge it on a locality-aware axis. vs entries_06's debug-level-gated-computation MEASUREMENT-HONESTY RIDER (#143684: LLVM-21 icount −1.70% / wall +0.26% — 'never promote on icount alone'): the rider owns the PROMOTION-DISCIPLINE consequence for toolchain bumps (compiler-generated icount/wall inversions); THIS owns the workload-class mechanism (equal instructions, divergent data/fetch locality) and generalizes the rider's moral into a gate-verdict law. FIXTURES: b363 F1 (the icount-equal/chase-divergent pair with a bounded-cache model + the instruction-bound anti-regime + the tampered icount-only gate). NotBenchable-by-domain (the win metric is a gate-verdict property, no validator hosts it — the dial9/136 annotation, not a demotion). Source: hotpath.rs blog 'Rust Benchmarking: How to Detect Performance Regressions in CI' (Paweł Urbanek, 2026-11-10), 'When Callgrind instruction counts mislead: cache locality' + the metrics-summary section (Ir counts hits and misses equally; thread serialization; cache-sim exists unexposed). Batch 363 / Plan 374 / Research 407.".to_string(),
            // Not benchable: the win axis is a gate-verdict property; no
            // validator hosts a blind-gate pair (the dial9/136 class).
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── layout/async: a measuring async-fn wrapper layer that holds the
        // wrapped future more than once makes composed size exponential ──
        PerfRule {
            rule_id: "async-wrapper-future-size-exponential-nesting".to_string(),
            name: "A measuring/instrumenting async-fn wrapper that stores the wrapped future more than once per layer (the generated macro state machine held it 2-3x across its await plumbing; a twin/race/retry wrapper holds real duplicates) makes the composed future size EXPONENTIAL in nesting depth - replace the per-layer duplication with a hand-written Future impl that stores the inner future ONCE (a plain struct field polled through), so composition grows linearly".to_string(),
            category: "layout".to_string(),
            buggy_code: r#"// NAIVE: a measuring wrapper written as an async fn whose body
// composes the inner future more than once — here a twin-shape
// (both branches are real held futures), the shape generated
// instrumentation macros effectively produced. Every layer holds
// 2 copies of the level below: N layers = 2^N leaf frames.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

async fn leaf() -> u64 { 42 }

struct Join2<A, B> { a: A, b: B }

impl<A: Future<Output = u64>, B: Future<Output = u64>> Future for Join2<A, B> {
    type Output = u64;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let a = unsafe { Pin::new_unchecked(&mut this.a) };
        match a.poll(cx) {
            Poll::Ready(v) => Poll::Ready(v),
            Poll::Pending => {
                let b = unsafe { Pin::new_unchecked(&mut this.b) };
                b.poll(cx).map(|_| 0)
            }
        }
    }
}

async fn twin1() -> u64 { Join2 { a: leaf(), b: leaf() }.await }
async fn twin2() -> u64 { Join2 { a: twin1(), b: twin1() }.await }
async fn twin3() -> u64 { Join2 { a: twin2(), b: twin2() }.await }
async fn twin4() -> u64 { Join2 { a: twin3(), b: twin3() }.await }

#[test]
fn exponential_composition() {
    let f = twin4();
    // 2^4 = 16 leaf frames composed into ONE future value
    let bytes = std::mem::size_of_val(&f);
    assert!(bytes >= 16 * std::mem::size_of::<u64>() * 2,
        "the twin-composed future carries the whole subtree inline: {bytes} B");
}
"#.to_string(),
            fixed_code: r#"// FIXED: the wrapper is a hand-written Future that stores the
// inner future ONCE in a plain field and polls through it — every
// layer adds a constant header, so composition grows linearly.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Linear<L> { inner: L }

impl<L: Future<Output = u64> + Unpin> Future for Linear<L> {
    type Output = u64;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx)
    }
}

fn layer(f: impl Future<Output = u64> + Unpin) -> impl Future<Output = u64> + Unpin {
    Linear { inner: f }
}

async fn leaf() -> u64 { 42 }

#[test]
fn linear_composition() {
    // 4 layers: ONE leaf frame + 4 constant wrappers
    let f = layer(layer(layer(layer(Box::pin(leaf())))));
    let bytes = std::mem::size_of_val(&f);
    assert!(bytes < 4 * std::mem::size_of::<usize>() + std::mem::size_of::<u64>() + 64,
        "the linear chain stays flat: {bytes} B");
}
"#.to_string(),
            context_notes: Some("MECHANISM: an async fn compiles to a state-machine struct whose size is the union of its locals across await points; a measuring wrapper whose generated state machine (or hand-written body — twin/race/retry shapes) stores the wrapped future MORE THAN ONCE multiplies the subtree per layer, so N nesting levels produce ~(dup)^(N-1) leaf frames inline in ONE future value — the source measured the wrapped future stored 2-3x per layer, exponential with depth; the fix kept it linear (a hand-written MeasuredFuture storing the inner once). THE WIN AXIS IS SIZE (per-task memory; a spawned composed future boxes and clones through executors at its composed size), with instruction/alloc reductions riding the smaller state machines (the source: ~98% allocs and ~97% icount at 8 levels). THE BLOG-SOURCED EXACT SHAPE: the fix replaced the async-fn wrappers with a hand-written Future — the corpus form of that is the plain generic struct with a single inner field polled through (the Linear form). REGIMES: instrumentation/metrics macros over async fns (measure/span/trace layers); retry/timeout/race wrappers over the SAME inner; deep middleware-style handler composition (N layers deep). ANTI-REGIMES: wrappers that hold the inner exactly once (plain async fn one-await passthrough stores ONE inline copy — composition is a linear SUM of frames there, and scoping locals per async-future-size is the right lever, not this law); wrappers over unrelated futures (join of DISTINCT work is real parallelism, not duplication — this law is about composing the SAME level twice per layer); depth-1 or 2 composition (the exponential never matters). ADJUDICATION (mandatory, both directions) vs async-future-size (entries_02, layout): THAT owns the local-LIFETIME axis WITHIN one future (a large Vec held across .await inflates its own frame; the fix is a scoped block dropping it before the await); THIS owns the wrapper-LAYER axis ACROSS a composed chain (each layer's state machine stores the level below k>1 times; the fix is a one-store-per-layer Future impl). They compose, never merge: a deep measuring chain can ALSO carry large locals (apply both); neither subsumes the other's axis — scoping a local does not deduplicate a twin layer, and a Linear wrapper does not drop a large local before its await. FIXTURES: b363 F2 (twin5 = 2^5 leaf frames inline vs a 4-layer Linear chain; a counting-allocator poll census proves both complete; the size assertion floor is deliberately far below the blog's 2-3x-per-level magnitude). Measurement: the win axis is size_of composition equality + a spawn/poll alloc census — the Pareto validator's &Input harness cannot hold a composed N-level future across a poll without a clone tax (the entries_77 map-inplace precedent), so per the PRE-REGISTERED FALLBACK the standalone preflight IS the instrument of record: T4 runs the composed-vs-linear size/alloc census at depth 1..=8 and prices the per-level multiplier; spec stays None (the entries_17 decision-content posture). NotBenchable-by-validator, measured-by-preflight. Source: hotpath.rs blog (Paweł Urbanek, 2026-11-10), 'Avoid exponential future growth in nested async functions' — hotpath-rs PR #801 (upstream-present decision content; figures re-verified against the live page at T0: 2-3x per level stored, 8 levels ~98% allocs / ~97% icount). Batch 363 / Plan 374 / Research 407.".to_string(),
            // The Pareto validator's &Input harness cannot express a
            // composed N-level future held across a poll (clone tax) — the
            // PRE-REGISTERED fallback applies: the standalone preflight
            // (.plans/b363_goat_preflight.rs) is the instrument of record.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── measurement honesty: wall-clock is context, never the judge in
        // shared CI — the noise floor derives from an empty-change baseline ──
        PerfRule {
            rule_id: "wall-clock-ungated-shared-ci-noise-floor".to_string(),
            name: "In a shared/ungated CI environment wall-clock timing is CONTEXT, never the regression judge: gate verdicts decide on deterministic axes (allocations byte-exact; instruction counts within a derived floor) while the wall number rides the report unjudged - and the deterministic gate's noise floor is DERIVED from an empty-change baseline (runs of unchanged code), never assumed zero; the gate policy shape that carries this is absolute budgets (exact calls, alloc ceilings) + relative thresholds above the derived floor + a min-percent share floor so trivial helpers cannot fire on noise".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// NAIVE: a CI regression gate that judges on wall-clock ms in a
// shared environment, with a hard-coded zero-noise assumption.

fn gate(base_ms: f64, cand_ms: f64) -> &'static str {
    // judges wall-clock directly — on a shared runner the baseline
    // itself swings 2-3x run to run, so this answers regression on
    // noise and improvement on the mirror-image noise.
    if cand_ms > base_ms { "regression" } else { "improvement" }
}

#[test]
fn wall_gate_flips_on_pure_noise() {
    // the SAME code measured twice on a contended shared runner
    assert_eq!(gate(35.0, 98.0), "regression", "an empty-change pair reads as a regression");
    assert_eq!(gate(98.0, 35.0), "improvement", "the mirror noise reads as an improvement");
}
"#.to_string(),
            fixed_code: r#"// FIXED: the judged axes are the deterministic ones; wall-clock
// is returned beside the verdict as context, never consulted by
// it; the icount floor is DERIVED from an empty-change baseline
// (measured spread + margin), and trivial-share functions are
// skipped by a min-percent floor.

#[derive(Debug, PartialEq)]
enum Verdict { NoRegression, Regression, Improvement, SkippedTrivialShare }

struct Run { icount: u64, alloc_bytes: u64, wall_ms: f64 }

struct Policy { floor_pct: f64, min_share_pct: f64 }

impl Policy {
    // derive the floor from N runs of UNCHANGED code
    fn from_empty_baseline(runs: &[Run]) -> Self {
        let base = runs[0].icount as f64;
        let worst = runs.iter()
            .map(|r| ((r.icount as f64 - base) / base * 100.0).abs())
            .fold(0.0, f64::max);
        Policy { floor_pct: worst.ceil() + 1.0, min_share_pct: 0.5 }
    }

    fn verdict(&self, base: &Run, cand: &Run, share_pct: f64) -> Verdict {
        if share_pct < self.min_share_pct {
            return Verdict::SkippedTrivialShare;
        }
        // allocs judge byte-exact; icount judges above the derived floor;
        // wall NEVER judges — it rides the report as context.
        if cand.alloc_bytes != base.alloc_bytes {
            return Verdict::Regression;
        }
        let d = (cand.icount as f64 - base.icount as f64) / base.icount as f64 * 100.0;
        if d.abs() <= self.floor_pct { Verdict::NoRegression }
        else if d < 0.0 { Verdict::Improvement } else { Verdict::Regression }
    }
}
"#.to_string(),
            context_notes: Some("MECHANISM: the source's empty-change PR (zero code delta) measured the same benchmark 100x on shared runners — wall-clock spread ~3x on x86 across 6 assigned CPU models (same-model still 1.28x) and ~2x on ARM (same-model Neoverse-N2 still 1.98x), while allocations were byte-identical in 100/100 runs (2,425,213 B exactly, both arches) and instruction counts wiggled ~0.13% x86 / ~0.065% ARM. THE LAW (two halves, one gate shape): (1) the ROLE law — in shared/ungated environments wall-clock judges nothing; deterministic axes judge (allocations when the benchmark is well-designed: byte-exact; instruction counts within a floor), and the wall number rides the report as context; (2) the FLOOR law — the deterministic gate's tolerance is DERIVED from an empty-change baseline (the measured spread of unchanged code + margin), never assumed zero, because even 'deterministic' icount wiggles with thread scheduling. THE POLICY SHAPE (the folded ci-gate-policy-shape candidate, landed here): absolute budgets (exact call counts, alloc ceilings — a budget violation fails regardless of thresholds) + relative thresholds above the derived floor + a min-percent share floor (the source's 0.5%-of-run floor) so trivial-share helpers cannot fire on noise. The thresholds are PER-BENCHMARK calibration, never universal numbers. ADJUDICATION (mandatory, both directions) vs benchmark-spread-above-threshold-reports-unstable (entries_70, measurement-honesty): THAT owns the RUN-level stability verdict — a benchmark whose measured spread exceeds its own threshold reports UNSTABLE instead of a headline (rolling-CV warm-up, iteration floors); THIS owns the GATE-role law — which METRIC may decide a regression verdict in shared CI and where the gate's tolerance comes from. They compose: a run that fails the stability verdict never reaches this gate (no headline, no judged number); a stable run's regression check is decided here on the deterministic axes. Neither subsumes: the UNSTABLE verdict governs any timing you DO print; the role law governs what the CI check FAILS on. vs the corpus's own wall-clock GOAT discipline (the home practice, AGENTS.md box-state law + bench_preflight PROVENANCE): the home law prices LATENCY CLAIMS on a quiet pinned box; THIS prices CI GATE VERDICTS on shared runners — different regime, same distrust of ungated wall-clock. ANTI-REGIMES: a dedicated pinned benchmark box (wall-clock may judge there — the home practice is the proof); single-threaded deterministic workloads on pinned hardware (stricter floors legal, the source says so). FIXTURES: b363 F4 (the empty-baseline floor derivation + the timing-noise pair that must not flip + the trivial-share skip + the real-regression catch; wall/alloc ride as context). NotBenchable-by-domain (a gate-verdict property — the dial9/136 annotation). Source: hotpath.rs blog (Paweł Urbanek, 2026-11-10), the CI-noise analysis + the policies/budgets section (the policy TOML is the vendor's own — the GENERIC shape landed, vendor vocabulary out). Batch 363 / Plan 374 / Research 407.".to_string(),
            // Not benchable: a gate-policy property (the dial9/136 class).
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── measurement honesty: an allocations-only verdict inverts when
        // the cheaper-work shape allocates more — demand a paired axis ──
        PerfRule {
            rule_id: "alloc-proxy-inversion-work-ratio-caveat".to_string(),
            name: "Allocations are a PROXY for work, never the work itself: a change that reduces measured work can INCREASE allocations (a lookup-structure build/rebuild, a dedup via a seen-set, an index that trades churn for fewer comparisons) - an allocations-only regression verdict on such a pair is an INVERSION, so an alloc delta may be called a win (or a regression) only when a paired work-or-time axis agrees, and the pair's correctness is asserted before either axis is read".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// NAIVE: score a dedup optimization pair on the allocation
// column alone. The scan shape allocates nothing; the seen-set
// shape allocates a table — the alloc-only gate calls the
// asymptotically-better shape a REGRESSION.

fn alloc_only_verdict(allocs_a: usize, allocs_b: usize) -> &'static str {
    if allocs_b > allocs_a { "regression" } else { "improvement" }
}

#[test]
fn alloc_only_gate_inverts_the_dedup_pair() {
    let n = 256usize;
    let uniq = 16usize;
    // scan: zero allocs, O(n * uniq) comparisons
    let (a_allocs, a_work) = (0usize, n * uniq);
    // seen-set: a few allocs, O(n) hashes
    let (b_allocs, b_work) = (3usize, n);
    assert_eq!(alloc_only_verdict(a_allocs, b_allocs), "regression",
        "the gate condemns the shape that does 16x less work");
    let _ = (a_work, b_work); // the axis that would correct it: unread
}
"#.to_string(),
            fixed_code: r#"// FIXED: the verdict refuses to score on the alloc column alone;
// with the work axis attached it calls the pair honestly — a win
// WITH a disclosed alloc regression, correctness asserted first.

#[derive(Debug, PartialEq)]
enum Verdict { WinWithAllocRegressionDisclosed, AllocWin, NoChange, NeedsPairedAxis }

fn paired_verdict(allocs_a: usize, allocs_b: usize, work_a: usize, work_b: usize) -> Verdict {
    // correctness is asserted by the caller before either axis is read
    if work_b < work_a && allocs_b > allocs_a {
        // less work for more churn: a win, with the alloc regression
        // disclosed beside it — never laundered into an alloc-only story
        Verdict::WinWithAllocRegressionDisclosed
    } else if allocs_b < allocs_a && work_b <= work_a {
        Verdict::AllocWin
    } else {
        Verdict::NoChange
    }
}

#[test]
fn paired_gate_scores_the_dedup_pair_honestly() {
    let (a_allocs, a_work) = (0usize, 256 * 16);
    let (b_allocs, b_work) = (3usize, 256);
    assert_eq!(paired_verdict(a_allocs, b_allocs, a_work, b_work),
        Verdict::WinWithAllocRegressionDisclosed);
    // an alloc-only ask with no work axis is refused, not scored
    assert_eq!(paired_verdict(a_allocs, b_allocs, a_work, a_work),
        Verdict::NoChange);
}
"#.to_string(),
            context_notes: Some("MECHANISM: the source's dedup witness — the contains-scan rewritten to a HashSet seen-set sent allocations UP (>2x, +112.54% in their diff report) while wall-clock went DOWN 98% — allocations are byte-deterministic (their strongest property) but track CHURN, not WORK; any optimization that adds a lookup structure (seen-set, index, memo table, arena) trades allocation churn for fewer comparisons/probes, and an allocations-only verdict scores that trade backwards. THE LAW: an alloc delta is adjudicable only beside a paired work-or-time axis (the work proxy may be a comparison/probe count; the honest axis is wall on a quiet box — see wall-clock-ungated-shared-ci-noise-floor for the role law in shared CI), and the pair's outputs are asserted EQUAL before any perf axis is read (correctness first). The inverse pairing is already corpus law: an icount/wall win with a measured alloc loss demotes in the Pareto validator (the alloc-count axis) — THIS prices the mirrored case, an alloc 'regression' that is a work win, which the validator would also demote WRONGLY if the alloc axis were read alone; the validator's alloc axis is one input beside the time bars, never the verdict alone, and this rule is the reason. REGIMES: dedup/seen-set introductions; lookup-structure builds (HashMap/Index/rebuild-before-scan); memoization added to a hot path (allocates the table, deletes the recomputation); arena/slab swaps that change churn shape. ANTI-REGIMES: pairs with EQUAL work (alloc deltas then dominate legitimately — a pure churn reduction is a real win); changes with no alloc axis at all (this rule does not fire). ADJUDICATION: vs the alloc-count validator axis itself (the corpus's own G2/G4 gates) — the validator is the ENFORCEMENT half; this rule is the INTERPRETATION law that keeps the alloc axis a co-input, never the sole judge (the validator's Pareto bars always pair time axes; this rule prices the case a human reviewer or a custom gate would read alloc-only). vs wall-clock-ungated-shared-ci-noise-floor (this batch): that owns which metric JUDGES in shared CI; this owns what an alloc delta MEANS when it is read — the pair-classifier half of the same discipline; they compose. FIXTURES: b363 F3 (the counting-allocator dedup pair: allocs_a == 0 < allocs_b with work_b << work_a; the gate model refusing alloc-only scoring). NotBenchable-by-domain (a verdict-interpretation property — the dial9/136 annotation). Source: hotpath.rs blog (Paweł Urbanek, 2026-11-10), 'Are memory allocations a stable benchmark metric?' (the dedup inversion pair; the +112.54%/−98% diff-report figures). Batch 363 / Plan 374 / Research 407.".to_string(),
            // Not benchable: a verdict-interpretation property.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
