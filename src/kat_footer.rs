//! The KAT run footer — the human readout of a heal run's billing.
//!
//! Pure presentation over primitives: amounts arrive as [`MicroKat`] µ
//! counts (the riir-kat `kat_units` vocabulary) plus raw token counts,
//! rendered through `render_kat`'s 6-decimal protocol shape — the form
//! riir-dao/riir-dapps pin cross-repo. The µ/whole-KAT unit split is now a
//! compile fact (riir-dapps Plan 035 T3): a whole-KAT count passed where µ
//! is expected does not typecheck. This module never touches the wire; it
//! renders the same facts for the terminal, where trailing `.000000`s and a
//! twice-stated disclaimer were wall-of-text noise (the 2026-09-13
//! redesign discussion).
//!
//! Shape (spec v2 + the dim-track Braille amendments):
//!
//! ```text
//! kat  this run  8,791 tok → 0.0088 KAT   (1 KAT / 1M tokens · meter v1 · charge v1)
//!      grant     ⣿…⣄⣿⣿⣿  84.92 KAT left · 15.08 of 100 burned
//!      tuna      ⣿…⣄⣿⣿⣿  47.20 of 100 TUNA left · expires 2026-10-18 (12d) · trial credit — heals only
//!      sync      ✓ current — ledger matches · mint settles hourly · leaderboard ↗ (you: c79c…5345)
//! ```
//!
//! (The `tuna` line — the Proposal 006 trial credit — renders ONLY when
//! the burn ack carries a server-reported TUNA balance; `None` (a pre-v4
//! ack / pre-TUNA server) renders nothing and the footer stays
//! byte-identical to the 3-line shape.)
//!
//! - The bar is a Braille fuel gauge: filled cells are the REMAINING grant
//!   (bright, threshold-colored), the burned-through tail is dim full
//!   blocks, and the partial cell fills bottom-up (`⠐ ⣀ ⣄ ⣤ ⣦ ⣶ ⣷ ⣿`)
//!   so the bottom edge stays continuous — a liquid level, appropriate for
//!   a tank. 24 cells × 8 dots = 192 steps. Plain (piped/`NO_COLOR`) builds
//!   fall back to the dotted `⣀` track.
//! - Numbers round for reading and round AS A PAIR: `left = grant −
//!   burned` is computed after rounding, so the caption always sums.
//! - The sync line is state-aware. The old static "accounting only,
//!   reconciles at sync" disclaimer was true in neither direction: it
//!   printed "local only" even when the tail push had just been accepted,
//!   and said nothing when the delta was actually stuck.
//! - `Unknown` (a `kat_service`-only dev build with no transport) renders
//!   the classic honest one-liner.
//!
//! Every figure the ledger needs stays canonical: exact micro-KAT lives in
//! the charge ledger + burn ack and surfaces via `render_kat` in `sync` /
//! `account` output. This footer is a readout, which is what its sync line
//! says.

/// Bar width in cells. 24 × 8 dots = 192 steps ≈ 0.52% per dot.
pub const BAR_WIDTH: usize = 24;

use std::io::IsTerminal as _;

// The client-side unit vocabulary: the µ newtype every footer field
// carries, the ONE µ↔KAT conversion (re-exported so the token-million
// sites can name BOTH millions beside each other), and the token million
// defined below (riir-dapps Plan 035 T3 + T10).
pub use riir_kat::kat_units::{MicroKat, MICRO_PER_KAT};

/// Tokens per **million tokens (Mtok)** — the pricing axis's million, NOT
/// the money one. Per-Mtok $ rates and µ-per-Mtok KAT billing
/// (`CHARGE_MICRO_KAT_PER_MILLION_TOKENS`) divide TOKEN counts by this;
/// a µ↔KAT conversion uses [`MICRO_PER_KAT`]. Same VALUE (10⁶), different
/// DIMENSION — that coincidence is exactly the unit-confusion class the
/// µ/KAT gates exist to make findable (riir-dapps Plan 035 T10, the
/// `micro_literal_gate`).
pub const TOKENS_PER_MTOK: u64 = 1_000_000;

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
/// The bar's burned-through track: dim + 256-color gray 240 — the bare
/// `2m` attribute renders too close to the fill on some terminals, and
/// `38;5;240` alone ignores themes; together they dim clearly everywhere.
const TRACK_DIM: &str = "\x1b[2m\x1b[38;5;240m";
const GREEN: &str = "\x1b[32m";
const AMBER: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

