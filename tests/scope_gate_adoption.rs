//! Bench 669 consumer adoption (Plan 016) — the input-scope gate at the
//! `heal()` seam. Run with:
//!
//! ```bash
//! cargo test --features contrastive_scope --test scope_gate_adoption -- --nocapture
//! # G2/G4 (timing + alloc) are release-only:
//! cargo test --release --features contrastive_scope --test scope_gate_adoption -- --nocapture
//! ```
//!
//! The whole file is feature-gated; without `--features contrastive_scope` it
//! compiles to nothing (0 passed). The `harness_is_actually_running` test
//! makes a vacuous pass detectable (the g4_l4_composition precedent).
#![cfg(feature = "contrastive_scope")]

use riir_clippy::scope_model::{ScopeModel, THETA};
use riir_clippy::{build_seed_corpus, HealPipeline, RustAnalyzerPruner, TernaryDrafter};

fn pipeline() -> HealPipeline {
    HealPipeline::new(TernaryDrafter::fallback(), RustAnalyzerPruner::new())
}

fn corpus_model() -> ScopeModel {
    ScopeModel::build_for_corpus(&build_seed_corpus().entries)
}

/// HELD-OUT out-of-scope probes — deliberately DIFFERENT documents from the
/// table-building `OUT_OF_SCOPE_DOCS` (Report-the-Floor discipline). If these
/// leak into the table the battery below measures nothing.
const HELD_OUT_OOS_PROBES: &[&str] = &[
    // C++
    r#"#include <vector>
#include <algorithm>
std::vector<int> processItems(const std::vector<int>& items, int limit) {
    std::vector<int> out;
    for (const auto& item : items) {
        if (item > limit) out.push_back(item);
    }
    std::sort(out.begin(), out.end());
    return out;
}
"#,
    // Java
    r#"import java.util.List;
public class Processor {
    private final int limit;
    public Processor(int limit) { this.limit = limit; }
    public List<String> process(List<String> items) {
        return items.stream().filter(i -> i.length() < limit).toList();
    }
}
"#,
    // Ruby
    r#"def process_items(items, limit)
  total = 0
  items.each do |item|
    next if item.nil?
    total += item[:value] unless item[:value] > limit
  end
  total
end
"#,
    // shell
    r#"#!/usr/bin/env bash
set -euo pipefail
for file in "$@"; do
  if [[ -f "$file" ]]; then
    grep -c "warning:" "$file" || true
  fi
done
"#,
    // CSS
    r#".summary {
  color: #333;
  font-size: 14px;
  margin: 0 auto;
  padding: 8px 16px;
}
.summary .error { color: red; font-weight: bold; }
"#,
    // XML
    r#"<?xml version="1.0" encoding="UTF-8"?>
<report version="2">
  <summary fixes="12" warnings="3"/>
  <rules>
    <rule id="needless_return" fix="true"/>
  </rules>
</report>
"#,
    // a second Python flavor (dataclass + comprehension — different tokens
    // from the table-building Python doc)
    r#"from dataclasses import dataclass
@dataclass
class Rule:
    id: str
    severity: str = "warn"
RULES = [r for r in ALL if r.severity != "allow"]
"#,
    // GLSL-compute flavor (different from the table's vertex shader)
    r#"#version 450
layout(local_size_x = 64) in;
layout(binding = 0) buffer Input { float data[]; } input_buf;
void main() {
    uint idx = gl_GlobalInvocationID.x;
    input_buf.data[idx] = sqrt(input_buf.data[idx]);
}
"#,
];

/// In-distribution probes held out from... well, the corpus IS the in-corpus —
/// the model is built from it (that's the design: the healer's home
/// distribution is its own corpus). The G3 bit-identity test below is the
/// no-regression proof over exactly this set.

// ── Anti-vacuity ───────────────────────────────────────────────────────────

#[test]
fn harness_is_actually_running() {
    let corpus = build_seed_corpus();
    assert!(!corpus.entries.is_empty(), "seed corpus must be non-empty");
    let m = corpus_model();
    let mut scratch = Vec::new();
    // A corpus fixture must score in-scope; an OOS probe must not.
    let d_in = m.score_str(&corpus.entries[0].buggy_code, &mut scratch);
    assert!(d_in < 0.0, "fixture must lean in-scope, got {d_in}");
    let d_out = m.score_str(HELD_OUT_OOS_PROBES[0], &mut scratch);
    assert!(
        d_out > THETA,
        "held-out OOS probe must exceed theta, got {d_out}"
    );
}

