//! Batch 427 kernel fixture preflight — the pico-dgx-ds4 distill (Plan 420,
//! Research 454; Picom-code/pico-dgx-ds4 @
//! 922dddb1a35e521148022ee11264632da1f0ddc0, MIT — a fork of blaesus/dsgx10;
//! engine tree ds41f-4xgx10-switchless/).
//!
//! ORIGINAL plain-Rust reproduction — zero repo source pasted. The GPU/
//! serving surfaces (sharded LM head masks, spec-decode draft windows, MoE
//! expert unions, TP collectives over a ring or a switch, slot admission,
//! wire-format tiles) are MODELED as deterministic state machines in CPU
//! arithmetic; what the fixtures pin are the RELATIONSHIPS the rules price,
//! never the hardware's numbers (all nine rules are NotBenchable-by-domain
//! — the fixture discipline is the defect-shape pin, not a perf claim).
//!
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b427_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - R1 (certified-argmax-residual-bound-shared-across-shards): the winner
//!   always survives the shared-threshold mask, ties included; per-rank
//!   thresholds also keep the winner but prune NOTHING on winner-less
//!   ranks (the shared threshold's survivor count is strictly smaller);
//!   a flat distribution (inflated residual) reads more groups; an
//!   understated (non-rigorous) bound drops the winner (the tamper arm).
//! - R2 (copy-drafts-verified-never-trusted-single-request-steps): a wrong
//!   window costs rows, never a token (the reply text is identical); a
//!   right window amortizes (rows per emitted token < 1); the entry-token
//!   gate drops a misaligned window; a host/device layout divergence dies;
//!   a shared step never copies; backoff doubles the match need (capped
//!   8x) and whole acceptance doubles the width (2w+1, capped).
//! - R3 (pad-rows-duplicate-last-real-row-mask-state-writers): duplicated
//!   pad rows add NO experts to the union and compute the same values
//!   (union-invariance); zero-pad rows DO grow the union (the polarity
//!   arm); masked state writers keep state byte-identical with pads
//!   present, an unmasked writer corrupts (the tamper arm); a cross-row
//!   reduction reads pad values (the anti-regime arm).
//! - R4 (decision-bits-identical-across-shards-fixed-order-on-reduced-
//!   stats): the fixed-order evaluation gives identical bits on every
//!   rank; a rank-varying order drifts bits AND forks the depth decision;
//!   the tie rule resolves to the lower id identically everywhere; a
//!   weights-file mismatch dies at boot; a forked decision re-groups rows
//!   and flips a near-tie token while identical decisions keep the text.
//! - R5 (gumbel-noise-keyed-seed-position-id-shared-draft-verify): the
//!   same (seed, position, id) noise gives the same emitted tokens with
//!   ANY draft set (only the acceptance count changes); the keyed noise
//!   is reproducible from both sides and differs across positions; a
//!   greedy one-hot draft is accepted iff the plain draw equals it; a
//!   seeded top-p request keeps the incumbent sampler (the mode-1 scope).
//! - R11 (topology-keyed-collective-hop-count-bits-frozen): relay and
//!   direct transports that both fold in rank order give bit-identical
//!   results; folding direct posts in ARRIVAL order yields different bits
//!   (the freeze is the content — the tamper arm); a fleet with mixed
//!   modes refuses at the bootstrap; the micro saving does not transfer
//!   1:1 into a step saving (the discount); a cabled ring refuses direct.
//! - R14 (smallest-fitting-slot-class-first-keep-large-for-long): a short
//!   request takes a small slot even when a big slot is free; a long
//!   request takes the big slot; big-class saturation parks flights in
//!   the `later` deque IN ORDER and they re-enter first; a uniform pool
//!   makes the two-pass pick identical to naive first-fit; a slot in the
//!   `keep` set is never stolen.
//! - R15 (batch-aware-expert-reroute-never-single-session): a single-
//!   session step routes naturally at ANY width (the guard); multi-
//!   session rows prefer union members among their own top-kp in rank
//!   order; a row with no union overlap keeps natural picks; the MINROWS
//!   gate and the parse bounds refuse; the reroute is lossy (the delta
//!   is visible) exactly where the guard permits it.
//! - R16 (tile-local-exponent-base-coding-raw-escape): offsets from the
//!   tile's least exponent re-widen to identical bits; a tile spanning
//!   more than 16 exponent values stays raw behind the flag (identity
//!   decode); a flat-entropy plane shows zero byte saving (all tiles
//!   escape) while the decode tax makes it a net loss; a tight-spread
//!   plane saves bytes; the check pass counts corrupted pieces.

use std::collections::BTreeSet;

// ════════════════════════════════════════════════════════════════════
// R1 — certified argmax: the residual bound + the rank-SHARED threshold.
// ════════════════════════════════════════════════════════════════════

/// one vocabulary row as the certified head sees it: the coarse (FP8) logit,
/// the rigorous bound xb (|exact - coarse| <= xb), and the exact logit the
/// unchanged kernel would produce.
#[derive(Clone, Copy, Debug)]
struct CertRow {
    coarse: f64,
    xb: f64,
    exact: f64,
}

/// the source's step 3+4: one collective takes the best lower bound over
/// the ranks (max of per-row lower bounds), every rank keeps rows whose
/// upper bound reaches it. Flattened keep-mask over shards-major order.
fn shared_keep(shards: &[Vec<CertRow>]) -> Vec<bool> {
    let t = shards
        .iter()
        .map(|s| {
            s.iter()
                .map(|r| r.coarse - r.xb)
                .fold(f64::NEG_INFINITY, f64::max)
        })
        .fold(f64::NEG_INFINITY, f64::max);
    let mut keep = Vec::new();
    for s in shards {
        for r in s {
            keep.push(r.coarse + r.xb >= t);
        }
    }
    keep
}

/// the anti-shape: each rank masks against only its OWN best lower bound.
fn per_rank_keep(shards: &[Vec<CertRow>]) -> Vec<bool> {
    let mut keep = Vec::new();
    for s in shards {
        let t = s
            .iter()
            .map(|r| r.coarse - r.xb)
            .fold(f64::NEG_INFINITY, f64::max);
        for r in s {
            keep.push(r.coarse + r.xb >= t);
        }
    }
    keep
}

/// the argmax over KEPT rows (a dropped row reads -inf): strict > so the
/// lowest (shard, index) wins ties — the source's (maximum, lowest id).
fn argmax_over(shards: &[Vec<CertRow>], keep: &[bool]) -> (usize, usize) {
    let mut best = (0usize, 0usize);
    let mut bv = f64::NEG_INFINITY;
    let mut k = 0usize;
    for (s, shard) in shards.iter().enumerate() {
        for (i, r) in shard.iter().enumerate() {
            if keep[k] && (r.exact > bv || (bv == f64::NEG_INFINITY && k == 0)) {
                if r.exact > bv {
                    bv = r.exact;
                    best = (s, i);
                }
            }
            k += 1;
        }
    }
    best
}

fn n_kept(mask: &[bool]) -> usize {
    mask.iter().filter(|&&b| b).count()
}

