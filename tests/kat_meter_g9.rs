//! KAT meter G9 fixture gates — Proposal 005 Phase 0 T0.5 (Plan 023).
//!
//! G9 governs the billing counter because it is money-adjacent: it must be a
//! **function of pinned inputs only**.
//!
//! - **G9a** — same input → bit-identical count, any run, any host. The
//!   host-independence half is pinned by the line-ending fixture: the only
//!   host-dependent byte difference for identical repo content is EOL
//!   (CRLF on Windows checkouts, LF elsewhere), and `\r` is ASCII
//!   whitespace — so the count cannot move. A literal M3 ↔ Windows run
//!   belongs to the first CI with a Windows runner (Proposal 003's release
//!   pipeline); the property it would check is what these fixtures pin.
//! - **G9b** — the definition is version-stamped with an explicit changelog
//!   (no silent reprice); the version rides the persisted file.
//! - **G9c** — spans that fail to parse still bill (the raw-bytes rule): a
//!   fixture of deliberately unparseable spans must bill non-zero and
//!   identically. Parse-gating would bill zero on exactly the L4 miss class
//!   the reachability contract (Issue 030) routes to the expensive path.
//! - **G9d** — client counts never mint or spend (accounting-only; enforced
//!   by construction — the meter has no balance math to test).
//! - **G2** — meter overhead is alloc-free on the count path
//!   (the `CountingAllocator` convention).
//!
//! ```bash
//! cargo test --features kat_service --test kat_meter_g9
//! ```

#![cfg(feature = "kat_service")]

use riir_clippy::kat_meter::{
    count_word_tokens, kat_for_tokens, ChargeState, KatCharge, KatMeter, MeterState,
    CHARGE_CHANGELOG, CHARGE_KAT_PER_MILLION_TOKENS, CHARGE_VERSION, METER_CHANGELOG,
    METER_VERSION,
};

// ── G9a: purity + determinism ──────────────────────────────────────────────

#[test]
fn g9a_same_input_bills_bit_identically_across_repeated_counts() {
    let src = include_str!("fixtures/kat_meter_sample.rs");
    let bytes = src.as_bytes();
    let first = count_word_tokens(bytes);
    assert!(first > 0);
    for _ in 0..100 {
        assert_eq!(count_word_tokens(bytes), first);
    }
}

#[test]
fn g9a_line_endings_do_not_move_the_count() {
    // The same code, as it would be checked out on different hosts. `\r` is
    // ASCII whitespace, so CRLF / LF / CR produce identical token counts —
    // the load-bearing host-independence property of the raw-bytes rule.
    let lf = "fn main() {\n    let x: u32 = 1;\n    println!(\"{}\", x);\n}\n";
    let crlf = lf.replace('\n', "\r\n");
    let cr = lf.replace('\n', "\r");
    let mixed = "fn main() {\r\n    let x: u32 = 1;\n    println!(\"{}\", x);\r\n}\n";
    let expected = count_word_tokens(lf.as_bytes());
    assert!(expected > 0);
    assert_eq!(count_word_tokens(crlf.as_bytes()), expected);
    assert_eq!(count_word_tokens(cr.as_bytes()), expected);
    assert_eq!(count_word_tokens(mixed.as_bytes()), expected);
}

#[test]
fn g9a_path_separators_never_enter_the_count() {
    // Path separators appear in arguments, never in file content — but pin
    // that the counter is byte-only: a path-like string bills as its words.
    assert_eq!(count_word_tokens(b"src/bin/heal.rs"), 1);
    assert_eq!(count_word_tokens(b"src\\bin\\heal.rs"), 1);
}

// ── G9c: unparseable spans still bill ─────────────────────────────────────

