//! Batch 374 rust_perf fixture preflight — the laya 0.5.0-surface re-distill
//! (Plan 390 / Research 425): `NandhaKishorM/laya`
//! @ `416dd9ed20da2d43fc40e23712e9adceefabf634` (Apache-2.0; the pin is 2
//! commits past the v0.5.0 tag — the research note's "0.4.35" label was the
//! stale spelling, corrected in the T0 landing record). ORIGINAL plain-Rust
//! reproductions — zero repo source pasted (the MECHANISMS are re-authored
//! with our own constants and model harnesses; the source quotes stay in
//! the corpus context_notes). Compiled AND executed BEFORE entry (the
//! b363..b372 pre-entry discipline):
//! `rustc --edition 2021 --test b374_fixtures.rs && ./b374_fixtures`
//!
//! Arms per rule (buggy / fixed / anti-regime / tamper):
//! - F1 `batch-mixed-length-pads-to-longest-group-by-length` (BENCHABLE —
//!   the GOAT lead): a model batched pass pads every row of a pass to the
//!   pass's longest row and folds REAL padded work (fill + fold over
//!   [0..pass_max]); the ungated arm cuts chunks in input order over a
//!   MIXED-length distribution, the grouped arm stable-sorts by row
//!   length within a BOUNDED 8-chunk lookahead and restores results to
//!   input order — per-row results BIT-IDENTICAL between arms (pad
//!   contributes zero to the fold), the padded-element census strictly
//!   smaller in the grouped arm, the UNIFORM-length control arm Ties
//!   (the anti-regime is part of the fixture), and the tampered arm
//!   (sort WITHOUT the order restore) reds on per-row results.
//! - F2 `confidence-threshold-bucketed-by-answer-shape` (+ the R3
//!   tri-state face): twelve bucket keys (3 qtypes × 4 option bands) +
//!   `default`; the per-shape arm hits its abstention target on BOTH
//!   shapes while the single flat cut misses them in OPPOSITE directions
//!   (the measured ordering flip is the premise); the gate result echoes
//!   the threshold THAT answer's bucket applied; an unknown qtype falls
//!   through to the default bucket; the face: with no gate configured an
//!   answer reports NOTHING at all (absent is an answer, never a pass)
//!   and a NaN confidence lands in its own `unevaluated` state — never a
//!   pass, never an abstain.
//! - F3 `long-doc-window-aggregation-picks-deciding-window`: a long
//!   document scored per window — `noul` takes the strongest window (any
//!   supporting window ⇒ true), `choice`/`score` take the MOST-CONFIDENT
//!   window; the averaging negative: the mean lets the neutral majority
//!   out-vote the one window that saw the deciding span and flips the
//!   answer; per-window attribution (index + span + support count)
//!   survives the pick.
//! - F4 `budget-room-from-constructed-artifact-not-configured-cap`: a
//!   head grows past its configured cap once per-option token floors
//!   dominate (past ~cap/4 options) — a state sized from the cap-based
//!   figure (`max_len - head_max_len`) TRUNCATES; the fixed arm sizes
//!   from the head that was actually BUILT (per-option floors summed,
//!   clamped at the cap from below); the anti-regime: few options make
//!   the two figures agree and the cap-based size is merely conservative;
//!   the tampered arm (floor-0 assumption) under-builds the state and
//!   truncates exactly like the bug.

use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════
// F1 — batch-mixed-length-pads-to-longest-group-by-length
// ═══════════════════════════════════════════════════════════════════════

/// A model encoded row: its token VALUES (the work is real: the pass
/// fills a padded buffer of pass-max length and folds it).
struct Row {
    values: Vec<u64>,
}

/// One batched pass: pads every row to `pass_max` (zero fill), then folds
/// Σ v² over the padded width — the pad contributes 0, so the PER-ROW
/// result is independent of how wide the pass padded (the bit-identity
/// premise), while the WORK (elements touched) is rows × pass_max.
fn fold_padded(row: &[u64], pass_max: usize, padded_touched: &mut u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..pass_max {
        let v = if i < row.len() { row[i] } else { 0 };
        acc = acc.wrapping_add(v.wrapping_mul(v));
        *padded_touched += 1;
    }
    acc
}

