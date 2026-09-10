//! Every long flag the `cargo heal` usage text advertises must have a
//! parser arm — Plan 064, filed from a real defect.
//!
//! `pay --request` was in the usage block from T1.3 (Plan 062) and was
//! **not** in the parser, so the documented invocation printed
//! `unknown flag "--request"` and the whole usage page. Nothing caught it:
//! the flag's absence is silent in every compile, every clippy state and
//! every unit test, because the usage text is a string literal and the
//! parser is a `match` — two structures that never have to agree.
//!
//! This is a source-text audit rather than a behavioural one, because the
//! parser calls `usage()` (which `exit`s) on an unknown flag, so it cannot
//! be driven from a unit test without restructuring `main`. Auditing the
//! text is weaker than auditing behaviour and stronger than nothing; it
//! catches exactly the drift class that occurred.
//!
//! **The instrument checks itself first** (`selftest`-style): if either
//! extraction returns an implausibly small set the test FAILS rather than
//! reporting parity over nothing — a regex regression here would otherwise
//! print a confident green over zero flags, which is the
//! `bench_doc_audit` lesson applied.

use std::collections::BTreeSet;

/// Flags that appear in the usage text but are deliberately NOT heal's own
/// parser arms. Keep this list short and justified — every entry is a hole
/// in the audit.
const NOT_HEAL_FLAGS: &[&str] = &[
    // `--features <set>` in the usage text refers to CARGO's flag (the
    // "needs --features kat_service" hints), never a flag heal parses.
    "--features",
];

fn heal_source() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bin/heal.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// Long flags mentioned inside `fn usage()`'s body.
fn usage_flags(src: &str) -> BTreeSet<String> {
    let start = src
        .find("fn usage()")
        .expect("heal.rs must define `fn usage()` — the audit has nothing to read without it");
    // The body ends at the first column-0 `}` after the signature.
    let end = src[start..]
        .find("\n}\n")
        .map(|o| start + o)
        .expect("`fn usage()` must be closed at column 0");
    scan_flags(&src[start..end])
}

/// Long flags the parser matches (`"--flag" =>` and `"--flag" | ...`).
fn parser_flags(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    // Deliberately scans the WHOLE file rather than one function: the
    // parser arms have moved between helpers before, and an audit that
    // hard-codes their home would go quietly vacuous when they move again
    // (the "extraction hollows out path guards" lesson).
    for (i, _) in src.match_indices("\"--") {
        let rest = &src[i + 1..];
        let Some(close) = rest.find('"') else {
            continue;
        };
        let flag = &rest[..close];
        if !is_long_flag(flag) {
            continue;
        }
        // Only count it when it is being MATCHED, not merely mentioned in a
        // message. Three arm shapes, and the third was a real blind spot:
        // `"--flag" =>`, `"--flag" | "--other" =>`, and a GUARDED arm
        // `"--flag" if <cond> =>`. Missing the guard shape made
        // `--dry-run` — parsed at `"--dry-run" if matches!(args.command,
        // Some(KatCommand::Wetrun))` — read as advertised-but-unparsed, so
        // the parity assertion failed on a flag that works. An extractor
        // that cannot see a legal arm shape reports a confident FALSE
        // finding, which is the same defect class as reporting a green
        // over nothing.
        let after = rest[close + 1..].trim_start();
        if after.starts_with("=>") || after.starts_with('|') || after.starts_with("if ") {
            out.insert(flag.to_string());
        }
    }
    out
}

fn is_long_flag(s: &str) -> bool {
    s.len() > 2
        && s.starts_with("--")
        && s[2..]
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn scan_flags(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for token in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')) {
        if is_long_flag(token) {
            out.insert(token.trim_end_matches('-').to_string());
        }
    }
    out
}

/// The instrument's own pin: both extractions must find a plausible
/// population, and the extractor must recognise the shapes it depends on.
#[test]
fn the_audit_can_see_what_it_audits() {
    let src = heal_source();
    let usage = usage_flags(&src);
    let parser = parser_flags(&src);
    assert!(
        usage.len() >= 20,
        "the usage extractor found only {} flag(s) — it regressed, and parity over nothing \
         is a false green",
        usage.len()
    );
    assert!(
        parser.len() >= 20,
        "the parser extractor found only {} flag(s) — it regressed",
        parser.len()
    );
    // Shape pins: flags whose ARM SHAPES this extractor must handle.
    // `--dry-run` is the guarded-arm pin (`"--dry-run" if matches!(…) =>`) —
    // it is here because the extractor did not recognise that shape and
    // produced a false "advertised but unparsed" finding.
    // `--unmine` is the plain-arm pin for the consent tier (owner mandate
    // 2026-09-08): a tier the usage text stops advertising is a tier
    // nobody can find.
    for f in ["--suggest", "--fix", "--history", "--dry-run", "--unmine"] {
        assert!(usage.contains(f), "usage extractor missed {f}");
        assert!(parser.contains(f), "parser extractor missed {f}");
    }
    // And a token that must NOT be read as a flag.
    assert!(!is_long_flag("--"));
    assert!(!is_long_flag("--Fix"), "flags are lowercase on this CLI");
}

/// The parity assertion itself.
#[test]
fn every_advertised_flag_has_a_parser_arm() {
    let src = heal_source();
    let usage = usage_flags(&src);
    let parser = parser_flags(&src);
    let missing: Vec<&String> = usage
        .iter()
        .filter(|f| !parser.contains(*f) && !NOT_HEAL_FLAGS.contains(&f.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "the usage text advertises flag(s) the parser rejects: {missing:?} — either add the \
         parser arm or stop advertising them (this is how `pay --request` shipped broken)"
    );
}

/// The other direction is a WARNING, not a failure: an unadvertised flag
/// can be a deliberate internal escape hatch. Printed so it is visible.
#[test]
fn unadvertised_parser_flags_are_reported() {
    let src = heal_source();
    let usage = usage_flags(&src);
    let extra: Vec<String> = parser_flags(&src)
        .into_iter()
        .filter(|f| !usage.contains(f))
        .collect();
    if !extra.is_empty() {
        eprintln!("note: parser flags absent from the usage text: {extra:?}");
    }
}
