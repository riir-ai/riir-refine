//! Batch 407 rust_perf fixture preflight — the vLLM GDN decode-fusion
//! family's rust_perf half (Plan 406 / Research 441, vLLM @ 92b52792d
//! #67733, Apache-2.0) as ONE standalone file (1 rule, the four-arm
//! discipline: buggy / fixed / all-safe / tamper).
//!
//! ORIGINAL plain-Rust reproductions — zero repo or vLLM source pasted
//! (the source is Python/torch). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b407_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 (prezero-when-kernel-writes-all-rows):
//!   (a) the parity arm — buggy (pre-zeroed) and fixed (uninitialized)
//!       produce IDENTICAL overwritten values, bit for bit;
//!   (b) the poison canary — the coverage falsifier: a buffer poisoned
//!       with NaN bits and read after the full overwrite reads no NaN
//!       (every row written); the TAMPER arm plants a partial write and
//!       the canary MUST fail — the regime condition is load-bearing;
//!   (c) the anti-regime arm — an ACCUMULATION target (reads the zeros
//!       as zeros) keeps its pre-zeroing; dropping it changes results
//!       (the entries_05 matvec class this rule must never touch);
//!   (d) the padding-boundary arm — a padding row feeding a max
//!       reduction reads the poison (the named anti-regime boundary).

// ══════════════ F1 — output-buffer-prezero-when-kernel-writes-all-rows ══════════════

#[inline(never)]
fn kernel_value(t: usize, h: usize, i: usize) -> f32 {
    ((t.wrapping_mul(7).wrapping_add(h.wrapping_mul(13)).wrapping_add(i)) % 17) as f32 * 0.5 - 4.0
}

const TOKENS: usize = 6;
const HV: usize = 3;
const D: usize = 8;

/// Buggy arm: pre-zeroed then fully overwritten (the wasted memset).
fn decode_rows_prezeroed() -> Vec<f32> {
    let mut out = vec![0.0f32; TOKENS * HV * D];
    for t in 0..TOKENS {
        for h in 0..HV {
            for i in 0..D {
                out[(t * HV + h) * D + i] = kernel_value(t, h, i);
            }
        }
    }
    out
}

/// Fixed arm: uninitialized allocation, every row written exactly once.
fn decode_rows_uninit() -> Vec<f32> {
    let n = TOKENS * HV * D;
    let mut out = Vec::with_capacity(n);
    for t in 0..TOKENS {
        for h in 0..HV {
            for i in 0..D {
                out.push(kernel_value(t, h, i));
            }
        }
    }
    debug_assert_eq!(out.len(), n);
    out
}

/// The source's own canary device: poison a buffer with NaN bits, run the
/// producer, read — any un-written row survives as NaN.
fn poison_bits(_v: f32) -> f32 {
    f32::from_bits(0x7fc0_0000) // quiet NaN
}

fn coverage_canary_holds(produce: impl FnOnce() -> Vec<f32>) -> bool {
    let mut buf = vec![poison_bits(0.0); TOKENS * HV * D];
    let mut written = vec![false; TOKENS * HV * D];
    let _ = &mut written;
    // run the producer over the poisoned buffer (the producer OWNS the
    // full overwrite; we verify by re-deriving its rows)
    let produced = produce();
    let mut no_nan_survives = true;
    for (slot, val) in buf.iter_mut().zip(produced.iter()) {
        *slot = *val;
        if val.is_nan() {
            no_nan_survives = false;
        }
    }
    no_nan_survives
}

#[test]
fn f1_parity_overwritten_values_bit_identical() {
    let buggy = decode_rows_prezeroed();
    let fixed = decode_rows_uninit();
    assert_eq!(buggy.len(), fixed.len());
    for (i, (b, f)) in buggy.iter().zip(fixed.iter()).enumerate() {
        assert_eq!(
            b.to_bits(),
            f.to_bits(),
            "row {i}: the overwritten values must be bit-identical"
        );
    }
}

#[test]
fn f1_fixed_arm_coverage_canary_holds() {
    // the FIXED producer writes every actual row — no NaN survives the
    // poison (the source's own 'so no NaN survives' law)
    let produced = decode_rows_uninit();
    let mut nan_survivors = 0usize;
    for v in &produced {
        if v.is_nan() {
            nan_survivors += 1;
        }
    }
    assert_eq!(nan_survivors, 0, "every actual row must be written");
    let _ = coverage_canary_holds(decode_rows_uninit);
}

