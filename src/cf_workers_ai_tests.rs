use super::*;

// ── body/parse wiring (issue 124) ─────────────────────────
//
// The eleven build_request_body / parse_chat_result tests moved to the
// riir-llm leaf with the code (qwen suffix, glm-4 kwarg, glm-5
// reasoning_effort, both thinking-mode arms, the llama pass-through,
// reasoning_content extraction + longer-field preference, the overflow
// prefix, the stop-finish shape error, the legacy string/array
// schemas). What stays pinned HERE is the consumer's half: the
// `RIIR_CF_GLM_THINKING` env read maps to the leaf's thinking flag,
// and the client's model/model-free body contract holds.

#[test]
fn generate_billed_body_mode_maps_the_glm_thinking_env() {
    // The env read is the only policy left on this side; assert the
    // two modes produce the leaf bodies this client sends (via the
    // same fns generate_billed calls — an integration pin on the
    // dispatch, not a re-test of the leaf).
    let with_switches = riir_llm::chat_body_for(
        riir_llm::Wire::Cf,
        CfClient::DEFAULT_MODEL,
        "fix this",
        512,
        false,
    );
    assert!(
        with_switches.contains("reasoning_effort"),
        "{with_switches}"
    );
    let thinking = riir_llm::chat_body_for(
        riir_llm::Wire::Cf,
        CfClient::DEFAULT_MODEL,
        "fix this",
        512,
        true,
    );
    assert!(!thinking.contains("reasoning_effort"), "{thinking}");
    // The CF wire stays model-free (the model rides the URL path).
    for body in [with_switches, thinking] {
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(
            v.get("model").is_none(),
            "CF body must stay model-free: {body}"
        );
    }
}

#[test]
fn resolve_model_precedence_explicit_over_env_over_default() {
    // Pure core — no process-env races (parallel tests never touch it).
    let d = CfClient::DEFAULT_MODEL;
    assert_eq!(
        CfClient::resolve_model_from(
            Some("@cf/qwen/qwen3.8-27b"),
            Some("@cf/zai-org/glm-4.7-flash")
        ),
        "@cf/qwen/qwen3.8-27b",
        "explicit arg wins over env"
    );
    assert_eq!(
        CfClient::resolve_model_from(None, Some("@cf/zai-org/glm-4.7-flash")),
        "@cf/zai-org/glm-4.7-flash",
        "env wins over default"
    );
    assert_eq!(
        CfClient::resolve_model_from(None, None),
        d,
        "default falls through"
    );
    assert_eq!(
        CfClient::resolve_model_from(Some(""), Some("")),
        d,
        "empty strings are unset, not overrides"
    );
}

#[test]
fn parse_shapes_still_pin_through_the_leaf_result_walk() {
    // Consumer-side pins on the moved parse: the CF result walk is the
    // leaf's `parse_cf_result` now — assert the shapes THIS harness
    // feeds it still parse (GLM reasoning_content, overflow prefix,
    // legacy string/array), end to end through `parse_cf_chat`.
    let glm = r#"{"result":{"choices":[{"finish_reason":"stop","message":{"content":"fn fixed() {}","reasoning":"","reasoning_content":"think think"}}]}}"#;
    let (t, usage) = riir_llm::parse_cf_chat(glm).unwrap();
    assert_eq!(t, "fn fixed() {}");
    assert_eq!(usage.reasoning_chars, "think think".len());

    let starved = r#"{"result":{"choices":[{"finish_reason":"length","message":{"content":null,"reasoning":"x"}}]}}"#;
    let e = riir_llm::parse_cf_chat(starved).expect_err("overflow");
    match e {
        riir_llm::LlmFailure::Overflow(m) => {
            assert!(m.starts_with(REASONING_OVERFLOW), "got: {m}");
        }
        other => panic!("expected Overflow, got {other:?}"),
    }

    let legacy_string = r#"{"result":{"response":"legacy text"}}"#;
    let (t, _) = riir_llm::parse_cf_chat(legacy_string).unwrap();
    assert_eq!(t, "legacy text");
    let legacy_array = r#"{"result":{"response":["a","b"]}}"#;
    let (t, _) = riir_llm::parse_cf_chat(legacy_array).unwrap();
    assert_eq!(t, "ab");
}

// ── prompt_module / extract / normalize ──────────────────────────────

#[test]
fn prompt_module_strips_the_instruction() {
    let p = format!("{TRAINED_INSTRUCTION}\n\nfn a() {{}}");
    assert_eq!(prompt_module(&p), "fn a() {}");
    assert_eq!(prompt_module("fn a() {}"), "fn a() {}");
}

#[test]
fn extract_code_block_handles_fences_and_bare() {
    assert_eq!(extract_code_block("```rust\nfn a() {}\n```"), "fn a() {}");
    assert_eq!(extract_code_block("plain"), "plain");
    assert_eq!(extract_code_block("```\nunclosed"), "unclosed");
}

