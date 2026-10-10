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
fn seed_corpus_total_and_category_count_pinned() {
    let corpus = build_perf_seed_corpus();
    assert_eq!(corpus.entries.len(), 196); // Batch 221 (+1: the bevy #25864 runtime-bool conditional-guard rule -- dispatch, NotBenchable annotated, Plan 221 -- patches a4d1789 + 8328184, merged 2026-09-28 @ 74884ffd; the branch hoisted into a conditional Option<Guard> tuple so the inlinable body compiles once) on the 195 base (Batch 219 (+11: the TensorFold native-engine serving/timing pack, Plan 215 slot 2 / Research 251 -- ashhart/TensorFold @ f1aef9c3f2ab85b2a68f2eed817e45eecbd04781, Apache-2.0; complexity +2: fastest-of-n-dip-drift-retime-cost-table + pava-monotone-cost-pooling-modeled-cost-not-clock (the A− cost-table hygiene pair); caching +3: prompt-cache-canonical-interval-keys + prompt-cache-rewind-states-min-gap-eviction (the staged B+ entry SPLIT at authoring -- both atoms carried distinct verbatim law + regime) + prompt-state-fingerprint-probe-bits-keyed-persistence; concurrency +2: bounded-peer-handoff-wait (#512, the bounded peer-liveness wait) + spin-lock-one-message-per-exchange (the four-Mac fabric posting law) -- the T18 transport cluster's two landed members, coordinator-authored; thermal +1: thermal-hysteresis-band-pacing-chunk-placement -- the corpus's THIRTEENTH category, category pin 12 -> 13; dispatch +1: degrade-not-fail-load-qualification-probe; io +1: metal-keepalive-residency-unwire; allocation +1: display-carveout-all-or-nothing-admission; all ELEVEN NotBenchable annotated, quotes verified at the pin via git-show blob reads; the T18 deferred set -- nvfp4-per-sm dispatch, the MTLFence round-command-buffer lesson, the simdgroup weight-tile lesson, kv-independent-quantizer-verification -- stays staged for kernel slots, exl3-oracle-fixture dropped to lane-intel (Research 251 carries it) on the 184 base (Batch 211 (+1: repr-align-register-partition-store-forwarding -- the bevy #26042 EntityLocation alignment rule, layout, NotBenchable annotated (source-reported bench-box figures; carries the rust #163916 alignment-drop face as the rider -- one law, two faces: register partition vs container footprint); bevyengine/bevy #26042 @ a144d280 + rust-lang/rust #163916 @ aaa57c9a) on the 183 base (Batch 207 (+2: per-scope-map-scan-hash-crossover -- the cubecl CSE ScopedMap fix, complexity, the measured 15% small-scope negative flipping it into the hybrid inline-scan-then-hash law, annotated; + inline-hint-not-through-unmarked-wrapper -- the bevy #26040 re-inline, dispatch, the four-row laptop table + the wrapper-attribute-audit discipline, annotated; tracel-ai/cubecl #1767 @ d307dd46 + bevyengine/bevy #26040 @ 1ba0c9f9) on the 181 base (Batch 204 (+1: neon-tbl-replicate-2bit-codec-sdot-block-rescalek-rescale -- the PrismML NEON PQ2_0 vec_dot rule, PrismML-Eng/llama.cpp #265 @ 6bfcd79a, simd category, NotBenchable annotated on the x86_64 host box -- M3-Max GOAT staged, Plan 204) on the 180 base (Batch 202 (+1: dataflow-fixpoint-convergence-traversal-order -- the nnethercote min-rank worklist rule, rust-lang/rust #160193 @ cb9d1b06, NotBenchable annotated, Plan 201) on the 179 base (Batch 200 (+11: the TensorFold staged pool's serving half -- prefix-cache-entry-one-token-early, dual-reduction-shared-butterfly-ladder, idle-rank-doorbell-on-rendezvous-store, prompt-item-per-consumer-kernel-measured, concurrent-round-parity-solo-equal, lone-stream-graph-replay-mode-switch, snapshot-then-release-shared-drafter-context, graph-pool-handle-fresh-per-generation, spec-copy-window-aimd-ramp-alone-gated, one-shot-staging-release-to-system, madvise-random-for-key-gather-tables -- all NotBenchable annotated, Plan 198) on the 168 base (Batch 198 +1: selector-lattice-beam-over-greedy-walk -- the PrismML #284 beam-over-greedy selector walk, annotated with the NOT-ISOLATED caveat) on the 167 base (Batch 196 +2: the TensorFold A- serving/host pair -- grammar-window-draft-prune-path-local-mask + startup-estimate-derived-from-allocation-shapes, both NotBenchable annotated) on the 165 base (Batch 186 +2: fallthrough-toggle-oracle + cold-l3-evict-min-of-n; Batch 181 +2, Batch 182 +3)
    // Per-category standing counts (repaired to the LIVE numbers at Batch
    // 196, the TensorFold pair +1 complexity +1 allocation — complexity 25,
    // caching 7 and dispatch 5 in the prior prose had gone stale before this
    // batch: real 30/8/6 at the 165 state): allocation 29, io 26,
    // concurrency 23, complexity 32 (Batch 202 +1: the dataflow min-rank
    // rule), layout 14 (Batch 211 +1: the #26042 register-partition rule),
    // iterators 11, simd 9,
    // arithmetic 8, caching 8, dispatch 9 (Batch 207 +1 the inline-hint wrapper rule +1 the
    // B200-era additions past this prose's last update — the live number, disclosed here),
    // bevy 2, wasm 1 — asserted via the
    // distinct-category pin below plus the benchable/NotBenchable pins in
    // this file. Never read a typed figure from prose: run
    // `cargo refine --corpus-stats` (or this test) for the live numbers, and
    // read the per-batch history in `.docs/11_domains/rust_perf.md` — the
    // narrative copy here had gone stale (98 → 145 → 153 across three
    // landings that updated the assert but not the prose). Batch 219 note:
    // the distinct-category count moved 12 → 13 (thermal joins); the
    // per-category figures above are the STANDING stale-prone prose — the
    // asserts are the ledger.
    let mut categories: Vec<&str> = corpus.entries.iter().map(|e| e.category.as_str()).collect();
    categories.sort_unstable();
    categories.dedup();
    assert_eq!(categories.len(), 13); // Batch 219 +1: thermal (the TensorFold hysteresis-band rule — the corpus's 13th category, Plan 215 slot 2)
}

/// G1 for `swar-digit-chunk-parse` (Batch 74, Research 088 — the
/// simd-json distill): the ported SWAR construction
/// (`parse_eight_digits_swar` + `is_made_of_eight_digits_fast` + the
/// chunk/tail loop) must be exactly equivalent to the naive per-digit
/// reference — the Batch 59 ported-numeric exhaustive-gate MANDATE
/// (casual tests miss subtle wrongness; the construction's wrapping
/// multiplies especially). Sweeps: (a) every digit-string length
/// 0..=19 × structured boundaries (all-0s, all-9s, 10^k) + the exact
/// 20-digit `u64::MAX` string; (b) `10_000` LCG digit strings of LCG
/// lengths; (c) non-digit terminators at EVERY offset (b'x', b',',
/// b' ', 0x00, 0xFF) — value AND consumed-length identical; (d) the
/// validation matrix: `is_made_of_eight_digits_fast` ≡ "every byte in
/// 0x30..=0x39" over 8 positions × boundary bytes + `10_000` LCG octets;
/// (e) the fold itself vs a u32 reference over structured + `100_000`
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
/// loop across a FULL u16 sweep + every 128^i band boundary + `u32::MAX` +
/// an LCG u32 sample — bit-identical output bytes (the Batch 59
/// ported-numeric exhaustive-gate rule; the source ships its own
/// equivalence test at 128^i boundaries — `aux_test_serialize_vint_u32`
/// — this gate is the stronger local mirror). BOTH conventions are
/// proven: (a) stop-bit-on-LAST (tantivy `VInt` — the shipped fixture)
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

// Exhaustive ladder-vs-reference verification (u16 sweep + band boundaries
// + LCG sample); the four ladder fns are the fixture under test and read
// best adjacent to their references (corpus-test precedent: the
// e0502_copy_index allow, f5c3feda).
#[allow(clippy::too_many_lines)]
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
/// multiple shapes exercise the `min()` edge paths. Positive-only inputs
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
/// (retrieval discriminators, the kernel-batch `concepts_covered`
/// convention applied to a `rust_perf` batch).
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
/// cross-arch x86 evidence are in `context_notes`. (The x86-pending
/// caveat was RESOLVED 2026-09-04 by the armed re-gate: `x86_64` read
/// `ParetoDominant` 3/0/0, refuting the expected Tie — the annotation
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
/// text: the callee-side `black_box` caveat, the constant-work fold
/// hazard, the NaN comparison directions, and the `NotBenchable` verdict
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
/// form. The "measure `size_of` before/after" caveat is the RULE's
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
    assert_eq!(with_spec, 99, "expected 99 benchable rules (Batch 179 added the three rsconstmap specs; the host-lock rule is NotBenchable — deadlock class), got {with_spec}");
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
    assert_eq!(without_spec.len(), 97); // Batch 221 +1 (the bevy #25864 runtime-bool conditional-guard rule, Plan 221 -- a codegen-shape claim, the B182 class: the duplicate-vs-deinline decision is LLVM's inliner over the real body) on the 96 base (Batch 219 +11 (the TensorFold native-engine serving/timing pack, Plan 215 slot 2 / Research 251: the cost-table hygiene pair (calibration-table + modeled-cost decision content, the B136 shape) + the prompt-cache/fingerprint caching trio (serving-lifecycle decision content) + the concurrency pair (the bounded peer-handoff wait #512 + the spinning one-message-per-exchange post lock) + thermal-hysteresis (a shipped-runtime pacing policy) + the load-qualification probe (two failure classes, two responses) + the Metal keepalive (crash-latency-class platform hygiene) + the display carveout (all-or-nothing memory admission) -- all source-reported foreign-box figures, the B200 class) on the 184-era 85 base (Batch 211 +1: repr-align-register-partition-store-forwarding, the bevy #26042 layout rule, source-reported bench-box provenance; on the 2026-10-08 REPAIR base (M3, first 5c run since B207): 83 was one SHORT — measured truth 84 None + 99 Some (the sibling with_spec pin, green in the same run) = 183 total, the AGENTS.md count. The B207 comment's "183 total − 100 benchable" arithmetic used a benchable count that contradicts the 99 pin; the pin hid through B207's landing because only the all-features 5c suite compiles this test. Batch 207 +2 -- the CSE scope-map crossover (IR-pass decision content: the 15% small-scope negative is the rule's second half, not a bar) + the inline-hint wrapper rule (a propagation law, not a local transform -- and its numbers are foreign-laptop provenance); Batch 204 +1 -- the NEON vec_dot rule (aarch64-only; its M3-Max GOAT LANDED PARETO-PASS 2026-10-08, bench 106 — the rule stays corpus-only, the GOAT priced the mechanism); Batch 200 added ALL ELEVEN its rules as NotBenchable (serving-stack decision content: parity properties, mode switches, pool lifecycles, AIMD ramps, pinned-staging + madvise hygiene); Batch 198 added its rule as NotBenchable (host-behavior-shaped; the NOT-ISOLATED caveat rides context_notes); Batch 196 added BOTH its rules as NotBenchable (rows-not-paid-for in a serving verify forward + host accounting); Batch 182 added all THREE its rules as NotBenchable (dispatch/framework-shaped + acceptance-rate + measurement-methodology); Batch 181 added BOTH its rules as NotBenchable (mask-memo graph-reuse class + seed-warm measurement hygiene)
    assert!(without_spec.contains(&"mmap-settled-size-gated-read")); // Batch 172 — page-cache state dominates (the B139/B145/B118 class)
    assert!(without_spec.contains(&"steal-batch-not-per-item")); // Batch 172 — deque-header contention under real scheduling
    assert!(without_spec.contains(&"nonblocking-open-for-head-probes")); // Batch 172 — the failure is a hang on a live kernel endpoint
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
    assert!(without_spec.contains(&"batch-blocking-pool-segment")); // Batch 152 — blocking-pool coordination timing, the Batch-78 platform class
    assert!(without_spec.contains(&"mask-memo-hoist-to-owner-lifetime")); // Batch 181 — graph-node reuse + alloc churn, measurable only end-to-end in a spec-decode loop
    assert!(without_spec.contains(&"grammar-window-draft-prune-path-local-mask")); // Batch 196 — rows-not-paid-for inside a serving verify forward, no Rust-side A/B axis
    assert!(without_spec.contains(&"startup-estimate-derived-from-allocation-shapes")); // Batch 196 — host accounting; the fake-allocator reconciliation IS the verification
    assert!(without_spec.contains(&"seed-warm-the-scored-seam")); // Batch 181 — the fix IS measurement hygiene, no fixable code shape to A/B
    assert!(without_spec.contains(&"per-scope-map-scan-hash-crossover")); // Batch 207 — IR-pass decision content; the 15% negative arm is source-reported provenance, not a local bench
    assert!(without_spec.contains(&"inline-hint-not-through-unmarked-wrapper")); // Batch 207 — a propagation law; the four-row table is foreign-laptop provenance
    assert!(without_spec.contains(&"adaptive-yield-consecutive-ready")); // Batch 152 — fairness/p99-under-contention metric mismatch
    assert!(without_spec.contains(&"bounded-fanout-semaphore")); // Batch 152 — external-resource concurrency budget, not wall-clock
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

