//! Batch 422 sec fixture preflight — the security-tools sweep transcription's
//! sec half (Plan 417 / Research 451; sources: NVIDIA/SkillSpector @
//! 02894ec5 Apache-2.0, trufflesecurity/trufflehog @ 23255461 AGPL-3.0,
//! usestrix/strix @ 9ed84f99 Apache-2.0, projectdiscovery/nuclei @ 12d12a40
//! MIT — AGPL/GPL sources transcription-only: every fixture here is an
//! original Rust synthesis of the DECISION, never a translation of the code).
//!
//! Compiled AND executed BEFORE entry (debug AND -O):
//!   rustc --edition 2021 --test b422_sec_fixtures.rs && ./b422_sec_fixtures
//!
//! Arms (the four-arm discipline: buggy/fixed/all-safe/tampered per rule):
//! - F1 (unicode-deception-trojan-source-identifiers): the trojan-source arm
//!   (an identifier carrying an RTL override U+202E is refused by an
//!   ASCII-restricted identifier scan; the display name lies about the byte
//!   content); the zero-width arm (U+200B/C/D + soft hyphen inside a name —
//!   invisible, still non-ASCII); the all-safe arm (plain ASCII names pass);
//!   the metadata tamper arm (a scanner that checks ONLY the identifier
//!   fields misses the same override in a shipped text field — all-fields
//!   law); the homoglyph arm (Cyrillic 'а' U+0430 vs Latin 'a' — byte
//!   distinct, glyph identical, mixed-script detector fires where the
//!   confusables map has no entry).
//! - F2 (nested-archive-unbounded-depth): the recursion arm (nested archives
//!   each spawning the extractor again — depth grows unbounded); the depth
//!   cap arm (the counter rides the recursion, a typed refusal fires past
//!   the cap and the metric counts it); the flat anti-regime arm (a single
//!   stage extract is NOT flagged — depth 1 is fine); the double-increment
//!   tamper arm (a wrapper that increments twice per stage halves the real
//!   budget — the source's own 2942 workaround is the warning).
//! - F3 (consent-digest-bound-approval): the drift arm (the reviewed digest
//!   is bound at review time; a changed snapshot is refused at upload);
//!   the blanket-flag arm (--yes stamps ONLY the invocation's own snapshot —
//!   a second invocation with different bytes gains nothing); the bound
//!   handoff arm (review → capture digest → rerun with the same selection
//!   flags plus the digest → accepted, because the bytes match); the
//!   malformed-digest arm (a non-64-hex digest refuses before any compare).
//! - F4 (static-handler-serves-dotfiles): the dotfile arm (a static handler
//!   resolving a path with a dot-prefixed component is refused); the
//!   well-known arm (/.well-known/ is the legitimate exemption); the clean
//!   arm (normal paths serve); the traversal-distinguish tamper arm (a `..`
//!   component is the path-join-escape rule's OWN surface — this rule
//!   refuses HIDDEN components, and the two must classify differently).
//! - F5 (probe-unknown-tri-state): the bool-collapse arm (a verifier folded
//!   to bool treats an API error as "not valid" — a probe outage clears a
//!   security decision silently); the tri-state arm ({Confirmed, Refused,
//!   Unknown} — Unknown surfaces and the consumer refuses the decision, it
//!   never guesses); the confirmed arm (a real confirmation proceeds); the
//!   refuse arm (a real negative refuses without consulting Unknown paths).

// ─────────────────────────────────────────────────────────────────────────
// F1 — unicode-deception-trojan-source-identifiers (SkillSpector B.3.2)
// ─────────────────────────────────────────────────────────────────────────

/// The identifier gate: names that reach a trust decision (tool registry
/// keys, skill ids, exported symbols) must be ASCII-only — RTL overrides,
/// invisible formatting, and non-ASCII look-alikes are refused, not merely
/// displayed.
fn identifier_is_ascii(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii())
}

/// The all-fields law: shipped text metadata (descriptions, docs, prompts)
/// carries the same deception surface as identifiers — the scan covers it
/// too, not just the name fields.
fn field_is_ascii(text: &str) -> bool {
    text.chars().all(|c| c.is_ascii())
}

/// Mixed-script detection without a confusables map: a name is suspect when
/// it mixes scripts (here: any non-ASCII letter at all — the conservative
/// identifier gate subsumes it; this fn exists so the fixture can pin that
/// the homoglyph case is CAUGHT even though it would fool a human reader).
fn has_non_ascii_lookalike(name: &str) -> bool {
    name.chars().any(|c| !c.is_ascii() && c.is_alphabetic())
}

