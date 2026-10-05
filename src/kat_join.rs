//! The first-join environment choice (riir-dapps Plan 045 Phase A1/A2/A3,
//! Proposal 008 §2): `--mine` / `login` on a machine with NO service
//! config offers a one-time network choice instead of silently riding the
//! mainnet default.
//!
//! ## The shape (Proposal 008 §2, byte-pinned in the module tests)
//!
//! ```text
//! Join which kat network?  [devnet] / mainnet
//!   devnet  — the live test network: real pipeline, test credit (KAT/TUNA carry no cash value)
//!   mainnet — canonical host; payout planes are owner-armed later; today it settles nothing
//! > devnet
//! recorded to config (cargo refine config set --url devnet). Change any time.
//! ```
//!
//! - **Default = devnet** (Enter / `--yes` / `RIIR_REFINE_YES=1`): devnet
//!   is the only env where a fresh account's full loop works today (grant →
//!   burn → mine → settle → vest).
//! - **Recorded through the EXISTING `service_url.json` writer** — the same
//!   record `cargo refine config set` writes ([`riir_kat::kat_service_config::store`]),
//!   never a second record format — so `config get` reports it and every
//!   explicit money command ([`crate::kat_lease_client::service_url_for_command`])
//!   resolves it.
//! - [`crate::kat_lease_client::DEFAULT_SERVICE_URL`] **never moves** (the
//!   QR-goldens law): this is an onboarding-time record, not a const flip.
//! - [`JOIN_ENV_ENV_VAR`] `=devnet|mainnet` forces non-interactively
//!   (agents/CI); an invalid value is a loud error naming the valid set.
//! - A machine with an EXISTING configuration — config record OR env
//!   override, any source — is never prompted: the consent / soft-gate
//!   flows stay byte-identical for it (the prompt fires ONLY on the
//!   no-config first-join path).
//! - **testnet is never offered** (A2): the host was torn down 2026-10-03
//!   and answers nothing. The alias stays VALID in
//!   [`riir_kat::kat_service_config::validate_service_url`] (wire compat) —
//!   a picker just must not offer hosts that do not answer (the
//!   torn-down-env rule the site env dropdown already follows).
//!
//! ## Deliberate non-features (the consent-module rules)
//!
//! - **Never gated.** Compiles in EVERY feature build: `--mine` is the
//!   join command in every build, and the choice must fire in each.
//! - **Never hangs.** A non-TTY stdin with no `--yes` and no
//!   [`JOIN_ENV_ENV_VAR`] REFUSES with the env-var remedy (exit 2 at the
//!   call site) — the join never silently continues on the old default,
//!   and never blocks on a closed pipe.
//! - **No network.** The choice, the record and the copy are local. A
//!   mainnet joiner's unarmed planes are named up front (A3), never
//!   discovered at the first 503.

use std::path::{Path, PathBuf};

/// The env var that forces the first-join choice non-interactively
/// (`devnet|mainnet` — the full words, case-insensitive, like the
/// `config set` aliases).
pub const JOIN_ENV_ENV_VAR: &str = "RIIR_REFINE_JOIN_ENV";

/// The environments the first-join picker offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinEnv {
    /// The live test network — the default.
    Devnet,
    /// The canonical host.
    Mainnet,
}

impl JoinEnv {
    /// The pinned alias (the `config set --url <alias>` spelling).
    #[must_use]
    pub fn alias(self) -> &'static str {
        match self {
            Self::Devnet => "devnet",
            Self::Mainnet => "mainnet",
        }
    }

    /// The pinned URL the alias resolves to (the `config set` writer
    /// stores exactly this — the validate step in [`record`] proves it).
    #[must_use]
    pub fn url(self) -> &'static str {
        match self {
            Self::Devnet => riir_kat::kat_service_urls::DEVNET_SERVICE_URL,
            Self::Mainnet => riir_kat::kat_service_urls::MAINNET_SERVICE_URL,
        }
    }

    /// Full-word, case-insensitive parse over [`JOIN_CHOICES`]. `testnet`
    /// (and every other word) is `None` here BY DESIGN — see the module
    /// doc's A2 note.
    #[must_use]
    pub fn from_alias(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        JOIN_CHOICES
            .iter()
            .find(|(_, alias)| alias.eq_ignore_ascii_case(trimmed))
            .map(|(env, _)| *env)
    }
}

