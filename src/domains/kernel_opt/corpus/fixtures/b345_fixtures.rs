//! Batch 345 fixture — the accelerator-backend first-tile acceptance gate
//! (`accelerator-backend-first-tile-acceptance-gate`) as ONE standalone file.
//!
//! Compiled AND executed BEFORE entry: `rustc --edition 2021 --test
//! b345_fixtures.rs` (the B336/B338/B339/B341 discipline). The rule's
//! ref/opt arms in `entries_gpu_154.rs` embed the same executed code.
//! ORIGINAL Rust throughout — the accelerator/Core ML surface is modeled as
//! a pluggable backend trait; the fixture never calls any real ML runtime.
//!
//! F1 = accelerator-backend-first-tile-acceptance-gate (kernel_opt):
//! (a) the acceptance gate: a new backend must reproduce the reference
//! result within the quality floor on the FIRST REAL INPUT and must not be
//! meaningfully slower than the reference path, or the job keeps the
//! reference; (b) the verdict is CACHED per machine identity (a later job
//! skips the gate; a changed identity re-gates); (c) the naive arm adopts
//! on conversion success alone — a wrong-but-fast backend silently serves;
//! (d) the input-shape ceiling: an input above the measured conversion
//! ceiling keeps the reference instead of attempting the conversion.

use std::cell::Cell;
use std::collections::HashMap;

/// a tiny deterministic "model": a 4-tap box blur over u8 rows, the
/// reference answer computed in plain scalar code.
fn reference_forward(input: &[u8], width: usize) -> Vec<u8> {
    let height = input.len() / width;
    let mut out = vec![0u8; input.len()];
    for y in 0..height {
        for x in 0..width {
            let mut acc = 0u32;
            let mut n = 0u32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let ny = y as i32 + dy;
                    let nx = x as i32 + dx;
                    if ny >= 0 && (ny as usize) < height && nx >= 0 && (nx as usize) < width {
                        acc += u32::from(input[ny as usize * width + nx as usize]);
                        n += 1;
                    }
                }
            }
            out[y * width + x] = (acc / n) as u8;
        }
    }
    out
}

/// the candidate backend: fast but subtly wrong (an off-by-one tap window)
/// unless `faithful` is set.
fn candidate_forward(input: &[u8], width: usize, faithful: bool) -> Vec<u8> {
    let height = input.len() / width;
    let mut out = vec![0u8; input.len()];
    for y in 0..height {
        for x in 0..width {
            let mut acc = 0u32;
            let mut n = 0u32;
            let x1 = if faithful { x } else { x + 1 }; // the subtle defect
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let ny = y as i32 + dy;
                    let nx = x1 as i32 + dx;
                    if ny >= 0 && (ny as usize) < height && nx >= 0 && (nx as usize) < width {
                        acc += u32::from(input[ny as usize * width + nx as usize]);
                        n += 1;
                    }
                }
            }
            out[y * width + x] = if n > 0 { (acc / n) as u8 } else { 0 };
        }
    }
    out
}

fn psnr_like(reference: &[u8], candidate: &[u8]) -> f64 {
    // fixture-grade quality proxy: mean absolute error -> a pseudo-PSNR
    let mae: f64 = reference
        .iter()
        .zip(candidate.iter())
        .map(|(a, b)| f64::from(a.abs_diff(*b)))
        .sum::<f64>()
        / reference.len().max(1) as f64;
    if mae == 0.0 {
        100.0
    } else {
        (20.0 * (255.0 / mae).log10()).min(100.0)
    }
}

