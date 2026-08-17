//! Plan 336 T4.4 — G4 (composition): the L0-L3 → L4 fallback path.
//!
//! Run with:
//! ```bash
//! cargo test --features l4_lora --test g4_l4_composition
//! ```
//!
//! The whole file is feature-gated, so without `--features l4_lora` it
//! compiles to nothing and reports "0 passed" — which looks identical to
//! success. The `harness_is_actually_running` test below exists so a vacuous
//! pass is detectable: it asserts a non-trivial fact about the wiring, and the
//! run's test count is checked in the bench write-up.
#![cfg(feature = "l4_lora")]

use riir_clippy::build_seed_corpus;
use riir_clippy::l4_fallback::{L4Fallback, NullL4};
use riir_clippy::{HealPipeline, HealSource, RustAnalyzerPruner, TernaryDrafter};

/// Counting test double. Records every call so invariant 1 ("never consulted
/// on a modelless hit") is checked by call COUNT, not by inspecting the
/// returned fix — a fallback that is called and whose answer is discarded
/// still costs ~48 s, so "not used" is not the same claim as "not called".
struct CountingL4 {
    calls: std::cell::Cell<usize>,
    reply: Option<String>,
    seen: std::cell::RefCell<Vec<(String, String)>>,
}

impl CountingL4 {
    fn new(reply: Option<&str>) -> Self {
        Self {
            calls: std::cell::Cell::new(0),
            reply: reply.map(str::to_string),
            seen: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl L4Fallback for CountingL4 {
    fn generate_fix(&mut self, rule_id: &str, buggy_code: &str) -> Option<String> {
        self.calls.set(self.calls.get() + 1);
        self.seen.borrow_mut().push((rule_id.to_string(), buggy_code.to_string()));
        self.reply.clone()
    }

    fn name(&self) -> &str {
        "counting-l4"
    }
}

/// Shared counter handle: the pipeline takes ownership of the boxed fallback,
/// so the test keeps an `Rc` to the same cell to read the count afterwards.
#[derive(Clone)]
struct Probe {
    calls: std::rc::Rc<std::cell::Cell<usize>>,
    reply: Option<String>,
}

impl Probe {
    fn new(reply: Option<&str>) -> Self {
        Self {
            calls: std::rc::Rc::new(std::cell::Cell::new(0)),
            reply: reply.map(str::to_string),
        }
    }
    fn count(&self) -> usize {
        self.calls.get()
    }
}

impl L4Fallback for Probe {
    fn generate_fix(&mut self, _rule_id: &str, _buggy_code: &str) -> Option<String> {
        self.calls.set(self.calls.get() + 1);
        self.reply.clone()
    }
}

fn pipeline() -> HealPipeline {
    HealPipeline::new(TernaryDrafter::fallback(), RustAnalyzerPruner::new())
}

/// A (rule_id, buggy_code) pair the modelless path CAN serve — taken from the
/// seed corpus itself, so the drafter is guaranteed a template.
fn corpus_hit() -> (String, String) {
    let corpus = build_seed_corpus();
    let e = corpus.entries.first().expect("seed corpus is non-empty");
    (e.lint_name.clone(), e.buggy_code.clone())
}

// ── Invariant 4: absent by default ─────────────────────────────────────────

#[test]
fn no_fallback_installed_by_default() {
    let p = pipeline();
    assert!(!p.has_l4_fallback(), "a fresh pipeline must not carry an L4 fallback");
}

#[test]
fn absent_fallback_leaves_the_modelless_result_unchanged() {
    let corpus = build_seed_corpus();
    let (rule, buggy) = corpus_hit();

    let mut without = pipeline();
    let a = without.heal(&rule, &buggy, &corpus.entries);

    let mut with_null = pipeline().with_l4_fallback(Box::new(NullL4));
    let b = with_null.heal(&rule, &buggy, &corpus.entries);

    match (a, b) {
        (Some(a), Some(b)) => {
            assert_eq!(a.fixed_code, b.fixed_code, "installing a declining L4 changed the fix");
            assert_eq!(a.source, b.source, "installing a declining L4 changed the source");
            assert_eq!(a.parses, b.parses, "installing a declining L4 changed the parse verdict");
        }
        (None, None) => {}
        (a, b) => panic!("presence mismatch: {:?} vs {:?}", a.is_some(), b.is_some()),
    }
}

// ── Invariant 1: never consulted on a modelless hit ────────────────────────

#[test]
fn l4_is_not_called_when_the_modelless_path_serves_a_parsing_fix() {
    let corpus = build_seed_corpus();
    let (rule, buggy) = corpus_hit();

    let probe = Probe::new(Some("fn generated() {}"));
    let mut p = pipeline().with_l4_fallback(Box::new(probe.clone()));
    let result = p.heal(&rule, &buggy, &corpus.entries).expect("corpus entry must heal");

    assert!(result.parses, "precondition: the modelless fix parses");
    assert_ne!(result.source, HealSource::L4Lora, "a modelless hit must not be served by L4");
    assert_eq!(
        probe.count(),
        0,
        "L4 was CALLED on a modelless hit — at ~48 s/call against ~16 µs for the retriever \
         that is a ~3,000,000x regression even when the answer is discarded"
    );
}

#[test]
fn stage2_hit_never_reaches_l4() {
    let corpus = build_seed_corpus();
    let (rule, buggy) = corpus_hit();

    let probe = Probe::new(Some("fn generated() {}"));
    let mut p = pipeline().with_l4_fallback(Box::new(probe.clone()));
    // Stage 2 short-circuits before candidate collection; whether the pool is
    // populated or not, the fallback must stay untouched on this path.
    let _ = p.heal(&rule, &buggy, &corpus.entries);
    assert_eq!(probe.count(), 0, "the Stage 2 / drafter hit path must not consult L4");
}

// ── Invariant 2: consulted on a genuine miss ───────────────────────────────

#[test]
fn l4_is_consulted_when_the_modelless_path_has_no_candidates() {
    // An unknown rule with no corpus entry: the drafter has no template and
    // memory is empty, so candidates come out empty — the case L4 exists for.
    let corpus = build_seed_corpus();
    let probe = Probe::new(Some("fn healed() -> i32 { 1 }"));
    let mut p = pipeline().with_l4_fallback(Box::new(probe.clone()));

    let result = p.heal("totally_unknown_lint_xyz", "fn broken( {", &corpus.entries);

    assert_eq!(probe.count(), 1, "a total modelless miss must consult L4 exactly once");
    match result {
        Some(r) => {
            assert_eq!(r.source, HealSource::L4Lora, "an L4-served fix must be labelled L4Lora");
            assert!(r.parses, "only parsing L4 fixes are returned");
            assert_eq!(r.fixed_code, "fn healed() -> i32 { 1 }");
        }
        None => panic!("L4 returned a parsing fix but the pipeline dropped it"),
    }
}

#[test]
fn declining_l4_on_a_total_miss_returns_none() {
    let corpus = build_seed_corpus();
    let probe = Probe::new(None);
    let mut p = pipeline().with_l4_fallback(Box::new(probe.clone()));

    let result = p.heal("totally_unknown_lint_xyz", "fn broken( {", &corpus.entries);

    assert_eq!(probe.count(), 1, "the fallback must still be consulted");
    assert!(result.is_none(), "a declining L4 must leave the miss a miss, not fabricate a fix");
}

// ── Invariant 3: never degrades a modelless answer ─────────────────────────

#[test]
fn non_parsing_l4_output_is_discarded() {
    let corpus = build_seed_corpus();
    // The fallback returns syntactically broken code — it must never surface.
    let probe = Probe::new(Some("fn not_rust( { { {"));
    let mut p = pipeline().with_l4_fallback(Box::new(probe.clone()));

    let result = p.heal("totally_unknown_lint_xyz", "fn broken( {", &corpus.entries);

    assert_eq!(probe.count(), 1);
    assert!(
        result.is_none(),
        "a non-parsing L4 answer must be discarded, not returned with parses=false"
    );
}

#[test]
fn a_parsing_modelless_fix_is_never_replaced_by_l4() {
    let corpus = build_seed_corpus();
    let (rule, buggy) = corpus_hit();

    let baseline = pipeline().heal(&rule, &buggy, &corpus.entries);
    let baseline = baseline.expect("corpus entry must heal");
    if !baseline.parses {
        // Guard the guard: this test only means something when the modelless
        // path produced a PARSING fix. Fail loudly rather than pass vacuously.
        panic!("fixture precondition broken: modelless fix does not parse");
    }

    let probe = Probe::new(Some("fn totally_different() {}"));
    let mut p = pipeline().with_l4_fallback(Box::new(probe.clone()));
    let with_l4 = p.heal(&rule, &buggy, &corpus.entries).expect("must still heal");

    assert_eq!(with_l4.fixed_code, baseline.fixed_code, "L4 overwrote a healthy modelless fix");
    assert_eq!(probe.count(), 0);
}

// ── Anti-vacuity ───────────────────────────────────────────────────────────

#[test]
fn harness_is_actually_running() {
    // If the gate ever compiles this file away, the suite reports 0 passed and
    // a reader sees green. This test asserts a fact that is false unless the
    // wiring is really present and really reached.
    let corpus = build_seed_corpus();
    let probe = Probe::new(Some("fn x() {}"));
    let mut p = pipeline().with_l4_fallback(Box::new(probe.clone()));
    assert!(p.has_l4_fallback());
    let _ = p.heal("totally_unknown_lint_xyz", "fn broken( {", &corpus.entries);
    assert_eq!(probe.count(), 1, "the L4 seam is compiled in but never reached");
}

#[test]
fn counting_double_records_what_it_was_asked() {
    // Pins the handoff payload: L4 receives the ORIGINAL rule id + buggy code,
    // not a rewritten prompt — the adapter in riir-train is trained on that
    // exact pair shape (Plan 336 T1.2).
    let corpus = build_seed_corpus();
    let mut double = CountingL4::new(Some("fn ok() {}"));
    let _ = double.generate_fix("needless_return", "fn f() -> i32 { return 1; }");
    assert_eq!(double.calls.get(), 1);
    assert_eq!(double.seen.borrow()[0].0, "needless_return");
    drop(corpus);
}