/// The picker's choices — EXACTLY devnet + mainnet, devnet first (the
/// bracketed default). `testnet` stays a valid `config set` alias (wire
/// compat) and is deliberately absent: the host answers nothing
/// (torn down 2026-10-03).
pub const JOIN_CHOICES: &[(JoinEnv, &str)] = &[
    (JoinEnv::Devnet, "devnet"),
    (JoinEnv::Mainnet, "mainnet"),
];

/// What the first-join choice decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinDecision {
    /// A configuration already exists (record OR env override) — never
    /// prompt, never rewrite. The common machine; the zero-output path.
    AlreadyConfigured,
    /// The choice was made non-interactively (env force or `--yes`).
    Choose(JoinEnv),
    /// Ask on the terminal (a non-TTY stdin refuses instead of hanging).
    Ask,
}

/// A first-join refusal — printed loud, exit 2 at the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinRefusal {
    /// [`JOIN_ENV_ENV_VAR`] was set to something other than `devnet|mainnet`.
    BadJoinEnv(String),
    /// No force, no `--yes`, and stdin is not a terminal (or answered EOF).
    NonInteractive,
}

impl JoinRefusal {
    /// The stderr line — the remedy named, always.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::BadJoinEnv(value) => format!(
                "join: {JOIN_ENV_ENV_VAR}={value:?} is not one of devnet|mainnet — \
                 set it to `devnet` or `mainnet` (or unset it to be asked)"
            ),
            Self::NonInteractive => format!(
                "join: no service config and no way to answer the first-join prompt \
                 (stdin is not a terminal; no --yes; no {JOIN_ENV_ENV_VAR}) — set \
                 {JOIN_ENV_ENV_VAR}=devnet|mainnet (or pass --yes for the devnet default) \
                 and re-run"
            ),
        }
    }
}

/// The pure decision. The env force is validated FIRST — an explicitly
/// invalid value is a user error at any posture (an existing config does
/// not make a mistyped variable harmless; it is still never a prompt).
/// Then the never-prompt rule, then the non-interactive defaults.
///
/// # Errors
///
/// [`JoinRefusal::BadJoinEnv`] when [`JOIN_ENV_ENV_VAR`] carries a value
/// outside `devnet|mainnet` (the caller prints [`JoinRefusal::message`]
/// and exits 2). Every other input decides.
pub fn decide(
    env_join: Option<&str>,
    yes: bool,
    configured: bool,
) -> Result<JoinDecision, JoinRefusal> {
    // An EMPTY value is "unset" (the SERVICE_URL_ENV_VAR spelling).
    let forced = match env_join {
        Some(raw) if !raw.trim().is_empty() => Some(
            JoinEnv::from_alias(raw)
                .ok_or_else(|| JoinRefusal::BadJoinEnv(raw.to_string()))?,
        ),
        _ => None,
    };
    if configured {
        return Ok(JoinDecision::AlreadyConfigured);
    }
    if let Some(env) = forced {
        return Ok(JoinDecision::Choose(env));
    }
    if yes {
        return Ok(JoinDecision::Choose(JoinEnv::Devnet));
    }
    Ok(JoinDecision::Ask)
}

/// One answered prompt line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Empty line / bare Enter — the bracketed default (devnet).
    Default,
    /// A full-word choice (case-insensitive).
    Pick(JoinEnv),
    /// Anything else — the asker re-prompts.
    Invalid,
}

/// Parse one answered line. Trimmed, case-insensitive, full words only —
/// the same matching discipline as the `config set` aliases (a typo can
/// never resolve to a network).
#[must_use]
pub fn parse_answer(line: &str) -> Answer {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Answer::Default;
    }
    match JoinEnv::from_alias(trimmed) {
        Some(env) => Answer::Pick(env),
        None => Answer::Invalid,
    }
}

/// The ask, byte-pinned (Proposal 008 §2's transcript; the trailing `> `
/// is the answer prompt — printed without a trailing newline).
pub const PROMPT_BLOCK: &str = "Join which kat network?  [devnet] / mainnet
  devnet  — the live test network: real pipeline, test credit (KAT/TUNA carry no cash value)
  mainnet — canonical host; payout planes are owner-armed later; today it settles nothing
> ";

