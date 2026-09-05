//! Entries slice -- Batch 115 (6 rules) -- the qwen-3.8-mtp-challenge
//! submission-diff-history walk (Research 139 / Plan 088). Source:
//! Layr-Labs/qwen-3.8-mtp-challenge @ `0863b06ac16e26e48fc06e97444095b00feb66d4`
//! (Apache-2.0 harness; the ~50 Accept/Validate submission diffs 08-16..08-23 are
//! the mineable object -- B100 read the tip TREE, this batch read the diff
//! HISTORY; complementary). Geometry: Qwen 3.8 27B MLX 4-bit affine group-64,
//! 64L GDN+attention hybrid, hidden 5120, vocab 248320 -- the Bonsai-27B shape
//! class. Two read lanes (A prefill / B decode+draft-head+apparatus) + a
//! coordinator synthesis produced 6 rules + 8 riders after falsification; both
//! seed verdicts resolved (the activation digit-fold = confirmed B96 rider with
//! two-posture exactness content; the in-kernel input-side offset sum = already
//! covered by B96's bit-plane rider). Fixtures are original minimal Rust
//! reproductions (parse-only; no `--fix` path; no source text in the corpus).
//! Category membership lives ONLY in `KERNEL_CATEGORIES` (mod.rs) -- never
//! encode it here.
use crate::domains::kernel_opt::KernelRule;

/// Data table, not logic -- grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<KernelRule> {
    vec![

// ============================================================
// -- ordinal-map-exact-tail-selection -- Batch 115 / Research 139 / Plan 088.
        KernelRule {
            rule_id: "ordinal-map-exact-tail-selection".to_string(),
            gpu_concept: "Replace a reference library sort's tail-K with a selection kernel EXACTLY by order-encoding the reference's TOTAL order into a monotone integer key -- then any integer selection primitive reproduces the reference output set and order bit-for-bit. The map must encode every corner of the reference comparator's semantics, not just the value plane: stable ascending (ties break toward the HIGHER index, because the tail of an ascending stable sort keeps the last of each tie class), NaN ranking ABOVE every number (a LessThan-style reference comparator that returns (!an) & bn), and signed zero folded (negative zero maps to the same key class as positive zero so the pair ties and breaks by index). The canonical float-to-uint ordinal for an ascending-with-NaN-first reference: NaN maps to the all-ones key, plus-zero maps to the sign-bit-set identity, negative values map to their bitwise complement, positive values map to their sign-bit-flipped identity -- a monotone map on the key plane induces exactly the reference order. The verify contract has three legs, all load-bearing: (1) the replacement reads the SAME selected SET as the reference (assert both sides against a shared oracle -- 'the kernel is a function of the SET the reference chose, never of how it ordered that set'); (2) tie-forcing fixtures by construction (rows quantized hard enough to force heavy ties plus one all-equal row where the selection is entirely at the reference's discretion); (3) a positive control that swaps one selected index for one rejected index and must FAIL ('a gate that cannot fail is not a gate'). Two dispatch bonuses ride the same contract: the finalize stage emits ASCENDING so the result is element-wise identical to the reference slice, and fused address arithmetic into the selection epilogue is exact because an injective function applied element-wise to the tail preserves element-wise identity. Costs: the incumbent stub this replaces resolved a top-32 question with a full merge-sort argsort of 98,330 elements = 14 dependent dispatches + 5 device temporaries; the ordinal two-stage replacement answers it in 2. Distinct from packed-total-order-multi-field-argmax (B107 -- packs (score, id) into ONE key for a fresh argmax; this derives the key FROM an incumbent comparator's corner semantics so a DIFFERENT primitive reproduces it), from radix-select-ordered-key-topk (B107 -- the multi-pass selection MECHANISM this composes with), from order-stable-topk-via-index-sort (B110 -- the mechanics family), and from coarse-precision-first-exact-refine (B110 -- approximate-then-refine; the ordinal map makes the narrow key EXACT so no refine pass exists). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06 (Apache-2.0), qwen_top32_ordinal + the top32 partial/finalize pair (MLX argPartition stub 'we direct arg partition to sort for now' as the incumbent); Batch 115 / Research 139 / Plan 088.".to_string(),
            ref_shape: "// naive: answer a tail-K question with the library's full sort.
// (the vendored stub lowers arg-partition to a complete merge sort:
// every element is ordered so that 32 can be read.)
fn tail32_naive(scores: &[f32], k: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..scores.len()).collect();
    // NaN ranks above every number; ties keep the LAST index
    // (stable ascending sort); -0.0 == 0.0.
    order.sort_by(|&a, &b| {
        scores[b].partial_cmp(&scores[a]).unwrap_or(
            if scores[a].is_nan() { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater },
        )
    });
    order.truncate(k);
    order
}
".to_string(),
            opt_pattern: "// optimized: encode the reference's TOTAL order into a monotone u32
// key, then select by integer key -- bit-identical output, 2 dispatches.
fn ordinal(v: f32) -> u32 {
    // NaN above all; BOTH zeros fold into one key class (ties break by
    // index); negatives complement, positives sign-flip: monotone on the
    // key plane, inducing exactly the reference comparator's order.
    if v.is_nan() { return u32::MAX; }
    if v == 0.0 { return 0x8000_0000; }
    let u = v.to_bits();
    if u & 0x8000_0000 != 0 { !u } else { u | 0x8000_0000 }
}

fn tail32_exact(scores: &[f32], k: usize) -> Vec<usize> {
    // stage 1: per-block local top-k of (ordinal, index) pairs --
    // keep the k LARGEST keys of each block.
    let mut candidates: Vec<(u32, usize)> = scores
        .chunks(1024)
        .map(|blk| {
            let mut local: Vec<(u32, usize)> =
                blk.iter().enumerate().map(|(i, s)| (ordinal(*s), i)).collect();
            local.sort_unstable();
            let start = local.len().saturating_sub(k);
            local.split_off(start)
        })
        .flatten()
        .collect();
    // stage 2: one final reduce; the tail-K is the LAST k of the
    // (key, index)-ascending order -- ties resolve toward the higher
    // index and the output is ASCENDING, element-wise identical to the
    // reference sort's tail slice.
    candidates.sort_unstable();
    let start = candidates.len().saturating_sub(k);
    candidates[start..].iter().map(|(_, i)| *i).collect()
}
".to_string(),
        },

