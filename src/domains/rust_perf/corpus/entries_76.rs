//! Batch 360 — the polars revisit #2 delta `rust_perf` batch (Plan 371 /
//! Research 405): the B66 surface re-opened by the 2.0 kernel layer. Two
//! BENCHABLE rules — the first-hit flag guard (a one-bit flag in the hot
//! entry gates the aux-structure write; the bit-steal face rides the index
//! word's top bit) and the u128 funnel shift-combine (two loaded WORDS
//! shifted as one 128-bit wide word; the `% 64` is both the s=0 correctness
//! and the periodicity) — plus one count-neutral FACE on B244
//! `iter-count-bulk-specialization` (the char-count identity, landed
//! THERE per the T2 adjudication, not here).
//!
//! Source: `pola-rs/polars` @ `437948302e11e15d47fc8b2882f9edb0848e5ebc`
//! (master tip 2026-11-13T15:59:46Z; MIT — copyright Ritchie Vink; every
//! quote verified byte-exact at this pin this batch — all nine Research-405
//! sites stood the 2.5h tip move; clone removed at close). Prior lineage:
//! Research 073 / Plan 011 / Batch 66 (2026-08-19, the FIRST polars
//! distill — `sparse-set-bit-iteration-tz-blsr` +
//! `sortedness-check-branchless-accumulate` both `ParetoDominant`). The T2
//! face-resolve precedent is the B66 float-sum kill: the count mechanism
//! resolved to a positive-instance FACE on B244 (`entries_30`), the
//! char-count identity carried as face text there.
//!
//! ROUTING per row: `rust_perf` — host-side hot-path laws over in-memory
//! structures; `kernel_opt` loses (no GPU surface — the bitmap/string
//! kernels here are CPU kernels), sec loses (no attacker-facing input
//! validation — the flag guard is duplicate-heavy STREAM correctness, the
//! funnel is pure arithmetic).
//!
//! ORIGINAL Rust fixtures (`corpus/fixtures/b360_fixtures.rs`,
//! standalone-compiled AND executed before entry — rustc --edition 2021
//! --test, four-arm discipline per law), never pasted source.
//! Batch 360 / Plan 371 / Research 405.

