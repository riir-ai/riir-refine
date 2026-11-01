//! The first-run consent (Plan 325 T2; the exact UI words are T16.1 §5 of
//! `.docs/01_orientation/points_privacy_id_design.md` — the ONE home; this
//! module renders them verbatim and the charitable gate pins the sentence).
//!
//! State machine (machine-local, in the account dir beside the other
//! consent records): `Unset` → the first `--fix` that could ever send asks
//! ONCE — **join** (run the join flow), **anonymous** (share secret-blind
//! under the client UUID), **never** (the persistent offline posture).
//! Every send lane consults [`Answer::send_allowed`]; `Unset` means DO NOT
//! SEND (the safe direction by construction — the stats-consent precedent).
//!
//! Non-interactive runs never nag and never answer: `Unset` stays, the
//! sends stay off, the run proceeds (local healing is unconditional). A TTY
//! prompt honors `--yes` / `RIIR_REFINE_YES=1` by taking the DEFAULT
//! answer — **anonymous** (§13.2: "the default is anonymous"), never join
//! (join is an explicit act — it runs `cargo refine join`).
//!
//! The vocabulary law (GAP 10) holds in every rendered word: contribute /
//! contribution / contributor / contribution points — never mining.

use std::path::{Path, PathBuf};

/// The consent record's file name inside the account dir.
pub const CONSENT_FILE: &str = "first_run_consent.json";

/// The stored answer's version (bump on any field change; an
/// unreadable/future record reads [`Answer::Unset`]).
pub const CONSENT_RECORD_VERSION: u32 = 1;

/// The consent screen — the T16.1 §5 words VERBATIM, one array entry per
/// rendered line. The charitable gate's positive assertion pins the two
/// load-bearing sentences by exact substring (T6); editing a line here is
/// a consent-text amendment (privacy note §8 — owner gate).
pub const CONSENT_SCREEN_LINES: &[&str] = &[
    "Does cargo refine ever send my code anywhere?",
    "Local healing never touches the network. Sharing is opt-in and",
    "secret-blind: the redaction layer never lets credentials or secrets",
    "leave your machine, and what is shared is fix metadata — rule",
    "verdicts and the code spans each fix touched — never whole files.",
    "Your install is identified by a random ID we generate on your",
    "machine; we never read your hardware, your hostname, or your",
    "network. You can share anonymously, join with GitHub to get",
    "contribution credit and a leaderboard rank, or run fully offline",
    "(RIIR_REFINE_OFFLINE=1). Leave any time — cargo refine leave —",
    "and the tool is identical locally either way.",
];

/// The three-way answer plus the unset state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Never asked (or an unreadable record) — DO NOT SEND.
    Unset,
    /// Share secret-blind under the client UUID; no identity.
    Anonymous,
    /// The user ran `join` — contribution credit + rank eligibility.
    Joined,
    /// The user chose never-send — the persistent offline posture.
    Never,
}

impl Answer {
    /// The wire/storage token.
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Answer::Unset => "unset",
            Answer::Anonymous => "anonymous",
            Answer::Joined => "joined",
            Answer::Never => "never",
        }
    }

    /// Parse the stored token; an unknown token is `Unset` (safe direction).
    #[must_use]
    pub fn parse(token: &str) -> Self {
        match token {
            "anonymous" => Answer::Anonymous,
            "joined" => Answer::Joined,
            "never" => Answer::Never,
            _ => Answer::Unset,
        }
    }

    /// May a send lane send? Only an explicit yes-state allows it.
    #[must_use]
    pub fn send_allowed(self) -> bool {
        matches!(self, Answer::Anonymous | Answer::Joined)
    }
}

/// The on-disk record (serde; the account dir may not exist yet).
#[derive(serde::Serialize, serde::Deserialize)]
struct ConsentRecord {
    version: u32,
    answer: String,
    /// Unix seconds of the answer — provenance only, never a gate.
    answered_unix: u64,
}

/// The full path of the consent record inside `dir`.
#[must_use]
pub fn consent_path(dir: &Path) -> PathBuf {
    dir.join(CONSENT_FILE)
}

/// Read the stored answer. Missing/unreadable/malformed/future → `Unset`.
#[must_use]
pub fn read(dir: &Path) -> Answer {
    let Ok(text) = std::fs::read_to_string(consent_path(dir)) else {
        return Answer::Unset;
    };
    let Ok(record) = serde_json::from_str::<ConsentRecord>(&text) else {
        return Answer::Unset;
    };
    if record.version != CONSENT_RECORD_VERSION {
        return Answer::Unset;
    }
    Answer::parse(&record.answer)
}

