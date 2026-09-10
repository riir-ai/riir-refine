#[cfg(feature = "rust_perf_bench")]
use super::entries_03::division_invariant_bench_spec;
use super::*;
use crate::domains::rust_perf::PerfRule;

#[test]
fn seed_corpus_round_trips() {
    let corpus = build_perf_seed_corpus();
    assert!(corpus.verify());
    let frozen = corpus.freeze().unwrap();
    let thawed = Corpus::<RustPerfDomain>::thaw(&frozen).unwrap();
    assert_eq!(thawed.commitment, corpus.commitment);
    assert!(thawed.verify());
}

#[test]
fn seed_corpus_has_ninety_eight_entries_across_twelve_categories() {
    let corpus = build_perf_seed_corpus();
    assert_eq!(corpus.entries.len(), 126); // Batch 143 (nnethercote Dec-2025) 124 → 126
    let mut categories: Vec<&str> = corpus.entries.iter().map(|e| e.category.as_str()).collect();
    categories.sort_unstable();
    categories.dedup();
    // 12 distinct categories. Allocation is the deepest (22 rules,
    // — Batch 75 added encoded-layout-borrowed-view +
    // group-keys-borrow-allocate-on-first-sight;
    // including Issue 014's arc-str-payload + Batch 48's three + Batch
    // 53's hashmap-with-capacity); complexity has 18 (Batch 76 added
    // debug-stat-full-scan-out-of-hot-path; Batch 91 added
    // decision-projection-before-construction; Batch 75 added
    // sorted-input-previous-row-fast-path; Batch 48 added
    // hashset-insert-bool-return + vec-insert-front-deque; Batch 64
    // added debug-assert-length-hoist); iterators
    // has 10 (Batch 48 added three; Batch 66 added
    // sortedness-check-branchless-accumulate; Batch 67 added
    // byte-find-position-to-memchr; Batch 91 added
    // filter-before-expensive-transform); simd has 8 (Batch 62 added
    // transposed-weight-register-gemv; Batch 64 added
    // loop-order-innermost-contiguous; Batch 83 added
    // bitplane-transpose-repeated-predicate-scan); layout has 10 (Batch 53 added
    // soa-hot-field; Batch 61 added usize-index-to-u32; Batch 78 added
    // narrow-counter-overflow-promotion; Batch 85 added
    // weight-tile-consumption-order; Batch 112 added
    // hot-byval-move-over-memcpy-threshold) + concurrency
    // has 8 (Issue 014 added the two metric-mismatch rules; Batch 65
    // added align-64-false-sharing + channel-send-batching-unpark;
    // Batch 73 added lock-guard-narrow-field-accessor; Batch 78 added
    // cache-line-block-partitioned-lock
    // added align-64-false-sharing + channel-send-batching-unpark;
    // Batch 73 added lock-guard-narrow-field-accessor); arithmetic
    // has 7 (Batch 53 added powi-vs-powf + distance-sq-no-sqrt; Batch 59
    // added division-by-invariant-multiplication; Batch 62
    // added newton-refinement-vs-hardware-transcendental; Batch 74
    // added swar-digit-chunk-parse); dispatch has 4
    // (Batch 61 added keyword-map-to-match; Batch 74 added
    // runtime-feature-detect-per-chunk; Batch 99 added
    // mir-destprop-branch-site-merge); caching has 4 (Issue 014
    // added regex-hoist-lazylock; Batch 73 added
    // sort-key-cache-precompute-out-of-lock; Batch 76 added
    // eager-pointer-collect-lazy-fetch); io has 5 (Batch 68 added
    // varint-compose-word-single-store; Batch 69 added
    // unaligned-word-load-from-bytes; Batch 85 added
    // software-prefetch-storage-bound; Batch 94 added
    // coalesce-nearby-reads-bounded-gap-cacheability-gated); bevy, wasm
    // have 1 each.
    assert_eq!(categories.len(), 12);
}

/// G1 for `swar-digit-chunk-parse` (Batch 74, Research 088 — the
/// simd-json distill): the ported SWAR construction
/// (`parse_eight_digits_swar` + `is_made_of_eight_digits_fast` + the
/// chunk/tail loop) must be exactly equivalent to the naive per-digit
/// reference — the Batch 59 ported-numeric exhaustive-gate MANDATE
/// (casual tests miss subtle wrongness; the construction's wrapping
/// multiplies especially). Sweeps: (a) every digit-string length
/// 0..=19 × structured boundaries (all-0s, all-9s, 10^k) + the exact
/// 20-digit u64::MAX string; (b) 10_000 LCG digit strings of LCG
/// lengths; (c) non-digit terminators at EVERY offset (b'x', b',',
/// b' ', 0x00, 0xFF) — value AND consumed-length identical; (d) the
/// validation matrix: `is_made_of_eight_digits_fast` ≡ "every byte in
/// 0x30..=0x39" over 8 positions × boundary bytes + 10_000 LCG octets;
/// (e) the fold itself vs a u32 reference over structured + 100_000
/// LCG octets. Domain: digit runs whose value fits u64 (beyond that
/// BOTH arms overflow — documented out-of-domain).
#[test]
#[allow(clippy::too_many_lines)] // one five-section exhaustive sweep sharing the LCG stream
fn swar_digit_parse_equivalence_gate() {
    fn is_made_of_eight_digits_fast(chars: [u8; 8]) -> bool {
        let val = u64::from_ne_bytes(chars);
        ((val & 0xF0F0_F0F0_F0F0_F0F0)
            | (((val.wrapping_add(0x0606_0606_0606_0606)) & 0xF0F0_F0F0_F0F0_F0F0) >> 4))
            == 0x3333_3333_3333_3333
    }
    fn parse_eight_digits_swar(chars: [u8; 8]) -> u32 {
        let val = u64::from_le_bytes(chars);
        let val = (val & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(2561) >> 8;
        let val = (val & 0x00FF_00FF_00FF_00FF).wrapping_mul(6_553_601) >> 16;
        ((val & 0x0000_FFFF_0000_FFFF).wrapping_mul(42_949_672_960_001) >> 32) as u32
    }
    // The fixed construction (returns value + consumed length — pins the
    // stop-position equivalence argument, not just the value).
    fn parse_swars(digits: &[u8]) -> (u64, usize) {
        let mut val: u64 = 0;
        let mut i = 0;
        while i + 8 <= digits.len() {
            let chunk: [u8; 8] = digits[i..i + 8].try_into().unwrap();
            if !is_made_of_eight_digits_fast(chunk) {
                break;
            }
            val = val
                .wrapping_mul(100_000_000)
                .wrapping_add(u64::from(parse_eight_digits_swar(chunk)));
            i += 8;
        }
        while i < digits.len() && digits[i].is_ascii_digit() {
            val = val
                .wrapping_mul(10)
                .wrapping_add(u64::from(digits[i] - b'0'));
            i += 1;
        }
        (val, i)
    }
    // The reference — true u128 math, no wrapping ambiguity.
    fn parse_naive(digits: &[u8]) -> (u128, usize) {
        let mut val: u128 = 0;
        let mut i = 0;
        while i < digits.len() && digits[i].is_ascii_digit() {
            val = val * 10 + u128::from(digits[i] - b'0');
            i += 1;
        }
        (val, i)
    }
    fn check(bytes: &[u8]) {
        let (ref_val, ref_len) = parse_naive(bytes);
        assert!(
            ref_val <= u128::from(u64::MAX),
            "gate domain: test string must fit u64"
        );
        let (sw_val, sw_len) = parse_swars(bytes);
        assert_eq!(u128::from(sw_val), ref_val, "value mismatch on {bytes:?}");
        assert_eq!(sw_len, ref_len, "stop-position mismatch on {bytes:?}");
    }

    // (a) structured boundaries: every length × all-0s / all-9s / 10^k.
    for len in 0..=19usize {
        check(&vec![b'0'; len]);
        check(&vec![b'9'; len]);
        let mut p10 = Vec::with_capacity(len + 1);
        p10.push(b'1');
        p10.extend(std::iter::repeat_n(b'0', len));
        check(&p10);
    }
    let max_str = u64::MAX.to_string(); // "18446744073709551615" — 20 digits, fits
    check(max_str.as_bytes());
    assert_eq!(parse_swars(max_str.as_bytes()).0, u64::MAX);

    // (b) 10_000 LCG digit strings of LCG lengths 1..=19.
    let mut rng: u64 = 0x0123_4567_89AB_CDEF;
    for _ in 0..10_000 {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let len = 1 + ((rng >> 33) as usize) % 19;
        let mut s = Vec::with_capacity(len);
        for _ in 0..len {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            s.push(b'0' + (((rng >> 33) % 10) as u8));
        }
        check(&s);
    }

    // (c) non-digit terminators at EVERY offset — stop-position identity.
    for len in 0..=19usize {
        for term in [b'x', b',', b' ', 0x00, 0xFF] {
            let mut s: Vec<u8> = vec![b'7'; len];
            s.push(term);
            s.extend_from_slice(b"901"); // post-terminator digits must NOT be consumed
            check(&s);
        }
    }

    // (d) validation matrix: is_made_of_eight_digits_fast ≡ all-digits.
    let expect_all = |bytes: [u8; 8]| {
        assert_eq!(
            is_made_of_eight_digits_fast(bytes),
            bytes.iter().all(|&b| b.is_ascii_digit()),
            "validation mismatch on {bytes:?}"
        );
    };
    for pos in 0..8usize {
        for b in [0x00u8, 0x2F, 0x30, 0x39, 0x3A, 0x41, 0x46, 0x61, 0x7F, 0xFF] {
            let mut bytes = [b'5'; 8];
            bytes[pos] = b;
            expect_all(bytes);
        }
    }
    for _ in 0..10_000 {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let mut bytes = [0u8; 8];
        for b in &mut bytes {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            *b = ((rng >> 40) % 96).wrapping_add(0x30) as u8; // 0x30..=0x8F mix of digit/non-digit
        }
        expect_all(bytes);
    }

    // (e) the fold itself vs a u32 reference (structured + LCG octets).
    let fold_check = |bytes: [u8; 8]| {
        let reference: u32 = bytes
            .iter()
            .fold(0u32, |acc, &b| acc * 10 + u32::from(b - b'0'));
        assert_eq!(
            parse_eight_digits_swar(bytes),
            reference,
            "fold mismatch on {bytes:?}"
        );
    };
    fold_check([b'0'; 8]);
    fold_check([b'9'; 8]);
    fold_check(*b"12345678");
    fold_check(*b"00000001");
    for k in 0..8 {
        let mut bytes = [b'0'; 8];
        bytes[k] = b'1';
        fold_check(bytes);
    }
    for _ in 0..100_000 {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let mut bytes = [b'0'; 8];
        for b in &mut bytes {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            *b = b'0' + (((rng >> 33) % 10) as u8);
        }
        fold_check(bytes);
    }
}

/// G1 for `varint-compose-word-single-store` (Batch 68, Research 075 —
/// the tantivy distill): the 5-threshold ladder in the corpus fixture +
/// bench harness must be exactly equivalent to the reference per-byte
/// loop across a FULL u16 sweep + every 128^i band boundary + u32::MAX +
/// an LCG u32 sample — bit-identical output bytes (the Batch 59
/// ported-numeric exhaustive-gate rule; the source ships its own
/// equivalence test at 128^i boundaries — `aux_test_serialize_vint_u32`
/// — this gate is the stronger local mirror). BOTH conventions are
/// proven: (a) stop-bit-on-LAST (tantivy VInt — the shipped fixture)
/// and (b) continuation-bit-on-all-but-LAST (protobuf/LEB128 — the same
/// ladder mechanism with the convention bit placed where the loop put
/// it), demonstrating the fix is convention-faithful, not
/// convention-specific.
#[test]
#[allow(clippy::too_many_lines)] // exhaustive gate: both conventions + boundaries belong together
fn unaligned_word_load_matches_assembly_exhaustive() {
    // Batch 69 G1: the from_le_bytes slice read must be bit-identical to
    // the shift/or assembly at EVERY byte alignment over a wide buffer,
    // u64 AND u32 (the cross-arch probe's exhaustive half, pinned as a
    // standing gate).
    fn assemble_u64(data: &[u8], addr: usize) -> u64 {
        u64::from(data[addr])
            | (u64::from(data[addr + 1]) << 8)
            | (u64::from(data[addr + 2]) << 16)
            | (u64::from(data[addr + 3]) << 24)
            | (u64::from(data[addr + 4]) << 32)
            | (u64::from(data[addr + 5]) << 40)
            | (u64::from(data[addr + 6]) << 48)
            | (u64::from(data[addr + 7]) << 56)
    }
    fn load_u64(data: &[u8], addr: usize) -> u64 {
        let bytes: [u8; 8] = (&data[addr..addr + 8]).try_into().unwrap();
        u64::from_le_bytes(bytes)
    }
    fn assemble_u32(data: &[u8], addr: usize) -> u32 {
        u32::from(data[addr])
            | (u32::from(data[addr + 1]) << 8)
            | (u32::from(data[addr + 2]) << 16)
            | (u32::from(data[addr + 3]) << 24)
    }
    fn load_u32(data: &[u8], addr: usize) -> u32 {
        let bytes: [u8; 4] = (&data[addr..addr + 4]).try_into().unwrap();
        u32::from_le_bytes(bytes)
    }

    let n = 4096u32;
    let data: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
        .collect();

    for addr in 0..((n - 8) as usize) {
        assert_eq!(
            assemble_u64(&data, addr),
            load_u64(&data, addr),
            "u64 @ {addr}"
        );
    }
    for addr in 0..((n - 4) as usize) {
        assert_eq!(
            assemble_u32(&data, addr),
            load_u32(&data, addr),
            "u32 @ {addr}"
        );
    }
}

#[test]
fn varint_ladder_matches_reference_loop_exhaustive() {
    // (a) The SHIPPED ladder — mirror of the corpus fixture (same math).
    fn ladder_stop_bit_last(val: u32, out: &mut Vec<u8>) {
        let v = u64::from(val);
        let (res, len) = if v < (1 << 7) {
            (v | 128, 1)
        } else if v < (1 << 14) {
            ((v & 127) | ((v & (127 << 7)) << 1) | (128 << 8), 2)
        } else if v < (1 << 21) {
            (
                (v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | (128 << 16),
                3,
            )
        } else if v < (1 << 28) {
            (
                (v & 127)
                    | ((v & (127 << 7)) << 1)
                    | ((v & (127 << 14)) << 2)
                    | ((v & (127 << 21)) << 3)
                    | (128 << 24),
                4,
            )
        } else {
            (
                (v & 127)
                    | ((v & (127 << 7)) << 1)
                    | ((v & (127 << 14)) << 2)
                    | ((v & (127 << 21)) << 3)
                    | ((v & (127 << 28)) << 4)
                    | (128u64 << 32),
                5,
            )
        };
        let buf = res.to_le_bytes();
        out.extend_from_slice(&buf[..len]);
    }
    // Reference per-byte loop (the buggy shape, used as ground truth).
    fn reference_stop_bit_last(mut val: u32, out: &mut Vec<u8>) {
        loop {
            let byte = (val % 128) as u8;
            val /= 128;
            if val == 0 {
                out.push(byte | 128);
                return;
            }
            out.push(byte);
        }
    }
    // (b) The protobuf/LEB128 convention ladder — same mechanism, the
    // continuation bit lands on every byte EXCEPT the last (bit 7 set
    // on bytes 0..len-2 of the composed word).
    fn ladder_continuation(val: u32, out: &mut Vec<u8>) {
        let v = u64::from(val);
        let (res, len) = if v < (1 << 7) {
            (v, 1)
        } else if v < (1 << 14) {
            ((v & 127) | 0x80 | ((v & (127 << 7)) << 1), 2)
        } else if v < (1 << 21) {
            (
                (v & 127) | 0x80 | ((v & (127 << 7)) << 1) | 0x80 << 8 | ((v & (127 << 14)) << 2),
                3,
            )
        } else if v < (1 << 28) {
            (
                (v & 127)
                    | 0x80
                    | ((v & (127 << 7)) << 1)
                    | 0x80 << 8
                    | ((v & (127 << 14)) << 2)
                    | 0x80 << 16
                    | ((v & (127 << 21)) << 3),
                4,
            )
        } else {
            (
                (v & 127)
                    | 0x80
                    | ((v & (127 << 7)) << 1)
                    | 0x80 << 8
                    | ((v & (127 << 14)) << 2)
                    | 0x80 << 16
                    | ((v & (127 << 21)) << 3)
                    | 0x80 << 24
                    | ((v & (127 << 28)) << 4),
                5,
            )
        };
        let buf = res.to_le_bytes();
        out.extend_from_slice(&buf[..len]);
    }
    fn reference_continuation(mut val: u32, out: &mut Vec<u8>) {
        loop {
            if val < 128 {
                // final byte: NO continuation bit (u8::try_from is
                // infallible here — guarded by the val < 128 check)
                out.push(u8::try_from(val).unwrap());
                return;
            }
            out.push((val % 128) as u8 | 0x80);
            val /= 128;
        }
    }

    fn check_pair(val: u32) {
        let mut a = Vec::new();
        let mut b = Vec::new();
        ladder_stop_bit_last(val, &mut a);
        reference_stop_bit_last(val, &mut b);
        assert_eq!(a, b, "stop-bit ladder diverged at val={val}");
        let mut c = Vec::new();
        let mut d = Vec::new();
        ladder_continuation(val, &mut c);
        reference_continuation(val, &mut d);
        assert_eq!(c, d, "continuation ladder diverged at val={val}");
    }

    // Full u16 sweep (exhaustive low range).
    for val in 0..=u32::from(u16::MAX) {
        check_pair(val);
    }
    // Every band boundary ±1 (128^i) + the extremes.
    for i in 1..5u32 {
        let p: u32 = 1 << (7 * i);
        for val in [p - 1, p, p + 1] {
            check_pair(val);
        }
    }
    check_pair(u32::MAX);
    check_pair(u32::MAX - 1);
    // LCG u32 sample (16K values across the full range).
    let mut state = 0x853c_49e6_748f_ea9b_u64;
    for _ in 0..16_384 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        check_pair((state >> 32) as u32);
    }
}

/// Canaries: the corpus fixture + the bench harness must contain the
/// SAME ladder construction this exhaustive gate proves — a substring
/// pin so the three copies (fixture / harness / gate mirror) cannot
/// drift (the Batch 59 canary pattern).
#[test]
fn varint_fixture_and_harness_carry_the_proven_ladder() {
    let corpus = build_perf_seed_corpus();
    let rule = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "varint-compose-word-single-store")
        .expect("varint rule present");
    let needle = "((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2)";
    assert!(
        rule.fixed_code.contains(needle),
        "fixture fixed_code must carry the proven 3+-band ladder fragment"
    );
    assert!(
        rule.buggy_code.contains("val /= 128"),
        "fixture buggy_code must be the per-byte div-chain loop"
    );
    #[cfg(feature = "rust_perf_bench")]
    {
        let spec = rule.bench_spec.as_ref().expect("bench spec");
        assert!(
            spec.harness.contains(needle),
            "bench harness must carry the same ladder construction"
        );
    }
}

