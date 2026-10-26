//! KAT account G9 fixture gates — Proposal 005 Phase 0 T0.1 (Plan 025).
//!
//! G9 governs money-adjacent client state. For the ACCOUNT half:
//!
//! - **G9a** — the account id is a pure function of the pubkey: same key →
//!   same id, any run, any host (BLAKE3 over raw bytes — host-independent by
//!   construction, the meter's raw-bytes rule applied to identity).
//! - **G9b** — the record shape is version-stamped with an explicit changelog;
//!   the version rides the persisted file.
//! - **G9d** — accounting-only: the grant is a LOCAL projection (since
//!   Issue 081, literally the [`FREE_TIER_GRANT_MICRO_KAT`] meter constant — the
//!   canonical record no longer carries a grant field). No code path in the
//!   account module mints, spends, or mutates the grant; re-login must not
//!   move anything (the "once per account key" property, client half).
//! - **Composition** — the account grant × the repo-local charge ledger → the
//!   balance projection `account` prints (`grant.saturating_sub(charged)`),
//!   read back through the exact persisted JSON the CLI reads.
//! - **Tamper evidence** — key↔record disagreement is caught by `verify`.
//!
//! ```bash
//! cargo test --features kat_service --test kat_account_g9
//! ```

#![cfg(feature = "kat_service")]

use riir_refine::kat_account::{
    account_id_for, AccountState, KatAccount, LoginOutcome, ACCOUNT_CHANGELOG, ACCOUNT_VERSION,
};
use riir_refine::kat_meter::{ChargeState, KatCharge, KatMeter, FREE_TIER_GRANT_MICRO_KAT};

// Issue 179 NOW-1: the ONE renderer file, included here by PATH so the
// pinned suite tests the exact source the bins compile (the test target
// cannot name a bin-local module; the file is pure — `riir_refine::` paths
// resolve as the extern crate in both hosts).
#[path = "../src/bin/kat_info/provider_bindings.rs"]
mod provider_bindings;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kat_acct_g9_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// ── G9a: the id derivation is pure ─────────────────────────────────────────

#[test]
fn g9a_account_id_is_a_pure_function_of_the_pubkey() {
    let pubkey = [0x5au8; 32];
    // Independent recompute — the derivation is pinned against blake3 inline,
    // not against itself (the riir-dapps "pin every reimplemented derivation"
    // lesson).
    let expected = blake3::hash(&pubkey).to_hex().to_string();
    let first = account_id_for(&pubkey);
    assert_eq!(first, expected);
    assert_eq!(first.len(), 64);
    for _ in 0..100 {
        assert_eq!(account_id_for(&pubkey), first);
    }
    // Distinct pubkeys → distinct ids (no truncation collision at 32 bytes).
    assert_ne!(account_id_for(&[0x5b_u8; 32]), first);
}

// ── G9b: version stamp + changelog coverage ────────────────────────────────

#[test]
fn g9b_account_version_is_changelog_covered_and_rides_the_record() {
    assert!(
        ACCOUNT_CHANGELOG.iter().any(|(v, _)| *v == ACCOUNT_VERSION),
        "ACCOUNT_VERSION {ACCOUNT_VERSION} has no changelog entry"
    );
    let dir = temp_dir("stamp");
    let (account, _) = KatAccount::login(&dir).unwrap();
    let _ = account;
    let text = std::fs::read_to_string(dir.join(KatAccount::RECORD_FILE)).unwrap();
    assert!(
        text.contains(&format!("\"account_version\": {ACCOUNT_VERSION}")),
        "record must carry the version stamp: {text}"
    );
}

// ── G9d: the grant is a projection, never mutated ──────────────────────────