/// The `leaderboard ↗` page for the account (OSC-8 hyperlink when
/// colored). The mint page is per-RECEIPT (`/mint/<32-hex receipt id>` —
/// keyed by the receipt's own `[u8; 16]` id, not the account), so the
/// account-scoped human surface is the leaderboard, where the row key is
/// the pseudonym (see [`leaderboard_pseudonym`]).
pub enum SyncState<'a> {
    /// The tail push was accepted (or there was nothing new to push): the
    /// on-disk ack watermark covers the local cumulative burn.
    Current {
        /// The leaderboard page.
        ledger_url: &'a str,
    },
    /// The push failed (or never happened for this delta): whole-KAT
    /// gaps beyond the sub-KAT wire sliver are stacked, git-style. The
    /// amount is the pushable floor minus the ack watermark (micro-KAT,
    /// whole-KAT multiples only — the wire truncates, so the sliver is
    /// never a stuck delta).
    Unsynced {
        /// The stacked delta, micro-KAT.
        micro_kat: MicroKat,
        /// Mining batches queued in the outbox (0 = none).
        queued_batches: u32,
        /// The leaderboard page.
        ledger_url: &'a str,
    },
    /// No account on this machine — nothing ever leaves it.
    LoggedOut,
    /// No transport in this build (a `kat_service`-only dev build): the
    /// push cannot run here, so no state claim is made.
    Unknown,
}

/// One trial-credit (TUNA) gauge line — Proposal 006: the footer's second
/// tank, rendered between the grant bar and the sync line. Present ONLY
/// when the burn ack carries a server-reported TUNA balance (ack v4);
/// its absence renders nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TunaLine {
    /// The trial-credit balance, µ (negative clamps to 0 at render —
    /// `tuna:acct:` is burn-clamped server-side; the clamp is display
    /// armor, not accounting).
    pub balance_micro: MicroKat,
    /// The claim scale the bar gauges against — the caller's projection
    /// (B2's real claim size re-aims it when the server reports one).
    pub grant_micro: MicroKat,
    /// Stored-at-issue expiry (never recomputed — the D4a rule), unix
    /// seconds.
    pub expires_unix: u64,
    /// Now, captured at construction so `render` stays pure.
    pub now_unix: u64,
}

/// The footer's inputs: money as [`MicroKat`] µ, tokens raw.
pub struct KatFooter<'a> {
    /// Code word tokens billed this run.
    pub run_tokens: u64,
    /// KAT burned this run, µ.
    pub run_micro_kat: MicroKat,
    /// Cumulative burn, µ (the charge ledger's `charged_kat`).
    pub cum_micro_kat: MicroKat,
    /// The grant projection, µ.
    pub grant_micro_kat: MicroKat,
    /// The trial-credit line (Proposal 006). `None` renders nothing — the
    /// pre-TUNA footer, byte for byte.
    pub tuna: Option<TunaLine>,
    /// The v-rate, µ per million tokens.
    pub rate_micro_per_mtok: MicroKat,
    /// The meter version stamp (`MeterState::version`).
    pub meter_version: u32,
    /// The charge version stamp (`ChargeState::charge_version`).
    pub charge_version: u32,
    /// The client account id (64-hex, BLAKE3(pubkey)) — the leaderboard
    /// row hint derives from its ledger-width prefix. `None` when logged
    /// out (no hint renders).
    pub account_hex: Option<&'a str>,
    /// The sync line's state.
    pub sync: SyncState<'a>,
}

