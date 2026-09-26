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
            gpu_concept: "When a fused kernel's transcendental must be bit-identical to a reference implementation and the INPUT DOMAIN is exhaustively sweepable (16-bit inputs: 2^16 words total, 65,280 finite values -- an offline enumeration is cheap), do NOT tune the expression toward tolerance: enumerate the WHOLE domain offline against the reference, find the divergent inputs (typically 1-3 of 65,280), and patch exactly those inputs via a direct output-word mapping applied before the lossless widening expansion -- every other input retains the inherited expression unchanged. The patch is bit-level: map the input's raw bit pattern to the reference's raw output word, so the fused single-launch form is bit-exact over the entire finite domain and the fusion survives without a tolerance gate. f32 domains are NOT sweepable (2^32) -- fall back to tolerance gates there; the sweepability boundary is the rule's precondition. Two mandatory gate legs: (1) an exhaustive post-patch comparison (every finite input, zero mismatches -- 65,280-point oracle, not a sample); (2) a boundary negative control (perturb the geometry -- e.g. one extra window row -- and require EXACTLY the outputs reading that geometry to fail, proving the gate is live rather than vacuously green). Validation scale precedent: 12.98M element comparisons zero-mismatch when the sweep feeds a full forward equivalence. Distinct from half-domain-exact-band-dequant-with-raw-bit-oracle (B97 -- an ALGEBRAIC band condition proving a narrowed-arithmetic form exact on a band, with the exhaustive oracle as the PROMOTION gate; this patches the residual divergences of an otherwise-correct fused expression), from fused-vector-site-transcendental-form-nonportable (B98 -- transcendental form choice is per-site and must be re-validated; this says WHAT to do when that validation finds exactly a handful of divergent inputs on a sweepable domain), and from sign-branch-stable-sigmoid (B32a -- forward overflow hygiene by algebraic argument; orthogonal). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06 (Apache-2.0), the bf16 sigmoid 1-ulp divergence found by the all-65,280-input sweep (input 0xC0DB mapped to the reference's output word 0x3A8B before the fp32 expansion); Batch 115 / Research 139 / Plan 088. ADDENDUM RIDER (Batch 115 collision salvage -- the per-OUTPUT exclusion half this rule's closure presupposes): before any sweep can license a fusion, the ADMISSION decision is per-output and fidelity-driven -- an output feeding an ABSOLUTE-FIDELITY consumer (a verify recurrence, a commitment) stays EXCLUDED from the fused kernel while any op on its path lacks a bit-exactness proof, even at a priced dispatch cost: 'beta feeds the VERIFY recurrence where fidelity is absolute, so it stays a plain graph sigmoid(b) -- one [1,S,48] elementwise launch, ~0.06 ms of the ~0.7 ms saving, in exchange for removing an unquantifiable knife-edge' (diff e3b4531d9:27). The exclusion is the conservative state; the exhaustive sweep + singleton patch is what LIFTS it per-output, and until the sweep exists the priced launch is the correct answer, not a tolerance gate.".to_string(),
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
            gpu_concept: "JIT-specializing backends compile a kernel variant on the FIRST dispatch of each unseen signature (shape, dtype, input count, host/device mix all select variants), and an ADAPTIVE schedule reaches a SET of signatures -- so warm the REACHABLE SET, not the one path arm 0 takes: enumerate every signature the schedule can dispatch (per-width tiers, chunk-split arms, cache-window depths, extra-input counts 0..maxDepth), fabricate minimal throwaway state at each signature (values are irrelevant; SIGNATURES are not -- zeros and already-eval'd dummies work), and dispatch the exact expression scored rounds will dispatch at each one before the timed window opens. Three refinements are load-bearing: (1) warm the EXPRESSION, not a call-chain prefix -- warming only the wrapper leaves the published post-processing (the normed-output variant, the concatenated multi-input form) to cold-compile inside the first timed round; (2) EXTEND the warm set when the schedule's reachable set changes -- an adaptive policy that can return 0..8 draft depths must have every legal extra-input count warmed, and a chunk splitter that divides a wide verify into arm A at depth 5 plus arm B at depths 2..4 must warm arm B's depths too, each extension carrying its measured receipt; (3) warm at the GEOMETRY the run reaches, via fabricated throwaway caches -- after a short seed the decode walk reaches window depths the seed never compiled, so host-extend throwaway K/V to the reached depth and dispatch the fused shapes there (token-neutral: dummy state never enters a scored forward). Failure mode is silent: a cold first-touch inside the timed window reads as a slower first interval that per-slot receipts then mis-attribute. Distinct from shape-gated-warm-start-slot-copy (B32c -- warming slot CONTENT by shape tuple for correctness of the copy; this warms COMPILED VARIANTS of the dispatch set for timing validity), from hot-swap-weights-preserve-autotune-caches (B25 -- preserving caches across weight swaps; this enumerates the variant set ahead of the window), and from dead-result-timing-loop-folded-under-lto (B109 -- the measurement-honesty sink; this is the sibling cold-JIT leak that mis-attributes intervals). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06 (Apache-2.0), the warm-set family (warm the same-expression; the concat-signature warm with throwaway values across every legal extra-count; the qL{1..5} extension with receipt; the later-window SDPA warm at token-neutral fabricated caches); Batch 115 / Research 139 / Plan 088. RIDER (Batch 181, mlxfast-bonsai2 @ cfb35b0d -- AttentionV1.swift warmVerifyBlock, the ALIGNMENT-CLASS instance 2): warm the verify block's composition once per key count on throwaway zeros -- the two FP32 matmuls pick their steel GEMM pipeline by whether the key count fills whole tiles (align_N for the scores, align_K for the output), so 'a window whose key count lands in an alignment class the warm verify did not see built that pipeline inside a timed round'; the signature axis here is the PIPELINE-VARIANT axis (align_N/align_K), instance 2 of this rule.".to_string(),
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
            gpu_concept: "An environment-gated A/B or promoted default is VACUOUS unless the gate survives four plumbing hazards, each of which silently measures the same code twice or never arms the promoted path: (1) SANITIZER ALLOWLIST -- a hardened runtime forwards only allowlisted prefixes to the scored worker (e.g. MLX_, METAL_, MTL_ style members); a gate spelled outside the allowlist never reaches the process that runs the scored round, so both arms of the A/B run the shipped default -- spell the gate within the allowlist and verify the name is witnessable in the binary (a fixed-length name is grep-able via strings); (2) AMBIENT-VALUE DEFEAT -- a library knob cached on FIRST USE cannot be injected with no-overwrite semantics, because the ambient stock default is usually already exported in the environment: an injected per-profile default must FORCE-SET (overwrite), while no-overwrite remains reserved for genuine user opt-ins and kill switches -- state the precedence in code ('command-buffer budgets are per-profile absolutes; only the opt-in flags use no-overwrite semantics'); (3) ARM WITNESS -- record the RESOLVED arm into the durable artifact (an append-only trace file), never stderr alone: a parent that drains the worker's stderr through a swallowing emitter discards every line of a successful leg, so an unwitnessed arm is unverifiable after the fact; (4) BARE-LEG RECEIPT -- prove a default-on claim with a no-environment leg that reads the default path and records the read (a leg that exports nothing must resolve to the default arm with its counter at zero -- the proof the fused/optimized kernels are the compiled default rather than an opt-in). The failure in every case is invisible: the run completes, the numbers look plausible, and the comparison compared nothing. Distinct from env-flag-resolved-once-not-per-dispatch (B38 -- resolve the flag ONCE off the dispatch path; this is whether the flag REACHES and ARMS the scored process at all), from dead-result-timing-loop-folded-under-lto (B109 -- the folded-work fake measurement; this is the config-plumbing fake-A/B twin, the second rule of the measurement-honesty family), and from derived-key-single-constructor-parity-test (B106 -- one derivation constructor both sides call; this is one ALLOWLISTED NAME both the exporter and the consumer spell). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06 (Apache-2.0), the sanitized-worker-environment allowlist (a legacy kill switch 'has never had any effect on a worker leg'), the deleted E85 gate whose doc recorded the lesson, and the force-set fix for the cached-on-first-use command-buffer budgets; Batch 115 / Research 139 / Plan 088. RIDER (Batch 170, mlxfast-gemma4 @ 27c821c4 -- DenseMLPQMVV1.swift L59-90, the GATEUP flip): the INSTRUMENT arm of the measurement-honesty family -- a static width-1 replay tape 'compares the wrong contexts after the first legitimate divergence': under the frozen cohort_reference_replay predicate (the reference teacher-forced on the CANDIDATE's own committed journal) the tree WITH THE ARM OFF already diverges at row 1, so the flip's own 'zero mismatches' belief was an artefact of the instrument, not a property of the arm ('every mismatch the arm adds is a near-tie; the non-near-tie count is 2 on both sides' -- the budget-table form of the lossy-surface per-family retention rule, an external production confirmation). Composes with this rule's arm-witness clause: witness the arm through the CANDIDATE's journal, never a fixed tape".to_string(),
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