#[test]
fn g9d_grant_projection_never_moves_across_relogins() {
    let dir = temp_dir("grant");
    let (first, outcome) = KatAccount::login(&dir).unwrap();
    assert!(matches!(outcome, LoginOutcome::Generated { .. }));
    // G9d post-081: the grant lives in the RENDER, not on the record state.
    // Issue 126 follow-up 3 (owner 2026-10-14): the grant line names the
    // TUNA trial credit — never a KAT figure (a KAT number reads as a mint
    // promise that does not exist).
    assert!(first
        .summary_lines()
        .iter()
        .any(|l| { l.contains("grant:") && l.contains("TUNA trial credit") }));

    // Re-login: Existing — the record (incl. grant + created_at) is preserved
    // byte-for-byte semantics: "granted once per account key".
    let record_before = std::fs::read_to_string(dir.join(KatAccount::RECORD_FILE)).unwrap();
    let (second, outcome) = KatAccount::login(&dir).unwrap();
    assert!(matches!(outcome, LoginOutcome::Existing { .. }));
    assert_eq!(second.state(), first.state());
    let record_after = std::fs::read_to_string(dir.join(KatAccount::RECORD_FILE)).unwrap();
    assert_eq!(
        record_before, record_after,
        "an idempotent re-login must not rewrite the record"
    );

    // Import of the SAME key into a second dir derives the same identity
    // (the id is the key, not the directory).
    let dir2 = temp_dir("grant2");
    let (imported, outcome) =
        KatAccount::login_import(&dir2, &dir.join(KatAccount::KEY_FILE)).unwrap();
    assert!(matches!(outcome, LoginOutcome::Imported { .. }));
    assert_eq!(imported.state().account_id, first.state().account_id);
}

// ── Composition: account grant × repo-local charge → balance projection ────

#[test]
fn g9_composition_account_grant_minus_repo_charge_is_the_balance_projection() {
    // The exact composition `cargo refine account` prints: the machine-global
    // grant projection against THIS repo's local charge ledger.
    let account_dir = temp_dir("comp_acct");
    let (account, _) = KatAccount::login(&account_dir).unwrap();

    let repo_dir = temp_dir("comp_repo");
    // The bin's exact order (Plan 024's composition fixture): meter open →
    // charge open (PRE-session — that is what makes the delta correct) →
    // record → settle at exit → save.
    let mut meter = KatMeter::open(&repo_dir);
    let mut charge = KatCharge::open(&repo_dir, meter.state());
    let tokens = meter.record_span(b"fn main() { println!(\"kat\"); }");
    assert!(tokens > 0);
    charge.settle(meter.state());
    charge.save().unwrap();

    // Read back through the persisted JSON — the same file the CLI reads.
    let text = std::fs::read_to_string(
        repo_dir
            .join(".refine")
            .join(riir_refine::kat_meter::KatCharge::FILE_NAME),
    )
    .unwrap();
    let persisted: ChargeState = serde_json::from_str(&text).unwrap();
    assert_eq!(persisted.charged_kat, tokens, "1 KAT/token at the v1 rate");

    // Issue 081: the grant projection is the meter constant (G9d). The
    // composition math runs in MicroKat — µ/whole mixing needs an explicit
    // conversion to compile (riir-dapps Plan 035 T3).
    let grant = FREE_TIER_GRANT_MICRO_KAT;
    let remaining = grant.saturating_sub(riir_refine::kat_units::MicroKat::from(
        persisted.charged_kat,
    ));
    assert_eq!(
        remaining,
        grant - riir_refine::kat_units::MicroKat::from(tokens)
    );
    // And the ledger files are distinct roots by design: the account is
    // machine-global, the charge is repo-local.
    assert_ne!(
        account.dir(),
        repo_dir.join(".refine"),
        "account (identity) and charge (per-repo burn) live in different roots"
    );
}

// ── L4 (riir-dapps Plan 035 T6, client half): the display VALUE pin ────────

/// Parse a `render_kat` shape ("100.000000 KAT") back to µ — the value half
/// of the L4 pin. Independent of the renderer (string split + decimal
/// parse), so a renderer bug cannot validate itself.
fn parse_kat_micro(rendered: &str) -> Option<i64> {
    let (amount, unit) = rendered.split_once(' ')?;
    if unit != "KAT" {
        return None;
    }
    let (whole, frac) = match amount.split_once('.') {
        Some((w, f)) => (w, f),
        None => (amount, ""),
    };
    if !frac.is_empty() && frac.len() != 6 {
        return None;
    }
    if !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let frac_micro: i64 = if frac.is_empty() {
        0
    } else {
        frac.parse().ok()?
    };
    let per_kat = i64::try_from(riir_refine::kat_units::MICRO_PER_KAT).ok()?;
    whole.checked_mul(per_kat)?.checked_add(frac_micro)
}