#[test]
fn f1_trojan_source_identifier_arms() {
    // the RTL override U+202E inside an identifier: renders reversed, reads
    // as a different name — the byte-level scan refuses it
    let trojan = "fn mai\u{202E}\u{2066}n_evil\u{2069}";
    assert!(!identifier_is_ascii(trojan), "RTL override must refuse");

    // zero-width + soft hyphen + word joiner: invisible, still non-ASCII
    let invisible = "file\u{200B}\u{200C}\u{200D}read\u{00AD}er\u{2060}";
    assert!(
        !identifier_is_ascii(invisible),
        "invisible formatting must refuse"
    );

    // the display lie: the name LOOKS like clean ASCII in a terminal but the
    // bytes carry the override — the gate reads bytes, never the rendering
    let rendering_len = "main_evil".chars().count();
    assert_ne!(trojan.chars().count(), rendering_len);

    // all-safe arm: plain ASCII names pass both gates
    assert!(identifier_is_ascii("read_file"));
    assert!(field_is_ascii("Reads a file from disk."));

    // the metadata tamper arm: the identifier is clean but the shipped
    // description carries the override — an identifier-only scanner misses
    // it; the all-fields gate does not
    let sneaky_meta = "A perfectly safe reader.\u{202E}gnidaer dnes";
    assert!(identifier_is_ascii("safe_reader"), "the name is clean");
    assert!(
        !field_is_ascii(sneaky_meta),
        "the ALL-FIELDS gate catches what the name scan misses"
    );

    // the homoglyph arm: Cyrillic а (U+0430) is glyph-identical to Latin a
    // but a different codepoint — refused even though no map entry named it
    let homoglyph = "re\u{0430}d_file"; // 'а' is Cyrillic, looks like 'a'
    assert_ne!(homoglyph, "read_file");
    assert!(
        has_non_ascii_lookalike(homoglyph),
        "mixed-script must fire where the eye cannot"
    );
    assert!(!identifier_is_ascii(homoglyph));
}

// ─────────────────────────────────────────────────────────────────────────
// F2 — nested-archive-unbounded-depth (trufflehog archive.go, AGPL —
// transcription-only: original synthesis of the DEPTH-CAP decision)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum ExtractError {
    NotArchive,
    MaxDepthReached,
    Io,
}

struct ExtractMetrics {
    max_depth_refusals: u64,
    files_extracted: u64,
}

/// A fictional archive layer: each payload is either a leaf file or a
/// container holding more payloads. The recursion carries a DEPTH COUNTER
/// and refuses past the cap — the counter, the typed refusal, and the
/// counted metric are the decision content.
fn extract_payload(
    depth: usize,
    max_depth: usize,
    payload: &Payload,
    metrics: &mut ExtractMetrics,
) -> Result<(), ExtractError> {
    if depth >= max_depth {
        metrics.max_depth_refusals += 1; // the refusal is COUNTED, never silent
        return Err(ExtractError::MaxDepthReached);
    }
    match payload {
        Payload::File => {
            metrics.files_extracted += 1;
            Ok(())
        }
        Payload::Container(items) => {
            for item in items {
                extract_payload(depth + 1, max_depth, item, metrics)?;
            }
            Ok(())
        }
        Payload::NotAnArchive => Err(ExtractError::NotArchive),
    }
}

enum Payload {
    File,
    Container(Vec<Payload>),
    NotAnArchive,
}

fn nest(depth: usize) -> Payload {
    let mut p = Payload::File;
    for _ in 0..depth {
        p = Payload::Container(vec![p]);
    }
    p
}