// ============================================================
// -- Batch 115 ADDENDUM (collision salvage) -- the sibling Batch 115
// (11863f8) landed first with 6 different rules; per the push-wins
// tie-breaker (the B103-addendum precedent) the duplicate yielded and these
// 4 rules land as the salvage the landed batch genuinely lacks. Quotes were
// coordinator-verified at the pin BEFORE the collision (the re-verification
// corrected the lane-C radix misattribution now codified in the sibling's
// B107 rider).
// ============================================================
// -- dispatch-cliff-bitexact-shape-confinement -- Batch 115 addendum (collision salvage) / Research 139 / Plan 088.
        KernelRule {
            rule_id: "dispatch-cliff-bitexact-shape-confinement".to_string(),
            gpu_concept: "A batched multi-row op with a kernel-family dispatch threshold must keep every dispatch INSIDE the bit-proven width range of one family: crossing the threshold changes the kernel family AND the accumulation order of every score/output element, so a consumer reading VALUES (top-2 evidence, tolerance gates, rank-sensitive orderings) diverges even when argmax ids stay stable. The source's own diagnosis, verbatim: 'fused sdpa vector path serves qL * gqa <= 32; above it the dispatch changes kernel family and the accumulation order of every score -- the measured source of the MTP width wall's top-2 VALUE drift.' The fix is NOT diagnosing the culprit op: remove the wide shapes -- split the batched verify into sub-chunks confined to the proven width range, each provably window-identical to a small round (query rows split with bottom-right causal alignment so every row sees the window a small round would; keys re-sliced, never recomputed; ONE cache update over read-only views; the recurrence twin: split [S-4, 4] with the fp32 state chained across the boundary, conv windows taken as slices of the SAME concatenated input). Load-bearing contract: the boundary constant is the SINGLE SOURCE OF TRUTH shared by forward AND replay -- 'Replay MUST mirror the same boundary ... which is why this is the one shared source of truth for it' -- a replay that re-runs rows as one call re-introduces the wide shape and diverges from the forward's own chaining. Only after the crack is closed by construction is the width cap raisable ('the wall is cracked structurally, by removing the wide shapes rather than diagnosing the culprit op'). Distinct from tile-loop-back-edge-needs-multi-tile-bitexact-gate (B57 -- the GATE half: a fixture must CROSS the boundary to test it; this is the production fix the gate protects), from reduction-restructure-needs-source-owned-comparator (B95 -- the ORACLE half; composes: the confined chunks are what the comparator pins), from shape-class-keyed-capability (B103 -- decides whether the fast path MAY run per shape class; this preserves bit-exactness when it DOES, the same threshold read from the numerics side), from carried-state-reduction-stays-scalar (B113 -- decides which products may reassociate at all; this avoids reassociation by never crossing the family boundary), and from batch-invariance-locks-reduction-geometry (B110 -- locks the geometry against BATCH COMPOSITION for cross-run reproducibility; this locks the shape against the KERNEL-FAMILY threshold for value stability within one binary). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06ac -- Vendor/mlx-swift-lm/Libraries/MLXLMCommon/AttentionUtils.swift:105-108 (the threshold comment, verbatim), the sdpa split with byte-identical windows (diff b6ce964b1, comment survives at tip), the GDN width-wall boundary 'widths 2...5 are bit-exact, drift starts at the 6th row' + the shared verifyChunkBoundary (diff 08897af24, at tip). Batch 115 addendum (collision salvage) / Research 139 / Plan 088.".to_string(),
            ref_shape: "// naive: a batched verify runs ALL query rows through whichever kernel the
// row count selects -- the wide family switches in above the fused vector
// path's width gate, and its accumulation order differs from the small
// family's. Argmax ids survive; top-2 VALUE evidence drifts.
const GQA: usize = 4;
const WIDE_GATE: usize = 32;

fn verify_rows_naive(q: &[f32], k: &[f32], n_q: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(n_q * 2);
    if n_q * GQA <= WIDE_GATE {
        small_family(q, k, n_q, &mut out);
    } else {
        wide_family(q, k, n_q, &mut out); // different accumulation order
    }
    out
}

fn small_family(_q: &[f32], _k: &[f32], _n: usize, _out: &mut Vec<f32>) {}
fn wide_family(_q: &[f32], _k: &[f32], _n: usize, _out: &mut Vec<f32>) {}
".to_string(),
            opt_pattern: "// optimized: confine every dispatch to the bit-proven width range; when the
// batch exceeds it, split so each piece is inside the proven range, and make
// the boundary constant the single source of truth for forward AND replay.
const GQA: usize = 4;
const PROVEN_MAX_ROWS: usize = 8; // bit-proven: qL * gqa <= 32
const DIM: usize = 128;

fn verify_rows_confined(q: &[f32], k: &[f32], n_q: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(n_q * 2);
    let mut row = 0;
    while row < n_q {
        let take = PROVEN_MAX_ROWS.min(n_q - row);
        // keys re-sliced, never recomputed; one cache update per call site
        small_family(&q[row * DIM..], k, take, &mut out);
        row += take;
    }
    out
}

fn small_family(_q: &[f32], _k: &[f32], _n: usize, _out: &mut Vec<f32>) {}
".to_string(),
        },