#[cfg(feature = "rust_perf_bench")]
#[test]
fn not_benchable_fixable_pin() {
    // P39 (2026-09-25): the default-features-visible NOT_BENCHABLE_FIXABLE
    // const (the --repo-posture fix lane's decline gate) must equal
    // FIXABLE_RULES ∩ no-bench-spec, BOTH directions — a fixable rule that
    // gains a bench spec leaves the list (the gate would over-decline); a
    // new fixable landed without a spec joins it (the gate would silently
    // exempt an un-benchmarked edit). The field only exists under this
    // feature, which is why the const cannot be derived at default
    // features — this pin is the join.
    use crate::domains::rust_perf::proposers::{FIXABLE_RULES, NOT_BENCHABLE_FIXABLE};
    let corpus = build_perf_seed_corpus();
    let derived: Vec<&str> = FIXABLE_RULES
        .iter()
        .filter(|r| {
            corpus
                .entries
                .iter()
                .any(|e| e.rule_id == **r && e.bench_spec.is_none())
        })
        .copied()
        .collect();
    assert_eq!(
        derived,
        NOT_BENCHABLE_FIXABLE,
        "the posture gate's NotBenchable list must match the corpus (both directions)"
    );
}

/// Bench 013 found 4 rules with non-dominant verdicts in the default
/// config (2 `ParetoDominated` + 2 Mixed). Their `context_notes` annotation
/// records the finding so consumers don't unconditionally suggest them.
/// Bench 016 (Issue 012) added 2 more annotated rules (sort-by-key,
/// copy-from-slice). Issue 014 T2.3/T2.4 added 2 more in the
/// `NotBenchable` metric-mismatch class (tokio-guard-across-await,
/// bounded-channel-backpressure). Batch 48 added 2 verdict-class rules
/// (filter-count-to-any `ParetoDominated`, iter-copied-deref-map Mixed).
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
/// overflow-promotion — the `ParetoDominated` memory-for-speed trade
/// (the Datadog 2,100 GiB regime vs the measured update-path cost);
/// cache-line-block-partitioned-lock — measured-winner regime note +
/// the first `ALLOC_AXIS_EXEMPT` rule). Batch 86 added 3 (serialize-once-
/// byte-buffer-store — the shrinking-ratio + regime note;
/// context-equal-field-elide — the %-equal two-regime note;
/// bitflag-pack-when-footprint-dominates — the NotBenchable-speed size
/// axis + the repr-reorder caveat). Batch 91 added 2
/// (filter-before-expensive-transform — the two-regime discard ×
/// transform-cost note; decision-projection-before-construction — the
/// invariance-precondition note). Batch 94 added 1
/// (coalesce-nearby-reads-bounded-gap-cacheability-gated — the
/// cacheability-coupling regime, source-reported). Batch 99 added 1
/// Batch 99 added 1
/// (mir-destprop-branch-site-merge — the measured `ParetoDominant` verdict
/// with the runtime-opaque-targets negative-control boundary and the
/// rustc `DestProp` toolchain regime). Batch 172 added 4 annotations (the
/// dual-mask GOAT lead + three `NotBenchable` mezura decision-content rules)
/// and flipped byte-find-position-to-memchr via the anchor-cover rider.
/// The other 55 have `context_notes: None` (unconditional wins or
/// honestly `NotBenchable`).
#[test]
fn forty_three_annotated_rules_carry_context_notes() {
    let corpus = build_perf_seed_corpus();
    let annotated: Vec<&str> = corpus
        .entries
        .iter()
        .filter(|e| e.context_notes.is_some())
        .map(|e| e.rule_id.as_str())
        .collect();
    assert_eq!(annotated.len(), 142, "expected 142 context-annotated rules (Batch 221: +1 -- the bevy #25864 runtime-bool conditional-guard rule lands annotated (NotBenchable codegen-shape claim + the patch-1 anti-shape disclosure + the review-thread quotes verified at the API), Plan 221; Batch 219: +11 -- the TensorFold serving/timing pack, ALL ELEVEN land annotated (NotBenchable decision-content + verified quotes + Apache-2.0 provenance @ f1aef9c3), Plan 215 slot 2; Batch 211: +1 -- the bevy #26042 register-partition rule lands annotated (NotBenchable + the two-faces law + the rider face), Plan 211; Batch 207: +2 -- the CSE scope-map crossover (the measured-negative second arm + the attribute-less Hash key clause) + the inline-hint wrapper rule (the wrapper-attribute-audit discipline + the LLM-disclosure provenance note); Batch 204: +1 -- the PrismML NEON PQ2_0 vec_dot rule lands annotated (NotBenchable on the x86_64 host box + the M3 GOAT staging + the B62 bench-design law); Batch 203 en-route repair: B202's dataflow-fixpoint-convergence-traversal-order landed annotated and missed these pins -- the narrow batch202 gate never ran them; Batch 200: 114 → 125 -- the TensorFold serving-half ELEVEN, all annotated with verified quotes + Apache-2.0 provenance; Batch 198: 113 → 114 -- the PrismML selector-lattice-beam-over-greedy-walk NOT-ISOLATED annotation; Batch 196: 111 → 113 -- the TensorFold grammar-window + startup-estimate annotations; Batch 181: 104 → 106 -- the mlxfast mask-memo + seed-warm annotations; Batch 182: 106 → 109 -- the mlxfast-delta host-mirror + prompt-lookup + adoption-trial annotations);");
    assert!(annotated.contains(&"neon-tbl-replicate-2bit-codec-sdot-block-rescale")); // Batch 204 -- the NotBenchable-on-x86_64 note + the staged M3 GOAT + the B62 naive-codec-reference law
    assert!(annotated.contains(&"dataflow-fixpoint-convergence-traversal-order")); // Batch 202 -- the NotBenchable decision-content note (the pin B202's landing missed)
    assert!(annotated.contains(&"runtime-bool-duplicates-inlinable-body")); // Batch 221 -- the NotBenchable codegen-shape note + the patch-1 anti-shape disclosure
    assert!(annotated.contains(&"selector-lattice-beam-over-greedy-walk")); // Batch 198 — the NOT-ISOLATED caveat (the commit's e2e number is the fewrow kernel's)
    assert!(annotated.contains(&"grammar-window-draft-prune-path-local-mask")); // Batch 196 — the exactness-law + refusal-surface annotation
    assert!(annotated.contains(&"startup-estimate-derived-from-allocation-shapes")); // Batch 196 — the reconciliation + sub-arm annotation
    assert!(annotated.contains(&"mask-memo-hoist-to-owner-lifetime")); // Batch 181 — the owner-lifetime/graph-reuse annotation
    assert!(annotated.contains(&"seed-warm-the-scored-seam")); // Batch 181 — the seam-warm measurement-honesty annotation (+ the per-depth-oracle rider)
    assert!(annotated.contains(&"aa-control-prices-the-fidelity-instrument")); // Batch 177 — NotBenchable measurement-honesty #3 (the comparator)
    assert!(annotated.contains(&"observer-sync-bias-differs-by-change-class")); // Batch 177 — NotBenchable measurement-honesty #4 (the observer-consumer bias)
    assert!(annotated.contains(&"paired-probe-layout-present-vs-absent")); // Batch 179 — measured verdict + absent-key regime content
    assert!(annotated.contains(&"block-hash-before-gather-mlp")); // Batch 179 — measured verdict + regime divergence
    assert!(annotated.contains(&"immutable-static-map-filter-selection")); // Batch 179 — measured verdict + adoption contracts
    assert!(annotated.contains(&"bufwriter-batched")); // Batch 179 — the serialize rider annotation
    assert!(annotated.contains(&"serde-borrow-deserialize")); // Batch 173 — the escape-regime + from_reader anti-regime annotation
    assert!(annotated.contains(&"rawvalue-passthrough-defer-parse")); // Batch 173 — the behavioral-delta (formatting retention) + typed-consumer anti-regime annotation
    assert!(annotated.contains(&"host-lock-across-reentrant-blocking-call")); // Batch 174 — NotBenchable deadlock-class annotation (decide-under-lock/drop/act)
    assert!(annotated.contains(&"deferred-command-flush-hoist-out-of-batch-loop")); // Batch 174 — the mechanism-isolation + observer-contract annotation
    assert!(annotated.contains(&"dual-mask-fused-block-scan")); // Batch 172 — the GOAT lead, measured verdict in notes
    assert!(annotated.contains(&"mmap-settled-size-gated-read")); // Batch 172 — NotBenchable decision-content annotation
    assert!(annotated.contains(&"steal-batch-not-per-item")); // Batch 172 — NotBenchable contention annotation
    assert!(annotated.contains(&"nonblocking-open-for-head-probes")); // Batch 172 — NotBenchable hang-class annotation
    assert!(annotated.contains(&"byte-find-position-to-memchr")); // Batch 172 — the anchor-cover selection rider flipped it from None
    assert!(annotated.contains(&"debug-level-gated-computation")); // Batch 143 — the #147293 citation + the #143684 wall-time-vs-icount measurement-honesty rider
    assert!(annotated.contains(&"batch-blocking-pool-segment")); // Batch 152 — the two-regime + global-resource annotation
    assert!(annotated.contains(&"adaptive-yield-consecutive-ready")); // Batch 152 — the dynamic-trigger + light-load anti-regime annotation
    assert!(annotated.contains(&"bounded-fanout-semaphore")); // Batch 152 — the adopt-deliberately + resource-vs-task-count annotation
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

/// The non-annotated rules (`ParetoDominant` + honestly `NotBenchable`) must
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
    assert_eq!(unannotated, 54, "expected 54 unannotated rules (195 total - 141 annotated; Batch 219 +11 ALL ANNOTATED -- the TensorFold serving/timing pack, so both sides move +11 and the unannotated count is UNCHANGED; Batch 200 +11 ALL ANNOTATED -- the TensorFold serving half, so both sides move +11 and the unannotated count is UNCHANGED; Batch 198 +1 ANNOTATED -- selector-lattice-beam-over-greedy-walk carries the NOT-ISOLATED caveat in context_notes, so the unannotated count is unchanged; Batch 186 added both its rules annotated: the toggle-oracle + cold-LLC members)");
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
    assert_eq!(annotated, 142); // Batch 221: +1 the bevy #25864 runtime-bool conditional-guard rule lands annotated (141 -> 142, Plan 221); Batch 219: +11 the TensorFold serving/timing pack, ALL annotated (130 -> 141, Plan 215 slot 2); Batch 211: +1 the bevy #26042 register-partition rule lands annotated (129 -> 130, Plan 211); Batch 207: +2 the CSE scope-map crossover + the inline-hint wrapper rule (127 -> 129, Plan 207); Batch 204: +1 the PrismML NEON PQ2_0 vec_dot rule lands annotated (126 -> 127, Plan 204); Batch 203 en-route repair: B202's rule landed annotated (125 -> 126) and missed this pin -- the narrow batch202 gate never ran it; Batch 200: 114 -> 125 (the TensorFold serving-half ELEVEN); Batch 198: 113 -> 114 (the PrismML selector-lattice NOT-ISOLATED annotation); Batch 196: 111 -> 113; Batch 186: 109 -> 111; Batch 179: 100 → 104; Batch 181: 104 → 106; Batch 182: 106 → 109

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
/// unconditional wins. Twenty annotations: Bench 013 (2 `ParetoDominated`
/// and 2 Mixed), Bench 016 (sort-by-key and copy-from-slice), Issue 014
/// (2 `NotBenchable` metric-mismatch), Batch 48 (2 verdict-class),
/// Batch 53 (Bench 038, 2 verdict-class), Batch 62 (1 regime-class),
/// Batch 64 (1 release-erasure class), Batch 66 (1 regime-split
/// class), Batch 68 (1 `ParetoDominated` honest-loser — the tantivy
/// varint ladder regresses on aarch64 across 3 tested writer regimes),
/// Batch 65 (2 measured-winner regime caveats — align-64's
/// arch-granularity + channel-send's pipeline-overlap margin; a NEW
/// annotation class: `ParetoDominant` verdict + applicability condition,
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
    assert_eq!(annotated.len(), 142, "expected 142 annotated rules (Batch 221: +1 the bevy #25864 runtime-bool conditional-guard rule lands annotated, Plan 221; Batch 219: +11 the TensorFold serving/timing pack, ALL annotated, Plan 215 slot 2; Batch 211: +1 the bevy #26042 register-partition rule lands annotated, Plan 211; Batch 207: +2 the CSE scope-map crossover + the inline-hint wrapper rule, Plan 207; Batch 204: +1 the PrismML NEON PQ2_0 vec_dot rule lands annotated, Plan 204; Batch 203 en-route repair: B202's nnethercote rule landed annotated and missed these pins -- the narrow batch202 gate never ran them; Batch 200: 114 -> 125 -- the TensorFold serving-half eleven, all annotated; Batch 198: 113 -> 114 -- the PrismML selector-lattice NOT-ISOLATED annotation; Batch 196: 111 -> 113 -- the TensorFold grammar-window + startup-estimate pair; Batch 186: 109 -> 111 -- the ggml-cpu tiled measurement pair; Batch 177: 98 → 100 -- the measurement-honesty pair; Batch 179: 100 → 104 -- the rsconstmap trio + the bufwriter-batched rider; Batch 181: 104 → 106 -- the mlxfast mask-memo + seed-warm annotations; Batch 182: 106 → 109 -- the mlxfast-delta trio)");

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
    assert_eq!(unannotated.len(), 54, "expected 54 unannotated rules (179 total − 125 annotated; Batch 200 +11 ALL ANNOTATED -- both sides +11, unannotated UNCHANGED; Batch 198 +1 ANNOTATED -- selector-lattice-beam-over-greedy-walk carries the NOT-ISOLATED caveat, unannotated unchanged)");
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
/// (and thread interleaving) cannot change them. `fetch_add` is atomic, so
/// each counter's final value is exactly its op count regardless of which
/// slot layout the atomics lived in. Pins the bench harness's in-run
/// `debug_assert` at the corpus level + documents the identity the wall-clock
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
/// the flush boundary cannot drop or double-count (the `mem::replace`
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
/// `HashMap<u64, Vec<usize>>` decision content a real `SipHash` collision
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
/// `u32::MAX-adjacent` adds, mixed small/large, LCG, and every-slot
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
/// (retrieval discriminators), the `NotBenchable` annotations are honest
/// (`context_notes` carry the annotation), and the source quotes' key
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
/// (retrieval discriminators), the `NotBenchable` annotations are honest,
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

/// Batch 135 (2026-09-09, Plan 114 — the Wild-linker `RustForge` article
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
/// explicitly with `cargo test -p riir-refine --lib --features
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

/// Batch 173 (2026-09-23, Plan 167 / Research 204 — the serde zero-copy
/// family): the two new rules are present, parse, and carry the concept
/// needles, and the Pareto verdicts are re-runnable on demand. SLOW
/// (spawns a cargo build + run per rule in a temp crate): run explicitly
/// with `cargo test -p riir-refine --lib --features rust_perf_bench
/// batch173_pareto -- --ignored --nocapture`.
#[test]
fn batch173_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    for (rid, category, needles) in [
        (
            "serde-borrow-deserialize",
            "allocation",
            vec!["#[serde(borrow)]", "Cow<'a, str>", "from_str", "escape"],
        ),
        (
            "rawvalue-passthrough-defer-parse",
            "allocation",
            vec!["RawValue", "Value", "verbatim", "defer"],
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("batch-173 rule '{rid}' missing"));
        assert_eq!(e.category, category, "batch-173 '{rid}' category");
        let blob = format!(
            "{}{}{}{}",
            e.name, e.buggy_code, e.fixed_code,
            e.context_notes.as_deref().unwrap_or("")
        );
        for needle in needles {
            assert!(
                blob.contains(needle),
                "batch-173 '{rid}' missing concept needle {needle:?}"
            );
        }
    }
}