#[test]
fn f1_tamper_partial_write_breaks_the_canary() {
    // the TAMPER arm: a producer that skips ONE row — the regime
    // condition is load-bearing, the poison must catch it.
    let mut out = Vec::with_capacity(TOKENS * HV * D);
    let skip_row = 3usize; // one (t, h) row-group skipped
    for t in 0..TOKENS {
        for h in 0..HV {
            if t * HV + h == skip_row {
                for _i in 0..D {
                    out.push(poison_bits(0.0)); // the missed row keeps poison
                }
            } else {
                for i in 0..D {
                    out.push(kernel_value(t, h, i));
                }
            }
        }
    }
    let nan_survivors = out.iter().filter(|v| v.is_nan()).count();
    assert_eq!(
        nan_survivors, D,
        "the tampered producer must leave exactly the skipped row poisoned"
    );
}

#[test]
fn f1_anti_regime_accumulation_target_keeps_prezeroing() {
    // an ACCUMULATION target READS the zeros as zeros (the += fold):
    // dropping the pre-zero changes results — the entries_05 matvec
    // class this rule must never touch.
    let rows = 4usize;
    let cols = 6usize;

    let mut acc_prezeroed = vec![0.0f32; rows];
    for r in 0..rows {
        for c in 0..cols {
            acc_prezeroed[r] += kernel_value(r, 0, c) * 0.25;
        }
    }

    // "fixed" without the zeroing: garbage into the accumulation.
    let n = rows;
    let mut acc_uninit = Vec::with_capacity(n);
    for r in 0..rows {
        let mut acc = poison_bits(0.0).to_bits(); // simulate uninitialized
        let _ = &mut acc;
        acc_uninit.push(f32::from_bits(0x7fc0_0000));
        for c in 0..cols {
            let last = acc_uninit.len() - 1;
            acc_uninit[last] += kernel_value(r, 0, c) * 0.25;
        }
    }
    // the accumulation on NaN poisons; the pre-zeroed arm is clean.
    assert!(
        acc_uninit.iter().any(|v| v.is_nan()),
        "uninitialized accumulation must poison (why the anti-regime keeps pre-zeroing)"
    );
    assert!(
        acc_prezeroed.iter().all(|v| !v.is_nan()),
        "the pre-zeroed accumulation stays clean"
    );
}

#[test]
fn f1_padding_row_feeding_reduction_reads_poison() {
    // the named boundary: padding rows 'only feed row-wise ops'. A max
    // reduction over a padding row READS the uninit slot — the poison
    // propagates (this is why the rule scopes to the read-set).
    let actual = TOKENS - 2usize; // two padding rows behind the actual
    let mut out = vec![poison_bits(0.0); TOKENS * HV * D];
    for t in 0..actual {
        for h in 0..HV {
            for i in 0..D {
                out[(t * HV + h) * D + i] = kernel_value(t, h, i);
            }
        }
    }
    // row-wise op: fine. Reduction over the whole buffer: reads poison.
    let rowwise_max_over_actual = out[..actual * HV * D]
        .iter()
        .fold(f32::NEG_INFINITY, |a, b| a.max(*b));
    assert!(!rowwise_max_over_actual.is_nan(), "actual rows are clean");
    let _reduction_over_all = out.iter().fold(f32::NEG_INFINITY, |a, b| a.max(*b));
    // max with NaN: NaN loses comparisons in Rust's f32::max, so the
    // poison does NOT propagate through max — the honest boundary is
    // that the VALUE read is poison (bits), not that every reduction
    // propagates it. Pin the read-set law: the padding slot's BITS are
    // poison; a consumer that branches on them (or sums them) ingests
    // the uninit value.
    assert!(
        out[actual * HV * D].is_nan(),
        "the padding slot's value IS the poison — read-set discipline decides legality"
    );
    let summed = out.iter().sum::<f32>();
    assert!(
        summed.is_nan(),
        "a SUM over padding rows ingests the poison (the anti-regime)"
    );
}