#[test]
fn normalize_drops_blank_lines_and_trailing_ws() {
    assert_eq!(normalize_text("a  \n\nb\n"), "a\nb");
}

#[test]
fn exact_match_is_fence_symmetric() {
    assert_eq!(
        exact_match("```rust\nfn a() {}\n```", Some("fn a() {}")),
        Some(true)
    );
    assert_eq!(exact_match("fn b() {}", Some("fn a() {}")), Some(false));
    assert_eq!(exact_match("fn a() {}", None), None);
}

// ── merge ─────────────────────────────────────────────────────────────

/// The REAL first row of v2edit chunk 00 (read from the eval corpus on
/// 2026-08-25): a two-site edit with omitted unchanged closing braces —
/// the exact shape whole-fn substitution cannot apply but the LCS merge
/// must.
const V2EDIT_ROW: &str = "fn fetch(s: &str) -> usize {\n    s.split_whitespace().count()\n}\n\nfn hash_label(v: i32) -> i64 { v as i64 }\nfn baz(x: Option<i32>) -> Option<i64> {\n    match x {\n        Some(v) => Some(hash_label(v)),\n        None => None,\n    }\n}\n\nfn review(v: &[u32]) -> Option<u32> {\n    v.iter().copied().min()\n}\n\nfn show(x: &i32) -> i32 { *x }\nfn call(x: &i32) -> i32 {\n    show(&x)\n}";
const V2EDIT_RESP: &str = "fn baz(x: Option<i32>) -> Option<i64> {\n    x.map(hash_label)\nfn call(x: &i32) -> i32 {\n    show(x)";

#[test]
fn lcs_merge_applies_the_real_v2edit_reference_row() {
    let orig: Vec<&str> = V2EDIT_ROW.lines().collect();
    let resp: Vec<&str> = V2EDIT_RESP.lines().collect();
    let merged = lcs_merge(&orig, &resp, true).expect("depth-bounded merge must apply");
    let m = merged.join("\n");
    assert!(m.contains("x.map(hash_label)"), "baz body replaced: {m}");
    assert!(m.contains("show(x)"), "call body replaced: {m}");
    assert!(!m.contains("show(&x)"), "old call body gone: {m}");
    // The untouched items survive (verbatim replacement would drop them).
    assert!(m.contains("fn fetch"));
    assert!(m.contains("fn review"));
    assert!(m.contains("fn hash_label"));
    // Replacing a `match {}` block with a single expression legitimately
    // removes a brace PAIR — the invariant is net balance, not count.
    assert_eq!(
        m.matches('{').count(),
        m.matches('}').count(),
        "net brace balance"
    );
    assert!(syn::parse_file(&m).is_ok(), "merged result must parse");
}

#[test]
fn apply_edit_picks_fn_spans_for_a_full_fn_response() {
    let module = "pub fn f(v: &Vec<u8>) -> usize { v.len() }";
    let resp = "pub fn f(v: &[u8]) -> usize { v.len() }";
    let (m, s) = apply_edit(module, resp, true).expect("must merge");
    assert_eq!(s, MergeStrategy::Verbatim);
    assert!(m.contains("&[u8]"));
}

#[test]
fn apply_edit_refuses_verbatim_for_v2edit_sources() {
    // A complete-fn generation against a multi-fn module must NOT take
    // the verbatim path (it would drop the other fns).
    let (m, s) = apply_edit(
        V2EDIT_ROW,
        "fn fetch(s: &str) -> usize {\n    s.split_whitespace().count()\n}",
        false,
    )
    .expect("must merge via a non-verbatim strategy");
    assert_ne!(s, MergeStrategy::Verbatim);
    assert!(m.contains("fn baz"), "other fns survive: {m}");
}

#[test]
fn apply_edit_fails_loudly_on_garbage() {
    assert!(apply_edit("fn a() {}", "}}}garbage{{{", true).is_none());
}

#[test]
fn apply_edit_takes_a_faithful_whole_module_rewrite() {
    // Bench 114: a whole-module response that preserves every top-level
    // item merges WHOLE — the module-applicable pool member's shape. The
    // snippet paths mangle real multi-fn rewrites (`substitute_fn_spans`
    // splits on blank lines; the LCS line-merge drops mid-item lines)
    // into parseable-but-broken modules, and the census undercounted
    // gold by a third before this strategy existed.
    let module = "use std::collections::HashMap;\n\nfn fetch(s: &str) -> usize {\n    s.split_whitespace().count()\n}\n\nfn baz() -> u8 {\n    7\n}";
    let fixed = "use std::collections::HashMap;\n\nfn fetch(s: &str) -> usize {\n    s.split_whitespace().count() + 1\n}\n\nfn baz() -> u8 {\n    8\n}";
    let (m, s) = apply_edit(module, fixed, false).expect("faithful rewrite merges");
    assert_eq!(s, MergeStrategy::Rewrite);
    assert_eq!(m, fixed);
}

