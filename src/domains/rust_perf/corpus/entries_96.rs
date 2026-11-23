// Batch 402 — the mlxfast-27b delta residue's `rust_perf` half (Plan 402 /
// Research 437; Layr-Labs/mlxfast-bonsai2-27b-engine, delta cfb35b0d→0c7c3923,
// MIT — quotes re-verified at the pin at this batch's T0).
//
// rust_perf 390 → 392: two transcription rules:
// - `score-is-lower-median-pair-never-mean` (measurement-honesty) — a score
//   aggregated over paired control-vs-candidate measurements is the LOWER
//   MEDIAN PAIR (middle pair on odd count, lower of the two central pairs on
//   even), never a mean over pairs, never a best pair; every enforced figure
//   comes from that one pair and the other pairs stay recorded as measured.
// - `caller-exclusive-lock-asserted-never-taken` (concurrency) — a helper
//   that must operate inside a caller-owned exclusive window verifies the
//   window IS HELD (its own non-blocking try-lock must fail) and refuses by
//   name when nobody holds it; it never takes the lock itself, because its
//   own acquisition would be released at the helper's exit, not the
//   window's end.
//
// Fixtures land in src/domains/rust_perf/corpus/tests.rs (the shape arms)
// + fixtures/b402_perf_fixtures.rs. NOTHING INVENTED — every quote read at
// the clone this session.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

/// Batch 402 — the mlxfast-27b delta residue's `rust_perf` rules.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b402_rules() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "score-is-lower-median-pair-never-mean".to_string(),
            name: "A score aggregated over paired control-vs-candidate measurements is the \
                   LOWER MEDIAN PAIR — the middle pair on an odd count, the lower of the two \
                   central pairs on an even count — never a mean over pairs and never a best \
                   pair; every enforced figure in the artifact comes from that ONE pair, and \
                   the other pairs stay recorded as measured"
                .to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// NAIVE: N prompts are each measured as a PAIR (own serial control
// leg vs the candidate leg, same box, same job) — and the headline
// score is then derived by AVERAGING the per-prompt composites (or
// worse, taking the BEST pair). A mean lets one unrepresentative
// prompt drag the score (or lift it), a best pair is cherry-picking,
// and either way the number printed is no single measured pair: the
// artifact's figures no longer trace to a measurement anyone ran.
#[derive(Clone, Copy)]
struct Pair {
    composite: f64, // candidate / its OWN control leg
}

impl Pair {
    fn headline_score_mean(pairs: &[Pair]) -> f64 {
        pairs.iter().map(|p| p.composite).sum::<f64>() / pairs.len() as f64
    }

    fn headline_score_best(pairs: &[Pair]) -> f64 {
        pairs.iter().map(|p| p.composite).fold(f64::MIN, f64::max)
    }
}

// the artifact: one number, no pair it belongs to
fn artifact(pairs: &[Pair]) -> f64 {
    Pair::headline_score_mean(pairs)
}"#.to_string(),
            fixed_code: r#"// FIXED: score = the LOWER MEDIAN PAIR. The pairs are ordered by
// composite; odd count -> the middle pair; even count -> the LOWER
// of the two central pairs (never averaged, never best). Every
// enforced figure in the artifact is that one pair's; the other
// pairs stay in the artifact AS MEASURED (never discarded, never
// pooled into the score).
#[derive(Clone, Copy, PartialEq)]
struct Pair {
    composite: f64,
}

impl Pair {
    fn headline_score_lower_median(mut pairs: Vec<Pair>) -> Option<Pair> {
        if pairs.is_empty() {
            return None;
        }
        pairs.sort_by(|a, b| a.composite.total_cmp(&b.composite));
        let n = pairs.len();
        let idx = if n % 2 == 1 {
            n / 2 // the middle pair (odd count)
        } else {
            n / 2 - 1 // the LOWER of the two central pairs (even count)
        };
        Some(pairs[idx])
    }
}

