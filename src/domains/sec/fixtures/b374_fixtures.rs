//! Batch 374 sec fixtures — the laya serving-boundary pair (Plan 390 /
//! Research 425): `NandhaKishorM/laya`
//! @ `416dd9ed20da2d43fc40e23712e9adceefabf634` (Apache-2.0). ORIGINAL
//! plain-Rust stand-ins — zero repo source pasted (the laws are
//! re-authored; the source quotes stay in the corpus). Compiled AND
//! executed BEFORE entry:
//! `rustc --edition 2021 --test b374_fixtures.rs && ./b374_fixtures`
//!
//! Arms per rule:
//! - F5 `request-amplification-guard-before-expensive-path`: a
//!   caller-controlled per-question option count drives per-option token
//!   work — the naive arm runs the expensive pass at ANY count (the work
//!   census explodes linearly past the cap), the guarded arm refuses with
//!   a TYPED error BEFORE any inference work (a counted census proves the
//!   refusal precedes the work), exactly-at-cap is allowed, and the
//!   refusal names the field and the cap in its Display.
//! - F6 `callable-naming-wire-fields-refused-not-dropped`: a request body
//!   field naming a server-side executable binding (`hooks`,
//!   `on_predict_start`) — the naive arm silently DROPS the field and
//!   serves without the hook (the caller believes it ran — the
//!   false-consent pin), the fixed arm refuses typed naming the field,
//!   benign unknown fields (`notes`) still pass (only capability-naming
//!   fields refuse), and the tampered arm (refuse EVERYTHING unknown)
//!   reds on the benign-field arm.

use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
// F5 — request-amplification-guard-before-expensive-path
// ═══════════════════════════════════════════════════════════════════════

const MAX_CHOICE_OPTIONS: usize = 100;

/// A typed refusal raised BEFORE the expensive path — the error names the
/// field and the cap (the caller can re-split their question).
#[derive(Debug, PartialEq)]
enum Rejection {
    TooManyOptions { field: &'static str, got: usize, cap: usize },
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::TooManyOptions { field, got, cap } => {
                write!(f, "{field}={got} exceeds the {cap}-option cap; split the question")
            }
        }
    }
}

/// The per-option work model: each option costs real scored work (a
/// folded pseudo-token pass). The census counts it.
fn option_work(options: usize, work: &mut u64) -> u64 {
    let mut acc = 0u64;
    for o in 0..options {
        let mut h = (o as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        for _ in 0..64 {
            h = h.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            acc = acc.wrapping_add(h >> 33);
        }
        *work += 64;
    }
    acc
}

/// The naive arm: no amplification guard — a caller-specified option
/// count drives the whole expensive pass.
fn serve_naive(options: usize) -> (u64, u64) {
    let mut work = 0u64;
    let answer = option_work(options, &mut work);
    (answer, work)
}

/// The guarded arm: the cap check runs FIRST and refuses TYPED — the
/// work census stays zero on the refusal path.
fn serve_guarded(options: usize) -> Result<(u64, u64), Rejection> {
    if options > MAX_CHOICE_OPTIONS {
        return Err(Rejection::TooManyOptions {
            field: "options",
            got: options,
            cap: MAX_CHOICE_OPTIONS,
        });
    }
    let mut work = 0u64;
    let answer = option_work(options, &mut work);
    Ok((answer, work))
}

// ═══════════════════════════════════════════════════════════════════════
// F6 — callable-naming-wire-fields-refused-not-dropped
// ═══════════════════════════════════════════════════════════════════════

/// Request fields. `Hook` names a server-side executable binding — the
/// caller's extension point; `Text`/`Notes` are payload.
enum Field {
    Text(String),
    Hooks(Vec<String>),
    Notes(String),
}

fn field_name(f: &Field) -> &'static str {
    match f {
        Field::Text(_) => "text",
        Field::Hooks(_) => "hooks",
        Field::Notes(_) => "notes",
    }
}

/// The capability-naming vocabulary: a wire field whose name binds a
/// server-side executable surface.
const CALLABLE_FIELDS: &[&str] = &["hooks", "on_predict_start", "on_predict_end", "hooks_raise"];

#[derive(Debug, PartialEq)]
enum ServeError {
    /// the typed refusal — names the offending field
    CallableFieldRefused(String),
}

#[derive(Debug)]
struct Served {
    ran_hook: bool,
    answer: String,
}

/// The naive arm: unknown/capability fields are silently DROPPED — the
/// caller believes its hook ran. The false-consent surface.
fn serve_naive_body(fields: &[Field]) -> Served {
    let mut text = String::new();
    for f in fields {
        match f {
            Field::Text(t) => text.push_str(t),
            // BUG: the hooks field is dropped on the floor — no refusal,
            // no execution, no trace
            _ => {}
        }
    }
    Served {
        ran_hook: false,
        answer: text,
    }
}

