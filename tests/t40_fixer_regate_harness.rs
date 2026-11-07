//! Issue 040 — offline validation of the fixer re-gate harness.
//!
//! The planted-control battery + corpus self-consistency that `fixer_regate
//! --arm DRY` prints, pinned as tests. These validate the INSTRUMENT (merge,
//! oracle, vetoes), not any model — the model arms A/B/C are what the issue
//! gates on, and they need a CF token / a Bonsai checkpoint respectively.
//!
//! Slow (spawns real `cargo clippy` per case, ~10 s for the planted battery;
//! the full-corpus pass is `#[ignore]`d, ~1 min) — run with:
//!
//! ```sh
//! cargo test --features cf_workers_ai --test t40_fixer_regate_harness -- --nocapture
//! cargo test --features cf_workers_ai --test t40_fixer_regate_harness \
//!     v2edit_full_corpus_self_consistency -- --ignored --nocapture
//! ```

#![cfg(feature = "cf_workers_ai")]

use riir_refine::cf_workers_ai::{self, reachable_samples, MissClass, RegateHarness, SampleSource};
use riir_refine::ClippyVerifier;

fn harness() -> RegateHarness {
    RegateHarness::new().expect("harness init (corpus verify + reachable premise)")
}

fn behav_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("t40_behav_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&d).ok();
    d
}

/// The reachable set is the bar's sample set: all off-corpus, all declined by
/// the modelless pipeline (t51 arm0's assertion, re-measured through the
/// Issue 040 harness).
#[test]
fn reachable_set_is_off_corpus_and_declined() {
    let mut h = harness();
    for s in reachable_samples() {
        let (class, modelless) = h.classify(&s.lints[0], &s.module);
        assert_eq!(class, MissClass::OffCorpus, "{}", s.id);
        assert!(
            modelless.is_none(),
            "{}: expected decline, got a serve",
            s.id
        );
    }
}

/// Plan 335 P0: every reachable fixture must FIRE its own lint — a dead
/// control makes that case unscoreable in every arm, so the "n=9" bar
/// silently reads over n=7 (found live: the clear_with_drain + unused_self
/// re-picks never fired on clippy 0.1.98). Scored as a noop echo: the
/// control half is all this reads.
#[test]
fn every_reachable_fixture_fires_its_control() {
    let verifier = ClippyVerifier::new();
    let dir = behav_dir("controls");
    let dead: Vec<String> = reachable_samples()
        .iter()
        .filter(|s| !cf_workers_ai::score_case(&verifier, s, &s.module, Some(&dir)).control_fires)
        .map(|s| s.id.clone())
        .collect();
    assert!(
        dead.is_empty(),
        "dead reachable controls (re-pick the fixture shape): {dead:?}"
    );
}

/// The planted-control battery: known-good fixes must `keep_strict`, and the
/// three negative shapes (noop echo / destructive item-drop / garbage) must
/// not. This is the instrument's positive AND negative validation — the
/// Bench 039 lesson (validate a verifier on real data in both directions or
/// it is the same mistake with more steps) applied to this harness.
#[test]
fn planted_controls_score_correctly_through_the_real_instrument() {
    // Harness construction itself asserts the corpus + reachable premise.
    let _ = harness();
    let samples = reachable_samples();
    let dir = behav_dir("planted");
    let verifier = ClippyVerifier::new();

    let find = |id: &str| samples.iter().find(|s| s.id == id).unwrap();
    let score = |id: &str, gen: &str| {
        let s = find(id);
        cf_workers_ai::score_case(&verifier, s, gen, Some(&dir))
    };

    // Positives (Bench 039's verified-clean wins, full-module verbatim).
    // `manual_strip` replaced `ptr_arg` here when the 2026-09-18 re-pick
    // (Batch 155) made ptr_arg a corpus entry — probe-checked to FIRE on its
    // fixture against this box's clippy before landing (the clear_with_drain
    // and unused_self re-picks did NOT fire on their first fixture shapes;
    // Plan 335 P0 repaired both — `every_reachable_fixture_fires_its_control`
    // now walls a dead control, since a dead one silently shrinks every
    // arm's n=9 bar to n=7 scoreable).
    for (id, gen) in [
        (
            "reachable:manual_strip",
            "pub fn f(s: &str) -> Option<&str> { s.strip_prefix(\"ab\") }",
        ),
        (
            "reachable:or_fun_call",
            "pub fn f(opt: Option<u32>) -> u32 { opt.unwrap_or_else(expensive) }\npub fn expensive() -> u32 { 42 }",
        ),
        (
            "reachable:redundant_clone",
            "pub fn f() { let s = String::from(\"hi\"); drop(s); }",
        ),
    ] {
        let s = score(id, gen);
        assert!(s.keep_oracle, "{id}: positive control must pass the oracle filter: {s:?}");
        assert!(s.keep_strict, "{id}: positive control must pass the strict filter: {s:?}");
    }

    // Negative: noop echo — findings unchanged, never kept.
    let s = score(
        "reachable:manual_strip",
        "pub fn f(s: &str) -> Option<&str> { if s.starts_with(\"ab\") { Some(&s[2..]) } else { None } }",
    );
    assert!(!s.keep_oracle, "noop must not pass the oracle filter");
    assert!(s.noop);

    // Negative: destructive item-drop — lint resolves but `mk` is lost; the
    // contract veto must fire (this is Bench 039's route 2).
    let s = score("reachable:box_collection", "pub struct S { v: Vec<u8> }");
    assert!(
        s.keep_oracle,
        "destructive control must pass the raw oracle filter (that is the point — the oracle alone cannot see it)"
    );
    assert!(!s.contract_ok, "contract veto must fire on a dropped item");
    assert!(
        !s.keep_strict,
        "destructive control must NOT survive the strict filter"
    );

    // Negative: garbage never merges.
    let s = score("reachable:manual_strip", "}}}not rust{{{");
    assert!(s.merge_strategy.is_none());
    assert!(!s.keep_oracle && !s.keep_strict);
}