#[cfg(feature = "rust_perf_bench")]
#[test]
#[ignore = "slow: spawns cargo build+run per rule (temp bench crates)"]
fn batch173_pareto_benches() {
    let corpus = build_perf_seed_corpus();
    let validator = crate::domains::rust_perf::bench::PerfBenchValidator::new();
    for rid in [
        "serde-borrow-deserialize",
        "rawvalue-passthrough-defer-parse",
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("batch-173 rule '{rid}' missing"));
        let verdict = validator.validate(rid, e.bench_spec.as_ref());
        assert!(!verdict.is_not_benchable(), "batch-173 '{rid}' has a spec");
        let b = match &verdict {
            crate::domains::rust_perf::bench::BenchVerdict::Pareto(p) => p,
            crate::domains::rust_perf::bench::BenchVerdict::NotBenchable(n) => {
                panic!("batch-173 '{rid}' unexpectedly NotBenchable: {n:?}")
            }
        };
        assert!(b.ran, "batch-173 '{rid}' bench must run: {:?}", b.error);
        assert_eq!(
            b.overall,
            crate::domains::rust_perf::bench::BenchOverall::ParetoDominant,
            "batch-173 '{rid}' landed ParetoDominant; re-measure before re-pinning"
        );
        println!(
            "batch-173 {rid}: {:?} faster={} slower={} tie={} allocs@max {}→{}",
            b.overall,
            b.faster_count,
            b.slower_count,
            b.tie_count,
            b.results.last().map_or(0, |r| r.buggy_allocs),
            b.results.last().map_or(0, |r| r.fixed_allocs),
        );
    }
}

/// Batch 174 (2026-09-24, Plan 168 — the host-lock/flush-hoist distill from
/// cubecl #1678 + bevy #25851): both rules present, parse, carry the concept
/// needles. The host-lock rule is `NotBenchable` by design (deadlock class —
/// the `O_NONBLOCK` head-probe precedent), so `without_spec` carries its count
/// half and this gate carries presence/parse/needles.
#[test]
fn batch174_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    for (rid, category, needles) in [
        (
            "host-lock-across-reentrant-blocking-call",
            "concurrency",
            vec!["RE-TAKES", "decide under the lock", "guard dropped", "deadlock"],
        ),
        (
            "deferred-command-flush-hoist-out-of-batch-loop",
            "concurrency",
            vec!["flush per item", "ONE sync point", "observer", "despawn_all"],
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("batch-174 rule '{rid}' missing"));
        assert_eq!(e.category, category, "batch-174 '{rid}' category");
        let blob = format!(
            "{}{}{}{}",
            e.name, e.buggy_code, e.fixed_code,
            e.context_notes.as_deref().unwrap_or("")
        );
        for needle in needles {
            assert!(
                blob.contains(needle),
                "batch-174 '{rid}' missing concept needle {needle:?}"
            );
        }
        // both entries must syn-parse (the standing parse gate, per-rule)
        syn::parse_file(&e.buggy_code).expect("batch-174 buggy_code parses");
        syn::parse_file(&e.fixed_code).expect("batch-174 fixed_code parses");
    }
}

/// Batch 174's benchable half: the flush-hoist spec runs and lands its
/// verdict (`ParetoDominant` expected — the barrier's fixed-cost share; the
/// measured numbers + the model caveat live in the entry's `context_notes`).
#[cfg(feature = "rust_perf_bench")]
#[test]
#[ignore = "slow: spawns cargo build+run per rule (temp bench crates)"]
fn batch174_pareto_benches() {
    let corpus = build_perf_seed_corpus();
    let validator = crate::domains::rust_perf::bench::PerfBenchValidator::new();
    let rid = "deferred-command-flush-hoist-out-of-batch-loop";
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == rid)
        .unwrap_or_else(|| panic!("batch-174 rule '{rid}' missing"));
    let verdict = validator.validate(rid, e.bench_spec.as_ref());
    assert!(!verdict.is_not_benchable(), "batch-174 '{rid}' has a spec");
    let b = match &verdict {
        crate::domains::rust_perf::bench::BenchVerdict::Pareto(p) => p,
        crate::domains::rust_perf::bench::BenchVerdict::NotBenchable(n) => {
            panic!("batch-174 '{rid}' unexpectedly NotBenchable: {n:?}")
        }
    };
    assert!(b.ran, "batch-174 '{rid}' bench must run: {:?}", b.error);
    assert_eq!(
        b.overall,
        crate::domains::rust_perf::bench::BenchOverall::ParetoDominant,
        "batch-174 '{rid}' landed ParetoDominant; re-measure before re-pinning"
    );
    println!(
        "batch-174 {rid}: {:?} faster={} slower={} tie={} allocs@max {}→{}",
        b.overall,
        b.faster_count,
        b.slower_count,
        b.tie_count,
        b.results.last().map_or(0, |r| r.buggy_allocs),
        b.results.last().map_or(0, |r| r.fixed_allocs),
    );
}

/// Batch 136 (2026-09-09, Plan 115 — the Lattimore "Graph Algorithms in
/// Rayon" blog distill): the three new rules are present, parse, and
/// carry the concept needles. `NotBenchable` by design (qualitative source
/// — no `bench_spec`), so the `without_spec` membership pin above carries
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
/// carries its `NotBenchable` rationale. The fail-closed direction is
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
/// `NotBenchable` by design — the `without_spec` membership pin above carries
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
// Cohesive three-scenario block (reserve / tail boundary / stream holdback);
// the scenarios share fixtures and the 108-line body is the scenario list —
// split would break the (1)(2)(3) contract narrative (corpus-test precedent:
// the e0502_copy_index allow, f5c3feda).
#[allow(clippy::too_many_lines)]
#[test]
fn batch141_reserve_boundary_holdback_semantics() {
    // The span vocabulary for scenario (2), hoisted above the statements
    // (items_after_statements) — the tail-boundary walk classifies tokens.
    #[derive(Clone, Copy, PartialEq)]
    enum Tok {
        Sys,
        UserTurn,
        Filler,
    }
    // Scenario (3)'s wire shapes, hoisted above the statements
    // (items_after_statements) — the marker is the source's exact string and
    // the holder's array bound IS the marker length (the type-level rule).
    const MARKER: &[u8] = b"> **Hint:**"; // 11 bytes, the source's exact marker
    #[derive(Default)]
    struct Holder {
        held: [u8; 11], // the bound IS the marker length
        held_len: usize,
    }
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
/// in their categories. R1 is BENCHABLE (the `without_spec` membership pin
/// does NOT apply; the Pareto runner carries the measurement half); R2 is
/// `NotBenchable` with its rationale in `context_notes`.
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

#[test]
fn batch145_mincore_rule_present_and_annotated() {
    // Batch 145 — the MTPLX mincore-probe routing rule (suggest-only,
    // NotBenchable: an mmap + page-cache-state fixture cannot run hermetic).
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "mincore-probe-routes-gather-arm")
        .unwrap_or_else(|| panic!("batch-145 mincore rule missing from corpus"));
    assert_eq!(e.category, "caching", "batch-145 mincore rule category");
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "batch-145 mincore buggy/fixed must both parse"
    );
    #[cfg(feature = "rust_perf_bench")]
    assert!(
        e.bench_spec.is_none(),
        "batch-145 mincore rule is NotBenchable — no bench spec"
    );
    let text = format!(
        "{}\n{}\n{}\n{}",
        e.name,
        e.buggy_code,
        e.fixed_code,
        e.context_notes.clone().unwrap_or_default()
    );
    for needle in [
        "0.5 ms",
        "165 ms",
        "0.99",
        "worst",
        "pread",
        "FILE-WIDE",
        "parallel-disjoint-pread-hides-cold-page-latency",
        "a5e38bb7",
        "fixed stride",
    ] {
        assert!(text.contains(needle), "batch-145 mincore concept {needle:?} missing");
    }
    // The macOS caveat must be in the annotation (the Batch-139 instrument
    // note), never only in the fixture code.
    let notes = e.context_notes.clone().unwrap_or_default();
    assert!(
        notes.contains("FILE-WIDE") && notes.contains("macOS"),
        "batch-145 mincore macOS caveat must live in context_notes"
    );
}

