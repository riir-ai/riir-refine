//! Entries slice — Batches 14-46 era (rules 287-324 of 340) — riir-gpu production-kernel mining.
//! Pure data moved verbatim from the former corpus.rs monolith
//! (2026-08-15 split); append new batches to the LAST gpu slice or
//! open a new one past ~900 lines. Category membership lives ONLY in
//! `KERNEL_CATEGORIES` (mod.rs) — never encode it here.

use crate::domains::kernel_opt::KernelRule;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<KernelRule> {
    vec![
        // ── identical-tensor-handle-clone-zero-reupload — absent V ≡ K (A) ──
        // Source: riir-gpu gemma4_q4k_train/weight_buffers.rs L229-232
        // ("// V is optional (may share K). Fall back to K's blocks." →
        // "let attn_wv = match gguf_ref.tensor_info(&ln.attn_v) {
        // Some(_) => upload_q4k_gguf(...)?, None => attn_wk.clone(), };")
        // + the backward analog L410-415 (wv_t = wk_t.clone()) +
        // gemv_q4k_cubecl.rs L794-796 (Q4KHandle is #[derive(Clone)] —
        // the clone is a refcount bump on the two device buffers).
        //
        // Distinct from twin-projection-shared-delta-adapter (B31 — two
        // DISTINCT tensors with shape-twin projections share one LoRA
        // ADAPTER: compute sharing; here the SAME tensor is referenced
        // twice and the share is the device BUFFER: memory sharing,
        // absence-detected), pool-buffer-aliasing-dedicated-output (B21 —
        // forces a dedicated output to dodge an aliasing hazard: the
        // opposite direction).
        KernelRule {
            rule_id: "identical-tensor-handle-clone-zero-reupload".to_string(),
            gpu_concept: "when a source format legally OMITS a tensor because it is IDENTICAL to a sibling (GGUF exports that skip attn_v when it shares attn_k), the load path must detect the absence and CLONE the sibling's device handle — a refcount bump — never re-read the sibling's bytes and upload a second copy. The share is the device BUFFER itself: zero re-upload, zero duplicate device memory, and the aliasing is detected by the tensor's ABSENCE in the source. Distinct from twin-projection-shared-delta-adapter (two DISTINCT tensors with shape-twin projections sharing one LoRA ADAPTER — compute sharing; here it is the SAME tensor referenced twice — memory sharing) and from pool-buffer-aliasing-dedicated-output (an output-aliasing hazard workaround — the opposite direction: forcing a dedicated output, not exploiting an identity).".to_string(),
            ref_shape: "// naive: V absent from the export == identical to K, but the loader\n// re-reads K's bytes and uploads a SECOND device copy anyway\nfn load_kv_naive(gguf: &Gguf, names: &Names, kv_dim: usize,\n                 n: usize, client: &Client) -> (Handle, Handle) {\n    let wk = upload(client, gguf, &names.attn_k, kv_dim, n);\n    let wv = upload(client, gguf, &names.attn_k, kv_dim, n); // duplicate\n    (wk, wv) // bytes + duplicate device memory for the same tensor\n}".to_string(),
            opt_pattern: "// optimized: absence means identity — clone the handle (refcount)\nfn load_kv_aliased(gguf: &Gguf, names: &Names, kv_dim: usize,\n                   n: usize, client: &Client) -> (Handle, Handle) {\n    let wk = upload(client, gguf, &names.attn_k, kv_dim, n);\n    let wv = match gguf.tensor_info(&names.attn_v) {\n        Some(_) => upload(client, gguf, &names.attn_v, kv_dim, n), // real\n        None => wk.clone(), // refcount bump: zero re-upload, zero\n    }; // duplicate device memory for the identical twin\n    (wk, wv)\n}".to_string(),
        },
        // ── sparse-gather-scale-at-read-not-table-prepass — fold at the gather (B) ──
        // Source: riir-gpu gemma4_q4k_train/weight_buffers.rs L77-96
        // (WteLookup::row_scaled_into: "*o = v[off + i] * scale;" — the
        // multiply fused into the row-copy loop, BOTH variants) + the
        // shared-consumer proof L147-148 (wte_q4k is quantized from the
        // UNSCALED weights.wte BEFORE the Owned clone — pre-scaling the
        // table would need a permanent second ~4 GB host copy) + hot-path
        // callsites (mod.rs L1102-1103 per position; forward_batched.rs
        // L216 inlines the same multiply on the Owned fast path).
        //
        // Distinct from constant-scale-folded-into-upload-copy (B36 — the
        // INVERSE regime: dense hot-path operand + sole consumer → fold
        // at the upload copy, never the shared source; here sparse read
        // set + shared unscaled consumer → the gather is the ONLY correct
        // fold point) and fold-norm-scalar-into-matmul (B14 — folds INTO
        // weights: exactly the full-table pre-pass this rule refuses).
        KernelRule {
            rule_id: "sparse-gather-scale-at-read-not-table-prepass".to_string(),
            gpu_concept: "when a constant scale must meet gathered rows (an embedding lookup scaled by sqrt(n)), the fold point is decided by READ-SET DENSITY and by OTHER CONSUMERS of the table: sparse reads (seq_len rows of vocab — ~4096x fewer elements) plus a shared UNSCALED consumer (the lm_head quantized from the raw table before it is cloned) select the GATHER loop as the only correct fold — the multiply rides a copy that must happen anyway (the FMA is free next to the memory movement), while pre-scaling would either add a full-table pass or force a permanent second host copy. This is the deliberately INVERSE regime of constant-scale-folded-into-upload-copy (dense hot-path operand with a sole consumer — fold at the upload copy, never the shared source) and the refusal of fold-norm-scalar-into-matmul's full-table pre-pass (which folds INTO weights — exactly the dense regime).".to_string(),
            ref_shape: "// naive: full-table scale pre-pass to serve a sparse read set —\n// and the unscaled table is still needed by the lm_head quantization\nfn embed_prepass_then_copy(table: &[f32], scale: f32, tokens: &[usize],\n                           hidden: &mut [f32], n: usize) {\n    let mut scaled: Vec<f32> = table.iter().map(|v| v * scale).collect();\n    for (pos, &tok) in tokens.iter().enumerate() { // vocab*n scaled to\n        let off = tok * n; // serve a seq_len*n read set (+ a second\n        hidden[pos * n..pos * n + n] // host copy of the table)\n            .copy_from_slice(&scaled[off..off + n]);\n    }\n}".to_string(),
            opt_pattern: "// optimized: fold the scale into the sparse gather itself — the\n// multiply rides the copy that must happen anyway\nfn embed_scale_at_gather(table: &[f32], scale: f32, tokens: &[usize],\n                         hidden: &mut [f32], n: usize) {\n    for (pos, &tok) in tokens.iter().enumerate() {\n        let off = tok * n;\n        let row = &mut hidden[pos * n..pos * n + n];\n        for (i, o) in row.iter_mut().enumerate() {\n            *o = table[off + i] * scale; // seq_len*n multiplies; the\n        } // table stays UNSCALED for the lm_head consumer\n    }\n}".to_string(),
        },
        // ── tiled-host-transpose-before-quantize — cache-block the transpose (A) ──
        // Source: riir-gpu gemma4_q4k_train/weight_buffers.rs L460-466
        // (transpose_quantize_upload: the naive loop — "let mut
        // transposed = vec![0.0f32; rows * cols]; for r in 0..rows {
        // for c in 0..cols { transposed[c * rows + r] = data[r * cols
        // + c]; } }" — inner-c store strides by rows*4 B = 15,360 B at
        // n_embd=3840). Runs once per tensor × ~337 tensors per backward
        // build (7×48 layers + wte), the largest tensor 221 MB.
        //
        // Distinct from pre-transposed-consumer-layout-handles-build-once
        // (B34 — the residency DECISION that a transposed handle must
        // exist; this file already ships it — this rule is the transpose
        // ALGORITHM that runs under that decision), layout-permute-for-
        // coalescing (B8/M — GPU-side layout choice, not the host loop).
        KernelRule {
            rule_id: "tiled-host-transpose-before-quantize".to_string(),
            gpu_concept: "a host-side transpose feeding a quantize+upload pipeline must be cache-BLOCKED (tiled), not naive-indexed: the naive inner loop stores with stride rows*4 B (15,360 B at n_embd=3840), so every store lands on a cold line and each 64 B destination line is write-allocated 16 separate times; 64x64 tiling makes both the load stream and the store stream sequential within a hot tile window. This is the transpose ALGORITHM that runs UNDER pre-transposed-consumer-layout-handles-build-once (the residency DECISION that mandates a transposed handle at build time — a different axis), and the host-side sibling of layout-permute-for-coalescing (GPU-side layout choice). At one transpose per tensor x ~337 tensors per backward-handles build, the naive loop is seconds of pure line-traffic waste.".to_string(),
            ref_shape: "// naive: inner-loop store strides by rows*4 B — every store a cold\n// line; each 64 B destination line written 16 separate times\nfn transpose_naive(data: &[f32], rows: usize, cols: usize) -> Vec<f32> {\n    let mut t = vec![0.0f32; rows * cols];\n    for r in 0..rows {\n        for c in 0..cols {\n            t[c * rows + r] = data[r * cols + c];\n        }\n    }\n    t\n}".to_string(),
            opt_pattern: "// optimized: 64x64 cache-blocked transpose — loads AND stores walk\n// a hot tile window instead of striding the whole matrix\nfn transpose_tiled(data: &[f32], rows: usize, cols: usize) -> Vec<f32> {\n    let mut t = vec![0.0f32; rows * cols];\n    const TILE: usize = 64;\n    for r0 in (0..rows).step_by(TILE) {\n        for c0 in (0..cols).step_by(TILE) {\n            let r_hi = (r0 + TILE).min(rows);\n            let c_hi = (c0 + TILE).min(cols);\n            for r in r0..r_hi {\n                for c in c0..c_hi {\n                    t[c * rows + r] = data[r * cols + c];\n                }\n            }\n        }\n    }\n    t\n}".to_string(),
        },
        // ── materialized-fast-path-bulk-slice-accessor — branch once per step (D) ──
        // Source: riir-gpu gemma4_q4k_train/weight_buffers.rs L100-105
        // (WteLookup::owned_slice: "pub fn owned_slice(&self) ->
        // Option<&[f32]> { match self { WteLookup::Owned(v) => Some(v),
        // WteLookup::GgufBacked { .. } => None } }") + the consumer
        // forward_batched.rs L208-227 (ONE per-step branch → the Owned
        // arm runs scratch.hidden.par_chunks_mut(n) direct indexing; the
        // GgufBacked arm falls back to per-row row_scaled_into).
        //
        // Distinct from static-classification-hoist-to-routing-table
        // (B32b — hoists to CONSTRUCTION; impossible here: the
        // representation is a per-load-path property and the hoist is
        // legitimately per-step), enum-dispatch-separate-kernel-per-
        // variant (B29 — GPU kernel specialization; this is the host API
        // shape that unlocks zero per-row dispatch + rayon).
        KernelRule {
            rule_id: "materialized-fast-path-bulk-slice-accessor".to_string(),
            gpu_concept: "when an enum wraps a materialized-vs-backed representation (an owned f32 table vs an mmap-backed lazy-row variant), the hot path must branch ONCE PER STEP on a bulk zero-copy accessor (Option<&[f32]>) and run a representation-specialized loop body — direct indexing + parallel chunking over the destination on the materialized arm — instead of dispatching every row through the method that internally matches the enum. The accessor shape is what makes the fast path both zero-dispatch and rayon-parallelizable. Distinct from static-classification-hoist-to-routing-table (hoists classification to CONSTRUCTION — impossible here: the representation is a per-load-path property and the branch is legitimately per-step) and from enum-dispatch-separate-kernel-per-variant (GPU kernel specialization per variant — this is the host API shape).".to_string(),
            ref_shape: "// naive: per-row method dispatch — the enum is matched and the\n// method called for EVERY row even when the table is materialized\nfn embed_via_method(lookup: &WteLookup, tokens: &[usize], scale: f32,\n                    hidden: &mut [f32], n: usize) {\n    for (pos, &tok) in tokens.iter().enumerate() {\n        let row = &mut hidden[pos * n..pos * n + n];\n        lookup.row_scaled_into(tok, scale, row); // per-row match + call\n    }\n}".to_string(),
            opt_pattern: "// optimized: ONE branch per step on a bulk zero-copy accessor; the\n// materialized arm runs a specialized direct-index + parallel loop\nfn embed_specialized(lookup: &WteLookup, tokens: &[usize], scale: f32,\n                     hidden: &mut [f32], n: usize) {\n    if let Some(slice) = lookup.owned_slice() {\n        hidden.par_chunks_mut(n).enumerate().for_each(|(pos, row)| {\n            let off = tokens[pos] * n;\n            for (i, o) in row.iter_mut().enumerate() {\n                *o = slice[off + i] * scale; // direct indexing + rayon\n            }\n        });\n    } else {\n        for (pos, &tok) in tokens.iter().enumerate() {\n            let row = &mut hidden[pos * n..pos * n + n];\n            lookup.row_scaled_into(tok, scale, row); // backed fallback\n        }\n    }\n}".to_string(),
        },
        // ── env-flag-resolved-once-not-per-dispatch — per-dispatch env lookup → resolve once (D) ──
        // Source: riir-gpu gemma4_q4k_train/forward_batched.rs L382+L385
        // (RAYON_FWD + FUSED_RMSNORM read INSIDE the per-layer fn = 48x/step
        // each; RAYON_FWD read AGAIN at L203 in the outer fn) +
        // gemv_autotune.rs L121-131 (the GEMV_FORCE_TILED env check executes
        // BEFORE the L130 papaya lock-free cache hit, once per autotuned
        // dispatch) + kimi_k3_gpu_forward.rs L1861-1872/L1938-1948 (OURO_*
        // per token per layer) + the correct live-toggle form at
        // gemma2_cubecl/mod.rs L1353-1358 (cheap layer/pos guard ordered
        // BEFORE the env read so the lookup fires only in the narrow window).
        //
        // Distinct from predicate-gated-backend (WHICH kernel is safe for a
        // shape — this is WHERE the decision input is resolved), from
        // derived-scalar-host-computed-in-uniform (a config-derived NUMERIC
        // constant computed once — this is a path-selection boolean resolved
        // once), and from the B36-F5 falsification note (killed "an env var
        // selects path A/B" as rule CONTENT; this rule's content is
        // resolution timing + per-dispatch cost).
        KernelRule {
            rule_id: "env-flag-resolved-once-not-per-dispatch".to_string(),
            gpu_concept: "A runtime path-selection flag read from the environment (a rayon on/off switch, a fused-kernel experiment toggle, a diagnostic force-variant) is CONFIG, not data: it cannot meaningfully change mid-step, so resolve it ONCE — a bool/enum field on the orchestrator filled at construction, or a OnceLock<bool> for a process-lifetime flag — never per dispatch, per layer, or per token. std::env::var takes the global environment lock and heap-allocates a String per lookup; on the dispatch path that cost multiplies by sites x layers x positions (97 lookups/step in one trainer), and one instance sits IN FRONT of a deliberately lock-free papaya cache hit in the autotuner — the env lock poisons the exact property the cache was chosen for. When a flag genuinely must stay live-togglable mid-run (a trace switch), order the CHEAP guard before the env read so the lookup fires only inside the narrow window (layer == 0 && pos <= 1 && env::var(..) is the correct live-toggle form; env-before-cache is the anti-form). Distinct from predicate-gated-backend (WHICH kernel is safe for a shape — this is WHERE the decision input is resolved), from derived-scalar-host-computed-in-uniform (a config-derived NUMERIC constant computed once — this is a path-selection boolean resolved once), and from the B36-F5 falsification note (that killed an env-var A/B gate as rule CONTENT — this rule's content is resolution timing + per-dispatch cost).".to_string(),
            ref_shape: "// naive: the flag is re-resolved on the hot path — env lock + String\n// alloc per lookup, multiplied by layers/sites/tokens\nstruct TrainerNaive { /* ... */ }\nimpl TrainerNaive {\n    fn forward_layer(&mut self, layer: usize, hidden: &mut [f32]) {\n        let use_rayon = std::env::var(\"RAYON_FWD\").as_deref() != Ok(\"0\"); // 48x/step\n        if use_rayon { /* parallel path */ } else { /* sequential path */ }\n    }\n}\nfn select_variant(cache: &AutotuneCache, m: usize, n: usize) -> Variant {\n    if std::env::var(\"FORCE_VARIANT\").as_deref() == Ok(\"1\") {\n        return Variant::Tiled; // EVERY dispatch — in front of the lock-free\n    }                          // papaya cache hit the design chose\n    if let Some(v) = cache.pin().get(&(m, n)) { return *v; }\n    Variant::Plane /* slow path */\n}".to_string(),
            opt_pattern: "// optimized: flags are CONFIG — resolved once at construction (or OnceLock\n// for process-lifetime); live-togglable flags guard the env read narrowly\nstruct Trainer {\n    use_rayon: bool,\n    fused_rmsnorm: bool,\n}\nimpl Trainer {\n    fn new() -> Self {\n        Self {\n            use_rayon: std::env::var(\"RAYON_FWD\").as_deref() != Ok(\"0\"), // ONCE\n            fused_rmsnorm: std::env::var(\"FUSED_RMSNORM\").as_deref() == Ok(\"1\"),\n        }\n    }\n    fn forward_layer(&mut self, layer: usize, hidden: &mut [f32]) {\n        if self.use_rayon { /* parallel path */ } else { /* sequential path */ }\n    }\n}\nstatic FORCE_VARIANT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();\nfn select_variant_fast(m: usize, n: usize, force: &std::sync::OnceLock<bool>,\n                       cache: &AutotuneCache) -> Variant {\n    if *force.get_or_init(|| std::env::var(\"FORCE_VARIANT\").as_deref() == Ok(\"1\")) {\n        return Variant::Tiled; // first call resolves; the rest are a bool read\n    }\n    if let Some(v) = cache.pin().get(&(m, n)) { return *v; } // lock-free hit\n    Variant::Plane\n}\n// genuinely live toggle: cheap guard FIRST, env read only inside the window\nfn maybe_trace(layer: usize, pos: usize) {\n    if layer == 0 && pos <= 1 && std::env::var(\"TRACE\").is_ok() { /* emit */ }\n}".to_string(),
        },
        // ── flat-activation-slots-as-readback-target — nested slots force scratch + scatter → flat slots ARE the target (A) ──
        // Source: riir-gpu gemma4_q4k_train/forward_batched.rs — the ~15
        // per-position save sweeps into nested [pos][..dim] slots (L348-351
        // x_input, L424-427+L441-444 normed_attn, L525-531 q/k/v_pre_norm,
        // L567-572 q/k_pre_rope, L586-589 v_cached, L622-625 q_post_rope,
        // L726-729 attn_out, L758-761 x_after_wo, L787-790 x_after_attn,
        // L812-815 normed_mlp, L871-875 gate/up, L897-900 hidden, L929-932
        // x_after_down, L299-303 logits) + mod.rs L753-755 (the batched
        // dispatch's readback lands in a FLAT caller slice, which nested
        // [pos] -> Vec<f32> slots cannot BE — the forcing cause).
        //
        // Distinct from preallocated-activation-slots-inplace-save (pre-sizes
        // NESTED slots and keeps the per-position bounded memcpy — its
        // optimized form retains the nesting and therefore the post-readback
        // scatter; this rule's fix is the flatness that deletes the second
        // pass), from per-layer-strided-grad-destination (backward-side
        // consumer-selected grad destinations — this is the forward-side
        // activation mirror), and from batched-downloads-single-sync-cycle
        // (readback SYNC mechanics — this is the DESTINATION layout those
        // reads write into).
        KernelRule {
            rule_id: "flat-activation-slots-as-readback-target".to_string(),
            gpu_concept: "Activation containers shaped as nested per-position Vecs ([pos] -> Vec<f32>) force every batched GPU producer into a two-copy tail: the dispatch helper's readback lands in a flat caller slice, then a per-position save sweep scatters that slice into the nested slots — ~15 such sweeps per layer in one trainer, each a full seq x dim memcpy. Flattening the container to row-major [seq x dim] with stride indexing makes the activation slot itself the readback target: the dispatch writes the slot sub-slice directly, deleting the scratch intermediary AND the scatter sweep. The container layout IS the copy count. Honest cost note: when the container is shared cross-crate with a CPU reference path and the backward, flattening is a coordinated refactor — the rule fixes the destination contract, not one call site. Distinct from preallocated-activation-slots-inplace-save (pre-sizes NESTED slots and keeps the per-position bounded memcpy — its optimized form retains the nesting and therefore the post-readback scatter; this rule's fix is the flatness that deletes the second pass), from per-layer-strided-grad-destination (backward-side consumer-selected grad destinations — this is the forward-side activation mirror), and from batched-downloads-single-sync-cycle (readback SYNC mechanics — this is the DESTINATION layout those reads write into).".to_string(),
            ref_shape: "// naive: nested activation slots cannot be a readback target — every\n// batched producer pays scratch + scatter (2 copies per saved tensor)\nfn save_layer(act: &mut LayerAct, scratch: &[f32], seq: usize, dim: usize) {\n    for pos in 0..seq { // copy 2: flat scratch ->\n        act.q_pre_norm[pos][..dim] // NESTED per-pos Vecs\n            .copy_from_slice(&scratch[pos * dim..pos * dim + dim]);\n    } // x15 save sweeps per layer; the nesting forces every one\n}".to_string(),
            opt_pattern: "// optimized: activation slots are FLAT [seq x dim] row-major — the slot\n// sub-slice IS the dispatch output buffer; one readback, zero scatters\nstruct LayerActFlat { q_pre_norm: Vec<f32> /* [seq * q_dim], pre-allocated */ }\nimpl LayerActFlat {\n    fn q_pre_norm_slot(&mut self, seq: usize, q_dim: usize) -> &mut [f32] {\n        &mut self.q_pre_norm[..seq * q_dim] // contiguous — a legal GEMV target\n    }\n}\nfn layer_forward(gpu: &Gpu, w: &Handle, input: &[f32], act: &mut LayerActFlat,\n                 seq: usize, q_dim: usize) {\n    // the readback writes the activation slot DIRECTLY — no scratch, no sweep\n    gpu.dispatch_gemv_batched_into(w, input, act.q_pre_norm_slot(seq, q_dim), seq);\n}".to_string(),
        },
        // ── transient-spike-pins-persistent-scratch — one-shot oversized phase routes around the reused scratch (A) ──
        // Source: riir-gpu gemma4_q4k_train/backward.rs L409 (Phase-1-ONLY
        // scratch_in.resize(batch * vocab, 0.0) ~= 134 MB at vocab 262144)
        // vs L557 (batch * mlp * 2 = the Phase-2 recurring max ~16 MB) +
        // L750; bw_scratch_in is a persistent RefCell<Vec<f32>> field
        // (mod.rs L377, constructed Vec::new() at L426, never shrunk) — the
        // transient Phase-1 spike pins its capacity ~8x above the recurring
        // watermark for the trainer's lifetime.
        //
        // Distinct from superset-sized-scratch-across-variants (sizes UP to
        // cover recurring variants — this forbids a NON-recurring consumer
        // from setting the superset) and from staging-reuse-grow-only-map-
        // prefix (grow-only is the mechanism being poisoned — this rule
        // decides who is allowed to grow it).
        KernelRule {
            rule_id: "transient-spike-pins-persistent-scratch".to_string(),
            gpu_concept: "A persistent, reused scratch buffer's capacity is pinned at its historical maximum — so a one-shot phase that needs an order-of-magnitude larger buffer must NOT route through it. A multi-phase pipeline whose phases are dimensioned by different quantities (one by vocab, the rest by hidden/mlp) otherwise retains the transient peak for the lifetime of the object: one resize(batch x vocab) ~= 134 MB pins an ~8x over-retention (~16 MB recurring max) on a trainer that lives for the whole run. Grow-only reuse is the right policy for RECURRING sizes and exactly the wrong policy for a non-recurring spike — the fix is caller discipline: transient oversized phases use a local buffer dropped at phase end; the persistent scratch's watermark is set only by the recurring maximum. Distinct from superset-sized-scratch-across-variants (sizes UP to cover recurring variants — this forbids a NON-recurring consumer from setting the superset) and from staging-reuse-grow-only-map-prefix (grow-only is the mechanism being poisoned — this rule decides who is allowed to grow it).".to_string(),
            ref_shape: "// naive: Phase 1 (one-shot) routes its batch*vocab buffer through the\n// persistent scratch reused by every later phase\nstruct Trainer { scratch: Vec<f32> } // persistent, reused across layers\nfn backward(t: &mut Trainer, batch: usize, vocab: usize, mlp: usize) {\n    t.scratch.resize(batch * vocab, 0.0); // 134MB — ONCE, but pins capacity\n    phase1_lm_head(&mut t.scratch);\n    for _layer in 0..48 {\n        t.scratch.resize(batch * mlp * 2, 0.0); // 17MB recurring — but the\n        phase2_layer(&mut t.scratch); // 134MB capacity is retained\n    } // forever: ~117MB dead RSS\n}".to_string(),
            opt_pattern: "// optimized: transient oversized phases use a LOCAL buffer; the persistent\n// scratch's watermark is set only by the RECURRING maximum\nfn backward(t: &mut Trainer, batch: usize, vocab: usize, mlp: usize) {\n    let mut logits_batch = vec![0.0f32; batch * vocab]; // local — dropped at\n    phase1_lm_head(&mut logits_batch); // phase end\n    for _layer in 0..48 {\n        t.scratch.resize(batch * mlp * 2, 0.0); // recurring max only:\n        phase2_layer(&mut t.scratch); // persistent footprint stays ~17MB\n    }\n}".to_string(),
        },
        // ── transposed-consumption-accumulate-not-gather — k-outer column gather → i-outer row accumulation (A) ──
        // Source: riir-gpu gemma4_q4k_train/backward.rs L923-930 (the LoRA
        // z-pass: sum += lora.b[i * rank + k] * gi with k OUTER — column
        // access at stride rank*4B) vs the forward's row-major consumer of
        // the SAME B layout at forward_batched.rs L1013-1018 (b_row = &lora_b
        // [i*rank..(i+1)*rank] then a sequential zip dot); the identical
        // strided shape duplicated at mod.rs L695-701 (per-position path).
        //
        // Distinct from pre-transposed-consumer-layout-handles-build-once
        // (materializes a transposed copy once at build — this is a
        // zero-copy loop reorder) and from tiled-host-transpose-before-
        // quantize (a materialized transpose ALGORITHM feeding a kernel —
        // here no transpose exists at all).
        KernelRule {
            rule_id: "transposed-consumption-accumulate-not-gather".to_string(),
            gpu_concept: "When a host-side backward needs z = B^T g against a matrix stored row-major [out_dim x rank] (the layout the forward's row-dot consumes), the naive k-outer loop gathers columns at stride rank x 4B — at rank >= 16 every 64B cache line contributes 4 useful bytes (up to 16x read amplification, and the whole B matrix is re-streamed once per k). Interchange the loops: zero the z row, then i-outer accumulate z_row += b_row x g[i] — same flops, B streams sequentially exactly as the forward does. The tell is the forward/backward asymmetry on the SAME buffer: one consumer streams rows while the sibling strided-gathers columns. Bounded fix-space: loop interchange only (no transposed copy — that is the heavier materialize-once option). Caveat: interchange changes floating-point summation order — fine for gradient accumulation, NOT for bit-identity gates. Distinct from pre-transposed-consumer-layout-handles-build-once (materializes a transposed copy once at build; this is zero-copy loop reorder) and from tiled-host-transpose-before-quantize (a materialized transpose algorithm feeding a kernel — here no transpose exists at all).".to_string(),
            ref_shape: "// naive: z[k] = sum_i B[i,k]*g[i] with k OUTER — column gather at stride rank\nfn lora_z_gather(b: &[f32], g: &[f32], out_dim: usize, rank: usize,\n                 z: &mut [f32]) {\n    for k in 0..rank { // for each k, walk ALL of B's rows\n        let mut sum = 0.0;\n        for i in 0..out_dim {\n            sum += b[i * rank + k] * g[i]; // stride rank*4B: 256B at rank 64\n        } // — 4 useful bytes per 64B line\n        z[k] = sum;\n    }\n}".to_string(),
            opt_pattern: "// optimized: i-outer row accumulation — B streams row-major, same as the\n// forward's dot; z built by axpy into a zeroed row\nfn lora_z_accumulate(b: &[f32], g: &[f32], out_dim: usize, rank: usize,\n                     z_row: &mut [f32]) {\n    z_row[..rank].fill(0.0);\n    for i in 0..out_dim {\n        let gi = g[i];\n        let b_row = &b[i * rank..(i + 1) * rank]; // sequential, cache-line dense\n        for (z, &bv) in z_row.iter_mut().zip(b_row) {\n            *z += bv * gi;\n        }\n    } // same flops; summation order differs (fine for grads, not bit-gates)\n}".to_string(),
        },
        // ── autotune-cache-extract-outlives-owner — salvage the tuning cache across owner destruction (K) ──
        // Source: riir-gpu kimi_k3_gpu_forward.rs L296-L302 (the training-loop
        // constructor doc: re-benchmarking every unique GEMV shape costs
        // ~5-10 min at d=2560; the weight upload is only ~64 ms),
        // L324-L329 (the Arc inject contract: all per-layer kernels share ONE
        // cache instance; reconstructed-per-step owners extract via
        // into_autotune + pass to the next constructor), L344-L346/L352-L354/
        // L393-L395 (the three per-layer kernel constructors each take
        // Arc::clone(&gemv_autotune)), + L1735-L1743 (into_autotune consumes
        // self and returns the Arc).
        //
        // Distinct from hot-swap-weights-preserve-autotune-caches (B25 keeps a
        // persistent object alive while mutating weights; here the owner is
        // DESTROYED per step and the cache is salvaged/transplanted across
        // lifetimes) and from share-pipeline-set-across-subsystems (B30 shares
        // compile artifacts within one run; this shares across owner
        // lifetimes via an inject/extract pair).
        KernelRule {
            rule_id: "autotune-cache-extract-outlives-owner".to_string(),
            gpu_concept: "When a struct owning a GPU pipeline is destroyed and reconstructed every training step, the expensive benchmark-derived state (autotune variant selections — minutes of re-benchmarking) must not live and die with the owner. Wrap the tuning cache in an Arc, inject it through a new_with_* constructor, share ONE instance across all sub-kernels, and provide a consuming into_*() extractor so the caller can salvage the cache at drop time and transplant it into the next construction. The upload is cheap (~64 ms); the re-tuning is not (~5-10 min). This is the ownership-lifecycle complement to hot-swap: instead of keeping the object alive to preserve its caches, decouple the cache lifetime from the object lifetime. Distinct from hot-swap-weights-preserve-autotune-caches (keeps a persistent object alive while mutating weights; here the owner is destroyed per step and the cache is salvaged across lifetimes) and from share-pipeline-set-across-subsystems (shares compile artifacts within one run; this shares across owner lifetimes via an inject/extract pair).".to_string(),
            ref_shape: "// naive: struct + its autotune cache reconstructed every training step —\n// every unique GEMV shape is re-benchmarked each time (~minutes)\nstruct Forwarder { autotune: GemvAutotune, client: Client }\nfn train_step(weights: &Weights, token: u32) -> Logits {\n    let mut fwd = Forwarder { autotune: GemvAutotune::new(), client: connect() };\n    fwd.upload_weights(weights);\n    fwd.forward_token(token) // cold autotune EVERY step\n}".to_string(),
            opt_pattern: "// optimized: the cache is Arc-shared, injected, + extractable — its\n// lifetime is decoupled from the owner's\nstruct Forwarder { autotune: Arc<GemvAutotune>, client: Client }\nimpl Forwarder {\n    fn new_with_autotune(cache: Arc<GemvAutotune>, w: &Weights) -> Self {\n        Self { autotune: cache, client: connect(), kernels: build(w, &cache) } // ONE instance\n    }\n    fn into_autotune(self) -> Arc<GemvAutotune> { self.autotune } // salvage at drop\n}\nfn train_step(prev: Arc<GemvAutotune>, w: &Weights, token: u32) -> (Logits, Arc<GemvAutotune>) {\n    let mut fwd = Forwarder::new_with_autotune(Arc::clone(&prev), w); // warm: no re-bench\n    let logits = fwd.forward_token(token);\n    (logits, fwd.into_autotune()) // transplant into the next step\n}".to_string(),
        },
        // ── append-cache-rewind-overwrite-in-place — re-execution overwrites, never re-appends (A) ──
        // Source: riir-gpu kimi_k3_gpu_forward.rs L2061-L2066 (rewind_mla_caches_to
        // doc: "Used by the Ouro looped forward (Plan 324) before each
        // iteration t >= 1, so the forward overwrites the cache entry at
        // position pos instead of appending a new entry. Without this, the
        // cache grows T-x per token. KDA layers are not affected (recurrent
        // state, no append-only cache).") + L2068-L2073 (the impl: only the
        // Mla variant's cache rewinds; Kda is a structural no-op) +
        // L2084-L2089 (the caller contract: set_hidden -> rewind ->
        // forward_token_hidden_saved per iteration).
        //
        // Distinct from kv-window-device-resident-ring-append (B37 — DEVICE
        // RESIDENCY of the ring buffer; this is the LOGICAL pointer rollback
        // for re-execution) and from mirror-ring-store-sliding-window-
        // contiguity (B34 — LAYOUT contiguity for windows; this is cache
        // EXTENT management).
        KernelRule {
            rule_id: "append-cache-rewind-overwrite-in-place".to_string(),
            gpu_concept: "An append-only autoregressive cache (KV cache) grows monotonically — which is wrong for re-execution workloads (looped/prediction-style passes over the same position, speculative-decode draft rejection, beam rollback). Expose a rewind_to(pos) that rolls the append pointer back so re-execution overwrites the existing slot instead of appending: memory stays O(max_seq) instead of O(iters x max_seq). The rewind is heterogeneous-aware: only append-only caches (KV) rewind; fixed-size recurrent-state layers (SSM/linear-attention state) are structurally unaffected no-ops — document which is which so callers do not try to rewind state that has no position axis. Distinct from kv-window-device-resident-ring-append (device residency of the ring buffer; this is the logical pointer rollback for re-execution) and from mirror-ring-store-sliding-window-contiguity (layout contiguity for windows; this is cache extent management).".to_string(),
            ref_shape: "// naive: append-only KV cache under a looped forward — every iteration\n// appends a fresh slot at pos; cache grows iters-x per token\nfn looped_pass(fwd: &mut Forward, hidden: &[f32], iters: usize, pos: usize) -> Logits {\n    let mut saved = Saved::default();\n    for t in 0..iters {\n        fwd.set_hidden(hidden);\n        fwd.forward_token_saved(pos as u32, &mut saved); // APPENDS at pos each t\n    } // cache len ~= iters * max_seq — memory exhaustion\n    saved.final_logits\n}".to_string(),
            opt_pattern: "// optimized: roll the append pointer back — re-execution OVERWRITES the\n// slot; cache stays O(max_seq)\nfn looped_pass(fwd: &mut Forward, hidden: &[f32], iters: usize, pos: usize) -> Logits {\n    let mut saved = Saved::default();\n    for t in 0..iters {\n        fwd.set_hidden(hidden);\n        fwd.rewind_mla_caches_to(pos); // append pointer -> pos; recurrent-state layers are no-ops\n        fwd.forward_token_hidden_saved(pos, t, &mut saved); // OVERWRITES slot pos\n    } // cache len stays max_seq\n    saved.final_logits\n}".to_string(),
        },
        // ── slow-path-bootstraps-raw-buffer-hotswap-cache — boot harvests, fast path writes in place (A) ──
        // Source: riir-gpu kimi_k3_gpu_forward.rs L1551-L1558 (the two-tier doc:
        // first call delegates to update_weights (boot allocation) +
        // populates weight_cache by extracting wgpu::Buffer references via
        // client.get_resource; subsequent calls write directly into the
        // cached buffers via queue.write_buffer, bypassing CubeCL's per-call
        // allocation), L1585-L1627 (the boot: update_weights + build the
        // slot vec + extract_buffer lazy->eager), L1630-L1647 (the fast path:
        // write_in_place[_parallel] per slot), + L1650-L1653 ("The cache was
        // built with the same iteration order (layer-major, gate->up->down
        // per dense layer), so we mirror it here").
        //
        // Distinct from hot-swap-weights-preserve-autotune-caches (B25 —
        // preserves the TUNING caches; this preserves the raw device buffers
        // themselves, eliminating allocation churn), from
        // weight-handle-cache-keyed-by-allocation-identity (B29 — pointer-keys
        // UNCHANGED CPU weights for dedup; training weights change every step,
        // so stable slot ORDER replaces keying), and from
        // cross-runtime-raw-handle-extraction (B21 — the extraction MECHANIC;
        // this rule is the two-tier bootstrap + order-keyed replaceable slots
        // built on it).
        KernelRule {
            rule_id: "slow-path-bootstraps-raw-buffer-hotswap-cache".to_string(),
            gpu_concept: "For per-step weight updates (training), allocate-and-replace handle swapping churns the device allocator on every optimizer step. Land the in-place path as a two-tier method: the FIRST call delegates to the existing allocate-and-replace path (boot), then harvests the raw native buffers out of the runtime handles (lazy-to-eager extraction) into a persistent slot cache keyed by stable construction order; SUBSEQUENT calls write new bytes directly into the cached raw buffers via the raw queue (write_buffer), zero allocation. In training the CPU weight buffers change identity every step, so pointer/key-based dedup caching is useless — the stable slot ORDER is the key, and the fast path must mirror the build loop's iteration order exactly (add a length/dtype assert so a build-loop reorder fails loudly instead of writing weights into the WRONG buffers silently). Distinct from hot-swap-weights-preserve-autotune-caches (preserves tuning caches; this preserves the raw device buffers themselves), from weight-handle-cache-keyed-by-allocation-identity (pointer-keys unchanged weights for dedup; changing weights need order-keyed replaceable slots), and from cross-runtime-raw-handle-extraction (the extraction mechanic this rule builds its two-tier policy on).".to_string(),
            ref_shape: "// naive: allocate-and-replace EVERY optimizer step — the device allocator\n// churns a fresh buffer per weight per step\nimpl Forward {\n    fn update_weights(&mut self, w: &Weights) {\n        self.w_a = self.client.create_from_slice(bytes(&w.a)); // fresh device alloc\n        self.w_b = self.client.create_from_slice(bytes(&w.b)); // + old buffer freed\n    } // x N weights x steps — allocator churn all run long\n}".to_string(),
            opt_pattern: "// optimized: first call BOOT-harvests the raw buffers into order-keyed\n// slots; later calls write new bytes in place — zero allocation\nimpl Forward {\n    fn update_in_place(&mut self, w: &Weights) -> Result<(), String> {\n        if self.slots.is_none() {\n            self.update_weights(w); // boot: slow path allocates\n            let mut slots = self.handles_in_build_order().map(Slot::from_handle).collect::<Vec<_>>();\n            for s in &mut slots { s.extract_buffer(&self.client)?; } // lazy -> eager\n            self.slots = Some(slots); // slot ORDER is the key\n            return Ok(());\n        }\n        let queue = self.raw_queue();\n        let slots = self.slots.as_mut().unwrap();\n        slots[0].write_in_place(&self.client, &queue, &w.lm_head); // zero-alloc overwrite\n        for (i, data) in w.matrices_in_build_order().enumerate() { // MUST mirror build order\n            slots[i + 1].write_in_place(&self.client, &queue, data);\n        }\n        Ok(())\n    }\n}".to_string(),
        },
        // ── tiny-param-buffer-rebuild-over-cache — below a cache line, rebuild beats caching (A) ──
        // Source: riir-gpu kimi_k3_gpu_forward.rs L2437-L2441 (the fused SiTU
        // doc: "The 3-element [beta, linear_beta, has_linear_beta] buffer is
        // constructed per call (3 f32 = 12 bytes — cheaper to rebuild than to
        // cache a Handle)") + L2473-L2476 + L2539-L2542 (both fused FFN
        // variants rebuild the params buffer per call from fresh scalars —
        // no Option<Handle> state, nothing to invalidate).
        //
        // Distinct from identical-uniforms-share-single-slot (B27 — share one
        // slot when writes are identical AND large enough to matter; this is
        // the threshold BELOW which even sharing loses) and from
        // pack-scalar-params-into-combined-buffer (B29 — packing many scalars
        // to fit an Array budget; this is the rebuild-vs-cache POLICY
        // decision).
        KernelRule {
            rule_id: "tiny-param-buffer-rebuild-over-cache".to_string(),
            gpu_concept: "Caching a device buffer for a kernel's scalar-parameter block (a handful of f32s) costs more than the upload it saves: the Option<Handle> state, the invalidation bookkeeping, and the lifetime plumbing exceed a ~12-byte create_from_slice. Below roughly a cache line or two, rebuild the params buffer per call from fresh scalars and state the threshold in a comment so a future 'optimization' does not cache it. This is the size-threshold complement to the sharing/packing rules: share when identical-and-large, pack when count-bound, rebuild when tiny. Distinct from identical-uniforms-share-single-slot (share when identical and large enough to matter; this is the threshold below which even sharing loses) and from pack-scalar-params-into-combined-buffer (packing many scalars to fit an Array budget; this is the rebuild-vs-cache policy decision).".to_string(),
            ref_shape: "// naive: cache EVERY param buffer, however tiny — state + invalidation\n// exceed the upload saved\nstruct SituKernel { params_h: Option<Handle> }\nimpl SituKernel {\n    fn launch(&mut self, beta: f32, lb: Option<f32>) {\n        if self.params_h.is_none() {\n            let p = [beta, lb.unwrap_or(1.0), flag(lb)]; // beta varies per config\n            self.params_h = Some(self.client.create_from_slice(bytes(&p)));\n        } // miss path + stale-cache trap: a config change silently reuses old bytes\n        dispatch(self.params_h.clone());\n    }\n}".to_string(),
            opt_pattern: "// optimized: 3 f32 = 12 bytes — rebuild per call; nothing to invalidate\nimpl SituKernel {\n    fn launch(&self, beta: f32, lb: Option<f32>) {\n        let has_lb = if lb.is_some() { 1.0f32 } else { 0.0f32 };\n        let lb = lb.unwrap_or(1.0); // unused when has_lb == 0.0\n        let params = [beta, lb, has_lb];\n        let params_h = self.client.create_from_slice(bytes(&params)); // 12B upload\n        dispatch(params_h); // stateless launch: no cache to invalidate\n    }\n}".to_string(),
        },
        // ── readback-destination-passing-free-fn — dst: &mut [f32], not -> Vec<f32> (A) ──
        // Source: riir-gpu kimi_k3_gpu_forward.rs L807-L828 (read_handle_into:
        // the destination-passing free fn — "Free function (not a method) so
        // the caller can split the borrow: pass &self.client + &mut
        // self.hidden without locking the whole self"; feature-gated + single
        // handle only) vs the still-live naive shapes in the SAME file:
        // L505-L523 (dispatch_gemv returns f32::from_bytes(&bytes).to_vec()),
        // L613-L614/L662-L663/L763-L764/L914-L915 (read_batched(...).remove(0)
        // then self.logits.copy_from_slice — fresh vocab-sized Vec + copy per
        // token), + gemv_batched.rs L96-L105 (read_batched maps every handle
        // to a fresh Vec) / L110-L112 (read_handle — the single-handle
        // convenience wrapper with the same churn).
        //
        // Distinct from flat-activation-slots-as-readback-target (B38 — fixes
        // the destination CONTAINER layout so the slot is the readback
        // target; this fixes the HELPER SIGNATURE to destination-passing),
        // from staging-reuse-grow-only-map-prefix (B31 — staging-buffer reuse
        // at the map level, below this API), and from
        // batched-downloads-single-sync-cycle (B26 — sync batching,
        // orthogonal: an into-variant can still batch).
        KernelRule {
            rule_id: "readback-destination-passing-free-fn".to_string(),
            gpu_concept: "A GPU-to-host readback helper should be destination-passing — fn(client, handle, dst: &mut [f32]) — not value-returning — fn(client, handle) -> Vec<f32>. The value-returning shape forces every read to allocate a fresh output Vec, and when the caller owns a persistent buffer (logits, hidden, scratch) it additionally forces a second copy out of the fresh Vec. The destination-passing shape eliminates both — and for terminal vocab-sized drains (the LM head logits, ~640 KB per token) that is a fresh allocation + full copy deleted per token. Critically, the helper should be a FREE function over the context pointer, not a method — so callers can pass &ctx and &mut self.dst simultaneously without locking the whole orchestration struct (a method takes &self/&mut self and re-collapses the borrow). Batched readers should expose an into variant too, so per-token terminal drains do not allocate + copy a Vec every token. Distinct from flat-activation-slots-as-readback-target (fixes the destination container layout; this fixes the helper signature), from staging-reuse-grow-only-map-prefix (staging reuse below this API), and from batched-downloads-single-sync-cycle (sync batching, orthogonal: an into-variant can still batch).".to_string(),
            ref_shape: "// naive: value-returning readback — every read allocates a fresh Vec,\n// + a second copy when the caller owns a persistent buffer\nfn read_handle(client: &Client, h: Handle) -> Vec<f32> {\n    let bytes = client.read_one(h).unwrap();\n    f32::from_bytes(&bytes).to_vec() // fresh Vec per read\n}\nfn forward(model: &mut Model, client: &Client, w: &Handle) {\n    let logits = dispatch_gemv(client, w, &model.hidden, model.vocab); // -> Vec\n    model.logits.copy_from_slice(&logits); // copy 2: fresh Vec -> persistent buffer\n}".to_string(),
            opt_pattern: "// optimized: destination-passing FREE FN — zero fresh Vecs, zero second\n// copies; the borrow splits so caller + dst coexist\nfn read_handle_into(client: &Client, h: Handle, dst: &mut [f32]) {\n    let bytes = client.read_one(h).unwrap();\n    dst.copy_from_slice(f32::from_bytes(&bytes)); // direct into the persistent buffer\n}\nfn read_batched_into(client: &Client, handles: Vec<Handle>, dsts: &mut [&mut [f32]]) {\n    let bufs = client.read(handles); // still ONE sync for the whole batch\n    for (b, d) in bufs.into_iter().zip(dsts.iter_mut()) {\n        d.copy_from_slice(f32::from_bytes(&b));\n    }\n}\nfn forward(model: &mut Model, client: &Client, w: &Handle) {\n    let h = launch_gemv(client, w, &model.hidden, model.vocab);\n    read_handle_into(client, h, &mut model.logits); // one read, one copy, zero allocs\n}".to_string(),
        },
        // ── variant-owned-output-buffer-clone-escape — match-arm borrows force clones; hoist + split (A) ──
        // Source: riir-gpu kimi_k3_gpu_forward.rs — the enum-variant output
        // buffers (GpuFfn::Dense { out_buf, ... } at L109-L129) forced out of
        // the match arm because the result must outlive the &mut
        // self.ffn_layers[idx] borrow: out_buf.clone() at L1069/L1192/L1304/
        // L1416 (+ (out_buf.clone(), None, Some(ds)) at L1979/L2240) and
        // fresh vec![0.0; d] per MoE arm at L1072/L1195/L1307/L1419 — one
        // heap alloc per layer per token across ALL forward variants. The
        // consumer forcing the escape: L1201-L1204 (prefix-sum needs &mut
        // self.hidden while ffn_out borrows self.ffn_layers). The file's own
        // half-fix: L2308-L2315 ("Standalone helpers (avoid borrow-checker
        // conflicts with &mut self.ffn_layers)... Extracted from the method
        // so the caller can hold &mut self.ffn_layers[idx] while dispatching
        // GEMVs through &self.client + &self.gemv_autotune") — the free fns
        // split the client borrow but the OUTPUT still escapes by clone
        // because out_buf is owned by the VARIANT.
        //
        // Merged from BOTH mining agents (independently proposed as the
        // free-fn borrow split + the ownership-placement halves of one fix).
        //
        // Distinct from caller-owned-split-scratch (B31 — callee-side scratch
        // carving via split_at_mut with a borrowed-view return; this is the
        // caller-side ownership placement vs enum variants) and from
        // needless-swap-temp-hoist-to-scratch (B32b — deleting a
        // false-premise temp).
        KernelRule {
            rule_id: "variant-owned-output-buffer-clone-escape".to_string(),
            gpu_concept: "When a per-layer op is dispatched by matching on &mut self.layers[idx] (an enum of kernel variants), any output buffer owned INSIDE the variant cannot be referenced after the match arm ends — the result must outlive the &mut self.layers borrow because the caller subsequently mutates sibling state (the hidden accumulator). The common escape is materialization: out_buf.clone() per Dense arm + a fresh vec![0.0; d] per MoE arm — a heap allocation per layer per token, purely to satisfy the borrow checker. The fix is ownership placement + borrow splitting: hoist the output buffer to the parent struct (a persistent ffn_out), dispatch via free functions over the DISJOINT fields (client, autotune, the variant borrow, the caller-owned out buffer) so everything coexists in one method — the arm writes into memory the caller can read independently, zero clones, zero fresh allocs. The tell that the fix is missing: a free-fn helper family extracted 'to avoid borrow-checker conflicts' while the match arms STILL clone their results out. Distinct from caller-owned-split-scratch (callee-side scratch carving with a borrowed-view return; this is caller-side ownership placement) and from needless-swap-temp-hoist-to-scratch (deleting a false-premise temp).".to_string(),
            ref_shape: "// naive: output buffer owned by the VARIANT — the result must outlive the\n// &mut self.ffn_layers[idx] borrow, so every arm materializes it\nfn forward_layer(model: &mut Model, idx: usize) {\n    let ffn_out = match &mut model.ffn_layers[idx] {\n        LayerFfn::Dense { w_gate, out_buf, .. } => {\n            run_ffn(&model.client, w_gate, out_buf);\n            out_buf.clone() // heap alloc per layer per token — borrow escape\n        }\n        LayerFfn::Moe { kernel } => {\n            let mut out = vec![0.0f32; model.d]; // fresh alloc per layer per token\n            kernel.run(&model.hidden, &mut out);\n            out\n        }\n    };\n    for (h, f) in model.hidden.iter_mut().zip(ffn_out.iter()) { *h += f; } // needs &mut hidden\n}".to_string(),
            opt_pattern: "// optimized: output buffer hoisted to the PARENT + free fn over the\n// disjoint fields — zero clones, zero fresh allocs\nstruct Model { ffn_layers: Vec<LayerFfn>, ffn_out: Vec<f32>, hidden: Vec<f32>, client: Client }\nfn dense_ffn(client: &Client, w: &Handle, out: &mut [f32]) {\n    launch_into(client, w, out); // writes IN PLACE into the caller-owned buffer\n}\nfn forward_layer(model: &mut Model, idx: usize) {\n    match &mut model.ffn_layers[idx] {\n        LayerFfn::Dense { w_gate, .. } => dense_ffn(&model.client, w_gate, &mut model.ffn_out),\n        LayerFfn::Moe { kernel } => kernel.run(&model.hidden, &mut model.ffn_out),\n    } // disjoint-field borrows: variant + client + ffn_out coexist\n    for (h, f) in model.hidden.iter_mut().zip(model.ffn_out.iter()) { *h += f; }\n}".to_string(),
        },
        // ── elementwise-offload-needs-device-side-anchor — a sandwiched GPU op is compute-offload ONLY (C) ──
        // Source: riir-gpu kimi_k3_gpu_forward.rs L969-L972 + L1011-L1014 (the
        // unanchored per-layer norms: upload scratch -> GPU RMSNorm -> read
        // back into the SAME buffer — 2 bus crossings replacing 0), L930-L935
        // (the file's own honest label: "This is the honest intermediate
        // state: the RMSNorm CPU compute is eliminated, but the attention/FFN
        // GEMV input still takes a CPU slice (future phases will make those
        // kernels accept Handles to eliminate that readback too)"), vs the
        // ANCHORED contrast at L737-L741 + L747-L762 (fused final norm feeds
        // the LM head GEMV directly on device — "This eliminates 1 sync point
        // per token"), + L2492 (launch_gemv_handle — the handle-input
        // consumer entry that closes the chain later without re-plumbing).
        //
        // Found independently by BOTH mining agents (convergent discovery).
        //
        // Distinct from hybrid-cpu-elementwise-gpu-throughput-sync-points
        // (B34 — the division-of-labor policy; this is the payoff condition
        // for the exception), from kernelize-host-chain-after-sync-batching-
        // floor (B35 — when to fuse a CHAIN of host ops; this is the
        // single-op guard), and from chain-dispatches-single-sync-point
        // (B19 — the chaining mechanic; this is the migration accounting
        // rule: which migrations actually reduce the sync count).
        KernelRule {
            rule_id: "elementwise-offload-needs-device-side-anchor".to_string(),
            gpu_concept: "In a hybrid CPU/GPU pipeline, migrating one elementwise op to GPU removes a host round trip ONLY when at least one adjacent consumer stays device-side — the offloaded op's output must feed another GPU dispatch directly (a device-side anchor), so the offload forms a handle chain. A GPU op sandwiched between an upload and a readback (upload slice, GPU kernel, read back to the same slice) keeps the sync count identical: it is compute-offload only — 2 bus crossings replacing 0 while saving only CPU compute, a net loss whenever the round trip exceeds the elementwise cost. Ship such intermediate states deliberately and label them honestly in the doc ('the honest intermediate state') instead of claiming a sync win — and provide the handle-input variant of the consumer op (launch_gemv_handle beside launch_gemv) so the chain can close later without re-plumbing. Distinct from hybrid-cpu-elementwise-gpu-throughput-sync-points (the division-of-labor policy; this is the payoff condition for the exception), from kernelize-host-chain-after-sync-batching-floor (when to fuse a chain of host ops; this is the single-op guard), and from chain-dispatches-single-sync-point (the chaining mechanic; this is the migration accounting rule).".to_string(),
            ref_shape: "// naive: migrate ONE op to GPU, neighbors stay host-side — book it as a\n// win. It isn't: sync count UNCHANGED, 2 bus crossings ADDED\nfn layer(&mut self, x: &mut [f32]) {\n    let xh = self.upload(x);                 // host -> device\n    let normed = self.gpu_rmsnorm(xh);       // device compute\n    self.read_into(normed, x);               // device -> host (sync unchanged!)\n    let attn = self.attn_from_slice(x);      // consumer takes &[f32]\n}".to_string(),
            opt_pattern: "// optimized: offload only when a device-side ANCHOR consumes the output\nfn layer(&mut self, x: &[f32]) -> Vec<f32> {\n    let xh = self.upload(x);\n    let normed = self.gpu_rmsnorm(xh);          // handle -> handle\n    let out_h = self.launch_gemv_handle(&self.w_attn, normed); // ANCHOR: consumer takes the Handle\n    self.read_one(out_h)                        // the ONE sync for the whole chain\n}\n// until the consumer accepts handles, keep the op on CPU — or fuse it\n// into the adjacent GEMV so its output never crosses back".to_string(),
        },
        // ── entrypoint-variants-share-layer-body — versioned entry points, ONE shared layer body (D) ──
        // Source: riir-gpu kimi_k3_gpu_forward.rs L566-L576 (forward_token_batched
        // doc: "Produces bit-identical results... The sync reduction (7->3 for
        // MLA, 8->3 for KDA, 3->2 for Dense FFN, ~12->4 for MoE)"),
        // L619-L627 (batched_v2: "near-identical... < 1e-5 relative" — each
        // variant's doc states its parity class), + the two failure modes:
        // L977-L983/L986-L992 (Issue 401 Phase 2.1: the batched kernels "were
        // already shipped by Issue 396 but was not wired into the resident
        // forward" — a fork initially shipped calling the OLD kernels) and
        // the saved-forward divergence: forward_token_saved L1862-L1896
        // dispatches on OURO_* env vars while its ~310-line clone
        // forward_token_hidden_saved L2162-L2177/L2220 calls the batched
        // kernel unconditionally — same model, same saved contract, different
        // kernel selection by entry point.
        //
        // Distinct from shape-kind-dispatch / heuristic-compile-dispatch /
        // op-family-type-dispatch (runtime SELECTION among impls inside one
        // entry point; this is the AUTHORING discipline for API-surface
        // evolution) and from hardcoded-fast-path-jit-specialization
        // (kernel-internal fast path; this is host API structure).
        KernelRule {
            rule_id: "entrypoint-variants-share-layer-body".to_string(),
            gpu_concept: "Evolving a GPU pipeline's sync behavior lands as version-suffixed entry points (forward, ..._batched, ..._batched_v2, ..._fused_x, ..._resident) rather than in-place rewrites — each variant's doc quantifies the sync delta ('7 to 3') and states the parity class (bit-identical vs <1e-5 GPU-fp non-associativity), which is what makes A/B benches and regression gates possible. But the per-layer body must stay factored into ONE shared method per axis (forward_layer_gpu, forward_layer_gpu_batched, ...); forking the whole loop body per variant guarantees divergence — improvements shipped in one fork silently fail to reach the others (observed twice in one file: a resident fork that initially called the OLD unbatched kernels, and two saved forwards whose kernel-variant selection diverged under the same env vars). Bounded fix-space: entry points may multiply; loop bodies are shared per variant axis.".to_string(),
            ref_shape: "// naive: each new optimization forks the WHOLE loop body; the forks drift\npub fn forward_saved(&mut self, tok: u32, saved: &mut Saved) -> &[f32] {\n    for l in 0..self.n_layers {\n        self.saved_layer_body_v1(l, saved); // 150-line body, pasted\n    }\n    self.head()\n}\npub fn forward_saved_hidden(&mut self, tok: u32, saved: &mut Saved) -> &[f32] {\n    for l in 0..self.n_layers {\n        self.saved_layer_body_v1_copy(l, saved); // stale fork: misses the v2 fix\n    } // + silently selects a DIFFERENT kernel variant than its sibling\n    self.head()\n}".to_string(),
            opt_pattern: "// optimized: entry points multiply, loop bodies are SHARED per axis —\n// the variant axis owns only the sync strategy\n/// Sync: 3 (was 7). Bit-identical to [forward_token].\npub fn forward_token_batched(&mut self, tok: u32) -> &[f32] {\n    for l in 0..self.n_layers { self.forward_layer_gpu_batched(l); } // ONE body\n    self.head()\n}\n/// Sync: 2 (was 3). Parity < 1e-5 (GEMM vs GEMV fp non-associativity).\npub fn forward_token_batched_v2(&mut self, tok: u32) -> &[f32] {\n    for l in 0..self.n_layers { self.forward_layer_gpu_batched_v2(l); } // ONE body\n    self.head()\n}\n// saved entry points compose the SAME layer bodies + a save adapter —\n// kernel-variant selection can never diverge between siblings".to_string(),
        },
        // ── pipeline-layer-boundary-device-handle — the Handle IS the pipeline currency (D) ──
        // Source: riir-gpu kimi_k3_gpu_forward.rs L937 (the boundary signature:
        // fn forward_layer_gpu_resident(&mut self, hidden_handle: Handle,
        // layer_idx: usize) -> Handle), L871-L874 (the loop threading the
        // handle: hidden_handle = self.forward_layer_gpu_resident(
        // hidden_handle, layer_idx)), L944-L945 + L1086-L1087 (the internal
        // read/upload at CPU-consume/produce sites — host materialization is
        // an INTERNAL detail of each layer), + L1018-L1024 (the payoff: the
        // Dense FFN later fused to ONE sync WITHOUT touching any layer
        // boundary or caller).
        //
        // Distinct from hybrid-cpu-elementwise-gpu-throughput-sync-points
        // (B34 — op placement POLICY cpu-vs-gpu within a layer; this is the
        // boundary TYPE between layers), from training-forward-shed-decode-
        // loop-shape (B36 — loop granularity, not boundary typing), from
        // chain-dispatches-single-sync-point (B19 — intra-command-buffer
        // chaining, below this level), and from side-consumer-readback-
        // alias-live-handle-for-dependent-dispatch (B35 — a side consumer +
        // main chain at ONE op; this is pipeline-wide currency).
        KernelRule {
            rule_id: "pipeline-layer-boundary-device-handle".to_string(),
            gpu_concept: "In a sequential multi-layer pipeline where each layer mixes GPU kernels with CPU glue ops, the inter-layer boundary signature should be fn layer(hidden: Handle, idx) -> Handle — the device handle is the pipeline currency, moved by value. Even when every layer still reads the activation back internally (CPU glue ops require it) and re-uploads at exit, this signature makes host materialization an INTERNAL detail of each layer rather than a property of the pipeline. That is the unlock: downstream kernels can later be migrated to consume handles (fusing norm + GEMV + activation + GEMV into one terminal sync) without touching any layer boundary or caller — observed when a 4-dispatch fused FFN landed inside one layer with ZERO boundary changes. If the boundary instead passes host slices/Vecs, every internal fusion attempt re-introduces the host round trip at the boundary and the migration never composes. The handle crossing the boundary costs a refcount bump; the host round trips it saves are per-layer, per-token. Distinct from hybrid-cpu-elementwise-gpu-throughput-sync-points (op placement policy within a layer; this is the boundary type between layers), from training-forward-shed-decode-loop-shape (loop granularity, not boundary typing), from chain-dispatches-single-sync-point (intra-command-buffer chaining, below this level), and from side-consumer-readback-alias-live-handle-for-dependent-dispatch (one op's side consumer; this is pipeline-wide currency).".to_string(),
            ref_shape: "// naive: the boundary passes HOST state — every internal fusion attempt\n// re-introduces the host round trip at the boundary\nfn forward_layer(&mut self, idx: usize) {\n    let normed = rmsnorm_cpu(&self.hidden, &self.gammas[idx]);\n    let attn_out = self.attn(idx, &normed); // slice in, Vec out\n    for (h, a) in self.hidden.iter_mut().zip(attn_out.iter()) { *h += a; }\n}\nfn forward_token(&mut self, token: u32) -> Vec<f32> {\n    self.hidden.copy_from_slice(&self.embed_row(token));\n    for idx in 0..self.n_layers { self.forward_layer(idx); } // host currency\n    self.lm_head(&self.hidden)\n}".to_string(),
            opt_pattern: "// optimized: the Handle IS the currency — host materialization becomes an\n// internal detail; later fusion composes WITHOUT boundary changes\nfn forward_layer_resident(&mut self, hidden_h: Handle, idx: usize) -> Handle {\n    read_handle_into(&self.client, hidden_h, &mut self.hidden); // internal: CPU glue needs it\n    let normed_h = self.dispatch_rmsnorm_gpu(upload(&self.hidden), &self.gamma_h[idx], self.d);\n    let fused_h = self.fused_ffn_handle(idx, normed_h); // fused later — boundary untouched\n    let out_h = launch_gemv_handle(&self.client, &self.w_down, fused_h, self.d);\n    read_handle_into(&self.client, out_h, &mut self.ffn_out);\n    self.accumulate_hidden();\n    self.client.create_from_slice(&self.hidden) // internal: re-upload at exit\n}\nfn forward_token(&mut self, token: u32) -> Vec<f32> {\n    let mut h = self.client.create_from_slice(&self.embed_row(token));\n    for idx in 0..self.n_layers { h = self.forward_layer_resident(h, idx); } // handle currency\n    read_handle(&self.client, self.lm_head_handle(h))\n}".to_string(),
        },
        // ── host-scratch-mirrors-slot-order-for-coalesced-writes — upload-side
        // coalescing enabler: the scratch layout IS the write-call count (A) ──
        //
        // Source: kimi_k3_gpu_backward.rs — POSITIVE instances: L109-L116 (the
        // dense `batch_scratch` doc: "Reusable CPU scratch holding ALL 3
        // transposed weights contiguously (Phase 9f). Enables
        // write_slots_batched to coalesce contiguous GPU buffer slots into the
        // minimum number of queue.write_buffer calls.") + L346-L365 (the
        // split_at_mut carve -> data_slices -> write_slots_batched), L1963-L1966
        // (MLA sizes batch_scratch to the SUM: `let total_elems: usize =
        // slot_lens.iter().sum();`) + L2033-L2035. ANTI-instance in the SAME
        // file: L973-L986 (MoE latent path) — `let max_len = rd.len().max(ru.len());`
        // then two sequential `cache[idx].write_in_place(...)` calls that can
        // never coalesce despite the sibling paths proving the pattern.
        //
        // Distinct from batched-downloads-single-sync-cycle (B26 — READ-side
        // sync count; this is WRITE-side call count), from
        // flat-activation-slots-as-readback-target (B38 — readback destination
        // container), from readback-destination-passing-free-fn (B39 — helper
        // signature).
        KernelRule {
            rule_id: "host-scratch-mirrors-slot-order-for-coalesced-writes".to_string(),
            gpu_concept: "When a per-step weight update writes N derived matrices (transposes) into N harvested device slots, size ONE persistent CPU scratch to the SUM of slot lengths — not the max — carve it with split_at_mut into sub-slices in cache slot order, transform directly into the sub-slices (no intermediate copy), and hand the slice array to the coalescing batched writer. The host scratch layout IS the write-call count: slot-ordered contiguity is what lets adjacent device buffers merge into one staging copy + one write_buffer per contiguous run. The anti-pattern is a max-sized scratch reused sequentially per slot: N transforms into the same buffer + N independent write calls that can never coalesce — observed as the latent-MoE path in the same file whose sibling paths 60 lines earlier ship the sum-carve. Distinct from batched-downloads-single-sync-cycle (B26 — the READ side sync count; this is the WRITE side call count), from flat-activation-slots-as-readback-target (B38 — the readback destination container; this is the upload source layout), and from readback-destination-passing-free-fn (B39 — the helper signature; this is the buffer layout policy that makes coalescing possible at all).".to_string(),
            ref_shape: "// naive: max-sized scratch reused sequentially — N transforms + N independent writes\nfn update(&mut self, cache: &mut Vec<WeightBufferSlot>, client: &Client, queue: &Queue) {\n    let max_len = self.w_down.len().max(self.w_up.len());\n    if self.scratch.len() < max_len { self.scratch.resize(max_len, 0.0); }\n    let mut idx = 0;\n    transpose_f32_into(&w_down, d_moe, d, &mut self.scratch);\n    cache[idx].write_in_place(client, queue, &self.scratch[..w_down.len()]); idx += 1;\n    transpose_f32_into(&w_up, d, d_moe, &mut self.scratch);\n    cache[idx].write_in_place(client, queue, &self.scratch[..w_up.len()]);\n    // two independent write calls — the two slots can never coalesce\n}".to_string(),
            opt_pattern: "// optimized: scratch = SUM of slot lengths, carved in slot order, ONE coalescing write\nfn update(&mut self, cache: &mut Vec<WeightBufferSlot>, client: &Client, queue: &Queue) {\n    let total: usize = cache.iter().map(|s| s.size_bytes / 4).sum();\n    if self.batch_scratch.len() < total { self.batch_scratch.resize(total, 0.0); }\n    let (down_slice, rest) = self.batch_scratch.split_at_mut(w_down.len());\n    let up_slice = &mut rest[..w_up.len()];\n    transpose_f32_into(&w_down, d_moe, d, down_slice);\n    transpose_f32_into(&w_up, d, d_moe, up_slice);\n    let data_slices: [&[f32]; 2] = [\n        &self.batch_scratch[..w_down.len()],\n        &self.batch_scratch[w_down.len()..w_down.len() + w_up.len()],\n    ];\n    write_slots_batched(cache, client, queue, &data_slices); // contiguous run merges\n}".to_string(),
        },
        // ── write-plan-derived-from-cache-slots-not-recomputed — the mirrored
        // enumeration is the drift surface; derive from the cache (A) ──
        //
        // Source: kimi_k3_gpu_backward.rs MLA update_from_cpu_in_place — the
        // boot cache pushes the w_g slot iff the HANDLE exists (L1907-L1909)
        // while the fast-path slot_lens includes it only if handle AND source
        // exist (L1952-L1954). The ONLY tripwire is L1961
        // `debug_assert_eq!(slot_lens.len(), n_slots, "MLA slot count mismatch")`
        // — compiled out in release, where the shifted writes land in the WRONG
        // buffers and the tail write goes out of bounds (the downstream
        // write_slots_batched check in weight_buffer_cache.rs L290 is ALSO a
        // debug_assert). Today's builders make the mismatch unreachable, but
        // the drift surface is the pattern, not the current reachability.
        //
        // Distinct from slow-path-bootstraps-raw-buffer-hotswap-cache (B39 —
        // PRESCRIBES the mirror-with-assert shape; this rule eliminates the
        // mirror), from derive-shape-from-payload-not-cached-field (B32b — the
        // inverse direction: payload truth over cached field).
        KernelRule {
            rule_id: "write-plan-derived-from-cache-slots-not-recomputed".to_string(),
            gpu_concept: "A bootstrapped update path has two natural enumerations of the same slot set: the boot loop that pushes WeightBufferSlots (guard: handle presence) and the fast path that recomputes lengths from config (guard: handle AND source presence — they drift). The mirrored hand-maintained list IS the bug factory: when the guards diverge, the only tripwire is a debug_assert compiled out in release — every slot after the drift point receives the previous slot's data (wrong-length bytes into the wrong buffer, stale tail) and the final write indexes out of bounds. The fix is structural: the slot cache already stores size_bytes per slot — derive the write plan (count, order, lengths) from the cache itself and assert only that the provided data matches. One enumeration, zero drift surface. Distinct from slow-path-bootstraps-raw-buffer-hotswap-cache (B39 — prescribes the two-tier bootstrap and says the fast path must mirror the build loop exactly, add a length assert; this rule says the prescribed mirror is itself the defect class and eliminates it by deriving from the cache) and from derive-shape-from-payload-not-cached-field (B32b — payload truth over a cached field; here the harvested cache IS the authoritative plan and the config-derived recomputation is the redundant copy).".to_string(),
            ref_shape: "// naive: two hand-maintained lists — boot pushes slots, fast path recomputes lens\nfn boot(&mut self, cache: &mut Vec<WeightBufferSlot>) {\n    if let Some(ref h) = self.w_g_t { cache.push(WeightBufferSlot::from_handle(h.clone(), d * v)); }\n    cache.push(WeightBufferSlot::from_handle(self.w_kr_t.clone(), r * d));\n}\nfn update(&mut self, cache: &mut Vec<WeightBufferSlot>, w: &Weights) {\n    let mut lens: Vec<usize> = vec![];\n    if self.w_g_t.is_some() && w.w_g.is_some() { lens.push(v * d); } // guard DRIFTED vs boot\n    lens.push(r * d);\n    debug_assert_eq!(lens.len(), cache.len()); // debug-only tripwire\n    write_plan(cache, &lens); // release: shifted writes + tail out-of-bounds\n}".to_string(),
            opt_pattern: "// optimized: the cache IS the plan — lengths derived from stored size_bytes\nfn update(&mut self, cache: &mut Vec<WeightBufferSlot>, w: &Weights) {\n    let lens: Vec<usize> = cache.iter().map(|s| s.size_bytes / 4).collect(); // ONE list\n    let total: usize = lens.iter().sum();\n    if self.scratch.len() < total { self.scratch.resize(total, 0.0); }\n    fill_scratch_in_slot_order(&mut self.scratch, &lens, w); // asserts data.len() matches\n    write_slots_batched(cache, client, queue, &scratch_slices(&self.scratch, &lens));\n}".to_string(),
        },
        // ── cpu-source-builder-skips-device-roundtrip — when the host owns the
        // source bytes, the *_from_cpu twin deletes the per-matrix syncs (A) ──
        //
        // Source: kimi_k3_gpu_backward.rs — the from_cpu family exists exactly
        // for this (L253-L256: "Identical to [build_dense_ffn_handles] except
        // it takes the CPU weight slices directly instead of reading them back
        // from GPU handles. This skips the synchronous client.read_one()
        // round-trip (one per weight matrix) that dominates the per-step
        // backward_handles.update() cost."). The GPU-source builders remain
        // un-batched: dense L225-L237 (3 sequential read_f32), MoE L1153-L1195
        // (3 x (N_routed + N_shared) + router + latent — ~195 sequential syncs
        // at the 63-expert config), MLA L2179-L2225, KDA L2965-L2994. The
        // trainer already routes per-step through from_cpu (kimi_k3_train.rs
        // L2320-L2331) — the GPU-source twins are boot-only.
        //
        // Distinct from batched-downloads-single-sync-cycle (B26 — N syncs -> 1;
        // this is N -> 0 when the host owns the bytes), from
        // verbatim-upload-on-format-match (B37 — format conversion skip),
        // from identical-tensor-handle-clone-zero-reupload (B37 — identical
        // tensors need no transform; derived artifacts do).
        KernelRule {
            rule_id: "cpu-source-builder-skips-device-roundtrip".to_string(),
            gpu_concept: "Every derived-handle builder that reads device handles back to CPU, transforms, and re-uploads pays one blocking read_one sync per matrix (~1.3 ms measured — dominates wall-clock at scale: thousands of GEMVs x per-matrix reads). When the caller holds the authoritative CPU weights — training is exactly this case, the optimizer just mutated them — ship a parallel *_from_cpu constructor that transposes + uploads directly from host slices and route the per-step update path through it; keep the GPU-source variant only for boot-from-device or no-CPU-copy consumers. This deletes the round-trips entirely: batching them (batched-downloads-single-sync-cycle) is the lesser fix — the host already owns the bytes, so the from_cpu path is the strictly better tier. Distinct from batched-downloads-single-sync-cycle (B26 — reduces N syncs to 1; this deletes all N when the host already owns the source), from verbatim-upload-on-format-match (B37 — skips a format conversion; this skips the readback itself), and from identical-tensor-handle-clone-zero-reupload (B37 — handle clone for byte-identical tensors; here the artifact is derived/transformed so a host transform is genuinely required).".to_string(),
            ref_shape: "// naive: read every GPU handle back, transform, re-upload — 1 blocking sync per matrix\nfn update(&mut self, client: &ComputeClient, w: &GpuWeights) -> Result<(), String> {\n    let gate_cpu = read_f32(client, &w.w_gate)?;  // client.read_one — ~1.3ms each\n    let up_cpu = read_f32(client, &w.w_up)?;\n    let down_cpu = read_f32(client, &w.w_down)?;\n    self.w_gate_t = transpose_and_upload(client, &gate_cpu)?;\n    self.w_up_t = transpose_and_upload(client, &up_cpu)?;\n    self.w_down_t = transpose_and_upload(client, &down_cpu)?;\n    Ok(())\n}".to_string(),
            opt_pattern: "// optimized: the host already owns the source — transform + upload directly, ZERO syncs\nfn update_from_cpu(&mut self, client: &ComputeClient, w: &CpuWeights) -> Result<(), String> {\n    self.w_gate_t = transpose_and_upload(client, w.w_gate)?;   // no read_one\n    self.w_up_t = transpose_and_upload(client, w.w_up)?;\n    self.w_down_t = transpose_and_upload(client, w.w_down)?;\n    Ok(())\n}\n// per-step: update_from_cpu(&optimizer_weights) — the GPU-source build_* runs at BOOT only".to_string(),
        },
        // ── expert-loop-two-phase-split-batch-across-iterations — the helper's
        // encapsulated syncs serialize independent experts (H) ──
        //
        // Source: kimi_k3_gpu_backward.rs moe_backward_gpu + expert_backward_into
        // — L1682-L1684 ("d_act = down_proj^T @ d_output (1 sync — needed for
        // CPU SiTU)" + the launch_gemv -> read_batched pair) and L1707-L1710
        // ("Issue 393: launch BOTH independent GEMVs, then read both in ONE
        // batched sync." — the WITHIN-iteration batching landed; the
        // cross-iteration dimension did not). Every expert's stage-A input is
        // loop-entry-ready: L1563-L1564 builds `d_expert_out` from
        // `d_latent_prenorm` computed BEFORE the loop (L1522-L1542). Per MoE
        // layer per token: shared 2 + k_r x 2 + routed_up 1 + routed_down 1
        // syncs; the two-phase split achieves ~3.
        //
        // Distinct from sibling-dispatches-before-any-readback (B36 — one code
        // point, not loop-carried), from op-major-backward-loop-inversion-
        // batch-sequence-axis (B34 — needs a FREE axis + converts GEMV->GEMM;
        // here only the stage halves are free), from
        // fan-out-shared-input-single-sync (B25 — shared upload, not loop
        // structure).
        KernelRule {
            rule_id: "expert-loop-two-phase-split-batch-across-iterations".to_string(),
            gpu_concept: "A per-iteration helper that encapsulates launch head-GEMV -> sync-read -> CPU transform -> launch tail-GEMVs -> sync-read hides cross-iteration independence: the reads inside iteration k block the launches of iteration k+1 even though every iteration's stage-A input is ready at loop entry (in the MoE case each expert's d_output is a scalar multiple of a vector computed BEFORE the loop). Restructure to a two-phase split: launch ALL stage-A GEMVs -> ONE batched read -> all CPU transforms (the outer products fill the read window per independent-cpu-work-fills-dispatch-read-window) -> launch ALL stage-B GEMVs -> ONE batched read. A shared + k_r routed experts + the routed up/down projections drop from 2 + 2*k_r + 2 syncs per layer per token to ~3. Distinct from sibling-dispatches-before-any-readback (B36 — siblings at ONE code point; here the serialization is loop-carried ACROSS iterations), from op-major-backward-loop-inversion-batch-sequence-axis (B34 — requires a free axis within the loop body AND converts GEMVs to GEMMs; here the axis is blocked mid-iteration by the CPU transform, only the stage halves are free, and the fix keeps GEMVs), and from fan-out-shared-input-single-sync (B25 — one shared upload fanning out; this is loop structure).".to_string(),
            ref_shape: "// naive: per-expert helper encapsulates launch -> read -> CPU -> launch -> read\nfor k in 0..k_routed {\n    let d_act_h = launch_gemv(client, tune, &h[k].w_down_t, &d_out[k], d_ffn, d);\n    let d_act = read_batched(client, vec![d_act_h]).remove(0); // sync blocks expert k+1\n    let (d_gate, d_up) = situ_backward(&d_act, &saved[k]);\n    let g_h = launch_gemv(client, tune, &h[k].w_gate_t, &d_gate, d, d_ffn);\n    let u_h = launch_gemv(client, tune, &h[k].w_up_t, &d_up, d, d_ffn);\n    let d_h = read_batched(client, vec![g_h, u_h]);            // sync blocks expert k+1\n    accumulate_dh(d_h, &mut d_h_acc);\n}".to_string(),
            opt_pattern: "// optimized: split at the CPU-dependency point, batch each half across ALL experts\nlet mut stage_a = Vec::with_capacity(k_routed);\nfor k in 0..k_routed {\n    stage_a.push(launch_gemv(client, tune, &h[k].w_down_t, &d_out[k], d_ffn, d));\n}\nlet d_acts = read_batched(client, stage_a);                   // ONE sync for all k\nlet mut cpu_out = Vec::with_capacity(k_routed);\nfor k in 0..k_routed {\n    cpu_out.push(situ_backward(&d_acts[k], &saved[k]));       // fills the read window\n    outer_products(&d_out[k], &saved[k], &mut grads[k]);      // independent CPU work\n}\nlet mut stage_b = Vec::with_capacity(2 * k_routed);\nfor k in 0..k_routed {\n    stage_b.push(launch_gemv(client, tune, &h[k].w_gate_t, &cpu_out[k].0, d, d_ffn));\n    stage_b.push(launch_gemv(client, tune, &h[k].w_up_t, &cpu_out[k].1, d, d_ffn));\n}\nlet d_hs = read_batched(client, stage_b);                     // ONE sync for all k\nfor k in 0..k_routed { accumulate_dh(&d_hs[2 * k..2 * k + 2], &mut d_h_acc); }".to_string(),
        },
        // ── rmsnorm-backward-gamma-scope-correction-term — gamma scopes to the
        // direct term, never the normalization correction (N) ──
        //
        // Source: kimi_k3_gpu_backward.rs — WRONG (production MoE latent norm
        // backward): L1537 `dx[i] = norm_w[i] * r * (d_latent_postnorm[i] -
        // x_i * r2 * dot * inv_d_moe);` (gamma outside the parenthesis). RIGHT
        // (same file, the shared helper): L2839 `dx[i] = r * (gamma[i] *
        // d_y[i] - x_raw[i] * r2 * dot * inv_n);`. The workspace's own
        // katgpt-attn mla_backward.rs L681-L684 names the exact incorrect
        // form: "NOTE: gamma multiplies ONLY d_y[k], NOT the dot term. The
        // common incorrect form `gamma * r * (d_y - x*r2*dot*inv_n)`
        // over-applies gamma to the normalization correction." The slip lived
        // in the batched twin at mining time (kimi_k3_gpu_backward_batched.rs
        // L407) — the fix has since landed at exactly that line (verified
        // Batch 51, corrective comment + gamma scoped to the dy term only) —
        // and in the CPU reference (katgpt-transformer moe_backward.rs L627) —
        // masked by moe_backward_grad_check.rs tolerance 1.5e-2 vs gamma init
        // 1.0 +/- 0.1. Verified by finite difference: gamma=(1,2), x=(1,1),
        // dy=(1,0), n=2 -> true dx2 = -0.5, the slipped form gives -1.0.
        //
        // Distinct from exact-op-parity-for-analytic-backward (B34 — the
        // forward/backward op axis), from
        // saturating-nonlinearity-recovery-breaks-near-cap (B35 — numeric
        // saturation, not symbolic scope).
        KernelRule {
            rule_id: "rmsnorm-backward-gamma-scope-correction-term".to_string(),
            gpu_concept: "For RMSNorm y_i = gamma_i * x_i * r with r = 1/rms(x), the input gradient is dx_i = r * (gamma_i * dy_i - x_i * r2 * dot / n) with dot = sum_j gamma_j * x_j * dy_j: gamma multiplies ONLY the direct dy term — the normalization correction term must NOT carry gamma. The common slip factors gamma outside the whole parenthesis: gamma_i * r * (dy_i - x_i * r2 * dot / n), an error of (gamma_i - 1) * correction that is invisible when gamma initializes near 1 (typical 1.0 +/- 0.1) under percent-level grad-check tolerances — so it ships silently and corrupts every downstream gradient fed by the norm's dx. The same workspace usually holds a correct sibling implementation (diff them); gate with finite-difference checks at the widest affordable gamma range (0.5 to 2.0), where the slip is unmistakable. Found live-wrong in a production MoE backward while the SAME FILE carried the correct form sixty lines away in a helper for a different norm. Distinct from exact-op-parity-for-analytic-backward (B34 — forward/backward op identity; this is backward-internal algebra scope) and from saturating-nonlinearity-recovery-breaks-near-cap (B35 — an f32 saturation boundary; this is symbolic scope, exact at every magnitude).".to_string(),
            ref_shape: "// naive: gamma factored outside the WHOLE parenthesis — spurious on the correction\nfn moe_latent_norm_backward(x: &[f32], dy: &[f32], gamma: &[f32], r: f32) -> Vec<f32> {\n    let n = x.len() as f32;\n    let r2 = r * r;\n    let post: Vec<f32> = (0..x.len()).map(|j| x[j] * gamma[j] * r).collect();\n    let dot = (0..x.len()).map(|j| post[j] * dy[j]).sum::<f32>() / r; // dot itself is fine\n    let mut dx = vec![0.0f32; x.len()];\n    for i in 0..x.len() {\n        dx[i] = gamma[i] * r * (dy[i] - x[i] * r2 * dot / n); // WRONG: (gamma_i-1)*correction\n    }\n    dx\n}".to_string(),
            opt_pattern: "// optimized: gamma multiplies ONLY the direct dy term — the correction stays bare\nfn rmsnorm_backward_into(x: &[f32], dy: &[f32], gamma: &[f32], r: f32,\n                         dx: &mut [f32], dgamma: &mut [f32]) {\n    let inv_n = 1.0 / x.len() as f32;\n    let r2 = r * r;\n    let dot: f32 = (0..x.len()).map(|j| x[j] * gamma[j] * dy[j]).sum();\n    for i in 0..x.len() {\n        dgamma[i] += x[i] * r * dy[i];\n        dx[i] = r * (gamma[i] * dy[i] - x[i] * r2 * dot * inv_n); // gamma scopes to dy ONLY\n    }\n}".to_string(),
        },
        // ── per-position-scalar-factored-gemv-collapse — linearity collapses
        // the GEMV loop; consumer polarity is the hazard (D) ──
        //
        // Source: kimi_k3_gpu_backward.rs mla_backward_gpu — the CORRECT form
        // ships twice: L2557-L2559 ("Issue 393 scalar factoring: since
        // d_v_c_j_h = attn_w_h[j] * d_o_h for all j, the per-position GEMV
        // W_UV^T @ d_v_c_j_h = attn_w_h[j] * (W_UV^T @ d_o_h)") and L2596-L2598
        // (the W_UK twin, whose consumer L2621-L2623 correctly re-applies ds).
        // The FAILURE MODE is live at L2576-L2584: `let scaled = aw *
        // base_uv[i]; d_ckv_j[i] += scaled; s += scaled * c_kv_j[i];` — the
        // same scaled term feeds the UNSCALED consumer, so d_attn_w[j]
        // carries a spurious attention-weight factor (the correct form is
        // `s += base_uv[i] * c_kv_j[i];`). Filed as riir-ai Issue 693 H1.
        //
        // Distinct from fold-norm-scalar-into-matmul (B14 — per-row CONSTANT
        // folded in; this factors a data-dependent per-position scalar OUT),
        // from constant-scale-folded-into-upload-copy (B36 — config constant),
        // from sparse-gather-scale-at-read-not-table-prepass (B37 — sparse
        // gather fold-point).
        KernelRule {
            rule_id: "per-position-scalar-factored-gemv-collapse".to_string(),
            gpu_concept: "When every position j's GEMV input is a scalar multiple of one fixed vector (d_v_j = w_j * d_o), factor the scalar out by linearity — W_T(s * x) = s * (W_T x) — collapsing n_h x seq GEMVs into n_h base GEMVs plus O(seq * d) scalar multiplies on CPU (observed 32 round-trips -> 1 per token). The hazard is consumer polarity: consumers of the SCALED gradient take w_j * base (the d_c_kv accumulation wants the weight-scaled gradient), while consumers of the UNSCALED projection — attention-weight gradients dL/d(attn_w[j]) = dot(d_o, v_j) — take base RAW with NO weight factor. Fusing both onto one scaled accumulator smuggles the scalar into the unscaled consumer and silently corrupts every downstream gradient it feeds; keep two accumulator expressions (or rescale deliberately at the second consumer). Distinct from fold-norm-scalar-into-matmul (B14 — folds a per-row CONSTANT into dot elements; this factors a data-dependent per-position scalar OUT of the GEMV input), from constant-scale-folded-into-upload-copy (B36 — a config constant folded into a copy), and from sparse-gather-scale-at-read-not-table-prepass (B37 — fold-point selection for sparse gathers).".to_string(),
            ref_shape: "// naive: one GEMV per (head, position) — n_h * seq dispatches + round-trips\nfor h in 0..n_h {\n    for j in 0..seq {\n        let mut d_v = d_o[h].clone();\n        for i in 0..d_v.len() { d_v[i] *= attn_w[h][j]; }   // scale BEFORE the GEMV\n        let d_ckv_h = launch_gemv(client, tune, &w_uv_t[h], &d_v, d_c, v_h);\n        let d_ckv = read_batched(client, vec![d_ckv_h]).remove(0);\n        for i in 0..d_c { d_c_kv[j][i] += d_ckv[i]; }\n        d_attn_w[h][j] = dot(&d_ckv, &c_kv[j]);             // WRONG: carries the attn_w scale\n    }\n}".to_string(),
            opt_pattern: "// optimized: linearity — one base GEMV per head; scalars applied AT CONSUMPTION,\n// scaled and unscaled consumers kept on separate accumulator expressions\nlet mut bases = Vec::with_capacity(n_h);\nfor h in 0..n_h { bases.push(launch_gemv(client, tune, &w_uv_t[h], &d_o[h], d_c, v_h)); }\nlet base_uv = read_batched(client, bases);                   // ONE sync for all heads\nfor h in 0..n_h {\n    for j in 0..seq {\n        let aw = attn_w[h][j];\n        let mut s = 0.0f32;\n        for i in 0..d_c {\n            d_c_kv[j][i] += aw * base_uv[h][i];              // scaled consumer: re-apply aw\n            s += base_uv[h][i] * c_kv[j][i];                 // unscaled consumer: base RAW\n        }\n        d_attn_w[h][j] = s;                                  // dot(d_o, v_j) — NO aw factor\n    }\n}".to_string(),
        },
        // ── foreign-accumulator-bridge-copy-hoist-to-outer-loop — the KDA
        // 14-field gradient ping-pong, hoisted to sequence scope (A) ──
        //
        // Source: kimi_k3_gpu_backward.rs kda_backward_gpu — L3082 (`let mut
        // cpu_grads = CpuKdaGradients::zeros_like(weights);` — fresh 14-Vec
        // allocation per token), L3085-L3098 (fourteen copy_from_slice
        // copy-in lines), L3256-L3269 (the same fourteen reversed — copy-out).
        // The comment at L3079-L3081 justifies the bridge by naming FOUR
        // fields ("The core backward accumulates into o_proj, o_norm_weight,
        // a_log, dt_bias") while the code copies all fourteen both ways.
        //
        // Distinct from variant-owned-output-buffer-clone-escape (B39 —
        // enum-arm ownership, not cross-crate API shape), from
        // transient-spike-pins-persistent-scratch (B38 — sizing policy, not
        // copy traffic), from adapter-grad-join-mirrors-forward-delta-
        // insertion-point (B35 — LoRA join placement).
        KernelRule {
            rule_id: "foreign-accumulator-bridge-copy-hoist-to-outer-loop".to_string(),
            gpu_concept: "Bridging a GPU path to a foreign crate's whole-struct gradient accumulator via alloc-zeros + copy-in every field -> compute -> copy-out every field, per CALL, makes the full gradient state ping-pong across the loop: one fresh 14-Vec allocation + 2x full-state memcpy per token, doubling the memory traffic of the projection-gradient matrices the bridge exists to serve. When the caller's loop is the only writer between calls, hoist the bridge: persist the foreign accumulator across the outer loop (copy-in once at sequence start, copy-out once at sequence end) or accumulate directly into the caller's granular fields. The comment that justifies the bridge usually under-counts the fields the core actually accumulates — copy what the contract says, not what the type contains. Distinct from variant-owned-output-buffer-clone-escape (B39 — enum-match-arm buffer ownership; this is a cross-crate API shape), from transient-spike-pins-persistent-scratch (B38 — scratch sizing policy; this is copy traffic), and from adapter-grad-join-mirrors-forward-delta-insertion-point (B35 — LoRA join placement).".to_string(),
            ref_shape: "// naive: full foreign-state alloc + copy-in/out PER TOKEN\nfor token in 0..seq {\n    let mut cpu_grads = ForeignGradients::zeros_like(&weights); // fresh 14-Vec alloc\n    cpu_grads.q_proj.copy_from_slice(&grads.q_proj);            // full-state copy-in\n    cpu_grads.k_proj.copy_from_slice(&grads.k_proj);\n    // ... 12 more fields ...\n    foreign_backward_token(&mut cpu_grads, &saved[token], &activations[token]);\n    grads.q_proj.copy_from_slice(&cpu_grads.q_proj);            // full-state copy-out\n    grads.k_proj.copy_from_slice(&cpu_grads.k_proj);\n    // ... 12 more fields ...\n}".to_string(),
            opt_pattern: "// optimized: the bridge lives at SEQUENCE scope — one copy-in, one copy-out\nlet mut cpu_grads = ForeignGradients::zeros_like(&weights);\ncpu_grads.q_proj.copy_from_slice(&grads.q_proj);                // copy-in ONCE\ncpu_grads.k_proj.copy_from_slice(&grads.k_proj);\n// ... 12 more fields ...\nfor token in 0..seq {\n    foreign_backward_token(&mut cpu_grads, &saved[token], &activations[token]); // in place\n}\ngrads.q_proj.copy_from_slice(&cpu_grads.q_proj);                // copy-out ONCE\ngrads.k_proj.copy_from_slice(&cpu_grads.k_proj);\n// ... 12 more fields ...".to_string(),
        },
        // ── concurrent-disjoint-write-buffer-parallelizes-uploads — host-side
        // upload parallelism: native backends do not serialize disjoint writes (E) ──
        //
        // Source: riir-gpu weight_buffer_cache.rs — POSITIVE: L237-L257
        // (parallel_write_buffer: 256-aligned chunks from rayon workers, clamp 16
        // threads, single-write fallback) + kimi_k3_gpu_backward_sequence.rs
        // L546-L555 (per-layer update parallelism docs: "~8x parallelism for the
        // per-layer update... ~49ms sequential to ~8ms parallel on M3 Max") +
        // kimi_k3_gpu_backward.rs L342-L345 (3 Dense FFN matrices transposed in
        // parallel under rayon::scope, ~2-3x on multi-core).
        //
        // Distinct from chunk-encoder-submits-below-driver-limit (B26) and
        // per-slot-uniform-buffer-write-ordering (B26) — neither covers concurrent
        // upload bandwidth; independent-cpu-work-fills-dispatch-read-window (B36)
        // overlaps CPU work inside a dispatch-read window instead.
        KernelRule {
            rule_id: "concurrent-disjoint-write-buffer-parallelizes-uploads".to_string(),
            gpu_concept: "A large host-to-device upload serialized on one thread is host-bandwidth-bound, not device-bound. Native wgpu backends (Metal/Vulkan/DX12) do NOT serialize queue.write_buffer calls that target disjoint offsets, so one big upload splits into 256-byte-aligned chunks written concurrently from rayon workers (the Queue and ComputeClient are both Sync, proven in the expert loop), and N independent per-unit update+upload tasks (per-layer weight transposes) parallelize under rayon::scope with a first-error Mutex<Option<E>> collector. Chunk size must round UP to COPY_BUFFER_ALIGNMENT (256) - a mis-aligned chunk offset panics wgpu validation - and the code must fall back to a single write when n_chunks <= 1 or chunk_bytes >= total. Measured on M3 Max: ~8x parallelism for 8 per-layer updates (~49ms -> ~8ms). Distinct from chunk-encoder-submits-below-driver-limit (B26 - encoder submit count vs driver limit; this is upload bandwidth), from independent-cpu-work-fills-dispatch-read-window (B36 - overlap inside a dispatch-read window; this is parallelizing the upload itself), and from per-slot-uniform-buffer-write-ordering (B26 - uniform-write collision inside one encoder; disjoint weight ranges never collide).".to_string(),
            ref_shape: "// naive: one thread writes the whole upload serially - host-bandwidth-bound\nfn update_all(queue: &Queue, buffer: &wgpu::Buffer, data: &[u8]) {\n    queue.write_buffer(buffer, 0, data); // 640 MB from a single thread\n}\nfn update_layers(&mut self) {\n    for layer in &mut self.layers {\n        layer.transpose_and_upload(); // sequential: ~49 ms for 8 layers\n    }\n}".to_string(),
            opt_pattern: "// optimized: disjoint-offset writes parallelize on native backends\nfn parallel_write_buffer(queue: &Queue, buffer: &wgpu::Buffer, base: u64, data: &[u8]) {\n    let total = data.len();\n    let n = rayon::current_num_threads().clamp(1, 16);\n    let chunk = total.div_ceil(n);\n    let chunk = (chunk + 255) & !255; // round UP to COPY_BUFFER_ALIGNMENT\n    if n > 1 && chunk < total {\n        rayon::scope(|s| {\n            for off in (0..total).step_by(chunk) {\n                let end = (off + chunk).min(total);\n                let part = &data[off..end];\n                s.spawn(move |_| queue.write_buffer(buffer, base + off as u64, part));\n            }\n        });\n    } else {\n        queue.write_buffer(buffer, base, data);\n    }\n}".to_string(),
        },
        // ── aliased-slot-allocate-replace-fallback-severs-binding — the
        // boundary condition B39 omitted: aliased slots cannot fall back (A) ──
        //
        // Source: riir-gpu weight_buffer_cache.rs L158-L161 (the severing fallback:
        // `self.handle = client.create_from_slice(...)` replaces the SLOT's handle
        // clone while the consumer binds the original) + L128-L132 (cuda_backend
        // extract_buffer is a no-op Ok(()) - the fast path can never engage);
        // kimi_k3_gpu_backward_sequence.rs L478-L481 (from_handle aliasing
        // self.lm_head.lm_head_t) + L527-L535 (the CPU-fallback branch feeds this);
        // kimi_k3_gpu_backward.rs L321-L325 (per-layer systemic instance).
        //
        // Distinct from slow-path-bootstraps-raw-buffer-hotswap-cache (B39 - that
        // rule PRESCRIBES the two-tier fallback for slots the consumer reads via
        // the slot's own handle; this rule is when the fallback is WRONG) and
        // per-dispatch-pointer-re-extraction (B21 - movement, not control flow).
        KernelRule {
            rule_id: "aliased-slot-allocate-replace-fallback-severs-binding".to_string(),
            gpu_concept: "A two-tier in-place write cache slot built from a handle CLONE (from_handle, aliasing a handle owned by another struct) must treat 'raw view unavailable' as a hard error - never fall back to allocate-and-replace. The fallback branch writes into the slot's OWN fresh allocation while the consumer keeps binding the ORIGINAL handle elsewhere: every subsequent in-place update is orphaned and the consumer silently reads boot weights forever. The fallback is only sound for slots whose own handle is the sole consumer-read reference (from_data). On backends where raw-view extraction is a no-op returning Ok (cuda_backend), the fast path can never engage - disable in-place writes entirely rather than let every update take the severing fallback. Distinct from slow-path-bootstraps-raw-buffer-hotswap-cache (B39 - the two-tier pattern for slots the consumer reads through the slot's own handle; this is the boundary condition that makes the fallback semantically wrong for ALIASED slots), from per-dispatch-pointer-re-extraction (B21 - pool-movement staleness; this is fallback control flow severing the alias).".to_string(),
            ref_shape: "// naive: the fallback silently severs the alias\nfn write_in_place(&mut self, client: &ComputeClient, data: &[f32]) {\n    if let Some((buf, off)) = &self.extracted {\n        self.queue.write_buffer(buf, *off, f32::as_bytes(data)); // fast path\n    } else {\n        // SLOW PATH: replaces the SLOT's handle - but the consumer binds\n        // the ORIGINAL handle elsewhere: these writes are orphaned forever\n        self.handle = client.create_from_slice(f32::as_bytes(data));\n    }\n}".to_string(),
            opt_pattern: "// optimized: aliased slots refuse the fallback - fail loudly instead\nfn write_in_place(&mut self, client: &ComputeClient, queue: &Queue, data: &[f32]) -> Result<(), String> {\n    let (buf, off) = self.extracted.as_ref().ok_or_else(|| {\n        \"aliased slot: raw view not extracted - create_from_slice would orphan writes\".to_string()\n    })?;\n    debug_assert_eq!(data.len(), self.size_bytes, \"slot size contract\");\n    queue.write_buffer(buf, off, f32::as_bytes(data)); // lands where the consumer reads\n    Ok(())\n}\n// construction: extraction must succeed BEFORE any fast-path call\nslot.extract_buffer(client)?; // hard error on backends that cannot extract".to_string(),
        },
        // ── pool-slot-write-coalescing-owns-only-declared-ranges — writer-side
        // complement of the B40 build-side rule: never write gap bytes (A) ──
        //
        // Source: riir-gpu weight_buffer_cache.rs write_slots_batched — L263-L283
        // (the doc contract: group by buffer identity, sort by offset, coalesce
        // exact-adjacency runs; "we don't write padding bytes that might belong
        // to other allocations") + L309-L311 (grouping) + L325 (sort) +
        // L334-L335 (prev_end == offset adjacency test) + L415-L416 (ONE
        // write_buffer per run).
        //
        // Distinct from host-scratch-mirrors-slot-order-for-coalesced-writes
        // (B40 - build-side scratch layout; this is the writer-side algorithm +
        // gap-ownership contract) and batched-downloads-single-sync-cycle (B26 -
        // the READ side).
        KernelRule {
            rule_id: "pool-slot-write-coalescing-owns-only-declared-ranges".to_string(),
            gpu_concept: "When N cached slots are sub-allocations inside pool-managed device buffers, per-slot write_buffer calls coalesce at runtime: group slots by parent buffer identity, sort by offset, and merge runs where prev_end == next offset into ONE staging copy + ONE write per run. The load-bearing safety contract: a coalesced write may cover ONLY the union of declared slot ranges - gap bytes between non-contiguous slots may belong to OTHER pool tenants and must never be written. Contiguity is an opportunistic pool-placement property, so the writer must DISCOVER it (group + sort + extend only on exact adjacency), never assume it from construction order. Isolated slots and gapped runs fall back to per-slot writes. Reduces ~246 calls/step to a handful when the pool places consecutive allocations in the same page. Distinct from host-scratch-mirrors-slot-order-for-coalesced-writes (B40 - the BUILD-side enabler: host scratch laid out in slot order; this is the writer-side algorithm + the never-widen-beyond-owned-ranges contract), from batched-downloads-single-sync-cycle (B26 - the READ side).".to_string(),
            ref_shape: "// naive: N independent write calls - one per slot, every step\nfor (slot, data) in slots.iter().zip(data_slices) {\n    queue.write_buffer(slot.buffer(), slot.offset, f32::as_bytes(data)); // ~246 calls/step\n}".to_string(),
            opt_pattern: "// optimized: discover contiguous runs at runtime, write ONE call per run\nlet mut groups: Vec<(wgpu::Buffer, Vec<usize>)> = Vec::new();\nfor (i, slot) in slots.iter().enumerate() {\n    let buf = slot.buffer().clone();\n    match groups.iter_mut().find(|(b, _)| *b == buf) {\n        Some((_, idxs)) => idxs.push(i),\n        None => groups.push((buf, vec![i])),\n    }\n}\nfor (_, idxs) in &mut groups {\n    idxs.sort_by_key(|&i| slots[i].offset); // adjacency by offset, not construction order\n    let mut run: Vec<usize> = Vec::new();\n    for &i in idxs.iter() {\n        match run.last() {\n            Some(&p) if slots[p].offset + slots[p].size_bytes as u64 == slots[i].offset => run.push(i),\n            Some(_) => { flush_run(&run); run.clear(); run.push(i); } // GAP - never widen into neighbor bytes\n            None => run.push(i),\n        }\n    }\n    flush_run(&run); // one staging copy + ONE write_buffer for the whole run\n}".to_string(),
        },
        // ── pool-reservation-token-outlives-raw-view — the raw view is not
        // ownership; the wrapper Handle is the title (A) ──
        //
        // Source: riir-gpu weight_buffer_cache.rs L40-L43 ("The Handle must stay
        // alive (held by the cache) to prevent CubeCL from recycling the buffer")
        // + L56-L62 ("The Handle owns the CubeCL pool reservation that backs the
        // buffer") + L61-L62 (pub handle: Handle - the load-bearing field).
        //
        // Distinct from per-dispatch-pointer-re-extraction (B21 - pool MOVEMENT
        // staleness; this is the drop-re-issues direction) and
        // slow-path-bootstraps-raw-buffer-hotswap-cache (B39 - builds slots;
        // this keeps them sound).
        KernelRule {
            rule_id: "pool-reservation-token-outlives-raw-view".to_string(),
            gpu_concept: "A cached raw sub-allocation view (native buffer + offset) is NOT an ownership token for the pooled range. The wrapper Handle owns the pool reservation; if it drops, the manager marks the range free and re-issues it to the next allocation - the still-alive raw view then aliases a DIFFERENT tenant's bytes. The cache struct must carry the wrapper handle as a load-bearing field: it looks vestigial (nothing binds it), but deleting it corrupts unrelated tensors. Keep handle and view together in the slot struct, document the handle as the reservation token, and on wrapper-runtime swap re-derive the view from a live handle. Distinct from per-dispatch-pointer-re-extraction (B21 - the pool may MOVE the allocation, so re-extract pointers; this is the opposite direction: DROP the handle and the range is re-issued while the view survives), from slow-path-bootstraps-raw-buffer-hotswap-cache (B39 - builds the slot cache; this states the retention invariant that keeps it sound).".to_string(),
            ref_shape: "// naive: \"cleaning up\" the seemingly-unused wrapper handle\nstruct Slot {\n    buffer: wgpu::Buffer, // raw view - kept\n    offset: u64,\n    // handle: Handle,  <- deleted as \"unused\": the pool frees the range\n    //           and re-issues it - this slot now writes a NEIGHBOR's bytes\n}".to_string(),
            opt_pattern: "// optimized: the wrapper handle is the reservation token - load-bearing\nstruct WeightBufferSlot {\n    handle: Handle,       // OWNS the pool reservation - never drop\n    buffer: wgpu::Buffer, // raw view for queue.write_buffer\n    offset: u64,\n    size_bytes: usize,\n    // doc: buffer+offset is a VIEW, handle is the TITLE. Deleting the\n    // handle re-issues the range; the view silently aliases a new tenant.\n}".to_string(),
        },
        // ── gpu-type-bridge-borrows-not-clones — the CPU-to-GPU type seam
        // borrows activations instead of cloning them twice (A) ──
        //
        // Source: kimi_k3_gpu_backward_sequence.rs L892-L896 (MLA double clone:
        // all_saved_cpu via .clone().unwrap() then convert_mla_saved over .iter()
        // - the owned Vec is dead after the map) + L1082-L1095 (Dense: 4 field
        // clones per token per layer into a field-identical type) + L941-L943
        // (KDA clone forced by the owned-slice API of kda_backward_sequence) +
        // L1179-L1227 (convert_mla_saved/convert_moe_saved clone every field -
        // zero transformation anywhere).
        //
        // Distinct from foreign-accumulator-bridge-copy-hoist-to-outer-loop (B40 -
        // accumulator copy-in/out per call; this is conversion field-cloning) and
        // variant-owned-output-buffer-clone-escape (B39 - match-arm escape).
        KernelRule {
            rule_id: "gpu-type-bridge-borrows-not-clones".to_string(),
            gpu_concept: "Bridging CPU reference types to GPU module types around per-layer batched calls by cloning the saved-activation payload multiplies activation memory traffic per layer per step. The worst shape clones TWICE: first materialize an owned copy of the source collection, then run a converter that clones every field again into the GPU type - the first clone is 100% dead when the converter only takes &T. GPU 'backward saved' types are frequently field-identical to the CPU types (pure copy, zero math), so the bridge can borrow the source (per-element as_ref, or lifetime-parameterized bridge types) instead of owning. Where a batched callee's owned-slice API (&[T]) forces the clone, the fix is the API (&[&T] when the callee only reads), not the clone. Distinct from foreign-accumulator-bridge-copy-hoist-to-outer-loop (B40 - cross-crate accumulator copy-in/out per call; this is type-conversion field cloning at the CPU-GPU seam), from variant-owned-output-buffer-clone-escape (B39 - match-arm ownership escape).".to_string(),
            ref_shape: "// naive: double clone - the owned intermediate is dead after the map\nlet all_saved_cpu: Vec<CpuSaved> = (0..l)\n    .map(|t| tokens[t].layers[i].mla_saved.clone().unwrap()) // clone #1 (owned)\n    .collect();\nlet all_saved_gpu: Vec<GpuSaved> = all_saved_cpu.iter().map(convert).collect(); // clone #2 per field".to_string(),
            opt_pattern: "// optimized: borrow the source - the converter only needs &T\nlet all_saved_gpu: Vec<GpuSaved> = (0..l)\n    .map(|t| convert(tokens[t].layers[i].mla_saved.as_ref().unwrap())) // &T in, one clone out\n    .collect();\n// where the callee's owned-slice API forces the clone, fix the API instead:\n// callee reads - takes refs, not an owned Vec<CpuSaved>\nfn backward_batched(saved: &[&CpuSaved], grads: &mut Grads) {}".to_string(),
        },
        // ── superseded-primitive-machinery-zombie — retire support machinery
        // with its consumer; test-only callers are the tell (A) ──
        //
        // Source: kimi_k3_gpu_backward_sequence.rs L13 (header still claims
        // "LM head backward: lm_head_backward_gpu (T6.5) - 1 GPU GEMV") vs
        // L735-L741 + L790-L796 (production LM head is CPU:
        // batched_transpose_matvec + batched_outer_product_acc) + L476-L481
        // (boot builds the handle) + L489-L526 (per-step 640 MB-class staging
        // upload + GPU transpose dispatch + submit) + L281-L286 (staging buffer
        // sized hidden*vocab*4); lm_head_backward_gpu itself
        // (kimi_k3_gpu_backward.rs L457) has exactly one caller - its own test
        // (L3617). riir-train kimi_k3_train.rs L2200-L2210 still justifies the
        // wiring with the pre-replacement perf claim (~641ms vs ~17ms).
        //
        // Distinct from derive-shape-from-payload-not-cached-field (B32b) and
        // pre-transposed-consumer-layout-handles-build-once (B34 - LIVE-handle
        // staleness; this handle has no reader at all).
        KernelRule {
            rule_id: "superseded-primitive-machinery-zombie".to_string(),
            gpu_concept: "When a composition path replaces a per-module GPU primitive with a new one (per-token GEMV -> CPU batched primitives), the superseded primitive's support machinery - pre-built handle, per-step transpose + re-upload pipeline, caches, staging buffers - keeps executing every step because its lifecycle hangs off a long-lived struct's update schedule, not the now-replaced call site. The tell: the superseded fn's only remaining caller is its own parity test. A maintained-but-never-read pub handle is invisible to dead-code lints, and the cost scales with the largest matrix it maintains (a 640 MB-class staging upload + dispatch every step, plus ~2x that in permanently-dead device memory). Retire the consumer and the machinery together, or gate the machinery behind the test/bench that still consumes it; at minimum sync the architecture header that still describes the dead path. Distinct from derive-shape-from-payload-not-cached-field (B32b - stale metadata), from pre-transposed-consumer-layout-handles-build-once (B34 - handle staleness under hot-swap of LIVE handles; this handle has no reader at all).".to_string(),
            ref_shape: "// naive: superseded GPU primitive, machinery still maintained every step\nstruct BackwardHandles {\n    lm_head: LmHeadHandle,         // built at boot...\n    lm_head_cache: Option<Slot>,   // ...transposed + re-uploaded EVERY step\n    staging: Option<wgpu::Buffer>, // 640 MB staging buffer for the transpose\n}\nfn update(&mut self, w: &Weights, queue: &Queue) {\n    // GPU transpose + parallel upload - for a GEMV that production\n    // no longer calls (the CPU batched path replaced it)\n    self.write_lm_head_transposed(w, queue); // ~640 MB upload/step, zero readers\n}".to_string(),
            opt_pattern: "// optimized: retire the consumer and the machinery together\nfn update(&mut self, w: &Weights, queue: &Queue) {\n    // LM head gradient is CPU batched now (batched_outer_product_acc) -\n    // the GPU transpose pipeline, staging buffer, and handle cache are\n    // deleted with the primitive they served. Zero per-step upload.\n}\n// keep the parity test honest: gate the primitive behind the test feature\n#[cfg(any(test, feature = \"lm_head_gpu_parity\"))]\nfn lm_head_backward_gpu(handle: &LmHeadHandle, d_logits: &[f32]) -> Vec<f32> {\n    vec![0.0; d_logits.len()]\n}".to_string(),
        },
        // ── refine-loop-invariant-prefix-elision — an iterative refinement
        // loop whose mutation set is closed must persist the complement's
        // forward artifacts (D) ──
        //
        // Source: riir-gpu gemma2_d2f/mod.rs — the D2F denoise loop re-forwards
        // the FULL sequence every step: L499-502 (design doc admits it: "The KV
        // cache is reset at the start of each call. Each call is independent
        // (suitable for D2F denoising where each step re-runs the full sequence
        // with different masks)") + L525-529 (fresh CpuKVCache per call) +
        // L1488/L1491-1493 (full forward per step) vs L1498+L1528 (tokens
        // mutated ONLY in block_start..seq_len). The invariance PROOF is the
        // kernel's own masking: attention_cubecl/mod.rs L1214-1217 t_n closes
        // prompt attention within the prompt (pos < prompt_len => t_n =
        // prompt_len), so prompt K/V + hidden states are bit-identical across
        // steps. Precondition boundary: gemma2_d2f_sc.rs L158 — with SC
        // enabled, prompt x0 estimates feed prompt embeddings and break the
        // invariance (restrict SC to the refine set to restore it).
        //
        // Distinct from append-cache-rewind-overwrite-in-place (B39 — cache
        // EXTENT management; still recomputes everything, saving memory) and
        // lm-head-loss-positions-only (B37 — the per-position consumer-set
        // sibling; this is the cross-iteration producer-invariance complement
        // in the same dispatch-elision class).
        KernelRule {
            rule_id: "refine-loop-invariant-prefix-elision".to_string(),
            gpu_concept: "In an iterative refinement loop over a sequence (denoise loops, speculative re-ranks, any re-run-until-stable decode), the mutation set is usually a proper subset S (here: only the generation block's tokens are rewritten between iterations). When the attention mask closes the complement against S - block-causal: prompt positions attend only to 0..prompt_len - the complement's per-layer K/V and hidden states are PROVABLY identical across iterations. Compute the prefix's forward artifacts ONCE before the loop, persist them (device-resident KV + hidden slots), and run each iteration's forward over S only, reading the persisted prefix state. Iteration cost drops from O(steps x seq) to O(seq + steps x |S|) in FLOPs, dispatches, AND bus traffic. The invariance is provable from the coupling structure (masking + mutation-set closure), not heuristic - work with a proven-identical result is pure waste. Precondition: the refine set must be closed (SC-style conditioning that feeds prompt embeddings next step breaks it - restrict the conditioning to S). Distinct from append-cache-rewind-overwrite-in-place (B39 - cache extent management for re-execution, still recomputes; this ELIDES the recompute), from lm-head-loss-positions-only (B37 - per-position consumer-set elision; this is the cross-iteration producer-invariance complement), from training-forward-shed-decode-loop-shape (B36 - batches the position axis WITHIN one forward; this hoists across loop ITERATIONS).".to_string(),
            ref_shape: "// naive: each refinement iteration resets the cache and re-forwards EVERYTHING\nfor step in 0..max_steps {\n    self.kv_cache = CpuKVCache::new(n_layer, kv_dim); // reset per step\n    let logits = self.forward_all(&tokens, prompt_len); // prompt + block, all layers\n    for pos in block_start..seq_len {\n        tokens[pos] = sample(logits[pos]); // the ONLY mutation site\n    }\n}\n// prompt K/V are bit-identical every step (mask closes prompt vs block)\n// -> 7/8 of prompt work is pure waste at defaults (8 steps)".to_string(),
            opt_pattern: "// optimized: forward the invariant prefix ONCE, iterate over the refine set only\nlet prefix_state = self.forward_prefix_persist(&tokens[..prompt_len]); // once\nfor step in 0..max_steps {\n    let logits = self.forward_block_against(&tokens[block_start..seq_len], &prefix_state);\n    for pos in block_start..seq_len {\n        tokens[pos] = sample(logits[pos - block_start]);\n    }\n}\n// cost: O(seq) once + O(steps x |S|) instead of O(steps x seq)".to_string(),
        },
        // ── softmax-entropy-via-shift-identity — ln(e_t) = logit_t - max,
        // so the entropy sum needs ONE transcendental, not N (B) ──
        //
        // Source: riir-gpu gemma2_d2f/mod.rs L103-109 + L121-127 + L151-154
        // (SamplerFeatures::from_logits — the POSITIVE instance: a single fused
        // sweep over the vocab computes sum_exp, sum_e_ln_e via
        // e * logit_shifted, and a top-3 cascade, all off one e_t = exp(shift)
        // evaluation per token; entropy recovered as ln(S) - (1/S)*sum_e_ln_e).
        //
        // Distinct from fuse-xent-logsumexp / fuse-jsd-logsumexp (pass FUSION
        // of softmax+log+NLL — this is transcendental-count elimination via an
        // algebraic identity) and exp2-log2-base2 (base conversion).
        KernelRule {
            rule_id: "softmax-entropy-via-shift-identity".to_string(),
            gpu_concept: "To accumulate an entropy-style sum over a softmax, never materialize probabilities and call ln() per element: with e_t = exp(logit_t - max), ln(e_t) = logit_t - max EXACTLY (the shift is exp's argument, so its log is the shift itself). Accumulate sum_e_ln_e as e_t * logit_shifted in the SAME sweep that builds the softmax denominator and any top-k cascade, then recover the entropy as ln(S) - (1/S) * sum_e_ln_e - ONE transcendental for the whole distribution instead of N. Guard the underflow tail before accumulating (e > 0 && shift.is_finite(): 0 * -inf = NaN). Max-shifted stability is preserved. Composes with one-sweep statistics fanouts over the same e_t. Distinct from fuse-xent-logsumexp / fuse-jsd-logsumexp (fusing softmax+log+NLL into one kernel - a pass-count concern; this is per-element transcendental COUNT via algebraic identity) and exp2-log2-base2 (base conversion of exp/ln - this ELIMINATES the ln calls).".to_string(),
            ref_shape: "// naive: materialize probs, then N log evaluations\nlet probs = softmax(logits); // N exps + division\nlet mut entropy = 0.0;\nfor t in 0..vocab {\n    if t != mask { entropy -= probs[t] * probs[t].ln(); } // N ln() calls\n}".to_string(),
            opt_pattern: "// optimized: ln(e_t) = logit_t - max_logit — the shift IS the log\nlet max_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);\nlet mut sum_exp = 0.0; let mut sum_e_ln_e = 0.0;\nfor t in 0..vocab {\n    if t == mask { continue; }\n    let shift = logits[t] - max_logit;\n    let e = shift.exp();\n    sum_exp += e;\n    if e > 0.0 && shift.is_finite() { sum_e_ln_e += e * shift; } // 0*(-inf)=NaN guard\n}\n// ONE ln for the whole distribution:\nlet entropy = sum_exp.ln() - sum_e_ln_e / sum_exp; // -Sum p ln p".to_string(),
        },
        // ── runtime-identity-gate-cached-verdict-elides-side-projection —
        // content-classified inertness cached write-through at the mutation
        // boundary gates whole-stage elision (D) ──
        //
        // Source: riir-gpu gemma2_d2f/mod.rs + gemma2_d2f_sc.rs — the POSITIVE
        // instance: mod.rs L556 (`apply_projection = apply_sc &&
        // !self.w_sc_is_identity`) + L543-546 (lazy init sets the flag
        // directly) + L671-674 (set_w_sc RE-DERIVES the verdict from buffer
        // contents at the only mutation boundary, so an externally-supplied
        // identity-shaped buffer still earns the fast path) + L350-355 (the
        // cached bool field) + sc.rs L281-287 (the O(n^2) scan NEVER touches
        // the hot loop — the doc contract, honored). The completeness hazard
        // this file also demonstrates: the producer (update_from_logits,
        // mod.rs L1541-1543) is gated only by sc_enabled, NOT by the identity
        // verdict — a gated consumer with an ungated producer computes data
        // the loop provably discards (riir-ai Issue 695 H16).
        //
        // Distinct from zero-init-side-branch-frozen-base-path (B30 — the
        // VALUE contract: B=0 makes the math exact while the SAME code RUNS;
        // this ELIDES the math under a runtime check) and
        // min-rank-inert-adapter-zero-alpha (B31 — the ALLOCATION contract,
        // annihilated by scale, no gate). Composes with
        // derive-shape-from-payload-not-cached-field (the cached bool is
        // write-through so it cannot desync).
        KernelRule {
            rule_id: "runtime-identity-gate-cached-verdict-elides-side-projection".to_string(),
            gpu_concept: "A learnable side component initialized to an algebraically-neutral value (identity/zero-padded: W_sc = [I|0]) ships with a runtime content-classification: the still-neutral verdict is computed ONCE per WRITE of the buffer (at the sole mutation/setter boundary - re-derived from CONTENTS so externally-supplied buffers are re-classified honestly and a trained-to-nothing buffer still earns the fast path), cached as a plain bool, and every stage of the side channel - the side-input PRODUCTION and the projection CONSUMPTION - branches on the cached flag to ELIDE its work. The hot loop never re-scans (O(1) branch per position); the O(n^2) classification amortizes to one pass per write. This turns enabled-but-untrained = free from a construction-time hope into a runtime invariant that SURVIVES arbitrary buffer writes. The gate must be COMPLETE: a gated consumer with an ungated producer computes inputs nothing reads (the D2F file's own update_from_logits hazard). Distinct from zero-init-side-branch-frozen-base-path (B30 - VALUE contract, the inert math still RUNS in one shared path; this elides the math entirely) and min-rank-inert-adapter-zero-alpha (B31 - ALLOCATION contract, scale-annihilated, no gate). Composes with derive-shape-from-payload-not-cached-field (write-through cache cannot desync by construction).".to_string(),
            ref_shape: "// naive: always run the side math; inertness is only a construction-time hope\nfn embed_step(&mut self, tokens: &[usize], sc: Option<&[Vec<f32>]>) -> Vec<Vec<f32>> {\n    let mut out = Vec::with_capacity(tokens.len());\n    for (pos, &tok) in tokens.iter().enumerate() {\n        let mut h = self.lookup_scale(tok);\n        if let Some(sc) = sc { // enabled check only — trained-ness never checked\n            h = project_sc(&h, &sc[pos], &self.w_sc); // O(n^2) matvec on EVERY position,\n        }                                  // provably identity while untrained\n        out.push(h);\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: cached content-classified verdict, re-derived at the write boundary,\n// gating BOTH producer and consumer\nstruct Engine { w_sc: Option<Vec<f32>>, w_sc_is_identity: bool }\nimpl Engine {\n    fn set_w_sc(&mut self, w: Vec<f32>) { // the ONLY mutation boundary\n        self.w_sc_is_identity = is_identity_padded(&w, self.n); // re-derive from CONTENTS\n        self.w_sc = Some(w);\n    }\n    fn step(&mut self, logits: &[Vec<f32>], sc_on: bool) {\n        let identity = self.w_sc_is_identity; // O(1) branch, cached\n        if sc_on && !identity { self.update_sc_input(logits); } // producer gated\n        // consumer gated by the same flag — skip the O(n^2) projection while identity\n    }\n}".to_string(),
        },
        // ── expected-embedding-transposed-gemv-resident-head — the soft-token
        // expectation is the tied lm_head's transposed GEMV on the
        // already-resident table (A) ──
        //
        // Source: riir-gpu gemma2_d2f_sc.rs L225 + L229-246
        // (compute_x0_estimate: x0[i] = Sum_t softmax_t * wte[t*n_embd + i] —
        // a host loop streaming the ENTIRE V x N CPU mirror per position per
        // step, 2.36 GB + 590M MAC per position at Gemma-2-2B) + mod.rs L605-610
        // (the SAME table device-resident + GEMV'd per position for the tied
        // lm_head: logits = wte @ h) + L455 (wte_cpu: weights.wte.clone() — the
        // CPU mirror kept alongside) + L1541-1543 (update_from_logits walks
        // the mirror per step). The vocabulary translation is load-bearing:
        // "expected embedding over softmax" reads as a gather idiom but
        // factors as the transposed GEMV of the tied head.
        //
        // Distinct from offload-cpu-bottleneck-to-gpu-kernel (B14 — data-PREP
        // transforms; no resident-operand reuse, no orientation flip) and
        // device-side-argmax-minimal-download (B25 — small result of a large
        // DEVICE array; here the host-mirror WALK is deleted).
        KernelRule {
            rule_id: "expected-embedding-transposed-gemv-resident-head".to_string(),
            gpu_concept: "A probability-weighted embedding expectation (x_hat = Sum_v softmax(logits)_v * wte[v, :]) is algebraically the TRANSPOSED GEMV of the tied lm_head the forward just dispatched: logits = wte @ h implies x_hat = wte^T @ p. Recognize the identity, compute the probs on the host (softmax over already-read logits), upload only the vector (V floats), run ONE transposed-GEMV dispatch against the table that is ALREADY device-resident, and read back only the n-float result. The host mirror of the table is never walked: at Gemma-2-2B (V=256128, N=2304) the naive host loop streams 2.36 GB and 590M MAC PER POSITION PER STEP while the GPU copy sits resident having just been GEMV'd in the same orientation family for the logits. One dispatch replaces the entire stream; only V floats up and n floats back cross the bus. The vocabulary translation is the load-bearing insight: expected-embedding-over-softmax reads as a scatter/gather idiom but factors as a GEMV against a weight the device already holds. Distinct from offload-cpu-bottleneck-to-gpu-kernel (B14 - data-prep transforms into pooled memory; no resident-operand reuse, no orientation flip), from device-side-argmax-minimal-download (B25 - shrinking a device-array download; this DELETES a host mirror walk), from transposed-consumption-accumulate-not-gather (B40 - host loop ORDER for B^T g; this moves the op across the bus).".to_string(),
            ref_shape: "// naive: host loop over every vocab row per position per step\nfn x0_estimate(logits: &[f32], wte_cpu: &[f32], n: usize, mask: usize) -> Vec<f32> {\n    let probs = softmax_masked(logits, mask);\n    let mut est = vec![0.0f32; n];\n    for t in 0..vocab {\n        if t == mask { continue; }\n        let off = t * n;\n        for i in 0..n { est[i] += probs[t] * wte_cpu[off + i]; } // full V x N stream\n    }                                                           // PER POSITION PER STEP\n    est // 2.36 GB host walk + 590M MAC at Gemma-2-2B, per position\n}".to_string(),
            opt_pattern: "// optimized: x_hat = wte^T @ p — ONE transposed GEMV on the resident tied head\nfn x0_estimate_gpu(&mut self, logits: &[f32], mask: usize) -> Vec<f32> {\n    let p = softmax_masked(logits, mask);          // host, over read-back logits\n    let p_handle = self.client.create_from_slice(&p); // V floats up\n    // wte is ALREADY resident — the forward just GEMV'd it for logits = wte @ h\n    let out = self.dispatch_gemv_t(&self.wte_handle, &p_handle); // wte^T @ p\n    self.read_into(&out, self.n_embd)              // n floats back — no mirror walk\n}".to_string(),
        },
        // ── mutable-input-scratch-residency-write-per-call — per-iteration
        // input allocations vs scratch-resident handles written per call (A) ──
        //
        // Source: riir-gpu weaver_gpu_corrector.rs — the per-depth
        // `correct_marginals` path allocates its INPUTS fresh inside the
        // Phase B loop: L461 (`let mut raw_flat: Vec<f32> = vec![0.0; 2 * h];`
        // per call) + L464-466 (verifier row re-packed EVERY depth — it is
        // constant across the loop) + L468-477 (THREE `create_from_slice`
        // device allocations per depth: raw/topk_ids/dflash_logits) + L438-439
        // (two `vec![0.0f32; k]` per depth). The same file's batched twin is
        // the POSITIVE contrast: L686-687 packs flat `[depth * k]` buffers +
        // L729-737 uploads once per call (3 total, not 3 x depth). The scratch
        // precedent: L168-189 — `GpuWeaverBatchedScratch` persists 18
        // intermediate handles via `client.empty` while the inputs churn.
        // `client.write(handle, bytes)` into an existing handle is the
        // established in-repo path (ternary_deltanet_gpu_forward.rs
        // rollback_speculative).
        //
        // Distinct from immutable-operand-upload-at-set-creation (B33 —
        // never-changing operands; these change every call), from
        // slow-path-bootstraps-raw-buffer-hotswap-cache (B39 — weight identity
        // churn; plain write suffices at known sizes), from
        // fan-out-shared-input-single-sync (B25 — shared input for sibling
        // dispatches; here the shared row rides INSIDE the packed buffer).
        KernelRule {
            rule_id: "mutable-input-scratch-residency-write-per-call".to_string(),
            gpu_concept: "A GPU hot path that has already adopted scratch-resident intermediates often still allocates its per-call INPUT buffers fresh — create_from_slice per invocation, and worse per loop-iteration inside one call, makes the device allocator service a new allocation request every iteration and retire the previous buffer, while re-uploading a sub-row that never changes across iterations (the constant verifier row packed alongside the changing drafter row). Give changing inputs the same residency discipline the intermediates already enjoy: allocate their handles once in the scratch struct (client.empty, beside the existing intermediate handles) and write contents per call with client.write(handle, bytes) — the established write-into-existing-handle path; flat per-call buffers sized [depth x K] replace per-iteration [K] allocations when all iterations' data is known before the loop (here Phase A computes every depth's top-K first, so the whole upload set is known pre-loop). Size honesty: at or below the tiny-param threshold the per-call rebuild form is correct (tiny-param-buffer-rebuild-over-cache) — the anti-pattern is (a) per-iteration allocation of real-sized inputs (raw = 2 x h floats = 16 KB at h=2048; 3 x depth device alloc requests per call) and (b) per-ITERATION tiny buffers when one flat per-call buffer serves the loop. Distinct from immutable-operand-upload-at-set-creation (operands that NEVER change get the constructor; these change every call, so the constructor does not apply but the scratch does), from slow-path-bootstraps-raw-buffer-hotswap-cache (weight identity churn needs raw-slot machinery; plain client.write suffices when sizes are known at scratch construction), from fan-out-shared-input-single-sync (one shared input for N sibling dispatches; here the shared sub-row rides INSIDE a per-iteration packed buffer — the fix is residency, not fan-out), and from tiny-param-buffer-rebuild-over-cache (the size counterweight this rule defers to below a cache line).".to_string(),
            ref_shape: "// naive: fresh device allocation for the inputs EVERY loop iteration\nfn correct_one_depth(client: &Client, h_verifier: &[f32], h_drafter: &[f32],\n                     topk_ids: &[f32], dflash_logits: &[f32], h: usize) {\n    let mut raw_flat: Vec<f32> = vec![0.0; 2 * h];\n    raw_flat[..h].copy_from_slice(h_verifier);      // constant row re-packed\n    raw_flat[h..2 * h].copy_from_slice(h_drafter);  // per iteration\n    let raw_handle = client.create_from_slice(f32::as_bytes(&raw_flat));\n    let ids_handle = client.create_from_slice(f32::as_bytes(topk_ids));\n    let logits_handle = client.create_from_slice(f32::as_bytes(dflash_logits));\n    dispatch_forward(client, &raw_handle, &ids_handle, &logits_handle);\n    // 3 device alloc requests x depth per call; allocator churn + retire\n}\n".to_string(),
            opt_pattern: "// optimized: inputs are scratch RESIDENTS — handles allocated once beside\n// the intermediates, contents written per call into the existing buffers\nstruct CorrectorScratch {\n    raw: Handle,            // [2 * h]\n    ids: Handle,            // [depth * k] — flat, one write per call\n    logits: Handle,         // [depth * k]\n    raw_staging: Vec<f32>,  // host staging, clear + reuse\n}\nfn correct_depths(client: &Client, scratch: &CorrectorScratch,\n                  h_verifier: &[f32], h_dflash: &[&[f32]], h: usize) {\n    scratch.raw_staging[..h].copy_from_slice(h_verifier); // once per call\n    for (di, h_d) in h_dflash.iter().enumerate() {\n        scratch.raw_staging[h..2 * h].copy_from_slice(h_d);\n        client.write(scratch.raw.clone(), f32::as_bytes(&scratch.raw_staging));\n        dispatch_forward_at(client, &scratch.raw, &scratch.ids, &scratch.logits, di);\n    }\n    // zero device alloc requests per call — same residency as the intermediates\n}\n".to_string(),
        },
        // ── topk-select-nth-over-full-vocab-sort — O(V log V) full pair sort
        // vs select_nth partition + K-prefix sort (B) ──
        //
        // Source: riir-gpu weaver_gpu_corrector.rs L421-430 (per-depth path —
        // materializes (vid, prob) pairs for the WHOLE vocab row + full
        // `sort_unstable_by`; duplicated at L694-703 in the batched path);
        // L432-433 (`n_take = n_valid.min(k)` — only K survivors are consumed);
        // L436 (`to_vec()` snapshot — a second V-to-K reduction point). At
        // V=256k/K=8: ~4.6M comparisons + 4 MB of 16-B pair movement per depth
        // vs ~256k comparisons + 4-B index movement. Scale-conditional on
        // K << V (trivial at the test config V=64).
        //
        // Distinct from order-stable-topk-via-index-sort (B32c — the ordering
        // contract; its naive AND optimized forms both fully sort — compose:
        // select_nth + prefix sort + ascending re-sort). Parity caution: when
        // CPU and GPU paths must agree, both sides run the same host
        // selection — change both or neither.
        KernelRule {
            rule_id: "topk-select-nth-over-full-vocab-sort".to_string(),
            gpu_concept: "Host-side top-K selection feeding a GPU upload commonly materializes the full candidate collection as (id, score) pairs — 4x the score row's bytes at 16 B/entry on 64-bit — and fully sorts it: O(V log V) comparisons plus physical movement of all V payloads, then keeps K. When K << V (speculative-decode top-K over a 256k-vocab marginal row with K=8-64) the full sort is ~18x the selection work at K=8: select_nth_unstable_by partitions in O(V) average, after which sorting only the K-prefix costs O(K log K); selecting over a u32 index buffer that probes the score row (4 B/element) also quarters the data the partition moves versus sorting 16-B (usize, f32) pairs. Determinism: select_nth_unstable + prefix sort is deterministic given the same input sequence; composing with the order-stable rule (re-sort survivors ascending) preserves the canonical-order contract. Parity caution: when a CPU reference and a GPU path must agree (a G3 gate), both sides run the same host selection — change both or neither, since tie order and last-ulp softmax sums can shift under a different selection algorithm. Distinct from order-stable-topk-via-index-sort (its naive AND optimized forms both fully sort — its content is the ordering contract and tie determinism, not the selection cost; compose, don't substitute), from argmax-rng-tie-break-collect-ties (top-1 sweep whose subject is tie-breaking under RNG — not top-K cost), and from device-side-argmax-minimal-download (download size at the boundary — this is the selection algorithm on the host side of it).".to_string(),
            ref_shape: "// naive: materialize (id, score) pairs for the WHOLE vocab, full sort, keep K\nfn topk_full_sort(row: &[f32], k: usize, pairs: &mut Vec<(usize, f32)>) -> usize {\n    pairs.clear();\n    pairs.reserve(row.len());\n    for (vid, &p) in row.iter().enumerate() {\n        if p.is_finite() {\n            pairs.push((vid, p));\n        }\n    }\n    pairs.sort_unstable_by(|a, b| {\n        b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal)\n    });\n    k.min(pairs.len()) // O(V log V) + 16 B/pair movement, V=256k, K=8\n}\n".to_string(),
            opt_pattern: "// optimized: partition in O(V), sort only the K-prefix, move only 4-B indices\nfn topk_partial_select(row: &[f32], k: usize, idx: &mut Vec<u32>) -> usize {\n    idx.clear();\n    idx.reserve(row.len());\n    for (vid, &p) in row.iter().enumerate() {\n        if p.is_finite() {\n            idx.push(vid as u32);\n        }\n    }\n    let take = k.min(idx.len());\n    if take == 0 {\n        return 0;\n    }\n    let desc = |&a: &u32, &b: &u32| {\n        row[a as usize]\n            .partial_cmp(&row[b as usize])\n            .unwrap_or(core::cmp::Ordering::Equal)\n    };\n    idx.select_nth_unstable_by(take - 1, desc);\n    idx[..take].sort_unstable_by(desc);\n    take // O(V) partition + O(K log K) prefix — compose with ascending\n         // re-sort for the canonical-order contract\n}\n".to_string(),
        },
        // ── post-scatter-renorm-support-set-only — dense re-sum + rescale
        // after a K-scatter vs sum-during-scatter + support-only rescale (A) ──
        //
        // Source: riir-gpu weaver_gpu_corrector.rs L547-563 (per-depth Phase C:
        // zero-fill -> scatter n_take <= K values -> `marg_out.iter().sum()`
        // over the FULL vocab row -> rescale pass over the FULL row);
        // duplicated byte-for-byte at L797-812 in the batched path (the
        // duplication itself is Issue 697 H9). V-K of the re-read elements are
        // exact zeros just written; the support's values were live during the
        // scatter. At V=256k, K=8, depth=4: ~2M memory ops / ~8 MB row traffic
        // per call collapse to ~64 ops. Bit-identical by IEEE: +0.0 addition
        // is exact, 0 x inv = +0.0.
        //
        // Distinct from softmax-topk-renormalize-scatter (MoE gate pipeline
        // that sums the picked set BEFORE building the sparse matrix — here
        // the dense row exists in-place and is re-scanned AFTER the scatter).
        KernelRule {
            rule_id: "post-scatter-renorm-support-set-only".to_string(),
            gpu_concept: "After zeroing a dense in-place row and scattering K corrected values into it, renormalizing by summing the ENTIRE row re-reads V elements of which V-K are exact zeros, and the rescale pass re-multiplies all V (0 x inv = 0) — two full O(V) sweeps over a row whose non-zero support is exactly the K positions just written, whose values were in registers/cache during the scatter. Accumulate the sum DURING the scatter (K adds) and rescale only the support (K multiplies). Bit-identical to the dense form: adding +0.0 is exact in IEEE f32 and 0 x inv is +0.0, so removing the zeros from both passes cannot change the result — the optimization carries no numerical risk, unlike most reduction reshapes. The zero-fill sweep stays (the in-place output contract requires zeros outside the top-K); only the read-back sweep and the re-scale sweep are deleted. At V=256k, K=8, depth=4: ~2M memory ops and ~8 MB of row traffic per correct_marginals call collapse to ~64 ops. Distinct from softmax-topk-renormalize-scatter (the MoE gate pipeline softmax-then-topk that BUILDS a sparse routing matrix and sums the picked set BEFORE scattering — here the dense row already exists in-place and is re-scanned AFTER), from broadcast-uniform-skip-reduce (kernel-side thread-uniform reduction skipping — this is the host-side read/write set of a post-scatter renorm), and from order-stable-topk-via-index-sort (selection mechanics — this is the write-back pass).".to_string(),
            ref_shape: "// naive: after the K-scatter, re-read and re-scale the ENTIRE vocab row\nfn scatter_renorm_dense(row: &mut [f32], vids: &[usize], probs: &[f32]) {\n    for v in row.iter_mut() {\n        *v = 0.0;\n    }\n    for (ki, &vid) in vids.iter().enumerate() {\n        row[vid] = probs[ki];\n    }\n    let sum: f32 = row.iter().sum(); // V-K of these are exact zeros\n    if sum > 1e-30 {\n        let inv = 1.0 / sum;\n        for v in row.iter_mut() {\n            *v *= inv; // 0 x inv = +0.0 — pure waste on V-K lanes\n        }\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: accumulate the sum DURING the scatter, rescale the support only\nfn scatter_renorm_support_only(row: &mut [f32], vids: &[usize], probs: &[f32]) {\n    for v in row.iter_mut() {\n        *v = 0.0;\n    }\n    let mut sum = 0.0f32;\n    for (ki, &vid) in vids.iter().enumerate() {\n        row[vid] = probs[ki];\n        sum += probs[ki]; // the K written values were live right here\n    }\n    if sum > 1e-30 {\n        let inv = 1.0 / sum;\n        for &vid in vids.iter() {\n            row[vid] *= inv; // bit-identical: +0.0 adds are exact anyway\n        }\n    }\n}\n".to_string(),
        },
        // ── stack-array-row-view-bridge — Vec<&[f32]> heap collect vs a
        // fixed [&[f32]; MAX] stack bridge (A) ──
        //
        // Source: riir-gpu weaver_gpu_dflash.rs L134-146 (the POSITIVE
        // instance: `h_dflash_slices: [&[f32]; 64]` built on the stack, passed
        // as `&h_dflash_slices[..count]`). The anti-instance: the weaver
        // bench (bench_weaver_gpu_436.rs L499-500 —
        // `let h_dflash_refs: Vec<&[f32]> = ...collect()`, a heap alloc per
        // call in the hot path). The idiom recurs at 5+ sites: katgpt-rs
        // step_gdn_tree.rs L50-62 (build_marginals_view returns the array BY
        // VALUE as the shared pub helper) + its mirrored twins (rollback +
        // qwen_deltanet tree steps) + riir-engine dflash.rs L232-245 (the CPU
        // wrapper this file mirrors).
        //
        // Distinct from caller-owned-split-scratch (mutable scratch carving),
        // from flat-activation-slots-as-readback-target (flattens the
        // readback destination — this is flat-to-nested view construction on
        // already-host data), from ragged-pack-offset-table-fused-dispatch
        // (device-side ragged packing — this is host-side uniform stride).
        KernelRule {
            rule_id: "stack-array-row-view-bridge".to_string(),
            gpu_concept: "A host-side orchestration step holds a flat, densely-packed capture buffer (steps x row_len f32s) but the consumer API is shaped &[&[f32]] (per-row borrowed views — the API shape owned by the sibling CPU implementation). The naive bridge collects a Vec<&[f32]> — a heap allocation per call in the decode hot path (a live in-tree anti-instance: the weaver bench itself does let h_dflash_refs: Vec<&[f32]> = ...collect()), or worse copies into Vec<Vec<f32>>. The optimized bridge constructs the view on the STACK: a fixed-size [&[f32]; MAX_STEPS] initialized to &[], filled for count = steps.min(MAX), passed as &arr[..count] — 64 fat pointers = 1 KB of stack writes, zero heap, zero data copy (rows are borrows of the capture buffer). The pattern recurs at 5+ in-tree sites (katgpt-rs build_marginals_view returns the array by value as a shared pub helper; the rollback + qwen_deltanet tree steps mirror it per its doc; the engine dflash wrapper and this file inline it) — it is the established bridge idiom of the speculative-decode hot path. Distinct from caller-owned-split-scratch (carves ONE caller-owned MUTABLE scratch via split_at_mut for intermediate+output reuse inside a fn — this is an immutable borrowed-view bridge to a foreign API shape, no mutation, no scratch), from flat-activation-slots-as-readback-target (flattens the readback DESTINATION to kill a scatter copy — here nothing is read back; the flat buffer is already host-side and only the view shape changes, flat-to-nested, the opposite direction), and from ragged-pack-offset-table-fused-dispatch (device-side ragged packing — this is host-side uniform-stride view construction).".to_string(),
            ref_shape: "// naive: heap-allocate the per-row view every call (decode hot path)\nfn submit_rows_naive(buf: &[f32], steps: usize, row_len: usize, sink: &mut dyn FnMut(&[&[f32]])) {\n    let rows: Vec<&[f32]> = (0..steps)\n        .map(|i| &buf[i * row_len..(i + 1) * row_len])\n        .collect(); // one heap alloc per call, 16 B fat pointers\n    sink(&rows);\n}\n".to_string(),
            opt_pattern: "// optimized: fixed-max stack array of borrowed views — zero heap, zero copy\nfn submit_rows_stack(buf: &[f32], steps: usize, row_len: usize, sink: &mut dyn FnMut(&[&[f32]])) {\n    let mut views: [&[f32]; 64] = [&[]; 64];\n    let count = steps.min(64);\n    for (i, slot) in views.iter_mut().enumerate().take(count) {\n        let start = i * row_len;\n        let end = start + row_len;\n        *slot = &buf[start..end];\n    }\n    sink(&views[..count]); // 1 KB of stack writes; rows borrow the flat buffer\n}\n".to_string(),
        },
    ]
}