/// Behavioural equivalence is wired and REFUTES on a real silent logic
/// change: `o.unwrap_or(0) + 1` resolves `option_if_let_else`-style lints,
/// preserves every contract identifier, and changes `None`-branch behaviour —
/// exactly the Bench 039 route-3 case.
#[test]
fn behavioural_veto_refutes_the_real_route3_generation() {
    let mut h = harness();
    let samples = reachable_samples();
    let s = samples
        .iter()
        .find(|s| s.id == "reachable:option_if_let_else")
        .unwrap();
    let (class, _) = h.classify(&s.lints[0], &s.module);
    assert_eq!(class, MissClass::OffCorpus);

    let dir = behav_dir("route3");
    let verifier = ClippyVerifier::new();
    let wrong_but_passes_names = "pub fn f(o: Option<u32>) -> u32 { o.unwrap_or(0) + 1 }";
    let sc = cf_workers_ai::score_case(&verifier, s, wrong_but_passes_names, Some(&dir));
    assert!(
        sc.contract_ok,
        "precondition: the route-3 shape preserves every name (names cannot catch it)"
    );
    assert!(
        matches!(
            sc.behavioural,
            Some(cf_workers_ai::BehavSummary::Divergent { .. })
        ),
        "behavioural check must refute the silent logic change: {:?}",
        sc.behavioural
    );
    assert!(
        !sc.keep_strict,
        "route-3 generation must NOT survive the strict filter"
    );
}

/// v2edit self-consistency on chunk 00 (60 rows): EM must be 1.0 by
/// construction (the reference IS the expected text — anything else means
/// the EM machinery diverged from the driver's semantics), and the merge
/// rate + control rate are the measured corpus properties recorded in
/// Bench 049.
#[test]
fn v2edit_chunk00_self_consistency() {
    // Harness construction itself asserts the corpus + reachable premise.
    let _ = harness();
    let dir = std::path::Path::new("/tmp/t40_chunk00");
    let data = std::path::Path::new("../riir-train/data");
    if !data.join("clippy_l4_v2edit_test_chunk_00.jsonl").exists() {
        eprintln!("SKIP: v2edit chunks not reachable from this cwd");
        return;
    }
    let samples = cf_workers_ai::load_v2edit(data, &[0], 0).expect("chunk 00 loads");
    assert_eq!(samples.len(), 60);

    let verifier = ClippyVerifier::new();
    let (mut em, mut merge, mut control) = (0usize, 0, 0);
    for s in &samples {
        let exp = s.expected.as_deref().unwrap();
        let sc = cf_workers_ai::score_case(&verifier, s, exp, Some(dir));
        em += usize::from(sc.em == Some(true));
        merge += usize::from(sc.merge_strategy.is_some());
        control += usize::from(sc.control_fires);
    }
    // EM 60/60 is the load-bearing assertion (extract+normalize match the
    // driver's EM semantics exactly). The merge/control numbers are recorded
    // measurements, not gates — they encode corpus properties.
    assert_eq!(em, 60, "EM must be exact against the reference text");
    eprintln!(
        "[t40] chunk00: merge {merge}/60, control {control}/60 (corpus properties — see Bench 049)"
    );
    assert!(merge >= 1, "at least row 0 must merge (lcs-bounded)");
}

/// The full 408-row corpus self-consistency (`#[ignore]`d, ~1 min): the
/// merge-rate + control-rate + invented-lint-label numbers quoted in
/// Bench 049 come from this run.
#[test]
#[ignore = "full-corpus clippy sweep (~1 min, 800+ cargo invocations)"]
fn v2edit_full_corpus_self_consistency() {
    // Premise (corpus verify + reachable-miss assertion) is asserted at
    // construction; no further harness state is needed here.
    let _ = harness();
    let data = std::path::Path::new("../riir-train/data");
    if !data.join("clippy_l4_v2edit_test_chunk_00.jsonl").exists() {
        eprintln!("SKIP: v2edit chunks not reachable from this cwd");
        return;
    }
    let samples = cf_workers_ai::load_v2edit(data, &[0, 1, 2, 3, 4, 5, 6], 0).unwrap();
    assert_eq!(samples.len(), 408);
    let verifier = ClippyVerifier::new();
    let dir = std::path::Path::new("/tmp/t40_fullcorpus");
    let (mut em, mut merge, mut control, mut kept) = (0usize, 0, 0, 0);
    for s in &samples {
        let exp = s.expected.as_deref().unwrap();
        let sc = cf_workers_ai::score_case(&verifier, s, exp, Some(dir));
        em += usize::from(sc.em == Some(true));
        merge += usize::from(sc.merge_strategy.is_some());
        control += usize::from(sc.control_fires);
        kept += usize::from(sc.keep_oracle);
    }
    assert_eq!(em, 408, "EM must be exact across the full corpus");
    eprintln!("[t40] full corpus: merge {merge}/408, control {control}/408, keep_o {kept}/408");
    // The recorded Bench 049 numbers (2026-08-25): merge 14/408, control
    // 341/408, keep_o 9/408. A drift here is a corpus or clippy-version
    // change worth noting, not necessarily a regression — but EM must hold.
    assert_eq!(samples[0].source, SampleSource::V2edit);
}