// ============================================================
// -- complete-coverage-scatter-collapses-operand-to-dense -- Batch 115.
        KernelRule {
            rule_id: "complete-coverage-scatter-collapses-operand-to-dense".to_string(),
            gpu_concept: "When exact 'island' rows overwrite a quantized operand's rows AFTER the packed compute (a put-scatter applying corrected values), ROW COVERAGE decides the whole path: if the island index set is a COMPLETE permutation of the output range, every quantized value the packed compute produced is overwritten before any consumer reads it -- the packed compute is DEAD WORK and the path collapses to one dense matmul over the exact rows. Detect completeness on the host ONCE at install; when the island rows are a permutation, fold the scatter away entirely by reordering the island rows into output order at install time (argSort of a permutation is its inverse -- a one-time take, zero runtime scatter), and serve a plain dense matmul split at the island boundary. PARTIAL coverage keeps the packed-plus-scatter path (the only correct form: un-covered rows must still be computed), and the per-arm allocation rule applies: a partial arm allocates only the tensors it installs, so an uninstalled island never occupies resident memory. Two contract subtleties are load-bearing: (1) whether the REPLACED packed computation would even have run can be UNKNOWABLE at install time (the installer wires islands before the quantizer wires the projections) -- resolve that eligibility lazily ONCE on first use, mirroring the replaced computation's own gating conditions, and cache the resolution; (2) the same coverage lens bounds PRUNING in the other direction -- a candidate-set trim must cover the COMMITTED support (the argmax ids actually observed and committed), because a measured halving that trimmed three committed ids REGRESSED acceptance 1.00 to 0.877 while saving ~0.6 ms of per-step read (the reject path prices several times the read saving). So: coverage governs BOTH elimination (complete coverage of overwritten rows = dead compute) and retention (a prune below committed support = measured regression). Distinct from superseded-format-constructor-live-fields-only (B56 -- dead UPLOAD at construction when a second format supersedes; this is dead COMPUTE from runtime overwrite coverage), from host-cache-immutable-device-weights (B26 -- immutable weight residency; the island rows here are exact patches, not the whole operand), and from lm-head-loss-positions-only (B37 -- positional dead-compute by support-window analysis; this is row-coverage by overwrite analysis). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06 (Apache-2.0), the BF16 precision-island consumption path (isCompletePermutation + the argSort-inverse install reorder) and the measured 49,152-prefix-trim regression; Batch 115 / Research 139 / Plan 088.".to_string(),
            ref_shape: "// naive: compute the full packed operand, then scatter exact
// rows over it -- including the case where the scatter covers
// EVERY row, making the packed compute dead work.
fn forward_naive(packed: &PackedOperand, islands: &Islands, x: &[f32]) -> Vec<f32> {
    // full quantized compute over every row
    let mut out = packed.quantized_matvec(x);
    // then overwrite island rows with exact values
    for (row, exact) in islands.iter() {
        let acc = row * out.row_stride();
        for (c, v) in exact.iter().enumerate() {
            out[acc + c] = *v;
        }
    }
    out.to_vec()
}
".to_string(),
            opt_pattern: "// optimized: install-time coverage detection. A complete
// permutation means the packed rows are never read -> dense path
// with island rows pre-reordered (argSort of a permutation is its
// inverse). Partial coverage keeps the scatter.
enum IslandPlan {
    Complete { dense: DenseOperand },   // islands reordered to output order
    Partial { packed: PackedOperand, scatter: Vec<(usize, Vec<f32>)> },
}

fn install(packed: PackedOperand, islands: Vec<(usize, Vec<f32>)>) -> IslandPlan {
    let n = packed.rows();
    let mut idx: Vec<usize> = islands.iter().map(|(r, _)| *r).collect();
    idx.sort_unstable();
    idx.dedup();
    if idx.len() == n && idx.iter().enumerate().all(|(i, r)| *r == i) {
        // complete permutation: reorder island rows to natural order once,
        // drop the packed operand entirely (never uploaded).
        let mut rows = vec![Vec::new(); n];
        for (r, exact) in islands { rows[r] = exact; }
        IslandPlan::Complete { dense: DenseOperand::from_rows(rows) }
    } else {
        IslandPlan::Partial { packed, scatter: islands }
    }
}

fn forward(plan: &IslandPlan, x: &[f32]) -> Vec<f32> {
    match plan {
        IslandPlan::Complete { dense } => dense.matvec(x),
        IslandPlan::Partial { packed, scatter } => {
            let mut out = packed.quantized_matvec(x);
            for (row, exact) in scatter {
                let acc = row * out.row_stride();
                for (c, v) in exact.iter().enumerate() { out[acc + c] = *v; }
            }
            out
        }
    }
}
".to_string(),
        },

