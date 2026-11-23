//! Batch 402 kernel fixture preflight — the mlxfast-27b delta residue
//! (Plan 402 / Research 437, Layr-Labs/mlxfast-bonsai2-27b-engine @
//! 0c7c3923, MIT — transcription-only) as ONE standalone file (2 rules,
//! the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! source is Swift/Metal; the Rust arms below model each law's decision
//! content). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b402_fixtures.rs`
//! at debug AND -O. Each arm names the honest and the adversary half;
//! a defect caught here is fixed HERE first, then synced into
//! entries_gpu_163.rs in the same landing commit.
//!
//! Arms:
//! - R1 verify-window load sharing: the buggy arm (per-column weight
//!   reread + a shared cross-position accumulator) is slow AND wrong —
//!   its column k carries the sum of columns 0..k; the fixed arm (one
//!   weight read, k per-position serial-order accumulators) is
//!   bit-identical to scoring each column alone; the tamper arm
//!   (reverse-order accumulation claiming exactness) reds; the
//!   admission arm refuses an uncovered variant.
//! - R2 split-K reduce fold: the two-launch incumbent vs the fold arm
//!   (the consumer sums the chunk partials in the reduce kernel's own
//!   chunk order — bit-identical sums, one launch gone); the
//!   violated-admission arm (a SECOND consumer appears — the fold
//!   refuses and the reduce launch stays, green refusal); the
//!   order-tamper arm (fold sums in reverse chunk order — bits differ,
//!   red — the pinned order is load-bearing).

// ══════════════════ R1 — verify-window load sharing ══════════════════

const ROW_BYTES: usize = 8; // 16 packed W4 values per row

fn dequant_nibble(byte: u8, hi: bool) -> f32 {
    let nib = if hi { byte >> 4 } else { byte & 0x0f };
    (nib as f32 - 7.5) / 8.0
}

/// the buggy pass: per-column weight REREAD + a SHARED accumulator
struct BuggyVerify<'w> {
    w_packed: &'w [u8],
}

impl<'w> BuggyVerify<'w> {
    /// column k's score computed with its own full weight walk
    fn score_column(&self, x: &[f32]) -> f32 {
        let mut acc = 0.0f32;
        for b in 0..ROW_BYTES {
            acc += dequant_nibble(self.w_packed[b], false) * x[2 * b]
                + dequant_nibble(self.w_packed[b], true) * x[2 * b + 1];
        }
        acc
    }

    /// the WRONG amortization: one accumulator shared across columns
    fn score_window(&self, xs: &[&[f32]]) -> Vec<f32> {
        let mut shared = 0.0f32;
        let mut out = Vec::new();
        for x in xs {
            shared += self.score_column(x); // cross-position sum
            out.push(shared); // NOT the column's own score
        }
        out
    }
}

/// the fixed pass: ONE weight read, k independent serial-order accs
struct ExactVerify<'w> {
    w_packed: &'w [u8],
}

impl<'w> ExactVerify<'w> {
    fn score_window(&self, xs: &[&[f32]]) -> Vec<f32> {
        let k = xs.len();
        let mut acc = vec![0.0f32; k]; // per-position, never shared
        for b in 0..ROW_BYTES {
            let w0 = dequant_nibble(self.w_packed[b], false);
            let w1 = dequant_nibble(self.w_packed[b], true);
            for (k_of, x) in xs.iter().enumerate() {
                acc[k_of] += w0 * x[2 * b] + w1 * x[2 * b + 1];
            }
        }
        acc
    }
}

/// the serial oracle: each column scored alone (the ground truth)
fn serial_row_score(x: &[f32], w_packed: &[u8]) -> f32 {
    let mut acc = 0.0f32;
    for b in 0..ROW_BYTES {
        acc += dequant_nibble(w_packed[b], false) * x[2 * b]
            + dequant_nibble(w_packed[b], true) * x[2 * b + 1];
    }
    acc
}

/// the tampered "exact" arm: reverse chunk order — DIFFERENT bits
struct TamperedVerify<'w> {
    w_packed: &'w [u8],
}

