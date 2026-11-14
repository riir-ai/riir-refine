//! Batch 362 fixtures (Plan 373 / Research 406) — ORIGINAL std-only
//! reproductions of the ThreeRT laws, never pasted source. One standalone
//! file per the B336/B338/B339/B341/B357 discipline: compiled AND executed
//! BEFORE entry as `rustc --edition 2021 --test b362_fixtures.rs &&
//! ./b362_fixtures`. Each rule's ref/opt arms in `entries_gpu_157.rs`
//! (kernel_opt) embed the same executed code — the fixture pass is the
//! authoring gate: a defect caught here is fixed HERE first, then synced
//! back into the entries in the same landing commit (the b361 precedent:
//! its held gate caught two fixture bugs pre-landing).
//!
//! The wgpu/ray-query/WESL surface is modeled as a mock layer; the fixture
//! never calls a real GPU.
//!
//! Arms (rule → what the tests pin):
//! - F1 `hw-rayquery-candidate-loop-loses-hits-metal`: the vendor candidate
//!   loop's seeded run-to-run surface flip vs the ordered restart walk; the
//!   per-mode step budgets (closest 16 degrades to a miss, shadow 64 reaches
//!   the ground, shadow exhaustion is a TYPED blocked answer); the
//!   distance-scaled epsilon vs a fixed epsilon at t ~ 1e6; the strict-
//!   variant ray screen (the R7 face).
//! - F2 `sah-tree-depth-bounded-by-traversal-stack`: an unbounded build
//!   breaks a fixed-stack walker (drops = missed geometry); the in-builder
//!   fit gate keeps a 10k-leaf build inside a 45-deep budget the walker
//!   walks whole; the discard-and-rebuild median fallback.
//! - F3 `walk-terminating-fill-for-unused-buffer-capacity`: fresh capacity
//!   and released room end a walk in one read; zero and stale fills poison
//!   it (the 0/1 cycle); the read-only-consumer anti-regime.
//! - F4 `refit-slack-loose-tree-vs-rebuild`: the refit output is a VALID
//!   tree over the same leaves (validity-equivalence, never bit-identity)
//!   with the shape untouched; the hysteresis (astray → Loose, small scene
//!   → RebuildAll). The timing cell lives in `.plans/b362_goat_preflight.rs`.
//! - F5 `edge-avoiding-atrous-denoise-variance-guided`: the per-pixel
//!   variance leaves a dark hole the 3×3-filtered width fills; the positive
//!   width floor admits at zero variance; demodulate/re-modulate preserves
//!   the guide texture.
//! - F6 `wesl-variant-flag-set-total-fail-closed`: an unnamed flag is a
//!   compile error naming it (the silent-off naive arm compiles the region
//!   out); all-provided compiles; missing constants refuse; variant keys
//!   compose over the total set.
//! - F7 `qmc-fixed-dims-hash-for-unbounded-draws`: the data-dependent loop
//!   consumes ZERO Sobol dimensions (the naive arm burns one a step); live
//!   spends Sobol only at the camera vertex; dimension ids compose; the
//!   unrolled direction numbers are bit-identical to the table form.
//! - F8 `scene-function-compile-failure-degrade-loud`: the core fails
//!   closed, the user functions degrade LOUD with a typed log line; the
//!   legacy shim is byte-scoped and tripwired; the naive path takes the app
//!   down on user staleness.

use std::collections::BTreeSet;

// ─────────────────────────────────────────────────────────────────────────────
// F1 — hw-rayquery-candidate-loop-loses-hits-metal
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
struct Hit {
    t: f32,
    tri: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Shadow,
    Closest,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Answer {
    Hit {
        t: f32,
        tri: u32,
    },
    Miss,
    /// shadow mode only: budget exhausted with cutouts still ahead — the ray
    /// is BLOCKED (light does not leak through dense foliage)
    BlockedAtBudget,
}

const STEPS_SHADOW: u32 = 64;
const STEPS_CLOSEST: u32 = 16;

/// the landed remedy: every query FORCE_OPAQUE; the cutouts are handled by
/// an ordered restart walk — each query starts past the last cutout its
/// alpha passed, the epsilon DISTANCE-SCALED, the step budget per mode and
/// exhaustion TYPED.
fn trace_ordered_restart(
    mut o: [f32; 3],
    d: [f32; 3],
    tmax: f32,
    mode: Mode,
    opaque_hit: impl Fn([f32; 3], [f32; 3], f32) -> Option<Hit>,
    alpha_pass: impl Fn(u32) -> bool,
) -> Answer {
    let steps = match mode {
        Mode::Shadow => STEPS_SHADOW,
        Mode::Closest => STEPS_CLOSEST,
    };
    let mut t0 = 0.0f32;
    for _ in 0..steps {
        let Some(h) = opaque_hit(o, d, tmax - t0) else {
            return Answer::Miss; // a clear line past the last cutout: unblocked
        };
        let t = t0 + h.t;
        if !alpha_pass(h.tri) {
            return Answer::Hit { t, tri: h.tri }; // opaque, or a cutout its alpha blocked
        }
        // a cutout its alpha let through: restart past it, epsilon distance-scaled
        let eps = 1e-4 * t.max(1.0);
        t0 = t + eps;
        o = [
            o[0] + d[0] * (h.t + eps),
            o[1] + d[1] * (h.t + eps),
            o[2] + d[2] * (h.t + eps),
        ];
    }
    match mode {
        Mode::Shadow => Answer::BlockedAtBudget,
        Mode::Closest => Answer::Miss,
    }
}

/// the fixed-epsilon variant — the defect the distance-scaled form cures:
/// at t ~ 1e6 an f32 cannot hold +1e-4 (ulp ~0.06), so the restart origin
/// lands ON the hit and the same cutout re-hits, burning the budget.
fn trace_fixed_epsilon(
    mut o: [f32; 3],
    d: [f32; 3],
    tmax: f32,
    mode: Mode,
    opaque_hit: impl Fn([f32; 3], [f32; 3], f32) -> Option<Hit>,
    alpha_pass: impl Fn(u32) -> bool,
) -> Answer {
    let steps = match mode {
        Mode::Shadow => STEPS_SHADOW,
        Mode::Closest => STEPS_CLOSEST,
    };
    let mut t0 = 0.0f32;
    for _ in 0..steps {
        let Some(h) = opaque_hit(o, d, tmax - t0) else {
            return Answer::Miss;
        };
        let t = t0 + h.t;
        if !alpha_pass(h.tri) {
            return Answer::Hit { t, tri: h.tri };
        }
        t0 = t + 1e-4; // FIXED epsilon
        o = [
            o[0] + d[0] * (h.t + 1e-4),
            o[1] + d[1] * (h.t + 1e-4),
            o[2] + d[2] * (h.t + 1e-4),
        ];
    }
    match mode {
        Mode::Shadow => Answer::BlockedAtBudget,
        Mode::Closest => Answer::Miss,
    }
}

/// a cutout/ground slab on the +x ray, sorted by `lo`. Slabs have extent,
/// so a restart landing INSIDE one re-hits it at the minimum distance —
/// the double-hit hazard the epsilon law governs.
#[derive(Clone, Copy)]
struct Tri {
    lo: f32,
    hi: f32,
    tri: u32,
    passes: bool, // the cutout's alpha test lets the ray through
}

fn line_query<'a>(tris: &'a [Tri]) -> impl Fn([f32; 3], [f32; 3], f32) -> Option<Hit> + 'a {
    move |o, _d, rem| {
        let x = o[0];
        tris.iter()
            .find(|t| t.hi > x && t.lo < x + rem)
            .map(|t| Hit {
                t: if t.lo > x { t.lo - x } else { 1e-6 },
                tri: t.tri,
            })
    }
}

