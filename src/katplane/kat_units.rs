//! Vendored from `riir-kat` (`863c590798b5`, 2026-11-09, issue 221
//! private-dep minimization): the feature-free protocol tier refine's
//! UNGATED surfaces consume (secret scan, units, consent dir, config
//! store, hosts, net pin, state stamp, soft gate). std-only by the
//! leaf's protocol-tier purity rule. The canonical copy stays in the
//! private sibling (`riir-kat` `src/kat_units.rs`); drift is caught by the
//! golden pins in `tests/katplane_parity.rs` and repaired by
//! re-vendoring via `scripts/vendor_katplane.sh` — never hand-edit one
//! side.

//! The KAT unit system — the network's shared money vocabulary: the
//! lamport-law documentation every money-touching file references, the
//! [`MicroKat`]-typed economics constants (free-tier grant, per-million-token
//! price), the µ → display renderer, and the [`MicroKat`] / [`WholeKat`]
//! newtypes. Feature-free by construction: rendering a KAT amount and
//! knowing the grant/price is NOT client behavior — it is the wire's unit
//! system, the same vocabulary the riir-dapps ledger plane pins server-side.
//! Zero deps, zero policy, no transport (riir-clippy issue 088 — the
//! `riir-kat` spin-out's protocol tier).
//!
//! # The lamport law (KAT≈SOL, µKAT≈lamport)
//!
//! The KAT economy is shaped on Solana's denomination discipline — the same
//! ROLES, deliberately different magnitudes:
//!
//! | Solana | KAT network | role |
//! |---|---|---|
//! | SOL | KAT | base/display unit — **never stored** |
//! | `lamport` | `µKAT` (micro-KAT) | smallest unit — **the only stored form** |
//! | `LAMPORTS_PER_SOL` = 10⁹ | `MICRO_PER_KAT` = 10⁶ | the ONE conversion — role-shaped, not magnitude-shaped |
//! | RPC returns lamports | the wire returns µ | storage/wire discipline |
//!
//! ## The three laws
//!
//! 1. **The name carries the unit.** A `_KAT` name holds WHOLE KAT; a
//!    `_MICRO_KAT` name holds `µKAT`. The pre-Issue-094 spelling
//!    `FREE_TIER_GRANT_KAT` held a µ value under a whole-KAT name, and two
//!    sibling repos duly disagreed about its unit by 10⁶× (riir-dapps
//!    Issue 094). **No back-compat alias**: a deprecated `_KAT` spelling
//!    would re-admit the very ambiguity Issue 094 exists to kill.
//! 2. **The ledger never stores KAT.** Every stored, wired, or committed
//!    money amount is an integer µ count (the dapps `MicroUnits`, the local
//!    charge ledger's `charged_kat`, the WAL's raw `i64` LE). Whole KAT is
//!    a display projection only — rendered at the last moment through
//!    [`render_kat`], never persisted.
//! 3. **`MICRO_PER_KAT` is the only conversion.** No second factor, no
//!    per-call-site 10⁶, no bare `× 1_000_000` literals — µ↔KAT always
//!    goes through (or is justified by) this constant, and code-level
//!    µ↔whole moves go through [`MicroKat`] / [`WholeKat`].
//!
//! The workspace-side registry of every money constant (repo, file, name →
//! declared value + unit, each row verified against the sibling's source
//! text) lives in `tests/unit_registry.rs` (riir-dapps Plan 035 T0.2).
//!
//! ```
//! use riir_refine::kat_units::{MicroKat, WholeKat, MICRO_PER_KAT, render_kat};
//!
//! // the ONE conversion, pinned (LAMPORTS_PER_SOL is 10⁹ — same role,
//! // different magnitude, on purpose)
//! assert_eq!(MICRO_PER_KAT, 1_000_000);
//!
//! // wire/storage quantities are µ; whole KAT is display-only
//! assert_eq!(render_kat(1_500_000), "1.500000 KAT");
//!
//! // the free-tier grant is 100 KAT = 10⁸ µ — stored as µ, displayed as KAT
//! let grant = MicroKat::from(100_000_000_i64);
//! assert_eq!(render_kat(grant.0), "100.000000 KAT");
//! assert_eq!(grant.to_whole(), Some(WholeKat(100)));
//!
//! // the product semantics (Plan 035 T4): grant = price × free tokens
//! assert_eq!(
//!     grant,
//!     riir_refine::kat_units::CHARGE_MICRO_KAT_PER_MILLION_TOKENS
//!         * riir_refine::kat_units::FREE_TIER_TOKENS_MTOK
//! );
//! ```

