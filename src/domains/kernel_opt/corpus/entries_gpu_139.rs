//! Entries slice 139 — Batch 307 (Plan 312): the 10-29b staged
//! `kernel_opt` pool. Sources: vllm-project/vllm @ fa1cf1d (#64419) +
//! 9c7e8ff (#64345) (Apache-2.0), ml-explore/mlx @ 9e2e8e2 (#4791, MIT).
//! Two own rules; the batch's rider faces are attached in place on their
//! family homes (`gpu_17`, `gpu_33`, `gpu_134`, `gpu_137`). Every quote
//! re-verified byte-for-byte at the fetched .patch pins (plan 312 T0,
//! 2026-10-29). The fixtures are ORIGINAL minimal Rust reproductions —
//! nothing transcribed from the sources. Both rules are `NotBenchable`
//! (host write-coverage and dispatch-variant axes — no Pareto bench in
//! this corpus hosts them; the Batch-136 domain-reason class).

use super::KernelRule;

#[allow(clippy::too_many_lines)]
fn slice_b307_pool() -> Vec<KernelRule> {
    vec![
        // ── Rule 1 (category A — memory access): vllm fa1cf1d #64419 +
        // 9c7e8ff #64345, TWO same-day instances of ONE atom. Skip the
        // zero-fill when the write coverage is enumerable and the
        // never-written region is provably unread; the proof discipline
        // IS the content. ──
        KernelRule {
            rule_id: "write-coverage-proven-uninitialized-alloc".to_string(),
            gpu_concept: "MECHANISM (zero-fill elision under an enumerated write-coverage contract; category A — memory access): a host-side output allocation is zero-filled (`torch.zeros`) although a kernel assigns EVERY row a consumer can reach, so the fill is redundant full-buffer work paid every layer and step. The repair flips the allocation to uninitialized (`torch.empty`) — but the flip is justified by a PROOF, never by speed: (instance 1, the GDN core output) \"on every GDN layer and step, although _forward_core assigns every row up\" / \"to num_actual_tokens. Allocate it with torch.empty like the other\" / \"attention backends. Rows past num_actual_tokens (piecewise CUDA graph\" / \"padding) are only read by row-wise ops and never reach sampled outputs:\" — the incumbent even carried a comment arguing the opposite (\"# Note: we should not use torch.empty here like other attention backends,\"), and the PR re-opens that decision WITH the coverage proof: the write loop's extent (every row up to num_actual_tokens) is enumerated, the never-written region (padding rows behind it) is named, and each of its readers is classified (row-wise ops whose result never reaches sampled outputs); the accompanying test pins the contract by keying the split path on `_forward_core`. (instance 2, the SimpleCPUOffload host pool) \"CPU blocks become loadable only after their GPU-to-CPU store completes.\" / \"Allocate the host KV pool with torch.empty to avoid a redundant full-buffer\" — the never-read window is producer-ordered away: a block is loadable only after its store COMPLETES, so the zero-fill can never be observed (\"# CPU blocks are loaded only after a completed GPU-to-CPU store.\" / \"tensor = torch.empty(cpu_shape, dtype=gpu_tensor.dtype, device=\\\"cpu\\\")\"). THE LAW: skip a zero-fill only when the write coverage is ENUMERABLE and the never-written region is PROVABLY UNREAD — the proof has three load-bearing clauses (enumerate the write coverage precisely, name the never-written region, pin the producer-before-consumer ordering or the reader classification that makes the gap unobservable); the same contract in the OPPOSITE direction is equally load-bearing — a graph-replay consumer that DEMANDS physical zeros keeps the fill (the FLA `cuda-graph-static-buffer-zero-replay` pole), and the writer-side complement (a fused replacement must WRITE every legacy output on every path) is entries_gpu_138's own rule — same coverage contract, opposite duty. REGIME: host allocations feeding kernels whose write extent is a known function of a runtime count (num_actual_tokens, completed stores), padding/gap rows consumed only by reductions or row-wise transforms whose outputs are discarded. ANTI-REGIME: any reader that can observe the gap (sampling, graph replay with stale-region reuse, debug/dump paths); write extents that depend on data-dependent control flow (not enumerable); buffers shared with consumers that assume zero-init as an API contract. KINSHIP (do not double-count): the island-permutation row-coverage rule (entries_gpu_26) owns the OVERWRITE axis — complete overwrite coverage makes packed compute DEAD; THIS owns the INIT axis — enumerated write coverage makes the PRE-FILL dead; both are coverage proofs but over different lifetimes (overwrite-after-compute vs write-before-first-read). identity-keyed-graph-retention-stable-arena (entries_gpu_08) owns the zeroing-RESIZE face (which resizes re-zero) with no coverage contract; the FLA `cuda-graph-static-buffer-zero-replay` owns the replay-demands-zeros pole. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'torch.empty|zero-fill|zeros fill' -> the staging and pool families own allocation SIZING — no init-elision owner; 'uninitialized|MaybeUninit|set_len' -> zero owners in this corpus (the audit-grade Rust shape); 'write coverage|coverage proof' -> only the entries_gpu_26 overwrite axis (cited, different lifetime). NotBenchable here (an allocation-bandwidth axis measured on THEIR runtime; the coverage enumeration, the unread-region classification, and the producer-ordering guard are CPU-testable and ARE the fixtures). Source: vllm-project/vllm @ fa1cf1d #64419 (the GDN core output alloc + tests/kernels/mamba/test_gdn_forward_core_split.py) + 9c7e8ff #64345 (the SimpleCPUOffload host KV pool), Apache-2.0, quotes byte-verified at the fetched .patch pins (plan 312 T0, 2026-10-29). Batch 307 / Plan 312.".to_string(),
            ref_shape: "// ref_shape (rule: write-coverage-proven-uninitialized-alloc):\n// the host zero-fills a full output buffer although the kernel writes\n// every row any consumer can reach -- the fill bandwidth is paid on\n// every layer and step.\n#![allow(dead_code)]\n\nconst PAD_ROWS: usize = 8;\nconst LIVE_ROWS: usize = 5;\nconst COLS: usize = 4;\n\nstruct CoreOut {\n    data: Vec<u64>,\n}\n\nfn alloc_zero_filled(rows: usize) -> CoreOut {\n    CoreOut {\n        data: vec![0u64; rows * COLS],\n    }\n}\n\n// the kernel assigns every row up to the live count; padding rows behind\n// it are consumed only by a row-wise op whose result is discarded.\nfn forward_core_all_live_rows(out: &mut CoreOut, live: usize) {\n    for r in 0..live {\n        for c in 0..COLS {\n            out.data[r * COLS + c] = (r * COLS + c) as u64 + 1;\n        }\n    }\n}\n\n// a row-wise op reads the whole padded buffer but its per-row result\n// never reaches the sampled output.\nfn rowwise_stat_discarded(out: &CoreOut) -> usize {\n    (0..PAD_ROWS)\n        .filter(|&r| out.data[r * COLS..(r + 1) * COLS].iter().all(|&v| v > 0))\n        .count()\n}\n\n#[test]\nfn the_fill_covers_every_row_every_step() {\n    let mut out = alloc_zero_filled(PAD_ROWS);\n    // the fill touched EVERY cell of the padded buffer -- the full\n    // allocation, paid before the kernel runs a single row.\n    assert_eq!(out.data.len(), PAD_ROWS * COLS);\n    assert!(out.data.iter().all(|&v| v == 0));\n    forward_core_all_live_rows(&mut out, LIVE_ROWS);\n    // the kernel rewrote ONLY the live prefix; the padding rows still\n    // hold the fill they never needed.\n    for r in 0..LIVE_ROWS * COLS {\n        assert!(out.data[r] > 0);\n    }\n    for r in LIVE_ROWS..PAD_ROWS {\n        assert!(out.data[r * COLS..(r + 1) * COLS].iter().all(|&v| v == 0));\n    }\n    // the discarded row-wise op walked all PAD_ROWS rows; only the live\n    // ones were ever all-positive.\n    assert_eq!(rowwise_stat_discarded(&out), LIVE_ROWS);\n}\n".to_string(),
            opt_pattern: r"// opt_pattern (rule: write-coverage-proven-uninitialized-alloc):
// the allocation flips to uninitialized UNDER A PROOF: the write
// coverage is enumerated (live rows), the never-written region is named
// (padding), and its readers are classified (row-wise only, results
// discarded). The proof is asserted next to the allocation.
#![allow(dead_code)]

const PAD_ROWS: usize = 8;
const LIVE_ROWS: usize = 5;
const COLS: usize = 4;

struct CoreOut {
    data: Vec<u64>,
}

enum Init {
    // coverage proven: writes cover [0, live) x COLS; padding rows are
    // read only by row-wise ops whose results never reach the output.
    UninitProven,
    Zeroed,
}

fn alloc(init: Init) -> CoreOut {
    match init {
        // the uninitialized arm: same capacity, no fill
        Init::UninitProven => {
            let mut data: Vec<u64> = Vec::with_capacity(PAD_ROWS * COLS);
            unsafe {
                data.set_len(PAD_ROWS * COLS);
            }
            CoreOut { data }
        }
        Init::Zeroed => CoreOut {
            data: vec![0u64; PAD_ROWS * COLS],
        },
    }
}

fn forward_core_all_live_rows(out: &mut CoreOut, live: usize) {
    for r in 0..live {
        for c in 0..COLS {
            out.data[r * COLS + c] = (r * COLS + c) as u64 + 1;
        }
    }
}

// the only padding-region reader: a row-wise op whose per-row result is
// discarded -- the classification that makes the gap unobservable.
fn sampled_output(out: &CoreOut) -> Vec<u64> {
    out.data[..LIVE_ROWS * COLS].to_vec()
}

#[test]
fn the_uninit_arm_reads_only_proven_written_cells() {
    let mut out = alloc(Init::UninitProven);
    forward_core_all_live_rows(&mut out, LIVE_ROWS);
    let sampled = sampled_output(&out);
    assert_eq!(sampled.len(), LIVE_ROWS * COLS);
    for (i, v) in sampled.iter().enumerate() {
        assert_eq!(*v, i as u64 + 1);
    }
}

#[test]
fn a_sampling_reader_demands_the_zeroed_arm() {
    // the anti-regime: a consumer that reads the gap keeps the fill.
    let out = alloc(Init::Zeroed);
    assert!(out.data[LIVE_ROWS * COLS..].iter().all(|&v| v == 0));
}
".to_string(),
        },
        // ── Rule 2 (category E — dispatch): mlx 9e2e8e2 #4791. An
        // auxiliary operand whose RANK varies (scalar vs per-channel 4-D)
        // must select an indexing VARIANT at dispatch — a function
        // constant + kernel-name suffix + chunk-size switch, never one
        // stride math for both shapes. ──
        KernelRule {
            rule_id: "gate-rank-dispatch-function-constant-variant".to_string(),
            gpu_concept: "MECHANISM (rank-variant auxiliary-operand indexing selected at dispatch; category E — dispatch): the Metal gated-delta kernel indexed a per-head/per-channel gate (a 4-D `g`) with SCALAR-gate stride math — one rank's strides applied to another rank's tensor is SILENT WRONG INDEXING (values read from wrong offsets, no error). The fix selects an indexing VARIANT at dispatch: `bool vectorized_g = g.ndim() == 4;` becomes a shader function constant (`{&vectorized_g, MTL::DataType::DataTypeBool, 201}` feeding `[[function_constant(201)]]`), the kernel NAME gains the variant suffix (`base_name + (vectorized_g ? \"_vecg\" : \"\")` — a separately compiled specialization, not a runtime branch inside one kernel), and the CHUNK SIZE switches with the rank (`int C = vectorized_g ? 1 : gated_delta_chunk_size(T);` — the per-channel gate consumes one time step per dispatch, the scalar gate keeps the chunked walk). In-kernel the indexing splits on the constant: `auto g_ = g + (vectorized_g ? b_idx * T * Hv * Dk : b_idx * T * Hv);` / `state[i] * (vectorized_g ? g_[hv_idx * Dk + s_idx] : g_[hv_idx]);` / `g_ += vectorized_g ? (Hv * Dk) : Hv;` — per-channel strides (head × channel) vs the scalar stride (head only), each variant carrying only its own address math. THE LAW: an auxiliary operand whose RANK varies across call sites (scalar broadcast vs per-channel/per-head tensor) must select an indexing variant AT DISPATCH — a function constant + variant-suffixed kernel + the matching chunk/tile geometry; one shared stride computation for both ranks is silent wrong indexing, and a runtime `if` inside the kernel pays the branch in every thread to save one host-side variant selection. REGIME: auxiliary/conditioning operands (gates, scales, masks, biases) with a scalar fast path and a per-channel general path; kernel libraries exposing both postures over one entry point. ANTI-REGIME: ranks fixed by the API (validate and refuse the wrong rank instead — the extent-guard family); operands whose differing indexing the hardware broadcast unit already resolves (stride-0 promotion — the GQA zero-stride family owns that shape). KINSHIP (do not double-count): pipeline-cache-key-covers-every-function-constant (entries_gpu_35) owns the CACHE-KEY side — every cache tier must key the constant so a variant change recompiles; THIS owns the DISPATCH side — the rank test that picks the variant and the variant's own indexing. adjacent-step-kernels-fuse-mode-specialized (entries_gpu_134) owns mode SPECIALIZE for fusion with per-mode OPTIONAL params; THIS specializes for INDEXING correctness (no optional params, no fusion). extent-one-dim-skips-every-layout-check-symmetrically (entries_gpu_134) owns size-1 dims contributing NO layout fact — the degenerate-rank cousin; THIS is the rank-promotion path (rank-0 scalar vs the per-channel rank). GREP EVIDENCE (over the corpus, 3+ vocabularies): 'ndim|rank variant|scalar gate' -> zero owners; 'function_constant|function constant' -> the cache-key rule (entries_gpu_35) + the fusion rule (entries_gpu_134) — neither owns operand-rank indexing; '_vecg|variant suffix' -> zero. NotBenchable here (a correctness/dispatch axis — the defect is silent wrong values, not slow ones; the rank test, the variant selection, and both indexing walks are CPU-testable and ARE the fixtures). Source: ml-explore/mlx @ 9e2e8e2 #4791 (the metal gated-delta sources: the `vectorized_g` dispatch, function_constant(201), the `_vecg` suffix, the C=1 chunk switch), MIT, quotes byte-verified at the fetched .patch pin (plan 312 T0, 2026-10-29). Batch 307 / Plan 312.".to_string(),
            ref_shape: "// ref_shape (rule: gate-rank-dispatch-function-constant-variant):\n// ONE stride computation serves TWO operand ranks: a scalar gate is\n// indexed with per-channel strides (or the reverse) -- the values come\n// back wrong and nothing raises.\n#![allow(dead_code)]\n\nconst HV: usize = 2; // heads\nconst DK: usize = 4; // channels per head\n\n// the auxiliary gate arrives as EITHER a scalar per head (rank-1) or a\n// per-head/per-channel tensor (rank-2). The kernel assumes one shape.\nfn gated_step_wrong_strides(\n    state: &mut [[f32; DK]; HV],\n    g_flat: &[f32],\n    per_channel: bool,\n) {\n    // WRONG when per_channel: g_flat is laid out [hv][dk], but the walk\n    // below consumes ONE value per head.\n    for (h, row) in state.iter_mut().enumerate() {\n        let g = if per_channel {\n            g_flat[h * DK] // took only the head's FIRST channel as the scalar\n        } else {\n            g_flat[h]\n        };\n        for s in row.iter_mut() {\n            *s *= g;\n        }\n    }\n}\n\n#[test]\nfn the_scalar_stride_silently_misreads_the_ranked_gate() {\n    let mut state = [[1.0f32; DK]; HV];\n    let ranked: Vec<f32> = (0..HV * DK).map(|i| 0.5 + i as f32 * 0.1).collect();\n    gated_step_wrong_strides(&mut state, &ranked, true);\n    // every channel of head 0 got channel 0's gate: the four channel\n    // values collapsed onto ONE -- and nothing raised.\n    assert_eq!(state[0][0], state[0][1]);\n    assert_eq!(state[0][0], state[0][DK - 1]);\n    assert_eq!(state[1][0], state[1][DK - 1]);\n    // the value applied IS the first channel's gate (0.5): channel 1's\n    // 0.6 was never applied anywhere in the row.\n    assert!((state[0][0] - 0.5).abs() < 1e-6);\n    assert!((state[0][1] - 0.6).abs() > 1e-6);\n}\n".to_string(),
            opt_pattern: r"// opt_pattern (rule: gate-rank-dispatch-function-constant-variant):
// the rank test happens ONCE at dispatch; the chosen variant carries
// only its own indexing -- a separately named specialization per rank
// (the function constant the cache keys), never a branch in the hot
// loop.
#![allow(dead_code)]

const HV: usize = 2;
const DK: usize = 4;

#[derive(Clone, Copy, PartialEq)]
enum GateVariant {
    Scalar,
    PerChannel, // the _vecg specialization
}

fn select_variant(g_rank: usize) -> GateVariant {
    match g_rank {
        2 => GateVariant::PerChannel,
        _ => GateVariant::Scalar,
    }
}

fn gated_step_scalar(state: &mut [[f32; DK]; HV], g: &[f32]) {
    for (h, row) in state.iter_mut().enumerate() {
        for s in row.iter_mut() {
            *s *= g[h]; // one value per head, every channel
        }
    }
}

fn gated_step_per_channel(state: &mut [[f32; DK]; HV], g: &[f32]) {
    for (h, row) in state.iter_mut().enumerate() {
        for (s_idx, s) in row.iter_mut().enumerate() {
            *s *= g[h * DK + s_idx]; // head x channel strides
        }
    }
}

fn gated_step_dispatch(state: &mut [[f32; DK]; HV], g: &[f32], rank: usize) {
    match select_variant(rank) {
        GateVariant::PerChannel => gated_step_per_channel(state, g),
        GateVariant::Scalar => gated_step_scalar(state, g),
    }
}

#[test]
fn the_ranked_variant_applies_every_channel_gate() {
    let mut state = [[1.0f32; DK]; HV];
    let ranked: Vec<f32> = (0..HV * DK).map(|i| 0.5 + i as f32 * 0.1).collect();
    gated_step_dispatch(&mut state, &ranked, 2);
    for h in 0..HV {
        for (s_idx, v) in state[h].iter().enumerate() {
            let expect = 0.5 + (h * DK + s_idx) as f32 * 0.1;
            assert!((v - expect).abs() < 1e-6);
        }
    }
}

#[test]
fn the_scalar_variant_broadcasts_per_head() {
    let mut state = [[1.0f32; DK]; HV];
    let scalars = vec![0.9f32, 0.8];
    gated_step_dispatch(&mut state, &scalars, 1);
    assert!((state[0][0] - state[0][DK - 1]).abs() < 1e-6);
    assert!((state[1][0] - 0.8).abs() < 1e-6);
}
".to_string(),
        },
    ]
}

pub fn entries() -> Vec<KernelRule> {
    slice_b307_pool()
}
