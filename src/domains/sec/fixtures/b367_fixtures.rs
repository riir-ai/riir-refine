//! Batch 367 sec fixtures — the opal harness boundary/admission half
//! (Plan 381 / Research 413): `xingjian-bai/opal` @
//! `97762859ba5e2527e5c6f63d9890dfbab860d51f` (MIT). ORIGINAL plain-Rust
//! stand-ins — zero opal source pasted (the taxonomy and the merge law are
//! re-authored; the source's own categories are the ideas, the Rust shape
//! is ours). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b367_fixtures.rs && ./b367_fixtures`.
//!
//! Arms per rule:
//! - F2 `agent-code-static-boundary-diagnostics-never-gate`: a scanner
//!   over a proposed-source string with a category taxonomy (io /
//!   dynamic-eval-analog / introspection / protected-type-mutation) and
//!   import-alias resolution — the scan records diagnostics and NEVER
//!   decides: an io-touching proposal is flagged (and the caller's
//!   acceptance is recorded either way), a benign proposal is clean, a
//!   MALFORMED source yields empty diagnostics and never panics (the
//!   never-gate contract), a deliberate-looking-but-benign use is flagged
//!   (a flag can be benign — the observation is recorded either way), and
//!   the vocabulary-bound regime is demonstrated (an indirectly-named
//!   surface evades the CATEGORY vocabulary — the walker is not the
//!   bound);
//! - F3 `stale-proposal-merges-against-shown-base`: a proposal computed
//!   against base B applied onto current C after a revert of B — the
//!   naive wholesale apply RESURRECTS the reverted content (the buggy
//!   arm), the delta-against-shown-base apply does not (the fixed arm);
//!   the kill arm: a full-file rewrite degenerates the merge to a
//!   wholesale write and carries the stale view — the named regime
//!   condition (protection holds where edits are local).

// ─────────────────────────────────────────────────────────────────────────────
// F2 — agent-code-static-boundary-diagnostics-never-gate
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone, Copy)]
enum Category {
    Io,
    DynamicEval,
    Introspection,
    ProtectedMutation,
}

/// The scanner's category vocabulary (the IDEAS from opal's taxonomy, our
/// own Rust-shaped tables): a category is keyed on NAME SURFACE only.
fn category_of(name: &str) -> Option<Category> {
    let root = name.split("::").next().unwrap_or(name);
    match root {
        "process" | "fs" | "net" | "io" => Some(Category::Io),
        "eval" | "exec" | "dyn_load" => Some(Category::DynamicEval),
        "frames" | "globals_of" | "type_of_reflected" => Some(Category::Introspection),
        _ => None,
    }
}

/// An attribute/method surface that mutates a protected object.
fn protected_method(attr: &str) -> bool {
    attr == "register_update_hook" || attr == "set_param" || attr == "zero_grad"
}

#[derive(Debug)]
struct Diagnostic {
    line: usize,
    category: Category,
    snippet: String,
}

/// Scan a proposed source (line-based, alias-resolving, BEST-EFFORT):
/// returns diagnostics; a malformed input yields an empty list — the scan
/// never panics and NEVER gates (acceptance is the caller's).
fn scan(source: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut aliases: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for (idx, raw) in source.lines().enumerate() {
        let lineno = idx + 1;
        let line = raw.trim();
        // malformed-source containment: a line we cannot interpret is
        // skipped — the scan degrades to fewer observations, never a panic
        if line.starts_with("?!") {
            continue;
        }
        // alias resolution: `use x as y;`
        if let Some(rest) = line.strip_prefix("use ") {
            if let Some((path, alias)) = rest.trim_end_matches(';').split_once(" as ") {
                aliases.insert(alias.trim(), path.trim());
                continue;
            }
        }
        // name surface: the first word-ish token (our walker is line-level)
        let token = line
            .split(['(', ' ', ';'])
            .find(|t| !t.is_empty())
            .unwrap_or("");
        // alias resolution rewrites the LEADING segment (:: paths)
        let head = token.split("::").next().unwrap_or(token);
        let resolved = match aliases.get(head) {
            Some(real) => format!("{real}{}", &token[head.len()..]),
            None => token.to_string(),
        };
        if let Some(cat) = category_of(&resolved) {
            out.push(Diagnostic {
                line: lineno,
                category: cat,
                snippet: line.chars().take(60).collect(),
            });
        }
        // protected-object mutation: a call on the protected name
        if line.contains("guarded.") {
            for m in ["register_update_hook", "set_param", "zero_grad"] {
                if line.contains(&format!("guarded.{m}")) && protected_method(m) {
                    out.push(Diagnostic {
                        line: lineno,
                        category: Category::ProtectedMutation,
                        snippet: line.chars().take(60).collect(),
                    });
                }
            }
        }
    }
    out
}