/// Persist an answer (the ask and the join/leave flows write through here).
///
/// # Errors
/// The dir could not be created or the file could not be written.
pub fn write(dir: &Path, answer: Answer) -> Result<(), String> {
    let path = consent_path(dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("consent: creating {}: {e}", parent.display()))?;
    }
    let record = ConsentRecord {
        version: CONSENT_RECORD_VERSION,
        answer: answer.token().to_string(),
        answered_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("consent: writing {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Print the consent screen (the [`CONSENT_SCREEN_LINES`] verbatim) + the
/// three-way ask, and return the answer.
///
/// `yes_armed` is the run-global `--yes` / `RIIR_REFINE_YES=1` state — the
/// default answer ([`Answer::Anonymous`]) is taken without prompting.
/// Returns `Unset` when no prompt was possible (non-interactive) — the
/// caller proceeds without sending.
#[must_use]
pub fn ask_once(yes_armed: bool) -> Answer {
    for line in CONSENT_SCREEN_LINES {
        println!("{line}");
    }
    println!();
    println!("Share fix metadata (secret-blind, contributes to the open corpus)?");
    println!("  [a] anonymously — share under the random ID (the default)");
    println!("  [j] join — also link GitHub now (contribution credit + rank)");
    println!("  [n] never — keep everything local (also: RIIR_REFINE_OFFLINE=1)");
    let answer = if crate::offline::offline() {
        // The env already speaks for the user; the ask would be noise.
        Answer::Never
    } else if yes_armed {
        Answer::Anonymous
    } else if !stdin_is_tty() {
        // Non-interactive: never guess. Unset keeps every send off; the
        // user answers at a terminal, or via `join` / the env.
        Answer::Unset
    } else {
        prompt_loop()
    };
    println!();
    answer
}

fn prompt_loop() -> Answer {
    for _ in 0..3 {
        eprint!("answer [a/j/n] (default a): ");
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            return Answer::Unset;
        }
        match line.trim().to_lowercase().as_str() {
            "a" | "" => return Answer::Anonymous,
            "j" => return Answer::Joined,
            "n" => return Answer::Never,
            other => eprintln!("unrecognized {other:?} — a, j or n"),
        }
    }
    Answer::Unset
}

fn stdin_is_tty() -> bool {
    // std has no isatty; the cheap unix probe opens /dev/tty.
    #[cfg(unix)]
    {
        std::fs::File::open("/dev/tty").is_ok()
    }
    #[cfg(not(unix))]
    {
        // Windows: prompt only when the session looks interactive;
        // otherwise Unset (the safe direction).
        std::env::var_os("SESSIONNAME").is_some() || std::env::var_os("PROMPT").is_some()
    }
}

/// The one-line status the fix path prints (after the ask, or when the
/// state exists) — the stats-consent status-line shape.
#[must_use]
pub fn status_line(answer: Answer) -> &'static str {
    match answer {
        Answer::Unset => {
            "consent: not asked yet — fix metadata is NOT shared (answer at a terminal, or `cargo refine join`)"
        }
        Answer::Anonymous => {
            "consent: anonymous — fix metadata is shared secret-blind under the random ID"
        }
        Answer::Joined => {
            "consent: joined — fix metadata is shared secret-blind; contribution credit on"
        }
        Answer::Never => {
            "consent: never — nothing is ever sent (RIIR_REFINE_OFFLINE=1 is the per-run form)"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "refine_first_run_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn unset_by_default_and_send_safe() {
        let dir = tmp_dir();
        assert_eq!(read(&dir), Answer::Unset);
        assert!(!Answer::Unset.send_allowed());
        assert!(!Answer::Never.send_allowed());
        assert!(Answer::Anonymous.send_allowed());
        assert!(Answer::Joined.send_allowed());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn round_trips_and_refuses_future_versions() {
        let dir = tmp_dir();
        write(&dir, Answer::Anonymous).unwrap();
        assert_eq!(read(&dir), Answer::Anonymous);
        let path = consent_path(&dir);
        let text = std::fs::read_to_string(&path).unwrap();
        let bumped = text.replace(
            &format!("\"version\": {CONSENT_RECORD_VERSION}"),
            &format!("\"version\": {}", CONSENT_RECORD_VERSION + 1),
        );
        std::fs::write(&path, bumped).unwrap();
        assert_eq!(read(&dir), Answer::Unset);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_is_closed_over_the_vocabulary() {
        assert_eq!(Answer::parse("joined"), Answer::Joined);
        assert_eq!(Answer::parse("garbage"), Answer::Unset);
    }

    #[test]
    fn consent_screen_says_the_law_and_never_the_banned_words() {
        let screen = CONSENT_SCREEN_LINES.join("\n");
        // The T16.1 §5 load-bearing sentences, verbatim.
        assert!(screen.contains("a random ID we generate on your"));
        assert!(screen.contains("we never read your hardware, your hostname, or your"));
        assert!(screen.contains("cargo refine leave"));
        assert!(screen.contains("Local healing never touches the network"));
        // The vocabulary law (GAP 10).
        for banned in ["mining", " mine", "miner", "KAT"] {
            assert!(
                !screen.to_lowercase().contains(&banned.to_lowercase()),
                "{banned}"
            );
        }
    }
}
