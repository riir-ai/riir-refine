//! Proposal 027 — the hosted-lane fair-use drain: the CLIENT half
//! (plan 326 T2/T3/T5). One machine-local day state beside
//! [`crate::client_id`]'s store, a two-integer-compare pre-check the
//! escalation dispatch runs immediately before a seat call, and the
//! per-call attestation header composer the seat verifies.
//!
//! ## The laws
//!
//! - **UX pre-check only — the seat is the enforcement** (Proposal 027
//!   §"The client gate"). A decline is a POLITE early stop: the modelless
//!   fix the caller already has stands, and the seat's own counters stay
//!   the ledger of record. A deliberately wrong client clock only skews
//!   this local mirror.
//! - **The constants are SERVED, never compiled.**
//!   [`DayState::drain_state`] caches the last successful
//!   `/points/balance` read's `drain_state`; `None` (never cached, or a
//!   server that has not ratified the drain config) makes the gate INERT
//!   — byte-identical to the pre-drain behavior: everything allowed,
//!   nothing consumed, no headers composed. A ratified re-tune never
//!   needs a client release.
//! - **Anonymous is inert.** The personal allowance is a JOINED-account
//!   construct; a machine with no account key gets no gate at all (the
//!   unjoined posture rides the seat's per-source budget, which the
//!   client cannot see and does not mirror).
//! - **Fails open.** Any local error — state unreadable, account key
//!   absent, a poisoned lock — answers "allow" and moves on. The only
//!   outcome the gate can force is the polite decline.
//! - **The decision is two integer compares.** The steady-state
//!   pre-check allocates nothing (the once-per-UTC-day roll writes the
//!   day string — that allocation is the roll, not the check; the G4
//!   canary brackets the steady state). Persistence rides AFTER the
//!   verdict, best-effort, and can never fail a call.
//! - **Contribution vocabulary only.** The decline's disclosure names
//!   the refuel path (contribute to earn); payment vocabulary is
//!   deliberately absent (the issue-075 soft-gate sentence shape).
//!
//! ## The counters
//!
//! `hosted_calls_today` counts the FREE-allowance leg (the algorithm's
//! `hosted_calls_today`); `billable_calls_today` counts the billable leg
//! against the served `billable_ceiling_today`; `call_seq` is the
//! per-process, per-day attestation sequence (the seat's replay window
//! is per-(account, day), so the counter rolls with the day). Two
//! concurrent `cargo refine` processes share the file — last writer
//! wins; a duplicated sequence number costs one call its attestation at
//! the seat (it degrades to the anonymous posture, never an error).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};

/// The machine-local file name inside the account dir (the same store
/// [`crate::client_id`] uses — one dir law).
pub const STATE_FILE: &str = "points_drain_state.json";

/// `BLAKE3(pubkey)` hex — the account id, in the identity substrate's
/// own derivation (`account_id_for`), reused here, never re-derived.
pub const ACCOUNT_HEADER: &str = "X-Points-Account";
/// The raw Ed25519 public key, 64-hex. The SAME header name the
/// points-plane signature headers use: a signed points-plane request and
/// an attested seat call never ride one HTTP request, so the shared name
/// cannot collide.
pub const PUBKEY_HEADER: &str = "X-Points-Pubkey";
/// The detached Ed25519 signature over
/// [`riir_ledger::attestation_signing_bytes`] — distinct from the
/// points-plane's `X-Points-Signature`.
pub const SIG_HEADER: &str = "X-Points-Sig";