#[derive(Clone, Copy)]
struct Cand {
    t: f32,
    tri: u32,
    passes: bool,
}

/// the vendor candidate loop, mocked: candidates arrive in a seeded order
/// and one is DROPPED per seed — the run-to-run loss, "confirmed ones too".
fn trace_candidate_loop(cands: &[Cand], seed: u64) -> Option<u32> {
    let skip = (seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
        >> 33)
        % cands.len() as u64;
    let mut best: Option<(f32, u32)> = None;
    for (k, c) in cands.iter().enumerate() {
        if k as u64 == skip {
            continue; // the lost hit
        }
        if !c.passes {
            best = Some(match best {
                Some(b) if b.0 <= c.t => b,
                _ => (c.t, c.tri),
            });
        }
    }
    best.map(|b| b.1)
}

/// the strict-variant face (R7): the driver is given nothing undefined —
/// a non-finite ray or a negative length is screened BEFORE the query.
fn ray_bad(o: [f32; 3], d: [f32; 3], tmax: f32) -> bool {
    let fin = |v: [f32; 3]| v.iter().all(|c| c.is_finite());
    !fin(o) || !fin(d) || tmax < 0.0 || !tmax.is_finite()
}

#[test]
fn f1_vendor_candidate_answers_flip_across_seeds_walk_does_not() {
    // cutout at 1.0 (its alpha passes), opaque ground at 2.0. The walk is
    // seed-free and always reports the ground; the vendor candidate loop
    // drops a seeded candidate, so the SAME ray flips surfaces run to run
    // ("missed even the opaque ground behind it").
    let tris = [
        Tri {
            lo: 1.0,
            hi: 1.001,
            tri: 100,
            passes: true,
        },
        Tri {
            lo: 2.0,
            hi: 2.5,
            tri: 7,
            passes: false,
        },
    ];
    let query = line_query(&tris);
    let d = [1.0f32, 0.0, 0.0];
    let walk = trace_ordered_restart([0.0; 3], d, 10.0, Mode::Closest, query, |tri| tri != 7);
    assert!(
        matches!(walk, Answer::Hit { tri: 7, .. }),
        "the walk reports the ground: {walk:?}"
    );

    let cands = [
        Cand {
            t: 1.0,
            tri: 100,
            passes: true,
        },
        Cand {
            t: 2.0,
            tri: 7,
            passes: false,
        },
    ];
    let mut answers = BTreeSet::new();
    for seed in 0..64u64 {
        answers.insert(trace_candidate_loop(&cands, seed));
    }
    assert!(
        answers.len() >= 2,
        "the vendor path flipped surfaces across seeds: {answers:?}"
    );
}

#[test]
fn f1_per_mode_budgets_closest_degrades_shadow_reaches() {
    // twenty thin cutouts (leaf-like, 2e-4 thick) then the opaque ground:
    // the CLOSEST budget (16) burns out mid-foliage and the pixel degrades
    // to a miss; the SHADOW budget (64) walks past all twenty and reaches
    // the ground.
    let mut tris: Vec<Tri> = (1..=20)
        .map(|k| Tri {
            lo: k as f32,
            hi: k as f32 + 2e-4,
            tri: 100 + k as u32,
            passes: true,
        })
        .collect();
    tris.push(Tri {
        lo: 21.5,
        hi: 23.0,
        tri: 7,
        passes: false,
    });
    let d = [1.0f32, 0.0, 0.0];
    let alpha = |tri: u32| tri != 7;

    let closest = trace_ordered_restart([0.0; 3], d, 40.0, Mode::Closest, line_query(&tris), alpha);
    assert_eq!(
        closest,
        Answer::Miss,
        "an exhausted CLOSEST walk degrades the pixel"
    );

    let shadow = trace_ordered_restart([0.0; 3], d, 40.0, Mode::Shadow, line_query(&tris), alpha);
    assert!(
        matches!(shadow, Answer::Hit { tri: 7, .. }),
        "shadow reaches the ground: {shadow:?}"
    );
    if let Answer::Hit { t, .. } = shadow {
        assert!(
            (t - 21.5).abs() < 0.05,
            "absolute hit distance tracked: {t}"
        );
    }
}

#[test]
fn f1_shadow_budget_exhaustion_is_a_typed_blocked_answer() {
    // seventy cutouts: the shadow budget exhausts with cutouts still ahead —
    // the answer is TYPED (blocked-at-budget), never a silent "unblocked":
    // light does not leak through dense foliage.
    let mut tris: Vec<Tri> = (1..=70)
        .map(|k| Tri {
            lo: k as f32,
            hi: k as f32 + 2e-4,
            tri: 100 + k as u32,
            passes: true,
        })
        .collect();
    tris.push(Tri {
        lo: 71.5,
        hi: 73.0,
        tri: 7,
        passes: false,
    });
    let shadow = trace_ordered_restart(
        [0.0; 3],
        [1.0, 0.0, 0.0],
        90.0,
        Mode::Shadow,
        line_query(&tris),
        |tri| tri != 7,
    );
    assert_eq!(shadow, Answer::BlockedAtBudget);
}

#[test]
fn f1_fixed_epsilon_double_hits_at_scale_scaled_epsilon_restarts() {
    // thick slabs at t ~ 1e6: the fixed epsilon (1e-4) is under the f32 ulp
    // there, so the restart lands ON the hit and slab 0 re-hits until the
    // budget burns (the pixel degrades to a miss); the distance-scaled
    // epsilon (1e-4 * max(1, t) = 100) actually advances and reaches the
    // ground behind.
    let mut tris: Vec<Tri> = (0..500)
        .map(|k| Tri {
            lo: 1e6 + (2 * k) as f32,
            hi: 1e6 + (2 * k + 1) as f32,
            tri: 1000 + k as u32,
            passes: true,
        })
        .collect();
    tris.push(Tri {
        lo: 1e6 + 1000.0,
        hi: 1e6 + 1002.0,
        tri: 7,
        passes: false,
    });
    let d = [1.0f32, 0.0, 0.0];
    let alpha = |tri: u32| tri != 7;

    let fixed = trace_fixed_epsilon(
        [0.0; 3],
        d,
        1e6 + 2000.0,
        Mode::Closest,
        line_query(&tris),
        alpha,
    );
    assert_eq!(
        fixed,
        Answer::Miss,
        "the fixed epsilon burned its 16 steps inside slab 0"
    );

    let scaled = trace_ordered_restart(
        [0.0; 3],
        d,
        1e6 + 2000.0,
        Mode::Closest,
        line_query(&tris),
        alpha,
    );
    assert!(
        matches!(scaled, Answer::Hit { tri: 7, .. }),
        "the scaled epsilon reaches the ground: {scaled:?}"
    );
}