// Vendored lint posture (issue 221): the canonical copy passes the sibling's
// gate, not this crate's pedantic set; the µ↔whole conversions are pinned
// by the golden tests, so the `as` forms stay VERBATIM (allow, never
// rewrite money math).
#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]

/// Micro-KAT per KAT — the denomination every money amount crossing the
/// service boundary carries (the dapps `MicroUnits` unit), and the ONLY
/// µ↔KAT conversion in the network (lamport law, module docs; the
/// `LAMPORTS_PER_SOL` of this stack — role-shaped, not magnitude-shaped).
///
/// A plain `u64`, deliberately: this is the conversion FACTOR, not a money
/// amount — a `MicroKat` here would let a µ amount and the µ-per-KAT rate
/// be added, which is exactly the confusion the type split exists to end.
/// The single canonical definition (issue-088 follow-up: riir-dao's
/// hand-rolled `units.rs` twin retired against it).
pub const MICRO_PER_KAT: u64 = 1_000_000;

use std::iter::Sum;
use std::ops::{Add, Mul, Sub};

/// A KAT money amount in **micro-KAT (µ)** — the stored/wire unit (the
/// lamport of the network; see the module docs for the lamport law).
///
/// `#[repr(transparent)]` over [`i64`]: zero-cost by construction — no
/// allocation anywhere, `Copy`, and the WAL byte layout is exactly raw
/// `i64` little-endian ([`MicroKat::to_le_bytes`] /
/// [`MicroKat::from_le_bytes`]).
///
/// **The `pub` field is deliberate.** Opacity here is enforced by the type
/// split ([`MicroKat`] µ vs [`WholeKat`] whole) and by the Plan 035 gates
/// (the unit registry in `tests/unit_registry.rs` + the suffix grep gate),
/// NOT by privacy: a private field would block `const` construction, and
/// the protocol constants must stay const-constructible (`pub const
/// FREE_TIER_GRANT_MICRO_KAT: MicroKat = MicroKat(100_000_000);`). The
/// tradeoff: `.0` is an honest escape hatch at a call site that has
/// already argued its unit — the gates catch the lie at the NAME level,
/// where Issue 094 lived.
///
/// Arithmetic mirrors `i64` semantics exactly — `+`/`-`/`*` ARE the
/// primitives (debug builds panic on overflow, release wraps), so ledger
/// call sites keep compiling and keep behaving. Prefer the
/// [`MicroKat::checked_add`] / [`MicroKat::saturating_add`] families where
/// the money rule ("integer math only — never a float, never a silent
/// wrap") deserves to be explicit.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MicroKat(
    /// The raw micro-KAT count. Balances and the economics constants are
    /// positive by construction; ledger deltas may be negative.
    pub i64,
);

impl MicroKat {
    /// The largest representable µ amount (`i64::MAX` µ ≈ 9.2 × 10¹² KAT —
    /// the network cap, 10¹⁸ µ, fits with two decades to spare).
    pub const MAX: MicroKat = MicroKat(i64::MAX);

    /// Checked µ → whole-KAT conversion: `Some` only for a NON-NEGATIVE
    /// amount that is an EXACT whole number of KAT, `None` otherwise.
    ///
    /// Exact-or-`None` is deliberate for money: a silent floor would let a
    /// sub-KAT remainder vanish (`1_500_000` µ is 1.5 KAT, not 1 KAT).
    /// Arbitrary amounts are a DISPLAY concern — render the raw µ
    /// (`.0`) through [`render_kat`]; this conversion is for genuinely
    /// whole-KAT quantities (the [`WholeKat`] contract).
    #[must_use]
    pub fn to_whole(self) -> Option<WholeKat> {
        if self.0 < 0 || self.0 % MICRO_PER_KAT as i64 != 0 {
            return None;
        }
        Some(WholeKat(self.0 as u64 / MICRO_PER_KAT))
    }