const SORT_WINDOW_BATCHES: usize = 8;

/// The ungated arm: chunks in input order — each pass pads to ITS
/// longest row, so a short row co-chunked with a long one pays the long
/// one's width.
fn run_ungated(rows: &[Row], chunk: usize) -> (Vec<u64>, u64) {
    let mut results = vec![0u64; rows.len()];
    let mut padded_touched = 0u64;
    for (start, pass) in rows.chunks(chunk).enumerate() {
        let start = start * chunk;
        let pass_max = pass.iter().map(|r| r.values.len()).max().unwrap_or(0);
        for (i, r) in pass.iter().enumerate() {
            results[start + i] = fold_padded(&r.values, pass_max, &mut padded_touched);
        }
    }
    (results, padded_touched)
}

/// The grouped arm: within each window of `SORT_WINDOW_BATCHES` chunks,
/// stably sort rows by length, cut the sorted window into chunks, fold,
/// then write each result back to its ORIGINAL index. Order-preserving
/// by construction; per-pass pad width shrinks on mixed lengths.
fn run_grouped(rows: &[Row], chunk: usize) -> (Vec<u64>, u64) {
    let mut results = vec![0u64; rows.len()];
    let mut padded_touched = 0u64;
    let window = chunk * SORT_WINDOW_BATCHES;
    let mut start = 0;
    while start < rows.len() {
        let end = (start + window).min(rows.len());
        let mut idx: Vec<usize> = (start..end).collect();
        idx.sort_by_key(|&i| rows[i].values.len()); // stable
        for group in idx.chunks(chunk) {
            let pass_max = group
                .iter()
                .map(|&i| rows[i].values.len())
                .max()
                .unwrap_or(0);
            for &i in group {
                results[i] = fold_padded(&rows[i].values, pass_max, &mut padded_touched);
            }
        }
        start = end;
    }
    (results, padded_touched)
}

/// Deterministic bimodal + long-tail mixed-length distribution (the
/// regime the rule serves): mostly short rows, a long tail.
fn mixed_rows(n: usize) -> Vec<Row> {
    let mut s = 0x243f_6a88_85a3_08d3u64;
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let len = if i % 16 == 0 {
            200 + (s >> 40) as usize % 200 // the long tail
        } else {
            4 + (s >> 45) as usize % 8 // the short mode
        };
        let mut values = Vec::with_capacity(len);
        for j in 0..len {
            values.push(((s >> j % 64) as u64) % 7 + 1); // non-zero: pad is visible
        }
        rows.push(Row { values });
    }
    rows
}