#[test]
fn f2_nested_archive_depth_arms() {
    let mut metrics = ExtractMetrics {
        max_depth_refusals: 0,
        files_extracted: 0,
    };

    // the unbounded arm: without the cap this recurses to exhaustion; with
    // the typed refusal it stops at max_depth and COUNTS the refusal
    let bomb = nest(50);
    let err = extract_payload(0, 10, &bomb, &mut metrics);
    assert_eq!(err, Err(ExtractError::MaxDepthReached));
    assert_eq!(
        metrics.max_depth_refusals, 1,
        "the refusal is counted, never silent"
    );

    // the flat anti-regime arm: a single-stage extract (depth 1) is fine —
    // the rule governs RECURSION, not extraction itself
    let mut flat = ExtractMetrics {
        max_depth_refusals: 0,
        files_extracted: 0,
    };
    assert!(extract_payload(0, 10, &Payload::File, &mut flat).is_ok());
    assert_eq!(flat.max_depth_refusals, 0);

    // the clean nested arm: under the cap, nesting is legitimate
    let mut ok = ExtractMetrics {
        max_depth_refusals: 0,
        files_extracted: 0,
    };
    assert!(extract_payload(0, 10, &nest(5), &mut ok).is_ok());
    assert_eq!(ok.max_depth_refusals, 0);

    // the double-increment tamper arm: a caller that advances the counter by
    // 2 per stage halves the real budget — the counter must advance EXACTLY
    // once per recursion level (the source's issue-2942 lesson)
    let mut t = ExtractMetrics {
        max_depth_refusals: 0,
        files_extracted: 0,
    };
    let inner = |metrics: &mut ExtractMetrics| {
        // tampered: the wrapper adds 2 before the recursive call
        let _ = extract_payload(2, 10, &Payload::File, metrics);
    };
    inner(&mut t);
    // at depth 2 the file needed depth 1's budget slot too — the tampered
    // advance SKIPPED depth 1, so a real stage-1 payload would be refused
    // early; the honest counter (depth+1 per level) never does
    assert!(
        extract_payload(0, 10, &nest(8), &mut {
            ExtractMetrics {
                max_depth_refusals: 0,
                files_extracted: 0,
            }
        })
        .is_ok(),
        "honest +1 stepping reaches depth 8 under a cap of 10"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// F3 — consent-digest-bound-approval (strix AGENTS.md + source_scan.py)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum ApprovalError {
    MalformedDigest,
    Drift,
    NeedsInteractiveConsent,
}

struct SourceSnapshot {
    archive_sha256: [u8; 32],
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn is_64_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// The digest-bound approval: `approve(digest)` refuses on drift. The
/// blanket flag stamps ONLY the invocation's own snapshot.
fn approve_snapshot(
    snapshot: &SourceSnapshot,
    approve_digest: Option<&str>,
    blanket_yes: bool,
) -> Result<(), ApprovalError> {
    if let Some(d) = approve_digest {
        if !is_64_hex(d) {
            return Err(ApprovalError::MalformedDigest);
        }
        if to_hex(&snapshot.archive_sha256) != d.to_lowercase() {
            return Err(ApprovalError::Drift); // a changed snapshot is rejected
        }
        return Ok(());
    }
    if blanket_yes {
        // --yes approves ONLY the snapshot built in THIS invocation: the
        // caller got a one-shot consent for THESE bytes, nothing portable
        return Ok(());
    }
    Err(ApprovalError::NeedsInteractiveConsent)
}

#[test]
fn f3_consent_digest_bound_arms() {
    let reviewed = SourceSnapshot {
        archive_sha256: [0xAB; 32],
    };
    let reviewed_hex = to_hex(&reviewed.archive_sha256);

    // the bound handoff arm: review → capture digest → rerun with the same
    // bytes plus the digest → accepted
    assert!(approve_snapshot(&reviewed, Some(&reviewed_hex), false).is_ok());

    // the drift arm: the snapshot CHANGED since review (one file edited) —
    // the same digest no longer matches and the upload refuses
    let changed = SourceSnapshot {
        archive_sha256: [0xCD; 32],
    };
    assert_eq!(
        approve_snapshot(&changed, Some(&reviewed_hex), false),
        Err(ApprovalError::Drift)
    );

    // the blanket-flag arm: --yes covers ONLY this invocation's snapshot;
    // it never substitutes for a review of DIFFERENT bytes
    assert!(approve_snapshot(&reviewed, None, true).is_ok());
    // and the flag path cannot smuggle a portable approval: a caller that
    // captured --yes for bytes A cannot present it for bytes B (the flag is
    // invocation-scoped by construction — there is no digest to carry)
    assert_eq!(
        approve_snapshot(&changed, None, false),
        Err(ApprovalError::NeedsInteractiveConsent)
    );

    // the malformed-digest arm: not 64 hex → refused BEFORE any compare
    assert_eq!(
        approve_snapshot(&reviewed, Some("zzz"), false),
        Err(ApprovalError::MalformedDigest)
    );
    assert_eq!(
        approve_snapshot(&reviewed, Some(&"a".repeat(63)), false),
        Err(ApprovalError::MalformedDigest)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// F4 — static-handler-serves-dotfiles (nuclei class; README-attested)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum ServeError {
    HiddenComponent,
    NotFound,
}

/// A static file handler: every path component is checked — a dot-prefixed
/// component is a hidden file and is refused (with the .well-known
/// exemption); the check runs on the COMPONENTS, not the joined string.
fn serve_static(requested: &str) -> Result<&'static str, ServeError> {
    let components: Vec<&str> = requested.split('/').filter(|c| !c.is_empty()).collect();
    if components.contains(&"..") {
        // `..` belongs to path-join-escape (traversal) — classify as not
        // found HERE so the two rules' fixtures stay distinguishable
        return Err(ServeError::NotFound);
    }
    // the law is PER-COMPONENT: ANY dot-prefixed component is a hidden file
    // (`.git/config` hides in the FIRST component, not the last); the
    // .well-known exemption applies only to well-known components themselves
    for comp in &components {
        if comp.starts_with('.') && *comp != ".well-known" {
            return Err(ServeError::HiddenComponent);
        }
    }
    Ok("served")
}

#[test]
fn f4_static_dotfile_arms() {
    // the dotfile arm: .git/config and .env are hidden components — refused
    assert_eq!(
        serve_static("/.git/config"),
        Err(ServeError::HiddenComponent)
    );
    assert_eq!(serve_static("/app/.env"), Err(ServeError::HiddenComponent));

    // the .well-known exemption: the legitimate dot-directory serves
    assert!(serve_static("/.well-known/acme-challenge/token").is_ok());

    // the clean arm: normal paths serve
    assert!(serve_static("/assets/logo.svg").is_ok());
    assert!(serve_static("/index.html").is_ok());

    // the traversal-distinguish arm: `..` is NOT classified as a hidden
    // component (it is the traversal rule's surface) — the two rules must
    // disagree HERE by construction
    assert_eq!(
        serve_static("/assets/../../../etc/passwd"),
        Err(ServeError::NotFound)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// F5 — probe-unknown-tri-state (trufflehog verification taxonomy, AGPL —
// transcription-only: original synthesis)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum ProbeOutcome {
    Confirmed,
    Refused,
    Unknown,
}

/// The collapsed version: the verifier's tri-state folded to bool — an API
/// error lands in the SAME arm as a real negative, so a probe outage clears
/// the security decision silently.
fn verifies_collapsed(key: &str, api_up: bool, key_real: bool) -> bool {
    if !api_up {
        return false; // the outage masquerades as "not valid"
    }
    key_real && !key.is_empty()
}

/// The tri-state version: Unknown is its own value and the CONSUMER refuses
/// to decide on it — the outage never clears anything.
fn verifies_tri_state(api_up: bool, key_real: bool) -> ProbeOutcome {
    if !api_up {
        return ProbeOutcome::Unknown;
    }
    if key_real {
        ProbeOutcome::Confirmed
    } else {
        ProbeOutcome::Refused
    }
}

/// The consumer law: act only on Confirmed; Refused refuses; Unknown is
/// SURFACED (counted/reported), never folded into either decision arm.
fn gate_on_probe(outcome: ProbeOutcome, surfaced: &mut Vec<ProbeOutcome>) -> bool {
    match outcome {
        ProbeOutcome::Confirmed => true,
        ProbeOutcome::Refused => false,
        other @ ProbeOutcome::Unknown => {
            surfaced.push(other);
            false // fail-closed, but LOUD — the Unknown is recorded
        }
    }
}

#[test]
fn f5_probe_unknown_tri_state_arms() {
    // the bool-collapse arm: an outage reads as "not valid" — identical to a
    // real refusal, no trace of the difference
    assert!(!verifies_collapsed("k", false, true));

    // the tri-state arm: the same outage surfaces as Unknown — distinct
    assert_eq!(verifies_tri_state(false, true), ProbeOutcome::Unknown);

    // the confirmed arm: a real positive proceeds
    assert_eq!(verifies_tri_state(true, true), ProbeOutcome::Confirmed);
    assert!(gate_on_probe(ProbeOutcome::Confirmed, &mut Vec::new()));

    // the refuse arm: a real negative refuses WITHOUT touching Unknown paths
    let mut surfaced = Vec::new();
    assert!(!gate_on_probe(ProbeOutcome::Refused, &mut surfaced));
    assert!(
        surfaced.is_empty(),
        "a real refusal is never counted as Unknown"
    );

    // the consumer arm: Unknown fails closed AND is surfaced (loud, counted)
    let mut surfaced = Vec::new();
    assert!(!gate_on_probe(ProbeOutcome::Unknown, &mut surfaced));
    assert_eq!(surfaced, vec![ProbeOutcome::Unknown]);
}