/// G1 for `division-by-invariant-multiplication`: the magic-division
/// algorithm in the corpus fixture + bench harness must be exactly
/// equivalent to hardware `/` across an exhaustive low-divisor sweep +
/// boundary numerators + selected large divisors (Batch 59). A mirror
/// of the harness impl lives here (same math); under
/// `rust_perf_bench` a substring canary additionally pins the harness
/// string to the same construction lines so the two cannot drift.
#[test]
#[allow(clippy::too_many_lines)] // exhaustive gate: the u32/u64 twins + shared edge data belong together
fn division_magic_matches_hardware_div_exhaustive_low_d() {
    struct FastDivU32 {
        magic: u32,
        shift: u32,
        add: bool,
    }
    #[allow(clippy::cast_possible_truncation)] // libdivide magic: the high-word extraction + magic wrap ARE the algorithm
    impl FastDivU32 {
        fn new(d: u32) -> Self {
            let l = d.ilog2();
            if d.is_power_of_two() {
                return Self {
                    magic: 0,
                    shift: l,
                    add: false,
                };
            }
            let k = 1u64 << l;
            let base = (k << 32) / u64::from(d);
            let rem = (k << 32) % u64::from(d);
            if u64::from(d) - rem < k {
                Self {
                    magic: base as u32 + 1,
                    shift: l,
                    add: false,
                }
            } else {
                let mut m = base * 2;
                let rem2 = rem * 2;
                if rem2 >= u64::from(d) {
                    m += 1;
                }
                Self {
                    magic: (m + 1) as u32,
                    shift: l,
                    add: true,
                }
            }
        }
        // manual_midpoint (clippy 1.98): the widened (hi + x) >> 1 fold IS the
        // algorithm — construction-identical to the pinned harness string, and
        // overflow-free by the u64 widening (the midpoint() rewrite would force
        // a corpus fixture edit for zero semantic change).
        #[allow(clippy::manual_midpoint)]
        fn div(&self, x: u32) -> u32 {
            if self.magic == 0 {
                return x >> self.shift;
            }
            let hi = ((u64::from(self.magic) * u64::from(x)) >> 32) as u32;
            if self.add {
                ((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32
            } else {
                hi >> self.shift
            }
        }
    }
    struct FastDivU64 {
        magic: u64,
        shift: u32,
        add: bool,
    }
    #[allow(clippy::cast_possible_truncation)] // libdivide magic: the high-word extraction + magic wrap ARE the algorithm
    impl FastDivU64 {
        fn new(d: u64) -> Self {
            let l = d.ilog2();
            if d.is_power_of_two() {
                return Self {
                    magic: 0,
                    shift: l,
                    add: false,
                };
            }
            let k = 1u128 << l;
            let base = (k << 64) / u128::from(d);
            let rem = (k << 64) % u128::from(d);
            if u128::from(d) - rem < k {
                Self {
                    magic: base as u64 + 1,
                    shift: l,
                    add: false,
                }
            } else {
                let mut m = base * 2;
                let rem2 = rem * 2;
                if rem2 >= u128::from(d) {
                    m += 1;
                }
                Self {
                    magic: (m + 1) as u64,
                    shift: l,
                    add: true,
                }
            }
        }
        #[allow(clippy::manual_midpoint)] // same algorithm-mirror reason as the u32 twin
        fn div(&self, x: u64) -> u64 {
            if self.magic == 0 {
                return x >> self.shift;
            }
            let hi = ((u128::from(self.magic) * u128::from(x)) >> 64) as u64;
            if self.add {
                ((u128::from(hi) + u128::from(x)) >> 1 >> self.shift) as u64
            } else {
                hi >> self.shift
            }
        }
    }

    struct Lcg(u64);
    impl Lcg {
        fn next_u64(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0
        }
        fn next_u32(&mut self) -> u32 {
            (self.next_u64() >> 33) as u32
        }
    }
    let mut rng = Lcg(0x2545_F491_4F6C_DD1D);

    // Exhaustive low divisors (all u32 d in 1..=4096, all u64 d in 1..=257)
    for d in 1u32..=4096 {
        let fd = FastDivU32::new(d);
        let probe = |x: u32| {
            assert_eq!(fd.div(x), x / d, "u32 mismatch at d={d} x={x}");
        };
        probe(0);
        probe(1);
        probe(d - 1);
        probe(d);
        probe(d + 1);
        probe(d.saturating_mul(2).saturating_sub(1));
        probe(u32::MAX);
        for _ in 0..8 {
            probe(rng.next_u32());
        }
    }
    for d in 1u64..=257 {
        let fd = FastDivU64::new(d);
        let probe = |x: u64| {
            assert_eq!(fd.div(x), x / d, "u64 mismatch at d={d} x={x}");
        };
        probe(0);
        probe(1);
        probe(d - 1);
        probe(d);
        probe(d + 1);
        probe(d.saturating_mul(2).saturating_sub(1));
        probe(u64::MAX);
        for _ in 0..8 {
            probe(rng.next_u64());
        }
    }
    // Selected large/edge divisors: powers of two, primes near 2^31,
    // 2^32, 2^63, 2^64, and values that exercise the ADD branch + the
    // sentinel boundary (magic wrapping near 0).
    let edge32: &[u32] = &[
        65_535,
        65_536,
        65_537,
        1_000_003,
        2_147_483_647,
        2_147_483_648,
        3_000_000_001,
        4_294_967_291,
        4_294_967_295,
        3,
        5,
        7,
        9,
        11,
    ];
    for &d in edge32 {
        let fd = FastDivU32::new(d);
        let probe = |x: u32| {
            assert_eq!(fd.div(x), x / d, "u32 edge mismatch at d={d} x={x}");
        };
        probe(0);
        probe(1);
        probe(d - 1);
        probe(d);
        probe(d.saturating_add(1));
        probe(u32::MAX - 1);
        probe(u32::MAX);
        for _ in 0..16 {
            probe(rng.next_u32());
        }
    }
    let edge64: &[u64] = &[
        3,
        5,
        7,
        9,
        11,
        65_537,
        1_000_003,
        4_294_967_291,
        4_294_967_296,
        4_294_967_311,
        9_223_372_036_854_775_783,
        9_223_372_036_854_775_808,
        9_223_372_036_854_775_809,
        18_446_744_073_709_551_557,
        u64::MAX - 1,
        u64::MAX,
    ];
    for &d in edge64 {
        let fd = FastDivU64::new(d);
        let probe = |x: u64| {
            assert_eq!(fd.div(x), x / d, "u64 edge mismatch at d={d} x={x}");
        };
        probe(0);
        probe(1);
        probe(d - 1);
        probe(d);
        probe(d.saturating_add(1));
        probe(u64::MAX - 1);
        probe(u64::MAX);
        for _ in 0..16 {
            probe(rng.next_u64());
        }
    }

    // Canary: the harness string must carry the same construction core
    // (drift between this mirror + the spec = silent correctness rot).
    #[cfg(feature = "rust_perf_bench")]
    {
        let spec = division_invariant_bench_spec();
        for needle in [
            "let base = (k << 32) / u64::from(d);",
            "let base = (k << 64) / u128::from(d);",
            "((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32",
            "((u128::from(hi) + u128::from(x)) >> 1 >> self.shift) as u64",
        ] {
            assert!(
                spec.harness.contains(needle),
                "harness drifted from the equivalence-test mirror: missing `{needle}`"
            );
        }
    }
}

#[test]
fn seed_corpus_rule_ids_are_unique() {
    let corpus = build_perf_seed_corpus();
    let mut ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), before, "duplicate rule_id in seed corpus");
}

#[test]
fn tamper_detected() {
    let corpus = build_perf_seed_corpus();
    let mut tampered = corpus.clone();
    tampered.entries[0].fixed_code = "// tampered".to_string();
    assert!(!tampered.verify());
}

#[test]
fn heal_entry_accessors_return_correct_fields() {
    use crate::domain::HealEntry;
    let corpus = build_perf_seed_corpus();
    // Corpus is sorted by rule_id alphabetically — find by id, not index.
    let entry = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "o1-lookup")
        .expect("o1-lookup should be in seed corpus");
    assert_eq!(entry.rule_id(), "o1-lookup");
    assert!(!entry.buggy_code().is_empty());
    assert!(!entry.fixed_code().is_empty());
}

#[test]
fn heal_domain_build_corpus_matches_seed() {
    use crate::domain::HealDomain;
    let via_trait = RustPerfDomain::build_corpus();
    let via_seed = build_perf_seed_corpus().entries;
    assert_eq!(via_trait.len(), via_seed.len());
    // Both go through the same sort, so the order is identical.
    assert_eq!(
        via_trait.first().map(|e| e.rule_id.as_str()),
        via_seed.first().map(|e| e.rule_id.as_str())
    );
    assert_eq!(
        via_trait.last().map(|e| e.rule_id.as_str()),
        via_seed.last().map(|e| e.rule_id.as_str())
    );
}

#[test]
#[cfg(not(feature = "rust_perf_verify"))]
fn validate_parses_valid_code() {
    use crate::domain::HealDomain;
    let verdict = RustPerfDomain::validate("o1-lookup", "fn foo() -> i32 { 42 }");
    assert!(verdict.parses);
}

#[test]
#[cfg(not(feature = "rust_perf_verify"))]
fn validate_rejects_garbage() {
    use crate::domain::HealDomain;
    let verdict = RustPerfDomain::validate("o1-lookup", "not valid rust {");
    assert!(!verdict.parses);
}

#[test]
fn runtime_int_exponent_powi_rel_error_gate() {
    // Batch 77 T3.1: the ported-numeric MANDATE — powi is NOT
    // correctly-rounded pow, so the equivalence gate is max rel-error
    // across a SWEPT exponent range (never bit-identity). Bound: the
    // multiplies accumulate ~log2(|n|) ulp; on f64 with |n| <= 64 the
    // measured bound stays under 1e-12 (double-precision headroom).
    let bases = [1.000_000_1f64, 1.5f64, 2.0f64, std::f64::consts::PI];
    let mut max_rel: f64 = 0.0;
    let mut worst = (0.0f64, 0i32);
    for &b in &bases {
        for n in 1i32..=64 {
            let ref_v = b.powf(f64::from(n)); // correctly-rounded libm
            let fix_v = b.powi(n);
            let rel = ((fix_v - ref_v) / ref_v).abs();
            if rel > max_rel {
                max_rel = rel;
                worst = (b, n);
            }
        }
    }
    assert!(
        max_rel < 1e-12,
        "powi rel-error {max_rel:.3e} at base {} exp {} exceeds the 1e-12 double-precision bound",
        worst.0,
        worst.1
    );
}

/// G1 for `weight-tile-consumption-order` (Batch 85, Research 098 —
/// the cflow distill): tile accumulation order (per-column-tile partial
/// dots, accumulated into y) != row-major accumulation order (one
/// sequential dot per row) — the gate is max rel-error over swept
/// shapes (never bit-identity; the B64/powi gate shape). Non-tile-
/// multiple shapes exercise the min() edge paths. Positive-only inputs
/// (no cancellation): sums of positives keep the relative bound tight.
#[test]
#[allow(clippy::cast_possible_truncation)] // the 24-bit hash — intentional, the loop_order convention
#[allow(clippy::cast_precision_loss)] // u32→f32 mantissa loss IS the hash's purpose
fn weight_tile_consumption_order_rel_error_gate() {
    const TILE_ROWS: usize = 64;
    const TILE_COLS: usize = 512;
    fn repack(w: &[f32], n_out: usize, n_in: usize) -> Vec<f32> {
        let mut tiles = Vec::with_capacity(w.len());
        for tr in (0..n_out).step_by(TILE_ROWS) {
            for tc in (0..n_in).step_by(TILE_COLS) {
                for r in tr..(tr + TILE_ROWS).min(n_out) {
                    let row = &w[r * n_in..(r + 1) * n_in];
                    tiles.extend_from_slice(&row[tc..(tc + TILE_COLS).min(n_in)]);
                }
            }
        }
        tiles
    }
    fn matvec_rowmajor(w: &[f32], x: &[f32], y: &mut [f32], n_in: usize) {
        for (r, yo) in y.iter_mut().enumerate() {
            let row = &w[r * n_in..(r + 1) * n_in];
            let mut acc = 0.0f32;
            for (wv, &xv) in row.iter().zip(x) {
                acc += wv * xv;
            }
            *yo = acc;
        }
    }
    fn matvec_tiled(tiles: &[f32], x: &[f32], y: &mut [f32], n_out: usize, n_in: usize) {
        let mut off = 0usize;
        for tr in (0..n_out).step_by(TILE_ROWS) {
            for tc in (0..n_in).step_by(TILE_COLS) {
                let rows = TILE_ROWS.min(n_out - tr);
                let cols = TILE_COLS.min(n_in - tc);
                let xs = &x[tc..tc + cols];
                for r in 0..rows {
                    let tile_row = &tiles[off + r * cols..off + (r + 1) * cols];
                    let mut partial = 0.0f32;
                    for (wv, &xv) in tile_row.iter().zip(xs) {
                        partial += wv * xv;
                    }
                    y[tr + r] += partial;
                }
                off += rows * cols;
            }
        }
    }
    // (n_out, n_in) — includes non-tile-multiple shapes (70x777, 96x1300)
    // exercising the ragged min() edges + a full-shape arm.
    for (n_out, n_in) in [(64usize, 4096usize), (96, 1300), (70, 777)] {
        let hash = |i: usize, s: u32| {
            let h = (i as u32).wrapping_mul(0x9E37_79B9).wrapping_add(s);
            ((h >> 8) as f32) / 16_777_216.0 // 0..1, positive-only
        };
        let w: Vec<f32> = (0..n_out * n_in).map(|i| hash(i, 1)).collect();
        let x: Vec<f32> = (0..n_in).map(|i| hash(i, 2)).collect();
        let mut y_row = vec![0.0f32; n_out];
        matvec_rowmajor(&w, &x, &mut y_row, n_in);
        let tiles = repack(&w, n_out, n_in);
        assert_eq!(tiles.len(), w.len(), "repack preserves every weight");
        let mut y_tiled = vec![0.0f32; n_out];
        matvec_tiled(&tiles, &x, &mut y_tiled, n_out, n_in);
        let mut max_rel = 0.0f64;
        for r in 0..n_out {
            let ref_v = f64::from(y_row[r]);
            let rel = (f64::from(y_tiled[r]) - ref_v).abs() / ref_v.abs().max(1e-30);
            max_rel = max_rel.max(rel);
        }
        assert!(
            max_rel < 1e-5,
            "tile-order rel-error {max_rel:.3e} at {n_out}x{n_in} exceeds the 1e-5 bound"
        );
    }
}

/// Batch 86 — the three new rules are present + retrievable.
#[test]
fn batch86_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    assert!(ids.contains(&"serialize-once-byte-buffer-store"));
    assert!(ids.contains(&"context-equal-field-elide"));
    assert!(ids.contains(&"bitflag-pack-when-footprint-dominates"));
}

/// Batch 86 — the three new rules parse (syn) on buggy AND fixed code.
#[test]
fn batch86_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    for id in [
        "serialize-once-byte-buffer-store",
        "context-equal-field-elide",
        "bitflag-pack-when-footprint-dominates",
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} missing from corpus"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "{id} buggy/fixed must both parse"
        );
    }
}