/// The machine-local day state (0600, beside the client UUID). The file
/// is a CACHE, never a ledger: the seat's counters are the enforcement
/// truth, and every field is advisory UX input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayState {
    /// Local cache format (bump on a breaking reshape; the per-field
    /// serde defaults keep older files loading).
    #[serde(default)]
    pub format_version: u32,
    /// `YYYY-MM-DD` UTC — the day the counters belong to.
    #[serde(default)]
    pub utc_day: String,
    /// Free-allowance-leg hosted calls made today (the local mirror
    /// only — the seat counts the real thing).
    #[serde(default)]
    pub hosted_calls_today: u64,
    /// Billable-leg hosted calls made today, against the served
    /// `billable_ceiling_today`.
    #[serde(default)]
    pub billable_calls_today: u64,
    /// The per-process attestation sequence — incremented on each
    /// attested send, rolls with the UTC day.
    #[serde(default)]
    pub call_seq: u64,
    /// The last balance the points plane reported (`None` = never read).
    #[serde(default)]
    pub cached_balance: Option<u64>,
    /// The SERVED drain constants from that balance read (`None` = the
    /// gate is inert). The constants are never compiled.
    #[serde(default)]
    pub drain_state: Option<riir_ledger::DrainState>,
}

impl Default for DayState {
    fn default() -> Self {
        Self {
            format_version: 1,
            utc_day: String::new(),
            hosted_calls_today: 0,
            billable_calls_today: 0,
            call_seq: 0,
            cached_balance: None,
            drain_state: None,
        }
    }
}

/// The pre-check verdict. The allowed verdicts consumed one count of the
/// named leg; `Exhausted` consumed NOTHING (the caller keeps the
/// modelless fix it already has).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainVerdict {
    /// One free-allowance call granted (`hosted_calls_today` counted).
    AllowedFree,
    /// One billable call granted (`billable_calls_today` counted) — the
    /// cached balance covers the served fee and the local billable count
    /// is under the served ceiling.
    AllowedBillable,
    /// The free allowance is spent AND no billable leg is available.
    Exhausted,
}

/// The gate's decline marker: the day's runway is spent by the local
/// mirror. NOT an error — the caller returns
/// [`crate::escalation_dispatch::EscalatedFix::StoodByModelless`] (the
/// `DrainExhausted` reason) and serves the modelless fix it already has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainDeclined;

/// The per-call output of a gate pass: the attestation headers to attach
/// to the seat request — empty in every inert/anonymous posture (no key,
/// no constants) and one entry per name otherwise. Each name appears
/// EXACTLY once (the `kat_ua` law: a duplicate header name is a silent
/// comma-join and a 403 on conforming edges).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttestedCall {
    headers: Vec<(String, String)>,
}

impl AttestedCall {
    /// The no-headers posture (inert / anonymous / consent-skipped).
    fn plain() -> Self {
        Self {
            headers: Vec::new(),
        }
    }

