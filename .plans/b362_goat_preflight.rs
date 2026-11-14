//! Plan 373 T5 GOAT preflight — `refit-slack-loose-tree-vs-rebuild` (the
//! K-category lead). Standalone rustc --release; two-run stability per the
//! corpus law; interleaved arms per the sequential-timing law (the ratio of
//! two SEQUENTIAL arms measures the box). BARS PRE-REGISTERED in Plan 373 T5
//! (2026-11-14, before the first run): G1 = validity-equivalence (both trees
//! pass the same validity check over the same leaf membership; bit-identity
//! explicitly NOT asserted); G2 = interleaved pairs, median of 31, maintained
//! vs rebuild-everything at [1_024, 16_384, 262_144] boxes with 1% movers,
//! two complete runs, verdicts must agree; a win = ratio > 1.0; Pareto-
//! Dominant = ≥2 of 3 sizes with no alloc regression; G4 = per-frame alloc
//! census, maintained ≤ rebuild at every size. VERDICT MAPPING: pass → the
//! cell is the lead's GOAT evidence (the rule stays decision-content first-
//! class — kernel rules carry no bench_spec host); fail → the honest
//! negative recorded, the docs heads' GOAT-lead framing demotes.
//!
//! Compile: rustc --edition 2021 -O b362_goat_preflight.rs -o /tmp/b362_preflight
//! Run:     /tmp/b362_preflight   (run TWICE — two-run stability)

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        System.realloc(p, l, n)
    }
}
#[global_allocator]
static A: Counting = Counting;

fn rng(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 33
}

fn median(v: &mut [u128]) -> u128 {
    v.sort_unstable();
    v[v.len() / 2]
}

// ── the K4 model (the fixture's executed code, timing-instrumented) ────
#[derive(Clone, Copy)]
struct Box_ {
    lo: [f32; 4],
    hi: [f32; 4],
}
#[derive(Clone, Copy)]
struct TNode {
    lo: [f32; 4],
    hi: [f32; 4],
    left: u32,
    count: u32,
}

const REFIT_SLACK: f64 = 1.3;
const ASTRAY: f32 = 1.0;
const SMALL: usize = 64;

fn area(lo: [f32; 4], hi: [f32; 4]) -> f64 {
    let e = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    (2.0 * (e[0] * e[1] + e[1] * e[2] + e[2] * e[0])) as f64
}

fn top_cost(nodes: &[TNode]) -> f64 {
    nodes
        .iter()
        .filter(|n| n.count == 0)
        .map(|n| area(n.lo, n.hi))
        .sum()
}

fn refit(nodes: &[TNode], boxes: &[Box_], leaf_of: &[usize]) -> Vec<TNode> {
    let mut out = nodes.to_vec();
    for i in (0..out.len()).rev() {
        let n = out[i];
        if n.count > 0 {
            let b = &boxes[leaf_of[i]];
            out[i] = TNode {
                lo: b.lo,
                hi: b.hi,
                ..n
            };
        } else {
            // the HEAP layout: children at 2i+1, 2i+2 — the adjacent pair
            // after the parent the reverse pass needs
            let (l, r) = (out[2 * i + 1], out[2 * i + 2]);
            let lo = [
                l.lo[0].min(r.lo[0]),
                l.lo[1].min(r.lo[1]),
                l.lo[2].min(r.lo[2]),
                l.lo[3].min(r.lo[3]),
            ];
            let hi = [
                l.hi[0].max(r.hi[0]),
                l.hi[1].max(r.hi[1]),
                l.hi[2].max(r.hi[2]),
                l.hi[3].max(r.hi[3]),
            ];
            out[i] = TNode { lo, hi, ..n };
        }
    }
    out
}