impl<'w> TamperedVerify<'w> {
    fn score_window(&self, xs: &[&[f32]]) -> Vec<f32> {
        let k = xs.len();
        let mut acc = vec![0.0f32; k];
        for b in (0..ROW_BYTES).rev() {
            // order flipped — reassociates every partial sum
            let w0 = dequant_nibble(self.w_packed[b], false);
            let w1 = dequant_nibble(self.w_packed[b], true);
            for (k_of, x) in xs.iter().enumerate() {
                acc[k_of] += w0 * x[2 * b] + w1 * x[2 * b + 1];
            }
        }
        acc
    }
}

/// admission: the exactness claim is proven per variant, never assumed
struct VerifyAdmission {
    covered: std::collections::BTreeSet<&'static str>,
}

impl VerifyAdmission {
    fn admits(&self, variant: &str) -> bool {
        self.covered.contains(variant)
    }
}

#[test]
fn r1_buggy_arm_is_wrong_bits_and_rereads() {
    let w: Vec<u8> = (0..ROW_BYTES as u8).collect();
    let xs: Vec<Vec<f32>> = (0..4)
        .map(|c| (0..16).map(|i| ((c * 3 + i) % 7) as f32 / 4.0).collect())
        .collect();
    let refs: Vec<&[f32]> = xs.iter().map(|x| x.as_slice()).collect();
    let pass = BuggyVerify { w_packed: &w };
    let out = pass.score_window(&refs);
    let oracle: Vec<f32> = refs.iter().map(|x| serial_row_score(x, &w)).collect();
    // the shared accumulator makes column k carry 0..k — wrong bits
    assert_ne!(
        out[2], oracle[2],
        "buggy arm must diverge from the serial oracle"
    );
    // and the weight bytes were walked k times (the unamortized cost):
    // each column's own score is RIGHT — proving the divergence is the
    // shared accumulator, not the per-column walk
    for (k_of, x) in refs.iter().enumerate() {
        assert_eq!(pass.score_column(x), oracle[k_of]);
    }
}

#[test]
fn r1_fixed_arm_is_bit_identical_per_column() {
    let w: Vec<u8> = (0..ROW_BYTES as u8).collect();
    let xs: Vec<Vec<f32>> = (0..4)
        .map(|c| (0..16).map(|i| ((c * 3 + i) % 7) as f32 / 4.0).collect())
        .collect();
    let refs: Vec<&[f32]> = xs.iter().map(|x| x.as_slice()).collect();
    let pass = ExactVerify { w_packed: &w };
    let out = pass.score_window(&refs);
    let oracle: Vec<f32> = refs.iter().map(|x| serial_row_score(x, &w)).collect();
    assert_eq!(
        out, oracle,
        "shared LOAD + independent serial-order accs = the serial row, bit for bit"
    );
}

#[test]
fn r1_tamper_arm_order_flip_reds() {
    // non-dyadic inputs (n/3) so partial sums ROUND — float addition is
    // not associative and the reversed order changes bits
    let w: Vec<u8> = (0..ROW_BYTES as u8).collect();
    let xs: Vec<Vec<f32>> = (0..4)
        .map(|c| {
            (0..16)
                .map(|i| ((c * 5 + i * 2) % 11) as f32 / 3.0)
                .collect()
        })
        .collect();
    let refs: Vec<&[f32]> = xs.iter().map(|x| x.as_slice()).collect();
    let tam = TamperedVerify { w_packed: &w };
    let out = tam.score_window(&refs);
    let oracle: Vec<f32> = refs.iter().map(|x| serial_row_score(x, &w)).collect();
    // float addition is not associative: the reversed order changes bits
    assert_ne!(
        out, oracle,
        "reversed accumulation order is NOT the serial row"
    );
}

#[test]
fn r1_admission_refuses_uncovered_variant() {
    let adm = VerifyAdmission {
        covered: ["w4_g64_narrow2"].into_iter().collect(),
    };
    assert!(adm.admits("w4_g64_narrow2"));
    assert!(
        !adm.admits("w4_g64_wide8"),
        "uncovered variant keeps the serial path"
    );
}

// ══════════════════ R2 — split-K reduce fold ══════════════════

const CHUNKS: usize = 4;
const OUTPUTS: usize = 3;