#[test]
fn rewrite_gate_refuses_a_fn_snippet_for_a_multi_fn_module() {
    // The trust is the ITEM SET, not the caller's flag: a single-fn
    // response against a two-fn module parses but is not a faithful
    // rewrite — it falls through to the snippet strategies (this is the
    // corpus-snippet pool member's shape, and the content-drop shape).
    let module = "fn a() {}\nfn b() {}";
    let resp = "fn a() { fixed(); }";
    let (m, s) = apply_edit(module, resp, false).expect("falls to fn-spans");
    assert_eq!(s, MergeStrategy::FnSpans);
    assert!(m.contains("fn b"), "the other fn survives: {m}");
}

#[test]
fn rewrite_gate_allows_use_changes() {
    // `use` items are exempt from preservation: a fix may legitimately
    // add or remove imports (e.g. removing a borrow removed the last
    // use of an import).
    let module = "use std::io;\nfn a() { let _ = io::stdout(); }";
    let fixed = "fn a() {}";
    let (m, s) = apply_edit(module, fixed, false).expect("use drop is faithful");
    assert_eq!(s, MergeStrategy::Rewrite);
    assert_eq!(m, fixed);
}

#[test]
fn lcs_merge_leading_run_is_unlocatable() {
    let orig: Vec<&str> = "fn a() {\n}".lines().collect();
    let resp: Vec<&str> = "fn b() {\n}".lines().collect();
    assert!(lcs_merge(&orig, &resp, true).is_none());
}

#[test]
fn depth_bound_stops_at_the_enclosing_brace() {
    // A trailing run must not eat the rest of the file.
    let orig = "fn a() {\n    old();\n}\nfn b() {\n    keep();\n}".to_string();
    let orig_lines: Vec<&str> = orig.lines().collect();
    let resp: Vec<&str> = "fn a() {\n    new();".lines().collect();
    let merged = lcs_merge(&orig_lines, &resp, true).expect("must apply");
    let m = merged.join("\n");
    assert!(m.contains("new();"));
    assert!(m.contains("keep();"), "trailing items survive: {m}");
}

// ── prompts ────────────────────────────────────────────────────────────

#[test]
fn prompt_raw_is_the_trained_distribution() {
    let p = prompt_raw("fn a() {}");
    assert!(p.starts_with("Fix all clippy lints in this Rust code."));
    assert!(p.ends_with("fn a() {}"));
}

#[test]
fn prompt_prepass_carries_candidate_and_fewshots_deterministically() {
    let fs = vec![FewShot {
        rule_id: "manual_map".into(),
        buggy: "b".into(),
        fixed: "f".into(),
    }];
    let with = prompt_prepass("MOD", Some("CAND"), &fs);
    assert!(with.starts_with(TRAINED_INSTRUCTION));
    assert!(with.contains("MOD"));
    assert!(with.contains("CAND"));
    assert!(with.contains("example (manual_map) before"));
    assert!(with.contains("--- after ---"));
    let again = prompt_prepass("MOD", Some("CAND"), &fs);
    assert_eq!(with, again, "prompt construction must be deterministic");

    let declined = prompt_prepass("MOD", None, &[]);
    assert!(declined.contains("(no candidate"));
    assert!(!declined.contains("example ("));
}

// ── Issue 049 — the growing-input prompt ───────────────────────────

#[test]
fn prompt_growing_carries_both_channels_and_keeps_baselines_intact() {
    let fs = vec![FewShot {
        rule_id: "manual_map".into(),
        buggy: "b".into(),
        fixed: "f".into(),
    }];
    let traj = vec![TrajectoryExemplar {
        lint_key: "uninlined_format_args".into(),
        shape: "format!(\"{}\", x)".into(),
        fix: "format!(\"{x}\")".into(),
        success: 4,
        fail: 1,
        tier: "Certified",
    }];
    let patterns = vec![FailurePattern {
        rule: "match_bool".into(),
        attempts: 6,
        kept: 3,
        success_rate: 0.5,
        representative_failures: vec![(
            "match b { true => x, false => y }".into(),
            "if b { x } else { y }".into(),
        )],
    }];
    let g = prompt_growing("MOD", Some("CAND"), &fs, &traj, &patterns);
    // Channel 1: prior outcomes block.
    assert!(g.contains("Prior fix attempts on shape-similar code"));
    assert!(g.contains("uninlined_format_args [Certified: 4 ok / 1 failed]"));
    // Channel 3: failure-pattern block.
    assert!(g.contains("Historical fix-reliability by rule"));
    assert!(g.contains("match_bool: 3/6 attempts kept (50%)"));
    assert!(g.contains("REJECTED"));
    // Arm B's content carries through (channel 2 + few-shots).
    assert!(g.contains("CAND"));
    assert!(g.contains("example (manual_map) before"));
    // The closing line is last (spliced, not duplicated).
    assert!(g.ends_with("no explanation."));
    assert_eq!(g.matches("Respond with ONLY").count(), 1);
    // Deterministic.
    assert_eq!(
        g,
        prompt_growing("MOD", Some("CAND"), &fs, &traj, &patterns)
    );
    // Empty channels → byte-identical to arm B (the degradation case).
    assert_eq!(
        prompt_growing("MOD", Some("CAND"), &fs, &[], &[]),
        prompt_prepass("MOD", Some("CAND"), &fs)
    );
}