// ============================================================
// -- exhaustive-domain-input-patch-bit-identity -- Batch 115.
        KernelRule {
            rule_id: "exhaustive-domain-input-patch-bit-identity".to_string(),
            gpu_concept: "When a fused kernel's transcendental must be bit-identical to a reference implementation and the INPUT DOMAIN is exhaustively sweepable (16-bit inputs: 2^16 words total, 65,280 finite values -- an offline enumeration is cheap), do NOT tune the expression toward tolerance: enumerate the WHOLE domain offline against the reference, find the divergent inputs (typically 1-3 of 65,280), and patch exactly those inputs via a direct output-word mapping applied before the lossless widening expansion -- every other input retains the inherited expression unchanged. The patch is bit-level: map the input's raw bit pattern to the reference's raw output word, so the fused single-launch form is bit-exact over the entire finite domain and the fusion survives without a tolerance gate. f32 domains are NOT sweepable (2^32) -- fall back to tolerance gates there; the sweepability boundary is the rule's precondition. Two mandatory gate legs: (1) an exhaustive post-patch comparison (every finite input, zero mismatches -- 65,280-point oracle, not a sample); (2) a boundary negative control (perturb the geometry -- e.g. one extra window row -- and require EXACTLY the outputs reading that geometry to fail, proving the gate is live rather than vacuously green). Validation scale precedent: 12.98M element comparisons zero-mismatch when the sweep feeds a full forward equivalence. Distinct from half-domain-exact-band-dequant-with-raw-bit-oracle (B97 -- an ALGEBRAIC band condition proving a narrowed-arithmetic form exact on a band, with the exhaustive oracle as the PROMOTION gate; this patches the residual divergences of an otherwise-correct fused expression), from fused-vector-site-transcendental-form-nonportable (B98 -- transcendental form choice is per-site and must be re-validated; this says WHAT to do when that validation finds exactly a handful of divergent inputs on a sweepable domain), and from sign-branch-stable-sigmoid (B32a -- forward overflow hygiene by algebraic argument; orthogonal). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06 (Apache-2.0), the bf16 sigmoid 1-ulp divergence found by the all-65,280-input sweep (input 0xC0DB mapped to the reference's output word 0x3A8B before the fp32 expansion); Batch 115 / Research 139 / Plan 088.".to_string(),
            ref_shape: "// naive: a fused kernel's hand-written activation diverges from the
// reference by 1 ulp somewhere; shipped under a tolerance gate.
fn fused_gate_tol(x_bits: u16) -> f32 {
    let x = f16_from_bits(x_bits);
    let s = sigmoid_f32(x);          // diverges 1 ulp vs reference
    let e = widen_lossless(s);
    e * gate_from(x)
}

#[test]
fn matches_reference_within_tolerance() {
    for w in 0..u16::MAX {
        let got = fused_gate_tol(w);
        let want = reference(w);
        assert!((got - want).abs() < 1e-2, \"sample {w}\");
    }
}
".to_string(),
            opt_pattern: "// optimized: enumerate the whole 16-bit domain offline, patch ONLY the
// divergent inputs at the bit level, keep the inherited expression
// everywhere else; exhaustive zero-mismatch gate + boundary negative.
fn fused_gate_exact(x_bits: u16) -> f32 {
    // 3 of 65,280 finite inputs diverged; map them to the reference's
    // raw output words before the lossless expansion.
    if let Some(patched) = patch_table(x_bits) {
        return f32_from_bits(patched);
    }
    let x = f16_from_bits(x_bits);
    let e = widen_lossless(sigmoid_f32(x));
    e * gate_from(x)
}

// generated offline by the sweep; committed as data.
fn patch_table(x_bits: u16) -> Option<u32> {
    match x_bits {
        0xC0DB => Some(0x3A8B),
        _ => None,
    }
}

#[test]
fn exhaustive_zero_mismatch_vs_reference() {
    for w in 0..=u16::MAX {
        assert_eq!(fused_gate_exact(w).to_bits(), reference(w).to_bits());
    }
}

#[test]
fn boundary_perturbation_fails_exactly_the_window_consumers() {
    // +1 window row: exactly the outputs reading that row must diverge.
    assert!(window_consumers_diverge_by(1) == EXPECTED_CONSUMER_COUNT);
}
".to_string(),
        },

// ============================================================
// -- derived-index-from-pinned-checkpoint-is-a-cache -- Batch 115.
        KernelRule {
            rule_id: "derived-index-from-pinned-checkpoint-is-a-cache".to_string(),
            gpu_concept: "A retrieval index derived at load/warm time from the PINNED CHECKPOINT ALONE -- no file, no prompt, no request state -- is cache-class, not parameter-class: it is a fixed function of the checkpoint (the same class as a dequantized weight cache or a shape table) and computing it inside the untimed warm is legal wherever recomputing it from raw weights would be. The rule is the contract that keeps it cache-class, four clauses: (1) LEGALITY BOUNDARY -- the derivation reads only pinned tensors; the moment it reads request state or a mutable file it is a parameter and needs governance; (2) CANONICAL ORDER -- the derivation must produce a canonical ordering (the same order a stable argsort of the assignment produces) so a runtime-derived table and an offline-screened table are comparable and interchangeable; (3) FAIL-LOUD OVER SILENT FALLBACK -- a derived index that disagrees with its declared shape must abort, not silently fall back to a path that would report a plausible time for the WRONG mechanism (a silent fallback corrupts the measurement while returning valid numbers); (4) ONE ATTEMPT ONLY -- a checkpoint that cannot support derivation keeps its dense readout and never re-attempts per call (a per-call re-derivation attempt is a per-call cost disguised as a feature). Derivation-side discipline that makes the index deterministic and batch-shaped: balanced bisection (a node with target L splits into ceil(L/2) and floor(L/2) so empty clusters are impossible and no separate rebalance pass exists), furthest-point init with no RNG, squared distances formed as norm(x)^2 - 2 x.c + norm(c)^2 so no [m, s, D] difference tensor materializes, and nodes sharing a row count dispatched as ONE batched call. Distinct from host-cache-immutable-device-weights (B26 -- caching the IMMUTABLE UPLOADED weights themselves; this derives a NEW retrieval structure from them), from derive-shape-from-payload-not-cached-field (B32b -- shape truth from payload at DECODE time; this is a derived search structure at LOAD time), and from derived-key-single-constructor-parity-test (B106 -- the KEY-derivation topology must have one constructor both sides call; this is the cache-vs-parameter legality boundary for derived indexes). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06 (Apache-2.0), the two-level cluster index derived from the pinned head tensors at the untimed warm (12,292 leaves of 8 rows, leaf-mean centroids, permutation-mapped padding rows); Batch 115 / Research 139 / Plan 088.".to_string(),
            ref_shape: "// naive: a derived retrieval index rebuilt on the hot path and
// silently abandoned when its shape disagrees.
fn candidate_ids(&mut self, x: &[f32]) -> Vec<u32> {
    if self.index.is_none() {
        // re-derives EVERY draft step; and if the leaf shapes disagree
        // with the header it quietly returns the dense scan instead --
        // a plausible time for the wrong mechanism.
        match DerivedIndex::build(&self.pinned) {
            Ok(ix) if ix.rows() == self.expected_rows() => self.index = Some(ix),
            _ => return self.dense_scan(x),
        }
    }
    self.index.as_ref().unwrap().shortlist(x)
}
".to_string(),
            opt_pattern: "// optimized: derive once in the warm from pinned tensors alone,
// canonical order, fail loud on disagreement, one attempt forever.
struct DerivedIndexState {
    index: Option<DerivedIndex>,
    attempted: bool,
}

impl DerivedIndexState {
    fn warm(&mut self, pinned: &PinnedTensors) {
        if self.attempted { return; }
        self.attempted = true;
        match DerivedIndex::build(pinned) {
            Ok(ix) => {
                assert_eq!(ix.rows(), pinned.declared_rows(),
                    \"derived index shape disagrees with the checkpoint header\");
                self.index = Some(ix);
            }
            Err(e) => panic!(\"derived index unusable, refusing silent fallback: {e}\"),
        }
    }

    fn candidate_ids(&self, x: &[f32]) -> Vec<u32> {
        match &self.index {
            Some(ix) => ix.shortlist(x),
            // one attempt only: the dense readout is the permanent shape.
            None => Self::dense_scan_static(x),
        }
    }
}
".to_string(),
        },

// ============================================================
// -- warm-reachable-dispatch-signatures-before-timed-window -- Batch 115.
        KernelRule {
            rule_id: "warm-reachable-dispatch-signatures-before-timed-window".to_string(),
            gpu_concept: "JIT-specializing backends compile a kernel variant on the FIRST dispatch of each unseen signature (shape, dtype, input count, host/device mix all select variants), and an ADAPTIVE schedule reaches a SET of signatures -- so warm the REACHABLE SET, not the one path arm 0 takes: enumerate every signature the schedule can dispatch (per-width tiers, chunk-split arms, cache-window depths, extra-input counts 0..maxDepth), fabricate minimal throwaway state at each signature (values are irrelevant; SIGNATURES are not -- zeros and already-eval'd dummies work), and dispatch the exact expression scored rounds will dispatch at each one before the timed window opens. Three refinements are load-bearing: (1) warm the EXPRESSION, not a call-chain prefix -- warming only the wrapper leaves the published post-processing (the normed-output variant, the concatenated multi-input form) to cold-compile inside the first timed round; (2) EXTEND the warm set when the schedule's reachable set changes -- an adaptive policy that can return 0..8 draft depths must have every legal extra-input count warmed, and a chunk splitter that divides a wide verify into arm A at depth 5 plus arm B at depths 2..4 must warm arm B's depths too, each extension carrying its measured receipt; (3) warm at the GEOMETRY the run reaches, via fabricated throwaway caches -- after a short seed the decode walk reaches window depths the seed never compiled, so host-extend throwaway K/V to the reached depth and dispatch the fused shapes there (token-neutral: dummy state never enters a scored forward). Failure mode is silent: a cold first-touch inside the timed window reads as a slower first interval that per-slot receipts then mis-attribute. Distinct from shape-gated-warm-start-slot-copy (B32c -- warming slot CONTENT by shape tuple for correctness of the copy; this warms COMPILED VARIANTS of the dispatch set for timing validity), from hot-swap-weights-preserve-autotune-caches (B25 -- preserving caches across weight swaps; this enumerates the variant set ahead of the window), and from dead-result-timing-loop-folded-under-lto (B109 -- the measurement-honesty sink; this is the sibling cold-JIT leak that mis-attributes intervals). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06 (Apache-2.0), the warm-set family (warm the same-expression; the concat-signature warm with throwaway values across every legal extra-count; the qL{1..5} extension with receipt; the later-window SDPA warm at token-neutral fabricated caches); Batch 115 / Research 139 / Plan 088.".to_string(),
            ref_shape: "// naive: warm arm 0's exact path; every other reachable
// variant cold-compiles inside the timed window.
fn warm_for_bench(model: &Model) {
    // one expression, one depth, one input count
    let _ = model.call_with_hidden(&dummy(512));
    // the schedule can emit widths 1..=5, chunk splits A+B,
    // window depths to 1024, and 0..8 extra inputs -- none warmed.
}
".to_string(),
            opt_pattern: "// optimized: enumerate the reachable signature set, fabricate
// throwaway state per signature, dispatch the scored expression
// at every one -- untimed, token-neutral.
fn warm_for_bench(model: &Model) {
    // (1) the scored expression INCLUDING post-processing
    let _ = model.call_with_hidden_and_normed(&dummy(512));
    // (2) every legal extra-input count the adaptive schedule may pass
    for extra in 0..=8 {
        let ids = dummy_ids(extra);
        let _ = model.verify_concat(&dummy(512), &ids); // zeros; result discarded
    }
    // (3) every chunk-split arm: A at depth 5, B at depths 2..=4
    for ql in 2..=5 {
        let _ = model.verify_chunk(&dummy(512), ql);
    }
    // (4) the reached cache-window depth, fabricated token-neutral
    let mut throwaway = model.fresh_fa_caches();
    throwaway.host_extend_to(1024);
    let _ = model.fused_sdpa(&dummy(512), &throwaway);
}
".to_string(),
        },

// ============================================================
// -- env-gate-reaches-and-arms-the-scored-process -- Batch 115.
        KernelRule {
            rule_id: "env-gate-reaches-and-arms-the-scored-process".to_string(),
            gpu_concept: "An environment-gated A/B or promoted default is VACUOUS unless the gate survives four plumbing hazards, each of which silently measures the same code twice or never arms the promoted path: (1) SANITIZER ALLOWLIST -- a hardened runtime forwards only allowlisted prefixes to the scored worker (e.g. MLX_, METAL_, MTL_ style members); a gate spelled outside the allowlist never reaches the process that runs the scored round, so both arms of the A/B run the shipped default -- spell the gate within the allowlist and verify the name is witnessable in the binary (a fixed-length name is grep-able via strings); (2) AMBIENT-VALUE DEFEAT -- a library knob cached on FIRST USE cannot be injected with no-overwrite semantics, because the ambient stock default is usually already exported in the environment: an injected per-profile default must FORCE-SET (overwrite), while no-overwrite remains reserved for genuine user opt-ins and kill switches -- state the precedence in code ('command-buffer budgets are per-profile absolutes; only the opt-in flags use no-overwrite semantics'); (3) ARM WITNESS -- record the RESOLVED arm into the durable artifact (an append-only trace file), never stderr alone: a parent that drains the worker's stderr through a swallowing emitter discards every line of a successful leg, so an unwitnessed arm is unverifiable after the fact; (4) BARE-LEG RECEIPT -- prove a default-on claim with a no-environment leg that reads the default path and records the read (a leg that exports nothing must resolve to the default arm with its counter at zero -- the proof the fused/optimized kernels are the compiled default rather than an opt-in). The failure in every case is invisible: the run completes, the numbers look plausible, and the comparison compared nothing. Distinct from env-flag-resolved-once-not-per-dispatch (B38 -- resolve the flag ONCE off the dispatch path; this is whether the flag REACHES and ARMS the scored process at all), from dead-result-timing-loop-folded-under-lto (B109 -- the folded-work fake measurement; this is the config-plumbing fake-A/B twin, the second rule of the measurement-honesty family), and from derived-key-single-constructor-parity-test (B106 -- one derivation constructor both sides call; this is one ALLOWLISTED NAME both the exporter and the consumer spell). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06 (Apache-2.0), the sanitized-worker-environment allowlist (a legacy kill switch 'has never had any effect on a worker leg'), the deleted E85 gate whose doc recorded the lesson, and the force-set fix for the cached-on-first-use command-buffer budgets; Batch 115 / Research 139 / Plan 088.".to_string(),
            ref_shape: "// naive: an A/B gated by a var the scored process never sees,
// injected no-overwrite into a first-use-cached knob.
fn main() {
    // ambient stock default is ALREADY exported upstream:
    // OVERWRITE=0 keeps it; the promoted budget never lands.
    std::env::set_var(\"MAX_BUDGET_MB\", opts.budget.to_string());
    // (set_var cannot express no-overwrite; the reference runtime
    // used overwrite=0 and lost silently.)

    let arm = std::env::var(\"MLXFAST_MY_TOGGLE\"); // dropped by the
    // worker's sanitizer allowlist: never reaches the scored leg.
    let use_fast = arm.as_deref() != Ok(\"0\");
    run_scored_round(use_fast);
    // stderr logging only -- the parent swallows it on success.
    eprintln!(\"arm={:?}\", arm);
}
".to_string(),
            opt_pattern: "// optimized: allowlisted name + force-set default + durable arm
// witness + bare-leg receipt.
const GATE: &str = \"MLX_MY_TOGGLE\"; // inside the sanitizer allowlist

fn resolve_arm() -> Arm {
    match std::env::var(GATE).as_deref() {
        Ok(\"0\") => Arm::Baseline,
        Ok(_) | Err(_) => Arm::Fast, // default-on
    }
}

fn main() {
    // injected per-profile default FORCE-SETS over the ambient stock;
    // no-overwrite is reserved for user opt-ins (checked before this).
    std::env::set_var(\"MAX_BUDGET_MB\", opts.budget.to_string());

    let arm = resolve_arm();
    run_scored_round(arm);

    // durable witness, not stderr: the parent discards worker stderr.
    witness_append(\"arm\", &arm.to_string());

    // bare-leg receipt: a leg with no env must read the default arm.
    if !env_exported(GATE) {
        assert_eq!(arm, Arm::Fast, \"default-on claim failed its bare leg\");
    }
}
".to_string(),
        },
    ]
}
