//! Batch 412 sec fixture preflight — the mlxfast staged pool's sec half
//! (Plan 411 / Research 445, the Research 437 staged candidates,
//! Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923, MIT) as ONE
//! standalone file (2 rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! sources are the arena's declaration parser + its tripwire test).
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b412_sec_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 (retired-mode-named-refusal-not-deletion): the deleted-variant
//!   arm (a retired mode mis-reports as an unknown-source typo); the
//!   named-refusal arm (mode + replacement + reason); the
//!   negative-control arm (a well-formed retired declaration with the
//!   organizer's own URL is still refused — the refusal is about the
//!   SOURCE, not the URL); the distinct-diagnostics arm (unknown
//!   spelling keeps the typo diagnostic, never the retired one).
//! - F2 (entity-keyed-envelope-widen-tripwire): the per-entity arm
//!   (two entities, two independent envelopes — depth 16 permitted for
//!   one, refused for the other); the two-layer arm (structural ceiling
//!   outer, contract-membership inner — the inner set is authoritative);
//!   the widen-tripwire arm (widening the envelope turns the tripwire
//!   red — the tested envelope may tighten, never widen silently); the
//!   malformed arm (non-integer / negative / unknown key refuse before
//!   any engine spawns).
#![allow(dead_code)]

// ══════════════ F1 — retired-mode-named-refusal-not-deletion ══════════════

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum WeightSource {
    /// the only accepted source: the organizer's pinned weights
    Pinned,
    /// RETIRED: "load bytes the participant chose", spelling one
    RetiredRemote,
    /// RETIRED: the same class, spelling two
    RetiredInBranch,
}

#[derive(Debug)]
enum DeclError {
    /// a typo — the source spelling is not any known variant
    UnknownSource(String),
    /// a RETIRED mode — names the mode, the accepted replacement, the why
    RetiredSource {
        mode: &'static str,
        replacement: &'static str,
        why: &'static str,
    },
}

impl std::fmt::Display for DeclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeclError::UnknownSource(s) => write!(f, "unknown weight source '{s}'"),
            DeclError::RetiredSource {
                mode,
                replacement,
                why,
            } => write!(
                f,
                "weight source '{mode}' is retired; this track accepts {replacement} only — {why}"
            ),
        }
    }
}

/// BUGGY SHAPE: the retired variants were DELETED and the match
/// collapsed — a participant using a retired mode gets the typo
/// diagnostic, and nothing reds when someone restores a fetch path.
fn parse_deleted_variants(raw: &str) -> Result<WeightSource, DeclError> {
    match raw {
        "pinned" => Ok(WeightSource::Pinned),
        other => Err(DeclError::UnknownSource(other.to_string())),
    }
}

/// FIXED SHAPE: retired modes are KEPT as enum cases and refuse with a
/// typed error naming (a) the mode, (b) the replacement, (c) the reason.
fn parse_named_refusal(raw: &str) -> Result<WeightSource, DeclError> {
    match raw {
        "pinned" => Ok(WeightSource::Pinned),
        "remote" => Err(DeclError::RetiredSource {
            mode: "remote",
            replacement: "'pinned'",
            why: "a submission carries no drafter weights of its own",
        }),
        "in_branch" => Err(DeclError::RetiredSource {
            mode: "in_branch",
            replacement: "'pinned'",
            why: "a submission carries no drafter weights of its own",
        }),
        other => Err(DeclError::UnknownSource(other.to_string())),
    }
}

#[test]
fn f1_deleted_variant_misreports_as_typo() {
    // the defect: the retired mode reads exactly like a misspelling.
    let err = parse_deleted_variants("remote").unwrap_err();
    match err {
        DeclError::UnknownSource(s) => assert_eq!(s, "remote"),
        other => panic!("deleted variant collapsed into {other:?}"),
    }
}

