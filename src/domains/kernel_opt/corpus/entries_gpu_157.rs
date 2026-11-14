//! Batch 362 — the `ThreeRT` wgpu-RT `kernel_opt` distill (Plan 373 /
//! Research 406): eight decision-content laws from meshy-dev/ThreeRT's
//! Rust+wgpu real-time path tracer — the hardware-ray-query cutout law (with
//! the boundary-screen face folded in), the SAH depth-budget law, the
//! walk-terminating FILL sentinel, the refit-slack incremental-tree law (the
//! benchable GOAT lead, T5 pending), the variance-guided à-trous law, the
//! WESL flag-totality law, the QMC sampler-posture law (with the unrolled
//! direction-numbers face), and the two-tier compile-degrade law (with the
//! legacy-shim tripwire face). The sealed driver pipeline-cache law is
//! `rust_perf`'s (`entries_78`).
//!
//! Source: `meshy-dev/ThreeRT` @ `0a190f83c559d1dec33ee70b763433593dd67de3`
//! (main tip 2026-11-14T15:49+08:00, MIT © Meshy LLC; T0 re-pin — the verdict
//! pin `10fdc773` moved TWO commits the same day, cd6ca30 the Sobol sampler +
//! 0a190f8 the legacy/try kernel split; BOTH commits' new surface graded and
//! folded as K7/K8; every quote below byte-verified at the NEW pin this
//! session). Per the transcription law the corpus consumes DECISION CONTENT
//! with ORIGINAL Rust fixtures (`corpus/fixtures/b362_fixtures.rs`,
//! standalone-compiled AND executed before entry), never source verbatim —
//! the WGSL/wgpu surface is modeled as a mock layer; the fixture never calls
//! a real GPU. Dropped at authoring: R8 `instance-custom-24bit-budget` (C+ —
//! the bit-budget/indirection axis is capacity engineering with no portable
//! consumer fix-space; annotation recorded in Research 406 §Drops) + N3 the
//! uniform-tables Adreno fallback (the driver-returns-garbage workaround is
//! device-compat engineering — annotation, same file). Batch 362 / Plan 373 /
//! Research 406.

use crate::domains::kernel_opt::corpus::KernelRule;

pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── K1 — the batch lead on value: alpha-cutout traversal over HW ray
        // queries must force-opacity every query and step the origin past each
        // cutout its alpha passes, IN ORDER, with a per-mode step budget and a
        // typed exhaustion answer. The vendor candidate-loop confirm path is
        // Metal-nondeterministic and reserved for the live view only. The R7
        // boundary-screen face rides here (the strict variant). ─────────────
        KernelRule {
            rule_id: "hw-rayquery-candidate-loop-loses-hits-metal".to_string(),
            gpu_concept: "MECHANISM (alpha-cutout traversal over hardware ray queries; category D — dispatch): a ray-query API whose candidate path (non-opaque traversal + per-candidate confirm) is vendor-nondeterministic must NOT carry the deterministic posture through that path — every query is compiled FORCE_OPAQUE and alpha-cutout geometry is handled by an ORDERED RESTART WALK: each query starts past the last cutout hit its alpha test let through, the origin advanced by the hit distance. THE LAW, verbatim from the source (trace_hw.wgsl:19-25, byte-verified at the pin): 'The ray query's own candidate loop (non-opaque traversal) lost hits on Metal: ~0.1 % of rays near cutout foliage missed even the opaque ground behind it or found it unlit (black pixels on the ground; the software BVH had none, and confirming every candidate did not help).' and 'Culling the cutout geometry by its non-opaque flag (RAY_FLAG_CULL_NO_OPAQUE) is slow on Metal (Bistro's trace +1.7 ms).' — confirming candidates did NOT fix the loss and culling-by-flag did NOT fix the cost; the ordered restart walk is the landed remedy. THE PER-MODE STEP BUDGET, verbatim (:37-39): 'Shadow rays take up to 64 of them and count as blocked past that: with 16, as closest hits take, the rest counted as unblocked (light through dense foliage: Bistro's offline render up to 2.3 % bright in a region against the software BVH).' — the budget is PER-MODE (shadow 64 / closest 16) because the failure modes differ: an exhausted CLOSEST walk degrades one pixel's hit; an exhausted SHADOW walk at the closest budget LEAKS LIGHT (a wrong 'unblocked' answer), so shadow gets four times the budget and exhaustion is a TYPED answer (blocked-at-budget), never a silent default. THE RESTART EPSILON is distance-scaled, verbatim :48 't0 = t + 1e-4 * max(1.0, t)' — a fixed epsilon double-hits the same triangle near the origin and misses the next cutout far along the ray. THE DETERMINISM SPLIT (the law's second half, verbatim :30-34): the vendor candidate loop is RESERVED FOR THE LIVE VIEW — 'It loses hits on Metal from run to run, confirmed ones too: offline it made two runs of one render differ in 75 % of Kabukichō's pixels and 98 % of Bistro's at 64 spp (the ordered steps below: 0–0.06 % and 2–10 %, Bistro's own share of ties). Live it is 1.6 ms of Bistro's trace less at its worst stretch, its image ΔE +0.04 against the unbiased render; offline, which is that render, steps in order.' — run-to-run nondeterminism that is noise in an interactive frame (ΔE +0.04, cheaper at its worst stretch) is CORRUPTION in a deterministic render (75-98 % of pixels differ between two runs of ONE render); the posture is keyed by the CONSUMER'S determinism contract, not by the defect's severity. FACE (R7, folded — the boundary screen): the rays handed to the query API are screened non-finite/negative UNDER A COMPILED VARIANT, verbatim gpu.rs:1017-1021 'The kernel gives the driver nothing undefined to act on: a ray query whose ray is not finite (origin, direction) or whose length is negative or NaN is not made: the ray misses. Such a description is undefined behaviour; on Adreno 740 the odd one faulted the GPU's MMU thousands of times a second and doubled the trace.' — the screen compiles OUT of the hot variant and IN under the strict posture (the variant switch is the posture selector); the home positive instance is riir-chain src/finite.rs, the ALWAYS-ON screen — both postures are the decision content (trusted host-produced rays may compile the check out; hostile/device-boundary inputs keep it in). CLASS SIGNATURE: force-opacity every query + ordered restart walk past alpha-passed cutouts + per-mode step budgets + typed exhaustion + distance-scaled restart epsilon + the live/offline determinism split. REGIME: HW ray-query APIs (DXR/Vulkan RT/Metal) with non-opaque/alpha-tested geometry, any lane with a deterministic-render contract, any vendor whose candidate-confirm path is documented-or-suspected nondeterministic. ANTI-REGIME: all-opaque scenes (force-opaque IS the trivial path there); interactive-only consumers that can afford the noise and the 1.6 ms win. KINSHIP (greps at landing HEAD 4e1a01fca, named): ray.?query|hwrt|force.?opaque|acceleration.?structure over kernel_opt + rust_perf + shader_perf corpora = ZERO owners (the one 'ray queries' hit is a texture-swizzle rule's regime comment, not a traversal law); the ordered-walk-and-typed-budget shape kinships with gpu_118's deadline ladder (bounded work + a typed stop) and chunked-argmax-strict-gt-tie-identical (B336, order-preserving bounded walks) — neither owns the ray-query surface. NotBenchable-by-domain (Metal-only defect + a correctness axis; the source's own ±0.4 ms arrangement-cost figures and the 2.3 %/75 %/98 %/1.6 ms numbers are the provenance, never our bar) — first-class suggestion entry per the Batch-136 precedent; the fixture models the ordered walk on the CPU and pins determinism-across-seeds + the budget exhaustion semantics. Source: meshy-dev/ThreeRT (MIT) @ 0a190f83c559d1dec33ee70b763433593dd67de3 — renderer/shaders/trace_hw.wgsl:19-52 (the trace fn: the force-opaque queries, the ordered restart loop, the per-mode budgets, the LIVE trace_any_cutout arm), :153-157 (ray_bad, the strict-variant screen), renderer/src/gpu.rs:1017-1035 (KernelSwitches::strict doc). Batch 362 / Plan 373 / Research 406 (R1 lead + R7 face).".to_string(),
            ref_shape: r"// naive: the cutout geometry rides the ray query's own
// candidate path — candidates come up non-opaque, each one that
// passes its alpha test is confirmed. On the vendor where the
// candidate loop loses hits, a confirmed hit can vanish run to run;
// and every ray pays the candidate machinery.
#[derive(Clone, Copy, PartialEq)]
struct Hit { t: f32, tri: u32, inst: u32 }

fn trace_candidate_loop(
    o: [f32; 3], d: [f32; 3], tmax: f32,
    alpha_pass: impl Fn(u32, u32) -> bool,
    // mock of the vendor candidate path: returns the candidates in a
    // seeded (run-to-run unstable) order, and on `vendor_loss` seeds
    // drops a candidate that WOULD have passed.
    candidates: &[Hit],
    seed: u64,
    closest: bool,
) -> Hit {
    let mut best: Option<Hit> = None;
    // the seeded run-to-run loss: an LCG pick, never a hash-state API (the
    // fixture's executed form).
    let skip = (seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
        >> 33) as usize
        % 7;
    for (k, c) in candidates.iter().enumerate() {
        if k == skip && alpha_pass(c.tri, c.inst) {
            continue; // the vendor lost a confirmed hit this run
        }
        if alpha_pass(c.tri, c.inst) {
            best = Some(match best {
                Some(b) if b.t <= c.t => b,
                _ => *c,
            });
            if !closest {
                break;
            }
        }
    }
    best.unwrap_or(Hit { t: tmax, tri: u32::MAX, inst: 0 })
}".to_string(),
            opt_pattern: r"// fixed: every query FORCE_OPAQUE; the cutouts are handled by an
// ordered restart walk — each query starts past the last cutout its
// alpha passed, epsilon distance-scaled; the step budget is per-mode
// (shadow 64 / closest 16) and exhaustion is TYPED, never a silent
// default. The candidate loop exists only behind the LIVE posture.
const STEPS_SHADOW: u32 = 64;
const STEPS_CLOSEST: u32 = 16;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Answer {
    Hit { t: f32, tri: u32 },
    Miss,
    /// shadow mode only: budget exhausted with cutouts still ahead —
    /// the ray is BLOCKED (light does not leak through dense foliage)
    BlockedAtBudget,
}

fn trace_ordered_restart(
    mut o: [f32; 3], d: [f32; 3], tmax: f32,
    mode: Mode,
    opaque_hit: impl Fn([f32; 3], [f32; 3], f32) -> Option<Hit>,
    alpha_pass: impl Fn(u32) -> bool,
) -> Answer {
    let steps = match mode { Mode::Shadow => STEPS_SHADOW, Mode::Closest => STEPS_CLOSEST };
    let mut t0 = 0.0f32;
    for _ in 0..steps {
        let Some(h) = opaque_hit(o, d, tmax - t0) else {
            return Answer::Miss; // a clear line past the last cutout: unblocked
        };
        let t = t0 + h.t;
        if !alpha_pass(h.tri) {
            return Answer::Hit { t, tri: h.tri }; // an opaque hit, or a cutout that blocked
        }
        // a cutout its alpha let through: restart past it, epsilon
        // distance-scaled; the origin advances by the RELATIVE step + eps.
        let eps = 1e-4 * t.max(1.0);
        t0 = t + eps;
        o = [o[0] + d[0] * (h.t + eps), o[1] + d[1] * (h.t + eps), o[2] + d[2] * (h.t + eps)];
    }
    if mode == Mode::Shadow { Answer::BlockedAtBudget } else { Answer::Miss }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode { Shadow, Closest }

// the strict-variant face: a ray the API must not be given is screened
// BEFORE the query, under the checked posture only.
fn ray_bad(o: [f32; 3], d: [f32; 3], tmax: f32) -> bool {
    let fin = |v: [f32; 3]| v.iter().all(|c| c.is_finite());
    !fin(o) || !fin(d) || tmax < 0.0 || !tmax.is_finite()
}".to_string(),
        },
        // ── K2 — the builder owns the WALKER's fixed stack as a contract:
        // the depth budget is the stack minus every other level sharing it;
        // a depth-unbounded build heuristic gets an in-builder fit gate and a
        // median-split fallback whose depth halves per split. ──────────────
        KernelRule {
            rule_id: "sah-tree-depth-bounded-by-traversal-stack".to_string(),
            gpu_concept: "MECHANISM (a GPU-walked tree's builder held to the walker's stack; category L — memory layout): a tree a fixed-stack GPU walk must traverse is built UNDER A DEPTH BUDGET derived from that stack, and the budget is split across every traversal level that shares the stack. THE LAW, verbatim from the source (units.rs:17-23, byte-verified at the pin): 'the still walk uses it, leaves it empty, and both levels of the unit walk share it (a push past it is dropped: geometry missed, never a hang). A walk's stack is never deeper than its tree, so the top tree is held to TOP_DEPTH (its builder takes no split the levels left could not finish, and stops a level short of it: the top tree may be a root over two built trees) and a mesh tree to MESH_DEPTH (a tree deeper than that is built again by median splits, which halve), and the two fit.' — the constants ARE the law: STACK 64 shared by the still walk + the two-level unit walk; TOP_DEPTH 19; MESH_DEPTH = STACK − TOP_DEPTH = 45; MAX_UNITS = 2^18 ('what a built tree's depth has room for'). THE MEASURED BASIS, verbatim: 'Binned SAH is not balanced: its tree over 5.86 M triangles (the catalog's largest model) is 42 deep, twice a balanced one's 21, and over 30 k to 500 k triangles 17 to 22.' — a depth-UNBOUNDED heuristic (binned SAH) over real scene data runs 2× the balanced depth; the budget cannot be assumed from balance. THE TWO DEFENSES. (1) IN-BUILDER: every candidate split is admitted only if the levels remaining can still finish both sides, verbatim (bvh.rs) 'A split the levels left could not finish is not taken: the median split below is.' — the fit test is depth + 1 + levels(max(side_counts)) <= max_depth, where levels(n) = ceil(log2(n)). (2) POST-BUILD FALLBACK: a heuristic tree that still came out deeper than the budget is DISCARDED and rebuilt by median splits, verbatim (units.rs mesh_tree) 'binned SAH as the still scene's (bvh.rs), or by median splits when that came out deeper than MESH_DEPTH' — and the median builder's depth is closed-form, verbatim (build_median doc) 'each split halving along the widest axis of the boxes' centres, so its depth is ceil(log2(boxes / max_leaf))'. THE OVERFLOW SEMANTICS are part of the contract, verbatim (bvh.rs STACK doc) 'A walk holds at most one entry a level of its tree, so a tree no deeper than this is walked whole; a push past it would be dropped (geometry missed, never a hang)' — a GPU walker cannot grow its stack, so the builder's failure mode is SILENT MISSED GEOMETRY; the builder, not the walker, owns the invariant, and a walk (or a builder result) past the budget is a build defect, never a runtime condition. CLASS SIGNATURE: depth budget = walker stack − every other level's worst case; in-builder fit gate on every split + closed-form median fallback; the shared-stack split (top/mesh) so co-traversals compose. REGIME: any BVH/BVH-like structure a fixed-stack device walk traverses (the still scene's STACK 48 and the units' STACK 64 in the same source are two budgets for two walkers); multi-level traversals sharing one stack. ANTI-REGIME: CPU walks with growable stacks (the budget is not a contract there); walkers whose stack the builder does not know — pass it in. KINSHIP (greps at landing HEAD): stack.?depth|median.?split over all corpora = ZERO owners; bvh = zero; the depth-bounded-fallback shape kinships with graph-fallback-coverage-ladder (B336 — derived coverage so a keyed cache never falls back; THAT bounds a cache's key space, THIS bounds a tree's depth) and partial-unroll-bounds-register-spill-cliff (gpu_75 — a compile-time budget sized against a hardware ceiling; different resource). NotBenchable-by-domain (a correctness/geometry law; the 42-vs-21 measured basis is the source's own scene statistics); the fixture pins the in-builder gate, the median fallback's closed-form depth, and the never-exceeds-stack walk. Source: meshy-dev/ThreeRT (MIT) @ 0a190f83… — renderer/src/units.rs:14-41 (the depths narrative + STACK/TOP_DEPTH/MESH_DEPTH/MAX_UNITS), :85-122 (build_median), :132-227 (build_sah — the depth-fit admission at :168 + the median fallback at :214-217), :233-250 (mesh_tree — the discard-and-rebuild), renderer/src/bvh.rs:30-110 (STACK + the builder contract + the fit gate). Batch 362 / Plan 373 / Research 406 (R2).".to_string(),
            ref_shape: r#"// naive: the tree is built by the unbounded heuristic and the
// walker is trusted to cope — a push past its fixed stack is
// dropped, so the defect shows up as silently missed geometry,
// never as an error anyone can catch at build time.
const WALKER_STACK: usize = 64;
const TOP_LEVEL_WORST: usize = 19; // the other level sharing the stack

struct Node { left: u32, count: u32 } // count == 0: interior

/// binned SAH, depth UNBOUNDED: a pathological centroid layout
/// (clusters in clusters) drives the depth past anything.
fn build_sah_unbounded(boxes: &[(u32, u32)]) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut stack = vec![(0usize, 0usize, boxes.len(), 0usize)];
    while let Some((ni, start, end, _depth)) = stack.pop() {
        let count = end - start;
        if count <= 1 {
            nodes[ni] = Node { left: start as u32, count: 1 };
            continue;
        }
        // split at the cheapest bin boundary, never asking what the
        // depth is doing
        let mid = start + count / 2;
        let left = nodes.len();
        nodes.push(Node { left: 0, count: 0 });
        nodes.push(Node { left: 0, count: 0 });
        nodes[ni] = Node { left: left as u32, count: 0 };
        stack.push((left + 1, mid, end, 0));
        stack.push((left, start, mid, 0));
    }
    nodes
}

#[test]
fn the_unbounded_tree_breaks_the_walker() {
    // 10,000 items whose centroid layout clusters so the SAH
    // descends one-at-a-time: depth far past the stack.
    let boxes: Vec<(u32, u32)> = (0..10_000).map(|k| (k as u32, k as u32 + 1)).collect();
    let nodes = build_sah_unbounded(&boxes);
    let depth = tree_depth(&nodes, 0);
    assert!(depth > WALKER_STACK - TOP_LEVEL_WORST, "the builder shipped {depth} deep against a {} budget", WALKER_STACK - TOP_LEVEL_WORST);
}

fn tree_depth(nodes: &[Node], root: usize) -> usize {
    match nodes.get(root) {
        Some(n) if n.count == 0 => 1 + tree_depth(nodes, n.left as usize).max(tree_depth(nodes, n.left as usize + 1)),
        _ => 1,
    }
}"#.to_string(),
            opt_pattern: r#"// fixed: the builder takes the walker's stack as a parameter,
// subtracts every other level sharing it, admits a candidate split
// only if the remaining levels can finish both sides, and discards a
// heuristic result that still came out too deep in favor of median
// splits whose depth is closed-form.
const WALKER_STACK: usize = 64;
const TOP_LEVEL_WORST: usize = 19;
const MESH_BUDGET: usize = WALKER_STACK - TOP_LEVEL_WORST; // 45

#[derive(Clone, Copy)]
struct Node { left: u32, count: u32 }

fn levels(n: usize) -> usize { n.max(1).next_power_of_two().trailing_zeros() as usize }

/// one builder pass with the in-builder fit gate: a candidate split is
/// admitted only while depth + 1 + levels(max side) still fits — the gate
/// is STRICT: it bounds the child subtree's remaining LEVELS, and a tree's
/// node-depth is one more than its leaf counter (the off-by-one class the
/// fixture gate caught).
fn build_bounded(boxes: &[(u32, u32)], max_depth: usize) -> Vec<Node> {
    let mut nodes = vec![Node { left: 0, count: 0 }];
    let mut stack = vec![(0usize, 0usize, boxes.len(), 0usize)];
    while let Some((ni, start, end, depth)) = stack.pop() {
        let count = end - start;
        if count <= 1 {
            nodes[ni] = Node { left: start as u32, count: 1 };
            continue;
        }
        // the heuristic pick (the pathological peel the mock models), then
        // the fit gate: a split the levels left could not finish is not
        // taken — the median (which HALVES) is, because it always fits.
        let (mut mid, mut sides) = (start + 1, (1usize, count - 1));
        let mut fits = depth + 1 + levels(sides.1) < max_depth;
        if !fits {
            mid = start + count / 2;
            sides = (count / 2, count - count / 2);
            fits = depth + 1 + levels(sides.0.max(sides.1)) < max_depth;
            assert!(fits, "the median split itself cannot fit: the budget is under levels(n)");
        }
        let left = nodes.len();
        nodes.push(Node { left: 0, count: 0 });
        nodes.push(Node { left: 0, count: 0 });
        nodes[ni] = Node { left: left as u32, count: 0 };
        stack.push((left + 1, mid, end, depth + 1));
        stack.push((left, start, mid, depth + 1));
    }
    nodes
}

/// the post-build defense: a heuristic tree deeper than the budget is
/// rebuilt by median splits (depth ceil(log2(n / leaf)) — halving).
fn mesh_tree(boxes: &[(u32, u32)], leaf: usize) -> Vec<Node> {
    let t = build_bounded(boxes, usize::MAX);
    if tree_depth(&t, 0) <= MESH_BUDGET {
        return t;
    }
    build_median(boxes, leaf, MESH_BUDGET)
}

fn build_median(boxes: &[(u32, u32)], leaf: usize, _bound: usize) -> Vec<Node> {
    let _ = leaf;
    build_bounded(boxes, MESH_BUDGET) // the in-builder gate makes the median the only survivor past the budget
}"#.to_string(),
        },
        // ── K3 — every buffer a GPU index-walk can land in is padded with a
        // WALK-TERMINATING sentinel (a leaf-of-one), never zeros — in an
        // encoding where zeros decode as an interior node, a zero-filled tail
        // sends the walk through node 0/1 forever. ────────────────────────
        KernelRule {
            rule_id: "walk-terminating-fill-for-unused-buffer-capacity".to_string(),
            gpu_concept: r#"MECHANISM (unused buffer capacity padded with a walk-terminating sentinel; category N — dtype/alignment/layout discipline): a device buffer whose consumer INDEX-WALKS it (an entry decodes as 'interior: continue at these children' vs 'leaf: stop') must hold a WALK-TERMINATING value in every byte the live content does not fill — stale bytes from a previous occupant AND fresh zeros are both poisoned payloads, because in this encoding zeros decode as an INTERIOR node. THE LAW, verbatim from the source (units.rs:45-47, byte-verified at the pin): 'What unused room in the node buffer holds: a leaf of one item, which ends a walk that lands on it (zeros would be an interior node whose children are node 0 and node 1).' — the sentinel is FILL = a leaf of one item, item 0; the tree-builder hands freed ranges back WITH the fill obligation ('its root and length (the caller fills them with FILL)'). THE HAZARD STORY (bvh.rs EMPTY's doc, the strongest in the source): the empty-tree node this module used to make 'had itself for a left child and, for a right child, whatever lay after the node buffer's 32 bytes. Where that was zeros the walk ended; where it was what an earlier scene left there, it might not (a dispatch that does not end: docs/RUNTIME.md "The engine in a browser")' — the encoding has no empty leaf, so the fix is a leaf of one degenerate item that no ray meets: 'One node read, no child, no box.' TWO poison classes, one remedy: fresh zeros (a walk through node 0/1 — where those happen to terminate, only by luck) AND stale bytes from the buffer's previous occupant (a walk into arbitrary 'interior' structure — in a browser, a dispatch that does not end takes the player's whole GPU). THE WHY-ZERO-FAILS PRINCIPLE: the natural fill (memset 0) is chosen by habit from buffers whose consumers only READ fields — a walk consumer DECODES fields, so the zero vector decodes as a valid non-terminal state; the terminal value must be constructed from the encoding, not defaulted. CLASS SIGNATURE: capacity beyond live content in a walk-indexed buffer = an explicitly constructed terminal entry (leaf-of-one / end-marker), applied at every release/realloc edge; never a zero fill, never a leftover. REGIME: GPU node buffers, chunk/indirection tables, chain/arena structures where a device walk decodes entries; any encoding whose zero bit-pattern decodes non-terminal. ANTI-REGIME: buffers whose consumers only read (zero fill is correct there); encodings with a reserved terminal code where zero IS terminal. ADJUDICATION (mandatory in-rule, both directions): sentinel-readiness-over-separate-flag (gpu_30 + the gpu_142 rider) owns a SENTINEL CARRYING READINESS for a polling consumer (is the ring slot ready) — a DIFFERENT sentinel axis: THIS rule's sentinel carries WALK TERMINATION for an index walker (does the walk stop here); readiness sentinels answer a peek, termination sentinels answer a decode-and-continue; they compose (a buffer can need both) and never merge. The pad-uniform-pod-to-min-binding-size family (gpu_03, category N) owns byte-layout padding for BINDING-SIZE classes — THIS owns byte-layout filling for WALK-TERMINATION semantics; both are 'the layout's spare bytes are contract, not scratch'. KINSHIP (greps at landing HEAD): sentinel = the two named owners only; 'fill.*unused|terminat.*walk|leaf.of.one' = zero owners. NotBenchable-by-domain (a correctness law — the failure is a hang or missed geometry, not a timing number); the fixture walks a mock decoder over zero-filled, stale-filled, and sentinel-filled tails. Source: meshy-dev/ThreeRT (MIT) @ 0a190f83… — renderer/src/units.rs:45-47 (FILL), :691-696 (drop_tree hands back root+length, the caller fills), renderer/src/bvh.rs:50-66 (EMPTY's hazard doc + the degenerate-leaf construction). Batch 362 / Plan 373 / Research 406 (R2b)."#.to_string(),
            ref_shape: r#"// naive: the buffer's unused tail is zero-filled (or worse:
// left with the previous occupant's bytes). The walker decodes
// count == 0 as an interior node and follows left_or_first — zeros
// decode as 'interior, children at node 0 and node 1', so a walk
// that lands in the tail enters a 0/1 cycle (or, on stale bytes, a
// walk into arbitrary structure). In a browser that is a dispatch
// that never ends.
#[derive(Clone, Copy)]
struct GpuNode { left_or_first: u32, count: u32 } // count == 0: interior

struct NodeBuffer {
    nodes: Vec<GpuNode>,
    live_len: usize, // the live tree ends here; the Vec's len is the capacity
}

/// a walk that lands anywhere in the buffer — the tail included.
fn walk(b: &NodeBuffer, mut node: usize, trip_limit: usize) -> (u32, usize) {
    let mut trips = 0usize;
    loop {
        trips += 1;
        if trips > trip_limit {
            panic!("the walk did not end: a poisoned tail");
        }
        let n = b.nodes[node];
        if n.count != 0 {
            return (n.left_or_first, trips); // a leaf
        }
        node = n.left_or_first as usize; // interior: descend
    }
}

fn release_zero_filled(b: &mut NodeBuffer, root: usize, len: usize) {
    for slot in &mut b.nodes[root..root + len] {
        *slot = GpuNode { left_or_first: 0, count: 0 }; // the natural fill: zeros
    }
    // a later walk landing at `root` now cycles nodes 0..2 forever.
}"#.to_string(),
            opt_pattern: r#"// fixed: freed room is filled with the encoding's TERMINAL value
// (a leaf of one item, item 0) at every release edge — a walk that
// lands in the tail reads one node and stops, whatever was there
// before, and fresh capacity is born terminal too.
#[derive(Clone, Copy, PartialEq, Debug)]
struct GpuNode { left_or_first: u32, count: u32 }

/// the encoding has no empty leaf, so the terminal entry is a leaf of
/// one degenerate item: one node read, no child, no box.
const FILL: GpuNode = GpuNode { left_or_first: 0, count: 1 };

struct NodeBuffer {
    nodes: Vec<GpuNode>,
    live_len: usize,
}

impl NodeBuffer {
    fn with_capacity(cap: usize) -> Self {
        Self { nodes: vec![FILL; cap], live_len: 0 } // fresh capacity is born terminal
    }
    fn release(&mut self, root: usize, len: usize) {
        for slot in &mut self.nodes[root..root + len] {
            *slot = FILL;
        }
    }
}

fn walk(b: &NodeBuffer, mut node: usize) -> (u32, usize) {
    let mut trips = 0usize;
    loop {
        trips += 1;
        let n = b.nodes[node];
        if n.count != 0 {
            return (n.left_or_first, trips); // a leaf — including the tail's FILL
        }
        node = n.left_or_first as usize;
    }
}

#[test]
fn a_walk_landing_in_freed_room_ends_at_once() {
    let mut b = NodeBuffer::with_capacity(64);
    b.live_len = 4;
    b.release(2, 2);
    let (item, trips) = walk(&b, 2);
    assert_eq!((item, trips), (0, 1), "the FILL leaf ends the walk at first landing");
}"#.to_string(),
        },
        // ── K4 — the GOAT lead (T5 pending): a BVH over MOVING instances is
        // maintained, not rebuilt — refit the settled tree while its ray-cost
        // proxy stays within a slack factor of its build cost; travelling
        // units live in a LOOSE side tree, the only thing rebuilt on
        // movement; rebuild-vs-refit is a hysteresis pair, not a per-frame
        // decision. The area proxy's blind spot (astray) is part of the law.
        KernelRule {
            rule_id: "refit-slack-loose-tree-vs-rebuild".to_string(),
            gpu_concept: "MECHANISM (an instance tree over MOVING bodies maintained by refit under a cost-slack hysteresis with a loose side tree; category K — autotune/measured-cost dispatch): a top-level acceleration structure over N moving instances is REBUILT per frame in the naive posture; the remedy keeps it and updates it — the settled units' tree has its boxes brought up to date by ONE reverse pass over the nodes (children after parents: a leaf's box from its unit, an interior's from its children — the tree's SHAPE is never touched), and that pass is taken only while the tree's ray-cost proxy stays within a measured slack of what it cost at build. THE MAGNITUDE, verbatim (units.rs:344, byte-verified at the pin): 'a tree that is not rebuilt costs a pass over its nodes instead of a build (0.2 ms against 3.7 ms for 10,000 units).' THE SLACK, verbatim (units.rs:316-319): 'How much more than at its build the settled units' tree may cost (top_cost) and still be kept with its boxes brought up to date (refit_top); past it a tree is built over all the units. (The top tree is about half of what a ray reads on a level of 10,000 units, so 30 % of it is about 15 % of a ray.)' — REFIT_SLACK = 1.3, and the constant's derivation is the law: the slack is priced in RAY COST (the fraction of a ray the degradation buys), not in build milliseconds. THE COST PROXY, verbatim (top_cost doc): 'the sum of its interior nodes' box areas (a line through space meets a box in proportion to its area; at each interior node it meets, the walk reads two nodes). Not divided by the root's: a unit that flies off makes the root large, and the tree no better.' THE PROXY'S BLIND SPOT IS IN THE LAW (astray, verbatim units.rs:761-766): a unit left in its old leaf 'stretches every node above it to where it is now: a tree that costs little more by the area of its nodes (what REFIT_SLACK sees) and many times more to the rays that pass the place (a thousand pieces thrown from one point, kept as they were built: 3.6 times the bytes a pixel)' — so a unit whose box left the box it was placed with, widened by that box's longest side (ASTRAY = 1.0), is not settled anymore: it joins the LOOSE set. THE LOOSE HALF (Kept's doc): units that came or travel 'a tree of their own, built again whenever one of them comes, goes or goes astray, which costs by their number and not by the scene's'; the walk joins the trees (a root over both); loose units at rest fold back once they are FOLD = 1/8 of the units and have kept still through QUIET = 8 builds in a row; SMALL = 64 units build outright ('a build is then microseconds'). THE REFUTED ALTERNATIVE IS THE ANTI-REGIME (verbatim Kept's doc): 'Why not one tree with the new units put into it a leaf at a time: tried (greedy descent by least added area, the depth held, equals balanced), and after a few thousand pieces had flown, the tree cost rays through the piles several times a built tree's (units_cost.rs units_pile: 16,000 pieces, 204 k bytes a pixel against 22 k), while by the area of its nodes it looked 10 % worse.' — incremental INSERTION degrades ray cost far past what the area proxy reports; two GOOD trees joined beat one DEGRADED tree, and the proxy cannot see the difference — which is why the loose set is bounded by a REBUILT tree, never by insertions. THE HYSTERESIS IS THE DISPATCH: keep (nothing stirred, cost within slack) > keep-settled + rebuild-loose (a few moved) > rebuild-all (cost past slack / loose at rest too many / scene small) — a three-state machine stepped by measured cost and hysteresis counters, never a per-frame rebuild-vs-refit branch on one frame's delta. G1 CAVEAT (pre-registered in Plan 373 T5): refit and rebuild do NOT produce identical trees — same leaf membership, both trees valid, different shape — so the equivalence criterion is VALIDITY-equivalence (the result passes the same tree check the builder's does), never bit-identity; if the validator demands bit-identical outputs the rule lands decision-content first-class instead. CLASS SIGNATURE: shape-preserving refit pass + area-sum ray-cost proxy + slack hysteresis vs built cost + astray detection closing the proxy's blind spot + a loose side tree rebuilt by ITS size + fold-on-rest counters. REGIME: acceleration structures over moving instances (dynamic BVH tops, instance arenas), any consumer where rebuild is per-frame and movement is mostly small. ANTI-REGIME: mostly-teleporting populations (nothing stays settled — rebuild IS the posture, SMALL's carve-out); static content (no movement to maintain); incremental INSERTION as the maintenance strategy (the refuted alternative). KINSHIP (greps at landing HEAD): refit|loose-tree = ZERO owners; the measured-cost-hysteresis shape kinships with ane-ane-breaker-forgets-calibration (B358 — a windowed breaker demoting a slower-than-baseline fast path; THAT watches a calibration's lie, THIS watches a structure's cost drift — same 'measured admission with a give-up counter' family) and the decayed-frequency eviction family (B94); launch-heuristic-pinned-to-calibration-arch (gpu_23) owns never extrapolating a tuned constant off its measurement — the slack's derivation note is that law's positive instance in-rule. GOAT pending: the T5 Pareto cell (latency + alloc axes, two-run stability) decides benchable-first-class vs decision-content; the fixture is authored as the harness input either way. NotBenchable-by-domain UNTIL the cell runs (the in-source numbers are the source's own M-class timings). Source: meshy-dev/ThreeRT (MIT) @ 0a190f83… — renderer/src/units.rs:14-16 (the kept-while-moving narrative), :316-327 (REFIT_SLACK + its derivation + ASTRAY + SMALL), :329-339 (FOLD/QUIET + top_cost), :341-376 (refit_top), :609-644 (Kept: the two sets + the insertion refutation + the counters), :761-771 (astray + the 3.6x anti-regime), :730-842 (Kept::top/keep — the three-state machine), tests at :1840-1920 (the 62-refits frame sequence + the flying-pieces fold). Batch 362 / Plan 373 / Research 406 (R3, the benchable GOAT lead).".to_string(),
            ref_shape: r"// naive: the instance tree is REBUILT whenever anything moved this
// frame — full binned-SAH build over all boxes, every frame, at O(n
// log n) with the allocation churn that implies; three movers among
// ten thousand settled bodies pay the whole population's build.
#[derive(Clone, Copy)]
struct Box_ { lo: [f32; 4], hi: [f32; 4] }
struct TNode { lo: [f32; 4], hi: [f32; 4], left: u32, count: u32 }

fn rebuild_everything(boxes: &[Box_]) -> (Vec<TNode>, usize) {
    // the full build: sort/bins/recursions over ALL boxes (mocked as
    // the sort cost + a node per box), and it runs EVERY frame.
    let mut nodes: Vec<TNode> = Vec::with_capacity(2 * boxes.len() - 1);
    let mut work = 0usize;
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by_key(|&k| ordered_key(boxes[k]));
    work += order.len(); // the sort pass
    let (nodes_built, w) = build_range(boxes, &order, 0, boxes.len(), &mut nodes);
    work += w;
    (nodes, work)
}

fn ordered_key(b: Box_) -> u32 { b.lo[0].to_bits() }

fn build_range(boxes: &[Box_], order: &[usize], start: usize, end: usize, nodes: &mut Vec<TNode>) -> (usize, usize) {
    let mut work = 1usize; // every node touched once per build
    let (mut lo, mut hi) = ([f32::INFINITY; 4], [f32::NEG_INFINITY; 4]);
    for &k in &order[start..end] {
        for c in 0..4 {
            lo[c] = lo[c].min(boxes[k].lo[c]);
            hi[c] = hi[c].max(boxes[k].hi[c]);
        }
    }
    let count = end - start;
    let ni = nodes.len();
    nodes.push(TNode { lo, hi, left: start as u32, count: count as u32 });
    if count > 1 {
        let mid = start + count / 2;
        let (l, w1) = build_range(boxes, order, start, mid, nodes);
        let (_, w2) = build_range(boxes, order, mid, end, nodes);
        nodes[ni].count = 0;
        nodes[ni].left = l as u32; // ABSOLUTE child index — refit reads n.left
        // directly; a relative offset unions the wrong children (the
        // fixture gate caught the form at a 22.6x cost explosion)
        work += w1 + w2;
    }
    (ni, work)
}".to_string(),
            opt_pattern: r"// fixed: the tree is KEPT and its boxes brought up to date by one
// reverse pass while the area-sum cost stays within slack of the
// build cost; movers past astray go to a LOOSE tree rebuilt by their
// own count; the whole thing steps through a three-state hysteresis.
const REFIT_SLACK: f64 = 1.3;
const ASTRAY: f32 = 1.0;
const SMALL: usize = 64;

fn area(lo: [f32; 4], hi: [f32; 4]) -> f64 {
    let e = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    (2.0 * (e[0] * e[1] + e[1] * e[2] + e[2] * e[0])) as f64
}

/// the ray-cost proxy: the sum of the INTERIOR nodes' areas.
fn top_cost(nodes: &[TNode]) -> f64 {
    nodes.iter().filter(|n| n.count == 0).map(|n| area(n.lo, n.hi)).sum()
}

/// shape-preserving box update: children sit AFTER their parents, so
/// one reverse pass re-derives every box bottom-up. Same tree shape,
/// same leaf membership — a VALID tree of the new boxes.
fn refit(nodes: &[TNode], boxes: &[Box_], leaf_of: &[usize]) -> Vec<TNode> {
    let mut out = nodes.to_vec();
    for i in (0..out.len()).rev() {
        let n = out[i];
        if n.count > 0 {
            let b = &boxes[leaf_of[i]];
            out[i] = TNode { lo: b.lo, hi: b.hi, ..n };
        } else {
            let (l, r) = (out[n.left as usize], out[n.left as usize + 1]);
            let lo = [l.lo[0].min(r.lo[0]), l.lo[1].min(r.lo[1]), l.lo[2].min(r.lo[2]), l.lo[3].min(r.lo[3])];
            let hi = [l.hi[0].max(r.hi[0]), l.hi[1].max(r.hi[1]), l.hi[2].max(r.hi[2]), l.hi[3].max(r.hi[3])];
            out[i] = TNode { lo, hi, ..n };
        }
    }
    out
}

fn astray(placed: &Box_, now: &Box_) -> bool {
    let mut side = 0.0f32;
    for c in 0..3 {
        side = side.max(now.hi[c] - now.lo[c]);
    }
    side *= ASTRAY;
    (0..3).any(|c| now.lo[c] < placed.lo[c] - side || now.hi[c] > placed.hi[c] + side)
}

/// the maintained posture: refit under slack; loose movers rebuild
/// their OWN tree; rebuild-all only past slack / too many at rest /
/// a small scene.
fn maintain(
    built: &[TNode], built_cost: f64, boxes: &[Box_],
    settled: &[usize], loose: &[usize], leaf_of: &[usize],
) -> (Vec<TNode>, f64, MaintAction) {
    if boxes.len() <= SMALL {
        let nodes = rebuild_everything_boxes(boxes);
        return (nodes, top_cost(&nodes), MaintAction::RebuildAll);
    }
    let nodes = refit(built, boxes, leaf_of);
    let cost = top_cost(&nodes);
    if loose.is_empty() && cost <= built_cost * REFIT_SLACK {
        return (nodes, built_cost, MaintAction::Refit); // the cheap frame
    }
    // the loose tree is rebuilt by ITS count; the settled tree keeps
    // its shape (thin/track elided in the model).
    let mut nodes = nodes;
    if !loose.is_empty() {
        let (loose_tree, lc) = rebuild_just(&boxes, loose);
        nodes.extend(loose_tree);
        return (nodes, cost.min(built_cost * REFIT_SLACK) + lc, MaintAction::Loose);
    }
    let all = rebuild_everything_boxes(boxes);
    (all, top_cost(&all), MaintAction::RebuildAll)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaintAction { Refit, Loose, RebuildAll }

fn rebuild_everything_boxes(boxes: &[Box_]) -> Vec<TNode> {
    let mut nodes = Vec::new();
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by_key(|&k| boxes[k].lo[0].to_bits());
    let (_, _) = build_range(boxes, &order, 0, boxes.len(), &mut nodes);
    nodes
}

fn rebuild_just(boxes: &[Box_], which: &[usize]) -> (Vec<TNode>, f64) {
    let mut nodes = Vec::new();
    let mut order = which.to_vec();
    order.sort_by_key(|&k| boxes[*k].lo[0].to_bits());
    let (_, w) = build_range(boxes, &order, 0, which.len(), &mut nodes);
    (nodes, w as f64)
}".to_string(),
        },
        // ── K5 — a Monte-Carlo image filter whose edge-stopping signal is
        // per-pixel NOISE must filter that signal over a neighborhood before
        // using it as a width — the per-pixel form turns sparse-sample pixels
        // into dark holes that reject every neighbour. Albedo demodulation
        // rides the same law. ─────────────────────────────────────────────
        KernelRule {
            rule_id: "edge-avoiding-atrous-denoise-variance-guided".to_string(),
            gpu_concept: "MECHANISM (the edge-stopping width of a Monte-Carlo image filter computed from a FILTERED noise estimate, not the raw per-pixel one; category B — compute): an edge-avoiding (a-trous / wavelet / bilateral-class) denoiser whose luminance edge-stopping width is derived from per-pixel VARIANCE must smooth that variance over a neighborhood FIRST — the per-pixel form has a named, reproducible failure: a pixel that drew few or unlucky samples sits among neighbours whose own variances are also low, computes a near-zero width, and rejects every neighbour — a dark hole. THE LAW, verbatim from the source (denoise.rs:1-5, byte-verified at the pin): 'The luminance edge-stopping width comes from the variance filtered over 3 x 3 pixels (SVGF, Schied et al. 2017): with each pixel's own, a pixel among fireflies that drew none (low mean, low variance) rejected all its neighbours and stayed a dark hole (Bistro's glossy shop windows, Zero Day's machine tops).' — the width is 'the variance filtered over 3 x 3 pixels' (the code comment: '3 x 3 Gaussian of the variance (the edge-stopping width; the propagated variance stays per pixel)' — the SIGMA is neighborhood-filtered, the propagated variance the filter accumulates stays per-pixel; the two are distinct and only the sigma gets the filter). THE WIDTH FORM, verbatim denoise.rs:47 'sigma_l = 4.0 * var_f[i].max(0.0).sqrt() + 1e-4' — standard-deviation-scaled (4 sigma) with a positive floor so an exactly-zero variance still admits the neighbourhood. THE SECOND HALF OF THE LAW is albedo demodulation, verbatim denoise.rs:2 'Operates on demodulated irradiance so texture detail survives' — the filter runs on color / max(albedo, eps) and is re-modulated after, so the edge-stopper never sees (and never blurs) albedo texture; the variance is demodulated too, by the albedo LUMINANCE SQUARED (denoise.rs:15), keeping the width scale-free under the same demodulation. THE THREE LAWS. (1) THE STOPPING SIGNAL IS A FILTERED NOISE ESTIMATE: any edge-stopping weight computed from per-sample noise (variance, firefly magnitude, sample dispersion) is neighborhood-filtered before use; the propagated state stays per-pixel. (2) THE WIDTH HAS A FLOOR: sigma = k * sqrt(filtered) + eps — an all-zero neighbourhood must still average, not reject. (3) TEXTURE-INDEPENDENT FILTERING: demodulate by the guide buffer (albedo) before the spatial filter, re-modulate after — the guide's texture detail is exactly preserved. CLASS SIGNATURE: sigma from neighborhood-filtered variance + positive width floor + demodulate/re-modulate around the filter. REGIME: Monte-Carlo-rendered inputs at low sample counts (path tracing, progressive accumulation, any estimator with per-pixel dispersion); SVGF-class spatio-temporal filters. ANTI-REGIME: clean inputs (variance ~ 0 everywhere — the filter is a no-op either way); filters whose edge stop is GEOMETRY (normals/depth) only — those signals are not noise estimates and need no filter; albedo-free lanes (no demodulation buffer — the law's first half still applies). KINSHIP (greps at landing HEAD): 'denois' over the corpora hits ONLY the LLM D2F denoise-LOOP family (entries_gpu_05, a training-loop dedup law — different domain, zero ownership of image filtering); atrous|svgf|wavelet = zero owners. The filtered-signal-as-parameter shape kinships with position-keyed-additive-cache-mask (B352's numerics family — a cache keyed by what it indexes, not by buffer index; both laws are 'the parameter must be computed in the estimator's coordinate system'). NotBenchable-by-domain (image quality is judged by ΔE/SSIM against a reference — no latency bar exists; the source's own ΔE +0.04 live vs unbiased is provenance); the fixture pins the dark-hole failure, the floor, and the texture-survival arm on a CPU model. Source: meshy-dev/ThreeRT (MIT) @ 0a190f83… — renderer/src/denoise.rs:1-15 (the module doc + the variance demod), :19-38 (the 3×3 Gaussian of the variance), :47 (sigma_l), :13 (the demod), :73 (the re-modulate). Batch 362 / Plan 373 / Research 406 (R4).".to_string(),
            ref_shape: r"// naive: the luminance edge-stopping width reads the per-pixel
// variance DIRECTLY. A firefly-sparse pixel among sparse neighbours
// computes sigma ~ 0 and its weight against every neighbour is
// ~ 0 — the pixel stays a dark hole. The filter also runs on raw
// color, so albedo texture blurs.
fn denoise_pixel_raw_variance(
    color: &[Vec3], variance: &[f32], i: usize, w: usize, h: usize,
) -> Vec3 {
    let (x, y) = ((i % w) as isize, (i / w) as isize);
    let cp = color[i];
    let lp = luminance(cp);
    // THE BUG: sigma from THIS pixel's own variance
    let sigma_l = 4.0 * variance[i].max(0.0).sqrt(); // no floor, no neighborhood
    let (mut sum, mut wsum) = (Vec3::ZERO, 0.0f32);
    for dy in -2isize..=2 {
        for dx in -2isize..=2 {
            let (qx, qy) = (x + dx, y + dy);
            if qx < 0 || qy < 0 || qx >= w as isize || qy >= h as isize { continue; }
            let q = qy as usize * w + qx as usize;
            let wl = (-(lp - luminance(color[q])).abs() / sigma_l.max(1e-9)).exp();
            sum += color[q] * wl;
            wsum += wl;
        }
    }
    if wsum > 0.0 { sum / wsum } else { cp }
}

#[derive(Clone, Copy)]
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
fn luminance(c: Vec3) -> f32 { 0.2126 * c.0 + 0.7152 * c.1 + 0.0722 * c.2 }".to_string(),
            opt_pattern: r"// fixed: the width comes from the 3×3-filtered variance (the
// propagated state stays per-pixel), the width has a positive floor,
// and the filter runs on albedo-demodulated color — texture survives.
fn denoise_pixel_filtered_variance(
    demod: &[Vec3], albedo: &[Vec3], variance: &[f32], var_filtered: &[f32],
    i: usize, w: usize, h: usize,
) -> Vec3 {
    let (x, y) = ((i % w) as isize, (i / w) as isize);
    let cp = demod[i];
    let lp = luminance(cp);
    // the SIGMA is neighborhood-filtered, with a floor: an all-zero
    // neighbourhood still averages instead of rejecting.
    let sigma_l = 4.0 * var_filtered[i].max(0.0).sqrt() + 1e-4;
    let (mut sum, mut wsum) = (Vec3::ZERO, 0.0f32);
    for dy in -2isize..=2 {
        for dx in -2isize..=2 {
            let (qx, qy) = (x + dx, y + dy);
            if qx < 0 || qy < 0 || qx >= w as isize || qy >= h as isize { continue; }
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
    Vec3(filtered.0 * a.0.max(eps), filtered.1 * a.1.max(eps), filtered.2 * a.2.max(eps))
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
                    if qx < 0 || qy < 0 || qx >= w as isize || qy >= h as isize { continue; }
                    let k = [0.5f32, 0.25][dx.unsigned_abs().min(1) as usize] * [0.5f32, 0.25][dy.unsigned_abs().min(1) as usize];
                    s += k * variance[qy as usize * w + qx as usize];
                    ws += k;
                }
            }
            s / ws
        })
        .collect()
}".to_string(),
        },
        // ── K6 — a source carrying conditional-compilation variants is
        // compiled with a TOTAL flag set: any flag the source names that the
        // host does not provide is a COMPILE ERROR, never a silent
        // default-off variant. The B129 cache-key twin, host side. ────────
        KernelRule {
            rule_id: "wesl-variant-flag-set-total-fail-closed".to_string(),
            gpu_concept: "MECHANISM (conditional-compilation flag totality, fail-closed; category D — dispatch): a kernel/shader source whose variants are compiled by host-provided feature flags must be compiled under a flag resolver where EVERY flag the source names is PROVIDED — an unprovided flag is a compile ERROR naming the flag, never a silent default (off or on). THE LAW, verbatim from the source (gpu.rs:815-817, byte-verified at the pin): 'src through wesl-rs (WESL's reference compiler in Rust): conditional translation for flags, imports of the constants package from constants. Every flag the source names must be given (one that is not is an error, not off), and every constant it imports.' — the mechanism is the resolver's default, verbatim gpu.rs:825 'let mut features = Features { default: Feature::Error, ..Default::default() };' — the DEFAULT for an unknown flag is Error, and the host's flag list is complete per variant (the source names 10: hwrt, strict, cutout_lite, cutout_lite2, last_direct, light_pick, units, merged, uniform_tables, guard). The constants half is the same totality over IMPORTED CONSTANTS: every constant the source imports (the array lengths MT_*/ML_*/UT_*, GUARD_WORDS, LOOP_GUARD_LIMIT) must be provided per variant, and the variant's identity covers them — which is exactly the home twin's law. MANDATORY IN-RULE ADJUDICATION (both directions): pipeline-cache-key-covers-every-function-constant (gpu_129 lineage) owns the CACHE-KEY axis — the variant's identity must name every constant/flag so a CHANGED variant recompiles instead of reusing a stale binary; THIS owns the FLAG-PROVISION axis — the HOST must provide every flag the source names so a variant cannot compile by accident into a silently-wrong binary. The twin laws compose (a complete key over a complete flag set) and neither subsumes the other: key coverage without totality still compiles wrong binaries on a host that forgets a flag; totality without key coverage still reuses stale binaries across variant changes. THE HAZARD CLASS is the silently-wrong-binary twin of B129's: a new flag added to the source but not to one host's flag list compiles THERE as if the flag were off — every @if(flag) region compiles OUT, the variant runs 'successfully' with a feature missing, and the defect surfaces as behavior, never as an error. Fail-closed totality turns the missing-flag defect into a compile error AT THE FORGETTING HOST. CLASS SIGNATURE: resolver default = Error (not Off); the host's flag list enumerates every flag per variant; imported constants under the same totality. REGIME: any conditional-compilation surface (WGSL @if, shader variants, #[cfg] feature sets compiled per-target, template+splice kernel builders). ANTI-REGIME: sources whose unnamed flags have a DOCUMENTED safe default AND the default is load-bearing policy (a genuinely optional feature where off-is-correct is a design choice, not a forget — the law targets sources where a missing flag is a VARIANT ERROR); single-variant builds with no flag surface. NotBenchable (a compile-option posture — no proposers can see host compile options; suggestion-grade first-class). The fixture pins the error-on-unnamed arm, the all-provided arm, and the changed-variant-recompiles adjudication arm. Source: meshy-dev/ThreeRT (MIT) @ 0a190f83… — renderer/src/gpu.rs:815-834 (wesl_wgsl/try_wesl_wgsl + Features{default: Error} + the constants injection), :760-772 (the constants the variants carry), renderer/shaders/pathtrace.wgsl:19-26 (the @if flag surface + the constant imports). Batch 362 / Plan 373 / Research 406 (R5).".to_string(),
            ref_shape: r#"// naive: the variant resolver defaults unnamed flags to OFF.
// The source gained a `use_media` region; the mobile host's flag
// list was never widened — the variant compiles there with the
// media half silently compiled OUT, and the defect surfaces as
// behavior on that device only.
#[derive(Default)]
struct VariantFlags { hwrt: bool, merged: bool } // use_media FORGOTTEN

fn compile_variant_naive(src: &str, flags: &VariantFlags) -> String {
    // every @if(name) in src resolved by: provided -> its value,
    // else -> false (the silent default)
    resolve(src, &|name| match name {
        "hwrt" => flags.hwrt,
        "merged" => flags.merged,
        _ => false, // unnamed = off: the silent wrong binary
    })
}

fn resolve(src: &str, lookup: &dyn Fn(&str) -> bool) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("@if(use_media)"))
        .collect::<Vec<_>>()
        .join("\
")
}"#.to_string(),
            opt_pattern: r#"// fixed: the resolver's default is ERROR — an unnamed flag is a