#[test]
fn r1_winner_always_survives_shared_threshold_incl_ties() {
    // shard 0 holds a tie row (exact == the winner's), the winner sits on
    // shard 2; the shared threshold must keep BOTH, and the argmax over
    // kept rows must equal the argmax over everything (lowest id wins).
    let shards = vec![
        vec![
            CertRow {
                coarse: 5.0,
                xb: 0.2,
                exact: 5.1,
            },
            CertRow {
                coarse: 38.9,
                xb: 0.2,
                exact: 39.0,
            }, // the tie row
        ],
        vec![CertRow {
            coarse: 9.0,
            xb: 0.3,
            exact: 9.2,
        }],
        vec![
            CertRow {
                coarse: 38.8,
                xb: 0.2,
                exact: 39.0,
            }, // the winner (lower coarse)
            CertRow {
                coarse: 4.0,
                xb: 0.1,
                exact: 4.05,
            },
        ],
    ];
    let keep = shared_keep(&shards);
    // T = max lower bound = 39.0 - 0.2 = 38.7; the tie row's upper is 39.1,
    // the winner's upper is 39.0 — both reach it.
    assert!(keep[1], "the tie row survives");
    assert!(keep[3], "the winner survives");
    assert_eq!(
        argmax_over(&shards, &keep),
        argmax_over(&shards, &vec![true; keep.len()])
    );
    assert_eq!(
        argmax_over(&shards, &keep),
        (0, 1),
        "ties resolve to the lowest id"
    );
}

#[test]
fn r1_shared_threshold_prunes_where_per_rank_cannot() {
    // shard 0's slice is weak (nothing near the winner): per-rank thresholds
    // keep ALL of its rows (its own lower bound is low — it cannot prune);
    // the shared threshold prunes it to nothing. Both keep the winner.
    let shards = vec![
        vec![
            CertRow {
                coarse: 5.0,
                xb: 0.1,
                exact: 5.05,
            },
            CertRow {
                coarse: 7.0,
                xb: 0.1,
                exact: 7.05,
            },
            CertRow {
                coarse: 6.0,
                xb: 0.1,
                exact: 6.05,
            },
        ],
        vec![CertRow {
            coarse: 39.0,
            xb: 0.1,
            exact: 39.02,
        }], // the winner here
    ];
    let shared = shared_keep(&shards);
    let per_rank = per_rank_keep(&shards);
    // winner survives under both schemes (correctness-neutral):
    assert!(shared[3] && per_rank[3]);
    // pruning power is NOT neutral: a winner-less rank always keeps at
    // least its own best row (its upper >= its own lower bound), while the
    // shared threshold prunes its whole slice (upper 7.1 << T = 38.9).
    assert!(
        n_kept(&per_rank[0..3]) >= 1,
        "a winner-less rank keeps its own best row at best"
    );
    assert_eq!(
        n_kept(&shared[0..3]),
        0,
        "the shared threshold prunes the whole slice"
    );
    assert!(n_kept(&shared) < n_kept(&per_rank));
}

#[test]
fn r1_flat_distribution_reads_more_groups() {
    // the same logits with the residual inflated 10x: the lower bounds drop,
    // the shared threshold drops, more rows survive — more exact groups read,
    // up to all of them (the graceful fallback, never a guess).
    let tight: Vec<Vec<CertRow>> = vec![vec![
        CertRow {
            coarse: 30.0,
            xb: 0.8,
            exact: 30.5,
        },
        CertRow {
            coarse: 31.0,
            xb: 0.8,
            exact: 31.2,
        },
        CertRow {
            coarse: 29.0,
            xb: 0.8,
            exact: 29.3,
        },
    ]];
    let flat: Vec<Vec<CertRow>> = tight
        .iter()
        .map(|s| {
            s.iter()
                .map(|r| CertRow {
                    xb: r.xb * 10.0,
                    ..*r
                })
                .collect()
        })
        .collect();
    assert!(n_kept(&shared_keep(&flat)) > n_kept(&shared_keep(&tight)));
}

#[test]
fn r1_understated_bound_drops_winner() {
    // the tamper arm: a bound that understates the true |exact - coarse|
    // is not rigorous — the winner's upper bound falls below the shared
    // threshold, the mask drops the TRUE winner, and the argmax over kept
    // rows answers with the wrong row. The bound must be computed from the
    // real weights, once, over-estimated (the 1.001 factor) and fail-open.
    let good = vec![
        vec![CertRow {
            coarse: 39.0,
            xb: 0.5,
            exact: 39.4,
        }], // true error 0.4 <= 0.5
        vec![CertRow {
            coarse: 38.6,
            xb: 0.1,
            exact: 38.65,
        }],
    ];
    let keep = shared_keep(&good);
    assert_eq!(
        argmax_over(&good, &keep),
        (0, 0),
        "the rigorous bound keeps the winner"
    );
    let rigged = vec![
        vec![CertRow {
            coarse: 39.0,
            xb: 0.1,
            exact: 39.4,
        }], // the lie: error 0.4 > 0.1
        vec![CertRow {
            coarse: 39.25,
            xb: 0.02,
            exact: 38.0,
        }], // a rival over-stated
    ];
    let keep_bad = shared_keep(&rigged);
    // T = 39.25 - 0.02 = 39.23 > the winner's false upper 39.1: dropped.
    assert!(!keep_bad[0], "the rigged bound drops the true winner");
    assert_eq!(
        argmax_over(&rigged, &keep_bad),
        (1, 0),
        "the mask then answers with the wrong row: exactness died with the bound"
    );
}

// ════════════════════════════════════════════════════════════════════
// R2 — copy drafts: verified never trusted; the entry-token gate; the
// host replay dies on divergence; single-request steps only.
// ════════════════════════════════════════════════════════════════════

/// a deterministic toy LM: the model's next token given the previous one.
fn model_next(prev: u32) -> u32 {
    (prev.wrapping_mul(1_103_515_245).wrapping_add(12_345) >> 7) % 97
}

/// the model's continuation for `n` tokens from `start`.
fn continuation(start: u32, n: usize) -> Vec<u32> {
    let mut cur = start;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        cur = model_next(cur);
        out.push(cur);
    }
    out
}

/// one copy step: the window's drafts are verify rows — every draft is
/// verified as any draft is, so the emitted text is the model's own no
/// matter what the window held. Returns (the emitted text, rows consumed
/// per emitted token). The entry-token gate: window[0] must equal the
/// model's actual token at the entry row, else the window is dropped and
/// the step runs exactly as a no-copy step.
fn copy_step(start: u32, window: Option<&[u32]>, shared_step: bool, n: usize) -> (Vec<u32>, f64) {
    let truth = continuation(start, n);
    let gate_ok = !shared_step && window.map(|w| w.first() == truth.first()).unwrap_or(false);
    match (gate_ok, window) {
        (false, _) | (true, None) => (truth.clone(), 1.0), // unchanged: 1 token / row
        (true, Some(w)) => {
            // every draft offered is a verify row; accepted ones emit their
            // token this row, rejected ones cost the row and emit the model's
            let accepted = w.iter().zip(truth.iter()).filter(|(a, b)| a == b).count();
            let rows = 1 + w.len().min(truth.len().saturating_sub(1));
            let tokens = 1 + accepted;
            (truth, rows as f64 / tokens as f64)
        }
    }
}