#[test]
fn f1_ray_screen_rejects_undefined_rays_under_the_strict_variant() {
    let nan = f32::NAN;
    assert!(ray_bad([1.0, 1.0, 1.0], [nan, 0.0, 0.0], 10.0));
    assert!(ray_bad([nan, 0.0, 0.0], [1.0, 0.0, 0.0], 10.0));
    assert!(ray_bad([1.0, 0.0, 0.0], [0.0, f32::INFINITY, 0.0], 10.0));
    assert!(ray_bad([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], -1.0));
    assert!(!ray_bad([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 10.0));
}

// ─────────────────────────────────────────────────────────────────────────────
// F2 — sah-tree-depth-bounded-by-traversal-stack
// ─────────────────────────────────────────────────────────────────────────────

const WALKER_STACK: usize = 64;
const TOP_LEVEL_WORST: usize = 19;
const MESH_BUDGET: usize = WALKER_STACK - TOP_LEVEL_WORST; // 45

#[derive(Clone, Copy)]
struct BNode {
    left: u32,
    count: u32, // count == 0: interior, children at left, left+1
}

/// ceil(log2(n)) — the closed form the median builder's depth follows.
fn levels(n: usize) -> usize {
    n.max(1).next_power_of_two().trailing_zeros() as usize
}

/// the unbounded heuristic, mocked with a pathological centroid layout: the
/// cheapest bin always peels ONE item — the source's measured 42-vs-21
/// imbalance taken to its logic end over this mock distribution.
fn build_unbalanced(boxes: &[(u32, u32)]) -> Vec<BNode> {
    let mut nodes = vec![BNode { left: 0, count: 0 }];
    let mut stack = vec![(0usize, 0usize, boxes.len())];
    while let Some((ni, start, end)) = stack.pop() {
        let count = end - start;
        if count <= 1 {
            nodes[ni] = BNode {
                left: start as u32,
                count: 1,
            };
            continue;
        }
        let mid = start + 1; // the pathological pick: one item peels off
        let left = nodes.len();
        nodes.push(BNode { left: 0, count: 0 });
        nodes.push(BNode { left: 0, count: 0 });
        nodes[ni] = BNode {
            left: left as u32,
            count: 0,
        };
        stack.push((left + 1, mid, end));
        stack.push((left, start, mid));
    }
    nodes
}

/// one builder pass with the in-builder fit gate: a candidate split is
/// admitted only while depth + 1 + levels(max side) still fits the budget;
/// past that the MEDIAN split (which halves) is taken instead.
fn build_bounded(boxes: &[(u32, u32)], max_depth: usize) -> Vec<BNode> {
    let mut nodes = vec![BNode { left: 0, count: 0 }];
    let mut stack = vec![(0usize, 0usize, boxes.len(), 0usize)];
    while let Some((ni, start, end, depth)) = stack.pop() {
        let count = end - start;
        if count <= 1 {
            nodes[ni] = BNode {
                left: start as u32,
                count: 1,
            };
            continue;
        }
        // the heuristic pick (the same pathological peel). The gate is
        // STRICT: depth+1+levels bounds the child subtree's remaining
        // LEVELS, and tree_depth counts NODES (a leaf at counter d sits at
        // node-depth d+1) — the off-by-one class the fixture gate caught.
        let (mut mid, mut sides) = (start + 1, (1usize, count - 1));
        let mut fits = depth + 1 + levels(sides.1) < max_depth;
        if !fits {
            // the median split — it halves, so its depth is closed-form
            mid = start + count / 2;
            sides = (count / 2, count - count / 2);
            fits = depth + 1 + levels(sides.0.max(sides.1)) < max_depth;
            assert!(
                fits,
                "the median split itself cannot fit: the budget is under levels(n)"
            );
        }
        let left = nodes.len();
        nodes.push(BNode { left: 0, count: 0 });
        nodes.push(BNode { left: 0, count: 0 });
        nodes[ni] = BNode {
            left: left as u32,
            count: 0,
        };
        stack.push((left + 1, mid, end, depth + 1));
        stack.push((left, start, mid, depth + 1));
    }
    nodes
}

/// the post-build defense: a heuristic tree deeper than the budget is
/// DISCARDED and rebuilt by median splits.
fn mesh_tree(boxes: &[(u32, u32)]) -> Vec<BNode> {
    let t = build_unbalanced(boxes);
    if tree_depth(&t, 0) <= MESH_BUDGET {
        return t;
    }
    build_bounded(boxes, MESH_BUDGET)
}

fn tree_depth(nodes: &[BNode], root: usize) -> usize {
    let mut max_d = 1usize;
    let mut stack = vec![(root, 1usize)];
    while let Some((n, d)) = stack.pop() {
        max_d = max_d.max(d);
        let nd = nodes[n];
        if nd.count == 0 {
            stack.push((nd.left as usize, d + 1));
            stack.push((nd.left as usize + 1, d + 1));
        }
    }
    max_d
}

/// the fixed-stack walker mock: a push past its capacity is DROPPED —
/// geometry missed, never a hang. Returns (leaves visited, pushes dropped).
fn walk_with_fixed_stack(nodes: &[BNode], root: usize, cap: usize) -> (usize, usize) {
    let (mut visited, mut dropped) = (0usize, 0usize);
    let mut stack: Vec<usize> = Vec::new();
    stack.push(root);
    while let Some(n) = stack.pop() {
        let nd = nodes[n];
        if nd.count != 0 {
            visited += 1;
            continue;
        }
        for child in [nd.left as usize, nd.left as usize + 1] {
            if stack.len() >= cap {
                dropped += 1; // the push past it is dropped
                continue;
            }
            stack.push(child);
        }
    }
    (visited, dropped)
}

fn mock_boxes(n: usize) -> Vec<(u32, u32)> {
    (0..n).map(|k| (k as u32, k as u32 + 1)).collect()
}

#[test]
fn f2_unbounded_build_breaks_the_walker() {
    let boxes = mock_boxes(10_000);
    let nodes = build_unbalanced(&boxes);
    let depth = tree_depth(&nodes, 0);
    assert!(
        depth > MESH_BUDGET,
        "the unbounded build shipped {depth} deep against a {MESH_BUDGET} budget"
    );
    let (visited, dropped) = walk_with_fixed_stack(&nodes, 0, WALKER_STACK);
    assert!(dropped > 0, "the fixed-stack walker dropped pushes");
    assert!(
        visited < boxes.len(),
        "geometry missed: {visited} of {} leaves",
        boxes.len()
    );
}

#[test]
fn f2_bounded_build_walks_whole_under_the_stack() {
    let boxes = mock_boxes(10_000);
    let nodes = build_bounded(&boxes, MESH_BUDGET);
    let depth = tree_depth(&nodes, 0);
    assert!(
        depth <= MESH_BUDGET,
        "the fit gate shipped {depth} deep against a {MESH_BUDGET} budget"
    );
    let (visited, dropped) = walk_with_fixed_stack(&nodes, 0, WALKER_STACK);
    assert_eq!(dropped, 0, "no push past the stack");
    assert_eq!(visited, boxes.len(), "the walk covers every leaf");
}

#[test]
fn f2_mesh_tree_discards_a_too_deep_heuristic_and_rebuilds() {
    let boxes = mock_boxes(10_000);
    let tree = mesh_tree(&boxes);
    assert!(tree_depth(&tree, 0) <= MESH_BUDGET);
    let (visited, dropped) = walk_with_fixed_stack(&tree, 0, WALKER_STACK);
    assert_eq!((visited, dropped), (boxes.len(), 0));
}

// ─────────────────────────────────────────────────────────────────────────────
// F3 — walk-terminating-fill-for-unused-buffer-capacity
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
struct GpuNode {
    left_or_first: u32,
    count: u32, // count == 0: interior
}

/// the encoding has no empty leaf, so the terminal entry is a leaf of one
/// degenerate item: one node read, no child, no box. ZEROS decode as an
/// interior node whose children are nodes 0 and 1.
const FILL: GpuNode = GpuNode {
    left_or_first: 0,
    count: 1,
};

struct NodeBuffer {
    nodes: Vec<GpuNode>,
    live_len: usize,
}

impl NodeBuffer {
    fn with_capacity(cap: usize) -> Self {
        Self {
            nodes: vec![FILL; cap],
            live_len: 0,
        } // fresh capacity is born terminal
    }

    fn release_fill(&mut self, root: usize, len: usize) {
        for slot in &mut self.nodes[root..root + len] {
            *slot = FILL;
        }
    }

    fn release_zero(&mut self, root: usize, len: usize) {
        for slot in &mut self.nodes[root..root + len] {
            *slot = GpuNode {
                left_or_first: 0,
                count: 0,
            }; // the natural fill: zeros
        }
    }

    fn release_stale(&mut self, root: usize, prev: &[GpuNode]) {
        self.nodes[root..root + prev.len()].copy_from_slice(prev);
    }
}

#[derive(Debug, PartialEq)]
enum WalkStop {
    Ended(u32, usize),
    DidNotEnd,
    OutOfBounds,
}

fn walk_bounded(b: &NodeBuffer, mut node: usize, trip_limit: usize) -> WalkStop {
    let mut trips = 0usize;
    loop {
        trips += 1;
        if trips > trip_limit {
            return WalkStop::DidNotEnd;
        }
        if node >= b.nodes.len() {
            return WalkStop::OutOfBounds;
        }
        let n = b.nodes[node];
        if n.count != 0 {
            return WalkStop::Ended(n.left_or_first, trips); // a leaf — including the tail's FILL
        }
        node = n.left_or_first as usize; // interior: descend
    }
}

#[test]
fn f3_fresh_capacity_and_released_room_end_the_walk_at_once() {
    let mut b = NodeBuffer::with_capacity(64);
    b.live_len = 4;
    // a real small tree at 0..: root interior -> two leaves
    b.nodes[0] = GpuNode {
        left_or_first: 2,
        count: 0,
    };
    b.nodes[2] = GpuNode {
        left_or_first: 7,
        count: 1,
    };
    b.nodes[3] = GpuNode {
        left_or_first: 9,
        count: 1,
    };
    assert_eq!(walk_bounded(&b, 0, 32), WalkStop::Ended(7, 2));
    // release the subtree: the walk that lands there reads one node and stops
    b.release_fill(2, 2);
    assert_eq!(walk_bounded(&b, 2, 32), WalkStop::Ended(0, 1));
    // fresh capacity beyond live_len is born terminal too
    assert_eq!(walk_bounded(&b, 63, 32), WalkStop::Ended(0, 1));
}

#[test]
fn f3_zero_and_stale_fills_poison_the_walk() {
    let mut b = NodeBuffer::with_capacity(64);
    b.nodes[0] = GpuNode {
        left_or_first: 2,
        count: 0,
    }; // the real root points at the freed range
    // the natural fill: zeros decode as an interior node whose children are
    // nodes 0 and 1 — the walk cycles and never ends
    b.release_zero(2, 2);
    assert_eq!(walk_bounded(&b, 2, 32), WalkStop::DidNotEnd);
    // the previous occupant's bytes decode as arbitrary interior structure
    let prev = [
        GpuNode {
            left_or_first: 3,
            count: 0,
        },
        GpuNode {
            left_or_first: 2,
            count: 0,
        },
    ];
    b.release_stale(2, &prev);
    assert_eq!(walk_bounded(&b, 2, 32), WalkStop::DidNotEnd);
}

#[test]
fn f3_read_only_consumers_are_the_anti_regime() {
    // a consumer that only READS fields (never decodes-and-continues) is
    // safe on zeros — the natural fill is correct THERE; the law targets
    // walk consumers, whose zero vector decodes as a valid non-terminal state.
    let zeroed = GpuNode {
        left_or_first: 0,
        count: 0,
    };
    assert_eq!((zeroed.left_or_first, zeroed.count), (0, 0));
    let mut b = NodeBuffer::with_capacity(8);
    b.nodes[5] = zeroed;
    assert_eq!(
        b.nodes[5].count, 0,
        "a read of a zeroed tail's fields is harmless"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F4 — refit-slack-loose-tree-vs-rebuild
// ─────────────────────────────────────────────────────────────────────────────

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

/// the ray-cost proxy: the sum of the INTERIOR nodes' areas (a line through
/// space meets a box in proportion to its area). Not divided by the root's:
/// a unit that flies off makes the root large, and the tree no better.
fn top_cost(nodes: &[TNode]) -> f64 {
    nodes
        .iter()
        .filter(|n| n.count == 0)
        .map(|n| area(n.lo, n.hi))
        .sum()
}

/// shape-preserving box update: children sit AFTER their parents, so one
/// reverse pass re-derives every box bottom-up. Same tree shape, same leaf
/// membership — a VALID tree of the new boxes, never bit-identical to a
/// rebuild.
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
            // the HEAP layout: children at 2i+1, 2i+2
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

/// the proxy's blind spot, closed: a unit whose box left the box it was
/// placed with, widened by that box's longest side, is not settled anymore.
fn astray(placed: &Box_, now: &Box_) -> bool {
    let mut side = 0.0f32;
    for c in 0..3 {
        side = side.max(now.hi[c] - now.lo[c]);
    }
    side *= ASTRAY;
    (0..3).any(|c| now.lo[c] < placed.lo[c] - side || now.hi[c] > placed.hi[c] + side)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MaintAction {
    Refit,
    Loose,
    RebuildAll,
}

/// the maintained posture: refit under slack; loose movers rebuild their OWN
/// tree; rebuild-all only past slack / a small scene.
fn maintain(
    built: &[TNode],
    built_cost: f64,
    boxes: &[Box_],
    loose: &[usize],
    leaf_of: &[usize],
) -> (Vec<TNode>, f64, MaintAction) {
    if boxes.len() <= SMALL {
        let nodes = rebuild_everything_boxes(boxes);
        let cost = top_cost(&nodes);
        return (nodes, cost, MaintAction::RebuildAll);
    }
    let nodes = refit(built, boxes, leaf_of);
    let cost = top_cost(&nodes);
    let (refit_cost, built_snap) = (cost, built_cost);
    if loose.is_empty() && cost <= built_cost * REFIT_SLACK {
        return (nodes, built_cost, MaintAction::Refit); // the cheap frame
    }
    let mut nodes = nodes;
    if !loose.is_empty() {
        let (loose_tree, lc) = rebuild_just(boxes, loose);
        nodes.extend(loose_tree); // the loose tree is rebuilt by ITS count
        return (
            nodes,
            cost.min(built_cost * REFIT_SLACK) + lc,
            MaintAction::Loose,
        );
    }
    let all = rebuild_everything_boxes(boxes);
    let cost = top_cost(&all);
    eprintln!(
        "DBG maintain n={} loose={} refit_cost={refit_cost} built={built_snap} ratio={}",
        boxes.len(),
        loose.len(),
        refit_cost / built_snap
    );
    (all, cost, MaintAction::RebuildAll)
}

fn rebuild_everything_boxes(boxes: &[Box_]) -> Vec<TNode> {
    let n = boxes.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&k| boxes[k].lo[0].to_bits());
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
    build_heap(boxes, &order, &mut nodes);
    nodes
}

fn rebuild_just(boxes: &[Box_], which: &[usize]) -> (Vec<TNode>, f64) {
    // the loose tree is rebuilt by ITS count; the work unit is the number
    // of nodes the build touches (2 * count - 1 in the heap layout).
    let n = which.len();
    let mut order = which.to_vec();
    order.sort_by_key(|&k| boxes[k].lo[0].to_bits());
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
    build_heap(boxes, &order, &mut nodes);
    (nodes, (2 * p - 1) as f64)
}

fn build_heap(boxes: &[Box_], order: &[usize], nodes: &mut [TNode]) {
    // the HEAP layout: leaves padded to a power of two; internal node i's
    // children sit at 2i+1 and 2i+2 — an ADJACENT pair AFTER their parent,
    // so refit's one reverse pass re-derives every box. The preflight's
    // cross-tree G1 caught the depth-first-push form this fixture
    // originally carried: its children land a whole subtree apart, refit's
    // adjacent union read the wrong pair, and THIS fixture's own G1 was
    // VACUOUS under it (refit and the checker shared the assumption, so
    // they agreed on garbage — the fixture gate can miss a shared-
    // assumption class; the preflight's second tree exposed it). Padded
    // leaves carry the last real leaf's box: every union is unchanged.
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

/// NODE-indexed leaf map for the heap layout.
fn heap_leaf_map(n: usize, order: &[usize]) -> Vec<usize> {
    let p = n.next_power_of_two();
    let mut map = vec![usize::MAX; 2 * p - 1];
    for k in 0..p {
        map[p - 1 + k] = if k < n { order[k] } else { order[n - 1] };
    }
    map
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

#[test]
fn f4_refit_output_is_a_valid_tree_over_the_same_leaves() {
    // G1 (validity-equivalence, pre-registered in Plan 373 T5): refit and
    // rebuild do NOT produce identical trees — same leaf membership, both
    // trees valid, different shape. Bit-identity is explicitly NOT asserted.
    let boxes: Vec<Box_> = (0..256)
        .map(|k| unit_box((k % 16) as f32 * 4.0, (k / 16) as f32 * 4.0, 0.0))
        .collect();
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by_key(|&k| boxes[k].lo[0].to_bits());
    let p = boxes.len().next_power_of_two();
    let mut nodes = vec![
        TNode {
            lo: [0.0; 4],
            hi: [0.0; 4],
            left: 0,
            count: 0
        };
        2 * p - 1
    ];
    build_heap(&boxes, &order, &mut nodes);
    let leaf_of = heap_leaf_map(boxes.len(), &order);
    let built_cost = top_cost(&nodes);

    // jitter three boxes slightly (the movers stay near their placed box)
    let mut moved = boxes.clone();
    for k in [3usize, 40, 200] {
        moved[k].lo[0] += 0.01;
        moved[k].hi[0] += 0.01;
    }
    let (refit_nodes, cost, action) = maintain(&nodes, built_cost, &moved, &[], &leaf_of);
    assert_eq!(action, MaintAction::Refit);
    assert!(
        cost <= built_cost * REFIT_SLACK,
        "cost {cost} vs build {built_cost}"
    );
    assert_valid_tree(&refit_nodes, &moved, &leaf_of);
    // same shape: the structural fields never move
    assert_eq!(refit_nodes.len(), nodes.len());
    for (a, b) in refit_nodes.iter().zip(nodes.iter()) {
        assert_eq!((a.left, a.count), (b.left, b.count));
    }
}

#[test]
fn f4_hysteresis_loose_movers_and_small_scene_rebuild() {
    let boxes: Vec<Box_> = (0..256)
        .map(|k| unit_box((k % 16) as f32 * 4.0, (k / 16) as f32 * 4.0, 0.0))
        .collect();
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by_key(|&k| boxes[k].lo[0].to_bits());
    let p = boxes.len().next_power_of_two();
    let mut nodes = vec![
        TNode {
            lo: [0.0; 4],
            hi: [0.0; 4],
            left: 0,
            count: 0
        };
        2 * p - 1
    ];
    build_heap(&boxes, &order, &mut nodes);
    let leaf_of = heap_leaf_map(boxes.len(), &order);
    let built_cost = top_cost(&nodes);

    // a teleporting unit: its box left the box it was placed with, widened
    // by that box's longest side — astray, so it joins the LOOSE set
    let mut moved = boxes.clone();
    moved[5] = Box_ {
        lo: [500.0, 500.0, 0.0, 0.0],
        hi: [501.0, 501.0, 1.0, 0.0],
    };
    assert!(astray(&boxes[5], &moved[5]));
    let (_, _, action) = maintain(&nodes, built_cost, &moved, &[5], &leaf_of);
    assert_eq!(action, MaintAction::Loose);

    // a small scene builds outright (a build is then microseconds)
    let small: Vec<Box_> = (0..32)
        .map(|k| unit_box(k as f32 * 2.0, 0.0, 0.0))
        .collect();
    let (_, _, action) = maintain(&nodes, built_cost, &small, &[], &leaf_of);
    assert_eq!(action, MaintAction::RebuildAll);
}

// ─────────────────────────────────────────────────────────────────────────────
// F5 — edge-avoiding-atrous-denoise-variance-guided
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    pub const ZERO: Vec3 = Vec3(0.0, 0.0, 0.0);
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3(self.0 + o.0, self.1 + o.1, self.2 + o.2)
    }
}

impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        self.0 += o.0;
        self.1 += o.1;
        self.2 += o.2;
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3(self.0 * s, self.1 * s, self.2 * s)
    }
}

impl std::ops::Div<f32> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f32) -> Vec3 {
        Vec3(self.0 / s, self.1 / s, self.2 / s)
    }
}