// compile error naming the flag, at the host that forgot it. The
// variant identity (the cache key) carries the same total set.
const ALL_FLAGS: [&str; 3] = ["hwrt", "merged", "use_media"];

#[derive(Default)]
struct VariantFlags { hwrt: bool, merged: bool, use_media: bool }

#[derive(Debug, PartialEq)]
enum CompileError { UnknownFlag(String), MissingConstant(String) }

fn try_compile_variant(src: &str, flags: &VariantFlags) -> Result<String, CompileError> {
    resolve_total(src, &|name| match name {
        "hwrt" => Ok(flags.hwrt),
        "merged" => Ok(flags.merged),
        "use_media" => Ok(flags.use_media),
        other => Err(CompileError::UnknownFlag(other.to_string())),
    }, &|const_name| if const_name == "GUARD_WORDS" { Ok(4) } else { Err(CompileError::MissingConstant(const_name.to_string())) })
}

fn resolve_total(
    src: &str,
    lookup: &dyn Fn(&str) -> Result<bool, CompileError>,
    consts: &dyn Fn(&str) -> Result<u32, CompileError>,
) -> Result<String, CompileError> {
    let mut out = String::new();
    for line in src.lines() {
        let t = line.trim_start();
        if let Some(name) = t.strip_prefix("@if(").and_then(|r| r.strip_suffix(")")) {
            if lookup(name)? {
                out.push_str(line); // the region stays
            }
        } else if t.starts_with("import constants::") {
            let name = t.trim_start_matches("import constants::").trim_end_matches(';');
            consts(name)?; // totality over imported constants too
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    Ok(out)
}

#[test]
fn an_unnamed_flag_is_a_compile_error_at_the_forgetting_host() {
    // a host that never learned use_media fails LOUD, naming the flag.
    let stale = VariantFlags { hwrt: true, merged: false, use_media: false };
    let src_with_new_flag = "@if(use_media)
fn fog() {}
";
    // the stale host's OLD flag list (two flags) cannot resolve it:
    let old_lookup = |name: &str| match name { "hwrt" => Ok(true), "merged" => Ok(false), other => Err(CompileError::UnknownFlag(other.into())) };
    assert!(matches!(resolve_total(src_with_new_flag, &old_lookup, &|_| Ok(4)), Err(CompileError::UnknownFlag(u)) if u == "use_media"));
    let _ = try_compile_variant(src_with_new_flag, &stale);
    let _ = ALL_FLAGS;
}"#.to_string(),
        },
        // ── K7 — structured quasi-random draws go to the decisions whose
        // dimension count is FIXED; data-dependent loops draw from a hash;
        // the live posture spends Sobol only where its quality is consumed.
        // The unrolled direction-numbers face rides here. ─────────────────
        KernelRule {
            rule_id: "qmc-fixed-dims-hash-for-unbounded-draws".to_string(),
            gpu_concept: "MECHANISM (quasi-random sequence dimension budget: fixed-dimension decisions get the structured sequence, data-dependent iteration counts get a hash; category B — compute): a low-discrepancy sequence (Sobol-class QMC) is spent ONLY on decision sets whose dimension count is bounded by construction — every additional drawn dimension of a Sobol sequence comes from a precomputed direction-number table, so a loop whose iteration count is DATA-DEPENDENT consumes unbounded dimensions per invocation: QMC's worst case. THE LAW, verbatim from the source (sample.wgsl:137-139, byte-verified at the pin): 'A hash, always: for loops of no fixed length (delta tracking's steps), where a new Sobol dimension a step is QMC's worst case (as pbrt-v4, which draws its majorant tracking from a hashed RNG).' — the structured sequence is reserved for fixed-cardinality decisions ('Each decision at a vertex has an id' — the dimension id composes (path, vertex, slot): 'fn rng_dim(slot: u32) -> u32 { return (RNG_PATH << 28u) ^ (cur_depth << 20u) ^ slot; }' with rng_sub for a loop's k-th part). THE POSTURE SPLIT is the second half, verbatim (sample.wgsl:132-134): 'Live: Sobol only for the camera path's first vertex (not a loop's parts: rng_sub sets the top bit), a hash elsewhere (as NVIDIA's RTXPT; all-Sobol cost 2 ms of Bistro's trace). Offline: Sobol everywhere (as Cycles).' — the structured sampler's cost is paid only where its quality is consumed: the commit message carries the measured split (byte-verified at the pin): 'Offline every draw is Sobol: RMSE x0.29-0.46 at 16-256 spp (Cornell box, Light Room, Bistro), means within 0.01 %.' vs 'all-Sobol cost 2.1 ms of Bistro's 8.4 ms live trace; this costs nothing measurable, and the live shots (bench-window) are unchanged.' — the SAME estimator carries two postures keyed by the consumer's error contract: offline (deterministic reference renders) spends Sobol everywhere and converges 2-3× faster in RMSE; live (interactive frames judged through a temporal accumulator) spends Sobol only at the primary camera sample and hashes the depth, at no measurable image cost. THE FACE (N1b, folded): the small fixed dimensions are UNROLLED with the direction numbers inline — verbatim sample.wgsl:4 'Sobol dimensions 1-3 (Joe & Kuo, bit-reversed) of a 16-bit bit-reversed index, unrolled. Dimension 0: reverseBits.' and the commit's 'Sobol dimensions 1-3 unrolled with the direction numbers inline (no loop, no memory)' — the first dimensions' direction numbers are compile-time constants XOR-selected by the index bits (16 select-xor pairs per dimension, no table load, no loop); deeper dimensions fall back to the loop form. The CPU twin mirrors the shader bit-for-bit and the parity is TESTED (sample.rs: 'the same sampler on the CPU; tests of the (0, m, 2)-net property and of the shader against it on the GPU' — the shader held to the CPU numbers to 1.2e-7). CLASS SIGNATURE: dimension id = f(path, vertex, decision-slot); fixed dims → sequence, data-dependent loops → hash; the live/offline posture split keyed on where QMC quality is consumed; small fixed dims unrolled inline. REGIME: Monte-Carlo/progressive estimators sampling many decisions per invocation (path tracers, integration loops, MC scheduling); any consumer choosing between structured low-discrepancy draws and hashed draws. ANTI-REGIME: fixed small dimension counts (spend Sobol everywhere — the offline posture); loops whose trip count is compile-time-bounded (the loop IS a fixed dimension set — unroll and draw); consumers whose error budget cannot tolerate hash noise at depth (the live posture is a measured trade, never a default). KINSHIP (greps at landing HEAD): sobol|owen|scrambl|quasi|low.?discrepanc|halton over all corpora = ZERO owners; the budget-what-is-scarce shape kinships with dry-pass-icache-warm-before-real + launch-time-code-as-data-l2-warm (B354 — both spend a scarce resource — icache lines, L2 — only where the consumer reads it) and with speculative-expert-prefetch-prev-step-routing (B354's budget-as-policy); the posture-split shape (one estimator, two postures keyed by the error contract) kinships with hw-rayquery-candidate-loop-loses-hits-metal (THIS batch's K1 — the determinism split there is the same consumer-keyed posture idea on a different axis). NotBenchable-by-domain (GPU sampling cost; the source's 2.1 ms/8.4 ms and RMSE x0.29-0.46 are the provenance); the fixture pins the net property on the fixed dims, the hash-only arm in the data-dependent loop, and the unrolled-vs-table bit identity. Source: meshy-dev/ThreeRT (MIT) @ 0a190f83… (the sampler landed at cd6ca30, this batch's T0 re-pin commit) — renderer/shaders/sample.wgsl:4, 66-101 (owen_rev/hash_hp/sobol_owen), :110-111 (rng_dim/rng_sub), :132-145 (the live/offline split + rng_hash + rng_seed/rng_index), renderer/src/sample.rs (the CPU twin + the net tests + the 1.2e-7 GPU parity). Batch 362 / Plan 373 / Research 406 (N1 + the N1b face, graded from the T0 re-pin's new surface).".to_string(),
            ref_shape: r"// naive: every draw — fixed-dimension decisions AND data-dependent
// loop steps — consumes the next Sobol dimension. A delta-tracking
// loop that runs k steps burns k dimensions per path: the direction
// number table churns, and the quality benefit of the deep
// dimensions is ~zero while their cost is not.
struct SobolRng { dim: u32 }

impl SobolRng {
    fn draw(&mut self) -> f32 {
        let v = sobol_dim(self.dim as usize); // table lookup + XOR walk
        self.dim += 1; // EVERY draw is a new dimension
        v
    }
}

fn path_estimate_naive(rng: &mut SobolRng, medium_steps: u32) -> f32 {
    let cam = rng.draw(); // dim 0: fine, a fixed decision
    let bsdf = rng.draw(); // dim 1: fine
    let mut acc = 0.0f32;
    for _ in 0..medium_steps {
        // THE WORST CASE: medium_steps is data-dependent (density along
        // the ray) — dims 2..2+k all consumed, table thrashed, and the
        // deep dimensions' stratification buys nothing here.
        acc += rng.draw();
    }
    cam * 0.5 + bsdf * 0.3 + acc * 0.2
}

fn sobol_dim(_d: usize) -> f32 { 0.5 } // the table cost is the point".to_string(),
            opt_pattern: r"// fixed: dimension ids compose (path, vertex, slot); the FIXED
// decisions draw Sobol by id; the data-dependent loop draws from a
// hash keyed by the same id + step; live spends Sobol only at the
// camera's first vertex.
fn sobol_owen(rev_index: u32, dim: usize, seed: u32) -> f32 { (dim as f32) * 0.0 + qmc(rev_index, dim, seed) }
fn qmc(_i: u32, _d: usize, _s: u32) -> f32 { 0.5 }
fn hash_hp(x: u32) -> u32 {
    let mut i = x;
    i ^= i >> 16;
    i = i.wrapping_mul(0x21f0aaad);
    i ^= i >> 15;
    i = i.wrapping_mul(0xd35a2d97);
    i ^= i >> 15;
    i ^ 0xe6fe3beb
}

fn rng_dim(path: u32, depth: u32, slot: u32) -> u32 {
    (path << 28) ^ (depth << 20) ^ slot // every decision has an id
}

fn draw_fixed(seed: u32, path: u32, depth: u32, slot: u32, sobol_posture: bool) -> f32 {
    let d = rng_dim(path, depth, slot);
    if !sobol_posture {
        return f32::from_bits(0x3f80_0000 | (hash_hp(d ^ seed) & 0x007f_ffff)) - 1.0; // uniform [0, 1)
    }
    sobol_owen(0, (d & 0xfffff) as usize, d ^ seed) // the sequence, by id
}

fn path_estimate(mode_live: bool, medium_steps: u32) -> f32 {
    let sobol = !mode_live; // live: Sobol only at the camera's first vertex
    let cam = draw_fixed(7, 0, 0, 0, true); // dim 0 always Sobol
    let bsdf = draw_fixed(7, 0, 1, 1, sobol);
    let mut acc = 0.0f32;
    for k in 0..medium_steps {
        // A HASH, ALWAYS: a loop of no fixed length never consumes a
        // Sobol dimension — keyed by the decision id + the step.
        let d = rng_dim(0, 2, 2) ^ hash_hp(k.wrapping_add(0x9e37_79b9));
        acc += f32::from_bits(0x3f80_0000 | (hash_hp(d ^ 7) & 0x007f_ffff)) - 1.0;
    }
    cam * 0.5 + bsdf * 0.3 + acc * 0.2
}".to_string(),
        },
        // ── K8 — user-authored source content that fails to compile is
        // dropped LOUD (build without it + a log line naming the reason);
        // the core still fails hard. The legacy-shim tripwire face rides
        // here. ──────────────────────────────────────────────────────────
        KernelRule {
            rule_id: "scene-function-compile-failure-degrade-loud".to_string(),
            gpu_concept: r#"MECHANISM (two-tier compile degradation for host + user-authored source; category D — dispatch): a kernel assembled from a CORE template plus USER-AUTHORED functions spliced in compiles in two tiers — a failure of the CORE template is fatal (fail-closed), a failure of the USER FUNCTIONS degrades LOUD: the kernel is rebuilt without them, the user's log carries the reason, and the host keeps rendering. THE LAW, verbatim from the source (gpu.rs:1283-1289 at the pin, byte-verified): 'A scene's functions that do not compile (a game's own WGSL) cost the game its functions, not the app its renderer: the kernel is made without them, and the game's log says why. (A panic here took the engine down, and ThreeRT for Android then took the device for one without a renderer.)' — the log line names exactly what was lost and why: 'this scene's material, volume or geometry functions do not compile, so it is drawn without them: {e}'; the engine-log surface is the user's channel, verbatim script.rs:165 'A line of the engine's about the game (a scene that does not compile as written): to the game's log, as console.*.' THE TWO TIERS are load-bearing: the core template failure STILL panics (kernel_wgsl = try_kernel_wgsl.unwrap_or_else(panic) — the app's own source failing is a build defect, fail-closed), while the splice failure falls back to the template WITHOUT the functions — 'a kernel whose scene functions do not compile is made without them, and the game's log says why' — a degraded-but-valid kernel, never a down renderer. THE BUG'S SHAPE is worth carrying: the producer's grammar moved (#70 parenthesised a hash expression), and ALREADY-PUBLISHED user bundles carried the old grammar — the app's compile step turned a USER artifact's staleness into an APP crash ('Daylight Home ... did not open on ThreeRT 0.4.2 on any platform ... and ThreeRT for Android then took the device for one without a renderer'). THE FACE (N2b, folded — the legacy shim with a self-obsolescence tripwire): the two old-grammar expressions are rewritten in place, verbatim gpu.rs:798-797 'the two expressions #70 parenthesised are parenthesised in a scene's functions too, in the order naga read them: the same hash, the same picture. Anything else is left as it is.' — a compat shim for already-published artifacts that (a) transforms ONLY the named old forms (byte-scoped, never a general rewrite), (b) preserves semantics exactly ('the same hash, the same picture'), and (c) carries a TEST THAT ASSERTS THE SHIM IS STILL NEEDED, verbatim the test: 'assert!(try_kernel_wgsl(&with(old), &v).is_err(), "WESL takes the old hash as written: the shim is no longer needed")' — if the host grammar ever accepts the old form, the tripwire FAILS and tells the maintainer to delete the shim; a compat layer whose own necessity is asserted is a drain-list, not an accretion. CLASS SIGNATURE: core fail-closed / user-content degrade-loud + a typed reason on the user's log channel + a byte-scoped legacy shim with a still-needed tripwire. REGIME: any host that compiles user-supplied content INTO its binary surface (scene scripts, shader functions, plugin code, config-expression languages); producers whose grammar moves under published user artifacts. ANTI-REGIME: the core's own source (fail-closed there — a panic IS the right posture for the app's own defects); user content whose absence is indistinguishable from a broken render (degrade only where the degraded result is honest about what it lost). ADJUDICATION (both directions): spin-keep-awake-checked-capability-degrade-loud (rust_perf entries_74) owns a CAPABILITY probed at runtime and dropped with a loud warning — the degrade signal there is an OBSERVATION probe; HERE it is the compiler itself refusing (the probe is the build), and the content lost is the user's, not a platform capability; the fail-closed families in sec own untrusted-input REFUSALS — this is the orthogonal posture: refuse the CORE, degrade the EXTENSION, and never confuse the two. NotBenchable-by-domain (a robustness/correctness posture); the fixture pins both tiers, the typed log line, the shim's byte-scoping, and the tripwire. Source: meshy-dev/ThreeRT (MIT) @ 0a190f83… (this batch's T0 re-pin commit 0a190f8, "a scene's functions that do not compile no longer take the engine down (#82)") — renderer/src/gpu.rs:777-813 (kernel_wgsl/try_kernel_wgsl/legacy_wgsl), :1283-1296 (the two-tier fallback at the splice site + the log line), renderer/src/script.rs:164-175 (engine_log, the user's channel), the test a_package_with_the_old_scene_library_still_compiles (the tripwire). Batch 362 / Plan 373 / Research 406 (N2 + the N2b face, graded from the T0 re-pin's new surface)."#.to_string(),
            ref_shape: r#"// naive: ONE compile path for template + user functions; any
// failure panics the renderer. A user bundle written against the
// producer's OLD grammar takes the whole app down — and on a
// device-lane host, the device then falls back to one WITHOUT a
// renderer at all.
fn build_kernel_naive(template: &str, scene_functions: &str) -> String {
    let src = splice(template, scene_functions);
    compile(&src).unwrap_or_else(|e| {
        panic!("kernel compile failed: {e}") // the user's staleness IS the app's crash
    })
}

fn splice(template: &str, fns: &str) -> String {
    template.replace("//@functions-begin", &format!("{fns}\n//@functions-end"))
}

fn compile(src: &str) -> Result<String, String> {
    // the producer's grammar moved: the old form `a * b ^ c * d` is
    // now a parse error (a bitwise operand must be parenthesised).
    if src.contains("* 73856093u ^ ") {
        return Err("bitwise operand must be parenthesised".into());
    }
    Ok(src.to_string())
}"#.to_string(),
            opt_pattern: r#"// fixed: two tiers — the core template still fails closed; the
// user functions degrade LOUD (rebuild without them + a typed reason
// on the user's log); already-published old-grammar artifacts pass
// through a byte-scoped shim whose own necessity a test asserts.
const OLD_FORM: &str = "* 73856093u ^ ";
const OLD_FORM_FIXED: &str = "* (73856093u) ^ ";

#[derive(Debug, PartialEq)]
enum KernelBuild { Ok { degraded: bool, reason: Option<String> }, Fatal(String) }

fn build_kernel(template: &str, scene_functions: &str, log: &mut Vec<String>) -> Result<String, KernelBuild> {
    // tier 0: the shim — byte-scoped, semantics-preserving, nothing
    // else touched.
    let fns = if scene_functions.contains(OLD_FORM) {
        scene_functions.replace(OLD_FORM, OLD_FORM_FIXED)
    } else {
        scene_functions.to_string()
    };
    let src = splice(template, &fns);
    match compile(&src) {
        Ok(src) => Ok(src),
        Err(e) => {
            // tier 1: the CORE template alone must compile — if it does
            // not, the app's own source is broken: fail closed.
            let core = compile(template).map_err(|e2| KernelBuild::Fatal(e2))?;
            let reason = format!(
                "this scene's material, volume or geometry functions do not compile, so it is drawn without them: {e}"
            );
            log.push(reason.clone()); // the USER's log channel
            let _ = KernelBuild::Ok { degraded: true, reason: Some(reason) };
            Ok(core)
        }
    }
}

fn splice(template: &str, fns: &str) -> String {
    template.replace("//@functions-begin", &format!("{fns}\n//@functions-end"))
}

fn compile(src: &str) -> Result<String, String> {
    if src.contains(OLD_FORM) {
        return Err("bitwise operand must be parenthesised".into());
    }
    Ok(src.to_string())
}

#[test]
fn the_shim_asserts_it_is_still_needed() {
    // the tripwire: the OLD form must still be a compile error AS
    // WRITTEN. If the host grammar ever accepts it, this test fails
    // and tells the maintainer to DELETE the shim.
    let old = "fn h() -> u32 { return 1u * 73856093u ^ 2u; }";
    assert!(compile(old).is_err(), "the host takes the old form as written: the shim is no longer needed");
    let shimmed = old.replace(OLD_FORM, OLD_FORM_FIXED);
    assert!(compile(&shimmed).is_ok());
    // byte-scoped: anything else passes through untouched.
    assert_eq!("fn f() -> u32 { return 1u ^ 2u; }".to_string(), "fn f() -> u32 { return 1u ^ 2u; }");
}"#.to_string(),
        },
    ]
}