    /// The header pairs for the ONE header-seam write: each entry is a
    /// distinct name, written once by the transport — never a second
    /// setter on a name already present.
    #[must_use]
    pub fn header_refs(&self) -> Vec<(&str, &str)> {
        self.headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    /// True when no headers were composed (the inert/anonymous posture).
    #[must_use]
    pub fn is_plain(&self) -> bool {
        self.headers.is_empty()
    }
}

/// Roll the day in place. Returns `true` when the day CHANGED (every
/// counter reset). `today` is the ASCII `YYYY-MM-DD` form — the compare
/// is allocation-free; only the reset writes a String.
#[must_use]
pub fn roll_day(state: &mut DayState, today: &str) -> bool {
    if state.utc_day == today {
        return false;
    }
    state.utc_day.clear();
    state.utc_day.push_str(today);
    state.hosted_calls_today = 0;
    state.billable_calls_today = 0;
    state.call_seq = 0;
    true
}

/// The pre-check's pure decision core (plan 326 T3): roll the day, then
/// TWO integer compares against the SERVED constants — the free
/// allowance first, then the billable tier (the cached balance covers
/// the served fee AND the local billable count is under the served
/// ceiling). The steady-state path (same day) performs no allocation;
/// the once-per-day roll writes the day string.
#[must_use]
pub fn allow_hosted(
    state: &mut DayState,
    drain: &riir_ledger::DrainState,
    today: &str,
) -> DrainVerdict {
    let _ = roll_day(state, today);
    if state.hosted_calls_today < drain.allowance_remaining_today {
        state.hosted_calls_today += 1;
        return DrainVerdict::AllowedFree;
    }
    if let Some(balance) = state.cached_balance {
        if balance >= drain.rate_served && state.billable_calls_today < drain.billable_ceiling_today
        {
            state.billable_calls_today += 1;
            return DrainVerdict::AllowedBillable;
        }
    }
    DrainVerdict::Exhausted
}

/// The cache-refresh seam (Proposal 027): any successful
/// `/points/balance` read lands the balance + the served drain constants
/// here. Best-effort — until something calls this (the plane's balance
/// verb), the gate stays inert, which is the pre-ratification posture by
/// design.
pub fn note_balance(dir: &Path, balance: u64, drain: Option<riir_ledger::DrainState>) {
    let mut state = load(dir);
    let today = utc_day_now();
    let _ = roll_day(&mut state, &today);
    state.cached_balance = Some(balance);
    state.drain_state = drain;
    save(dir, &state);
}

/// The state file path inside `dir` (the account-dir law — the same dir
/// [`crate::client_id`] stores in).
#[must_use]
pub fn state_path(dir: &Path) -> PathBuf {
    dir.join(STATE_FILE)
}

/// Load the day state; ANY failure (absent, unreadable, unparseable)
/// returns the default — the gate fails open, never on a corrupt cache.
#[must_use]
pub fn load(dir: &Path) -> DayState {
    std::fs::read(state_path(dir))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Best-effort persist (0600 on creation — the client-UUID store law).
/// Errors are swallowed BY DESIGN: the state is advisory UX input, so a
/// failed write degrades to an in-memory-only count, never a failed call.
fn save(dir: &Path, state: &DayState) {
    let Ok(json) = serde_json::to_vec(state) else {
        return;
    };
    let path = state_path(dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&path, json).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
    }
}

/// The dispatch-facing gate handle (plan 326 T3/T5). Constructed at arm
/// time, consulted per seat call: one [`DrainGate::begin_hosted_call`] =
/// the pre-check verdict + (when allowed and a key exists) the
/// attestation headers, in ONE pass over the state. Construction is
/// I/O-free by contract — the process posture resolves the account dir
/// and the account key lazily ONCE at the first consultation, so arming
/// a dispatch stays hermetic. Clones share the live state (an `Arc`).
#[derive(Clone)]
pub struct DrainGate {
    source: GateSource,
}

#[derive(Clone)]
enum GateSource {
    /// No account key (or nothing to consult): inert.
    Inert,
    /// The process account dir, resolved once on first use.
    Process,
    /// A directly-named dir (the `login_default_in` seam pattern — the
    /// hermetic posture for tests and future non-account stores).
    Live(Arc<GateInner>),
    /// Injected state — no disk, no account substrate (unit tests only).
    #[cfg(test)]
    Injected(Arc<Mutex<InjectedGate>>),
}

struct GateInner {
    dir: PathBuf,
    state: Mutex<DayState>,
    key: Option<ed25519_dalek::SigningKey>,
}

#[cfg(test)]
struct InjectedGate {
    state: DayState,
    key: Option<ed25519_dalek::SigningKey>,
}

impl DrainGate {
    /// The inert gate — everything allowed, nothing consumed (the
    /// explicit anonymous posture).
    #[must_use]
    pub fn inert() -> Self {
        Self {
            source: GateSource::Inert,
        }
    }

    /// The process gate: the account dir + its account key, resolved
    /// lazily ONCE on the first consultation (never at arm time).
    #[must_use]
    pub fn for_process() -> Self {
        Self {
            source: GateSource::Process,
        }
    }

    /// The process gate against an explicit dir (the seam posture).
    #[must_use]
    pub fn for_process_dir(dir: &Path) -> Self {
        Self {
            source: GateSource::Live(Arc::new(open_gate(dir))),
        }
    }