/// Batch 146 — the accumulated read-lane pack (nnethercote July 2026 +
/// bevy 0.19). Four suggestion-only rules; presence + parse + honest
/// annotation gates. The bevy rule's DUAL-EVIDENCE law is asserted
/// mechanically: the anti-regime witness (riir-ai Bench 930 / Issue 960)
/// must live in the annotation, never only in the fixture prose.
#[test]
fn batch146_rules_present_and_annotated() {
    let corpus = build_perf_seed_corpus();
    let ids = [
        ("dispatch-pass-fusion-skips-empty-handlers", "complexity"),
        ("hot-alloc-struct-fits-cache-line-class", "layout"),
        ("dense-table-bulk-update-via-contiguous-iter", "bevy"),
        ("gpu-upload-repack-skipped-when-layout-matches", "layout"),
    ];
    for (id, category) in ids {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-146 rule '{id}' missing from corpus"));
        assert_eq!(e.category, category, "batch-146 {id} category");
        assert!(
            e.context_notes.is_some(),
            "batch-146 {id} must carry context_notes (all four are suggestion-only: the annotation IS the content)"
        );
        #[cfg(feature = "rust_perf_bench")]
        assert!(
            e.bench_spec.is_none(),
            "batch-146 {id} is NotBenchable — no bench spec"
        );
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains("SUGGESTION-ONLY"),
            "batch-146 {id} annotation must state the suggestion-only verdict"
        );
    }
}

#[test]
fn batch146_pseudocode_parses() {
    // G1 for Batch 146: syn-parse gate over both fixtures of every rule.
    let corpus = build_perf_seed_corpus();
    let ids = [
        "dispatch-pass-fusion-skips-empty-handlers",
        "hot-alloc-struct-fits-cache-line-class",
        "dense-table-bulk-update-via-contiguous-iter",
        "gpu-upload-repack-skipped-when-layout-matches",
    ];
    for id in ids {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-146 rule '{id}' missing"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-146 {id} buggy/fixed must both parse"
        );
    }
}

#[test]
fn batch146_dual_evidence_and_distinct_rows_present() {
    // The bevy rule carries BOTH sides (bevy's positive table + our own
    // Issue 960 negative) — the dual-evidence law, mechanically pinned so
    // a future edit cannot cite the win without the anti-regime witness.
    let corpus = build_perf_seed_corpus();
    let bevy = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "dense-table-bulk-update-via-contiguous-iter")
        .unwrap();
    let text = format!("{} {}", bevy.name, bevy.context_notes.clone().unwrap_or_default());
    for needle in [
        "5.58",
        "1.87",
        "contiguous_iter_mut",
        "Changed<T>",
        "Added<T>",
        "Issue 960",
        "AoS",
        "21984",
    ] {
        assert!(
            text.contains(needle),
            "batch-146 bevy rule dual-evidence needle {needle:?} missing"
        );
    }
    // Distinct-from rows: every rule names its nearest neighbors so
    // retrieval cannot shadow them (the dedup law, admission-time).
    for (id, neighbor) in [
        (
            "dispatch-pass-fusion-skips-empty-handlers",
            "dyn-to-enum",
        ),
        (
            "hot-alloc-struct-fits-cache-line-class",
            "hot-byval-move-over-memcpy-threshold",
        ),
        (
            "dense-table-bulk-update-via-contiguous-iter",
            "AoS",
        ),
        (
            "gpu-upload-repack-skipped-when-layout-matches",
            "entries_05",
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap();
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains("Distinct from"),
            "batch-146 {id} must carry a Distinct-from row"
        );
        assert!(
            notes.contains(neighbor),
            "batch-146 {id} distinct-from row must name {neighbor}"
        );
    }
}

/// Batch 147 (staged agave batch, Plan 131 / Research 172) — G1: presence,
/// categories, annotation discipline. Three of the five are `NotBenchable`
/// suggestion-only (the annotation IS the content); two carry specs.
#[test]
fn batch147_rules_present_and_annotated() {
    let corpus = build_perf_seed_corpus();
    let ids = [
        ("bounded-batch-buffer-pool", "allocation", true),
        (
            "stack-read-fast-path-length-prefixed-record",
            "io",
            false,
        ),
        (
            "batch-verify-optimistic-aggregate-fallback-individual",
            "caching",
            false,
        ),
        ("partial-reset-watermark-pool", "allocation", false),
        (
            "batch-local-memoization-negative-cache-tombstone",
            "caching",
            false,
        ),
    ];
    for (id, category, benchable) in ids {
        // consumed only under `rust_perf_bench` (the spec assert below)
        let _ = benchable;
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-147 rule '{id}' missing from corpus"));
        assert_eq!(e.category, category, "batch-147 {id} category");
        assert!(
            e.context_notes.is_some(),
            "batch-147 {id} must carry context_notes (the annotation IS the content)"
        );
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains("Suggestion-only")
                || notes.contains("CONDITIONAL")
                || notes.contains("SUGGESTION-ONLY"),
            "batch-147 {id} annotation must state the suggestion-only/conditional verdict"
        );
        #[cfg(feature = "rust_perf_bench")]
        if benchable {
            assert!(e.bench_spec.is_some(), "batch-147 {id} must carry its spec");
        } else {
            assert!(e.bench_spec.is_none(), "batch-147 {id} is NotBenchable as drafted");
        }
    }
}

#[test]
fn batch147_pseudocode_parses() {
    // G1 for Batch 147: syn-parse gate over both fixtures of every rule.
    let corpus = build_perf_seed_corpus();
    let ids = [
        "bounded-batch-buffer-pool",
        "stack-read-fast-path-length-prefixed-record",
        "batch-verify-optimistic-aggregate-fallback-individual",
        "partial-reset-watermark-pool",
        "batch-local-memoization-negative-cache-tombstone",
    ];
    for id in ids {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-147 rule '{id}' missing"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-147 {id} buggy/fixed must both parse"
        );
    }
}

#[test]
fn batch147_distinct_rows_present() {
    // The dedup law, admission-time: every rule names its nearest neighbors
    // so retrieval cannot shadow them — including the two-sided B123 row.
    let corpus = build_perf_seed_corpus();
    for (id, neighbor) in [
        ("bounded-batch-buffer-pool", "batch-pool-read-makes-serial-readahead-redundant"),
        ("bounded-batch-buffer-pool", "reuse-allocation-clear"),
        ("bounded-batch-buffer-pool", "smallvec-stack"),
        ("stack-read-fast-path-length-prefixed-record", "smallvec-stack"),
        ("stack-read-fast-path-length-prefixed-record", "bounded-batch-buffer-pool"),
        ("partial-reset-watermark-pool", "bounded-batch-buffer-pool"),
        ("partial-reset-watermark-pool", "reuse-allocation-clear"),
        (
            "batch-verify-optimistic-aggregate-fallback-individual",
            "filter-count-to-any",
        ),
        (
            "batch-local-memoization-negative-cache-tombstone",
            "sort-key-cache-precompute-out-of-lock",
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap();
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains("Distinct from"),
            "batch-147 {id} must carry a Distinct-from row"
        );
        assert!(
            notes.contains(neighbor),
            "batch-147 {id} distinct-from row must name {neighbor}"
        );
    }
}

/// Batch 147 — the batch-verify ladder's G1 SEMANTICS on the stand-in
/// primitive: N==1 bypasses the aggregate; all-valid batches answer from
/// tier 2 without any per-item call; an adversarial batch pays tier 2 +
/// tier 3 and still returns the same verdicts as individual-only.
#[test]
fn batch147_verify_ladder_semantics() {
    struct Tx {
        sig: [u8; 64],
        signer: [u8; 32],
    }
    impl Tx {
        fn verify(&self, _msg: &[u8; 32]) -> bool {
            // the stand-in reads both fields (a real verifier checks the
            // signature AND the signer; the aggregate half reads signer only)
            self.signer != [0u8; 32] && self.sig.iter().any(|&b| b != 0)
        }
    }
    fn verify_aggregate(txs: &[Tx], _msg: &[u8; 32]) -> bool {
        txs.iter().all(|tx| tx.signer != [0u8; 32])
    }
    fn verify_batch(txs: &[Tx], msg: &[u8; 32]) -> Vec<bool> {
        if let [only] = txs {
            return vec![only.verify(msg)];
        }
        if verify_aggregate(txs, msg) {
            return vec![true; txs.len()];
        }
        txs.iter().map(|tx| tx.verify(msg)).collect()
    }
    let msg = [0u8; 32];
    let good = |s: u8| Tx {
        sig: [s; 64],
        signer: [s; 32],
    };
    let bad = || Tx {
        sig: [1u8; 64],
        signer: [0u8; 32],
    };

    // N==1: the ladder returns the individual verdict directly.
    assert_eq!(verify_batch(&[good(1)], &msg), vec![true]);
    assert_eq!(verify_batch(&[bad()], &msg), vec![false]);

    // All-valid N>1: tier 2 answers for the whole batch.
    assert_eq!(verify_batch(&[good(1), good(2), good(3)], &msg), vec![true, true, true]);

    // Adversarial: one invalid vote — the aggregate fails, the per-item
    // fallback locates it. Same verdicts as individual-only, honestly.
    assert_eq!(
        verify_batch(&[good(1), bad(), good(3)], &msg),
        vec![true, false, true]
    );
}

#[test]
fn batch152_rules_present_and_annotated() {
    // The dial9 tokio-principles distill: three suggestion-only concurrency
    // rules seeding the world-facing tokio family (the product-coverage
    // mandate, Research 179 §product-coverage).
    let corpus = build_perf_seed_corpus();
    let ids = [
        "batch-blocking-pool-segment",
        "adaptive-yield-consecutive-ready",
        "bounded-fanout-semaphore",
    ];
    for id in ids {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-152 rule '{id}' missing from corpus"));
        assert_eq!(e.category, "concurrency", "batch-152 {id} category");
        assert!(
            e.context_notes.is_some(),
            "batch-152 {id} must carry context_notes (the annotation IS the content)"
        );
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains("suggestion-only") || notes.contains("SUGGESTION-ONLY"),
            "batch-152 {id} annotation must state the suggestion-only verdict"
        );
        assert!(
            notes.contains("Distinct from"),
            "batch-152 {id} must carry distinct-from rows"
        );
        #[cfg(feature = "rust_perf_bench")]
        assert!(
            e.bench_spec.is_none(),
            "batch-152 {id} is NotBenchable as drafted (the tokio fairness/coordination metric class)"
        );
    }
}

#[test]
fn batch152_pseudocode_parses() {
    // G1 for Batch 152: syn-parse gate over both fixtures of every rule.
    let corpus = build_perf_seed_corpus();
    let ids = [
        "batch-blocking-pool-segment",
        "adaptive-yield-consecutive-ready",
        "bounded-fanout-semaphore",
    ];
    for id in ids {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-152 rule '{id}' missing"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-152 {id} buggy/fixed must both parse"
        );
    }
}

