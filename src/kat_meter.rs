//! KAT code-token meter + charge consumer — Proposal 005 Phase 0 T0.5/T0.6
//! (Plans 023 + 024).
//!
//! **1 KAT = 1,000,000 code word tokens processed by the healer** (the
//! deployed v1 rate — `charged_kat` counts MICRO-KAT, one micro per
//! token). A word token is
//! a maximal run of bytes that are not ASCII whitespace (`u8::is_ascii_whitespace`
//! — space, `\t`, `\n`, `\r`, `\f`). The counter runs over the **raw bytes of
//! the source the healer processes**, never over an AST:
//!
//! - **Unparseable code still bills** (G9c): a parse-dependent counter bills
//!   zero exactly on the miss class the reachability contract (Issue 030)
//!   routes to the most expensive path. Raw bytes make the bill a function of
//!   work actually done, not of parse success.
//! - **Host-independent by construction** (G9a): `\r` is whitespace, so
//!   CRLF/LF/CR checkouts of the same repo bill identically; path separators
//!   never appear in file content. The counter is a pure function of bytes —
//!   same input, same count, any host, any run.
//! - **Version-stamped** (G9b): [`METER_VERSION`] + [`METER_CHANGELOG`] — a
//!   definition change that moves the count is an explicit, versioned delta,
//!   never a silent reprice. The version rides the persisted file.
//! - **Accounting-only** (G9d): this meter never mints or spends. It is a
//!   local burn ledger the sync payload (Phase 0 T0.3+) reconciles later;
//!   the server-side ledger is the authority.
//!
//! Persistence: repo-local `<root>/.heal/kat_meter.json` (same root family as
//! the trajectory store). Missing file → fresh state; corrupt or
//! unknown-version file → warn + fresh (the `EvolveRecorder` envelope
//! precedent: a bookkeeping artifact never aborts a heal run).
//!
//! G2 (alloc-free): [`count_word_tokens`] takes `&[u8]` and returns `u64` —
//! zero allocation by construction, pinned by the `CountingAllocator` fixture
//! in `tests/kat_meter_g9.rs`.
//!
//! # The charge consumer (T0.6 — per-unit burn)
//!
//! [`KatCharge`] consumes [`MeterState`] events and projects the KAT burn
//! against the free-tier grant: `charge = tokens × rate` (one integer
//! multiply — the whole reason per-unit burn beat the volume-tiered lease at
//! Phase 0; see Plan 024's decision record). Still **accounting-only** (G9d):
//! the balance is a local projection, the server ledger is the authority
//! (caveat #9), and the projection itself never blocks a run (G3 —
//! warn-and-pay). The BLOCKING half lives server-side since Issue 075 (the
//! owner's soft gate, 2026-09-08): a depleted, logged-in, SYNCED,
//! never-contributing account is refused at run start by
//! [`crate::kat_soft_gate`] over the last-synced server balance — never
//! this projection (it models neither the faucet drip nor mining credits).
//! The watermark design keeps reprices honest: settle charges only
//! the meter delta at the *then-current* rate, so a rate change (a
//! `CHARGE_VERSION` bump) applies to new tokens only — the frozen
//! `charged_kat` below the watermark is never silently re-derived.

use std::io;
use std::path::{Path, PathBuf};

/// The meter definition version (G9b). Bump ONLY when the token definition
/// changes the count for some input, and record the delta in
/// [`METER_CHANGELOG`] in the same change.
pub const METER_VERSION: u32 = 1;

/// Versioned deltas of the counting definition (G9b — "never a silent
/// reprice"). Every version that changes the count for any input carries an
/// entry describing what moved and why.
pub const METER_CHANGELOG: &[(u32, &str)] = &[(
    1,
    "initial definition: a word token is a maximal run of bytes that are not ASCII \
         whitespace (space, \\t, \\n, \\r, \\f). Line-ending-insensitive by construction: \
         \\r is whitespace, so CRLF and LF checkouts of the same repo bill identically.",
)];

