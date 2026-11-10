//! The per-repo consent scope — the hosted fix path's THIRD consent
//! surface (Plan 333 T4; the memo is
//! `.docs/01_orientation/fixer_leak_posture_signoff_memo.md`, item 2 + 5).
//!
//! Scope law (the `code_egress` shape): a consent that means two things
//! means neither, so this surface has its OWN record, keyed per repo,
//! answering exactly one question — may THIS repo's fix spans go to the
//! hosted fixer lane. It rides ON TOP of `RIIR_REFINE_CODE_EGRESS`
//! (never instead of it): the env stays the per-run egress master; the
//! per-repo record adds the durable, per-repo answer the memo's
//! "interactive prompt asks once per repo and persists the choice"
//! (memo item 2) requires.
//!
//! State machine (machine-local, in the account dir, one record per
//! repo fingerprint): `Unset` → the first hosted-fix escalation in that
//! repo asks ONCE (the memo item-5 words, pinned below) and the answer
//! persists. Every send lane consults [`consult`]; the safe direction is
//! by construction — `Unset` and non-interactive runs refuse, never
//! guess (the first-run-consent precedent). The memo's three options map
//! verbatim: *ask once per this repo* persists [`RepoAnswer::Ask`] (the
//! next escalation in this repo shows the one-time allow/never confirm
//! and persists THAT), *always* persists [`RepoAnswer::Allowed`],
//! *never* persists [`RepoAnswer::Refused`].
//!
//! ## The ENABLE flip (the T4 "opt-in-dark" law)
//!
//! [`ENABLED`] is `false` in every build until Plan 333 Phase B: the
//! wiring may land dark (this module + the dispatch consults), the ENABLE
//! flip may not. While `false`, `consult` returns [`RepoConsentDecision::Disabled`]
//! WITHOUT reading the record and WITHOUT ever prompting — the shipped
//! behavior is byte-identical to pre-T4. Flipping the const is an owner
//! act (the memo signature IS the gate) and lands with the pin test
//! updated in the same commit.
//!
//! The vocabulary law holds in every rendered word: this surface never
//! says mining; the disclosure speaks of "our LLM provider" per the memo.

use std::path::{Path, PathBuf};

/// The ENABLE flip — `false` until the Plan 333 Phase B owner signature
/// (memo §7). Every consult short-circuits on `false`: no disk read, no
/// prompt, byte-identical shipped behavior. The pin test
/// (`tests/repo_consent_gates.rs`) asserts `false` and REDS on a flip —
/// flipping `ENABLED` without touching that test is a boundary violation.
pub const ENABLED: bool = false;

/// The consent record's directory name inside the account dir.
pub const REPO_CONSENT_DIR: &str = "repo_consent";

/// The stored record's version (bump on any field change; an
/// unreadable/future record reads [`RepoAnswer::Unset`]).
pub const RECORD_VERSION: u32 = 1;

/// The consumer-facing disclosure — the memo item-5 words VERBATIM (the
/// `≤ N lines` placeholder is resolved at render time with the actual
/// snippet line count; every other byte is pinned by
/// `tests/repo_consent_gates.rs` against the memo — a change to any of
/// these lines reopens the memo, per its own item 2).
pub const DISCLOSURE_PREFIX: &str = "This fix sends the code snippet (";
pub const DISCLOSURE_MIDDLE: &str = " lines, the span plus minimal context) to our LLM provider (Cloudflare Workers AI) over an encrypted connection. Their terms commit that it is not used to train their models and is not retained after the response. A local secret scanner runs first; anything with a finding never leaves your machine. Everything else — the scan, the draft, the verify — stays local.";
/// The option row labels, in the memo's bracket order.
pub const OPTION_LABELS: &[&str] = &["ask once per this repo", "always", "never"];

/// The exact refusal vocabulary (printed, and carried in the dispatch's
/// `Declined` reason, when this surface blocks a send).
pub const REFUSAL_NEVER: &str = "repo consent: this repo is set to never send code off-machine (delete repo_consent/<repo>.json in the account dir to re-ask)";
pub const REFUSAL_UNSET: &str = "repo consent: not asked yet for this repo — refusing (the hosted fix path asks once per repo; RIIR_REFINE_CODE_EGRESS=snippet still governs the egress scope)";