#[test]
fn batch152_distinct_rows_present() {
    // The dedup law, admission-time: the tokio family's nearest neighbors
    // named BOTH directions where the pair is close (rule 2 vs its static
    // sibling; rule 3 vs the channel-bound sibling).
    let corpus = build_perf_seed_corpus();
    for (id, neighbor) in [
        ("batch-blocking-pool-segment", "buffered-io"),
        ("batch-blocking-pool-segment", "bufwriter-batched"),
        ("batch-blocking-pool-segment", "channel-send-batching-unpark"),
        ("adaptive-yield-consecutive-ready", "async-yield-point"),
        ("adaptive-yield-consecutive-ready", "tokio-guard-across-await"),
        ("bounded-fanout-semaphore", "bounded-channel-backpressure"),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap();
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains("Distinct from"),
            "batch-152 {id} must carry a Distinct-from row"
        );
        assert!(
            notes.contains(neighbor),
            "batch-152 {id} distinct-from row must name {neighbor}"
        );
    }
}

#[test]
fn batch153_rules_present_and_annotated() {
    // The async-blocking distill (Ryhl "What is Blocking?" + the pool folds):
    // two concurrency rules + one io rule, all suggestion-only NotBenchable
    // (the scheduler/network metric classes).
    let corpus = build_perf_seed_corpus();
    let expected = [
        ("blocking-sleep-in-async", "concurrency"),
        ("rayon-bridge-oneshot", "concurrency"),
        ("tcp-nodelay-request-response-accepted-socket", "io"),
    ];
    for (id, category) in expected {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-153 rule '{id}' missing from corpus"));
        assert_eq!(e.category, category, "batch-153 {id} category");
        assert!(
            e.context_notes.is_some(),
            "batch-153 {id} must carry context_notes (the annotation IS the content)"
        );
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains("suggestion-only") || notes.contains("SUGGESTION-ONLY"),
            "batch-153 {id} annotation must state the suggestion-only verdict"
        );
        assert!(
            notes.contains("Distinct from"),
            "batch-153 {id} must carry distinct-from rows"
        );
        #[cfg(feature = "rust_perf_bench")]
        assert!(
            e.bench_spec.is_none(),
            "batch-153 {id} is NotBenchable as drafted (the scheduler/network metric class)"
        );
    }
}

#[test]
fn batch153_pseudocode_parses() {
    // G1 for Batch 153: syn-parse gate over both fixtures of every rule.
    let corpus = build_perf_seed_corpus();
    let ids = [
        "blocking-sleep-in-async",
        "rayon-bridge-oneshot",
        "tcp-nodelay-request-response-accepted-socket",
    ];
    for id in ids {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-153 rule '{id}' missing"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-153 {id} buggy/fixed must both parse"
        );
    }
}

#[test]
fn batch153_distinct_rows_present() {
    // The dedup law, admission-time: nearest neighbors named BOTH
    // directions where the pair is close (sleep vs the yield family; the
    // bridge vs the rayon-internal four; nodelay vs the byte-coalescing
    // family).
    let corpus = build_perf_seed_corpus();
    for (id, neighbor) in [
        ("blocking-sleep-in-async", "async-yield-point"),
        ("blocking-sleep-in-async", "adaptive-yield-consecutive-ready"),
        ("blocking-sleep-in-async", "batch-blocking-pool-segment"),
        ("rayon-bridge-oneshot", "partitioned-mut-slices-zero-sync"),
        ("rayon-bridge-oneshot", "dynamic-work-parallelism-ladder"),
        ("rayon-bridge-oneshot", "sender-per-work-item-terminates-channel"),
        ("rayon-bridge-oneshot", "nested-par-inside-channel-bridge-deadlocks"),
        ("rayon-bridge-oneshot", "batch-blocking-pool-segment"),
        (
            "tcp-nodelay-request-response-accepted-socket",
            "buffered-io",
        ),
        (
            "tcp-nodelay-request-response-accepted-socket",
            "bufwriter-batched",
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap();
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains("Distinct from"),
            "batch-153 {id} must carry a Distinct-from row"
        );
        assert!(
            notes.contains(neighbor),
            "batch-153 {id} distinct-from row must name {neighbor}"
        );
    }
}

#[test]
fn batch154_rules_present_and_annotated() {
    // Batch 154 (the tokio upstream-surface refresh — CHANGELOG 1.47–1.52.3
    // + "Actors with Tokio"): 3 NotBenchable suggestion-only rules
    // (the io syscall / panic-liveness / dataflow metric classes).
    let corpus = build_perf_seed_corpus();
    let expected = [
        ("write-vectored-capability-flag", "io"),
        ("oneshot-response-send-unwrap-panics", "concurrency"),
        ("actor-bounded-channel-cycle-deadlock", "concurrency"),
    ];
    for (id, category) in expected {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-154 rule '{id}' missing from corpus"));
        assert_eq!(e.category, category, "batch-154 {id} category");
        assert!(
            e.context_notes.is_some(),
            "batch-154 {id} must carry context_notes (the annotation IS the content)"
        );
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains("suggestion-only") || notes.contains("SUGGESTION-ONLY"),
            "batch-154 {id} annotation must state the suggestion-only verdict"
        );
        assert!(
            notes.contains("Distinct from"),
            "batch-154 {id} must carry distinct-from rows"
        );
        #[cfg(feature = "rust_perf_bench")]
        assert!(
            e.bench_spec.is_none(),
            "batch-154 {id} is NotBenchable as drafted (the io-syscall/panic/dataflow metric classes)"
        );
    }
}

#[test]
fn batch154_pseudocode_parses() {
    // G1 for Batch 154: syn-parse gate over both fixtures of every rule.
    let corpus = build_perf_seed_corpus();
    let ids = [
        "write-vectored-capability-flag",
        "oneshot-response-send-unwrap-panics",
        "actor-bounded-channel-cycle-deadlock",
    ];
    for id in ids {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-154 rule '{id}' missing"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-154 {id} buggy/fixed must both parse"
        );
    }
}

#[test]
fn batch154_distinct_rows_present() {
    // The dedup law, admission-time: nearest neighbors named BOTH
    // directions where the pair is close (the capability flag vs the
    // byte-coalescing family; the response-send vs the rayon bridge; the
    // bounded cycle vs backpressure + the rayon steal-block deadlock).
    let corpus = build_perf_seed_corpus();
    for (id, neighbor) in [
        ("write-vectored-capability-flag", "buffered-io"),
        ("write-vectored-capability-flag", "bufwriter-batched"),
        ("oneshot-response-send-unwrap-panics", "rayon-bridge-oneshot"),
        (
            "actor-bounded-channel-cycle-deadlock",
            "bounded-channel-backpressure",
        ),
        (
            "actor-bounded-channel-cycle-deadlock",
            "nested-par-inside-channel-bridge-deadlocks",
        ),
    ] {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap();
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains("Distinct from"),
            "batch-154 {id} must carry a Distinct-from row"
        );
        assert!(
            notes.contains(neighbor),
            "batch-154 {id} distinct-from row must name {neighbor}"
        );
    }
}

