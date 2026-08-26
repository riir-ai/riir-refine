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

use riir_clippy::kat_meter::{count_word_tokens, KatMeter, MeterState, METER_CHANGELOG, METER_VERSION};

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
    let text =
        std::fs::read_to_string(dir.join(".heal").join(KatMeter::FILE_NAME)).unwrap();
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
    assert_eq!(m2.state().cumulative_tokens, a, "burn persists across reopen");
    assert_eq!(m2.session_tokens(), 0, "session resets on open");
    assert_eq!(m2.state().sessions, m1.state().sessions + 1, "one session per open");
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
    assert!(!line.contains('$'), "summary must not carry a currency figure: {line}");
    assert!(!line.to_lowercase().contains("usd"));
    assert!(line.contains("token"));
    let _ = std::fs::remove_dir_all(&dir);
}

// ── G2: alloc-free count path ──────────────────────────────────────────────
// Lives in its own test binary (`tests/kat_meter_g2_alloc.rs`): the global
// CountingAllocator counts EVERY thread's allocations, so parallel sibling
// tests would pollute the before/after bracket. A single-test binary has no
// siblings.
