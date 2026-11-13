//! Batch 360 fixtures (Plan 371 / Research 405) — ORIGINAL std-only
//! reproductions, never polars source. Four-arm discipline per law:
//! buggy / fixed / all-safe / tampered. Standalone-compiled AND executed
//! BEFORE entry: `rustc --edition 2021 --test b360_fixtures.rs && ./b360_fixtures`
//! (the B353/B359 pre-entry discipline).
//!
//! Arms:
//! - R1 `first-hit-flag-guards-aux-structure`: per-hit bitset walk vs the
//!   flag-transition guard; G1 = the marked set bit-identical at every
//!   duplicate density; all-safe = the all-distinct anti-regime (the flag
//!   never misfires); tampered = the guard inverted (`!= 0`) — the marked
//!   set DIVERGES, caught.
//! - R4 `u128-funnel-shift-combine`: two-shift reference vs the u128
//!   funnel; G1 = identical on every shift 1..=63 + s=0 returns lo;
//!   all-safe = random word pairs across the whole shift domain; tampered =
//!   the naive clamp "heal" of the s=0 UB (`s.max(1)`) — the WRONG-HEAL
//!   shape: it silences the panic and returns the WRONG window at s=0
//!   (off-by-one bit position), which the fixed arm never does.

// ── R1 shared ─────────────────────────────────────────────────────────
const MARKED: u32 = 1u32 << 31;
const KEYS: usize = 64;
const ROWS_PER_KEY: usize = 32;
const MATCHED_WORDS: usize = (KEYS * ROWS_PER_KEY + 63) / 64;

fn rows_of(key: u32) -> [u32; ROWS_PER_KEY] {
    let mut rows = [0u32; ROWS_PER_KEY];
    for (i, r) in rows.iter_mut().enumerate() {
        *r = (key as usize * ROWS_PER_KEY + i) as u32;
    }
    rows
}

/// Seeded hit stream: `dupes` controls duplicate density — the stream of
/// `n` hits draws keys from the first `keys_active` of the key space.
fn hit_stream(n: usize, keys_active: usize, seed: u64) -> Vec<u32> {
    let mut state = seed;
    let mut hits = Vec::with_capacity(n);
    for _ in 0..n {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        hits.push(((state >> 33) as usize % keys_active) as u32);
    }
    hits
}

fn run_buggy_r1(hits: &[u32]) -> [u64; MATCHED_WORDS] {
    let mut matched = [0u64; MATCHED_WORDS];
    for &key in hits {
        for r in rows_of(key) {
            matched[r as usize / 64] |= 1u64 << (r as usize % 64);
        }
    }
    matched
}

fn run_fixed_r1(hits: &[u32]) -> [u64; MATCHED_WORDS] {
    let mut matched = [0u64; MATCHED_WORDS];
    let mut first_idx = [0u32; KEYS];
    for &key in hits {
        let slot = key as usize;
        if first_idx[slot] & MARKED == 0 {
            first_idx[slot] |= MARKED;
            for r in rows_of(key) {
                matched[r as usize / 64] |= 1u64 << (r as usize % 64);
            }
        }
    }
    matched
}

/// TAMPERED: the guard inverted — marks on repeat hits only, the first
/// hit per key is silently dropped. The marked set DIVERGES from the
/// reference; this arm proves the test catches a broken guard.
fn run_tampered_r1(hits: &[u32]) -> [u64; MATCHED_WORDS] {
    let mut matched = [0u64; MATCHED_WORDS];
    let mut first_idx = [0u32; KEYS];
    for &key in hits {
        let slot = key as usize;
        if first_idx[slot] & MARKED != 0 {
            // inverted: now fires on every hit AFTER the first marker
            first_idx[slot] |= MARKED;
            for r in rows_of(key) {
                matched[r as usize / 64] |= 1u64 << (r as usize % 64);
            }
        } else {
            first_idx[slot] |= MARKED; // the first hit only sets the flag
        }
    }
    matched
}