// ── Issue 067 — arm G' (same-rule channels, condition-first) ────

fn gprime_channels() -> (Vec<TrajectoryExemplar>, Vec<FailurePattern>) {
    let traj = vec![TrajectoryExemplar {
        lint_key: "ptr_arg".into(),
        shape: "fn f(v: &Vec<u8>) -> usize { v.len() }".into(),
        fix: "fn f(v: &[u8]) -> usize { v.len() }".into(),
        success: 3,
        fail: 0,
        tier: "Certified",
    }];
    let patterns = vec![FailurePattern {
        rule: "ptr_arg".into(),
        attempts: 4,
        kept: 3,
        success_rate: 0.75,
        representative_failures: vec![(
            "fn g(v: &Vec<u8>) {}".into(),
            "fn g(v: Vec<u8>) {}".into(),
        )],
    }];
    (traj, patterns)
}

#[test]
fn prompt_growing_same_rule_places_channels_before_the_module() {
    let fs = vec![FewShot {
        rule_id: "manual_map".into(),
        buggy: "b".into(),
        fixed: "f".into(),
    }];
    let (traj, patterns) = gprime_channels();
    let g = prompt_growing_same_rule("MODCODE", Some("CAND"), &fs, &traj, &patterns);
    // Condition-first: instruction header < channels < code module
    // (arm G appends the channels at the tail instead).
    let hdr = g.find(TRAINED_INSTRUCTION).expect("instruction header");
    let chan = g
        .find("Prior fix attempts on shape-similar code")
        .expect("channel 1 present");
    let module = g.find("MODCODE").expect("module present");
    assert!(
        hdr < chan && chan < module,
        "condition-first: header < channels < module"
    );
    assert!(
        module < g.find("A static modelless analyzer").unwrap(),
        "the candidate block follows the module (arm B's body intact)"
    );
    // Both channels carried; exactly one respond-line.
    assert!(g.contains("Historical fix-reliability by rule"));
    assert_eq!(g.matches("Respond with ONLY").count(), 1);
    // Deterministic.
    assert_eq!(
        g,
        prompt_growing_same_rule("MODCODE", Some("CAND"), &fs, &traj, &patterns)
    );
}

#[test]
fn prompt_growing_same_rule_empty_channels_is_byte_identical_to_arm_b() {
    let fs = vec![FewShot {
        rule_id: "manual_map".into(),
        buggy: "b".into(),
        fixed: "f".into(),
    }];
    assert_eq!(
        prompt_growing_same_rule("MOD", Some("CAND"), &fs, &[], &[]),
        prompt_prepass("MOD", Some("CAND"), &fs),
    );
    // And the arm-G byte contract still holds after the renderer
    // extraction (both builders share it).
    let (traj, patterns) = gprime_channels();
    assert_eq!(
        prompt_growing("MOD", Some("CAND"), &fs, &traj, &patterns),
        prompt_growing("MOD", Some("CAND"), &fs, &traj, &patterns),
    );
}

// ── Issue 073 — anchor-primacy + entity perturbation (XBridge) ──
//
// XBridge (arXiv:2608.11676) role separation: the DISCRETE anchor
// channel determines WHICH entities appear in output; the continuous
// channel only shapes HOW the receiver reasons about them. Applied to
// the fixer builders: identity fields (rule ids, symbol names, the
// module itself) must cross as verbatim anchors. Every builder in this
// file already does — these tests PIN it, so a refactor toward a
// paraphrasing/summarizing builder (the Bench-868 dense-gist failure
// class) cannot land silently.

/// Real frozen fixture code with distinctive symbols (`greedy_path`,
/// `marginals`) — the perturbation twin renames `greedy_path`. Inlined
/// verbatim from `score_bench::fixtures()` id
/// `katgpt-rs-bench_ptrm_width_scaling-002` (mined
/// katgpt-rs@9e0e6a57:bench_ptrm_width_scaling.rs) so these pins run
/// under `cf_workers_ai` alone; `issue073_fixture_matches_frozen_set`
/// tripwires the copy against the frozen set when `score_bench` is on.
const ISSUE073_MODULE: &str = "/// Compute greedy argmax path as baseline reference.\nfn greedy_path(marginals: &[Vec<f32>]) -> Vec<usize> {\n    marginals\n        .iter()\n        .map(|m| {\n            m.iter()\n                .enumerate()\n                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))\n                .map(|(i, _)| i)\n                .unwrap_or(0)\n        })\n        .collect()\n}\n";

fn issue073_module() -> &'static str {
    ISSUE073_MODULE
}