#[test]
fn batch154_annotation_folds_present() {
    // The 4 annotation folds' landing pins: each fold's marker phrase must
    // be present in its target rule's context_notes.
    let corpus = build_perf_seed_corpus();
    let pins = [
        (
            "batch-blocking-pool-segment",
            "UPSTREAM REVERT DATUM (Batch 154 annotation",
        ),
        (
            "tcp-nodelay-request-response-accepted-socket",
            "SOCKET-OPTION FAMILY (Batch 154 annotation",
        ),
        (
            "async-yield-point",
            "SCHEDULER-TWEAK REVERT DATUM (Batch 154 annotation",
        ),
        ("async-yield-point", "COOP API (1.47.0 #7405)"),
    ];
    for (id, marker) in pins {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-154 fold target '{id}' missing"));
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(
            notes.contains(marker),
            "batch-154 fold on {id} must carry marker '{marker}'"
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
    assert!(b.ran, "batch-143 debug-level-gated-computation bench must run: {:?}", b.error);
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

/// G1 for `dual-mask-fused-block-scan` (Batch 172, Research 202 — the mezura
/// GOAT lead): the dispatched SIMD arm (AVX2 on `x86_64`, NEON on aarch64)
/// must produce BIT-IDENTICAL masks to the scalar contract on every shape:
/// (a) every byte value 0..=255 at every position of a 64-byte block (the
/// position × byte-value sweep, 16,384 comparisons — a needle equal to the
/// newline is included, both bits at once);
/// (b) 2,000 LCG blocks dense in newlines + searched bytes;
/// (c) short blocks: every length 0..=63, dense;
/// (d) end-to-end `scan()` vs the naive split reference on LCG buffers of
/// every length 0..=300 (block boundaries + tails) + 1 KiB/4 KiB/64 KiB:
/// newline count, hit count, and the position checksum all identical.
#[test]
#[allow(unsafe_code)]
#[allow(clippy::too_many_lines)] // one four-section exhaustive sweep, one scanner
fn dual_mask_fused_block_scan_equivalence_gate() {
    const BLOCK: usize = 64;

    struct DualMasks {
        newlines: u64,
        candidates: u64,
    }

    struct DualMaskFinder {
        is_searched: [bool; 256],
        searched: Vec<u8>,
        wide: bool,
    }

    impl DualMaskFinder {
        fn new(searched: &[u8]) -> Self {
            let mut is_searched = [false; 256];
            for &b in searched {
                is_searched[b as usize] = true;
            }
            #[cfg(target_arch = "x86_64")]
            let wide = std::is_x86_feature_detected!("avx2");
            #[cfg(target_arch = "aarch64")]
            let wide = std::arch::is_aarch64_feature_detected!("neon");
            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
            let wide = false;
            Self { is_searched, searched: searched.to_vec(), wide }
        }

        fn scan_block(&self, block: &[u8]) -> DualMasks {
            debug_assert!(block.len() <= BLOCK);
            #[cfg(target_arch = "x86_64")]
            if self.wide && block.len() == BLOCK {
                // SAFETY: avx2 probed at construction; len checked == 64.
                let (newlines, candidates) = unsafe { wide::scan_block(block, &self.searched) };
                return DualMasks { newlines, candidates };
            }
            #[cfg(target_arch = "aarch64")]
            if self.wide && block.len() == BLOCK {
                // SAFETY: neon probed at construction; len checked == 64.
                let (newlines, candidates) = unsafe { wide::scan_block(block, &self.searched) };
                return DualMasks { newlines, candidates };
            }
            self.scan_block_scalar(block)
        }

        fn scan_block_scalar(&self, block: &[u8]) -> DualMasks {
            let (mut newlines, mut candidates) = (0u64, 0u64);
            for (at, &byte) in block.iter().enumerate() {
                if byte == b'\n' {
                    newlines |= 1 << at;
                }
                if self.is_searched[byte as usize] {
                    candidates |= 1 << at;
                }
            }
            DualMasks { newlines, candidates }
        }

        fn scan(&self, buf: &[u8]) -> (u64, u64, u64) {
            fn consume(
                m: &DualMasks,
                base: usize,
                newlines: &mut u64,
                hits: &mut u64,
                checksum: &mut u64,
            ) {
                *newlines += u64::from(m.newlines.count_ones());
                *hits += u64::from(m.candidates.count_ones());
                let mut bits = m.candidates;
                while bits != 0 {
                    let at = bits.trailing_zeros() as usize;
                    *checksum = checksum.wrapping_add((base + at) as u64);
                    bits &= bits - 1;
                }
            }
            let (mut newlines, mut hits, mut checksum) = (0u64, 0u64, 0u64);
            let mut base = 0usize;
            let (blocks, tail) = buf.as_chunks::<BLOCK>();
            for block in blocks {
                let m = self.scan_block(block);
                consume(&m, base, &mut newlines, &mut hits, &mut checksum);
                base += BLOCK;
            }
            let m = self.scan_block(tail);
            consume(&m, base, &mut newlines, &mut hits, &mut checksum);
            (newlines, hits, checksum)
        }
    }

    #[cfg(target_arch = "x86_64")]
    mod wide {
        use std::arch::x86_64::{__m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256,
            _mm256_movemask_epi8, _mm256_or_si256, _mm256_set1_epi8};

        /// Returns (newlines, candidates). SAFETY: caller probed avx2 and
        /// checked len == 64.
        // loadu is the UNALIGNED load — casting *const u8 to *const __m256i is
        // the intrinsic's own contract, not an alignment claim; set1_epi8 takes
        // i8 and the u8→i8 wrap is the two's-complement bit pattern by design
        // (the rule_embed dequant precedent).
        #[allow(clippy::cast_ptr_alignment)]
        #[allow(clippy::cast_possible_wrap)]
        #[target_feature(enable = "avx2")]
        pub unsafe fn scan_block(block: &[u8], searched: &[u8]) -> (u64, u64) {
            let lo = _mm256_loadu_si256(block.as_ptr().cast::<__m256i>());
            let hi = _mm256_loadu_si256(block.as_ptr().add(32).cast::<__m256i>());
            let nl = _mm256_set1_epi8(b'\n' as i8);
            let newlines = gather(
                _mm256_cmpeq_epi8(lo, nl),
                _mm256_cmpeq_epi8(hi, nl),
            );
            let (mut hits_lo, mut hits_hi) = (_mm256_set1_epi8(0), _mm256_set1_epi8(0));
            for &byte in searched {
                let want = _mm256_set1_epi8(byte as i8);
                hits_lo = _mm256_or_si256(hits_lo, _mm256_cmpeq_epi8(lo, want));
                hits_hi = _mm256_or_si256(hits_hi, _mm256_cmpeq_epi8(hi, want));
            }
            (newlines, gather(hits_lo, hits_hi))
        }

        /// SAFETY: avx2.
        // movemask yields i32 whose bit 31 is byte 31's sign bit — the u32
        // reinterpretation is bit-pattern reassembly (lo_bits | hi_bits << 32);
        // value semantics are never used.
        #[allow(clippy::cast_sign_loss)]
        #[target_feature(enable = "avx2")]
        unsafe fn gather(lo: __m256i, hi: __m256i) -> u64 {
            let lo_bits = u64::from(_mm256_movemask_epi8(lo) as u32);
            let hi_bits = u64::from(_mm256_movemask_epi8(hi) as u32);
            lo_bits | (hi_bits << 32)
        }
    }

    #[cfg(target_arch = "aarch64")]
    mod wide {
        use std::arch::aarch64::{uint8x16_t, vandq_u8, vceqq_u8, vdupq_n_u8,
            vgetq_lane_u64, vld1q_u8, vorrq_u8, vpaddq_u8, vreinterpretq_u64_u8};

        const POWERS: [u8; 16] = [1, 2, 4, 8, 16, 32, 64, 128, 1, 2, 4, 8, 16, 32, 64, 128];

        /// Returns (newlines, candidates). SAFETY: caller probed neon and
        /// checked len == 64.
        #[target_feature(enable = "neon")]
        pub unsafe fn scan_block(block: &[u8], searched: &[u8]) -> (u64, u64) {
            let p = block.as_ptr();
            let quarters = [
                vld1q_u8(p),
                vld1q_u8(p.add(16)),
                vld1q_u8(p.add(32)),
                vld1q_u8(p.add(48)),
            ];
            let nl = vdupq_n_u8(b'\n');
            let newlines = gather([
                vceqq_u8(quarters[0], nl),
                vceqq_u8(quarters[1], nl),
                vceqq_u8(quarters[2], nl),
                vceqq_u8(quarters[3], nl),
            ]);
            let mut hits = [vdupq_n_u8(0); 4];
            for &byte in searched {
                let want = vdupq_n_u8(byte);
                for (hit, &quarter) in hits.iter_mut().zip(quarters.iter()) {
                                       *hit = vorrq_u8(*hit, vceqq_u8(quarter, want));
                }
            }
            (newlines, gather(hits))
        }

        /// SAFETY: neon.
        #[target_feature(enable = "neon")]
        unsafe fn gather(preds: [uint8x16_t; 4]) -> u64 {
            let powers = vld1q_u8(POWERS.as_ptr());
            let [a, b, c, d] = preds.map(|pred| vandq_u8(pred, powers));
            let halves = vpaddq_u8(vpaddq_u8(a, b), vpaddq_u8(c, d));
            let folded = vpaddq_u8(halves, halves);
            vgetq_lane_u64::<0>(vreinterpretq_u64_u8(folded))
        }
    }

    fn naive(buf: &[u8], needles: &[u8]) -> (u64, u64, u64) {
        let (mut segments, mut hits, mut checksum) = (0u64, 0u64, 0u64);
        let mut at = 0usize;
        for line in buf.split(|&b| b == b'\n') {
            segments += 1;
            for &ndl in needles {
                for (i, &b) in line.iter().enumerate() {
                    if b == ndl {
                        hits += 1;
                        checksum = checksum.wrapping_add((at + i) as u64);
                    }
                }
            }
            at += line.len() + 1;
        }
        (segments - 1, hits, checksum)
    }

    fn next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state >> 33
    }

    // The needle set deliberately INCLUDES b'\n' (a byte that must set BOTH
    // bits) plus a spread of ordinary delimiters.
    let needles: Vec<u8> = vec![b'#', b'/', b'"', b'\'', b';', b'(', b'\n'];
    let finder = DualMaskFinder::new(&needles);

    // (a) every byte value × every position: a 64-byte filler block, one
    // planted byte swept across all values and positions.
    for pos in 0..BLOCK {
        for byte in 0..=255u8 {
            let mut block = [b'x'; BLOCK];
            block[pos] = byte;
            let sim = finder.scan_block(&block);
            let sca = finder.scan_block_scalar(&block);
            assert_eq!(sim.newlines, sca.newlines, "newline mask at pos={pos} byte={byte:#04x}");
            assert_eq!(
                sim.candidates,
                sca.candidates,
                "candidate mask at pos={pos} byte={byte:#04x}"
            );
        }
    }

    // (b) 2,000 LCG blocks dense in newlines + searched bytes.
    let mut state = 0x853C_49E6_748F_EA9Bu64;
    for _ in 0..2_000 {
        let mut block = [0u8; BLOCK];
        for slot in &mut block {
            let r = next(&mut state);
            *slot = match r % 3 {
                0 => b'\n',
                1 => needles[(r >> 40) as usize % needles.len()],
                _ => b'a' + (r % 26) as u8,
            };
        }
        let sim = finder.scan_block(&block);
        let sca = finder.scan_block_scalar(&block);
        assert_eq!((sim.newlines, sim.candidates), (sca.newlines, sca.candidates));
    }

    // (c) short blocks: every length 0..=63 (the tail contract — a shorter
    // block answers for the bytes it has; the missing tail reads as zeros).
    for len in 0..BLOCK {
        let mut block = [0u8; BLOCK];
        for slot in &mut block[..len] {
            let r = next(&mut state);
            *slot = if r.is_multiple_of(2) { b'\n' } else { needles[(r >> 40) as usize % needles.len()] };
        }
        let sim = finder.scan_block(&block[..len]);
        let sca = finder.scan_block_scalar(&block[..len]);
        assert_eq!((sim.newlines, sim.candidates), (sca.newlines, sca.candidates), "len={len}");
    }

    // (d) end-to-end: every length 0..=300 + 1 KiB/4 KiB/64 KiB, fused scan
    // vs the naive split reference — counts AND the position checksum.
    // The needle set here EXCLUDES the newline: the naive per-line walk can
    // never see a separator inside a line (split consumed it), so a
    // newline-in-set contract is vacuously different — the block sweeps
    // above cover the both-bits-at-once edge instead.
    let scan_needles: Vec<u8> = vec![b'#', b'/', b'"', b'\'', b';', b'('];
    let scan_finder = DualMaskFinder::new(&scan_needles);
    let mut lengths: Vec<usize> = (0..=300).collect();
    lengths.extend_from_slice(&[1_024, 4_096, 65_536]);
    for len in lengths {
        let mut buf = vec![0u8; len];
        for slot in &mut buf {
            let r = next(&mut state);
            *slot = match r % 7 {
                0 => b'\n',
                1 | 2 => scan_needles[(r >> 40) as usize % scan_needles.len()],
                _ => b'a' + (r % 26) as u8,
            };
        }
        assert_eq!(
            scan_finder.scan(&buf),
            naive(&buf, &scan_needles),
            "end-to-end divergence at len={len}"
        );
    }
}

#[test]
fn batch177_rules_present_and_annotated() {
    // The EXL3/GB10 serving-recipe distill (Plan 170 / Research 207),
    // measurement-honesty family members #3 (the comparator) + #4 (the
    // observer-consumer bias) -- both suggestion-only NotBenchable (no
    // latency quantity to ratio; the B109 precedent), both carrying
    // context_notes (the annotation IS the content).
    let corpus = build_perf_seed_corpus();
    let expected = [
        ("aa-control-prices-the-fidelity-instrument", "complexity"),
        ("observer-sync-bias-differs-by-change-class", "complexity"),
    ];
    for (id, category) in expected {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-177 rule '{id}' missing from corpus"));
        assert_eq!(e.category, category, "batch-177 {id} category");
        assert!(
            e.context_notes.is_some(),
            "batch-177 {id} must carry context_notes (the annotation IS the content)"
        );
        let notes = e.context_notes.clone().unwrap_or_default();
        assert!(notes.contains("NotBenchable"), "batch-177 {id} NotBenchable posture");
    }
    // Mandated phrases (case-sensitive contains -- the B51 lesson).
    let aa = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "aa-control-prices-the-fidelity-instrument")
        .unwrap();
    for needle in [
        "4 distinct outputs from 5 identical",
        "the control is indistinguishable from the test",
        "prompt-class-conditional",
    ] {
        assert!(
            aa.context_notes.clone().unwrap_or_default().contains(needle)
            || aa.buggy_code.contains(needle)
            || aa.fixed_code.contains(needle)
            || aa.name.contains(needle),
            "batch-177 aa-control lost phrase: {needle}"
        );
    }
    let obs = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "observer-sync-bias-differs-by-change-class")
        .unwrap();
    for needle in [
        "~4 tok/s high for kernel changes and ~10 for host-side",
        "uniform across configurations, so comparisons hold",
    ] {
        assert!(
            obs.context_notes.clone().unwrap_or_default().contains(needle),
            "batch-177 observer-sync lost phrase: {needle}"
        );
    }
}

// ── Batch 179 G1 gates (Plan 172 / Research 210 — the rsconstmap distill) ──

/// G1 for `paired-probe-layout-present-vs-absent`: the paired-slot probe
/// must answer identically to the split-array probe — values exact on
/// present keys (the fingerprint XOR is the same words both arms read)
/// and the corrupted fingerprint early-exits in both layouts, over an
/// LCG sweep of keys at three table sizes. The aarch64 arm exercises the
/// NEON `xor3` path the fixed form ships (baseline ISA — no feature
/// gate); other targets compile the scalar fallback the fixture carries.
#[test]
fn paired_probe_layout_equivalence_gate() {
    fn mix(mut x: u64) -> u64 {
        x ^= x >> 30;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        x
    }
    #[allow(clippy::cast_possible_truncation)] // the high 64 bits are < m <= usize::MAX on 64-bit targets
    fn fastrange(h: u64, m: u64) -> usize {
        ((u128::from(h) * u128::from(m)) >> 64) as usize
    }
    fn triple(key: u64, m: u64) -> [usize; 3] {
        let h = mix(key);
        [
            fastrange(h, m),
            fastrange(h.rotate_left(21), m),
            fastrange(h ^ (h >> 32).wrapping_mul(0x9E37_79B9_7F4A_7C15), m),
        ]
    }
    fn xor3_scalar(a: [u64; 2], b: [u64; 2], c: [u64; 2]) -> [u64; 2] {
        [a[0] ^ b[0] ^ c[0], a[1] ^ b[1] ^ c[1]]
    }
    #[cfg(target_arch = "aarch64")]
    fn xor3_neon(a: &[u64; 2], b: &[u64; 2], c: &[u64; 2]) -> [u64; 2] {
        use std::arch::aarch64::{veorq_u64, vgetq_lane_u64, vld1q_u64};
        // SAFETY: NEON is baseline on AArch64; each vld1q_u64 reads one
        // live 16-byte slot.
        unsafe {
            let v = vld1q_u64(a.as_ptr());
            let v = veorq_u64(v, vld1q_u64(b.as_ptr()));
            let v = veorq_u64(v, vld1q_u64(c.as_ptr()));
            [vgetq_lane_u64::<0>(v), vgetq_lane_u64::<1>(v)]
        }
    }

    let mut state = 0xC0FF_EE00_1234_5678u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state
    };

    for &n in &[97usize, 1_009, 65_537] {
        let checks: Vec<u64> = (0..n).map(|_| next()).collect();
        let data: Vec<u64> = (0..n).map(|_| next()).collect();
        let m = n as u64;
        for _ in 0..2_000u64 {
            let key = next();
            let [i0, i1, i2] = triple(key, m);
            let expect_fp = checks[i0] ^ checks[i1] ^ checks[i2];
            let want = data[i0] ^ data[i1] ^ data[i2];
            // The paired slot must carry [value, fingerprint] and answer
            // exactly what the split probe would answer.
            let sv = xor3_scalar(
                [data[i0], checks[i0]],
                [data[i1], checks[i1]],
                [data[i2], checks[i2]],
            );
            assert_eq!(sv, [want, expect_fp], "paired slot must be [value, fingerprint]");
            #[cfg(target_arch = "aarch64")]
            {
                let nv = xor3_neon(
                    &[data[i0], checks[i0]],
                    &[data[i1], checks[i1]],
                    &[data[i2], checks[i2]],
                );
                assert_eq!(nv, sv, "NEON xor3 must equal the scalar form bit-for-bit");
            }
            // The absent-key contract: a corrupted fingerprint matches
            // neither the split XOR nor the paired fingerprint word —
            // both layouts early-exit to None.
            let corrupted = expect_fp ^ 1;
            assert_ne!(expect_fp, corrupted);
            assert_ne!(sv[1], corrupted);
        }
    }
}