    /// Checked add — `None` on overflow (the [`i64::checked_add`]
    /// semantics, µ preserved).
    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(MicroKat)
    }

    /// Checked subtract — `None` on overflow.
    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(MicroKat)
    }

    /// Checked multiply by a scalar — `None` on overflow.
    #[must_use]
    pub fn checked_mul(self, rhs: i64) -> Option<Self> {
        self.0.checked_mul(rhs).map(MicroKat)
    }

    /// Saturating add — pins at [`MicroKat::MAX`] / `i64::MIN`.
    #[must_use]
    pub fn saturating_add(self, rhs: Self) -> Self {
        MicroKat(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtract.
    #[must_use]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        MicroKat(self.0.saturating_sub(rhs.0))
    }

    /// Saturating multiply by a scalar.
    #[must_use]
    pub fn saturating_mul(self, rhs: i64) -> Self {
        MicroKat(self.0.saturating_mul(rhs))
    }

    /// The WAL byte layout: exactly raw `i64` little-endian — a WAL written
    /// before this type existed reads back unchanged, and vice versa.
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    /// Inverse of [`MicroKat::to_le_bytes`] — the same raw-`i64`-LE law.
    #[must_use]
    pub const fn from_le_bytes(bytes: [u8; 8]) -> Self {
        MicroKat(i64::from_le_bytes(bytes))
    }

    /// `self.0 > 0` — the [`i64`] predicate, µ preserved.
    #[must_use]
    pub fn is_positive(self) -> bool {
        self.0.is_positive()
    }

    /// The [`i64::abs`] — debug-panics on `i64::MIN` exactly like the
    /// primitive (the money rule prefers an honest panic over a silent
    /// wrap; use the checked family when in doubt).
    #[must_use]
    pub fn abs(self) -> Self {
        MicroKat(self.0.abs())
    }
}

impl From<i64> for MicroKat {
    fn from(micro: i64) -> Self {
        MicroKat(micro)
    }
}

/// Infallible by the raw-repr law: the µ wire is `i64`, so a `u64` source
/// keeps its low 64 bits (a value past `i64::MAX` re-reads as negative).
/// Real µ amounts never go there — the network cap is 10¹⁸ µ, a decade
/// under the `i64` ceiling — but the conversion does not pretend to check
/// what the caller did not ask it to.
impl From<u64> for MicroKat {
    fn from(micro: u64) -> Self {
        MicroKat(micro as i64)
    }
}

impl Add for MicroKat {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        MicroKat(self.0 + rhs.0)
    }
}

impl Sub for MicroKat {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        MicroKat(self.0 - rhs.0)
    }
}

impl Mul<i64> for MicroKat {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self {
        MicroKat(self.0 * rhs)
    }
}

impl Mul<u64> for MicroKat {
    type Output = Self;

    /// µ × scalar. The scalar keeps its low 64 bits under the `as` (see
    /// `From<u64>`); real rates and counts never go there.
    fn mul(self, rhs: u64) -> Self {
        MicroKat(self.0 * rhs as i64)
    }
}

impl Sum for MicroKat {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(MicroKat(0), |acc, m| acc + m)
    }
}

impl<'a> Sum<&'a MicroKat> for MicroKat {
    fn sum<I: Iterator<Item = &'a MicroKat>>(iter: I) -> Self {
        iter.fold(MicroKat(0), |acc, m| acc + *m)
    }
}

impl serde::Serialize for MicroKat {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        <i64 as serde::Serialize>::serialize(&self.0, serializer)
    }
}

impl<'de> serde::Deserialize<'de> for MicroKat {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(MicroKat(<i64 as serde::Deserialize>::deserialize(
            deserializer,
        )?))
    }
}

/// A KAT money amount in **whole KAT** — the display/unit-of-account form
/// (`_KAT` names). NOT a wire or storage type: by the lamport law the
/// ledger never stores KAT. This exists so a whole-KAT quantity can travel
/// without a bare `u64` that a later call site might mistake for µ — the
/// Issue-094 defect shape, made a type error instead of a review catch.
///
/// Deliberately minimal — defined + conversions only
/// ([`WholeKat::to_micro`], `From<u64>`); nothing adopts it yet and
/// nothing is forced to. No serde on purpose: a whole-KAT value must not
/// silently cross a wire that speaks µ.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WholeKat(
    /// The raw whole-KAT count — display vocabulary, never stored.
    pub u64,
);

impl WholeKat {
    /// The largest representable whole-KAT count.
    pub const MAX: WholeKat = WholeKat(u64::MAX);