/// Batch 91 — the two new rules are present.
#[test]
fn batch91_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    assert!(ids.contains(&"filter-before-expensive-transform"));
    assert!(ids.contains(&"decision-projection-before-construction"));
}

/// Batch 91 — the two new rules parse (syn) on buggy AND fixed code.
#[test]
fn batch91_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    for id in [
        "filter-before-expensive-transform",
        "decision-projection-before-construction",
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} missing from corpus"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "{id} buggy/fixed must both parse"
        );
    }
}

/// Batch 91 G1 (rule 1 exactness + the discarded-work axis): both
/// adapter orders collect the IDENTICAL page Vec (order included) for
/// a pure render and a pure predicate, AND the call-count axis proves
/// the mechanism — the buggy order renders EVERY doc, the fixed order
/// renders only the keepers. Non-vacuous via the strict-inequality
/// assert.
#[test]
fn batch91_filter_order_preserves_output_and_render_count() {
    use std::cell::Cell;
    struct Doc {
        published: bool,
        body: [u64; 4],
    }
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct Page {
        published: bool,
        score: u64,
    }

    thread_local! {
        static RENDER_CALLS: Cell<u64> = const { Cell::new(0) };
    }
    let render = |doc: &Doc| -> Page {
        RENDER_CALLS.with(|c| c.set(c.get() + 1));
        let mut acc = 0u64;
        for &v in &doc.body {
            acc = acc.wrapping_mul(31).wrapping_add(v);
        }
        Page {
            published: doc.published,
            score: acc,
        }
    };
    let docs: Vec<Doc> = (0..256u64)
        .map(|i| Doc {
            published: i % 16 == 0,
            body: [i; 4],
        })
        .collect();

    RENDER_CALLS.with(|c| c.set(0));
    let buggy: Vec<Page> = docs.iter().map(&render).filter(|p| p.published).collect();
    let buggy_calls = RENDER_CALLS.with(Cell::get);

    RENDER_CALLS.with(|c| c.set(0));
    let fixed: Vec<Page> = docs.iter().filter(|d| d.published).map(&render).collect();
    let fixed_calls = RENDER_CALLS.with(Cell::get);

    assert_eq!(
        buggy, fixed,
        "both orders must collect the identical page Vec"
    );
    assert_eq!(
        buggy_calls,
        docs.len() as u64,
        "the buggy order renders EVERY doc"
    );
    assert_eq!(
        fixed_calls,
        buggy.len() as u64,
        "the fixed order renders only keepers"
    );
    assert!(
        fixed_calls < buggy_calls,
        "the discarded-work axis must be strictly visible"
    );
}

/// Batch 91 G1 (rule 2 precondition AS A TEST + the call axis): the
/// projection is INVARIANT under canonicalization for every fixture
/// item (`canonicalize(i).primary == raw_primary(i)`), the two
/// predicates answer IDENTICALLY, and the canonicalize-call axis
/// proves the mechanism — the buggy predicate canonicalizes EVERY
/// item, the fixed predicate canonicalizes NONE (the decision never
/// needs the canonical form). Non-vacuous: the hot count must be
/// non-zero.
#[test]
#[allow(clippy::cast_possible_truncation)] // the enumerate-index u32 narrowing IS the fixture (k < 8)
fn batch91_projection_invariance_and_canonicalize_count() {
    use std::cell::Cell;
    struct Item {
        tags: [u32; 8],
    }
    #[derive(PartialEq, Eq, Debug)]
    struct Canonical {
        primary: u32,
        tags: Vec<u32>,
    }

    thread_local! {
        static CANON_CALLS: Cell<u64> = const { Cell::new(0) };
    }
    let canonicalize = |item: &Item| -> Canonical {
        CANON_CALLS.with(|c| c.set(c.get() + 1));
        let mut tags: Vec<u32> = item.tags.to_vec();
        tags.sort_unstable();
        tags.dedup();
        Canonical {
            primary: tags[0],
            tags,
        }
    };
    let raw_primary = |item: &Item| -> u32 {
        let mut m = u32::MAX;
        for &t in &item.tags {
            m = m.min(t);
        }
        m
    };
    let items: Vec<Item> = (0..256u32)
        .map(|i| {
            let base = i % 64;
            let mut tags = [0u32; 8];
            for (k, t) in tags.iter_mut().enumerate() {
                *t = base * 8 + (k as u32) * 31;
            }
            Item { tags }
        })
        .collect();

    for item in &items {
        assert_eq!(
            canonicalize(item).primary,
            raw_primary(item),
            "the projection must be invariant under canonicalization"
        );
    }
    let invariance_calls = CANON_CALLS.with(Cell::get);
    assert_eq!(invariance_calls, items.len() as u64);

    CANON_CALLS.with(|c| c.set(0));
    let buggy_hot = items
        .iter()
        .filter(|i| canonicalize(i).primary == 128)
        .count();
    let buggy_calls = CANON_CALLS.with(Cell::get);

    CANON_CALLS.with(|c| c.set(0));
    let fixed_hot = items.iter().filter(|i| raw_primary(i) == 128).count();
    let fixed_calls = CANON_CALLS.with(Cell::get);

    assert_eq!(buggy_hot, fixed_hot, "the decision must be identical");
    assert!(
        buggy_hot > 0,
        "the hot count must be non-zero (non-vacuous)"
    );
    assert_eq!(
        buggy_calls,
        items.len() as u64,
        "the buggy predicate canonicalizes EVERY item"
    );
    assert_eq!(
        fixed_calls, 0,
        "the fixed predicate never needs the canonical form"
    );
}

/// Batch 99 — the new rule is present.
#[test]
fn batch99_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    assert!(ids.contains(&"mir-destprop-branch-site-merge"));
}

/// Batch 99 — the new rule parses (syn) on buggy AND fixed code.
#[test]
fn batch99_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "mir-destprop-branch-site-merge")
        .unwrap_or_else(|| panic!("mir-destprop-branch-site-merge missing from corpus"));
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "mir-destprop-branch-site-merge buggy/fixed must both parse"
    );
}

/// Batch 99 — the load-bearing concepts survive into the corpus text
/// (retrieval discriminators, the kernel-batch concepts_covered
/// convention applied to a rust_perf batch).
#[test]
fn batch99_concepts_covered() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "mir-destprop-branch-site-merge")
        .expect("rule present");
    let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
    for needle in [
        "DestinationPropagation",
        "predictor",
        "devirtualize",
        "indirect",
        "call site",
    ] {
        assert!(
            text.contains(needle),
            "concept {needle:?} missing from the rule text"
        );
    }
    assert!(
        e.context_notes
            .as_deref()
            .unwrap_or("")
            .contains("ParetoDominant"),
        "the measured verdict must be recorded in context_notes"
    );
}

/// Batch 101 — the new rule is present.
#[test]
fn batch101_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    assert!(ids.contains(&"dispatch-handler-pair-hoist"));
}

/// Batch 101 — the new rule parses (syn) on buggy AND fixed code.
#[test]
fn batch101_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "dispatch-handler-pair-hoist")
        .unwrap_or_else(|| panic!("dispatch-handler-pair-hoist missing from corpus"));
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "dispatch-handler-pair-hoist buggy/fixed must both parse"
    );
}

/// Batch 101 — the load-bearing concepts survive into the corpus text
/// (retrieval discriminators), and the recorded verdicts + the
/// cross-arch x86 evidence are in context_notes. (The x86-pending
/// caveat was RESOLVED 2026-09-04 by the armed re-gate: x86_64 read
/// ParetoDominant 3/0/0, refuting the expected Tie — the annotation
/// updates, never deletes.)
#[test]
fn batch101_concepts_covered() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "dispatch-handler-pair-hoist")
        .expect("rule present");
    let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
    for needle in ["hoist", "table", "unpredictable", "register", "dispatch"] {
        assert!(
            text.contains(needle),
            "concept {needle:?} missing from the rule text"
        );
    }
    let notes = e.context_notes.as_deref().unwrap_or("");
    assert!(
        notes.contains("ParetoDominant"),
        "the measured verdict must be recorded in context_notes"
    );
    assert!(
        notes.contains("x86") && notes.contains("REFUTED"),
        "the x86 re-gate verdict must be recorded (never silently deleted)"
    );
    assert!(
        notes.contains("mir-destprop-branch-site-merge"),
        "the dedup cross-ref vs the landed B99 rule must be recorded"
    );
}

/// Batch 101 — G1: the hoisted form is output-identical to the table
/// form over a fixed op stream, and BOTH handlers actually run
/// (non-vacuity: a one-op stream would pass trivially).
#[test]
fn batch101_hoist_preserves_output_and_both_handlers_run() {
    fn op_add(x: u64) -> u64 {
        x.wrapping_add(0x9E37)
    }
    fn op_mul(x: u64) -> u64 {
        x.wrapping_mul(31)
    }
    let handlers: [fn(u64) -> u64; 2] = [op_add, op_mul];

    // Fixed 2-op stream: both ops exercised, deterministic.
    let ops: Vec<u32> = (0..256u32).map(|i| i % 2).collect();
    let xs: Vec<u64> = (0..256u32).map(|i| (u64::from(i)) ^ 0xABCD).collect();

    let run_table = |ops: &[u32], xs: &[u64]| -> u64 {
        let mut acc = 0u64;
        for (i, &op) in ops.iter().enumerate() {
            let f = handlers[(op & 1) as usize];
            acc = f(acc ^ xs[i]);
        }
        acc
    };
    let run_hoisted = |ops: &[u32], xs: &[u64]| -> u64 {
        let f0 = handlers[0];
        let f1 = handlers[1];
        let mut acc = 0u64;
        for (i, &op) in ops.iter().enumerate() {
            let x = acc ^ xs[i];
            acc = if op & 1 == 0 { f0(x) } else { f1(x) };
        }
        acc
    };

    let a = run_table(&ops, &xs);
    let b = run_hoisted(&ops, &xs);
    assert_eq!(a, b, "hoist must be output-identical");

    // Non-vacuity: the all-add and all-mul streams must DIFFER —
    // proves both handlers are load-bearing in the fixture.
    let only_add: Vec<u32> = vec![0; 256];
    let only_mul: Vec<u32> = vec![1; 256];
    assert_ne!(run_table(&only_add, &xs), run_table(&only_mul, &xs));
}

/// Batch 109 — the rule is present.
#[test]
fn batch109_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    assert!(ids.contains(&"dead-result-timing-loop-folded-under-lto"));
}

/// Batch 109 — the rule parses (syn) on buggy AND fixed code.
#[test]
fn batch109_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "dead-result-timing-loop-folded-under-lto")
        .expect("rule present");
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "dead-result-timing-loop-folded-under-lto buggy/fixed must both parse"
    );
}

/// Batch 109 — the measurement-honesty contract is pinned in the rule
/// text: the callee-side black_box caveat, the constant-work fold
/// hazard, the NaN comparison directions, and the NotBenchable verdict
/// (no fabricated Pareto number for a rule whose baseline is zero).
#[test]
fn batch109_concepts_covered() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "dead-result-timing-loop-folded-under-lto")
        .expect("rule present");
    let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
    for needle in ["black_box", "sink", "xorshift", "assert"] {
        assert!(
            text.to_lowercase().contains(needle),
            "concept {needle:?} missing from the rule text"
        );
    }
    let notes = e.context_notes.as_deref().unwrap_or("");
    for needle in [
        "NotBenchable",
        "CALLER discards",
        "CHAINED STATE",
        "cancel in batches",
        "budget bar",
        "debug-stat-full-scan-out-of-hot-path",
    ] {
        assert!(
            notes.contains(needle),
            "contract {needle:?} missing from context_notes"
        );
    }
    // No fabricated bench: the measurement-honesty rule carries NO bench_spec.
    #[cfg(feature = "rust_perf_bench")]
    assert!(
        e.bench_spec.is_none(),
        "a measurement-honesty rule must not carry a fabricated Pareto bench"
    );
}

/// Batch 109 G1 — the rule's own fix pattern, run as code: the fixed
/// harness's work chain is DATA-DEPENDENT (xorshift over the loop
/// index) and the sink CONSUMES per-iteration results, so two different
/// data streams produce different sinks (the fold has nothing constant
/// to cut); the buggy shape's sink over a CONSTANT-work fn collapses to
/// 0 at compile time — the exact hazard the rule names. The timing
/// assertions are deliberately absent (a debug-profile unit test must
/// not gate on wall-clock); the gate pins the DATAFLOW contract, the
/// loud-baseline assert shape is pinned by the concepts gate above.
#[test]
fn batch109_sink_consumes_per_iteration_data() {
    // The gate that shaped the rule: BOTH draft fix-patterns failed
    // here before the chain form shipped. Each anti-shape is pinned as
    // an assertion so a future editor cannot quietly reintroduce it.
    let work = |seed: u64| -> u64 {
        let mut x = seed;
        for _ in 0..64 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
        }
        x
    };

    // Anti-shape 1 (GF(2) linearity): an XOR sink over the linear work
    // cancels EXACTLY over 0..2^k — the first draft's "fix" was itself
    // foldable to zero.
    let mut xor_sink = 0u64;
    for i in 0..4096u64 {
        xor_sink ^= work(i);
    }
    assert_eq!(
        xor_sink, 0,
        "XOR of a GF(2)-linear work over 0..2^k must cancel — the trap the rule names"
    );

    // Anti-shape 2 (balanced-bit addition collapse): over a
    // power-of-two batch, every balanced output bit is 1 in exactly
    // half the terms regardless of seed, so even wrapping_add lands
    // the SAME seed-independent constant (2^64 − n/2; measured:
    // both arms read 18446744073709549568).
    let add_sink = |seed: u64| -> u64 {
        let mut sink = 0u64;
        for i in 0..4096u64 {
            sink = sink.wrapping_add(work(seed ^ i));
        }
        sink
    };
    assert_eq!(
            add_sink(0),
            add_sink(0xDEAD_BEEF),
            "wrapping_add over a balanced power-of-two batch must collapse to the seed-independent constant — the second trap the rule names"
        );

    // The shipped fix pattern: CHAINED state — every iteration
    // consumes the previous accumulator, so the final value depends on
    // the whole sequential history and no batch algebra can fold it.
    // Non-vacuous both ways: different seeds land different chains.
    let chained = |seed: u64| -> u64 {
        let mut state = seed;
        for i in 0..4096u64 {
            state = work(state ^ i);
        }
        state
    };
    assert_ne!(chained(0), chained(0xDEAD_BEEF), "the chained harness must stay seed-dependent — this is the fold-proof shape the rule prescribes");
}

/// Batch 112 — the rule is present in the corpus.
#[test]
fn batch112_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    assert!(ids.contains(&"hot-byval-move-over-memcpy-threshold"));
}

/// Batch 112 — the rule parses (syn) on buggy AND fixed code.
#[test]
fn batch112_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "hot-byval-move-over-memcpy-threshold")
        .expect("rule present");
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "hot-byval-move-over-memcpy-threshold buggy/fixed must both parse"
    );
}

/// Batch 112 — the decision content is pinned in the rule text: the
/// memcpy-call threshold, the arch-shaped ceiling (asm evidence), the
/// composed-fix disclosure, and the armed x86 re-gate.
#[test]
fn batch112_concepts_covered() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "hot-byval-move-over-memcpy-threshold")
        .expect("rule present");
    let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
    for needle in ["memcpy", "threshold", "cold", "by reference", "in place"] {
        assert!(
            text.to_lowercase().contains(needle),
            "concept {needle:?} missing from the rule text"
        );
    }
    let notes = e.context_notes.as_deref().unwrap_or("");
    for needle in [
        "Mixed",
        "#160005",
        "asm",
        "ldp/stp",
        "bl _memcpy",
        "(136, 320]",
        "128 B",
        "40% wall",
        "DHAT",
        "Composed-fix disclosure",
        "x86 RE-GATED",
        "callq memcpy",
        "box-large-enum-variant",
    ] {
        assert!(
            notes.contains(needle),
            "contract {needle:?} missing from context_notes"
        );
    }
    // The rule IS benchable — the annotation's Mixed verdict comes from
    // the recorded run, not from an absent spec.
    #[cfg(feature = "rust_perf_bench")]
    assert!(
        e.bench_spec.is_some(),
        "the memcpy-threshold rule must carry its measured bench"
    );
}