fn luminance(c: Vec3) -> f32 {
    0.2126 * c.0 + 0.7152 * c.1 + 0.0722 * c.2
}

/// the naive arm: the width reads THIS pixel's own variance — no floor, no
/// neighborhood — so a firefly-sparse pixel rejects every neighbour and
/// stays a dark hole.
fn denoise_pixel_raw_variance(
    color: &[Vec3],
    variance: &[f32],
    i: usize,
    w: usize,
    h: usize,
) -> Vec3 {
    let (x, y) = ((i % w) as isize, (i / w) as isize);
    let cp = color[i];
    let lp = luminance(cp);
    let sigma_l = 4.0 * variance[i].max(0.0).sqrt();
    let (mut sum, mut wsum) = (Vec3::ZERO, 0.0f32);
    for dy in -2isize..=2 {
        for dx in -2isize..=2 {
            let (qx, qy) = (x + dx, y + dy);
            if qx < 0 || qy < 0 || qx >= w as isize || qy >= h as isize {
                continue;
            }
            let q = qy as usize * w + qx as usize;
            let wl = (-(lp - luminance(color[q])).abs() / sigma_l.max(1e-9)).exp();
            sum += color[q] * wl;
            wsum += wl;
        }
    }
    if wsum > 0.0 { sum / wsum } else { cp }
}