/// The per-repo answer. `Unset` means DO NOT SEND (the safe direction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepoAnswer {
    /// Never asked (or an unreadable record) — DO NOT SEND.
    Unset,
    /// The user chose "ask once per this repo" — the next escalation in
    /// this repo shows the one-time allow/never confirm.
    Ask,
    /// This repo may use the hosted fix path without further asks.
    Allowed,
    /// This repo never sends — the persistent per-repo refusal.
    Refused,
}

impl RepoAnswer {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            RepoAnswer::Unset => "unset",
            RepoAnswer::Ask => "ask-once-armed",
            RepoAnswer::Allowed => "always",
            RepoAnswer::Refused => "never",
        }
    }
}

/// The stored record's fields (versioned JSON via `serde_json::json!`;
/// `answer` spellings: "ask" | "allowed" | "refused" — anything else, or
/// a future version, reads Unset). No dedicated struct: the record has
/// three fields and the safe-read direction is the version check.
/// The dispatch-consumable decision (the `EgressConsent` shape — the
/// `_checked_with` cores take this injected, never a process env).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepoConsentDecision {
    /// [`ENABLED`] is `false` — no gate, no read, no prompt. The shipped
    /// posture; behaviorally identical to pre-T4.
    Disabled,
    /// The consult allows (record `Allowed`; `Ask` consumed by the ask).
    Allowed,
    /// The consult refuses — carries the exact refusal vocabulary.
    Refused(&'static str),
}

impl RepoConsentDecision {
    /// The dark-posture identity every existing caller passes until the
    /// flip (and arm H) lands.
    #[must_use]
    pub fn disabled() -> Self {
        RepoConsentDecision::Disabled
    }
}

/// The per-repo fingerprint: blake3 hex of the canonicalized repo root.
/// A moved repo re-asks (the fingerprint follows the path, by design).
#[must_use]
pub fn repo_fingerprint(repo_root: &Path) -> String {
    let canonical = std::fs::canonicalize(repo_root).unwrap_or_else(|_| repo_root.to_path_buf());
    let digest = blake3::Hasher::new()
        .update(canonical.to_string_lossy().as_bytes())
        .finalize();
    digest.to_hex().to_string()
}

/// The record's file path for one repo inside the account dir.
#[must_use]
pub fn repo_consent_path(dir: &Path, repo_root: &Path) -> PathBuf {
    dir.join(REPO_CONSENT_DIR)
        .join(format!("{}.json", repo_fingerprint(repo_root)))
}

/// Read one repo's record. Any problem — missing file, unparsable JSON,
/// future version — reads [`RepoAnswer::Unset`] (the safe direction; the
/// first-run-consent convention).
#[must_use]
pub fn read(dir: &Path, repo_root: &Path) -> RepoAnswer {
    let path = repo_consent_path(dir, repo_root);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return RepoAnswer::Unset;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return RepoAnswer::Unset;
    };
    if v.get("version").and_then(serde_json::Value::as_u64) != Some(u64::from(RECORD_VERSION)) {
        return RepoAnswer::Unset;
    }
    match v.get("answer").and_then(|x| x.as_str()) {
        Some("ask") => RepoAnswer::Ask,
        Some("allowed") => RepoAnswer::Allowed,
        Some("refused") => RepoAnswer::Refused,
        _ => RepoAnswer::Unset,
    }
}