/// The fixed arm: a capability-naming field REFUSES typed; benign
/// unknown fields still pass.
fn serve_guarded_body(fields: &[Field]) -> Result<Served, ServeError> {
    for f in fields {
        let name = field_name(f);
        if CALLABLE_FIELDS.contains(&name) {
            return Err(ServeError::CallableFieldRefused(name.to_string()));
        }
    }
    let mut text = String::new();
    for f in fields {
        if let Field::Text(t) = f {
            text.push_str(t);
        }
    }
    Ok(Served {
        ran_hook: false,
        answer: text,
    })
}

// ═══════════════════════════════════════════════════════════════════════

#[test]
fn f5_guard_refuses_before_work_and_names_field_and_cap() {
    // the guarded arm at an adversarial count: TYPED refusal, ZERO work
    let err = serve_guarded(250_000).unwrap_err();
    assert_eq!(
        err,
        Rejection::TooManyOptions {
            field: "options",
            got: 250_000,
            cap: 100
        }
    );
    let msg = err.to_string();
    assert!(msg.contains("options") && msg.contains("100"), "the refusal names the field and the cap: {msg}");

    // and the naive arm at the same count PAYS the work (the amplification)
    let (_, naive_work) = serve_naive(250_000);
    assert_eq!(naive_work, 250_000 * 64, "the unguarded arm runs the full per-option work");
}

#[test]
fn f5_exactly_at_cap_is_allowed_and_below_runs_normally() {
    // the boundary: exactly at the cap is NOT a refusal (the cap is a
    // ceiling, not an exclusive bound)
    let (_, work_at_cap) = serve_guarded(MAX_CHOICE_OPTIONS).expect("at-cap must serve");
    assert_eq!(work_at_cap, MAX_CHOICE_OPTIONS as u64 * 64);
    let (_, work_small) = serve_guarded(3).expect("small must serve");
    assert_eq!(work_small, 3 * 64);
}

#[test]
fn f5_naive_arm_serves_any_count_without_a_refusal_path() {
    // the defect pin: the naive arm has NO refusal path at all — the
    // same 250_000-option request runs to completion (the work the cap
    // exists to bound)
    let (answer, work) = serve_naive(250_000);
    assert_eq!(work, 250_000 * 64);
    assert_ne!(answer, 0);
}

#[test]
fn f6_callable_field_refused_typed_and_named() {
    let body = vec![
        Field::Text("is this url phishing?".to_string()),
        Field::Hooks(vec!["on_predict_start".to_string()]),
    ];
    let err = serve_guarded_body(&body).unwrap_err();
    assert_eq!(
        err,
        ServeError::CallableFieldRefused("hooks".to_string()),
        "the refusal names the offending field"
    );
}

#[test]
fn f6_naive_arm_drops_the_hook_and_serves_anyway() {
    // the false-consent pin: the same body is SERVED by the naive arm —
    // the hooks field silently dropped, no refusal, no trace. The caller
    // believes its instrumentation is in place; it never ran.
    let body = vec![
        Field::Text("q".to_string()),
        Field::Hooks(vec!["on_predict_start".to_string()]),
    ];
    let served = serve_naive_body(&body);
    assert!(
        !served.ran_hook && served.answer == "q",
        "the naive arm serves without the hook — the silent drop"
    );
}

#[test]
fn f6_benign_unknown_fields_still_pass() {
    // the anti-regime: only CAPABILITY-NAMING fields refuse. A notes
    // field is payload-adjacent metadata — ignoring it is correct, and
    // the fixed arm keeps serving.
    let body = vec![
        Field::Text("q".to_string()),
        Field::Notes("cached client tag".to_string()),
    ];
    let served = serve_guarded_body(&body).expect("benign fields must pass");
    assert_eq!(served.answer, "q");
}

#[test]
fn f6_tampered_refuse_everything_reds_on_benign_fields() {
    // the tamper: refuse EVERY unknown field (not just the callable
    // vocabulary) — the benign-field arm must break, proving the
    // vocabulary gate is the load-bearing half.
    let body = vec![Field::Notes("harmless".to_string())];
    let tampered = body
        .iter()
        .try_for_each(|f| {
            if CALLABLE_FIELDS.contains(&field_name(f)) || field_name(f) != "text" {
                Err(ServeError::CallableFieldRefused(field_name(f).to_string()))
            } else {
                Ok(())
            }
        });
    assert!(tampered.is_err(), "the over-broad refusal must red on a benign field");
    // ...while the real gate passes the same body
    assert!(serve_guarded_body(&body).is_ok());
}