/// the fixed arm: the width comes from the 3×3-filtered variance (the
/// propagated state stays per-pixel), the width has a positive floor, and
/// the filter runs on albedo-demodulated color — texture survives.
fn denoise_pixel_filtered_variance(
    demod: &[Vec3],
    albedo: &[Vec3],
    variance: &[f32],
    var_filtered: &[f32],
    i: usize,
    w: usize,
    h: usize,
) -> Vec3 {
    let (x, y) = ((i % w) as isize, (i / w) as isize);
    let cp = demod[i];
    let lp = luminance(cp);
    let sigma_l = 4.0 * var_filtered[i].max(0.0).sqrt() + 1e-4;
    let (mut sum, mut wsum) = (Vec3::ZERO, 0.0f32);
    for dy in -2isize..=2 {
        for dx in -2isize..=2 {
            let (qx, qy) = (x + dx, y + dy);
            if qx < 0 || qy < 0 || qx >= w as isize || qy >= h as isize {
                continue;
            }
            let q = qy as usize * w + qx as usize;
            let wl = (-(lp - luminance(demod[q])).abs() / sigma_l).exp();
            sum += demod[q] * wl;
            wsum += wl;
        }
    }
    let filtered = if wsum > 0.0 { sum / wsum } else { cp };
    // re-modulate: the guide's texture detail is exactly preserved.
    let a = albedo[i];
    let eps = 1e-3;
    Vec3(
        filtered.0 * a.0.max(eps),
        filtered.1 * a.1.max(eps),
        filtered.2 * a.2.max(eps),
    )
}

