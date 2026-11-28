//! Batch 430 kernel fixture preflight — the omarchy-mlx Vulkan
//! barrier-scope distill (Research 433; joshuaswarren/omarchy-mlx @
//! `52ac565fc08cee86bf437b0ec7d29f4d617f4c9c`, PR 604, MIT; the article
//! joshuawarren.com/blog/mlx-vulkan-beats-llama-cpp-cuda-on-rtx-5060-ti-egpu,
//! 2026-11-22).
//!
//! ORIGINAL plain-Rust reproduction — zero repo source pasted. The Vulkan
//! encoder surface is MODELED as a deterministic state machine; what the
//! fixtures pin are the RELATIONSHIPS the rule prices, never the hardware's
//! numbers (the rule is NotBenchable-by-domain — the GPU busy-time win is
//! the source's own measurement).
//!
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b430_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - R1 (compute-to-compute-narrow-barrier-transfer-guarded): a
//!   dispatch-only hazard past the batch head with no transfer since the
//!   last full barrier and wave scheduling off takes the NARROW
//!   compute-to-compute barrier; a transfer (copy/fill/inline update)
//!   since the last full barrier forces the FULL barrier until the next
//!   one clears the flag; the batch head itself and wave-scheduled
//!   submits keep the full barrier; the env override beats the device
//!   default in both directions; only the full barrier clears the
//!   transfer flag (the narrow barrier leaves it set — the hazard class
//!   is not discharged by a dispatch-only sync).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Barrier {
    /// full ALL_COMMANDS/MEMORY both-ways dependency
    Full,
    /// narrow COMPUTE_SHADER -> COMPUTE_SHADER, SHADER_WRITE -> READ|WRITE
    ComputeToCompute,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DeviceKind {
    DiscreteGpu,
    IntegratedGpu,
    Cpu, // software Vulkan
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Node {
    /// a compute dispatch with a dispatch-only hazard
    Dispatch,
    /// a transfer: copy, fill, or inline update
    Transfer,
}

/// the encoder's hazard state (encoder.cpp fields + compute_barrier()).
struct Encoder {
    device: DeviceKind,
    /// past the batch head? the first node of a fresh command buffer is not
    head_synced: bool,
    /// a copy/fill/update ran since the last FULL barrier
    transfer_since_barrier: bool,
    /// wave scheduling armed for this submit
    wave_sched: bool,
    /// env override: None = unset (device default), Some(true/false) = 1/0
    compute_barrier_env: Option<bool>,
    /// the tracker says this node's ranges overlap unsynced work (so a
    /// barrier is OWED at all — the gated-barriers layer runs first)
    hazard_owed: bool,
}

impl Encoder {
    /// select_compute_barrier: default on for DISCRETE GPUs only, env
    /// override 1/0 in both directions (device_local_activations.h).
    fn compute_barrier_enabled(&self) -> bool {
        match self.compute_barrier_env {
            Some(v) => v,
            None => self.device == DeviceKind::DiscreteGpu,
        }
    }

    /// the barrier a hazard-owed node takes (encoder.cpp
    /// record_dispatch_barrier):
    /// if (!compute_barrier() || !head_synced_ || transfer_since_barrier_)
    ///     -> FULL   else -> NARROW.
    fn barrier_for(&self, node: Node) -> Barrier {
        if !self.hazard_owed {
            // gated barriers elided it entirely — no barrier recorded
            return Barrier::Full; // unreachable in these arms; full = safe
        }
        match node {
            Node::Transfer => Barrier::Full,
            Node::Dispatch => {
                if !self.compute_barrier_enabled()
                    || !self.head_synced
                    || self.transfer_since_barrier
                    || self.wave_sched
                {
                    Barrier::Full
                } else {
                    Barrier::ComputeToCompute
                }
            }
        }
    }

    /// state update after the barrier records. The source's own state
    /// machine: EVERY barrier — narrow or full — restarts the batch-head
    /// sync (head_synced_ = true in both record paths); only the FULL
    /// barrier clears transfer_since_barrier_. A transfer NODE sets the
    /// flag when it records, and its own barrier is always full.
    fn commit(&mut self, node: Node, took: Barrier) {
        if took == Barrier::Full {
            // record_dependency_barrier: tracker_.clear(); head_synced_ =
            // true; transfer_since_barrier_ = false.
            self.head_synced = true;
            self.transfer_since_barrier = false;
        } else {
            // record_dispatch_barrier: tracker_.clear(); head_synced_ =
            // true; the transfer flag NOT touched.
            self.head_synced = true;
        }
        if node == Node::Transfer {
            // the copy/fill/update sets the flag AFTER its own (full)
            // barrier — the next dispatch's hazard includes it
            self.transfer_since_barrier = true;
        }
    }
}

/// models one recorded node through the encoder and returns the barrier.
fn record(enc: &mut Encoder, node: Node) -> Barrier {
    let took = enc.barrier_for(node);
    enc.commit(node, took);
    took
}

#[test]
fn r1_pure_compute_chain_takes_the_narrow_barrier() {
    let mut enc = Encoder {
        device: DeviceKind::DiscreteGpu,
        head_synced: true, // past the batch head
        transfer_since_barrier: false,
        wave_sched: false,
        compute_barrier_env: None,
        hazard_owed: true,
    };
    // a long chain of dependent compute dispatches: every barrier is the
    // narrow compute-to-compute one; the transfer flag never sets.
    for _ in 0..321 {
        assert_eq!(record(&mut enc, Node::Dispatch), Barrier::ComputeToCompute);
        assert!(!enc.transfer_since_barrier);
    }
}

#[test]
fn r1_transfer_since_barrier_forces_full_until_the_next_full() {
    let mut enc = Encoder {
        device: DeviceKind::DiscreteGpu,
        head_synced: true,
        transfer_since_barrier: false,
        wave_sched: false,
        compute_barrier_env: None,
        hazard_owed: true,
    };
    // a copy lands: it takes the FULL barrier itself, which clears the
    // flag first, then the transfer NODE sets it for the next hazard
    assert_eq!(record(&mut enc, Node::Transfer), Barrier::Full);
    assert!(enc.transfer_since_barrier);
    // the next dispatch's hazard may include the transfer: FULL, narrow
    // refused even on a discrete GPU — and that full barrier clears the
    // flag again
    assert_eq!(record(&mut enc, Node::Dispatch), Barrier::Full);
    // only the FULL barrier cleared the flag — now narrow again
    assert!(!enc.transfer_since_barrier);
    assert_eq!(record(&mut enc, Node::Dispatch), Barrier::ComputeToCompute);
}

#[test]
fn r1_batch_head_and_wave_sched_keep_the_full_barrier() {
    // the batch head: work arriving after host writes takes the full one
    let mut head = Encoder {
        device: DeviceKind::DiscreteGpu,
        head_synced: false,
        transfer_since_barrier: false,
        wave_sched: false,
        compute_barrier_env: None,
        hazard_owed: true,
    };
    assert_eq!(record(&mut head, Node::Dispatch), Barrier::Full);
    assert!(head.head_synced); // the full barrier started the batch head

    // wave scheduling: submit-order reasoning is not per-node, the full
    // barrier stands
    let mut wave = Encoder {
        device: DeviceKind::DiscreteGpu,
        head_synced: true,
        transfer_since_barrier: false,
        wave_sched: true,
        compute_barrier_env: None,
        hazard_owed: true,
    };
    assert_eq!(record(&mut wave, Node::Dispatch), Barrier::Full);
}

#[test]
fn r1_device_default_and_env_override_both_directions() {
    let mk = |device, env| Encoder {
        device,
        head_synced: true,
        transfer_since_barrier: false,
        wave_sched: false,
        compute_barrier_env: env,
        hazard_owed: true,
    };
    // default: discrete narrow, integrated + software full
    assert_eq!(
        record(&mut mk(DeviceKind::DiscreteGpu, None), Node::Dispatch),
        Barrier::ComputeToCompute
    );
    assert_eq!(
        record(&mut mk(DeviceKind::IntegratedGpu, None), Node::Dispatch),
        Barrier::Full
    );
    assert_eq!(
        record(&mut mk(DeviceKind::Cpu, None), Node::Dispatch),
        Barrier::Full
    );
    // =0 forces the full barrier ON a discrete GPU
    assert_eq!(
        record(
            &mut mk(DeviceKind::DiscreteGpu, Some(false)),
            Node::Dispatch
        ),
        Barrier::Full
    );
    // =1 forces the narrow barrier ON an integrated GPU
    assert_eq!(
        record(
            &mut mk(DeviceKind::IntegratedGpu, Some(true)),
            Node::Dispatch
        ),
        Barrier::ComputeToCompute
    );
}

#[test]
fn r1_schedule_shape_is_identical_only_the_scope_narrows() {
    // the source's own invariant: the barrier COUNT is unchanged (still
    // one per hazard-owed node), only the SCOPE narrows — model the
    // schedule as hazard-owed flags and count barriers both ways.
    let hazards = [true, true, false, true, true, true, false, true];
    let mut full_count = 0;
    let mut narrow_count = 0;
    let mut enc = Encoder {
        device: DeviceKind::DiscreteGpu,
        head_synced: true,
        transfer_since_barrier: false,
        wave_sched: false,
        compute_barrier_env: None,
        hazard_owed: true,
    };
    for &owed in &hazards {
        enc.hazard_owed = owed;
        if !owed {
            // gated barriers elided it entirely — no barrier recorded
            continue;
        }
        let took = record(&mut enc, Node::Dispatch);
        match took {
            Barrier::Full => full_count += 1,
            Barrier::ComputeToCompute => narrow_count += 1,
        }
    }
    // every hazard-owed node took exactly one barrier (schedule identical);
    // the scope split is the win
    assert_eq!(
        full_count + narrow_count,
        hazards.iter().filter(|&&h| h).count()
    );
    assert!(narrow_count >= 1, "the pure-compute majority went narrow");
}