/// L4 value-level pin (riir-dapps Plan 035 T6, client half). The g9d pin
/// above is COUPLING-only: it asserts the line contains
/// `render_kat(FREE_TIER_GRANT_MICRO_KAT)` and so passes at ANY constant
/// value — the exact blind spot Plan 035 L7 names. This pin holds the
/// VALUE: the client's rendered grant string is the committed
/// "100.000000 KAT" the dao and dapps twins pin on their side, AND it
/// parses back to exactly 100_000_000 µ — the 10⁸ × 10⁶ coincidence
/// cannot survive string equality AND value equality together.
#[test]
fn l4_free_tier_grant_display_pins_string_and_value() {
    let rendered = riir_refine::kat_payreq_client::render_kat(FREE_TIER_GRANT_MICRO_KAT.0);
    assert_eq!(
        rendered, "100.000000 KAT",
        "the free-tier grant display drifted from the committed cross-repo string"
    );
    let micro = parse_kat_micro(&rendered).expect("the pinned display shape parses");
    assert_eq!(
        micro, 100_000_000,
        "the rendered grant must read back 10⁸ µ"
    );
    assert_eq!(micro, FREE_TIER_GRANT_MICRO_KAT.0);
    // The µ→whole-KAT reading is exact at this value (the WholeKat contract).
    assert_eq!(FREE_TIER_GRANT_MICRO_KAT.to_whole().map(|w| w.0), Some(100));
    // The footer's trimmed reading of the SAME constant agrees ("100" — the
    // other client display of the grant, the fuel-gauge caption).
    assert_eq!(
        riir_refine::kat_footer::fmt_kat_display(FREE_TIER_GRANT_MICRO_KAT),
        "100"
    );
}

// ── Tamper evidence ────────────────────────────────────────────────────────

#[test]
fn g9_verify_catches_a_key_swap_and_load_follows_the_key() {
    let dir = temp_dir("tamper");
    let (account, _) = KatAccount::login(&dir).unwrap();
    account.verify().unwrap();

    // A record rewritten with a foreign id: `load` recovers (key is the
    // authority) — pin that the recovery DERIVES the true id back.
    let mut state = account.state().clone();
    state.account_id = "f".repeat(64);
    std::fs::write(
        dir.join(KatAccount::RECORD_FILE),
        serde_json::to_string(&state).unwrap(),
    )
    .unwrap();
    let recovered = KatAccount::load(&dir).unwrap();
    assert_eq!(recovered.state().account_id, account.state().account_id);
    recovered.verify().unwrap();

    // The mid-process key swap — the tamper the `verify` contract exists for.
    // Three distinct evidence properties, pinned in order:
    //   (1) a HELD account (loaded pre-swap) fails verification;
    //   (2) a FRESH load follows the key (identity = the key, not the dir);
    //   (3) a private-key-only swap leaves a STALE pub sidecar — caught too
    //       ("regenerate before syncing"), and consistent again after the
    //       sidecar follows.
    let other_dir = temp_dir("tamper_other");
    let (_, _) = KatAccount::login(&other_dir).unwrap();
    std::fs::copy(
        other_dir.join(KatAccount::KEY_FILE),
        dir.join(KatAccount::KEY_FILE),
    )
    .unwrap();
    // (1) held state vs swapped key:
    assert!(
        account.verify().is_err(),
        "held state must not verify against a swapped key"
    );
    // (2) fresh load follows the key:
    let followed = KatAccount::load(&dir).unwrap();
    assert_eq!(
        followed.state().account_id,
        KatAccount::load(&other_dir).unwrap().state().account_id,
        "a fresh load follows the key (identity = the key, not the dir)"
    );
    // (3) stale sidecar:
    assert!(
        followed.verify().is_err(),
        "a private-key-only swap must fail on the stale pub sidecar"
    );
    std::fs::copy(
        other_dir.join(KatAccount::PUB_FILE),
        dir.join(KatAccount::PUB_FILE),
    )
    .unwrap();
    let consistent = KatAccount::load(&dir).unwrap();
    consistent.verify().unwrap();
}

// ── Record shape pin (the persisted projection is what sync will replay) ──

#[test]
fn g9_record_round_trips_through_the_persisted_json() {
    let dir = temp_dir("roundtrip");
    let (account, _) = KatAccount::login(&dir).unwrap();
    let text = std::fs::read_to_string(dir.join(KatAccount::RECORD_FILE)).unwrap();
    let parsed: AccountState = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed, *account.state());
    assert_eq!(parsed.account_version, ACCOUNT_VERSION);
    // The canonical record shape: provenance instead of a grant field (the
    // grant is the meter constant — the render carries it).
    assert!(parsed.github_user.is_none());
}

// ── G9f (issue 179 NOW half): provider-binding rows survive the refine ─────
// ── consumer, and the render is additive ──────────────────────────────────