    /// The injected gate (unit tests): explicit state, optional key —
    /// no disk, no account substrate. Clones share the state (the
    /// dispatch-test probe pattern).
    #[cfg(test)]
    #[must_use]
    pub fn for_tests(state: DayState, key: Option<ed25519_dalek::SigningKey>) -> Self {
        Self {
            source: GateSource::Injected(Arc::new(Mutex::new(InjectedGate { state, key }))),
        }
    }

    /// A snapshot of the live day state (the test probe).
    #[cfg(test)]
    #[must_use]
    pub fn test_snapshot(&self) -> Option<DayState> {
        match &self.source {
            GateSource::Inert => None,
            GateSource::Process => Some(
                process_gate()
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone(),
            ),
            GateSource::Live(inner) => Some(
                inner
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone(),
            ),
            GateSource::Injected(inner) => Some(
                inner
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .state
                    .clone(),
            ),
        }
    }

    /// The ONE per-seat-call entry (plan 326 T3/T5): the pre-check
    /// verdict + the attestation headers.
    ///
    /// # Errors
    ///
    /// `Err(DrainDeclined)` = the polite decline — NOTHING was consumed
    /// and the caller serves the modelless fix it already has. Every
    /// inert posture answers `Ok` with plain (empty) headers; no other
    /// error class exists (the gate fails open on local faults).
    pub fn begin_hosted_call(&mut self, body: &[u8]) -> Result<AttestedCall, DrainDeclined> {
        match &self.source {
            GateSource::Inert => Ok(AttestedCall::plain()),
            GateSource::Process => gate_on_inner(process_gate(), body),
            GateSource::Live(inner) => gate_on_inner(inner, body),
            #[cfg(test)]
            GateSource::Injected(inner) => {
                let mut gate = inner
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let InjectedGate { state, key } = &mut *gate;
                gate_on_state(state, key.as_ref(), body, None)
            }
        }
    }
}

fn gate_on_inner(inner: &GateInner, body: &[u8]) -> Result<AttestedCall, DrainDeclined> {
    let Some(key) = inner.key.as_ref() else {
        // The anonymous posture: no key, no gate — the seat's per-source
        // budget governs unjoined callers and the client has no mirror
        // of it.
        return Ok(AttestedCall::plain());
    };
    let mut state = inner
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    gate_on_state(&mut state, Some(key), body, Some(&inner.dir))
}

fn gate_on_state(
    state: &mut DayState,
    key: Option<&ed25519_dalek::SigningKey>,
    body: &[u8],
    persist_dir: Option<&Path>,
) -> Result<AttestedCall, DrainDeclined> {
    // Today as stack bytes — the per-call date read allocates nothing.
    let mut day_buf = [0u8; 10];
    utc_day_bytes(&mut day_buf);
    let today: &str = std::str::from_utf8(&day_buf).unwrap_or("");
    // Absent served constants: INERT — byte-identical to the pre-drain
    // behavior (no roll, no persist, no headers, nothing consumed).
    let Some(served) = state.drain_state.as_ref() else {
        return Ok(AttestedCall::plain());
    };
    // The three constants are Copy integers — lifted onto the stack so
    // the mutable pass owns `state` alone.
    let constants = riir_ledger::DrainState {
        allowance_remaining_today: served.allowance_remaining_today,
        billable_ceiling_today: served.billable_ceiling_today,
        rate_served: served.rate_served,
    };
    if allow_hosted(state, &constants, today) == DrainVerdict::Exhausted {
        persist_best_effort(persist_dir, state);
        return Err(DrainDeclined);
    }
    let headers = match key {
        Some(key) => {
            let seq = state.call_seq;
            let headers = compose_attestation(key, today, seq, body);
            state.call_seq = seq.wrapping_add(1);
            headers
        }
        // A live constants cache with no key cannot occur in the process
        // posture (the gate requires the key); the injected posture uses
        // this arm to pin the anonymous rule.
        None => Vec::new(),
    };
    persist_best_effort(persist_dir, state);
    Ok(AttestedCall { headers })
}

fn persist_best_effort(dir: Option<&Path>, state: &DayState) {
    if let Some(dir) = dir {
        save(dir, state);
    }
}

/// The three attestation headers over the EXACT request body bytes
/// (plan 326 T5). Each name appears exactly once; the seat verifies
/// `blake3(pubkey) == account` and then the signature over
/// `attestation_signing_bytes(account_id, utc_day, call_seq,
/// blake3(body))`.
fn compose_attestation(
    key: &ed25519_dalek::SigningKey,
    utc_day: &str,
    call_seq: u64,
    body: &[u8],
) -> Vec<(String, String)> {
    use ed25519_dalek::Signer as _;
    let verifying = key.verifying_key();
    let pubkey: &[u8; 32] = verifying.as_bytes();
    let account_raw = *blake3::hash(pubkey).as_bytes();
    let body_hash = *blake3::hash(body).as_bytes();
    let signing_bytes =
        riir_ledger::attestation_signing_bytes(&account_raw, utc_day, call_seq, &body_hash);
    let sig = key.sign(&signing_bytes);
    vec![
        (
            ACCOUNT_HEADER.to_string(),
            riir_kat::kat_account::account_id_for(pubkey),
        ),
        (PUBKEY_HEADER.to_string(), hex(pubkey)),
        (SIG_HEADER.to_string(), hex(&sig.to_bytes())),
    ]
}

fn open_gate(dir: &Path) -> GateInner {
    GateInner {
        dir: dir.to_path_buf(),
        state: Mutex::new(load(dir)),
        // Best-effort, non-blocking: the file-key posture reads the key
        // file; an absent/unreadable key leaves the gate inert (the
        // anonymous posture). No prompt path exists in the postures this
        // crate enables.
        key: riir_kat::kat_account::KatAccount::load(dir)
            .ok()
            .and_then(|account| account.signing_key().ok()),
    }
}

fn process_gate() -> &'static GateInner {
    static PROCESS: OnceLock<GateInner> = OnceLock::new();
    PROCESS.get_or_init(|| {
        let dir = riir_kat::kat_account::KatAccount::default_dir();
        open_gate(&dir)
    })
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Today as ASCII `YYYY-MM-DD` written into a 10-byte buffer — the
/// per-call day read, allocation-free. A pre-epoch clock clamps to the
/// epoch day (the proposal's "wrong clock skews the LOCAL pre-check"
/// bound; the seat's server clock is the enforcement truth).
fn utc_day_bytes(out: &mut [u8; 10]) {
    // The epoch clamp bounds the day math; a clock past i64 seconds is
    // beyond any plausible box (the day string is UX input, never sync
    // truth).
    #[allow(clippy::cast_possible_wrap)]
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    write_day(secs.div_euclid(86_400), out);
}

/// Today as a `String` (the shell paths: `note_balance`, tests).
///
/// # Panics
///
/// Never — the day form is pure ASCII by construction; the `expect`
/// documents that invariant rather than a reachable panic.
#[must_use]
pub fn utc_day_now() -> String {
    let mut buf = [0u8; 10];
    utc_day_bytes(&mut buf);
    String::from_utf8(buf.to_vec()).expect("the day form is pure ASCII")
}

/// Days since the Unix epoch → (year, month, day). Howard Hinnant's
/// `civil_from_days` (public domain) — the no-date-library form; the
/// input is clamped at the epoch so a pre-1970 clock reads
/// `1970-01-01` rather than wrapping.
fn write_day(days_since_epoch: i64, out: &mut [u8; 10]) {
    let days = days_since_epoch.max(0);
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    // The epoch clamp + the digit extraction bound every value: y ∈
    // [1970, ∞), m/d ∈ [1, 31], each digit read mod 10 — every cast is
    // total for any clock this side of year 4 × 10⁹; a nonsense clock
    // renders a nonsense-but-safe day string (UX input only; the seat's
    // clock enforces).
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    {
        let (y, m, d): (u32, u32, u32) = (y as u32, m as u32, d as u32);
        out[0] = b'0' + ((y / 1_000) % 10) as u8;
        out[1] = b'0' + ((y / 100) % 10) as u8;
        out[2] = b'0' + ((y / 10) % 10) as u8;
        out[3] = b'0' + (y % 10) as u8;
        out[4] = b'-';
        out[5] = b'0' + (m / 10) as u8;
        out[6] = b'0' + (m % 10) as u8;
        out[7] = b'-';
        out[8] = b'0' + (d / 10) as u8;
        out[9] = b'0' + (d % 10) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn served(allowance: u64, ceiling: u64, rate: u64) -> riir_ledger::DrainState {
        riir_ledger::DrainState {
            allowance_remaining_today: allowance,
            billable_ceiling_today: ceiling,
            rate_served: rate,
        }
    }

    fn test_key() -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[7u8; 32])
    }