#[allow(clippy::too_many_lines)] // the corpus entries ARE the payload
pub fn entries() -> Vec<crate::domains::rust_perf::PerfRule> {
    vec![
        // ── caching: a duplicate-heavy hit stream writes a shared side
        // structure on EVERY hit → keep a one-bit flag IN the hot entry
        // (or steal the index word's top bit) and touch the side structure
        // only on the first-hit TRANSITION. ─────────────────────────────
        // Source: pola-rs/polars @ 437948302, polars-expr/src/idx_table/
        // matched_rows.rs (+ the second in-source instance in idx_table/
        // single_key.rs).
        crate::domains::rust_perf::PerfRule {
            rule_id: "first-hit-flag-guards-aux-structure".to_string(),
            name: "a duplicate-heavy hit stream writes a shared side structure (bitset fetch_or RMW on a contended word) on EVERY hit → keep a one-bit flag IN the hot entry — stolen from the row-index word's top bit where the type has headroom — and touch the side structure only on the first-hit TRANSITION; repeat hits pay one load+branch, never the RMW".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// NAIVE: every hit on a key re-marks its rows into the matched
// bitset. The join semantics produce duplicate-heavy hit streams, so
// the shared bitset words take a read-modify-write on EVERY repeat
// hit — the RMW dominates and the marked SET is written identically
// hundreds of times.

pub struct JoinMatcher {
    // first row index of each key's row run
    first_idx: Vec<u32>,
    // the matched-rows bitset (cold side structure)
    matched: Vec<u64>,
}

impl JoinMatcher {
    pub fn mark_hit(&mut self, key: usize, rows: &[u32]) {
        // every hit RMWs the bitset word, even when the key was
        // already marked by an earlier hit of the same probe batch
        for &r in rows {
            let (w, b) = (r as usize / 64, r as usize % 64);
            self.matched[w] |= 1u64 << b;
        }
        let _ = key;
    }

    pub fn matched_count(&self) -> usize {
        self.matched.iter().map(|w| w.count_ones() as usize).sum()
    }
}
"#.to_string(),
            fixed_code: r#"// OPT: the hot entry carries the flag. polars steals the row-index
// word's TOP BIT (IdxSize::BITS - 1) so the flag costs no extra
// memory: the first row index of a key reads idx | MARKED once the
// key has been marked, and mark_rows checks the flag BEFORE touching
// the bitset — the transition fires ONCE per key, repeat hits pay
// one load + branch and never re-enter the side structure.

pub const MARKED: u32 = 1u32 << 31; // stolen from the index word's top bit

pub struct JoinMatcher {
    // first row index of each key's row run; carries MARKED once marked
    first_idx: Vec<u32>,
    // the matched-rows bitset (cold side structure)
    matched: Vec<u64>,
}

impl JoinMatcher {
    pub fn mark_hit(&mut self, key: usize, rows: &[u32]) {
        // the flag TRANSITION gates the side structure: only the first
        // hit of a key walks its rows into the bitset
        if self.first_idx[key] & MARKED == 0 {
            self.first_idx[key] |= MARKED;
            for &r in rows {
                let r = r & !MARKED; // strip the flag bit from row indices
                let (w, b) = (r as usize / 64, r as usize % 64);
                self.matched[w] |= 1u64 << b;
            }
        }
    }

    pub fn matched_count(&self) -> usize {
        self.matched.iter().map(|w| w.count_ones() as usize).sum()
    }
}

#[test]
fn flag_guard_matches_the_per_hit_bitset_exactly() {
    // duplicate-heavy stream: the same keys' rows hit repeatedly
    let mut matcher_naive = JoinMatcher {
        first_idx: vec![0; 64],
        matched: vec![0u64; 4],
    };
    let mut matcher_flag = JoinMatcher {
        first_idx: vec![0; 64],
        matched: vec![0u64; 4],
    };
    let key_rows: Vec<u32> = (0..32).collect();
    for key in 0..200usize {
        let k = key % 64;
        matcher_naive.mark_hit(k, &key_rows);
        matcher_flag.mark_hit(k, &key_rows);
    }
    // G1: the marked SET is bit-identical, however the writes amortize
    assert_eq!(
        matcher_naive.matched_count(),
        matcher_flag.matched_count()
    );
    assert_eq!(matcher_naive.matched, matcher_flag.matched);
}
"#.to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 TWO-RUN STABLE (13.62x/52.32x/43.03x then 12.77x/50.47x/63.83x) at n=[1024, 16384, 262144] over a seeded duplicate-heavy 64-key hit stream, allocs 0→0 bytes 0→0 every size both runs (amo, M5 Ultra / Mac, release, rust_perf_bench validator, 2026-11-13 ~23:52 +07, load 1.25 quiet — the Zed rustc had just drained; CARGO_TARGET_DIR=/tmp/b360_target; the validator numbers are ours). SOURCE (pola-rs/polars @ 437948302e11e15d47fc8b2882f9edb0848e5ebc, master tip 2026-11-13T15:59:46Z, MIT; quotes verified byte-exact at the .raw/polars pin clone this batch). THE LAW (polars-expr/src/idx_table/matched_rows.rs:4-5, verbatim): '/// Set on the first row index of a key once a probe key matched it.' + 'pub const MARKED: IdxSize = 1 << (IdxSize::BITS - 1);' — THE FLAG IS STOLEN FROM THE ROW-INDEX WORD'S TOP BIT, so the hot entry carries the marker at zero memory cost. THE TRANSITION GUARD (matched_rows.rs:28-40, verbatim shape): '/// Marks the rows of a key, unless its first row has the MARKED flag.' + 'if first_idx & MARKED == 0 { first.store(first_idx | MARKED); for row in rows { self.mark(row.load() & !MARKED); } }' — the side-structure walk fires ONCE per key; repeat hits pay one load + branch and never re-enter the bitset. SECOND IN-SOURCE INSTANCE (idx_table/single_key.rs:29, verified): 'nulls_marked: RelaxedCell<bool>' guarding the null-keys bitset marking — the pattern is not a one-site trick. REGIME: duplicate-heavy hit streams (the join semantics produce exactly those — the fixture's density ladder measured the 8-active-key extreme at the same verdict); any hot path writing a shared side structure (bitset, seen-set, dedup list) per event. ANTI-REGIMES: all-distinct hits (the flag adds a load + branch and never fires — the fixture's wide-key-space arm holds the verdict anyway because the buggy arm's repeat walk is what dominates; a truly single-hit-per-key stream erases the win, disclosed); the side structure IS the answer being accumulated per hit (no transition semantics to exploit); the flag bit cannot be stolen (index word at capacity — use a side flag array, the win survives, the bit-steal face does not). G1: the marked SET is bit-identical at every density (fixture arm 1 + the in-entry test). DEDUP ADJUDICATION: bitflag-pack-when-footprint-dominates (entries_05) owns the FOOTPRINT axis (bools → bitfield byte); THIS owns the probe-avoidance axis (the flag gates a WRITE, not a byte); single-entry-last-key-lookup-cache-hash-avoidance (B334) is the sequential-streak memo (N=1 slot, locality — THIS is per-key transitions over arbitrary order); sparse-set-bit-iteration-tz-blsr (B66) is the ITERATION side of the same bitset (tzcnt walk — compose). Batch 360 / Plan 371 / Research 405 (the polars revisit #2).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(first_hit_flag_guards_aux_structure_bench_spec()),
        },
        // ── arithmetic: two loaded WORDS combined across a word boundary
        // via a two-shift form that is UB at shift=0 → the u128 funnel —
        // one shift of the wide word, the `% 64` making shift=0 exact
        // (and the whole thing PERIODIC, which the domain law pins). ────
        // Source: pola-rs/polars @ 437948302, polars-arrow/src/bitmap/
        // iterator.rs FastU64BitmapIter::combine.
        crate::domains::rust_perf::PerfRule {
            rule_id: "u128-funnel-shift-combine".to_string(),
            name: "combining two loaded words across a shift boundary with (lo >> s) | (hi << (64 - s)) is UB at s=0 and three ops with a dependency chain → the u128 FUNNEL: join into one wide word and shift it once, ((hi as u128) << 64 | lo) >> (s % 64) — the % 64 makes s=0 exact AND makes the combine periodic, and the compiler emits the single SHRD".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: r#"// NAIVE: extract a window of bits that crosses a word boundary by
// shifting the two words separately and OR-ing. At s = 0 the second
// shift is 64 — UB in Rust (panics in debug, wraps wrong in release)
// — so every caller must remember to special-case the boundary the
// window actually starts at.

pub struct WindowStream<'a> {
    words: &'a [u64],
    pos: usize,
}

impl WindowStream<'_> {
    // one 64-bit window at bit offset `skip` past `pos`
    pub fn next_window(&mut self, skip: u32) -> u64 {
        let lo = self.words[self.pos];
        let hi = self.words[self.pos + 1];
        // the boundary trap: skip == 0 shifts hi by 64
        (lo >> skip) | (hi << (64 - skip))
    }
}
"#.to_string(),
            fixed_code: r#"// OPT: the funnel — the two words join into ONE 128-bit wide word
// and the window is a single shift of it. The % 64 is load-bearing
// twice: at s == 0 it returns lo exactly (where the two-shift form is
// UB), and it makes the shift PERIODIC (s == 64 wraps to s == 0's
// answer — the domain law; callers that need the high word shift by
// 64 explicitly). One wide shift also compiles to the single SHRD
// instruction on x86-64 instead of a three-op dependency chain.

pub struct WindowStream<'a> {
    words: &'a [u64],
    pos: usize,
}