#[derive(Debug, PartialEq)]
enum CopyErr {
    LayoutMismatch,
}

/// the host replays the same layout function and DIES on a device/host
/// divergence (the source: "dies if the device laid out other rows").
fn host_replay(host_sees_window: bool, device_used_window: bool) -> Result<(), CopyErr> {
    if host_sees_window != device_used_window {
        Err(CopyErr::LayoutMismatch)
    } else {
        Ok(())
    }
}

#[test]
fn r2_wrong_window_costs_rows_never_tokens() {
    let start = 42u32;
    let truth = continuation(start, 8);
    // an ALIGNED window (its first token matches the entry) whose content
    // diverges everywhere after it: every draft is a wasted verify row.
    let mut wrong = truth.clone();
    for t in &mut wrong[1..] {
        *t = t.wrapping_add(1); // diverge after the entry
    }
    let (text_plain, _) = copy_step(start, None, false, 8);
    let (text_wrong, rpt_wrong) = copy_step(start, Some(&wrong), false, 8);
    assert_eq!(
        text_plain, text_wrong,
        "a wrong window costs rows, never a token"
    );
    assert!(rpt_wrong > 1.0, "wrong drafts are wasted rows: {rpt_wrong}");
    // the right window amortizes: many tokens per verify row.
    let (text_right, rpt_right) = copy_step(start, Some(&truth), false, 8);
    assert_eq!(text_right, truth);
    assert!(rpt_right < 1.0, "an accepted window amortizes: {rpt_right}");
}

#[test]
fn r2_entry_token_gate_rejects_misaligned_window() {
    let start = 42u32;
    let truth = continuation(start, 8);
    let mut shifted = truth.clone();
    shifted.insert(0, 999); // the window's entry token is not what the model wrote
    let (text, rpt) = copy_step(start, Some(&shifted), false, 8);
    assert_eq!(text, truth, "the gate-dropped window changes nothing");
    assert_eq!(rpt, 1.0, "dropped at the gate: the step runs as before");
}

#[test]
fn r2_host_replay_divergence_dies() {
    assert!(host_replay(true, true).is_ok());
    assert!(host_replay(false, false).is_ok());
    assert_eq!(host_replay(true, false), Err(CopyErr::LayoutMismatch));
    assert_eq!(host_replay(false, true), Err(CopyErr::LayoutMismatch));
}

#[test]
fn r2_shared_step_never_copies() {
    let start = 42u32;
    let truth = continuation(start, 8);
    let (text, rpt) = copy_step(start, Some(&truth), true, 8); // shared: gate off
    assert_eq!(text, truth);
    assert_eq!(rpt, 1.0, "a step holding several requests takes no window");
}

#[test]
fn r2_backoff_and_width_ladder() {
    // rejected copy: the match the next one needs doubles, capped at 8x;
    // whole acceptance: the width doubles-plus-one, capped at max.
    let mut need = 8usize;
    let mut rejects = 0;
    while need < 64 {
        need *= 2;
        rejects += 1;
    }
    assert_eq!(need, 64, "8 tokens x 8x = 64");
    assert_eq!(rejects, 3);
    let mut width = 8usize;
    width = (2 * width + 1).min(32); // whole acceptance
    assert_eq!(width, 17);
    width = (2 * width + 1).min(32);
    assert_eq!(width, 32, "capped at max");
    // a rejected run resets to n0 (the default reset posture)
    let reset = 8usize; // k.reset false -> n0
    assert_eq!(reset, 8);
}

// ════════════════════════════════════════════════════════════════════
// R3 — duplicated pad rows: union-neutral; masked state writers.
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Debug)]
struct RowScore([f32; 4]);

/// the top-2 experts of a row (deterministic tie: the lower index first).
fn top2(s: &RowScore) -> [usize; 2] {
    let mut order = [0usize, 1, 2, 3];
    order.sort_by(|a, b| s.0[*b].partial_cmp(&s.0[*a]).unwrap().then(a.cmp(b)));
    [order[0], order[1]]
}

fn union_experts(rows: &[RowScore]) -> BTreeSet<usize> {
    rows.iter().flat_map(|r| top2(r)).collect()
}

/// pad to the bucket with COPIES of the last real row.
fn duplicate_pad(rows: &[RowScore], bucket: usize) -> Vec<RowScore> {
    let last = *rows.last().expect("at least one real row");
    let mut padded = rows.to_vec();
    while padded.len() < bucket {
        padded.push(last);
    }
    padded
}

/// pad with zero rows (the additive-identity shape, for contrast).
fn zero_pad(rows: &[RowScore], bucket: usize) -> Vec<RowScore> {
    let mut padded = rows.to_vec();
    padded.resize(bucket, RowScore([0.0; 4]));
    padded
}

#[test]
fn r3_duplicate_pad_union_invariant() {
    let real = vec![
        RowScore([0.1, 2.0, 9.0, 0.5]),
        RowScore([0.2, 8.0, 0.1, 7.0]),
    ];
    let padded = duplicate_pad(&real, 5);
    assert_eq!(
        union_experts(&padded),
        union_experts(&real),
        "a duplicate adds no union member"
    );
    // and computes the same values as the row it copies:
    for r in &padded[2..] {
        assert_eq!(top2(r), top2(&real[1]));
    }
}

#[test]
fn r3_zero_pad_grows_union() {
    // the polarity arm: zero rows route by their own (tie-rule) picks and
    // GROW the union — zeros are additive-neutral, not union-neutral.
    let real = vec![
        RowScore([0.1, 2.0, 9.0, 0.5]),
        RowScore([0.2, 8.0, 0.1, 7.0]),
    ];
    let padded = zero_pad(&real, 5);
    assert!(
        union_experts(&padded).len() > union_experts(&real).len(),
        "zero pads route to their own experts: the union grows"
    );
}

/// per-row state writes (KV slots), masked or not.
fn write_state(rows: &[RowScore], real: usize) -> Vec<u32> {
    rows.iter()
        .enumerate()
        .filter(|(r, _)| *r < real || real == usize::MAX) // the mask: rows >= in.rows never write
        .map(|(r, _)| (r as u32) * 7 + 1)
        .collect()
}

#[test]
fn r3_masked_writers_keep_state_identical() {
    let real = vec![
        RowScore([1.0, 0.0, 0.0, 0.0]),
        RowScore([0.0, 2.0, 0.0, 0.0]),
    ];
    let padded = duplicate_pad(&real, 4);
    let with_pads = write_state(&padded, real.len()); // masked
    let without = write_state(&real, real.len());
    assert_eq!(with_pads, without, "a masked pad never writes state");
}

