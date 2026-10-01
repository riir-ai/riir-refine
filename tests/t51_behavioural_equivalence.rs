//! Plan 336 T5.1/T5.3 — the behavioural-equivalence instrument, pinned against
//! the two silent semantic breaks Bench 039 scored as WINs.
//!
//! Bench 039's rule was `clippy clean AND contract identifiers preserved`. These
//! two pass that rule and are still wrong, which is why `p_win` was an upper
//! bound at any n. Each `divergent` case below MUST fail, and each `equivalent`
//! case MUST pass — a checker that rejects the 3 clean wins would be useless in
//! the other direction.

use riir_refine::behavioural_equivalence::{check, divergence_is_invisible_to_names, Verdict};

fn work(tag: &str) -> std::path::PathBuf {
    // Isolated under /tmp — never a shared target dir.
    let d = std::env::temp_dir().join(format!("riir_refine_equiv/{tag}"));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn v(tag: &str, orig: &str, cand: &str) -> Verdict {
    check(orig, cand, "f", 64, &work(tag))
}

// ── The two documented silent semantic breaks ───────────────────────────────

#[test]
fn option_if_let_else_silent_break_is_caught() {
    // orig: None -> 0.  candidate: None -> 0 + 1 = 1.
    let got = v(
        "option_if_let_else",
        "pub fn f(o: Option<u32>) -> u32 { if let Some(x) = o { x + 1 } else { 0 } }",
        "pub fn f(o: Option<u32>) -> u32 { o.unwrap_or(0) + 1 }",
    );
    match &got {
        Verdict::Divergent {
            input,
            original,
            candidate,
        } => {
            assert!(
                input.contains("None"),
                "the counterexample should be None, got {input}"
            );
            assert_eq!((original.as_str(), candidate.as_str()), ("0", "1"));
        }
        other => panic!("expected Divergent, got {other:?}"),
    }
    assert!(got.is_divergent());
    assert!(!got.is_equivalent());
}

#[test]
fn manual_strip_silent_break_is_caught() {
    // orig: "xyz" -> None (guard).  candidate: s.get(2..) -> Some("z").
    let got = v(
        "manual_strip",
        "pub fn f(s: &str) -> Option<&str> { if s.starts_with(\"ab\") { Some(&s[2..]) } else { None } }",
        "pub fn f(s: &str) -> Option<&str> { s.get(2..) }",
    );
    match &got {
        Verdict::Divergent {
            input,
            original,
            candidate,
        } => {
            assert!(
                input.contains("xyz"),
                "expected the xyz counterexample, got {input}"
            );
            assert_eq!(original, "None");
            assert!(
                candidate.contains('z'),
                "candidate should yield Some(\"z\"), got {candidate}"
            );
        }
        other => panic!("expected Divergent, got {other:?}"),
    }
}

#[test]
fn both_breaks_are_invisible_to_a_name_based_check() {
    // This is the load-bearing claim: the contract check CANNOT see either bug,
    // which is why behavioural equivalence had to be added rather than tightened.
    assert!(divergence_is_invisible_to_names(
        "pub fn f(o: Option<u32>) -> u32 { if let Some(x) = o { x + 1 } else { 0 } }",
        "pub fn f(o: Option<u32>) -> u32 { o.unwrap_or(0) + 1 }",
    ));
    assert!(divergence_is_invisible_to_names(
        "pub fn f(s: &str) -> Option<&str> { if s.starts_with(\"ab\") { Some(&s[2..]) } else { None } }",
        "pub fn f(s: &str) -> Option<&str> { s.get(2..) }",
    ));
}

// ── The 3 clean wins must still pass (no false positives) ───────────────────

#[test]
fn or_fun_call_correct_fix_is_equivalent() {
    let got = v(
        "or_fun_call",
        "pub fn f(opt: Option<u32>) -> u32 { opt.unwrap_or(expensive()) }\npub fn expensive() -> u32 { 42 }",
        "pub fn f(opt: Option<u32>) -> u32 { opt.unwrap_or_else(expensive) }\npub fn expensive() -> u32 { 42 }",
    );
    assert!(got.is_equivalent(), "clean win misjudged: {got:?}");
}

#[test]
fn ptr_arg_correct_fix_is_equivalent_across_the_signature_change() {
    // The fix IS a signature change (&Vec<u8> -> &[u8]); the checker has to
    // compare across it or the fix we most want to pass is unscoreable.
    let got = v(
        "ptr_arg",
        "pub fn f(v: &Vec<u8>) -> usize { v.len() }",
        "pub fn f(v: &[u8]) -> usize { v.len() }",
    );
    assert!(
        got.is_equivalent(),
        "signature-change fix misjudged: {got:?}"
    );
}

#[test]
fn redundant_clone_correct_fix_is_equivalent() {
    let got = v(
        "redundant_clone",
        "pub fn f() { let s = String::from(\"hi\"); let s2 = s.clone(); drop(s2); }",
        "pub fn f() { let s = String::from(\"hi\"); drop(s); }",
    );
    assert!(got.is_equivalent(), "zero-arg fix misjudged: {got:?}");
}

// ── The correct fixes for the two breaks must pass ──────────────────────────

#[test]
fn the_correct_fixes_for_both_breaks_are_equivalent() {
    let a = v(
        "option_if_let_else_ok",
        "pub fn f(o: Option<u32>) -> u32 { if let Some(x) = o { x + 1 } else { 0 } }",
        "pub fn f(o: Option<u32>) -> u32 { o.map_or(0, |x| x + 1) }",
    );
    assert!(a.is_equivalent(), "map_or fix should be equivalent: {a:?}");
    let b = v(
        "manual_strip_ok",
        "pub fn f(s: &str) -> Option<&str> { if s.starts_with(\"ab\") { Some(&s[2..]) } else { None } }",
        "pub fn f(s: &str) -> Option<&str> { s.strip_prefix(\"ab\") }",
    );
    assert!(
        b.is_equivalent(),
        "strip_prefix fix should be equivalent: {b:?}"
    );
}

// ── Unsupported must not read as a pass ────────────────────────────────────

#[test]
fn unsupported_is_not_equivalent() {
    // A user type has no Debug-comparable bounded domain here.
    let got = check(
        "pub struct S { v: Box<Vec<u8>> }\npub fn mk() -> S { S { v: Box::new(Vec::new()) } }",
        "pub struct S { v: Vec<u8> }\npub fn mk() -> S { S { v: Vec::new() } }",
        "mk",
        8,
        &work("box_collection"),
    );
    assert!(
        !got.is_equivalent(),
        "Unsupported must never read as a pass"
    );
    assert!(!got.is_divergent());
    assert!(matches!(got, Verdict::Unsupported { .. }), "got {got:?}");
}

#[test]
fn a_noncompiling_candidate_is_unsupported_not_equivalent() {
    let got = v(
        "broken",
        "pub fn f(o: Option<u32>) -> u32 { o.unwrap_or(0) }",
        "pub fn f(o: Option<u32>) -> u32 { o.unwrap_or( }",
    );
    assert!(matches!(got, Verdict::Unsupported { .. }), "got {got:?}");
}