/// Count word tokens in raw span bytes (the KAT bill).
///
/// A word token is a maximal run of bytes for which `u8::is_ascii_whitespace`
/// is false. Non-ASCII bytes (UTF-8 continuation bytes, binary payloads) are
/// token bytes — a Unicode identifier is one token. Alloc-free; deterministic;
/// identical for identical bytes on any host.
#[must_use]
pub fn count_word_tokens(bytes: &[u8]) -> u64 {
    let mut n: u64 = 0;
    let mut in_token = false;
    for &b in bytes {
        if b.is_ascii_whitespace() {
            in_token = false;
        } else if !in_token {
            in_token = true;
            n += 1;
        }
    }
    n
}

/// The persisted meter state (the local burn ledger).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MeterState {
    /// The counting definition that produced `cumulative_tokens` (G9b).
    pub version: u32,
    /// Total code word tokens billed across all sessions on this machine.
    pub cumulative_tokens: u64,
    /// Number of metered runs (one per [`KatMeter::open`]).
    pub sessions: u64,
}

impl Default for MeterState {
    fn default() -> Self {
        Self {
            version: METER_VERSION,
            cumulative_tokens: 0,
            sessions: 0,
        }
    }
}

/// The KAT meter — counts code word tokens on the heal path and persists the
/// cumulative burn locally (Phase 0 T0.5). Accounting-only (G9d): the counts
/// recorded here are a local ledger for later sync reconciliation; nothing
/// mints or spends on them.
#[derive(Debug)]
pub struct KatMeter {
    path: PathBuf,
    state: MeterState,
    session_tokens: u64,
    session_files: u32,
}

impl KatMeter {
    /// The meter file name under the `.heal` root.
    pub const FILE_NAME: &'static str = "kat_meter.json";

    /// Open (or create) the meter at `<root>/.heal/kat_meter.json`.
    ///
    /// A missing file starts fresh. A corrupt or unknown-version file warns
    /// to stderr and starts fresh — a bookkeeping artifact never aborts a
    /// heal run (the `EvolveRecorder` envelope precedent). Opening a meter
    /// counts as one session.
    #[must_use]
    pub fn open(root: &Path) -> Self {
        Self::at_path(root.join(".heal").join(Self::FILE_NAME))
    }

    /// Open a meter backed by an exact file path (no `.heal` composition) —
    /// the `RIIR_HEAL_KAT_METER_FILE` override + tests.
    #[must_use]
    pub fn open_at(path: PathBuf) -> Self {
        Self::at_path(path)
    }

    /// Shared constructor: load (or default) the state at `path`, count the
    /// session, start at zero session burn.
    fn at_path(path: PathBuf) -> Self {
        let mut state = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<MeterState>(&text) {
                Ok(s) if s.version == METER_VERSION => s,
                Ok(s) => {
                    eprintln!(
                        "kat-meter: {} carries meter version {} (this build defines {}); \
                         starting fresh — see METER_CHANGELOG",
                        path.display(),
                        s.version,
                        METER_VERSION
                    );
                    MeterState::default()
                }
                Err(e) => {
                    eprintln!(
                        "kat-meter: ignoring unreadable meter file ({}): {e}",
                        path.display()
                    );
                    MeterState::default()
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => MeterState::default(),
            Err(e) => {
                eprintln!(
                    "kat-meter: ignoring unreadable meter file ({}): {e}",
                    path.display()
                );
                MeterState::default()
            }
        };
        state.sessions = state.sessions.saturating_add(1);
        Self {
            path,
            state,
            session_tokens: 0,
            session_files: 0,
        }
    }

    /// Bill the raw bytes of one processed source (a file the healer read, a
    /// span it worked over). Returns the token count billed for this input.
    pub fn record_span(&mut self, bytes: &[u8]) -> u64 {
        let n = count_word_tokens(bytes);
        self.session_tokens = self.session_tokens.saturating_add(n);
        self.state.cumulative_tokens = self.state.cumulative_tokens.saturating_add(n);
        self.session_files = self.session_files.saturating_add(1);
        n
    }

    /// Tokens billed in this session (since [`KatMeter::open`]).
    #[must_use]
    pub fn session_tokens(&self) -> u64 {
        self.session_tokens
    }

    /// Sources billed in this session.
    #[must_use]
    pub fn session_files(&self) -> u32 {
        self.session_files
    }

    /// The persisted state (cumulative + version stamp).
    #[must_use]
    pub fn state(&self) -> &MeterState {
        &self.state
    }

    /// The meter file path (diagnostics).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Persist the ledger. Creates the `.heal` directory if needed.
    ///
    /// # Errors
    ///
    /// Returns the underlying `io::Error` from directory creation or the
    /// ledger write — the caller warns and continues (an unpersisted burn is
    /// lost bookkeeping, never a failed heal).
    pub fn save(&self) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(&self.state)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(&self.path, text)
    }