impl WindowStream<'_> {
    // one 64-bit window at bit offset `skip` past `pos`; s == 0 returns
    // lo exactly, s == 64 wraps (periodic) — documented domain law
    pub fn next_window(&mut self, skip: u32) -> u64 {
        let lo = self.words[self.pos];
        let hi = self.words[self.pos + 1];
        let wide = ((hi as u128) << 64) | lo as u128;
        (wide >> (skip % 64)) as u64
    }
}

#[test]
fn funnel_matches_the_two_shift_reference_on_every_shift() {
    let words = [
        0x8000_0000_0000_0001u64,
        0x1234_5678_9ABC_DEF0,
        0x0000_0000_0000_0001,
    ];
    for w in 0..words.len() - 1 {
        let (lo, hi) = (words[w], words[w + 1]);
        // s in 1..=63: the funnel equals the two-shift form, exactly
        for s in 1u32..64 {
            let want = (lo >> s) | (hi << (64 - s));
            let got = {
                let wide = ((hi as u128) << 64) | lo as u128;
                (wide >> (s % 64)) as u64
            };
            assert_eq!(got, want, "lo={lo:#x} hi={hi:#x} s={s}");
        }
        // s == 0: the funnel returns lo — the two-shift form is UB here
        let got = {
            let wide = ((hi as u128) << 64) | lo as u128;
            (wide >> (0 % 64)) as u64
        };
        assert_eq!(got, lo);
        // s == 64: periodic wrap — the % 64 lands back on s == 0
        let got64 = {
            let wide = ((hi as u128) << 64) | lo as u128;
            (wide >> (64 % 64)) as u64
        };
        assert_eq!(got64, lo);
    }
}
"#.to_string(),
            context_notes: Some("MEASURED MIXED TWO-RUN (run 1 ParetoDominant 2/0/1: 1.15x/1.08x/0.98x-tie; run 2 Mixed 1/0/2: 1.08x/1.03x-tie/0.99x-tie) at n=[1024, 16384, 262144] over seeded random word pairs, allocs 0→0 bytes 0→0 every size both runs (same box posture as R1 above: amo, release, rust_perf_bench validator, 2026-11-13, load 1.25 quiet, /tmp/b360_target). THE HONEST VERDICT: a small real win at the small-n end, noise-adjacent at 16K+, tie at 256K — the load/rotate chain dominates at stream scale, so this rule lands for CORRECTNESS-FIRST reasons, with the timing disclosed as marginal. THE LAW (polars-arrow/src/bitmap/iterator.rs:429-435, verbatim): 'fn combine(&self, lo: u64, hi: u64) -> u64 {' + '// Compiles to 128-bit SHRD instruction on x86-64.' + '// Yes, the % 64 is important for the compiler to generate optimal code.' + 'let wide = ((hi as u128) << 64) | lo as u128;' + '(wide >> (self.shift % 64)) as u64'. THE CORRECTNESS CONTENT IS THE POINT: the two-shift form (lo >> s) | (hi << (64 - s)) is UB AT s=0 (hi << 64 panics debug, wraps wrong release) — every naive caller must remember the boundary special-case; the u128 funnel makes s=0 exact BY CONSTRUCTION (wide >> 0 = lo) and THE % 64 MAKES THE COMBINE PERIODIC (s == 64 wraps to s == 0's answer — the domain law, fixture-pinned; callers needing the high word shift explicitly). CODEGEN: one wide shift = the single SHRD on x86-64 vs a three-op dependency chain; on aarch64 the funnel lowers to extr (the funnel-shift instruction exists on both ISAs — the source's comment names x86-64 because that is what they profiled). REGIME: bitstream window extraction across word boundaries (bitmap iterators, bit-packed decoders, varint-adjacent readers). ANTI-REGIMES: shifts bounded away from 0 by construction (the caller's own invariant already prevents the UB — the funnel buys compactness, not safety); hot loops where the rotate/xor chain dominates (the tie is honest). TAMPERED-ARM LESSON (fixture-pinned): the naive clamp heal (`s.max(1)`) silences the panic and returns the WRONG window at s=0 — silently, exactly at the boundary windows start at; the funnel needs no heal. DEDUP ADJUDICATION: unaligned-word-load-from-bytes (B69) assembles a word FROM BYTES (try_into + from_le_bytes); THIS shifts two loaded WORDS as one 128-bit unit — adjacent-distinct; full-vector-bit-identity-over-argmax is a numerics identity, unrelated. Batch 360 / Plan 371 / Research 405.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(u128_funnel_shift_combine_bench_spec()),
        },
    ]
}