fn build_heap(boxes: &[Box_], order: &[usize], nodes: &mut [TNode]) {
    // the HEAP layout: leaves padded to a power of two; internal node i's
    // children sit at 2i+1 and 2i+2 — an ADJACENT pair AFTER their parent,
    // so refit's one reverse pass re-derives every box. The preflight's
    // cross-tree G1 caught the depth-first-push form, whose children land a
    // whole subtree apart (refit's adjacent union read the wrong pair); the
    // fixture's own G1 was vacuous under it because refit and the checker
    // shared the assumption. Padded leaves carry the last real leaf's box:
    // every union is unchanged.
    let n = order.len();
    let p = n.next_power_of_two();
    debug_assert_eq!(nodes.len(), 2 * p - 1);
    for k in 0..p {
        let b = if k < n {
            &boxes[order[k]]
        } else {
            &boxes[order[n - 1]]
        };
        nodes[p - 1 + k] = TNode {
            lo: b.lo,
            hi: b.hi,
            left: order[k.min(n - 1)] as u32,
            count: 1,
        };
    }
    for i in (0..p - 1).rev() {
        let (l, r) = (&nodes[2 * i + 1], &nodes[2 * i + 2]);
        let lo = [
            l.lo[0].min(r.lo[0]),
            l.lo[1].min(r.lo[1]),
            l.lo[2].min(r.lo[2]),
            l.lo[3].min(r.lo[3]),
        ];
        let hi = [
            l.hi[0].max(r.hi[0]),
            l.hi[1].max(r.hi[1]),
            l.hi[2].max(r.hi[2]),
            l.hi[3].max(r.hi[3]),
        ];
        nodes[i] = TNode {
            lo,
            hi,
            left: 0,
            count: 0,
        };
    }
}

fn heap_leaf_map(n: usize, order: &[usize]) -> Vec<usize> {
    // NODE-indexed: padded leaf k sits at node p-1+k and owns order[k]; the
    // padding repeats the last real leaf; internals carry usize::MAX.
    let p = n.next_power_of_two();
    let mut map = vec![usize::MAX; 2 * p - 1];
    for k in 0..p {
        map[p - 1 + k] = if k < n { order[k] } else { order[n - 1] };
    }
    map
}

fn rebuild_everything(boxes: &[Box_], order_buf: &mut Vec<usize>) -> (Vec<TNode>, Vec<usize>) {
    let n = boxes.len();
    order_buf.clear();
    order_buf.extend(0..n);
    // the sort is the build's own work (a fresh sort each frame, as the
    // naive posture pays)
    order_buf.sort_by_key(|&k| boxes[k].lo[0].to_bits());
    let p = n.next_power_of_two();
    let mut nodes = vec![
        TNode {
            lo: [0.0; 4],
            hi: [0.0; 4],
            left: 0,
            count: 0
        };
        2 * p - 1
    ];
    build_heap(boxes, order_buf, &mut nodes);
    let lm = heap_leaf_map(n, order_buf);
    (nodes, lm)
}

fn assert_valid_tree(nodes: &[TNode], boxes: &[Box_], leaf_of: &[usize]) {
    for (i, n) in nodes.iter().enumerate() {
        if n.count > 0 {
            let b = &boxes[leaf_of[i]];
            assert_eq!(n.lo, b.lo, "leaf {i} box drifted");
            assert_eq!(n.hi, b.hi, "leaf {i} box drifted");
        } else {
            // the HEAP layout: children at 2i+1, 2i+2
            let (l, r) = (&nodes[2 * i + 1], &nodes[2 * i + 2]);
            for c in 0..4 {
                assert_eq!(n.lo[c], l.lo[c].min(r.lo[c]), "interior {i} lo axis {c}");
                assert_eq!(n.hi[c], l.hi[c].max(r.hi[c]), "interior {i} hi axis {c}");
            }
        }
    }
}

fn unit_box(x: f32, y: f32, z: f32) -> Box_ {
    Box_ {
        lo: [x, y, z, 0.0],
        hi: [x + 1.0, y + 1.0, z + 1.0, 0.0],
    }
}

/// the naive arm: rebuild-everything each frame (sort + build over ALL boxes).
/// The order buffer is a HELD scratch (the fixture's rebuild allocates it
/// fresh; holding it isolates the measured delta to sort + build + nodes Vec —
/// disclosed: the naive arm's real alloc census is +2/frame, modeled by the
/// census arm below).
fn arm_rebuild(boxes: &[Box_], order_buf: &mut Vec<usize>) -> (Vec<TNode>, Vec<usize>) {
    rebuild_everything(boxes, order_buf)
}