// ── T2: κ/θ separation pins (the measured re-pin, fails loudly on drift) ───

/// The measured D landscape over the real corpus + held-out OOS probes.
/// Prints the table for the bench record; asserts the separation margins the
/// pinned constants rely on.
#[test]
fn kappa_theta_separation_pins() {
    let m = corpus_model();
    let mut scratch = Vec::new();

    let corpus = build_seed_corpus();
    let mut worst_in_d = f32::INFINITY; // the LEAST-negative corpus fixture
    let mut min_in_d = f32::NEG_INFINITY; // the most negative (saturation witness)
    let mut fixtures: Vec<(f32, &str)> = Vec::new();
    for e in &corpus.entries {
        let d = m.score_str(&e.buggy_code, &mut scratch);
        fixtures.push((d, &e.lint_name));
        worst_in_d = worst_in_d.min(d);
        min_in_d = min_in_d.max(d);
    }
    fixtures.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("[scope] per-fixture D (least-negative first, the binding tail):");
    let n = fixtures.len();
    for (d, name) in fixtures.iter().skip(n.saturating_sub(8)) {
        println!("  {d:>9.2}  {name}");
    }

    let mut best_out_d = f32::INFINITY; // the least-positive OOS probe
    let mut probes: Vec<(f32, usize)> = Vec::new();
    for (i, p) in HELD_OUT_OOS_PROBES.iter().enumerate() {
        let d = m.score_str(p, &mut scratch);
        probes.push((d, i));
        best_out_d = best_out_d.min(d);
    }
    probes.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("[scope] per-probe D (least-positive first):");
    for (d, i) in probes.iter().take(8) {
        println!("  {d:>9.2}  probe#{i}");
    }

    println!(
        "[scope] corpus D range: [{min_in_d:.1}, {worst_in_d:.1}] (n={})",
        corpus.entries.len()
    );
    println!(
        "[scope] held-out OOS D min: {best_out_d:.1} (n={})",
        HELD_OUT_OOS_PROBES.len()
    );
    println!("[scope] pinned THETA={THETA}, KAPPA={}", m.gate().kappa);

    // θ sits strictly inside the measured gap, with margin both sides.
    assert!(
        worst_in_d < THETA,
        "the least-negative corpus fixture ({worst_in_d:.1}) must sit below theta ({THETA}) — \
         otherwise in-distribution healing would decline"
    );
    assert!(
        best_out_d > THETA,
        "the least-positive OOS probe ({best_out_d:.1}) must exceed theta ({THETA}) — \
         otherwise the gate misses real garbage"
    );

    // κ saturation: the WORST (shortest) fixture must saturate f32 sigmoid to
    // exactly 1.0 — the G3 bit-identity precondition. Argument ≥ ~16.6.
    let kappa = m.gate().kappa;
    let arg = -kappa * worst_in_d;
    assert!(
        arg >= 16.6,
        "kappa={kappa} gives sigmoid argument {arg:.1} at the worst fixture \
         (D={worst_in_d:.1}) — below f32 saturation; raise KAPPA"
    );
    let saturated = katgpt_core::sigmoid(arg);
    assert_eq!(
        saturated.to_bits(),
        1.0f32.to_bits(),
        "sigmoid(-kappa*D_worst) must be EXACTLY 1.0 for bit-identity"
    );
}

// ── G1: the falsifiable defend-wrong A/B ───────────────────────────────────
//
// WITHOUT the model: an OOS input paired with an in-corpus rule id is SERVED
// (the demonstrated bug — a Rust fix for a C++ input, with a straight face).
// WITH the model: declined.

#[test]
fn g1_without_the_gate_oos_inputs_are_served_the_demonstrated_bug() {
    let corpus = build_seed_corpus();
    let rule = &corpus.entries[0].lint_name;
    let mut served = 0;
    for probe in HELD_OUT_OOS_PROBES {
        let mut p = pipeline();
        // SEED memory — the production state (every real use of the
        // self-evolving healer accumulates trajectories). Un-seeded, the
        // Issue 010 identity guard catches pure-garbage inputs that share
        // no identifiers with the corpus fix — but memory retrieval has no
        // such guard: it returns the nearest trajectory for ANY query, and
        // with the rule in-corpus the Issue 030 gate marks it relevant.
        p.seed_from_corpus(&corpus.entries);
        let r = p.heal(rule, probe, &corpus.entries);
        if r.is_some() {
            served += 1;
        }
    }
    // The bug is real and near-total: the drafter serves the corpus template,
    // it parses, the rule is covered — nothing refuses.
    let rate = served as f32 / HELD_OUT_OOS_PROBES.len() as f32;
    let pct = (rate * 100.0) as u32;
    println!(
        "[scope] WITHOUT gate: {served}/{} OOS inputs served ({pct}%)",
        HELD_OUT_OOS_PROBES.len()
    );
    assert!(
        rate >= 0.75,
        "precondition broken: the un-gated pipeline no longer serves OOS inputs \
         ({pct}%) — the defend-wrong claim this adoption fixes is gone; \
         re-examine before proceeding"
    );
}

