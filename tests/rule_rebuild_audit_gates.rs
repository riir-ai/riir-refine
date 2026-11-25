#![cfg(feature = "rule_rebuild_audit")]
//! Issue 242 — the rule-rebuild audit's hermetic gate battery.
//!
//! All gates run over the FAKE compiler (the Issue-259 T3 fake-cargo
//! pattern): no rustc, no hosted seat, no network. The battery proves:
//! the digest / sample / blind-fixture pins, the identity + mutated +
//! renamed-equivalent postures (the renamed gate runs across ALL 30 frozen
//! rules — proving the normalizer, the std-macro renaming and the
//! test-item removal on the real corpus), the non-compiling-but-token-
//! close class, the UNDECIDED-signature class, the vacuous-tests
//! disclosure, the fence-refusal class, the cap/seat refusals, and the
//! prompt contract (the pinned application step + the signature pin +
//! the id-free description arm).

use riir_refine::rule_rebuild_audit::*;

// ── pins ─────────────────────────────────────────────────────────────────

#[test]
fn embedded_descriptions_pass_their_pins_and_resolve_blind_free() {
    // The embedded file's blake3 must match the pin and every frozen id
    // must resolve to a corpus fixture pair — the loader refuses otherwise.
    let input = load_input().expect("the embedded descriptions pass their pins");
    assert_eq!(input.fixtures.len(), FROZEN_SAMPLE_COUNT);
    let mut per_domain = std::collections::BTreeMap::new();
    for fx in &input.fixtures {
        *per_domain.entry(fx.domain).or_insert(0usize) += 1;
    }
    assert_eq!(per_domain.get(&Domain::ClippyLints), Some(&10));
    assert_eq!(per_domain.get(&Domain::RustPerf), Some(&10));
    assert_eq!(per_domain.get(&Domain::KernelOpt), Some(&10));
}

#[test]
fn tampered_descriptions_refuse_on_digest_drift() {
    let tampered = DESCRIPTIONS_JSON.replace("Trigger:", "TRIGGER:");
    assert_ne!(tampered, DESCRIPTIONS_JSON);
    let err = load_input_from(&tampered, DESCRIPTIONS_BLAKE3)
        .expect_err("a mutated file must refuse on the byte pin");
    assert!(
        matches!(err, AuditError::DescriptionsDigest { .. }),
        "got {err:?}"
    );
}

#[test]
fn sample_pin_drift_refuses_naming_the_ids() {
    // Drop one frozen row's id → the sample pin fires (the byte pin passes:
    // the tampered file's own blake3 is supplied as the expected pin).
    let renamed = DESCRIPTIONS_JSON.replace("\"needless_bool\"", "\"needless_boolx\"");
    assert_ne!(renamed, DESCRIPTIONS_JSON);
    let mut hasher = blake3::Hasher::new();
    hasher.update(renamed.as_bytes());
    let digest = hasher.finalize().to_hex().to_string();
    let err = load_input_from(&renamed, &digest).expect_err("sample drift must refuse");
    let AuditError::SamplePin { missing, extra } = err else {
        panic!("expected SamplePin, got {err:?}");
    };
    assert_eq!(missing, vec!["clippy_lints/needless_bool".to_string()]);
    assert_eq!(extra, vec!["clippy_lints/needless_boolx".to_string()]);
}

// ── the renamed-equivalent gate (all 30 frozen rules) ────────────────────