/// Same-kind rename — the lib helper (one implementation for the pins
/// and the Bt8 runner).
use super::perturb_symbol_ident as perturb_symbol;

#[cfg(feature = "score_bench")]
#[test]
fn issue073_fixture_matches_frozen_set() {
    let frozen = crate::score_bench::fixtures::fixtures()
        .into_iter()
        .find(|f| f.id == "katgpt-rs-bench_ptrm_width_scaling-002")
        .expect("frozen fixture present");
    assert_eq!(
        issue073_module(),
        frozen.code,
        "the inlined 073 fixture drifted from the frozen score-bench set"
    );
}

#[test]
fn anchor_primacy_all_builders_transport_identity_fields_verbatim() {
    let module = issue073_module();
    assert!(module.contains("greedy_path"), "fixture sanity");
    let fs = vec![FewShot {
        rule_id: "manual_map".into(),
        buggy: "b".into(),
        fixed: "f".into(),
    }];
    let (traj, patterns) = gprime_channels();
    let candidate = "-    x.map(f).unwrap_or(false)\n+    x.is_some_and(f)";

    // The whole module verbatim means every symbol in it is verbatim by
    // transitivity. `prompt_raw` is instruction + module ONLY (no
    // few-shots, no candidate); the other three carry the few-shot rule
    // anchor in its dedicated header position and the candidate
    // verbatim.
    assert!(
        prompt_raw(module).contains(module),
        "arm A: module crosses verbatim"
    );
    for p in [
        prompt_prepass(module, Some(candidate), &fs),
        prompt_growing(module, Some(candidate), &fs, &traj, &patterns),
        prompt_growing_same_rule(module, Some(candidate), &fs, &traj, &patterns),
    ] {
        assert!(
            p.contains(module),
            "module crosses verbatim (the anchor block)"
        );
        assert!(
            p.contains("--- example (manual_map) before ---"),
            "few-shot rule anchor position"
        );
        // The candidate is verbatim per-LINE (the builder indents each
        // line by 4) — pin every line in its indented form.
        for line in candidate.lines() {
            assert!(
                p.contains(&format!("    {line}")),
                "candidate line verbatim (indented): {line}"
            );
        }
    }
    let g = prompt_growing(module, Some(candidate), &fs, &traj, &patterns);
    let gp = prompt_growing_same_rule(module, Some(candidate), &fs, &traj, &patterns);
    for p in [&g, &gp] {
        assert!(
            p.contains("--- ptr_arg [Certified: 3 ok / 0 failed] ---"),
            "channel lint-key anchor position"
        );
        assert!(
            p.contains("ptr_arg: 3/4 attempts kept (75%)"),
            "failure-pattern rule anchor position"
        );
    }
    // Bt8 (Issue 073 T3): the anchor ledger restates the identity fields
    // VERBATIM, one per line, in its own block — the discrete channel.
    let anchors = FixAnchors {
        rule_id: Some("ptr_arg".into()),
        error_code: Some("E0599".into()),
        symbols: vec!["greedy_path".into(), "marginals".into()],
        span: Some("bytes 120..246".into()),
    };
    let bt8 = prompt_anchor_ledger(module, Some(candidate), &fs, &anchors);
    assert!(bt8.contains("  rule: ptr_arg\n"), "rule anchor verbatim");
    assert!(
        bt8.contains("  error code: E0599\n"),
        "error-code anchor verbatim"
    );
    assert!(
        bt8.contains("  symbols: greedy_path, marginals\n"),
        "symbol anchors verbatim"
    );
    assert!(
        bt8.contains("  span: bytes 120..246\n"),
        "span anchor verbatim"
    );
    // Placement: header < anchor ledger < module < candidate block.
    let hdr = bt8.find(TRAINED_INSTRUCTION).expect("instruction header");
    let ledger = bt8.find("Identity anchors").expect("ledger present");
    let mod_pos = bt8.find("greedy_path").expect("module symbol present");
    let cand = bt8
        .find("A static modelless analyzer")
        .expect("candidate block");
    assert!(
        hdr < ledger && ledger < mod_pos && mod_pos < cand,
        "condition-first: header < ledger < module < candidate"
    );
    // Arm B's body intact + exactly one respond-line.
    assert!(bt8.contains("--- example (manual_map) before ---"));
    assert_eq!(bt8.matches("Respond with ONLY").count(), 1);
    // Deterministic.
    assert_eq!(
        bt8,
        prompt_anchor_ledger(module, Some(candidate), &fs, &anchors)
    );
}