impl KatFooter<'_> {
    /// Render the footer (3 lines; a [`TunaLine`] adds one, the
    /// `Unsynced` sync state adds one). No trailing newline. `color` gates
    /// all ANSI escapes; plain output is byte-stable for pipes and
    /// `NO_COLOR`.
    #[must_use]
    pub fn render(&self, color: bool) -> String {
        let grant_disp_micro = round_kat_2dp(self.grant_micro_kat);
        let burned_micro = round_kat_2dp(self.cum_micro_kat);
        let left_micro = grant_disp_micro.saturating_sub(burned_micro);
        // Grant-scale micro values (≤ 10^14) sit far inside f64's
        // exact-integer range — this division is lossless at every real
        // ledger size.
        #[allow(clippy::cast_precision_loss)]
        let frac = if self.grant_micro_kat.0 == 0 {
            0.0
        } else {
            left_micro.0 as f64 / self.grant_micro_kat.0 as f64
        };

        let line1 = format!(
            "kat  this run  {} tok → {} KAT{}",
            fmt_tokens(self.run_tokens),
            fmt_kat_display(self.run_micro_kat),
            paint_dim(
                &format!(
                    "   ({} KAT / 1M tokens · meter v{} · charge v{})",
                    fmt_kat_display(self.rate_micro_per_mtok),
                    self.meter_version,
                    self.charge_version
                ),
                color
            )
        );
        let line2 = format!(
            "     grant     {}  {} KAT left · {} of {} burned",
            braille_bar(frac, BAR_WIDTH, color),
            fmt_kat_display(left_micro),
            fmt_kat_display(burned_micro),
            fmt_kat_display(grant_disp_micro),
        );
        match self.render_tuna(color) {
            Some(tuna) => format!("{line1}\n{line2}\n{tuna}\n{}", self.render_sync(color)),
            None => format!("{line1}\n{line2}\n{}", self.render_sync(color)),
        }
    }

    /// The trial-credit line — `None` renders nothing (the ack's absence
    /// of TUNA truth is the 3-line footer, byte for byte).
    fn render_tuna(&self, color: bool) -> Option<String> {
        let t = self.tuna?;
        let bal = MicroKat(t.balance_micro.0.max(0));
        let bal_disp = round_kat_2dp(bal);
        let grant_disp = round_kat_2dp(t.grant_micro);
        // Grant-scale micro values sit far inside f64's exact-integer
        // range — lossless at every real ledger size.
        #[allow(clippy::cast_precision_loss)]
        let frac = if t.grant_micro.0 == 0 {
            0.0
        } else {
            bal_disp.0 as f64 / t.grant_micro.0 as f64
        };
        let (y, m, d) = ymd_from_unix(t.expires_unix);
        let expiry = if t.expires_unix <= t.now_unix {
            format!("expired {y:04}-{m:02}-{d:02} — the sweep reclaims the rest")
        } else {
            let days = (t.expires_unix - t.now_unix) / 86_400;
            format!("expires {y:04}-{m:02}-{d:02} ({days}d)")
        };
        Some(format!(
            "     tuna      {}  {} of {} TUNA left · {expiry} · trial credit — heals only",
            braille_bar(frac, BAR_WIDTH, color),
            fmt_kat_display(bal_disp),
            fmt_kat_display(grant_disp),
        ))
    }

    fn render_sync(&self, color: bool) -> String {
        let gutter = "     sync      ";
        let you = self
            .account_hex
            .and_then(leaderboard_pseudonym)
            .map(|p| format!(" (you: {p})"))
            .unwrap_or_default();
        match &self.sync {
            SyncState::Current { ledger_url } => format!(
                "{gutter}✓ current — ledger matches · mint settles hourly · {}",
                link("leaderboard ↗", ledger_url, color) + &you
            ),
            SyncState::Unsynced {
                micro_kat,
                queued_batches,
                ledger_url,
            } => {
                let queued = if *queued_batches == 0 {
                    String::new()
                } else {
                    format!(" · {queued_batches} mining batch(es) queued")
                };
                format!(
                    "{gutter}⚠ {} KAT unsynced (burn){queued}\n               pushing again next \
                     run · or `cargo heal sync` · {}",
                    fmt_kat_display(*micro_kat),
                    link("leaderboard ↗", ledger_url, color) + &you
                )
            }
            SyncState::LoggedOut => format!(
                "{gutter}local only — not logged in · nothing pushed (`cargo heal login`)"
            ),
            SyncState::Unknown => format!(
                "{gutter}local accounting only — the server ledger reconciles at sync"
            ),
        }
    }
}

/// Civil date from unix seconds — Howard Hinnant's `civil_from_days`
/// (the well-known 20-line algorithm; no chrono dep for one label). The
/// shared date helper for every TUNA expiry readout (footer + `--info`).
#[must_use]
pub fn ymd_from_unix(unix: u64) -> (i64, u32, u32) {
    // u64 days → i64: a wrap needs unix ≈ 10^20 seconds; try_from is the
    // honest form (saturating at i64::MAX, which no date reaches).
    let days = i64::try_from(unix / 86_400).unwrap_or(i64::MAX);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, u32::try_from(m).unwrap_or(1), u32::try_from(d).unwrap_or(1))
}