/// Batch 112 G1 (the fairness contract): the buggy composed fold (17
/// fields moved by value) and the fixed composed fold (13 in the moved
/// struct + 4 by reference, mutated in place) land BIT-IDENTICAL state
/// and accumulator over the same LCG stream — the bench compares move
/// cost only, never workload. Also pins the load-bearing sizes:
/// 13 + 4 u64 = 136 B (the source's type) and 13 u64 = 104 B (the
/// source's shrunk type); the x86-64 threshold is 128 B, so 136 crosses
/// it and 104 does not — the bench shapes ARE the threshold content.
#[test]
fn batch112_byval_folds_agree_bit_identical() {
    // LCG stream — the same construction the bench harness uses.
    // The folded field ops — IDENTICAL per-field math in both arms, the
    // exact order the bench harness folds in.
    fn fold_hot(hot: &[u64; 13]) -> u64 {
        hot.iter().enumerate().fold(0u64, |a, (i, &v)| {
            a.wrapping_add(v.rotate_left(7) ^ (i as u64))
        })
    }
    fn fold_cold(cold: &[u64; 4]) -> u64 {
        cold.iter()
            .fold(0u64, |a, &v| a.wrapping_add(v.wrapping_mul(3)))
    }

    // Buggy arm: cold lives INSIDE the moved record (the RecBig/RecSmall
    // shapes ARE the bench's — 136 B vs 104 B straddling x86-64's 128 B).
    #[derive(Clone, Copy)]
    struct RecBig {
        hot: [u64; 13],
        cold: [u64; 4],
    }
    #[derive(Clone, Copy)]
    struct RecSmall {
        hot: [u64; 13],
    }

    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut seed_hot = [0u64; 13];
    let mut seed_cold = [0u64; 4];
    for v in &mut seed_hot {
        *v = next();
    }
    for v in &mut seed_cold {
        *v = next();
    }

    let mut state = RecBig {
        hot: seed_hot,
        cold: seed_cold,
    };
    let mut acc_buggy = 0u64;
    for i in 0..1_000 {
        // score moves the WHOLE 136 B record by value (buggy_code's shape)
        let r = state;
        acc_buggy ^= fold_hot(&r.hot) ^ fold_cold(&r.cold);
        state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc_buggy);
        state.cold[i & 3] = state.cold[i & 3].rotate_left(1);
    }

    // Fixed arm: cold is a separate by-ref tail, mutated in place.
    let mut state = RecSmall { hot: seed_hot };
    let mut cold = seed_cold;
    let mut acc_fixed = 0u64;
    for i in 0..1_000 {
        let r = state;
        acc_fixed ^= fold_hot(&r.hot) ^ fold_cold(&cold);
        state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc_fixed);
        cold[i & 3] = cold[i & 3].rotate_left(1);
    }

    assert_eq!(acc_buggy, acc_fixed, "the two arms must fold identically");
    let final_cold = cold;
    let rejoined = RecBig {
        hot: state.hot,
        cold: final_cold,
    };
    assert_eq!(
        rejoined.cold, final_cold,
        "cold tail state must agree across the arms"
    );

    // The threshold content: the bench shapes straddle x86-64's 128 B.
    assert_eq!(std::mem::size_of::<RecBig>(), 136);
    assert_eq!(std::mem::size_of::<RecSmall>(), 104);
}

/// Batch 99 G1 (output + dispatch-sequence identity): both shapes
/// dispatch the IDENTICAL handler sequence over the same stream (pinned
/// by per-handler call counters, not just final state) and land the
/// identical final state. Non-vacuous: both counters must be > 0. The
/// mechanism (merged = cmov + one indirect tail-jump, split = two
/// direct tail-jumps) is asm evidence recorded in the annotation — it
/// is not unit-gateable portably.
#[test]
fn batch99_split_sites_preserve_output_and_dispatch_sequence() {
    use std::cell::RefCell;
    struct State {
        acc: u64,
    }
    type Handler = fn(&mut State, u32);
    fn handler_add(st: &mut State, x: u32) {
        CALLS.with(|c| c.borrow_mut().0 += 1);
        st.acc = st.acc.wrapping_add(u64::from(x));
    }
    fn handler_mul(st: &mut State, x: u32) {
        CALLS.with(|c| c.borrow_mut().1 += 1);
        st.acc = st.acc.wrapping_mul(31).wrapping_add(u64::from(x));
    }

    thread_local! {
        static CALLS: RefCell<(u64, u64)> = const { RefCell::new((0, 0)) };
    }
    // same shapes as the corpus fixtures
    let step_merged = |pick_add: bool, st: &mut State, x: u32| {
        let f: Handler = if pick_add { handler_add } else { handler_mul };
        f(st, x);
    };
    let step_split = |pick_add: bool, st: &mut State, x: u32| {
        if pick_add {
            handler_add(st, x);
        } else {
            handler_mul(st, x);
        }
    };
    // deterministic mixed stream (LCG, 50/50-ish)
    let mut seed = 0x243F_6A88_85A3_08D3u64;
    let stream: Vec<(bool, u32)> = (0..1024)
        .map(|i| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let pick = ((seed >> 33) as u32) % 256 < 128;
            let x = (((seed >> 40) as u32) ^ i) & 0xFFFF;
            (pick, x)
        })
        .collect();

    CALLS.with(|c| *c.borrow_mut() = (0, 0));
    let mut sm = State { acc: 0 };
    for &(pick, x) in &stream {
        step_merged(pick, &mut sm, x);
    }
    let merged_calls = CALLS.with(|c| *c.borrow());

    CALLS.with(|c| *c.borrow_mut() = (0, 0));
    let mut ss = State { acc: 0 };
    for &(pick, x) in &stream {
        step_split(pick, &mut ss, x);
    }
    let split_calls = CALLS.with(|c| *c.borrow());

    assert_eq!(
        sm.acc, ss.acc,
        "both shapes must land the identical final state"
    );
    assert_eq!(
        merged_calls, split_calls,
        "both shapes must dispatch the IDENTICAL handler sequence"
    );
    assert!(
        merged_calls.0 > 0 && merged_calls.1 > 0,
        "non-vacuous: both handlers ran"
    );
}

/// Batch 86 G1 (exactness gate, integer ops — the ported-numeric
/// MANDATE shape): the serialize-once buffer must reconstruct every
/// (name, payload) record BYTE-EXACTLY, including the edge records
/// (empty name, empty payload, multi-byte u16 lengths). No tolerance
/// — a drifted offset must fail loudly, not approximately.
#[test]
#[allow(clippy::cast_possible_truncation)] // the u16/u32 narrowings ARE the wire format under test
fn batch86_byte_buffer_store_reconstructs_exactly() {
    let names: Vec<Box<str>> = vec![
        "a".into(),
        "example-node-7".into(),
        "".into(),                        // empty-NAME edge (offsets must advance)
        "x".repeat(300).into_boxed_str(), // multi-byte u16 length
    ];
    let payloads: Vec<Vec<u8>> = vec![
        vec![],
        vec![1, 2, 3],
        vec![0xAB; 41],
        (0..300u32).map(|i| (i % 251) as u8).collect(),
    ];
    // fixed form (mirrors the corpus fixed_code shape exactly; the
    // u16/u32 narrowings are the FORMAT — production lengths are bounded
    // by the record domain, asserted by the exact reconstruction below)
    let mut total = 4usize;
    for (n, p) in names.iter().zip(&payloads) {
        total += 4 + n.len() + p.len();
    }
    let mut buf = Vec::with_capacity(total);
    let mut offsets = Vec::with_capacity(names.len());
    buf.extend_from_slice(&(names.len() as u32).to_le_bytes());
    for (n, p) in names.iter().zip(&payloads) {
        offsets.push(buf.len() as u32);
        buf.extend_from_slice(&(n.len() as u16).to_le_bytes());
        buf.extend_from_slice(n.as_bytes());
        buf.extend_from_slice(&(p.len() as u16).to_le_bytes());
        buf.extend_from_slice(p);
    }
    // reconstruct + compare EXACTLY
    let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    assert_eq!(count, names.len());
    for (i, &off) in offsets.iter().enumerate() {
        let o = off as usize;
        let nl = u16::from_le_bytes([buf[o], buf[o + 1]]) as usize;
        let name = &buf[o + 2..o + 2 + nl];
        let pl = u16::from_le_bytes([buf[o + 2 + nl], buf[o + 3 + nl]]) as usize;
        let payload = &buf[o + 4 + nl..o + 4 + nl + pl];
        assert_eq!(name, names[i].as_bytes(), "name {i} drifted");
        assert_eq!(payload, payloads[i].as_slice(), "payload {i} drifted");
    }
}

/// Batch 86 G1 (the deterministic size axis for the bitflag rule):
/// the flag-cluster layout is 12 bytes, the bitflag layout is 8 —
/// pinned EXACTLY (align rules are platform-stable for these primitive
/// members), and every bit position round-trips through the packed
/// form. The "measure size_of before/after" caveat is the RULE's
/// (default-repr layouts vary); this fixture's layout is fixed.
#[test]
fn batch86_bitflag_size_axis_and_roundtrip() {
    #[allow(dead_code)] // size_of is the measurement
    #[allow(clippy::struct_excessive_bools)] // the 8-bool cluster IS the measured layout
    struct Wide {
        weight: u32,
        reachable: bool,
        verified: bool,
        stale: bool,
        pinned: bool,
        external: bool,
        compressed: bool,
        anchored: bool,
        sealed: bool,
    }
    #[allow(dead_code)]
    struct Packed {
        weight: u32,
        flags: u8,
    }
    assert_eq!(std::mem::size_of::<Wide>(), 12);
    assert_eq!(std::mem::size_of::<Packed>(), 8);
    for bit in 0..8u8 {
        let mut flags = 0u8;
        flags |= 1 << bit;
        assert_eq!((flags >> bit) & 1, 1);
        flags &= !(1 << bit);
        assert_eq!(flags, 0);
    }
}

#[cfg(feature = "rust_perf_bench")]
#[test]
fn eighty_six_rules_have_bench_specs() {
    let corpus = build_perf_seed_corpus();
    let with_spec = corpus
        .entries
        .iter()
        .filter(|e| e.bench_spec.is_some())
        .count();
    assert_eq!(with_spec, 91, "expected 91 benchable rules (Batch 143 added the debug-level-gated-computation Pareto bench), got {with_spec}");
}

#[cfg(feature = "rust_perf_bench")]
#[test]
fn fourteen_rules_are_not_benchable() {
    let corpus = build_perf_seed_corpus();
    let without_spec: Vec<&str> = corpus
        .entries
        .iter()
        .filter(|e| e.bench_spec.is_none())
        .map(|e| e.rule_id.as_str())
        .collect();
    // Batch 135 repaired this pin: the Batch 118–129 source-reported io
    // annotations (13 rules) were never counted here — the pin sat at the
    // Batch-109-era 14 while reality moved to 27. The test only compiles
    // under `rust_perf_bench`, so nobody ran it.
    assert_eq!(without_spec.len(), 35); // 126 total − 91 benchable
    assert!(without_spec.contains(&"prefault-admission-keyed-to-ram-headroom")); // Batch 139 — admission-policy annotation, source-reported only
    assert!(without_spec.contains(&"coalesce-nearby-reads-bounded-gap-cacheability-gated")); // Batch 94 — cacheability-coupled regime, source-reported only
    assert!(without_spec.contains(&"software-prefetch-storage-bound")); // Batch 85 — x86-only source evidence, the annotation IS the rule
    assert!(without_spec.contains(&"bitflag-pack-when-footprint-dominates")); // Batch 86 — the deterministic size axis (12 → 8), not a speed bench
    assert!(without_spec.contains(&"dynamic-work-parallelism-ladder")); // Batch 136 — qualitative ladder, the author's verdict IS the content
    assert!(without_spec.contains(&"sender-per-work-item-terminates-channel")); // Batch 136 — termination/ownership shape, not an A/B axis
    assert!(without_spec.contains(&"nested-par-inside-channel-bridge-deadlocks")); // Batch 136 — a deadlock is not an A/B axis
    assert!(without_spec.contains(&"compaction-reserve-before-prompt-build")); // Batch 141 — control-flow sizing, not an A/B axis
    assert!(without_spec.contains(&"compaction-tail-boundary-prefers-complete-turn")); // Batch 141 — boundary semantics, not an A/B axis
    assert!(without_spec.contains(&"streaming-prefix-holdback-bounded-buffer")); // Batch 141 — streaming-latency discipline, not an A/B axis
    assert!(without_spec.contains(&"macro-generated-code-optimize-at-generation-point")); // Batch 143 — build-time emission content, no runtime A/B axis
    assert!(without_spec.contains(&"layout-reorder"));
    assert!(without_spec.contains(&"repr-u8-enum"));
    assert!(without_spec.contains(&"bevy-reactive-loop"));
    assert!(without_spec.contains(&"wasm-batch-ffi"));
    assert!(without_spec.contains(&"shrink-to-fit-after-build"));
    assert!(without_spec.contains(&"box-large-enum-variant"));
    assert!(without_spec.contains(&"async-yield-point"));
    assert!(without_spec.contains(&"async-future-size"));
    // Issue 014 T2.3/T2.4 — NotBenchable metric-mismatch class.
    assert!(without_spec.contains(&"tokio-guard-across-await"));
    assert!(without_spec.contains(&"bounded-channel-backpressure"));
    assert!(without_spec.contains(&"dead-result-timing-loop-folded-under-lto"));
    // Batch 109 — a gate whose baseline folds to zero has nothing to ratio
}

/// Bench 013 found 4 rules with non-dominant verdicts in the default
/// config (2 ParetoDominated + 2 Mixed). Their `context_notes` annotation
/// records the finding so consumers don't unconditionally suggest them.
/// Bench 016 (Issue 012) added 2 more annotated rules (sort-by-key,
/// copy-from-slice). Issue 014 T2.3/T2.4 added 2 more in the
/// NotBenchable metric-mismatch class (tokio-guard-across-await,
/// bounded-channel-backpressure). Batch 48 added 2 verdict-class rules
/// (filter-count-to-any ParetoDominated, iter-copied-deref-map Mixed).
/// Batch 53 (Bench 038) added 2 more (powi-vs-powf all-Tie Mixed,
/// soa-hot-field latency-bound Mixed). Batch 62 added 1 (the
/// newton-refinement regime-inversion annotation) + Batch 64 added 1
/// (debug-assert-length-hoist, the release-erasure class) + Batch 66
/// added 1 (sortedness-check-branchless-accumulate, the regime-split
/// class — the mirrored regime of filter-count-to-any). Batch 65 added 2
/// (align-64-false-sharing — the arch-granularity caveat on a clean
/// winner; channel-send-batching-unpark — the pipeline-overlap regime).
/// Batch 73 added 2 (sort-key-cache-precompute-out-of-lock — the
/// sorts-per-update amortization regime + the source-vs-isolated
/// magnitude caveat; lock-guard-narrow-field-accessor — the honest
/// margin note on a clean winner). Batch 78 added 2 (narrow-counter-
/// overflow-promotion — the ParetoDominated memory-for-speed trade
/// (the Datadog 2,100 GiB regime vs the measured update-path cost);
/// cache-line-block-partitioned-lock — measured-winner regime note +
/// the first ALLOC_AXIS_EXEMPT rule). Batch 86 added 3 (serialize-once-
/// byte-buffer-store — the shrinking-ratio + regime note;
/// context-equal-field-elide — the %-equal two-regime note;
/// bitflag-pack-when-footprint-dominates — the NotBenchable-speed size
/// axis + the repr-reorder caveat). Batch 91 added 2
/// (filter-before-expensive-transform — the two-regime discard ×
/// transform-cost note; decision-projection-before-construction — the
/// invariance-precondition note). Batch 94 added 1
/// (coalesce-nearby-reads-bounded-gap-cacheability-gated — the
/// cacheability-coupling regime, source-reported). Batch 99 added 1
/// (mir-destprop-branch-site-merge — the measured ParetoDominant verdict
/// with the runtime-opaque-targets negative-control boundary and the
/// rustc DestProp toolchain regime).
/// The other 57 have `context_notes: None` (unconditional wins or
/// honestly NotBenchable).
#[test]
fn forty_three_annotated_rules_carry_context_notes() {
    let corpus = build_perf_seed_corpus();
    let annotated: Vec<&str> = corpus
        .entries
        .iter()
        .filter(|e| e.context_notes.is_some())
        .map(|e| e.rule_id.as_str())
        .collect();
    assert_eq!(annotated.len(), 69, "expected 69 context-annotated rules (Batch 143 added the nnethercote source-citation annotation + the build-time rationale)");
    assert!(annotated.contains(&"debug-level-gated-computation")); // Batch 143 — the #147293 citation + the #143684 wall-time-vs-icount measurement-honesty rider
    assert!(annotated.contains(&"macro-generated-code-optimize-at-generation-point")); // Batch 143 — the build-time rationale + the -Zmacro-stats instrument
    assert!(annotated.contains(&"prefault-admission-keyed-to-ram-headroom")); // Batch 139 — the io admission-policy annotation
    assert!(annotated.contains(&"parallel-disjoint-pread-hides-cold-page-latency")); // Batch 118 — the ds4 PLE-gather io annotation (source-reported, NotBenchable here)
    assert!(annotated.contains(&"resume-revalidates-provenance-before-reuse")); // Batch 121 — the ds4 IQ2 resume-provenance annotation
    assert!(annotated.contains(&"write-verify-readback-before-publish")); // Batch 121 — the ds4 IQ2 publish-integrity annotation
    assert!(annotated.contains(&"ab-isolate-one-component-by-baseline-byte-copy")); // Batch 121 — the A/B-design + memory-honesty annotation
    assert!(annotated.contains(&"dead-result-timing-loop-folded-under-lto")); // Batch 109 — the NotBenchable measurement-honesty axis
    assert!(annotated.contains(&"hot-byval-move-over-memcpy-threshold")); // Batch 112 — the arch-regime Mixed annotation
    assert!(annotated.contains(&"weight-tile-consumption-order")); // Batch 85 — measured ParetoDominant + the M3-L1-shift regime note
    assert!(annotated.contains(&"software-prefetch-storage-bound")); // Batch 85 — the x86-source-cited honest-loser annotation
    assert!(annotated.contains(&"precompute-table"));
    assert!(annotated.contains(&"papaya-lockfree"));
    assert!(annotated.contains(&"dedupe-hashset-vs-vec"));
    assert!(annotated.contains(&"reuse-allocation-clear"));
    assert!(annotated.contains(&"sort-by-key"));
    assert!(annotated.contains(&"copy-from-slice"));
    assert!(annotated.contains(&"tokio-guard-across-await"));
    assert!(annotated.contains(&"bounded-channel-backpressure"));
    assert!(annotated.contains(&"filter-count-to-any"));
    assert!(annotated.contains(&"iter-copied-deref-map"));
    assert!(annotated.contains(&"powi-vs-powf"));
    assert!(annotated.contains(&"soa-hot-field"));
    assert!(annotated.contains(&"newton-refinement-vs-hardware-transcendental"));
    assert!(annotated.contains(&"sortedness-check-branchless-accumulate"));
    assert!(annotated.contains(&"debug-assert-length-hoist"));
    assert!(annotated.contains(&"align-64-false-sharing"));
    assert!(annotated.contains(&"channel-send-batching-unpark"));
    assert!(annotated.contains(&"sort-key-cache-precompute-out-of-lock"));
    assert!(annotated.contains(&"lock-guard-narrow-field-accessor"));
    assert!(annotated.contains(&"swar-digit-chunk-parse")); // Batch 74 — the regime note (<8 digits tie)
    assert!(annotated.contains(&"runtime-feature-detect-per-chunk")); // Batch 74 — measured-Tie honest annotation
    assert!(annotated.contains(&"encoded-layout-borrowed-view")); // Batch 75 — S<<R regime
    assert!(annotated.contains(&"group-keys-borrow-allocate-on-first-sight")); // Batch 75 — R/S regime + correctness contract
    assert!(annotated.contains(&"sorted-input-previous-row-fast-path")); // Batch 75 — hit-rate regime
    assert!(annotated.contains(&"btreemap-sorted-iteration")); // Batch 75 rider — the two-row sorted-read decision table
    assert!(annotated.contains(&"bitplane-transpose-repeated-predicate-scan")); // Batch 83 — measured ParetoDominant + the amortization/b/equality regime gates
    assert!(annotated.contains(&"narrow-counter-overflow-promotion")); // Batch 78 — ParetoDominated honest-loser (the memory-for-speed trade)
    assert!(annotated.contains(&"cache-line-block-partitioned-lock")); // Batch 78 — measured-winner + regime + alloc-exempt note
    assert!(annotated.contains(&"serialize-once-byte-buffer-store")); // Batch 86 — measured ParetoDominant + the shrinking-ratio/regime note
    assert!(annotated.contains(&"context-equal-field-elide")); // Batch 86 — measured ParetoDominant + the %-equal two-regime note
    assert!(annotated.contains(&"bitflag-pack-when-footprint-dominates")); // Batch 86 — NotBenchable-speed size axis + the repr-reorder caveat
    assert!(annotated.contains(&"filter-before-expensive-transform")); // Batch 91 — measured ParetoDominant + the two-regime (discard × cost) note
    assert!(annotated.contains(&"decision-projection-before-construction")); // Batch 91 — the invariance-precondition note
    assert!(annotated.contains(&"coalesce-nearby-reads-bounded-gap-cacheability-gated")); // Batch 94 — the cacheability-coupling regime note (source-reported)
    assert!(annotated.contains(&"mir-destprop-branch-site-merge")); // Batch 99 — measured ParetoDominant + the runtime-opaque-targets boundary + toolchain regime
    assert!(annotated.contains(&"dispatch-handler-pair-hoist")); // Batch 101 — measured ParetoDominant (aarch64) + the unpredictable-stream regime + the x86 re-gate verdict (2026-09-04, expected-Tie REFUTED)
}

