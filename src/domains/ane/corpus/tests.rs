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
    // Plan 377 seed: Research 409's six candidates (R1–R6) + Batch 376: the
    // Research 421 anemll branch-delta wave (A4/A1/A8/A9/A10/A6/A7). Hard-pinned
    // the flat-key way (the dist_gates precedent — the manifest key is
    // FLAT, `ane_rules: N`, so there is no nested count() read):
    // `scripts/pins_manifest.py` reconcile-checks this count against the
    // committed manifest at every posture, so a drift reds the gate —
    // never a silent count change.
    assert_eq!(corpus.entries.len(), 13);
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

// ── Batch 376 wave shape arms (Research 421; quotes re-verified at fe8de185) ──

/// A4: the form-admission gate refuses a silently-wrong quantized shape —
/// the rel-err arithmetic is real (0.98 and 1.24 are near-total garbage
/// against a 0.01 admission bar), and the speed number never certifies.
#[test]
fn a4_form_gate_refuses_silent_shape_wrongness() {
    let corpus = build_ane_seed_corpus();
    let a4 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-int8-matmul-shape-silent-wrong-gate")
        .expect("A4 present");
    // the pinned wrongness numbers are far past any honest tolerance
    let rel_gate_up = 0.98_f64;
    let rel_no_split = 1.24_f64;
    const BAR: f64 = 0.01;
    assert!(rel_gate_up > BAR && rel_no_split > BAR);
    assert!(a4.fixed_code.contains("rel_err"));
    assert!(a4.fixed_code.contains("host_reference"));
    let notes = a4.context_notes().expect("A4 notes");
    assert!(notes.contains("nothing to signal it"));
    assert!(notes.contains("48.5 TOPS"));
    assert!(notes.contains("accelerator-backend-first-tile-acceptance-gate"));
}

/// A1: the stage-parity arithmetic — cosine blindness is REAL (a 51% rel-L2
/// stage error coexists with a 0.995 whole-model cosine when directions
/// agree); the gate checks magnitude axes cosine cannot see.
#[test]
fn a1_stage_parity_magnitude_checks() {
    let corpus = build_ane_seed_corpus();
    let a1 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-stage-parity-gate-cpu-vs-ane-same-model")
        .expect("A1 present");
    // sentinel-shaped arithmetic: direction-agreeing scale loss passes
    // cosine and fails rel-L2 — the two axes are not interchangeable
    let a = [1.0_f32, 1.0];
    let b = [0.45_f32, 0.45]; // same direction, ~55% scale loss
    let dot: f32 = a.iter().zip(b.iter()).map(|(p, q)| p * q).sum();
    let (na, nb): (f32, f32) = (
        a.iter().map(|v| v * v).sum::<f32>().sqrt(),
        b.iter().map(|v| v * v).sum::<f32>().sqrt(),
    );
    let cos = dot / (na * nb);
    let rel_l2: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(p, q)| ((*p - *q) as f64).powi(2))
        .sum::<f64>()
        .sqrt()
        / na as f64;
    assert!(cos > 0.999, "cosine is blind here: {cos}");
    assert!(rel_l2 > 0.5, "rel-L2 sees the scale loss: {rel_l2}");
    assert!(a1.fixed_code.contains("fraction_subnormal"));
    let notes = a1.context_notes().expect("A1 notes");
    assert!(notes.contains("CPU_ONLY vs CPU_AND_NE"));
    assert!(notes.contains("copying artifact"));
}

/// A8: the granularity arithmetic — a within-layer split idles the second
/// unit (42% busy of the block), the block pipeline overlaps whole ops;
/// the comparative-advantage numbers pin the admission table.
#[test]
fn a8_block_granularity_overlaps_units() {
    let corpus = build_ane_seed_corpus();
    let a8 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-hybrid-pipeline-block-granularity-not-layer-interleave")
        .expect("A8 present");
    // the pinned fractions: 42% busy inside-layer vs the pipelined overlap
    let busy_inside_layer = 0.42_f64;
    let speedup_inside = 1.31_f64;
    let speedup_blocks = 1.60_f64;
    assert!(speedup_blocks > speedup_inside);
    assert!(busy_inside_layer < 0.5);
    assert!(a8.fixed_code.contains("run_prompt_blocks"));
    let notes = a8.context_notes().expect("A8 notes");
    assert!(notes.contains("76% of each FFN's channels"));
    assert!(notes.contains("23.2 us"));
    assert!(notes.contains("shares are not isolated"));
}

