// Batch 360 pre-entry G1 correctness check (standalone, std-only) — plan 371.
// Timing/bench arms are T4 (quiet box); this file checks ONLY the identities.

// ── R4: u128 funnel vs two-shift reference, exhaustive shifts + boundary pairs ──
fn combine_funnel(lo: u64, hi: u64, s: u32) -> u64 {
    let wide = ((hi as u128) << 64) | lo as u128;
    (wide >> (s % 64)) as u64
}

// reference: the bit-window law the funnel implements, on the domain where
// the two-shift form is even expressible. s == 0 is the TRAP: the direct
// two-shift form is UB there (hi << 64), the funnel returns lo — correct.
// s == 64 wraps: the % 64 makes the funnel PERIODIC (returns lo again) —
// the upstream iterator never calls s >= 64; the periodicity is the law's
// pinned shape, asserted separately.
fn combine_ref(lo: u64, hi: u64, s: u32) -> u64 {
    if s == 0 {
        return lo;
    }
    (lo >> s) | (hi << (64 - s))
}

#[test]
fn funnel_matches_reference_on_all_shifts_and_boundary_pairs() {
    let words = [
        0x0000_0000_0000_0000u64,
        0xFFFF_FFFF_FFFF_FFFF,
        0x8000_0000_0000_0001,
        0x1234_5678_9ABC_DEF0,
        0x0000_0000_0000_0001,
    ];
    for &lo in &words {
        for &hi in &words {
            // s in 1..=63: funnel == the two-shift form, exactly
            for s in 1u32..64 {
                let got = combine_funnel(lo, hi, s);
                let want = combine_ref(lo, hi, s);
                assert_eq!(got, want, "lo={lo:#x} hi={hi:#x} s={s}");
            }
            // s == 0: the funnel returns lo — the direct form would be UB
            assert_eq!(combine_funnel(lo, hi, 0), lo, "s=0 must return lo");
            // s == 64: periodic wrap — the % 64 lands back on s == 0
            assert_eq!(combine_funnel(lo, hi, 64), combine_funnel(lo, hi, 0));
        }
    }
    // a chained 3-word stream drains bit-exactly through the funnel
    let w = [
        0xDEAD_BEEF_CAFE_0001u64,
        0x0F0F_0F0F_0000_000F,
        0x8000_0000_0000_0000,
    ];
    for s in 1u32..64 {
        let expect = (w[0] >> s) | (w[1] << (64 - s));
        assert_eq!(combine_funnel(w[0], w[1], s), expect);
        let expect2 = (w[1] >> s) | (w[2] << (64 - s));
        assert_eq!(combine_funnel(w[1], w[2], s), expect2);
    }
}

// ── R1: flag-guarded aux write leaves the aux structure bit-identical ──
pub const MARKED: u32 = 1u32 << 31;

fn mark_naive(rows_state: &mut [u32], seen: &mut [u64], rows: &[u32]) {
    for &r in rows {
        let r = r as usize;
        seen[r / 64] |= 1u64 << (r % 64);
    }
    let _ = rows_state;
}

fn mark_flagged(rows_state: &mut [u32], seen: &mut [u64], first_slot: usize, rows: &[u32]) {
    if rows_state[first_slot] & MARKED == 0 {
        rows_state[first_slot] |= MARKED;
        for &r in rows {
            let r = (r & !MARKED) as usize;
            seen[r / 64] |= 1u64 << (r % 64);
        }
    }
}

#[test]
fn flag_guard_leaves_aux_structure_bit_identical() {
    // duplicate-heavy hit stream: the same key's rows marked many times
    let key_rows: Vec<u32> = (0..24).collect();
    let mut hits = Vec::new();
    for rep in 0..200u32 {
        let slot = (rep % 4) as usize;
        hits.push((slot, key_rows.clone()));
    }
    let mut seen_naive = vec![0u64; 1];
    let mut state_naive = vec![0u32; 4];
    for &(_slot, ref rows) in &hits {
        mark_naive(&mut state_naive, &mut seen_naive, rows);
    }
    let mut seen_flag = vec![0u64; 1];
    let mut state_flag = vec![0u32; 4];
    for &(slot, ref rows) in &hits {
        mark_flagged(&mut state_flag, &mut seen_flag, slot, rows);
    }
    assert_eq!(
        seen_naive, seen_flag,
        "G1: the marked set must be bit-identical"
    );
    // distinct hits still mark through the guard
    let mut seen2 = vec![0u64; 1];
    let mut state2 = vec![0u32; 4];
    for slot in 0..4usize {
        let rows: Vec<u32> = ((slot * 8) as u32..((slot + 1) * 8) as u32).collect();
        mark_flagged(&mut state2, &mut seen2, slot, &rows);
    }
    // rows 0..32 -> bits 0..31; the top 32 stay clear (MARKED never leaks in)
    assert_eq!(seen2, vec![0x0000_0000_FFFF_FFFFu64; 1]);
    // the flag bits are visible in the row state and never reach `seen`
    assert_eq!(state2, vec![MARKED; 4]);
}

// ── R2 face identity: char count = byte len - continuation-byte count (valid UTF-8) ──
fn continuation_count(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| (b & 0xC0) == 0x80).count()
}

#[test]
fn char_count_identity_holds_on_mixed_utf8() {
    let cases = [
        "hello world",
        "héllo wörld — em-dash and … ellipsis",
        " mixed สวัสดี emoji 🦀 byte",
        "",
        "ish",
    ];
    for case in &cases {
        let via_identity = case.len() - continuation_count(case.as_bytes());
        assert_eq!(via_identity, case.chars().count(), "case={case:?}");
    }
}