/// The NEVER-GATE posture: the scan result is RECORDED; the acceptance
/// decision is the caller's, made either way.
#[derive(Debug, PartialEq)]
enum Acceptance {
    Accepted,
    Declined,
}

fn review(source: &str, caller_says: Acceptance) -> (Vec<Diagnostic>, Acceptance) {
    let diags = scan(source);
    // observations, not rejections: the caller's verdict stands whatever
    // the scan found
    (diags, caller_says)
}

#[test]
fn f2_io_touching_proposal_is_flagged_and_recorded() {
    let src = "use fs as filesystem;\nfilesystem::read_config()\n";
    let (diags, acceptance) = review(src, Acceptance::Accepted);
    assert_eq!(diags.len(), 1, "the io surface is flagged once: {diags:?}");
    assert_eq!(diags[0].category, Category::Io);
    assert_eq!(diags[0].line, 2);
    assert!(
        diags[0].snippet.contains("read_config"),
        "the diagnostic carries the surface: {:?}",
        diags[0].snippet
    );
    // the flag did NOT gate: the caller's acceptance stands
    assert_eq!(acceptance, Acceptance::Accepted);
}

#[test]
fn f2_benign_proposal_is_clean() {
    let src = "let x = compute(4);\nlet y = x + 1;\n";
    let (diags, _) = review(src, Acceptance::Accepted);
    assert!(diags.is_empty(), "a benign proposal scans clean: {diags:?}");
}

#[test]
fn f2_malformed_source_yields_empty_and_never_panics() {
    // the never-gate contract: even a broken source must not turn the
    // diagnostic into a failure — empty observations, no panic
    let malformed = "?!\nbroken line (((\n?! garbage\n";
    let (diags, _) = review(malformed, Acceptance::Declined);
    assert!(
        diags.is_empty(),
        "malformed source yields empty diagnostics: {diags:?}"
    );
    // and the caller's decision still lands
    let (_, acceptance) = review(malformed, Acceptance::Declined);
    assert_eq!(acceptance, Acceptance::Declined);
}

#[test]
fn f2_deliberate_looking_but_benign_use_is_flagged_recorded_either_way() {
    // a flag can be benign: an io-named call inside a comment-like
    // context is still flagged — the observation is recorded, review
    // decides. The scan does NOT second-guess the surface.
    let src = "process::handle(request) // naming looks dynamic, the use is benign\n";
    let (diags, acceptance) = review(src, Acceptance::Accepted);
    assert_eq!(
        diags.len(),
        1,
        "the surface is flagged whatever the intent: {diags:?}"
    );
    assert_eq!(diags[0].category, Category::Io);
    assert_eq!(
        acceptance,
        Acceptance::Accepted,
        "the flag did not gate the benign case"
    );
}

#[test]
fn f2_vocabulary_bound_regime_indirect_naming_evades() {
    // The regime condition: the CATEGORY VOCABULARY, not the walker, is
    // the bound — an indirectly-constructed name evades the scan. The
    // honest bound is carried in-rule; the fixture demonstrates it.
    let src = "let name = build_name();\nname::call();\n";
    let (diags, _) = review(src, Acceptance::Accepted);
    assert!(
        diags.is_empty(),
        "an indirect name evades the category vocabulary (the documented bound): {diags:?}"
    );
}

