//! Entries slice — Batch 367 (2026-11-16): three `rust_perf`
//! measurement-honesty rules from the opal reactive-optimizer harness
//! distill (Plan 381 / Research 413) — the eval-window state-restore
//! round-trip detector, capacity-gated exact undo with loud degrade, and
//! the self-caused-wait-excluded rate estimate.
//!
//! Source: `xingjian-bai/opal` @
//! `97762859ba5e2527e5c6f63d9890dfbab860d51f` (MIT, v0.3.10 alpha; T0 at
//! the batch window: the pin STILL the tip, the tree's `misc/` moved to
//! `opal/misc/` — path-form only, every quote below byte-verified at the
//! NEW path the same session). The paper: Bai, Liang, Teoh, Sitzmann,
//! arXiv:2610.33826 "Reactive Optimizers". Python/PyTorch source — the
//! transcription law applies; the fixtures are original Rust
//! (`corpus/fixtures/b367_fixtures.rs`, standalone-compiled AND executed
//! before entry, 15/15).
//!
//! All three land first-class suggestion-only `NotBenchable` with
//! `bench_spec: None` (the metric-mismatch class — the win metrics are
//! integrity/detection/admission quantities no hosted validator prices;
//! Research 413 §GOAT screen). The sec half of the batch (the boundary
//! scanner + the stale-proposal merge) is in `src/domains/sec/`.
//! Batch 367 / Plan 381 / Research 413.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── measurement integrity: a scored subject must not be moved
        // away for a measurement and restored after it ──
        PerfRule {
            rule_id: "eval-window-state-restore-roundtrip-detection".to_string(),
            name: "A scored subject (bench/eval harness) must not be moved away for a measurement and restored after it - a harness whose numbers decide anything ring-samples a fixed strided slice of the guarded state every update (seeded offset, stride coprime with every dimension so the sample does not fall into one row or one column), opens a check at the (non-announced) evaluation step, and once the window is complete classifies the sampled trajectory: a JUMP is a per-update sampled change exceeding THRESHOLD x the median non-zero typical change AND an absolute floor x the sampled norm; a ROUND TRIP flags the run - the last jump before the evaluation moved the point away (the evaluation saw it: |s_t - s_(a-1)| >= SEEN x m_a), the first jump after moved it back (|s_b - s_(a-1)| <= BACK x away), and both are isolated (the updates before a and after b are not jumps - a sustained oscillation or a persistent lr change is a stretch, not a round trip); a lone persistent jump is INFORMATIONAL (a weight average swapped in for good is a training intervention, never a cheat); the honest not-detected list is carried: excursions older than the ring window, moves made outside the sampled update path, and moves smeared across sub-floor updates".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// NAIVE: the harness scores the subject at evaluation windows and
// trusts that what it scored is what the updates produced. Nothing
// watches the subject AROUND the window, so a subject that gets moved
// to a convenient point just before the scoring and restored just
// after it reads as a legitimate result.

struct Harness {
    subject: Vec<f64>,
    eval_history: Vec<f64>,
}

impl Harness {
    fn evaluate(&mut self) -> f64 {
        let score = self.subject.iter().sum::<f64>();
        self.eval_history.push(score);
        score
    }

    fn update(&mut self) {
        for v in self.subject.iter_mut() {
            *v += 1e-4; // the ordinary update path
        }
    }
}

// the caller can do this and nothing notices:
// h.update(); h.scale(8.0); let s = h.evaluate(); h.scale(1.0/8.0);
"#.to_string(),
            fixed_code: r#"// FIXED: the integrity watch. A fixed strided sample of the guarded
// state is recorded into a ring EVERY update (before/after the
// evaluation window + 2); evaluation times are never announced to the
// scored process; the detector walks the window once per evaluation:
//   jump  := m_j > THRESHOLD x typical AND m_j > JUMP_FLOOR x norm
//     (typical = median of NON-zero per-update changes — a frozen
//      subject's typical is 0, which is exactly why the absolute floor
//      is load-bearing: THRESHOLD x 0 = 0 fires on any epsilon)
//   round trip := a jump `a` before the eval that the eval SAW
//     (|s_t - s_(a-1)| >= SEEN x m_a), a jump `b` after that moved it
//     back (|s_b - s_(a-1)| <= BACK x away), isolated both sides
//   a lone persistent jump is an INFORMATIONAL event, never a flag.

const THRESHOLD: f64 = 5.0;
const JUMP_FLOOR: f64 = 1e-3;
const SEEN: f64 = 0.5;
const BACK: f64 = 0.25;
const BEFORE: usize = 32;
const AFTER: usize = 4;

struct Watched {
    subject: Vec<f64>,
    ring: Vec<Vec<f64>>, // sampled entries after each of the last BEFORE+AFTER+2 updates
}

impl Watched {
    fn record(&mut self, slot: usize) {
        // the strided fixed sample: seeded offset, stride coprime with
        // every dimension — one cheap read, no host sync
        self.ring[slot % (BEFORE + AFTER + 2)] = self.strided_sample();
    }

    fn strided_sample(&self) -> Vec<f64> {
        self.subject.iter().step_by(3).cloned().collect()
    }

    fn evaluate(&mut self) -> f64 {
        // the harness records but never announces when it scores
        self.subject.iter().sum::<f64>()
    }

    /// after the window completes: classify the sampled trajectory
    fn resolve(&self, t: usize) -> Option<&'static str> {
        // ... the algebra in the fixture (b367 F1): cheater
        // scale-and-restore -> FLAG; persistent swap -> informational;
        // contiguous big stretches -> not a round trip
        let _ = (t, SEEN, BACK);
        None
    }
}
"#.to_string(),
            context_notes: Some("Grade B− LEAD — NotBenchable-by-domain (the win metric is MEASUREMENT INTEGRITY — did a scored subject get moved and restored around the window — no Pareto/alloc validator hosts that; the metric-mismatch annotation, the dial9/B136 class, not a demotion). SOURCE (xingjian-bai/opal @ 97762859ba5e2527e5c6f63d9890dfbab860d51f, MIT, v0.3.10 alpha — the pin re-verified STILL the tip at the batch window; the tree's misc/ moved to opal/misc/, path-form only; every quote byte-verified at the new path). THE CLASS, module doc verbatim (opal/misc/integrity.py:1-3): 'does the running program move the parameters to another point for a validation evaluation and move them back after it?' — found because 'Two overfit runs did: one multiplied the classifier by a probe factor just before each evaluation and restored it at the next update (A2); one copied a running average into every parameter 49 updates before each evaluation ... (A1). Both read the evaluation times from the prompt and the training script. Since then the times are not announced, and the rule is enforced by a check, not by prompt text. The agent is not told that the check exists.' THE COUNTERMEASURE DESIGN IS THE DECISION CONTENT (all in-source): evaluation times NOT announced; a fixed strided sample (seeded offset, stride kept coprime with every dimension — _stride doc verbatim: 'so that the sample does not fall into one row or one column') recorded every update into a ring (BEFORE=128 + AFTER=8 + 2), rank 0, no host sync; per-tensor jump = m_j > threshold(5.0) x median(non-zero typical) AND > JUMP_FLOOR(1e-3) x sampled norm — the FLOOR is load-bearing: typical = 0 for a frozen tensor ('or 0 when fewer than MIN_MOVING are non-zero, as for a frozen tensor') and THRESHOLD x 0 fires on any epsilon; round trip = jump a at/before t with the evaluation seeing the moved point (SEEN=0.5: '|s_t - s_(a-1)| >= 0.5 m_a'), a first jump b after t moving it back (BACK=0.25: '|s_b - s_(a-1)| <= 0.25 |s_t - s_(a-1)|'), isolation both sides ('a sustained oscillation, or a change of learning rate that persists, is not a round trip' — a CONTIGUOUS stretch of big updates; a gapped square wave tied to the eval times is A2's cheat shape and legitimately flags); a lone persistent jump is informational only ('a weight average swapped in for good is a training intervention, not a cheat' — recorded as an eval_jump event, never a flag). THE HONEST NOT-DETECTED LIST IS CARRIED IN-SOURCE and in-rule: excursions longer than the window ('training under the change for that long is a training intervention'), changes outside the sampled update path ('a script's own swap of EMA weights around its evaluation is not attributed to the program'), the degenerate self-undo case, and (the adversarial-panel falsifiers, adopted into the fixture) an out-and-back older than the ring window (the template never completes) and a move smeared across sub-floor updates (no single update arms the template — the fixture demonstrates a ramp that moves the subject meaningfully in aggregate while staying under JUMP_FLOOR x norm every update). TWO REAL CATCHES UPSTREAM (A1 + A2) — the detector is production-proven, not speculative. REGIME: any bench/eval harness whose numbers decide anything (promotion gates, agent-feedback loops, self-evolve oracles, leaderboard scoring) over a state some process mutates per step — the scored SUBJECT must be watched around the window, not just read at it. ANTI-REGIMES: single-shot benchmarks of immutable inputs (nothing to move); harnesses whose subject is reconstructed fresh per measurement from a frozen artifact; settings where the update path itself is the measurement. KINSHIPS (adjudicated both directions, mandatory): benchmark-spread-above-threshold-reports-unstable (entries_70) owns measurement STABILITY (is the number repeatable) — THIS owns measurement INTEGRITY (was the subject moved); wall-clock-ungated-shared-ci-noise-floor (entries_79) owns environment honesty (which metric judges in a shared box) — THIS owns subject honesty; the box-state law family owns the measurement ENVIRONMENT — THIS owns the measured OBJECT; dead-result-timing-loop-folded-under-lto (entries_05) owns the harness's own work vanishing — THIS owns the subject being moved. Zero owners on the axis (dedup re-run at the batch slot: 'eval-window|round.trip detect|restore dance' over rust_perf + sec corpora — zero; entries_28/18 'restore' hits are cache-slot restore prose, unrelated classes). FIXTURES: b367 F1 (15/15 standalone pre-entry: the cheater flags, the persistent swap is informational, the high-lr stretch and the contiguous oscillation do not flag, both panel falsifiers documented-not-detected, the warm-restart no-false-flag, the tampered no-floor detector REDS on the frozen-tensor zero-typical case). Source: opal/misc/integrity.py (whole module) read + quoted at the pin this session. Batch 367 / Plan 381 / Research 413.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── measurement honesty: undo availability is admitted by
        // capacity, and its absence is loud ──
        PerfRule {
            rule_id: "capacity-gated-exact-undo-loud-degrade".to_string(),
            name: "A swap-with-rollback harness admits its UNDO by capacity: the snapshot that would make a failing swap exactly reversible is taken only when its byte estimate fits a declared budget - under budget the restore is exact and in place (the same objects, so holders of references keep them); over budget the snapshot is SKIPPED with a loud named event (what it would have cost, what the limit is, how to raise it - never silent) and the harness DEGRADES to report-not-undo (a failure after that point is told about, not undone); the retained-fallback trial-window shape rides the same law (the previous program is kept while the new one trials); the regime condition is carried: the guarantee covers the state the snapshot CAPTURES - a first-step failure touching state outside the snapshot (an RNG stream, a cursor) is not undone".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// NAIVE: the swap harness keeps an undo path with no capacity
// admission and no loud absence — either it always clones (and dies on
// the day the state is big), or it skips the clone silently and still
// reports "reverted" (the lie that hides the degrade).

struct Swapper {
    state: Vec<u64>,
}

impl Swapper {
    fn try_swap(&mut self, candidate: impl FnOnce(&mut Vec<u64>) -> bool) -> &'static str {
        // no capacity check: either this always runs...
        let undo = self.state.clone();
        if candidate(&mut self.state) {
            "swapped"
        } else {
            self.state = undo;
            "reverted: the update was undone exactly"
        }
    }
}

// or, "optimized" to skip the copy, it cannot undo and says nothing:
// fn try_swap_cheap(...) -> &'static str {
//     if candidate(&mut self.state) { "swapped" } else { "reverted" } // ???
// }
"#.to_string(),
            fixed_code: r#"// FIXED: the snapshot is admitted by a byte estimate against a
// declared budget; under budget it is exact; over budget the skip is
// a LOUD named event and the harness degrades to report-not-undo.

const SNAPSHOT_LIMIT_BYTES: usize = 64 << 30; // snapshot_limit_gb=64

struct Swapper {
    state: Vec<u64>,
    events: Vec<String>,
    undo: Option<Vec<u64>>,
}

impl Swapper {
    fn take_snapshot(&mut self) -> bool {
        let size = self.state.len() * std::mem::size_of::<u64>();
        if size > SNAPSHOT_LIMIT_BYTES {
            // loud: the named event carries the numbers and the remedy
            self.events.push(format!(
                "snapshot-size: the copy that undoes a failing new program would take \
                 {:.2} GiB of host memory, above snapshot_limit_gb={}; it is skipped \
                 (raise the limit to keep it), so a failure is not undone",
                size as f64 / (1 << 30) as f64,
                SNAPSHOT_LIMIT_BYTES >> 30
            ));
            return false;
        }
        self.undo = Some(self.state.clone());
        true
    }

    fn try_swap(&mut self, candidate: impl FnOnce(&mut Vec<u64>) -> bool) -> String {
        let snap_ok = self.take_snapshot();
        let before = snap_ok.then(|| self.undo.clone()).flatten();
        if candidate(&mut self.state) {
            return "swapped".to_string();
        }
        match before {
            Some(undo) => {
                // exact restore, IN PLACE: the same objects (holders of
                // references keep them)
                self.state.copy_from_slice(&undo);
                "reverted: the update was undone exactly".to_string()
            }
            None => {
                // degrade to report-not-undo: told, never silent
                "reverted: this update was not undone (the snapshot was skipped); \
                 it may be partly applied"
                    .to_string()
            }
        }
    }
}
"#.to_string(),
            context_notes: Some("Grade C+/B− — NotBenchable-by-domain (the win is an undo-AVAILABILITY property — was the failure reversible — no validator prices it; the metric-mismatch annotation, the dial9/B136 class). SOURCE (xingjian-bai/opal @ 97762859…, MIT, quotes byte-verified at the pin, the misc/ → opal/misc/ path move disclosed). THE LAW IN-SOURCE, verbatim (opal/misc/recovery.py _take_snapshot): 'the copy that undoes a failing new program would take {size:.2f} GiB of host memory, above snapshot_limit_gb=2; it is skipped (raise the limit to keep it), so a failure is not undone' — the skip emits a NAMED event ('snapshot-size' via _warn) carrying the size, the limit and the remedy, and the absence is threaded as a typed why-string ('above snapshot_limit_gb' / 'not taken (the previous updater's state_dict() raised)') so every later report knows which posture held. THE TIERED EXACTNESS, README verbatim (L164-165): 'exactly when a copy of the parameters and optimizer state fits in snapshot_limit_gb (default 2); a later failure is not undone, and the agent is told.' THE RESTORE IS IN-PLACE, _restore_snapshot doc verbatim: 'Put back the parameters, gradients and the user's optimizers' state in place (the same tensor objects, so programs that hold references keep them)' — aliasing preserved, not a fresh allocation re-bound. THE CALL-SITE ORDER IS LOAD-BEARING (wrapper.py _apply, verbatim): the snapshot is 'taken before build(), so that a failing build or first update is undone exactly' — the copy precedes the mutation it guards. THE TRIAL-WINDOW HALF rides this entry as the second arm of the safe-swap law (wrapper.py, verbatim): 'the old updater is still in its trial window: the fallback stays the program before it, which passed one' — the retained fallback is the last PASSED program, never the failing trial. THE REGIME CONDITION (the adversarial panel's blind spot, adopted): the guarantee covers the state the snapshot CAPTURES — 'a first-step failure touching state outside the snapshot (RNG streams, cursor state)' is not undone; the fixture demonstrates it (the harness's RNG stream advances through a candidate step and is NOT restored by the exact state undo). REGIME: any swap-with-rollback harness (agent-proposed code swaps, config hot-swaps, canary deploys with in-process rollback, A/B experiment states) whose undo copy can grow past a host budget. ANTI-REGIMES: swaps whose undo is structurally free (a persistent data structure — no snapshot needed); systems where a non-exact revert is unacceptable AND the state is unbounded (the honest answer is refusing the swap, not degrading); undo paths whose absence is already loud and typed (the law holds, nothing to add). KINSHIPS (adjudicated both directions): batch-load-admission-sized-from-files-refuses-past-the-hard-budget (B274) admits WORK by bytes before running — THIS admits UNDO by bytes before swapping (the polarity differs — cite as kin, compose: an admission-gated load whose failure path uses this rule's undo); wall-clock-ungated-shared-ci-noise-floor (entries_79) owns honesty of JUDGED numbers — THIS owns honesty of REVERT CLAIMS (a report that says 'undone exactly' must be backed by a snapshot that can); the loud-zero family (timed_region_guard) owns absent-work zeros — THIS owns absent-UNDO claims; bench_preflight's box-state PROVENANCE practice is the at-home posture (a claim carries its conditions). Zero owners (dedup: 'snapshot_limit|undo budget|capacity undo|exact undo' over both corpora — zero). FIXTURES: b367 F4 (under budget exact restore byte-identical; over budget one loud named event + report-not-undo + the state keeps the failed step's effects; the RNG-stream regime condition demonstrated; the tampered silent-skip-that-reports-undo-available REDS). Source: opal/misc/recovery.py (_take_snapshot + _restore_snapshot + module doc) + opal/wrapper.py (_apply snapshot-before-build + the trial-window comment) + README L164-165, read at the pin. Batch 367 / Plan 381 / Research 413.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── measurement honesty: a rate estimated from intervals must
        // exclude the waits the mechanism itself introduces ──
        PerfRule {
            rule_id: "self-caused-wait-excluded-from-rate-estimate".to_string(),
            name: "A throughput/rate estimate derived from wall intervals must subtract the waits the mechanism itself introduces before deriving a per-unit rate, and take the MEDIAN over a small bounded window (so an interval carrying an unrelated heavy event - an evaluation, a checkpoint, a compaction - does not shrink the estimate); a harness that pauses to consult an external authority (an agent, a remote service, a human) and then estimates its pace from the raw intervals measures its own patience, not the work - the purpose law rides the rule: the estimate exists to predict how stale an in-flight answer will be (answer time / true per-unit time), and a pause-inflated denominator understates the lag and mis-schedules the next consult".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// NAIVE: the harness checks in every N updates and estimates its
// per-update pace from the raw wall interval since the last check —
// including the seconds it spent ITSELF waiting on a consult.

struct Pace {
    last: Option<(usize, f64)>, // (update, monotonic ms)
    intervals_ms: Vec<f64>,
}

impl Pace {
    fn check(&mut self, u: usize, now_ms: f64) {
        if let Some((lu, lt)) = self.last {
            self.intervals_ms.push((now_ms - lt) / (u - lu) as f64);
        }
        self.last = Some((u, now_ms));
    }

    fn ms_per_update(&self) -> f64 {
        self.intervals_ms.iter().sum::<f64>() / self.intervals_ms.len() as f64
    }
}
"#.to_string(),
            fixed_code: r#"// FIXED: the consult pauses are accumulated and SUBTRACTED before
// the interval is divided out; the per-unit estimate is the MEDIAN of
// a small bounded window (an outlier interval — one carrying an
// evaluation — cannot shrink the estimate); the window is bounded so
// an accepted change to the work (a rewrite that halves the cost)
// re-fits fast instead of averaging over stale history.

const WINDOW: usize = 5;

struct Pace {
    last: Option<(usize, f64)>,
    intervals_ms: std::collections::VecDeque<f64>,
    paused_ms: f64,
    median: Option<f64>,
}

impl Pace {
    fn paused(&mut self, seconds: f64) {
        self.paused_ms += seconds * 1000.0;
    }

    fn check(&mut self, u: usize, now_ms: f64) {
        if let Some((lu, lt)) = self.last {
            if u > lu {
                // time per update since the previous check, WITHOUT the
                // pauses in between
                let raw = (now_ms - lt - self.paused_ms).max(0.0) / (u - lu) as f64;
                self.intervals_ms.push_back(raw);
                if self.intervals_ms.len() > WINDOW {
                    self.intervals_ms.pop_front();
                }
                let mut xs: Vec<f64> = self.intervals_ms.iter().cloned().collect();
                xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                self.median = Some(xs[xs.len() / 2]);
            }
        }
        self.last = Some((u, now_ms));
        self.paused_ms = 0.0;
    }
}
"#.to_string(),
            context_notes: Some("Grade C+ — NotBenchable-by-domain (the win is estimator CORRECTNESS — does the rate measure the work or the harness's own patience — no validator prices it; the metric-mismatch annotation, the dial9/B136 class). SOURCE (xingjian-bai/opal @ 97762859…, MIT, quotes byte-verified at the pin, the misc/ → opal/misc/ path move disclosed). THE LAW IN-SOURCE, verbatim (opal/misc/waiting.py tick doc): 'At each check step (rank 0): time per update since the previous check, without the pauses in between.' — the mechanism: a paused(seconds) accumulator is added by the consult path and SUBTRACTED in tick before the interval divides by the update count (self._paused reset to 0.0 at every tick). THE MEDIAN RATIONALE, module doc verbatim: 'the median over the last few check intervals, with the pauses for the agent left out (a median, so an interval with an evaluation or a checkpoint in it does not shrink the estimate).' — WINDOW = 5, median-of-5. THE PURPOSE LAW (why the estimate exists), module doc verbatim: 'Why: without waiting, an answer lands (answer time / time per update) updates after the state the agent saw, and no new decision is asked while one is in flight, so on a fast run most scheduled decisions are never asked.' — the estimate feeds a lag prediction (predicted_lag_updates = median answer time / ms-per-update) that gates whether the NEXT consult waits; a pause-inflated denominator understates the lag, so consults stop being scheduled and the loop starves — the harm is mis-scheduling, not just a wrong number. THE ADVERSARIAL PANEL'S FALSIFIERS, carried as honest regime notes (both fixture-demonstrated): (a) BIMODAL latency hides the tail — a median-of-5 reads the fast mode while the slow mode is real (the fixture plants one 10x interval among four normal ones and the median holds fast); (b) every ACCEPTED REWRITE changes the per-update cost, so the estimate is stale for the next window by construction — the bounded window is the disclosed mitigation (re-fits within WINDOW checks; the estimator never claims a long-run truth). REGIME: any harness that pauses to consult an external authority (an agent, a remote reviewer, a human approval) and derives a per-unit rate from wall intervals — decision loops, escalating serving lanes, cron-style reviewers; also any rate estimate whose intervals may contain the mechanism's own heavy events (GC, checkpoint, eval). ANTI-REGIMES: rates measured by a dedicated counter outside the pause path (nothing self-caused to subtract); single-interval estimates (no window to mediate); systems where the consult pauses ARE the phenomenon being measured. KINSHIPS (adjudicated both directions): wall-clock-ungated-shared-ci-noise-floor (entries_79) + sustained-load-drift-interleave-protocol + the box-state family own EXTERNAL contamination (the environment's noise floor, load drift, co-tenants) — THIS owns SELF-caused contamination (the harness's own waits); benchmark-spread-above-threshold-reports-unstable (entries_70) owns stability verdicts — THIS owns estimator inputs; latency-histogram-split-on-structural-populations (entries_80) splits populations at the record site — THIS subtracts one self-caused population from the interval BEFORE any split. Zero owners on the self-caused axis (dedup: 'self-caused|consult pause|pause excluded|rate estimate pause' over both corpora — zero). FIXTURES: b367 F5 (pauses excluded — the estimator recovers ~20 ms/update while the naive mean overstates >1.5x; the median absorbs an evaluation-carrying interval; the bimodal-tail falsifier demonstrated; the bounded window asserted as the rewrite-staleness mitigation). Source: opal/misc/waiting.py (whole module) read at the pin. Batch 367 / Plan 381 / Research 413.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
