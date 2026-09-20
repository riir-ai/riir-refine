//! Issue 127 (riir-clippy) — Kani oracle POC: machine-checked property evidence
//! for sec rules. Standalone fixture crate; runs ONLY under `cargo kani` (Kani
//! 0.68.0 / CBMC 6.11.0, pinned nightly toolchain), never under the workspace's
//! stable rustc — see README.md in this directory.
//!
//! Two rule targets:
//! - **G1 — the shipped sec rule `nan-unsafe-sort-comparator`** (PanicSafety
//!   family): the buggy form `partial_cmp(..).unwrap_or(Equal)` VIOLATES
//!   comparator transitivity-of-Equal (a sort comparator must induce a total
//!   order); `f64::total_cmp` satisfies it, plus antisymmetry/totality and
//!   NaN self-equality.
//! - **G2 — the Research-193 staged lead `unchecked-wire-arithmetic`**
//!   (InputValidation): `+` directly on `u32::from_le_bytes` results REACHES
//!   overflow on some wire input (existence witness); the `checked_add` form
//!   proves silent wrapping impossible (Some ⟺ exact mathematical sum,
//!   None ⟺ genuine overflow).
//! - **G2c/G2d — the LANDED rule's corpus-fixture shape** (Batch 165, Plan
//!   156 T6): one `from_be_bytes` parse + a CONSTANT header-size add — the
//!   shipped `framed_payload_len` fixture — in its u32-domain twin. (The
//!   fixture's `as usize` widening absorbs the overflow on 64-bit hosts;
//!   the property lives in the wire-width domain the rule's advice
//!   targets — checked_add BEFORE any widening.)

use std::cmp::Ordering;

/// The buggy comparator shape the sec rule flags: NaN in either operand
/// silently degrades to `Equal` instead of participating in a total order.
fn buggy_cmp(a: &f64, b: &f64) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Equal)
}

/// G2 buggy form: wire-decoded u32s summed with bare `+` — wraps silently.
fn buggy_wire_sum(a: [u8; 4], b: [u8; 4]) -> u32 {
    u32::from_le_bytes(a) + u32::from_le_bytes(b)
}

/// G2 checked form: the fix the lead's bounded fix-space proposes — checked
/// op + an error path the caller must handle (returned here as `Option`).
fn checked_wire_sum(a: [u8; 4], b: [u8; 4]) -> Option<u32> {
    u32::from_le_bytes(a).checked_add(u32::from_le_bytes(b))
}

/// The landed rule's corpus-fixture constant (`FRAME_OVERHEAD`).
const FRAME_OVERHEAD: u32 = 8;

/// G2c buggy form — the SHIPPED fixture's shape in its wire-width domain:
/// one `from_be_bytes` parse + a bare constant add.
fn buggy_framed_len(header: [u8; 4]) -> u32 {
    u32::from_be_bytes(header) + FRAME_OVERHEAD
}

/// G2d checked form — the rule's bounded repair for that exact shape.
fn checked_framed_len(header: [u8; 4]) -> Option<u32> {
    u32::from_be_bytes(header).checked_add(FRAME_OVERHEAD)
}

#[cfg(kani)]
mod proofs {
    use super::*;

    // ---------------- G1: nan-unsafe-sort-comparator ----------------

    /// G1a — EXPECTED VERIFICATION FAILURE (that failure IS the evidence):
    /// Kani must exhibit a violating triple (x, y, z) with
    /// cmp(x,y)==Equal ∧ cmp(y,z)==Equal ∧ cmp(x,z)!=Equal.
    /// Witness shape: any non-NaN x < z with y = NaN (e.g. 1.0, NaN, 2.0).
    /// A sort comparator violating this makes `sort_by`'s total-order
    /// precondition false — mis-sorts / UB-adjacent behavior per std docs.
    #[kani::proof]
    fn g1a_buggy_cmp_violates_transitivity_of_equal() {
        let x: f64 = kani::any();
        let y: f64 = kani::any();
        let z: f64 = kani::any();
        if buggy_cmp(&x, &y) == Ordering::Equal && buggy_cmp(&y, &z) == Ordering::Equal {
            assert!(
                buggy_cmp(&x, &z) == Ordering::Equal,
                "comparator not total: Equal∘Equal must imply Equal"
            );
        }
    }