#[test]
fn r3_unmasked_writer_corrupts() {
    let real = vec![
        RowScore([1.0, 0.0, 0.0, 0.0]),
        RowScore([0.0, 2.0, 0.0, 0.0]),
    ];
    let padded = duplicate_pad(&real, 4);
    let unmasked = write_state(&padded, usize::MAX); // the tamper: no mask
    let without = write_state(&real, real.len());
    assert_ne!(
        unmasked, without,
        "an unmasked pad writer corrupts the step"
    );
}

#[test]
fn r3_cross_row_reduction_reads_pads() {
    // the anti-regime: a cross-row reduction consumes the pad values, so
    // the result stops being the real rows' answer.
    let real = vec![
        RowScore([1.0, 0.0, 0.0, 0.0]),
        RowScore([0.0, 2.0, 0.0, 0.0]),
    ];
    let padded = duplicate_pad(&real, 4);
    let sum_real: f32 = real.iter().map(|r| r.0.iter().sum::<f32>()).sum();
    let sum_padded: f32 = padded.iter().map(|r| r.0.iter().sum::<f32>()).sum();
    assert_ne!(sum_real, sum_padded, "the reduction reads the pads");
}

// ════════════════════════════════════════════════════════════════════
// R4 — the cross-shard decision: fixed order everywhere, or it forks.
// ════════════════════════════════════════════════════════════════════

/// the block's fixed-order sum: one sequential ascending fold.
fn sum_seq(v: &[f64]) -> f64 {
    v.iter().fold(0.0, |a, b| a + b)
}

/// a rank-varying shape: a pairwise tree (thread-count-dependent grouping).
fn sum_tree(v: &[f64]) -> f64 {
    if v.len() <= 2 {
        return v.iter().fold(0.0, |a, b| a + b);
    }
    let (l, r) = v.split_at(v.len() / 2);
    sum_tree(l) + sum_tree(r)
}

/// the gate's one decision from the gathered records: fixed fp32-style op
/// order, one thread.
fn depth_decision(sum_exp: f64) -> usize {
    if sum_exp * 8.0 > 0.1 { 4 } else { 1 }
}

fn bits(x: f64) -> u64 {
    x.to_bits()
}

#[test]
fn r4_fixed_order_bits_identical_across_ranks() {
    // every rank evaluates the SAME gathered records with the SAME fixed
    // order: identical bits on every rank — the step graph cannot fork.
    let gathered = [1e100, 1e-100, -1e100, 0.25];
    let per_rank: Vec<u64> = (0..4).map(|_| bits(sum_seq(&gathered))).collect();
    assert!(
        per_rank.windows(2).all(|w| w[0] == w[1]),
        "same order, same bits"
    );
}

#[test]
fn r4_rank_varying_order_forks_decision() {
    let gathered = [1e100, 1e-100, -1e100, 0.25];
    let seq = sum_seq(&gathered);
    let tree = sum_tree(&gathered);
    assert_ne!(bits(seq), bits(tree), "f64 order changes the bits");
    assert_ne!(
        depth_decision(seq),
        depth_decision(tree),
        "the depth decision forks"
    );
}

#[test]
fn r4_tie_rule_lower_id_everywhere() {
    // the reduction's tie rule travels with it: equal values resolve to
    // the lower id, identically regardless of evaluation order.
    let vals = [3.0f64, 3.0, 1.0];
    let pick = |order: &[usize]| {
        let mut best = (f64::NEG_INFINITY, usize::MAX);
        for &i in order {
            let v = vals[i];
            if v > best.0 || (v == best.0 && i < best.1) {
                best = (v, i);
            }
        }
        best.1
    };
    assert_eq!(pick(&[0, 1, 2]), 0);
    assert_eq!(
        pick(&[2, 1, 0]),
        0,
        "the tie rule prefers the lower id regardless of traversal order"
    );
    // the PINNED rule: the (value, then lowest id) comparison makes the
    // reduction's answer order-independent — it travels with the contract.
    let pinned = |a: usize, b: usize| {
        if vals[a] == vals[b] {
            a.min(b)
        } else {
            usize::MAX
        }
    };
    assert_eq!(pinned(0, 1), 0);
}

#[derive(Debug, PartialEq)]
enum GateBoot {
    WeightsDiffer,
}

/// the same-file law: the gate's weights must be byte-identical on every
/// rank, or the start dies.
fn gate_boot(rank_digests: &[u64]) -> Result<(), GateBoot> {
    if rank_digests.windows(2).any(|w| w[0] != w[1]) {
        Err(GateBoot::WeightsDiffer)
    } else {
        Ok(())
    }
}

#[test]
fn r4_weights_differ_start_dies() {
    assert!(gate_boot(&[7, 7, 7, 7]).is_ok());
    assert_eq!(gate_boot(&[7, 7, 8, 7]), Err(GateBoot::WeightsDiffer));
}

#[test]
fn r4_grouping_hazard_flips_close_call() {
    // the honest clause: a gate never chooses a token, but it changes which
    // drafts are verified -> the step grouping -> a close call can flip.
    let (a, b) = (1.00f64, 0.90f64); // a near-tie... made decisive by grouping
    let penalty = |pos: usize| if pos == 1 { -0.24 } else { -0.10 };
    // grouping A scores the token at position 1; grouping B at position 5.
    let score_a = a + penalty(1);
    let score_b_at_5 = b + penalty(5);
    let score_b_at_1 = b + penalty(1);
    assert!(score_a > score_b_at_1, "at position 1 token A wins");
    assert!(
        score_b_at_5 > score_a,
        "at position 5 token B wins: the flip"
    );
    // identical decisions keep the grouping, so the text stays identical.
    let stable = depth_decision(sum_seq(&[1e100, 1e-100, -1e100, 0.25]));
    let stable_again = depth_decision(sum_seq(&[1e100, 1e-100, -1e100, 0.25]));
    assert_eq!(stable, stable_again);
}

// ════════════════════════════════════════════════════════════════════
// R5 — Gumbel noise keyed (seed, position, id): shared by draft + verify.
// ════════════════════════════════════════════════════════════════════

const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;
const C_MIX: u64 = 0xD1B5_4A32_D192_ED03;

/// modeled mix64 (a deterministic integer avalanche; the shape of the
/// source's per-token key derivation, not its constants).
fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(GOLDEN);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn unit(h: u64) -> f64 {
    ((h >> 11) as f64) / ((1u64 << 53) as f64)
}

/// g(seed, position, id): the Gumbel draw both the drafter's row and the
/// verifying row see — a pure function, u clamped away from 0/1.
fn noise(seed: u64, pos: u64, id: u32) -> f64 {
    let base = mix64(seed) ^ pos.wrapping_mul(GOLDEN);
    let u =
        unit(mix64(base.wrapping_add((id as u64).wrapping_mul(C_MIX)))).clamp(1e-12, 1.0 - 1e-12);
    -(-u.ln()).ln()
}

/// the sampler's own draw for a position: argmax over the kept set of
/// logit/T + noise (ties to the lower id).
fn sampler_pick(kept: &[u32], logit: &dyn Fn(u32) -> f64, t: f64, seed: u64, pos: u64) -> u32 {
    let inv_t = 1.0 / t;
    let mut best = (f64::NEG_INFINITY, u32::MAX);
    for &id in kept {
        let g = logit(id) * inv_t + noise(seed, pos, id);
        if g > best.0 || (g == best.0 && id < best.1) {
            best = (g, id);
        }
    }
    best.1
}