/// Persist one repo's answer (creates the directory; best-effort — an
/// unwritable account dir fails the SEND at the next consult's Unset,
/// never the local run).
///
/// # Errors
///
/// Directory creation or the file write fails (the account dir unwritable).
pub fn write(dir: &Path, repo_root: &Path, answer: RepoAnswer) -> Result<(), String> {
    let path = repo_consent_path(dir, repo_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("repo consent dir: {e}"))?;
    }
    let text = serde_json::to_string_pretty(&serde_json::json!({
        "version": RECORD_VERSION,
        "answer": match answer {
            RepoAnswer::Unset => "unset",
            RepoAnswer::Ask => "ask",
            RepoAnswer::Allowed => "allowed",
            RepoAnswer::Refused => "refused",
        },
        "repo_root": repo_root.display().to_string(),
    }))
    .map_err(|e| format!("repo consent: {e}"))?;
    std::fs::write(&path, text).map_err(|e| format!("repo consent write: {e}"))
}

/// Render the ask: the disclosure with the snippet line count resolved,
/// then the memo's option row. The prompt LOOP and the persistence live
/// in [`ask_once`] — this is the pure renderer so the pin test can hold
/// the words without a TTY.
#[must_use]
pub fn render_prompt_lines(snippet_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "{DISCLOSURE_PREFIX}≤ {snippet_lines}{DISCLOSURE_MIDDLE}"
    ));
    lines.push(String::new());
    lines.push(format!(
        "  [o] {} (the next escalation in this repo confirms once)",
        OPTION_LABELS[0]
    ));
    lines.push(format!(
        "  [a] {} — no further asks for this repo",
        OPTION_LABELS[1]
    ));
    lines.push(format!(
        "  [n] {} — this repo never sends",
        OPTION_LABELS[2]
    ));
    lines
}

fn stdin_is_tty() -> bool {
    // The same probe shape as first_run_consent (the /dev/tty open) —
    // kept inline because first_run_consent is points-gated and this
    // module is ungated; a behavior change in one is a change in both.
    #[cfg(unix)]
    {
        std::fs::File::open("/dev/tty").is_ok()
    }
    #[cfg(not(unix))]
    {
        std::env::var_os("SESSIONNAME").is_some() || std::env::var_os("PROMPT").is_some()
    }
}

/// The once-per-repo ask: renders the memo words, reads the answer,
/// persists, returns the resulting decision. Non-interactive (`--yes`
/// armed or not): NEVER guesses — Unset stays, the send refuses (the
/// first-run-consent law; `--yes` takes no default here because the safe
/// answer is per-repo durable, unlike the stats plane's anonymous
/// default — a silent yes would write an Allowed record the user never
/// saw a prompt for).
#[must_use]
pub fn ask_once(dir: &Path, repo_root: &Path, snippet_lines: usize, yes_armed: bool) -> RepoAnswer {
    let _ = yes_armed;
    if !stdin_is_tty() {
        return RepoAnswer::Unset;
    }
    for line in render_prompt_lines(snippet_lines) {
        println!("{line}");
    }
    for _ in 0..3 {
        eprint!("answer [o/a/n]: ");
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            return RepoAnswer::Unset;
        }
        match line.trim() {
            "o" => {
                let _ = write(dir, repo_root, RepoAnswer::Ask);
                return RepoAnswer::Ask;
            }
            "a" => {
                let _ = write(dir, repo_root, RepoAnswer::Allowed);
                return RepoAnswer::Allowed;
            }
            "n" => {
                let _ = write(dir, repo_root, RepoAnswer::Refused);
                return RepoAnswer::Refused;
            }
            _ => {}
        }
    }
    RepoAnswer::Unset
}