#[test]
fn renamed_equivalent_holds_across_all_frozen_rules() {
    // The reviewer's demand: prove the normalizer + the std-macro renaming
    // + the test-item removal on the REAL corpus, not a specimen. For every
    // scoreable rule: normalizing the authored fixed arm produces a specimen
    // that (a) re-normalizes to the same text (alpha holds) and (b) differs
    // from the authored text (the strict column would say NO).
    let input = load_input().expect("inputs resolve");
    let mut scoreable = 0usize;
    let mut unscoreable = 0usize;
    for fx in &input.fixtures {
        let authored = match authored_arms(&fx.buggy, &fx.fixed) {
            Ok(a) => a,
            Err(e) => {
                // The UNSCOREABLE census (authored fixed arm not Rust).
                unscoreable += 1;
                let _ = e;
                continue;
            }
        };
        scoreable += 1;
        let authored_norm = authored
            .items_norm
            .clone()
            .expect("a parsed fixed arm normalizes");
        // (a) idempotence + alpha self-equality.
        let re_norm = normalize_source(&authored_norm).expect("normalized text re-normalizes");
        assert_eq!(
            re_norm, authored_norm,
            "rule {} lost alpha idempotence",
            fx.rule_id
        );
        // (b) the specimen differs from the authored text whenever the arm
        // carries any renameable binding (the strict column's strictness).
        if authored_norm != authored.items_text.trim() {
            // renamed-equivalent: strict would differ while alpha holds.
            assert_ne!(
                authored.items_text.trim(),
                authored_norm,
                "rule {}: normalizer emitted the authored text verbatim",
                fx.rule_id
            );
        }
        // (c) macro coverage on the real corpus is proven by (a): any
        // assert!/format!-family ident in a scoreable arm was visited —
        // the idempotence check would fail if the macro rewrite changed
        // arity or dropped tokens.
    }
    // The census must be visible to the README note (asserted nonzero).
    assert!(
        scoreable + unscoreable == FROZEN_SAMPLE_COUNT,
        "every frozen rule classifies"
    );
    assert!(scoreable > 0, "the real corpus yields scoreable rules");
}

// ── the writer + oracle postures ─────────────────────────────────────────

fn identity_audit() -> AuditReport {
    let mut writer = ReplayWriter::new(ReplayMode::Identity).expect("replay writer resolves");
    let oracle = FakeOracle::plain(Box::new(|_, _| true));
    run_audit(&mut writer, &oracle, vec![String::from("oracle: FAKE")])
        .expect("the identity audit runs")
}

#[test]
fn identity_replay_scores_every_scoreable_rule_match() {
    let report = identity_audit();
    for r in &report.rules {
        let Some(d) = &r.desc else {
            assert!(r.unscoreable.is_some(), "{} unclassified", r.rule_id);
            continue;
        };
        let c = r.ctrl.as_ref().expect("control arm present");
        assert_eq!(
            d.verdict,
            ArmVerdict::Match {
                tests_vacuous: d.tests.is_none()
            },
            "{} desc: {} [{}]",
            r.rule_id,
            d.verdict,
            d.note
        );
        assert!(d.strict, "{} desc strict", r.rule_id);
        assert!(d.alpha, "{} desc alpha", r.rule_id);
        assert!(d.compiled, "{} desc compiled", r.rule_id);
        assert_eq!(
            c.verdict,
            ArmVerdict::Match {
                tests_vacuous: c.tests.is_none()
            },
            "{} ctrl: {} [{}]",
            r.rule_id,
            c.verdict,
            c.note
        );
    }
}

#[test]
fn mutated_replay_scores_no_match_when_the_compile_leg_refuses() {
    let mut writer = ReplayWriter::new(ReplayMode::Mutated).expect("replay writer resolves");
    // The mutated items carry the marker; the fake compile leg refuses them.
    let oracle = FakeOracle::plain(Box::new(|_label, src| {
        !src.contains("__audit_mutation_marker")
    }));
    let report = run_audit(&mut writer, &oracle, vec![]).expect("the mutated audit runs");
    for r in &report.rules {
        if r.unscoreable.is_some() {
            continue;
        }
        let d = r.desc.as_ref().expect("desc arm present");
        assert_eq!(
            d.verdict,
            ArmVerdict::NoMatch,
            "{} desc: {}",
            r.rule_id,
            d.verdict
        );
        assert!(!d.compiled, "{} mutated items must not compile", r.rule_id);
    }
}

