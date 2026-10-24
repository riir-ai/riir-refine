//! Entries slice - Batch 274, Slice B (2026-10-24): the splash 122B
//! release-wave pool (1 allocation rule) - incoai/splash @
//! 28d0e98ce4a98c88edeb562ddea2f9c38009fadd, Apache-2.0; every quote
//! re-verified verbatim at the vendor clone (the `splash` checkout at
//! `vendor/splash`, origin/main `3135ab4`) this batch. The rule is corpus-only `NotBenchable` (decision-content
//! admission law; the grade and the source-reported figures ride the
//! `context_notes`) - no proposer files. Fixtures are ORIGINAL machinery (never
//! splash code verbatim), each compiled and executed standalone
//! (rustc --edition 2021 --test) before this table was assembled (arms 5/5 +
//! 7/7, exit 0 both); the buggy/fixed arm pair shares contract tests and
//! differs only on the law.
use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "batch-load-admission-sized-from-files-refuses-past-the-hard-budget".to_string(),
            name: "A batch load admits against the budget BEFORE the first unit loads: the count is sized from the FILES (metadata, nothing read yet), each streaming store at the SIZE its options give it but NO MORE THAN ITS PART WHOLE (all a store holds -- the consumer planner's own resolution of a SIZE past the part), the ceiling is the planner's HARD budget -- the largest budget any consumer run plans in, so a knob that only lowers it can never cause a false refusal -- and a refusal names BOTH counts and the options that bring the load under the ceiling; a load without the fit check reads unit after unit and then fails mid-load with a device error that names no remedy, and a count at the uncapped SIZE mis-prices the load by the whole SIZE-past-part difference"
                .to_string(),
            category: "allocation".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `batch-load-admission-sized-from-files-refuses-past-the-hard-budget`
// (riir-refine rust_perf corpus, Batch 274 / Slice B) -- the ANTI-PATTERN arm.
//
// Re-authored shape from incoai/splash dev/tools/{weight_digests.mm,LoadOptions.hpp}
// @ 28d0e98 (Apache-2.0): a digest tool that loads every weight image its
// options leave resident with NO fit check -- a load the device's working set
// cannot hold reads image after image and then fails mid-load with a device
// error that names no remedy -- and a byte count that takes each streaming
// store at its option SIZE, past what the load can hold (a store holds no
// more than its part whole).
//
// ORIGINAL machinery -- never splash code verbatim. Images are byte counts
// sized from the files before any read; a streaming store "loads" its whole
// part regardless of the SIZE the options name (the SIZE only decides WHICH
// images exist).

use std::fmt;

pub const K_MIB: u64 = 1 << 20;

/// What the model's FILES say a load holds, read before anything loads.
pub struct Sizing {
    /// The dense layers' images, in load order (bytes).
    pub dense_images: Vec<u64>,
    /// The routed experts' image -- exists only while experts do NOT stream.
    pub expert_image: u64,
    /// The table's image -- exists only while the table does NOT stream.
    pub table_image: u64,
    /// The part whole of the expert store: all the store can hold.
    pub expert_part: u64,
    /// The part whole of the table store: all the store can hold.
    pub table_part: u64,
}

#[derive(Clone, Copy, Default)]
pub struct LoadOptions {
    /// The SIZE the options give the expert store (None: it does not stream).
    pub expert_size: Option<u64>,
    /// The SIZE the options give the table store (None: it does not stream).
    pub table_size: Option<u64>,
}

impl LoadOptions {
    pub fn streams_experts(&self) -> bool {
        self.expert_size.is_some()
    }
    pub fn streams_table(&self) -> bool {
        self.table_size.is_some()
    }
}

/// The images a load leaves resident: streaming a part leaves its image out.
pub fn resident_images(s: &Sizing, o: &LoadOptions) -> u64 {
    let mut total: u64 = s.dense_images.iter().sum();
    if !o.streams_experts() {
        total += s.expert_image;
    }
    if !o.streams_table() {
        total += s.table_image;
    }
    total
}

/// THE ANTI-COUNT: each streaming store at its option SIZE -- past what the
/// load holds. A SIZE past the part counts bytes the load never touches.
pub fn weight_bytes_naive(s: &Sizing, o: &LoadOptions) -> u64 {
    resident_images(s, o) + o.expert_size.unwrap_or(0) + o.table_size.unwrap_or(0)
}