    /// Checked whole-KAT → µ conversion: `None` only past the `i64` µ
    /// ceiling (≈ 9.2 × 10¹² whole KAT; the `u64` multiply itself gives out
    /// a little above that, at ≈ 1.8 × 10¹³). Unreachable for real money —
    /// the network cap is 10¹⁸ µ = 10¹² KAT — the check is the type being
    /// honest, not a permit for unbounded counts.
    #[must_use]
    pub fn to_micro(self) -> Option<MicroKat> {
        let micro = self.0.checked_mul(MICRO_PER_KAT)?;
        if micro > i64::MAX as u64 {
            return None;
        }
        Some(MicroKat(micro as i64))
    }
}

impl From<u64> for WholeKat {
    fn from(kat: u64) -> Self {
        WholeKat(kat)
    }
}

/// The once-per-account free-tier grant, in **micro-KAT**:
/// `MicroKat(100_000_000)` = **100 KAT** (`MICRO_PER_KAT` = 10⁶ µ per
/// KAT). At [`CHARGE_MICRO_KAT_PER_MILLION_TOKENS`] = `1_000_000` µ per
/// MILLION code word tokens, that grant funds **100 million code word
/// tokens** (10⁸ µ ÷ 10⁶ µ/Mtok = 100 Mtok) — the proposal's
/// "100M tokens ≈ $300" intent. The pre-Issue-094 header read "100M KAT ≈
/// 100M code word tokens", which conflated the µ amount with whole KAT by
/// 10⁶×; the VALUE was right and the prose was wrong (riir-dapps Issue 094
/// D1).
///
/// Minted server-side at `login`; the local charge view projects the
/// remaining balance against this constant purely for the warn UX — the
/// server ledger is the authority (caveat #9).
///
/// **The unit-name law** (riir-dapps Plan 035 — the lamport law, module
/// docs): this name says `_MICRO_KAT` and means it; a `_KAT` name holds
/// WHOLE KAT. The pre-094 spelling `FREE_TIER_GRANT_KAT` held this exact
/// value under a name that read as whole KAT, and two sibling repos duly
/// disagreed about its unit by 10⁶×. **No back-compat alias**: a
/// deprecated `_KAT` spelling would re-admit the very ambiguity Issue 094
/// exists to kill.
///
/// Positive by construction — a call site may read `.0` (or `.0 as u64`)
/// knowing the conversion is exact.
pub const FREE_TIER_GRANT_MICRO_KAT: MicroKat = MicroKat(100_000_000);

/// MICRO-KAT charged per 1,000,000 code word tokens — the per-unit burn
/// rate. v1: `MicroKat(1_000_000)` = 1 KAT per MILLION tokens (the
/// economics — the 100 KAT free grant funds ~100M tokens of healing, the
/// proposal's "100M tokens ≈ $300" intent). Encoded per-million so a
/// future fractional rate (e.g. `MicroKat(500_000)` = 0.5 KAT/M tokens)
/// needs no type change. `kat_for_tokens` yields MICRO-KAT — the same unit
/// as this grant-side constant and the ledger's `MicroUnits`; the burn wire
/// is WHOLE KAT (converts at the wire boundary). Any change to this
/// constant is a reprice → the meter's `CHARGE_VERSION` bump +
/// `CHARGE_CHANGELOG` entry in the same change — AND a move of the other
/// side of the product identity ([`FREE_TIER_TOKENS_MTOK`], Plan 035 T4).
///
/// Positive by construction — a call site may read `.0` knowing the value
/// is exact.
pub const CHARGE_MICRO_KAT_PER_MILLION_TOKENS: MicroKat = MicroKat(1_000_000);

/// The free tier's token allowance, in **million tokens (Mtok)** — the
/// product-side half of the grant identity.
///
/// 100M tokens; flash parity 2026-09 (GLM-5.3-Flash $0.075/$0.25 per M,
/// `OpenRouter`).
///
/// The L2 product-semantics pin (riir-dapps Plan 035 T4 — the Issue-025
/// failure mode was a re-scale that moved the grant and left the meter
/// price where it was, crediting every account 10⁶× the intent):
///
/// ```
/// use riir_refine::kat_units::{
///     CHARGE_MICRO_KAT_PER_MILLION_TOKENS, FREE_TIER_GRANT_MICRO_KAT,
///     FREE_TIER_TOKENS_MTOK,
/// };
/// assert_eq!(
///     FREE_TIER_GRANT_MICRO_KAT,
///     CHARGE_MICRO_KAT_PER_MILLION_TOKENS * FREE_TIER_TOKENS_MTOK
/// );
/// ```
///
/// A future re-scale must move BOTH sides of that identity in the same
/// commit — the pin reds otherwise. Whole Mtok (`u64`) by design: the
/// price constant already carries the sub-KAT fraction in µ.
pub const FREE_TIER_TOKENS_MTOK: u64 = 100;