fn uniform_rows(n: usize, len: usize) -> Vec<Row> {
    (0..n)
        .map(|_| Row {
            values: vec![3u64; len],
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════
// F2 — confidence-threshold-bucketed-by-answer-shape (+ the R3 face)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum QType {
    Choice,
    Score,
    Noul,
}

/// The option band: the answer SHAPE axis the same confidence field
/// orders differently across.
fn option_band(k: usize) -> &'static str {
    match k {
        0..=2 => "1-2",
        3..=5 => "3-5",
        6..=10 => "6-10",
        _ => "11+",
    }
}

fn bucket_key(q: QType, k: usize) -> String {
    let q = match q {
        QType::Choice => "choice",
        QType::Score => "score",
        QType::Noul => "noul",
    };
    format!("{q}:{}", option_band(k))
}

#[derive(Debug, PartialEq)]
enum Outcome {
    /// the answer carried the gate: the confidence cleared THAT bucket's
    /// threshold (the applied threshold is echoed — the re-split axis)
    Passed { applied: f64 },
    /// the confidence did not clear the bucket's threshold
    Abstained { applied: f64 },
    /// a non-finite / unusable confidence: neither a pass nor an abstain
    /// (R3 face — its own state, never folded into either pole)
    Unevaluated,
}

/// The model answer: `None` confidence = no gate was configured for this
/// lane — the answer reports NOTHING at all (absent is an answer).
struct Answer {
    confidence: Option<f64>,
}

fn gate(answer: &Answer, q: QType, k: usize, thresholds: &HashMap<String, f64>) -> Option<Outcome> {
    let Some(min_confidence) = thresholds.values().next().copied() else {
        // the deliberately-absent arm: no thresholds table at all — the
        // gate reports nothing, it never fabricates a pass
        return None;
    };
    let _ = min_confidence;
    let Some(c) = answer.confidence else {
        return None; // no gate configured: report nothing
    };
    if !c.is_finite() {
        return Some(Outcome::Unevaluated); // NaN/inf: its own state
    }
    let t = thresholds
        .get(&bucket_key(q, k))
        .or_else(|| thresholds.get("default"))
        .copied()
        .unwrap_or(0.0);
    if c >= t {
        Some(Outcome::Passed { applied: t })
    } else {
        Some(Outcome::Abstained { applied: t })
    }
}

// ═══════════════════════════════════════════════════════════════════════
// F3 — long-doc-window-aggregation-picks-deciding-window
// ═══════════════════════════════════════════════════════════════════════

struct WindowScore {
    index: usize,
    /// p(the window supports the statement) for noul / p(top option) for
    /// choice
    confidence: f64,
    /// the window's own yes/no read (noul) or its top option id (choice)
    supports: bool,
    span: (usize, usize),
}

/// The buggy arm: AVERAGE the windows. A long mostly-neutral document
/// lets the neutral majority out-vote the one window that saw the
/// deciding span.
fn aggregate_mean(windows: &[WindowScore]) -> (bool, f64) {
    let n = windows.len() as f64;
    let mean_conf = windows.iter().map(|w| w.confidence).sum::<f64>() / n;
    let mean_support = windows.iter().filter(|w| w.supports).count() as f64 / n;
    (mean_support > 0.5, mean_conf)
}

/// The fixed arm: pick the DECIDING window. noul: the strongest window
/// (any supporting window ⇒ true — the max-confidence read decides).
/// choice/score: the most-confident window. Attribution rides with it.
fn aggregate_deciding(windows: &[WindowScore]) -> (bool, f64, usize) {
    let best = windows
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.confidence.total_cmp(&b.1.confidence))
        .map(|(i, w)| (i, w))
        .expect("at least one window");
    (best.1.supports, best.1.confidence, best.1.index)
}

// ═══════════════════════════════════════════════════════════════════════
// F4 — budget-room-from-constructed-artifact-not-configured-cap
// ═══════════════════════════════════════════════════════════════════════

const MAX_LEN: usize = 512;
const HEAD_MAX_LEN_CAP: usize = 320;
const OPTION_TOKEN_FLOOR: usize = 4;

/// The naive arm: the state room from the CONFIGURED cap —
/// `max_len - head_max_len` — a figure the doc must fit the head into.
fn state_room_cap_based() -> usize {
    MAX_LEN - HEAD_MAX_LEN_CAP // 192 — "the safe figure in neither direction"
}

/// The constructed head: each option carries a token FLOOR, so past
/// ~cap/4 options the built head grows BEYOND the cap (77 options × 4 =
/// 308 + instruction/state markers → past 320 at the tail shapes).
fn constructed_head_len(options: usize, instruction_tokens: usize) -> usize {
    let mut head = 4 + instruction_tokens; // [CLS] type + question markers + instruction
    head += options * OPTION_TOKEN_FLOOR; // [MASK] + option text floor each
    head += 2; // trailing [SEP]s
    head
}

/// The fixed arm: the state room from what was BUILT —
/// `max_len - constructed_head` (never negative).
fn state_room_constructed(options: usize, instruction_tokens: usize) -> usize {
    MAX_LEN.saturating_sub(constructed_head_len(options, instruction_tokens))
}

/// The buggy arm's fate: the state truncated to the cap-based room even
/// though the built head left LESS room than that.
fn state_tokens_fitted(state_len: usize, room: usize) -> usize {
    state_len.min(room)
}