/// The interactive half of [`JoinDecision::Ask`]. A non-TTY stdin refuses
/// (the check runs BEFORE the first read — never a hang); EOF refuses
/// (Ctrl-D is not an answer). An invalid line re-prompts once per read.
/// `read_line` yields `None` at EOF/error; the caller owns the real stdin.
///
/// # Errors
///
/// [`JoinRefusal::NonInteractive`] when the stdin is not a terminal or
/// answers EOF — the caller prints [`JoinRefusal::message`] and exits 2.
pub fn answer_ask(
    tty: bool,
    read_line: &mut dyn FnMut() -> Option<String>,
) -> Result<JoinEnv, JoinRefusal> {
    if !tty {
        return Err(JoinRefusal::NonInteractive);
    }
    loop {
        let Some(line) = read_line() else {
            return Err(JoinRefusal::NonInteractive);
        };
        match parse_answer(&line) {
            Answer::Default => return Ok(JoinEnv::Devnet),
            Answer::Pick(env) => return Ok(env),
            Answer::Invalid => {
                eprintln!("join: answer devnet or mainnet (Enter = devnet)");
            }
        }
    }
}

/// The post-record copy — local by construction (no network calls). Devnet:
/// the recorded line. Mainnet: the recorded line + the A3 posture copy,
/// three lines naming every unarmed plane up front (pool 0, TUNA
/// unconfigured, crank unset) so the joiner never discovers them at the
/// first 503.
#[must_use]
pub fn join_lines(env: JoinEnv) -> Vec<String> {
    let mut lines = vec![format!(
        "recorded to config (cargo refine config set --url {}). Change any time.",
        env.alias()
    )];
    if env == JoinEnv::Mainnet {
        lines.push(
            "mainnet posture: payout planes are owner-armed later — today the reward pool \
             is 0,"
                .to_string(),
        );
        lines.push(
            "  TUNA trial credit is unconfigured (claim answers \"unconfigured\"), and the \
             crank"
                .to_string(),
        );
        lines.push(
            "  token is unset (settle + ops surfaces answer 503 fail-closed). The loop \
             runs; it settles nothing yet."
                .to_string(),
        );
    }
    lines
}

/// Persist the choice through the EXISTING config-set record path — the
/// same `service_url.json` writer `cargo refine config set` uses (never a
/// second record format). The alias validates to the pinned URL first, so
/// the stored value is exactly what `config set` would store.
///
/// # Errors
///
/// The validation or write error, verbatim (the CLI prints it and exits).
pub fn record(dir: &Path, env: JoinEnv) -> Result<PathBuf, String> {
    let validated = riir_kat::kat_service_config::validate_service_url(env.alias())?;
    riir_kat::kat_service_config::store(dir, &validated)
}

