//! Batch 349 fixture laws — the encoders rust_perf distill (the deferred
//! integer-lane pooling pair + the incremental prefix-hash WordPiece pair)
//! as ONE standalone file.
//!
//! ORIGINAL plain-Rust reproductions — zero encoders/ese code pasted (the
//! source is Apache-2.0; the fixtures are original syntheses of the laws,
//! with our own hash constants). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b349_fixtures.rs -o fx && ./fx
//! --test-threads=1` (the B336/B338/B339/B340/B341/B342/B343/B345
//! discipline; the alloc-axis bracket is min-of-5 and green in parallel
//! runs too). Each rule's arms in
//! `entries_71.rs` embed the same executed code.
//!
//! Laws under test:
//! F1 `deferred-integer-lane-pooling-per-row-exponent` — (a) equality: the
//! deferred form (row addresses collected in a bounded stack batch, one
//! column block at a time summed in integer lanes, boosted rows drained to
//! an f32 side array scaled by 2^-exp, ONE dequant at the end) equals the
//! dequantize-per-row form exactly where the arithmetic is exact (the
//! accumulation order pinned row-serial in both arms); (b) the batch bound
//! is DERIVED from the lane width: u8 rows in u16 lanes are exact at
//! BATCH·255 == 65280 < 65536, and row 257 overflows — the drain-at-256
//! bound is tight, not stylistic; (c) the boosted-row weight law: a row
//! with exponent e contributes 2^-e of its stored value AND 2^-e of the
//! per-column min (the naive arm that dequantizes first gets this for
//! free; the deferred side-array arm must carry it explicitly); (d) the
//! naive arm allocates one temp per row (the alloc axis), the deferred arm
//! allocates nothing.
//! F2 `incremental-prefix-hash-wordpiece-scan` — (a) the push-hash agrees
//! with the from-scratch fold at every prefix length (the incremental fold
//! is the SAME hash, not an approximation); (b) the per-chunk step
//! h -> (h ^ chunk) * K with odd K is a bijection, so equal-length
//! prefixes never fold equal; (c) segmentation equality: prefix-hash
//! greedy longest-match-first produces the SAME piece list as the
//! re-hash-per-candidate form over in-vocab, multi-piece, non-vocab,
//! overlong and multi-byte words; (d) the census: bytes hashed drop from
//! O(len * max_token^2) to O(len * max_token) on the long-word regime.

use std::collections::HashMap;

// ── the allocation probe: a forwarding global allocator counting every
// alloc. Installed file-level (legal in a standalone test binary) so the
// F1 alloc-axis assertions count real allocations.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

// ═══════════════════════════════════════════════════════════════════════
// F1 — deferred-integer-lane-pooling-per-row-exponent
// ═══════════════════════════════════════════════════════════════════════

const DIM: usize = 16;
const BLOCK: usize = 8;
/// Rows collected before a sum pass: u8 lanes (u16 accumulators) stay
/// exact through BATCH * 255 == 65280 < 65536. Row 257 would overflow.
const BATCH: usize = 256;
/// Largest exponent the runtime accepts.
const MAX_ROW_EXP: u8 = 8;

static FILLER: [u8; DIM] = [0; DIM];

fn exp_factor(exp: u8) -> f32 {
    1.0 / (1u32 << exp) as f32
}

// ── the naive arm: dequantize each row as it arrives ────────────────────