// ═══════════════════════════════════════════════════════════════════════
// fixtures
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn f1_grouped_results_bit_identical_and_pad_census_shrinks() {
    let rows = mixed_rows(4_000);
    let chunk = 32;

    let (ungated, ungated_padded) = run_ungated(&rows, chunk);
    let (grouped, grouped_padded) = run_grouped(&rows, chunk);

    // G1: per-row results are BIT-IDENTICAL across arms — the sort is
    // reorder-only, the pad contributes zero.
    assert_eq!(ungated, grouped, "per-row results must not move");

    // the work axis: strictly less padded work on the mixed distribution
    assert!(
        grouped_padded < ungated_padded,
        "grouped padded work {} must be below ungated {}",
        grouped_padded,
        ungated_padded
    );
    // and the cut is material on the bimodal+tail shape (≥ 25% here;
    // the GOAT cell prices the wall separately at the registered sizes)
    let cut = 1.0 - (grouped_padded as f64 / ungated_padded as f64);
    assert!(cut > 0.25, "expected a material pad cut, got {:.3}", cut);
}

#[test]
fn f1_uniform_control_ties() {
    // the anti-regime: uniform lengths have nothing to reorder — the
    // grouped arm must NOT claim a cut on them (the census Ties).
    let rows = uniform_rows(2_000, 64);
    let chunk = 32;
    let (_, a) = run_ungated(&rows, chunk);
    let (_, b) = run_grouped(&rows, chunk);
    assert_eq!(a, b, "uniform-length work must Tie (the anti-regime)");
}

#[test]
fn f1_tampered_sort_without_restore_reds() {
    // the tamper arm: sorting WITHOUT writing results back to their
    // original indices produces DIFFERENT per-row answers — the pin that
    // the order restore is load-bearing, not cosmetic.
    let rows = mixed_rows(512);
    let chunk = 32;
    let (expected, _) = run_ungated(&rows, chunk);

    let mut results = vec![0u64; rows.len()];
    let mut padded = 0u64;
    let window = chunk * SORT_WINDOW_BATCHES;
    let mut start = 0;
    while start < rows.len() {
        let end = (start + window).min(rows.len());
        let mut idx: Vec<usize> = (start..end).collect();
        idx.sort_by_key(|&i| rows[i].values.len()); // stable
        for group in idx.chunks(chunk) {
            let pass_max = group
                .iter()
                .map(|&i| rows[i].values.len())
                .max()
                .unwrap_or(0);
            // BUG: results written in SORTED position, not original index
            for (slot, &i) in group.iter().enumerate() {
                results[start + slot] = fold_padded(&rows[i].values, pass_max, &mut padded);
            }
        }
        start = end;
    }
    assert_ne!(
        results, expected,
        "the tampered arm (no restore) must red — the sort moved rows"
    );
}