#[derive(Default)]
pub struct Outcome {
    pub images_read: usize,
    pub digests: Vec<String>,
    pub failure: Option<&'static str>,
}

/// The device error that ends a doomed load: a command failure the message
/// does not explain and no remedy names.
pub const OPAQUE_DEVICE_ERROR: &str = "device command 1338 failed: Insufficient Memory";

/// THE ANTI-PATTERN: no fit check before the reads. Every image the options
/// leave resident is read in load order; a load past the working set stops
/// MID-LOAD with the opaque error -- every read so far was wasted, and the
/// message names no option that would stream the model down to size.
pub fn run_naive(s: &Sizing, o: &LoadOptions, working_set: u64) -> Outcome {
    let mut out = Outcome::default();
    let mut held = 0u64; // a streaming store's bytes never land: no image, no hold
    for (i, &b) in s.dense_images.iter().enumerate() {
        held += b;
        out.images_read += 1;
        if held > working_set {
            out.failure = Some(OPAQUE_DEVICE_ERROR);
            return out;
        }
        out.digests.push(format!("dense_{i}"));
    }
    if !o.streams_experts() {
        held += s.expert_image;
        out.images_read += 1;
        if held > working_set {
            out.failure = Some(OPAQUE_DEVICE_ERROR);
            return out;
        }
        out.digests.push("experts".to_string());
    }
    if !o.streams_table() {
        held += s.table_image;
        out.images_read += 1;
        if held > working_set {
            out.failure = Some(OPAQUE_DEVICE_ERROR);
            return out;
        }
        out.digests.push("table".to_string());
    }
    out
}

/// The start's own floor: a start refuses weights whose bytes WITH its
/// reserves exceed the hard budget -- so a start holds only bytes that fit
/// beside the reserves. (The reserves are the start's, never the tool's.)
pub struct EngineMemoryPolicy {
    pub hard_budget: u64,
    pub reserves: u64,
}

impl EngineMemoryPolicy {
    pub fn start_would_hold(&self, weight_bytes: u64) -> bool {
        weight_bytes + self.reserves <= self.hard_budget
    }
}

impl fmt::Debug for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Outcome")
            .field("images_read", &self.images_read)
            .field("digests", &self.digests.len())
            .field("failure", &self.failure)
            .finish()
    }
}

// -- shared contract tests (both arms) ----------------------------------------

pub fn fixture_sizing() -> Sizing {
    Sizing {
        dense_images: vec![600 * K_MIB, 900 * K_MIB, 700 * K_MIB], // 2.2 GiB dense
        expert_image: 4_000 * K_MIB,
        table_image: 300 * K_MIB,
        expert_part: 2_000 * K_MIB, // the expert store's part whole: 2 GiB
        table_part: 250 * K_MIB,    // the table store's part whole: 250 MiB
    }
}

#[test]
fn eq_resident_images_follow_the_options() {
    let s = fixture_sizing();
    let no_stream = LoadOptions::default();
    assert_eq!(
        resident_images(&s, &no_stream),
        2_200 * K_MIB + 4_000 * K_MIB + 300 * K_MIB
    );
    let both = LoadOptions { expert_size: Some(1_000 * K_MIB), table_size: Some(100 * K_MIB) };
    assert_eq!(
        resident_images(&s, &both),
        2_200 * K_MIB,
        "a streaming part leaves its image out"
    );
}

#[test]
fn eq_a_size_within_its_part_names_the_bytes_the_load_holds() {
    let s = fixture_sizing();
    let o = LoadOptions { expert_size: Some(1_000 * K_MIB), table_size: Some(100 * K_MIB) };
    // a SIZE at or under the part: the count and the held bytes agree
    assert_eq!(weight_bytes_naive(&s, &o), 2_200 * K_MIB + 1_000 * K_MIB + 100 * K_MIB);
    let out = run_naive(&s, &o, 6_000 * K_MIB);
    assert!(out.failure.is_none());
    assert_eq!(out.digests.len(), 3, "dense images only: both streaming parts left theirs out");
}