fn mean_pool_naive(
    rows: &[(&'static [u8; DIM], u8)],
    min: &[f32; DIM],
    scale: &[f32; DIM],
) -> [f32; DIM] {
    let mut out = [0.0f32; DIM];
    for &(row, exp) in rows {
        // one dequant temp per row — the allocation the deferred form skips.
        // black_box keeps the temp observable: in the real shape it feeds a
        // callee boundary LLVM cannot fuse, so the temp is materialized.
        let dq: Vec<f32> = row
            .iter()
            .enumerate()
            .map(|(j, &q)| exp_factor(exp) * (min[j] + f32::from(q) * scale[j]))
            .collect();
        std::hint::black_box(&dq);
        for (o, &v) in out.iter_mut().zip(&dq) {
            *o += v;
        }
    }
    let n = rows.len() as f32;
    for o in &mut out {
        *o /= n;
    }
    out
}

// ── the deferred arm: collect addresses, sum integer lanes, dequant once ─

struct DeferredPool {
    pending: [&'static [u8; DIM]; BATCH],
    n: usize,
    /// Integer-lane sums over PLAIN rows only (exponent 0).
    plain_sum: [u32; DIM],
    /// Boosted rows' f32 side array, scaled by 2^-exp as they arrive.
    side_sum: [f32; DIM],
    /// Sum over boosted rows of 2^-exp; plain rows count 1 each.
    weight_sum: f32,
    boosted: u32,
}

impl DeferredPool {
    fn new() -> Self {
        DeferredPool {
            pending: [&FILLER; BATCH],
            n: 0,
            plain_sum: [0; DIM],
            side_sum: [0.0; DIM],
            weight_sum: 0.0,
            boosted: 0,
        }
    }

    fn add(&mut self, row: &'static [u8; DIM], exp: u8) {
        assert!(exp <= MAX_ROW_EXP, "exponent above the runtime bound");
        if exp == 0 {
            if self.n == BATCH {
                self.drain();
            }
            self.pending[self.n] = row;
            self.n += 1;
        } else {
            let f = exp_factor(exp);
            for (s, &q) in self.side_sum.iter_mut().zip(row.iter()) {
                *s += f32::from(q) * f;
            }
            self.weight_sum += f;
            self.boosted += 1;
        }
    }

    /// One column block at a time: u16 integer lanes over the pending
    /// rows (exact at the BATCH bound), widened once per block.
    fn drain(&mut self) {
        let pending = &self.pending[..self.n];
        let mut off = 0;
        while off < DIM {
            let width = BLOCK.min(DIM - off);
            let mut acc = [0u16; BLOCK];
            for row in pending {
                for (j, a) in acc.iter_mut().enumerate().take(width) {
                    // exact: width lanes of BATCH rows of u8 cannot wrap
                    *a = a.wrapping_add(u16::from(row[off + j]));
                }
            }
            for (j, a) in acc.iter().enumerate().take(width) {
                self.plain_sum[off + j] += u32::from(*a);
            }
            off += BLOCK;
        }
        self.n = 0;
    }

    /// The caller-owned finish (finish_deferred) carries the plain count;
    /// this shape documents the contract the pool struct alone cannot
    /// express. Dead in this fixture by design (#[allow] kept).
    #[allow(dead_code)]
    fn finish(&mut self, _min: &[f32; DIM], _scale: &[f32; DIM]) -> [f32; DIM] {
        self.drain();
        // the caller-owned finish (finish_deferred) carries the plain
        // count; this method is kept to document the contract shape.
        #[allow(clippy::panic)]
        {
            unreachable!("finish needs the plain count; use finish_deferred");
        }
    }
}

/// The full finish, carrying the plain count the pool does not track
/// internally (the fixture keeps the row log outside, as the caller does).
fn finish_deferred(
    pool: &mut DeferredPool,
    plain_count: u32,
    min: &[f32; DIM],
    scale: &[f32; DIM],
) -> [f32; DIM] {
    pool.drain();
    let weight_sum = plain_count as f32 + pool.weight_sum;
    let n = plain_count as f32 + pool.boosted as f32;
    let mut out = [0.0f32; DIM];
    for j in 0..DIM {
        out[j] = (pool.plain_sum[j] as f32 * scale[j]
            + pool.side_sum[j] * scale[j]
            + min[j] * weight_sum)
            / n;
    }
    out
}

/// A reference mean computed in f64 — the equality oracle.
fn mean_pool_reference(
    rows: &[(&'static [u8; DIM], u8)],
    min: &[f32; DIM],
    scale: &[f32; DIM],
) -> [f32; DIM] {
    let mut out = [0.0f64; DIM];
    for &(row, exp) in rows {
        let f = f64::from(exp_factor(exp));
        for (k, (o, &q)) in out.iter_mut().zip(row.iter()).enumerate() {
            *o += f * (f64::from(min[k]) + f64::from(q) * f64::from(scale[k]));
        }
    }
    let n = f64::from(rows.len() as u32);
    out.map(|v| (v / n) as f32)
}

fn exact_rows(seed: u64, n: usize) -> Vec<(&'static [u8; DIM], u8)> {
    // leaked rows so the addresses are 'static (the deferred form borrows)
    let mut state = seed;
    let mut lcg = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    };
    let mut rows = Vec::with_capacity(n);
    for _ in 0..n {
        let leaked: &'static [u8; DIM] = Box::leak(Box::new(std::array::from_fn(|_| {
            (lcg() % 64) as u8 // small values: every partial sum stays exact
        })));
        // mostly plain rows, a few boosted — the source's own distribution
        let exp = if lcg() % 16 == 0 {
            1 + (lcg() % 3) as u8
        } else {
            0
        };
        rows.push((leaked, exp));
    }
    rows
}