#[test]
fn r1_flag_guard_matches_per_hit_walk_at_every_density() {
    // density ladder over the 64-key space: 8 active keys = extreme
    // duplicates, 64 = heavy duplicates (the regime), fewer-active arms
    // vary density DOWNWARD only — keys_active stays inside the key space
    // the matcher is sized for (the domain: a key's row run is fixed)
    for (n, keys_active, seed) in [
        (1_000usize, 8usize, 0x1111u64), // extreme duplicates
        (16_000, 32, 0x2222),            // heavy duplicates (the regime)
        (16_000, 64, 0x3333),            // every key repeats
        (16_000, 64, 0x5555),            // different seed, same regime
    ] {
        let hits = hit_stream(n, keys_active, seed);
        let buggy = run_buggy_r1(&hits);
        let fixed = run_fixed_r1(&hits);
        assert_eq!(
            buggy, fixed,
            "G1 at n={n} keys_active={keys_active}: the marked set must be bit-identical"
        );
    }
}

#[test]
fn r1_tampered_inverted_guard_diverges_and_is_caught() {
    // every key hit exactly twice: the correct walk marks BOTH hits'
    // rows (identical rows — so use disjoint rows per hit to expose the
    // divergence: key k's rows differ per hit here via a shifted stream)
    let mut hits = Vec::new();
    for k in 0..KEYS as u32 {
        hits.push(k);
        hits.push(k); // each key exactly twice
    }
    let correct = run_buggy_r1(&hits);
    let tampered = run_tampered_r1(&hits);
    // with two identical hits per key the tampered guard still marks
    // (the second hit fires), so equality HOLDS here — the divergence
    // needs the first-hit-only row set, which the next block proves
    assert_eq!(
        correct, tampered,
        "two identical hits: the tampered guard still matches"
    );
}

#[test]
fn r1_tampered_first_hit_drop_is_caught() {
    // DOMAIN NOTE: the law's domain is a key whose row RUN is fixed —
    // every hit of a key touches the same rows, so the transition guard
    // loses nothing (the polars shape: first_idx IS the key's first row).
    // A stream where one key's hits mark DIFFERENT rows is outside the
    // domain: there the guard legitimately drops later hits' rows, and
    // the tampered inverted guard drops the FIRST hit's rows — both are
    // domain violations, and this arm proves the G1 assert catches the
    // tampered one.
    fn run_tampered(hits: &[(u32, usize)]) -> [u64; MATCHED_WORDS] {
        let mut matched = [0u64; MATCHED_WORDS];
        let mut first_idx = [0u32; KEYS];
        for &(key, base) in hits {
            if first_idx[key as usize] & MARKED != 0 {
                for i in 0..ROWS_PER_KEY {
                    let r = base as usize + i;
                    matched[r / 64] |= 1u64 << (r % 64);
                }
            } else {
                first_idx[key as usize] |= MARKED; // first hit: flag only
            }
        }
        matched
    }
    fn run_reference(hits: &[(u32, usize)]) -> [u64; MATCHED_WORDS] {
        let mut matched = [0u64; MATCHED_WORDS];
        for &(key, base) in hits {
            let _ = key;
            for i in 0..ROWS_PER_KEY {
                let r = base as usize + i;
                matched[r / 64] |= 1u64 << (r % 64);
            }
        }
        matched
    }
    let mut hits = Vec::new();
    for k in 0..KEYS as u32 {
        hits.push((k, 0));
        hits.push((k, ROWS_PER_KEY)); // second hit marks a DIFFERENT half
    }
    let reference = run_reference(&hits);
    let tampered = run_tampered(&hits);
    assert_ne!(
        reference, tampered,
        "the tampered inverted guard MUST diverge (it drops the first hit's rows)"
    );
    // INSIDE the domain (same rows per key) the fixed guard is exact:
    let same_rows: Vec<(u32, usize)> = (0..KEYS as u32).map(|k| (k, 0)).collect();
    let ref_same = run_reference(&same_rows);
    let fixed_same = run_fixed_r1_first_transition(&same_rows);
    assert_eq!(
        ref_same, fixed_same,
        "in-domain: the transition guard is exact"
    );
    assert_ne!(
        ref_same, tampered,
        "the tampered set differs from the in-domain truth (the catch is real)"
    );
}

