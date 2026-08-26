//! KAT code-token meter — Proposal 005 Phase 0 T0.5 (Plan 023).
//!
//! **1 KAT = 1 word token of code processed by the healer.** A word token is
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

use std::io;
use std::path::{Path, PathBuf};

/// The meter definition version (G9b). Bump ONLY when the token definition
/// changes the count for some input, and record the delta in
/// [`METER_CHANGELOG`] in the same change.
pub const METER_VERSION: u32 = 1;

/// Versioned deltas of the counting definition (G9b — "never a silent
/// reprice"). Every version that changes the count for any input carries an
/// entry describing what moved and why.
pub const METER_CHANGELOG: &[(u32, &str)] = &[
    (
        1,
        "initial definition: a word token is a maximal run of bytes that are not ASCII \
         whitespace (space, \\t, \\n, \\r, \\f). Line-ending-insensitive by construction: \
         \\r is whitespace, so CRLF and LF checkouts of the same repo bill identically.",
    ),
];

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
    pub fn open(root: &Path) -> KatMeter {
        Self::at_path(root.join(".heal").join(Self::FILE_NAME))
    }

    /// Open a meter backed by an exact file path (no `.heal` composition) —
    /// the `RIIR_HEAL_KAT_METER_FILE` override + tests.
    #[must_use]
    pub fn open_at(path: PathBuf) -> KatMeter {
        Self::at_path(path)
    }

    /// Shared constructor: load (or default) the state at `path`, count the
    /// session, start at zero session burn.
    fn at_path(path: PathBuf) -> KatMeter {
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
            METER_CHANGELOG
                .iter()
                .any(|(v, _)| *v == METER_VERSION),
            "METER_VERSION {METER_VERSION} has no METER_CHANGELOG entry"
        );
        assert_eq!(METER_VERSION, 1);
    }

    #[test]
    fn meter_open_counts_a_session() {
        let dir = std::env::temp_dir().join(format!(
            "kat_meter_unit_{}_{}",
            std::process::id(),
            line!()
        ));
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
        assert_eq!(m.state().cumulative_tokens, 0, "future version must not leak counts");
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
}