#[test]
fn f2_tampered_scanner_that_gates_reds() {
    // The gate: a scanner that turns its own diagnostics into a rejection
    // breaks the observations-never-gate contract — the deliberate-but-
    // benign arm MUST still be accepted, so a gating scanner is wrong.
    fn tampering_gate(source: &str) -> Acceptance {
        let diags = scan(source);
        if diags.is_empty() {
            Acceptance::Accepted
        } else {
            Acceptance::Declined // the tamper: diagnostics decide
        }
    }
    let src = "process::handle(request) // benign\n";
    assert_eq!(
        tampering_gate(src),
        Acceptance::Declined,
        "the tampered scanner gates on the benign flag (the defect demonstrated)"
    );
    // the honest posture: the same source, the caller accepts
    let (_, acceptance) = review(src, Acceptance::Accepted);
    assert_eq!(
        acceptance,
        Acceptance::Accepted,
        "the honest scanner never gates"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F3 — stale-proposal-merges-against-shown-base
// ─────────────────────────────────────────────────────────────────────────────

/// One file of the working set.
#[derive(Clone, PartialEq, Debug)]
struct File {
    name: String,
    body: String,
}

/// The apply lanes: wholesale (the buggy arm) vs delta-against-shown-base
/// (the fixed arm, the three-way merge at file level).
fn apply_wholesale(current: &mut Vec<File>, proposal: &[File]) {
    for p in proposal {
        if let Some(f) = current.iter_mut().find(|f| f.name == p.name) {
            f.body = p.body.clone();
        } else {
            current.push(p.clone());
        }
    }
}

fn apply_delta_against_shown_base(current: &mut Vec<File>, proposal: &[File], shown_base: &[File]) {
    // apply only what the proposal changed relative to the files it was
    // shown: for each name in proposal ∪ shown_base, where they differ,
    // the proposal's version LANDS (inserted when the revert removed it —
    // the degenerate kill-arm case), where the proposal DELETED a file
    // the base had, it is removed; unchanged names carry current.
    let mut names: Vec<&str> = proposal.iter().map(|f| f.name.as_str()).collect();
    for f in shown_base {
        if !names.contains(&f.name.as_str()) {
            names.push(&f.name);
        }
    }
    for name in names {
        let in_prop = proposal.iter().find(|f| f.name == name);
        let in_base = shown_base.iter().find(|f| f.name == name);
        match (in_prop, in_base) {
            (Some(p), Some(b)) if p.body != b.body => {
                // changed relative to the shown base: it lands — inserted
                // when missing (the degenerate kill-arm case, asserted in
                // f3_kill_arm)
                if let Some(f) = current.iter_mut().find(|f| f.name == name) {
                    f.body = p.body.clone();
                } else {
                    current.push(p.clone());
                }
            }
            (Some(p), None) => {
                // added by the proposal: apply
                if let Some(f) = current.iter_mut().find(|f| f.name == name) {
                    f.body = p.body.clone();
                } else {
                    current.push(p.clone());
                }
            }
            (None, Some(_)) => {
                // deleted by the proposal: remove
                current.retain(|f| f.name != name);
            }
            _ => {} // unchanged relative to the shown base: carry current
        }
    }
}

fn files(pairs: &[(&str, &str)]) -> Vec<File> {
    pairs
        .iter()
        .map(|(n, b)| File {
            name: n.to_string(),
            body: b.to_string(),
        })
        .collect()
}

#[test]
fn f3_revert_between_ask_and_apply_wholesale_resurrects_delta_does_not() {
    // base B (shown to the agent) has helper.rs with `alpha`; the revert
    // removed helper.rs from current C; the late proposal was computed
    // against B and still carries helper.rs. Wholesale resurrects it;
    // the delta-apply applies only what the proposal CHANGED vs B —
    // helper.rs is unchanged relative to B, so it does not come back.
    let shown_base = files(&[
        ("engine.rs", "fn tick() {}"),
        ("helper.rs", "fn alpha() {}"),
    ]);
    // the proposal (computed against B, unaware of the revert): it edited
    // engine.rs and carries helper.rs verbatim
    let proposal = files(&[
        ("engine.rs", "fn tick() { step(); }"),
        ("helper.rs", "fn alpha() {}"),
    ]);
    let mut c_wholesale = files(&[("engine.rs", "fn tick() {}")]);
    apply_wholesale(&mut c_wholesale, &proposal);
    assert!(
        c_wholesale.iter().any(|f| f.name == "helper.rs"),
        "the wholesale apply RESURRECTS the reverted file (the bug demonstrated)"
    );
    let mut c_delta = files(&[("engine.rs", "fn tick() {}")]);
    apply_delta_against_shown_base(&mut c_delta, &proposal, &shown_base);
    assert!(
        !c_delta.iter().any(|f| f.name == "helper.rs"),
        "the delta-apply does NOT resurrect the reverted file"
    );
    assert!(
        c_delta
            .iter()
            .any(|f| f.name == "engine.rs" && f.body.contains("step();")),
        "the delta-apply still lands the proposal's real edit"
    );
}

#[test]
fn f3_normal_apply_unchanged_between_arms_when_no_revert() {
    // without a revert, both lanes land the same result — the merge law
    // changes the REVERT case only. The proposal carries every file it
    // was shown (unchanged files identical to the base produce no diff);
    // a proposal MISSING a base file reads as a deletion under the delta
    // law — that is the third arm's test, not this one.
    let shown_base = files(&[("engine.rs", "a"), ("util.rs", "u")]);
    let mut c1 = files(&[("engine.rs", "a"), ("util.rs", "u")]);
    let mut c2 = c1.clone();
    let proposal = files(&[("engine.rs", "b"), ("util.rs", "u")]);
    apply_wholesale(&mut c1, &proposal);
    apply_delta_against_shown_base(&mut c2, &proposal, &shown_base);
    assert_eq!(c1, c2, "no revert: both arms agree");
}

#[test]
fn f3_proposal_deletion_of_a_base_file_applies_as_removal() {
    // the delta law's third arm: the proposal DELETED a file the shown
    // base had — the removal lands (a deliberate deletion is a change)
    let shown_base = files(&[("engine.rs", "a"), ("util.rs", "u")]);
    let mut current = files(&[("engine.rs", "a"), ("util.rs", "u")]);
    let proposal = files(&[("engine.rs", "a2")]); // util.rs removed by the proposal
    apply_delta_against_shown_base(&mut current, &proposal, &shown_base);
    assert!(
        !current.iter().any(|f| f.name == "util.rs"),
        "a deliberate deletion in the proposal applies as a removal"
    );
}

#[test]
fn f3_kill_arm_full_file_rewrite_degenerates_to_wholesale() {
    // The kill arm (the regime condition, named in-rule): a proposal that
    // rewrote a file that was ALSO reverted between ask and apply — the
    // file differs from the shown base, so the delta-apply carries the
    // proposal's stale view of it. Protection holds where edits are
    // local; a whole-file rewrite of a reverted file degenerates the
    // merge to a wholesale write.
    let shown_base = files(&[
        ("engine.rs", "fn tick() {}"),
        ("victim.rs", "fn old_body() {}"),
    ]);
    // the revert removed victim.rs from current
    let mut c_delta = files(&[("engine.rs", "fn tick() {}")]);
    // the proposal rewrote victim.rs WHOLE (computed against B, unaware
    // of the revert) and touched engine.rs
    let proposal = files(&[
        ("engine.rs", "fn tick() { step(); }"),
        ("victim.rs", "fn old_body() { /* rewritten */ }"),
    ]);
    apply_delta_against_shown_base(&mut c_delta, &proposal, &shown_base);
    assert!(
        c_delta.iter().any(|f| f.name == "victim.rs"),
        "the kill arm: a full-file rewrite of a reverted file is carried (the degenerate case — \
         the named regime condition, protection holds where edits are local)"
    );
}

#[test]
fn f3_tampered_delta_that_ignores_the_shown_base_reds() {
    // The gate: a "delta" apply that keys on the proposal alone (no shown
    // base) is the wholesale lane wearing the name — it must resurrect
    // the reverted file and RED.
    let shown_base = files(&[("engine.rs", "a"), ("helper.rs", "fn alpha() {}")]);
    let mut current = files(&[("engine.rs", "a")]);
    let proposal = files(&[("engine.rs", "b"), ("helper.rs", "fn alpha() {}")]);
    // tampered: "delta" without the base = every proposal name applied
    let tampered_names: Vec<&str> = proposal.iter().map(|f| f.name.as_str()).collect();
    for name in tampered_names {
        let p = proposal.iter().find(|f| f.name == name).unwrap();
        if let Some(f) = current.iter_mut().find(|f| f.name == name) {
            f.body = p.body.clone();
        } else {
            current.push(p.clone());
        }
    }
    let _ = shown_base; // the tamper: the base is never consulted
    assert!(
        current.iter().any(|f| f.name == "helper.rs"),
        "the tampered base-less delta resurrects the reverted file (the gate reds)"
    );
}