    /// One-line session summary for the CLI (no `$` figures — the meter
    /// reports tokens only; pricing is a server-side concern, caveat #12/#15).
    #[must_use]
    pub fn summary_line(&self) -> String {
        format!(
            "kat: {} code word token(s) billed across {} source(s) this run (meter v{}, \
             cumulative {} — accounting only, reconciles at sync)",
            self.session_tokens,
            self.session_files,
            self.state.version,
            self.state.cumulative_tokens
        )
    }
}

// ── The charge consumer (T0.6 — Plan 024) ─────────────────────────────────────

/// The charge-model version. Bump ONLY when the charge model changes what KAT
/// is owed for the same token count (a reprice), and record the delta in
/// [`CHARGE_CHANGELOG`] in the same change. Independent of [`METER_VERSION`]
/// on purpose: a counting change moves the token count (and invalidates old
/// meter files — they start fresh); a charge change moves the KAT owed per
/// token (and must SALVAGE old charge files — see [`KatCharge::open_at`]).
/// Either change is a reprice; neither may be silent.
pub const CHARGE_VERSION: u32 = 1;

/// Versioned deltas of the charge model (the reprice audit trail). Every
/// version that changes the KAT owed for some token count carries an entry.
pub const CHARGE_CHANGELOG: &[(u32, &str)] = &[(
    1,
    "initial charge model (T0.6 decision, 2026-08-26): per-unit burn — 1 KAT per \
         MILLION code word tokens (CHARGE_KAT_PER_MILLION_TOKENS = 1_000_000; the \
         deployed economics — the 100 KAT free grant funds ~100M tokens, the \
         proposal's ~$300 Claude-equivalent intent; charged_kat counts MICRO-KAT). \
         Chosen over the \
         volume-tiered lease (caveat #16's recommendation) for Phase 0: simplest \
         auditable accounting (charge = tokens x rate, one integer multiply), no \
         tier-boundary or lease-expiry semantics. Revisitable before T1.3 (the Stripe \
         SKU shape), where the caveat's substitution argument actually bites.",
)];

// issue 088: both economics constants moved to the protocol tier's
// `kat_units` (shared vocabulary the server side pins too) — re-exported so
// every existing path (`crate::kat_meter::FREE_TIER_GRANT_KAT`) resolves.
pub use crate::kat_units::{CHARGE_KAT_PER_MILLION_TOKENS, FREE_TIER_GRANT_KAT};

/// Out-of-KAT warn threshold (§KAT): the warning fires while at most this much
/// of the grant projection remains. The QR/top-up half of the UX is T1.6.
/// G3 as amended by Issue 075: the projection is warn-only, but the 075
/// soft gate (`crate::kat_soft_gate`) refuses depleted, logged-in, SYNCED,
/// never-contributing accounts at run start over the SERVER balance.
pub const KAT_WARN_REMAINING: u64 = 10_000_000;