/// A providers-bearing record fixture (the riir-auth Plan 001 T3 v3 shape,
/// spelled as JSON so the test needs no named binding type — the EXACT
/// hazard under test: a consumer binary that never names the type must
/// still parse, hold, and re-write the rows; serde drops what its struct
/// does not name, and the parse here proves the STRUCT names them).
fn providers_fixture_record(account_id: &str) -> String {
    serde_json::json!({
        "account_version": ACCOUNT_VERSION,
        "account_id": account_id,
        "created_at_unix": 1_700_000_000u64,
        "github_user": "octocat",
        "providers": [
            {
                "provider": "github",
                "numeric_id": 5_832_31u64,
                "login_display": "octocat",
                "bound_at_unix": 1_764_000_000u64
            }
        ]
    })
    .to_string()
}

#[test]
fn g9f_provider_rows_round_trip_through_load_and_a_recovery_rewrite() {
    // The consumer-side byte-preserve law (issue 179 NOW-2): a record WITH
    // provider rows, loaded by THIS crate's facade (which names the type
    // only inside riir-auth), survives a load → record-rewrite cycle
    // byte-identically. This is the feature-off-rewrite hazard asserted
    // for the refine consumer specifically: refine enables NO auth
    // binding feature, and must still preserve the rows it cannot create.
    let dir = temp_dir("g9f");
    let (account, _) = KatAccount::login(&dir).unwrap();
    let account_id = account.state().account_id.clone();

    // Overwrite the fresh record with the providers-bearing fixture — the
    // key is untouched, so verify() still passes after the load.
    let fixture = providers_fixture_record(&account_id);
    std::fs::write(dir.join(KatAccount::RECORD_FILE), &fixture).unwrap();

    let loaded = KatAccount::load(&dir).unwrap();
    loaded.verify().unwrap();
    let rows = &loaded.state().providers;
    assert_eq!(rows.len(), 1, "the binding row must survive the parse");
    assert_eq!(
        rows[0].numeric_id, 5_832_31,
        "the NUMERIC id is THE identity"
    );
    assert_eq!(rows[0].login_display, "octocat");

    // The rewrite half: serialize the loaded state exactly as the
    // substrate's write_record does (pretty + trailing newline) — the
    // provider rows must be byte-present, not silently dropped.
    let rewritten = serde_json::to_string_pretty(loaded.state()).unwrap();
    let reparsed: AccountState = serde_json::from_str(&rewritten).unwrap();
    assert_eq!(
        reparsed.providers, *rows,
        "a rewrite must carry the rows forward (the serde-drop class)"
    );
    assert!(rewritten.contains("\"numeric_id\""));
}

#[test]
fn g9f_render_is_additive_and_labels_the_numeric_identity() {
    // The empty-record arm: ZERO render lines — byte-identical to the
    // pre-179 surfaces (the additive law; pinned, not conventional).
    let dir = temp_dir("g9f_empty");
    let (account, _) = KatAccount::login(&dir).unwrap();
    let empty_lines = provider_bindings::provider_binding_lines(account.state());
    assert!(
        empty_lines.is_empty(),
        "an unbound record must add zero lines, got: {empty_lines:?}"
    );

    // The bound arm: a lead count line + one continuation per binding —
    // numeric id FIRST (the identity), display login as provenance, the
    // UTC bound date — through the ONE account_line grid (issue 126
    // follow-up 6: two-space indent, the 13-col label column).
    let dir2 = temp_dir("g9f_bound");
    let (account2, _) = KatAccount::login(&dir2).unwrap();
    let fixture = providers_fixture_record(&account2.state().account_id);
    std::fs::write(dir2.join(KatAccount::RECORD_FILE), &fixture).unwrap();
    let loaded = KatAccount::load(&dir2).unwrap();
    let lines = provider_bindings::provider_binding_lines(loaded.state());
    assert_eq!(lines.len(), 2, "lead + one row: {lines:?}");
    assert!(
        lines[0].starts_with("  bindings:"),
        "grid lead: {}",
        lines[0]
    );
    assert!(lines[0].contains("1 provider(s) bound"));
    assert!(lines[1].starts_with("  "), "continuation keeps the indent");
    assert!(
        lines[1].contains("github #583231") && lines[1].contains("#5"),
        "the numeric id renders: {}",
        lines[1]
    );
    assert!(lines[1].contains("as octocat"), "display login renders");
    assert!(
        lines[1].contains("(bound 2025-"),
        "UTC date renders (1_764_000_000 ≈ 2025-11)"
    );
}
