//! riir-dapps Proposal 009 §9 / issue 129 (owner-mandated 2026-10-17) —
//! the marketing TUNA airdrop ADMIN hand: the operator's CLI over the
//! leaf's [`crate::kat_tuna_admin_client`]. The machine-facing
//! surface is the worker route; this module is the human hand that signs
//! the batch — never a dao agent surface (the agent-surface refusal test
//! in riir-dao pins that separation).
//!
//! ```text
//! cargo refine get 100 tuna                       # airdrop to MY OWN account id
//! cargo refine get 100 tuna --dry-run             # local-only: hash + row + body + caps
//! cargo refine send 100 tuna <32-hex-account-id>  # airdrop to ANOTHER account
//! cargo refine send 0.5k tuna <id> --gov 7        # pinned mandate, glued amount
//! ```
//!
//! The full campaign lifecycle stays governance's: `POST /gov/open` →
//! staked-tranche vote → 48 h timelock → `POST /gov/execute` (the wetrun
//! helper `tuna_wetrun` signs those legs). This CLI executes the FINAL
//! step — the can-opener-signed batch — resolving the Executed mandate by
//! its unique table hash when `--gov` is not pinned.
//!
//! ## Key discipline (§9)
//!
//! - `TUNA_CAN_OPENER_KEY` — the 64-hex ed25519 seed. Read from the env,
//!   refused PRE-NETWORK when missing or malformed, never echoed, never
//!   logged, never accepted on the command line (a shell-history leak is
//!   a key disclosure).
//! - `KAT_CRANK_TOKEN` — the ops bearer the `/admin` routes gate on. Same
//!   pre-network refusal rule. gen-1/devnet only.
//! - `--totp <code>` — the gen-2 posture (riir-dapps Plan 058 T7): the
//!   worker pins a genesis owner and refuses the bearer; the account key
//!   (the owner's) signs the request and the code is the owner account's
//!   TOTP. One code authorizes ONE request, so the owner posture needs
//!   `--gov <id>` (the mandates listing would spend a second code).
//! - `--dry-run` touches neither: it composes the table hash, the resolved
//!   recipient row, the exact request body shape, and the locally
//!   mirrorable cap arithmetic — no network, no secrets.
//!
//! The client mirrors the route's amount checks LOCALLY (the free-tier
//! floor + the hard campaign cap, below); the ROUTE re-validates
//! everything against ITS ledger config — a local mirror drifting from
//! the server surfaces as a coded server refusal, never a wrong airdrop.

use ed25519_dalek::{Signer as _, SigningKey};

use crate::kat_admin_client::AdminAuth;
use crate::kat_protocol_tuna_airdrop::{
    account_from_hex, account_to_hex, canonical_table_bytes, sig_message, table_hash,
    TunaAirdropRecipientWire, TunaAirdropRequestWire,
};
use crate::kat_tuna_admin_client::{
    fetch_mandates, resolve_mandate_by_hash, submit_airdrop, MandateMatch,
};

/// The route's free-tier floor, mirrored locally (Proposal 009 §4.3 rev 3
/// — an airdrop below the free grant refuses: a small credit would consume
/// the recipient's whole once-per-life signup claim). `TunaConfig`'s
/// `free_grant` documented genesis value; the route re-validates.
pub const FREE_GRANT_FLOOR_MICRO_DEFAULT: u64 = 100_000_000;

/// The per-campaign cap's HARD arm (dapps `tuna.rs`: `min(2% reservoir,
/// 2_000 TUNA)`). The 2%-reservoir arm needs ledger truth — server-checked
/// only; this constant catches the operator's fat-finger locally.
pub const CAMPAIGN_CAP_HARD_MICRO: u64 = 2_000_000_000;

