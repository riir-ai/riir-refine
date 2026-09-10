//! The KAT unit system — the network's shared money vocabulary: the
//! micro-unit → display renderer and the two protocol constants (free-tier
//! grant, per-million-token price) every side of the network must agree on.
//!
//! Feature-free by construction: rendering a KAT amount and knowing the
//! grant/price is NOT client behavior — it is the wire's unit system, the
//! same vocabulary the riir-dapps ledger plane pins server-side. Zero deps,
//! zero policy, no transport (riir-clippy issue 088 — the `riir-kat`
//! spin-out's protocol tier).

/// The once-per-account free-tier grant (§KAT): 100M KAT ≈ 100M code word
/// tokens. Minted server-side at `login` (T0.1+); the local charge view
/// projects the remaining balance against this constant purely for the warn
/// UX — the server ledger is the authority (caveat #9).
pub const FREE_TIER_GRANT_KAT: u64 = 100_000_000;

/// KAT charged per 1,000,000 code word tokens — the per-unit burn rate.
/// v1: `1_000_000` = 1 KAT per MILLION tokens (the deployed economics —
/// the 100 KAT free grant funds ~100M tokens of healing, the proposal's
/// "100M tokens ≈ $300" intent). Encoded per-million so a future
/// fractional rate (e.g. `500_000` = 0.5 KAT/M tokens) needs no type
/// change. `kat_for_tokens` yields MICRO-KAT — the same unit as this
/// grant-side constant and the ledger's `MicroUnits`; the burn wire is
/// WHOLE KAT (`wire_kat_total` converts). Any change to this constant is a
/// reprice → the meter's `CHARGE_VERSION` bump + `CHARGE_CHANGELOG` entry
/// in the same change.
pub const CHARGE_KAT_PER_MILLION_TOKENS: u64 = 1_000_000;

/// Render micro-units as a KAT decimal (the invoice/credit lines; exactly
/// the precision the value carries — trailing zeros kept so a pinned
/// `1.500000` reads back the way it was written).
#[must_use]
pub fn render_kat(micro: i64) -> String {
    let sign = if micro < 0 { "-" } else { "" };
    let abs = micro.unsigned_abs();
    format!("{}{}.{:06} KAT", sign, abs / 1_000_000, abs % 1_000_000)
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
    fn grant_is_one_hundred_million_kat_and_price_one_kat_per_million() {
        assert_eq!(FREE_TIER_GRANT_KAT, 100_000_000);
        assert_eq!(CHARGE_KAT_PER_MILLION_TOKENS, 1_000_000);
        // the deployed economics: the free grant funds exactly the proposal's
        // 100M tokens of healing at the v1 price
        assert_eq!(FREE_TIER_GRANT_KAT / CHARGE_KAT_PER_MILLION_TOKENS, 100);
    }
}