#[test]
fn f1_named_refusal_names_mode_replacement_reason() {
    let err = parse_named_refusal("remote").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("remote"), "{msg}");
    assert!(msg.contains("'pinned'"), "{msg}");
    assert!(msg.contains("retired"), "{msg}");
    assert!(msg.contains("no drafter weights"), "{msg}");
    let err2 = parse_named_refusal("in_branch").unwrap_err();
    assert!(err2.to_string().contains("in_branch"));
}

#[test]
fn f1_negative_control_well_formed_retired_still_refused() {
    // the refusal is about the SOURCE, not the URL: a well-formed
    // retired declaration — valid URL shape, even the organizer's own
    // host — is still refused, because the parser is the named-refusal
    // arm regardless of what the declaration carries.
    #[allow(dead_code)]
    struct Decl<'a> {
        source: &'a str,
        url: &'a str,
    }
    let organizer_shaped = Decl {
        source: "remote",
        url: "https://weights.example.org/head.safetensors",
    };
    let res = parse_named_refusal(organizer_shaped.source);
    assert!(
        res.is_err(),
        "a well-formed retired declaration must still refuse"
    );
    assert!(matches!(res.unwrap_err(), DeclError::RetiredSource { .. }));
}

#[test]
fn f1_unknown_spelling_keeps_typo_diagnostic() {
    // the two diagnostics stay DISTINCT: a genuine typo ("pinend") must
    // never read as the retired-mode refusal.
    let typo = parse_named_refusal("pinend").unwrap_err();
    assert!(matches!(typo, DeclError::UnknownSource(_)));
    assert!(typo.to_string().contains("unknown weight source"));
    assert!(!typo.to_string().contains("retired"));
}

// ══════════════ F2 — entity-keyed-envelope-widen-tripwire ══════════════

/// One entity's admission envelope: the structural ceiling (outer
/// type/range guard) + the contract's permitted set (the tighter,
/// AUTHORITATIVE inner membership).
#[derive(Clone)]
struct Envelope {
    /// entity key — each decoder owns its OWN envelope
    entity: &'static str,
    /// the outer structural 0..ceiling guard
    structural_ceiling: u64,
    /// the inner, authoritative permitted set
    permitted: &'static [u64],
}

#[derive(Debug, PartialEq, Eq)]
enum Admit {
    Ok,
    Refused(String),
}

fn admit(env: &Envelope, raw: &str) -> Admit {
    // layer 0: it must be an integer at all
    let Ok(tokens) = raw.parse::<i64>() else {
        return Admit::Refused(format!(
            "num_speculative_tokens must be an integer (got '{raw}')"
        ));
    };
    if tokens < 0 {
        return Admit::Refused(format!("negative depth refused"));
    }
    // layer 1: the structural ceiling (outer guard)
    if tokens > i64::try_from(env.structural_ceiling).unwrap_or(i64::MAX) {
        return Admit::Refused(format!(
            "outside the permitted range 0..{} for entity {}",
            env.structural_ceiling, env.entity
        ));
    }
    // layer 2: the contract's permitted set — the AUTHORITY (0 is the
    // serial no-op and always admitted; a positive depth must be a member)
    if tokens > 0 && !env.permitted.contains(&u64::try_from(tokens).unwrap()) {
        let list: Vec<String> = env.permitted.iter().map(|d| d.to_string()).collect();
        return Admit::Refused(format!(
            "not a contract-permitted {} draft depth (permitted: {})",
            env.entity,
            list.join(", ")
        ));
    }
    Admit::Ok
}

/// THE TRIPWIRE: the tested envelope may TIGHTEN silently; WIDENING is a
/// schema change that must turn this red. Pinned at the first depth
/// ABOVE each contract set (the acceptance cases pin every permitted
/// depth; this pins the boundary).
fn widen_tripwire(env: &Envelope, first_refused_depth: u64) -> Result<(), String> {
    match admit(env, &first_refused_depth.to_string()) {
        Admit::Refused(_) => Ok(()), // the envelope still refuses the pinned depth
        Admit::Ok => Err(format!(
            "TRIPWIRE: entity {} now admits depth {} — the tested envelope was WIDENED; \
             that is a schema change and must not ride a fixture edit silently",
            env.entity, first_refused_depth
        )),
    }
}