/// One parsed invocation: everything the run needs, decided BEFORE any
/// secret or network touch.
pub struct TunaAdminArgs<'a> {
    /// The verb (`get` | `send`) — receipt wording only; both share the flow.
    pub verb: &'a str,
    /// The amount word (6-dec whole-TUNA; the glued `0.5k` unit form is
    /// already split post-parse by the CLI dispatch).
    pub amount: Option<&'a str>,
    /// The unit word — must be `tuna` (case-insensitive).
    pub unit: Option<&'a str>,
    /// `send`'s recipient (32-hex account id); `get` targets the own id.
    pub target: Option<&'a str>,
    pub gov: Option<u64>,
    pub dry_run: bool,
    /// `--totp <code>`: the gen-2 owner posture (Plan 058 T7) — the owner
    /// key signs, the code is the owner account's TOTP; `None` = the gen-1
    /// `KAT_CRANK_TOKEN` bearer.
    pub totp: Option<&'a str>,
    /// The resolved service URL (the dial seam: env > `--net` > the mainnet
    /// default — the CLI's one resolver passes it in; this module never
    /// re-derives the dialing law).
    pub url: &'a str,
}

/// The entry point — follows the house command convention (the wetrun /
/// buy precedent): prints and EXITS the process itself, so the dispatch
/// arm cannot silently drop an exit code. Exits: 0 applied/replayed ·
/// 1 refused (coded server refusal, missing secret, unresolvable
/// mandate) · 2 usage/cap-mirror (the buy-verb exit convention).
pub fn run(args: &TunaAdminArgs<'_>) -> ! {
    let amount_micro = parse_amount(args);
    let target = resolve_recipient(args);
    let (th, th_hex) = compose_table(target, amount_micro, args.verb);
    let batch = Batch {
        gov_id: 0,
        th,
        th_hex,
        recipients: vec![TunaAirdropRecipientWire {
            account_hex: account_to_hex(&target),
            amount_micro,
        }],
        target,
    };
    if args.dry_run {
        print_dry_run(
            args.verb,
            args.gov,
            &target,
            amount_micro,
            &batch.th_hex,
            &batch.recipients,
        );
        std::process::exit(0);
    }
    // Secrets — refused PRE-NETWORK, in dependency order (key first: it is
    // the rarer and more dangerous omission), never echoed.
    let opener = expect(args.verb, can_opener_key());
    let transport = crate::kat_tuna_admin_client::HttpTunaAdminTransport::new(args.url, 10);
    if let Some(code) = args.totp {
        if args.gov.is_none() {
            eprintln!(
                "{}: --totp needs --gov <id> — one authenticator code authorizes ONE admin request, and resolving the mandate would spend it on the listing",
                args.verb
            );
            std::process::exit(2);
        }
        let signer = expect(args.verb, crate::admin_call::owner_signer(args.url, code));
        submit_and_report(args, &transport, &AdminAuth::Owner(&signer), &opener, batch);
    }
    let bearer = expect(args.verb, ops_bearer());
    submit_and_report(
        args,
        &transport,
        &AdminAuth::Bearer(&bearer),
        &opener,
        batch,
    );
}

/// The amount + unit words → µ, with the glued split and the local cap
/// mirror. Exits 2 on any bad word (the buy-verb convention).
fn parse_amount(args: &TunaAdminArgs<'_>) -> u64 {
    let (amount_word, unit_word) = match (args.amount, args.unit) {
        (Some(a), Some(u)) => (a, u),
        (Some(a), None) => {
            // The glued `<amt>tuna` form: split at the first alphabetic
            // char (the buy-verb precedent — the dispatch stores the word
            // whole).
            if let Some(i) = a.find(|c: char| c.is_ascii_alphabetic()) {
                (&a[..i], &a[i..])
            } else {
                eprintln!(
                    "{}: a unit is required (tuna) — cargo refine {} <amt> tuna",
                    args.verb, args.verb
                );
                std::process::exit(2);
            }
        }
        (None, _) => {
            eprintln!(
                "{}: an amount is required — cargo refine {} <amt> tuna",
                args.verb, args.verb
            );
            std::process::exit(2);
        }
    };
    if !unit_word.eq_ignore_ascii_case("tuna") {
        eprintln!("{0}: unknown unit {unit_word:?} — tuna (the marketing airdrop moves TUNA trial credit)", args.verb);
        std::process::exit(2);
    }
    let Some(micro) = crate::kat_payreq_client::parse_kat_amount(amount_word) else {
        eprintln!(
            "{0}: bad amount {amount_word:?} (6-dec decimals, e.g. 100 or 0.5)",
            args.verb
        );
        std::process::exit(2);
    };
    if micro <= 0 {
        eprintln!("{}: the amount must be above zero", args.verb);
        std::process::exit(2);
    }
    let Ok(amount_micro) = u64::try_from(micro) else {
        eprintln!("{}: the amount overflows", args.verb);
        std::process::exit(2);
    };
    if let Err(msg) = local_cap_checks(amount_micro) {
        eprintln!("{0}: {msg}", args.verb);
        std::process::exit(2);
    }
    amount_micro
}