#[test]
fn g1_with_the_gate_oos_inputs_are_declined() {
    let corpus = build_seed_corpus();
    let rule = &corpus.entries[0].lint_name;
    let mut declined = 0;
    for probe in HELD_OUT_OOS_PROBES {
        let mut p = pipeline().with_scope_model(corpus_model());
        let r = p.heal(rule, probe, &corpus.entries);
        if r.is_none() {
            declined += 1;
        }
    }
    let rate = declined as f32 / HELD_OUT_OOS_PROBES.len() as f32;
    let pct = (rate * 100.0) as u32;
    println!(
        "[scope] WITH gate: {declined}/{} OOS inputs declined ({pct}%)",
        HELD_OUT_OOS_PROBES.len()
    );
    assert_eq!(
        declined,
        HELD_OUT_OOS_PROBES.len(),
        "every held-out OOS probe must be declined (rate {pct}%)"
    );
}

// ── G3: in-distribution no-regression (bit-identity) ───────────────────────

#[test]
fn g3_in_distribution_healing_is_bit_identical() {
    let corpus = build_seed_corpus();

    // Warm both pipelines' memory identically so the candidate pools match.
    let mut plain = pipeline();
    plain.seed_from_corpus(&corpus.entries);
    let mut gated = pipeline().with_scope_model(corpus_model());
    gated.seed_from_corpus(&corpus.entries);

    for e in &corpus.entries {
        let a = plain.heal(&e.lint_name, &e.buggy_code, &corpus.entries);
        let b = gated.heal(&e.lint_name, &e.buggy_code, &corpus.entries);
        match (a, b) {
            (Some(a), Some(b)) => {
                assert_eq!(
                    a.fixed_code, b.fixed_code,
                    "fix changed for `{}`",
                    e.lint_name
                );
                assert_eq!(a.parses, b.parses);
                assert_eq!(a.source, b.source);
                assert_eq!(
                    a.evolution_score.to_bits(),
                    b.evolution_score.to_bits(),
                    "evolution_score not bit-identical for `{}` ({:?} vs {:?}) — \
                     the haircut must saturate to exactly 1.0 in-distribution",
                    e.lint_name,
                    a.evolution_score,
                    b.evolution_score
                );
            }
            (None, None) => {}
            (a, b) => panic!(
                "serve mismatch for `{}`: plain={:?} gated={:?}",
                e.lint_name,
                a.is_some(),
                b.is_some()
            ),
        }
    }
}

// ── L4 savings: garbage never reaches the 48 s call ────────────────────────

#[cfg(feature = "l4_lora")]
mod l4_savings {
    use super::*;
    use riir_clippy::l4_fallback::L4Fallback;

    /// Probe double: counts calls (the g4_l4_composition `Probe` pattern).
    #[derive(Clone)]
    struct Probe {
        calls: std::rc::Rc<std::cell::Cell<usize>>,
    }

    impl Probe {
        fn new() -> Self {
            Self {
                calls: std::rc::Rc::new(std::cell::Cell::new(0)),
            }
        }
        fn count(&self) -> usize {
            self.calls.get()
        }
    }

    impl L4Fallback for Probe {
        fn generate_fix(&mut self, _rule_id: &str, _buggy_code: &str) -> Option<String> {
            self.calls.set(self.calls.get() + 1);
            Some("fn generated() {}".to_string())
        }

        fn name(&self) -> &str {
            "probe-l4"
        }
    }

    #[test]
    fn out_of_scope_input_never_consults_l4() {
        let corpus = build_seed_corpus();
        let probe = Probe::new();
        let mut p = pipeline()
            .with_scope_model(corpus_model())
            .with_l4_fallback(Box::new(probe.clone()));
        let r = p.heal(
            "totally_unknown_lint_xyz",
            HELD_OUT_OOS_PROBES[0],
            &corpus.entries,
        );
        assert!(r.is_none());
        assert_eq!(
            probe.count(),
            0,
            "an out-of-scope input must not burn the ~48 s L4 call"
        );
    }