/// The dispatch consult: record-or-ask, then the decision. While
/// [`ENABLED`] is `false` this is [`RepoConsentDecision::Disabled`] with
/// zero side effects — no record read, no prompt, no file.
#[must_use]
pub fn consult(
    dir: &Path,
    repo_root: &Path,
    snippet_lines: usize,
    yes_armed: bool,
) -> RepoConsentDecision {
    if !ENABLED {
        return RepoConsentDecision::Disabled;
    }
    match read(dir, repo_root) {
        RepoAnswer::Allowed => RepoConsentDecision::Allowed,
        RepoAnswer::Refused => RepoConsentDecision::Refused(REFUSAL_NEVER),
        RepoAnswer::Unset => {
            let asked = ask_once(dir, repo_root, snippet_lines, yes_armed);
            match asked {
                RepoAnswer::Refused => RepoConsentDecision::Refused(REFUSAL_NEVER),
                // `Allowed` (unreachable here — the arm holds Allowed) and
                // `Ask` (the armed mode: the just-given answer proceeds
                // this call; a later run re-asks once) both allow.
                RepoAnswer::Allowed | RepoAnswer::Ask => RepoConsentDecision::Allowed,
                RepoAnswer::Unset => RepoConsentDecision::Refused(REFUSAL_UNSET),
            }
        }
        // The armed mode reached at a LATER run: the one-time confirm.
        RepoAnswer::Ask => {
            let asked = ask_once(dir, repo_root, snippet_lines, yes_armed);
            match asked {
                RepoAnswer::Allowed => RepoConsentDecision::Allowed,
                RepoAnswer::Refused => RepoConsentDecision::Refused(REFUSAL_NEVER),
                _ => RepoConsentDecision::Refused(REFUSAL_UNSET),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_is_the_default_and_never_reads_disk() {
        // The deliberate tripwire: reds LOUD at the flip, forcing the
        // flipper to touch this pin (clippy's const-assert note is met
        // by the const-block form).
        const {
            assert!(
                !ENABLED,
                "the ENABLE flip is an owner act (plan 333 Phase B) — flip this pin in the same commit as the signature"
            )
        };
        let tmp = std::env::temp_dir().join(format!("repo_consent_dark_{}", std::process::id()));
        let dir = tmp.join("account");
        let repo = tmp.join("repo");
        std::fs::create_dir_all(&dir).unwrap();
        let d = consult(&dir, &repo, 40, false);
        assert_eq!(d, RepoConsentDecision::Disabled);
        // No record was created, no dir side effect beyond the account dir.
        assert!(!dir.join(REPO_CONSENT_DIR).exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn record_round_trip_and_safe_reads() {
        let tmp = std::env::temp_dir().join(format!("repo_consent_rt_{}", std::process::id()));
        let dir = tmp.join("account");
        let repo = tmp.join("repo");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(read(&dir, &repo), RepoAnswer::Unset);
        write(&dir, &repo, RepoAnswer::Allowed).unwrap();
        assert_eq!(read(&dir, &repo), RepoAnswer::Allowed);
        write(&dir, &repo, RepoAnswer::Refused).unwrap();
        assert_eq!(read(&dir, &repo), RepoAnswer::Refused);
        // Corrupt + future-version records read Unset.
        let path = repo_consent_path(&dir, &repo);
        std::fs::write(&path, "not json at all").unwrap();
        assert_eq!(read(&dir, &repo), RepoAnswer::Unset);
        std::fs::write(
            &path,
            format!(
                "{{\"version\":{},\"answer\":\"allowed\"}}",
                RECORD_VERSION + 7
            ),
        )
        .unwrap();
        assert_eq!(read(&dir, &repo), RepoAnswer::Unset);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn prompt_words_carry_the_memo_pinned_sentences() {
        let lines = render_prompt_lines(37);
        let full = lines.join("\n");
        // Memo item 5's load-bearing sentences, pinned by substring.
        assert!(full.contains("sends the code snippet"));
        assert!(full.contains("lines, the span plus minimal context) to our LLM provider (Cloudflare Workers AI) over an encrypted connection."));
        assert!(full.contains("not used to train their models"));
        assert!(full.contains("not retained after the response"));
        assert!(full.contains("A local secret scanner runs first"));
        assert!(full.contains("never leaves your machine"));
        assert!(full.contains("Everything else — the scan, the draft, the verify — stays local."));
        // The resolved line count and the option labels in memo order.
        assert!(full.contains("≤ 37"));
        for label in OPTION_LABELS {
            assert!(full.contains(label));
        }
    }

    #[test]
    fn fingerprint_follows_the_path() {
        let a = repo_fingerprint(Path::new("/tmp/one"));
        let b = repo_fingerprint(Path::new("/tmp/two"));
        assert_ne!(a, b);
        assert_eq!(a, repo_fingerprint(Path::new("/tmp/one")));
    }
}