#[test]
fn f2_per_shape_buckets_hit_their_targets_flat_cut_misses_in_opposite_directions() {
    // the measured premise, modeled: the SAME confidence field orders
    // OPPOSITE between two shapes. Shape A (choice, 3 options) skews
    // LOW-mean; shape B (choice, 20 options) skews HIGH-mean.
    let shape_a: Vec<f64> = (0..200)
        .map(|i| 0.50 + (i % 20) as f64 / 100.0)
        .collect(); // uniform 0.50..0.69, mean 0.60
    let shape_b: Vec<f64> = (0..200)
        .map(|i| 0.85 + (i % 20) as f64 / 200.0)
        .collect(); // uniform 0.85..0.94, mean 0.90

    // per-shape buckets: each shape's threshold tuned to ITS
    // distribution (the bucket table's whole point).
    let mut buckets = HashMap::new();
    buckets.insert("choice:3-5".to_string(), 0.60f64);
    buckets.insert("choice:11+".to_string(), 0.90f64);
    buckets.insert("default".to_string(), 0.8f64);

    let abstain = |confs: &[f64], q, k, t: &HashMap<String, f64>| -> (usize, Vec<Outcome>) {
        let mut n = 0;
        let outs: Vec<Outcome> = confs
            .iter()
            .filter_map(|c| gate(&Answer { confidence: Some(*c) }, q, k, t))
            .map(|o| {
                if matches!(o, Outcome::Abstained { .. }) {
                    n += 1;
                }
                o
            })
            .collect();
        (n, outs)
    };

    let (a_abst, a_outs) = abstain(&shape_a, QType::Choice, 3, &buckets);
    let (b_abst, b_outs) = abstain(&shape_b, QType::Choice, 20, &buckets);
    // each shape lands NEAR its own target (here: ~half abstain on the
    // low-mean shape, ~half on the high-mean shape against its own cut)
    let a_rate = a_abst as f64 / shape_a.len() as f64;
    let b_rate = b_abst as f64 / shape_b.len() as f64;
    assert!(
        (0.3..0.7).contains(&a_rate) && (0.3..0.7).contains(&b_rate),
        "per-shape buckets must land both shapes near target: {a_rate} {b_rate}"
    );
    // the echo: the applied threshold is the ANSWER'S bucket's, and the
    // two shapes clearly saw DIFFERENT applied values
    let echo_of = |outs: &[Outcome]| match outs.first() {
        Some(Outcome::Abstained { applied }) | Some(Outcome::Passed { applied }) => *applied,
        _ => panic!("expected an abstain to read the echo from"),
    };
    let ta = echo_of(&a_outs);
    let tb = echo_of(&b_outs);
    assert_ne!(ta, tb, "the echo must name each answer's own bucket");

    // the flat cut: ONE threshold for both shapes cannot hit both —
    // tuned for A it passes nearly all of B (abstains ~0); tuned for B
    // it abstains nearly all of A. The ordering-flip premise.
    let mut flat_for_a = HashMap::new();
    flat_for_a.insert("default".to_string(), 0.60f64);
    let (flat_b_abst_under_a, _) = abstain(&shape_b, QType::Choice, 20, &flat_for_a);
    assert!(
        flat_b_abst_under_a as f64 / (shape_b.len() as f64) < 0.05,
        "a flat cut tuned for the low-mean shape must over-pass the high-mean shape"
    );
    let mut flat_for_b = HashMap::new();
    flat_for_b.insert("default".to_string(), 0.90f64);
    let (flat_a_abst_under_b, _) = abstain(&shape_a, QType::Choice, 3, &flat_for_b);
    assert!(
        flat_a_abst_under_b as f64 / (shape_a.len() as f64) > 0.95,
        "the same flat cut tuned for the high-mean shape must over-abstain the low-mean shape"
    );
}

#[test]
fn f2_unknown_qtype_falls_through_to_default() {
    let mut buckets = HashMap::new();
    buckets.insert("default".to_string(), 0.9f64);
    // score with an odd option count whose exact bucket key is absent:
    // the default fall-through keeps the gate ANSWERING (never refusing)
    let out = gate(
        &Answer {
            confidence: Some(0.95),
        },
        QType::Score,
        7,
        &buckets,
    );
    assert_eq!(
        out,
        Some(Outcome::Passed {
            applied: 0.9,
        }),
        "the default bucket answers when the exact key is absent"
    );
}

#[test]
fn f3_absent_gate_reports_nothing_and_nan_is_unevaluated() {
    // R3 face: the deliberately-absent arm — no thresholds table at all.
    let empty = HashMap::new();
    let out = gate(
        &Answer {
            confidence: Some(0.99),
        },
        QType::Noul,
        2,
        &empty,
    );
    assert_eq!(out, None, "with no gate configured the answer reports NOTHING at all");

    // the NaN arm: a non-finite confidence is neither a pass nor an
    // abstain — reporting it as either would be equally wrong.
    let mut buckets = HashMap::new();
    buckets.insert("default".to_string(), 0.5f64);
    let out = gate(
        &Answer {
            confidence: Some(f64::NAN),
        },
        QType::Noul,
        2,
        &buckets,
    );
    assert_eq!(out, Some(Outcome::Unevaluated));
}

#[test]
fn f3_deciding_window_beats_the_average_on_a_neutral_majority() {
    // a long mostly-neutral document: 19 neutral windows + ONE window
    // that saw the deciding span.
    let mut windows: Vec<WindowScore> = (0..19)
        .map(|i| WindowScore {
            index: i,
            confidence: 0.55,
            supports: false,
            span: (i * 100, i * 100 + 90),
        })
        .collect();
    windows.push(WindowScore {
        index: 19,
        confidence: 0.98,
        supports: true, // the deciding span lives here
        span: (1900, 1999),
    });

    // the averaging negative: the neutral majority out-votes the
    // deciding window — the answer FLIPS.
    let (mean_supports, mean_conf) = aggregate_mean(&windows);
    assert!(
        !mean_supports,
        "the mean must lose the deciding window (19/20 neutral)"
    );
    assert!(mean_conf < 0.6);

    // the fixed arm: noul takes the strongest window — any supporting
    // window ⇒ true, and the attribution rides with the pick.
    let (supports, conf, picked) = aggregate_deciding(&windows);
    assert!(supports, "the strongest window carries the answer");
    assert!((conf - 0.98).abs() < 1e-12);
    assert_eq!(picked, 19, "the picked window's index IS the attribution");
    // the span attribution names where the deciding span lives
    let w = &windows[windows.len() - 1];
    assert_eq!(w.span, (1900, 1999));
}