/// The recipient. `get` = the own account id (pre-network, local state);
/// `send` = the given 32-hex id, with the b58 shape refused by name.
fn resolve_recipient(args: &TunaAdminArgs<'_>) -> [u8; 16] {
    if args.verb == "get" {
        if args.target.is_some() {
            eprintln!("get: takes no recipient — it targets your OWN account id (use `send <amt> tuna <account-id>` for another account)");
            std::process::exit(2);
        }
        expect(args.verb, own_account_id())
    } else {
        match resolve_target(args.target) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("send: {e}");
                std::process::exit(2);
            }
        }
    }
}

/// The canonical table (ONE recipient row) + its blake3 hash — the
/// mandate's evidence and the signature's third term.
fn compose_table(target: [u8; 16], amount_micro: u64, verb: &str) -> ([u8; 32], String) {
    let Some(table) = canonical_table_bytes(&[(target, amount_micro)]) else {
        eprintln!(
            "{verb}: the recipient table is not canonical (impossible for one positive row — bug)"
        );
        std::process::exit(2);
    };
    let th = table_hash(&table);
    (th, crate::kat_payreq_client::hex_encode(&th))
}

/// The composed campaign, carried from composition through submission so
/// no helper re-derives a byte of it (and so the helpers stay under the
/// argument-width lint).
struct Batch {
    /// Filled by [`resolve_mandate`] (mutated in place before submit).
    gov_id: u64,
    th: [u8; 32],
    th_hex: String,
    recipients: Vec<TunaAirdropRecipientWire>,
    target: [u8; 16],
}

/// Unwrap a refusal as exit 1 — the missing-secret / missing-account /
/// unreadable-listing shape.
fn expect<T>(verb: &str, r: Result<T, String>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{verb}: {e}");
            std::process::exit(1);
        }
    }
}

/// `--gov` pins; else the listing's unique-hash match (Proposal 009
/// §9.1). Exits 1 on zero/ambiguous/unreadable.
fn resolve_mandate(
    args: &TunaAdminArgs<'_>,
    transport: &dyn crate::kat_tuna_admin_client::TunaAdminTransport,
    auth: &AdminAuth<'_>,
    batch: &Batch,
) -> u64 {
    let Some(gov) = args.gov else {
        let mandates = expect(args.verb, fetch_mandates(transport, auth));
        let th_hex = batch.th_hex.as_str();
        return match resolve_mandate_by_hash(&mandates, th_hex) {
            MandateMatch::Unique(id) => id,
            MandateMatch::None => {
                eprintln!(
                    "{verb}: no Executed mandate carries this table hash {th_hex}.\
                     \nOpen one first — the governed lifecycle is\
                     \n`POST /gov/open` (kind tuna_airdrop, evidence = this hash) →\
                     \nstaked vote → 48 h timelock → `POST /gov/execute`;\
                     \nthe signing helper is the worker example `tuna_wetrun`\
                     \n(runbook: riir-dapps .docs/11_kat_service/2026-10-18_tuna_marketing_airdrop).",
                    verb = args.verb
                );
                std::process::exit(1);
            }
            MandateMatch::Ambiguous(n) => {
                eprintln!(
                    "{verb}: {n} Executed mandates carry this table hash — pin one with `--gov <id>`",
                    verb = args.verb
                );
                std::process::exit(1);
            }
        };
    };
    gov
}