// the artifact: the scored pair IS a measured pair; the others ride
// along as measured rows (paired_legs), never averaged in.
struct Artifact {
    scored: Pair,
    paired_legs: Vec<Pair>,
}

fn artifact(pairs: Vec<Pair>) -> Option<Artifact> {
    let scored = Pair::headline_score_lower_median(pairs.clone())?;
    Some(Artifact { scored, paired_legs: pairs })
}"#.to_string(),
            context_notes: Some(r##"Grade A- - NotBenchable-by-domain (the deciding metric is SCORING-AGGREGATION STRUCTURE - which pair a scoreboard certifies - no validator prices that in cycles; a league-honesty gain, not a latency gain). SOURCE, verbatim anchors (Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923, MIT): docs/participant-contract.md:630-635 - "One ranked run measures the pairs the fixture's `official_pairs` declares, and scores ONE of them. Each pair has its own composite, from its own control leg. The run scores the pair whose composite is the lower median over the pairs: the middle pair on an odd count, the lower of the two central pairs on an even count. Pairs are never averaged. Every enforced figure in the artifact is that one pair's, and the other pairs stay in `metrics.paired_legs` as measured." + the pairing law (:573-590 README): "A ranked run measures pairs, on the SAME box, in the SAME job. The fixture lists the live prompts in `live_goldens`. A ranked run measures each live prompt once, one pair per prompt, and the score is the lower-median pair." + "Each pair has its own composite, from its own control leg" - the CONTROL IS PER-PAIR (never one pooled control leg shared across prompts - a pooled control makes every ratio share one noise draw). THE ANTI-CHERRY-PICK DIRECTION IS IN THE ORDER: lower median (even count takes the LOWER central pair) - the statistic leans AGAINST the submitter, the opposite of a best-pair headline. THE POOL INTEGRITY RIDERS (participant-contract.md + ranked-box-preflight.sh): every live_golden must name a pin-verified pool entry and "timed_prompt_pool has two entries with the same ${field}; pool entries must be distinct" - a duplicated prompt would let one workload vote twice in the median. KINSHIPS (adjudicated): the paired-interleave median-of-per-pair-ratios law (rust_perf entries_53 / the ab_timing doctrine) owns the MEASUREMENT-SCHEDULING axis for ONE A/B comparison (interleave arms, median across chunk pairs against scheduler alternation tax) - THIS owns SCORE SELECTION ACROSS PROMPTS (which pair's composite the scoreboard certifies; the enemy is prompt variance and cherry-picking, not scheduler noise) - compose: a league cell interleaves within a pair AND lower-medians across pairs; benchmark-spread-above-threshold-reports-unstable (entries_70) owns the STABILITY VERDICT axis (a wide spread reports UNSTABLE, never a headline) - THIS presumes stable-enough pairs and selects the certified one; calibration-probe-swept-band-single-point-outlier (entries_94) owns the CALIBRATION-INPUT axis (a probe must sweep the range) - THIS is the OUTPUT statistic (the single-point-outlier family's scoring-side sibling); unique-prompts-defeat-prefix-cache-and-cold-warm-reported-separately (entries_84) owns prompt DIVERSITY for cache honesty - complementary (multi-prompt also defeats single-prompt tuning), different mechanism, cite; specdec-composite-decomposed-into-per-step-time-and-accepted-length (kernel gpu_161) owns formula decomposition - unrelated; the league's sequential_ab_timing gate is house practice (the within-pair half). Regime: any scoreboard/league/GOAT-bar cell that aggregates per-prompt (per-workload, per-fixture) paired ratios into one certified number - perf-rematch scoreboard rows, decision-index scoring, multi-fixture bench verdicts. ANTI-REGIME: single-pair comparisons (the median of one pair is that pair - nothing to select); unpaired aggregates where each prompt was measured under different conditions (no pairing, no median - the honesty problem is upstream); distributions where the mean IS the contract (cost accounting, not competitive scoring). Suggestion-only. Batch 402 / Plan 402 / Research 437 (mlxfast-bonsai2-27b-engine delta cfb35b0d..0c7c3923; the arena's reorg moved scoring from one live golden to three + lower-median pairs - the methodology record is docs/participant-contract.md + the README scoring section). Fixture original."##.to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "caller-exclusive-lock-asserted-never-taken".to_string(),
            name: "A helper that must operate inside a CALLER-OWNED exclusive window (a GPU \
                   lock, a bench lease) verifies the window IS HELD — its own non-blocking \
                   try-lock must FAIL — and refuses by name when nobody holds it; it never \
                   takes the lock itself, because a lock the helper took would be released \
                   at the helper's exit, which is not the window"
                .to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// NAIVE (both defects): the boot helper needs the box's exclusive GPU
// window, so it "just" takes the lock for its own duration — but the
// window the caller owns outlives the helper: the lock is RELEASED at
// the helper's exit while the resident it booted keeps holding 18.5
// GiB OUTSIDE any window (the next contender walks in). And on the
// path where nobody holds the window at all, the helper runs anyway —
// no signal that the exclusivity contract was already broken.
use std::sync::{Mutex, MutexGuard, TryLockError};

struct GpuWindow(Mutex<()>);

impl GpuWindow {
    /// defect 1: the helper ACQUIRES the lock (released at its own exit)
    /// defect 2: no check that anyone holds the window at all
    fn boot_resident(window: &GpuWindow) -> MutexGuard<'_, ()> {
        window.lock().unwrap() // held only for THIS call
        // ... boot the 18.5 GiB resident ...
        // guard drops HERE — the resident outlives the lock it "took"
    }
}