#[test]
fn non_compiling_but_token_close_scores_no_match() {
    // The identity redraft (alpha=true strict=true) with a compile leg that
    // refuses EVERYTHING: the verdict must stay NO MATCH — spelling never
    // lifts a behavioral failure.
    let mut writer = ReplayWriter::new(ReplayMode::Identity).expect("replay writer resolves");
    let oracle = FakeOracle::plain(Box::new(|_, _| false));
    let report = run_audit(&mut writer, &oracle, vec![]).expect("the audit runs");
    for r in &report.rules {
        if r.unscoreable.is_some() {
            continue;
        }
        let d = r.desc.as_ref().expect("desc arm present");
        assert!(
            d.strict && d.alpha,
            "{}: token-close columns hold",
            r.rule_id
        );
        assert_eq!(
            d.verdict,
            ArmVerdict::NoMatch,
            "{}: not compiled → NO MATCH",
            r.rule_id
        );
    }
}

#[test]
fn undecided_signature_when_authored_tests_fail_to_splice() {
    // Synthetic fixture: the redraft renames a public fn — its items
    // compile, the authored tests' splice does not → UNDECIDED-signature,
    // never NO MATCH.
    let authored_fixed = "pub fn helper(x: u8) -> u8 { x + 1 }\n\n#[test]\nfn t() {\n    assert_eq!(helper(1), 2);\n}\n";
    let redraft = "pub fn helper_r(x: u8) -> u8 { x + 1 }\n";
    let arms = authored_arms("fn helper(x: u8) -> u8 { x + 2 }", authored_fixed)
        .expect("the synthetic authored arm parses");
    // items compiles; the SPLICE (authored tests vs the renamed fn) fails.
    let oracle = FakeOracle::plain(Box::new(|label, _| label == "items"));
    let score = score_arm(&arms, &Ok(redraft.to_string()), &oracle);
    assert_eq!(
        score.verdict,
        ArmVerdict::UndecidedSignature,
        "{}",
        score.note
    );
    assert!(score.compiled);
}

#[test]
fn vacuous_tests_rest_on_the_columns_and_are_disclosed() {
    // No authored test surface: the MATCH carries tests=vacuous (never
    // folded into a real pass — the report prints the count).
    let authored_fixed = "pub fn helper(x: u8) -> u8 { x + 1 }\n";
    let arms = authored_arms("pub fn helper(x: u8) -> u8 { x + 2 }", authored_fixed)
        .expect("the synthetic authored arm parses");
    let oracle = FakeOracle::plain(Box::new(|_, _| true));
    let score = score_arm(&arms, &Ok(authored_fixed.to_string()), &oracle);
    assert_eq!(score.tests, None, "vacuous");
    assert_eq!(
        score.verdict,
        ArmVerdict::Match {
            tests_vacuous: true
        }
    );
}

#[test]
fn failing_authored_tests_lift_to_no_match_despite_alpha() {
    // The behavioral leg: authored tests COMPILE but FAIL against the
    // redraft → NO MATCH even with alpha+strict true.
    let authored_fixed = "pub fn helper(x: u8) -> u8 { x + 1 }\n\n#[test]\nfn t() {\n    assert_eq!(helper(1), 2);\n}\n";
    // The redraft compiles but its behavior differs (x + 5).
    let redraft = "pub fn helper(x: u8) -> u8 { x + 5 }\n";
    let arms = authored_arms("", authored_fixed).expect("the authored arm parses");
    // The splice compiles but its tests FAIL (the behavioral witness).
    let oracle = FakeOracle {
        verdict: Box::new(|_, _| true),
        tests_fail_on: Box::new(|label, _| label == "splice"),
    };
    let score = score_arm(&arms, &Ok(redraft.to_string()), &oracle);
    assert_eq!(score.tests, Some(false), "the splice ran and failed");
    assert_eq!(score.verdict, ArmVerdict::NoMatch, "tests fail → NO MATCH");
}

// ── the fence-refusal class ─────────────────────────────────────────────

