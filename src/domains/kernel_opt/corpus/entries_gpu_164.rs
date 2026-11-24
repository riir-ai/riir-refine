//! Batch 404 — the mlx #4927 "[CUDA] faster window attention" pack
//! (Plan 403 / Research 438): ml-explore/mlx PR #4927, merged 2026-11-22
//! at `58d91ee81a86dd9710123ec42e7ff91f6d1d21b1` (commits `318a72d` +
//! `5a7769e`, MIT — pierreablin, zcbenz approved). The CUDA sibling of
//! Batch 197's Metal SDPA pack — same file family
//! (`scaled_dot_product_attention.cpp`).
//!
//! BEFORE: CUDA window attention hit `has_fused_kernel`'s NYI gate —
//! `return {false, "window attention support for CUDA is NYI"}` — so
//! EVERY windowed call ran the unfused fallback = full attention:
//! O(L²) time and O(L²) peak memory (L=32768/W=512 fwd 133.49→0.43 ms
//! ~310×, peak 41.3→1.35 GB, fwd+bwd OOM→1.99 ms; zero-window rows
//! byte-flat 0.24 vs 0.25 ms). AFTER: the window encodes EXACTLY into
//! cuDNN's `fe::graph` SDPA diagonal-band attributes, the decode path
//! slices the KV operand to the window, the version gate refuses the
//! known-wrong cuDNN window, and the fused VJP falls back instead of
//! throwing on ungeneric gradient directions.
//!
//! - `windowed-parameter-nyi-gate-forces-full-work-fallback` — the
//!   dispatch lead (D): a capability gate answering not-implemented for
//!   a parameter the fast path's API encodes EXACTLY routes every such
//!   call to the generic path that ignores the parameter's structure.
//! - `slice-operand-to-window-before-kernel` — the decode operand
//!   reduction (A): reduce the operand to the last W keys, kernel
//!   untouched; HOME-VALIDATED (riir-infer-gpu `gemma4_cubecl`).
//! - `dependency-version-known-wrong-window-refusal` — the version
//!   gate (N): refuse the fast path below a pinned-wrong version with
//!   the bug class named.
//! - `fused-vjp-ungeneric-direction-falls-back-not-throw` — the VJP
//!   dispatch (D): ungeneric gradient directions route to the
//!   composition fallback, never throw.
//!
//! Faces (count-neutral, same commit): `gqa-zero-stride-batch-broadcast`
//! (`gpu_65`) += the stride-0 decode refusal (the REFUSAL polarity of the
//! same stride-0 observation); `compiled-handle-direct-launch-key-completeness`
//! (`gpu_150`) += the `is_equivalent` primitive-identity window face.
//!
//! THE CENSUS + BOOKKEEPING REPAIR rides this landing: `entries_gpu_163`
//! (B402's file) joined `expected_corpus_files`/`per_file_entries` —
//! B402 landed the file without arming the census (both lists blind
//! together, the B379/B380 class), and B402's re-pin trail row (1109)
//! was never written to the tests.rs chain — both repaired here, the
//! A/D/N category headers repaired to measured truth with the drift
//! disclosed (the B310-era pin-drift precedent).
//!
//! Fixtures: `corpus/fixtures/b404_fixtures.rs` — standalone-compiled
//! AND executed BEFORE entry (debug AND -O), the four-arm discipline.
//! Dedup re-verified at the live corpus at authoring time (Protocol A;
//! the verdict's greps are same-day). NOTHING INVENTED — every quote
//! read at the clone this session (pin `58d91ee8…`).
#![allow(dead_code)]

use crate::domains::kernel_opt::KernelRule;