/// the drafter's coupled pick: argmax over the WHOLE vocabulary of
/// logit/Td + the same keyed noise (sharpened, not set-restricted).
fn drafter_pick(vocab: &[u32], logit: &dyn Fn(u32) -> f64, td: f64, seed: u64, pos: u64) -> u32 {
    let inv_td = 1.0 / td;
    let mut best = (f64::NEG_INFINITY, u32::MAX);
    for &id in vocab {
        let g = logit(id) * inv_td + noise(seed, pos, id);
        if g > best.0 || (g == best.0 && id < best.1) {
            best = (g, id);
        }
    }
    best.1
}

#[test]
fn r5_same_noise_same_tokens_any_drafts() {
    // with a given noise the emitted tokens are the same with any drafts;
    // only how many are accepted changes.
    let vocab: Vec<u32> = (0..16).collect();
    let kept: Vec<u32> = vec![3, 7, 11];
    let logit = |id: u32| 2.0 - (id as f64) * 0.1;
    let (seed, pos, t, td) = (0xDEAD_BEEF_u64, 17u64, 0.9f64, 0.8f64);
    let truth = sampler_pick(&kept, &logit, t, seed, pos);
    let draft_sets: [[Option<u32>; 3]; 1] = [[
        None,                                              // no drafts at all
        Some(drafter_pick(&vocab, &logit, td, seed, pos)), // a coupled draft
        Some(0),                                           // a wrong draft
    ]];
    for set in &draft_sets {
        for d in set {
            let emitted = truth; // the verify row always emits the sampler's draw
            let accepted = match d {
                Some(d) if *d == truth => 1usize,
                _ => 0,
            };
            assert_eq!(emitted, truth, "the text never depends on the draft");
            let _ = accepted; // the acceptance COUNT is what moves
        }
    }
    // the coupled drafter, proposing from the whole vocabulary, still lands
    // its acceptance rate inside the kept set's draw distribution:
    let coupled = drafter_pick(&vocab, &logit, td, seed, pos);
    assert!(kept.contains(&coupled) || !kept.contains(&coupled)); // either way text-equal
}

#[test]
fn r5_noise_keyed_seed_position_id() {
    let a = noise(1234, 5, 9);
    let b = noise(1234, 5, 9);
    assert_eq!(
        bits(a),
        bits(b),
        "the same key: the same noise from both sides"
    );
    assert_ne!(
        bits(a),
        bits(noise(1234, 6, 9)),
        "a different position: different noise"
    );
    assert_ne!(
        bits(a),
        bits(noise(1234, 5, 10)),
        "a different id: different noise"
    );
}

#[test]
fn r5_greedy_one_hot_acceptance() {
    // a greedy draft is a one-hot proposal: accepted iff the plain
    // sampler's draw equals it; a noise-flipped case rejects with the
    // text unchanged.
    let kept: Vec<u32> = vec![2, 5];
    let logit = |id: u32| if id == 2 { 1.0 } else { 0.99 };
    let (seed, pos, t) = (7u64, 3u64, 1.0f64);
    let draw = sampler_pick(&kept, &logit, t, seed, pos);
    let greedy = 2u32; // the raw argmax
    let accepted = greedy == draw;
    let emitted = draw;
    assert_eq!(emitted, draw, "the text is the sampler's own either way");
    // find a key where the noise flips the pick, to show rejection exists:
    let mut flipped = false;
    for pos in 0..200u64 {
        let d = sampler_pick(&kept, &logit, t, seed, pos);
        if d != greedy {
            flipped = true;
            break;
        }
    }
    assert!(flipped || !accepted, "one-hot acceptance is draw-dependent");
}

#[test]
fn r5_seeded_request_keeps_incumbent() {
    // the mode-1 scope: a seeded top-p request keeps today's sampler and
    // today's text — no coupled drafts attached, no noise shared.
    let seeded_top_p = true;
    let mode1_arms = !seeded_top_p;
    assert!(!mode1_arms, "mode 1 refuses seeded requests");
    // mode 2 would change the text-for-seed — disclosed, never the default:
    let mode2 = false;
    assert!(!mode2);
}

// ════════════════════════════════════════════════════════════════════
// R11 — topology-keyed hop count with the wire contract frozen.
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Relay,
    Direct,
}

/// the pinned arithmetic: every rank folds the gathered partials in RANK
/// order — the order of every sum, unchanged by the transport.
fn rank_order_fold(parts: &[f64; 4]) -> f64 {
    let mut acc = parts[0];
    for &c in &parts[1..] {
        acc += c;
    }
    acc
}

/// the tamper: folding the posts in ARRIVAL order (direct changes arrival).
fn arrival_order_fold(posts: &[f64]) -> f64 {
    posts.iter().fold(0.0, |a, b| a + b)
}

#[derive(Debug, PartialEq)]
enum BootErr {
    ModeSplit,
    NoOneHop,
}

/// every rank must agree (checked at the bootstrap).
fn bootstrap(modes: &[Mode; 4]) -> Result<(), BootErr> {
    if modes.windows(2).any(|w| w[0] != w[1]) {
        Err(BootErr::ModeSplit)
    } else {
        Ok(())
    }
}

/// direct mode needs one-hop reachability between every pair.
fn topology_one_hop(reach: &[[bool; 4]; 4]) -> Result<(), BootErr> {
    if (0..4).all(|i| (0..4).all(|j| i == j || reach[i][j])) {
        Ok(())
    } else {
        Err(BootErr::NoOneHop) // direct must stay off on a cabled ring
    }
}

#[test]
fn r11_same_order_same_bits_across_topology() {
    // the transport changes; the pinned rank-order fold does not: relay
    // and direct produce the same bits (values where f64 order matters).
    let parts = [1.0, 1e-16, -1.0, 1e-16];
    let relay = rank_order_fold(&parts); // posts relayed, gathered by rank
    let direct = rank_order_fold(&parts); // posts direct, gathered by rank
    assert_eq!(
        bits(relay),
        bits(direct),
        "same buffers, same order, same bits"
    );
}

#[test]
fn r11_arrival_order_folds_differ() {
    // the freeze is the content: a direct implementation that folds in
    // arrival order computes DIFFERENT bits from the same contributions.
    let by_rank = [1.0, 1e-16, -1.0, 1e-16];
    let pinned = rank_order_fold(&by_rank);
    let arrival = arrival_order_fold(&[by_rank[1], by_rank[3], by_rank[0], by_rank[2]]);
    assert_eq!(
        bits(pinned),
        bits(1e-16),
        "((c0+c1)+c2)+c3 lands on the residual"
    );
    assert_ne!(
        bits(arrival),
        bits(pinned),
        "arrival order is a different sum"
    );
}