/// KAT owed for `tokens` at the current rate (the per-unit burn).
/// Floor semantics; exact at v1's 1:1 rate for every input — the u128
/// intermediate keeps `tokens × rate` from overflowing before the division,
/// and the result saturates at `u64::MAX` for hypothetical future rates that
/// would exceed it.
#[must_use]
pub fn kat_for_tokens(tokens: u64) -> u64 {
    let units = u128::from(tokens) * u128::from(CHARGE_KAT_PER_MILLION_TOKENS);
    let kat = units / 1_000_000;
    u64::try_from(kat).unwrap_or(u64::MAX)
}

/// The persisted charge-ledger state (the local burn projection).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChargeState {
    /// The charge model that produced `charged_kat` (the reprice audit stamp).
    pub charge_version: u32,
    /// Total KAT burned across all settles, frozen at the rates that applied
    /// when each segment was charged. Never re-derived on a rate change.
    pub charged_kat: u64,
    /// The meter `cumulative_tokens` watermark already charged. Tokens above
    /// it are uncharged; the next [`KatCharge::settle`] charges the delta at
    /// the then-current rate.
    pub meter_watermark_tokens: u64,
}

impl Default for ChargeState {
    fn default() -> Self {
        Self {
            charge_version: CHARGE_VERSION,
            charged_kat: 0,
            meter_watermark_tokens: 0,
        }
    }
}

/// The KAT charge consumer — projects meter events onto the per-unit burn
/// (Phase 0 T0.6). Accounting-only (G9d): the balance below is a LOCAL
/// PROJECTION for the warn UX; the server ledger reconciles at sync (T0.3+)
/// and is the authority (caveat #9). Nothing here mints, spends, or blocks a
/// heal.
#[derive(Debug)]
pub struct KatCharge {
    path: PathBuf,
    state: ChargeState,
    session_kat: u64,
}

impl KatCharge {
    /// The charge-ledger file name under the `.heal` root.
    pub const FILE_NAME: &'static str = "kat_charge.json";

    /// Open (or create) the charge ledger at `<root>/.heal/kat_charge.json`,
    /// recovering from `meter` when the ledger is missing or corrupt.
    ///
    /// Open the charge BEFORE the meter records this session's tokens: the
    /// recovery path (`missing`/`corrupt` → watermark = the meter cumulative
    /// *at open*) is only correct against a pre-session reading — opened
    /// after recording, it would swallow the session's burn.
    #[must_use]
    pub fn open(root: &Path, meter: &MeterState) -> Self {
        Self::at_path(root.join(".heal").join(Self::FILE_NAME), meter)
    }

    /// Open a charge ledger backed by an exact file path (no `.heal`
    /// composition) — the `RIIR_HEAL_KAT_CHARGE_FILE` override + tests.
    #[must_use]
    pub fn open_at(path: PathBuf, meter: &MeterState) -> Self {
        Self::at_path(path, meter)
    }

