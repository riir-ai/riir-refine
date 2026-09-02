//! Entries slice — Batches 14-46 era (rules 249-286 of 340) — riir-gpu production-kernel mining.
//! Pure data moved verbatim from the former corpus.rs monolith
//! (2026-08-15 split); append new batches to the LAST gpu slice or
//! open a new one past ~900 lines. Category membership lives ONLY in
//! `KERNEL_CATEGORIES` (mod.rs) — never encode it here.

use crate::domains::kernel_opt::KernelRule;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<KernelRule> {
    vec![
        // ── non-saturating-outcome-reward-floor — raw 0/1 outcome → affine ramp off the extremes (B) ──
        // Source: riir-gpu edge_lora/go_self_play.rs L410-421 (the doc:
        // "win → 0.85, draw → 0.5, loss → 0.15. The 0.15/0.85 split (rather
        // than 0.0/1.0) keeps the gradient magnitude non-vanishing on losses
        // — losing games still teach "avoid this direction" with
        // non-trivial weight." + the body "0.15 + 0.70 *
        // terminal_reward.clamp(0.0, 1.0)") + bomber_self_play.rs L170-178 +
        // fft_self_play.rs L196-199 (the identical mapping triplicated — the
        // dedup is tracked in the batch's riir-ai issue).
        //
        // Distinct from sign-branch-stable-sigmoid (how to compute a sigmoid
        // without exp overflow — this is whether the outcome map should
        // saturate at all), from additive-sigmoid-gates-not-softmax-routing
        // (composition of multiple gates vs softmax — this is the bounds of a
        // single outcome-to-scalar map), and from
        // sigmoid-deriv-recovered-from-output (derivative reuse — no
        // derivative involved, pure label shaping).
        KernelRule {
            rule_id: "non-saturating-outcome-reward-floor".to_string(),
            gpu_concept: "feeding the raw terminal outcome (0.0 loss / 1.0 win) straight into a reward-weighted training objective puts the weight on every losing episode's gradient at exactly zero — the learner is never taught what to avoid, and win-side targets sit on the objective's saturated edge. Map terminal outcomes through a one-line affine clamp-ramp off the extremes instead: 0.15 + 0.70 · clamp(terminal, 0, 1), bounded in [0,1] like a sigmoid-shaped scalar (never softmax) but never saturating — draws stay centered at 0.5 and losses retain 15% weight, enough for the avoid-this-direction signal to survive. This is label-shaping discipline for reward-weighted regression: the map is monotone, order-preserving, and constant-cost. Distinct from sign-branch-stable-sigmoid (how to compute a sigmoid without exp overflow — this is whether the outcome map should saturate at all), from additive-sigmoid-gates-not-softmax-routing (composition of multiple gates vs softmax — this is the bounds of a single outcome-to-scalar map), and from sigmoid-deriv-recovered-from-output (derivative reuse — no derivative involved, pure label shaping).".to_string(),
            ref_shape: "// naive: identity outcome mapping — losses map to exactly 0.0 and the\n// reward-weighted objective multiplies the loss-side gradient by zero:\n// losing trajectories teach nothing\nfn outcome_raw(terminal: f32) -> f32 {\n    terminal.clamp(0.0, 1.0)\n}\nfn episode_loss_naive(out: f32, target: f32, terminal: f32) -> f32 {\n    let reward = outcome_raw(terminal); // loss -> 0.0\n    reward * (out - target).powi(2) // 0.0 * anything == 0.0 gradient\n}\n".to_string(),
            opt_pattern: "// optimized: affine clamp-ramp off the extremes — losses sit at 0.15,\n// wins at 0.85, draws at 0.5; bounded in [0,1] (sigmoid-style shape,\n// no softmax) but never saturating, so every episode keeps\n// non-zero gradient weight\nfn sigmoid_outcome_reward(terminal_reward: f32) -> f32 {\n    0.15 + 0.70 * terminal_reward.clamp(0.0, 1.0)\n}\nfn episode_loss_floored(out: f32, target: f32, terminal: f32) -> f32 {\n    let reward = sigmoid_outcome_reward(terminal); // loss -> 0.15, not 0.0\n    reward * (out - target).powi(2) // losing moves still teach\n} // \"avoid this direction\" with non-trivial weight\n".to_string(),
        },
        // ── collect-then-resolve-snapshot-batch — interleaved select/resolve → two frozen phases (E) ──
        // Source: riir-gpu edge_lora/fft_self_play.rs L297-300 ("//
        // Collect-then-resolve ordering (deterministic ATB rule from
        // arena_runner). / // Encode BEFORE → select → resolve (mutate) →
        // encode AFTER, per ready unit.") + L316-317 ("// Resolve actions in
        // collected order, emitting one episode per action. / for (unit_id,
        // action, input_enc) in actions {").
        //
        // Distinct from merge-dispatches-into-single-compute-pass (merging
        // dependent GPU dispatches into one pass — this batches decision
        // reads then confines mutation for determinism), from
        // chain-dispatches-single-sync-point (sync amortization — no syncs
        // here; the win is snapshot consistency + parallel select), and from
        // row-owned-register-blocked-recurrence (independence within a
        // recurrence — the independence here is manufactured by freezing the
        // state).
        KernelRule {
            rule_id: "collect-then-resolve-snapshot-batch".to_string(),
            gpu_concept: "in simultaneous-turn loops the naive interleaved form selects and resolves per actor — every actor after the first sees earlier actors' mutations (information leak), outcomes become order-dependent, and the select phase cannot be batched because the state moves underneath it. Split the tick into two phases: a collect phase where every ready unit selects against the identical frozen pre-tick snapshot — the phase is read-only over shared state, so it is embarrassingly parallel and iteration order provably cannot leak information — and a resolve phase that applies the collected actions in fixed order, the single place where mutation happens. This is the phased-execution analogue of read/write phase separation in kernels: batching reads against an immutable view removes false dependencies and buys determinism for free. The episode encode rides the same structure (encode BEFORE in the collect phase, encode AFTER in the resolve phase), so each transition captures exactly one unit's action against the snapshot it was chosen from. Distinct from merge-dispatches-into-single-compute-pass (merging dependent GPU dispatches into one pass — this batches decision reads then confines mutation for determinism), from chain-dispatches-single-sync-point (sync amortization — no syncs here; the win is snapshot consistency + parallel select), and from row-owned-register-blocked-recurrence (independence within a recurrence — the independence here is manufactured by freezing the state).".to_string(),
            ref_shape: "// naive: select-and-resolve interleaved — each actor sees the mutations of\n// every earlier actor in the same tick (information leak, order-dependent\n// outcomes), and select cannot be batched because state moves under it\nfn tick_interleaved(\n    battle: &mut Battle,\n    players: &mut [Player],\n    ready: &[u8],\n    rng: &mut Rng,\n) {\n    for &unit in ready {\n        // battle already mutated by earlier units this tick\n        let action = players[unit as usize].select(unit, battle, rng);\n        resolve_action(battle, unit, &action, rng); // mutates for NEXT selector\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: collect-then-resolve — phase 1 selects ALL actions against\n// the frozen pre-tick snapshot (read-only, order-independent, batchable);\n// phase 2 applies them in fixed collected order (deterministic resolution)\nfn tick_collect_then_resolve(\n    battle: &mut Battle,\n    players: &mut [Player],\n    ready: &[u8],\n    rng: &mut Rng,\n) {\n    // phase 1: SELECT against the frozen state — no mutation in this loop,\n    // so iteration order cannot leak information and the phase parallelizes\n    let mut actions: Vec<(u8, Action, Vec<f32>)> = Vec::with_capacity(ready.len());\n    for &unit in ready {\n        let input_enc = encode(battle, unit); // encode BEFORE, same snapshot\n        let action = players[unit as usize].select(unit, battle, rng);\n        actions.push((unit, action, input_enc));\n    }\n    // phase 2: RESOLVE in collected order — all mutation confined here,\n    // identical outcome for a fixed action list\n    for (unit, action, input_enc) in actions {\n        resolve_action(battle, unit, &action, rng);\n        let target_enc = encode(battle, unit); // encode AFTER\n        record(unit, input_enc, target_enc);\n        if battle.check_winner().is_some() {\n            break;\n        }\n    }\n}\n".to_string(),
        },
        // ── timeout-scalar-tiebreak-decisive-outcome — flat draw on cap → maintained-scalar tiebreak (D) ──
        // Source: riir-gpu edge_lora/fft_self_play.rs L201-205 ("/// On
        // timeout (no winner from `check_winner`), fall back to HP-tiebreak —
        // / /// matches `arena_runner.rs` L120-129 semantics.") + L211-228
        // (the None arm: "// Draw or timeout — HP tiebreak." comparing
        // `state.team_hp(my_team)` vs `state.team_hp(enemy_team)` — decisive
        // 1.0/0.0 except exact tie).
        //
        // Distinct from non-saturating-outcome-reward-floor (bounds of the
        // reward map — this decides the value feeding the map), from
        // predicate-gated-skip-empty-workset-step (eliding whole steps when
        // idle — this salvages signal from a capped step rather than skipping
        // it), and from argmax-rng-tie-break-collect-ties (selection among
        // candidates — this derives the terminal label itself).
        KernelRule {
            rule_id: "timeout-scalar-tiebreak-decisive-outcome".to_string(),
            gpu_concept: "the naive terminal classifier maps every non-win — timeout, true draw, mutual destruction — to the same flat 0.5, so capped games drown the reward signal in uninformative draws no matter how lopsided the state was when the clock expired. Gate the fallback path: when the primary win condition returns None, compare an existing maintained scalar (team HP totals) and emit 1.0/0.0/0.5 — decisive except on an exact tie. The win is signal density at zero maintenance cost: no extra accumulators, no new state, the tiebreak reads two sums the state already tracks, and the semantics stay consistent with the evaluation runner so training episodes and arena scoring agree on what timeout means. Placement matters: the tiebreak belongs on the None arm of the outcome match, not baked into the reward map — the reward map stays a pure monotone function of the terminal value, and this rule decides which terminal value a capped game produces. Distinct from non-saturating-outcome-reward-floor (bounds of the reward map — this decides the value feeding the map), from predicate-gated-skip-empty-workset-step (eliding whole steps when idle — this salvages signal from a capped step rather than skipping it), and from argmax-rng-tie-break-collect-ties (selection among candidates — this derives the terminal label itself).".to_string(),
            ref_shape: "// naive: timeout emits a flat draw — every capped game lands on the same\n// uninformative 0.5 label no matter how lopsided the state was when the\n// clock ran out; the reward signal drowns in draws\nfn terminal_naive(state: &Battle) -> f32 {\n    match state.check_winner() {\n        Some(w) if w == MY_TEAM => 1.0,\n        Some(_) => 0.0,\n        None => 0.5, // timeout AND true draw AND mutual destruction: all 0.5\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: timeout falls through to an EXISTING state scalar tiebreak —\n// the outcome stays decisive (0.5 only on an exact tie), so capped games\n// still carry directional signal with zero extra state maintained\nfn terminal_with_tiebreak(state: &Battle) -> f32 {\n    match state.check_winner() {\n        Some(w) if w == MY_TEAM => 1.0,\n        Some(_) => 0.0,\n        None => {\n            // draw or timeout — HP tiebreak\n            let my_hp = state.team_hp(MY_TEAM); // scalar the state\n            let enemy_hp = state.team_hp(ENEMY); // already maintains\n            if my_hp > enemy_hp {\n                1.0\n            } else if my_hp < enemy_hp {\n                0.0\n            } else {\n                0.5 // exact tie only\n            }\n        }\n    }\n}\n".to_string(),
        },
        // ══ Batch 33 (Research 037): the spectralquant calibration cluster — ══
        // calibration.rs + rotation.rs + qjl.rs (the calibration path
        // Research 016 explicitly deferred; the cluster is bench-only
        // today, sole consumer benches/spectralquant_bench.rs). Host-side
        // wgpu dispatch orchestration + the WGSL kernels they drive. One
        // seed REJECTED during verification: two-stage tile→reduce is NOT
        // what covariance.wgsl does — the producer writes no tile
        // partials, so n_tiles > 1 would reduce uninitialized memory
        // (filed as riir-ai Issue 686 H1, not a rule — encoding it would
        // put a bug in the corpus as wisdom).
        // ── grid-over-output-thread-private-sweep — parallelize-long-axis → grid over output (E) ──
        // Source: riir-gpu spectralquant/calibration.rs L288-290 ("//
        // Workgroup 16×16: dispatch ceil(d_h/16) × ceil(d_h/16)" +
        // dispatch_2d(d_h, d_h, 16, 16)) + covariance.wgsl L50-79 (kernel
        // side: "var accum: f32 = 0.0;" then the loop "for (var i = 0u; i
        // < params.seq_len; i = i + 1u)" sweeping ALL positions
        // in-register, "accum += s_row[local_row] * s_col[local_col];" per
        // position, and ONE final store "output_cov[global_row * d_h +
        // global_col] = accum;" — each output element owned by exactly one
        // thread). The bench consumer pins the single-pass shape
        // (spectralquant_bench.rs L166: "let n_tiles = 1; // single-pass
        // for d_h=128").
        //
        // Distinct from tree-reduction-depth-over-sequential-accumulate and
        // workgroup-equals-plane-native-reduction (both govern reductions
        // that SPAN threads — here sequential accumulation is optimal
        // precisely because no thread ever shares reduction work), from
        // split-reduction-scalar-prepass (splits a scalar reduction OUT
        // into a second pass — this avoids splitting at all by
        // privatizing the sweep), and from single-plane-sufficient-
        // reduction (redundant-plane elimination for cooperative
        // reductions).
        KernelRule {
            rule_id: "grid-over-output-thread-private-sweep".to_string(),
            gpu_concept: "when a reduction's output is a full matrix (a d_h × d_h covariance, Gram matrix, attention statistic) and one input axis is long (seq_len), size the dispatch grid over the OUTPUT elements and keep the long axis entirely thread-private: each output element is owned by exactly one thread, which sweeps the reduction axis sequentially in a register accumulator and stores exactly once at the end — zero cross-thread reduction, zero reduction barriers, no atomics, no partials buffer, no second reduce kernel. This is the dual of split-K: instead of parallelizing the long axis and then combining per-tile partials, parallelize the (short) output axis and serialize the (long) sweep inside the thread — correct whenever the output fits the grid and the per-element sweep fits the time budget, and it removes the entire partials + reduce machinery by construction. Distinct from tree-reduction-depth-over-sequential-accumulate and workgroup-equals-plane-native-reduction (both govern reductions that span threads — sequential accumulation here is optimal precisely because no thread shares reduction work), from split-reduction-scalar-prepass (splits a scalar reduction out into a second pass — this avoids splitting at all by privatizing the sweep), and from single-plane-sufficient-reduction (redundant-plane elimination for cooperative reductions).".to_string(),
            ref_shape: "// naive: parallelize over the LONG axis (sequence tiles), then combine\n// per-element partials across workgroups — atomic contention, or an\n// n_tiles partials buffer + a second reduce kernel; every output\n// element is written by many threads\nfn covariance_split_axis(x: &[f32], seq_len: usize, d_h: usize,\n                         n_tiles: usize, cov: &mut [f32]) {\n    for tile in 0..n_tiles {\n        // parallel over seq_len tiles: each computes a PARTIAL matrix\n        let per_tile = seq_len / n_tiles;\n        for row in 0..d_h {\n            for col in 0..d_h {\n                let mut partial = 0.0f32;\n                for i in tile * per_tile..(tile + 1) * per_tile {\n                    partial += x[i * d_h + row] * x[i * d_h + col];\n                }\n                cov[row * d_h + col] += partial; // cross-workgroup combine\n            }\n        }\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: grid over the OUTPUT matrix (dispatch_2d(d_h, d_h, 16, 16));\n// each output element has exactly ONE owning thread, which sweeps ALL\n// seq_len positions in a register accumulator and stores once — zero\n// cross-thread reduction, no partials buffer, no reduce kernel\nfn covariance_output_owned(x: &[f32], seq_len: usize, d_h: usize,\n                           cov: &mut [f32]) {\n    // dispatch: one workgroup per output tile; thread (row, col) owns\n    // cov[row][col] for its whole lifetime\n    for row in 0..d_h {\n        for col in 0..d_h {\n            let mut accum = 0.0f32; // register-private per element\n            for i in 0..seq_len {\n                // the long axis swept IN-THREAD (covariance.wgsl L53-74)\n                accum += x[i * d_h + row] * x[i * d_h + col];\n            }\n            cov[row * d_h + col] = accum; // each element written ONCE\n        }\n    }\n}\n".to_string(),
        },
        // ── buffer-set-struct-two-tier-dispatch — loose buffers per call → buffer-set struct + preferred entry (D) ──
        // Source: riir-gpu spectralquant/rotation.rs L74-79 ("/// GPU
        // buffers for eigenbasis rotation dispatch. ... /// Created once
        // per (layer, head) calibration; reused across sequence
        // positions.") + L198-205 ("/// Dispatch eigenbasis rotation
        // kernel using pre-allocated buffers. ... /// This is the
        // preferred dispatch method — reuses pre-allocated GPU buffers /
        // /// from [GpuEigenbasisRotationBuffers].") + L212-216 (the
        // preferred path only WRITES the uniform — write_buffer, no
        // create) vs L263-269 (the raw path: create_uniform_buffer +
        // write_buffer per call). Same shape in qjl.rs L186-200.
        //
        // Distinct from prebuilt-bind-groups-fixed-buffer-combos (that
        // rule pre-builds the bind-group OBJECTS at init for fixed
        // combos — this rule's reuse unit is the buffer-set struct
        // itself plus the two-tier API discipline; note the shipped
        // code still creates its bind group per call, a missed
        // application of that adjacent rule, filed as Issue 686 H4).
        KernelRule {
            rule_id: "buffer-set-struct-two-tier-dispatch".to_string(),
            gpu_concept: "hoist ALL device buffers a kernel family needs (input, immutable operand, output, uniform) into ONE struct whose lifetime is bound to the config (created once per layer/head/dims), and expose a two-tier dispatch API: the preferred entry takes the pre-allocated struct and only writes uniform CONTENT (write, never allocate), while the raw entry takes loose buffers and self-allocates a uniform per call — a convenience for cold/one-shot callers whose per-call device allocation is exactly what makes it secondary. The struct is the reuse unit: buffer identity is stable across dispatches, so bind groups over it are also stable (prebuilt-bind-groups is the natural next step once combos are fixed). Distinct from prebuilt-bind-groups-fixed-buffer-combos (that rule pre-builds the bind-group objects at init — this rule's unit is the buffer-set struct plus the two-tier API discipline that makes the allocating path explicitly secondary).".to_string(),
            ref_shape: "// naive: the ONLY entry takes loose buffers and allocates a uniform\n// buffer on EVERY dispatch — no reuse unit; hot and cold callers pay\n// the same per-call device allocation\nfn dispatch(gpu: &Gpu, encoder: &mut Encoder, input: &Buffer, vt: &Buffer,\n            out: &Buffer, cfg: &Cfg) {\n    let bytes = bytemuck::cast_slice(std::slice::from_ref(cfg));\n    let uniform = create_uniform_buffer(&gpu.device, bytes, \"cfg\"); // per call\n    gpu.queue.write_buffer(&uniform, 0, bytes);\n    let bg = simple_bind_group(&gpu.device, &layout,\n        &[(0, input), (1, vt), (2, out), (3, &uniform)]);\n    dispatch_pass(encoder, &bg, cfg.seq_len, cfg.n_heads);\n}\n".to_string(),
            opt_pattern: "// optimized: the buffer SET is a struct created once per config; the\n// preferred entry takes the struct and only WRITES uniform content —\n// the raw loose-buffer entry remains for cold/one-shot callers\nstruct RotationBuffers {\n    input_kv: Buffer,\n    eigenbasis: Buffer,\n    output: Buffer,\n    uniform: Buffer,\n}\n// RotationBuffers::new(device, queue, dims, vt_data) creates all four ONCE\n\nfn dispatch_buffers(gpu: &Gpu, encoder: &mut Encoder,\n                    bufs: &RotationBuffers, cfg: &Cfg) {\n    // preferred entry: write, not allocate\n    let bytes = bytemuck::cast_slice(std::slice::from_ref(cfg));\n    gpu.queue.write_buffer(&bufs.uniform, 0, bytes);\n    let bg = simple_bind_group(&gpu.device, &layout,\n        &[(0, &bufs.input_kv), (1, &bufs.eigenbasis),\n          (2, &bufs.output), (3, &bufs.uniform)]);\n    dispatch_pass(encoder, &bg, cfg.seq_len, cfg.n_heads);\n}\n\nfn dispatch(gpu: &Gpu, encoder: &mut Encoder, input: &Buffer, vt: &Buffer,\n            out: &Buffer, cfg: &Cfg) {\n    // cold path: self-allocates a uniform + bind group per call\n    let _ = (gpu, encoder, input, vt, out, cfg);\n}\n".to_string(),
        },
        // ── slot-offset-partial-buffer-write — full re-upload → computed-offset delta write (A) ──
        // Source: riir-gpu spectralquant/rotation.rs L146-160
        // (upload_input: "let offset = ((pos * n_heads + head) * d_h *
        // std::mem::size_of::<f32>()) as u64;" +
        // "queue.write_buffer(&self.input_kv, offset,
        // bytemuck::cast_slice(data));" — d_h×4 bytes written, not the
        // whole seq_len*n_heads*d_h tensor) + eigenbasis_rotation.wgsl
        // L44-45 (the kernel-side twin the offset must mirror: "// Base
        // offset into flat [seq_len * n_heads * d_h] for this (pos,
        // head)" + "let base = pos * total_dim + head * d_h;").
        //
        // Distinct from mapped-at-creation-init-upload (a one-shot
        // FULL-buffer initialization mechanism at creation — this is
        // incremental delta updates into an already-live buffer during
        // steady state) and from per-slot-uniform-buffer-write-ordering
        // (uniform slots racing one encoder — this is a storage-tensor
        // slot update; distinct slots cannot collide).
        KernelRule {
            rule_id: "slot-offset-partial-buffer-write".to_string(),
            gpu_concept: "keep the destination tensor resident in one persistent storage buffer and update only the changed slot via queue.write_buffer at a computed byte offset, instead of re-uploading the whole tensor: one (position, head) slot of a [seq_len × n_heads × d_h] KV tensor is d_h×4 bytes; the full tensor is seq_len×n_heads×d_h×4 — the delta write is n_heads×seq_len× cheaper. The offset formula is the host-side mirror of the kernel's indexing base (pos * total_dim + head * d_h), so a slot written by the host lands exactly where the kernel reads it — the same flat-layout contract, evaluated on both sides of the boundary. Distinct from mapped-at-creation-init-upload (a one-shot full-buffer initialization mechanism at creation — this is incremental delta updates into an already-live buffer during steady state) and from per-slot-uniform-buffer-write-ordering (uniform slots racing one encoder — this is a storage-tensor slot update; distinct slots cannot collide).".to_string(),
            ref_shape: "// naive: refresh the ENTIRE KV tensor when one (pos, head) slot changes\nfn upload_slot_full(queue: &Queue, buf: &Buffer, kv_flat: &[f32],\n                    pos: usize, head: usize, n_heads: usize,\n                    d_h: usize, data: &[f32]) {\n    let mut flat = kv_flat.to_vec(); // whole-tensor copy on the host\n    let base = (pos * n_heads + head) * d_h;\n    flat[base..base + d_h].copy_from_slice(data);\n    // seq_len * n_heads * d_h * 4 bytes over the bus for ONE slot\n    queue.write_buffer(buf, 0, bytemuck::cast_slice(&flat));\n}\n".to_string(),
            opt_pattern: "// optimized: persistent buffer + byte-offset delta write of just the\n// slot; the offset mirrors the kernel's base = pos * total_dim + head * d_h\nstruct RotationBuffers {\n    input_kv: Buffer,\n    _pad: u8,\n}\nimpl RotationBuffers {\n    pub fn upload_input(&self, queue: &Queue, pos: usize, head: usize,\n                        n_heads: usize, d_h: usize, data: &[f32]) {\n        let offset =\n            ((pos * n_heads + head) * d_h * std::mem::size_of::<f32>()) as u64;\n        // d_h * 4 bytes over the bus — that is the whole update\n        queue.write_buffer(&self.input_kv, offset, bytemuck::cast_slice(data));\n    }\n}\n".to_string(),
        },
        // ── immutable-operand-upload-at-set-creation — per-dispatch operand upload → constructor-owned once (A) ──
        // Source: riir-gpu spectralquant/rotation.rs L121-129 (new():
        // "let eigenbasis = { let buf = make_storage(d_h * d_h,
        // \"sq_rot_eigenbasis\"); queue.write_buffer(&buf, 0,
        // bytemuck::cast_slice(eigenbasis_data)); buf };" while input_kv
        // and output are created EMPTY) + L79 ("/// Created once per
        // (layer, head) calibration; reused across sequence positions.")
        // + calibration.rs L187-199 (upload_calibration: the same
        // create-then-upload-once shape for eigenbasis, per_dim_bits,
        // codebooks) + qjl.rs L126-130 (sign_matrix uploaded inside
        // GpuSelectiveQjlBuffers::new).
        //
        // Distinct from mapped-at-creation-init-upload (the upload
        // MECHANISM — mapped_at_creation avoiding staging; this rule is
        // the upload PLACEMENT: bind the operand to the buffer-set's
        // lifetime, whichever mechanism writes it) and from
        // host-cache-immutable-device-weights (the host keeps a copy to
        // avoid device DOWNLOADS — this avoids device re-UPLOADS of a
        // never-changing operand).
        KernelRule {
            rule_id: "immutable-operand-upload-at-set-creation".to_string(),
            gpu_concept: "bind an immutable-per-config operand (the eigenbasis V^T matrix, calibration codebooks, per-dim bit widths, a Rademacher sign matrix) to the buffer-set's lifetime: upload it exactly once inside the constructor, immediately after creating its buffer, while mutable buffers (input, output) are created empty. The dispatch path never sees the operand's bytes again — no per-dispatch re-upload, and operand freshness becomes the constructor's guarantee rather than every caller's responsibility. The constructor is the single place where config-derived constants cross the bus. Distinct from mapped-at-creation-init-upload (the upload mechanism — mapped_at_creation avoiding staging; this rule is the upload placement: bind the operand to the set's lifetime, whichever mechanism writes it) and from host-cache-immutable-device-weights (the host keeps a copy to avoid device downloads — this avoids device re-uploads of a never-changing operand).".to_string(),
            ref_shape: "// naive: the never-changing V^T matrix crosses the bus on EVERY dispatch\nfn rotate(gpu: &Gpu, encoder: &mut Encoder, input: &Buffer,\n          out: &Buffer, vt: &[f32], cfg: &Cfg) {\n    let vt_buf = gpu.device.create_buffer(&desc(vt.len() * 4));\n    gpu.queue.write_buffer(&vt_buf, 0, bytemuck::cast_slice(vt)); // every call\n    dispatch_rotation(gpu, encoder, input, &vt_buf, out, cfg);\n}\n".to_string(),
            opt_pattern: "// optimized: the constructor owns the immutable operand + uploads ONCE;\n// mutable buffers are created empty — the dispatch path never sees vt bytes\nstruct RotationBuffers {\n    input_kv: Buffer,\n    eigenbasis: Buffer,\n    output: Buffer,\n}\nimpl RotationBuffers {\n    pub fn new(device: &Device, queue: &Queue, seq_len: usize,\n               d_h: usize, n_heads: usize, vt: &[f32]) -> Self {\n        let input_kv = make_storage(device, seq_len * n_heads * d_h); // empty\n        let eigenbasis = {\n            let buf = make_storage(device, d_h * d_h);\n            queue.write_buffer(&buf, 0, bytemuck::cast_slice(vt)); // ONCE\n            buf\n        };\n        let output = make_storage(device, seq_len * n_heads * d_h); // empty\n        Self { input_kv, eigenbasis, output }\n    }\n}\n".to_string(),
        },
        // ── selective-projection-effective-rank-head — project all d_h → project top d_eff only (B) ──
        // Source: riir-gpu spectralquant/qjl.rs L3+L5 ("//! Applies JL
        // projection only on the top d_eff coordinates of rotated
        // vectors." + "//! Bottom (d_h - d_eff) coords are NOT projected
        // — quantized separately at b_low bits.") + L166-168 ("/// The
        // sign matrix is (qjl_dim × d_eff) Rademacher ±1 — much smaller
        // than / /// full (qjl_dim × d_h) when d_eff ≈ 4–6.") +
        // selective_qjl.wgsl L57-62 ("// Only sum over top d_eff
        // coordinates, NOT full d_h" + "for (var j = 0u; j < d_eff; j =
        // j + 1u) { sum += sign_matrix[out_dim * d_eff + j] *
        // input[input_base + j]; }") + L13-14 ("// The sign matrix is
        // small: (qjl_dim × d_eff) where d_eff ≈ 4-6, qjl_dim ≈ 32, / //
        // so the inner loop is short and register-friendly.").
        //
        // Distinct from water-fill-tail-dim-shared-codebook (allocates
        // BITS by spectral importance — this allocates PROJECTION WORK
        // by spectral importance: the operator itself shrinks from
        // [qjl_dim × d_h] to [qjl_dim × d_eff]) and from
        // block-compression-before-routing (selection from coarse block
        // POOLING — here selection comes from a prior transform's energy
        // concentration, no pooling pass).
        KernelRule {
            rule_id: "selective-projection-effective-rank-head".to_string(),
            gpu_concept: "when a preceding concentrating transform (an eigenbasis rotation) has already packed the energy into the top d_eff coordinates, apply the random projection operator ONLY to those top coordinates — the (d_h − d_eff) tail bypasses projection entirely and quantizes directly at low bits in a separate path. Projection FLOPs and projection-matrix size scale with the effective rank (d_eff ≈ 4–6), not the ambient dimension (d_h = 128): the inner loop shrinks from 128 to ~6 multiply-adds (~21×) and the projection operator shrinks from [qjl_dim × d_h] = 4096 to [qjl_dim × d_eff] = 192 f32 (~21×, cache-trivially resident). The general shape: after any spectral/energy-concentrating transform, gate downstream expensive operators by the transform's effective rank instead of running them over the whole ambient space. Distinct from water-fill-tail-dim-shared-codebook (allocates bits by spectral importance — this allocates projection work by spectral importance: the operator itself shrinks) and from block-compression-before-routing (selection from coarse block pooling — here selection comes from a prior transform's energy concentration, no pooling pass).".to_string(),
            ref_shape: "// naive: full-dimension JL projection — the projection operator spans\n// ALL d_h coordinates, so the sign matrix is [qjl_dim × d_h] and every\n// output element reduces over the whole vector even though the\n// rotation already concentrated the energy into the top few coords\nfn project_full(input: &[f32], signs: &[f32], qjl_dim: usize,\n                d_h: usize, scale: f32) -> Vec<f32> {\n    // signs: [qjl_dim * d_h] = 32 * 128 = 4096 f32\n    let mut out = vec![0.0; qjl_dim];\n    for m in 0..qjl_dim {\n        let mut sum = 0.0;\n        for j in 0..d_h {\n            sum += signs[m * d_h + j] * input[j]; // d_h = 128 MACs\n        }\n        out[m] = sum * scale;\n    }\n    out\n}\n".to_string(),
            opt_pattern: "// optimized: the rotation concentrated the energy in the top d_eff\n// coords — project ONLY those; the tail bypasses projection (quantized\n// directly at b_low bits in a separate kernel). The operator shrinks\n// to [qjl_dim × d_eff]; the inner loop is d_eff (~6) long, not d_h (128)\nfn project_selective(input: &[f32], signs: &[f32], qjl_dim: usize,\n                     d_eff: usize, scale: f32) -> Vec<f32> {\n    // signs: [qjl_dim * d_eff] = 32 * 6 = 192 f32 (~21× smaller)\n    let mut out = vec![0.0; qjl_dim];\n    for m in 0..qjl_dim {\n        let mut sum = 0.0;\n        for j in 0..d_eff {\n            // only the top d_eff coordinates participate\n            sum += signs[m * d_eff + j] * input[j];\n        }\n        out[m] = sum * scale;\n    }\n    out\n    // tail input[d_eff..d_h] is never read here: it feeds the low-bit\n    // quantizer directly — no projection work, no projection storage\n}\n".to_string(),
        },
        // ── derived-scalar-host-computed-in-uniform — per-thread/device recompute → host-once + uniform (N) ──
        // Source: riir-gpu spectralquant/qjl.rs L48-49 ("/// JL correction
        // scale: `sqrt(d_eff / qjl_dim)`." + "pub scale: f32,") + L59
        // ("let scale = (d_eff as f32 / qjl_dim as f32).sqrt();" inside
        // SelectiveQjlConfig::new — host, once, at construction) +
        // selective_qjl.wgsl L22 ("scale: f32, // sqrt(d_eff / qjl_dim)
        // for JL correction") + L66 ("output[output_base + out_dim] =
        // sum * config.scale;" — the kernel READS the uniform field; no
        // sqrt anywhere in the kernel body).
        //
        // Distinct from precompute-scalar-reduction (hoists a
        // DATA-dependent kernel row-reduction out of a per-block loop —
        // this derives a CONFIG-derived constant on the host before any
        // data exists), from pack-scalar-params-into-combined-buffer
        // (packing layout for the array budget — same computation site,
        // different concern), and from broadcast-uniform-skip-reduce
        // (thread-uniform values must not be REDUCED — this is about
        // where the value is COMPUTED).
        KernelRule {
            rule_id: "derived-scalar-host-computed-in-uniform".to_string(),
            gpu_concept: "a scalar derived purely from config/shape metadata (a JL correction sqrt(d_eff/qjl_dim), a stride, a scale factor) is computed ONCE on the host at config construction and shipped as a uniform field; the kernel reads it and never recomputes. The perf half is trivial — the load-bearing half is cross-implementation bit-identity: with a single computation site, the CPU reference and the GPU kernel consume bit-identical f32 bits by construction. WGSL's mandated f32 sqrt accuracy is 2.5 ULP (not correctly rounded), so an in-shader sqrt can legitimately differ from the host's — CPU-vs-GPU parity assertions would drift by ulps for reasons no code review would catch. One computation site is one source of truth: bit-stable parity, zero per-thread recompute. Distinct from precompute-scalar-reduction (hoists a data-dependent kernel row-reduction out of a per-block loop — this derives a config-derived constant on the host before any data exists), from pack-scalar-params-into-combined-buffer (packing layout for the array budget — same computation site, different concern), and from broadcast-uniform-skip-reduce (thread-uniform values must not be reduced — this is about where the value is computed).".to_string(),
            ref_shape: "// naive: each consumer recomputes the derived correction scalar itself.\n// An in-shader sqrt is only 2.5-ULP accurate in WGSL — it can differ\n// from the host's correctly-rounded sqrt, so CPU reference and GPU\n// output drift by ulps and parity tests flap. Every thread burns a sqrt.\nfn kernel_side(d_eff: u32, qjl_dim: u32, sums: &[f32]) -> Vec<f32> {\n    sums.iter()\n        .map(|&s| {\n            let scale = (d_eff as f32 / qjl_dim as f32).sqrt(); // per-thread\n            s * scale // GPU sqrt != host sqrt (ulp-level) -> parity drift\n        })\n        .collect()\n}\n".to_string(),
            opt_pattern: "// optimized: compute the config-derived scalar ONCE on the host at\n// config construction; ship it as a uniform field. One computation\n// site means the CPU reference and the WGSL kernel consume\n// bit-identical f32 bits.\nstruct Config {\n    d_eff: u32,\n    qjl_dim: u32,\n    scale: f32, // host-computed — the single source of truth\n}\nimpl Config {\n    fn new(d_eff: u32, qjl_dim: u32) -> Self {\n        Self {\n            d_eff,\n            qjl_dim,\n            scale: (d_eff as f32 / qjl_dim as f32).sqrt(),\n        }\n    }\n}\nfn kernel_side(config: &Config, sums: &[f32]) -> Vec<f32> {\n    // the kernel reads the uniform field, never recomputes\n    sums.iter().map(|&s| s * config.scale).collect()\n}\n".to_string(),
        },
        // ── rademacher-signs-over-gaussian-projection — Gaussian entries → ±1 entries (B) ──
        // Source: riir-gpu spectralquant/qjl.rs L4 ("//! Uses Rademacher ±1
        // random signs (not Gaussian) for the sign matrix.") + L84 ("///
        // Rademacher ±1 sign matrix: `[qjl_dim * d_eff]` f32 (storage,
        // read).") + selective_qjl.wgsl L8 + L28 ("sign_matrix:
        // array<f32>; // [qjl_dim * d_eff] (±1.0)") +
        // spectralquant_bench.rs L264-267 ("// Rademacher ±1 sign
        // matrix" + "let sign_matrix: Vec<f32> = (0..QJL_DIM * D_EFF as
        // usize)" + ".map(|_| if fastrand::bool() { 1.0 } else { -1.0
        // })" + ".collect();" — one coin flip per entry).
        //
        // Honest caveat: the shipped kernel does a plain f32 multiply
        // (selective_qjl.wgsl L61) — the conditional-negate strength
        // reduction is an unrealized follow-up, not a shipped win; the
        // exactness argument is what ships today. Distinct from
        // deterministic-tiling-projection-tables (deterministic
        // CONSTRUCTION of fixed projections — this is the entry
        // DISTRIBUTION choice) and from the packing rules (per the
        // bool-fields-over-bitset-hot-reads regime boundary, the 768 B
        // sign matrix is correctly NOT packed).
        KernelRule {
            rule_id: "rademacher-signs-over-gaussian-projection".to_string(),
            gpu_concept: "for random projection matrices, prefer two-valued Rademacher ±1 entries over Gaussian entries when the math license exists (the JL guarantee holds up to constants for ±1 — Achlioptas' database-friendly construction). Three concrete wins: (1) generation is one coin flip per entry (one random bit) instead of a Gaussian sampler (several RNG words + Box-Muller/Ziggurat transcendentals); (2) ±1.0 is exact in f32, so both the stored coefficient and every product s×x are exact — a Gaussian coefficient and its products each round; (3) in a hotter consumer the multiply strength-reduces to a conditional negate / sign-bit flip (an unrealized follow-up in the source kernel, which does a plain multiply — recorded as the licensed direction, not a shipped win). Distinct from deterministic-tiling-projection-tables (deterministic construction of fixed projections — this is the entry distribution choice) and from the packing rules (per the bool-fields-over-bitset-hot-reads regime boundary, the 768-byte sign matrix is correctly not packed).".to_string(),
            ref_shape: "// naive: Gaussian projection entries — each entry needs a Gaussian\n// sample (Box-Muller/Ziggurat: several RNG words + transcendentals),\n// and the f32 coefficient is itself rounded, so coefficient * x rounds\n// twice.\nfn gaussian_signs(n: usize) -> Vec<f32> {\n    (0..n).map(|_| gaussian_sample()).collect() // expensive per-entry RNG\n}\nfn project(input: &[f32], signs: &[f32]) -> f32 {\n    signs.iter().zip(input).map(|(s, x)| s * x).sum() // inexact product\n}\n".to_string(),
            opt_pattern: "// optimized: Rademacher ±1 entries — the JL guarantee holds up to\n// constants (Achlioptas); generation is one coin flip per entry, ±1.0\n// is exact in f32, and s * x is exact (sign-bit-only effect) —\n// strength-reducible to a conditional negate in a hotter consumer\nfn rademacher_signs(n: usize) -> Vec<f32> {\n    (0..n)\n        .map(|_| if coin_flip() { 1.0 } else { -1.0 }) // 1 random bit/entry\n        .collect()\n}\nfn project(input: &[f32], signs: &[f32]) -> f32 {\n    // as shipped: plain multiply (exact for ±1 operands)\n    signs.iter().zip(input).map(|(s, x)| s * x).sum()\n}\n".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 34 (Research 038): riir-gpu production kernels — the Gemma-4
        // training cluster (gemma4_cubecl_train/: mod.rs + backward_gpu.rs —
        // hybrid GPU training forward + Phase 2.5 GPU backward; the training
        // counterpart of Batch 25's inference-side Gemma cluster). 9 rules.
        // Bug yield filed as riir-ai Issue 687 (mirrored-ring backward OOB
        // for seq_len > 2*sw + backward-handle staleness/LoRA doc
        // contradiction + Phase 2 processes zero-grad positions + the
        // alloc/round-trip storms + 9 more).
        // ════════════════════════════════════════════════════════════════════
        // ── exact-op-parity-for-analytic-backward — fast approx → exact op in the training forward (N) ──
        // Source: riir-gpu gemma4_cubecl_train/mod.rs L44-48 ("//! The
        // training forward uses **standard tanh** (not `fast_tanh`) for
        // GeGLU and logit softcapping — ... so the analytic backward's
        // tanh-based derivative is exact."), L198-205 (softcap site:
        // `*v = cap * (*v / cap).tanh();` with the "STANDARD tanh, not
        // fast_tanh" comment), L375-379 (GeGLU site), L459-470
        // (geglu_standard_tanh helper); consumer side riir-engine
        // gemma4_train/backward.rs L268-272 (`let t_val = arg.tanh();
        // g_dot[ti] = g_s[ti] * scale * (1.0 - t_val * t_val);`) and
        // gegelu_backward L80-89 — the backward differentiates standard
        // tanh, so the training forward MUST evaluate it.
        //
        // Doc caveat (riir-ai Issue 687 H13): the header's contrast claim
        // ("the inference forward (Phase 1) uses fast_tanh") is FALSE for
        // the GPU path — gemma4_cubecl's gelu_tanh + softcap both call
        // `.tanh()`; fast_tanh exists only in the CPU inference softcap
        // (riir-engine gemma4.rs). The RULE stands on the code + the CPU
        // contrast; the header misattribution is filed separately.
        // Distinct from dtype-restricted-backend (dtype-gated fast paths
        // — this gates on train-vs-infer PHASE), from fast-math-
        // contraction-behind-feature-flag (bit-identity across builds —
        // this is gradient correctness across forward/backward op pairs),
        // and from derived-scalar-host-computed-in-uniform (host/kernel
        // parity — this is forward/backward parity).
        KernelRule {
            rule_id: "exact-op-parity-for-analytic-backward".to_string(),
            gpu_concept: "when a hand-written analytic backward differentiates a SPECIFIC implementation of an op (d/dx[c·tanh(x/c)] = 1−tanh²(x/c)), the training forward must evaluate that exact op — reusing an inference-speed approximation (fast_tanh, polynomial) makes the forward value and its derivative come from two different functions, so every gradient is systematically biased wherever the approximation deviates (worst mid-range |x|), and a finite-difference gradient oracle only passes to the approximation error. The training path pays the slower exact op; the inference path keeps the approximation. Distinct from dtype-restricted-backend (gates the fast path on dtype — this gates on train-vs-infer phase), from fast-math-contraction-behind-feature-flag (bit-identity across builds — this is gradient correctness across the forward/backward op pair), and from derived-scalar-host-computed-in-uniform (host/kernel scalar parity — this is forward/backward op parity).".to_string(),
            ref_shape: "// naive: the training forward reuses the INFERENCE activation helper\n// (fast_tanh / polynomial approximation \"for speed\") while the analytic\n// backward differentiates the true tanh:\n//   forward:  y = fast_gelu(g)          // ~= gelu(g) but NOT equal\n//   backward: dy/dg = f'(true tanh)     // derivative of a DIFFERENT fn\nfn geglu_train_naive(gate: &[f32], up: &[f32], out: &mut [f32]) {\n    for ((o, &g), &u) in out.iter_mut().zip(gate.iter()).zip(up.iter()) {\n        *o = fast_tanh_gelu(g) * u; // forward != what backward differentiates\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: a dedicated training-path activation that evaluates the\n// EXACT op the analytic backward differentiates. Slower op, exact\n// gradient: the softcap backward g = scale * (1 - tanh^2(x/cap))\n// assumes the forward computed cap*tanh(x/cap).\nfn geglu_standard_tanh(gate: &[f32], up: &[f32], out: &mut [f32]) {\n    const SQRT_2_OVER_PI: f32 = 0.797_884_6;\n    for ((o, &g), &u) in out.iter_mut().zip(gate.iter()).zip(up.iter()) {\n        let inner = SQRT_2_OVER_PI * (g + 0.044715 * g * g * g);\n        let t = inner.tanh(); // true tanh — matches backward's (1 - t*t)\n        *o = 0.5 * g * (1.0 + t) * u;\n    }\n}\n".to_string(),
        },
        // ── mirror-ring-store-sliding-window-contiguity — single ring + wrap assembly → mirrored double-store (A) ──
        // Source: riir-gpu gemma4_cubecl_train/mod.rs L473-500
        // (store_kv_into_multi_cache: "writes at `pos % sw` AND mirrors to
        // `pos % sw + sw`, so any window of ≤ `sw` contiguous positions is
        // always contiguous in the buffer"); the substrate contract
        // katgpt-transformer kv_cache.rs L57-61 + L186-188 (capacity
        // `2 * sw * kvd`, "Mirrored: 2× sliding_window for
        // contiguous-window reads"); the consumer riir-engine
        // gemma4_train/mod.rs L618-622 (`let read_start = (t_start %
        // sw_cache) * kvd;` then ONE contiguous slice — zero wrap
        // handling). The contrast that proves the tradeoff in the SAME
        // crate: gemma4_cubecl/kv_cache.rs get_combined_kv_window
        // L110-124 — the un-mirrored single ring pays the wrap-assembly
        // branch (primary + wrapped extends) whenever the window crosses
        // the seam.
        //
        // Invariant (must hold): raw-index reads are valid only while
        // every attended t < 2·sw (seq_len ≤ 2·sliding_window) — no
        // assertion guards it (riir-ai Issue 687 H1). Distinct from
        // combined-kv-buffer-keys-then-values (upload layout — composes
        // with this) and from the IR modulo-strength-reduction rules
        // (compile % faster — this REMOVES the modulo from the read path
        // via layout).
        KernelRule {
            rule_id: "mirror-ring-store-sliding-window-contiguity".to_string(),
            gpu_concept: "a sliding-window KV ring of logical capacity sw can be physically allocated as 2·sw slots with each write landing at BOTH pos % sw and pos % sw + sw, so any attended window of ≤ sw positions is contiguous by construction — readers take one contiguous slice (or raw t·kvd indexing valid while t < 2·sw) and never pay per-element modulo or wrap-assembly branches in the hot attention loop. Cost: 2× KV memory + a second bounded memcpy per store. The un-mirrored single ring pays the wrap branch at every seam-crossing window (the same crate's GPU cache does exactly that — the contrast proves the tradeoff). Invariant: raw-index reads require seq_len ≤ 2·sliding_window; enforce it with an assert at the boundary. Distinct from combined-kv-buffer-keys-then-values (the [keys|values] upload layout — composes with this) and from IR modulo strength reduction (compiles % faster — this deletes the modulo from the read path via layout).".to_string(),
            ref_shape: "// naive: single ring of capacity sw — every read window that crosses\n// the ring seam must be assembled from two disjoint slices (or indexed\n// with per-element modulo); the wrap branch lives in the hot loop.\nfn store_kv_naive(key: &mut [f32], k: &[f32], pos: usize, sw: usize, kvd: usize) {\n    let ring = pos % sw;\n    key[ring * kvd..ring * kvd + kvd].copy_from_slice(k); // one copy\n}\nfn read_window_naive(key: &[f32], t_start: usize, n_pos: usize, sw: usize, kvd: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n_pos * kvd];\n    for ti in 0..n_pos {\n        let t = (t_start + ti) % sw; // per-element modulo in the hot loop\n        out[ti * kvd..ti * kvd + kvd].copy_from_slice(&key[t * kvd..t * kvd + kvd]);\n    }\n    out\n}\n".to_string(),
            opt_pattern: "// optimized: mirrored ring — write BOTH halves (2*sw slots); any\n// window of <= sw positions starting at (t_start % sw) is ONE contiguous\n// slice. Invariant: raw-index reads are valid while t < 2*sw, i.e.\n// seq_len <= 2*sliding_window — assert it at the boundary.\nfn store_kv_mirrored(key: &mut [f32], v: &mut [f32], k: &[f32], val: &[f32], pos: usize, sw: usize, kvd: usize) {\n    let ring_pos = pos % sw;\n    let primary_off = ring_pos * kvd;\n    let mirror_off = (ring_pos + sw) * kvd; // second copy at +sw\n    key[primary_off..primary_off + kvd].copy_from_slice(k);\n    key[mirror_off..mirror_off + kvd].copy_from_slice(k);\n    v[primary_off..primary_off + kvd].copy_from_slice(val);\n    v[mirror_off..mirror_off + kvd].copy_from_slice(val);\n}\nfn read_window_mirrored(key: &[f32], t_start: usize, n_pos: usize, sw: usize, kvd: usize) -> &[f32] {\n    let start = (t_start % sw) * kvd;\n    &key[start..start + n_pos * kvd] // zero wrap handling\n}\n".to_string(),
        },
        // ── split-dispatch-expose-fused-intermediate — re-launch producer → split entry returning both outputs (D) ──
        // Source: riir-gpu gemma4_cubecl_train/mod.rs L325-344 — the file
        // documents its own anti-pattern: "The GPU fused attention+Wo
        // path doesn't expose attn_out separately. ... To get attn_out, we
        // re-run just the attention (no Wo) on GPU. This is one extra GPU
        // launch per layer — acceptable for training (not the hot
        // inference path). The alternative (split dispatch like Gemma-2's
        // `dispatch_attention_wo_split`) would be cleaner"; upstream
        // gemma4_cubecl/dispatch.rs L152 ("Returns Wo output `[n_embd]`")
        // + L176-214 (attn_out_handle written by attention, consumed by
        // the Wo GEMV, NEVER read back — discarded); the fix exists as
        // the proven sibling precedent gemma2_cubecl/dispatch.rs L353-362
        // (dispatch_attention_wo_split: "returns BOTH `attn_out` (q_dim)
        // and `wo_out` (n_embd) ... Used when LoRA is active so the
        // O-projection LoRA delta can be applied with `attn_out` as
        // input").
        //
        // Cost of the re-run (riir-ai Issue 687 H8): per layer per
        // position the re-dispatch rebuilds the combined KV window
        // (+1 CPU copy), re-uploads Q + KV (+2 uploads), re-launches
        // attention (+1 launch), adds its own GPU→CPU sync (+1 sync) —
        // ×48 layers × seq_len per step. Distinct from
        // merge-dispatches-into-single-compute-pass (merging passes —
        // this is the RETURN SURFACE of an already-fused chain) and from
        // fan-out-shared-input-single-sync (N siblings of one input —
        // this is ONE chain with two outputs of different lifetimes).
        KernelRule {
            rule_id: "split-dispatch-expose-fused-intermediate".to_string(),
            gpu_concept: "a fused dispatch chain (attention → Wo) that returns only the final output makes its intermediate invisible to callers who need it (activation saving for an analytic backward, LoRA insertion at the chain seam, tracing) — forcing them to re-launch the whole producer. Provide a split entry that returns both outputs from ONE execution (both read in the same sync); the fused entry stays the default for consumers that don't need the intermediate. The intermediate already exists on-device — the fused entry reads only the chain tail and discards it; the split entry is the same two kernels plus one extra readback, never a second launch + second KV-window build + second sync. Distinct from merge-dispatches-into-single-compute-pass (merging dependent dispatches into fewer passes — this is the return surface of one already-fused chain) and from fan-out-shared-input-single-sync (N sibling consumers of one input — this is one chain with two outputs of different lifetimes).".to_string(),
            ref_shape: "// naive: the fused dispatch returns only the chain's final output; a\n// consumer that also needs the intermediate (training saves attn_out\n// for the backward) re-launches the whole producer — extra launch +\n// extra KV-window rebuild + extra sync, per layer per position.\nfn training_layer_naive(gpu: &Gpu, q: &[f32], layer: usize) -> (Vec<f32>, Vec<f32>) {\n    let wo_out = gpu.dispatch_attention_wo(layer, q); // attn_out computed, DISCARDED\n    let attn_out = gpu.dispatch_attention_only(layer, q); // SECOND attention launch\n    (wo_out, attn_out)\n}\n".to_string(),
            opt_pattern: "// optimized: a split entry on the dispatch surface — same two\n// kernels, ONE execution, both outputs read in the same sync. Inference\n// consumers keep the fused entry (cheaper); training/LoRA/trace\n// consumers take the split entry and never re-launch.\nfn dispatch_attention_wo_split(gpu: &Gpu, layer: usize, q: &[f32]) -> (Vec<f32>, Vec<f32>) {\n    let attn_handle = gpu.launch_attention(layer, q); // kernel 1\n    let wo_handle = gpu.launch_gemv_consuming(&attn_handle); // kernel 2, chain intact\n    let attn_out = gpu.read_handle(&attn_handle); // one sync reads both\n    let wo_out = gpu.read_handle(&wo_handle);\n    (wo_out, attn_out)\n}\n".to_string(),
        },
        // ── preallocated-activation-slots-inplace-save — push-Vec storage → pre-sized in-place slots (A) ──
        // Source: riir-gpu gemma4_cubecl_train/mod.rs — 16 save sites,
        // all the same shape: L190/L208 (logits), L245-246 (x_input),
        // L256 (normed_attn), L263-265 (q/k/v pre_norm), L278-279
        // (q/k pre_rope), L284 (v_cached), L290 (q_post_rope), L345
        // (attn_out), L348 (x_after_wo), L357 (x_after_attn), L367
        // (normed_mlp), L372-373 (gate/up), L381 (hidden), L392
        // (x_after_down) — every one `layer_act.field[pos][..dim]
        // .copy_from_slice(&src[..dim])` into the caller-allocated
        // Gemma4TrainingActivations, size-asserted once (L115-119) and
        // constructed for the whole sequence up front. Every saved tensor
        // is consumed by the backward (verified: hidden feeds the
        // down_proj weight-grad outer product, gate/up + normed_mlp feed
        // the gate/up grads) — the save set is complete, not speculative.
        //
        // Distinct from activation-recompute-budget (H — the WHAT to
        // store vs recompute axis; this is the HOW to store axis:
        // pre-sized slots, in-place copies, zero alloc) and from
        // staging-reuse-grow-only-map-prefix (GPU readback staging —
        // this is forward-side activation storage).
        KernelRule {
            rule_id: "preallocated-activation-slots-inplace-save".to_string(),
            gpu_concept: "a training forward followed by an analytic backward should write every intermediate it will need directly into pre-allocated activation storage indexed [layer][pos][..dim] via copy_from_slice at the moment it is produced — the save is a bounded memcpy into already-reserved memory, so the per-position loop performs zero allocations regardless of how many tensors are saved; size is asserted once up front (activations.seq_len == seq_len) instead of per-site. The caller allocates for the whole sequence at construction; the save sites become branch-free bounded copies with no push/Vec::new in the loop. Distinct from activation-recompute-budget (the WHAT to store vs recompute axis — this is the HOW-to-store axis) and from staging-reuse-grow-only-map-prefix (GPU readback staging growth — this is forward-side activation storage).".to_string(),
            ref_shape: "// naive: save activations by pushing fresh Vecs into per-step\n// storage — allocator churn scales with layers * positions * tensors.\nfn save_naive(storage: &mut Vec<Vec<Vec<f32>>>, layer: usize, pos: usize, x: &[f32]) {\n    while storage.len() <= layer {\n        storage.push(Vec::new());\n    }\n    while storage[layer].len() <= pos {\n        storage[layer].push(Vec::new());\n    }\n    storage[layer][pos] = x.to_vec(); // alloc per save site\n}\n".to_string(),
            opt_pattern: "// optimized: pre-sized [seq_len][dim] slots at construction; every\n// save is a bounded in-place copy — the forward loop is allocation-free\n// no matter how many intermediates are captured.\nstruct LayerAct {\n    x_input: Vec<Vec<f32>>, // [seq_len][n], pre-allocated\n}\nfn save_opt(act: &mut LayerAct, pos: usize, n: usize, hidden: &[f32]) {\n    act.x_input[pos][..n].copy_from_slice(&hidden[..n]); // no alloc\n}\n".to_string(),
        },
        // ── hybrid-cpu-elementwise-gpu-throughput-sync-points — all-GPU elementwise → GPU throughput ops only, CPU fills sync gaps (D) ──
        // Source: riir-gpu gemma4_cubecl_train/mod.rs L16-32 (the phase
        // table: "Training forward | GPU (this module) | GEMV (Q/K/V, Wo,
        // gate/up/down, lm_head) + flash attention + CPU RMSNorm/RoPE/
        // QKNorm/GeGLU" + "The forward dominates compute ... GPU-
        // accelerating just the forward GEMVs captures most of the
        // speedup") + the explicit per-sync annotations L258 ("// GPU:
        // QKV GEMVs (batched, 1 sync) → Sync 1."), L311 ("// GPU:
        // Attention + Wo (2 launches, 1 sync) → Sync 2."), L369 ("// GPU:
        // Gate + Up GEMVs (batched, 1 sync) → Sync 3."), L383 ("// GPU:
        // Down GEMV (1 sync) → Sync 4.") with CPU ops filling every gap
        // (L254, L267, L286, L350, L375, L394). The sibling module's
        // header records the measured claim for the same split on the
        // backward side (gemma2_cubecl_train: "~5-10× speedup over the
        // pure-CPU backward ... without writing any new GPU kernels").
        //
        // Distinct from offload-cpu-bottleneck-to-gpu-kernel (the
        // INVERSE decision — move a CPU data-prep bottleneck INTO a GPU
        // kernel; this rule is the decision BOUNDARY + sync-point
        // choreography for keeping elementwise on the CPU side).
        KernelRule {
            rule_id: "hybrid-cpu-elementwise-gpu-throughput-sync-points".to_string(),
            gpu_concept: "in a hybrid training forward, route only the throughput-bound kernels (GEMVs, attention — batched where siblings share an input) to the GPU and keep latency-bound elementwise ops (norms, RoPE, activation) on the CPU BETWEEN dispatches — each sync point returns data the CPU transforms immediately and the next dispatch re-uploads, so launch overhead is paid only where compute dominates it. The sync-point choreography is the design artifact: per layer per position exactly 4 syncs (qkv → attn+wo → gate_up → down), each a batched dispatch of the expensive ops, with every elementwise op executing launch-free in the gaps. Pushing the elementwise ops to GPU instead costs upload+launch+sync for a few µs of work each; keeping the GEMVs on CPU forfeits the entire throughput win. Distinct from offload-cpu-bottleneck-to-gpu-kernel (the inverse decision — moving a CPU data-prep bottleneck into a GPU kernel; this is the boundary + choreography for what stays CPU).".to_string(),
            ref_shape: "// naive: all-GPU — every elementwise op becomes its own kernel\n// launch + sync; at n_embd ~ 4K the launch overhead exceeds the op's\n// cost.\nfn rmsnorm_gpu_dispatch(gpu: &Gpu, x: &mut Vec<f32>, gamma: &[f32], eps: f32) {\n    let h = gpu.upload(x); // upload\n    let o = gpu.launch_rmsnorm(&h, gamma, eps); // launch for a ~us pass\n    *x = gpu.read(&o); // sync + download — all for microseconds of work\n}\n".to_string(),
            opt_pattern: "// optimized: hybrid split with sync-point choreography — GPU takes\n// only the throughput-bound kernels (batched where siblings share an\n// input); the CPU transforms data AT the sync points between\n// dispatches. Elementwise ops never launch.\nfn layer_hybrid(gpu: &Gpu, hidden: &mut Vec<f32>, pos: usize) {\n    let (mut q, mut k, _v) = gpu.dispatch_qkv(hidden); // GPU, 1 sync (batched)\n    qk_norm_rope_cpu(&mut q, &mut k, pos); // CPU in the gap — no launch\n    let wo_out = gpu.dispatch_attention_wo(&q); // GPU, 1 sync\n    post_attn_norm_residual_cpu(hidden, &wo_out); // CPU\n    let (gate, up) = gpu.dispatch_gate_up(hidden); // GPU, 1 sync (batched)\n    let h = geglu_cpu(&gate, &up); // CPU\n    let down = gpu.dispatch_gemv_down(&h); // GPU, 1 sync\n    post_mlp_norm_residual_cpu(hidden, &down); // CPU\n}\n".to_string(),
        },
        // ── pre-transposed-consumer-layout-handles-build-once — per-step transpose → build-once dual-layout residency (L) ──
        // Source: riir-gpu gemma4_cubecl_train/backward_gpu.rs L20-26
        // ("The backward GEMV `g_input = W^T @ g_output` is computed as
        // `dispatch_gemv(&w_t, &g_output, cols, rows)` ... The
        // pre-transpose is a one-time cost at training setup (call
        // `build_backward_handles` once)."), L69-79 (Gemma4BackwardHandles
        // struct: 7 per-layer transposed handles + wte_t, "Built once ...
        // and reused across training steps"), L92-166 (the builder: read
        // handle → CPU transpose_f32 → re-upload, per weight). Triple
        // cross-model recurrence: gemma2_cubecl_train/mod.rs L93-103 +
        // L931-935 ("Build backward handles ONCE — this transposes all 7
        // weights per layer (+ lm_head) and uploads to GPU. ~2× weight
        // storage, one-time cost.") and gemma4_q4k_train/mod.rs L570-578
        // (the memoized `if self.backward_handles.is_none()` variant).
        //
        // Honest caveats: (1) the setup transpose is a naive CPU scalar
        // transpose — at 12B scale (47.8 GB) that is minute-class; the
        // transpose itself should be a GPU tiled kernel when setup cost
        // matters (offload-cpu-bottleneck-to-gpu-kernel). (2) The
        // frozen-base contract needs enforcement — a weight replacement
        // does NOT invalidate the handles (riir-ai Issue 687 H2).
        // Distinct from host-cache-immutable-device-weights (host CPU
        // copy of device bytes — this materializes DIFFERENT bytes, the
        // transpose), from immutable-operand-upload-at-set-creation
        // (upload placement — this is the derived second LAYOUT), and
        // from block-contiguous-weight-layout (one layout for one
        // kernel — this is dual-layout residency for two pipeline
        // stages).
        KernelRule {
            rule_id: "pre-transposed-consumer-layout-handles-build-once".to_string(),
            gpu_concept: "when a frozen operand is consumed by a different pipeline stage in the OPPOSITE layout (backward needs W^T @ g while forward stores W row-major), materialize the consumer-layout copy ONCE at training setup into resident handles and reuse it across all steps — trading ~2× weight memory for zero per-step transposes or strided reads. Validity rests on the frozen-base contract: rebuild only when the base weights are replaced (and ENFORCE that — a hot-swap must invalidate the handles or they silently transpose stale bytes). The alternative shapes are both worse: per-step CPU transpose (read+transpose+upload every step) or a GEMV kernel reading row-major W with transposed/strided access (cache-hostile). Distinct from host-cache-immutable-device-weights (caches the SAME bytes on host — this materializes different bytes, the transpose), from immutable-operand-upload-at-set-creation (upload placement of a never-changing operand — this is the derived second layout), and from block-contiguous-weight-layout (one layout for one kernel — this is dual-layout residency for two pipeline stages).".to_string(),
            ref_shape: "// naive: the backward GEMV needs W^T — pay the transpose (or a\n// strided-read GEMV) every step.\nfn backward_step_per_step_transpose(gpu: &Gpu, w: &Handle, g_out: &[f32], rows: usize, cols: usize) -> Vec<f32> {\n    let cpu_w = gpu.read_handle(w); // device -> host, EVERY step\n    let w_t = transpose_f32(&cpu_w, rows, cols); // CPU transpose, EVERY step\n    let w_t_handle = gpu.create_from_slice(&w_t); // host -> device, EVERY step\n    gpu.dispatch_gemv(&w_t_handle, g_out, cols, rows)\n}\n".to_string(),
            opt_pattern: "// optimized: materialize the consumer-layout copy ONCE at setup and\n// keep both layouts resident (~2x memory); the frozen-base contract\n// makes it valid — rebuild only when the base weights are replaced,\n// and invalidate on hot-swap so stale handles cannot be used.\nstruct BackwardHandles {\n    w_t: Handle, // the pre-transposed [cols x rows] resident copy\n}\nfn build_backward_handles(gpu: &Gpu, w: &Handle, rows: usize, cols: usize) -> BackwardHandles {\n    let cpu_w = gpu.read_handle(w); // one-time\n    let w_t = transpose_f32(&cpu_w, rows, cols);\n    BackwardHandles { w_t: gpu.create_from_slice(&w_t) }\n}\nfn backward_step(gpu: &Gpu, h: &BackwardHandles, g_out: &[f32], cols: usize, rows: usize) -> Vec<f32> {\n    gpu.dispatch_gemv(&h.w_t, g_out, cols, rows) // zero per-step transpose\n}\n".to_string(),
        },
        // ── shape-matched-sibling-weight-grads-share-input-loop — two walks of x → one loop, two FMA streams (C) ──
        // Source: riir-gpu gemma4_cubecl_train/backward_gpu.rs L532-540
        // (wk+wv: `for row in 0..kvd { let gr_k = g_k_pre_norm[row]; let
        // gr_v = g_v_pre_norm[row]; ... layer_grad.attn_wk[w_off + col]
        // += gr_k * normed_attn[col]; layer_grad.attn_wv[w_off + col] +=
        // gr_v * normed_attn[col]; }`) and L358-366 (gate+up over
        // normed_mlp, same shape); the precondition is visible in the
        // code — wq gets its OWN loop (L525-531) because q_dim != kvd;
        // only the row-count-matched pairs merge.
        //
        // Honest caveat: at this CPU instantiation with n=3840 (15 KB,
        // L1-resident) the bandwidth win is marginal — the win is loop-
        // overhead halving; the load-traffic win is real on device or
        // when the shared input exceeds cache. Distinct from
        // fuse-weight-grad-input-grad-single-matrix-visit (ONE matrix,
        // two gradient ROLES — this is TWO matrices, one input, same
        // role) and from fuse-sibling-target-grads-single-kernel (N
        // sibling TARGETS fused per kernel kind on device — this is the
        // host-loop axis: the shared INPUT operand visited once).
        KernelRule {
            rule_id: "shape-matched-sibling-weight-grads-share-input-loop".to_string(),
            gpu_concept: "when two sibling weight matrices have identical output-row counts AND consume the identical saved input operand (g_W_k += g_k ⊗ x and g_W_v += g_v ⊗ x), accumulate both rank-1 updates in ONE pass over the shared input — load x[col] once, feed two FMA streams — instead of walking the input (and the whole index space) twice. Precondition is shape-matched rows (q_dim ≠ kvd keeps wq in its own loop — only row-count-matched pairs merge). Honest regime note: at CPU scale with an L1-resident input the win is loop-overhead halving; the load-traffic win is real on device or when the shared input exceeds cache. Distinct from fuse-weight-grad-input-grad-single-matrix-visit (one matrix, two gradient roles — this is two matrices, one input, same role) and from fuse-sibling-target-grads-single-kernel (N sibling targets fused per kernel kind on device — this is the host-loop axis where the shared input is visited once).".to_string(),
            ref_shape: "// naive: two separate accumulations, each walks the shared input\nfn grad_wk_wv_naive(g_k: &[f32], g_v: &[f32], x: &[f32], wk: &mut [f32], wv: &mut [f32], kvd: usize, n: usize) {\n    for row in 0..kvd {\n        let off = row * n;\n        for col in 0..n {\n            wk[off + col] += g_k[row] * x[col]; // x walked 1st time\n        }\n    }\n    for row in 0..kvd {\n        let off = row * n;\n        for col in 0..n {\n            wv[off + col] += g_v[row] * x[col]; // x walked AGAIN\n        }\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: shape-matched siblings share ONE loop — x[col] loaded\n// once, two FMA streams (two output matrices) per iteration.\n// Precondition: identical row counts (wq stays separate: q_dim != kvd).\nfn grad_wk_wv_shared(g_k: &[f32], g_v: &[f32], x: &[f32], wk: &mut [f32], wv: &mut [f32], kvd: usize, n: usize) {\n    for row in 0..kvd {\n        let gr_k = g_k[row];\n        let gr_v = g_v[row];\n        let off = row * n;\n        for col in 0..n {\n            let xc = x[col]; // ONE load of the shared input\n            wk[off + col] += gr_k * xc; // sibling 1\n            wv[off + col] += gr_v * xc; // sibling 2\n        }\n    }\n}\n".to_string(),
        },
        // ── op-major-backward-loop-inversion-batch-sequence-axis — position-major syncs → op-stage batching (D) ──
        // Source: riir-gpu gemma4_cubecl_train/backward_gpu.rs — the
        // position loop L305-429 interleaves 4 blocking GEMVs with CPU
        // ops (down L338, gate L354, up L355, wo L418) and loop C adds 3
        // more (wq/wk/wv L520-522); the ONLY cross-position coupling is
        // attention backward (L436-453), which already takes all
        // positions and sits BETWEEN the per-position stages — proving
        // the sequence axis is free inside each stage. dispatch_gemv is
        // a full round-trip per call (gemma4_cubecl/dispatch.rs L19-43:
        // create_from_slice + empty + launch + read_handle).
        //
        // Quantified (12B: 48 layers, vocab 262144; seq_len 1024):
        // 7 GEMVs × 48 × 1024 + 1024 lm_head = 345,088 blocking
        // upload+launch+readback round-trips per training step; each
        // wte_t GEMV re-reads the 4 GB transposed embedding per position
        // (~4 TB device traffic/step for Phase 1 alone — the
        // row-tile-amortize-input-load win at matrix scale).
        // Distinct from weight-grad-batched-outer-product (H — batches
        // the WEIGHT-grad accumulation; this is the host loop-nest
        // inversion that exposes batchability for ALL stages), from
        // row-tile-amortize-input-load (in-kernel M-widening — this is
        // the host restructure that turns N GEMVs into one GEMM), from
        // derive-params-from-global-invocation-id (per-item params in
        // the grid — composes with this), and from
        // batched-downloads-single-sync-cycle (batches N readbacks —
        // this removes the per-item structure that made them N).
        KernelRule {
            rule_id: "op-major-backward-loop-inversion-batch-sequence-axis".to_string(),
            gpu_concept: "a per-position backward loop that interleaves GPU GEMV + blocking readback with CPU elementwise ops pays one device round-trip per item per op; when the sequence axis is free (positions independent within each op stage — the only cross-position op, attention backward, sits BETWEEN stages), invert the loop nest from (layer → position → op-chain) to (layer → op-stage → all positions) so every per-item GEMV becomes one sequence-batched GEMM with ONE dispatch and ONE readback: 7 round-trips per layer per step instead of 7 × seq_len. The attention-backward-between-stages placement is the proof the axis is free — it already consumes all positions jointly. Distinct from weight-grad-batched-outer-product (batches the weight-grad accumulation — this is the loop-nest inversion that exposes batchability for ALL stages), from row-tile-amortize-input-load (in-kernel M-widening of one GEMV — this is the host restructure turning N GEMVs into one GEMM), and from batched-downloads-single-sync-cycle (batches N readbacks into one sync — this removes the per-item structure that made them N).".to_string(),
            ref_shape: "// naive: position-major — every position pays the full\n// upload/launch/readback round-trip for EVERY GEMV in the chain\n// (positions are independent; the loop structure serializes them).\nfn backward_item_major(gpu: &Gpu, bw: &Handles, g_x_out_all: &[Vec<f32>]) {\n    for p in 0..g_x_out_all.len() {\n        let g_x_down = rmsnorm_bwd(&g_x_out_all[p]);\n        let g_hidden = gpu.dispatch_gemv(&bw.down_t, &g_x_down, 0, 0); // SYNC per item\n        let (g_gate, g_up) = gegelu_bwd(&g_hidden);\n        let a = gpu.dispatch_gemv(&bw.gate_t, &g_gate, 0, 0); // SYNC per item\n        let b = gpu.dispatch_gemv(&bw.up_t, &g_up, 0, 0); // SYNC per item\n        // ... 4 more round-trips for this one position\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: op-major — the dependency chain is op->op, NOT\n// position->position (the only cross-position op, attention backward,\n// sits BETWEEN stages). Batch each stage across ALL positions: each\n// GEMV stage becomes one [seq x in] @ [in x out] GEMM — one dispatch +\n// one readback per stage per layer (was 7 * seq_len).\nfn backward_op_major(gpu: &Gpu, bw: &Handles, g_x_out_all: &[Vec<f32>], seq: usize) {\n    let g_x_down_all = all_positions_norm_bwd(g_x_out_all); // CPU, independent\n    let g_hidden_all = gpu.dispatch_gemm(&bw.down_t, &g_x_down_all, seq); // 1 sync\n    let (g_gate_all, g_up_all) = all_positions_geglu_bwd(&g_hidden_all); // CPU\n    let _gu = gpu.dispatch_gemm_pair(&bw.gate_t, &bw.up_t, &g_gate_all, &g_up_all, seq); // 1 sync\n    // cross-position attention backward runs BETWEEN stages as before\n}\n".to_string(),
        },
        // ── backward-helper-expose-shared-norm-statistic — hidden inv_rms → helper returns the statistic (H) ──
        // Source: riir-engine gemma4_train/backward.rs L39-49
        // (rmsnorm_backward computes `let sum_sq: f32 = x.iter().map(|&v|
        // v * v).sum(); let rms_sq = sum_sq / n as f32 + eps; let inv_rms
        // = 1.0 / rms_sq.sqrt();` then returns ONLY g_x) + all the
        // gemma4_cubecl_train/backward_gpu.rs gamma-grad sites that
        // recompute it right after calling the helper: L274-279
        // (final_norm), L325-330 (post_mlp), L381-387 (pre_mlp),
        // L404-411 (post_attn), L486-492 (attn_q, per head), L506-512
        // (attn_k, per head), L552-558 (input_norm). Worst case 3× at
        // the final norm (L250 x_normed recompute + inside L271's helper
        // + L274-276).
        //
        // ~393K redundant n-element reduction passes per step at 12B ×
        // seq 1024. Distinct from precompute-scalar-reduction (hoists a
        // LOOP-INVARIANT row reduction — here the reduction is
        // per-position/per-call, NOT invariant; the waste is the
        // helper's return shape hiding a value the same call site needs)
        // and from derived-scalar-host-computed-in-uniform (config-
        // derived constants — this is a data-dependent reduction).
        KernelRule {
            rule_id: "backward-helper-expose-shared-norm-statistic".to_string(),
            gpu_concept: "an elementwise-backward helper that internally computes a normalization statistic (inv_rms from sum_sq) which the parameter-gradient path at the SAME call site also needs must return or expose that statistic (or fuse the gamma-grad accumulation into the helper) — otherwise every call site recomputes the full reduction pass, doubling (at the final norm: tripling) reduction work per norm per position. The fix is the helper's return shape: (g_x, inv_rms) instead of g_x alone. Distinct from precompute-scalar-reduction (hoists a loop-INVARIANT row reduction — this reduction is per-position/per-call; the waste is the return shape hiding a value the same site needs) and from derived-scalar-host-computed-in-uniform (config-derived constants — this is a data-dependent reduction over activations).".to_string(),
            ref_shape: "// naive: the helper computes inv_rms internally and returns only\n// g_x; the caller needs inv_rms AGAIN for the gamma gradient —\n// recomputing the full reduction pass at every call site.\nfn rmsnorm_backward(g_y: &[f32], x: &[f32], gamma: &[f32], eps: f32) -> Vec<f32> {\n    let n = x.len();\n    let sum_sq: f32 = x.iter().map(|&v| v * v).sum(); // reduction pass 1\n    let inv_rms = 1.0 / (sum_sq / n as f32 + eps).sqrt();\n    let mut g_x = vec![0.0f32; n];\n    // ... g_x math using inv_rms ...\n    g_x // statistic hidden from the caller\n}\nfn caller(g_y: &[f32], x: &[f32], gamma: &[f32], g_gamma: &mut [f32], eps: f32) {\n    let _g_x = rmsnorm_backward(g_y, x, gamma, eps);\n    let n = x.len();\n    let sum_sq: f32 = x.iter().map(|&v| v * v).sum(); // pass 2 — SAME reduction\n    let inv_rms = 1.0 / (sum_sq / n as f32 + eps).sqrt();\n    for i in 0..n {\n        g_gamma[i] += g_y[i] * inv_rms * x[i];\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: the helper exposes the statistic it already computed —\n// the gamma-grad accumulation consumes it without a second reduction.\nstruct RmsBwd {\n    g_x: Vec<f32>,\n    inv_rms: f32,\n}\nfn rmsnorm_backward_with_stat(g_y: &[f32], x: &[f32], gamma: &[f32], eps: f32) -> RmsBwd {\n    let n = x.len();\n    let sum_sq: f32 = x.iter().map(|&v| v * v).sum(); // reduction ONCE\n    let inv_rms = 1.0 / (sum_sq / n as f32 + eps).sqrt();\n    let mut g_x = vec![0.0f32; n];\n    // ... g_x math ...\n    RmsBwd { g_x, inv_rms }\n}\nfn caller(g_y: &[f32], x: &[f32], gamma: &[f32], g_gamma: &mut [f32], eps: f32) {\n    let r = rmsnorm_backward_with_stat(g_y, x, gamma, eps);\n    for i in 0..x.len() {\n        g_gamma[i] += g_y[i] * r.inv_rms * x[i]; // no 2nd reduction pass\n    }\n}\n".to_string(),
        },
        // ── mask-sentinel-matches-reduction-identity — skip → identity-element
        // participation (E) ──
        // Source: riir-gpu gemma2_cubecl_train/attention_gpu.rs L149-152 (masked
        // lanes write neg_inf into smem_scores), L194-198 (the neg-inf → 0.0
        // sentinel REWRITE between the max and sum reductions), L222-225 (the
        // normalize pass re-zeroes the sum identity), L235 (phase 3 bounds at
        // `while j3 <= i` — the non-reduction consumer).
        KernelRule {
            rule_id: "mask-sentinel-matches-reduction-identity".to_string(),
            gpu_concept: "when a block-wide reduction runs over a masked/variable-extent domain (causal attention: only j <= i is valid), masked lanes must PARTICIPATE by writing the reduction's IDENTITY element (neg-inf for max, 0.0 for sum) rather than skipping — the tree stays full-width and extent-independent, and no lane ever reduces garbage — and when the same buffer feeds two reductions with DIFFERENT identities, rewrite the sentinel between them (neg-inf for the max pass, then 0.0 before the sum pass). Consumers outside the reduction may bound at the valid extent instead (the weighted-sum loop iterates j <= i directly). Distinct from derive-params-from-global-invocation-id (derives WHICH lanes are masked — this governs what the masked lanes WRITE) and from tree-reduction-depth-over-sequential-accumulate (tree vs sequential for accuracy — this is participation shape) and from broadcast-uniform-skip-reduce (post-reduction thread-uniformity — this is pre-reduction filling). RIDER (Batch 98 / Research 118, the POINTER-STATE complement): in paired/split-range kernels with ragged output extents, out-of-range lanes point at the matrix BASE as a safe sentinel so ONE shared loop body serves valid and invalid lanes without branching the K walk, and reductions and stores are gated by the active flags -- including skipping the dead arm's reduction entirely (antirez/ds4 @ 110afdd8 metal/dense.metal L233-239 + L288-289: the base-pointer sentinel + the selective simd_sum).".to_string(),
            ref_shape: "// naive: masked lanes opt out entirely — their slots stay uninitialized\n// (or hold stale values), and the full-width reduction tree reads them\nfn score_masked_naive(j: usize, i: usize, score: f32, smem_scores: &mut [f32]) {\n    if j <= i {\n        smem_scores[j] = score;\n    } // else: SKIP — slot left uninitialized; the max tree reads ALL lanes\n}\n// max over every lane up to seq_len: max(score, garbage) == garbage\nfn max_tree_full_width(smem: &[f32], seq_len: usize) -> f32 {\n    let mut m = f32::NEG_INFINITY;\n    for t in 0..seq_len {\n        m = m.max(smem[t]); // masked slots poison the max (or the sum)\n    }\n    m\n}\n".to_string(),
            opt_pattern: "// optimized: masked lanes PARTICIPATE — writing the reduction's identity\nfn score_masked_identity(j: usize, i: usize, score: f32, neg_inf: f32, smem_scores: &mut [f32]) {\n    if j <= i {\n        smem_scores[j] = score;\n    } else {\n        smem_scores[j] = neg_inf; // identity for max: max(x, -inf) == x\n    }\n}\n// BEFORE the sum-reduction, REWRITE the sentinel (different identity!)\nfn exp_masked_identity(j: usize, i: usize, max_score: f32, smem_scores: &mut [f32]) {\n    if j <= i {\n        let diff = smem_scores[j] - max_score;\n        smem_scores[j] = diff.exp();\n    } else {\n        smem_scores[j] = 0.0; // identity for sum: + 0.0 is an exact no-op\n    }\n}\n// outside reductions, bound at the valid extent (skip IS safe there)\nfn weighted_sum_bounded(weights: &[f32], v: &[f32], kv_dim: usize, i: usize, kv_off: usize, d: usize) -> f32 {\n    let mut acc = 0.0;\n    let mut j3 = 0;\n    while j3 <= i {\n        acc += weights[j3] * v[j3 * kv_dim + kv_off + d];\n        j3 += 1;\n    }\n    acc\n}\n".to_string(),
        },
        // ── smem-staging-gated-by-cross-thread-reuse — stage everything →
        // stage iff multi-reader (A) ──
        // Source: riir-gpu gemma2_cubecl_train/attention_gpu.rs L123-129 (phase-0
        // cooperative staging of Q[i,h,:] into smem_q) + L141 (the SAME dot loop
        // reads Q from smem (multi-reader) while K streams from global
        // (single-reader)) + L18-19 (the doc stating the asymmetry).
        KernelRule {
            rule_id: "smem-staging-gated-by-cross-thread-reuse".to_string(),
            gpu_concept: "stage an operand into shared memory iff its within-workgroup reuse count exceeds one — each element read by MULTIPLE threads (a per-workgroup-invariant row such as Q[i,h,:] is re-read by every scoring thread, up to block x head_dim reads) — while operands read exactly once by exactly one thread (each K row j is private to thread j) stream directly from global: staging them buys nothing and costs smem plus a barrier. The selection gate (including the negative half) is the distinct content: one dot loop legitimately reads Q from smem and K from global in the same breath. Distinct from cooperative-block-wide-quantize-staging (the mechanics of staging when reuse exists — this is the WHEN/when-not gate) and from kv-block-streaming (blocks streamed THROUGH smem for capacity — the single-use operand here never enters smem at all).".to_string(),
            ref_shape: "// naive: (a) every thread pulls the shared row from global inside its dot\n// loop — block x head_dim redundant global reads for ONE row; or (b) the\n// symmetric over-correction — stage BOTH operands, wasting smem + a barrier\n// on the single-use one\nfn dot_naive_global(q_global: &[f32], k: &[f32], q_off: usize, head_dim: usize, j: usize, kv_dim: usize) -> f32 {\n    let mut dot = 0.0f32;\n    for d in 0..head_dim {\n        dot += q_global[q_off + d] * k[j * kv_dim + d]; // q re-read by EVERY thread\n    }\n    dot\n}\nfn dot_naive_stage_both(smem_q: &[f32], smem_k_row: &[f32], head_dim: usize) -> f32 {\n    // K row j was staged into smem — but ONLY thread j ever reads it:\n    // the staging pass + barrier bought nothing\n    let mut dot = 0.0f32;\n    for d in 0..head_dim {\n        dot += smem_q[d] * smem_k_row[d];\n    }\n    dot\n}\n".to_string(),
            opt_pattern: "// optimized: stage ONLY the cross-thread-shared operand; the single-use\n// one streams from global in the same dot loop — the asymmetry is the rule\nfn dot_asymmetric(smem_q: &[f32], k: &[f32], k_off: usize, head_dim: usize, j: usize, kv_dim: usize) -> f32 {\n    let mut dot = 0.0f32;\n    for d in 0..head_dim {\n        // Q from SHARED (every thread re-reads all of it — reuse = block)\n        // K from GLOBAL (read by exactly one thread — reuse = 1, no stage)\n        dot += smem_q[d] * k[j * kv_dim + k_off + d];\n    }\n    dot\n}\n".to_string(),
        },
        // ── kernelize-host-chain-after-sync-batching-floor — post-batching
        // floor of 2 → one fused dispatch (C) ──
        // Source: riir-gpu gemma2_cubecl_train/attention_gpu.rs L3-4 ("Eliminates
        // the 52 GPU↔CPU sync points per forward") + resident.rs L687-688 (the
        // decomposition: "Attention is computed on CPU (2 syncs per layer for QKV
        // readback and attn_out upload)") + L844-851 (Option C ALREADY reads QKV
        // in one batched sync — the floor is post-batching) + L862
        // (attn_out_clone — the external-consumer boundary) + dispatch.rs
        // L1583-1617 (dispatch_attention_fused_gpu). n_layer=26 → 52.
        KernelRule {
            rule_id: "kernelize-host-chain-after-sync-batching-floor".to_string(),
            gpu_concept: "a host-serviced op-chain inside a GPU-resident pipeline has a sync floor of 2 per invocation (download operands + upload result) — download batching can collapse N operand reads into 1 but never below 2 — so when the segment repeats per outer-loop iteration (2 syncs x 26 layers = 52 per forward), its intermediates are chain-local (the seq-squared x heads score matrix is consumed only by the next op, never inspected by the host), and the host does no productive work in the gap, kernelize the whole chain into one dispatch with zero syncs — placing the output boundary at the first tensor consumed OUTSIDE the chain (attn_out stays a handle for the backward; the scores never materialize anywhere at all). Distinct from merge-dispatches-into-single-compute-pass / chain-dispatches-single-sync-point (fuse GPU-to-GPU dispatches that never touched the host), from batched-downloads-single-sync-cycle (reduces sync count but KEEPS the host boundary — this rule is the floor that proves batching alone cannot win), and from hybrid-cpu-elementwise-gpu-throughput-sync-points (the complement: when the boundary should STAY because the host fills the gaps with real work).".to_string(),
            ref_shape: "// naive: the host-serviced segment AFTER batching was already applied —\n// downloads collapsed to ONE batched sync, but a host round trip floors\n// at 2 (down + up). 2 syncs x 26 layers = 52 per forward, and batching\n// cannot reduce it further.\nfn attention_cpu_path(client: &Client, q: &Handle, k: &Handle, v: &Handle) -> Handle {\n    let qkv = read_batched(client, vec![q.clone(), k.clone(), v.clone()]); // sync 1 (down)\n    let attn = compute_attention_cpu(&qkv[0], &qkv[1], &qkv[2]); // host transforms only\n    client.create_from_slice(&attn) // sync 2 (up) — the floor; nothing left to batch\n}\n".to_string(),
            opt_pattern: "// optimized: intermediates are chain-local and the host does no work in\n// the gap -> kernelize the chain; the output boundary is the first tensor\n// consumed OUTSIDE the chain.\nfn attention_fused(client: &Client, q: &Handle, k: &Handle, v: &Handle) -> Handle {\n    let attn_out = dispatch_attention_fused(client, q, k, v); // ONE dispatch, 0 syncs\n    // attn_out is the boundary: the backward clones the handle; the\n    // seq-squared x heads score matrix never materializes ANYWHERE\n    attn_out\n}\n".to_string(),
        },
        // ── block-reduction-separate-scratch-runtime-strides — in-place +
        // hardcoded steps → scratch + runtime strides (B) ──
        // Source: riir-gpu gemma2_cubecl_train/attention_gpu.rs L157-167 (the
        // shipped bug-fix record: Issue 430 G1 FAIL on 4090 — in-place reduction
        // destroyed scores; the 7-step unroll was incomplete at 256 and
        // double-counted at 4) + L168-188 (separate smem_reduce + while-loop
        // stride from seq_len/2) + L193 ("smem_scores still holds the ORIGINAL
        // scores") + L204-216 (the sum tree repeats the shape).
        KernelRule {
            rule_id: "block-reduction-separate-scratch-runtime-strides".to_string(),
            gpu_concept: "a multi-pass reduction consumer (softmax: max-pass then exp then sum-pass, both passes needing the ORIGINAL scores) must reduce into a SEPARATE scratch buffer, never in-place — the first tree overwrites values the later elementwise pass still reads — and the tree's stride sequence must be derived from the runtime extent (stride = extent/2, halving while stride > 0), never a hardcoded unrolled step count: a 7-step unroll on 256 lanes needs 8 (silently reducing only half the buffer), and the same hardcode at extent 4 executes the final step redundantly and double-counts in the sum (systematic 2/3 scaling of the output). Both failure modes shipped and failed G1 on real hardware before the fix. Distinct from tree-reduction-depth-over-sequential-accumulate (choosing tree vs sequential for accuracy — this is aliasing plus size-genericity) and from dual-accumulator-single-reduction-sweep (fusing two INDEPENDENT reductions — softmax's max and sum cannot fuse: the sum depends on the max).".to_string(),
            ref_shape: "// naive: both shipped failure modes — (1) reduce IN-PLACE on the source\n// buffer, destroying the scores the exp pass still needs; (2) hardcoded\n// unrolled step count for one assumed extent (7 steps; extent 256 needs 8,\n// extent 4 double-counts the tail)\nfn max_reduce_naive(smem_scores: &mut [f32], tid: usize) {\n    if tid < 128 {\n        let a = smem_scores[tid];\n        let b = smem_scores[tid + 128];\n        smem_scores[tid] = if a > b { a } else { b }; // source destroyed in place\n    }\n    if tid < 64 {\n        let a = smem_scores[tid];\n        let b = smem_scores[tid + 64];\n        smem_scores[tid] = if a > b { a } else { b };\n    }\n    // ... 5 more hardcoded steps — wrong count for any other extent, and\n    // smem_scores no longer holds the scores the exp pass reads\n}\n".to_string(),
            opt_pattern: "// optimized: separate scratch + strides derived from the runtime extent\nfn max_reduce_generic(smem_scores: &[f32], smem_reduce: &mut [f32], tid: usize, seq_len: usize) -> f32 {\n    if tid < seq_len {\n        smem_reduce[tid] = smem_scores[tid]; // source PRESERVED for the exp pass\n    }\n    sync_cube();\n    let mut stride = seq_len / 2;\n    while stride > 0 {\n        // log2(extent) steps — correct for ANY power-of-2 extent\n        if tid < stride {\n            let a = smem_reduce[tid];\n            let b = smem_reduce[tid + stride];\n            smem_reduce[tid] = if a > b { a } else { b };\n        }\n        sync_cube();\n        stride /= 2;\n    }\n    smem_reduce[0]\n}\n".to_string(),
        },
        // ── saturating-nonlinearity-recovery-breaks-near-cap — recovery from
        // cached output → raw-arg recompute when saturation reachable (N) ──
        // Source: riir-engine transformer/gemma2_train/backward.rs L354-358 (the
        // comment: "recovering tanh from the clamped score (score/softcap) loses
        // all precision when the score saturates near the softcap ceiling") +
        // L408-416 (the raw recompute: d/dot = scale*(1 - tanh^2(dot*scale/
        // softcap)) evaluated on the raw dot) — CONTRASTED with the recovery
        // form at the LM head (riir-gpu gemma2_cubecl_train/mod.rs L572-578,
        // tanh_val = logits[v]/logit_cap), kept there because 9 x cap ~ 270
        // is unreachable for LM-head logits.
        KernelRule {
            rule_id: "saturating-nonlinearity-recovery-breaks-near-cap".to_string(),
            gpu_concept: "recovering a saturating nonlinearity's derivative from its cached OUTPUT (sech-squared from tanh via 1 - t*t, sigmoid-prime from sigma) is exact algebra but numerically void in the saturation region: once the cached output rounds to 1.0 in f32 (tanh arg >= 9, sigmoid arg >= 17), 1 - t*t yields EXACTLY 0 while the true derivative is ~1e-7 and decaying — silently zeroing gradients precisely where the forward clamped hardest. When saturation is REACHABLE in your domain (attention scores x scale can plausibly hit 9 x softcap), cache or recompute the RAW pre-nonlinearity argument instead (recomputing the dot from saved Q,K is Theta(t x d) and vanishes next to the GEMVs); reserve the recovery trick for paths where saturation is unreachable (LM-head logits reaching 9 x cap is ~270 — the recovery form is fine there). Delimits sigmoid-deriv-recovered-from-output (the recovery family) by naming the f32 saturation boundary where it silently fails — the codebase itself makes the opposite choice at the two sites. Distinct from exact-op-parity-for-analytic-backward (forward/backward op identity — this is the cached-value recovery boundary).".to_string(),
            ref_shape: "// naive (the recovery form): derivative from the CACHED post-cap output —\n// exact algebra, numerically void once the cached output rounds to 1.0\nfn softcap_bwd_recovery(g: &[f32], logits_post_cap: &[f32], logit_cap: f32, g_raw: &mut [f32]) {\n    for v in 0..g.len() {\n        let tanh_val = logits_post_cap[v] / logit_cap; // == 1.0f32 exactly in saturation\n        g_raw[v] = g[v] * (1.0 - tanh_val * tanh_val); // 1 - 1 = exactly 0: gradient dead\n        // the true sech^2(arg) at the same point is ~1e-7 and decaying, never 0\n    }\n}\n".to_string(),
            opt_pattern: "// optimized (the recompute form): when saturation is reachable, rebuild the\n// RAW pre-nonlinearity argument from the linear saves and evaluate there\nfn softcap_bwd_raw_recompute(g_s: &[f32], q: &[f32], k_cache: &[f32], q_off: usize, k_off: usize, head_dim: usize, scale: f32, softcap: f32, g_dot: &mut [f32], t_n: usize) {\n    for t in 0..t_n {\n        let mut dot = 0.0f32;\n        for d in 0..head_dim {\n            dot += q[q_off + d] * k_cache[t * head_dim + k_off + d]; // raw pre-cap arg\n        }\n        let arg = dot * scale / softcap;\n        let t_val = arg.tanh(); // full precision in the saturation region\n        g_dot[t] = g_s[t] * scale * (1.0 - t_val * t_val); // never collapses early\n    }\n}\n".to_string(),
        },
        // ── adapter-grad-join-mirrors-forward-delta-insertion-point — merged
        // convention → additive join + immortal handles (H) ──
        // Source: riir-gpu gemma2_cubecl_train/mod.rs L197-204 (forward:
        // apply_lora_delta on GEMV outputs — additive CPU deltas) + L613-630
        // (backward: lora_proj_backward + dispatch_gemv(down_t) + elementwise
        // join — repeated at all 7 sites: L636-669, L693-709, L746-810) +
        // L931-933 (build_backward_handles ONCE — no invalidation surface: no
        // replace_weights exists on this class; the optimizer touches only CPU
        // adapter params). Contrast: Issue 687 H2 (the Gemma-4 merged-weight
        // variant DROPS the adapter input-grad term).
        KernelRule {
            rule_id: "adapter-grad-join-mirrors-forward-delta-insertion-point".to_string(),
            gpu_concept: "when LoRA adapters are applied as additive CPU-side deltas on GEMV outputs (not merged into weights), the backward must add the adapter's input-gradient at exactly the forward's insertion point — base W^T @ g on the GPU via pre-transposed handles PLUS A^T(scale x B^T x g) on the CPU, summed elementwise — which keeps the base transposes immortal by construction (optimizer steps mutate only CPU adapter params, so the build-once handles have no invalidation surface at all) and keeps adapter traffic off the GPU. The handle contract is a direct consequence of the application convention, not a free choice: under the MERGED-weight convention the same base-only handles silently DROP the delta-W^T x g input-grad term (the Issue 687 H2 class — the gradient is wrong, not just slower). Distinct from pre-transposed-consumer-layout-handles-build-once (build-once mechanics + invalidation — this is the convention-to-contract implication that determines whether invalidation is even needed) and from zero-init-side-branch-frozen-base-path (B=0 value identity — this is the gradient-join placement).".to_string(),
            ref_shape: "// naive: MERGED convention — adapter baked into the weight; the backward\n// handle built from the base only silently drops the adapter grad term\nfn forward_merged(w_base: &Mat, lora: &Lora, x: &[f32]) -> Vec<f32> {\n    let w_merged = w_base.add_scaled_outer(&lora.b, &lora.a, lora.scale); // baked in\n    gemv(&w_merged, x)\n}\nfn backward_merged_handles(w_base_t: &Mat, g_y: &[f32]) -> Vec<f32> {\n    gemv_t(w_base_t, g_y) // base term only — the delta-W^T x g term is DROPPED\n}\n".to_string(),
            opt_pattern: "// optimized: ADDITIVE convention — base GEMV on GPU + CPU delta at the\n// output; the backward joins the adapter input-grad at the same point\nfn forward_additive(gpu: &Gpu, w: &Handle, lora: &Lora, x: &[f32]) -> Vec<f32> {\n    let mut y = gpu.dispatch_gemv(w, x); // frozen base — handle immortal by construction\n    apply_lora_delta(&mut y, x, lora); // y += scale * B(A x), CPU-side\n    y\n}\nfn backward_additive(gpu: &Gpu, w_t: &Handle, lora: &Lora, g_y: &[f32], x_saved: &[f32], g_a: &mut [f32], g_b: &mut [f32]) -> Vec<f32> {\n    let g_lora = lora_proj_backward(g_y, x_saved, lora, g_a, g_b); // A^T(scale B^T g), CPU\n    let mut g_x = gpu.dispatch_gemv(w_t, g_y); // base term, GPU, build-once handle\n    for i in 0..g_x.len() {\n        g_x[i] += g_lora[i]; // the JOIN mirrors the forward's insertion point\n    }\n    g_x\n}\n".to_string(),
        },
        // ── side-consumer-readback-alias-live-handle-for-dependent-dispatch —
        // read-then-reupload → read + alias the live handle (A) ──
        // Source: riir-gpu gemma2_cubecl/dispatch.rs L387-392 ("Sync attn_out
        // (needed for O-LoRA input)" followed by "Re-upload for Wo GEMV" —
        // create_from_slice of bytes still resident) vs the Q4_K sibling
        // L498-503 ("passes attn_out_handle directly to the Wo Q4_K GEMV
        // without reading to CPU") — consumed by gemma2_cubecl_train/mod.rs
        // L223-226 (dispatch_attention_wo_split, Sync 2 of the training
        // forward). q_dim x 4 B x 26 layers x seq = ~53 MB/step at seq 256.
        KernelRule {
            rule_id: "side-consumer-readback-alias-live-handle-for-dependent-dispatch".to_string(),
            gpu_concept: "when a GPU-resident intermediate must be read to CPU for side consumers (a LoRA input, an activation save) AND fed to the next dependent dispatch, read it for the side consumers but hand the dependent launch the still-live device handle — re-uploading the CPU copy burns an extra device allocation plus an upload of bytes that never left the device, and the read sync already establishes ordering so the alias is safe (the Q4_K sibling path passes the handle directly as its standing proof). At Gemma-2-2B seq 256 the wasted re-upload is ~53 MB per forward step (~213 MB at seq 1024). Distinct from fan-out-shared-input-single-sync (N parallel siblings share one UPLOAD — here the data never left the device at all), from defer-readback-until-after-submit (readback timing — the read here is needed and kept), and from split-dispatch-expose-fused-intermediate (avoids a producer re-launch — here the producer runs exactly once and the waste is the re-upload of its still-live output).".to_string(),
            ref_shape: "// naive: read the intermediate for its side consumers, then re-upload the\n// CPU copy for the dependent dispatch — an allocation + upload of bytes\n// that never left the device\nfn chain_naive(client: &Client, attn_out_handle: &Handle) -> Handle {\n    let attn_out = client.read_handle(attn_out_handle); // sync — needed (LoRA input + save)\n    let rehandle = client.create_from_slice(&attn_out); // WASTEFUL re-upload\n    dispatch_gemv(client, &rehandle) // ~53 MB/step of pure re-upload traffic\n}\n".to_string(),
            opt_pattern: "// optimized: read for the side consumers, ALIAS the live handle for the\n// dependent launch — the read sync already ordered the writes\nfn chain_alias(client: &Client, attn_out_handle: &Handle) -> Handle {\n    let _attn_out_cpu = client.read_handle(attn_out_handle); // side consumers only\n    dispatch_gemv(client, &attn_out_handle.clone()) // the device copy is still live\n}\n".to_string(),
        },
        // ── quadratic-attention-intermediates-recompute-from-linear-saves —
        // save-all → save linear, recompute quadratic (A) ──
        // Source: riir-gpu gemma2_cubecl_train/mod.rs L216-220 (the linear
        // save-set: q_post_rope + kv_cache.store — K/V ride the cache that
        // already holds them) + riir-engine transformer/gemma2_train/backward.rs
        // L333-344 (hoisted scratch: "buffers sized to seq_len cover every
        // (p, h) iteration... Hoisting here eliminates 7 x seq_len x n_head
        // allocations per call") + L361-372 (the Theta(t x d) recompute inside
        // the (p, h) loop).
        KernelRule {
            rule_id: "quadratic-attention-intermediates-recompute-from-linear-saves".to_string(),
            gpu_concept: "checkpoint the attention save-set at exactly the Theta(N x d) tensors (post-RoPE Q, plus K and V via the already-resident KV cache) and recompute every Theta(N-squared) intermediate (raw dots, capped scores, softmax weights) at backward in hoisted per-(position, head) scratch — storing the weights instead costs seq-squared x n_head x 4 bytes per layer (~33 MB/layer, ~870 MB across a 26-layer model at seq 1024) while the recompute is Theta(t x d) FLOPs per (p, h) that vanish next to the GEMVs, and the recompute additionally yields the RAW pre-softcap argument that the numerically-correct saturating backward requires — a save-the-scores design would both blow memory AND force the lossy recovery. Distinct from preallocated-activation-slots-inplace-save (slot mechanics for what you DO save — this is the save/recompute size-class boundary that decides WHAT to save) and from latent-encode-at-capture (stores a lossy projection instead of full state — this saves the exact linear inputs and recomputes the rest exactly).".to_string(),
            ref_shape: "// naive: checkpoint every intermediate the backward touches — the\n// Theta(N^2) softmax weights alone cost seq^2 x n_head x 4B per layer\n// (~33 MB/layer at seq 1024; ~870 MB across 26 layers)\nfn forward_save_everything(weights: &mut Vec<Vec<f32>>, q: &[f32], k: &[f32], seq_len: usize, n_head: usize) {\n    for p in 0..seq_len {\n        for h in 0..n_head {\n            let w = softmax(softcap(dot_rows(q, k, p, h))); // Theta(seq) per (p, h)\n            weights.push(w); // Theta(N^2) total per layer — memory blows up\n        }\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: save ONLY the Theta(N x d) linear tensors; recompute the\n// quadratic chain at backward in hoisted per-(p, h) scratch\nfn forward_save_linear(layer_act: &mut LayerAct, kv: &mut KvCache, layer: usize, pos: usize, q_post_rope: &[f32], k: &[f32], v: &[f32], q_dim: usize) {\n    layer_act.q_post_rope[pos][..q_dim].copy_from_slice(&q_post_rope[..q_dim]); // Theta(d)\n    kv.store(layer, pos, k, v); // K/V were already resident in the KV cache\n}\nfn backward_recompute(dots: &mut [f32], q: &[f32], k_cache: &[Vec<f32>], p: usize, n_head: usize, head_dim: usize) {\n    // scratch hoisted once per call, sized to seq_len, fully overwritten —\n    // 7 x seq_len x n_head allocations eliminated\n    for h in 0..n_head {\n        for t in 0..=p {\n            let mut d = 0.0f32;\n            for dd in 0..head_dim {\n                d += q[p * n_head * head_dim + h * head_dim + dd] * k_cache[t][h * head_dim + dd];\n            }\n            dots[t] = d; // raw dot — also feeds the saturation-safe softcap backward\n        }\n    }\n}\n".to_string(),
        },

        // ── constant-scale-folded-into-upload-copy — scale kernel → fold into the upload copy (C) ──
        // Source: riir-gpu gemma2_cubecl_train/resident.rs L85-90 ("let
        // scale = adp.alpha / adp.rank as f32;" + "// Pre-scale B by
        // alpha/rank: b_scaled[r][k] = b[r][k] * scale" + the
        // create_from_slice of b_scaled) + L78-79 ("B is pre-scaled so the
        // GPU GEMV `B @ (A @ input)` directly produces the scaled delta
        // without an extra scale kernel."). Backward contract verified:
        // the backward calls lora_proj_backward_into with the ORIGINAL CPU
        // adapter (resident.rs L1336-1358, L1539-1580) — the engine
        // backward re-derives scale + produces g_b w.r.t. UNSCALED b, so
        // the fold is forward-only and complete.
        //
        // Distinct from factor-affine-quant-scale-out-of-element-loop
        // (hoists the scale out of the loop but still APPLIES it per
        // launch — operand bytes stay unscaled), from
        // delay-rms-scale-past-gemv-epilogue (that scale is DATA-DEPENDENT
        // per-row so it cannot be folded into bytes — constant vs
        // data-dependent is the axis), and from
        // inplace-gate-scale-at-funnel-bottleneck (runtime placement; its
        // frozen-committed-weights prohibition is exactly why THIS rule
        // folds into a per-step COPY, never the source operand).
        KernelRule {
            rule_id: "constant-scale-folded-into-upload-copy".to_string(),
            gpu_concept: "a per-operand scale that is a CONSTANT derived from hyperparameters (LoRA alpha/rank — known at upload time, input-independent) is folded into the byte-level COPY being uploaded, so no kernel ever applies it: the GEMV B_scaled @ (A @ x) directly produces the scaled delta with zero extra scale dispatches and zero in-loop multiplies. The load-bearing safety condition: fold into the upload COPY, never the source operand — the backward and optimizer must still see the unscaled original (the backward re-derives scale from the ORIGINAL adapter and produces gradients w.r.t. the unscaled b). Distinct from factor-affine-quant-scale-out-of-element-loop (hoists the scale out of the loop but still applies it per launch — operand bytes stay unscaled), from delay-rms-scale-past-gemv-epilogue (that scale is DATA-DEPENDENT per-row so it cannot be folded into bytes — constant vs data-dependent is the placement axis), and from inplace-gate-scale-at-funnel-bottleneck (runtime placement of a scale that must stay runtime-visible; its frozen-committed-weights prohibition is exactly why this rule folds into a per-step copy).".to_string(),
            ref_shape: "// naive: upload unscaled B + apply alpha/rank at use — either a\n// scale dispatch per adapter per call, or an in-loop multiply in\n// every consumer\nfn upload_adapter_naive(client: &Client, adp: &LoraAdapter) -> AdapterHandles {\n    let a = client.create_from_slice(f32::as_bytes(&adp.a));\n    let b = client.create_from_slice(f32::as_bytes(&adp.b)); // unscaled\n    // every consumer must apply the scale itself:\n    // inter = gemv(a, x); raw = gemv(b, inter);\n    // delta = scale_mul(raw, alpha / rank) // extra dispatch, every call\n    AdapterHandles { a, b }\n}\n".to_string(),
            opt_pattern: "// optimized: constant scale folded into the UPLOAD COPY — zero\n// scale ops in any kernel; the source operand stays unscaled for\n// the backward / optimizer\nfn upload_adapter_folded(client: &Client, adp: &LoraAdapter) -> AdapterHandles {\n    let scale = adp.alpha / adp.rank as f32; // hyperparameter-derived\n    let b_scaled: Vec<f32> = adp.b.iter().map(|&v| v * scale).collect(); // COPY\n    AdapterHandles {\n        a: client.create_from_slice(f32::as_bytes(&adp.a)),\n        b: client.create_from_slice(f32::as_bytes(&b_scaled)), // pre-scaled\n    }\n}\n// delta = gemv(b_scaled, gemv(a, x)) — no scale anywhere;\n// the backward keeps using the ORIGINAL unscaled adp.b + re-derived scale\n".to_string(),
        },
        // ── side-branch-operand-residency-matches-mutation-lifetime — CPU side branch → per-step device handles (D) ──
        // Source: riir-gpu gemma2_cubecl_train/resident.rs L12-14 ("This
        // eliminates all CPU sync points from the LoRA computation — the
        // previous hybrid approach (CPU LoRA) added ~70K sync round-trips
        // per forward, making the GPU-resident path 4.8× slower than CPU
        // (Issue 004).") + L56-58 ("Call this once per training step (LoRA
        // params change each step).") + L383-385 / L742-743 (the
        // once-per-forward build_lora_gpu_handles call at step entry) +
        // L97-101 ("entirely on GPU (handle-to-handle, zero CPU sync)").
        //
        // Distinct from immutable-operand-upload-at-set-creation (the
        // config-lifetime rung — a constructor cannot host step-mutable
        // operands), from weight-handle-cache-keyed-by-allocation-identity
        // (a reuse cache sound only for immutable contents — an in-place
        // optimizer update keeps pointer identity while changing bytes,
        // staling it), and from hot-swap-weights-preserve-autotune-caches
        // (what to PRESERVE given a refresh cadence — this rule establishes
        // WHERE the upload lives: the step boundary).
        KernelRule {
            rule_id: "side-branch-operand-residency-matches-mutation-lifetime".to_string(),
            gpu_concept: "an auxiliary computation riding a GPU hot path (the LoRA delta beside the base GEMV) must keep ITS OWN operands device-resident, with upload cadence matched to mutation cadence — three rungs: config-lifetime immutable → upload once in the constructor; step-lifetime mutable (LoRA params change each optimizer step) → upload ONCE per step at step entry, amortized across every use site in the step; call-local → per call. A CPU-resident side branch on a GPU pipeline pays a readback+upload round-trip PER USE SITE — the hybrid CPU-LoRA approach added ~70K sync round-trips per forward and ran 4.8x SLOWER than pure CPU. Distinct from immutable-operand-upload-at-set-creation (the config-lifetime rung — cannot host step-mutable operands), from weight-handle-cache-keyed-by-allocation-identity (a reuse cache sound only for immutable contents — an in-place optimizer update stales it), and from hot-swap-weights-preserve-autotune-caches (what to preserve given a refresh — this rule fixes where the upload lives).".to_string(),
            ref_shape: "// naive: side branch computed on CPU while the base runs on GPU —\n// every use site pays read-back + compute + re-upload; syncs scale\n// with USES, not with data\nfn gemv_with_lora_hybrid(client: &Client, w: &Handle, x_cpu: &[f32],\n                         lora: &LoraAdapter) -> Vec<f32> {\n    let base = client.read_one(client.dispatch_gemv(w, upload(x_cpu))); // sync 1\n    let inter = cpu_matvec(&lora.a, x_cpu); // side-branch operands CPU-side\n    let delta = cpu_matvec(&lora.b, &inter);\n    let mut out = base;\n    for i in 0..out.len() {\n        out[i] += delta[i]; // + re-upload merge = sync 2 per use site\n    }\n    out // 7 adapters x 26 layers x seq positions ~= 70K syncs / forward\n}\n".to_string(),
            opt_pattern: "// optimized: adapter operands uploaded ONCE per step at step entry;\n// the delta chain composes handle-to-handle with zero syncs at use\n// sites\nfn step_entry(client: &Client, lora: &GemmaLora) -> Vec<LayerLoraHandles> {\n    lora.layers.iter().map(|l| upload_all_7_adapters(client, l)).collect()\n}\nfn gemv_lora_device(client: &Client, w: &Handle, x: &Handle,\n                    adp: Option<&AdapterHandles>) -> Handle {\n    let base = client.dispatch_gemv(w, x.clone());\n    match adp {\n        Some(a) => {\n            let inter = client.dispatch_gemv(&a.a_handle, x.clone()); // no sync\n            let delta = client.dispatch_gemv(&a.b_handle, inter); // no sync\n            client.dispatch_add(base, delta) // still no sync — handles only\n        }\n        None => base,\n    }\n}\n".to_string(),
        },
        // ── activation-checkpoint-as-handle-clone-deferred-batch-readback — inline reads → clone now, batch-read at tail (D) ──
        // Source: riir-gpu gemma2_cubecl_train/resident.rs L36-37 ("These
        // are GPU handles cloned during the forward pass. They're read
        // back to CPU in a batch at the end of the full forward.") +
        // L959-963 ("Read all 14 activation handles in ONE batched sync
        // (Issue 430 Option C). Previously 14 separate read_handle calls
        // = 14 blocking GPU↔CPU syncs per layer (26 layers × 14 = 364
        // syncs/forward). read_batched issues a single client.read() for
        // all 14 handles = 1 sync per layer (26 total).") + L403-404
        // ("Clone the wte handle before the loop ... Handle::clone is
        // cheap (reference count).").
        //
        // Distinct from batched-downloads-single-sync-cycle (the READ
        // mechanics — this is the save-side contract that makes the batch
        // possible and the compute phase sync-free), from
        // preallocated-activation-slots-inplace-save (the CPU-side
        // destination slots — this is the GPU-side source medium; the two
        // compose: the tail read lands in exactly those slots), and from
        // side-consumer-readback-alias-live-handle-for-dependent-dispatch
        // (inline reads for side consumers — here NOTHING is read during
        // the forward at all).
        KernelRule {
            rule_id: "activation-checkpoint-as-handle-clone-deferred-batch-readback".to_string(),
            gpu_concept: "in a GPU-resident training forward that must checkpoint every intermediate for the backward, the checkpoint medium DURING COMPUTE is the device-handle clone (a refcount bump — zero bytes move, no stall is possible), never an inline readback: this keeps the forward zero-sync and defers all materialization decisions to a tail phase after every dispatch is queued, where read_batched collapses 14 handles per layer into ONE client.read — 364 syncs collapse to 26. Honest footprint caveat: the clones keep ~1.2 GiB of activation handles device-resident at seq 256 / 26 layers, but those bytes must reach the CPU anyway; what the handle medium buys is the freedom to choose the read SHAPE at the tail. Distinct from batched-downloads-single-sync-cycle (the read mechanics — this is the save-side contract that enables them and keeps compute sync-free), from preallocated-activation-slots-inplace-save (the CPU-side destination slots — this is the GPU-side source medium; they compose), and from side-consumer-readback-alias-live-handle-for-dependent-dispatch (inline reads for side consumers — here nothing is read during the forward at all).".to_string(),
            ref_shape: "// naive: read each activation back inline at the production site —\n// the forward serializes on the GPU at every save; syncs scale with\n// saves (14 x layers x positions)\nfn save_inline(client: &Client, produced: &Handle, act: &mut CpuSlots,\n               layer: usize, pos: usize, n: usize) {\n    // blocking read at EVERY production site:\n    act.x[layer][pos][..n].copy_from_slice(&client.read_one(produced.clone()));\n}\n".to_string(),
            opt_pattern: "// optimized: clone the handle at production (a refcount bump — zero\n// bytes move); batch-read at the tail, N handles per ONE client.read\nstruct LayerActHandles {\n    x_input: Handle, // + 13 more, one per saved activation\n}\nfn forward_save(produced: &Handle) -> Handle {\n    produced.clone() // in-loop save — cannot stall\n}\nfn tail_readback(client: &Client, all: &[LayerActHandles], act: &mut CpuSlots) {\n    for (li, h) in all.iter().enumerate() {\n        let v = read_batched(client, vec![h.x_input.clone() /* + 13 more */]);\n        for pos in 0..SEQ {\n            act.layers[li].x_input[pos].copy_from_slice(&v[0][pos * N..(pos + 1) * N]);\n        }\n    } // 14 handles -> 1 sync per layer (was 14); 26 layers: 26 (was 364)\n}\n".to_string(),
        },
        // ── training-forward-shed-decode-loop-shape — per-position loop → whole-sequence tensors (D) ──
        // Source: riir-gpu gemma2_cubecl_train/resident.rs L545-557 (the
        // Plan 482 header: "Processes the full sequence as batched
        // `[seq_len × n_embd]` matrix operations instead of per-position
        // GEMVs. Reduces ~55K per-position dispatches to ~182 batched GEMM
        // dispatches + ~26 CPU attention syncs." + the architecture
        // table) + L683-685 (same claim on the fn) + the decode-shaped
        // contrast (L411-428 the per-position loop + per-position upload;
        // L211-223 the incremental KvStoreCubeCL::launch per (layer, pos))
        // + L757-769 (one-shot embedding build + ONE upload) + L1055-1070
        // (the tail CPU-KV pass from saved activations).
        //
        // Distinct from op-major-backward-loop-inversion-batch-sequence-
        // axis (the BACKWARD instance — here the coupling op is attention
        // itself, and the incremental KV cache is the structural blocker
        // the restructure must shed), from
        // kernelize-host-chain-after-sync-batching-floor (the attention
        // CPU-vs-GPU decision WITHIN the batched shape), and from
        // token-flattened-elementwise-modulo-shared-weights (in-kernel
        // flattening — this is the host-side loop restructure).
        KernelRule {
            rule_id: "training-forward-shed-decode-loop-shape".to_string(),
            gpu_concept: "a teacher-forcing training forward that inherits the DECODE loop shape — per-position iteration, incrementally-written KV cache, per-position GEMVs, per-position input uploads — pays dispatch overhead x layers x positions (~55K dispatches) for work whose only position coupling is causal attention. The axis is freed by making the coupling op whole-sequence too: hoist attention to a full-sequence computation (CPU two-pass or a fused GPU kernel), batch every GEMV into a [seq_len x dim] GEMM, build + upload the embedding block once, and populate whatever cache the backward needs in a single tail pass from saved activations. The incremental KV cache is the structural blocker: it exists for decode, not training. Distinct from op-major-backward-loop-inversion-batch-sequence-axis (the BACKWARD instance — here the coupling op is attention itself and the added content is HOW the axis gets freed), from kernelize-host-chain-after-sync-batching-floor (the attention CPU-vs-GPU decision WITHIN the batched shape), and from token-flattened-elementwise-modulo-shared-weights (in-kernel flattening — this is the host-side loop restructure).".to_string(),
            ref_shape: "// naive (decode shape): per-position loop, incremental KV store,\n// per-position GEMVs + per-position input upload — dispatches scale\n// with layers x positions\nfn forward_decode_shaped(client: &Client, wte: &[f32], tokens: &[usize],\n                         seq_len: usize, n_layer: usize) {\n    for pos in 0..seq_len {\n        let hidden = embed_row(wte, tokens[pos]); // n floats, per pos\n        let x = client.create_from_slice(&hidden); // upload PER POSITION\n        for layer in 0..n_layer {\n            // 7+ GEMV dispatches per (layer, pos);\n            // kv_store.launch(...) into the cache at pos — decode-shaped\n        }\n    } // ~55K dispatches per forward at 26 layers / seq 256\n}\n".to_string(),
            opt_pattern: "// optimized: whole-sequence tensors; attention hoisted to a\n// full-seq op; the backward's cache written in ONE tail pass\nfn forward_batched(client: &Client, wte_cpu: &[f32], tokens: &[usize],\n                   seq_len: usize, n_layer: usize) {\n    let embeds = gather_all_positions(wte_cpu, tokens); // one CPU build\n    let mut x = client.create_from_slice(&embeds); // ONE upload\n    for layer in 0..n_layer {\n        let normed = dispatch_rmsnorm_batched(x, seq_len);\n        let q = gemm_lora(wq(layer), &normed, seq_len); // GEMM, not GEMV\n        // ... k, v, rope_batched(cos_sin_table), wo, gate/up/down ...\n        let attn = full_seq_attention(q, k, v, seq_len); // coupling op\n    }\n    populate_cpu_kv_cache_from_activations(); // tail pass, no per-pos store\n}\n".to_string(),
        },
        // ── sibling-dispatches-before-any-readback — dispatch-read interleave → all dispatches, then all reads (D) ──
        // Source: riir-gpu gemma2_cubecl_train/resident.rs L1087-1095 (the
        // dispatch_gemm_backward_gpu doc: "Returns a GPU handle
        // (non-blocking — call `read_handle` to get the result). This
        // allows dispatching multiple independent GEMVs (e.g. gate+up, or
        // Q+K+V) before reading any of them, so the GPU can pipeline
        // them.") + L1125-1126 ("Independent GEMMs (gate+up, Q+K+V) are
        // dispatched before any readback, allowing the GPU to pipeline
        // them.") + L1363-1372 (gate+up dispatched, THEN both read) +
        // L1515-1525 (q/k/v — three DIFFERENT input buffers — dispatched,
        // then all read).
        //
        // Distinct from fan-out-shared-input-single-sync (premise is ONE
        // shared uploaded input — q/k/v here consume three different
        // gradient buffers, so only the ordering applies), from
        // first-readback-pays-queue-drain (the read-side cost model that
        // MOTIVATES this ordering — it assumes the burst exists; this rule
        // is the code shape that CREATES it), and from
        // chain-dispatches-single-sync-point (dependent chains — these are
        // independent siblings).
        KernelRule {
            rule_id: "sibling-dispatches-before-any-readback".to_string(),
            gpu_concept: "when a non-blocking dispatch helper returns a handle, queue ALL independent sibling dispatches BEFORE issuing the FIRST readback — the interleaved form (dispatch A, read A, dispatch B, read B) serializes completely: read A blocks until sibling A completes, so sibling B's upload + launch latency lands on the critical path after the GPU has gone idle, while the batched form hides siblings 2..N's upload/launch behind sibling 1's on-device execution (one queue executes back-to-back). Applies verbatim when siblings consume DIFFERENT input buffers — the shared-input case is a different rule's premise. API prerequisite: the dispatch helper must return a Handle WITHOUT reading. Distinct from fan-out-shared-input-single-sync (premise is ONE shared uploaded input — here q/k/v consume three different gradient buffers, only the ordering applies), from first-readback-pays-queue-drain (the cost model that MOTIVATES the ordering — it assumes the burst exists; this rule is the code shape that creates it), and from chain-dispatches-single-sync-point (dependent chains — these are independent siblings).".to_string(),
            ref_shape: "// naive: dispatch-read interleave — each read drains before the\n// next sibling is even uploaded; the GPU idles between pairs\nfn gate_up_naive(gpu: &Gpu, bw: &Handles, g_gate: &[f32], g_up: &[f32]) -> (Vec<f32>, Vec<f32>) {\n    let g_normed_gate = gpu.read(gpu.dispatch_gemm(&bw.gate_t, g_gate)); // drain 1\n    let g_normed_up = gpu.read(gpu.dispatch_gemm(&bw.up_t, g_up)); // drain 2 —\n    (g_normed_gate, g_normed_up) // GPU was idle during upload+launch of 2\n}\n".to_string(),
            opt_pattern: "// optimized: dispatch ALL independent siblings, THEN read —\n// siblings' upload+launch latency hides behind sibling 1's\n// on-device execution\nfn gate_up_batched(gpu: &Gpu, bw: &Handles, g_gate: &[f32], g_up: &[f32]) -> (Vec<f32>, Vec<f32>) {\n    let h_gate = gpu.dispatch_gemm(&bw.gate_t, g_gate); // non-blocking Handle\n    let h_up = gpu.dispatch_gemm(&bw.up_t, g_up); // launch overlapped w/ GEMM-1\n    let g_normed_gate = gpu.read_handle(&h_gate); // read 1 drains the burst\n    let g_normed_up = gpu.read_handle(&h_up); // already complete — free\n    (g_normed_gate, g_normed_up)\n}\n".to_string(),
        },
        // ── independent-cpu-work-fills-dispatch-read-window — CPU after read → CPU inside the window (D) ──
        // Source: riir-gpu gemma2_cubecl_train/resident.rs L1326-1351 (the
        // visible-but-unexploited window: Step A2 dispatches the down-GEMM
        // non-blocking, Step A3's read is the very next statement, and the
        // LoRA-down CPU backward then runs — but its inputs are
        // g_x_down_all (the stage input, written in A1) + layer_act.hidden
        // (a saved activation), NEITHER depends on the readback; only the
        // join-add needs it) + the same window at L1365-1372. The
        // non-blocking dispatch API (L1087-1095) makes the hoist legal;
        // the file leaves it unexploited (hazard-class source — the rule
        // teaches the reordering the source misses).
        //
        // Distinct from hybrid-cpu-elementwise-gpu-throughput-sync-points
        // (WHICH side each op runs on — its CPU ops are the DEPENDENT
        // post-read transforms that consume the readback; this rule hoists
        // INDEPENDENT work INTO the dispatch→read window) and from
        // first-readback-pays-queue-drain (the cost model — silent on what
        // the CPU does inside the window).
        KernelRule {
            rule_id: "independent-cpu-work-fills-dispatch-read-window".to_string(),
            gpu_concept: "between a non-blocking dispatch and its blocking read the host is IDLE if the next statement is the read; any CPU work in the same stage that does NOT depend on the read result belongs INSIDE that window — hoisted above the read — so CPU compute overlaps GPU execution and wall time becomes max(GPU, CPU) + join instead of GPU + CPU + join. In a hybrid backward the param-gradient half of an adapter backward is exactly this work: it depends only on the stage INPUT and the saved activation, never on the base-GEMM readback — only the join-add must wait for the read. Distinct from hybrid-cpu-elementwise-gpu-throughput-sync-points (the which-side routing choreography — its CPU ops are the DEPENDENT transforms that consume the read; this hoists INDEPENDENT work INTO the window) and from first-readback-pays-queue-drain (the cost model — says nothing about what the CPU does inside the window).".to_string(),
            ref_shape: "// naive: dispatch -> read immediately -> CPU work that never\n// needed the read result — GPU executes unobserved, then the CPU\n// works unobserved\nfn down_backward_naive(gpu: &Gpu, bw: &Handles, g_x_down_all: &[f32],\n                       hidden: &[Vec<f32>], adp: &Adapter, ga: &mut [f32],\n                       gb: &mut [f32], lora_scratch: &mut [f32]) -> Vec<f32> {\n    let h = gpu.dispatch_gemm(&bw.down_t, g_x_down_all); // non-blocking\n    let g_hidden_all = gpu.read_handle(&h); // host blocks HERE, then...\n    for p in 0..hidden.len() {\n        lora_param_grad(&g_x_down_all[p], &hidden[p], adp, ga, gb); // CPU runs\n        g_hidden_all[p..].copy_from_slice(&lora_scratch[..M]); // AFTER the\n    } // read — but the param-grad half depends on the stage input only\n    g_hidden_all\n}\n".to_string(),
            opt_pattern: "// optimized: split the adapter backward — param-grads (independent\n// of the read) fill the dispatch->read window; only the join waits\nfn down_backward_windowed(gpu: &Gpu, bw: &Handles, g_x_down_all: &[f32],\n                          hidden: &[Vec<f32>], adp: &Adapter, ga: &mut [f32],\n                          gb: &mut [f32], lora_scratch: &mut [f32]) -> Vec<f32> {\n    let h = gpu.dispatch_gemm(&bw.down_t, g_x_down_all); // non-blocking\n    for p in 0..hidden.len() {\n        lora_param_grad(&g_x_down_all[p], &hidden[p], adp, ga, gb); // overlaps\n    } // the GEMM — deps: stage input + saved activation only\n    let g_hidden_all = gpu.read_handle(&h); // blocks AFTER the CPU work\n    for p in 0..hidden.len() {\n        g_hidden_all[p..].copy_from_slice(&lora_scratch[..M]); // join — needs read\n    }\n    g_hidden_all // wall time ~= max(GEMM, CPU LoRA) + join\n}\n".to_string(),
        },
        // ── grad-terms-merge-before-shared-link-jacobian — aux after Jacobian → sum at link input (H) ──
        // Source: riir-gpu gemma2_cubecl_train/resident.rs L1229-1248 (the
        // merge: "// Aux-loss backward, merged into g_logits BEFORE the
        // softcap backward — the same merge point as the CPU path. This
        // loop keeps the softcap fused into the CE expression, so the aux
        // term is added to `gl` rather than to `g_out`." + the fused
        // expression "let gl = softmax_scratch[v] * inv_sum * inv_n_tgt +
        // aux_grad_logits[v] * inv_n_tgt; ... g_out[v] = gl * (1.0 -
        // tanh_val * tanh_val);") + L1210-1212 (the NoAux +0.0
        // degeneration: the aux scratch stays zero so the merged add is
        // +0.0). Both terms are gradients w.r.t. the SAME capped logits —
        // activations.logits stores POST-cap values (L1047-1051), which is
        // exactly why they must merge pre-Jacobian.
        //
        // Distinct from exact-op-parity-for-analytic-backward (the
        // forward/backward op-MATCH axis — this is the merge ORDER of two
        // gradient sources under a shared Jacobian), from
        // fuse-loss-and-output-grad-one-pass (loss scalar + output-grad in
        // one sweep — here no loss scalar is produced at all), and from
        // adapter-grad-join-mirrors-forward-delta-insertion-point (the
        // same placement-matters family, different join: LoRA additive
        // convention, not multi-term linearity).
        KernelRule {
            rule_id: "grad-terms-merge-before-shared-link-jacobian".to_string(),
            gpu_concept: "when multiple loss terms (cross-entropy + an auxiliary loss) reach the parameters through the SAME nonlinear link (logit softcap), their upstream gradients must be SUMMED at the link's INPUT and the link Jacobian applied ONCE to the sum: g_raw = (g_ce + g_aux) * (1 - tanh^2(x/cap)). Adding a term AFTER the Jacobian application silently drops its scaling — the gradient is WRONG, not just slower (the aux term would miss the 1-t^2 factor entirely). Linearity of differentiation makes the pre-link merge both correct and cheapest (one Jacobian application for all terms), and the merge point must mirror the forward's composition order — both terms are gradients w.r.t. the SAME capped logits. The NoAux degeneration is free: the aux scratch stays zero, so the merged add is +0.0. Distinct from exact-op-parity-for-analytic-backward (the op-match axis — this is the merge ORDER of two gradient sources under a shared Jacobian), from fuse-loss-and-output-grad-one-pass (loss scalar + output-grad in one sweep — here no loss scalar is produced), and from adapter-grad-join-mirrors-forward-delta-insertion-point (the same placement-matters family, different join).".to_string(),
            ref_shape: "// naive/WRONG: apply the softcap Jacobian to the CE term, then add\n// the aux gradient AFTER — the aux term never receives the\n// (1 - t^2) scaling\nfn head_backward_wrong(g_ce: &[f32], g_aux: &[f32], logits: &[f32],\n                       cap: f32, g_out: &mut [f32]) {\n    for v in 0..g_ce.len() {\n        let t = logits[v] / cap;\n        g_out[v] = g_ce[v] * (1.0 - t * t); // CE scaled by the Jacobian...\n    }\n    for v in 0..g_aux.len() {\n        g_out[v] += g_aux[v]; // ...aux added UNSCALED — silently wrong grad\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: sum all upstream grads at the link INPUT, apply the\n// shared Jacobian ONCE to the sum — correct by linearity, one\n// fused sweep\nfn head_backward_merged(g_ce: &[f32], g_aux: &[f32], logits: &[f32],\n                        cap: f32, g_out: &mut [f32]) {\n    for v in 0..g_ce.len() {\n        let gl = g_ce[v] + g_aux[v]; // both w.r.t. the SAME capped logits\n        let t = logits[v] / cap; // tanh recovered from the cached output\n        g_out[v] = gl * (1.0 - t * t); // ONE Jacobian for BOTH terms\n    }\n}\n".to_string(),
        },
        // ── kv-window-device-resident-ring-append — append row, not window (A) ──
        // Source: riir-gpu gemma4_q4k_train/mod.rs L1246-1251 (the sliding
        // path: "let combined_kv = self.kv_cache.get_window(layer_idx,
        // t_start, n_pos);" → host copy of the WHOLE window per position)
        // + L979-980 (the re-upload: "let query_handle = ...create_from_slice"
        // + "let kv_handle = ...create_from_slice(f32::as_bytes(combined_kv))")
        // + CpuKVCache::get_window L263-272 (the per-t extend interleave).
        //
        // Distinct from mirror-ring-store-sliding-window-contiguity (B34 —
        // CPU-side LAYOUT so any <=sw window is contiguous; silent on where
        // the ring lives or how often it crosses the bus — this rule MOVES
        // B34's layout across the bus boundary), combined-kv-buffer-keys-
        // then-values (B23 — kernel-side layout for decode flash-attention;
        // here the interleave copy is the symptom, re-upload is the disease),
        // host-cache-immutable-device-weights (B26 — immutable operands; KV
        // is append-mutated every position), side-branch-operand-residency-
        // matches-mutation-lifetime (B36 — aux-branch step-lifetime rung; KV
        // is the main-path operand mutated intra-step).
        KernelRule {
            rule_id: "kv-window-device-resident-ring-append".to_string(),
            gpu_concept: "a KV cache mutated by append and consumed by attention EVERY position must live ON THE DEVICE as a persistent ring — only the NEW row crosses the bus per position. Re-copying the whole visible window to a host buffer and re-uploading it for every query position costs O(sw · kv_dim · 2 · 4 B) host→device bytes per position per sliding layer — the mutable-operand analog of host-cache-immutable-device-weights (immutable lifecycle there; append-mutated here) and the transfer-side complement of mirror-ring-store-sliding-window-contiguity (which fixes the CPU-side layout that this anti-pattern then re-uploads every call). Distinct from combined-kv-buffer-keys-then-values (kernel-side combined-buffer LAYOUT for a decode flash-attention kernel — here the per-call interleave copy is the SYMPTOM; the disease is re-uploading a mutable operand on every consumer call) and from kv-block-streaming (in-kernel smem streaming, not host↔device transfer). The append-vs-snapshot transfer rule: an operand mutated incrementally crosses the bus incrementally.".to_string(),
            ref_shape: "// naive: copy the whole visible window out of the host cache and\n// re-upload it for EVERY query position\nfn attention_window_reupload(cache: &CpuKv, layer: usize, t_start: usize,\n                             n_pos: usize, q: &[f32], gpu: &Gpu) -> Vec<f32> {\n    let mut combined = Vec::with_capacity(n_pos * 2 * cache.row_len(layer));\n    for t in t_start..t_start + n_pos {\n        combined.extend_from_slice(cache.k_row(layer, t)); // host copy of\n        combined.extend_from_slice(cache.v_row(layer, t)); // the WHOLE window\n    }\n    let kv = gpu.upload(&combined); // O(sw) host->device EVERY position\n    let qh = gpu.upload(q);\n    gpu.launch_attention(qh, kv)\n}".to_string(),
            opt_pattern: "// optimized: the ring lives on the device; ONLY the new row crosses\n// the bus per position\nfn attention_ring_append(kv_ring: &mut GpuRing, new_k: &[f32], new_v: &[f32],\n                         pos: usize, capacity: usize, q: &[f32],\n                         gpu: &Gpu) -> Vec<f32> {\n    let row = kv_ring.row_bytes;\n    let k_off = (pos % capacity) * row;\n    let v_off = (capacity + pos % capacity) * row;\n    kv_ring.write_region(k_off, new_k); // ONE row up per position\n    kv_ring.write_region(v_off, new_v); // (k and v)\n    let qh = gpu.upload(q);\n    gpu.launch_attention(qh, kv_ring.handle()) // device-resident ring\n}".to_string(),
        },
        // ── lm-head-loss-positions-only — dead prompt logits never launch (D) ──
        // Source: riir-gpu gemma4_q4k_train/mod.rs L1123 ("let logits =
        // self.dispatch_gemv_q4k(&self.weights_q4k.wte_q4k, &hidden);" —
        // inside training_forward_one_position_q4k, i.e. for EVERY position)
        // + L1077-1080 (the loss consumes only pred_pos = n_in-1+i) +
        // backward.rs L73-76 (the dead-proof: "if d_logits[p].is_empty()
        // { continue; }"). The batched path has the same shape
        // (forward_batched.rs L271-302: lm_head for ALL seq_len positions).
        //
        // Distinct from device-side-argmax-minimal-download (B25 — shrinks
        // a download; this never launches), predicate-gated-skip-empty-
        // workset-step (B30 — whole-step skip; this is a per-position
        // subset inside a live step), monotone-transform-elision-under-
        // argmax (B25 — skips an op under a consumer; this skips the
        // largest GEMV + readback + save under the loss-support set).
        KernelRule {
            rule_id: "lm-head-loss-positions-only".to_string(),
            gpu_concept: "the loss-support set (positions >= n_in - 1 in teacher forcing) is known BEFORE the forward runs — positions outside it never produce a loss term, and their gradients are provably empty (the backward already skips them with an is_empty guard), so the vocab-head GEMV — the single largest GEMV in the model — plus its V-wide readback and its V-wide activation save must be gated to the support set. This is dispatch-level elision under a consumer-set predicate: NOT output reduction (device-side-argmax-minimal-download shrinks a download that still runs; this never launches), NOT a whole-step skip (predicate-gated-skip-empty-workset-step gates an entire training step on an empty workset; this gates a per-position subset INSIDE a live step), and NOT a monotone-consumer elision (monotone-transform-elision-under-argmax skips an OP under a downstream transform; here the skipped work is the model's largest GEMV + its readback + its save).".to_string(),
            ref_shape: "// naive: vocab GEMV + V-wide readback + activation save for EVERY\n// position — prompt positions' logits are never read by the loss OR\n// the backward (their d_logits are provably empty)\nfn lm_head_all_positions(hidden_all: &[Vec<f32>], vocab_head: &Handle,\n                         logits_out: &mut [Vec<f32>]) {\n    for pos in 0..hidden_all.len() {\n        logits_out[pos] = gemv_readback(vocab_head, &hidden_all[pos]);\n    } // (n_in - 1) full vocab GEMVs + readbacks + saves are DEAD per step\n}".to_string(),
            opt_pattern: "// optimized: dispatch the vocab head ONLY at loss-support positions\nfn lm_head_loss_positions_only(hidden_all: &[Vec<f32>], vocab_head: &Handle,\n                               n_in: usize, n_tgt: usize,\n                               logits_out: &mut [Vec<f32>]) {\n    for i in 0..n_tgt {\n        let pos = n_in - 1 + i; // the ONLY positions with a loss term\n        logits_out[pos] = gemv_readback(vocab_head, &hidden_all[pos]);\n    } // prompt positions: no dispatch, no readback, no save — dead by\n} // the backward's d_logits[p].is_empty() skip".to_string(),
        },
        // ── verbatim-upload-on-format-match — Q4_K on disk → Q4_K on GPU (N) ──
        // Source: riir-gpu gemma4_q4k_train/weight_buffers.rs L498-509
        // (upload_q4k_gguf: "if info.ggml_type == GgmlType::Q4_K { //
        // Zero-copy path: cast mmap bytes to blocks + upload." →
        // Q4KHandle::from_blocks verbatim) + the Q6_K fallback L510-521 +
        // mod.rs L433-444 (the from_gguf doc: "This constructor skips the
        // f32 round-trip: Q4_K blocks are read from the GGUF mmap and
        // uploaded verbatim." — the 4090 OOM fix, ~48 GB → one tensor's
        // blocks) + the tiny-operand exception (norm gammas dequantized
        // normally, L442). Proposed independently by BOTH mining agents
        // (merged on collision).
        //
        // Distinct from dtype-normalize-once-at-upload (B31 — where a
        // NEEDED conversion lives; this rule is the decision to not
        // convert at all on format match), mapped-at-creation-init-upload
        // (B31 — one-shot upload mechanics), alignment-gated-zero-copy-view
        // (view gating, not load-path format negotiation),
        // host-cache-immutable-device-weights (B26 — residency).
        KernelRule {
            rule_id: "verbatim-upload-on-format-match".to_string(),
            gpu_concept: "when the on-disk artifact is ALREADY in the device's target quant format, the load path must negotiate on the stored ggml_type and upload the raw blocks verbatim (cast the mmap slice to the block type) — never dequantize the whole tensor to f32 and re-quantize it back. The transcode round-trip materializes the full-precision model in host RAM (GGUF Q4_K -> ~48 GB f32 -> re-quantize: the 4090 OOM class) vs one tensor's block array (~150 MB). dtype-normalize-once-at-upload decides WHERE a needed conversion happens; this rule decides the conversion must NOT happen at all when source format == destination format — with a size-gated exception for tiny operands (norm gammas stay dequantized) and a format-dispatch fallback for mixed-quant files (a Q6_K embedding falls through to dequant+requant). Not mapped-at-creation-init-upload (upload mechanics, orthogonal) and not host-cache-immutable-device-weights (residency, not transcoding).".to_string(),
            ref_shape: "// naive: every tensor goes through f32 regardless of its stored\n// format — Q4_K on disk -> full-precision host buffer -> re-quantize\nfn load_weight_transcode(client: &Client, gguf: &Gguf, name: &str,\n                         m: usize, n: usize) -> Handle {\n    let f32_all: Vec<f32> = gguf.dequant_to_f32(name); // ~48 GB peak\n    let blocks: Vec<Block> = quantize_rows(&f32_all, m, n); // re-quantize\n    Handle::from_blocks(client, &blocks, m, n)\n}".to_string(),
            opt_pattern: "// optimized: negotiate on the stored format — verbatim upload on\n// match, transcode only on mismatch\nfn load_weight_verbatim(client: &Client, gguf: &Gguf, name: &str,\n                        m: usize, n: usize) -> Result<Handle, String> {\n    let info = gguf.tensor_info(name)\n        .ok_or_else(|| format!(\"tensor {name} missing\"))?;\n    if info.ggml_type == GgmlType::Q4K {\n        let blocks: &[Block] = gguf.q4k_blocks(name)?; // cast the mmap\n        return Ok(Handle::from_blocks(client, blocks, m, n)); // verbatim\n    }\n    let f32_all: Vec<f32> = gguf.dequant_to_f32(name); // mismatch only\n    Ok(Handle::from_blocks(client, &quantize_rows(&f32_all, m, n), m, n))\n}".to_string(),
        },
        // ── stream-drain-free-source-during-build — peak O(unit), not O(model) (L) ──
        // Source: riir-gpu gemma4_q4k_train/mod.rs L610-635
        // (build_backward_handles_owned: "// Process wte first, then free
        // its f32 data." → weights.wte.clear() + shrink_to_fit() →
        // "for l in weights.layers.drain(..)" → "// l is dropped here,
        // freeing this layer's f32 data (~800 MB).") + weight_buffers.rs
        // L370-380 (the from_gguf analog: "The f32 buffer is dropped
        // before the next tensor starts").
        //
        // Distinct from superset-sized-scratch-across-variants (B32a —
        // sizes REUSED scratch; this frees the SOURCE during a one-shot
        // transform), staging-reuse-grow-only-map-prefix (B31 — reuse
        // across calls; nothing is consumed),
        // activation-checkpoint-as-handle-clone-deferred-batch-readback
        // (B36 — compute-time activation lifetime; this is construction-
        // time source lifetime).
        KernelRule {
            rule_id: "stream-drain-free-source-during-build".to_string(),
            gpu_concept: "a one-time build that transforms + uploads N independent units (layers) from a source representation must DRAIN the source per unit — process, upload, drop — so peak host memory is O(one unit), not O(model). Holding the full source alive while every unit's transformed copy accumulates pays the sum (full f32 model + per-layer transposes); the drain caps it at max(one unit's source + one transform buffer) and is what makes a 32 GB-RAM box able to build a 12B model's GPU buffers at all. The build-time analog of activation-checkpoint-as-handle-clone-deferred-batch-readback (compute-time lifetime) and NOT scratch sizing (superset-sized-scratch-across-variants sizes REUSED scratch; this frees the SOURCE during a one-shot transform) nor staging reuse (staging-reuse-grow-only-map-prefix reuses a buffer across calls; no source is consumed).".to_string(),
            ref_shape: "// naive: source alive for the WHOLE build + each unit's transformed\n// copy accumulates — peak = full model + one transform\nfn build_all_then_hold(layers: &[LayerSrc]) -> Vec<Handle> {\n    let mut handles = Vec::with_capacity(layers.len());\n    for layer in layers {\n        handles.push(transpose_quantize_upload(layer));\n    }\n    handles\n}".to_string(),
            opt_pattern: "// optimized: drain the source — each unit is dropped after its\n// upload, so peak = one unit + one transform buffer\nfn build_draining(layers: &mut Vec<LayerSrc>) -> Vec<Handle> {\n    let mut handles = Vec::with_capacity(layers.len());\n    for layer in layers.drain(..) {\n        handles.push(transpose_quantize_upload(&layer));\n    } // `layer` (and its f32 data) dropped per iteration\n    handles\n}".to_string(),
        },
    ]
}