const MINIMUM_QUALITY: f64 = 45.0;
const SPEED_MARGIN: f64 = 1.3;
/// measured conversion ceiling (inputs above it exhaust memory converting)
const MAXIMUM_PIXELS: usize = 64 * 64;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct MachineIdentity {
    backend_rev: u32,
    os_rev: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum BackendVerdict {
    Accepted,
    Rejected { why: &'static str },
}

struct Gate {
    identity: MachineIdentity,
    cache: HashMap<MachineIdentity, BackendVerdict>,
}

impl Gate {
    fn new(identity: MachineIdentity) -> Self {
        Self { identity, cache: HashMap::new() }
    }

    /// FIXED: the first real input is the gate — quality floor AND speed
    /// margin against the reference, verdict cached per machine identity.
    fn adopt_or_keep(
        &mut self,
        first_input: &[u8],
        width: usize,
        candidate_time_us: u64,
        reference_time_us: u64,
        candidate_time_fn: &dyn Fn(&[u8], usize) -> Vec<u8>,
    ) -> BackendVerdict {
        if let Some(v) = self.cache.get(&self.identity) {
            return *v;
        }
        if first_input.len() > MAXIMUM_PIXELS {
            let v = BackendVerdict::Rejected { why: "input above the measured conversion ceiling" };
            self.cache.insert(self.identity, v);
            return v;
        }
        let reference = reference_forward(first_input, width);
        let candidate = candidate_time_fn(first_input, width);
        let quality = psnr_like(&reference, &candidate);
        if quality < MINIMUM_QUALITY {
            let v = BackendVerdict::Rejected { why: "quality below the floor on the first real input" };
            self.cache.insert(self.identity, v);
            return v;
        }
        if (candidate_time_us as f64) > SPEED_MARGIN * reference_time_us as f64 {
            let v = BackendVerdict::Rejected { why: "meaningfully slower than the reference path" };
            self.cache.insert(self.identity, v);
            return v;
        }
        let v = BackendVerdict::Accepted;
        self.cache.insert(self.identity, v);
        v
    }
}

/// NAIVE: adopt because the backend COMPILED/converted — the first real
/// frame silently serves the wrong answer.
fn adopt_naive(converted_ok: bool) -> BackendVerdict {
    if converted_ok {
        BackendVerdict::Accepted
    } else {
        BackendVerdict::Rejected { why: "conversion failed" }
    }
}

#[test]
fn f1_quality_floor_refuses_a_wrong_but_convertible_backend() {
    let input: Vec<u8> = (0..32 * 32).map(|i| ((i * 7) % 251) as u8).collect();
    let faithful = |i: &[u8], w: usize| candidate_forward(i, w, true);
    let mut gate = Gate::new(MachineIdentity { backend_rev: 1, os_rev: 1 });
    let v = gate.adopt_or_keep(&input, 32, 500, 900, &faithful);
    assert_eq!(v, BackendVerdict::Accepted, "a faithful, faster backend is accepted");

    let unfaithful = |i: &[u8], w: usize| candidate_forward(i, w, false);
    let mut gate2 = Gate::new(MachineIdentity { backend_rev: 2, os_rev: 1 });
    let v2 = gate2.adopt_or_keep(&input, 32, 500, 900, &unfaithful);
    assert!(
        matches!(v2, BackendVerdict::Rejected { why: "quality below the floor on the first real input" }),
        "a wrong-but-fast backend refuses on quality, got {v2:?}"
    );
    // the naive arm adopted the same broken backend on conversion success
    assert_eq!(adopt_naive(true), BackendVerdict::Accepted);
}

#[test]
fn f1_speed_margin_refuses_a_slow_backend() {
    let input: Vec<u8> = (0..16 * 16).map(|i| ((i * 13) % 249) as u8).collect();
    let faithful = |i: &[u8], w: usize| candidate_forward(i, w, true);
    let mut gate = Gate::new(MachineIdentity { backend_rev: 3, os_rev: 1 });
    let v = gate.adopt_or_keep(&input, 16, 5_000, 900, &faithful);
    assert!(
        matches!(v, BackendVerdict::Rejected { why: "meaningfully slower than the reference path" }),
        "a slower backend keeps the reference, got {v:?}"
    );
}

#[test]
fn f1_verdict_cached_per_identity_new_identity_regates() {
    let input: Vec<u8> = (0..16 * 16).map(|i| ((i * 3) % 253) as u8).collect();
    let faithful = |i: &[u8], w: usize| candidate_forward(i, w, true);
    let mut gate = Gate::new(MachineIdentity { backend_rev: 9, os_rev: 4 });
    let runs = Cell::new(0u32);
    let counting = |i: &[u8], w: usize| -> Vec<u8> {
        runs.set(runs.get() + 1);
        faithful(i, w)
    };
    let v1 = gate.adopt_or_keep(&input, 16, 500, 900, &counting);
    let n1 = runs.get();
    let v2 = gate.adopt_or_keep(&input, 16, 500, 900, &counting);
    assert_eq!(v1, v2);
    assert_eq!(runs.get(), n1, "a cached verdict skips the gate entirely");

    // a toolchain/OS bump is a NEW identity: it re-gates
    gate.identity = MachineIdentity { backend_rev: 9, os_rev: 5 };
    let _ = gate.adopt_or_keep(&input, 16, 500, 900, &counting);
    assert!(runs.get() > n1, "the changed identity re-ran the gate");
}

#[test]
fn f1_input_above_the_ceiling_keeps_the_reference() {
    let big: Vec<u8> = vec![128; 200 * 200];
    let faithful = |i: &[u8], w: usize| candidate_forward(i, w, true);
    let mut gate = Gate::new(MachineIdentity { backend_rev: 5, os_rev: 1 });
    let v = gate.adopt_or_keep(&big, 200, 500, 900, &faithful);
    assert!(
        matches!(v, BackendVerdict::Rejected { why: "input above the measured conversion ceiling" }),
        "an over-ceiling input never attempts the conversion, got {v:?}"
    );
}