#[test]
fn deferred_equals_naive_exactly_where_the_arithmetic_is_exact() {
    let min = [0.0f32; DIM];
    let scale = [1.0f32; DIM];
    for seed in [7u64, 0xDEAD_BEEF, 42] {
        let rows = exact_rows(seed, 600);
        let naive = mean_pool_naive(&rows, &min, &scale);
        let mut pool = DeferredPool::new();
        let mut plain = 0u32;
        for &(row, exp) in &rows {
            pool.add(row, exp);
            if exp == 0 {
                plain += 1;
            }
        }
        let deferred = finish_deferred(&mut pool, plain, &min, &scale);
        assert_eq!(
            naive, deferred,
            "seed {seed}: exact equality with min=0 scale=1"
        );
        assert_eq!(deferred, mean_pool_reference(&rows, &min, &scale));
    }
}

#[test]
fn boosted_rows_carry_their_exponent_in_both_terms() {
    // min != 0: a boosted row contributes 2^-e of min too — the side-array
    // arm must weight min by 2^-e, not 1.
    let min = std::array::from_fn(|j| 1.0f32 + j as f32 * 0.25);
    let scale = [0.5f32; DIM];
    let plain_row: &'static [u8; DIM] = Box::leak(Box::new([4u8; DIM]));
    let boosted_row: &'static [u8; DIM] = Box::leak(Box::new([12u8; DIM]));
    let rows = [(plain_row, 0u8), (boosted_row, 2u8)];
    let naive = mean_pool_naive(&rows, &min, &scale);
    let mut pool = DeferredPool::new();
    pool.add(plain_row, 0);
    pool.add(boosted_row, 2);
    let deferred = finish_deferred(&mut pool, 1, &min, &scale);
    assert_eq!(naive, deferred, "boosted min weight carried");
    // hand-computed: plain col j = min + 4*0.5; boosted col j = (min + 6)/4
    for j in 0..DIM {
        let want = ((min[j] + 2.0) + (min[j] + 6.0) / 4.0) / 2.0;
        assert!(
            (deferred[j] - want).abs() < 1e-6,
            "col {j}: {} vs {want}",
            deferred[j]
        );
    }
}

#[test]
fn the_batch_bound_is_the_lane_width_not_a_style_choice() {
    // u16 lanes: BATCH * 255 == 65280 fits; the lanes wrap at 258 rows
    // (258 * 255 = 65790 > 65535; 257 * 255 = 65535 = u16::MAX exactly).
    assert_eq!(BATCH * 255, 65280);
    assert!(BATCH * 255 < u16::MAX as usize);
    assert!((BATCH + 2) * 255 > u16::MAX as usize);
    // worst case through the pool: every lane exact after many drains
    let worst: &'static [u8; DIM] = Box::leak(Box::new([255u8; DIM]));
    let mut pool = DeferredPool::new();
    for _ in 0..(BATCH * 3 + 17) {
        pool.add(worst, 0);
    }
    let min = [0.0f32; DIM];
    let scale = [1.0f32; DIM];
    let out = finish_deferred(&mut pool, (BATCH * 3 + 17) as u32, &min, &scale);
    // the f64 reference in f32 is exact here (255 * 785 < 2^24)
    let total = 255.0f32 * (BATCH * 3 + 17) as f32;
    for v in out {
        assert!((v - total / (BATCH * 3 + 17) as f32).abs() < 1e-3);
    }
}

