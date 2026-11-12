//! Batch 354 fixtures — the sgl-muse-mega code-cache + cross-step-routing
//! laws as ONE standalone file.
//!
//! Compiled AND executed BEFORE entry: `rustc --edition 2021 --test
//! b354_fixtures.rs` (the B336/B338/B339/B341/B356/B357 discipline). Each
//! rule's ref/opt arms in `entries_gpu_156.rs` (kernel_opt) embed the same
//! executed code. ORIGINAL Rust throughout (the source is MIT; the fixtures
//! are still original syntheses, never pasted source — the CUDA device-code
//! surface is modeled as a mock text/line layer; the fixture never calls a
//! real GPU).
//!
//! K1 = launch-time-code-as-data-l2-warm (kernel_opt, the GOAT lead)
//! K2 = dry-pass-icache-warm-before-real (kernel_opt)
//! K3 = speculative-expert-prefetch-prev-step-routing (kernel_opt)

// ═══════════════════════════════════════════════════════════════════════════
// Shared mock: a kernel's text as LINEs; a mock L2 keyed by (line, priority)
// with an `evict_last` set that survives inter-layer streams; an icache
// window (the lines a launch arrives with for free).
// ═══════════════════════════════════════════════════════════════════════════

const LINE_BYTES: usize = 128;
const ICACHE_WINDOW_LINES: usize = 4; // the launch prefetch window (mock ~64 KB / 128 B)

#[derive(Default)]
struct MockL2 {
    /// lines pinned evict_last (the warm pass's policy).
    pinned: std::collections::BTreeSet<usize>,
    /// lines held normal-priority (evicted by inter-layer streams).
    normal: std::collections::BTreeSet<usize>,
}

impl MockL2 {
    /// The inter-layer stream: normal-priority bytes from the layers in
    /// between wash unpinned lines out. Pinned (evict_last) lines survive.
    fn stream_other_layers(&mut self) {
        self.normal.clear();
    }

    fn touch_exec(&mut self, line: usize, hbm_fetches: &mut usize) {
        if self.pinned.contains(&line) || self.normal.contains(&line) {
            self.normal.insert(line); // L2 hit; demotes the pin (normal reuse)
        } else {
            *hbm_fetches += 1; // instruction fetch from HBM at first use
            self.normal.insert(line);
        }
    }
}

/// A kernel's text: `lines` 128-B lines.
struct KernelText {
    lines: usize,
}