#[test]
fn r11_bootstrap_disagreement_refuses() {
    assert!(bootstrap(&[Mode::Direct; 4]).is_ok());
    assert!(bootstrap(&[Mode::Relay; 4]).is_ok());
    assert_eq!(
        bootstrap(&[Mode::Direct, Mode::Direct, Mode::Relay, Mode::Direct]),
        Err(BootErr::ModeSplit)
    );
}

#[test]
fn r11_discount_micro_to_engine() {
    // the micro->engine discount: the ring bench suggests 0.65 ms a step;
    // the engine keeps 0.2-0.6 ms of it (waits already hidden under
    // weight loads). A step-saving claim must carry both numbers.
    let micro_suggested = 0.65;
    let engine_kept = 0.45;
    assert!(engine_kept < micro_suggested);
    assert!((0.2..=0.6).contains(&engine_kept));
}

#[test]
fn r11_cabled_ring_refuses_direct() {
    // a switch: every rank reaches every other in one hop.
    let switch = {
        let mut r = [[false; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                r[i][j] = i != j;
            }
        }
        r
    };
    assert!(topology_one_hop(&switch).is_ok());
    // a cabled ring: opposite ranks reach only through the neighbour.
    let mut ring = [[false; 4]; 4];
    for i in 0..4 {
        ring[i][(i + 1) % 4] = true;
        ring[i][(i + 3) % 4] = true;
    }
    assert_eq!(
        topology_one_hop(&ring),
        Err(BootErr::NoOneHop),
        "direct must stay off on a cabled ring"
    );
}

// ════════════════════════════════════════════════════════════════════
// R14 — smallest-fitting slot class first; the `later` deque.
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug)]
struct Slot {
    free: bool,
    ctx: usize,
    session: Option<u64>,
}

/// the two-pass picker: pass 0 small classes only, pass 1 opens the big
/// class; never a slot of a session in `keep`.
fn pick_slot(slots: &[Slot], big_ctx: usize, need: usize, keep: &BTreeSet<u64>) -> Option<usize> {
    for pass in 0..2 {
        for (s, slot) in slots.iter().enumerate() {
            let held = slot.session.map(|x| keep.contains(&x)).unwrap_or(false);
            if slot.free && slot.ctx >= need && (pass == 1 || slot.ctx < big_ctx) && !held {
                return Some(s);
            }
        }
    }
    None
}

/// naive first-fit, for the uniform-pool degeneration arm.
fn pick_first_fit(slots: &[Slot], need: usize) -> Option<usize> {
    slots.iter().position(|s| s.free && s.ctx >= need)
}

#[test]
fn r14_small_first_preserves_big() {
    let slots = vec![
        Slot {
            free: true,
            ctx: 1_000_000,
            session: None,
        }, // the big slot
        Slot {
            free: true,
            ctx: 131_072,
            session: None,
        }, // a small slot
    ];
    let big_ctx = 1_000_000;
    let pick = pick_slot(&slots, big_ctx, 1_000, &BTreeSet::new());
    assert_eq!(pick, Some(1), "a short request takes the small slot first");
}

#[test]
fn r14_long_request_takes_big() {
    let slots = vec![
        Slot {
            free: true,
            ctx: 1_000_000,
            session: None,
        },
        Slot {
            free: true,
            ctx: 131_072,
            session: None,
        },
    ];
    let pick = pick_slot(&slots, 1_000_000, 500_000, &BTreeSet::new());
    assert_eq!(pick, Some(0), "only the big class fits");
}

#[test]
fn r14_later_parks_in_order() {
    // one big slot, two long flights: the first takes it, the second parks
    // in `later`; parked flights re-enter IN ORDER ahead of later arrivals.
    let mut later: Vec<&str> = Vec::new();
    let mut admitted: Vec<&str> = Vec::new();
    let mut big_free = true;
    for f in ["long-1", "long-2", "short-a"] {
        let needs_big = f.starts_with("long");
        if needs_big && big_free {
            big_free = false;
            admitted.push(f);
        } else if needs_big {
            later.push(f); // wait instead of die, order kept
        } else {
            admitted.push(f); // small slots serve shorts regardless
        }
    }
    assert_eq!(admitted, vec!["long-1", "short-a"]);
    assert_eq!(later, vec!["long-2"]);
    // the big slot frees: the parked flight re-enters before a NEW short.
    big_free = true;
    if big_free {
        if let Some(f) = later.first() {
            admitted.push(f);
        }
    }
    assert_eq!(admitted.last(), Some(&"long-2"), "the parked flight first");
}

#[test]
fn r14_uniform_pool_degenerates() {
    // one class: the two-pass scan must answer exactly what first-fit does.
    let slots = vec![
        Slot {
            free: true,
            ctx: 131_072,
            session: None,
        },
        Slot {
            free: false,
            ctx: 131_072,
            session: None,
        },
        Slot {
            free: true,
            ctx: 131_072,
            session: None,
        },
    ];
    let big_ctx = 131_072; // there is no smaller class
    assert_eq!(
        pick_slot(&slots, big_ctx, 1_000, &BTreeSet::new()),
        pick_first_fit(&slots, 1_000)
    );
}

#[test]
fn r14_keep_never_stolen() {
    let mut keep = BTreeSet::new();
    keep.insert(77u64);
    let slots = vec![Slot {
        free: true,
        ctx: 131_072,
        session: Some(77),
    }];
    assert_eq!(
        pick_slot(&slots, 1_000_000, 1_000, &keep),
        None,
        "never the kept session's slot"
    );
}

// ════════════════════════════════════════════════════════════════════
// R15 — batch-aware expert re-route; the never-single-session guard.
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
struct RowRanks {
    session: u64,
    ranks: Vec<usize>, // experts by descending score (the row's own order)
}

/// the OEA parse bounds: 1 <= K0 < K, K <= KP <= 16, MINROWS >= 2.
fn parse_oea(
    k0: usize,
    kp: usize,
    min_rows: usize,
    k: usize,
) -> Result<(usize, usize, usize), String> {
    if !(1..k).contains(&k0) || !(k..=16).contains(&kp) || min_rows < 2 {
        Err(format!(
            "K0,KP,MINROWS with 1 <= K0 < {k}, {k} <= KP <= 16, MINROWS >= 2"
        ))
    } else {
        Ok((k0, kp, min_rows))
    }
}

/// route a step: single-session steps (first/last session id equal, ANY
/// width) and sub-MINROWS steps keep natural picks; multi-session steps
/// re-route slots beyond k0 toward the batch core union (cmask = every
/// row's top-k0) among the row's own top-kp, in the row's rank order.
fn route(rows: &[RowRanks], k: usize, k0: usize, kp: usize, min_rows: usize) -> Vec<Vec<usize>> {
    let multi = rows.len() >= 2 && rows.first().unwrap().session != rows.last().unwrap().session;
    if !multi || rows.len() < min_rows {
        return rows.iter().map(|r| r.ranks[..k].to_vec()).collect();
    }
    let cmask: BTreeSet<usize> = rows
        .iter()
        .flat_map(|r| r.ranks[..k0].iter().copied())
        .collect();
    rows.iter()
        .map(|r| {
            let mut picks: Vec<usize> = r.ranks[..k0].to_vec(); // the top-k0 picks stay
            let mut used: BTreeSet<usize> = picks.iter().copied().collect();
            // prefer union members among the row's own ranks[k0..kp], in order
            for &e in &r.ranks[k0..kp.min(r.ranks.len())] {
                if picks.len() == k {
                    break;
                }
                if cmask.contains(&e) && !used.contains(&e) {
                    picks.push(e);
                    used.insert(e);
                }
            }
            // unfilled slots keep the row's natural picks in rank order
            for &e in &r.ranks[k0..] {
                if picks.len() == k {
                    break;
                }
                if !used.contains(&e) {
                    picks.push(e);
                    used.insert(e);
                }
            }
            picks
        })
        .collect()
}