#[test]
fn f2_per_entity_envelopes_are_independent() {
    let mtp = Envelope {
        entity: "mtp",
        structural_ceiling: 8,
        permitted: &[1, 2, 3, 4, 5, 6, 7],
    };
    let dflash = Envelope {
        entity: "dflash",
        structural_ceiling: 16,
        permitted: &[1, 2, 4, 8, 16],
    };
    // depth 16: permitted for dflash (a member), refused for mtp (past
    // its structural ceiling) — the envelopes are independent, in both
    // directions.
    assert_eq!(admit(&dflash, "16"), Admit::Ok);
    match admit(&mtp, "16") {
        Admit::Refused(msg) => assert!(msg.contains("0..8"), "{msg}"),
        other => panic!("mtp admitted a dflash-only depth: {other:?}"),
    }
    // and the INNER sets differ too: depth 3 is mtp-permitted,
    // dflash-refused (not a member of {1,2,4,8,16}).
    assert_eq!(admit(&mtp, "3"), Admit::Ok);
    match admit(&dflash, "3") {
        Admit::Refused(msg) => assert!(msg.contains("contract-permitted"), "{msg}"),
        other => panic!("dflash admitted a non-member depth: {other:?}"),
    }
}

#[test]
fn f2_two_layers_structural_then_membership() {
    let mtp = Envelope {
        entity: "mtp",
        structural_ceiling: 8,
        permitted: &[1, 2, 3, 4, 5, 6, 7],
    };
    // depth 8: INSIDE the structural 0..8 ceiling but OUTSIDE the
    // permitted set — the INNER layer refuses it, by name.
    match admit(&mtp, "8") {
        Admit::Refused(msg) => assert!(msg.contains("contract-permitted"), "{msg}"),
        other => panic!("the structural ceiling alone admitted depth 8: {other:?}"),
    }
    // depth 9: beyond the structural ceiling — refuses THERE, before the
    // membership check.
    match admit(&mtp, "9") {
        Admit::Refused(msg) => assert!(msg.contains("0..8"), "{msg}"),
        other => panic!("expected the structural refusal: {other:?}"),
    }
    // the serial no-op (0) is always admitted.
    assert_eq!(admit(&mtp, "0"), Admit::Ok);
}

#[test]
fn f2_widen_tripwire_reds_on_widening() {
    let mtp = Envelope {
        entity: "mtp",
        structural_ceiling: 8,
        permitted: &[1, 2, 3, 4, 5, 6, 7],
    };
    // the tested posture: depth 8 is refused (the pin) — tripwire green.
    assert!(widen_tripwire(&mtp, 8).is_ok());
    // TAMPER ARM: widen the envelope (a "harmless fixture edit") and the
    // tripwire turns red — the widening cannot ride silently.
    let widened = Envelope {
        entity: "mtp",
        structural_ceiling: 8,
        permitted: &[1, 2, 3, 4, 5, 6, 7, 8], // the widening
    };
    let err = widen_tripwire(&widened, 8).unwrap_err();
    assert!(err.contains("TRIPWIRE"), "{err}");
    assert!(err.contains("WIDENED"), "{err}");
}

#[test]
fn f2_malformed_refuses_before_any_engine() {
    let mtp = Envelope {
        entity: "mtp",
        structural_ceiling: 8,
        permitted: &[1, 2, 3],
    };
    match admit(&mtp, "1.5") {
        Admit::Refused(msg) => assert!(msg.contains("integer"), "{msg}"),
        other => panic!("non-integer admitted: {other:?}"),
    }
    match admit(&mtp, "-1") {
        Admit::Refused(_) => {}
        other => panic!("negative admitted: {other:?}"),
    }
    // the refusals happen at the parse seam — the fixture models that by
    // returning BEFORE any spawn (the Admit result IS the pre-spawn gate).
    assert!(matches!(admit(&mtp, "2"), Admit::Ok));
}