#[test]
fn f3_most_confident_window_picked_for_choice_not_first() {
    // choice: the MOST-CONFIDENT window decides, never the first —
    // windows are unordered evidence, not a sequence to short-circuit.
    let windows = vec![
        WindowScore { index: 0, confidence: 0.51, supports: true, span: (0, 90) },
        WindowScore { index: 1, confidence: 0.49, supports: false, span: (90, 180) },
        WindowScore { index: 2, confidence: 0.83, supports: false, span: (180, 270) },
    ];
    let (_, conf, picked) = aggregate_deciding(&windows);
    assert_eq!(picked, 2);
    assert!((conf - 0.83).abs() < 1e-12);
}

#[test]
fn f4_cap_based_room_truncates_constructed_room_does_not() {
    // 90 options: the built head is 4 + 8 + 90×4 + 2 = 374 — PAST the
    // 320 cap. The cap-based state room (192) is WIDER than the real
    // room (512-374 = 138): a state sized to the cap-based figure
    // truncates 54 tokens of state.
    let options = 90;
    let instruction = 8;
    let head = constructed_head_len(options, instruction);
    assert!(head > HEAD_MAX_LEN_CAP, "the fixture's shape must exceed the cap");
    let cap_room = state_room_cap_based();
    let built_room = state_room_constructed(options, instruction);
    assert!(built_room < cap_room, "the built head must leave LESS room than the cap claims");

    let state_len = cap_room; // sized from the configured cap — the bug
    let fitted = state_tokens_fitted(state_len, built_room);
    assert!(
        fitted < state_len,
        "the cap-sized state must truncate against the built head ({} < {})",
        fitted,
        state_len
    );

    // the fixed arm: size the state from the CONSTRUCTED room — nothing
    // truncates.
    let state_fixed = built_room;
    let fitted_fixed = state_tokens_fitted(state_fixed, built_room);
    assert_eq!(fitted_fixed, state_fixed, "the constructed-room-sized state never truncates");
}

#[test]
fn f4_few_options_make_the_two_figures_agree() {
    // the anti-regime: 8 options build a head WELL below the cap — the
    // cap-based figure is merely conservative, nothing truncates.
    let options = 8;
    let instruction = 8;
    let head = constructed_head_len(options, instruction);
    assert!(head <= HEAD_MAX_LEN_CAP);
    let cap_room = state_room_cap_based();
    let built_room = state_room_constructed(options, instruction);
    assert!(built_room >= cap_room);
    let fitted = state_tokens_fitted(cap_room, built_room);
    assert_eq!(fitted, cap_room, "few options: the cap-based size is safe (conservative)");
}

#[test]
fn f4_tampered_floor_zero_understates_the_head_and_truncates() {
    // the tamper: assume a 0-token option floor (options are "cheap") —
    // the built-head estimate collapses to the cap and the fixture
    // proves the estimate is what truncates, not the cap itself.
    let options = 90;
    let instruction = 8;
    let true_head = constructed_head_len(options, instruction);
    let true_room = MAX_LEN - true_head;
    let assumed_head = 4 + instruction + 2; // floors dropped: options "free"
    let assumed_room = MAX_LEN - assumed_head;
    // the state sized on the optimistic estimate truncates against the
    // true built head — exactly the bug the rule names, via a wrong
    // CONSTRUCTION assumption instead of a wrong cap read.
    let fitted = state_tokens_fitted(assumed_room, true_room);
    assert!(
        fitted < assumed_room,
        "the floor-0 estimate must truncate against the true built head"
    );
}