// ============================================================
// -- readout-trim-needs-committed-id-coverage -- Batch 115 addendum (collision salvage) / Research 139 / Plan 088.
        KernelRule {
            rule_id: "readout-trim-needs-committed-id-coverage".to_string(),
            gpu_concept: "Any readout/vocabulary subspace trim (compact draft head, prefix-only projection) is valid only if every id the consumer can actually COMMIT lies inside the retained slice -- verified against the live committed-argmax distribution of the promotion gate, never against the top-k of any single eval prompt. Ids outside the slice become unproposable; every occurrence forces a reject whose round-base cost exceeds the trimmed read savings. The measured negative is encoded at the tip as the guard's own evidence, verbatim: 'Prefix 98_304, the promoted trim. A 49_152 halving was measured on the public longcopy gate and REGRESSED: three of its committed argmax ids live in [49_152, 248_044), the head could no longer propose them, and the forced rejects cost more round-bases than the halved compact-head read saved (accept 1.00 -> 0.877, 21.1 -> 22.8 ms/token).' The promoted scheme keeps a prefix PLUS an explicit escape range for control tokens; the guard is the committed-id histogram of the gate corpus with a required margin, and the read's own cost bounds the upside -- 'the ceiling of any further trim is small and the acceptance downside is not.' Kinship: complete-coverage-scatter-collapses-operand-to-dense (the same-batch lens cites this trim as the PRUNING bound -- full coverage lets a scatter collapse to a dense op, and PARTIAL coverage is exactly what invalidates a trim; this rule is the dedicated guard: the histogram check, the escape range, and the promotion-gate verification contract the lens points at). Distinct from lm-head-loss-positions-only (B37 -- elides the projection for provably-DEAD positions, a cost elision; this is a VALUE-coverage constraint on a KEPT projection), from coarse-precision-first-exact-refine (B110 -- a lossy coarse KEY with exact refinement downstream; here the trimmed slice loses ids no refinement can recover), and from degenerate-selection-falls-back-to-identity-uniform (B110 -- neutralizes pathological VALUES before selection; this guards the SELECTION SPACE itself). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06ac -- Vendor/mlx-swift-lm/Libraries/MLXLLM/Models/Qwen35.swift:5171-5177 (the guard comment, verbatim, verified at the pin); origin diff 61936f265 (compact draft vocabulary: row-subset compose + on-device remap + padding rows semantically unreachable before argmax). Batch 115 addendum (collision salvage) / Research 139 / Plan 088. SCOPE CORRECTION (Batch 177 rider / Research 207): the guard binds on the head whose proposals are the COMMITTING path -- a DRAFT-ONLY trim with verification kept on the FULL head is the architectural escape. An out-of-slice draft id 'becomes a rejection, not an error': accepted output is unchanged, and a coverage miss degrades into ACCEPTANCE LOSS rather than a correctness loss (measured prior: 97% of drafted argmaxes fall under id 65536 on code/prose/devops; the slice is 397 MB / 1.65 ms -> 105 MB / 0.39 ms, EXL3_MTP_HEAD_N=65536 default-ON). Second clause, and it is the subtle one: the dynamic-draft CONFIDENCE must be read from the slice too -- identical when in-slice, LOWER otherwise, so dynamic drafting stops EARLIER and the error is forced into the conservative direction by construction. The trim fails closed at the source (a non-trellis/biased/scale-free head sets the head cache to False and takes the full path). Rider source: vcruz305/exllamav3 patch 0002 (qwen4_exp_mtp.py) @ e0ebee8d. Batch 177 / Plan 170 / Research 207.".to_string(),
            ref_shape: "// naive: the draft head is trimmed to the eval prompts' top ids -- the trim
// is never checked against the promotion gate's committed-argmax set, so ids
// the gate actually commits become unproposable and every hit is a forced
// reject that costs more round-bases than the trimmed read saved.
fn trim_readout_naive(vocab: usize) -> usize {
    vocab / 4 // 'the eval prompt never leaves the first quarter'
}
".to_string(),
            opt_pattern: "// optimized: the trim bound is gated by the committed-id histogram of the
// promotion gate -- every id the gate can commit must sit inside the kept
// slice, with an explicit escape range for control tokens.
fn trim_readout_guarded(vocab: usize, committed_ids: &[u32]) -> Option<(usize, usize)> {
    let keep = vocab / 2;
    let escape_start = vocab - 276;
    let max_committed = committed_ids.iter().copied().max()?;
    if max_committed as usize >= escape_start {
        return Some((keep, escape_start)); // the escape range covers control ids
    }
    if committed_ids.iter().any(|id| *id as usize >= keep) {
        return None; // the trim would make a committed id unproposable: refuse
    }
    Some((keep, escape_start))
}
".to_string(),
        },

