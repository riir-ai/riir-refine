//! The ane seed-corpus gates (Plan 377) — the batch-fixture discipline at
//! seed scale: freeze/thaw round-trip, the pins-manifest count, a parse gate
//! over every buggy/fixed shape, and one SHAPE arm per rule (the fixture
//! executed before entry — each arm asserts the rule's decision content on
//! a synthetic instance, sentinel-shaped where the real oracle is the
//! CoreML/ANE runtime).

use super::build_ane_seed_corpus;
use crate::corpus::Corpus;
use crate::domain::{HealDomain, HealEntry};
use crate::domains::ane::{AneDomain, AneRule};

#[test]
fn seed_corpus_round_trips() {
    let corpus = build_ane_seed_corpus();
    assert!(corpus.verify());
    let frozen = corpus.freeze().expect("freeze");
    let thawed = Corpus::<AneDomain>::thaw(&frozen).expect("thaw");
    assert_eq!(thawed.commitment, corpus.commitment);
    assert!(thawed.verify());
}

#[test]
fn seed_corpus_has_expected_entry_count() {
    let corpus = build_ane_seed_corpus();
    // Plan 377 seed: Research 409's six candidates (R1–R6). Hard-pinned
    // the flat-key way (the dist_gates precedent — the manifest key is
    // FLAT, `ane_rules: N`, so there is no nested count() read):
    // `scripts/pins_manifest.py` reconcile-checks this count against the
    // committed manifest at every posture, so a drift reds the gate —
    // never a silent count change.
    assert_eq!(corpus.entries.len(), 6);
}

#[test]
fn seed_corpus_rule_ids_unique() {
    let corpus = build_ane_seed_corpus();
    let mut ids: Vec<&str> = corpus.entries.iter().map(AneRule::rule_id).collect();
    let n = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), n, "duplicate rule id in the ane seed corpus");
}

/// Every buggy AND fixed shape must parse as a standalone Rust file (the L2
/// contract the retrieval pruner rides — the kernel corpus's own bar).
#[test]
fn every_entry_parses_as_rust() {
    let corpus = build_ane_seed_corpus();
    for entry in &corpus.entries {
        for (kind, code) in [("buggy", entry.buggy_code()), ("fixed", entry.fixed_code())] {
            syn::parse_file(code)
                .unwrap_or_else(|e| panic!("{} {kind} shape must parse: {e}", entry.rule_id()));
        }
    }
}

/// The `context_notes` channel carries the honest disclosure on EVERY rule —
/// the `NotBenchable` annotation is never missing (a rule that looks
/// benchmark-backed downstream when it is suggestion-only is the failure).
#[test]
fn every_entry_discloses_not_benchable() {
    let corpus = build_ane_seed_corpus();
    for entry in &corpus.entries {
        let notes = entry
            .context_notes()
            .unwrap_or_else(|| panic!("{} carries no notes", entry.rule_id()));
        assert!(
            notes.contains("NotBenchable-by-domain"),
            "{} must disclose NotBenchable-by-domain",
            entry.rule_id()
        );
        assert!(
            notes.contains("@ "),
            "{} must cite its source pin",
            entry.rule_id()
        );
    }
}

/// The domain validator is parse-only, honestly: a valid pattern passes, a
/// garbage pattern fails, the same input twice gives the same verdict.
#[test]
fn validate_is_parse_only_and_deterministic() {
    let ok = AneDomain::validate("rule", "fn main() { let x = 1; }");
    assert!(ok.parses);
    let bad = AneDomain::validate("rule", "fn ( broken");
    assert!(!bad.parses);
    assert_eq!(
        ok.parses,
        AneDomain::validate("rule", "fn main() { let x = 1; }").parses
    );
}

// ── Per-rule shape arms (the fixtures executed before entry) ────────────────