/// Sign the EXACT bytes, submit, and report: the receipt line, the coded
/// refusal (+ the per-class hint), or the transport failure. The balance
/// re-read rides the success arms (best-effort, never fails the run).
fn submit_and_report(
    args: &TunaAdminArgs<'_>,
    transport: &dyn crate::kat_tuna_admin_client::TunaAdminTransport,
    auth: &AdminAuth<'_>,
    opener: &SigningKey,
    mut batch: Batch,
) -> ! {
    batch.gov_id = resolve_mandate(args, transport, auth, &batch);
    let (gov_id, th) = (batch.gov_id, batch.th);
    let signature = opener.sign(&sig_message(gov_id, th));
    let request = TunaAirdropRequestWire {
        gov_id,
        recipients: batch.recipients,
        signature_hex: crate::kat_payreq_client::hex_encode(&signature.to_bytes()),
    };
    match submit_airdrop(transport, auth, &request) {
        crate::kat_tuna_admin_client::TunaAdminOutcome::Ok(receipt) => {
            if receipt.already_applied {
                println!(
                    "airdrop: already applied — gov {gov_id}'s marker answered (nothing moved; once-per-batch)"
                );
            } else {
                let total = crate::kat_units::render_units(
                    i64::try_from(receipt.total_micro).unwrap_or(i64::MAX),
                    "TUNA",
                );
                let claims = receipt
                    .claims_remaining
                    .map(|c| format!(", claims_remaining {c}"))
                    .unwrap_or_default();
                println!(
                    "airdrop: applied gov {gov_id} — {} recipient(s), {total}{claims}",
                    receipt.recipients
                );
            }
            print_receipt_balance(args.verb, args.url, &account_to_hex(&batch.target));
            std::process::exit(0)
        }
        crate::kat_tuna_admin_client::TunaAdminOutcome::Refused {
            status,
            code,
            detail,
        } => {
            eprintln!(
                "{verb}: refused ({status} {}{detail})",
                code.as_deref()
                    .map_or_else(String::new, |c| format!("{c} — ")),
                verb = args.verb
            );
            for hint in refusal_hints(code.as_deref(), &batch.th_hex) {
                eprintln!("  hint: {hint}");
            }
            std::process::exit(1)
        }
        crate::kat_tuna_admin_client::TunaAdminOutcome::Unexpected(e) => {
            eprintln!("{}: {e}", args.verb);
            std::process::exit(1)
        }
    }
}

/// The locally mirrorable cap checks (§9: the client mirrors; the route
/// re-validates against its own config).
fn local_cap_checks(amount_micro: u64) -> Result<(), String> {
    if amount_micro < FREE_GRANT_FLOOR_MICRO_DEFAULT {
        return Err(format!(
            "amount {amount_micro} µ is below the free-tier floor ({FREE_GRANT_FLOOR_MICRO_DEFAULT} µ) — a small airdrop would consume the recipient's whole once-per-life signup claim (Proposal 009 §4.3)"
        ));
    }
    if amount_micro > CAMPAIGN_CAP_HARD_MICRO {
        return Err(format!(
            "amount {amount_micro} µ exceeds the per-campaign hard cap ({CAMPAIGN_CAP_HARD_MICRO} µ = 2000 TUNA); the 2%-of-reservoir arm is checked server-side"
        ));
    }
    Ok(())
}

/// `get`'s target: the OWN service account id — the local account state,
/// no network (the same derivation the `kat`/`tuna` verbs read with).
fn own_account_id() -> Result<[u8; 16], String> {
    let dir = crate::kat_account::KatAccount::default_dir();
    let account = crate::kat_account::KatAccount::load(&dir)
        .map_err(|e| format!("no local account ({e}) — run `cargo refine login` first"))?;
    let key = account
        .signing_key()
        .map_err(|e| format!("could not load the account signing key: {e}"))?;
    let pubkey = key.verifying_key().to_bytes();
    Ok(crate::kat_identity::service_account_for(&pubkey))
}