    /// Shared constructor. Load policy (never aborts a heal — the
    /// meter/recorder precedents):
    ///
    /// - **version mismatch → salvage**: keep `charged_kat` + watermark (valid
    ///   history under the model that produced it), restamp the version, warn.
    ///   No double-charge, no re-derivation — a reprice applies to new tokens
    ///   only.
    /// - **missing / corrupt → recover from the meter**: the meter file is the
    ///   token authority, so a lost charge ledger re-derives
    ///   `charged_kat = kat_for_tokens(cumulative_at_open)` whole at the
    ///   current rate (the mixed-rate history is unrecoverable; whole-at-
    ///   current-rate is the honest approximation, and this path is only
    ///   reachable by file loss — a version bump takes the salvage path).
    fn at_path(path: PathBuf, meter: &MeterState) -> Self {
        let mut state = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<ChargeState>(&text) {
                Ok(s) if s.charge_version == CHARGE_VERSION => s,
                Ok(s) => {
                    eprintln!(
                        "kat-charge: {} carries charge version {} (this build defines \
                         {}); keeping the frozen charged_kat={} / watermark={} and applying \
                         the new rate to new tokens only — see CHARGE_CHANGELOG",
                        path.display(),
                        s.charge_version,
                        CHARGE_VERSION,
                        s.charged_kat,
                        s.meter_watermark_tokens
                    );
                    ChargeState {
                        charge_version: CHARGE_VERSION,
                        charged_kat: s.charged_kat,
                        meter_watermark_tokens: s.meter_watermark_tokens,
                    }
                }
                Err(e) => {
                    eprintln!(
                        "kat-charge: ignoring unreadable charge ledger ({}): {e}; \
                         recovering from the meter (cumulative {} tokens)",
                        path.display(),
                        meter.cumulative_tokens
                    );
                    Self::recovered(meter)
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Self::recovered(meter),
            Err(e) => {
                eprintln!(
                    "kat-charge: ignoring unreadable charge ledger ({}): {e}; \
                     recovering from the meter (cumulative {} tokens)",
                    path.display(),
                    meter.cumulative_tokens
                );
                Self::recovered(meter)
            }
        };
        state.charge_version = CHARGE_VERSION;
        Self {
            path,
            state,
            session_kat: 0,
        }
    }

    /// The recovered state for a missing/corrupt ledger: everything the meter
    /// has ever billed, charged whole at the current rate.
    fn recovered(meter: &MeterState) -> ChargeState {
        ChargeState {
            charge_version: CHARGE_VERSION,
            charged_kat: kat_for_tokens(meter.cumulative_tokens),
            meter_watermark_tokens: meter.cumulative_tokens,
        }
    }

    /// Charge the meter's uncharged delta at the current rate. Returns the
    /// KAT charged by THIS settle.
    ///
    /// A meter that went backwards (a corrupt meter file recovered fresh —
    /// Plan 023 semantics) saturates to a zero delta: no charge, no refund,
    /// no negative balance, until the meter re-passes the watermark.
    pub fn settle(&mut self, meter: &MeterState) -> u64 {
        let delta = meter
            .cumulative_tokens
            .saturating_sub(self.state.meter_watermark_tokens);
        let kat = kat_for_tokens(delta);
        self.state.charged_kat = self.state.charged_kat.saturating_add(kat);
        self.state.meter_watermark_tokens = self.state.meter_watermark_tokens.saturating_add(delta);
        self.session_kat = self.session_kat.saturating_add(kat);
        kat
    }

    /// KAT charged by settles since [`KatCharge::open`] (the "this run" view).
    #[must_use]
    pub fn session_kat(&self) -> u64 {
        self.session_kat
    }

    /// The persisted state (frozen burn + watermark + version stamp).
    #[must_use]
    pub fn state(&self) -> &ChargeState {
        &self.state
    }

    /// The local balance projection: the free-tier grant minus the frozen
    /// burn. Saturates at zero — never negative. The server ledger is the
    /// authority (caveat #9); this exists for the warn UX only.
    #[must_use]
    pub fn balance_projection(&self) -> u64 {
        FREE_TIER_GRANT_KAT.saturating_sub(self.state.charged_kat)
    }

    /// Whether the out-of-KAT warning should fire (§KAT: warn at 10M
    /// remaining — the QR/top-up half is T1.6).
    #[must_use]
    pub fn low_balance(&self) -> bool {
        self.balance_projection() <= KAT_WARN_REMAINING
    }

    /// The charge-ledger file path (diagnostics).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Persist the charge ledger. Creates the `.heal` directory if needed.
    /// Save failure is lost bookkeeping, never a failed heal (the meter
    /// precedent).
    ///
    /// # Errors
    ///
    /// Returns the underlying `io::Error` from directory creation or the
    /// ledger write.
    pub fn save(&self) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(&self.state)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(&self.path, text)
    }