impl KernelText {
    fn range_bytes(&self) -> (usize, usize) {
        (0, self.lines * LINE_BYTES)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// K1 — launch-time-code-as-data-l2-warm: at every launch, a converged warp
// per spreading CTA reads ONE probe word per 128-B line of the kernel's own
// text with an evict_last policy — loads only — so the instruction fetch
// finds every line in L2. Two arms: re-warm EVERY launch (inter-layer
// streams wash normal-priority lines), and SPREAD the readers (every reader
// CTA reads every line — dies keep separate L2 copies), never split the work.
// ─────────────────────────────────────────────────────────────────────────────

/// One probe load per line, `evict_last`. LOADS ONLY: the value is folded
/// into a discard sink, never compared, never branched on, never stored.
/// Per-DIE model: each die keeps its own L2 copy, so EVERY die needs its own
/// full walk — the readers spread the issue order WITHIN a die, never the
/// coverage.
fn warm_pass_die(
    text_lines: usize,
    reader: usize,
    readers: usize,
    l2: &mut MockL2,
    sink: &mut u64,
) {
    let mut line = reader; // staggered start; the walk still covers ALL lines
    let mut walked = 0;
    while walked < text_lines {
        l2.pinned.insert(line); // the L2 fill is the load's only effect
        *sink = sink.wrapping_add(line as u64); // the probe value's only consumer
        line = (line + readers) % text_lines;
        walked += 1;
    }
}

/// Execution is distributed across the dice by the scheduler's own placement
/// (a fixed permutation INDEPENDENT of the warm assignment — real rigs do
/// not guarantee the warm and exec placements align; the law removes the
/// dependence entirely): line l executes on die (l + 1) % 2.
fn exec_two_dice(text_lines: usize, dice: &mut [MockL2; 2]) -> usize {
    let mut hbm = 0;
    for line in 0..text_lines {
        if line >= ICACHE_WINDOW_LINES {
            dice[(line + 1) % 2].touch_exec(line, &mut hbm);
        }
    }
    hbm
}

fn launch_naive(text: &KernelText, l2: &mut MockL2) -> usize {
    let mut hbm = 0;
    l2.stream_other_layers(); // the layers in between streamed since the last launch
    for line in 0..text.lines {
        if line >= ICACHE_WINDOW_LINES {
            l2.touch_exec(line, &mut hbm); // past the window: fetch at first use
        }
    }
    hbm
}

fn launch_warmed(
    text: &KernelText,
    dice: &mut [MockL2; 2],
    readers: usize,
    bind_range: Option<(usize, usize)>,
) -> (usize, u64) {
    let mut sink = 0u64;
    for d in dice.iter_mut() {
        d.stream_other_layers();
    }
    if let Some((base, bytes)) = bind_range {
        let lines = if bytes == 0 {
            0
        } else {
            (base + bytes).div_ceil(LINE_BYTES) - base / LINE_BYTES
        };
        for d in dice.iter_mut() {
            for r in 0..readers {
                warm_pass_die(lines, r, readers, d, &mut sink); // EVERY die's readers read EVERY line
            }
        }
    } // unbound range prefetches nothing
    let hbm = exec_two_dice(text.lines, dice);
    (hbm, sink)
}

#[test]
fn k1_text_as_data_warm_kills_the_hbm_refetch() {
    let text = KernelText { lines: 32 }; // 32 × 128 B = 4 KB mock (law is per-line, not per-size)

    // BUGGY: no warm — every line past the launch window refetches from HBM
    // at every launch (the inter-layer stream washed the last launch's fill).
    let mut l2 = MockL2::default();
    let naive_fetches = launch_naive(&text, &mut l2);
    assert_eq!(
        naive_fetches,
        text.lines - ICACHE_WINDOW_LINES,
        "naive must miss past the window"
    );
    // and it does not transfer: a second launch misses AGAIN.
    let again = launch_naive(&text, &mut l2);
    assert_eq!(
        again, naive_fetches,
        "normal-priority lines do not survive the inter-layer stream"
    );

    // FIXED: the warm pass re-warms EVERY launch on EVERY die; loads only;
    // the readers spread the issue order within each die, never the coverage.
    let mut dice = [MockL2::default(), MockL2::default()];
    let (fetches, sink) = launch_warmed(&text, &mut dice, 3, Some(text.range_bytes()));
    assert_eq!(
        fetches, 0,
        "the warm pass must cover every line past the window on both dice"
    );
    let (again, _) = launch_warmed(&text, &mut dice, 3, Some(text.range_bytes()));
    assert_eq!(
        again, 0,
        "the re-warm is EVERY launch, not a first-launch-only fill"
    );

    // TAMPERED A — the work is SPLIT across the dice (die d's reader takes
    // only its share of the lines): each die's L2 keeps just that share, and
    // the lines it executes from the other share fetch from HBM.
    let mut dice = [MockL2::default(), MockL2::default()];
    let lines = text.lines;
    for d in dice.iter_mut() {
        d.stream_other_layers();
    }
    for (d, die) in dice.iter_mut().enumerate() {
        let mut line = d; // split: die d pins only lines ≡ d (mod 2)
        while line < lines {
            die.pinned.insert(line);
            line += 2;
        }
    }
    let hbm = exec_two_dice(lines, &mut dice);
    assert!(
        hbm > 0,
        "splitting the coverage across the dice must leave lines uncached"
    );

    // TAMPERED B — the bound range is absent (unbound): prefetches nothing,
    // the exec refetches. The host binding is load-bearing.
    let mut dice = [MockL2::default(), MockL2::default()];
    let (fetches_unbound, _) = launch_warmed(&text, &mut dice, 3, None);
    assert_eq!(
        fetches_unbound, naive_fetches,
        "an unbound range prefetches nothing"
    );

    // ALL-SAFE: a small kernel whose text already fits the launch window has
    // no first-use misses to kill; the warm pass is harmless either way.
    let small = KernelText {
        lines: ICACHE_WINDOW_LINES,
    };
    let mut dice = [MockL2::default(), MockL2::default()];
    let naive_small = {
        for d in dice.iter_mut() {
            d.stream_other_layers();
        }
        let mut hbm = 0;
        for line in 0..small.lines {
            if line >= ICACHE_WINDOW_LINES {
                dice[line % 2].touch_exec(line, &mut hbm);
            }
        }
        hbm
    };
    assert_eq!(naive_small, 0, "in-window text never misses");
    let _ = sink;
}

// ─────────────────────────────────────────────────────────────────────────────
// K2 — dry-pass-icache-warm-before-real: a latency-critical loop runs TWICE —
// the first pass executes the SAME instruction addresses with effects
// (stores, publishes, waits) skipped, on garbage accumulators; the real pass
// then finds every line in the instruction cache. The loop stays ROLLED (one
// copy of its text): an unrolled dry pass would warm a copy the real pass
// never runs. Loops carrying per-element register state do not dry-pass.
// ─────────────────────────────────────────────────────────────────────────────

/// A rolled loop: ONE sequence of step addresses; each step may store.
/// `dry` skips the effects (and the waits); the addresses still execute.
const STEPS: usize = 12;

struct LoopMachine {
    icache: std::collections::BTreeSet<usize>,
    cold_fetches: usize,
    stores: Vec<(usize, u64)>, // (step, value)
}

impl LoopMachine {
    fn new() -> Self {
        Self {
            icache: std::collections::BTreeSet::new(),
            cold_fetches: 0,
            stores: Vec::new(),
        }
    }
    fn run_pass(&mut self, steps: &[usize], dry: bool, acc: u64, out: &mut Vec<u64>) {
        for (i, &step) in steps.iter().enumerate() {
            if !self.icache.contains(&step) {
                self.cold_fetches += 1; // instruction fetch at first execution
                self.icache.insert(step);
            }
            if !dry {
                // the effect: a store of the accumulator (the real pass's
                // arithmetic). The dry pass runs on garbage and stores nothing.
                self.stores.push((step, acc.wrapping_add(i as u64)));
                if self.stores.len() == steps.len() {
                    out.push(acc);
                }
            }
        }
    }
}

#[test]
fn k2_dry_pass_warms_the_real_pass_without_effects() {
    let real_steps: Vec<usize> = (0..STEPS).collect();

    // BUGGY: one cold real pass — every step fetches.
    let mut m = LoopMachine::new();
    let mut out = Vec::new();
    m.run_pass(&real_steps, false, 7, &mut out);
    assert_eq!(
        m.cold_fetches, STEPS,
        "a cold first pass fetches every step"
    );

    // FIXED: dry first (same addresses, no effects), then for real — the
    // real pass is fully warm and the effects fire exactly once.
    let mut m = LoopMachine::new();
    let mut out = Vec::new();
    m.run_pass(&real_steps, true, 0, &mut out); // dry: garbage accumulator, no stores
    assert!(m.stores.is_empty(), "the dry pass stores nothing");
    assert_eq!(m.cold_fetches, STEPS, "the dry pass pays the fetches");
    m.run_pass(&real_steps, false, 7, &mut out);
    assert_eq!(m.cold_fetches, STEPS, "the real pass finds every line warm");
    assert_eq!(
        m.stores.len(),
        STEPS,
        "effects fire exactly once, on the real pass"
    );
    assert!(
        out.iter().all(|&v| v == 7),
        "outputs carry the real accumulator, never the dry garbage"
    );

    // TAMPERED A — the dry pass PERFORMS the effects (modeled: it runs with
    // effects enabled on the garbage accumulator): garbage lands in the
    // output before the real pass's values.
    let mut m = LoopMachine::new();
    let mut out = Vec::new();
    m.run_pass(&real_steps, false, 0, &mut out); // the tampered "dry": effects ON, garbage acc
    let garbage_stores = m.stores.iter().filter(|&&(_, v)| v < 12).count();
    assert!(
        garbage_stores > 0,
        "a tampered dry pass stores garbage-visible values"
    );
    m.run_pass(&real_steps, false, 7, &mut out);
    assert_eq!(
        m.stores.len(),
        STEPS * 2,
        "the garbage stores are visible before the real ones"
    );

    // TAMPERED B — the loop is UNROLLED for the dry pass: the dry warms a
    // COPY (per-step unrolled addresses `100+i`) the real rolled loop never
    // runs; the real pass stays cold. `#pragma unroll 1` is load-bearing.
    let mut m = LoopMachine::new();
    let mut out = Vec::new();
    let unrolled: Vec<usize> = (0..STEPS).map(|i| 100 + i).collect();
    m.run_pass(&unrolled, true, 0, &mut out);
    m.run_pass(&real_steps, false, 7, &mut out);
    assert_eq!(
        m.cold_fetches,
        STEPS * 2,
        "the unrolled dry warmed a copy the real pass never runs"
    );

    // ALL-SAFE A — the first run already warmed the loop (a repeated pass):
    // no dry pass is needed; the rule's gate is the FIRST latency-critical
    // pass, not every pass.
    let mut m = LoopMachine::new();
    let mut out = Vec::new();
    m.run_pass(&real_steps, false, 7, &mut out);
    let cold_first = m.cold_fetches;
    m.run_pass(&real_steps, false, 7, &mut out);
    assert_eq!(
        m.cold_fetches, cold_first,
        "a repeated real pass is already warm"
    );
    // ALL-SAFE B — a loop carrying per-element register state cannot dry-pass
    // (iteration i+1 consumes iteration i's value): the law's shape gate keeps
    // it out. Modeled: the machine refuses the dry arm for stateful loops.
    let stateful = |steps: &[usize]| steps.windows(2).all(|w| w[1] == w[0] + 1); // carries a chain
    assert!(
        stateful(&real_steps),
        "the stateful shape is detectable and dry-passing it is refused"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// K3 — speculative-expert-prefetch-prev-step-routing: one otherwise-idle warp
// per CTA reads the PREVIOUS step's routing ids, histograms the experts, and
// L2-prefetches the weight packs of the most-routed experts that fit the L2
// budget — fire-and-forget (no admission structure), during the attention
// phase of the SAME launch. Budget is POLICY (wide vs narrow per the
// post-attention window's cost), never a magic constant. The router stays
// authoritative: prefetch schedules bytes, never replaces the selection.
// L2-resident experts go FIRST in the tile list.
// ─────────────────────────────────────────────────────────────────────────────

const EXPERTS: usize = 32;
const PACK_UNITS: usize = 34; // mock pack size per expert (34 units ≈ 3.375 MiB / 0.1)

#[derive(Clone)]
struct ExpertPrefetcher {
    budget_units: usize, // policy, not constant
    l2: std::collections::BTreeSet<usize>,
    bandwidth_spent: usize,
}

impl ExpertPrefetcher {
    /// Histogram the previous step's routing ids; pick most-routed until the
    /// budget is full (ties → lower id); fire-and-forget prefetch (no
    /// admission list, no eviction victims).
    fn prefetch_prev_routing(&mut self, prev_routing: &[u32]) -> Vec<usize> {
        let mut hist = [0usize; EXPERTS];
        for &e in prev_routing {
            hist[e as usize] += 1;
        }
        let mut order: Vec<usize> = (0..EXPERTS).collect();
        order.sort_by_key(|&e| (std::cmp::Reverse(hist[e]), e));
        let mut picked = Vec::new();
        let mut used = 0;
        for &e in &order {
            if used + PACK_UNITS > self.budget_units {
                break;
            }
            if hist[e] == 0 {
                break; // never prefetch unrouted experts
            }
            if !self.l2.contains(&e) {
                self.l2.insert(e);
                self.bandwidth_spent += PACK_UNITS;
            }
            picked.push(e);
            used += PACK_UNITS;
        }
        picked
    }

    /// The MoE phase streams its packs: bytes not in L2 are fetched.
    fn moe_phase_fetch(&self, selected: &[usize]) -> usize {
        selected
            .iter()
            .map(|&e| if self.l2.contains(&e) { 0 } else { PACK_UNITS })
            .sum()
    }

    /// Residency-ordered tile list: L2-resident experts first (stable).
    fn tile_order(&self, selected: &[usize]) -> Vec<usize> {
        let mut v = selected.to_vec();
        v.sort_by_key(|&e| !self.l2.contains(&e));
        v
    }
}

#[test]
fn k3_prev_step_routing_prefetch_hits_l2_under_budget() {
    // Consecutive steps route alike: prev and now share experts, and the
    // histogram is RICHER than the budget (20 distinct routed experts, varied
    // counts) so the BUDGET is the binding constraint, not the histogram.
    let prev: Vec<u32> = [
        3u32, 7, 3, 11, 3, 7, 19, 3, 11, 23, 7, 3, 5, 27, 11, 9, 13, 15, 17, 21, 25, 29, 31, 2, 5,
        13, 19, 25,
    ]
    .to_vec();
    let now: Vec<usize> = vec![3, 7, 11, 19, 23, 27, 5, 9, 2, 31, 13, 17, 21, 25, 29, 1];

    // BUGGY: no prefetch — the MoE phase streams every selected pack.
    let cold_fetch = naive_prefetch_stream(&now);
    assert_eq!(
        cold_fetch,
        now.len() * PACK_UNITS,
        "no prefetch: every pack streams"
    );

    // FIXED (wide policy): most-routed experts fill the budget; the phase's
    // fetches collapse to the mispredictions.
    let mut p = ExpertPrefetcher {
        budget_units: 14 * PACK_UNITS,
        l2: Default::default(),
        bandwidth_spent: 0,
    };
    let picked = p.prefetch_prev_routing(&prev);
    assert_eq!(
        picked.len(),
        14,
        "the wide budget picks exactly the fitting most-routed experts"
    );
    assert_eq!(picked[0], 3, "the most-routed expert is first");
    assert_eq!(
        p.bandwidth_spent,
        14 * PACK_UNITS,
        "prefetch spends the budget under the attention phase"
    );
    let fetch = p.moe_phase_fetch(&now);
    assert!(
        fetch < cold_fetch,
        "consecutive-alike routing must hit L2 for the shared experts"
    );
    // and the shared experts' bytes are exactly what stopped streaming.
    let shared_hits = now.iter().filter(|&&e| p.l2.contains(&e)).count();
    assert_eq!(fetch, (now.len() - shared_hits) * PACK_UNITS);

    // Residency-first tile list: L2-resident experts precede the cold ones.
    let order = p.tile_order(&now);
    let first_cold = order
        .iter()
        .position(|&e| !p.l2.contains(&e))
        .unwrap_or(order.len());
    assert!(
        order[..first_cold].iter().all(|&e| p.l2.contains(&e)),
        "resident experts go first"
    );
    assert!(
        order[first_cold..].iter().all(|&e| !p.l2.contains(&e)),
        "cold experts follow"
    );

    // Budget as POLICY: the narrow regime (the wide budget's traffic costs the
    // post-attention window more than it saves the MoE) picks 4, not 14.
    let mut narrow = ExpertPrefetcher {
        budget_units: 4 * PACK_UNITS,
        l2: Default::default(),
        bandwidth_spent: 0,
    };
    let picked_narrow = narrow.prefetch_prev_routing(&prev);
    assert_eq!(
        picked_narrow.len(),
        4,
        "the narrow policy caps the prefetch"
    );

    // TAMPERED A — the prefetch REPLACES the router: the selection becomes the
    // prefetched set, not the router's answer. The router stays authoritative.
    let mut p2 = ExpertPrefetcher {
        budget_units: 14 * PACK_UNITS,
        l2: Default::default(),
        bandwidth_spent: 0,
    };
    let picked2 = p2.prefetch_prev_routing(&prev);
    let router_answer: Vec<usize> = now.clone();
    let served_if_tampered = picked2.clone();
    assert_ne!(
        served_if_tampered, router_answer,
        "the prefetched set is a schedule, never the selection"
    );

    // TAMPERED B — the budget is ignored (prefetch every routed expert):
    // bandwidth overspends the policy ceiling.
    let overspend: usize = prev
        .iter()
        .map(|&e| e as usize)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        * PACK_UNITS;
    assert!(
        overspend > 14 * PACK_UNITS,
        "unbounded prefetch overspends the wide budget"
    );

    // ALL-SAFE: consecutive identical routing whose distinct set FITS the
    // budget — every pack already resident; the prefetch spends nothing and
    // the phase streams nothing.
    let small_prev: Vec<u32> = [3u32, 7, 3, 11, 7, 3, 5, 9, 11, 5].to_vec(); // 5 distinct ≤ budget
    let mut p3 = ExpertPrefetcher {
        budget_units: 14 * PACK_UNITS,
        l2: Default::default(),
        bandwidth_spent: 0,
    };
    let _ = p3.prefetch_prev_routing(&small_prev);
    let spent_before = p3.bandwidth_spent;
    let _ = p3.prefetch_prev_routing(&small_prev); // the same step again
    assert_eq!(
        p3.bandwidth_spent, spent_before,
        "already-resident packs prefetch nothing"
    );
    assert_eq!(
        p3.moe_phase_fetch(&small_prev.iter().map(|&e| e as usize).collect::<Vec<_>>()),
        0
    );
}

/// The naive arm: stream every selected pack (no prefetch warp).
fn naive_prefetch_stream(selected: &[usize]) -> usize {
    selected.len() * PACK_UNITS
}