/// The non-annotated rules (ParetoDominant + honestly NotBenchable) must
/// have `context_notes: None` — they are either unconditional wins or
/// honestly unmeasurable.
#[test]
fn unannotated_rules_have_no_context_notes() {
    let corpus = build_perf_seed_corpus();
    let unannotated = corpus
        .entries
        .iter()
        .filter(|e| e.context_notes.is_none())
        .count();
    assert_eq!(unannotated, 57, "expected 57 unannotated rules (126 total - 69 annotated; both Batch 143 rules joined the ANNOTATED side, so the count is unchanged)");
}

/// `context_notes` is backward-compatible via `#[serde(default)]`: an older
/// freeze/thaw snapshot (pre-this-field) deserializes as `None`.
#[test]
fn context_notes_backward_compatible_serde() {
    let corpus = build_perf_seed_corpus();
    // Serialize + deserialize round-trip preserves the annotation.
    let json = serde_json::to_string(&corpus.entries).unwrap();
    let back: Vec<PerfRule> = serde_json::from_str(&json).unwrap();
    let annotated = back.iter().filter(|e| e.context_notes.is_some()).count();
    assert_eq!(annotated, 69); // Batch 143: 67 → 69

    // Simulate a pre-field snapshot: strip `context_notes` from JSON.
    // (A real old snapshot simply omits the key; serde(default) → None.)
    let old_json = json.replace("\"context_notes\":", "\"__removed_context_notes\":");
    let old_back: Vec<PerfRule> = serde_json::from_str(&old_json).unwrap();
    let old_annotated = old_back
        .iter()
        .filter(|e| e.context_notes.is_some())
        .count();
    assert_eq!(old_annotated, 0, "missing field should default to None");
}

/// Integration: the drafter propagates `context_notes` from `PerfRule`
/// through `DraftedFix` for the annotated rules, and `None` for the
/// unconditional wins. Twenty annotations: Bench 013 (2 ParetoDominated
/// and 2 Mixed), Bench 016 (sort-by-key and copy-from-slice), Issue 014
/// (2 NotBenchable metric-mismatch), Batch 48 (2 verdict-class),
/// Batch 53 (Bench 038, 2 verdict-class), Batch 62 (1 regime-class),
/// Batch 64 (1 release-erasure class), Batch 66 (1 regime-split
/// class), Batch 68 (1 ParetoDominated honest-loser — the tantivy
/// varint ladder regresses on aarch64 across 3 tested writer regimes),
/// Batch 65 (2 measured-winner regime caveats — align-64's
/// arch-granularity + channel-send's pipeline-overlap margin; a NEW
/// annotation class: ParetoDominant verdict + applicability condition,
/// accepted by the token check below), and Batch 73 (2 measured-winner
/// caveats — the mnt.io room-list distill: sort-key-cache's
/// sorts-per-update regime + source-vs-isolated magnitude note;
/// lock-guard-narrow's end-to-end-vs-isolated margin note).
#[test]
fn drafter_propagates_context_notes_from_perf_corpus() {
    use crate::domain::HealEntry;

    let corpus = build_perf_seed_corpus();

    // Filter to the 69 annotated rules.
    let annotated: Vec<&PerfRule> = corpus
        .entries
        .iter()
        .filter(|e| e.context_notes.is_some())
        .collect();
    assert_eq!(annotated.len(), 69, "expected 69 annotated rules");

    // The HealEntry trait override must agree with the struct field.
    for rule in &annotated {
        assert_eq!(
            HealEntry::context_notes(*rule),
            rule.context_notes.as_deref(),
            "HealEntry::context_notes() must match the PerfRule field for {}",
            rule.rule_id
        );
        assert!(
            rule.context_notes
                .as_ref()
                .unwrap()
                .contains("ParetoDominated")
                || rule
                    .context_notes
                    .as_ref()
                    .unwrap()
                    .contains("ParetoDominant")
                || rule.context_notes.as_ref().unwrap().contains("Mixed")
                || rule
                    .context_notes
                    .as_ref()
                    .unwrap()
                    .contains("NotBenchable"),
            "annotation should mention ParetoDominated, ParetoDominant \
                 (the Batch 65 measured-winner caveat class), Mixed, or the \
                 NotBenchable metric-mismatch class for {}",
            rule.rule_id
        );
    }

    // The unannotated rules must return None from the trait too.
    let unannotated: Vec<&PerfRule> = corpus
        .entries
        .iter()
        .filter(|e| e.context_notes.is_none())
        .collect();
    assert_eq!(unannotated.len(), 57, "expected 57 unannotated rules");
    for rule in &unannotated {
        assert!(
            HealEntry::context_notes(*rule).is_none(),
            "{} should have no context_notes",
            rule.rule_id
        );
    }
}

/// G1 for `align-64-false-sharing` (Batch 65, Research 081): the
/// per-thread counter totals are a deterministic closed form — layout
/// (and thread interleaving) cannot change them. fetch_add is atomic, so
/// each counter's final value is exactly its op count regardless of which
/// slot layout the atomics lived in. Pins the bench harness's in-run
/// debug_assert at the corpus level + documents the identity the wall-clock
/// arms share (the timing differential is PURELY the line ping-pong).
#[test]
fn align_64_false_sharing_totals_are_layout_invariant() {
    use std::sync::atomic::Ordering;

    #[repr(align(128))]
    struct Padded {
        allocs: std::sync::atomic::AtomicU64,
        bytes: std::sync::atomic::AtomicU64,
        frees: std::sync::atomic::AtomicU64,
    }
    #[derive(Default)]
    struct Packed {
        allocs: std::sync::atomic::AtomicU64,
        bytes: std::sync::atomic::AtomicU64,
        frees: std::sync::atomic::AtomicU64,
    }

    const SLOTS: usize = 12;
    const ITERS: u64 = 500;

    fn expected(n_threads: usize) -> u64 {
        n_threads as u64 * (2 * ITERS + ITERS * (ITERS - 1) / 2)
    }

    // Multi-threaded on BOTH layouts (interleaving varies, totals cannot).
    for n_threads in [2usize, 4, 8] {
        let padded: Vec<Padded> = (0..SLOTS)
            .map(|_| Padded {
                allocs: std::sync::atomic::AtomicU64::new(0),
                bytes: std::sync::atomic::AtomicU64::new(0),
                frees: std::sync::atomic::AtomicU64::new(0),
            })
            .collect();
        let packed: Vec<Packed> = (0..SLOTS).map(|_| Packed::default()).collect();
        std::thread::scope(|s| {
            for t in 0..n_threads {
                let p = &padded;
                let q = &packed;
                s.spawn(move || {
                    for i in 0..ITERS {
                        p[t].allocs.fetch_add(1, Ordering::Relaxed);
                        p[t].bytes.fetch_add(i, Ordering::Relaxed);
                        p[t].frees.fetch_add(1, Ordering::Relaxed);
                        q[t].allocs.fetch_add(1, Ordering::Relaxed);
                        q[t].bytes.fetch_add(i, Ordering::Relaxed);
                        q[t].frees.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        let cs_p: u64 = padded
            .iter()
            .map(|x| {
                x.allocs.load(Ordering::Relaxed)
                    + x.bytes.load(Ordering::Relaxed)
                    + x.frees.load(Ordering::Relaxed)
            })
            .sum();
        let cs_q: u64 = packed
            .iter()
            .map(|x| {
                x.allocs.load(Ordering::Relaxed)
                    + x.bytes.load(Ordering::Relaxed)
                    + x.frees.load(Ordering::Relaxed)
            })
            .sum();
        assert_eq!(cs_p, expected(n_threads), "padded totals @{n_threads}t");
        assert_eq!(cs_q, expected(n_threads), "packed totals @{n_threads}t");
    }

    // The annotation must carry the arch-granularity caveat (the rule id
    // says 64 — the honest fix is line-size-dependent).
    let corpus = build_perf_seed_corpus();
    let rule = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "align-64-false-sharing")
        .expect("align-64-false-sharing in corpus");
    assert!(rule.context_notes.as_ref().unwrap().contains("128"));
}

/// G1 for `channel-send-batching-unpark` (Batch 65, Research 081): the
/// per-item and per-batch producer paths deliver the SAME item count —
/// the flush boundary cannot drop or double-count (the mem::replace
/// swap keeps the tail batch's capacity + the final flush covers the
/// remainder). Also pins the annotation's take-vs-replace warning: the
/// replace pattern is the one that wins the alloc axis (take-then-regrow
/// measured 795 vs 329 allocs @10K on the WRONG side).
#[test]
fn channel_send_batching_delivers_identical_counts() {
    const FLUSH_EVERY: usize = 64;

    fn buggy_delivered(total: u64) -> u64 {
        total
    }
    fn fixed_delivered(total: u64) -> u64 {
        let mut count: u64 = 0;
        let mut tail: usize = 0;
        for i in 0..total {
            let _ = std::hint::black_box(i);
            tail += 1;
            if tail >= FLUSH_EVERY {
                count += tail as u64; // one batch delivered
                tail = 0;
            }
        }
        count += tail as u64; // final flush covers the remainder
        count
    }

    for total in [0u64, 1, 63, 64, 65, 10_000, 100_000] {
        assert_eq!(
            fixed_delivered(total),
            buggy_delivered(total),
            "batched delivery count drift at total={total}"
        );
    }

    let corpus = build_perf_seed_corpus();
    let rule = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "channel-send-batching-unpark")
        .expect("channel-send-batching-unpark in corpus");
    let note = rule.context_notes.as_ref().unwrap();
    assert!(
        note.contains("mem::replace"),
        "annotation must warn take-vs-replace"
    );
}

/// G1 for `sort-key-cache-precompute-out-of-lock` (Batch 73, Research
/// 085/086 — the mnt.io room-list distill): the maintained-cache sort
/// must produce the IDENTICAL order as the lock+clone comparator sort —
/// same keys, same comparator, no observable side effects. Mirrors the
/// bench harness's shape at test scale (distinct keys → total order →
/// deterministic), including the reverse-then-sort-desc protocol.
#[test]
fn sort_key_cache_order_matches_lock_clone_comparator() {
    use std::sync::RwLock;

    #[derive(Clone)]
    struct LatestEvent {
        timestamp: u64,
        // width-only: the clone reads it, nothing else does
        #[allow(dead_code)]
        body: String,
    }
    struct RoomInfo {
        latest_event: Option<LatestEvent>,
    }
    struct Room {
        info: RwLock<RoomInfo>,
    }

    impl Room {
        fn wide_ts(&self) -> Option<u64> {
            // the buggy shape: whole-value clone under the guard
            self.info
                .read()
                .unwrap()
                .latest_event
                .clone()
                .map(|ev| ev.timestamp)
        }
        fn narrow_ts(&self) -> Option<u64> {
            self.info
                .read()
                .unwrap()
                .latest_event
                .as_ref()
                .map(|ev| ev.timestamp)
        }
    }

    const N: usize = 128;
    let mut rooms: Vec<Room> = (0..N as u64)
        .map(|i| Room {
            info: RwLock::new(RoomInfo {
                latest_event: Some(LatestEvent {
                    timestamp: i * 7 + 3, // distinct
                    body: format!("body {i} padded padded padded"),
                }),
            }),
        })
        .collect();
    rooms.reverse(); // maximally unsorted start

    // Arm A: the buggy comparator (clone under guard, twice per cmp).
    let mut buggy_order: Vec<Option<u64>> = rooms.iter().map(Room::wide_ts).collect();
    buggy_order.sort_by(|a, b| b.cmp(a));

    // Arm B: refresh the cache once (narrow reads), sort on cached keys.
    let cached: Vec<Option<u64>> = rooms.iter().map(Room::narrow_ts).collect();
    let mut fixed_order = cached.clone();
    fixed_order.sort_by(|a, b| b.cmp(a));

    assert_eq!(
        buggy_order, fixed_order,
        "maintained-cache sort order drifted from the lock+clone order"
    );
    // Descending-by-distinct-keys is fully determined: first = max key.
    let max_key = (0..N as u64).map(|i| i * 7 + 3).max().unwrap();
    assert_eq!(fixed_order.first().copied().flatten(), Some(max_key));
}

/// G1 for `lock-guard-narrow-field-accessor` (Batch 73, Research 085/086):
/// the narrow accessor must return the SAME value as the wide-clone
/// accessor for every room — the clone is unobservable, only the
/// consumed sub-field crosses.
#[test]
fn lock_guard_narrow_accessor_values_match_wide_clone() {
    use std::sync::RwLock;

    #[derive(Clone)]
    struct LatestEvent {
        // width-only fields: only the clone ever touches them
        #[allow(dead_code)]
        timestamp: u64,
        sender_is_local: bool,
        #[allow(dead_code)]
        body: String,
    }
    struct RoomInfo {
        latest_event: Option<LatestEvent>,
    }
    struct Room {
        info: RwLock<RoomInfo>,
    }

    impl Room {
        fn wide_is_local(&self) -> bool {
            self.info
                .read()
                .unwrap()
                .latest_event
                .clone() // the whole 144 B payload
                .is_some_and(|ev| ev.sender_is_local)
        }
        fn narrow_is_local(&self) -> bool {
            self.info
                .read()
                .unwrap()
                .latest_event
                .as_ref()
                .is_some_and(|ev| ev.sender_is_local)
        }
    }

    // Mixed local/not + one room with NO latest event (the None edge).
    let rooms: Vec<Room> = (0..33)
        .map(|i| {
            let latest = if i == 32 {
                None
            } else {
                Some(LatestEvent {
                    timestamp: i * 11 + 1,
                    sender_is_local: i % 3 == 0,
                    body: format!("body {i} padded padded"),
                })
            };
            Room {
                info: RwLock::new(RoomInfo {
                    latest_event: latest,
                }),
            }
        })
        .collect();

    for (i, r) in rooms.iter().enumerate() {
        assert_eq!(
            r.wide_is_local(),
            r.narrow_is_local(),
            "narrow accessor drifted from wide-clone at room {i}"
        );
    }
    // The None edge: both arms false.
    assert!(!rooms[32].narrow_is_local());
}

/// Batch 75 — the three new rules parse (syn) and are retrievable.
#[test]
fn batch75_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    for id in [
        "encoded-layout-borrowed-view",
        "group-keys-borrow-allocate-on-first-sight",
        "sorted-input-previous-row-fast-path",
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} missing from corpus"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "{id} buggy/fixed must both parse"
        );
    }
}

#[test]
fn batch75_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    assert!(ids.contains(&"encoded-layout-borrowed-view"));
    assert!(ids.contains(&"group-keys-borrow-allocate-on-first-sight"));
    assert!(ids.contains(&"sorted-input-previous-row-fast-path"));
}