/// Render micro-units as a KAT decimal (the invoice/credit lines; exactly
/// the precision the value carries — trailing zeros kept so a pinned
/// `1.500000` reads back the way it was written).
/// The [`render_kat`] precision law over a caller-named unit — the
/// consumer-side renderer for sibling micro-denominated planes (the
/// cegel plane's `CEGEL` — renamed from SEAL, riir-dapps Issue 114;
/// wallet-remake Plan 011 T6.3b). Same six-decimal, keep
/// the trailing zeros.
#[must_use]
pub fn render_units(micro: i64, unit: &str) -> String {
    let sign = if micro < 0 { "-" } else { "" };
    let abs = micro.unsigned_abs();
    format!(
        "{}{}.{:06} {unit}",
        sign,
        abs / MICRO_PER_KAT,
        abs % MICRO_PER_KAT
    )
}

#[must_use]
pub fn render_kat(micro: i64) -> String {
    render_units(micro, "KAT")
}

/// The [`render_kat`] of a `u128` amount — the width simulators and
/// cumulative aggregates carry their totals in (non-negative by
/// construction).
#[must_use]
pub fn render_kat_u128(micro: u128) -> String {
    let per = u128::from(MICRO_PER_KAT);
    format!("{}.{:06} KAT", micro / per, micro % per)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_kat_is_the_six_decimal_formatter() {
        assert_eq!(render_kat(100_000_000_000_000), "100000000.000000 KAT");
        assert_eq!(render_kat(1_500_000), "1.500000 KAT");
        assert_eq!(render_kat(0), "0.000000 KAT");
        assert_eq!(render_kat(-2_500_000), "-2.500000 KAT");
        // trailing zeros are kept — a pinned `1.500000` reads back as written
        assert_eq!(render_kat(1_000_001), "1.000001 KAT");
    }

    #[test]
    fn micro_per_kat_is_a_million_and_the_u128_width_agrees() {
        assert_eq!(MICRO_PER_KAT, 1_000_000);
        assert_eq!(render_kat_u128(1_400_000), "1.400000 KAT");
        assert_eq!(render_kat_u128(0), "0.000000 KAT");
        assert_eq!(render_kat_u128(1), "0.000001 KAT");
        // agreement with the i64 formatter on the shared (non-negative)
        // range — a value quoted in one repo's log reads back identically
        // in the other's.
        assert_eq!(render_kat_u128(1_500_000), render_kat(1_500_000));
    }

    #[test]
    fn grant_is_one_hundred_kat_in_micro_and_price_one_kat_per_million() {
        // 10⁸ µKAT — which is ONE HUNDRED KAT, not 100M (Issue 094 D1).
        assert_eq!(FREE_TIER_GRANT_MICRO_KAT, MicroKat(100_000_000));
        assert_eq!(FREE_TIER_GRANT_MICRO_KAT.0 / MICRO_PER_KAT as i64, 100);
        assert_eq!(render_kat(FREE_TIER_GRANT_MICRO_KAT.0), "100.000000 KAT");
        assert_eq!(CHARGE_MICRO_KAT_PER_MILLION_TOKENS.0, 1_000_000);
        // the deployed economics: the free grant funds exactly the proposal's
        // 100M code word tokens of healing at the v1 price — 100 Mtok
        assert_eq!(
            FREE_TIER_GRANT_MICRO_KAT.0 / CHARGE_MICRO_KAT_PER_MILLION_TOKENS.0,
            FREE_TIER_TOKENS_MTOK as i64
        );
    }

    /// The L2 product-semantics pin (Plan 035 T4): grant = price × mtok, in
    /// TYPES — a re-scale that moves one side without the other reds here
    /// (the Issue-025 failure mode).
    #[test]
    fn l2_product_semantics_grant_equals_price_times_mtok() {
        assert_eq!(
            FREE_TIER_GRANT_MICRO_KAT,
            CHARGE_MICRO_KAT_PER_MILLION_TOKENS * FREE_TIER_TOKENS_MTOK
        );
        assert_eq!(FREE_TIER_TOKENS_MTOK, 100);
    }

    #[test]
    fn micro_kat_wal_bytes_are_raw_i64_le() {
        for raw in [0_i64, 1, -1, 100_000_000, i64::MAX, i64::MIN] {
            let m = MicroKat::from(raw);
            assert_eq!(m.to_le_bytes(), raw.to_le_bytes());
            assert_eq!(MicroKat::from_le_bytes(raw.to_le_bytes()), m);
        }
    }

    #[test]
    fn micro_kat_serde_is_the_raw_micro_integer() {
        let m = MicroKat::from(-2_500_000_i64);
        assert_eq!(serde_json::to_string(&m).unwrap(), "-2500000");
        let back: MicroKat = serde_json::from_str("-2500000").unwrap();
        assert_eq!(back, m);
        let zero = MicroKat::default();
        assert_eq!(serde_json::to_string(&zero).unwrap(), "0");
    }

    #[test]
    fn conversions_are_checked_and_exact() {
        assert_eq!(
            MicroKat::from(100_000_000_i64).to_whole(),
            Some(WholeKat(100))
        );
        assert_eq!(
            MicroKat::from(1_500_000_i64).to_whole(),
            None,
            "exact-only: 1.5 KAT is not a whole-KAT quantity"
        );
        assert_eq!(
            MicroKat::from(-(MICRO_PER_KAT as i64)).to_whole(),
            None,
            "WholeKat is unsigned; negatives are a render concern"
        );
        assert_eq!(
            WholeKat(100).to_micro(),
            Some(MicroKat::from(100_000_000_i64))
        );
        assert_eq!(
            WholeKat(u64::MAX).to_micro(),
            None,
            "×10⁶ overflows the i64 µ ceiling"
        );
        // round trip at the type split
        let w = WholeKat::from(7_u64);
        assert_eq!(w.to_micro().and_then(MicroKat::to_whole), Some(w));
        assert_eq!(MicroKat::from(0_i64).to_whole(), Some(WholeKat(0)));
    }

    #[test]
    fn ops_mirror_i64_semantics() {
        let a = MicroKat::from(1_500_000_i64);
        let b = MicroKat::from(500_000_i64);
        assert_eq!(a + b, MicroKat::from(2_000_000_i64));
        assert_eq!(a - b, MicroKat::from(1_000_000_i64));
        assert_eq!(a * 2_i64, MicroKat::from(3_000_000_i64));
        assert_eq!(a * 2_u64, MicroKat::from(3_000_000_i64));
        assert_eq!(a.checked_add(b), Some(MicroKat::from(2_000_000_i64)));
        assert_eq!(MicroKat::MAX.checked_add(MicroKat::from(1_i64)), None);
        assert_eq!(
            MicroKat::from(i64::MIN).checked_sub(MicroKat::from(1_i64)),
            None
        );
        assert_eq!(MicroKat::MAX.checked_mul(2_i64), None);
        assert_eq!(
            MicroKat::MAX.saturating_add(MicroKat::from(1_i64)),
            MicroKat::MAX
        );
        assert_eq!(
            MicroKat::from(i64::MIN).saturating_sub(MicroKat::from(1_i64)),
            MicroKat::from(i64::MIN)
        );
        assert_eq!(MicroKat::MAX.saturating_mul(2_i64), MicroKat::MAX);
        assert!(a.is_positive());
        assert!(!MicroKat::from(-1_i64).is_positive());
        assert!(!MicroKat::from(0_i64).is_positive());
        assert_eq!(MicroKat::from(-1_500_000_i64).abs(), a);
        // Sum, borrowed and owned
        let borrowed: MicroKat = [a, b, MicroKat::from(1_i64)].iter().sum();
        assert_eq!(borrowed, MicroKat::from(2_000_001_i64));
        let owned: MicroKat = vec![a, b].into_iter().sum();
        assert_eq!(owned, MicroKat::from(2_000_000_i64));
        assert_eq!(MicroKat::default(), MicroKat(0));
        assert_eq!(MicroKat::MAX, MicroKat(i64::MAX));
        // ordering
        assert!(MicroKat::from(2_i64) > MicroKat::from(1_i64));
        assert!(MicroKat::from(-1_i64) < MicroKat::default());
        // From impls
        assert_eq!(MicroKat::from(7_i64), MicroKat(7));
        assert_eq!(MicroKat::from(7_u64), MicroKat(7));
        assert_eq!(WholeKat::from(7_u64), WholeKat(7));
    }
}