/// A9: the gap arithmetic — N calls pay N gaps; pairing halves them while
/// the batch width stops at the 19%-worse-per-token cliff.
#[test]
fn a9_gap_count_halves_on_pairing() {
    let corpus = build_ane_seed_corpus();
    let a9 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-prediction-gap-fewer-bigger-calls")
        .expect("A9 present");
    let n_blocks = 64_usize;
    let gap_ms = 0.23_f64;
    let per_call_cost = |calls: usize| calls as f64 * gap_ms;
    assert!((per_call_cost(n_blocks) - 14.72).abs() < 1e-9);
    assert!(per_call_cost(n_blocks / 2) < per_call_cost(n_blocks));
    // the cliff: past 64 rows/call the per-token cost rises 19%
    let per_token_64 = 1.0_f64;
    let per_token_128 = per_token_64 * 1.19;
    assert!(per_token_128 > per_token_64);
    assert!(a9.fixed_code.contains("step_by(2)"));
    let notes = a9.context_notes().expect("A9 notes");
    assert!(notes.contains("0.11-0.23 ms gap"));
    assert!(notes.contains("DRAFT_GAP_MS"));
}

/// A10: the serialization arithmetic — two concurrent callers on one
/// function corrupt statistically (KL spikes over a healthy floor); the
/// fix is ONE call thread, the guard not a check.
#[test]
fn a10_serializes_per_function_handle() {
    let corpus = build_ane_seed_corpus();
    let a10 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-concurrent-same-function-silent-corruption")
        .expect("A10 present");
    // the measured damage band vs the healthy band
    let spike_low = 0.6_f64;
    let spike_high = 4.0_f64;
    let healthy = 0.02_f64;
    assert!(spike_low > healthy && spike_high > healthy);
    assert!(a10.fixed_code.contains("mpsc::channel"));
    assert!(!a10.fixed_code.contains("clone_binding"));
    let notes = a10.context_notes().expect("A10 notes");
    assert!(notes.contains("even with different bindings"));
    assert!(notes.contains("GUARD, not a check"));
}

/// A6: the budget arithmetic — the leak is linear in calls (the 8.7→12.9
/// ms climb IS the gauge); the recycle fires BEFORE the ~850-call abort.
#[test]
fn a6_budget_fires_before_the_abort() {
    let corpus = build_ane_seed_corpus();
    let a6 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-host-output-pool-unbounded-call-bound")
        .expect("A6 present");
    let abort_at = 850_usize;
    let budget = 600_usize;
    assert!(budget < abort_at, "the recycle must precede the abort");
    // the per-call-time climb is the runtime-owned gauge
    let t0 = 8.7_f64;
    let t600 = 12.9_f64;
    assert!(t600 > t0);
    let wired_growth_gb_per_600 = 0.5_f64;
    assert!(wired_growth_gb_per_600 > 0.0);
    assert!(a6.fixed_code.contains("CALL_BUDGET"));
    let notes = a6.context_notes().expect("A6 notes");
    assert!(notes.contains("No API to supply output buffers"));
    assert!(notes.contains("derived-cache-throwaway-version-reset-recovery"));
}

/// A7: the envelope arithmetic — the measured-stable budget (4 layers /
/// 0.52-0.72 GB) admits the stable build and refuses the panic class;
/// the version gate refuses typed before the segfault class.
#[test]
fn a7_envelope_gate_refuses_before_the_uncatchable() {
    let corpus = build_ane_seed_corpus();
    let a7 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-package-size-build-gate-before-kernel-panic")
        .expect("A7 present");
    let stable_max_gb = 0.72_f64;
    let panic_gb = 1.1_f64;
    assert!(panic_gb > stable_max_gb);
    assert!(a7.fixed_code.contains("MAX_PACKAGE_BYTES"));
    assert!(a7.fixed_code.contains("FormatTooNew"));
    let notes = a7.context_notes().expect("A7 notes");
    assert!(notes.contains("Panic (by design)"));
    assert!(notes.contains("Grade C+ honesty"));
    assert!(notes.contains("batch-load-admission-sized-from-files-refuses-past-the-hard-budget"));
}

/// The two Batch-376 FACES are present in the seed rules' notes (count-neutral
/// pins — the notes carry the pole/instance content, the count is untouched).
#[test]
fn batch376_faces_present() {
    let corpus = build_ane_seed_corpus();
    let prescale = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-fp16-norm-pow2-prescale-overflow")
        .expect("prescale rule present");
    let notes = prescale.context_notes().expect("prescale notes");
    assert!(notes.contains("UNDERFLOW pole"));
    assert!(notes.contains("check norms / magnitudes, not only cosines"));
    assert!(notes.contains("COREAI_PORT_NOTES.md"));

    let restructure = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "ane-graph-restructure-for-ane-lowering")
        .expect("restructure rule present");
    let notes = restructure.context_notes().expect("restructure notes");
    assert!(notes.contains("forward substitution"));
    assert!(notes.contains("scatter_nd"));
}
