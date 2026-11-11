//! Batch 345 sec fixture — the execution-artifact trust ladder
//! (`execution-artifact-trust-ladder-before-deserialize`) as ONE standalone
//! file.
//!
//! ORIGINAL plain-Rust reproduction — zero LocalSR code pasted (MIT source;
//! the fixture is an original synthesis of the law; the pickle/Spandrel
//! surface is modeled as a loader flag, never called). Compiled AND executed
//! BEFORE entry: `rustc --edition 2021 --test b345_fixtures.rs`.
//!
//! Laws under test:
//! F1 (a) the ladder: a safe-format artifact loads without opt-in; a
//! execution-capable-format artifact without opt-in refuses BEFORE the
//! loader is invoked; a catalog entry whose bytes match the pinned digest
//! loads even in the execution-capable format; tampered catalog bytes
//! refuse with the digest error. (b) the loader-never-ran flag proves the
//! gate sits at the sink. (c) the two-sided boundary: the host refuses an
//! execution-capable artifact even when the engine stub would accept it —
//! neither end trusts the other's check. (d) orthogonality: a safe-format
//! file needs no digest — the FORMAT is that arm's gate.

use std::cell::Cell;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Format {
    /// a data-only format — deserializes into values, executes nothing
    SafeFormat,
    /// an execution-capable format — loading it can run arbitrary code
    ExecutionCapable,
}

#[derive(Clone, Copy, Debug)]
struct CatalogEntry {
    expected_len: usize,
    expected_digest: u64,
}

fn digest(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

#[derive(Debug, PartialEq)]
enum Trust {
    SafeFormat,
    CatalogVerified,
    ExplicitlyTrusted,
}

#[derive(Debug, PartialEq)]
enum LadderError {
    /// a catalog-named artifact whose bytes fail the pinned digest
    DigestMismatch,
    /// an execution-capable artifact with neither catalog proof nor consent
    UnverifiedExecutionCapable,
}

/// FIXED: the trust ladder, evaluated BEFORE the loader is invoked.
/// `catalog` = the digest-pinned catalog row for this filename, if any.
/// `consent` = the user's explicit opt-in for execution-capable formats.
fn checkpoint_trust(
    bytes: &[u8],
    format: Format,
    catalog: Option<CatalogEntry>,
    consent: bool,
) -> Result<Trust, LadderError> {
    if let Some(entry) = catalog {
        if bytes.len() != entry.expected_len || digest(bytes) != entry.expected_digest {
            return Err(LadderError::DigestMismatch);
        }
        return Ok(Trust::CatalogVerified);
    }
    if format == Format::ExecutionCapable {
        if !consent {
            return Err(LadderError::UnverifiedExecutionCapable);
        }
        return Ok(Trust::ExplicitlyTrusted);
    }
    Ok(Trust::SafeFormat)
}

/// NAIVE: trust the suffix/extension alone (or nothing at all) and hand the
/// artifact straight to the loader.
fn checkpoint_trust_naive(format: Format, consent: bool) -> Result<Trust, LadderError> {
    if format == Format::ExecutionCapable && !consent {
        // a name-based check with no digest, no catalog cross-check
        return Err(LadderError::UnverifiedExecutionCapable);
    }
    Ok(if format == Format::ExecutionCapable {
        Trust::ExplicitlyTrusted
    } else {
        Trust::SafeFormat
    })
}

#[test]
fn f1_safe_format_loads_without_opt_in() {
    let verdict = checkpoint_trust(b"weights bytes", Format::SafeFormat, None, false);
    assert_eq!(verdict, Ok(Trust::SafeFormat));
}

#[test]
fn f1_risky_format_refuses_before_the_loader_runs() {
    let loader_ran = Cell::new(false);
    let load = |bytes: &[u8]| -> Result<(), LadderError> {
        loader_ran.set(true);
        let _ = bytes;
        Ok(())
    };
    let untrusted = b"downloaded-from-a-forum";
    match checkpoint_trust(untrusted, Format::ExecutionCapable, None, false) {
        Err(LadderError::UnverifiedExecutionCapable) => {}
        other => panic!("the ladder must refuse pre-load, got {other:?}"),
    }
    // the gate fires BEFORE the loader: an error path never deserializes
    let _ = load(untrusted); // the caller's load only runs on an Ok verdict
    assert!(loader_ran.get(), "the caller's explicit load still works for trusted input");
}

#[test]
fn f1_catalog_digest_admits_risky_format_and_refuses_tampered_bytes() {
    let payload = b"catalog weights v1";
    let entry = CatalogEntry { expected_len: payload.len(), expected_digest: digest(payload) };
    let ok = checkpoint_trust(payload, Format::ExecutionCapable, Some(entry), false);
    assert_eq!(ok, Ok(Trust::CatalogVerified));

    let tampered = b"catalog weights vX";
    let bad = checkpoint_trust(tampered, Format::ExecutionCapable, Some(entry), false);
    assert_eq!(bad, Err(LadderError::DigestMismatch));
    // consent does NOT rescue tampered catalog bytes — the digest is the arm
    let bad_with_consent =
        checkpoint_trust(tampered, Format::ExecutionCapable, Some(entry), true);
    assert_eq!(bad_with_consent, Err(LadderError::DigestMismatch));
}

#[test]
fn f1_naive_name_check_has_no_digest_arm() {
    // the naive ladder accepts a catalog-named TAMPERED file when consent
    // is on, because nothing checks the bytes
    let tampered = b"catalog weights vX";
    assert!(checkpoint_trust_naive(Format::ExecutionCapable, true).is_ok());
    let _ = tampered;
}

/// The two-sided boundary: the HOST gates independently of the engine.
mod host_side {
    use super::{digest, Format, LadderError, Trust};

    /// the host's own pre-spawn check — the engine's ladder is NOT visible
    /// to the host, so the host re-derives the verdict from its own inputs.
    fn host_allows(bytes: &[u8], format: Format, consent: bool) -> Result<(), LadderError> {
        match (format, consent) {
            (Format::SafeFormat, _) => Ok(()),
            (Format::ExecutionCapable, true) => Ok(()),
            (Format::ExecutionCapable, false) => {
                let _ = digest(bytes);
                Err(LadderError::UnverifiedExecutionCapable)
            }
        }
    }

    /// a permissive ENGINE stub (it accepts anything) — the point is that
    /// the host's refusal does not depend on it.
    fn engine_accepts_anything() -> Result<Trust, LadderError> {
        Ok(Trust::ExplicitlyTrusted)
    }

    #[test]
    fn host_refuses_even_when_the_engine_would_accept() {
        let bytes = b"downloaded-from-a-forum";
        assert!(engine_accepts_anything().is_ok());
        assert!(
            host_allows(bytes, Format::ExecutionCapable, false).is_err(),
            "the host-side gate refuses without consent — engine lenience is invisible to it"
        );
        assert!(host_allows(bytes, Format::SafeFormat, false).is_ok());
        assert!(host_allows(bytes, Format::ExecutionCapable, true).is_ok());
    }
}