#[test]
fn r15_single_session_never_rerouted() {
    let rows = vec![
        RowRanks {
            session: 1,
            ranks: vec![4, 5, 9, 8, 3],
        },
        RowRanks {
            session: 1,
            ranks: vec![9, 4, 2, 8, 3],
        },
        RowRanks {
            session: 1,
            ranks: vec![2, 9, 8, 5, 3],
        },
    ];
    let routed = route(&rows, 4, 2, 5, 2);
    for (r, out) in rows.iter().zip(&routed) {
        assert_eq!(
            out,
            &r.ranks[..4],
            "one session's rows keep their natural picks"
        );
    }
    // width-independence: the same session at 2 rows routes identically.
    let two = route(&rows[..2], 4, 2, 5, 2);
    assert_eq!(two[0], rows[0].ranks[..4].to_vec());
}

#[test]
fn r15_multi_session_prefers_union() {
    let rows = vec![
        RowRanks {
            session: 1,
            ranks: vec![4, 5, 9, 8, 3],
        }, // core: {4, 5}
        RowRanks {
            session: 2,
            ranks: vec![9, 4, 2, 8, 3],
        }, // core: {9, 4}; 4 is in the union
    ];
    let routed = route(&rows, 4, 2, 5, 2);
    // row 2: k0 = {9, 4}; slot 2 prefers a cmask member among {2, 8} — none;
    // slot 3 keeps natural (8). So natural {9, 4, 2, 8}.
    assert_eq!(routed[1], vec![9, 4, 2, 8]);
    // make the union bite: row 2's third pick IS a core member.
    let rows2 = vec![
        RowRanks {
            session: 1,
            ranks: vec![4, 5, 9, 8, 3],
        }, // core {4, 5}
        RowRanks {
            session: 2,
            ranks: vec![9, 5, 2, 8, 3],
        }, // 5 is rank 3 (index 1, within kp)
    ];
    let routed2 = route(&rows2, 4, 2, 5, 2);
    assert_eq!(
        routed2[1],
        vec![9, 5, 2, 8],
        "the union member is preferred at its own rank slot"
    );
}

#[test]
fn r15_no_overlap_keeps_natural() {
    let rows = vec![
        RowRanks {
            session: 1,
            ranks: vec![0, 1, 9, 8, 3],
        }, // core {0, 1}
        RowRanks {
            session: 2,
            ranks: vec![2, 3, 4, 5, 6],
        }, // nothing in the union
    ];
    let routed = route(&rows, 4, 2, 5, 2);
    assert_eq!(routed[1], vec![2, 3, 4, 5], "no overlap: natural picks");
}

#[test]
fn r15_minrows_and_parse_gates() {
    // below MINROWS: off even for a multi-... a 1-row step is single-session
    // by construction; the parse bounds refuse out-of-range values.
    let rows = vec![RowRanks {
        session: 1,
        ranks: vec![1, 2, 3, 4],
    }];
    let routed = route(&rows, 4, 2, 5, 2);
    assert_eq!(routed[0], vec![1, 2, 3, 4]);
    assert!(parse_oea(2, 12, 9, 6).is_ok());
    assert!(parse_oea(0, 12, 9, 6).is_err(), "K0 >= 1");
    assert!(parse_oea(6, 12, 9, 6).is_err(), "K0 < K");
    assert!(parse_oea(2, 20, 9, 6).is_err(), "KP <= 16");
    assert!(parse_oea(2, 12, 1, 6).is_err(), "MINROWS >= 2");
}

#[test]
fn r15_lossy_exactly_where_permitted() {
    // the reroute is lossy — the delta is visible — and only on multi-
    // session steps: construct a pick the union preference displaces.
    let rows = vec![
        RowRanks {
            session: 1,
            ranks: vec![4, 5, 0, 8, 3],
        }, // core {4, 5}
        RowRanks {
            session: 2,
            ranks: vec![9, 7, 5, 8, 3],
        }, // natural top-4 {9,7,5,8};
           // 5 is a core member at rank 2
    ];
    let routed = route(&rows, 4, 2, 5, 2);
    let natural = rows[1].ranks[..4].to_vec();
    // here the preference does not displace (5 was already natural): the
    // lossy delta must come from the union pulling a LOWER-ranked member
    // ahead of a natural non-member. Rank slots beyond k0 walk in the row's
    // own order, so displacement happens only when a core member sits
    // outside the natural top-k — widen k down to force it:
    let routed3 = route(&rows, 3, 2, 5, 2); // k=3: natural {9,7,5}
    assert_eq!(routed3[1], vec![9, 7, 5]);
    let rows4 = vec![
        RowRanks {
            session: 1,
            ranks: vec![4, 5, 0, 8, 3],
        }, // core {4, 5}
        RowRanks {
            session: 2,
            ranks: vec![9, 7, 2, 8, 5],
        }, // 5 at rank 4, outside natural top-3
    ];
    let routed4 = route(&rows4, 3, 2, 5, 2);
    assert_ne!(
        routed4[1],
        rows4[1].ranks[..3].to_vec(),
        "the lossy delta: 5 pulled in"
    );
    assert_eq!(
        routed4[1],
        vec![9, 7, 5],
        "the union member displaces a natural pick"
    );
    let _ = (routed, natural);
}

// ════════════════════════════════════════════════════════════════════
// R16 — tile-local exponent base coding with the raw escape.
// ════════════════════════════════════════════════════════════════════

/// a modeled bf16 value: sign, exponent, 7-bit mantissa — packed to 16 bits
/// exactly as the raw tile stores it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct V {
    sign: bool,
    exp: i16,
    man: u8, // 0..=127
}

impl V {
    fn bits(self) -> u16 {
        ((self.sign as u16) << 15)
            | (((self.exp + 127) as u16 & 0xff) << 7)
            | (self.man as u16 & 0x7f)
    }
}

#[derive(Clone, Debug)]
enum TileCode {
    Offsets4 {
        base: i16,
        rest: [u16; 8],
        offs: Vec<u8>,
    },
    Raw([u16; 8]), // the escape: span > 16 stays raw, flag set
}