/// The join-path entry (the `--mine` / `login` call sites): decides,
/// prompts when asked, records, prints the post-record copy. `Ok(())` on
/// an already-configured machine too — the callers cannot (and must not)
/// distinguish it from a fresh devnet record: the zero-output path is the
/// configured machine's byte-identical behavior.
///
/// # Errors
///
/// The refusal message to print (the call sites exit 2) or the
/// validation/write error, verbatim.
pub fn run(yes: bool) -> Result<(), String> {
    let dir = riir_kat::kat_service_config::account_dir();
    // "An EXISTING config record (any source) → never prompt": the record
    // OR the env override. An env override with no record stays un-prompted
    // too — recording a file under it would manufacture the both-set state
    // `config get` warns about.
    let configured = riir_kat::kat_service_config::read(&dir).is_some()
        || riir_kat::kat_service_config::service_url_from_env().is_some();
    let env_join = std::env::var(JOIN_ENV_ENV_VAR).ok();
    let env = match decide(env_join.as_deref(), yes, configured).map_err(|r| r.message())? {
        JoinDecision::AlreadyConfigured => return Ok(()),
        JoinDecision::Choose(env) => env,
        JoinDecision::Ask => {
            use std::io::IsTerminal as _;
            use std::io::Write as _;
            let tty = std::io::stdin().is_terminal();
            // The TTY gate runs BEFORE the ask prints — a non-terminal
            // stdin never sees a half-asked prompt racing its refusal.
            if !tty {
                return Err(JoinRefusal::NonInteractive.message());
            }
            print!("{PROMPT_BLOCK}");
            let _ = std::io::stdout().flush();
            answer_ask(tty, &mut || {
                let mut line = String::new();
                match std::io::stdin().read_line(&mut line) {
                    Ok(0) | Err(_) => None,
                    Ok(_) => Some(line),
                }
            })
            .map_err(|r| r.message())?
        }
    };
    record(&dir, env)?;
    for line in join_lines(env) {
        println!("{line}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use riir_kat::kat_service_urls::TESTNET_SERVICE_URL;

    /// A scratch dir that never touches the real account dir (the
    /// `kat_service_config` tests' shape).
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir_heal_join_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn the_picker_offers_exactly_devnet_and_mainnet() {
        let aliases: Vec<&str> = JOIN_CHOICES.iter().map(|(_, a)| *a).collect();
        assert_eq!(aliases, ["devnet", "mainnet"]);
        assert!(!aliases.contains(&"testnet"), "A2: testnet is never offered");
        assert_eq!(JoinEnv::from_alias("testnet"), None);
        // …while the alias itself stays VALID for wire compat (the config
        // set surface is untouched — the host is gone, the word is not).
        assert_eq!(
            riir_kat::kat_service_config::resolve_alias("testnet"),
            Some(TESTNET_SERVICE_URL)
        );
        assert!(riir_kat::kat_service_config::validate_service_url("testnet").is_ok());
    }

    #[test]
    fn no_config_yes_records_devnet() {
        // The --yes / RIIR_REFINE_YES=1 arm: devnet, the bracketed default.
        assert_eq!(
            decide(None, true, false).expect("decides"),
            JoinDecision::Choose(JoinEnv::Devnet)
        );
        let dir = scratch("yes_devnet");
        record(&dir, JoinEnv::Devnet).expect("records");
        assert_eq!(
            riir_kat::kat_service_config::read(&dir).as_deref(),
            Some(JoinEnv::Devnet.url())
        );
        let lines = join_lines(JoinEnv::Devnet);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("cargo refine config set --url devnet"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn env_forces_both_values() {
        assert_eq!(
            decide(Some("devnet"), false, false).expect("decides"),
            JoinDecision::Choose(JoinEnv::Devnet)
        );
        assert_eq!(
            decide(Some("mainnet"), false, false).expect("decides"),
            JoinDecision::Choose(JoinEnv::Mainnet)
        );
        // Case-insensitive + trimmed, the alias discipline.
        assert_eq!(
            decide(Some(" MAINNET "), false, false).expect("decides"),
            JoinDecision::Choose(JoinEnv::Mainnet)
        );
        // The force wins over --yes (an explicit choice beats the default).
        assert_eq!(
            decide(Some("mainnet"), true, false).expect("decides"),
            JoinDecision::Choose(JoinEnv::Mainnet)
        );
    }

    #[test]
    fn invalid_env_is_loud_and_names_the_set() {
        let err = decide(Some("staging"), false, false).expect_err("invalid is loud");
        assert_eq!(err, JoinRefusal::BadJoinEnv("staging".to_string()));
        let msg = err.message();
        assert!(msg.contains("staging"), "names the value: {msg}");
        assert!(msg.contains("devnet|mainnet"), "names the set: {msg}");
        // Validation precedes --yes and the configured check: an invalid
        // value never resolves through any other arm.
        assert_eq!(
            decide(Some("staging"), true, false),
            Err(JoinRefusal::BadJoinEnv("staging".to_string()))
        );
        assert_eq!(
            decide(Some("staging"), false, true),
            Err(JoinRefusal::BadJoinEnv("staging".to_string()))
        );
        // An EMPTY value is unset, not invalid (the SERVICE_URL_ENV_VAR
        // spelling) — it falls through to the interactive defaults.
        assert_eq!(
            decide(Some("   "), false, false).expect("decides"),
            JoinDecision::Ask
        );
    }

    #[test]
    fn existing_config_never_prompts() {
        // Any source: the record OR the env override. The zero-output path.
        assert_eq!(
            decide(None, false, true).expect("decides"),
            JoinDecision::AlreadyConfigured
        );
        assert_eq!(
            decide(None, true, true).expect("decides"),
            JoinDecision::AlreadyConfigured
        );
        assert_eq!(
            decide(Some("mainnet"), false, true).expect("decides"),
            JoinDecision::AlreadyConfigured
        );
        // And a second record never overwrites the first: the Choose arm
        // is unreachable when configured (the env force is silently
        // mooted — the caller prints nothing).
        let dir = scratch("existing");
        record(&dir, JoinEnv::Devnet).expect("seed a devnet record");
        match decide(Some("mainnet"), true, true) {
            Ok(JoinDecision::AlreadyConfigured) => {}
            other => panic!("a configured machine must never be re-asked: {other:?}"),
        }
        assert_eq!(
            riir_kat::kat_service_config::read(&dir).as_deref(),
            Some(JoinEnv::Devnet.url()),
            "the existing record is untouched"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mainnet_env_records_mainnet_url_and_names_every_plane() {
        let dir = scratch("env_mainnet");
        record(&dir, JoinEnv::Mainnet).expect("records");
        assert_eq!(
            riir_kat::kat_service_config::read(&dir).as_deref(),
            Some(JoinEnv::Mainnet.url())
        );
        let lines = join_lines(JoinEnv::Mainnet);
        assert_eq!(
            lines.len(),
            4,
            "the recorded line + the 3-line posture copy"
        );
        let posture = lines[1..].join("\n");
        for needle in [
            "mainnet posture",
            "reward pool is 0",
            "TUNA trial credit is unconfigured",
            "crank",
            "503 fail-closed",
            "settles nothing",
        ] {
            assert!(posture.contains(needle), "posture must name {needle}: {posture}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_tty_refuses_without_reading() {
        let mut reads = 0usize;
        let err = answer_ask(false, &mut || {
            reads += 1;
            Some("devnet".to_string())
        })
        .expect_err("a non-terminal stdin refuses");
        assert_eq!(reads, 0, "the refusal must not read a closed pipe");
        assert_eq!(err, JoinRefusal::NonInteractive);
        let msg = err.message();
        assert!(msg.contains(JOIN_ENV_ENV_VAR), "names the env remedy: {msg}");
        assert!(msg.contains("--yes"), "names the flag remedy: {msg}");
        assert!(msg.contains("re-run"), "names the re-run: {msg}");
    }

    #[test]
    fn eof_refuses_enter_is_the_default() {
        // Ctrl-D is not an answer.
        assert_eq!(
            answer_ask(true, &mut || None),
            Err(JoinRefusal::NonInteractive)
        );
        // A bare Enter is the bracketed default.
        assert_eq!(
            answer_ask(true, &mut || Some("\n".to_string())).expect("decides"),
            JoinEnv::Devnet
        );
        assert_eq!(
            answer_ask(true, &mut || Some("  \n".to_string())).expect("decides"),
            JoinEnv::Devnet
        );
    }

    #[test]
    fn answers_parse_full_words_only_and_invalid_reasks() {
        assert_eq!(parse_answer("devnet"), Answer::Pick(JoinEnv::Devnet));
        assert_eq!(parse_answer(" DEVNET\n"), Answer::Pick(JoinEnv::Devnet));
        assert_eq!(parse_answer("mainnet"), Answer::Pick(JoinEnv::Mainnet));
        // The torn-down word parses as INVALID here even though the config
        // alias accepts it — the picker never offers it.
        assert_eq!(parse_answer("testnet"), Answer::Invalid);
        assert_eq!(parse_answer("y"), Answer::Invalid);
        assert_eq!(parse_answer("d"), Answer::Invalid);
        // The re-ask loop: invalid, then valid.
        let mut answers = vec!["staging".to_string(), "mainnet".to_string()];
        assert_eq!(
            answer_ask(true, &mut || {
                if answers.len() == 1 {
                    None
                } else {
                    answers.pop()
                }
            })
            .expect("the second answer wins"),
            JoinEnv::Mainnet
        );
    }

    #[test]
    fn the_prompt_block_is_the_proposal_shape() {
        let mut lines = PROMPT_BLOCK.lines();
        assert_eq!(
            lines.next(),
            Some("Join which kat network?  [devnet] / mainnet")
        );
        assert_eq!(
            lines.next(),
            Some(
                "  devnet  — the live test network: real pipeline, test credit \
                 (KAT/TUNA carry no cash value)"
            )
        );
        assert_eq!(
            lines.next(),
            Some(
                "  mainnet — canonical host; payout planes are owner-armed later; \
                 today it settles nothing"
            )
        );
        assert_eq!(lines.next(), Some("> "));
        assert!(lines.next().is_none(), "the block ends at the answer prompt");
        assert!(PROMPT_BLOCK.ends_with("> "));
    }

    #[test]
    fn devnet_record_survives_a_config_get_read() {
        // The record path IS the config-set record: `read` (what `config
        // get` prints) and the resolver see it with no second format.
        let dir = scratch("roundtrip");
        record(&dir, JoinEnv::Devnet).expect("records");
        let (url, warning) = riir_kat::kat_service_config::read_with_warning(&dir);
        assert!(warning.is_none(), "a fresh record reads clean: {warning:?}");
        assert_eq!(url.as_deref(), Some(JoinEnv::Devnet.url()));
        // …and validate_service_url accepts the stored value (the read
        // path re-validates; a record that fails reads as not-set).
        assert!(riir_kat::kat_service_config::validate_service_url(
            url.as_deref().expect("url")
        )
        .is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