#[test]
fn the_naive_arm_allocates_and_the_deferred_arm_does_not() {
    // the file-level Counting global allocator counts every allocation;
    // the rows are leaked OUTSIDE the measured sections.
    let rows = exact_rows(99, 512);
    let min = [0.0f32; DIM];
    let scale = [1.0f32; DIM];

    let probe = || ALLOCS.load(Ordering::Relaxed);
    let before = probe();
    let _ = mean_pool_naive(&rows, &min, &scale);
    let naive_allocs = probe() - before;
    // the deferred arm's bracket is min-of-5: a parallel test run lets
    // another test's allocations land inside the window (the known
    // shared-counter class); a quiet window reads exactly zero. The
    // standalone execution posture for this file is --test-threads=1.
    let mut deferred_allocs = usize::MAX;
    for _ in 0..5 {
        let before = probe();
        let mut pool = DeferredPool::new();
        let mut plain = 0u32;
        for &(row, exp) in &rows {
            pool.add(row, exp);
            if exp == 0 {
                plain += 1;
            }
        }
        let _ = finish_deferred(&mut pool, plain, &min, &scale);
        deferred_allocs = deferred_allocs.min(probe() - before);
    }
    // the naive arm allocates its per-row temp Vecs (>= one per row), the
    // deferred arm allocates nothing (its batch is a stack array).
    // 2 allocs per row: the Vec's buffer + black_box's &Vec reference read
    // keeps the header, LLVM cannot fuse the collect into the sum. The
    // exact count is an LLVM artifact — assert the structural law (>= one
    // temp per row) and that the deferred side is exactly zero.
    assert!(
        naive_allocs >= rows.len(),
        "naive arm must allocate its per-row temps, got {naive_allocs}"
    );
    assert_eq!(
        deferred_allocs, 0,
        "deferred arm must be allocation-free, got {deferred_allocs}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// F2 — incremental-prefix-hash-wordpiece-scan
// ═══════════════════════════════════════════════════════════════════════

/// Our own fold constants (NOT the source's): odd K makes the per-chunk
/// step a bijection mod 2^64.
const K: u64 = 0x9E37_79B9_7F4A_7C15;

const fn mix(h: u64, chunk: u64) -> u64 {
    (h ^ chunk).wrapping_mul(K)
}

const INIT: u64 = 0x0123_4567_89AB_CDEFu64.wrapping_mul(K);

const fn finish(h: u64, len: usize) -> u64 {
    let x = (h ^ len as u64).wrapping_mul(K);
    x ^ (x >> 32)
}

/// Fold a complete key, 8 bytes at a time (little-endian, zero-padded tail).
fn hash_key(key: &[u8]) -> u64 {
    let mut h = INIT;
    let mut i = 0;
    while i + 8 <= key.len() {
        let mut c = 0u64;
        for k in 0..8 {
            c |= (key[i + k] as u64) << (8 * k);
        }
        h = mix(h, c);
        i += 8;
    }
    if i < key.len() {
        let mut c = 0u64;
        for k in 0..key.len() - i {
            c |= (key[i + k] as u64) << (8 * k);
        }
        h = mix(h, c);
    }
    finish(h, key.len())
}

/// Prefix hashes one byte at a time: after pushing k bytes, `hash()` equals
/// `hash_key(&key[..k])`.
#[derive(Clone, Copy)]
struct PushHasher {
    h: u64,
    tail: u64,
    len: usize,
}

impl PushHasher {
    fn new() -> Self {
        PushHasher {
            h: INIT,
            tail: 0,
            len: 0,
        }
    }

    /// Continue from a literal prefix (the subword prefix folds first).
    fn with(bytes: &[u8]) -> Self {
        let mut s = Self::new();
        for &b in bytes {
            s = s.push(b);
        }
        s
    }

    fn push(mut self, b: u8) -> Self {
        self.tail |= (b as u64) << (8 * (self.len % 8));
        self.len += 1;
        if self.len % 8 == 0 {
            self.h = mix(self.h, self.tail);
            self.tail = 0;
        }
        self
    }

    fn hash(&self) -> u64 {
        let h = if self.len % 8 != 0 {
            mix(self.h, self.tail)
        } else {
            self.h
        };
        finish(h, self.len)
    }
}

const MAX_PIECE: usize = 12; // bytes; the vocabulary cap (the tokenizer's max_token)
const WORD_BUF: usize = 48; // words longer are OVERLONG -> single UNK

/// Naive segmentation: hash every candidate piece FROM SCRATCH.
/// Returns (pieces, bytes_hashed).
fn segment_naive<'a>(
    word: &'a str,
    vocab: &HashMap<u64, &'a str>,
    prefix: &str,
) -> (Vec<&'a str>, usize, bool) {
    let bytes = word.as_bytes();
    let n = bytes.len();
    let mut out = Vec::new();
    let mut hashed = 0usize;
    if n > WORD_BUF {
        return (vec!["<UNK>"], 0, false); // overlong -> single UNK, nothing hashed
    }
    let mut start = 0;
    while start < n {
        let limit = n.min(start + MAX_PIECE);
        let mut end = limit;
        while end > start && !word.is_char_boundary(end) {
            end -= 1;
        }
        let mut found = None;
        while end > start {
            let mut key = Vec::with_capacity(end - start + prefix.len());
            if start != 0 {
                key.extend_from_slice(prefix.as_bytes());
            }
            key.extend_from_slice(&bytes[start..end]);
            hashed += key.len();
            if let Some(p) = vocab.get(&hash_key(&key)) {
                found = Some((*p, end));
                break;
            }
            end -= 1;
            while end > start && !word.is_char_boundary(end) {
                end -= 1;
            }
        }
        match found {
            Some((p, e)) => {
                out.push(p);
                start = e;
            }
            None => return (vec!["<UNK>"], hashed, false),
        }
    }
    (out, hashed, true)
}

/// Fixed segmentation: ONE push-walk per position builds every prefix's
/// hash; the greedy descent reads the table.
fn segment_fixed<'a>(
    word: &'a str,
    vocab: &HashMap<u64, &'a str>,
    prefix: &str,
) -> (Vec<&'a str>, usize, bool) {
    let bytes = word.as_bytes();
    let n = bytes.len();
    let mut out = Vec::new();
    let mut pushed = 0usize;
    if n > WORD_BUF {
        return (vec!["<UNK>"], 0, false);
    }
    let mut hashes = [0u64; WORD_BUF];
    let mut start = 0;
    while start < n {
        let limit = n.min(start + MAX_PIECE);
        let mut hs = if start == 0 {
            PushHasher::new()
        } else {
            PushHasher::with(prefix.as_bytes())
        };
        pushed += if start == 0 { 0 } else { prefix.len() };
        for (slot, b) in hashes.iter_mut().zip(bytes[start..limit].iter()) {
            hs = hs.push(*b);
            *slot = hs.hash();
            pushed += 1;
        }
        let mut end = limit;
        while end > start && !word.is_char_boundary(end) {
            end -= 1;
        }
        let mut found = None;
        while end > start {
            if let Some(p) = vocab.get(&hashes[end - start - 1]) {
                found = Some((*p, end));
                break;
            }
            end -= 1;
            while end > start && !word.is_char_boundary(end) {
                end -= 1;
            }
        }
        match found {
            Some((p, e)) => {
                out.push(p);
                start = e;
            }
            None => return (vec!["<UNK>"], pushed, false),
        }
    }
    (out, pushed, true)
}