/// `send`'s target: 32 hex chars, else refused with the base58 hint.
fn resolve_target(arg: Option<&str>) -> Result<[u8; 16], String> {
    let Some(arg) = arg else {
        return Err("a recipient is required — cargo refine send <amt> tuna <account-id>".into());
    };
    if let Some(id) = account_from_hex(arg) {
        return Ok(id);
    }
    if looks_like_base58(arg) {
        return Err(format!(
            "{arg:?} looks like a base58 Solana pubkey — the airdrop target is the \
             SERVICE account id (32 hex chars = blake3(pubkey)[..16]). The recipient \
             finds theirs with `cargo refine account` (the `account:` line)."
        ));
    }
    Err(format!(
        "{arg:?} is not a 32-hex account id — the recipient's `cargo refine account` prints it"
    ))
}

/// The b58 SHAPE (not a decode — no bs58 dep here): alphanumeric, long
/// enough to be a pubkey, and missing 0/O/I/l (base58's excluded set).
fn looks_like_base58(s: &str) -> bool {
    (43..=50).contains(&s.len())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
        && !s.chars().any(|c| matches!(c, '0' | 'O' | 'I' | 'l'))
}

/// `TUNA_CAN_OPENER_KEY` — read, validated, NEVER echoed.
fn can_opener_key() -> Result<SigningKey, String> {
    let Ok(hex) = std::env::var("TUNA_CAN_OPENER_KEY") else {
        return Err("TUNA_CAN_OPENER_KEY is not set — export the 64-hex can-opener seed (the dapps .env value, or the ~/.kat/tuna_can_opener_*.key file's contents); it is never accepted on the command line".into());
    };
    let bytes = hex_decode(trimmed(&hex));
    let Some(seed): Option<[u8; 32]> = bytes.and_then(|b| b.try_into().ok()) else {
        return Err(
            "TUNA_CAN_OPENER_KEY is not a valid 64-hex ed25519 seed — refusing pre-network".into(),
        );
    };
    Ok(SigningKey::from_bytes(&seed))
}

/// `KAT_CRANK_TOKEN` — the ops bearer, read + refused pre-network.
fn ops_bearer() -> Result<String, String> {
    match std::env::var("KAT_CRANK_TOKEN") {
        Ok(t) if !t.trim().is_empty() => Ok(t.trim().to_string()),
        _ => Err(
            "KAT_CRANK_TOKEN is not set — export the ops bearer the /admin routes gate on".into(),
        ),
    }
}

fn trimmed(s: &str) -> &str {
    s.trim()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    if !b.len().is_multiple_of(2) {
        return None;
    }
    (0..b.len() / 2)
        .map(|i| {
            let hi = (b[i * 2] as char).to_digit(16)?;
            let lo = (b[i * 2 + 1] as char).to_digit(16)?;
            u8::try_from((hi << 4) | lo).ok()
        })
        .collect()
}

/// The local-only preview: hash + resolved row + body shape + the cap
/// arithmetic, then the network/ledger arms named as not-checked.
fn print_dry_run(
    verb: &str,
    gov: Option<u64>,
    target: &[u8; 16],
    amount_micro: u64,
    th_hex: &str,
    recipients: &[TunaAirdropRecipientWire],
) {
    let amount =
        crate::kat_units::render_units(i64::try_from(amount_micro).unwrap_or(i64::MAX), "TUNA");
    println!("{verb} — DRY RUN (no network, no secrets read)");
    println!(
        "  target: {}{}",
        account_to_hex(target),
        if verb == "get" {
            "  (own account id)"
        } else {
            ""
        }
    );
    println!("  amount: {amount} ({amount_micro} µ)");
    println!("  floor:  ok (≥ {FREE_GRANT_FLOOR_MICRO_DEFAULT} µ free-tier floor, local mirror)");
    println!(
        "  cap:    ok (≤ {CAMPAIGN_CAP_HARD_MICRO} µ per-campaign hard cap; the 2%-reservoir arm, the 90-day 5% window and the 8 000-claims floor are LEDGER truth — the route re-checks)"
    );
    println!("  table_hash: {th_hex}  (canonical: 1 row, 24 B, blake3)");
    let gov_shape = gov.map_or_else(
        || "RESOLVED_ON_RUN (unique-hash via GET /admin/tuna/airdrop/mandates)".to_string(),
        |g| g.to_string(),
    );
    println!("  mandate: {gov_shape}");
    println!(
        "  body:    {{\"gov_id\":<id>,\"recipients\":{},\"signature_hex\":\"<ed25519 over kat:tunamkt:v1: ‖ gov_id_le64 ‖ table_hash>\"}}",
        serde_json::to_string(recipients).unwrap_or_default()
    );
}