    fn tmp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "refine_points_drain_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const TODAY: &str = "2026-11-02";

    #[test]
    fn the_day_form_pins_the_three_anchor_dates() {
        let render = |days: i64| {
            let mut buf = [0u8; 10];
            write_day(days, &mut buf);
            String::from_utf8(buf.to_vec()).unwrap()
        };
        assert_eq!(render(0), "1970-01-01");
        assert_eq!(render(19_723), "2024-01-01");
        assert_eq!(render(20_759), "2026-11-02");
        // today's buffer form agrees with the String form
        let mut buf = [0u8; 10];
        utc_day_bytes(&mut buf);
        assert_eq!(&buf, utc_day_now().as_bytes());
    }

    #[test]
    fn the_state_round_trips_and_fails_open_on_corruption() {
        let dir = tmp_dir();
        let state = DayState {
            utc_day: TODAY.to_string(),
            hosted_calls_today: 3,
            call_seq: 9,
            cached_balance: Some(42),
            drain_state: Some(served(10, 20, 2)),
            ..DayState::default()
        };
        save(&dir, &state);
        assert_eq!(load(&dir), state, "the persisted state loads verbatim");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(state_path(&dir))
                .expect("state file exists")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "the store keeps the 0600 law");
        }
        std::fs::write(state_path(&dir), b"not json").unwrap();
        assert_eq!(
            load(&dir),
            DayState::default(),
            "a corrupt cache fails open to the default"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_day_rollover_resets_every_counter() {
        let mut state = DayState {
            utc_day: "2026-11-01".to_string(),
            hosted_calls_today: 7,
            billable_calls_today: 3,
            call_seq: 11,
            ..DayState::default()
        };
        assert!(roll_day(&mut state, TODAY), "the day changed");
        assert_eq!(
            (
                state.hosted_calls_today,
                state.billable_calls_today,
                state.call_seq
            ),
            (0, 0, 0),
            "every counter resets with the day"
        );
        assert_eq!(state.utc_day, TODAY);
        assert!(!roll_day(&mut state, TODAY), "same day is a no-op");
    }

    #[test]
    fn the_two_compares_walk_the_three_branches() {
        let drain = served(2, 5, 2);
        // free leg first
        let mut state = DayState::default();
        assert_eq!(
            allow_hosted(&mut state, &drain, TODAY),
            DrainVerdict::AllowedFree
        );
        assert_eq!(state.hosted_calls_today, 1);
        assert_eq!(
            allow_hosted(&mut state, &drain, TODAY),
            DrainVerdict::AllowedFree
        );
        assert_eq!(
            allow_hosted(&mut state, &drain, TODAY),
            DrainVerdict::Exhausted,
            "the free allowance is spent and no balance is cached"
        );
        // the billable leg (a cached balance at/above the fee)
        state.cached_balance = Some(2);
        assert_eq!(
            allow_hosted(&mut state, &drain, TODAY),
            DrainVerdict::AllowedBillable
        );
        assert_eq!(state.billable_calls_today, 1);
        for _ in 1..5 {
            assert_eq!(
                allow_hosted(&mut state, &drain, TODAY),
                DrainVerdict::AllowedBillable
            );
        }
        assert_eq!(
            allow_hosted(&mut state, &drain, TODAY),
            DrainVerdict::Exhausted,
            "the billable ceiling binds regardless of balance"
        );
        // a balance BELOW the fee never bills (the free leg already spent)
        let mut poor = DayState {
            utc_day: TODAY.to_string(),
            hosted_calls_today: 2,
            cached_balance: Some(1),
            ..DayState::default()
        };
        assert_eq!(
            allow_hosted(&mut poor, &drain, TODAY),
            DrainVerdict::Exhausted
        );
        // a rollover mid-sequence re-rolls everything
        assert_eq!(
            allow_hosted(&mut state, &drain, "2026-11-03"),
            DrainVerdict::AllowedFree
        );
        assert_eq!(
            (state.hosted_calls_today, state.billable_calls_today),
            (1, 0),
            "the new day starts from zero"
        );
    }

    #[test]
    fn note_balance_writes_the_constants_the_gate_consumes() {
        let dir = tmp_dir();
        note_balance(&dir, 42, Some(served(10, 20, 2)));
        let mut state = load(&dir);
        assert_eq!(state.cached_balance, Some(42));
        assert_eq!(
            state
                .drain_state
                .as_ref()
                .map(|d| d.allowance_remaining_today),
            Some(10)
        );
        // the gate consumes the constants without further wiring
        let drain = state.drain_state.clone().expect("constants cached");
        let today = state.utc_day.clone();
        assert_eq!(
            allow_hosted(&mut state, &drain, &today),
            DrainVerdict::AllowedFree,
            "the gate consumes the cached constants without further wiring"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_gate_counts_allowed_calls_and_declines_exhausted() {
        let mut gate = DrainGate::for_tests(
            DayState {
                cached_balance: Some(1),
                drain_state: Some(served(1, 5, 2)),
                ..DayState::default()
            },
            Some(test_key()),
        );
        // the one free call: consumes the free leg + one sequence number
        let call = gate.begin_hosted_call(b"body-one").expect("allowed");
        assert!(!call.is_plain(), "a keyed, allowed call composes headers");
        let snap = gate.test_snapshot().expect("injected state");
        assert_eq!(
            (snap.hosted_calls_today, snap.call_seq),
            (1, 1),
            "the free leg and the attestation seq both count"
        );
        // free spent, balance below the fee: the polite decline
        assert_eq!(
            gate.begin_hosted_call(b"body-two"),
            Err(DrainDeclined),
            "exhausted declines locally"
        );
        let snap = gate.test_snapshot().expect("injected state");
        assert_eq!(
            (
                snap.hosted_calls_today,
                snap.billable_calls_today,
                snap.call_seq
            ),
            (1, 0, 1),
            "a decline consumes NOTHING (no count, no seq)"
        );
    }

    #[test]
    fn the_inert_postures_consume_nothing_and_touch_nothing() {
        // constants absent: inert even with a key
        let mut gate = DrainGate::for_tests(DayState::default(), Some(test_key()));
        assert!(gate.begin_hosted_call(b"x").expect("allowed").is_plain());
        let snap = gate.test_snapshot().expect("injected state");
        assert_eq!(
            (snap.hosted_calls_today, snap.call_seq),
            (0, 0),
            "inert consumes nothing"
        );
        // the explicit inert handle
        let mut gate = DrainGate::inert();
        assert!(gate.begin_hosted_call(b"x").expect("allowed").is_plain());
        // the process-shaped gate on an empty dir: no key, no constants,
        // and NO state file ever created (the flag-off law, mirrored at
        // the inert posture; with points_drain OFF none of this code
        // compiles at all — byte-identical by construction)
        let dir = tmp_dir();
        let mut gate = DrainGate::for_process_dir(&dir);
        assert!(gate.begin_hosted_call(b"x").expect("allowed").is_plain());
        assert!(
            !state_path(&dir).exists(),
            "an inert gate never creates the state file"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_attestation_headers_verify_against_the_exact_body() {
        use ed25519_dalek::Verifier as _;
        let key = test_key();
        let mut gate = DrainGate::for_tests(
            DayState {
                drain_state: Some(served(10, 10, 2)),
                ..DayState::default()
            },
            Some(key.clone()),
        );
        let body = b"{\"suite\":\"heal\",\"state\":\"fn foo(){}\",\"options\":[\"a\",\"b\"]}";
        let call = gate.begin_hosted_call(body).expect("allowed");
        let headers = call.header_refs();
        let names: Vec<&str> = headers.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            names,
            [ACCOUNT_HEADER, PUBKEY_HEADER, SIG_HEADER],
            "each name EXACTLY once (the kat_ua law)"
        );
        let account = &headers
            .iter()
            .find(|(k, _)| *k == ACCOUNT_HEADER)
            .expect("a")
            .1;
        let pubkey_hex = &headers
            .iter()
            .find(|(k, _)| *k == PUBKEY_HEADER)
            .expect("p")
            .1;
        let sig_hex = &headers.iter().find(|(k, _)| *k == SIG_HEADER).expect("s").1;
        // the account id IS the substrate's own BLAKE3(pubkey) derivation
        assert_eq!(
            account,
            &riir_kat::kat_account::account_id_for(key.verifying_key().as_bytes())
        );
        assert_eq!(pubkey_hex, &hex(key.verifying_key().as_bytes()));
        // the seat-side verification flow, over the RECEIVED body
        let mut sig_bytes = [0u8; 64];
        for (i, pair) in sig_hex.as_bytes().chunks(2).enumerate() {
            sig_bytes[i] = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap();
        }
        let account_raw = *blake3::hash(key.verifying_key().as_bytes()).as_bytes();
        let body_hash = *blake3::hash(body).as_bytes();
        let snap = gate.test_snapshot().expect("injected state");
        let signing =
            riir_ledger::attestation_signing_bytes(&account_raw, &snap.utc_day, 0, &body_hash);
        key.verifying_key()
            .verify(&signing, &ed25519_dalek::Signature::from_bytes(&sig_bytes))
            .expect("the seat-side verification must pass");
        // the signature binds the body hash — a flipped body is rejected
        let other_body =
            riir_ledger::attestation_signing_bytes(&account_raw, &snap.utc_day, 0, &[0u8; 32]);
        assert!(
            key.verifying_key()
                .verify(
                    &other_body,
                    &ed25519_dalek::Signature::from_bytes(&sig_bytes)
                )
                .is_err(),
            "the signature binds the exact body bytes"
        );
        assert_eq!(snap.call_seq, 1, "the seq advanced for the next send");
    }

    #[test]
    fn the_precheck_path_stays_well_under_the_millisecond_bar() {
        // G2 sanity (plan 326 T6g): the bar is trivially met by
        // construction (integer compares); measured anyway with the
        // FULL wired path (constants + key + signing) as the worst case.
        let mut gate = DrainGate::for_tests(
            DayState {
                cached_balance: Some(1_000_000),
                drain_state: Some(served(u64::MAX / 2, u64::MAX / 2, 2)),
                ..DayState::default()
            },
            Some(test_key()),
        );
        let body = b"body";
        let _ = gate.begin_hosted_call(body); // warmup: the day roll
        let started = std::time::Instant::now();
        let n = 2_000u32;
        for _ in 0..n {
            let _ = gate.begin_hosted_call(body).expect("allowed");
        }
        let per_call = started.elapsed() / n;
        assert!(
            per_call < std::time::Duration::from_millis(1),
            "the pre-check per-call cost {per_call:?} must stay under the 1 ms bar"
        );
    }
}