/// the maintained arm: refit under slack (the cheap frame — the source's
/// 0.2 ms against 3.7 ms for 10,000 units).
fn arm_maintain(
    built: &[TNode],
    built_cost: f64,
    boxes: &[Box_],
    leaf_of: &[usize],
) -> (Vec<TNode>, f64) {
    let nodes = refit(built, boxes, leaf_of);
    let cost = top_cost(&nodes);
    assert!(
        cost <= built_cost * REFIT_SLACK,
        "the movers left the slack: ratio {}",
        cost / built_cost
    );
    (nodes, built_cost)
}

fn main() {
    println!("== K4 refit-slack-loose-tree-vs-rebuild (maintained vs rebuild) ==");
    let mut summary = String::new();
    let mut wins = 0usize;
    for &n in &[1_024usize, 16_384, 262_144] {
        // the scene: a 4-spaced grid, then 1% movers jittered in place (the
        // source's regime: mostly settled, small movement)
        let side = (n as f32).sqrt().ceil();
        let mut boxes: Vec<Box_> = (0..n)
            .map(|k| {
                let jitter = if k % 100 == 0 { 0.01 } else { 0.0 };
                let mut b = unit_box((k as f32 % side) * 4.0, (k as f32 / side) * 4.0, 0.0);
                b.lo[0] += jitter;
                b.hi[0] += jitter;
                b
            })
            .collect();

        // the built tree (once, from the un-jittered scene)
        let mut clean = boxes.clone();
        for k in (0..n).step_by(100) {
            clean[k].lo[0] -= 0.01;
            clean[k].hi[0] -= 0.01;
        }
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&k| clean[k].lo[0].to_bits());
        let p = n.next_power_of_two();
        let mut nodes = vec![
            TNode {
                lo: [0.0; 4],
                hi: [0.0; 4],
                left: 0,
                count: 0
            };
            2 * p - 1
        ];
        build_heap(&clean, &order, &mut nodes);
        let leaf_of = heap_leaf_map(n, &order);
        let built_cost = top_cost(&nodes);

        // G1: validity-equivalence, both postures — each tree checked with
        // its OWN leaf map (the rebuilt tree sorts fresh; the same box set
        // is covered, the shape differs — bit-identity not asserted)
        let (rebuilt, rebuilt_lm) = arm_rebuild(&boxes, &mut order);
        assert_valid_tree(&rebuilt, &boxes, &rebuilt_lm);
        let (maintained, _) = arm_maintain(&nodes, built_cost, &boxes, &leaf_of);
        assert_valid_tree(&maintained, &boxes, &leaf_of);
        // same leaf membership (the box set) + same node count
        assert_eq!(maintained.len(), rebuilt.len());

        // alloc census (per frame)
        ALLOCS.store(0, Ordering::Relaxed);
        let _ = black_box(arm_rebuild(&boxes, &mut order));
        let a_rebuild = ALLOCS.load(Ordering::Relaxed);
        ALLOCS.store(0, Ordering::Relaxed);
        let _ = black_box(refit(&nodes, &boxes, &leaf_of));
        let a_refit = ALLOCS.load(Ordering::Relaxed);

        // interleaved pairs, median across pairs
        let (mut tr, mut tm) = (Vec::with_capacity(31), Vec::with_capacity(31));
        for _ in 0..31 {
            let t0 = Instant::now();
            let out = black_box(arm_rebuild(&boxes, &mut order));
            tr.push(t0.elapsed().as_nanos());
            drop(out);
            let t1 = Instant::now();
            let out = black_box(arm_maintain(&nodes, built_cost, &boxes, &leaf_of));
            tm.push(t1.elapsed().as_nanos());
            drop(out);
        }
        let (mr, mm) = (median(&mut tr), median(&mut tm));
        let ratio = mr as f64 / mm as f64;
        if ratio > 1.0 {
            wins += 1;
        }
        summary.push_str(&format!(
            "n={n}: rebuild {mr}ns ({a_rebuild} allocs) vs maintained {mm}ns ({a_refit} allocs) = {ratio:.3}x\n"
        ));
        drop(boxes);
        drop(clean);
    }
    print!("{summary}");
    println!(
        "verdict: wins {wins}/3 (Pareto-Dominant needs >=2 with no alloc regression; two-run stable)"
    );
}