/// The per-code operator hints (the machine code is the branch; the prose
/// is the hand).
fn refusal_hints(code: Option<&str>, th_hex: &str) -> Vec<String> {
    match code {
        Some("unconfigured") => vec![
            "the worker has no TUNA_CAN_OPENER_PUBKEY — upload the verify key first (wrangler secret put)".into(),
        ],
        Some("bad_signature") => vec![
            "the signature did not verify against the worker's pubkey — wrong TUNA_CAN_OPENER_KEY for this net, or the wrong --net dial".into(),
        ],
        Some("bad_field") => vec![
            format!("the mandate must be Executed AND its evidence hash must equal this table's {th_hex} — GET /admin/tuna/airdrop/mandates lists what is open"),
        ],
        Some("ledger_refused") => vec![
            "a recipient already consumed its free tier — the once-per-life rule admits no retry for that account".into(),
        ],
        Some("unauthorized") => vec![
            "the ops bearer (KAT_CRANK_TOKEN) was refused — wrong net or wrong token".into(),
        ],
        // Plan 058 T7: the gen-2 owner posture's refusals.
        c @ Some(_) if crate::admin_call::hint(c).is_some() => {
            vec![crate::admin_call::hint(c).unwrap_or_default().to_string()]
        }
        _ => vec!["the route's `code` names the class; the ledger refuses before any write (whole-batch-or-nothing)".into()],
    }
}