/// R1: the admission gate is REAL — the receptive field (117) fits the
/// window (120), the mask zeroes the invalid tail, and the buggy shape is
/// the STATEFUL one (the chunked ≡ full decision content, sentinel-shaped).
#[test]
fn r1_fixed_window_admission_gate_and_mask() {
    let corpus = build_ane_seed_corpus();
    let r1 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-fixed-window-mask-stateless-streaming")
        .expect("R1 present");
    // The decision content, as pinned numbers (5*6 + 3*29 = 117 < 120).
    assert_eq!(5 * 6 + 3 * 29, 117);
    let window: usize = 120;
    assert!(
        5 * 6 + 3 * 29 <= window,
        "the admission gate must hold at the pinned shapes"
    );
    assert!(r1.fixed_code.contains("receptive_field() <= WINDOW"));
    assert!(r1.fixed_code.contains("mask_tail"));
    // The buggy shape is the stateful one (cache carried across chunks).
    assert!(r1.buggy_code.contains("cache: Vec<f32>"));
    assert!(!r1.fixed_code.contains("self.cache"));
}

/// R2: the prescale keeps the squared statistic under the fp16 max at the
/// pinned shapes (max |x| 2048 / 64 → 32; 32² = 1024 << 65,504).
#[test]
fn r2_prescale_keeps_sq_under_fp16_max() {
    let max_abs = 2048.0_f32;
    let prescale = 64.0_f32;
    let scaled_max = max_abs / prescale;
    assert!(scaled_max <= 32.0);
    assert!((scaled_max * scaled_max) < 65_504.0);
    let corpus = build_ane_seed_corpus();
    let r2 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-fp16-norm-pow2-prescale-overflow")
        .expect("R2 present");
    assert!(r2.fixed_code.contains("PRESCALE_SHIFT"));
    // The buggy shape squares the raw residual (the overflow pole).
    assert!(r2.buggy_code.contains("v * v"));
}

/// R3: the fixed shape pins cpuAndNeuralEngine and the buggy shape carries
/// the `.all` demotion (the placement decision content).
#[test]
fn r3_compute_units_pinned_off_all() {
    let corpus = build_ane_seed_corpus();
    let r3 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-compute-plan-pin-cpu-and-neural-engine")
        .expect("R3 present");
    assert!(r3.fixed_code.contains("CpuAndNeuralEngine"));
    assert!(r3.buggy_code.contains("All"));
}

/// R4: the fixed shape folds projections (one weight blob, conv1x1) while
/// the buggy shape separates them (the lowering decision content).
#[test]
fn r4_graph_shape_folds_projections() {
    let corpus = build_ane_seed_corpus();
    let r4 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-graph-restructure-for-ane-lowering")
        .expect("R4 present");
    assert!(r4.fixed_code.contains("conv1x1"));
    assert!(r4.fixed_code.contains("w_qk"));
    assert!(r4.buggy_code.contains("matmul(x, w_q)"));
    // The regime evidence (the negative control) is quoted in notes.
    assert!(r4.context_notes().is_some_and(|n| n.contains("386 of 464")));
}

/// R5: the fixed shape splits the DSP edge OUT of the learned stack (CPU
/// fp32 head/tail, ANE decode) — the buggy shape rides fp16 whole.
#[test]
fn r5_dsp_edge_stays_cpu_fp32() {
    let corpus = build_ane_seed_corpus();
    let r5 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-precision-dsp-stays-cpu-fp32")
        .expect("R5 present");
    assert!(r5.fixed_code.contains("stft_fp32_cpu"));
    assert!(r5.fixed_code.contains("overlap_add_fp32_cpu"));
    assert!(r5.fixed_code.contains("decode_ane"));
    assert!(r5.buggy_code.contains("stft_fp16"));
}

/// R6: the fixed shape GATES the palette depth on measured retention (the
/// buggy shape adopts a published claim unverified) — and the notes record
/// the two-source 6-bit discrepancy verbatim.
#[test]
fn r6_palette_depth_gated_on_measured_retention() {
    let corpus = build_ane_seed_corpus();
    let r6 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-weight-palette-depth-retention-gate")
        .expect("R6 present");
    assert!(r6.fixed_code.contains("retention_gate"));
    assert!(r6.buggy_code.contains("6-bit"));
    let notes = r6.context_notes().expect("R6 notes");
    assert!(notes.contains("word-for-word identical"));
    assert!(notes.contains("degraded the transcript"));
    assert!(notes.contains("METHOD-dependent"));
}