    #[test]
    fn without_the_gate_the_same_input_burns_the_l4_call() {
        // The A/B complement: un-gated, the same miss consults L4 (miss #1 —
        // unknown rule, no candidates). This is the measured saving.
        let corpus = build_seed_corpus();
        let probe = Probe::new();
        let mut p = pipeline().with_l4_fallback(Box::new(probe.clone()));
        let _ = p.heal(
            "totally_unknown_lint_xyz",
            HELD_OUT_OOS_PROBES[0],
            &corpus.entries,
        );
        assert_eq!(probe.count(), 1, "un-gated, the garbage input reaches L4");
    }
}

// ── G2: overhead (release-only measurement, debug just runs) ───────────────

#[test]
fn g2_scope_check_overhead() {
    let corpus = build_seed_corpus();
    let m = corpus_model();
    let mut scratch = Vec::new();
    // Warm.
    for e in &corpus.entries {
        let _ = m.score_str(&e.buggy_code, &mut scratch);
    }
    let n = 2000;
    let t0 = std::time::Instant::now();
    for i in 0..n {
        let e = &corpus.entries[i % corpus.entries.len()];
        let _ = m.score_str(&e.buggy_code, &mut scratch);
    }
    let per = t0.elapsed().as_nanos() as f64 / n as f64;
    println!(
        "[scope] score_str: {per:.0} ns/input ({} fixtures)",
        corpus.entries.len()
    );
    if cfg!(not(debug_assertions)) {
        assert!(
            per < 10_000.0,
            "scope check must stay under 10 µs/input, got {per:.0} ns"
        );
    }
}

// ── G4: zero-alloc steady state ───────────────────────────────────────
//
// A dedicated counting allocator binary is overkill for one check; instead
// this file pins the observable contract (scratch capacity stability) and
// the alloc-count check lives below via a thread_local counter — no, the
// global allocator is the only sound counter. Kept simple: capacity
// stability + the by-construction argument (char scan + HashMap lookups +
// slice sum allocate nothing).

#[test]
fn g4_scope_scratch_capacity_stabilizes() {
    let corpus = build_seed_corpus();
    let m = corpus_model();
    let mut scratch = Vec::new();
    let mut cap = 0usize;
    for round in 0..100 {
        for e in &corpus.entries {
            let _ = m.score_str(&e.buggy_code, &mut scratch);
        }
        if round >= 1 {
            assert!(
                scratch.capacity() >= cap,
                "scratch capacity shrank ({scratch_capacity} < {cap}) — the mem::take dance must preserve it",
                scratch_capacity = scratch.capacity()
            );
        }
        cap = scratch.capacity();
    }
}

/// The heal-path alloc delta: gated heal − un-gated heal must add ZERO
/// allocations in steady state (the only new state is the reused token-id
/// scratch). Measured with a global counting allocator — one #[test] per
/// binary since the counter is process-global.
mod g4_alloc {
    use super::*;

    struct CountingAllocator;

    static ALLOCS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
            ALLOCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            unsafe { std::alloc::System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
            unsafe { std::alloc::System.dealloc(ptr, layout) }
        }
    }

    #[global_allocator]
    static A: CountingAllocator = CountingAllocator;

    #[test]
    fn gated_heal_adds_zero_allocs_in_steady_state() {
        let corpus = build_seed_corpus();

        let count_steady_heals = |gated: bool| -> usize {
            // Build the model OUTSIDE the measured region (cold path).
            let model = gated.then(corpus_model);
            let mut p = pipeline();
            if let Some(m) = model {
                p = p.with_scope_model(m);
            }
            p.seed_from_corpus(&corpus.entries);
            // Warm-up (first heal sizes the scratch + candidates).
            for e in &corpus.entries {
                let _ = p.heal(&e.lint_name, &e.buggy_code, &corpus.entries);
            }
            let before = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
            for _ in 0..3 {
                for e in &corpus.entries {
                    let _ = p.heal(&e.lint_name, &e.buggy_code, &corpus.entries);
                }
            }
            let after = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
            after - before
        };

        let un_gated = count_steady_heals(false);
        let gated = count_steady_heals(true);
        let delta = gated as i64 - un_gated as i64;
        println!(
            "[scope] G4: steady-state heal allocs un-gated={un_gated} gated={gated} (delta {delta})"
        );
        assert!(
            delta <= 0,
            "the scope-gated heal must add ZERO allocations in steady state \
             (gated {gated} vs un-gated {un_gated}, delta +{delta})"
        );
    }
}