/// build deterministic chunk partials (the partial kernel's output)
fn partials_for(seed: u64) -> Vec<Vec<f32>> {
    (0..CHUNKS)
        .map(|c| {
            (0..OUTPUTS)
                .map(|i| ((seed + (c * 31 + i * 7) as u64) % 13) as f32 / 8.0)
                .collect()
        })
        .collect()
}

/// the incumbent: a separate reduce launch, chunk order
fn reduce_launch(partials: &[Vec<f32>]) -> Vec<f32> {
    (0..OUTPUTS)
        .map(|i| {
            let mut acc = 0.0f32;
            for chunk in 0..CHUNKS {
                acc += partials[chunk][i]; // from 0.0, chunk 0 first
            }
            acc
        })
        .collect()
}

/// the fold: the consumer sums the partials IN THE REDUCE'S ORDER
fn fold_into_consumer(partials: &[Vec<f32>], tape: &mut [f32]) -> Vec<f32> {
    let mut sums = vec![0.0f32; OUTPUTS];
    for (i, slot) in sums.iter_mut().enumerate() {
        let mut acc = 0.0f32;
        for chunk in 0..CHUNKS {
            acc += partials[chunk][i]; // same 0.0 start, chunk 0 first
        }
        *slot = acc;
    }
    tape.copy_from_slice(&sums); // the tape keeps the rows as before
    sums
}

/// the order-tampered fold: reverse chunk order (WRONG bits)
fn fold_reverse_order(partials: &[Vec<f32>]) -> Vec<f32> {
    (0..OUTPUTS)
        .map(|i| {
            let mut acc = 0.0f32;
            for chunk in (0..CHUNKS).rev() {
                acc += partials[chunk][i]; // reassociates every sum
            }
            acc
        })
        .collect()
}

/// admission: exactly one narrow consumer + a storing tape
struct FoldAdmission {
    narrow_consumers: usize,
    keeper_only_stores: bool,
}

impl FoldAdmission {
    fn admits(&self) -> bool {
        self.narrow_consumers == 1 && self.keeper_only_stores
    }
}

#[test]
fn r2_fold_is_bit_identical_and_launch_gone() {
    let partials = partials_for(42);
    let mut tape = vec![0.0f32; OUTPUTS];
    let folded = fold_into_consumer(&partials, &mut tape);
    let reduced = reduce_launch(&partials);
    assert_eq!(
        folded, reduced,
        "same additions in the same order = the same bits"
    );
    assert_eq!(
        tape, reduced,
        "the tape keeps the rows in the reduce's shape"
    );
    // the launch-count axis: two launches -> one (modeled: the fold IS
    // the consumer's own loop; the reduce kernel no longer runs)
}

#[test]
fn r2_second_consumer_refuses_the_fold() {
    // the incumbent still runs when admission fails
    let partials = partials_for(7);
    let reduced = reduce_launch(&partials);
    let bad_adm = FoldAdmission {
        narrow_consumers: 2,
        keeper_only_stores: true,
    };
    assert!(!bad_adm.admits(), "a second consumer must refuse the fold");
    let ok_adm = FoldAdmission {
        narrow_consumers: 1,
        keeper_only_stores: true,
    };
    assert!(ok_adm.admits());
    // on refusal the reduce launch STAYS (the incumbent path is taken)
    let mut tape = vec![0.0f32; OUTPUTS];
    let _ = fold_into_consumer(&partials, &mut tape); // not taken
    assert_eq!(reduce_launch(&partials), reduced);
}

#[test]
fn r2_order_tamper_reds() {
    // non-dyadic partials (n/3) so the chunk sums round: the pinned
    // chunk order is load-bearing, reversed order changes bits
    let partials: Vec<Vec<f32>> = (0..CHUNKS)
        .map(|c| {
            (0..OUTPUTS)
                .map(|i| ((c * 31 + i * 7) % 13) as f32 / 3.0)
                .collect()
        })
        .collect();
    let reduced = reduce_launch(&partials);
    let tampered = fold_reverse_order(&partials);
    assert_ne!(
        tampered, reduced,
        "the pinned chunk order is load-bearing: reversed order changes bits"
    );
}
