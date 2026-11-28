//! Batch 430 — the omarchy-mlx Vulkan barrier-scope distill (Research 433).
//! Source: joshuaswarren/omarchy-mlx @
//! `52ac565fc08cee86bf437b0ec7d29f4d617f4c9c` (PR 604, merged 2026-11-21;
//! MIT — Copyright © 2026 Joshua Warren and mlx-omarchy contributors, over
//! Apple ml-explore/mlx MIT; the fork-local Vulkan backend overlay is the
//! mineable surface, MLX upstream has NO Vulkan backend — decision content
//! only, never a PR). ONE rule — the narrow compute-to-compute dispatch
//! barrier whose four-condition predicate is the decision content; the
//! count-neutral census face rides its host
//! (`merge-dispatches-into-single-compute-pass`, the Batch-142
//! launch-count rider). NotBenchable-by-domain (Vulkan-host win metric —
//! the dial9/136 annotation; the state machine is CPU-testable and the
//! fixture pins it exact). Fixture preflight: `b430_fixtures` standalone-
//! compiled AND executed pre-entry (debug AND -O, 5/5 both postures).
use crate::domains::kernel_opt::KernelRule;

#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![

        KernelRule {
            rule_id: "compute-to-compute-narrow-barrier-transfer-guarded".to_string(),
            gpu_concept: r"MECHANISM (host encoder barrier-SCOPE narrowing; the sync-structure class): a long decode chain of short dependent compute kernels pays a FULL pipeline dependency between every pair — 90,003 dispatches over a 256-token Qwen3-4B run emitted 82,238 barriers (321/token) even after the gated tracker elided the non-overlapping hazards (10,678 skipped), and every one was a full `VK_PIPELINE_STAGE_ALL_COMMANDS_BIT` both-ways synchronization whose pipeline drain cost ~1.87 ms of GPU busy time per token (~2.0 ms wall). The repair keeps the barrier COUNT and the synchronization SCHEDULE bit-identical — still 82,238 emitted, 10,678 skipped, 1,525 submissions — and narrows the SCOPE of the barriers whose hazard can only be an earlier dispatch: `COMPUTE_SHADER` → `COMPUTE_SHADER`, `SHADER_WRITE` → `SHADER_READ | SHADER_WRITE`, instead of `ALL_COMMANDS`/`MEMORY` both ways. THE FOUR-CONDITION PREDICATE is the decision content (all four must hold together): (1) the node is a compute dispatch; (2) the encoder is past the batch head (`head_synced_`) — work arriving after host writes or an earlier batch takes the full barrier, because the first node of a fresh command buffer must order the host's noncoherent flush; (3) no transfer ran since the last full barrier (`transfer_since_barrier_`) — every `vkCmdCopyBuffer`, `vkCmdFillBuffer`, and inline `vkCmdUpdateBuffer` sets the flag, and only the full `record_dependency_barrier()` clears it, so a transfer-to-compute hazard can never take the narrow form and one copy poisons the narrow path until the next full barrier; (4) wave scheduling is off — submit-order reasoning is not per-node. `if (!compute_barrier() || !head_synced_ || transfer_since_barrier_) record_dependency_barrier();` else the narrow one. THE DEVICE GATE: default ON for discrete GPUs only, by device TYPE (`VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU`) — integrated (Apple) GPUs and software Vulkan keep the full barrier until their own A/B; the env override beats the device default in BOTH directions (`=1` forces narrow on integrated, `=0` refuses narrow on discrete), read ONCE (the gate shapes recorded commands; flipping mid-batch desyncs the tracker). BOTH barrier paths restart the batch-head sync and clear the tracker; only the full path clears the transfer flag. THE STATE MACHINE IS THE FIXTURE: pure-compute chains stay narrow forever (321 barriers/token all narrow), one transfer forces full-until-cleared. MEASURED (source-reported, the article + the in-source comment at the pin — provenance, never an adoption bar): RTX 5060 Ti 16 GB eGPU over USB4, Qwen3-4B 4-bit greedy, 256-token median of 5 interleaved pairs: 99 → 124 tok/s (+25.8%, 5/5 pairs), token-6-onward +27.4%, GPU busy −1.87 ms/token, generated token IDs identical across all ten runs (`f0684b4aa9b35796`) — beating llama.cpp CUDA (~101 tok/s) on the same card. LANE CONTEXT: the host-orchestration barrier-scope axis sits BESIDE the wgpu pass-merging answer our own wgpu postures already own (merge-dispatches-into-single-compute-pass owns collapsing the pass boundary; this narrows the dependency INSIDE the boundary — a stage-mask question that lives below wgpu's abstraction, reachable only via as_hal raw barriers; no lane of ours emits raw Vulkan barriers today, the residual lever is LATENT). REGIME: host command encoders over explicit-Sync APIs (Vulkan/D3D12/Metal) driving long chains of short dependent compute dispatches (autoregressive decode); hazard tracking with SPIR-V reflected read/write binding masks (read-read pairs skip entirely); discrete GPUs measured, Apple-class integrated unproven (the source's own carve). ANTI-REGIME: graphs containing ANY transfer-to-compute hazard since the last full barrier (the flag law — one copy poisons the chain); integrated/software Vulkan postures unmeasured; wave-scheduled submits (the barrier belongs to the wave head, not the node); consumers needing cross-queue or host-visibility semantics (the full barrier's domain). ⛔ KINSHIP (adjudicated both directions, dedup greps at landing HEAD 1,593 ids): `plane-storage-barrier-scope-covers-global-exchange` (gpu_144, Dispatch) owns the OPPOSITE duty — the barrier scope must CONTAIN every memory class the exchange crosses (narrow TOO far = the defect); THIS narrows only under a proven-disjoint hazard predicate and keeps the full barrier for every transfer/host class, the covering law intact. `merge-dispatches-into-single-compute-pass` (gpu_02, Dispatch) collapses the pass BOUNDARY (N passes → 1); THIS narrows the dependency INSIDE the boundary (scope, not count) — they compose (fewer boundaries, each as narrow as its hazard proof). `two-pass-partial-reduction-inter-dispatch-barrier` (gpu_116, Compute) owns the inter-dispatch barrier as a REQUIRED load-bearing dependency for split reductions — this rule never removes a required barrier, it only re-scopes the ones the schedule already records. `tail-simdgroup-cull-barriers-preserved` (gpu_12) is the device-side complement (cull compute, NEVER the barrier — collective divergence); `no-blocking-sync-in-async-pipeline` + `batched-downloads-single-sync-cycle` own the SYNC-COUNT axis (queue waits, readback polls), not the scope axis. GREP EVIDENCE (3+ vocabularies over kernel_opt at landing HEAD): 'barrier scope|stage-mask|ALL_COMMANDS|compute-to-compute|transfer_since' → zero owners of the scope axis beside the covering rule; 'narrow barrier' → zero. NotBenchable-by-domain (Vulkan host-orchestration win metric — the validator prices code deltas, not encoder decisions; the four-condition predicate, the flag law, and the device gate are CPU-testable and ARE the fixtures — the annotation class, never a demotion). UPSTREAM: fork-local, PRESENT at the pin default-on for discrete GPUs — decision content only, never a PR (MLX upstream has no Vulkan backend; the fork MIT over Apple MIT). Source: joshuaswarren/omarchy-mlx @ 52ac565fc08cee86bf437b0ec7d29f4d617f4c9c (PR 604, MIT) — overlay/mlx/backend/omarchy/encoder.cpp (record_dispatch_barrier :205-233, compute_barrier :190-202, transfer flag sites :503/:590/:637) + device_local_activations.h:233-240 (select_compute_barrier + the measured comment) + encoder.h:631-641 (the state contract) + docs/kernel-flags.md MLX_OMARCHY_COMPUTE_BARRIER row; the article joshuawarren.com/blog/mlx-vulkan-beats-llama-cpp-cuda-on-rtx-5060-ti-egpu (2026-11-22) for the 90,003/82,238/10,678/1,525 schedule counts + the +25.8% table. Every quote verified at the pin (fresh clone 2026-11-28, tip cccad655 past it — the mined surfaces untouched by the 83 commits between). Batch 430 / Research 433.".to_string(),
            ref_shape: r"// NAIVE: every hazard-owed dispatch takes the FULL dependency —
// ALL_COMMANDS/MEMORY both ways, 321 barriers/token, the pipeline
// drain paid even when the only hazard is compute->compute.
fn record_barrier(node: Node, state: &mut Enc) {
    let _ = node;
    full_barrier(state.cmd); // ALL_COMMANDS -> ALL_COMMANDS, MEMORY rw -> rw
    state.tracker.clear();
    state.head_synced = true;
    state.transfer_since_barrier = false;
}

// the defect shape: a pure-compute chain still pays the full scope
for _ in 0..321 {
    record_barrier(Node::Dispatch, &mut enc); // 321 full drains/token
}".to_string(),
            opt_pattern: r"// FIXED: the scope follows the HAZARD CLASS — a dispatch-only
// hazard past the batch head, transfer-free, wave-free takes the
// narrow compute->compute form; everything else keeps the full one.
fn record_dispatch_barrier(enc: &mut Enc) {
    if !compute_barrier(enc.device, enc.env)
        || !enc.head_synced          // the batch head orders the host flush
        || enc.transfer_since_barrier // a copy poisoned the chain
        || enc.wave_sched             // submit-order reasoning, not per-node
    {
        record_dependency_barrier(enc); // full: ALL_COMMANDS, rw -> rw
        return;                          // + clears the transfer flag
    }
    narrow_barrier(enc.cmd); // COMPUTE_SHADER -> COMPUTE_SHADER,
    enc.tracker.clear();     // SHADER_WRITE -> SHADER_READ|WRITE
    enc.head_synced = true;  // the transfer flag NOT touched
}

// per transfer node: the full barrier, THEN the flag
fn on_copy(enc: &mut Enc) {
    record_dependency_barrier(enc); // full first
    enc.transfer_since_barrier = true; // poisons the narrow path
} // until the next full barrier clears it".to_string(),
        },
    ]
}
