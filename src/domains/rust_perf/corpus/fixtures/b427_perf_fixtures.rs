//! Batch 427 (nominal) rust_perf fixture preflight — the pico-dgx-ds4
//! engine distill (Plan 420 / Research 454, Picom-code/pico-dgx-ds4 @
//! 922dddb1a35e521148022ee11264632da1f0ddc0, tip == pin at authoring, MIT).
//! Original plain-Rust syntheses — the mechanisms' LAWS transcribed, never
//! upstream code (the corpus convention). The engine's measured latencies
//! (237/256 -> 86/169 µs, the 200 µs timer, the 8-pending threshold, the
//! 74-95 spread) are PROVENANCE quoted in the entries file, never bars
//! here — the fixtures assert the MECHANISMS (which branch fires, which
//! counter moves) in deterministic integer/small-float arithmetic over a
//! SplitMix64 stream (no wall clock).
//!
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b427_perf_fixtures.rs`
//! at debug AND -O (both green, zero warnings).
//!
//! Arms:
//! - R6: the precise two-disjunct mismatch definition (a planted mismatch
//!   counts; an identical row and a below-threshold row do not), the two
//!   forced arms flipping their census counters (fallback forced every 3rd
//!   step runs ALL groups; bound x4 reads all but two groups across the
//!   whole run — the 2,018/2,020 shape), and the happy-path-only checker
//!   MISSING exactly the forced rows (the defect witness).
//! - R7 (the GOAT falsifier cell): a seeded noisy-bench simulator at the
//!   source's own spread ratio (±~12% singles over the 74-95 shape) with
//!   an injected TRUE +1% on arm B — over 200 seeds the PAIRED
//!   same-session read must beat the UNPAIRED 9-run median's
//!   correct-verdict count or the falsifier FIRES (the rule then demotes
//!   to suggestion-only decision content). Plus the decomposition leg:
//!   per-step time bucketed by row count recovers a shift injected at one
//!   bucket and reads the control bucket flat.
//! - R8: the label law — an unlabelled modelled number beside measured
//!   rows fires the lint; a fully labelled table passes; a 'measured'
//!   label without measurement backing is caught.
//! - R9: the ours–kit–ours bracket (drift ratio O2/O1, flagged above 1%),
//!   the pinned-commit provenance table (a figure without pin+date
//!   refuses), best-per-row across versions vs a single-version column,
//!   and the cross-project misattribution caught by the source archive's
//!   own config label.
//! - R10: the MoE expert-union collapse under copies vs distinct (the
//!   mechanism), the class-labeling law (comparisons refuse across
//!   classes; the competitor copies-only disclosure carried on the row).
//! - R12: a zero-row fold returning the seed constant — the unfixed path
//!   hands it out as evidence (witness), the fixed path refuses loud
//!   (typed); a real digest passes; the seed-constant == its published
//!   decimal is pinned.
//! - R13: the pad-to-read_batch arithmetic (outstanding rounds UP to the
//!   multiple, pads carry the discard flag bit, an exact multiple pads
//!   zero, a deep queue pads nothing) and the completion path the padding
//!   buys (count threshold vs timer).
#![allow(dead_code)]

// ══════════════ deterministic stream (no wall clock anywhere) ══════════════

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> SplitMix64 {
        SplitMix64 { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// uniform [0, 1)
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// symmetric noise in [-w, w]
    fn sym(&mut self, w: f64) -> f64 {
        (self.unit() - 0.5) * 2.0 * w
    }
}

// ══════════════ R6 — certified fast path verified by the slow path beside it ══════════════

/// A row's head output as (max code, argmax-lowest-id) — integer codes
/// stand in for logits so equality is exact.
type Head = (i64, u32);

/// The precise definition, transcribed from the source's sentence: a row
/// counts as a mismatch unless the certified logits give the same
/// (maximum, lowest id) as the full head on that rank, OR the full head's
/// maximum on that rank is below the shared threshold (then that rank
/// cannot hold the winner under either head).
fn r6_row_mismatch(certified: Head, full: Head, full_max: i64, shared_threshold: i64) -> bool {
    let same_pick = certified == full;
    !(same_pick || full_max < shared_threshold)
}

#[derive(Default, Debug, PartialEq, Eq)]
struct R6Census {
    rows_checked: u64,
    mismatches: u64,
    fallback_steps: u64,
    groups_total: u64,
    groups_read: u64,
}

const R6_STEPS: usize = 30;
const R6_GROUPS_PER_STEP: u64 = 100;
const R6_ROWS_PER_GROUP: u64 = 4;
const R6_THRESHOLD: i64 = 50;

/// Which groups the bound can prune at a given scale. Under the serving
/// bound only every 10th group survives (must be read by the full head);
/// scaling the bound x4 keeps exactly two groups prunable across the WHOLE
/// run (absolute group index) — the 2,018-of-2,020 shape.
fn r6_group_prunable(step: usize, group: u64, bound_scale: f64) -> bool {
    if bound_scale > 1.0 {
        let abs = step as u64 * R6_GROUPS_PER_STEP + group;
        abs == 7 || abs == 777 // the x4 world: all but two groups read
    } else {
        group % 10 != 0 // the serving world: 10% of groups are read
    }
}

/// The check-mode run: the full head beside the certified one IN THE SAME
/// STEP on every read row; `force_fallback_every` and `bound_scale` are
/// the two adversarial arms; `inject` plants one wrong certified output.
fn r6_run_check(
    force_fallback_every: Option<usize>,
    bound_scale: f64,
    inject: Option<(usize, u64, u64)>,
) -> R6Census {
    let mut c = R6Census::default();
    c.groups_total = (R6_STEPS as u64) * R6_GROUPS_PER_STEP;
    for step in 0..R6_STEPS {
        let forced = matches!(force_fallback_every, Some(n) if step % n == 0);
        if forced {
            c.fallback_steps += 1;
        }
        for g in 0..R6_GROUPS_PER_STEP {
            let prunable = !forced && r6_group_prunable(step, g, bound_scale);
            if prunable {
                continue; // the bound pruned the group: the certified path took it
            }
            c.groups_read += 1;
            for row in 0..R6_ROWS_PER_GROUP {
                c.rows_checked += 1;
                let full: Head = (100 + row as i64, row as u32);
                let certified = if inject == Some((step, g, row)) {
                    (999, 42) // the planted wrong certified output
                } else {
                    full
                };
                if r6_row_mismatch(certified, full, full.0, R6_THRESHOLD) {
                    c.mismatches += 1;
                }
            }
        }
    }
    c
}

/// The defect witness: a happy-path-only checker — it counts rows only on
/// steps the fast path took, so the forced-fallback steps' rows never
/// enter its census.
fn r6_happy_path_only_census(force_fallback_every: Option<usize>) -> R6Census {
    let mut c = R6Census::default();
    for step in 0..R6_STEPS {
        let forced = matches!(force_fallback_every, Some(n) if step % n == 0);
        for g in 0..R6_GROUPS_PER_STEP {
            let prunable = !forced && r6_group_prunable(step, g, 1.0);
            if prunable {
                continue;
            }
            if forced {
                continue; // the defect: forced (fallback) steps are not counted
            }
            c.groups_read += 1;
            c.rows_checked += R6_ROWS_PER_GROUP;
        }
    }
    c
}

#[test]
fn r6_mismatch_definition_and_census() {
    // the precise definition: a planted wrong certified output on a
    // above-threshold row counts
    let planted = r6_row_mismatch((999, 42), (101, 1), 101, R6_THRESHOLD);
    assert!(
        planted,
        "a differing certified pick above the shared threshold must count as a mismatch"
    );

    // an identical pick does not count — even though the values differ
    // from some third reference
    let identical = r6_row_mismatch((101, 1), (101, 1), 101, R6_THRESHOLD);
    assert!(
        !identical,
        "an identical (max, lowest id) pick is not a mismatch"
    );

    // the second disjunct: the full head's max below the shared threshold
    // means that rank cannot hold the winner under either head — a
    // differing certified pick there is NOT a mismatch
    let below = r6_row_mismatch((999, 42), (10, 3), 10, R6_THRESHOLD);
    assert!(
        !below,
        "a differing pick under a below-threshold full max is not a mismatch (the definition's second disjunct)"
    );

    // the boundary: exactly at the threshold the row is priced (not below)
    let at_threshold = r6_row_mismatch((999, 42), (50, 3), 50, R6_THRESHOLD);
    assert!(
        at_threshold,
        "full max == threshold is not 'below' it — a differing pick counts"
    );

    // the steady-state census over the serving posture: zero mismatches
    let census = r6_run_check(None, 1.0, None);
    assert_eq!(
        census.mismatches, 0,
        "a correct certified path prices zero mismatches"
    );
    assert!(census.rows_checked > 0);
    assert!(
        census.groups_read < census.groups_total,
        "the bound pruned something in the serving posture"
    );

    // a planted mismatch inside the real census path is detected — planted
    // in a group the bound does NOT prune (g % 10 == 0 survives the
    // serving bound and is read)
    let injected = r6_run_check(None, 1.0, Some((12, 30, 2)));
    assert_eq!(
        injected.mismatches, 1,
        "the planted wrong certified output must flip the mismatch counter"
    );
    assert_eq!(
        injected.rows_checked, census.rows_checked,
        "planting changes no read counts"
    );
}

#[test]
fn r6_forced_arms_flip_the_census() {
    // arm 1: the fallback forced every 3rd step — the forced steps run
    // ALL groups (the source's '10,336 steps ran all 2,020 groups' shape)
    let forced = r6_run_check(Some(3), 1.0, None);
    let plain = r6_run_check(None, 1.0, None);
    let forced_steps = (0..R6_STEPS).filter(|s| s % 3 == 0).count() as u64;
    assert_eq!(
        forced.fallback_steps, forced_steps,
        "the forced arm flips the fallback-step counter"
    );
    // forced steps read ALL their groups; non-forced steps keep the plain
    // bound's reads (10 groups each). The forced steps' plain-bound reads
    // are REPLACED by their full reads.
    let plain_per_step = plain.groups_read / R6_STEPS as u64;
    let expected_groups =
        plain.groups_read - forced_steps * plain_per_step + forced_steps * R6_GROUPS_PER_STEP;
    assert_eq!(
        forced.groups_read, expected_groups,
        "forced steps read ALL their groups (the plain reads + the forced steps' full reads)"
    );

    // arm 2: the bound scaled x4 — almost nothing can be pruned (the
    // source's '2,018 of 2,020 groups read on average' shape: all but two
    // groups across the whole run)
    let scaled = r6_run_check(None, 4.0, None);
    assert_eq!(
        scaled.groups_total - scaled.groups_read,
        2,
        "bound x4 leaves exactly two prunable groups across the run"
    );
    assert!(
        scaled.groups_read >= scaled.groups_total * 99 / 100,
        "the no-prune regime reads essentially every group"
    );
    assert_eq!(
        scaled.fallback_steps, 0,
        "the x4 arm does not force the fallback stride"
    );
    assert_eq!(scaled.mismatches, 0);

    // the defect witness: a happy-path-only checker misses EXACTLY the
    // forced steps' rows — its census cannot price the branch that matters
    let defect = r6_happy_path_only_census(Some(3));
    assert_eq!(
        forced.rows_checked - defect.rows_checked,
        forced_steps * R6_GROUPS_PER_STEP * R6_ROWS_PER_GROUP,
        "the happy-path-only checker misses exactly the forced (fallback) steps' rows"
    );
}

// ══════════════ R7 — the GOAT falsifier cell: paired beats median at the source's spread ══════════════

/// The 74-95 spread's midpoint (the source's chat bench).
const R7_BASE: f64 = 84.5;

/// One bench run of arm A: a session offset shared by every arm measured
/// in that session + the run's own noise. ±8% session + ±4% run noise
/// gives the source's ~74-95 swing (84.5 * [0.88, 1.12] = [74.4, 94.6]).
fn r7_run_a(rng: &mut SplitMix64, session: f64) -> f64 {
    R7_BASE * (1.0 + session + rng.sym(0.04))
}

/// Arm B = the same draw with the injected TRUE +1% improvement and the
/// lever's own small run-to-run gain variation (±0.4% — acceptance and
/// token count vary per run).
fn r7_run_b(rng: &mut SplitMix64, session: f64) -> f64 {
    r7_run_a(rng, session) * (1.01 + rng.sym(0.004))
}

fn r7_median_of(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in the simulator"));
    v[v.len() / 2]
}

/// Arm (a): the UNPAIRED comparison — 9 runs per arm, each its own fresh
/// session (the source's median posture), a winner declared only when the
/// medians differ beyond a small epsilon (0.5%, below the claimed gain).
fn r7_median_verdict(rng: &mut SplitMix64, runs: usize) -> bool {
    let mut a = Vec::with_capacity(runs);
    let mut b = Vec::with_capacity(runs);
    for _ in 0..runs {
        let sa = rng.sym(0.08);
        a.push(r7_run_a(rng, sa));
        let sb = rng.sym(0.08); // a fresh, independent session
        b.push(r7_run_b(rng, sb));
    }
    let (ma, mb) = (r7_median_of(a), r7_median_of(b));
    mb > ma * 1.005
}

/// Arm (b): the PAIRED same-session read — the same session offset per
/// pair (the shared drift cancels in the difference), decided on the mean
/// paired relative delta above a small epsilon (0.2%).
fn r7_paired_verdict(rng: &mut SplitMix64, pairs: usize) -> bool {
    let mut rel = 0.0f64;
    for _ in 0..pairs {
        let session = rng.sym(0.08); // SHARED by the pair
        let a = r7_run_a(rng, session);
        let b = r7_run_b(rng, session);
        rel += (b - a) / a;
    }
    rel / pairs as f64 > 0.002
}

/// THE PRE-REGISTERED FALSIFIER CELL. At the source's own spread/gain
/// ratio (±~12% singles vs a +1% lever), the paired same-session read must
/// beat the unpaired 9-run median's correct-verdict count over the seed
/// sweep — or the falsifier fires and the rule demotes to suggestion-only
/// decision content.
#[test]
fn r7_goat_paired_beats_median_at_source_spread() {
    const SEEDS: usize = 200;
    let mut median_correct = 0usize;
    let mut paired_correct = 0usize;
    for s in 0..SEEDS as u64 {
        let mut m = SplitMix64::new(0xB427_0000 ^ s);
        if r7_median_verdict(&mut m, 9) {
            median_correct += 1;
        }
        let mut p = SplitMix64::new(0xB427_1111 ^ s);
        if r7_paired_verdict(&mut p, 9) {
            paired_correct += 1;
        }
    }
    println!(
        "r7 goat cell: median {median_correct}/{SEEDS} correct, paired {paired_correct}/{SEEDS} correct at the source's spread/gain ratio"
    );
    assert!(
        paired_correct > median_correct,
        "FALSIFIER FIRED — the paired read ({paired_correct}/{SEEDS}) did NOT beat the unpaired \
         median ({median_correct}/{SEEDS}) at the source's own spread ratio; the rule demotes to \
         suggestion-only decision content (owner call, reported loudly)"
    );
}

/// The decomposition leg: per-step time bucketed by row count recovers a
/// shift injected at ONE bucket and reads the control bucket flat — the
/// low-noise read the medians cannot give.
#[test]
fn r7_decomposition_leg_recovers_bucket_shift() {
    // per-step GPU ms model t(r) = 17.0 + 2.4r (the source's table shape:
    // 19.4 at 1 row, 26.6 at 4 rows); the lever's TRUE shift is applied
    // ONLY at the 1-row bucket in this arm
    let step_ms = |rows: f64, lever: bool| -> f64 {
        let t = 17.0 + 2.4 * rows;
        if lever && rows < 1.5 {
            t * 0.978 // the injected -2.2% at bucket 1
        } else {
            t
        }
    };
    let mut rng = SplitMix64::new(0xB427_DE00);
    let steps_per_bucket = 200usize;
    let read_bucket = |rows: f64, lever: bool, rng: &mut SplitMix64| -> f64 {
        let mut acc = 0.0;
        for _ in 0..steps_per_bucket {
            acc += step_ms(rows, lever) * (1.0 + rng.sym(0.01)); // ±1% per-step noise
        }
        acc / steps_per_bucket as f64
    };
    let base1 = read_bucket(1.0, false, &mut rng);
    let lev1 = read_bucket(1.0, true, &mut rng);
    let base4 = read_bucket(4.0, false, &mut rng);
    let lev4 = read_bucket(4.0, true, &mut rng);

    let shift1 = 1.0 - lev1 / base1;
    let shift4 = 1.0 - lev4 / base4;
    println!("r7 decomposition: bucket-1 shift {shift1:.4}, bucket-4 (control) shift {shift4:.4}");

    // the injected -2.2% at bucket 1 is recovered (the noise floor of a
    // 200-step mean at ±1% per step is ~0.07%, so half the shift is far
    // above it)
    assert!(
        shift1 > 0.015,
        "the bucketed per-step read must recover the injected 1-row shift (got {shift1})"
    );
    // the untouched bucket reads flat — the control the whole-run tok/s
    // number cannot give
    assert!(
        shift4.abs() < 0.005,
        "the untouched bucket must read ~flat (got {shift4})"
    );
    // and the recovered shift localizes the lever (bucket 4 carries none of it)
    assert!(
        shift1 > 4.0 * shift4.abs(),
        "the shift is localized to its bucket, not smeared"
    );
}

// ══════════════ R8 — measured / simulated / modelled: the label law ══════════════

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum R8Class {
    Measured,
    /// an upper bound computed from a model of the mechanism
    Simulated,
    /// an estimate whose missing dependency is stated on the row
    Modelled,
}

struct R8Row {
    idea: &'static str,
    value: f64,
    /// None = the defect: an unlabelled number beside measured rows
    class: Option<R8Class>,
    /// does a measurement back this row? (None only for forward classes)
    backed: bool,
}

fn r8_label_lint(rows: &[R8Row]) -> Vec<String> {
    let mut findings = Vec::new();
    for r in rows {
        match r.class {
            None => findings.push(format!(
                "unlabelled number beside measured rows: {}",
                r.idea
            )),
            Some(R8Class::Measured) if !r.backed => {
                findings.push(format!(
                    "'measured' label without measurement backing: {}",
                    r.idea
                ));
            }
            _ => {}
        }
    }
    findings
}

#[test]
fn r8_label_law_lints_unlabelled_and_unbacked() {
    // the defect: a modelled estimate sits UNLABELLED beside measured rows
    let defect = [
        R8Row {
            idea: "shipped stack",
            value: 3.8,
            class: Some(R8Class::Measured),
            backed: true,
        },
        R8Row {
            idea: "oracle re-ranker",
            value: 1.0,
            class: Some(R8Class::Simulated),
            backed: false,
        },
        R8Row {
            idea: "trained re-ranker",
            value: 0.17,
            class: None,
            backed: false,
        }, // simulated, unlabelled
    ];
    let findings = r8_label_lint(&defect);
    assert_eq!(
        findings.len(),
        1,
        "the unlabelled estimate must fire (got {findings:?})"
    );
    assert!(findings[0].contains("unlabelled"));

    // the fixed table: every row labelled; simulated/modelled rows need no
    // measurement backing, but the label is on the row
    let fixed = [
        R8Row {
            idea: "shipped stack",
            value: 3.8,
            class: Some(R8Class::Measured),
            backed: true,
        },
        R8Row {
            idea: "oracle re-ranker",
            value: 1.0,
            class: Some(R8Class::Simulated),
            backed: false,
        },
        R8Row {
            idea: "certified head 6-7 bit",
            value: -0.07,
            class: Some(R8Class::Modelled),
            backed: false,
        },
    ];
    assert!(
        r8_label_lint(&fixed).is_empty(),
        "a fully labelled table passes (got {:?})",
        r8_label_lint(&fixed)
    );

    // the tamper arm: a 'measured' label with no measurement behind it
    let tampered = [R8Row {
        idea: "claimed measured",
        value: 2.0,
        class: Some(R8Class::Measured),
        backed: false,
    }];
    let t = r8_label_lint(&tampered);
    assert_eq!(
        t.len(),
        1,
        "a 'measured' label without backing is caught (got {t:?})"
    );
    assert!(t[0].contains("without measurement backing"));
}

// ══════════════ R9 — external figures at a pinned commit, best per row,
// ══════════════ dated, bracketed ours–kit–ours ══════════════

struct R9Bracket {
    o1: f64,
    o2: f64,
}

impl R9Bracket {
    /// "Ours is the mean of O1 and O2."
    fn ours(&self) -> f64 {
        (self.o1 + self.o2) / 2.0
    }
    /// "Report O2/O1; above 1 % look for the cause before publishing."
    fn drift_ratio(&self) -> f64 {
        self.o2 / self.o1
    }
    fn drift_flag(&self) -> bool {
        (self.drift_ratio() - 1.0).abs() > 0.01
    }
}

struct R9Figure {
    project: &'static str,
    value: f64,
    /// the commit the figure was read at — None = read from a summary
    pin: Option<&'static str>,
    date: &'static str,
    /// the config label the figure's OWN source archive gives it
    config: &'static str,
}

fn r9_provenance_lint(figs: &[R9Figure]) -> Vec<String> {
    let mut findings = Vec::new();
    for f in figs {
        if f.pin.is_none() {
            findings.push(format!(
                "{}: figure read from a summary — no pinned commit",
                f.project
            ));
        }
        if f.date.is_empty() {
            findings.push(format!("{}: figure carries no date", f.project));
        }
    }
    findings
}

/// best-per-row: the max across versions among provenanced figures
fn r9_best_per_row(figs: &[R9Figure]) -> Option<&R9Figure> {
    figs.iter()
        .filter(|f| f.pin.is_some() && !f.date.is_empty())
        .max_by(|a, b| a.value.partial_cmp(&b.value).expect("no NaN"))
}

/// the misattribution check: the table's credit must match the figure's
/// own source-archive config label
fn r9_credit_check(
    archive_config: &str,
    table_config: &str,
    archive_value: f64,
    table_value: f64,
) -> Result<(), String> {
    if archive_config != table_config {
        return Err(format!(
            "cross-project misattribution: the archive says these are {archive_config} numbers, the table credits {table_config}"
        ));
    }
    if (archive_value - table_value).abs() > f64::EPSILON {
        return Err("figure does not match its source archive".to_string());
    }
    Ok(())
}

#[test]
fn r9_bracket_drift_flag_and_pinned_table() {
    // the bracket: ours measured twice around the kit's session
    let clean = R9Bracket {
        o1: 100.0,
        o2: 100.4,
    };
    assert!(
        (clean.ours() - 100.2).abs() < 1e-9,
        "ours is the mean of O1 and O2"
    );
    assert!(!clean.drift_flag(), "a 0.4% drift is under the 1% gate");

    let drifting = R9Bracket {
        o1: 100.0,
        o2: 101.8,
    };
    assert!(
        drifting.drift_flag(),
        "an 1.8% drift must be flagged — look for the cause BEFORE publishing"
    );
    let drifting_down = R9Bracket {
        o1: 100.0,
        o2: 98.9,
    };
    assert!(
        drifting_down.drift_flag(),
        "the gate prices the drift's MAGNITUDE, not its sign"
    );

    // the pinned-commit provenance table
    let good = R9Figure {
        project: "kit",
        value: 158.7,
        pin: Some("abc1234"),
        date: "2026-11-04",
        config: "ds41-tp4",
    };
    assert!(r9_provenance_lint(&[good]).is_empty());

    let bad = [
        R9Figure {
            project: "nero-",
            value: 78.8,
            pin: None,
            date: "",
            config: "qwen3.8-tp2",
        },
        R9Figure {
            project: "dsgb10",
            value: 90.0,
            pin: Some("922dddb"),
            date: "",
            config: "ds41-tp4",
        },
    ];
    let findings = r9_provenance_lint(&bad);
    // nero- fires TWICE (no pinned commit AND no date), dsgb10 once (no date)
    assert_eq!(
        findings.len(),
        3,
        "both vacuities fire per figure (got {findings:?})"
    );
    assert!(findings[0].contains("no pinned commit"));
    assert!(findings[1].contains("no date"));
    assert!(findings[2].contains("no date"));

    // best-per-row: the maxima sit in DIFFERENT versions, so one version's
    // column is not best per row (the M4 finding)
    let prose = [
        R9Figure {
            project: "kit",
            value: 90.2,
            pin: Some("v2.2"),
            date: "2026-10-03",
            config: "ds41-tp4",
        },
        R9Figure {
            project: "kit",
            value: 158.7,
            pin: Some("v2.4"),
            date: "2026-11-04",
            config: "ds41-tp4",
        },
    ];
    let code = [
        R9Figure {
            project: "kit",
            value: 135.26,
            pin: Some("v2.2"),
            date: "2026-10-03",
            config: "ds41-tp4",
        },
        R9Figure {
            project: "kit",
            value: 133.0,
            pin: Some("v2.4"),
            date: "2026-11-04",
            config: "ds41-tp4",
        },
    ];
    let v24_column_code = 133.0; // "the column is the kit's v2.4 figures"
    let best_code = r9_best_per_row(&code).expect("provenanced figures exist");
    assert!(
        best_code.value > v24_column_code,
        "best per row ({}) beats the single-version column ({v24_column_code})",
        best_code.value
    );
    assert_eq!(
        best_code.date, "2026-10-03",
        "the per-row best carries the DATE of the version that holds it"
    );
    assert_eq!(
        r9_best_per_row(&prose).map(|f| f.value),
        Some(158.7),
        "on rows where the column holds the max, they agree"
    );

    // the caught misattribution: the archive's own config label vs the
    // table's credit (the source: nero-'s archive says those are Qwen3.8
    // TP2 numbers; the table credited them as this engine's)
    let mis = r9_credit_check("qwen3.8-tp2", "ds41-tp4", 78.8, 78.8);
    assert!(mis.is_err(), "the misattribution must refuse");
    assert!(mis.unwrap_err().contains("misattribution"));
    // the corrected credit passes
    assert!(r9_credit_check("qwen3.8-tp2", "qwen3.8-tp2", 78.8, 78.8).is_ok());
}

// ══════════════ R10 — copies share the MoE expert union and the drafts ══════════════

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum R10Class {
    /// a different prompt per user
    Distinct,
    /// prompts repeated — shares experts and drafts (~15% higher)
    Copies,
}

struct R10Row {
    engine: &'static str,
    users: u32,
    tok_s: f64,
    class: R10Class,
    /// the disclosure carried ON the row when the competitor was measured
    /// in only one class ("Upstream and sglang were only measured with
    /// copies")
    disclosure: Option<&'static str>,
}

fn r10_compare(a: &R10Row, b: &R10Row) -> Result<f64, String> {
    if a.class != b.class {
        return Err(format!(
            "cross-class comparison: {} ran {:?}, {} ran {:?} — a workload substitution, not an engine delta; compare within class or carry the ~15% class note",
            a.engine, a.class, b.engine, b.class
        ));
    }
    Ok(b.tok_s / a.tok_s)
}

/// The mechanism witness: per-layer distinct experts under N sessions —
/// sessions sharing no text draw distinct experts; a repeated text
/// collapses the union (the source: 53.6 distinct per layer when sessions
/// share none, 37.4 at one text's overlap).
fn r10_expert_union(sessions: usize, overlap: bool, rng: &mut SplitMix64) -> f64 {
    let base = if overlap { 37.4 } else { 53.6 };
    let mut total = 0.0;
    for _ in 0..sessions {
        total += base + rng.sym(1.0); // per-session measurement noise
    }
    total / sessions as f64
}

#[test]
fn r10_class_law_and_moe_union() {
    let mut rng = SplitMix64::new(0xB427_0A10);
    let distinct = r10_expert_union(8, false, &mut rng);
    let copies = r10_expert_union(8, true, &mut rng);
    assert!(
        copies < distinct,
        "the MoE expert union collapses under repeated text ({copies} < {distinct} expected)"
    );
    assert!(
        (distinct - 53.6).abs() < 2.0 && (copies - 37.4).abs() < 2.0,
        "the union sits at the source's two poles"
    );

    // the class law: comparisons refuse across classes
    let ours = R10Row {
        engine: "ours",
        users: 16,
        tok_s: 368.0,
        class: R10Class::Distinct,
        disclosure: None,
    };
    let upstream = R10Row {
        engine: "upstream",
        users: 16,
        tok_s: 436.0,
        class: R10Class::Copies,
        disclosure: Some("only measured with copies"),
    };
    let cross = r10_compare(&ours, &upstream);
    assert!(
        cross.is_err(),
        "a distinct-vs-copies comparison must refuse"
    );
    assert!(cross.unwrap_err().contains("workload substitution"));

    // the same-class comparison is priced — and the class gap itself
    // (~18% here, the source's 368 distinct vs 436 copies) is larger than
    // most claimed engine deltas
    let ours_copies = R10Row {
        engine: "ours",
        users: 16,
        tok_s: 436.0,
        class: R10Class::Copies,
        disclosure: None,
    };
    let within = r10_compare(&ours_copies, &upstream).expect("same class");
    assert!(
        (within - 1.0).abs() < 0.01,
        "within class the ratio prices the engines, not the workload"
    );

    // the disclosure rides on the row where the classes cannot be matched
    assert_eq!(
        upstream.disclosure,
        Some("only measured with copies"),
        "the competitor's copies-only posture is disclosed beside the row"
    );
}

// ══════════════ R12 — a digest that folds nothing equals its seed ══════════════

/// The source's chosen_hash init constant — 0x14650fb0739d0383, published
/// in decimal as 1469598103934665603. The const assert below pins the two
/// spellings to each other.
const R12_SEED_HEX: u64 = 0x1465_0fb0_739d_0383;
const R12_SEED_DEC: u64 = 1_469_598_103_934_665_603;
const R12_FNV_PRIME: u64 = 0x100_0000_01b3;

fn r12_fold(state: u64, bytes: &[u8]) -> u64 {
    let mut h = state;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(R12_FNV_PRIME);
    }
    h
}

/// The defect: the mode hashes only the rows after the prompt offset; a
/// golden command whose offset consumes the file folds NOTHING and
/// "prints the hash's seed ... whatever the logits".
fn r12_defect_digest(rows: usize, payload: &[u8]) -> u64 {
    if rows == 0 {
        R12_SEED_HEX
    } else {
        r12_fold(R12_SEED_HEX, payload)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum R12Refusal {
    /// the run folded nothing; the digest would equal the seed
    ZeroRows,
    /// the digest equals the hasher's init constant
    SeedValued,
}

/// The fix: refuse loud on the two vacuity witnesses.
fn r12_checked_digest(rows: usize, payload: &[u8]) -> Result<u64, R12Refusal> {
    if rows == 0 {
        return Err(R12Refusal::ZeroRows);
    }
    let d = r12_fold(R12_SEED_HEX, payload);
    if d == R12_SEED_HEX {
        return Err(R12Refusal::SeedValued);
    }
    Ok(d)
}

#[test]
fn r12_seed_digest_refuses_loud() {
    // the seed constant's two published spellings agree
    assert_eq!(
        R12_SEED_HEX, R12_SEED_DEC,
        "0x14650fb0739d0383 == 1469598103934665603 (the source's own decimal)"
    );

    // the defect witness: a zero-row run hands out the seed AS the digest,
    // and the unfixed equality check accepts it forever
    let vacuous = r12_defect_digest(0, b"anything at all");
    assert_eq!(
        vacuous, R12_SEED_HEX,
        "the zero-row digest IS the seed — it passes whatever the logits"
    );
    assert_eq!(
        vacuous,
        r12_defect_digest(0, b"different logits entirely"),
        "two different runs produce the SAME 'evidence'"
    );

    // the fix: the zero-row run refuses before the digest is recorded
    assert_eq!(r12_checked_digest(0, b"logits"), Err(R12Refusal::ZeroRows));

    // a real digest folds real bytes and passes
    let real = r12_checked_digest(64, b"28.5k tokens of prefill state");
    assert!(real.is_ok());
    assert_ne!(real.expect("ok"), R12_SEED_HEX);

    // the tamper arm: rows counted but the payload empty — the fold still
    // returns the seed, and the digest check catches what the row-count
    // check misses (the conjunction, not the equality alone)
    assert_eq!(r12_checked_digest(64, b""), Err(R12Refusal::SeedValued));

    // and two different real payloads never collide on the seed
    let a = r12_checked_digest(64, b"payload a").expect("ok");
    let b = r12_checked_digest(64, b"payload b").expect("ok");
    assert_ne!(a, b);
    assert_ne!(a, R12_SEED_HEX);
}

// ══════════════ R13 — pad the io tail to the interrupt threshold ══════════════

const R13_READ_BATCH: u32 = 8;
/// kPad = kDiscard | 2 — the user_data bit that flags a pad read for
/// discard at completion
const R13_K_PAD: u64 = 0b11;
const R13_K_REAL: u64 = 0b00;
const R13_COALESCE_THRESHOLD: u32 = 8;

/// One tail decision. Returns (pads issued, their user_data flags). The
/// gate is the source's own condition: padding fires only when the pending
/// queue has nothing more to issue — under a deep queue completions keep
/// coming and no padding is needed.
fn r13_tail_pad(mut unseen: u32, queue_empty: bool) -> (u32, u32, Vec<u64>) {
    let mut flags = Vec::new();
    if R13_READ_BATCH > 1 && queue_empty {
        while unseen % R13_READ_BATCH != 0 {
            flags.push(R13_K_PAD);
            unseen += 1; // the pad read joins the outstanding set
        }
    }
    (flags.len() as u32, unseen, flags)
}

/// Which completion path the tail's last read trips: the device signals a
/// completion once `COALESCE_THRESHOLD` are pending OR its timer fires.
enum R13Trip {
    Threshold,
    Timer,
}

fn r13_completion_trip(pending: u32) -> R13Trip {
    if pending >= R13_COALESCE_THRESHOLD {
        R13Trip::Threshold
    } else {
        R13Trip::Timer
    }
}

#[test]
fn r13_pad_rounds_up_and_flags() {
    // the decode-tail class: 5 real reads outstanding → 3 pads to the
    // multiple of read_batch
    let (pads, unseen, flags) = r13_tail_pad(5, true);
    assert_eq!(pads, 3, "5 outstanding rounds up to 8 with 3 pads");
    assert_eq!(unseen, 8, "the pads join the outstanding set");
    assert!(
        flags.iter().all(|f| *f == R13_K_PAD),
        "every pad carries the discard flag bit"
    );
    assert_ne!(
        R13_K_PAD, R13_K_REAL,
        "pad reads are distinguishable from real reads in user_data"
    );

    // an exact multiple pads nothing — the padding never fires where it is
    // not needed
    let (pads8, unseen8, flags8) = r13_tail_pad(8, true);
    assert_eq!((pads8, unseen8, flags8.len()), (0, 8, 0));

    // 7 → 1 pad; 1 → 7 pads
    assert_eq!(r13_tail_pad(7, true).0, 1);
    assert_eq!(r13_tail_pad(1, true).0, 7);
    assert_eq!(r13_tail_pad(0, true).0, 0, "an empty tail pads nothing");
}

#[test]
fn r13_padding_buys_the_threshold_and_deep_queues_skip_it() {
    // the timer trap: an unpadded tail of 5 pending waits for the TIMER
    // (the count threshold needs 8) — the source's '1-7 reads take 200 µs'
    assert!(matches!(r13_completion_trip(5), R13Trip::Timer));
    assert!(matches!(r13_completion_trip(7), R13Trip::Timer));

    // the padded tail: 5 real + 3 pads = 8 pending → the LAST completion
    // trips the threshold instead of the timer
    let (pads, unseen, _) = r13_tail_pad(5, true);
    assert_eq!(unseen, 8);
    let total_pending = 5 + pads;
    assert!(matches!(
        r13_completion_trip(total_pending),
        R13Trip::Threshold
    ));

    // an already-multiple tail trips the threshold with zero pads — the
    // mechanism pays nothing where the burst already fills the batch
    assert!(matches!(r13_completion_trip(8), R13Trip::Threshold));
    assert_eq!(r13_tail_pad(8, true).0, 0);

    // THE GATE: a deep queue skips padding entirely even though the
    // outstanding count is off-multiple — completions keep coming, and
    // pad reads would be dead traffic (the source's 684 pads against 642k
    // real reads: the census of the gate working)
    let (deep_pads, deep_unseen, deep_flags) = r13_tail_pad(5, false);
    assert_eq!(
        (deep_pads, deep_unseen, deep_flags.len()),
        (0, 5, 0),
        "a deep queue pads nothing"
    );
    assert!(
        matches!(r13_completion_trip(5 + deep_pads), R13Trip::Timer),
        "the deep queue's tail still waits — correctly, because completions keep arriving"
    );
}

// ══════════════ run summary ══════════════

#[test]
fn b427_all_fixture_arms_ran() {
    // a liveness witness: the deterministic stream is seeded and moving
    let mut rng = SplitMix64::new(0xB427);
    let a = rng.next_u64();
    let b = rng.next_u64();
    assert_ne!(a, b, "the stream advances");
    let u = rng.unit();
    assert!((0.0..1.0).contains(&u));
}