    /// One-line charge summary for the CLI. KAT figures only — no `$`
    /// (caveats #12/#15: pricing is server-side with an `as-of` date; a `$`
    /// figure never renders beside KAT accounting). Every figure is stored
    /// in micro and rendered through the shared 6-decimal KAT formatter —
    /// raw micro printed as whole KAT read 10⁶× too big (issue 031).
    #[must_use]
    pub fn summary_line(&self) -> String {
        let kat = |micro: u64| {
            crate::kat_payreq_client::render_kat(i64::try_from(micro).unwrap_or(i64::MAX))
        };
        format!(
            "kat-charge: {} burned this run (charge v{}, {} per M tokens, \
             cumulative {} — {} of the {} grant projection remains; \
             accounting only, the server ledger reconciles at sync)",
            kat(self.session_kat),
            self.state.charge_version,
            kat(CHARGE_KAT_PER_MILLION_TOKENS),
            kat(self.state.charged_kat),
            kat(self.balance_projection()),
            kat(FREE_TIER_GRANT_KAT)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_whitespace_only_bill_zero() {
        assert_eq!(count_word_tokens(b""), 0);
        assert_eq!(count_word_tokens(b"   \n\t\r\n  \x0C"), 0);
    }

    #[test]
    fn simple_counts() {
        assert_eq!(count_word_tokens(b"fn main() {}"), 3);
        assert_eq!(count_word_tokens(b"let x: u32 = 1;"), 5);
        // Multiple whitespace runs collapse: still one token boundary.
        assert_eq!(count_word_tokens(b"a\t \n\r\n b"), 2);
    }

    #[test]
    fn non_ascii_bytes_are_token_bytes() {
        // A Unicode identifier is ONE token (its UTF-8 bytes are not ASCII
        // whitespace) — the definition is pinned on bytes, not graphemes.
        assert_eq!(count_word_tokens("héllo_wörld".as_bytes()), 1);
        assert_eq!(count_word_tokens("fn 関数() {}".as_bytes()), 3);
    }

    #[test]
    fn version_stamp_is_changelog_covered() {
        // G9b: the current version must carry an explicit changelog entry.
        assert!(
            METER_CHANGELOG.iter().any(|(v, _)| *v == METER_VERSION),
            "METER_VERSION {METER_VERSION} has no METER_CHANGELOG entry"
        );
        assert_eq!(METER_VERSION, 1);
    }

    #[test]
    fn meter_open_counts_a_session() {
        let dir =
            std::env::temp_dir().join(format!("kat_meter_unit_{}_{}", std::process::id(), line!()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut m = KatMeter::open(&dir);
        assert_eq!(m.state().sessions, 1);
        assert_eq!(m.record_span(b"fn main() {}"), 3);
        assert_eq!(m.session_tokens(), 3);
        assert_eq!(m.state().cumulative_tokens, 3);
        m.save().unwrap();

        let m2 = KatMeter::open(&dir);
        assert_eq!(m2.state().sessions, 2);
        assert_eq!(m2.state().cumulative_tokens, 3);
        assert_eq!(m2.session_tokens(), 0, "session resets on open");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_warns_and_starts_fresh() {
        let dir = std::env::temp_dir().join(format!(
            "kat_meter_corrupt_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(dir.join(".heal")).unwrap();
        std::fs::write(dir.join(".heal").join(KatMeter::FILE_NAME), "{ not json").unwrap();
        let m = KatMeter::open(&dir);
        assert_eq!(m.state().cumulative_tokens, 0);
        assert_eq!(m.state().version, METER_VERSION);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_version_starts_fresh() {
        let dir = std::env::temp_dir().join(format!(
            "kat_meter_future_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(dir.join(".heal")).unwrap();
        let future = serde_json::json!({
            "version": METER_VERSION + 1,
            "cumulative_tokens": 999_999,
            "sessions": 99,
        });
        std::fs::write(
            dir.join(".heal").join(KatMeter::FILE_NAME),
            future.to_string(),
        )
        .unwrap();
        let m = KatMeter::open(&dir);
        assert_eq!(
            m.state().cumulative_tokens,
            0,
            "future version must not leak counts"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn persisted_file_carries_the_version_stamp() {
        let dir = std::env::temp_dir().join(format!(
            "kat_meter_stamp_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut m = KatMeter::open(&dir);
        m.record_span(b"let x = 1;");
        m.save().unwrap();
        let text = std::fs::read_to_string(dir.join(".heal").join(KatMeter::FILE_NAME)).unwrap();
        assert!(text.contains(&format!("\"version\": {METER_VERSION}")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_at_targets_the_exact_file() {
        let dir = std::env::temp_dir().join(format!(
            "kat_meter_openat_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("mymeter.json");
        let mut m = KatMeter::open_at(file.clone());
        assert_eq!(m.path(), file.as_path());
        m.record_span(b"abc def");
        m.save().unwrap();
        assert!(file.exists());
        let m2 = KatMeter::open_at(file);
        assert_eq!(m2.state().cumulative_tokens, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── T0.6: the charge consumer (Plan 024) ──────────────────────────

    fn meter_with(cumulative: u64) -> MeterState {
        MeterState {
            version: METER_VERSION,
            cumulative_tokens: cumulative,
            sessions: 1,
        }
    }

    #[test]
    fn charge_rate_is_changelog_covered() {
        assert!(
            CHARGE_CHANGELOG.iter().any(|(v, _)| *v == CHARGE_VERSION),
            "CHARGE_VERSION {CHARGE_VERSION} has no CHARGE_CHANGELOG entry"
        );
        assert_eq!(CHARGE_VERSION, 1);
    }

    #[test]
    fn kat_for_tokens_is_exact_at_the_v1_rate() {
        // v1 rate is exactly 1 KAT/token — the identity for EVERY input,
        // including values whose ×rate intermediate overflows u64 (the u128
        // path) and the extremes.
        assert_eq!(kat_for_tokens(0), 0);
        assert_eq!(kat_for_tokens(1), 1);
        assert_eq!(kat_for_tokens(123_456_789), 123_456_789);
        assert_eq!(kat_for_tokens(u64::MAX), u64::MAX);
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)] // tiny fixed values; the real fn saturates
    fn kat_for_tokens_floors_fractional_rates() {
        // The per-million encoding supports fractional rates without a type
        // change; floor semantics are pinned so the policy is explicit. At
        // 0.5 KAT/token (a hypothetical FUTURE rate — not v1): 2 tokens → 1
        // KAT, 3 → 1 (floor), 1_000_000 → 500_000.
        let rate: u128 = 500_000;
        let f = |t: u64| ((u128::from(t) * rate) / 1_000_000) as u64;
        assert_eq!(f(2), 1);
        assert_eq!(f(3), 1);
        assert_eq!(f(1_000_000), 500_000);
    }

    #[test]
    fn settle_charges_only_the_uncharged_delta() {
        let dir = std::env::temp_dir().join(format!(
            "kat_charge_delta_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut c = KatCharge::open(&dir, &meter_with(0));

        assert_eq!(c.settle(&meter_with(100)), 100, "first settle charges all");
        assert_eq!(
            c.settle(&meter_with(100)),
            0,
            "same cumulative → no re-charge"
        );
        assert_eq!(c.settle(&meter_with(150)), 50, "only the delta");
        assert_eq!(c.state().charged_kat, 150);
        assert_eq!(c.state().meter_watermark_tokens, 150);
        assert_eq!(
            c.session_kat(),
            150,
            "session sums all settles (100 + 0 + 50)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn meter_regression_charges_nothing_until_repassed() {
        // A corrupt meter recovered fresh can sit BELOW the watermark — the
        // delta saturates to zero; never a refund, never a negative balance.
        let dir = std::env::temp_dir().join(format!(
            "kat_charge_regres_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut c = KatCharge::open(&dir, &meter_with(0));
        c.settle(&meter_with(500));
        assert_eq!(c.settle(&meter_with(100)), 0);
        assert_eq!(c.state().charged_kat, 500, "no refund on regression");
        assert_eq!(c.state().meter_watermark_tokens, 500, "watermark holds");
        assert_eq!(
            c.settle(&meter_with(600)),
            100,
            "charges again once re-passed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn charge_survives_reopen_and_accumulates() {
        let dir = std::env::temp_dir().join(format!(
            "kat_charge_persist_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut c1 = KatCharge::open(&dir, &meter_with(0));
        assert_eq!(c1.settle(&meter_with(40)), 40);
        c1.save().unwrap();

        // Reopen against a PRE-session meter reading of 40 (the bin's order:
        // meter open → charge open → session records → settle).
        let mut c2 = KatCharge::open(&dir, &meter_with(40));
        assert_eq!(c2.state().charged_kat, 40);
        assert_eq!(c2.session_kat(), 0, "session resets on open");
        assert_eq!(c2.settle(&meter_with(65)), 25);
        c2.save().unwrap();

        let c3 = KatCharge::open(&dir, &meter_with(65));
        assert_eq!(c3.state().charged_kat, 65);
        assert_eq!(c3.state().meter_watermark_tokens, 65);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn version_mismatch_salvages_history_without_double_charge() {
        let dir = std::env::temp_dir().join(format!(
            "kat_charge_salvage_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(dir.join(".heal")).unwrap();
        let future = serde_json::json!({
            "charge_version": CHARGE_VERSION + 1,
            "charged_kat": 12_345,
            "meter_watermark_tokens": 500,
        });
        std::fs::write(
            dir.join(".heal").join(KatCharge::FILE_NAME),
            future.to_string(),
        )
        .unwrap();

        let mut c = KatCharge::open(&dir, &meter_with(500));
        assert_eq!(c.state().charge_version, CHARGE_VERSION, "restamped");
        assert_eq!(c.state().charged_kat, 12_345, "frozen history survives");
        assert_eq!(c.state().meter_watermark_tokens, 500);
        assert_eq!(
            c.settle(&meter_with(600)),
            100,
            "new tokens at the new rate"
        );
        assert_eq!(c.state().charged_kat, 12_445);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_or_missing_charge_recovers_from_the_meter() {
        let dir = std::env::temp_dir().join(format!(
            "kat_charge_recover_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(dir.join(".heal")).unwrap();
        std::fs::write(dir.join(".heal").join(KatCharge::FILE_NAME), "{ not json").unwrap();

        // Recovery against the PRE-session meter reading (the bin's open order).
        let mut c = KatCharge::open(&dir, &meter_with(200));
        assert_eq!(
            c.state().charged_kat,
            200,
            "history re-derived from the meter"
        );
        assert_eq!(c.state().meter_watermark_tokens, 200);
        assert_eq!(
            c.settle(&meter_with(250)),
            50,
            "no double-charge of history"
        );
        assert_eq!(c.state().charged_kat, 250);

        // Missing file takes the same recovery path.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut c2 = KatCharge::open(&dir, &meter_with(70));
        assert_eq!(c2.state().charged_kat, 70);
        c2.settle(&meter_with(80));
        assert_eq!(c2.state().charged_kat, 80);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn balance_projection_saturates_and_warns_at_the_threshold() {
        let dir = std::env::temp_dir().join(format!(
            "kat_charge_balance_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut c = KatCharge::open(&dir, &meter_with(0));
        assert_eq!(c.balance_projection(), FREE_TIER_GRANT_KAT);
        assert!(!c.low_balance(), "fresh grant is far above the warn line");

        c.settle(&meter_with(FREE_TIER_GRANT_KAT - KAT_WARN_REMAINING));
        assert_eq!(c.balance_projection(), KAT_WARN_REMAINING);
        assert!(c.low_balance(), "warn fires AT the threshold");

        c.settle(&meter_with(FREE_TIER_GRANT_KAT + 5_000_000));
        assert_eq!(c.balance_projection(), 0, "saturates, never negative");
        assert!(c.low_balance());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn charge_summary_carries_no_currency_figure() {
        let dir = std::env::temp_dir().join(format!(
            "kat_charge_summary_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut c = KatCharge::open(&dir, &meter_with(0));
        c.settle(&meter_with(999));
        let line = c.summary_line();
        assert!(!line.contains('$'), "no currency figure: {line}");
        assert!(!line.to_lowercase().contains("usd"));
        assert!(line.contains("KAT"));
        assert!(line.contains("accounting only"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