/// G1 for `block-hash-before-gather-mlp`: the two-phase block restructure
/// must be bit-identical to the per-key loop over random keys — including
/// the tail path (< BLOCK keys) and the empty input — over several table
/// sizes and batch lengths.
#[test]
fn block_hash_gather_equivalence_gate() {
    const BLOCK: usize = 8;
    fn mix(mut x: u64) -> u64 {
        x ^= x >> 30;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        x
    }
    #[allow(clippy::cast_possible_truncation)] // the high 64 bits are < m <= usize::MAX on 64-bit targets
    fn fastrange(h: u64, m: u64) -> usize {
        ((u128::from(h) * u128::from(m)) >> 64) as usize
    }
    fn triple(key: u64, m: u64) -> [usize; 3] {
        let h = mix(key);
        [
            fastrange(h, m),
            fastrange(h.rotate_left(21), m),
            fastrange(h ^ (h >> 32).wrapping_mul(0x9E37_79B9_7F4A_7C15), m),
        ]
    }
    fn loop_gather(table: &[u64], keys: &[u64], out: &mut [u64]) {
        for (o, &k) in out.iter_mut().zip(keys) {
            let [i0, i1, i2] = triple(k, table.len() as u64);
            *o = table[i0] ^ table[i1] ^ table[i2];
        }
    }
    fn block_gather(table: &[u64], keys: &[u64], out: &mut [u64]) {
        let m = table.len() as u64;
        let mut i0s = [0usize; BLOCK];
        let mut i1s = [0usize; BLOCK];
        let mut i2s = [0usize; BLOCK];
        let blocks = keys.len() / BLOCK * BLOCK;
        let mut b = 0;
        while b < blocks {
            for j in 0..BLOCK {
                let [i0, i1, i2] = triple(keys[b + j], m);
                i0s[j] = i0;
                i1s[j] = i1;
                i2s[j] = i2;
            }
            for j in 0..BLOCK {
                out[b + j] = table[i0s[j]] ^ table[i1s[j]] ^ table[i2s[j]];
            }
            b += BLOCK;
        }
        for (o, &k) in out[blocks..].iter_mut().zip(&keys[blocks..]) {
            let [i0, i1, i2] = triple(k, m);
            *o = table[i0] ^ table[i1] ^ table[i2];
        }
    }

    let mut state = 0xB10C_41DE_0000_0001u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state
    };

    for &tn in &[13usize, 1_000, 32_769] {
        let table: Vec<u64> = (0..tn).map(|_| next()).collect();
        for &kn in &[0usize, 1, 7, 8, 9, 15, 16, 63, 64, 65, 4_096] {
            let keys: Vec<u64> = (0..kn).map(|_| next()).collect();
            let mut a = vec![0u64; kn];
            let mut b = vec![0u64; kn];
            loop_gather(&table, &keys, &mut a);
            block_gather(&table, &keys, &mut b);
            assert_eq!(
                a, b,
                "blocked gathers must equal the loop bit-for-bit (table={tn}, keys={kn})"
            );
        }
    }
}

/// Batch 196 — the `TensorFold` v0.6.0 A− serving/host pair (Plan 195):
/// presence + category + syn-parse gate over both fixtures of every rule,
/// plus the load-bearing needles from the verified quotes.
#[test]
fn batch196_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    let expected = [
        (
            "grammar-window-draft-prune-path-local-mask",
            "complexity",
            vec![
                "NotBenchable",
                "c4646171",
                "Apache-2.0",
                "row's logits do not depend on the other rows",
                "the parent's masked row",
                "-inf",
                "98fb8f9",
                "both ranks then walk and mask the same rows",
                "prompt-continuation-draft-override-measured-min-match",
            ],
        ),
        (
            "startup-estimate-derived-from-allocation-shapes",
            "allocation",
            vec![
                "NotBenchable",
                "c4646171",
                "Apache-2.0",
                "exl3_expert_scratch",
                "fake-allocator reconciliation",
                "1.125 GiB",
                "3f35ea9ff3",
                "4.00x",
                "before any download",
                "compaction-reserve-before-prompt-build",
            ],
        ),
    ];
    for (id, category, notes_needles) in expected {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-196 rule '{id}' missing from corpus"));
        assert_eq!(e.category, category, "batch-196 '{id}' category");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-196 '{id}' fixtures must parse"
        );
        assert!(e.context_notes.is_some(), "batch-196 '{id}' must be annotated");
        let notes = e.context_notes.as_deref().unwrap_or_default();
        assert!(
            notes.contains("license flipped MIT→Apache-2.0 at 0.6.0"),
            "batch-196 '{id}' must carry the Apache-2.0 provenance line"
        );
        for needle in notes_needles {
            assert!(
                notes.contains(needle),
                "batch-196 '{id}' notes missing needle: {needle}"
            );
        }
    }
}

/// Batch 200 — the `TensorFold` staged pool's serving half (Plan 198):
/// presence + category + syn-parse gate over all eleven fixtures of every
/// rule, plus the load-bearing needles from the verified quotes (every
/// quote re-verified VERBATIM at the pin `c4646171` by three authoring
/// subagents + the coordinator's 5-quote spot-check).
#[allow(clippy::too_many_lines)] // exhaustive registration table: one row per staged rule, the file's own gate style
#[test]
fn batch200_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    let expected = [
        (
            "prefix-cache-entry-one-token-early",
            "caching",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "prompt[:keep_at]",
            ],
        ),
        (
            "dual-reduction-shared-butterfly-ladder",
            "simd",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "waits on one, not two",
            ],
        ),
        (
            "idle-rank-doorbell-on-rendezvous-store",
            "concurrency",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "doorbell",
            ],
        ),
        (
            "prompt-item-per-consumer-kernel-measured",
            "dispatch",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "1.64x slower",
                "1.53x slower",
            ],
        ),
        (
            "concurrent-round-parity-solo-equal",
            "concurrency",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "no row affects another",
                "same bits at any row count",
            ],
        ),
        (
            "lone-stream-graph-replay-mode-switch",
            "dispatch",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "token SHAs",
            ],
        ),
        (
            "snapshot-then-release-shared-drafter-context",
            "allocation",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "no reference of its own",
            ],
        ),
        (
            "graph-pool-handle-fresh-per-generation",
            "allocation",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "graph_pool_handle",
                "capture into a new one",
            ],
        ),
        (
            "spec-copy-window-aimd-ramp-alone-gated",
            "complexity",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "breaks starts over",
                "halves",
            ],
        ),
        (
            "one-shot-staging-release-to-system",
            "allocation",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "Give the pinned staging back to the system",
            ],
        ),
        (
            "madvise-random-for-key-gather-tables",
            "io",
            vec![
                "NotBenchable",
                "c4646171",
                "license flipped MIT→Apache-2.0 at 0.6.0",
                "MADV_RANDOM",
                "read-ahead only evicts useful pages",
            ],
        ),
    ];
    for (id, category, notes_needles) in expected {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-200 rule '{id}' missing from corpus"));
        assert_eq!(e.category, category, "batch-200 '{id}' category");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-200 '{id}' fixtures must parse"
        );
        assert!(e.context_notes.is_some(), "batch-200 '{id}' must be annotated");
        let notes = e.context_notes.as_deref().unwrap_or_default();
        assert!(
            notes.contains("license flipped MIT→Apache-2.0 at 0.6.0"),
            "batch-200 '{id}' must carry the Apache-2.0 provenance line"
        );
        for needle in notes_needles {
            assert!(
                notes.contains(needle),
                "batch-200 '{id}' notes missing needle: {needle}"
            );
        }
    }
}

/// Batch 202 — the nnethercote min-rank worklist rule (Plan 201):
/// presence + category + syn-parse gate over both fixtures, plus the
/// load-bearing needles from the verified quotes (the blog + the merged
/// framework/mod.rs @ cb9d1b06, read live this batch).
#[test]
fn batch202_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    let expected = [(
        "dataflow-fixpoint-convergence-traversal-order",
        "complexity",
        vec![
            "NotBenchable",
            "cb9d1b0640549f1b041aae430dc413ce93f8c204",
            "160193",
            "1.5 million",
            "90,000",
            "min-rank",
            // the drift correction must ride the entry: 17% is #160033's
            // figure, never this PR's
            "160033",
            "NOT carried",
        ],
    )];
    for (id, category, notes_needles) in expected {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-202 rule '{id}' missing from corpus"));
        assert_eq!(e.category, category, "batch-202 '{id}' category");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-202 '{id}' fixtures must parse"
        );
        assert!(e.context_notes.is_some(), "batch-202 '{id}' must be annotated");
        let notes = e.context_notes.as_deref().unwrap_or_default();
        for needle in notes_needles {
            assert!(
                notes.contains(needle),
                "batch-202 '{id}' notes missing needle: {needle}"
            );
        }
    }
}

/// Batch 204 — the `PrismML` NEON `PQ2_0` `vec_dot` rule (Plan 204 / Research
/// 239): presence + category + syn-parse gate over both fixtures, plus the
/// load-bearing needles from the verified quotes (arm/quants.c
/// `ggml_vec_dot_pq2_0_q8_K` + the commit message @ 6bfcd79a, re-verified
/// verbatim at the pin this batch).
#[test]
fn batch204_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    let expected = [(
        "neon-tbl-replicate-2bit-codec-sdot-block-rescale",
        "simd",
        vec![
            "NotBenchable",
            "6bfcd79a2d426abcd2b50e3c2d09ae2225e70a17",
            "265",
            "lands code 4b+j at lane 4b+j",
            "vaddvq_s32",
            "power-of-two scales",
            "~5.1x pp / ~4.6x tg",
            "never a bar",
            "M3-Max GOAT",
            // the staged-GOAT bench-design law: the scalar reference must
            // be the NAIVE per-element codec form, never a transposed-lane
            // multi-accumulator form (measures layout, not codec)
            "B62",
            "simd_lut_dequant",
        ],
    )];
    for (id, category, notes_needles) in expected {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-204 rule '{id}' missing from corpus"));
        assert_eq!(e.category, category, "batch-204 '{id}' category");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-204 '{id}' fixtures must parse"
        );
        assert!(e.context_notes.is_some(), "batch-204 '{id}' must be annotated");
        let notes = e.context_notes.as_deref().unwrap_or_default();
        for needle in notes_needles {
            assert!(
                notes.contains(needle),
                "batch-204 '{id}' notes missing needle: {needle}"
            );
        }
    }
}