/// The receipt's balance re-read — the EXISTING `cargo refine tuna` path
/// (the account `/history` read + `parse_history` + the ONE TUNA
/// formatter, `kat_units::render_units`). Best-effort: the airdrop itself
/// already succeeded; a failed re-read says so instead of failing the run.
fn print_receipt_balance(verb: &str, url: &str, acct_hex: &str) {
    use crate::kat_account_client::{AccountTransport as _, HttpAccountTransport};
    use crate::kat_lease_client::HttpResponse;
    let transport = HttpAccountTransport::new(url, 10);
    let (line, ok) = match transport.get_history(acct_hex, 1) {
        HttpResponse::Ok(body) => {
            match crate::kat_protocol_account::parse_history(&body) {
                Ok(h) => match (h.tuna_balance_micro, h.tuna_expires_unix) {
                    (Some(0), _) => (
                        "0.000000 TUNA spendable (spent or expired)".to_string(),
                        true,
                    ),
                    (Some(micro), Some(exp)) if exp > now_unix() => (
                        format!(
                            "{} spendable — expires in {} d",
                            crate::kat_units::render_units(micro, "TUNA"),
                            (exp - now_unix()) / 86_400
                        ),
                        true,
                    ),
                    (Some(micro), _) => (
                        format!("{} spendable", crate::kat_units::render_units(micro, "TUNA")),
                        true,
                    ),
                    (None, _) => (
                        "no spendable TUNA visible yet (the grant may be consumed, or the read lagged — `cargo refine tuna` re-reads)".into(),
                        true,
                    ),
                },
                Err(e) => (format!("the balance re-read did not parse ({e})"), false),
            }
        }
        _ => ("the balance re-read did not answer".into(), false),
    };
    if ok {
        println!("receipt: {acct_hex} — {line}");
    } else {
        eprintln!("{verb}: receipt: {line} — the airdrop itself succeeded (see above)");
    }
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kat_tuna_admin_client::TunaMandateRow;

    #[test]
    fn local_cap_floor_and_hard_cap() {
        assert!(local_cap_checks(99_999_999).is_err());
        assert!(local_cap_checks(100_000_000).is_ok());
        assert!(local_cap_checks(2_000_000_000).is_ok());
        assert!(local_cap_checks(2_000_000_001).is_err());
    }

    #[test]
    fn resolve_target_hex_ok_and_base58_hinted() {
        let hex = "a1".repeat(16);
        assert_eq!(resolve_target(Some(&hex)).unwrap(), [0xa1; 16]);
        // Uppercase hex decodes.
        assert_eq!(
            resolve_target(Some(&hex.to_uppercase())).unwrap(),
            [0xa1; 16]
        );
        // A 44-char base58 pubkey shape (no 0/O/I/l) → the hint error.
        let b58 = "5Fa4wW8xXjK9cQ2vBn7mLtR3yHuDgPeSsAhWkZqNbTnM4Rxd";
        assert!(b58.len() >= 43);
        let err = resolve_target(Some(b58)).unwrap_err();
        assert!(err.contains("base58"), "{err}");
        // Garbage → the generic refusal.
        assert!(resolve_target(Some("zz")).is_err());
        assert!(resolve_target(None).is_err());
    }

    #[test]
    fn looks_like_base58_rejects_hex_and_short_shapes() {
        // 32-hex is NOT b58-shaped (wrong length) — resolve_target takes the
        // hex path first anyway.
        assert!(!looks_like_base58(&"a".repeat(32)));
        assert!(!looks_like_base58("short"));
        // Contains an excluded char.
        assert!(!looks_like_base58(
            "O0Ilbadactorlongenoughshape1234567890ABCDEF"
        ));
    }

    #[test]
    fn missing_key_refuses_with_the_env_name() {
        // SAFETY: single-threaded test body; the env var is not read
        // elsewhere in this binary's test run concurrently.
        std::env::remove_var("TUNA_CAN_OPENER_KEY");
        let err = can_opener_key().unwrap_err();
        assert!(err.contains("TUNA_CAN_OPENER_KEY"), "{err}");
        assert!(!err.contains('\n') || err.len() < 400);
    }

    #[test]
    fn malformed_key_refuses_pre_network_without_echo() {
        // SAFETY: as above.
        std::env::set_var("TUNA_CAN_OPENER_KEY", "not-hex-at-all");
        let err = can_opener_key().unwrap_err();
        assert!(err.contains("not a valid 64-hex"), "{err}");
        // The bad VALUE is never echoed back.
        assert!(!err.contains("not-hex-at-all"));
        std::env::remove_var("TUNA_CAN_OPENER_KEY");
    }

    #[test]
    fn refusal_hints_name_the_class() {
        assert!(refusal_hints(Some("bad_signature"), "h")
            .iter()
            .any(|h| h.contains("TUNA_CAN_OPENER_KEY")));
        assert!(refusal_hints(Some("ledger_refused"), "h")
            .iter()
            .any(|h| h.contains("once-per-life")));
        assert_eq!(refusal_hints(None, "h").len(), 1);
    }

    #[test]
    fn mandate_match_over_listing_rows() {
        let rows = vec![
            TunaMandateRow {
                proposal_id: 7,
                evidence_hash_hex: "abc".into(),
                exec_eligible_tick: 1,
            },
            TunaMandateRow {
                proposal_id: 9,
                evidence_hash_hex: "def".into(),
                exec_eligible_tick: 2,
            },
        ];
        assert_eq!(
            resolve_mandate_by_hash(&rows, "ABC"),
            MandateMatch::Unique(7)
        );
        assert_eq!(resolve_mandate_by_hash(&rows, "f00"), MandateMatch::None);
    }

    #[test]
    fn one_positive_row_is_canonical_and_hashes_stably() {
        let target = [0x07u8; 16];
        let table = canonical_table_bytes(&[(target, 100_000_000)]).expect("canonical");
        assert_eq!(table.len(), 24);
        let th = table_hash(&table);
        assert_eq!(th, table_hash(&table), "deterministic");
    }
}