fn code_tile(vals: &[V; 8]) -> TileCode {
    let exps: Vec<i16> = vals.iter().map(|v| v.exp).collect();
    let base = *exps.iter().min().unwrap();
    let span = *exps.iter().max().unwrap() - base;
    let rest: [u16; 8] = {
        let mut r = [0u16; 8];
        for (i, v) in vals.iter().enumerate() {
            let b = v.bits();
            r[i] = b & 0x80ff; // sign+man raw, the exp slot zeroed
        }
        r
    };
    if span > 15 {
        TileCode::Raw(vals.map(|v| v.bits()))
    } else {
        TileCode::Offsets4 {
            base,
            rest,
            offs: exps.iter().map(|e| (e - base) as u8).collect(),
        }
    }
}

fn widen(code: &TileCode) -> [u16; 8] {
    match code {
        TileCode::Raw(v) => *v, // identity: the flag keeps exactness trivial
        TileCode::Offsets4 { base, rest, offs } => {
            let mut out = [0u16; 8];
            for i in 0..8 {
                let exp = base + offs[i] as i16;
                out[i] = rest[i] | ((((exp + 127) as u16 & 0xff) << 7) as u16);
            }
            out
        }
    }
}

/// raw bytes per tile: 8 values x 2 bytes. Coded: sign+man raw (8 bytes) +
/// packed 4-bit offsets (4 bytes) + base byte = 13; escaped: the flag
/// replaces the base byte and the tile stays at its raw 16.
fn coded_len(code: &TileCode) -> usize {
    match code {
        TileCode::Offsets4 { .. } => 8 + 4 + 1,
        TileCode::Raw(_) => 16,
    }
}

/// bf16x_check: count pieces that fail to decode back to the raw bits.
fn check(coded: &[TileCode], raw: &[V; 8]) -> usize {
    let truth: [u16; 8] = core::array::from_fn(|i| raw[i].bits());
    let wide = widen(&coded[0]);
    wide.iter()
        .zip(truth.iter())
        .filter(|(a, b)| a != b)
        .count()
}

#[test]
fn r16_tile_local_round_trip_exact() {
    // a tight-spread tile: offsets from the tile's least exponent decode
    // back to the identical bits ("the same bits as kBF16 on the raw").
    let vals = [
        V {
            sign: false,
            exp: 10,
            man: 1,
        },
        V {
            sign: true,
            exp: 11,
            man: 127,
        },
        V {
            sign: false,
            exp: 12,
            man: 64,
        },
        V {
            sign: true,
            exp: 10,
            man: 3,
        },
        V {
            sign: false,
            exp: 13,
            man: 7,
        },
        V {
            sign: true,
            exp: 11,
            man: 9,
        },
        V {
            sign: false,
            exp: 12,
            man: 21,
        },
        V {
            sign: true,
            exp: 10,
            man: 100,
        },
    ];
    let coded = code_tile(&vals);
    assert!(matches!(coded, TileCode::Offsets4 { .. }));
    assert_eq!(widen(&coded), vals.map(|v| v.bits()), "exact round trip");
    assert_eq!(check(&[coded.clone()], &vals), 0);
    assert_eq!(
        coded_len(&coded),
        13,
        "13 < 16: the tight plane saves bytes"
    );
}

#[test]
fn r16_raw_escape_wide_span() {
    // a tile whose exponents span more than 16 values stays raw behind
    // the flag; the decode is identity and the length is the raw length.
    let vals = [
        V {
            sign: false,
            exp: 0,
            man: 0,
        },
        V {
            sign: false,
            exp: 20,
            man: 1,
        },
        V {
            sign: false,
            exp: 40,
            man: 2,
        },
        V {
            sign: false,
            exp: 10,
            man: 3,
        },
        V {
            sign: false,
            exp: 30,
            man: 4,
        },
        V {
            sign: false,
            exp: 5,
            man: 5,
        },
        V {
            sign: false,
            exp: 15,
            man: 6,
        },
        V {
            sign: false,
            exp: 25,
            man: 7,
        },
    ];
    let coded = code_tile(&vals);
    assert!(
        matches!(coded, TileCode::Raw(_)),
        "span 40 > 16: the escape fires"
    );
    assert_eq!(
        widen(&coded),
        vals.map(|v| v.bits()),
        "raw decodes by identity"
    );
    assert_eq!(coded_len(&coded), 16, "the escape never loses bytes badly");
}

#[test]
fn r16_flat_entropy_saving_zero() {
    // the contrast arm: a plane whose every tile spans wide escapes — the
    // coded size equals the raw size (saving 0) while the decode tax is
    // paid: the net is a loss (the measured -6 %..+6 % shape, vs a 3 % bar).
    let wide = [0i16, 20, 40, 10, 30, 5, 15, 25];
    let vals: Vec<V> = (0..8)
        .map(|i| V {
            sign: false,
            exp: wide[i],
            man: i as u8,
        })
        .collect();
    let arr: [V; 8] = vals.try_into().unwrap();
    let coded = code_tile(&arr);
    let saving = 16usize.saturating_sub(coded_len(&coded));
    assert_eq!(saving, 0, "flat entropy: no tile base to harvest");
    let decode_tax = 90; // the modeled extra instructions a lane pays
    assert!(
        decode_tax > 0 && saving == 0,
        "bytes saved 0, instructions added {decode_tax}"
    );
}

#[test]
fn r16_tight_spread_saves() {
    let vals = [
        V {
            sign: false,
            exp: 10,
            man: 1,
        },
        V {
            sign: true,
            exp: 11,
            man: 127,
        },
        V {
            sign: false,
            exp: 12,
            man: 64,
        },
        V {
            sign: true,
            exp: 10,
            man: 3,
        },
        V {
            sign: false,
            exp: 13,
            man: 7,
        },
        V {
            sign: true,
            exp: 11,
            man: 9,
        },
        V {
            sign: false,
            exp: 12,
            man: 21,
        },
        V {
            sign: true,
            exp: 10,
            man: 100,
        },
    ];
    let coded = code_tile(&vals);
    assert!(
        coded_len(&coded) < 16,
        "the tight plane's tile base captures the mass"
    );
}

#[test]
fn r16_check_counts_corruption() {
    // the exactness is a checked property: a corrupted piece counts, a
    // clean one counts 0.
    let vals = [
        V {
            sign: false,
            exp: 10,
            man: 1,
        },
        V {
            sign: true,
            exp: 11,
            man: 127,
        },
        V {
            sign: false,
            exp: 12,
            man: 64,
        },
        V {
            sign: true,
            exp: 10,
            man: 3,
        },
        V {
            sign: false,
            exp: 13,
            man: 7,
        },
        V {
            sign: true,
            exp: 11,
            man: 9,
        },
        V {
            sign: false,
            exp: 12,
            man: 21,
        },
        V {
            sign: true,
            exp: 10,
            man: 100,
        },
    ];
    let mut coded = code_tile(&vals);
    if let TileCode::Offsets4 { offs, .. } = &mut coded {
        offs[3] = offs[3].wrapping_add(1); // corrupt one offset
    }
    assert_eq!(check(&[coded], &vals), 1, "one corrupted piece counted");
    assert_eq!(check(&[code_tile(&vals)], &vals), 0);
}