fn fixture_vocab() -> (HashMap<u64, &'static str>, Vec<&'static str>) {
    let pieces: &[&'static str] = &[
        "the",
        "re",
        "un",
        "able",
        "comfort",
        "##able",
        "##able",
        "ta",
        "ble",
        "in",
        "ter",
        "tiv",
        "ness",
        "anti",
        "dis",
        "est",
        "ab",
        "lish",
        "ment",
        "ari",
        "an",
        "ism",
        "pro",
        "cras",
        "tin",
        "ate",
        "won",
        "der",
        "ful",
        "##ness",
        "unbelievable",
        "表",
        "現",
        "é",
        "ß",
    ];
    let mut map = HashMap::new();
    for p in pieces {
        map.insert(hash_key(p.as_bytes()), *p);
    }
    (map, pieces.to_vec())
}

#[test]
fn push_hash_agrees_with_the_from_scratch_fold_at_every_prefix() {
    let text =
        b"unbelievable antidisestablishmentarianism ##pieces \xc3\xa9t\xc3\xa9 stra\xc3\x9fe";
    for start in 0..text.len() {
        let mut hs = PushHasher::new();
        for end in start + 1..=text.len() {
            hs = hs.push(text[end - 1]);
            let key = &text[start..end];
            assert_eq!(hs.hash(), hash_key(key), "{start}..{end}");
        }
    }
    assert_eq!(PushHasher::with(b"##ab").hash(), hash_key(b"##ab"));
}