#[test]
fn anchor_ledger_empty_anchors_degrade_byte_identically_to_arm_b() {
    let module = issue073_module();
    let fs = vec![FewShot {
        rule_id: "manual_map".into(),
        buggy: "b".into(),
        fixed: "f".into(),
    }];
    assert_eq!(
        prompt_anchor_ledger(module, None, &fs, &FixAnchors::default()),
        prompt_prepass(module, None, &fs),
        "empty anchors = arm B byte-for-byte (the degradation contract)",
    );
    // Partial anchors omit their blank lines — never render empty rows.
    let partial = FixAnchors {
        rule_id: Some("ptr_arg".into()),
        ..FixAnchors::default()
    };
    let p = prompt_anchor_ledger(module, None, &fs, &partial);
    assert!(p.contains("  rule: ptr_arg\n"));
    assert!(!p.contains("error code:"), "unset fields omitted");
    assert!(!p.contains("  symbols:"), "unset fields omitted");
    assert!(!p.contains("  span:"), "unset fields omitted");
}

#[test]
fn anchor_ledger_perturbation_flips_exactly_the_anchor_lines() {
    let module = issue073_module();
    let twin = perturb_symbol(module, "greedy_path", "greedy_route");
    assert!(ident_mentioned(&twin, "greedy_route"));
    assert!(!ident_mentioned(&twin, "greedy_path"));
    let anchors = FixAnchors {
        rule_id: Some("ptr_arg".into()),
        error_code: Some("E0599".into()),
        symbols: vec!["greedy_path".into(), "marginals".into()],
        span: Some("bytes 120..246".into()),
    };
    let anchors_twin = FixAnchors {
        rule_id: Some("ptr_arg".into()),
        error_code: Some("E0599".into()),
        symbols: vec!["greedy_route".into(), "marginals".into()],
        span: Some("bytes 120..246".into()),
    };
    let orig = prompt_anchor_ledger(module, None, &[], &anchors);
    let twin_prompt = prompt_anchor_ledger(&twin, None, &[], &anchors_twin);
    assert!(
        twin_prompt.contains("greedy_route"),
        "swapped symbol rides the discrete ledger"
    );
    assert!(
        twin_prompt.contains("  symbols: greedy_route, marginals\n"),
        "ledger line flips"
    );
    assert!(!twin_prompt.contains("greedy_path"));
    // The flip is EXACTLY the entity swap — inverting it on the twin
    // prompt reproduces the original byte-for-byte (the ledger
    // paraphrases nothing around the anchors).
    let inverted = twin_prompt.replace("greedy_route", "greedy_path");
    assert_eq!(inverted, orig, "the flip is exactly the entity swap");
}

#[test]
fn entity_perturbation_flips_the_anchor_and_nothing_else() {
    let module = issue073_module();
    let twin = perturb_symbol(module, "greedy_path", "greedy_route");
    assert_ne!(module, twin, "perturbation applies");
    assert!(ident_mentioned(&twin, "greedy_route"));
    assert!(!ident_mentioned(&twin, "greedy_path"));
    assert!(ident_mentioned("x greedy_path_alt y", "greedy_path_alt"));
    assert!(!ident_mentioned("x greedy_path_alt y", "greedy_path"));
    assert!(!ident_mentioned("x greedy_path_alt y", "alt"));
    assert!(ident_mentioned("greedy_path_alt", "greedy_path_alt"));
    assert!(!ident_mentioned("", "greedy_path"));
    assert!(
        !twin.contains("greedy_path"),
        "rename is total on the fixture"
    );
    let fs = vec![FewShot {
        rule_id: "manual_map".into(),
        buggy: "b".into(),
        fixed: "f".into(),
    }];
    let fs_twin = vec![FewShot {
        rule_id: "manual_unwrap_or".into(),
        buggy: "b".into(),
        fixed: "f".into(),
    }];

    // Both identity axes (module symbol + rule id) flip in the prompt,
    // and the flip is EXACTLY the perturbation: inverting it on the twin
    // prompt reproduces the original byte-for-byte, so the builder
    // transports the swap without paraphrasing anything around it.
    // `prompt_raw` carries no few-shots (arm A is instruction + module),
    // so it is asserted on the symbol axis only.
    let raw_orig = prompt_raw(module);
    let raw_twin = prompt_raw(&twin);
    assert!(raw_twin.contains("greedy_route") && !raw_twin.contains("greedy_path"));
    assert_eq!(
        raw_twin.replace("greedy_route", "greedy_path"),
        raw_orig,
        "arm A: the flip is exactly the symbol swap"
    );

    for build in [
        |m: &str, f: &[FewShot]| prompt_prepass(m, None, f),
        |m: &str, f: &[FewShot]| prompt_growing(m, None, f, &[], &[]),
        |m: &str, f: &[FewShot]| prompt_growing_same_rule(m, None, f, &[], &[]),
    ] {
        let orig = build(module, &fs);
        let twin_prompt = build(&twin, &fs_twin);
        assert!(
            twin_prompt.contains("greedy_route"),
            "swapped symbol rides the anchor"
        );
        assert!(!twin_prompt.contains("greedy_path"));
        assert!(
            twin_prompt.contains("--- example (manual_unwrap_or) before ---"),
            "few-shot rule anchor flips"
        );
        let inverted = twin_prompt
            .replace("greedy_route", "greedy_path")
            .replace("manual_unwrap_or", "manual_map");
        assert_eq!(inverted, orig, "the flip is exactly the entity swap");
    }
}

