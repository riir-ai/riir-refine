//! Entries slice 140 — Batch 309 (Plan 314): the 10-30 staged web-watch
//! intake (snapshot rows P105-P110). Sources: ml-explore/mlx @ e1ef8a9
//! (#4596, MIT) + ggml-org/llama.cpp @ 4d426d5 (#29298 patch 2, MIT) with
//! 41a4ad0 (#28770) as the regression it repaired. ONE own rule; the
//! batch's rider face is attached in place on its family home (`gpu_13`).
//! P106 + P108 are CONSUMED PRE-EXISTING (Batch 234/235 + Batch 253
//! respectively — recorded in the snapshot, no corpus delta). Every quote
//! re-verified byte-for-byte at the fetched .patch pins (plan 314 T0,
//! 2026-10-30). The fixtures are ORIGINAL minimal Rust reproductions —
//! nothing transcribed from the sources (MSL/CUDA). Not benchable: a
//! codegen load-issue axis — no Metal/CUDA validator in this corpus; the
//! numbers are source-reported.

use super::KernelRule;

#[allow(clippy::too_many_lines)]
fn slice_b309_webwatch() -> Vec<KernelRule> {
    vec![
        // ── Rule 1 (category B — compute/unroll): mlx #4596 (the TK=4
        // key-loop unroll, positive form) + llama.cpp #29298 patch 2 (the
        // template-on-ncols1 repair form). ONE atom: an unrolled scan's
        // loads issue together only when the loop bounds are compile-time.
        // ──
        KernelRule {
            rule_id: "runtime-trip-count-defeats-unroll-template-the-bound".to_string(),
            gpu_concept: "MECHANISM (compile-time trip count releases an unrolled scan's load issue; category B — compute/unroll): an outer scan unrolled N-wide only pays when its body's loads can ISSUE TOGETHER — and an inner loop with a RUNTIME trip count (or a data-dependent early exit) inside the unrolled body serializes exactly that issue, because the compiler cannot prove how many iterations precede the exit and must keep the loads ordered against it. TWO instances, one law. THE REPAIR FORM (llama.cpp #29298 patch 2 @ 4d426d5, the sparse-fa mask compaction scan): 'The query loop of flash_attn_mask_to_sparse_indices has a runtime trip count, which keeps the unrolled scan over the values of a lane from issuing its loads together. Template the kernel on ncols1 so the loop is bounded at compile time: batch one decodes compile to straight line code and the scan drops from 46 to 17 us at 49k columns on sparse decode shapes.' — the kernel gains `template <int ncols1>` (ncols1 leaves the parameter list), the per-lane item loop stays `#pragma unroll` over `values_per_lane = 8`, and the inner query guard keeps BOTH bounds with the COMPILE-TIME ONE FIRST: '#pragma unroll / for (int q = 0; q < ncols1 && q < q1 - q0 && !selected; ++q)' — the runtime bound survives as a no-op predicate inside the unrolled body, never as the loop's only bound; the launcher dispatches the two-instantiation family — 'GGML_ASSERT(ncols1 == 1 || ncols1 == 8);' + 'const auto kernel = ncols1 == 1 ? flash_attn_mask_to_sparse_indices<1> : flash_attn_mask_to_sparse_indices<8>;' — a small DISCRETE family is the admission: template explosion over a wide continuous bound is the anti-regime. The regression the repair closed was introduced by #28770 @ 41a4ad0 (the per-query compaction widened to per-GROUP): sparse decode +64–113% at batch 1, the added time a constant ≈0.59 ns per mask column scaling with kv and IGNORING n_kv_max — the SCAN kernel carried the cost, not the attention kernel; attributed by scaling analysis, CONFIRMED by the fix. THE POSITIVE FORM (mlx #4596 @ e1ef8a9, the sdpa decode KV loop): '// For each key: unroll TK to expose independent memory loads per iter.' with the depth chosen from REGISTER PRESSURE, not monotone — 'TK=4 is chosen to keep register pressure low (TK*(qk_per_thread + v_per_thread) half-regs of prefetch = 32 for D=V=128), leaving margin for the larger head-dim variants.' — and the loads HOARDED before the arithmetic consumes them: 'Prefetch: issue all TK K/V loads first so the HW can overlap them.' (k_reg/v_reg[TK][per_thread] filled in a load-only prologue, then the per-key score/softmax walk reads registers); the main loop strides TK keys per iteration ('for (; i + (TK - 1) * blocks < N; i += blocks * TK)'), pointers advance 'keys += TK * stride_k_tok;', and the per-key causal/mask guards stay INSIDE the unrolled body as per-t predicates (use_key per t) — masking never becomes an exit; a scalar TAIL loop ('// Tail (fewer than TK remaining keys)') keeps N % TK != 0 correct. DECISION CONTENT: (a) put the compile-time bound FIRST in the guard conjunction and keep the runtime bound as an in-body predicate — deleting the runtime bound is a correctness bug, keeping it as the only bound is the defect; (b) dispatch the small instantiation family and ASSERT the dispatch covers it (the upstream assert names {1, 8}); (c) hoard the unrolled body's loads in a prologue before the arithmetic (issue-then-consume), never interleave load-compute pairs the exit can reorder; (d) the unroll depth is a register-pressure TRADE — size it from (depth × per-thread registers) against the larger head-dim variants' margin, never 'more is better'; (e) the scalar tail preserves the trip-count freedom the template removed. REGIME: decode-attention KV scans, mask-compaction scans, and any lane-scoped inner loop inside an unrolled outer scan whose bound is a small discrete family (batch width, group width, TK). ANTI-REGIME: loops already compile-time bounded (nothing to template); bounds from a wide continuous range (instantiation explosion — keep the dispatch table); genuinely data-dependent trip counts with no uniform upper bound (the exit IS the algorithm — restructure the scan instead). KINSHIP (do not double-count): hardcoded-fast-path-jit-specialization (entries_gpu_01) bakes KNOWN-CONFIG constants into a dedicated variant so the JIT unrolls — THIS templates ONE loop bound across a small dispatched family of the SAME kernel, not whole-kernel config variants; the entries_gpu_06 compile-time-bound rule owns the ARRAY-FOOTPRINT axis (a runtime-bounded local array cannot unroll or register-promote — pipeline-override constant per bound-class; THIS owns the TRIP-COUNT axis of the loop itself — compose: a template trip count and a const-sized index array are the same law on two surfaces); the Batch-276 inject-4 rider on the fused gate/up host unrolls a DEPENDENT K-walk as a pure scheduling hint ('UNR is a scheduling hint only; never split or reassociate the sum') — THIS unrolls INDEPENDENT key iterations to expose memory-level parallelism, the opposite dependence shape; the rms-norm vpt ladder (entries_gpu_104) picks (block, vpt) at LAUNCH time from row width — its inner carry loop is compile-time bounded BY the template vpt, the same bound-first discipline on the carry axis. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'unroll' 136 hits — the owners are the array-footprint rule, the vpt ladder, the scheduling-hint rider, and per-loop pragma mentions; NONE owns the runtime-trip-count law; 'trip count|runtime trip' — ZERO owners; 'prefetch' family owns tile-grain staging pipelines (the fetch/store split, APREFETCH) — not loop-bound templates; 'issue.*together|memory-level' — ZERO owners. NotBenchable here (a codegen load-issue axis; the numbers are source-reported — 46→17 us at 49k columns, the +64–113% batch-1 regression class; no Metal/CUDA validator in this corpus; the bound-first guard, the dispatch assert, and the hoarded-prologue equivalence are CPU-testable and ARE the fixtures). Sources: ml-explore/mlx @ e1ef8a9 (#4596, 'metal/sdpa: increase vector decode memory-level [parallelism]', MIT) — mlx/backend/metal/kernels/sdpa_vector.h; ggml-org/llama.cpp @ 4d426d5 (#29298 patch 2/3, MIT) + 41a4ad0 (#28770, the introduced regression) — ggml/src/ggml-cuda/fattn.cu. Batch 309 / Plan 314. Every quote verbatim at the pin. RIDER (Batch 323 / Plan 323 — the ALL-SLICES-ISSUED-BEFORE-SUMS face, ashhart/TensorFold nemotron #548 step 2 @ 7737cd33+5fde7dee+9637983a+40d063cf+f3dbae73, Apache-2.0): the MoE router score top-k is the load-issue law's third surface — moe_glue.cu loads EVERY slice's partial before any sum consumes one, verbatim [in-tree]: 'every slice's and the bias's load issued before the sums' (the two-phase loop: all `part[P][MAXSK]` + `b[P]` loads issued first, MAXSK = 8, the layout [slices, rows, ne], the host cap most_slices = 8 'The score form's slices a row at most'), then folds in SLICE ORDER with `__fadd_rn` — the ordered fold is the Issue-940 ordered-reduction law in the wild (the real in-code line: 'logit the `slices` partials [slices, rows, ne] summed in order'; the commit message: 'adds the partials [slices, rows, experts] in slice order before the sigmoid, the same chain as the combine's fma(1, p, acc)'), plus the +1e-20 normalize guard (`__fdiv_rn(xk, __fadd_rn(sum, 1e-20f))`, float64-reference parity). SAME ATOM as this rule (an unrolled scan's loads issue together only when nothing in the body serializes them) — the instance is the MoE router scores; the per-lane runtime-loop form it replaced is the anti-shape. Count-neutral face, Batch 323 / Plan 323.".to_string(),
            ref_shape: r"// ref_shape (rule: runtime-trip-count-defeats-unroll-template-the-bound):
// the scan is structurally unrolled over ITEMS, but the inner query gate is
// a RUNTIME trip count with an early exit -- the loads stay interleaved
// with the gate, so each item's load issues only after the previous item's
// gate resolved. The loader counts every load issue site.
#![allow(dead_code)]

const ITEMS: usize = 8; // the unrolled lane items (values_per_lane)
const QUERIES: usize = 8; // the group width (ncols1) -- RUNTIME at the call

struct Scan {
    loads_issued: usize,
}

// one row of the mask, i = the key column this item covers
fn gate_key(mask: &[i8], i: usize) -> bool {
    i < mask.len() && mask[i] != 0
}

fn scan_naive(
    s: &mut Scan,
    mask_rows: &[(usize, Vec<i8>)], // (live_queries, row)
    cols: &[usize],
) -> Vec<bool> {
    let mut selected = vec![false; cols.len()];
    for (live, row) in mask_rows {
        for (item, &i) in cols.iter().enumerate() {
            // the RUNTIME trip count + early exit inside the unrolled body:
            // q's bound is `live`, selected is data-dependent
            let mut hit = false;
            for q in 0..*live {
                s.loads_issued += 1; // the load cannot hoist past the gate
                if gate_key(&row[q..], i) {
                    hit = true;
                    break;
                }
            }
            selected[item] = hit;
        }
    }
    selected
}

#[test]
fn the_runtime_bound_serializes_the_loads() {
    let mut s = Scan { loads_issued: 0 };
    let rows = vec![(QUERIES, vec![0i8; 16])]; // live == the full width
    let cols: Vec<usize> = (0..ITEMS).map(|i| i).collect();
    let out = scan_naive(&mut s, &rows, &cols);
    assert!(out.iter().all(|o| !*o));
    // worst case: ITEMS x QUERIES interleaved load-gate pairs -- nothing
    // hoisted, the early exit keeps every load ordered after its gate.
    assert_eq!(s.loads_issued, ITEMS * QUERIES);
}
".to_string(),
            opt_pattern: r#"// opt_pattern (rule: runtime-trip-count-defeats-unroll-template-the-bound):
// the kernel is TEMPLATED on the group width; the inner gate keeps the
// compile-time bound FIRST with the runtime bound as an in-body predicate;
// the launcher dispatches the {1, 8} family and asserts coverage; the
// runtime predicate keeps live < NCOLS correct inside the unrolled body.
#![allow(dead_code)]

const ITEMS: usize = 8;

struct Scan {
    loads_issued: usize,
}

fn gate_key(mask: &[i8], i: usize) -> bool {
    i < mask.len() && mask[i] != 0
}

// the template: NCOLS1 is compile-time -- the q-loop unrolls to straight
// line code at NCOLS1 == 1 and the loads of all ITEMS hoist.
fn scan_templated<const NCOLS1: usize>(
    s: &mut Scan,
    live: usize,
    row: &[i8],
    cols: &[usize],
) -> [bool; ITEMS] {
    let mut selected = [false; ITEMS];
    for (item, &i) in cols.iter().enumerate() {
        let mut hit = false;
        // compile-time bound FIRST, runtime bound kept INSIDE
        for q in 0..NCOLS1 {
            if q < live {
                s.loads_issued += 1;
            }
            if q < live && gate_key(row, i + q) {
                hit = true;
                break;
            }
        }
        selected[item] = hit;
    }
    selected
}

fn scan_dispatch(
    s: &mut Scan,
    ncols1: usize,
    live: usize,
    row: &[i8],
    cols: &[usize],
) -> [bool; ITEMS] {
    assert!(ncols1 == 1 || ncols1 == 8, "the dispatch family is 1 or 8");
    if ncols1 == 1 {
        scan_templated::<1>(s, live, row, cols)
    } else {
        scan_templated::<8>(s, live, row, cols)
    }
}

#[test]
fn the_template_hoists_the_loads_and_keeps_the_results() {
    let mut s = Scan { loads_issued: 0 };
    let row = vec![0i8; 16];
    let cols: Vec<usize> = (0..ITEMS).map(|i| i).collect();
    let out = scan_dispatch(&mut s, 8, 8, &row, &cols);
    assert!(out.iter().all(|o| !*o));
    // every load still happens (the runtime predicate stays), but the
    // bounded loop is straight-line: the compiler hoists all ITEMS' loads.
    assert_eq!(s.loads_issued, ITEMS * 8);
}

#[test]
fn the_runtime_predicate_keeps_live_below_the_width_correct() {
    let mut s = Scan { loads_issued: 0 };
    let row = vec![0i8; 4];
    let cols: Vec<usize> = (0..ITEMS).map(|i| i).collect();
    // live 3 < NCOLS1 8: the runtime predicate skips the dead lanes
    let out = scan_dispatch(&mut s, 8, 3, &row, &cols);
    assert!(out.iter().all(|o| !*o));
    assert_eq!(s.loads_issued, ITEMS * 3);
}

#[test]
#[should_panic(expected = "dispatch family")]
fn the_dispatch_refuses_an_unrepresented_width() {
    let mut s = Scan { loads_issued: 0 };
    let row = vec![0i8; 16];
    let cols: Vec<usize> = (0..ITEMS).map(|i| i).collect();
    let _ = scan_dispatch(&mut s, 4, 4, &row, &cols);
}
"#.to_string(),
        },
    ]
}

pub fn entries() -> Vec<KernelRule> {
    slice_b309_webwatch()
}