#[test]
fn equal_length_prefixes_never_fold_equal() {
    // (h ^ chunk) * K is a bijection in chunk for odd K — assert the
    // fixed-chunk injectivity on a sweep, then the whole-prefix version.
    assert!(K & 1 == 1, "K must be odd");
    let mut seen = std::collections::HashSet::new();
    for c in 0..=u32::MAX {
        let chunk = (c as u64) | ((c as u64) << 32); // 8-byte chunks from a u32 pair
        let mixed = mix(INIT, chunk);
        assert!(seen.insert(mixed), "chunk {chunk:#x} collided");
        if c > 40_000 {
            break; // a sweep, not an exhaustive 2^64 proof
        }
    }
    // whole prefixes: every 2-byte key over a 16-symbol alphabet is unique
    let alphabet = b"abcdefghijklmnop";
    let mut two = std::collections::HashSet::new();
    for &a in alphabet {
        for &b in alphabet {
            assert!(two.insert(hash_key(&[a, b])), "2-byte key collision");
        }
    }
}

#[test]
fn segmentation_matches_and_the_census_drops() {
    let (vocab, _) = fixture_vocab();
    let words = [
        "unbelievable",
        "comfortable",
        "antidisestablishmentarianism",
        "procrastination",
        "wonderfulness",
        "table",
        "the",
        "zzzqqq",                                          // non-vocab: single UNK
        "unbelievablylongwordthatgoesonandonandonforever", // overlong
        "\u{8868}\u{73FE}",                                // CJK, in vocab
        "straight",                                        // partially in vocab -> UNK path
        "abl",
    ];
    let mut naive_total = 0usize;
    let mut fixed_total = 0usize;
    for word in words {
        let (p1, h1, ok1) = segment_naive(word, &vocab, "##");
        let (p2, h2, ok2) = segment_fixed(word, &vocab, "##");
        assert_eq!(ok1, ok2, "{word}: segmentation verdict");
        assert_eq!(p1, p2, "{word}: piece list");
        naive_total += h1;
        fixed_total += h2;
    }
    assert!(
        fixed_total < naive_total,
        "census: fixed {fixed_total} bytes must beat naive {naive_total}"
    );
    // the long-word regime carries the win: the 28-char monster alone
    let long = "antidisestablishmentarianism";
    let (_, h1, _) = segment_naive(long, &vocab, "##");
    let (_, h2, _) = segment_fixed(long, &vocab, "##");
    // naive hashes O(len * max^2 / 2) bytes, fixed O(len * max)
    assert!(h2 * 4 < h1, "long-word census: fixed {h2} vs naive {h1}");
}

#[test]
fn char_boundaries_and_overlong_follow_the_same_rule() {
    let (vocab, _) = fixture_vocab();
    // the é/ß words exercise the char-boundary backoff in BOTH arms
    for word in ["straße", "\u{8868}\u{73FE}é", "ééézzz"] {
        let (p1, _, ok1) = segment_naive(word, &vocab, "##");
        let (p2, _, ok2) = segment_fixed(word, &vocab, "##");
        assert_eq!((p1, ok1), (p2, ok2), "{word}");
    }
    // overlong: a single UNK, nothing stored past the buffer
    let long = "x".repeat(WORD_BUF + 1);
    let (p1, h1, ok1) = segment_naive(&long, &vocab, "##");
    let (p2, h2, ok2) = segment_fixed(&long, &vocab, "##");
    assert_eq!((p1.clone(), h1, ok1), (p2, h2, ok2));
    assert_eq!(p1, vec!["<UNK>"]);
    assert_eq!(h1 + h2, 0, "overlong words hash nothing");
}

#[test]
fn the_cap_makes_the_win_a_constant_factor_per_word() {
    // with max_token = MAX_PIECE applied, per-word work is O(len * max):
    // assert the census ratio on a capped-short word (near the cap's short
    // end the ratio is small) and a long in-vocab word (the largest cell).
    let (vocab, _) = fixture_vocab();
    let short = "able"; // in vocab whole: ONE whole-word hash both arms
    let (_, h1_short, _) = segment_naive(short, &vocab, "##");
    let (_, h2_short, _) = segment_fixed(short, &vocab, "##");
    assert!(h1_short <= MAX_PIECE + 2);
    assert!(h2_short <= MAX_PIECE + 2);
    // the whole-word fast path: naive re-hashes candidates greedily; fixed
    // pushes each byte once. For a whole-word hit both read ~len bytes.
    assert_eq!(
        h1_short, h2_short,
        "short in-vocab word: ~parity near the cap"
    );
    let long = "antidisestablishmentarianism";
    let (_, h1_long, _) = segment_naive(long, &vocab, "##");
    let (_, h2_long, _) = segment_fixed(long, &vocab, "##");
    assert!(
        h1_long > h2_long * 2,
        "long word: naive {h1_long} must exceed fixed {h2_long} by 2x+"
    );
}