/// the 3×3 Gaussian of the variance — the edge-stopping width's input.
fn filter_variance_3x3(variance: &[f32], w: usize, h: usize) -> Vec<f32> {
    (0..variance.len())
        .map(|i| {
            let (x, y) = ((i % w) as isize, (i / w) as isize);
            let (mut s, mut ws) = (0.0f32, 0.0f32);
            for dy in -1isize..=1 {
                for dx in -1isize..=1 {
                    let (qx, qy) = (x + dx, y + dy);
                    if qx < 0 || qy < 0 || qx >= w as isize || qy >= h as isize {
                        continue;
                    }
                    let k = [0.5f32, 0.25][dx.unsigned_abs().min(1) as usize]
                        * [0.5f32, 0.25][dy.unsigned_abs().min(1) as usize];
                    s += k * variance[qy as usize * w + qx as usize];
                    ws += k;
                }
            }
            s / ws
        })
        .collect()
}

#[test]
fn f5_per_pixel_variance_leaves_a_dark_hole_the_filtered_width_fills_it() {
    let w = 5usize;
    let h = 5usize;
    let dark = Vec3(0.05, 0.05, 0.05);
    let bright = Vec3(2.0, 2.0, 2.0);
    let mut color = vec![bright; w * h];
    let mut variance = vec![1.0f32; w * h];
    let centre = 2 * w + 2;
    color[centre] = dark;
    variance[centre] = 0.0; // the firefly-sparse pixel drew none
    let ones = vec![Vec3(1.0, 1.0, 1.0); w * h]; // albedo 1: demod == color
    let var_filtered = filter_variance_3x3(&variance, w, h);

    let raw = denoise_pixel_raw_variance(&color, &variance, centre, w, h);
    assert!(
        raw.0 < 0.1,
        "the per-pixel width kept the dark hole: {raw:?}"
    );
    let fixed =
        denoise_pixel_filtered_variance(&color, &ones, &variance, &var_filtered, centre, w, h);
    assert!(
        fixed.0 > 1.5,
        "the filtered width filled the hole: {fixed:?}"
    );
}

#[test]
fn f5_zero_variance_with_noise_still_admits_under_the_floor() {
    let w = 5usize;
    let h = 5usize;
    let a = Vec3(1.0, 1.0, 1.0);
    let b = Vec3(1.0 + 1e-5, 1.0 + 1e-5, 1.0 + 1e-5); // numerical noise of "the same" input
    let mut color = vec![a; w * h];
    color[12] = b; // the centre differs from its neighbours by 1e-5
    let variance = vec![0.0f32; w * h];
    let ones = vec![Vec3(1.0, 1.0, 1.0); w * h];
    let var_filtered = filter_variance_3x3(&variance, w, h);

    let raw = denoise_pixel_raw_variance(&color, &variance, 12, w, h);
    assert_eq!(
        raw, b,
        "sigma 0 rejected the neighbours: the pixel stayed as it was"
    );
    let fixed = denoise_pixel_filtered_variance(&color, &ones, &variance, &var_filtered, 12, w, h);
    assert!(
        fixed.0 < raw.0 && fixed.0 > a.0,
        "the floor admits the neighbourhood (between a and the raw centre): {fixed:?}"
    );
}