/// The R1 density ladder: seeded duplicate-heavy hit streams over a
/// 64-key matcher at four duplicate densities × three stream sizes.
#[cfg(feature = "rust_perf_bench")]
fn first_hit_flag_guards_aux_structure_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        // stream sizes; the density axis is INSIDE make_input (n picks
        // the size, the seeded stream carries ~99% duplicates — the
        // duplicate-heavy regime the rule names; the anti-regime
        // all-distinct posture is re-measured by the batch360 test on
        // demand via the density-0 arm of the batch test, not here)
        sizes: vec![1_024, 16_384, 262_144],
        harness: r#"
const MARKED: u32 = 1u32 << 31;
const KEYS: usize = 64;
const ROWS_PER_KEY: usize = 32;
const MATCHED_WORDS: usize = (KEYS * ROWS_PER_KEY + 63) / 64;

pub struct Input {
    hits: Vec<u32>, // sequence of key ids (0..KEYS), heavily duplicated
}

pub fn make_input(n: usize) -> Input {
    // seeded duplicate-heavy stream: LCG keys over a 64-key space —
    // ~99% of hits land on already-marked keys (the join regime)
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut hits = Vec::with_capacity(n);
    for _ in 0..n {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        hits.push(((state >> 33) as usize % KEYS) as u32);
    }
    Input { hits }
}