/// Whether the footer should colorize: `RIIR_HEAL_FOOTER_COLOR` forces on
/// (`1`/`true`) or off (`0`/`false`); else `NO_COLOR` (set + non-empty)
/// disables; else stdout being a TTY decides.
#[must_use]
pub fn footer_color_enabled() -> bool {
    if let Ok(v) = std::env::var("RIIR_HEAL_FOOTER_COLOR") {
        if v == "1" || v.eq_ignore_ascii_case("true") {
            return true;
        }
        if v == "0" || v.eq_ignore_ascii_case("false") {
            return false;
        }
    }
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return false;
    }
    std::io::stdout().is_terminal()
}

/// Thousands-separated token count (`15080719` → `15,080,719`).
#[must_use]
pub fn fmt_tokens(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().copied().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(char::from(b));
    }
    out
}

/// Reading-friendly KAT amount, never the 6-decimal protocol shape: ≥ 1
/// KAT renders at 2 decimals, below 1 at 4, trailing zeros trimmed
/// (`MicroKat(100_000_000)` → `100`, `15_080_719` µ → `15.08`, `8_791` µ
/// → `0.0088`). Takes [`MicroKat`] — a whole-KAT count passed by mistake
/// is a compile error, not a 10⁶×-wrong display (riir-dapps Plan 035 T3).
#[must_use]
pub fn fmt_kat_display(micro: MicroKat) -> String {
    // Display math on bounded values: micro-KAT ≤ ~10^14 sits inside f64's
    // exact-integer range — the cast is lossless at every real ledger size.
    // MICRO_PER_KAT (the ONE µ↔KAT conversion — never a bare 1e6); the
    // cast is exact at 10⁶ (u64→f64 has no From impl — lossy in general).
    #[allow(clippy::cast_precision_loss)]
    let kat = micro.0 as f64 / MICRO_PER_KAT as f64;
    let s = if kat >= 1.0 {
        format!("{kat:.2}")
    } else {
        format!("{kat:.4}")
    };
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Round micro-KAT to 2 KAT decimals (the caption's display grid), so the
/// paired `grant − burned = left` identity survives rounding.
#[must_use]
fn round_kat_2dp(micro: MicroKat) -> MicroKat {
    let q = micro.0 / 10_000;
    let r = micro.0 % 10_000;
    MicroKat((q + i64::from(r >= 5_000)).saturating_mul(10_000))
}

/// Bottom-up partial-cell ladder, 1–7 dots (8 is the full block): fills the
/// bottom row first, then upward — `⠐ ⣀ ⣄ ⣤ ⣦ ⣶ ⣷`, then `⣿`.
const LADDER_MASKS: [u8; 7] = [0x40, 0xC0, 0xC4, 0xE4, 0xE6, 0xF6, 0xF7];

fn ladder_char(dots: usize) -> char {
    let mask = LADDER_MASKS[dots.clamp(1, 7) - 1];
    char::from_u32(0x2800 + u32::from(mask)).unwrap_or('\u{28FF}')
}

/// The Braille fuel gauge. Colored: the remaining fraction renders as
/// bright `⣿` cells (threshold-colored: green ≥ 25%, amber ≥ 10%, red
/// below) + one bottom-up partial cell, then dim full blocks to the end —
/// one continuous solid slab. Plain: the same fill over the dotted `⣀`
/// track (the pre-dim fallback).
#[must_use]
pub fn braille_bar(remaining_frac: f64, width: usize, color: bool) -> String {
    let width = width.max(1);
    let frac = remaining_frac.clamp(0.0, 1.0);
    let total = width.saturating_mul(8);
    // Display math on bounded values: frac ∈ [0,1] × total dots rounds
    // inside usize; total ≤ width×8 « 2^52. The casts are intentional.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let filled = (frac * total as f64).round() as usize;
    let n_colored = filled.div_ceil(8).min(width);
    let tone = if frac >= 0.25 {
        GREEN
    } else if frac >= 0.10 {
        AMBER
    } else {
        RED
    };

    let mut filled_cells = String::with_capacity(n_colored * 3);
    for cell in 0..n_colored {
        let dots = filled.saturating_sub(cell * 8).min(8);
        filled_cells.push(if dots >= 8 {
            '\u{28FF}'
        } else {
            ladder_char(dots)
        });
    }
    let n_empty = width - n_colored;
    if color {
        let mut out = String::with_capacity(width * 4 + 2 * RESET.len() + 8);
        if !filled_cells.is_empty() {
            out.push_str(tone);
            out.push_str(&filled_cells);
            out.push_str(RESET);
        }
        if n_empty > 0 {
            out.push_str(TRACK_DIM);
            for _ in 0..n_empty {
                out.push('\u{28FF}');
            }
            out.push_str(RESET);
        }
        out
    } else {
        filled_cells.push_str(&"\u{28C0}".repeat(n_empty));
        filled_cells
    }
}

/// The leaderboard row key for an account — the exact server projection
/// (dapps `routes::pseudonym`): first 4 + last 4 hex of the LEDGER-WIDTH
/// id with an ellipsis between (`c79c…5345`). The client's account id is
/// the full 64-hex BLAKE3(pubkey); the ledger id is its first 32 hex
/// (`account_for_pubkey` = `blake3(pubkey)[..16]` — "the client's own
/// display id truncated to the ledger width"). `None` when the id is
/// shorter than the ledger width (never hint a wrong row).
#[must_use]
pub fn leaderboard_pseudonym(account_hex: &str) -> Option<String> {
    let h = account_hex.get(..32)?; // ledger width; None under it — never hint a wrong row
    Some(format!("{}…{}", &h[..4], &h[h.len() - 4..]))
}

/// OSC-8 hyperlink when colored (label `leaderboard ↗`); plain dim `label
/// url`. An empty URL renders nothing (the logged-out state has no page).
fn link(label: &str, url: &str, color: bool) -> String {
    if url.is_empty() {
        return String::new();
    }
    if color {
        format!("\x1b]8;;{url}\x07{label}\x1b]8;;\x07")
    } else {
        paint_dim(&format!("{label} {url}"), false)
    }
}

fn paint_dim(s: &str, color: bool) -> String {
    if color {
        format!("{DIM}{s}{RESET}")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOARD: &str = "https://devnet.ai.gist.rs/leaderboard";
    const ACCT: &str = "c79c93b8615af9cc25968be2d0595345d1f03790910a2605d853f93af6192fd6";

    fn footer_left(left_micro: MicroKat) -> KatFooter<'static> {
        KatFooter {
            run_tokens: 8_791,
            run_micro_kat: MicroKat(8_791),
            cum_micro_kat: MicroKat(100_000_000 - left_micro.0),
            grant_micro_kat: MicroKat(100_000_000),
            tuna: None,
            rate_micro_per_mtok: MicroKat(1_000_000),
            meter_version: 1,
            charge_version: 1,
            account_hex: Some(ACCT),
            sync: SyncState::Current { ledger_url: BOARD },
        }
    }

    #[test]
    fn kat_display_trims_the_protocol_zeros() {
        assert_eq!(fmt_kat_display(MicroKat(100_000_000)), "100");
        assert_eq!(fmt_kat_display(MicroKat(15_080_719)), "15.08");
        assert_eq!(fmt_kat_display(MicroKat(84_919_281)), "84.92");
        assert_eq!(fmt_kat_display(MicroKat(8_791)), "0.0088");
        assert_eq!(fmt_kat_display(MicroKat(500_000)), "0.5");
        assert_eq!(fmt_kat_display(MicroKat(0)), "0");
    }

    #[test]
    fn tokens_get_thousands_separators() {
        assert_eq!(fmt_tokens(8_791), "8,791");
        assert_eq!(fmt_tokens(15_080_719), "15,080,719");
        assert_eq!(fmt_tokens(999), "999");
        assert_eq!(fmt_tokens(0), "0");
    }

    #[test]
    fn caption_rounds_as_a_pair() {
        let out = footer_left(MicroKat(84_919_281)).render(false);
        assert!(
            out.contains("84.92 KAT left · 15.08 of 100 burned"),
            "paired caption missing: {out}"
        );
    }

    #[test]
    fn plain_bar_is_width_cells_with_dotted_track() {
        let bar = braille_bar(0.8492, 24, false);
        assert_eq!(bar.chars().count(), 24);
        // 163/192 dots: 20 full cells + a 3-dot partial + 3 track cells.
        assert_eq!(bar.matches('\u{28FF}').count(), 20);
        assert_eq!(bar.matches('\u{28C0}').count(), 3);
        assert_eq!(bar.matches(ladder_char(3)).count(), 1);
    }

    #[test]
    fn bar_extremes_hold() {
        let full = braille_bar(1.0, 24, false);
        assert_eq!(full.matches('\u{28FF}').count(), 24);
        assert_eq!(full.matches('\u{28C0}').count(), 0);
        let empty = braille_bar(0.0, 24, false);
        assert_eq!(empty.matches('\u{28C0}').count(), 24);
        assert_eq!(empty.matches('\u{28FF}').count(), 0);
    }

    #[test]
    fn colored_bar_carries_dim_track_and_tone() {
        let bar = braille_bar(0.8492, 24, true);
        assert!(bar.contains(GREEN), "green tone missing: {bar:?}");
        assert!(bar.contains(TRACK_DIM), "dim gray-240 track missing: {bar:?}");
        // The dim tail is full blocks, never the dotted track.
        assert!(!bar.contains('\u{28C0}'));
    }

    #[test]
    fn low_levels_flip_the_tone() {
        assert!(braille_bar(0.20, 4, true).contains(AMBER));
        assert!(braille_bar(0.05, 4, true).contains(RED));
        assert!(braille_bar(0.60, 4, true).contains(GREEN));
    }

    #[test]
    fn color_mode_hyperlinks_the_board_label_plain_mode_spells_it() {
        let colored = footer_left(MicroKat(84_919_281)).render(true);
        assert!(colored.contains("\x1b]8;;"), "no OSC-8 link: {colored:?}");
        assert!(colored.contains("(you: c79c…5345)"), "row hint missing");
        let plain = footer_left(MicroKat(84_919_281)).render(false);
        assert!(plain.contains(BOARD), "plain mode must spell the URL");
        assert!(!plain.contains('\x1b'), "plain mode must be escape-free");
    }

    #[test]
    fn pseudonym_mirrors_the_server_projection() {
        // dapps routes::pseudonym over the ledger-width (first-32-hex) id:
        // first 4 + last 4 with an ellipsis.
        assert_eq!(
            leaderboard_pseudonym(ACCT).as_deref(),
            Some("c79c…5345")
        );
        assert_eq!(leaderboard_pseudonym("c79c93b8"), None);
        assert_eq!(leaderboard_pseudonym(""), None);
    }

    #[test]
    fn logged_out_renders_neither_link_nor_hint() {
        let out = KatFooter {
            account_hex: None,
            sync: SyncState::LoggedOut,
            ..footer_left(MicroKat(84_919_281))
        }
        .render(false);
        assert!(!out.contains(BOARD));
        assert!(!out.contains("you:"));
    }

    #[test]
    fn sync_states_render_their_story() {
        let unsynced = KatFooter {
            sync: SyncState::Unsynced {
                micro_kat: MicroKat(17_600),
                queued_batches: 1,
                ledger_url: BOARD,
            },
            ..footer_left(MicroKat(84_919_281))
        }
        .render(false);
        assert!(unsynced.contains("⚠ 0.0176 KAT unsynced (burn) · 1 mining batch(es) queued"));
        assert!(unsynced.contains("or `cargo heal sync`"));

        let logged_out = KatFooter {
            sync: SyncState::LoggedOut,
            ..footer_left(MicroKat(84_919_281))
        }
        .render(false);
        assert!(logged_out.contains("local only — not logged in"));

        let unknown = KatFooter {
            sync: SyncState::Unknown,
            ..footer_left(MicroKat(84_919_281))
        }
        .render(false);
        assert!(unknown.contains("reconciles at sync"));
    }

    /// The visual proof — run with `-- --nocapture` to SEE the footer at
    /// every grant level and sync state, colored + plain.
    #[test]
    fn visual_print_every_state() {
        for (name, left) in [
            ("green 84.92%", MicroKat(84_919_281)),
            ("amber 15.08%", MicroKat(15_080_000)),
            ("red 9.02%", MicroKat(9_020_000)),
            ("empty 0%", MicroKat(0)),
        ] {
            println!("── {name} ──");
            println!("{}", footer_left(left).render(true));
        }
        println!("── unsynced (plain) ──");
        let unsynced = KatFooter {
            sync: SyncState::Unsynced {
                micro_kat: MicroKat(17_600),
                queued_batches: 1,
                ledger_url: BOARD,
            },
            ..footer_left(MicroKat(84_919_281))
        };
        println!("{}", unsynced.render(false));
        println!("── logged out (plain) ──");
        let out = KatFooter {
            sync: SyncState::LoggedOut,
            ..footer_left(MicroKat(84_919_281))
        };
        println!("{}", out.render(false));
    }

    /// Parity with the meter's no-`$` pin (caveats #12/#15): a pricing
    /// figure must never render beside KAT accounting.
    #[test]
    fn footer_carries_no_currency_figure() {
        for state in [
            SyncState::Current {
                ledger_url: BOARD,
            },
            SyncState::Unsynced {
                micro_kat: MicroKat(1),
                queued_batches: 0,
                ledger_url: BOARD,
            },
            SyncState::LoggedOut,
            SyncState::Unknown,
        ] {
            let out = KatFooter {
                sync: state,
                ..footer_left(MicroKat(84_919_281))
            }
            .render(true);
            assert!(!out.contains('$'), "currency figure leaked: {out}");
        }
    }

    const TUNA_EXPIRES: u64 = 1_792_281_600; // 2026-10-18 UTC
    const TUNA_NOW: u64 = TUNA_EXPIRES - 12 * 86_400;

    fn tuna_line(balance_micro: i64) -> TunaLine {
        TunaLine {
            balance_micro: MicroKat(balance_micro),
            grant_micro: MicroKat(100_000_000),
            expires_unix: TUNA_EXPIRES,
            now_unix: TUNA_NOW,
        }
    }

    #[test]
    fn civil_date_known_answers() {
        assert_eq!(ymd_from_unix(0), (1970, 1, 1));
        assert_eq!(ymd_from_unix(86_399), (1970, 1, 1));
        assert_eq!(ymd_from_unix(86_400), (1970, 1, 2));
        assert_eq!(ymd_from_unix(1_792_281_600), (2026, 10, 18));
        // Leap day: 54*365 = 19710 + 13 leaps in [1970, 2024) = 19723
        // days to 2024-01-01; +31 (Jan) = 19754 to Feb 1; +28 = 19782.
        assert_eq!(ymd_from_unix(19_754 * 86_400), (2024, 2, 1));
        assert_eq!(ymd_from_unix(19_782 * 86_400), (2024, 2, 29));
        assert_eq!(ymd_from_unix(19_783 * 86_400), (2024, 3, 1));
    }

    #[test]
    fn tuna_line_renders_only_when_present_and_the_absence_is_byte_identical() {
        // None: the pre-TUNA 3-line footer, byte for byte.
        let plain = footer_left(MicroKat(84_919_281)).render(false);
        assert_eq!(plain.matches('\n').count(), 2, "no tuna line: {plain:?}");
        assert!(!plain.contains("TUNA"));

        // Some: the 4th line, between grant and sync.
        let with = KatFooter {
            tuna: Some(tuna_line(47_200_000)),
            ..footer_left(MicroKat(84_919_281))
        }
        .render(false);
        let lines: Vec<&str> = with.lines().collect();
        assert_eq!(lines.len(), 4, "{with:?}");
        assert!(lines[1].contains("grant"), "{with:?}");
        assert!(
            lines[2].contains("47.2 of 100 TUNA left · expires 2026-10-18 (12d) · trial credit — heals only"),
            "{with:?}"
        );
        assert!(lines[3].contains("sync"), "{with:?}");
    }

    #[test]
    fn tuna_line_handles_the_edges() {
        // Zero balance: the bar empties, the caption stays paired.
        let out = KatFooter {
            tuna: Some(tuna_line(0)),
            ..footer_left(MicroKat(84_919_281))
        }
        .render(false);
        assert!(out.contains("0 of 100 TUNA left"), "{out:?}");

        // Negative (defensive): clamps to 0 at render, never prints a
        // negative tank.
        let out = KatFooter {
            tuna: Some(tuna_line(-5)),
            ..footer_left(MicroKat(84_919_281))
        }
        .render(false);
        assert!(out.contains("0 of 100 TUNA left"), "{out:?}");
        assert!(!out.contains("-5"));

        // Expired (not yet swept): named, with the sweep note.
        let expired = TunaLine {
            now_unix: TUNA_EXPIRES + 86_400,
            ..tuna_line(47_200_000)
        };
        let out = KatFooter {
            tuna: Some(expired),
            ..footer_left(MicroKat(84_919_281))
        }
        .render(false);
        assert!(
            out.contains("expired 2026-10-18 — the sweep reclaims the rest"),
            "{out:?}"
        );
    }
}