#[test]
fn eq_a_fitting_no_stream_load_runs_to_completion() {
    let s = fixture_sizing();
    let working_set = 8_000 * K_MIB; // holds dense + experts + table whole
    let out = run_naive(&s, &LoadOptions::default(), working_set);
    assert!(out.failure.is_none());
    assert_eq!(out.images_read, 5);
    assert_eq!(out.digests.len(), 5);
}

// -- the anti-pattern's own shape ---------------------------------------------

#[test]
fn bug_a_doomed_load_reads_image_after_image_then_fails_naming_no_remedy() {
    // The measured shape (the source's 63,302 MiB of images against a
    // 27,729 MiB working set): the tool had no fit check, so it read image
    // after image and stopped MID-LOAD with an error that names no option
    // that would have made the load fit -- every read wasted.
    let s = fixture_sizing();
    let out = run_naive(&s, &LoadOptions::default(), 4_000 * K_MIB);
    assert_eq!(out.failure, Some(OPAQUE_DEVICE_ERROR));
    assert!(out.images_read > 0, "the reads happened before the failure");
    assert!(out.digests.len() < 5, "the load died before every image was digested");
    assert!(!out.failure.unwrap().contains("--expert-memory"), "no remedy named");
}

#[test]
fn bug_the_uncapped_count_drifts_past_what_the_load_holds() {
    // A SIZE past the part counts bytes the load never touches: 32 GiB of
    // option SIZE over a 2 GiB part whole -- the count reports 32 GiB, the
    // load holds 2 GiB. Any admission priced on the naive count mis-prices
    // the load, and the drift grows with the option SIZE.
    let s = fixture_sizing();
    let o = LoadOptions { expert_size: Some(32_000 * K_MIB), table_size: Some(64_000 * K_MIB) };
    assert_eq!(weight_bytes_naive(&s, &o), 2_200 * K_MIB + 96_000 * K_MIB);
    // the load itself holds only the part whole: 2.2 + 2.0 + 0.25 GiB
    let held = resident_images(&s, &o) + s.expert_part + s.table_part;
    assert_eq!(held, 4_450 * K_MIB);
    assert!(weight_bytes_naive(&s, &o) > 20 * held, "the count is far past the load");
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `batch-load-admission-sized-from-files-refuses-past-the-hard-budget`
// (riir-refine rust_perf corpus, Batch 274 / Slice B) -- the LAW arm.
//
// Re-authored shape from incoai/splash dev/tools/{weight_digests.mm,LoadOptions.hpp}
// @ 28d0e98 (Apache-2.0): the law -- a batch load admits against the budget
// BEFORE the first image loads. The count is sized from the FILES (nothing has
// been read), each streaming store at the SIZE its options give it but no more
// than its part whole (all a store holds -- the planner's own resolution), and
// the budget is the planner's HARD budget -- the largest budget any start of
// the engine plans in, so a load a start would hold is never refused. A
// refusal names both counts and the options that stream.
//
// ORIGINAL machinery -- never splash code verbatim.

pub const K_MIB: u64 = 1 << 20;

/// What the model's FILES say a load holds, read before anything loads.
pub struct Sizing {
    /// The dense layers' images, in load order (bytes).
    pub dense_images: Vec<u64>,
    /// The routed experts' image -- exists only while experts do NOT stream.
    pub expert_image: u64,
    /// The table's image -- exists only while the table does NOT stream.
    pub table_image: u64,
    /// The part whole of the expert store: all the store can hold.
    pub expert_part: u64,
    /// The part whole of the table store: all the store can hold.
    pub table_part: u64,
}

#[derive(Clone, Copy, Default)]
pub struct LoadOptions {
    /// The SIZE the options give the expert store (None: it does not stream).
    pub expert_size: Option<u64>,
    /// The SIZE the options give the table store (None: it does not stream).
    pub table_size: Option<u64>,
}

impl LoadOptions {
    pub fn streams_experts(&self) -> bool {
        self.expert_size.is_some()
    }
    pub fn streams_table(&self) -> bool {
        self.table_size.is_some()
    }
}

/// The images a load leaves resident: streaming a part leaves its image out.
pub fn resident_images(s: &Sizing, o: &LoadOptions) -> u64 {
    let mut total: u64 = s.dense_images.iter().sum();
    if !o.streams_experts() {
        total += s.expert_image;
    }
    if !o.streams_table() {
        total += s.table_image;
    }
    total
}

/// THE COUNT: what a load holds, sized from the files before it loads -- the
/// images that load, and each streaming store at the SIZE it was given, but
/// no more than the part whole, which is all a store holds. A SIZE past the
/// part counts the part (the load never holds the difference).
pub fn weight_bytes(s: &Sizing, o: &LoadOptions) -> u64 {
    resident_images(s, o)
        + o.expert_size.unwrap_or(0).min(s.expert_part)
        + o.table_size.unwrap_or(0).min(s.table_part)
}

/// The planner's HARD budget: the largest budget any start of the engine
/// plans in (--max-memory only lowers it), and a start refuses weights whose
/// bytes WITH its reserves exceed it. A tool admission priced on this budget
/// therefore never refuses a load a start would hold.
pub struct EngineMemoryPolicy {
    pub hard_budget: u64,
    pub reserves: u64,
}

impl EngineMemoryPolicy {
    pub fn start_would_hold(&self, weight_bytes: u64) -> bool {
        weight_bytes + self.reserves <= self.hard_budget
    }
}

pub enum Verdict {
    /// The load fits: run it (the digests, in load order).
    Load(Vec<String>),
    /// Refused BEFORE any image loaded: both counts and the options that
    /// stream are named.
    Refuse { count_bytes: u64, budget_bytes: u64, remedy: String },
}

/// THE LAW: the fit check runs before the first read. A refusal costs the
/// count, not the load.
pub fn run(s: &Sizing, o: &LoadOptions, policy: &EngineMemoryPolicy) -> Verdict {
    let count = weight_bytes(s, o);
    if count > policy.hard_budget {
        return Verdict::Refuse {
            count_bytes: count,
            budget_bytes: policy.hard_budget,
            remedy: "--expert-memory SIZE streams the experts' store and --ple-memory SIZE the table's".to_string(),
        };
    }
    let mut digests = Vec::new();
    for (i, _) in s.dense_images.iter().enumerate() {
        digests.push(format!("dense_{i}"));
    }
    if !o.streams_experts() {
        digests.push("experts".to_string());
    }
    if !o.streams_table() {
        digests.push("table".to_string());
    }
    Verdict::Load(digests)
}

// -- shared contract tests (both arms) ----------------------------------------

pub fn fixture_sizing() -> Sizing {
    Sizing {
        dense_images: vec![600 * K_MIB, 900 * K_MIB, 700 * K_MIB], // 2.2 GiB dense
        expert_image: 4_000 * K_MIB,
        table_image: 300 * K_MIB,
        expert_part: 2_000 * K_MIB, // the expert store's part whole: 2 GiB
        table_part: 250 * K_MIB,    // the table store's part whole: 250 MiB
    }
}

#[test]
fn eq_resident_images_follow_the_options() {
    let s = fixture_sizing();
    let no_stream = LoadOptions::default();
    assert_eq!(
        resident_images(&s, &no_stream),
        2_200 * K_MIB + 4_000 * K_MIB + 300 * K_MIB
    );
    let both = LoadOptions { expert_size: Some(1_000 * K_MIB), table_size: Some(100 * K_MIB) };
    assert_eq!(
        resident_images(&s, &both),
        2_200 * K_MIB,
        "a streaming part leaves its image out"
    );
}

#[test]
fn eq_a_size_within_its_part_names_the_bytes_the_load_holds() {
    let s = fixture_sizing();
    let o = LoadOptions { expert_size: Some(1_000 * K_MIB), table_size: Some(100 * K_MIB) };
    // a SIZE at or under the part: the count and the load agree
    assert_eq!(weight_bytes(&s, &o), 2_200 * K_MIB + 1_000 * K_MIB + 100 * K_MIB);
    let policy = EngineMemoryPolicy { hard_budget: 6_000 * K_MIB, reserves: 0 };
    match run(&s, &o, &policy) {
        Verdict::Load(d) => assert_eq!(d.len(), 3, "dense images only"),
        Verdict::Refuse { .. } => panic!("a fitting load is never refused"),
    }
}

#[test]
fn eq_a_fitting_no_stream_load_runs_to_completion() {
    let s = fixture_sizing();
    let policy = EngineMemoryPolicy { hard_budget: 8_000 * K_MIB, reserves: 0 };
    match run(&s, &LoadOptions::default(), &policy) {
        Verdict::Load(d) => {
            assert_eq!(d.len(), 5);
            assert_eq!(d[3], "experts");
            assert_eq!(d[4], "table");
        }
        Verdict::Refuse { .. } => panic!("a fitting load is never refused"),
    }
}

// -- the law's own shape -------------------------------------------------------

#[test]
fn fixed_the_count_is_capped_at_each_part_whole() {
    // A SIZE past the part counts the part: 32 GiB of option SIZE over a
    // 2 GiB part whole counts 2 GiB -- the planner's own resolution ("a
    // store holds no more than all of its part"). The count matches what
    // the load holds at ANY option SIZE.
    let s = fixture_sizing();
    let small = LoadOptions { expert_size: Some(1_000 * K_MIB), table_size: Some(100 * K_MIB) };
    let huge = LoadOptions { expert_size: Some(32_000 * K_MIB), table_size: Some(64_000 * K_MIB) };
    assert_eq!(weight_bytes(&s, &small), 2_200 * K_MIB + 1_000 * K_MIB + 100 * K_MIB);
    assert_eq!(weight_bytes(&s, &huge), 2_200 * K_MIB + 2_000 * K_MIB + 250 * K_MIB);
}

#[test]
fn fixed_the_refusal_fires_before_any_image_loads_and_names_both_counts_and_the_options() {
    // The measured shape: 63 GiB of images against a 27 GiB budget. The
    // refusal costs ONE file-metadata read -- zero images read, zero reads
    // wasted -- and the message names the weights' bytes, the budget's
    // bytes, and the options that stream the load down to size.
    let s = fixture_sizing();
    let policy = EngineMemoryPolicy { hard_budget: 4_000 * K_MIB, reserves: 0 };
    match run(&s, &LoadOptions::default(), &policy) {
        Verdict::Refuse { count_bytes, budget_bytes, remedy } => {
            assert_eq!(count_bytes, 6_500 * K_MIB);
            assert_eq!(budget_bytes, 4_000 * K_MIB);
            assert!(remedy.contains("--expert-memory"), "the remedy names the streaming options");
        }
        Verdict::Load(_) => panic!("a load past the hard budget must be refused"),
    }
}

#[test]
fn fixed_a_load_a_start_would_hold_is_never_refused() {
    // The no-false-refusal half: the hard budget is the largest budget any
    // start plans in, and a start's own refusal includes its reserves -- so
    // EVERY weight count a start would hold is at or under the tool's
    // budget, and the tool's refusal never fires on one.
    let s = fixture_sizing();
    let policy = EngineMemoryPolicy { hard_budget: 8_000 * K_MIB, reserves: 300 * K_MIB };
    for cut in 0..=6_500 * K_MIB / 100 {
        let weights = cut * 100;
        let o = LoadOptions {
            expert_size: if weights > 2_200 * K_MIB { Some(weights - 2_200 * K_MIB) } else { None },
            table_size: None,
        };
        if policy.start_would_hold(weights) {
            match run(&s, &o, &policy) {
                Verdict::Load(_) => {}
                Verdict::Refuse { .. } => panic!("a start would hold {weights}; the tool refused it"),
            }
        }
    }
}

#[test]
fn fixed_streaming_options_bring_a_doomed_load_under_the_budget() {
    // The remedy is real: the same model that is refused whole is served
    // with the streaming options -- the refusal's named options, applied,
    // turn the verdict from Refuse to Load.
    let s = fixture_sizing();
    let policy = EngineMemoryPolicy { hard_budget: 4_000 * K_MIB, reserves: 0 };
    let whole = LoadOptions::default();
    assert!(matches!(run(&s, &whole, &policy), Verdict::Refuse { .. }));
    let streamed = LoadOptions { expert_size: Some(1_500 * K_MIB), table_size: Some(2_000 * K_MIB) };
    match run(&s, &streamed, &policy) {
        Verdict::Load(d) => assert_eq!(d.len(), 3, "both streaming parts left their images out"),
        Verdict::Refuse { .. } => panic!("the streamed load fits the budget"),
    }
}
"#.to_string(),
            context_notes: Some("Grade B+ -- NotBenchable ON THIS BOX (the decision content is a PRE-LOAD ADMISSION law -- where the fit check sits, what the count counts, and which ceiling it names -- priced by the source's refusal-before-load figures; a Rust microbench of the stand-in cannot measure the policy, the source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ 28d0e98ce4a98c88edeb562ddea2f9c38009fadd 'Refuse a weight-digests load the Metal memory budget cannot hold', Apache-2.0, Jian Chen, 2026-10-21; every quote re-verified verbatim at the vendor clone (origin/main 3135ab4) this batch; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (message, verbatim): 'weight-digests loaded every image its options left resident with no fit check.' -- measured (message): '(63302 MiB of images against a 27729 MiB Metal memory budget) read weights and then stopped with 'Metal command 1338 failed: Insufficient Memory (kIOGPUCommandBufferCallbackErrorOutOfMemory)', which names no remedy.' THE COUNT'S LAW (dev/tools/LoadOptions.hpp:57-60 @ 28d0e98, verbatim): 'What a load of the model in `root` with `options` holds, sized from its files before it loads (model::streamableWeights): the images that load, and the store of each part that streams, of the SIZE it was given, but no more than the part whole, which is all a store holds.' -- over the body (:63-67): 'streamable.loadedBytes + std::min(options.expertMemoryBytes, streamable.expertBytes) + std::min(options.streamedTableBytes, streamable.tableBytes)'. THE PART-WHOLE CAP is the planner's OWN resolution (message, verbatim): 'The count now takes each store at its SIZE but no more than its part whole, as the planner takes a SIZE past the part (resolve in MemoryPlan.cpp): a store holds no more than all of its part, the expert store's slots stopping at every expert. The oracle counted such a store at its SIZE, past what the load holds, so its count changes only where a SIZE exceeds its part.' THE REFUSAL (dev/tools/weight_digests.mm:57-59 @ 28d0e98, verbatim): 'No start of the engine loads weights past its Metal memory budget, which --max-memory only lowers, and a load past it can fail in a Metal command for memory: refuse it before any image loads.' -- the ceiling (:60-61): 'engine::EngineMemoryPolicy::hardBudgetBytes(backend.capabilities().recommendedMaxWorkingSetBytes, 0)'; the remedy in the message (:63-66): '--expert-memory SIZE streams a MoE target's routed experts and --ple-memory SIZE a Qwen3.8-Flash-Next target's PLE table'. THE NO-FALSE-REFUSAL HALF (message, verbatim): 'That is the largest budget any start of the engine plans in, since --max-memory only lowers it, and a start refuses before loading weights that exceed it with its reserves, so no load a start would hold is refused.' SCOPE DISCIPLINE (message, verbatim): 'It plans no streaming of its own: what streams decides which images exist, and weights.json must be the same on both release Macs, and in both builds of a BASELINE comparison, given the same WEIGHT_DIGESTS_ARGS.' -- and the carve-out that names the transient-shortfall sibling: 'Nor does it take the oracle's bound on available host memory: memory comes back slowly after a large process exits, and backend_regression runs weight-digests right after its benchmark rounds.' (the host-memory bound and its message stay the oracle's, unchanged). MEASURED (message, source-reported, M5 Max 36 GB): 'ISTA-DASLab Q2_0 is refused in under a second, before any image loads, against a 27729 MiB budget: at 63302 MiB without options, 38067 MiB with --expert-memory 7G and 37884 MiB with --ple-memory 2G, as before; at 37884 MiB, its count with the experts whole, with --expert-memory 32G or 64G and --ple-memory 2G, where it counted 38249 and 71017 MiB before the cap' -- and on loads the tool's JSON is byte-identical to the 1.4.0 tool's (56 and 57 images; the 27B Q4_K_M byte for byte, with and without the options), so the admission changes only the refusal, never the load. REGIME: batch/tool loads over a device working set whose ceiling a consumer planner already pins (hardBudgetBytes); any pre-flight admission whose count is derivable from metadata before the expensive work begins. ANTI-REGIMES: ceilings no authority pins (a guessed budget false-refuses -- the hard-budget bound is what makes the refusal sound); loads whose held bytes are unknowable before loading (the count needs file-derived sizing: streamableWeights); consumers with no lower knob (nothing guarantees the ceiling is the max any run plans in). KINSHIP/DISTINCT-FROM (read + disclosed): reject-plan-runs-before-setup-with-outliving-side-effects (entries_44, B271, same repo) -- the plan that can reject runs BEFORE the side-effectful setup; THIS is the TOOL-side admission (the tool has no side-effectful setup; the cost the pre-check elides is the doomed READ itself -- a doomed 63 GiB load read for minutes and then died) -- compose, and its monotone-re-derivation half is a different decision. admission-tests-the-bound-the-admitted-mode-serves (entries_45, B271, same repo) -- WHICH of two bounds an admission arm tests; THIS prices the tool admission on the CONSUMER'S OWN ceiling (hardBudgetBytes, the max any start plans) so no load the consumer holds is refused -- the same discipline (test the bound the consumer serves under), a different decision. startup-estimate-derived-from-allocation-shapes (entries_11, B196) -- the admission estimate derived from the allocator's OWN shapes with a fake-allocator reconciliation; THIS derives the tool's count from the PLANNER'S OWN store resolution (min(SIZE, part whole), 'as the planner takes a SIZE past the part') -- one resolution, two consumers, never a parallel estimate. The Batch-232 HEADROOM-ADMISSION rider on degrade-not-fail-load-qualification-probe (entries_21) -- load-time RESOURCE-HEADROOM admission of an optional copy (fitsLoadLimit, fail-closed on unknown RAM, degrade-loud); THIS is the whole-load DEVICE-budget REFUSAL with the remedy named -- kin class (fail-closed load-time resource admission), distinct axis (aux-copy admit/degrade vs whole-load refuse-with-remedy). checkpoint-reads-skip-page-cache-with-refusal-fallback (entries_37, B257) -- the LOAD path's I/O mechanics once it runs; THIS is the admission BEFORE any I/O. reclaim-progress-keyed-on-refused-resource-returned + refusal-wait-keyed-on-shortfall-returning-transfer (entries_41, B270) -- the refusal-RETRY/WAIT family after a refusal; THIS is the pre-load refusal whose remedy is an OPTION, not a wait -- and the commit's own host-memory carve-out is exactly their transient-shortfall regime (memory comes back slowly after a large process exits), the reason the device-budget check refuses loud while the host-memory check deliberately does not exist here. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'hard.?budget|load-admission|weightBytes|part.{0,10}whole|before.{0,15}first.{0,15}image' -- the hits are the rules named above (entries_44/45/21/37/41/11); zero owner of a files-sized pre-load refusal against the planner's hard budget with the part-whole store cap. FIXTURE PROVENANCE: original machinery (never splash code) -- a Sizing read from the files (per-image bytes, each store's part whole), LoadOptions (each store's option SIZE; a SIZE only decides WHICH images exist), the naive arm (uncapped-SIZE count + a no-fit-check run that reads image after image and dies mid-load with an opaque device error), and the law arm (a capped count, a hard-budget policy with reserves, and a pre-read Verdict that names both counts and the streaming options); the shared-contract tests (resident images follow the options, a size within its part names the load's bytes, a fitting no-stream load completes) pass in BOTH arms; the discriminating tests pin the doomed mid-load failure and the uncapped drift (buggy: images_read > 0 at the failure, no remedy named, count 20x past the load) against the pre-image refusal, the capped count, the reserve-band no-false-refusal sweep (every weight count a start would hold loads), and the real remedy (the named options turn Refuse into Load). Arms 5/5 + 7/7 standalone (rustc --edition 2021 --test, exit 0 both). Wave: Batch 274, Slice B (the splash 122B release-wave pool). Cite: incoai/splash @ 28d0e98ce4a98c88edeb562ddea2f9c38009fadd -- dev/tools/LoadOptions.hpp:57-68, dev/tools/weight_digests.mm:56-66, DEVELOPMENT.md:2322-2326 + :3592-3594, commit message (all quotes); Batch 274, Slice B.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