    /// G1b — EXPECTED VERIFICATION SUCCESS: `total_cmp` induces a total order;
    /// the Equal relation it induces is transitive for ALL f64 bit patterns
    /// (every NaN payload, ±0.0, infinities included).
    #[kani::proof]
    fn g1b_total_cmp_equal_is_transitive() {
        let x: f64 = kani::any();
        let y: f64 = kani::any();
        let z: f64 = kani::any();
        if x.total_cmp(&y) == Ordering::Equal && y.total_cmp(&z) == Ordering::Equal {
            assert!(x.total_cmp(&z) == Ordering::Equal);
        }
    }

    /// G1c — EXPECTED VERIFICATION SUCCESS: `total_cmp` is total (returns some
    /// Ordering), antisymmetric (cmp(x,y) == reverse(cmp(y,x))), and
    /// NaN-deterministic in the self-comparison sense (every NaN payload
    /// compares Equal to itself — `partial_cmp` returns None there).
    #[kani::proof]
    fn g1c_total_cmp_is_total_antisymmetric_nan_self_equal() {
        let x: f64 = kani::any();
        let y: f64 = kani::any();
        assert!(matches!(
            x.total_cmp(&y),
            Ordering::Less | Ordering::Equal | Ordering::Greater
        ));
        assert!(x.total_cmp(&y) == y.total_cmp(&x).reverse());
        if x.is_nan() {
            assert!(x.total_cmp(&x) == Ordering::Equal);
        }
    }

    // ---------------- G2: unchecked-wire-arithmetic ----------------

    /// G2a — EXPECTED VERIFICATION FAILURE (existence witness): some wire
    /// input (a, b) makes the bare `+` overflow. The assert fires on the
    /// wrapped value; Kani's default arithmetic-overflow check may also fire
    /// inside `buggy_wire_sum` first — either failure mode is the witness.
    #[kani::proof]
    fn g2a_buggy_wire_sum_reaches_overflow() {
        let a: [u8; 4] = kani::any();
        let b: [u8; 4] = kani::any();
        let expected =
            u64::from(u32::from_le_bytes(a)) + u64::from(u32::from_le_bytes(b)); // u64 sum of two u32s cannot overflow
        let got = u64::from(buggy_wire_sum(a, b));
        assert!(got == expected, "wire arithmetic wrapped silently");
    }

    /// G2b — EXPECTED VERIFICATION SUCCESS (impossibility): the checked form
    /// never wraps silently. Two-sided: `Some(v)` implies v is the exact
    /// mathematical sum, and `None` occurs exactly when the true sum exceeds
    /// u32::MAX (the signal is complete — no false alarm, no silent wrap).
    #[kani::proof]
    fn g2b_checked_wire_sum_never_silently_wraps() {
        let a: [u8; 4] = kani::any();
        let b: [u8; 4] = kani::any();
        let expected =
            u64::from(u32::from_le_bytes(a)) + u64::from(u32::from_le_bytes(b));
        match checked_wire_sum(a, b) {
            Some(v) => assert!(u64::from(v) == expected),
            None => assert!(expected > u64::from(u32::MAX)),
        }
    }

    // ---------- G2c/G2d: the LANDED rule's fixture shape (Plan 156 T6) ----------

    /// G2c — EXPECTED VERIFICATION FAILURE (existence witness): some header
    /// makes the shipped fixture's bare `parse + FRAME_OVERHEAD` overflow
    /// (witness class: any header above `u32::MAX - 8`, e.g. all-0xFF).
    /// Kani's default arithmetic-overflow check fires inside
    /// `buggy_framed_len`; that failure IS the property violation evidence.
    #[kani::proof]
    fn g2c_buggy_framed_len_reaches_overflow() {
        let header: [u8; 4] = kani::any();
        let expected = u64::from(u32::from_be_bytes(header)) + u64::from(FRAME_OVERHEAD); // u64 cannot overflow
        let got = u64::from(buggy_framed_len(header));
        assert!(got == expected, "framed length wrapped silently");
    }

    /// G2d — EXPECTED VERIFICATION SUCCESS (impossibility): the checked
    /// repair for the shipped fixture shape is two-sided — `Some(v)` iff v
    /// is the exact mathematical length, `None` iff the true length exceeds
    /// `u32::MAX` (no false alarm, no silent wrap).
    #[kani::proof]
    fn g2d_checked_framed_len_never_silently_wraps() {
        let header: [u8; 4] = kani::any();
        let expected = u64::from(u32::from_be_bytes(header)) + u64::from(FRAME_OVERHEAD);
        match checked_framed_len(header) {
            Some(v) => assert!(u64::from(v) == expected),
            None => assert!(expected > u64::from(u32::MAX)),
        }
    }
}