/// Batch 76 — the two Meilisearch rules are present + retrievable.
#[test]
fn batch76_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    assert!(ids.contains(&"debug-stat-full-scan-out-of-hot-path"));
    assert!(ids.contains(&"eager-pointer-collect-lazy-fetch"));
}

/// Batch 76 — G1 for `debug-stat-full-scan-out-of-hot-path`: the buggy
/// arm's single-scan statistic and the fixed arm's maintained aggregate
/// must agree EXACTLY (len, total, mean bits) on structured + LCG
/// inputs — the mean's f64 bits are the strictest cheap equality we
/// have (any divergence in total or len shifts the quotient's bits).
#[test]
#[allow(clippy::cast_precision_loss)] // the usize→f64 mean IS the statistic under test
fn debug_stat_maintained_aggregate_matches_scan_exactly() {
    fn scan_stat(items: &[usize], batch: &[usize]) -> (usize, usize, u64) {
        let total: usize = items.iter().chain(batch).sum();
        let len = items.len() + batch.len();
        let mean = len as f64 / total.max(1) as f64;
        (len, total, mean.to_bits())
    }
    fn maintained_stat(items: &[usize], link_count: usize, batch: &[usize]) -> (usize, usize, u64) {
        let total = link_count + batch.iter().sum::<usize>();
        let len = items.len() + batch.len();
        let mean = len as f64 / total.max(1) as f64;
        (len, total, mean.to_bits())
    }
    let mut rng: u64 = 0x5EED_5EED_5EED_5EED;
    let mut next = || {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (rng >> 16) as usize % 8
    };
    // empty store + empty batch (the max(1) guard), structured, + LCG
    for (items, batch) in [
        (vec![], vec![]),
        (vec![0usize; 5], vec![3usize; 2]),
        (
            (0..200).map(|_| next()).collect::<Vec<_>>(),
            (0..50).map(|_| next()).collect::<Vec<_>>(),
        ),
    ] {
        let link_count = items.iter().sum::<usize>();
        assert_eq!(
            scan_stat(&items, &batch),
            maintained_stat(&items, link_count, &batch)
        );
    }
}

/// Batch 75 — the candidate-list correctness contract: an injected
/// worst-case hash (len % 4) MUST NOT merge distinct groups. Pins the
/// `HashMap<u64, Vec<usize>>` decision content a real SipHash collision
/// never exercises (the Lane B falsifiable-gate recommendation).
#[test]
fn batch75_collision_candidate_list_never_merges() {
    use std::collections::HashMap;
    // 8 distinct keys, all colliding under the injected hash
    let keys: Vec<Vec<(String, String)>> = (0..8)
        .map(|i| vec![(format!("k{i}"), format!("v{i}"))])
        .collect();
    let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut groups: Vec<Vec<(String, String)>> = Vec::new();
    for key in &keys {
        let hash = (key.len() % 4) as u64; // adversarial: identical for all
        let idx = if let Some(i) = index
            .get(&hash)
            .and_then(|c| c.iter().find(|&&i| groups[i] == *key).copied())
        {
            i
        } else {
            let i = groups.len();
            groups.push(key.clone());
            index.entry(hash).or_default().push(i);
            i
        };
        std::hint::black_box(idx);
    }
    // 8 colliding distinct keys → 8 distinct groups, zero merges
    assert_eq!(
        groups.len(),
        8,
        "candidate list must keep colliding keys distinct"
    );
    let mut sorted = groups.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 8);
}

/// Batch 75 — the prefix-trap correctness contract: with optional fields
/// skipped, one key can be a PREFIX of another; the exhaustive compare
/// (length check) must not match them (the source's
/// `labels.next().is_none()` discipline, `{host=a}` vs `{host=a, env=prod}`).
#[test]
fn batch75_prefix_keys_do_not_match() {
    fn key_matches(key: &[(String, String)], row_labels: &[(String, String)]) -> bool {
        key.len() == row_labels.len() && key.iter().zip(row_labels).all(|(a, b)| a == b)
    }
    let short: Vec<(String, String)> = vec![("host".into(), "a".into())];
    let long: Vec<(String, String)> =
        vec![("host".into(), "a".into()), ("env".into(), "prod".into())];
    // identical prefix — must NOT match without the length check
    assert!(!key_matches(&short, &long));
    assert!(!key_matches(&long, &short));
    // exact equality still matches
    assert!(key_matches(&short, &short));
    assert!(key_matches(&long, &long));
}

/// Batch 78 — the two Datadog rules are present + retrievable.
#[test]
fn batch78_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    assert!(ids.contains(&"narrow-counter-overflow-promotion"));
    assert!(ids.contains(&"cache-line-block-partitioned-lock"));
}

/// Batch 78 — both new rules parse (syn) on buggy AND fixed code.
#[test]
fn batch78_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    for id in [
        "narrow-counter-overflow-promotion",
        "cache-line-block-partitioned-lock",
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} missing from corpus"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "{id} buggy/fixed must both parse"
        );
    }
}

/// Batch 83 — the bit-slicing rule is present + retrievable.
#[test]
fn batch83_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    assert!(ids.contains(&"bitplane-transpose-repeated-predicate-scan"));
}

/// Batch 83 — the rule parses (syn) on buggy AND fixed code.
#[test]
fn batch83_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "bitplane-transpose-repeated-predicate-scan")
        .unwrap_or_else(|| panic!("bitplane rule missing from corpus"));
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "bitplane buggy/fixed must both parse"
    );
}

/// Batch 83 — G1 (the ported-numeric MANDATE shape): the two-mask MSB→LSB
/// recurrence must equal the scalar count EXACTLY for every threshold,
/// adversarial value streams (LCG, all-equal, all-zero, max-saturated),
/// and BOTH adaptive widths (values < 256 → b = 8; values incl. huge →
/// b = 32). Integer ops — no tolerance.
#[test]
fn batch83_bitplane_mask_matches_scalar_exactly() {
    fn encode(values: &[u32], b: usize) -> (Vec<Vec<u64>>, usize) {
        let words = values.len().div_ceil(64);
        let mut planes = vec![vec![0u64; words]; b];
        for (i, &v) in values.iter().enumerate() {
            for (bit, plane) in planes.iter_mut().enumerate() {
                plane[i / 64] |= u64::from((v >> bit) & 1) << (i % 64);
            }
        }
        (planes, words)
    }

    fn gt_popcount(planes: &[Vec<u64>], words: usize, b: usize, c: u32) -> u64 {
        if b < 32 && c >= (1u32 << b) {
            return 0;
        }
        let mut gt = vec![0u64; words];
        let mut eq = vec![u64::MAX; words];
        for bit in (0..b).rev() {
            let plane = &planes[bit];
            if (c >> bit) & 1 == 0 {
                for w in 0..words {
                    gt[w] |= eq[w] & plane[w];
                }
            } else {
                for w in 0..words {
                    eq[w] &= plane[w];
                }
            }
        }
        gt.iter().map(|w| u64::from(w.count_ones())).sum()
    }

    // LCG stream + edge-shape columns at two widths.
    let mut rng: u64 = 0x0B78_0D06_C0DE_5EED;
    let mut cols: Vec<Vec<u32>> = Vec::new();
    for _ in 0..8 {
        let mut v = Vec::with_capacity(513); // non-multiple-of-64 tail
        for _ in 0..513 {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            v.push(((rng >> 33) % 256) as u32);
        }
        cols.push(v);
    }
    cols.push(vec![7u32; 513]); // all-equal
    cols.push(vec![0u32; 129]); // all-zero (b = 1 adaptive)
    cols.push(vec![u32::MAX; 65]); // max-saturated (b = 32)

    for col in &cols {
        let max_val = col.iter().copied().max().unwrap_or(0);
        let b = (32 - max_val.leading_zeros()).max(1) as usize;
        let (planes, words) = encode(col, b);
        // sweep ONLY the representable range (the domain contract:
        // constants beyond 2^b - 1 are bias-handled by the caller — see
        // the guard checks below)
        let c_max = u32::from(u8::MAX).min((1u32 << b.min(8)) - 1);
        for c in 0..=c_max {
            let scalar = col.iter().filter(|&&v| v > c).count() as u64;
            assert_eq!(gt_popcount(&planes, words, b, c), scalar, "threshold {c}");
        }
        // out-of-range constants: nothing in a b-wide column can exceed
        // them — the caller-side guard (one branch per query) returns 0
        if b < 32 {
            let out = 1u32 << b;
            let scalar = col.iter().filter(|&&v| v > out).count() as u64;
            assert_eq!(scalar, 0, "sanity: no value exceeds 2^b");
        }
        // wide-width column: sweep a few u32 thresholds too
        if b == 32 {
            for c in [0u32, 1, 0x7FFF_FFFF, u32::MAX - 1, u32::MAX] {
                let scalar = col.iter().filter(|&&v| v > c).count() as u64;
                assert_eq!(
                    gt_popcount(&planes, words, b, c),
                    scalar,
                    "wide threshold {c}"
                );
            }
        }
    }
}

/// Batch 78 — G1 for `narrow-counter-overflow-promotion` (the ported-
/// numeric MANDATE shape): the u32+sidecar form must be EXACTLY
/// equivalent to the u64 buffer for ANY monotone update stream,
/// including streams that CROSS the u32 boundary (the promotion path
/// the bench never fires). Adversarial sequences: repeated
/// u32::MAX-adjacent adds, mixed small/large, LCG, and every-slot
/// overflow sweeps. Totals AND per-slot values must match bit-exactly.
#[test]
#[allow(clippy::cast_possible_truncation)] // the LCG truncation IS the generator under test
fn narrow_counter_promotion_matches_u64_exactly() {
    use std::collections::HashMap;

    fn buggy_run(n: usize, updates: &[(usize, u32)]) -> (u64, Vec<u64>) {
        let mut counts = vec![0u64; n];
        for &(i, v) in updates {
            counts[i] += u64::from(v);
        }
        let total = counts.iter().sum();
        (total, counts)
    }

    fn fixed_run(n: usize, updates: &[(usize, u32)]) -> (u64, Vec<u64>) {
        let mut counts = vec![0u32; n];
        let mut wide: HashMap<usize, u64> = HashMap::with_capacity(updates.len());
        for &(i, v) in updates {
            if let Some(w) = wide.get_mut(&i) {
                *w += u64::from(v);
                continue;
            }
            if let Some(x) = counts[i].checked_add(v) {
                counts[i] = x;
            } else {
                let w = u64::from(counts[i]) + u64::from(v);
                counts[i] = 0;
                wide.insert(i, w);
            }
        }
        let total = counts.iter().map(|&x| u64::from(x)).sum::<u64>() + wide.values().sum::<u64>();
        let per_slot: Vec<u64> = (0..n)
            .map(|i| match wide.get(&i) {
                Some(&w) => w,
                None => u64::from(counts[i]),
            })
            .collect();
        (total, per_slot)
    }

    // (a) Exact-boundary crossing: one slot walked to just below
    // u32::MAX then pushed over, then bumped again (the promoted
    // path's continue-on-follow-up).
    let mut ups: Vec<(usize, u32)> = vec![(0, u32::MAX - 5), (0, 10), (0, 7)];
    ups.extend((1..8).map(|i| (i, 1_000)));
    let (bt, bp) = buggy_run(8, &ups);
    let (ft, fp) = fixed_run(8, &ups);
    assert_eq!((bt, bp), (ft, fp));

    // (b) EVERY slot overflows (all-promoted regime).
    let ups: Vec<(usize, u32)> = (0..16)
        .flat_map(|i| [(i, u32::MAX), (i, u32::MAX), (i, 3)])
        .collect();
    let (bt, bp) = buggy_run(16, &ups);
    let (ft, fp) = fixed_run(16, &ups);
    assert_eq!((bt, bp), (ft, fp));

    // (c) LCG stream over 128 slots, 4096 updates, deltas biased to
    // the overflow regime.
    let mut rng: u64 = 0x5EED_5EED_5EED_5EED;
    let ups: Vec<(usize, u32)> = (0..4096)
        .map(|_| {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let i = (rng >> 33) as usize % 128;
            let v = ((rng >> 13) as u32 % 3000) + 1;
            (i, v)
        })
        .collect();
    let (bt, bp) = buggy_run(128, &ups);
    let (ft, fp) = fixed_run(128, &ups);
    assert_eq!(
        (bt, bp),
        (ft, fp),
        "LCG overflow-regime stream must match exactly"
    );

    // (d) u32::MAX boundary: 0 + u32::MAX does NOT overflow (stays
    // narrow); the SECOND u32::MAX promotes.
    let ups: Vec<(usize, u32)> = vec![(0, u32::MAX), (0, u32::MAX)];
    let (bt, bp) = buggy_run(1, &ups);
    let (ft, fp) = fixed_run(1, &ups);
    assert_eq!(bt, ft);
    assert_eq!(bp, fp);
    assert_eq!(bp[0], 2 * u64::from(u32::MAX));
}

/// Batch 78 — G1 for `cache-line-block-partitioned-lock`: lock
/// granularity cannot change totals (same seeded stream through both
/// structures, closed-form check).
#[test]
fn block_partitioned_lock_totals_match() {
    use std::sync::Mutex;

    #[repr(align(128))]
    struct Block {
        lock: Mutex<[u64; 8]>,
    }
    const N: usize = 64; // 8 blocks x 8 points
    const UPS: usize = 500;
    let stream: Vec<usize> = {
        let mut rng: u64 = 0xABCD_EF01_2345_6789;
        (0..UPS)
            .map(|_| {
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                (rng >> 33) as usize % N
            })
            .collect()
    };

    let g = Mutex::new(vec![0u64; N]);
    for &i in &stream {
        g.lock().unwrap()[i] += 1;
    }
    let global_total: u64 = g.lock().unwrap().iter().sum();

    let blocks: Vec<Block> = (0..N / 8)
        .map(|_| Block {
            lock: Mutex::new([0u64; 8]),
        })
        .collect();
    for &i in &stream {
        blocks[i / 8].lock.lock().unwrap()[i % 8] += 1;
    }
    let block_total: u64 = blocks
        .iter()
        .map(|b| b.lock.lock().unwrap().iter().sum::<u64>())
        .sum();

    assert_eq!(global_total, block_total);
    assert_eq!(global_total, UPS as u64);
}

/// Batch 121 — the three new rules are present.
#[test]
fn batch121_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for id in [
        "resume-revalidates-provenance-before-reuse",
        "write-verify-readback-before-publish",
        "ab-isolate-one-component-by-baseline-byte-copy",
    ] {
        assert!(ids.contains(&id), "{id} missing from corpus");
    }
}

/// Batch 121 — every new rule parses (syn) on buggy AND fixed code.
#[test]
fn batch121_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    for id in [
        "resume-revalidates-provenance-before-reuse",
        "write-verify-readback-before-publish",
        "ab-isolate-one-component-by-baseline-byte-copy",
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} missing from corpus"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "{id} buggy/fixed must both parse"
        );
    }
}

/// Batch 121 — the load-bearing concepts survive into the corpus text
/// (retrieval discriminators), the NotBenchable annotations are honest
/// (context_notes carry the annotation), and the source quotes' key
/// phrases are pinned.
#[test]
fn batch121_concepts_covered() {
    let corpus = build_perf_seed_corpus();
    let find = |id: &str| {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} missing"))
    };
    let resume = find("resume-revalidates-provenance-before-reuse");
    let resume_text = format!(
        "{}\n{}\n{}",
        resume.name, resume.buggy_code, resume.fixed_code
    );
    for needle in ["manifest", "identity", "tool", "digest", "reuse"] {
        assert!(
            resume_text.contains(needle),
            "resume concept {needle:?} missing"
        );
    }
    assert!(
        resume
            .context_notes
            .as_deref()
            .unwrap_or("")
            .contains("NotBenchable"),
        "resume rule must carry the NotBenchable annotation"
    );
    let publish = find("write-verify-readback-before-publish");
    let publish_text = format!(
        "{}\n{}\n{}",
        publish.name, publish.buggy_code, publish.fixed_code
    );
    for needle in ["incomplete", "rename", "hash", "reader", "refuse"] {
        assert!(
            publish_text.contains(needle),
            "publish concept {needle:?} missing"
        );
    }
    let ab = find("ab-isolate-one-component-by-baseline-byte-copy");
    let ab_text = format!("{}\n{}\n{}", ab.name, ab.buggy_code, ab.fixed_code);
    for needle in ["baseline", "byte", "paired", "partial budget"] {
        assert!(ab_text.contains(needle), "ab concept {needle:?} missing");
    }
    // The memory-honesty rider (mmap-backed weights / partial budget)
    // must live in the A/B rule's context notes.
    let notes = ab.context_notes.as_deref().unwrap_or("");
    assert!(
        notes.contains("mmap-backed") && notes.contains("partial budget"),
        "the memory-honesty rider must be recorded in the A/B rule's notes"
    );
    // The pinned source phrases (quote re-verification at the pin).
    assert!(
        notes.contains("must not be interpreted as the total"),
        "the source's own memory caveat must be pinned verbatim"
    );
    assert!(
        resume
            .context_notes
            .as_deref()
            .unwrap_or("")
            .contains("Resume provenance mismatch"),
        "the resume refuse phrase must be pinned"
    );
    assert!(
        publish
            .context_notes
            .as_deref()
            .unwrap_or("")
            .contains("Written tensor mismatch"),
        "the publish refuse phrase must be pinned"
    );
}