/// The non-vacuity arm: a builder that renders only the module's SHAPE
/// (line count, fn count, brace balance) preserves contextual semantics
/// while dropping discrete identity — the exact `XBridge` failure mode.
/// It is rename-INVARIANT on the same twins, which is what makes the
/// fixture pair a real discriminator: if the real builders ever regress
/// to summarizing, `entity_perturbation_flips_the_anchor_and_nothing_else`
/// reds; this test holds the other horn (the pair CAN distinguish).
fn shape_summary_builder(module: &str, rule_id: &str) -> String {
    let lines = module.lines().count();
    let fns = module.matches("fn ").count();
    let braces = module.matches('{').count();
    format!("fix this code (rule {rule_id}): {lines} lines, {fns} fns, {braces} blocks")
}

#[test]
fn entity_perturbation_distinguishes_anchor_primacy_from_a_summarizing_builder() {
    let module = issue073_module();
    let twin = perturb_symbol(module, "greedy_path", "greedy_route");
    // The naive summarizer does NOT see the entity swap (identity lost,
    // semantics kept) — the fixture pair is non-vacuous in BOTH
    // directions.
    assert_eq!(
        shape_summary_builder(&twin, "manual_map"),
        shape_summary_builder(module, "manual_map"),
        "the summarizing builder is rename-invariant on the twins",
    );
    // ...while every real builder flips (asserted by the sibling test).
    assert_ne!(
        prompt_prepass(module, None, &[]),
        prompt_prepass(&twin, None, &[]),
    );
}

