//! Batch 371 rust_perf fixture preflight — the codegraph kernel distill
//! (Plan 385 / Research 418): `colbymchenry/codegraph` @ `cc17b69a66adeb41eab9b4bf6d2f61e8cd311b81`
//! (MIT, re-pinned at T0 2026-11-18 — the tip moved 62 commits past the
//! verdict pin `44098dec`, quotes re-verified at the NEW pin). ORIGINAL
//! plain-Rust reproductions — zero repo source pasted (the MECHANISMS are
//! re-authored with our own constants; the source quotes stay in the corpus
//! context_notes). Compiled AND executed BEFORE entry (the b363..b369
//! pre-entry discipline):
//! `rustc --edition 2021 --test b371_fixtures.rs && ./b371_fixtures`
//!
//! Arms per rule (buggy / fixed / anti-regime / tamper):
//! - F1 `hex-encode-lut-two-chars-not-format-per-byte` (BENCHABLE — the GOAT
//!   lead): the LUT form is BYTE-IDENTICAL to the `format!` oracle over ALL
//!   256 byte values + the 16-byte digest shape; the buggy arm's per-byte
//!   `String` allocations are counted (the alloc axis); the LUT allocates
//!   once per id; the tampered LUT (one wrong nibble constant) fails
//!   byte-identity loudly.
//!
//! GOAT pre-registration (R1, the validator runs at the cargo window):
//! bytes = digest[..16] (the 16-byte node-id tail, 32 hex chars), sizes
//! n = 1K / 16K / 262K ids; bar: fixed ParetoDominant (≥1.05× at every
//! size, no axis loss) + allocs 17-class → 1 per id (buggy allocates the
//! `format!` String per byte + the `with_capacity` head, fixed allocates
//! the head only); G1 byte-identity is the fixture's exhaustive pin.

#[test]
fn f1_lut_is_byte_identical_to_the_format_oracle_over_all_bytes() {
    // The oracle: std's LowerHex via format! (what the buggy arm does).
    fn oracle_hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    // The fixed form: the const LUT, two pushes per byte, zero alloc per byte.
    const HEX_LUT: &[u8; 16] = b"0123456789abcdef";
    fn lut_hex(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        let mut raw = Vec::with_capacity(bytes.len() * 2);
        for b in bytes {
            raw.push(HEX_LUT[(b >> 4) as usize]);
            raw.push(HEX_LUT[(b & 0x0f) as usize]);
        }
        // SAFETY-free: the LUT is pure ASCII, the push_str borrows it.
        out.push_str(unsafe { std::str::from_utf8_unchecked(&raw) });
        out
    }

    // Exhaustive over every single byte value.
    for b in 0u8..=255u8 {
        assert_eq!(
            lut_hex(&[b]),
            oracle_hex(&[b]),
            "LUT diverges from the format! oracle at byte {b:#04x}"
        );
    }
    // The digest shape: 16-byte tails (the node-id tail), a few patterns.
    let mut digest = [0u8; 16];
    for i in 0..16 {
        digest[i] = (i as u8).wrapping_mul(37).wrapping_add(11);
    }
    assert_eq!(lut_hex(&digest), oracle_hex(&digest));
    let full = [0xffu8; 16];
    assert_eq!(lut_hex(&full), oracle_hex(&full));
    let zero = [0u8; 16];
    assert_eq!(lut_hex(&zero), oracle_hex(&zero));
}

#[test]
fn f1_alloc_axis_buggy_allocates_per_byte_fixed_allocates_per_id() {
    // A counting allocator over the two forms. The buggy arm allocates the
    // `format!` String PER BYTE (16 allocations per id beyond the head); the
    // fixed arm allocates the scratch once per id.
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    struct Counting;
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            System.alloc(l)
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            System.dealloc(p, l)
        }
    }

    #[global_allocator]
    static COUNTING: Counting = Counting;

    const HEX_LUT: &[u8; 16] = b"0123456789abcdef";
    let digest: [u8; 16] = std::array::from_fn(|i| (i * 17 + 3) as u8);

    fn buggy_hex(bytes: &[u8]) -> String {
        let mut s = String::new();
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    fn fixed_hex(bytes: &[u8]) -> String {
        let mut raw = Vec::with_capacity(bytes.len() * 2);
        for b in bytes {
            raw.push(HEX_LUT[(b >> 4) as usize]);
            raw.push(HEX_LUT[(b & 0x0f) as usize]);
        }
        String::from_utf8(raw).expect("LUT output is pure ASCII")
    }

    // Warm the allocator bookkeeping, then measure each form alone.
    let _ = buggy_hex(&digest);
    let _ = fixed_hex(&digest);
    let before_buggy = ALLOCS.load(Ordering::Relaxed);
    let _ = buggy_hex(&digest);
    let buggy_allocs = ALLOCS.load(Ordering::Relaxed) - before_buggy;

    let before_fixed = ALLOCS.load(Ordering::Relaxed);
    let _ = fixed_hex(&digest);
    let fixed_allocs = ALLOCS.load(Ordering::Relaxed) - before_fixed;

    assert!(
        buggy_allocs >= 16,
        "the per-byte form must allocate at least once per byte, got {buggy_allocs}"
    );
    assert!(
        fixed_allocs <= 6,
        "the LUT form must allocate a small constant per id (scratch growth + the return), got {fixed_allocs}"
    );
    assert!(
        buggy_allocs > fixed_allocs * 3,
        "the per-byte form must allocate ≥3× more than the LUT form ({buggy_allocs} vs {fixed_allocs})"
    );
}

#[test]
fn f1_tampered_lut_fails_byte_identity_loudly() {
    fn oracle_hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
    // THE TAMPER: one nibble constant corrupted (a transcription class).
    const BAD_LUT: &[u8; 16] = b"0123456789abcdfg"; // 'g' instead of 'e'
    fn bad_hex(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        let mut raw = Vec::with_capacity(bytes.len() * 2);
        for b in bytes {
            raw.push(BAD_LUT[(b >> 4) as usize]);
            raw.push(BAD_LUT[(b & 0x0f) as usize]);
        }
        // The tampered LUT is still ASCII so this is defined.
        out.push_str(std::str::from_utf8(&raw).unwrap());
        out
    }
    let digest = [0xaeu8; 8]; // hits the tampered nibble
    assert_ne!(
        bad_hex(&digest),
        oracle_hex(&digest),
        "the tampered LUT must fail the byte-identity pin, never pass silently"
    );
}