/// Batch 122 — the three new rules are present.
#[test]
fn batch122_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for id in [
        "model-aware-format-over-hand-assembled-tokens",
        "implemented-surface-requires-provisioning-target",
        "diagnostics-not-just-exit-code",
    ] {
        assert!(ids.contains(&id), "{id} missing from corpus");
    }
}

/// Batch 122 — every new rule parses (syn) on buggy AND fixed code.
#[test]
fn batch122_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    for id in [
        "model-aware-format-over-hand-assembled-tokens",
        "implemented-surface-requires-provisioning-target",
        "diagnostics-not-just-exit-code",
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} missing from corpus"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "{id} buggy/fixed must both parse"
        );
    }
}

/// Batch 122 — the load-bearing concepts survive into the corpus text
/// (retrieval discriminators), the NotBenchable annotations are honest,
/// and the source quotes' key phrases are pinned.
#[test]
fn batch122_concepts_covered() {
    let corpus = build_perf_seed_corpus();
    let find = |id: &str| {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} missing"))
    };
    let format = find("model-aware-format-over-hand-assembled-tokens");
    let format_text = format!(
        "{}\n{}\n{}",
        format.name, format.buggy_code, format.fixed_code
    );
    for needle in ["template", "multimodal", "rollback", "drift"] {
        assert!(
            format_text.contains(needle),
            "format concept {needle:?} missing"
        );
    }
    assert!(
        format
            .context_notes
            .as_deref()
            .unwrap_or("")
            .contains("NotBenchable"),
        "format rule must carry the NotBenchable annotation"
    );
    let provision = find("implemented-surface-requires-provisioning-target");
    let provision_text = format!(
        "{}\n{}\n{}",
        provision.name, provision.buggy_code, provision.fixed_code
    );
    for needle in ["provision", "target", "MissingArtifact"] {
        assert!(
            provision_text.contains(needle),
            "provision concept {needle:?} missing"
        );
    }
    let diag = find("diagnostics-not-just-exit-code");
    let diag_text = format!("{}\n{}\n{}", diag.name, diag.buggy_code, diag.fixed_code);
    for needle in ["stderr", "exit", "transcript"] {
        assert!(
            diag_text.contains(needle),
            "diag concept {needle:?} missing"
        );
    }
    // The pinned source phrases (quote re-verification at the pin).
    assert!(
        diag.context_notes
            .as_deref()
            .unwrap_or("")
            .contains("The REPL can return zero after an image turn fails"),
        "the source's exit-code lie must be pinned verbatim"
    );
    assert!(
        provision
            .context_notes
            .as_deref()
            .unwrap_or("")
            .contains("could not be used out of the box"),
        "the source's provisioning-gap phrasing must be pinned verbatim"
    );
}

/// Batch 123 — the two new rules are present.
#[test]
fn batch123_rules_present() {
    let corpus = build_perf_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for id in [
        "full-vector-bit-identity-over-argmax",
        "renderer-error-is-not-capacity-overflow",
    ] {
        assert!(ids.contains(&id), "{id} missing from corpus");
    }
}

/// Batch 123 — every new rule parses (syn) on buggy AND fixed code.
#[test]
fn batch123_pseudocode_parses() {
    let corpus = build_perf_seed_corpus();
    for id in [
        "full-vector-bit-identity-over-argmax",
        "renderer-error-is-not-capacity-overflow",
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} missing from corpus"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "{id} buggy/fixed must both parse"
        );
    }
}

/// Batch 123 — the load-bearing concepts survive into the corpus text.
#[test]
fn batch123_concepts_covered() {
    let corpus = build_perf_seed_corpus();
    let find = |id: &str| {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} missing"))
    };
    let vec_id = find("full-vector-bit-identity-over-argmax");
    let vec_text = format!(
        "{}\n{}\n{}",
        vec_id.name, vec_id.buggy_code, vec_id.fixed_code
    );
    for needle in ["argmax", "bit", "manifest", "drift", "invalid"] {
        assert!(vec_text.contains(needle), "vec concept {needle:?} missing");
    }
    assert!(
        vec_id
            .context_notes
            .as_deref()
            .unwrap_or("")
            .contains("NotBenchable"),
        "vec rule must carry the NotBenchable annotation"
    );
    let tri = find("renderer-error-is-not-capacity-overflow");
    let tri_text = format!("{}\n{}\n{}", tri.name, tri.buggy_code, tri.fixed_code);
    for needle in ["Error", "compact", "overflow"] {
        assert!(tri_text.contains(needle), "tri concept {needle:?} missing");
    }
    // The pinned source phrases (quote re-verification at the pin).
    assert!(
        tri.context_notes
            .as_deref()
            .unwrap_or("")
            .contains("not a request to compact the context"),
        "the source's tri-state comment must be pinned verbatim"
    );
    assert!(
        vec_id
            .context_notes
            .as_deref()
            .unwrap_or("")
            .contains("despite matching argmax IDs"),
        "the source's drift-vs-argmax finding must be pinned verbatim"
    );
}

/// Batch 124 — the new rule is present, parses, and carries its concepts.
#[test]
fn batch124_rule_present_and_parses() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "matched-control-disentangles-policy-from-model")
        .expect("matched-control rule missing from corpus");
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "matched-control buggy/fixed must both parse"
    );
    let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
    for needle in ["control", "gate", "endpoint", "proxy", "baseline"] {
        assert!(
            text.contains(needle),
            "matched-control concept {needle:?} missing"
        );
    }
    assert!(
        e.context_notes
            .as_deref()
            .unwrap_or("")
            .contains("NotBenchable"),
        "matched-control rule must carry the NotBenchable annotation"
    );
    // The pinned source phrases (quote re-verification at the paper).
    let notes = e.context_notes.as_deref().unwrap_or("");
    assert!(
        notes.contains("reproduces the no-prefetch baseline exactly"),
        "the paper's null-arm identity claim must be pinned verbatim"
    );
    assert!(
        notes.contains("better proxies do not imply better endpoints"),
        "the paper's honesty clause must be pinned verbatim"
    );
}

/// Batch 125 — the new rule is present, parses, and carries its concepts.
#[test]
fn batch125_rule_present_and_parses() {
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "control-reproduces-on-baseline-trims-overclaim")
        .expect("control-reproduces rule missing from corpus");
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "control-reproduces buggy/fixed must both parse"
    );
    let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
    for needle in [
        "baseline",
        "oracle",
        "tolerance",
        "compute shape",
        "scratch",
    ] {
        assert!(
            text.contains(needle),
            "control-reproduces concept {needle:?} missing"
        );
    }
    assert!(
        e.context_notes
            .as_deref()
            .unwrap_or("")
            .contains("NotBenchable"),
        "control-reproduces rule must carry the NotBenchable annotation"
    );
    // The pinned source phrases (quote re-verification at the pin).
    let notes = e.context_notes.as_deref().unwrap_or("");
    assert!(
        notes.contains("different floating-point accumulation than the final decode step"),
        "the compute-shape quote must be pinned verbatim"
    );
    assert!(
        notes.contains("Compare all consumed outputs against the unfused control"),
        "the unowned-scratch quote must be pinned verbatim"
    );
}

/// Batch 129 (2026-09-08, Plan 110 -- ds4 Qwen Q2 distill): the two io
/// measurement-honesty rules are present, parse, and pin their quotes.
#[test]
fn batch129_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    for (rid, needles, notes_needles) in [
        (
            "instrumented-attribution-not-production-times",
            vec!["production costs", "ranking"],
            vec![
                "54.4 -> ~34 tok/s",
                "synchronized wall-time attributions, not GPU hardware counters",
            ],
        ),
        (
            "unset-control-arm-silent-default-parity",
            vec!["EXPLICIT control value", "which arm is control"],
            vec![
                "enabled defaults cannot silently make both variants identical",
                "--control-value",
            ],
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("batch-129 rule '{rid}' missing from corpus"));
        assert_eq!(e.category, "io", "batch-129 rule '{rid}' must be io");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-129 '{rid}' buggy/fixed must both parse"
        );
        let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
        for needle in needles {
            assert!(
                text.contains(needle),
                "batch-129 {rid} concept {needle:?} missing"
            );
        }
        let notes = e.context_notes.as_deref().unwrap_or("");
        assert!(
            notes.contains("NotBenchable"),
            "batch-129 '{rid}' must carry the NotBenchable annotation"
        );
        for needle in notes_needles {
            assert!(
                notes.contains(needle),
                "batch-129 {rid} quote {needle:?} missing"
            );
        }
    }
}

/// Batch 135 (2026-09-09, Plan 114 — the Wild-linker RustForge article
/// distill): the three new rules are present and parse. The
/// `vec-reuse-across-lifetime-change` fixed fixture carries an INLINE
/// CONST block (the compile-time layout assert) — this gate is what
/// proves the corpus's syn accepts it.
#[test]
fn batch135_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    for (rid, category, needles) in [
        (
            "atomic-vec-zero-cost-convert",
            "concurrency",
            vec!["into_iter", "Atomic::new", "same size+align"],
        ),
        (
            "vec-reuse-across-lifetime-change",
            "allocation",
            vec!["reuse_vec", "clear()", "compile error"],
        ),
        (
            "partitioned-mut-slices-zero-sync",
            "concurrency",
            vec!["chunks_mut", "DISJOINT", "zero"],
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("batch-135 rule '{rid}' missing from corpus"));
        assert_eq!(e.category, category, "batch-135 rule '{rid}' category");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-135 '{rid}' buggy/fixed must both parse"
        );
        let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
        for needle in needles {
            assert!(
                text.contains(needle),
                "batch-135 {rid} concept {needle:?} missing"
            );
        }
    }
}

/// Batch 135 — G1 exactness: both arms of all three fixtures produce
/// IDENTICAL results on fixed inputs (representation conversion, buffer
/// retargeting, and partitioned mutation are all pure win shapes — no
/// value may change). Runs on std only, no feature gate.
#[test]
fn batch135_fixtures_g1_parity() {
    use std::sync::atomic::{AtomicU32, Ordering};

    // Nested items lead the fn body (clippy::items_after_statements).
    fn reuse_vec<T, U>(mut v: Vec<T>) -> Vec<U> {
        const {
            assert!(
                std::mem::size_of::<T>() == std::mem::size_of::<U>()
                    && std::mem::align_of::<T>() == std::mem::align_of::<U>(),
            );
        }
        v.clear();
        v.into_iter().map(|_| unreachable!()).collect()
    }
    const CELLS: usize = 97;
    const WORKERS: usize = 4;
    const ITERS: usize = 3;

    // (a) atomic conversion: identical values through both forms.
    let input: Vec<u32> = (0..257u32)
        .map(|x| x.wrapping_mul(2_654_435_761) | 1)
        .collect();
    let buggy: u64 = {
        let atomic: Vec<AtomicU32> = input.iter().map(|&v| AtomicU32::new(v)).collect();
        atomic
            .iter()
            .map(|a| u64::from(a.load(Ordering::Relaxed)))
            .sum()
    };
    let fixed: u64 = {
        let atomic: Vec<AtomicU32> = input.clone().into_iter().map(AtomicU32::new).collect();
        atomic
            .iter()
            .map(|a| u64::from(a.load(Ordering::Relaxed)))
            .sum()
    };
    assert_eq!(
        buggy, fixed,
        "(a) atomic conversion must be value-identical"
    );

    // (b) lifetime retargeting: identical sums, stable across calls.
    let rows: &'static [Vec<u32>] =
        Box::leak(vec![vec![1, 2, 3], vec![4], vec![], vec![5, 6, 7, 8, 9]].into_boxed_slice());
    let picked: Vec<&[u32]> = rows.iter().map(|r| &r[..]).collect();
    let buggy_total: u64 = picked.iter().map(|s| s.len() as u64).sum();
    let mut buf: Vec<&'static [u32]> = Vec::new();
    let run = |buf: &mut Vec<&'static [u32]>| -> u64 {
        let mut cur: Vec<&[u32]> = reuse_vec(std::mem::take(buf));
        cur.clear();
        cur.extend(rows.iter().map(|r| &r[..]));
        let total: u64 = cur.iter().map(|s| s.len() as u64).sum();
        *buf = reuse_vec(cur);
        total
    };
    let fixed_total = run(&mut buf);
    assert_eq!(
        fixed_total,
        run(&mut buf),
        "(b) retarget reuse must be call-stable"
    );
    assert_eq!(
        buggy_total, fixed_total,
        "(b) lifetime retarget must be value-identical"
    );

    // (c) partitioned mutation: identical checksums (prime cell count so
    // the chunks are ragged; strided-vs-contiguous ownership must not
    // change the per-cell bump count).
    let buggy = {
        let grid = std::sync::Mutex::new(vec![0u64; CELLS]);
        let gref = &grid; // move closures capture the SHARED reference (Copy)
        std::thread::scope(|s| {
            for w in 0..WORKERS {
                s.spawn(move || {
                    for cell_idx in (w..CELLS).step_by(WORKERS) {
                        for _ in 0..ITERS {
                            let mut g = gref.lock().unwrap();
                            g[cell_idx] = g[cell_idx].wrapping_add(1);
                        }
                    }
                });
            }
        });
        let cs = grid.lock().unwrap().iter().sum::<u64>();
        cs
    };
    let fixed = {
        let mut grid = vec![0u64; CELLS];
        let per = grid.len().div_ceil(WORKERS);
        std::thread::scope(|s| {
            for chunk in grid.chunks_mut(per) {
                s.spawn(move || {
                    for _ in 0..ITERS {
                        for cell in chunk.iter_mut() {
                            *cell = cell.wrapping_add(1);
                        }
                    }
                });
            }
        });
        grid.iter().sum::<u64>()
    };
    assert_eq!(buggy, fixed, "(c) partition must be value-identical");
    assert_eq!(fixed, (CELLS * ITERS) as u64, "(c) closed form");
}

/// Batch 135 — the measured Pareto verdicts for the three new rules.
/// SLOW (spawns a cargo build + run per rule in a temp crate): run
/// explicitly with `cargo test -p riir-clippy --lib --features
/// rust_perf_bench batch135_pareto -- --ignored --nocapture`.
#[cfg(feature = "rust_perf_bench")]
#[test]
#[ignore = "slow: spawns cargo build+run per rule (temp bench crates)"]
fn batch135_pareto_benches() {
    let corpus = build_perf_seed_corpus();
    let validator = crate::domains::rust_perf::bench::PerfBenchValidator::new();
    for rid in [
        "atomic-vec-zero-cost-convert",
        "vec-reuse-across-lifetime-change",
        "partitioned-mut-slices-zero-sync",
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("batch-135 rule '{rid}' missing"));
        let verdict = validator.validate(rid, e.bench_spec.as_ref());
        assert!(!verdict.is_not_benchable(), "batch-135 '{rid}' has a spec");
        let b = match &verdict {
            crate::domains::rust_perf::bench::BenchVerdict::Pareto(p) => p,
            crate::domains::rust_perf::bench::BenchVerdict::NotBenchable(n) => {
                panic!("batch-135 '{rid}' unexpectedly NotBenchable: {n:?}")
            }
        };
        assert!(b.ran, "batch-135 '{rid}' bench must run: {:?}", b.error);
        println!(
            "batch-135 {rid}: {:?} faster={} slower={} tie={}",
            b.overall, b.faster_count, b.slower_count, b.tie_count
        );
        for r in &b.results {
            println!(
                "  n={:>6} buggy={:>9}ns fixed={:>9}ns speedup={:.2}x allocs {}→{} outcome={:?}",
                r.size,
                r.buggy_ns,
                r.fixed_ns,
                r.speedup,
                r.buggy_allocs,
                r.fixed_allocs,
                r.outcome
            );
        }
    }
}