#[test]
fn g9c_unparseable_spans_bill_nonzero_and_stably() {
    // Binary garbage (not valid UTF-8 — the counter works on raw bytes).
    let binary = [0x00u8, 0xff, 0xfe, 0x89, 0x50, 0x00, 0x00, 0xab, 0xcd, 0xef];
    let n1 = count_word_tokens(&binary);
    assert!(n1 > 0, "binary garbage must bill non-zero");
    assert_eq!(count_word_tokens(&binary), n1);

    // Broken Rust (unbalanced, unparseable — the L4 miss class shape).
    let broken = "fn main() { let x: u32 = {{{ unbalanced }}} \"unterminated \n struct ";
    let n2 = count_word_tokens(broken.as_bytes());
    assert!(n2 > 0, "unparseable Rust must bill non-zero");
    assert_eq!(count_word_tokens(broken.as_bytes()), n2);

    // A file that fails to parse entirely still bills through the meter.
    let mut dir = std::env::temp_dir();
    dir.push(format!("kat_g9c_{}_{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut m = KatMeter::open(&dir);
    let billed = m.record_span(broken.as_bytes());
    assert_eq!(billed, n2, "meter and bare counter must agree");
    let _ = std::fs::remove_dir_all(&dir);
}

// ── G9b: version stamp ─────────────────────────────────────────────────────

#[test]
fn g9b_version_is_changelog_covered_and_rides_the_file() {
    assert!(
        METER_CHANGELOG.iter().any(|(v, _)| *v == METER_VERSION),
        "METER_VERSION {METER_VERSION} has no changelog entry"
    );
    let dir = std::env::temp_dir().join(format!("kat_g9b_{}_{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).unwrap();
    let m = KatMeter::open(&dir);
    m.save().unwrap();
    let text = std::fs::read_to_string(dir.join(".heal").join(KatMeter::FILE_NAME)).unwrap();
    let state: MeterState = serde_json::from_str(&text).unwrap();
    assert_eq!(state.version, METER_VERSION);
    let _ = std::fs::remove_dir_all(&dir);
}

// ── Persistence round-trip (the local burn ledger) ─────────────────────────

#[test]
fn meter_cumulative_burn_survives_reopen() {
    let dir = std::env::temp_dir().join(format!("kat_persist_{}_{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).unwrap();
    let src_a = b"fn main() { println!(\"hello\"); }";
    let src_b = b"let x: u32 = 42;";

    let mut m1 = KatMeter::open(&dir);
    let a = m1.record_span(src_a);
    m1.save().unwrap();

    let mut m2 = KatMeter::open(&dir);
    assert_eq!(
        m2.state().cumulative_tokens,
        a,
        "burn persists across reopen"
    );
    assert_eq!(m2.session_tokens(), 0, "session resets on open");
    assert_eq!(
        m2.state().sessions,
        m1.state().sessions + 1,
        "one session per open"
    );
    let b = m2.record_span(src_b);
    m2.save().unwrap();

    let m3 = KatMeter::open(&dir);
    assert_eq!(m3.state().cumulative_tokens, a + b);
    assert_eq!(m3.state().sessions, 3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn meter_summary_carries_no_currency_figure() {
    // Caveats #12/#15: no `$` beside token accounting — the summary line is
    // tokens-only by construction; pin it so a pricing string cannot creep in.
    let dir = std::env::temp_dir().join(format!("kat_summary_{}_{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut m = KatMeter::open(&dir);
    m.record_span(b"fn main() {}");
    let line = m.summary_line();
    assert!(
        !line.contains('$'),
        "summary must not carry a currency figure: {line}"
    );
    assert!(!line.to_lowercase().contains("usd"));
    assert!(line.contains("token"));
    let _ = std::fs::remove_dir_all(&dir);
}

// ── G2: alloc-free count path ──────────────────────────────────────────────
// Lives in its own test binary (`tests/kat_meter_g2_alloc.rs`): the global
// CountingAllocator counts EVERY thread's allocations, so parallel sibling
// tests would pollute the before/after bracket. A single-test binary has no
// siblings.

// ── T0.6: the charge consumer (Plan 024) ────────────────────────────────
// The composition fixtures: meter + charge through the bin's exact open/record/
// settle/save/reopen cycle, at the v1 per-unit rate.

fn meter_state(cumulative: u64) -> MeterState {
    MeterState {
        version: METER_VERSION,
        cumulative_tokens: cumulative,
        sessions: 1,
    }
}

#[test]
fn g9_charge_version_is_changelog_covered_and_rides_the_file() {
    assert!(
        CHARGE_CHANGELOG.iter().any(|(v, _)| *v == CHARGE_VERSION),
        "CHARGE_VERSION {CHARGE_VERSION} has no changelog entry"
    );
    let dir =
        std::env::temp_dir().join(format!("kat_chg_stamp_{}_{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut c = KatCharge::open(&dir, &meter_state(10));
    c.settle(&meter_state(15));
    c.save().unwrap();
    let text = std::fs::read_to_string(dir.join(".heal").join(KatCharge::FILE_NAME)).unwrap();
    let state: ChargeState = serde_json::from_str(&text).unwrap();
    assert_eq!(state.charge_version, CHARGE_VERSION);
    assert_eq!(
        state.charged_kat, 15,
        "v1 rate: KAT charged == tokens charged"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn g9_charge_composition_through_the_bin_cycle() {
    // The bin's exact order: meter open → charge open (pre-session) → record
    // → settle at exit → save both → reopen. Two full cycles; the KAT charged
    // must equal the tokens billed, cumulatively, with no double-charging.
    let dir =
        std::env::temp_dir().join(format!("kat_chg_cycle_{}_{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).unwrap();
    let src_a = b"fn main() { println!(\"hello\"); }";
    let src_b = b"let x: u32 = 42;";

    // Cycle 1.
    let mut m1 = KatMeter::open(&dir);
    let mut c1 = KatCharge::open(&dir, m1.state());
    let a = m1.record_span(src_a);
    let kat1 = c1.settle(m1.state());
    m1.save().unwrap();
    c1.save().unwrap();
    assert_eq!(kat1, a, "v1 rate: KAT == tokens, per cycle");

    // Cycle 2: reopen both against pre-session state, bill more, settle.
    let mut m2 = KatMeter::open(&dir);
    let mut c2 = KatCharge::open(&dir, m2.state());
    let b = m2.record_span(src_b);
    let kat2 = c2.settle(m2.state());
    m2.save().unwrap();
    c2.save().unwrap();
    assert_eq!(kat2, b);
    assert_eq!(
        c2.state().charged_kat,
        a + b,
        "cumulative KAT == cumulative tokens at the v1 rate"
    );
    assert_eq!(c2.state().meter_watermark_tokens, a + b);

    // Cycle 3: a third open with NO new burn charges nothing.
    let m3 = KatMeter::open(&dir);
    let mut c3 = KatCharge::open(&dir, m3.state());
    assert_eq!(c3.settle(m3.state()), 0);
    assert_eq!(
        c3.state().charged_kat,
        a + b,
        "no re-charge on empty session"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn g9_charge_is_deterministic_across_repeated_settles() {
    // G9a for the charge half: same starting watermark + same meter delta →
    // same KAT, every time. The charge is a pure function of
    // (watermark, delta, rate) — N independent ledgers must agree exactly.
    let dir = std::env::temp_dir().join(format!("kat_chg_det_{}_{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..25 {
        // Each iteration: a FRESH ledger file (recovered from the meter's
        // 500-token history at open), then a 500-token delta — every one must
        // produce the identical result.
        let mut c = KatCharge::open_at(dir.join(format!("det_{i}.json")), &meter_state(500));
        assert_eq!(c.settle(&meter_state(1_000)), 500);
        assert_eq!(c.state().charged_kat, 1_000, "recovery(500) + settle(500)");
        assert_eq!(c.state().meter_watermark_tokens, 1_000);
    }
    // And the persisted-and-reopened form agrees with the fresh form.
    let file = dir.join("persisted.json");
    let mut a = KatCharge::open_at(file.clone(), &meter_state(500));
    assert_eq!(a.settle(&meter_state(1_000)), 500);
    a.save().unwrap();
    let mut b = KatCharge::open_at(file, &meter_state(1_000));
    assert_eq!(b.settle(&meter_state(1_200)), 200);
    assert_eq!(
        b.state().charged_kat,
        1_200,
        "recovered(500) + settled(500+200)"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn g9_charge_rate_arithmetic_matches_the_independent_mirror() {
    // Test-side mirror computed independently — pins the u128 mul-div
    // arithmetic and the v1 identity.
    let mirror = |t: u64| {
        u64::try_from((u128::from(t) * u128::from(CHARGE_KAT_PER_MILLION_TOKENS)) / 1_000_000)
            .unwrap_or(u64::MAX)
    };
    for t in [
        0u64,
        1,
        2,
        999,
        1_000,
        1_000_000,
        1_000_001,
        u64::MAX / 2,
        u64::MAX,
    ] {
        assert_eq!(
            kat_for_tokens(t),
            mirror(t),
            "rate arithmetic diverged at {t}"
        );
    }
    // v1 pin: the rate IS 1 KAT per token — identity for every input.
    assert_eq!(CHARGE_KAT_PER_MILLION_TOKENS, 1_000_000);
    assert_eq!(kat_for_tokens(123_456_789), 123_456_789);
}