// ============================================================
// -- dead-output-rows-state-append-only-path -- Batch 115 addendum (collision salvage) / Research 139 / Plan 088.
        KernelRule {
            rule_id: "dead-output-rows-state-append-only-path".to_string(),
            gpu_concept: "In a multi-row forward where only trailing row(s) feed every consumer (a proposal flush: history rows + current row), leading rows execute through a STATE-APPEND-ONLY path -- projection + norm + rope + cache update, NO attention output, no MLP, no final norm, no vocab head -- so dead rows 'never stream or compute the unused query+gate projection.' The path is backed by a dedicated packed KV-only weight (k|v concatenated at first use, group/bits/mode matched to the live projections, per-projection fallback otherwise) so the cheap path still reads a cache-friendly layout. The capability FAILS CLOSED before mutating any state: a guard (single-layer + single-cache + packed weight present) runs BEFORE the first cache append and reports 'not capable' on any mismatch, the caller takes the generic full path, and a nil result means NO mutation happened. Family members in the same diffs: the seed-tail projection (a long seed's full logits are dead -- the vocab head runs on the one kept row only), KV-write without vocab projection for prefix rows (the 248,320-way head paid once, not per row), and publish-the-normed-block (the verify forward already computes the post-norm block on the way to the vocab projection; consumers row-slice it because 'RMSNorm reduces only over the final axis, so these are the same values' as re-normalizing each row). Distinct from lm-head-loss-positions-only (B37 -- the projection-ELISION member expressed as a GEMV support-set gate; this ships the whole alternative row path + packed weights + the fail-closed capability contract), from write-before-read-state-skips-checkpoint (B58 -- which STATE needs no checkpoint under append-only writes; this is which ROWS need no compute), and from batch-composition-invariant-grid-early-return (B100 -- over-launched GROUPS return early; here the rows are never launched onto the expensive path at all). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06ac -- Sources/MLXFastModel/Qwen36MTPBlockSession.swift:717 (feeds ONE row in and gets ONE row back, and lastHiddenWithKVOnlyHistory), Vendor/mlx-swift-lm/Libraries/MLXLLM/Models/Qwen35MTP.swift:218 (func lastHiddenWithKVOnlyHistory), Qwen35.swift:2977 ('never stream or compute the unused query+gate projection'), diff cf350293f (the fail-closed guard: layers.count == 1, cache.count == 1, checked before any cache append). Batch 115 addendum (collision salvage) / Research 139 / Plan 088.".to_string(),
            ref_shape: "// naive: the proposal flush runs the FULL decoder block for every row --
// history rows' attention output, MLP and vocab head are provably dead
// (only the trailing row feeds any consumer), but the loop cannot tell.
fn flush_naive(rows: usize) -> Vec<f32> {
    (0..rows).map(full_block).collect() // dead work on rows 0..rows-1
}

fn full_block(_row: usize) -> f32 {
    0.0
}
".to_string(),
            opt_pattern: "// optimized: history rows take the state-append-only path -- projection +
// norm + rope + cache update, no attention output / MLP / vocab head --
// backed by a packed KV-only weight; the capability FAILS CLOSED before any
// cache mutation, and the caller falls back when the guard refuses.
fn flush_rows(rows: usize, keep_last: usize) -> Vec<f32> {
    if !kv_only_capable() {
        return (0..rows).map(full_block).collect(); // fail closed: generic path
    }
    let mut out = Vec::with_capacity(keep_last);
    for r in 0..rows - keep_last {
        append_state_only(r); // dead rows never stream the query+gate projection
    }
    for r in rows - keep_last..rows {
        out.push(full_block(r));
    }
    out
}

fn kv_only_capable() -> bool {
    true // single-layer + single-cache + packed-weight guard
}

fn append_state_only(_row: usize) {}

fn full_block(_row: usize) -> f32 {
    0.0
}
".to_string(),
        },