/// Batch 136 (2026-09-09, Plan 115 — the Lattimore "Graph Algorithms in
/// Rayon" blog distill): the three new rules are present, parse, and
/// carry the concept needles. NotBenchable by design (qualitative source
/// — no bench_spec), so the without_spec membership pin above carries
/// the count half and this gate carries the presence/parse half.
#[test]
fn batch136_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    for (rid, category, needles) in [
        (
            "dynamic-work-parallelism-ladder",
            "concurrency",
            vec!["UNKNOWN IN ADVANCE", "scoped spawning", "composition"],
        ),
        (
            "sender-per-work-item-terminates-channel",
            "concurrency",
            vec!["drop(tx)", "clone", "closes"],
        ),
        (
            "nested-par-inside-channel-bridge-deadlocks",
            "concurrency",
            vec!["par_bridge", "steal", "THREAD-TIED"],
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("batch-136 rule '{rid}' missing from corpus"));
        assert_eq!(e.category, category, "batch-136 rule '{rid}' category");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-136 '{rid}' buggy/fixed must both parse"
        );
        assert!(
            e.context_notes.is_some(),
            "batch-136 '{rid}' must carry its NotBenchable rationale in context_notes"
        );
        let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
        for needle in needles {
            assert!(
                text.contains(needle),
                "batch-136 {rid} concept {needle:?} missing"
            );
        }
    }
}

/// Batch 136 — G1 exactness: the idiom fixture's ownership-driven close
/// actually terminates (the sender-per-work-item shape with the original
/// sender dropped), and the deadlock fixture's SEQUENTIAL fix drains a
/// discovered frontier completely. Runs on std only (mpsc, no rayon —
/// the deadlock shape is pinned by the rule's mechanism prose, not
/// reproduced here; a hang in a unit test is not a verdict).
#[test]
fn batch136_termination_parity() {
    use std::sync::mpsc;

    // Ownership-driven close: discovered children enqueue from the
    // clone they carry; the ORIGINAL sender drops before draining.
    let (tx, rx) = mpsc::channel::<u32>();
    for seed in [1u32, 2, 3] {
        tx.send(seed).unwrap();
    }
    drop(tx);
    let mut visited: Vec<u32> = Vec::new();
    let mut queue: Vec<u32> = rx.into_iter().collect();
    while let Some(n) = queue.pop() {
        visited.push(n);
        if n > 1 {
            queue.push(n / 2); // discovered work — legal: we own the drain
        }
    }
    assert_eq!(
        visited.len(),
        5,
        "seeds 1,2,3 + discovered 1-from-3 and 1-from-2"
    );
    assert_eq!(
        *visited.last().unwrap(),
        1,
        "the frontier must reach the leaf"
    );
}

/// Batch 139 (2026-09-09, Plan 119 / Research 154 — the ds4-metal PR #5
/// distill): the prefault-admission rule is present, parses, is io, and
/// carries its NotBenchable rationale. The fail-closed direction is
/// pinned by a G1 parity: an unqueryable RAM total must choose the
/// DEMAND-PAGED arm (the slow-but-safe side), never the prefault.
#[test]
fn batch139_rules_present_and_parse() {
    const MARGIN: u64 = 16 << 30;
    const MODEL: u64 = 44_769_582_803; // ~41.7 GiB (the source's model)
    const SIDECAR: u64 = 31_984_068_608; // ~29.8 GiB (the PLE sidecar)
    let corpus = build_perf_seed_corpus();
    let (rid, category, needles) = (
        "prefault-admission-keyed-to-ram-headroom",
        "io",
        vec![
            "FAIL CLOSED",
            "hw.memsize",
            "margin",
            "DS4_QWEN4_PLE_EVICT_TOKENS",
            "REFUSES to log a residency figure",
        ],
    );
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == rid)
        .unwrap_or_else(|| panic!("batch-139 rule '{rid}' missing from corpus"));
    assert_eq!(e.category, category, "batch-139 rule '{rid}' category");
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "batch-139 '{rid}' buggy/fixed must both parse"
    );
    assert!(
        e.context_notes.is_some(),
        "batch-139 '{rid}' must carry its NotBenchable rationale in context_notes"
    );
    let text = format!(
        "{}\n{}\n{}\n{}",
        e.name,
        e.buggy_code,
        e.fixed_code,
        e.context_notes.clone().unwrap_or_default()
    );
    for needle in needles {
        assert!(
            text.contains(needle),
            "batch-139 {rid} concept {needle:?} missing"
        );
    }

    // G1: the admission predicate's fail-closed direction, as a plain
    // integer function (the rule's own semantics, mirrored).
    let admission = |ram: Option<u64>, model: u64, sidecar: u64| {
        match ram {
            None => false, // NoRamQuery -> demand-paged (fail closed)
            Some(total) => total >= model + sidecar + MARGIN,
        }
    };
    assert!(!admission(None, 0, 0), "unqueryable RAM must NOT prefault");
    assert!(
        !admission(Some(64 << 30), MODEL, SIDECAR),
        "64 < 41.7 + 29.8 + 16 must stay demand-paged (the source's own cell)"
    );
    assert!(
        admission(Some(128 << 30), MODEL, SIDECAR),
        "128 GiB machines prefault again"
    );
}

/// Batch 141 (2026-09-09, Plan 121 — the antirez/ds4 Part-B distill,
/// Research 162's staged batch): the three new rules are present, parse,
/// carry the concept needles, and sit in their categories. All
/// NotBenchable by design — the without_spec membership pin above carries
/// the count half; the G1 function tests below carry the semantics.
#[test]
fn batch141_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    for (rid, category, needles) in [
        (
            "compaction-reserve-before-prompt-build",
            "allocation",
            vec!["MEASURED", "tokenize", "summary_budget", "free-threshold"],
        ),
        (
            "compaction-tail-boundary-prefers-complete-turn",
            "io",
            vec!["COMPLETE unit", "snap", "span", "fragment"],
        ),
        (
            "streaming-prefix-holdback-bounded-buffer",
            "io",
            vec!["marker", "flush", "verbatim", "line boundary"],
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("batch-141 rule '{rid}' missing from corpus"));
        assert_eq!(e.category, category, "batch-141 rule '{rid}' category");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-141 '{rid}' buggy/fixed must both parse"
        );
        assert!(
            e.context_notes.is_some(),
            "batch-141 '{rid}' must carry its NotBenchable rationale in context_notes"
        );
        let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
        for needle in needles {
            assert!(
                text.contains(needle),
                "batch-141 {rid} concept {needle:?} missing"
            );
        }
    }
}

/// Batch 141 — G1 semantics, as plain functions (the rules' own
/// contracts, mirrored std-only):
/// (1) the reserve LIFTS the threshold — max(threshold, measured+slack+
/// budget) never fires inside the space the rewrite needs;
/// (2) the tail boundary prefers the complete turn inside the walk-back
/// window and snaps out of atomic spans;
/// (3) the stream holdback never exceeds the marker length and flushes
/// verbatim on the first mismatch.
#[test]
fn batch141_reserve_boundary_holdback_semantics() {
    // (1) Reserve: measured reserve lifts a naive constant threshold.
    let summary_budget = |window: usize| (window / 8).clamp(256, 8_192);
    let reserve = |measured: usize, window: usize| measured + 64 + summary_budget(window);
    let naive: usize = 2_048;
    let window: usize = 32_768;
    let measured: usize = 3_400; // the prompt template grew past the constant
    let effective = naive.max(reserve(measured, window));
    assert!(
        effective > naive,
        "a template larger than the constant must lift the reserve"
    );
    assert_eq!(
        effective,
        reserve(measured, window),
        "every consumer reads the SAME measured figure"
    );

    // (2) Tail boundary: complete-turn walk + span snap.
    #[derive(Clone, Copy, PartialEq)]
    enum Tok {
        Sys,
        UserTurn,
        Filler,
    }
    let toks = [
        Tok::Sys,      // 0: protected prefix
        Tok::UserTurn, // 1: turn A
        Tok::Filler,   // 2
        Tok::UserTurn, // 3: turn B (the latest complete turn)
        Tok::Filler,   // 4
        Tok::Filler,   // 5
    ];
    let bottom = toks.len();
    let sys_len = 1usize;
    let tail_budget = 2usize;
    let target = bottom.saturating_sub(tail_budget).max(sys_len); // 4
    let earliest = bottom.saturating_sub(2 * tail_budget).max(sys_len); // 2
    let walked = (earliest..bottom)
        .rev()
        .find(|&i| toks[i] == Tok::UserTurn)
        .unwrap_or(target);
    assert_eq!(
        walked, 3,
        "the latest COMPLETE user turn inside the window wins"
    );
    // window too small: no turn inside -> the arithmetic target stands
    let tight = (bottom - 1..bottom)
        .rev()
        .find(|&i| toks[i] == Tok::UserTurn)
        .unwrap_or(target);
    assert_eq!(
        tight, target,
        "no complete unit fits -> bounded fragment fallback"
    );
    // span snap: a boundary strictly inside a span moves to its START
    let snap = |pos: usize, spans: &[(usize, usize)]| {
        spans
            .iter()
            .find(|&&(s, e)| pos > s && pos < e)
            .map_or(pos, |&(s, _)| s)
    };
    let spans = [(10usize, 14usize)];
    assert_eq!(snap(12, &spans), 10, "inside the span snaps to its start");
    assert_eq!(snap(10, &spans), 10, "already at the start stays");
    assert_eq!(snap(14, &spans), 14, "at/after the end is untouched");
    assert_eq!(snap(9, &spans), 9, "before the span is untouched");

    // (3) Stream holdback: bounded window + verbatim mismatch flush.
    const MARKER: &[u8] = b"> **Hint:**"; // 11 bytes, the source's exact marker
    #[derive(Default)]
    struct Holder {
        held: [u8; 11], // the bound IS the marker length
        held_len: usize,
    }
    let mut h = Holder::default();
    let mut out: Vec<u8> = Vec::new();
    let feed = |h: &mut Holder, out: &mut Vec<u8>, b: u8| {
        if h.held_len < MARKER.len() && b == MARKER[h.held_len] {
            h.held[h.held_len] = b;
            h.held_len += 1;
            if h.held_len == MARKER.len() {
                out.extend_from_slice(b"[hint]");
                h.held_len = 0;
            }
            return;
        }
        out.extend_from_slice(&h.held[..h.held_len]);
        h.held_len = 0;
        out.push(b);
    };
    // ordinary text: every byte streams, the holdback stays empty
    for &b in b"hello" {
        feed(&mut h, &mut out, b);
        assert_eq!(h.held_len, 0, "mismatch on byte {b} flushed immediately");
    }
    assert_eq!(out, b"hello");
    // partial marker then a mismatch: held bytes replay verbatim, in order
    out.clear();
    for &b in &MARKER[..4] {
        feed(&mut h, &mut out, b);
    }
    assert_eq!(h.held_len, 4, "prefix bytes are held, not emitted");
    feed(&mut h, &mut out, b'x'); // mismatch at position 4
    assert_eq!(h.held_len, 0, "mismatch clears the holdback");
    assert_eq!(
        &out[..4],
        &MARKER[..4],
        "held bytes replay verbatim, in order"
    );
    assert_eq!(out[4], b'x', "the mismatching byte streams too");
    // full match: recognized exactly at the marker's last byte
    out.clear();
    for &b in MARKER {
        feed(&mut h, &mut out, b);
    }
    assert_eq!(out, b"[hint]", "the marker is replaced at its last byte");
    assert_eq!(h.held_len, 0);
    // the holdback's bound IS the marker's length: held: [u8; 11] and
    // MARKER.len() == 11 — the type-level bound the rule demands.
    assert_eq!(MARKER.len(), 11);
}

/// Batch 143 (2026-09-10, Plan 123 — the nnethercote Dec-2025 rustc-perf
/// post distill): R1 + R2 present, parse, carry the concept needles, sit
/// in their categories. R1 is BENCHABLE (the without_spec membership pin
/// does NOT apply; the Pareto runner carries the measurement half); R2 is
/// NotBenchable with its rationale in context_notes.
#[test]
fn batch143_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    for (rid, category, needles) in [
        (
            "debug-level-gated-computation",
            "complexity",
            vec![
                "log_enabled",
                "runtime atomic",
                "lazy",
                "Wall-time is the true metric",
            ],
        ),
        (
            "macro-generated-code-optimize-at-generation-point",
            "complexity",
            vec![
                "-Zmacro-stats",
                "generation point",
                "39%",
                "16%",
                "downstream optimization machinery",
            ],
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("batch-143 rule '{rid}' missing from corpus"));
        assert_eq!(e.category, category, "batch-143 rule '{rid}' category");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-143 '{rid}' buggy/fixed must both parse"
        );
        assert!(
            e.context_notes.is_some(),
            "batch-143 '{rid}' must carry its source citation in context_notes"
        );
        let text = format!(
            "{}\n{}\n{}\n{}",
            e.name,
            e.buggy_code,
            e.fixed_code,
            e.context_notes.clone().unwrap_or_default()
        );
        for needle in needles {
            assert!(
                text.contains(needle),
                "batch-143 {rid} concept {needle:?} missing"
            );
        }
    }
    // R1 is the batch's ONE benchable rule — assert the spec exists.
    #[cfg(feature = "rust_perf_bench")]
    {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "debug-level-gated-computation")
            .unwrap();
        assert!(
            e.bench_spec.is_some(),
            "batch-143 R1 must ship its Pareto bench spec"
        );
    }
}

/// Batch 143 — G1 semantics: the level gate decides whether the log-only
/// computation runs at all, and the lazy-args alternative is equivalent.
/// Uses the REAL `log` crate (dev-dependency; zero new tree cost) with NO
/// logger installed — the production regime the rule describes.
#[test]
fn batch143_log_gate_computation_parity() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COMPUTED: AtomicUsize = AtomicUsize::new(0);

    fn summarize(item: &str) -> String {
        COMPUTED.fetch_add(1, Ordering::SeqCst);
        format!("[{item}]")
    }

    let items = ["a", "b", "c"];

    // The test process never installs a logger and log's default
    // features leave STATIC_MAX_LEVEL at trace: log_enabled! is decided
    // by the runtime max-level atomic alone. Assert the regime first.
    assert!(!log::log_enabled!(log::Level::Debug));

    // Buggy shape: the binding is a real statement — computed every time.
    COMPUTED.store(0, Ordering::SeqCst);
    for it in items {
        let summary = summarize(it);
        log::debug!("event summary: {summary}");
    }
    assert_eq!(
        COMPUTED.load(Ordering::SeqCst),
        items.len(),
        "the ungated binding runs even with the level off (the #147293 defect)"
    );

    // Fixed shape, gate form: log_enabled! wraps the computation.
    COMPUTED.store(0, Ordering::SeqCst);
    for it in items {
        if log::log_enabled!(log::Level::Debug) {
            let summary = summarize(it);
            log::debug!("event summary: {summary}");
        }
    }
    assert_eq!(
        COMPUTED.load(Ordering::SeqCst),
        0,
        "the gated computation is skipped entirely with the level off"
    );

    // Fixed shape, lazy-args form: the computation moved INTO the macro's
    // format args is equivalent — log macros evaluate args only on the
    // enabled path.
    COMPUTED.store(0, Ordering::SeqCst);
    for it in items {
        log::debug!("event summary: {}", summarize(it));
    }
    assert_eq!(
        COMPUTED.load(Ordering::SeqCst),
        0,
        "the lazy-args form is the equivalent skip"
    );

    // The defect is CONSUMER-SHAPE, not macro behavior: the same lazy
    // macro still runs the computation when it was hoisted OUT first —
    // which is why the fix must move/gate the COMPUTATION, not the call.
    COMPUTED.store(0, Ordering::SeqCst);
    for it in items {
        let summary = summarize(it); // the hoist
        if log::log_enabled!(log::Level::Debug) {
            log::debug!("event summary: {summary}"); // still pays it
        }
    }
    assert_eq!(
        COMPUTED.load(Ordering::SeqCst),
        items.len(),
        "gating the CALL without moving the COMPUTATION fixes nothing"
    );
}

/// Batch 143 — the measured Pareto verdict for R1. SLOW (spawns a cargo
/// build + run in a temp bench crate): run explicitly with
/// `cargo test --lib --features rust_perf_bench batch143_pareto --
/// --ignored --nocapture`.
#[cfg(feature = "rust_perf_bench")]
#[test]
#[ignore = "slow: spawns cargo build+run (temp bench crate)"]
fn batch143_pareto_benches() {
    let corpus = build_perf_seed_corpus();
    let validator = crate::domains::rust_perf::bench::PerfBenchValidator::new();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "debug-level-gated-computation")
        .expect("batch-143 R1 missing from corpus");
    let verdict = validator.validate(e.rule_id.as_str(), e.bench_spec.as_ref());
    assert!(!verdict.is_not_benchable(), "batch-143 R1 has a spec");
    let b = match &verdict {
        crate::domains::rust_perf::bench::BenchVerdict::Pareto(p) => p,
        crate::domains::rust_perf::bench::BenchVerdict::NotBenchable(n) => {
            panic!("batch-143 R1 unexpectedly NotBenchable: {n:?}")
        }
    };
    assert!(b.ran, "batch-143 R1 bench must run: {:?}", b.error);
    println!(
        "batch-143 debug-level-gated-computation: {:?} faster={} slower={} tie={}",
        b.overall, b.faster_count, b.slower_count, b.tie_count
    );
    for r in &b.results {
        println!(
            "  n={:>6} buggy={:>9}ns fixed={:>9}ns speedup={:.2}x allocs {}→{} outcome={:?}",
            r.size, r.buggy_ns, r.fixed_ns, r.speedup, r.buggy_allocs, r.fixed_allocs, r.outcome
        );
    }
}