fn rows_of(key: u32) -> [u32; ROWS_PER_KEY] {
    let mut rows = [0u32; ROWS_PER_KEY];
    for (i, r) in rows.iter_mut().enumerate() {
        *r = (key as usize * ROWS_PER_KEY + i) as u32;
    }
    rows
}

// BUGGY: every hit re-walks its rows into the bitset (per-hit RMW on
// the shared words, no transition guard). The count folds the whole
// marked set each call so the walk cannot be dead-code-eliminated.
pub fn run_buggy(input: &Input) {
    let mut matched = [0u64; MATCHED_WORDS];
    for &key in &input.hits {
        for r in rows_of(key) {
            matched[r as usize / 64] |= 1u64 << (r as usize % 64);
        }
    }
    let count: usize = matched.iter().map(|w| w.count_ones() as usize).sum();
    std::hint::black_box(count);
}

// FIXED: the flag TRANSITION gates the walk — repeat hits pay one
// load + branch and never re-enter the bitset. first_idx carries the
// MARKED bit per key (the polars bit-steal face).
pub fn run_fixed(input: &Input) {
    let mut matched = [0u64; MATCHED_WORDS];
    let mut first_idx = [0u32; KEYS];
    for &key in &input.hits {
        let slot = key as usize;
        if first_idx[slot] & MARKED == 0 {
            first_idx[slot] |= MARKED;
            for r in rows_of(key) {
                matched[r as usize / 64] |= 1u64 << (r as usize % 64);
            }
        }
    }
    let count: usize = matched.iter().map(|w| w.count_ones() as usize).sum();
    std::hint::black_box(count);
}
"#
        .to_string(),
    }
}

/// The R4 sweep: word-window extraction across boundaries at three
/// stream sizes; the funnel vs the two-shift form (s kept in 1..=63 so
/// the buggy arm is DEFINED — the s=0 UB is the trap the rule names,
/// pinned by the four-arm fixture's tampered arm instead).
#[cfg(feature = "rust_perf_bench")]
fn u128_funnel_shift_combine_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![1_024, 16_384, 262_144],
        harness: r#"
pub struct Input {
    words: Vec<u64>,
    skips: Vec<u32>, // one shift per window draw, all in 1..=63
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0xC0FF_EE12_3456_789A;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    };
    let words: Vec<u64> = (0..n + 1).map(|_| next()).collect();
    let skips: Vec<u32> = (0..n).map(|_| ((next() >> 33) as u32 % 63) + 1).collect();
    Input { words, skips }
}

// BUGGY: the three-op two-shift chain per window (s kept in 1..=63 so
// this arm is DEFINED — the s=0 UB is the trap the rule names; the
// four-arm fixture's tampered arm pins it)
pub fn run_buggy(input: &Input) {
    let mut acc = 0u64;
    for (i, &s) in input.skips.iter().enumerate() {
        let lo = input.words[i];
        let hi = input.words[i + 1];
        acc = acc.rotate_left(1) ^ ((lo >> s) | (hi << (64 - s)));
    }
    std::hint::black_box(acc);
}

// FIXED: the single u128 funnel shift
pub fn run_fixed(input: &Input) {
    let mut acc = 0u64;
    for (i, &s) in input.skips.iter().enumerate() {
        let lo = input.words[i];
        let hi = input.words[i + 1];
        let wide = ((hi as u128) << 64) | lo as u128;
        acc = acc.rotate_left(1) ^ ((wide >> (s % 64)) as u64);
    }
    std::hint::black_box(acc);
}
"#
        .to_string(),
    }
}