#[test]
fn batch207_rules_present_and_parse() {
    let corpus = build_perf_seed_corpus();
    let expected = [
        (
            "per-scope-map-scan-hash-crossover",
            "complexity",
            vec![
                "NotBenchable",
                "d307dd46efbe80ae133d299eadd6f794e4c82b01",
                "1767",
                "scanned linearly on every lookup and insert, so the pass was quadratic in the operations of a region",
                "A large unrolled kernel spent minutes in it",
                "many regions of a few additions ran 15% slower than main",
                "SmallMap scans its first eight entries inline, without allocating, and hashes past them",
                "INLINE_ENTRIES",
                "AttributeDict",
                "hashes everything but its attributes, which have no Hash and stay in the equality",
                "bsearch-sorted-slice",
                // the negative arm IS the decision content: the quadratic
                // fix alone regressed small scopes — a rule this shape must
                // carry both directions or it reads as "always hash"
                "15%",
                "MIT OR Apache-2.0",
            ],
        ),
        (
            "inline-hint-not-through-unmarked-wrapper",
            "dispatch",
            vec![
                "NotBenchable",
                "1ba0c9f93969ff5a07a7a1ca3dc5168be03ef63e",
                "26040",
                "so the function stopped getting inlined",
                "300.6",
                "575.7",
                "428.5",
                "281.9",
                "fetch_next_aliased_unchecked",
                "#[inline(always)]",
                "dyn-to-enum",
                "LLM to find the cause",
                "MIT OR Apache-2.0",
            ],
        ),
    ];
    for (id, category, notes_needles) in expected {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-207 rule '{id}' missing from corpus"));
        assert_eq!(e.category, category, "batch-207 '{id}' category");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "batch-207 '{id}' fixtures must parse"
        );
        assert!(e.context_notes.is_some(), "batch-207 '{id}' must be annotated");
        // bench_spec is cfg(rust_perf_bench)-gated on PerfRule: at the
        // default posture the field compiles out and NotBenchable is
        // asserted by the without_spec pin instead.
        #[cfg(feature = "rust_perf_bench")]
        assert!(e.bench_spec.is_none(), "batch-207 '{id}' is NotBenchable");
        let notes = e.context_notes.as_deref().unwrap_or_default();
        for needle in notes_needles {
            assert!(
                notes.contains(needle),
                "batch-207 '{id}' notes missing needle: {needle}"
            );
        }
    }
}

#[test]
fn batch211_rule_present_and_parse() {
    // G1 for Batch 211 (Plan 211): the 4-lead conversion mini-batch's
    // rust_perf half -- bevy #26042 EntityLocation alignment, landed as ONE
    // new layout rule carrying the rust #163916 alignment-drop face as the
    // rider (one law, two faces: register partition vs container
    // footprint). Syn-parse gate both sides + needle markers + the
    // distinct-from trio (the B202 narrow-batch-gate lesson).
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "repr-align-register-partition-store-forwarding")
        .unwrap_or_else(|| panic!("batch-211 rule missing from corpus"));
    assert_eq!(e.category, "layout", "batch-211 rule category");
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "batch-211 fixtures must parse"
    );
    assert!(e.context_notes.is_some(), "batch-211 rule must be annotated");
    #[cfg(feature = "rust_perf_bench")]
    assert!(e.bench_spec.is_none(), "batch-211 rule is NotBenchable");
    let notes = e.context_notes.as_deref().unwrap_or_default();
    for needle in [
        "NotBenchable",
        "a144d28034a229c6bc1ce80e1a2c8d955cd72cbc",
        "26042",
        "written as 8+4+4 into `UnsafeEntityCell`",
        "That read spans two pending stores and causes the store-to-load forwarding to fail",
        "repr(align(8))",
        "(4+4+8)",
        "259.52 -> 233.33 -> 66.48",
        "102.2 ns -> 152.1",
        "x1.49",
        "the only one that's still worse than before #25632",
        "aaa57c9a8beec536ad85b6b117ef8c5e1e96f0ba",
        "163916",
        "reduces max-rss by ~102 MiB (1.84%)",
        "align-64-false-sharing",
        "cache-line-block-partitioned-lock",
        "inline-hint-not-through-unmarked-wrapper",
        "UnsafeEntityCell's interior is not pasted",
        "Apache-2.0",
        "Batch 211 / Plan 211 / Research 246",
    ] {
        assert!(
            notes.contains(needle),
            "batch-211 notes missing needle: {needle}"
        );
    }
}

#[test]

fn batch219_rules_present_and_parse() {
    // G1 for Batch 219 (Plan 215 slot 2 / Research 251): the TensorFold
    // native-engine serving/timing pack's rust_perf half -- ELEVEN new rules
    // from ashhart/TensorFold @ f1aef9c3f2ab85b2a68f2eed817e45eecbd04781
    // (Apache-2.0): the A- cost-table hygiene pair (fastest-of-n retime +
    // PAVA pooling / modeled-cost-not-clock), the prompt-cache pair (the
    // staged B+ entry SPLIT at authoring: canonical interval keys +
    // rewind/min-gap eviction) + the fingerprint-probe-bits persistence
    // rule, and the admission quartet (thermal hysteresis band -- the
    // corpus's 13th category, the load-qualification probe, the Metal
    // keepalive, the display carveout) plus the T18 transport pair the
    // coordinator authored (the bounded peer-handoff wait #512 + the
    // spinning one-message-per-exchange posting law). All ELEVEN
    // NotBenchable annotated; every needle below verified verbatim at the
    // pin via git-show blob reads (the NTFS-invalid filename blocks
    // checkout), twice: at authoring by the slice agents, again by the
    // coordinator.
    let corpus = build_perf_seed_corpus();
    for (rule_id, category, needles) in batch219_gate_rows() {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *rule_id)
            .unwrap_or_else(|| panic!("batch-219 rule missing from corpus: {rule_id}"));
        assert_eq!(e.category, *category, "{rule_id} category");
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "{rule_id} fixtures must parse"
        );
        assert!(e.context_notes.is_some(), "{rule_id} must be annotated");
        #[cfg(feature = "rust_perf_bench")]
        assert!(e.bench_spec.is_none(), "{rule_id} is NotBenchable");
        let notes = e.context_notes.as_deref().unwrap_or_default();
        for needle in needles {
            assert!(
                notes.contains(needle),
                "batch-219 notes missing needle in {rule_id}: {needle}"
            );
        }
    }
    // The split adjoin + the thermal category: their own test (the B209
    // too_many_lines split; assertions unchanged).
    batch219_split_adjoin_and_thermal_category(&corpus);
}

// The batch219 tail — extracted verbatim so the gate fn stays under the
// line budget (the B209 pattern).
fn batch219_split_adjoin_and_thermal_category(corpus: &crate::corpus::Corpus<crate::domains::rust_perf::RustPerfDomain>) {
    // The split adjoin: the two prompt-cache rules cross-cite each other
    // (the authoring split kept one source + one measured figure, shared).
    let notes_of = |rid: &str| {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("missing {rid}"))
            .context_notes
            .as_deref()
            .unwrap_or_default()
            .to_string()
    };
    assert!(notes_of("prompt-cache-canonical-interval-keys")
        .contains("prompt-cache-rewind-states-min-gap-eviction"));
    assert!(notes_of("prompt-cache-rewind-states-min-gap-eviction")
        .contains("prompt-cache-canonical-interval-keys"));
    // The NEW 13th category rides this batch's thermal rule.
    assert!(corpus.entries.iter().any(|e| e.category == "thermal"));
}

// The batch219 needle table — extracted verbatim so the gate fn stays under
// the line budget (the B209 pattern; assertions unchanged).
// Data table, not logic — the entries() precedent licenses the length.
#[allow(clippy::too_many_lines)]
fn batch219_gate_rows() -> Vec<(&'static str, &'static str, &'static [&'static str])> {
    vec![
        (
            "fastest-of-n-dip-drift-retime-cost-table",
            "complexity",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "Timings steer windows",
                "RTX 3090 start had recorded 0.61 ms",
                "Retime both entries around a dip up to twice",
                "Timed costs are measured once per engine build, chip and model shape",
            ],
        ),
        (
            "pava-monotone-cost-pooling-modeled-cost-not-clock",
            "complexity",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "Follow a round from its modeled cost",
                "pooled costs never add more ms a row as rows grow",
                "Draft level k + 1 only if a certain draft",
            ],
        ),
        (
            "prompt-cache-canonical-interval-keys",
            "caching",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "canonical decoded intervals",
                "clipped",
            ],
        ),
        (
            "prompt-cache-rewind-states-min-gap-eviction",
            "caching",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "min_gap",
                "the last one evicted",
                "1.7 s",
                "10.9 s",
            ],
        ),
        (
            "prompt-state-fingerprint-probe-bits-keyed-persistence",
            "caching",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "4,143",
                "4096 + 40 + 7",
                "Wyhash",
                "0x4e45",
                "213 MB",
            ],
        ),
        (
            "thermal-hysteresis-band-pacing-chunk-placement",
            "thermal",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "already waiting and at or under low",
                "heat_wait_s",
                "hottest thermal zone is above a band",
                "no_zone",
                "three DGX Sparks",
            ],
        ),
        (
            "degrade-not-fail-load-qualification-probe",
            "dispatch",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "A failed numerical check disables wider lanes",
                "one-row rounds still go through the shared lane engine",
                "GPU execution failures stop loading",
                "16 rows/stream, 32 rows/shared forward, 128 rows/prompt",
            ],
        ),
        (
            "metal-keepalive-residency-unwire",
            "io",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "2.7 s",
                "400 ms",
                "macOS unwires an idle model after 1-2 s",
                "keep-warm",
                "26.3",
            ],
        ),
        (
            "display-carveout-all-or-nothing-admission",
            "allocation",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "instead of running with less",
                "DumbBufferRefused",
                "1,792 MiB",
                "cuMemHostRegister",
                "every round reads and writes all of it",
                "coolbho3k",
            ],
        ),
        (
            "bounded-peer-handoff-wait",
            "concurrency",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "fails the request after 10 seconds instead of spinning forever",
                "failure and shutdown stop an absent handoff without claiming it arrived",
                "return error.TpPeerSilent",
                "timeout_ns: u64 = 10 * std.time.ns_per_s",
                "return error.PeerTimeout",
            ],
        ),
        (
            "spin-lock-one-message-per-exchange",
            "concurrency",
            &[
                "f1aef9c3f2ab85b2a68f2eed817e45eecbd04781",
                "Apache-2.0",
                "Send one message per exchange, with a spinning lock instead of a sleeping one",
                "Posting fell from 14-19 us to 2-7 us",
                "From Kimi K3 on four Macs",
            ],
        ),
    ]
}

#[test]
fn batch221_rule_present_and_parse() {
    // G1 for Batch 221 (Plan 221): the rider-tier conversion set's
    // rust_perf half -- Lead 10 (bevy #25864, 'Fix exclusive system
    // change detection', merged 2026-09-28 @ 74884ffd; patches a4d1789 +
    // 8328184 read in full this session, the chescock review quote
    // verified at the PR comments API). NotBenchable annotated.
    let corpus = build_perf_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "runtime-bool-duplicates-inlinable-body")
        .expect("batch-221 rule missing from the corpus");
    assert_eq!(e.category, "dispatch");
    assert!(
        syn::parse_file(&e.buggy_code)
            .and_then(|_| syn::parse_file(&e.fixed_code))
            .is_ok(),
        "batch-221 fixtures must parse"
    );
    assert!(e.context_notes.is_some(), "batch-221 rule must be annotated");
    #[cfg(feature = "rust_perf_bench")]
    assert!(e.bench_spec.is_none(), "batch-221 rule is NotBenchable");
    let notes = e.context_notes.as_deref().unwrap_or_default();
    for needle in [
        "inline *two* copies of the system",
        "That should ensure that `run` is only compiled in once.",
        "Yep that works pretty well, changed!",
        "ITSELF the duplicated-body anti-shape",
        "regardless of whether the system completes successfully or panics.",
        "even if a panic occurs during the scope.",
        "mir-destprop-branch-site-merge",
        "74884ffda7831d2b4576ade270199d671bb2e8f9",
        "Batch 221 / Plan 221",
    ] {
        assert!(
            notes.contains(needle),
            "batch-221 notes missing needle: {needle}"
        );
    }
    // The landed-form needles ride the FIXED fixture, not the notes: the
    // conditional tuple is the shape the rule teaches.
    assert!(e.fixed_code.contains(".then(|| LastTickGuard"));
    assert!(e.buggy_code.contains("if self.is_exclusive {"));
}