#[test]
fn same_rule_context_excludes_other_rules_and_orders_certified_first() {
    let mut d = std::env::temp_dir();
    d.push(format!(
        "riir_gprime_ctx_{}_{}",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_dir_all(&d);
    let (store, bad) = crate::traj_store::TrajStore::open(&d).unwrap();
    assert_eq!(bad, 0);
    // Queried rule `ptr_arg`: one Heuristic (unverified) + one Certified.
    // Records interleave so a stable sort would NOT produce the
    // Certified-first order — only the explicit tier rank does.
    store
        .record("ptr_arg", "shape_h", "fix_h", true, false, "r")
        .unwrap();
    store
        .record("ptr_arg", "shape_c", "fix_c", true, true, "r")
        .unwrap();
    // Another rule at identical shape — arm G would carry it, G' must not.
    store
        .record("needless_borrow", "shape_c", "fix_x", true, true, "r")
        .unwrap();
    // A prefix-collision rule — the boundary-safe `ptr_arg:` key prefix
    // must NOT admit it (a bare-prefix filter would).
    store
        .record("ptr_arg_extra", "shape_c", "fix_y", true, true, "r")
        .unwrap();

    let (exemplars, patterns) = same_rule_context(&store, "shape_c", "ptr_arg", 3);
    assert_eq!(exemplars.len(), 2, "only the ptr_arg family: {exemplars:?}");
    assert!(exemplars.iter().all(|e| e.lint_key == "ptr_arg"));
    assert_eq!(
        exemplars[0].tier, "Certified",
        "Certified first: {exemplars:?}"
    );
    assert_eq!(exemplars[0].shape, "shape_c");
    assert_eq!(exemplars[1].tier, "Heuristic");
    // Channel 3: the family fold only — the other rules' attempts must
    // not appear.
    assert_eq!(patterns.len(), 1, "one family rule: {patterns:?}");
    assert_eq!(patterns[0].rule, "ptr_arg");
    assert_eq!(patterns[0].attempts, 2);
    // The all-rules build (arm G) still sees the other rules — the two
    // contexts differ by exactly the family restriction.
    let (all_ex, all_pat) = all_rule_context(&store, "shape_c", 3);
    assert!(all_ex.iter().any(|e| e.lint_key == "needless_borrow"));
    assert!(all_pat.iter().any(|p| p.rule == "ptr_arg_extra"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn same_rule_context_folds_rustc_code_family_and_excludes_withdrawn() {
    let mut d = std::env::temp_dir();
    d.push(format!(
        "riir_gprime_rustc_{}_{}",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_dir_all(&d);
    let (store, bad) = crate::traj_store::TrajStore::open(&d).unwrap();
    assert_eq!(bad, 0);
    // Two strategies under one rustc code — the `{code}:` prefix admits
    // both; the fold reports them as their own pattern rows.
    store
        .record("E0502:a-strategy", "&v[0]", "x", true, true, "r")
        .unwrap();
    store
        .record("E0502:b-strategy", "&w[0]", "y", false, false, "r")
        .unwrap();
    store
        .record("E0502:b-strategy", "&w[0]", "y", false, false, "r")
        .unwrap();
    // A Withdrawn row (3 consecutive fails = the absorbing transition) —
    // excluded from BOTH channels.
    store
        .record("E0502:dead-strategy", "&d[0]", "z", false, false, "r")
        .unwrap();
    store
        .record("E0502:dead-strategy", "&d[0]", "z", false, false, "r")
        .unwrap();
    store
        .record("E0502:dead-strategy", "&d[0]", "z", false, false, "r")
        .unwrap();
    // Another code entirely.
    store
        .record("E0382:other", "&m[0]", "q", true, true, "r")
        .unwrap();

    let (exemplars, patterns) = same_rule_context(&store, "&v[0]", "E0502", 3);
    assert!(!exemplars.is_empty() && exemplars.iter().all(|e| e.lint_key.starts_with("E0502:")));
    assert!(
        !exemplars
            .iter()
            .any(|e| e.lint_key == "E0502:dead-strategy"),
        "withdrawn excluded"
    );
    assert!(patterns.iter().all(|p| p.rule.starts_with("E0502:")));
    assert!(!patterns.iter().any(|p| p.rule == "E0502:dead-strategy"));
    assert!(patterns
        .iter()
        .any(|p| p.rule == "E0502:a-strategy" && p.kept == 1));
    let b = patterns
        .iter()
        .find(|p| p.rule == "E0502:b-strategy")
        .expect("b-strategy row");
    assert_eq!(
        (b.attempts, b.kept),
        (2, 0),
        "2 fails stays Heuristic (3 would withdraw)"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ── contract + first_fn ────────────────────────────────────────────────

#[test]
fn contract_preserved_flags_a_dropped_fn_but_not_a_dropped_local() {
    let destructive = contract_preserved("pub fn f(a: u8) {}\npub fn g() {}", "pub fn f(a: u8) {}");
    assert!(!destructive);
    let local_fix = contract_preserved("pub fn f() { let s2 = 1; let _ = s2; }", "pub fn f() { }");
    assert!(local_fix, "dropping a local let is a legitimate fix");
}

// ── Plan 225 T1.4 clause (c): attr silencing ───────────────────────

#[test]
fn attr_silence_added_flags_a_new_allowance_not_a_preexisting_one() {
    let original = "pub fn f(v: &Vec<u8>) -> usize { v.len() }";
    // The heal: the attribute is ADDED — refused.
    let silenced = "#[allow(clippy::ptr_arg)]\npub fn f(v: &Vec<u8>) -> usize { v.len() }";
    assert!(attr_silence_added(original, silenced));
    // The legitimate fix: no attribute anywhere — accepted.
    let healed = "pub fn f(v: &[u8]) -> usize { v.len() }";
    assert!(!attr_silence_added(original, healed));
    // A PRE-EXISTING allowance that survives the edit is not an addition.
    let orig_with = "#[allow(clippy::ptr_arg)]\npub fn f(v: &Vec<u8>) -> usize { v.len() }";
    assert!(!attr_silence_added(orig_with, orig_with));
    // `expect` is the same class.
    let expected = "#[expect(clippy::ptr_arg)]\npub fn f(v: &Vec<u8>) -> usize { v.len() }";
    assert!(attr_silence_added(original, expected));
}

#[test]
fn first_fn_name_finds_the_hint() {
    assert_eq!(
        first_fn_name("pub struct S;\npub fn mk() -> S { S }").as_deref(),
        Some("mk")
    );
    assert!(first_fn_name("pub enum E { A }").is_none());
}

// ── reachable premise (fast — no cargo, no oracle) ────────────────────

#[test]
fn reachable_premise_holds_against_the_seed_corpus() {
    let corpus = crate::build_seed_corpus();
    assert_reachable_premise(&corpus);
}

// ── Issue 051 T12: the fallback leg gets the free leg's hardening ──

#[test]
fn connect_timeout_is_bounded_well_under_the_call_timeout() {
    assert!(super::CONNECT_TIMEOUT < super::CALL_TIMEOUT);
    assert!(super::CONNECT_TIMEOUT.as_secs() <= 15);
    // A full window must not hang for hours on connects alone.
    assert!(super::CONNECT_TIMEOUT.as_secs() * 25 <= 300);
}

#[test]
fn clip_payload_bounds_a_server_controlled_dump() {
    let small = "{\"error\":\"nope\"}";
    assert_eq!(
        super::clip_payload(small),
        small,
        "short payloads pass through"
    );
    let big = "x".repeat(super::MAX_PAYLOAD_IN_ERROR * 4);
    let out = super::clip_payload(&big);
    assert!(out.len() < big.len(), "must shrink");
    assert!(out.contains("clipped"), "must say so: {out}");
    assert!(
        out.contains(&big.len().to_string()),
        "must report the true size"
    );
}

#[test]
fn clip_payload_never_splits_a_utf8_char() {
    // A multi-byte char straddling the cut would panic a naive slice.
    let s = "é".repeat(super::MAX_PAYLOAD_IN_ERROR);
    let out = super::clip_payload(&s);
    assert!(out.contains("clipped"));
    assert!(out.is_char_boundary(0));
}