#[test]
fn f5_demodulate_remodulate_preserves_the_guide_texture() {
    let w = 5usize;
    let h = 5usize;
    let albedo: Vec<Vec3> = (0..w * h)
        .map(|i| {
            let v = if (i % w + i / w) % 2 == 0 { 1.0 } else { 0.1 };
            Vec3(v, v, v)
        })
        .collect();
    let demod = vec![Vec3(0.5, 0.5, 0.5); w * h]; // flat: color = demod * albedo
    let variance = vec![1.0f32; w * h];
    let var_filtered = filter_variance_3x3(&variance, w, h);
    let i = 2 * w + 2;

    let fixed = denoise_pixel_filtered_variance(&demod, &albedo, &variance, &var_filtered, i, w, h);
    // the checkerboard survives: the output alternates with albedo[i]
    let expect = 0.5 * albedo[i].0;
    assert!(
        (fixed.0 - expect).abs() < 0.01,
        "texture survived: {fixed:?} vs {expect}"
    );

    // the raw-colour filter (no demodulation) blurs the same scene flat
    let color: Vec<Vec3> = (0..w * h)
        .map(|q| {
            Vec3(
                demod[q].0 * albedo[q].0,
                demod[q].1 * albedo[q].1,
                demod[q].2 * albedo[q].2,
            )
        })
        .collect();
    let raw = denoise_pixel_raw_variance(&color, &variance, i, w, h);
    assert!(
        raw.0 > 0.15 && raw.0 < 0.35,
        "the raw filter blurred to a mid grey: {raw:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F6 — wesl-variant-flag-set-total-fail-closed
// ─────────────────────────────────────────────────────────────────────────────

const ALL_FLAGS: [&str; 3] = ["hwrt", "merged", "use_media"];

#[derive(Default)]
struct VariantFlags {
    hwrt: bool,
    merged: bool,
    use_media: bool,
}

#[derive(Debug, PartialEq)]
enum CompileError {
    UnknownFlag(String),
    MissingConstant(String),
}

/// the naive resolver: unnamed flags default OFF — the silent wrong binary.
/// The @if directive governs the region body (the line that follows it).
fn resolve_naive(src: &str, lookup: &dyn Fn(&str) -> bool) -> String {
    let mut out = String::new();
    let mut pending = Some(true); // outside any region: emit
    for line in src.lines() {
        let t = line.trim_start();
        if let Some(name) = t.strip_prefix("@if(").and_then(|r| r.strip_suffix(")")) {
            pending = Some(lookup(name)); // the directive itself is not emitted
            continue;
        }
        if pending != Some(false) {
            out.push_str(line);
            out.push('\n');
        }
        pending = Some(true); // one-line regions in this model
    }
    out
}

/// the total resolver: every flag the source names must be PROVIDED — an
/// unprovided flag is a compile error naming it; imported constants under
/// the same totality. `Features { default: Feature::Error, .. }`.
fn resolve_total(
    src: &str,
    lookup: &dyn Fn(&str) -> Result<bool, CompileError>,
    consts: &dyn Fn(&str) -> Result<u32, CompileError>,
) -> Result<String, CompileError> {
    let mut out = String::new();
    let mut pending = Some(true); // outside any region: emit
    for line in src.lines() {
        let t = line.trim_start();
        if let Some(name) = t.strip_prefix("@if(").and_then(|r| r.strip_suffix(")")) {
            pending = Some(lookup(name)?); // an unprovided flag errors HERE
            continue; // the directive itself is not emitted
        } else if t.starts_with("import constants::") {
            let name = t
                .trim_start_matches("import constants::")
                .trim_end_matches(';');
            consts(name)?; // totality over imported constants too
        } else if pending != Some(false) {
            out.push_str(line); // the region body stays iff its flag held
        }
        out.push('\n');
        pending = Some(true); // one-line regions in this model
    }
    Ok(out)
}

/// the variant identity (the cache-key twin) composes over the SAME total
/// flag set — two sets differing anywhere name different variants.
fn variant_key(flags: &VariantFlags) -> u32 {
    flags.hwrt as u32 | (flags.merged as u32) << 1 | (flags.use_media as u32) << 2
}

#[test]
fn f6_an_unnamed_flag_is_an_error_and_the_silent_off_arm_compiles_wrong() {
    let src = "@if(use_media)\nfn fog() {}\nfn core() {}\n";
    // the forgetting host (the OLD two-flag list) fails LOUD, naming the flag
    let old_lookup = |name: &str| match name {
        "hwrt" => Ok(true),
        "merged" => Ok(false),
        other => Err(CompileError::UnknownFlag(other.to_string())),
    };
    match resolve_total(src, &old_lookup, &|_| Ok(4u32)) {
        Err(CompileError::UnknownFlag(name)) => assert_eq!(name, "use_media"),
        other => panic!("expected UnknownFlag, got {other:?}"),
    }
    // the naive resolver compiles the same source SILENTLY without the region:
    // the variant runs "successfully" with a feature missing — the defect
    // surfaces as behavior, never as an error.
    let naive_lookup = |name: &str| matches!(name, "hwrt" | "merged");
    let out = resolve_naive(src, &naive_lookup);
    assert!(
        !out.contains("fn fog"),
        "the media half silently compiled out"
    );
    assert!(out.contains("fn core"));
    let _ = ALL_FLAGS;
}

#[test]
fn f6_all_provided_compiles_and_missing_constants_refuse() {
    let src = "@if(hwrt)\nfn hw() {}\nimport constants::GUARD_WORDS;\nfn core() {}\n";
    let lookup = |name: &str| match name {
        "hwrt" => Ok(true),
        "merged" => Ok(false),
        "use_media" => Ok(false),
        other => Err(CompileError::UnknownFlag(other.to_string())),
    };
    let consts = |c: &str| {
        if c == "GUARD_WORDS" {
            Ok(4u32)
        } else {
            Err(CompileError::MissingConstant(c.to_string()))
        }
    };
    let out = resolve_total(src, &lookup, &consts).unwrap();
    assert!(out.contains("fn hw"));
    assert!(out.contains("fn core"));
    // a false flag drops its region without error
    let out2 = resolve_total("@if(merged)\nfn m() {}\n", &lookup, &consts).unwrap();
    assert!(!out2.contains("fn m"), "the region body dropped: {out2:?}");
    // an unprovided constant is an error, too
    assert!(matches!(
        resolve_total("import constants::MT_4;\n", &lookup, &consts),
        Err(CompileError::MissingConstant(_))
    ));
}

#[test]
fn f6_variant_keys_compose_over_the_total_flag_set() {
    let a = VariantFlags {
        hwrt: true,
        merged: false,
        use_media: false,
    };
    let b = VariantFlags {
        hwrt: true,
        merged: false,
        use_media: true,
    };
    assert_ne!(variant_key(&a), variant_key(&b));
    assert_eq!(variant_key(&a), variant_key(&a));
}

// ─────────────────────────────────────────────────────────────────────────────
// F7 — qmc-fixed-dims-hash-for-unbounded-draws
// ─────────────────────────────────────────────────────────────────────────────

fn hash_hp(x: u32) -> u32 {
    let mut i = x;
    i ^= i >> 16;
    i = i.wrapping_mul(0x21f0aaad);
    i ^= i >> 15;
    i = i.wrapping_mul(0xd35a2d97);
    i ^= i >> 15;
    i ^ 0xe6fe3beb
}

/// uniform [0, 1): 24 mantissa bits under a fixed exponent, minus one.
fn hash_draw(x: u32) -> f32 {
    f32::from_bits(0x3f80_0000 | (hash_hp(x) & 0x007f_ffff)) - 1.0
}

fn rng_dim(path: u32, depth: u32, slot: u32) -> u32 {
    (path << 28) ^ (depth << 20) ^ slot // every decision has an id
}

#[derive(Default)]
struct DrawCounter {
    sobol: usize,
    hash: usize,
}

fn sobol_value(_d: usize) -> f32 {
    0.5 // the table cost is the point, not the value
}

/// the NAIVE arm: every draw consumes the next Sobol dimension — a
/// data-dependent loop burns k dimensions per path (QMC's worst case).
fn path_estimate_naive(medium_steps: u32, c: &mut DrawCounter) -> f32 {
    c.sobol += 1;
    let cam = sobol_value(0);
    c.sobol += 1;
    let bsdf = sobol_value(1);
    let mut acc = 0.0f32;
    for _ in 0..medium_steps {
        c.sobol += 1; // dims 2..2+k, unbounded
        acc += sobol_value(2);
    }
    cam * 0.5 + bsdf * 0.3 + acc * 0.2
}

fn draw_fixed(
    seed: u32,
    path: u32,
    depth: u32,
    slot: u32,
    sobol_posture: bool,
    c: &mut DrawCounter,
) -> f32 {
    let d = rng_dim(path, depth, slot);
    if !sobol_posture {
        c.hash += 1;
        return hash_draw(d ^ seed); // the hash, keyed by the same id
    }
    c.sobol += 1;
    sobol_value((d & 0xfffff) as usize) // the sequence, by id
}

/// the FIXED arm: fixed-dimension decisions draw Sobol by id; the
/// data-dependent loop draws from a hash; live spends Sobol only at the
/// camera's first vertex.
fn path_estimate(mode_live: bool, medium_steps: u32, c: &mut DrawCounter) -> f32 {
    let sobol = !mode_live;
    let cam = draw_fixed(7, 0, 0, 0, true, c); // dim 0 always Sobol
    let bsdf = draw_fixed(7, 0, 1, 1, sobol, c);
    let mut acc = 0.0f32;
    for k in 0..medium_steps {
        let d = rng_dim(0, 2, 2) ^ hash_hp(k.wrapping_add(0x9e37_79b9));
        c.hash += 1;
        acc += hash_draw(d ^ 7); // A HASH, ALWAYS: a loop of no fixed length
    }
    cam * 0.5 + bsdf * 0.3 + acc * 0.2
}

#[test]
fn f7_fixed_dims_draw_sobol_loops_draw_hash() {
    let k = 37u32;
    let mut c = DrawCounter::default();
    let _ = path_estimate_naive(k, &mut c);
    assert_eq!(
        (c.sobol, c.hash),
        (2 + k as usize, 0),
        "the naive loop burned {k} Sobol dimensions"
    );

    let mut c = DrawCounter::default();
    let _ = path_estimate(true, k, &mut c);
    assert_eq!(
        (c.sobol, c.hash),
        (1, 1 + k as usize),
        "live: Sobol only at the camera vertex"
    );

    let mut c = DrawCounter::default();
    let _ = path_estimate(false, k, &mut c);
    assert_eq!(
        (c.sobol, c.hash),
        (2, k as usize),
        "offline: Sobol on the fixed dims, hash in the loop"
    );
}

#[test]
fn f7_dimension_ids_compose_from_path_vertex_and_slot() {
    assert_eq!(rng_dim(3, 4, 5), rng_dim(3, 4, 5));
    assert_ne!(rng_dim(0, 0, 0), rng_dim(0, 0, 1));
    assert_ne!(rng_dim(0, 1, 0), rng_dim(0, 0, 0));
    assert_ne!(rng_dim(1, 0, 0), rng_dim(0, 0, 0));
}

#[test]
fn f7_unrolled_direction_numbers_match_the_table() {
    // the N1b face: the first dimension's direction numbers unrolled inline
    // (16 select-xor pairs, no loop, no memory) are bit-identical to the
    // table-driven form on the CPU twin.
    for i in 0..1024u32 {
        assert_eq!(
            sobol_d0_unrolled(i).to_bits(),
            sobol_d0_table(i).to_bits(),
            "index {i}"
        );
    }
}

fn sobol_d0_unrolled(i: u32) -> f32 {
    let mut v = 0u32;
    if i & 0x0001 != 0 {
        v ^= 1 << 15;
    }
    if i & 0x0002 != 0 {
        v ^= 1 << 14;
    }
    if i & 0x0004 != 0 {
        v ^= 1 << 13;
    }
    if i & 0x0008 != 0 {
        v ^= 1 << 12;
    }
    if i & 0x0010 != 0 {
        v ^= 1 << 11;
    }
    if i & 0x0020 != 0 {
        v ^= 1 << 10;
    }
    if i & 0x0040 != 0 {
        v ^= 1 << 9;
    }
    if i & 0x0080 != 0 {
        v ^= 1 << 8;
    }
    if i & 0x0100 != 0 {
        v ^= 1 << 7;
    }
    if i & 0x0200 != 0 {
        v ^= 1 << 6;
    }
    if i & 0x0400 != 0 {
        v ^= 1 << 5;
    }
    if i & 0x0800 != 0 {
        v ^= 1 << 4;
    }
    if i & 0x1000 != 0 {
        v ^= 1 << 3;
    }
    if i & 0x2000 != 0 {
        v ^= 1 << 2;
    }
    if i & 0x4000 != 0 {
        v ^= 1 << 1;
    }
    if i & 0x8000 != 0 {
        v ^= 1;
    }
    v as f32 / 65536.0
}

fn sobol_d0_table(i: u32) -> f32 {
    const DIR: [u32; 16] = [
        1 << 15,
        1 << 14,
        1 << 13,
        1 << 12,
        1 << 11,
        1 << 10,
        1 << 9,
        1 << 8,
        1 << 7,
        1 << 6,
        1 << 5,
        1 << 4,
        1 << 3,
        1 << 2,
        1 << 1,
        1 << 0,
    ];
    let mut v = 0u32;
    for (j, d) in DIR.iter().enumerate() {
        if i & (1 << j) != 0 {
            v ^= d;
        }
    }
    v as f32 / 65536.0
}

// ─────────────────────────────────────────────────────────────────────────────
// F8 — scene-function-compile-failure-degrade-loud
// ─────────────────────────────────────────────────────────────────────────────

const OLD_FORM: &str = "* 73856093u ^ ";
const OLD_FORM_FIXED: &str = "* (73856093u) ^ ";

#[derive(Debug, PartialEq)]
enum KernelBuild {
    Ok {
        degraded: bool,
        reason: Option<String>,
    },
    Fatal(String),
}

fn compile(src: &str) -> Result<String, String> {
    if src.contains("BROKEN_CORE") {
        return Err("the app's own template is broken".into());
    }
    if src.contains(OLD_FORM) {
        return Err("bitwise operand must be parenthesised".into());
    }
    if src.contains("UNPARSED") {
        return Err("unparsed token".into());
    }
    Ok(src.to_string())
}

fn splice(template: &str, fns: &str) -> String {
    template.replace("//@functions-begin", &format!("{fns}\n//@functions-end"))
}

/// the two tiers: the CORE template fails closed; the USER functions
/// degrade LOUD (rebuild without them + a typed reason on the user's log).
/// Already-published old-grammar artifacts pass through a byte-scoped shim.
fn build_kernel(
    template: &str,
    scene_functions: &str,
    log: &mut Vec<String>,
) -> Result<String, KernelBuild> {
    let fns = if scene_functions.contains(OLD_FORM) {
        scene_functions.replace(OLD_FORM, OLD_FORM_FIXED)
    } else {
        scene_functions.to_string()
    };
    let src = splice(template, &fns);
    match compile(&src) {
        Ok(src) => Ok(src),
        Err(e) => {
            // tier 1: the CORE template alone must compile — if it does not,
            // the app's own source is broken: fail closed.
            let core = compile(template).map_err(KernelBuild::Fatal)?;
            let reason = format!(
                "this scene's material, volume or geometry functions do not compile, so it is drawn without them: {e}"
            );
            log.push(reason); // the USER's log channel
            Ok(core)
        }
    }
}

/// the naive path: ONE compile for template + user functions; any failure
/// panics the renderer — a user bundle's staleness IS the app's crash.
fn build_kernel_naive(template: &str, scene_functions: &str) -> String {
    let src = splice(template, scene_functions);
    compile(&src).unwrap_or_else(|e| panic!("kernel compile failed: {e}"))
}

const TEMPLATE: &str = "fn main() {}\n//@functions-begin\n//@functions-end\n";

#[test]
fn f8_core_fails_closed_user_content_degrades_loud() {
    // tier 1: user functions that do not compile (a form the shim does not
    // name) → degraded-but-valid + a typed reason on the user's log
    let mut log = Vec::new();
    let stale_fns = "fn h() -> u32 { return 1u + UNPARSED; }";
    let built = build_kernel(TEMPLATE, stale_fns, &mut log).unwrap();
    assert_eq!(built, TEMPLATE, "the kernel is made without them");
    assert_eq!(log.len(), 1);
    assert!(log[0].contains("do not compile"), "{}", log[0]);
    assert!(
        log[0].contains("unparsed token"),
        "the reason names the error: {}",
        log[0]
    );
    // tier 0: the CORE template failing is fatal — a panic IS the posture
    let broken = "fn main() { BROKEN_CORE }";
    let mut log2 = Vec::new();
    match build_kernel(broken, "fn fine() {}", &mut log2) {
        Err(KernelBuild::Fatal(_)) => {}
        other => panic!("expected Fatal, got {other:?}"),
    }
    assert!(log2.is_empty());
}

#[test]
fn f8_the_shim_is_byte_scoped_and_tripwired() {
    // byte-scoped: ONLY the named old form is rewritten, nothing else touched
    let fns = "fn a() -> u32 { return 1u * 73856093u ^ 2u; }\nfn b() -> u32 { return 3u ^ 4u; }";
    let shimmed = fns.replace(OLD_FORM, OLD_FORM_FIXED);
    assert!(shimmed.contains("* (73856093u) ^ "));
    assert!(shimmed.contains("3u ^ 4u"), "anything else left as it is");
    // the tripwire: the OLD form must still be a compile error AS WRITTEN.
    // If the host grammar ever accepts it, this assert fails and tells the
    // maintainer to DELETE the shim.
    let old = "fn h() -> u32 { return 1u * 73856093u ^ 2u; }";
    assert!(
        compile(old).is_err(),
        "the host takes the old form as written: the shim is no longer needed"
    );
    assert!(compile(&shimmed).is_ok());
}

#[test]
#[should_panic(expected = "kernel compile failed")]
fn f8_the_naive_path_takes_the_app_down_on_user_staleness() {
    // a published bundle carrying the old grammar, on a host with the naive
    // one-tier path: the user artifact's staleness IS the app's crash (and
    // on a device lane, the device then falls back to no renderer at all).
    let _ = build_kernel_naive(TEMPLATE, "fn h() -> u32 { return 1u * 73856093u ^ 2u; }");
}