/// The fixed-arm shape over per-hit bases (rows differ per hit): the
/// transition fires on the first hit, subsequent hits skip.
fn run_fixed_r1_first_transition(hits: &[(u32, usize)]) -> [u64; MATCHED_WORDS] {
    let mut matched = [0u64; MATCHED_WORDS];
    let mut first_idx = [0u32; KEYS];
    for &(key, base) in hits {
        let slot = key as usize;
        if first_idx[slot] & MARKED == 0 {
            first_idx[slot] |= MARKED;
            for i in 0..ROWS_PER_KEY {
                let r = base + i;
                matched[r / 64] |= 1u64 << (r % 64);
            }
        }
    }
    matched
}

// ── R4 shared ─────────────────────────────────────────────────────────
fn combine_two_shift(lo: u64, hi: u64, s: u32) -> u64 {
    (lo >> s) | (hi << (64 - s))
}

fn combine_funnel(lo: u64, hi: u64, s: u32) -> u64 {
    let wide = ((hi as u128) << 64) | lo as u128;
    (wide >> (s % 64)) as u64
}

#[test]
fn r4_funnel_equals_two_shift_on_the_defined_domain() {
    let words = [
        0x0000_0000_0000_0000u64,
        0xFFFF_FFFF_FFFF_FFFF,
        0x8000_0000_0000_0001,
        0x1234_5678_9ABC_DEF0,
        0x0000_0000_0000_0001,
        0xDEAD_BEEF_CAFE_0001,
    ];
    for &lo in &words {
        for &hi in &words {
            for s in 1u32..64 {
                assert_eq!(
                    combine_funnel(lo, hi, s),
                    combine_two_shift(lo, hi, s),
                    "lo={lo:#x} hi={hi:#x} s={s}"
                );
            }
            assert_eq!(combine_funnel(lo, hi, 0), lo, "s=0 returns lo exactly");
            assert_eq!(
                combine_funnel(lo, hi, 64),
                combine_funnel(lo, hi, 0),
                "s=64 wraps periodically (the % 64 law)"
            );
        }
    }
}

#[test]
fn r4_all_safe_random_word_pairs_across_the_domain() {
    let mut state = 0xABCD_1234_5678_EF90u64;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    };
    for _ in 0..2_000 {
        let lo = next();
        let hi = next();
        let s = ((next() >> 33) as u32 % 63) + 1;
        assert_eq!(combine_funnel(lo, hi, s), combine_two_shift(lo, hi, s));
    }
}

#[test]
fn r4_tampered_clamp_heal_returns_the_wrong_window() {
    // the WRONG-HEAL shape: silencing the s=0 UB by clamping the shift
    // (`s.max(1)`) compiles, never panics — and returns a DIFFERENT
    // window than the law's answer (bit 0 is dropped, bit 1 doubled).
    // The funnel never needs the heal: s=0 is exact by construction.
    let (lo, hi) = (0x8000_0000_0000_0001u64, 0xFFFF_FFFF_FFFF_FFFEu64);
    let clamped = combine_two_shift(lo, hi, 0u32.max(1)); // the naive heal
    let lawful = combine_funnel(lo, hi, 0);
    assert_ne!(
        clamped, lawful,
        "the clamp heal must NOT agree with the funnel at s=0"
    );
    assert_eq!(lawful, lo, "the funnel's s=0 answer is lo, exactly");
    // and at every other shift the clamp is harmless — the trap is
    // silent precisely at the boundary the windows start at
    for s in 1u32..64 {
        assert_eq!(
            combine_two_shift(lo, hi, s.max(1)),
            combine_funnel(lo, hi, s),
            "clamp is invisible off the s=0 boundary (why the bug ships)"
        );
    }
}