fn main() {
    let window = GpuWindow(Mutex::new(()));
    let _resident = GpuWindow::boot_resident(&window);
    // the resident is up; the "lock" is gone; another process starts
    // a benchmark on the same GPU — 12x measurement swings (Bench 649)
}"#.to_string(),
            fixed_code: r#"// FIXED: the helper ASSERTS custody instead of taking it. A
// non-blocking try-lock must FAIL (the caller holds it); if the
// try-lock SUCCEEDS — or errors with the lock poisoned/absent —
// nobody owns the window and the helper refuses by name. The helper
// never holds the guard: it drops the probe guard immediately (the
// caller's lock, taken elsewhere and held for the whole window, is
// untouched).
use std::sync::{Mutex, TryLockError};

struct GpuWindow(Mutex<()>);

impl GpuWindow {
    /// Err(names the failure) unless the CALLER provably holds the window
    fn assert_caller_holds(window: &GpuWindow) -> Result<(), String> {
        match window.try_lock() {
            // the try-lock SUCCEEDED -> nobody holds the window -> refuse
            Ok(_would_be_holder) => Err(
                "gpu-window: not held — the caller must own the exclusive GPU window \
                 (take it in the ranked workflow before booting a resident); this helper \
                 never takes the lock itself"
                    .to_string(),
            ),
            Err(TryLockError::WouldBlock) => Ok(()), // HELD by the caller — correct
            Err(TryLockError::Poisoned(_)) => {
                Err("gpu-window: poisoned — a window holder panicked; fix that first".to_string())
            }
        }
    }

    fn boot_resident(window: &GpuWindow) -> Result<Resident, String> {
        Self::assert_caller_holds(window)?;
        Ok(Resident) // boots INSIDE the caller's window, never owning it
    }
}

struct Resident;