/// Batch 404 — the mlx #4927 cuda-window-sdpa pack's `kernel_opt` half.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — the dispatch lead (A−): the NYI gate cost was never "no
        // fusion" — it was the quadratic the parameter exists to avoid.
        // The window IS a diagonal band; the fast path's API already
        // speaks bands. ──
        KernelRule {
            rule_id: "windowed-parameter-nyi-gate-forces-full-work-fallback".to_string(),
            gpu_concept: "MECHANISM (the NYI-gate dispatch law; category D — dispatch/admission): a capability gate that answers NOT-IMPLEMENTED for a parameter the fast path's backend API can encode EXACTLY forces every call carrying that parameter onto the generic path that IGNORES the parameter's structure — and the cost is not 'no fusion', it is the full-work scaling the parameter exists to avoid. The source's before-state is the whole law: mlx's CUDA window attention hit `return {false, \"window attention support for CUDA is NYI\"}` in `has_fused_kernel`, so EVERY windowed scaled-dot-product call ran the unfused fallback = FULL attention — O(L²) time AND O(L²) peak memory (their H100 table, bf16 16Q/8KV dim128: L=32768 window=512 fwd 133.49→0.43 ms ~310×, peak 41.3→1.35 GB, fwd+bwd out-of-memory→1.99 ms; the zero-window control rows byte-flat 0.24 vs 0.25 ms — the fix never touches the unaffected path). THE REMEDY ORDERING is the fix space: (1) ask whether the parameter has an EXACT encoding in the fast path's existing API — a sliding window IS a diagonal band, and cuDNN's fe::graph SDPA already speaks bands: `.set_diagonal_alignment(q.shape(2) == k.shape(2) ? fe::DiagonalAlignment_t::TOP_LEFT : fe::DiagonalAlignment_t::BOTTOM_RIGHT).set_diagonal_band_left_bound(window_size).set_diagonal_band_right_bound(do_causal ? 0 : window_size - 1)` — no new kernel, the parameter rides the graph attributes; (2) encode it; (3) refuse ONLY the classes the API genuinely cannot express, EACH REFUSAL NAMED — `// cuDNN does not support a window with an array mask, or when T_q > T_kv.` and `// The vector kernel reads only the keys in the window.` (decode admits the vector kernel instead, after the operand slice — see slice-operand-to-window-before-kernel); (4) version/admission-gate the encoding on known-wrong support (see dependency-version-known-wrong-window-refusal). NEVER: leave an NYI refusal standing while an exact encoding exists; refuse with an unnamed reason (a named refusal is a dispatch table row, an unnamed one is a dead end); encode lossily (the window→band map is EXACT — if you cannot map exactly, that class belongs in the named-refusal list, not in an approximation); delete the parameter at the generic path and hope (the fallback that ignores the structure is the bug, not the answer). THE COMPLEMENT RIDES IN-RULE (the source's own `clip_window`, verbatim `// Remove the window when it keeps all the keys.` → `return window_size < std::max(q.shape(2), k.shape(2)) ? window_size : -1;`): a constraint that excludes nothing is not a specialization — pass the plain path its no-op so it takes its fastest form; the band machinery is not free, so a band covering the whole domain must not be paid. THE VALIDATOR ARM: the work-quantity difference is Rust-benchable and exact — full-mask materialization writes L×L booleans where band iteration writes L×W (512× fewer at the source's 32768/512), bit-identical content by construction; the DECISION itself (dispatch routing to cuDNN) is NotBenchable-by-domain here (the win metric lives in cuDNN latencies — the gpu_65 annotation class, never a demotion). ⛔ KINSHIP (adjudicated both directions): quadratic-attention-intermediates-recompute-from-linear-saves (gpu_04) owns the BACKWARD memory axis (save the linear tensors, recompute the quadratic chain at backward) — THIS owns the FORWARD dispatch axis (the admission gate itself decides which path runs at all) — compose (a windowed op needs both: the band forward AND the backward that never materializes the band); fa-fast-path-validated-exact-shape-admission owns admission-by-SHAPE-validation (the fast path admits compatible shapes) — THIS owns admission-by-ENCODING (the parameter must be expressible in the fast path's API, a stronger and different test — a shape can be compatible while the parameter is unencodable: array-mask+window); sibling-kernel-admission-gap-absorbed-by-generalist (D) owns the INVERSE polarity (the generalist kernel absorbs a specialist kernel's refusals so the dispatch widens) — THIS removes a refusal by ENCODING the parameter the specialist already supports; cite both, never merge; sdpa-backward-as-tiled-matmul-bounded-materialization (gpu_65) is the Metal sibling of the same source file family — cite lineage, different axis (backward composition, not forward admission). REGIME: dispatch admission over parameterized op families where the fast path's backend API has an exact parameter encoding (band bounds, ranges, window counts, stride classes); ML-runtime op dispatch (attention windows, conv dilations, pool strides); any capability probe with a generic fallback. ANTI-REGIME: the parameter changes semantics the fast path cannot express (the named-refusal class is the correct answer there); the encoding is approximate; the encoded fast path is SLOWER than the generic fallback at the actual shapes (measure — the source kept the unfused path for zero-window rows and the vector path for decode); the API's encoding is version-gated wrong (the version rule owns that half). GOAT: the fixture pins (a) the full-mask write count L×L vs the band's L×W at 32768/512 (512×), (b) band-built mask bit-identical to the full mask, (c) the two-independent-bounds tamper arm (dropping the causal join produces a different mask — red), (d) the array-mask+window named refusal (green refusal). P113: MERGED upstream at the pin — their own landed decision, decision content only, never an outbound PR. Source: ml-explore/mlx @ 58d91ee81a86dd9710123ec42e7ff91f6d1d21b1 (PR #4927, merged 2026-11-22, MIT) — mlx/backend/cuda/scaled_dot_product_attention.cpp:385-400 (the window admission arms + comments), :226-240 (set_window + the alignment comment verbatim), :629-632 (clip_window + comment verbatim), :643-660 (the NYI removal site in has_fused_kernel); the PR's H100/B200 tables (provenance, never adoption bars). Quotes byte-verified at the pin this session. MIT. Batch 404 / Plan 403 / Research 438.".to_string(),
            ref_shape: r#"// NAIVE: the capability gate refuses the parameter, so every
// windowed call lands on the generic path that IGNORES the window
// and materializes the FULL mask — O(L*L) work and memory, exactly
// what the window parameter exists to avoid (133 ms / 41 GB at the
// source's 32768/512 vs 0.43 ms / 1.35 GB through the band path).
fn windowed_mask_nyi(l: usize, window: Option<usize>) -> Vec<bool> {
    if window.is_some() {
        // "window attention support is NYI" -> the full-attention
        // fallback runs instead, blind to the window
    }
    let mut m = vec![false; l * l]; // the quadratic the parameter avoids
    for r in 0..l {
        for c in 0..l {
            m[r * l + c] = c <= r; // causal only — the window is GONE
        }
    }
    m
}

// the unnamed refusal — a dead end, not a dispatch row
fn supports(_window: usize, has_array_mask: bool) -> Result<(), String> {
    if has_array_mask {
        Err("unsupported".to_string()) // WHICH class? why? not sayable
    } else {
        Ok(())
    }
}"#.to_string(),
            opt_pattern: r#"// FIXED: encode the parameter into the fast path's API when the
// encoding is EXACT (a window IS a diagonal band), refuse only the
// named-unencodable classes, and clip the vacuous constraint.
enum MaskPlan {
    Band { left: usize, right: usize }, // the exact band encoding
    Plain,                              // vacuous window -> the fast path
    Refused(&'static str),              // named-unencodable class
}

fn plan_window(l: usize, q_len: usize, window: usize, has_array_mask: bool) -> MaskPlan {
    // clip_window: a constraint that excludes nothing is not a
    // specialization — never pay band machinery for it
    if window >= l.max(q_len) {
        return MaskPlan::Plain;
    }
    // the named refusal: this class the band encoding cannot express
    if has_array_mask {
        return MaskPlan::Refused("window with an array mask");
    }
    if q_len > l {
        return MaskPlan::Refused("more queries than keys");
    }
    // the EXACT encoding: band bounds carry the window; the right
    // bound carries the causal join (0 = row-dependent causal edge)
    MaskPlan::Band {
        left: window,
        right: 0,
    }
}

// the band iterate: O(L*W) writes, bit-identical to the full mask
fn band_iterate(l: usize, w: usize) -> Vec<bool> {
    let mut m = vec![false; l * w]; // the work the parameter buys
    for r in 0..l {
        for c in (r.saturating_sub(w - 1))..=r {
            m[r * w + (c - (r.saturating_sub(w - 1)))] = true;
        }
    }
    m
}"#.to_string(),
        },
        // ── R2 — the decode operand reduction (B): the window makes the
        // last W keys derivable; slice the operand, keep the kernel. ──
        KernelRule {
            rule_id: "slice-operand-to-window-before-kernel".to_string(),
            gpu_concept: "MECHANISM (operand reduction at the dispatch edge; category A — memory access/masking): when a parameter makes a query-invariant reduction of an OPERAND derivable — decode attention with a window attends only the LAST W keys — reduce the operand BEFORE the kernel call and keep the kernel untouched; never teach the kernel the parameter. The source's decode fallback, verbatim `// With one query, the window keeps the last window_size keys.` — `slice_last_kv` builds `kv[..., -n:, :]` on the GPU (`slice_gpu`, no host roundtrip) and hands the EXISTING vector kernel the reduced operand: the kernel's code is unchanged, its reads drop O(L)→O(W), and the semantics are EXACT because with one query and bottom-right alignment the window IS the last W keys. THE HOME SECOND INSTANCE (cited as production precedent, not source): riir-infer-gpu's gemma4_cubecl lane already does exactly this — `//! CPU-side attention window slicing (the GPU attention kernel itself is unchanged from Gemma-2; the sliding-window mask is handled by slicing the KV cache window before upload, matching the CPU reference).` via `get_combined_kv_window` (decode AND prefill; sliding = `[pos-sw+1, pos]`) — the corpus rule's pattern validated in our own serving stack before this source landed it upstream. NEVER: teach the kernel a window parameter it must test per read (the predicate-in-loop form reads all L to keep W); slice a FRONT window (the window is suffix-anchored — newest keys, not oldest); global-slice a MULTI-query batch (per-row windows differ — the source refuses exactly that class at `q.shape(2) > k.shape(2)` and takes the vector kernel only at one query); roundtrip the slice through the host (the slice executes at the operand's device); slice an operand whose storage is appended in place without the ownership check (the source's `use_cudnn_for_decoding` refuses `kv.strides(2) == 0` FIRST — see the gqa-zero-stride face: a broadcast row is not a contiguous cache row and the stride-derived length math would misread it — the slice/reduction rides a CONTIGUITY predicate, never a hope). ⛔ KINSHIP (adjudicated both directions): gqa-zero-stride-batch-broadcast (gpu_65, A) owns stride-0-as-broadcast (bind the operand with a zero stride so the GEMM sees the group — a BINDING trick) — THIS reduces the operand to a window (a SHAPING trick); the stride-0 refusal polarity of the same observation folds as this rule's contiguity face on that rule; verify-window-weight-load-amortization-exact (gpu_163, B) amortizes a LOAD across verify positions — THIS reduces the operand itself (fewer elements exist at all); prefill-batch-pool-f16-conversion-sized-by-actual-length (A, B261) shapes the operand pool by ACTUAL length — compose (the window is one more actual-length key over the same dispatch-edge shaping); decode-attn-cache-rows-register-direct-occupancy-not-smem-tile (gpu_159) is the kernel-side pole (read cache rows once into registers) — THIS is the host-side pole (never hand the kernel rows outside the window) — compose (slice first, then the register-direct read). REGIME: decode/single-query attention with a window; any operand reduction derivable from a parameter that leaves the kernel's semantics unchanged ON THE REDUCED OPERAND (suffix windows, top-k pre-selection, length clamps at decode). ANTI-REGIME: multi-query where the per-row window differs (named refusal, not a slice); prefill with per-row masks (the band iterate owns that); operands where the slice's copy costs more than the reads it saves (tiny windows on tiny caches — measure); in-place ring caches where the suffix is NOT contiguous (the contiguity predicate refuses, the ring read owns it). GOAT: the fixture pins (a) full-scan-with-predicate vs slice-then-plain-scan bit-identical scores with the read count L vs W, (b) the front-window tamper (wrong keys — red), (c) the multi-query global-slice refusal (green refusal), (d) the stride-0 contiguity refusal before the slice (green refusal). P113: MERGED upstream at the pin — decision content only. Source: ml-explore/mlx @ 58d91ee81a86dd9710123ec42e7ff91f6d1d21b1 (PR #4927, MIT) — mlx/backend/cuda/scaled_dot_product_attention.cpp:129-136 (slice_last_kv), :751-754 (the fallback arm + comment verbatim), :97-102 (the contiguity refusal incl. the stride-0 arm); the home instance: riir-infer-gpu/src/gemma4_cubecl/mod.rs:36-38 + dispatch.rs get_combined_kv_window (our own production lane, read this session). Quotes byte-verified at the pin. MIT. Batch 404 / Plan 403 / Research 438.".to_string(),
            ref_shape: r"// NAIVE: the window rides INSIDE the kernel as a per-read
// predicate — every one of the L cache rows is read (and the
// predicate evaluated) to keep W of them. The kernel also grows a
// window parameter it must honor at every site forever.
fn decode_attention_predicated(
    q: &[f32], k: &[f32], l: usize, w: usize,
) -> f32 {
    let mut acc = 0.0f32;
    for i in 0..l {
        // the window test runs for every key, the L-W losers included
        if i + w >= l {
            acc += q[i % q.len()] * k[i];
        }
    }
    acc
}

// the front-window tamper: oldest keys, not newest
fn decode_attention_front(
    q: &[f32], k: &[f32], _l: usize, w: usize,
) -> f32 {
    let mut acc = 0.0f32;
    for i in 0..w {
        acc += q[i % q.len()] * k[i]; // WRONG end of the cache
    }
    acc
}".to_string(),
            opt_pattern: r#"// FIXED: the parameter reduces the OPERAND at the dispatch edge —
// the kernel never learns the window existed. O(W) reads, the code
// unchanged, the semantics exact (one query, suffix-anchored).
fn decode_attention_windowed(
    q: &[f32], k: &[f32], l: usize, w: usize,
) -> f32 {
    // the contiguity/broadcast predicate refuses before the slice
    // (a broadcast row is not a cache row — the stride-0 face)
    // ... refuses !contiguous || broadcast ...
    let start = l.saturating_sub(w); // suffix-anchored: the LAST w keys
    let kv = &k[start..];            // O(W) operand; kernel untouched
    let mut acc = 0.0f32;
    for (i, ki) in kv.iter().enumerate() {
        acc += q[(start + i) % q.len()] * ki; // the plain loop, no predicate
    }
    acc
}

// the named refusal: multi-query batches have PER-ROW windows — a
// global suffix slice is WRONG there, so the class refuses by name
fn admits_slice(t_q: usize, t_kv: usize) -> Result<(), &'static str> {
    if t_q > 1 && t_q != t_kv {
        Err("per-row windows differ at more than one query")
    } else {
        Ok(())
    }
}"#.to_string(),
        },
        // ── R3 — the version gate (B): a version window where the backend
        // is KNOWN-WRONG refuses loud, bug class named. ──
        KernelRule {
            rule_id: "dependency-version-known-wrong-window-refusal".to_string(),
            gpu_concept: "MECHANISM (the version-gated known-wrong admission; category N — dtype/alignment/exactness specialization): a fast path gated not on capability but on a KNOWN-WRONG dependency version window refuses loud and falls back, NAMING the bug class. The source's gate, verbatim: `// Older cuDNN does not support sinks (before 9.13) and gives wrong gradients when T_kv % 128 != 0 (before 9.15.1).` → `if (cudnnGetVersion() < 91501) { return false; }` — the whole windowed path (forward AND backward share the admission) stays off below the pinned version even where the modulus class cannot hit, because a version-window bug class is proven by the upstream fix, not re-derived per shape at the call site. THE SECOND CLAUSE rides the same admission: the encoding must match the version's required mask GEOMETRY, not the mathematically-equivalent one — verbatim `// Keep the keys less than window_size away from the query. If T_q == T_kv we align to the top left (the same mask) since cuDNN <= 9.9 on Hopper needs it.` — TOP_LEFT when T_q == T_kv, BOTTOM_RIGHT otherwise, chosen per what the backend's kernels were built to read. NEVER: admit a fast path inside a known-wrong version window because THIS shape evades the reported repro (the conservative whole-path refusal is the landed discipline — narrowing it is a re-derivation of the upstream fix, owed upstream evidence); refuse on a version without naming what breaks (an unnamed version gate is unactionable at bump time and rots into cargo-cult); pin a version you cannot cite the fix for; treat SLOWER-at-old-version as wrong (that is the autotune/capability axis, not the correctness axis). ⛔ KINSHIP (adjudicated both directions): compiled-vs-eager-per-op-exactness-admission (N) owns the per-op-CLASS axis (integer and MAC classes admit on zero differing pairs, transcendentals refuse on a demonstrated pick flip) — THIS owns the dependency-VERSION axis (a version window where the backend is known-wrong, fixed upstream at a version); mxu-op-exactness-scoping (N) owns width-scoped exact admission (exact up to 128 output columns, the head dim in halves) — the T_kv % 128 modulus here is its cousin (the bug keys on an ALIGNMENT class), but this rule's landed decision refuses the whole path below the pin rather than narrowing by the modulus; enumerable-domain-bit-identity-admission-gate (N) owns the enumerable-domain proof shape for admissions — the version axis has no enumerable domain at the call site (the proof lives upstream), which is exactly why the gate keys on the version. REGIME: fast paths over versioned backends (cuDNN, oneDNN, Metal, driver APIs, codec libs) where a specific version window computes wrong results for a named shape class; training-sensitive gradients behind inference-only fast paths. ANTI-REGIME: capability absence (no fast path at all — the shape/encoding admission's job, not the version's); old-but-slow (correctness fine — measure, never refuse); unpinned folk wisdom (the refusal must cite the fixed version, or it is a rumor in the dispatch path). GOAT: the fixture pins (a) the wrong-gradient model at version < pin on the modulus class vs the version-gated fallback exact, (b) the conservative form: below the pin the path refuses even where the modulus class cannot hit (the landed decision, disclosed), (c) the geometry tamper: TOP_LEFT/BOTTOM_RIGHT swapped at T_q == T_kv produces a different mask (red), (d) an unnamed version refusal (the disclosure arm: refuse-why-less is the rotable form). P113: MERGED upstream at the pin — decision content only. Source: ml-explore/mlx @ 58d91ee81a86dd9710123ec42e7ff91f6d1d21b1 (PR #4927, MIT) — mlx/backend/cuda/scaled_dot_product_attention.cpp:385-393 (the gate + comment verbatim), :226-240 (the alignment pin verbatim). Quotes byte-verified at the pin this session. MIT.  FACE (count-neutral, Batch 407 / Plan 406 / Research 441, vLLM @ 34c653886 #61428, Apache-2.0 — byte-verified at the pin): the SIGNATURE-INTROSPECTION variant of the version gate — when the dependency's fix cannot be read as a version number, read the API ITSELF: vLLM's AITER fused-GDN availability probe runs `params = inspect.signature(fused_gdn_decode_qkvz).parameters` and requires `'qkvz_layout' in params and 'query_start_loc' in params and 'out' in params`, with the version provenance in the comment verbatim: '# qkvz_layout, query_start_loc and out are added in ROCm/aiter#6488.' (vllm/_aiter_ops.py:2914-2917) — capability detection by parameter introspection, the known-wrong window keyed on WHICH PARAMETERS EXIST instead of which version string; a missing parameter name refuses the fast path exactly as the version gate does, and the probe composes with this rule (the introspection is the version-blind encoding of the same refusal).Batch 404 / Plan 403 / Research 438.".to_string(),
            ref_shape: r#"// NAIVE: the fast path admits on capability alone — the version
// window where the backend computes WRONG gradients is invisible,
// and one shape class (kv_len % 128 != 0) silently trains on
// garbage while every parity test at kv_len % 128 == 0 stays green.
fn windowed_backward(
    version: u32, kv_len: usize, up: &[f32],
) -> Result<Vec<f32>, String> {
    if version >= 90000 {
        // the fast path, blind to the known-wrong window: gradients
        // are WRONG below 91501 whenever kv_len % 128 != 0
        let mut g = vec![0.0f32; up.len()];
        for (i, u) in up.iter().enumerate() {
            g[i] = u * 2.0 + wrongness(kv_len); // the bug term
        }
        return Ok(g);
    }
    Err("no fast path".to_string())
}

fn wrongness(kv_len: usize) -> f32 {
    if kv_len % 128 != 0 { 1.0 } else { 0.0 } // the bug class
}"#.to_string(),
            opt_pattern: r#"// FIXED: the version gate refuses the whole fast path below the
// pinned-wrong version, naming the bug class; the fallback stays
// exact. The refusal carries WHAT breaks and the fix version.
const PINNED_WRONG_BELOW: u32 = 91501;
const FIX_VERSION: u32 = 91501;

fn windowed_backward_gated(
    version: u32, kv_len: usize, up: &[f32],
) -> Result<Vec<f32>, String> {
    if version < PINNED_WRONG_BELOW {
        // the conservative landed form: refuse below the pin even
        // where this kv_len's modulus cannot hit — the fix is
        // upstream at FIX_VERSION, never re-derived per shape
        return Err(format!(
            "window backward wrong gradients when kv % 128 != 0 below {FIX_VERSION}; upgrade or fall back"
        ));
    }
    let mut g = vec![0.0f32; up.len()];
    for (i, u) in up.iter().enumerate() {
        g[i] = u * 2.0; // exact: no bug term below the version pin
    }
    Ok(g)
}

// the fallback the refusal names: the composition path stays exact
fn windowed_backward_ref(up: &[f32]) -> Vec<f32> {
    up.iter().map(|u| u * 2.0).collect()
}"#.to_string(),
        },
        // ── R4 — the VJP dispatch (B−): valid-but-ungeneric gradient
        // directions route to the composition, never throw. ──
        KernelRule {
            rule_id: "fused-vjp-ungeneric-direction-falls-back-not-throw".to_string(),
            gpu_concept: "MECHANISM (the fused-backward direction dispatch; category D — dispatch/admission): a fused autodiff path that computes gradients for a SUBSET of inputs routes the remaining directions to its composition fallback — never throws — because the direction is VALID (the gradient is well-defined); only the fast path is narrow. The source's before-state: the fused SDPA VJP threw `invalid_argument` — \"[scale_dot_product_attention] Does not support VJP with respect to mask or attention sinks.\" — for any argnum >= 3, reachable from a plain vjp over ALL inputs, silently disabling correct gradients (and every nested grad-of-grad through the fused path) for mask/sink directions. The fix, verbatim `// The fused backward computes only the gradients of q, k and v.` — `bool other_grads = std::any_of(argnums.begin(), argnums.end(), [](int arg) { return arg >= 3; }); if (outputs.size() != 2 || other_grads) { return Custom::vjp(...); }` — the composition computes ALL directions via the reference op, and the fused primitive keeps only the q/k/v directions it can express. THE LATENT-INDEX BUG FIX rides the same stroke and is the rule's second half: the fallback closure read the sinks as `inputs.back()` — but the primitive's input tuple had grown to primals + output + logsumexp + cotangent, so `back()` had silently become the COTANGENT; the fix pins `inputs[primals_size - 1]`. The law: when a fast path's input tuple grows, every positional read inside it re-derives its index from the element's ROLE (primals_size − 1 for the last primal), never from the tuple's end — `back()` is a role claim that a tuple growth silently reassigns. NEVER: throw on a valid direction a fallback could compute (the throw is reachable from ordinary composed autodiff — the user's program dies computing a well-defined answer); refuse when you mean fall back (REFUSE is for invalid inputs — block-scaled-operand-declared-form-refusal-ladder's class; FALL BACK is for valid inputs the fast path cannot express — this rule's class; the two polarities must never blur); grow an input tuple without re-deriving every positional index from its role; route directions through a fallback that is itself wrong for them (verify the composition before it becomes the answer). ⛔ KINSHIP (adjudicated both directions): block-scaled-operand-declared-form-refusal-ladder (B) owns the typed-refusal pole for INVALID declared forms — THIS owns the fallback pole for VALID-but-ungeneric directions; the discriminator is input validity, stated in-rule both ways; fail-stop-deadline-trap-bounded-cross-rank-wait (E) owns the fail-stop pole for UNRECOVERABLE states (a co-resident wait that cannot degrade) — a mask gradient is RECOVERABLE by composition, the opposite regime; backward-as-conjugate (E) owns the backward-as-math-trick axis — THIS owns the backward-as-dispatch axis (which paths compute which directions); command-buffer-declared-input-pacing (D) is the declared-input sibling — same category, different lever (commit pacing vs gradient coverage). REGIME: custom autodiff primitives (vjp/jvp over multi-input ops), fused kernels with subset gradient support, any fast path with a composition-equivalent fallback behind it. ANTI-REGIME: no fallback exists (then the typed refusal IS the correct answer — name it and throw); the composition is wrong or unbounded for the direction; the direction request is genuinely invalid (out of range — keep the throw, typed). GOAT: the fixture pins (a) the fused path answering directions 0..3 while direction 3 routes to the composition — all four gradients exact, (b) the throw arm red (a plain vjp over all inputs panics), (c) the index arm: a grown input tuple read by back() returns the cotangent instead of the sinks (red), the role-offset read returns the sinks (green), (d) the all-fallback disclosure arm (correct but slower — the cost is counted, never hidden). P113: MERGED upstream at the pin — decision content only. Source: ml-explore/mlx @ 58d91ee81a86dd9710123ec42e7ff91f6d1d21b1 (PR #4927, MIT) — mlx/fast.cpp:1024-1040 (other_grads + the fallback slice, comments verbatim), the removed throw site in ScaledDotProductAttention::vjp; mlx/backend/cuda/scaled_dot_product_attention.cpp:782-785 (inputs[primals_size - 1], the sinks read). Quotes byte-verified at the pin this session. MIT. Batch 404 / Plan 403 / Research 438.".to_string(),
            ref_shape: r#"// NAIVE: the fused backward throws on the directions it cannot
// express — a plain vjp over ALL inputs (the ordinary composed
// case) panics computing a well-defined answer. And the fallback
// reads the last primal by `back()`, a role claim a tuple growth
// silently reassigns to the cotangent.
fn fused_vjp_n dirs: &[usize] -> Vec<f32> {
    let _ = dirs;
    unreachable!("set up below")
}

// the throw, verbatim class:
fn vjp_dirs(dirs: &[usize]) -> Result<Vec<f32>, String> {
    for &d in dirs {
        if d >= 3 {
            return Err("Does not support VJP with respect to \
                        mask or attention sinks.".to_string());
        }
    }
    Ok(dirs.iter().map(|_| 1.0).collect())
}

// the latent index bug: back() after the tuple grew
fn sinks_from_back(inputs: &[&str]) -> &str {
    inputs.last().copied().unwrap() // cotangent, NOT the sinks
}"#.to_string(),
            opt_pattern: r#"// FIXED: direction coverage is a dispatch — the fused path answers
// its expressible subset, everything else routes to the composition;
// positional reads derive from the element's ROLE, never the end.
const FUSED_DIRECTIONS: usize = 3; // q, k, v

fn vjp_dispatch(dirs: &[usize], fused: &[f32; 4], composed: &[f32; 4]) -> Vec<f32> {
    let other_grads = dirs.iter().any(|&d| d >= FUSED_DIRECTIONS);
    if other_grads {
        // valid directions the fast path cannot express: the
        // composition answers ALL of them, exactly
        return FUSED_DIRECTIONS..4
            .map(|i| composed[i])
            .chain(dirs.iter().filter(|&&d| d < 3).map(|&d| fused[d]))
            .collect();
    }
    dirs.iter().map(|&d| fused[d]).collect()
}

// the role-derived read: the last primal, not the tuple's back
fn sinks_by_role(inputs: &[&str], primals_size: usize) -> &str {
    inputs[primals_size - 1] // sinks: a role, stable under growth
}

// the tuple grew (output + logsumexp + cotangent appended) — the
// role read survives, back() would now name the cotangent
const PRIMALS_SIZE: usize = 3;
let inputs = ["q", "k", "v", "out", "lse", "d_out"];
assert_eq!(sinks_by_role(&inputs, PRIMALS_SIZE), "v");
assert_ne!(inputs.last().copied().unwrap(), "v"); // back() is the cotangent"#.to_string(),
        },
    ]
}