// ============================================================
// -- distinct-index-bitmap-ascending-compaction -- Batch 115 addendum (collision salvage) / Research 139 / Plan 088.
        KernelRule {
            rule_id: "distinct-index-bitmap-ascending-compaction".to_string(),
            gpu_concept: "When a selector's output is a SET OF DISTINCT INDICES (an argPartition segment -- no element repeats), ascending output order is a function of the set ALONE: no tie semantics exist, so the compaction needs neither a comparator nor an order-encoded key. One threadgroup marks the selected ids in a ceil(N/32)-word bitmap, runs an exclusive per-word popcount prefix scan (thread t owns the ascending word range), and emits set bits in ascending order -- replacing a multi-dispatch stable sort of thousands of keys with correctness by SET IDENTITY, one dispatch. The source's own contrast, verbatim: 'A counting sort therefore reproduces MLX.sorted exactly by construction: there is no tie rule to state' and 'the scan is exclusive over t, so the emitted ids ascend globally.' The boundary is explicit and rule-shaped: ordinal-map-exact-tail-selection (the same batch) handles the sibling kernel that selects tail-K over TIED float scores -- there a total order exists and must be order-encoded; HERE the producer emits distinct indices, so distinctness DELETES the tie machinery, and the only obligation is to verify the producer actually emits a set before dropping it. Deterministic by construction: the bitmap + scan order is input-order-independent and the emitted sequence is the unique ascending order of the set. Distinct from radix-select-ordered-key-topk (B107 -- selects over KEYED VALUES that tie, histogram passes over the key bytes; this compacts an already-selected index SET), from packed-total-order-multi-field-argmax (B107/B110 lineage -- manufactures a total order to break ties; this proves ties cannot exist), from ballot-matrix-tie-rank (B110 -- warp-ballot rank computation AMONG ties; here the tie class is empty), and from ordinal-map-exact-tail-selection (Batch 115 -- encodes a LIVE total order into a monotone key for bit-exact tail-K; this proves no order decision exists to encode). Source: Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06ac -- Vendor/mlx-swift-lm/Libraries/MLXLLM/Models/Qwen35.swift:4341-4358 (the counting-sort compaction comment, verbatim, verified at the pin: 'A counting sort therefore / reproduces MLX.sorted exactly by construction: there is no tie rule to / state' + 'the scan is exclusive over t, so the emitted ids ascend globally'). Batch 115 addendum (collision salvage) / Research 139 / Plan 088.".to_string(),
            ref_shape: "// naive: the selected ids are a SET (an argPartition segment -- no element
// repeats), but the compaction runs a comparator sort, paying tie machinery
// that cannot fire on a set and a multi-pass sort for what set identity
// already decides.
fn compact_naive(mut picked: Vec<u32>) -> Vec<u32> {
    picked.sort_by(|a, b| a.cmp(b)); // tie rules that cannot fire on a set
    picked
}
".to_string(),
            opt_pattern: "// optimized: mark the set in a bitmap, exclusive-scan per-word popcounts,
// emit set bits in ascending order -- correctness by SET IDENTITY, one pass,
// no comparator, no tie rule to state.
const WORDS: usize = (4096 + 31) / 32;

fn compact_ascending(selected: &[u32], vocab: usize) -> Vec<u32> {
    let mut bits = [0u32; WORDS];
    for id in selected {
        bits[(*id as usize) / 32] |= 1 << (*id % 32);
    }
    let mut out = Vec::with_capacity(selected.len());
    for (wi, w) in bits.iter().enumerate().take(vocab.div_ceil(32)) {
        let base = (wi * 32) as u32;
        let mut word = *w;
        while word != 0 {
            let bit = word.trailing_zeros();
            out.push(base + bit);
            word &= word - 1;
        }
    }
    out
}
".to_string(),
        },

    ]
}