fn main() {
    let window = GpuWindow(Mutex::new(()));
    // the ranked workflow: the CALLER takes the lock for the whole window
    let _window_guard = window.lock().unwrap();
    let resident = GpuWindow::boot_resident(&window).expect("caller holds the window");
    let _ = resident;
    // guard drops at window end — the resident never outlives it
}"#.to_string(),
            context_notes: Some(r##"Grade B+ - NotBenchable-by-domain (the deciding metric is EXCLUSIVE-WINDOW CUSTODY - whether a helper can leak residency past its lease or run with no window at all - no validator prices that in cycles; a measurement-infrastructure gain, not a latency gain). SOURCE, verbatim anchors (Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923, MIT): tools/resident-up.sh:46-51 - "# GPU LOCK -- NOT TAKEN HERE, BUT REQUIRED. The caller owns the box GPU window (the ranked workflow holds /tmp/mtplx-gpu-exclusive.lock for the whole measurement) and this script boots the resident inside it. A resident holds ~18.5 GB of unified memory, so it must never outlive that window. The script checks that the lock is HELD (a non-blocking try-lock must fail) and refuses by name when nobody holds it. It never takes the lock itself, because a lock this script took would be released when this script exits, which is not the window." THE MECHANISM: flock -n IS the try-lock probe - the helper attempts a NON-BLOCKING acquire of the same lock file and treats SUCCESS as the failure signal (nobody holds the window), WouldBlock as the correct posture (held). THE LIVENESS RIDER (:438-445): "ONE RESIDENT PER BOX, ACROSS TREES. The pidfile below lives under the tree that booted, so it cannot see a resident booted from the OTHER tree -- and under the paired design there are exactly two trees. A box-wide scan is what catches leg 2 booting before leg 1 was stopped, which would double-load 18.5 GB." - the pidfile is PER-TREE and therefore blind across trees; the box-wide process scan (pgrep -f 'bench-worker resident') is the cross-tree liveness check, and its refusal names the remedy (stop the previous leg; "Nothing has been loaded."). THE ONE-PER-BOX CHECK ITSELF is a FACE of conditional-reuseaddr-after-liveness-probe (Batch 337 - the refuse-when-a-live-instance-answers half at the socket/pidfile surface); the LOCK-HELD-ASSERTION-NEVER-SELF-ACQUIRE axis is the new content - the negative test (try-lock must FAIL) inverts the usual "acquire before touching the resource" instinct precisely because the resource's lease belongs to a CALLER whose lifetime the helper cannot own. THE WHY IS MEASURED AT HOME: GPU-exclusivity during correctness gates is the Bench-649 12.3x-swing discipline (riir-ai AGENTS) - a resident outside the window is exactly that class. KINSHIPS (adjudicated): conditional-reuseaddr-after-liveness-probe (B337) owns REFUSE-WHEN-LIVE at the bind surface + the probe-first posture - the one-per-box arm cites it, the custody-assertion arm is THIS rule; reserve-then-commit-capacity-accounting (entries_58) owns capacity RESERVATION accounting - different concern (budget, not custody); singleflight-identical-work-per-content-key (entries_86/B368) owns leader ELECTION for duplicate work - not window custody; pending-claim-registry-take-consumes-drop-clears (B365) owns claim CONSUMPTION - different lifecycle; fail-stop-deadline-trap (gpu_153) is the in-kernel deadline cousin - different layer; the GPU-exclusivity preflight is house PRACTICE (rig status / bench_preflight) - THIS is its corpus-shaped law, the general form: any helper inside a caller-owned lease asserts the lease is held via its own failing try-lock and refuses by name. Regime: bench/e2e boot helpers inside an exclusive GPU/lease window; resident/daemon booters under a benchmark lease; sandbox/profile setup scripts inside a job's lease; any long-lived artifact (resident memory, mounted device, spawned daemon) whose lifetime must stay inside a lock a HELPER did not take. ANTI-REGIME: helpers that legitimately own their own short lock (a config-file update - acquire, mutate, release IS the contract); advisory idempotence guards (an "already running" short-circuit needs no custody assertion); read-mostly helpers with no exclusive window in the protocol. Suggestion-only. Batch 402 / Plan 402 / Research 437 (mlxfast-bonsai2-27b-engine delta cfb35b0d..0c7c3923, the arena reorg's resident-worker infra). Fixture original."##.to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