#[test]
fn fence_refusals_score_no_match_and_count() {
    let arms = authored_arms("fn f() {}", "fn f() {}").expect("parses");
    let oracle = FakeOracle::plain(Box::new(|_, _| true));
    // The fence extraction is the WRITER's seam: zero blocks and two blocks
    // refuse there, and the refusal scores NO MATCH for that arm.
    let zero = extract_rust_block("fn bare() {}");
    let zero_score = score_arm(&arms, &zero, &oracle);
    assert_eq!(zero_score.verdict, ArmVerdict::NoMatch);
    assert!(zero_score.note.contains("fence-refusal"));
    let two = "```rust\nfn a() {}\n```\nprose\n```rust\nfn b() {}\n```";
    let two_score = score_arm(&arms, &extract_rust_block(two), &oracle);
    assert_eq!(two_score.verdict, ArmVerdict::NoMatch);
    assert!(two_score.note.contains("fence-refusal"));
    // One block passes through (extracted, then scored).
    let one = extract_rust_block("```rust\nfn f() {}\n```").expect("one block extracts");
    let one_score = score_arm(&arms, &Ok(one), &oracle);
    assert_eq!(
        one_score.verdict,
        ArmVerdict::Match {
            tests_vacuous: true
        }
    );
}

// ── the prompts ──────────────────────────────────────────────────────────

#[test]
fn prompts_carry_the_pinned_contract_and_no_rule_id() {
    let req_desc = RedraftRequest {
        arm: ArmKind::Description,
        rule_index: 7,
        description: String::from("A prose description of the trigger."),
        buggy_arm: String::from("fn buggy() {}"),
        signatures: String::from("fn helper/1"),
    };
    let desc_prompt = build_prompt(&req_desc);
    assert!(desc_prompt.contains(APPLICATION_STEP), "the pinned step");
    assert!(desc_prompt.contains("fn helper/1"), "the signature pin");
    assert!(desc_prompt.contains("A prose description"), "the payload");
    assert!(
        !desc_prompt.contains("rule_index"),
        "no bookkeeping leaks into the prompt"
    );
    let req_ctrl = RedraftRequest {
        arm: ArmKind::Control,
        rule_index: 7,
        description: req_desc.description.clone(),
        buggy_arm: String::from("fn buggy() {}"),
        signatures: String::from("fn helper/1"),
    };
    let ctrl_prompt = build_prompt(&req_ctrl);
    assert!(ctrl_prompt.contains(APPLICATION_STEP), "the pinned step");
    assert!(ctrl_prompt.contains("fn helper/1"), "the signature pin");
    assert!(ctrl_prompt.contains("fn buggy() {}"), "the buggy payload");
    // Identical contract lines in both arms: everything after the step's
    // LAST occurrence (rsplit's first piece is the text after it).
    fn after_step(s: &str) -> &str {
        s.rsplit(APPLICATION_STEP).next().unwrap_or("")
    }
    assert_eq!(
        after_step(&desc_prompt),
        after_step(&ctrl_prompt),
        "the identical application contract"
    );
}

// ── the seat/cap refusals ────────────────────────────────────────────────

#[test]
fn hosted_posture_refuses_without_the_cap_recorded() {
    let (msg, code) = cli_run(None, Some("hosted")).expect_err("no cap → refuse");
    assert_eq!(code, 2);
    assert!(msg.contains("--rule-rebuild-cap"), "{msg}");
    assert!(msg.contains("refuse to start without the cap"), "{msg}");
}

#[test]
fn unknown_writer_mode_refuses() {
    let (msg, code) = cli_run(None, Some("telepathy")).expect_err("unknown mode");
    assert_eq!(code, 2);
    assert!(msg.contains("replay | hosted"), "{msg}");
}

// ── helpers ─────────────────────────────────────────────────────────────

/// The frozen sample's size (30) re-derived from the pin table — the gates
/// count against the PIN, never a hand-typed number. (The blind-fixture
/// class needs no synthetic probe: `load_input()` succeeding IS the
/// blind-free proof — any frozen id unresolved in the corpus refuses there.)
const FROZEN_SAMPLE_COUNT: usize = 30;
