//! Entries slice — Batch 223 (Plan 222) — the lithos-metal pack, slice A:
//! R1 coherence-scope (`MemoryAccess`), R2 open-group barrier placement with
//! the bus-first sibling axis + the arena eligibility law folded (Dispatch),
//! R3 bounded cross-worker barrier protocol (Parallelism), R4 fusion scope
//! by barrier cost (Fusion). Source lithos-ai/lithos-metal @ 236475fa
//! (Apache-2.0), HEAD verified == the pin this session; every quote
//! re-verified at the pin clone before landing.
//!
//! DEDUP (re-run at authoring, 3+ vocabularies): zero prior owners —
//! R1 (`coherent` / `__ldg` / `read-only cache` / `weight cache`): nearest
//! are the L2 cache-POLICY family (`weight-stream-non-temporal-loads`,
//! `producer-store-evict-last-pins-cross-stream-consumer`,
//! `dead-line-l2-discard-after-last-consumer` — a line's LIFETIME, not the
//! cache PATH), `low-bit-warp-run-load-shuffle-redistribute` (load
//! direction, not qualification) and `sentinel-readiness-over-separate-flag`
//! (cross-kernel visibility, not intra-dispatch qualification); no rule
//! qualifies accesses by provable immutability.
//! R2 (`open-group` / `variant_group` / `whole-prefix` / `conflict swap`):
//! zero rule-content owners; nearest are the cuda-graph replay-contract
//! family (`captured-graph-bakes-buffer-addresses`,
//! `capture-region-dispatch-only`), `concurrent-dispatch-dependency-level-barriers`
//! (levels under a DROPPED implicit order — the inverse posture) and
//! `barrier-free-per-dependency-wait-from-loop-counters` (host-side waits).
//! R3 (`barrier protocol` / `fan-in` / `leader release` / `spin bound` /
//! `release epoch`): no cross-worker barrier-protocol owner; nearest are
//! `whole-sequence-zero-barrier-single-dispatch-scan` (the zero-barrier
//! pole), `threadgroup-scope-reduction-single-writer` (sharing scoped below
//! the barrier's necessity), `stripe-sms-l2-semaphore-reduce` (one
//! reduction's L2 semaphore) and the `gpu_30` cross-kernel poll family.
//! R4 (`fusion scope` / `megakernel` / `dispatch boundary` / `barrier
//! cost`): nearest are `grid-collapsing-mega-kernel-occupancy-trap` (the
//! WebGPU no-sync trap; here a protocol exists and still loses),
//! `fusion-payback-gated-on-per-dispatch-overhead` (the savings side) and
//! `dispatch-cost-sequential-measurement-not-single-op` (the measurement
//! protocol); none owns the barrier-vs-boundary cost decision or the
//! unfused-control discipline.

use super::KernelRule;

#[allow(clippy::too_many_lines)]
pub fn entries() -> Vec<KernelRule> {
    vec![
        KernelRule {
            rule_id: "coherence-scope-by-write-set-ordinary-loads-on-immutable".to_string(),
            gpu_concept: "MECHANISM (coherence qualification as a per-pointer WRITE-SET decision): on GPUs with a non-coherent read-only/weight-cache path (Metal's `coherent(device)` vs ordinary `device` qualification; the CUDA face is the same decision the compiler makes for `__ldg`/`const __restrict__` — read-only data cache vs L1), read data PROVABLY IMMUTABLE through ORDINARY (non-coherent) accesses and everything else through COHERENT ones — the ordinary qualifier keeps the weight-cache path, coherent qualification DEFEATS it. The source's staging pass lifts EVERY device pointer to coherent by DEFAULT (a whole-source device-to-coherent(device) rewrite, static_fusion.py:652), then strips coherence per-POINTER where the dispatch cannot change the data (:654-655). THREE IMMUTABLE CLASSES, each with its in-source justification, verbatim: (1) packed weights — 'uint4 pointers in this kernel address immutable packed weights only.' (static_fusion.py:657); (2) the KV prefix strictly before the write cursor — 'This helper only reads keys strictly before the current position. No stage in this dispatch writes that range. Tail loads/stores and every cross-task intermediate retain coherent device accesses.' (:640-642); (3) external inputs published by the preceding dispatch — 'The preceding dispatch publishes inputs; internal data stays coherent.' (:1005) — classified as exactly the buffers no stage writes. THE GRANULARITY PAIR (two granularities by PURPOSE in the same pass): conflict/barrier decisions classify at BUFFER granularity CONSERVATIVELY — 'Classify by the entire buffer name so aliases at different offsets remain coherent whenever any task writes to that allocation.' (:578-579, the written_buffers docstring) — while coherence is per-POINTER FINE: two views of ONE weight block take DIFFERENT qualifiers — 'Narrow scale loads are also views of the immutable weight block, not cross-worker activations. Preserve their ordinary weight-cache path.' (:659-660) — the narrow-scale reinterpret keeps ordinary, the activation view stays coherent. The framework tensor-view type cannot carry the qualifier at all — 'The tensor view type is part of MPP and cannot carry coherent qualification; other device pointers (including its backing activation pointer) can.' (:636-637) — so the classification is a per-binding decision, never a type-level one. PADDED-SPECULATION RIDER: the clamp that makes out-of-bounds speculation past a select LEGAL rides the same arm — 'Ordinary cached reads may be speculated past a select.' (:596) — the load index is clamped to the live extent so a speculated read lands inside the prefix even when the select would have rejected the lane. The merged-program form applies the same decision by buffer ROLE: weights stay plain ordinary device, everything else is coherent (merge()'s type table, :995), with const + __restrict kept available for the weight path. REGIME: single-dispatch persistent or multi-stage kernels where an immutable class is provable from the write-set (weights; append-only history read below the write cursor; prior-dispatch outputs). ANTI-REGIME: any alias the write-set cannot classify stays coherent (the conservative default is a LIFT to coherent, never a strip); racing scratch never qualifies; a kernel whose weight pointer is REUSED for scratch reinterprets into the coherent class. Distinct from weight-stream-non-temporal-loads (gpu_07), producer-store-evict-last-pins-cross-stream-consumer and dead-line-l2-discard-after-last-consumer (gpu_48) — the L2 cache-POLICY family decides a line's LIFETIME (evict priority, discard-after-death) through cache-policy operands; this decides the CACHE PATH through the coherence QUALIFIER, and no policy operand exists on Metal — from low-bit-warp-run-load-shuffle-redistribute (gpu_56 — load DIRECTION and coalescing for fragment ownership, the __ldg face there is address shape, not immutability), and from sentinel-readiness-over-separate-flag (gpu_30 — cross-KERNEL readiness whose fence-free claim is scoped to platforms where remote stores arrive coherent; this is INTRA-dispatch qualification of immutable classes). Composes with barrier-on-reader-open-group-conflict-swap-overlap (this batch — the same staging pass's buffer-granular conservative half: buffer-granular for conflicts, pointer-granular for coherence). Source: lithos-ai/lithos-metal @ 236475fa716434807668562cd878ae623c2c1fd5 (Apache-2.0) -- monolith/compiler/static_fusion.py:578-580 + 596-600 + 636-660 (stage/written_buffers) + 995-1006 (merge). NotBenchable here (Metal qualifier decision content; the mechanism is a compiler transform with no shipped numbers on this axis — the nearest source numbers are M5-class provenance-only on other axes). Batch 223 / Plan 222 / Research 249 (drafts; Research 248 is the quote authority).".to_string(),
            ref_shape: "// naive: ONE coherence posture for every pointer in the dispatch -- every
// load qualified coherent, including weights nothing in the kernel writes.
// The read-only weight cache is defeated for the whole dispatch, and the
// loss scales with the immutable fraction of the working set.
fn stage_all_coherent(
    weights: &[u128],    // 200 MB of packed weights -- never written here
    kv_prefix: &[f32],   // rows [0..pos) -- never written this step
    scratch: &mut [f32], // genuinely written mid-dispatch
    pos: usize,
) -> f32 {
    let mut acc = 0.0;
    for (i, w) in weights.iter().enumerate() {
        // both reads take the COHERENT path: the hardware treats the
        // weights like racing scratch and skips its read-only cache
        acc += coherent_load(w) as f32 * coherent_load(&kv_prefix[i % pos]);
    }
    scratch[0] = acc;
    acc
}

fn coherent_load<T: Copy>(x: &T) -> T { *x } // every load: coherent path
"
            .to_string(),
            opt_pattern: "// optimized: the staging pass classifies by WRITE-SET, per POINTER.
//   immutable across the dispatch -> ORDINARY loads (weight-cache path kept):
//     packed weights | KV prefix strictly before `pos` | prior-dispatch inputs
//   everything else (scratch, cross-task intermediates, tail rows) -> coherent.
// Conflict/barrier classification stays BUFFER-granular conservative in the
// same pass: aliases at different offsets coarsen to the whole buffer name.
#[derive(Clone, Copy, PartialEq)]
enum Scope {
    Ordinary, // non-coherent: the read-only/weight-cache path serves it
    Coherent, // racing or written: full coherence
}

fn classify(provably_immutable: bool) -> Scope {
    // conservative default: only a PROVEN immutable class strips coherence
    match provably_immutable {
        true => Scope::Ordinary,
        false => Scope::Coherent,
    }
}

fn stage_write_set_scoped(
    weights: &[u128],    // ordinary: no stage writes the weight buffer
    kv_prefix: &[f32],   // ordinary: reads clamped into [0..pos); writes start AT pos
    scratch: &mut [f32], // coherent: written mid-dispatch
    pos: usize,
) -> f32 {
    let mut acc = 0.0;
    for (i, w) in weights.iter().enumerate() {
        // ordinary loads; the KV read is CLAMPED so speculation past the
        // select is legal, and the clamp keeps the read inside the prefix
        acc += ordinary_load(w) as f32 * ordinary_load(&kv_prefix[i.min(pos - 1)]);
    }
    scratch[0] = acc; // coherent store: the write set, unchanged
    acc
}

fn ordinary_load<T: Copy>(x: &T) -> T { *x } // non-coherent path

// the two views of ONE weight block take DIFFERENT qualifiers: the narrow
// scale reinterpret keeps ordinary, the activation view stays coherent.
#[test]
fn narrow_scale_view_keeps_weight_cache_path() {
    assert_eq!(classify(is_weight_block_view(SCALE_REINTERPRET)), Scope::Ordinary);
    assert_eq!(classify(is_weight_block_view(ACTIVATION_VIEW)), Scope::Coherent);
}
"
            .to_string(),
        },
        KernelRule {
            rule_id: "barrier-on-reader-open-group-conflict-swap-overlap".to_string(),
            gpu_concept: "MECHANISM (dependency minimization under whole-prefix wait semantics): in a STATIC command buffer an encoded command's barrier waits for EVERY command before it in the buffer, not just its predecessor — 'A command's barrier makes it wait for every command before it in the buffer; commands without one may start while their predecessors still run (measured: `tools/bench`, 2026-09-24 — the flag on the *reader* is what orders a dependency).' (barriers.py:3-5; Metal ICB semantics, and CUDA graph capture's dependency edges are the same shape). v0 flagged everything — 'v0 set the flag on every op.' (:5) — and the minimal pass keeps the ordering primitive but minimizes its count: measure-then-remove-switch in its purest form. THE ALGORITHM (open group), verbatim: 'an op joins the *open group* (the ops since the last barrier, all possibly running together) unless it writes a buffer the group reads or writes, or reads a buffer the group writes — buffer granularity, so a value's row views alias their whole base' (:6-8) — WAR/WAW/RAW detected at buffer granularity, conservative by construction. THE SWAP (the overlap payoff), verbatim: 'When an op conflicts with the group but not with the op just before it, the barrier goes on that previous op instead and the two run together (a gate GEMV encoded before its mixer core still overlaps the core rather than the projection it followed)' (:8-10); in code the swap is GUARDED — the group has >= 2 members, the predecessor carries no variant_group, and the pair does not conflict (barriers.py:61-64). THREE FINISHING CLAUSES: 'The predicated per-T variants of one GEMV are mutually exclusive and join without a check' (:10-11) — the variant_group axis, only one will ever run; 'the first op of the step always waits (for the previous step's tail)' (:11-12) — the cross-step tail is real; and the conservative default for unknown effects — 'The emitter records each op's written bindings in' the op's meta under the writes key; 'an op without that record is treated as writing everything it binds.' (:16-17) — unknown never joins the group. THE PAIRS THIS FREES, verbatim: 'The pairs this frees are the ones §5.12 wants overlapped — a mixer's ALU-bound core and the bus-bound GEMV of its gate projection' (:14-15) plus whatever else the lowering left independent. THE SIBLING AXIS (encode order — the folded R9 clause, a SECOND scheduling axis distinct from barrier placement): where execution follows encode order, WHICH of two groupable ops encodes first is a profile-measured decision — 'Move each sibling gate GEMV in front of the mixer core it was emitted after (profiles where the bus-bound dispatch must be encoded first for the pair to overlap).' (emit.py:1475-1476, the _bus_first pass), wired profile-CONDITIONAL off the profile's sibling_order field (emit.py:1432-1433; the config default is either of alu_first/bus_first/either, config.py:31, swept in tools/bench/model_config_tune.py:86); the barrier pass decides WHICH commands may run together, encode order decides the ORDER THEY START — compose, never merge. The original design snapshot the pin's docs point at (design.md:10-12 -> 46b2bc4) records the measured arc as provenance: the pass freed the mixer gate GEMVs and the step dropped ~3.9 percent with the core encoded first, with the gate-first order trailing within noise on the M5 Pro. THE ARENA CORROBORATION (the folded T1 clause — the SAME law from the arena pass): scratch-reuse eligibility is the identical conservatism, verbatim: 'Only a value with a known producer and a subsequent consumer is eligible. Unknown write metadata is deliberately insufficient to prove production. Values first read before being written may carry state across ICB replays.' (arena.py:16-18) — unknown write metadata never proves eligibility, exactly as it never joins the group; and a value first READ before written can carry state ACROSS REPLAYS, which is why the first-op-waits clause exists. REGIME: any static replay lane with prefix-wait barriers — Metal ICB, CUDA graph capture, pre-encoded command lists. ANTI-REGIME: dynamically-encoded streams (no static order to reason over). Distinct from captured-graph-bakes-buffer-addresses and capture-region-dispatch-only (gpu_07 — the cuda-graph family owns the replay CONTRACT: what bakes at capture and what capture records; THIS owns the dependency SET inside one capture), from concurrent-dispatch-dependency-level-barriers (gpu_18 — dependency LEVELS under a concurrent dispatch type where the encoder's implicit ordering is DROPPED and order is re-imposed per level edge; this is the inverse posture: whole-prefix ordering RETAINED and the flag COUNT minimized), and from barrier-free-per-dependency-wait-from-loop-counters (gpu_07 — host-side loop-counter thresholds, no command-buffer flags at all). Composes with coherence-scope-by-write-set-ordinary-loads-on-immutable (this batch — the same staging pass's pointer-granular coherence half) and with the folded sibling-axis clause. Source: lithos-ai/lithos-metal @ 236475fa716434807668562cd878ae623c2c1fd5 (Apache-2.0) -- monolith/compiler/barriers.py:1-74 (whole pass; its own header cites design §5.1, §5.12; issue #29 — the numbered sections resolve to the original design snapshot @ 46b2bc4 per the pin's design.md:10-12 pointer) + monolith/compiler/emit.py:1432-1433 + 1474-1481 + monolith/compiler/arena.py:13-18. NotBenchable here (ICB decision content; the source's measured arc is M5-class provenance-only, carried from the original design snapshot via the pin's pointer, never a bar on our box). Batch 223 / Plan 222 / Research 249 (drafts; Research 248 is the quote authority).".to_string(),
            ref_shape: "// naive: v0 -- a barrier before EVERY encoded op. Correct under whole-prefix
// semantics by construction, and it serializes everything: a gate GEMV
// waits for the projection that merely ENCODED before it, not the op it
// actually depends on. One reader flag per op -- all set.
struct Op {
    reads: &'static [&'static str],
    writes: &'static [&'static str],
}

fn encode_v0(n_ops: usize) -> Vec<bool> {
    vec![true; n_ops] // barrier_before: every op waits for the whole prefix
}
"
            .to_string(),
            opt_pattern: "// optimized: OPEN-GROUP placement under whole-prefix semantics. Track the
// ops since the last barrier; a new op joins unless it WAR/WAW/RAW-conflicts
// with a member (buffer-granular: a row view aliases its whole base). On
// conflict with the GROUP but not the immediate predecessor, the barrier
// moves ONTO the predecessor -- the conflicting pair runs TOGETHER instead
// of sandwiching the whole prefix.
struct Op {
    reads: &'static [&'static str],
    writes: &'static [&'static str],
    writes_known: bool,                  // false = writes everything it binds
    variant_group: Option<&'static str>, // mutually exclusive per-T variants
}

// buffer granularity: compare BASE buffer names so aliases at different
// offsets of one allocation coarsen to the whole buffer
fn conflicts(a: &Op, b: &Op) -> bool {
    let base = |s: &&str| s.split('.').next().unwrap_or(s);
    let touches = |x: &Op| x.reads.iter().chain(x.writes.iter()).map(base);
    let wr = |x: &Op| x.writes.iter().map(base);
    wr(a).any(|w| touches(b).any(|t| t == w)) || wr(b).any(|w| touches(a).any(|t| t == w))
}

// one flag per op: barrier_before[i] -- the READER's flag orders the
// dependency; ops without one may start while their predecessors still run
fn encode_open_group(ops: &[Op]) -> Vec<bool> {
    let mut barrier_before = vec![false; ops.len()];
    let mut group: Vec<usize> = Vec::new(); // the open group
    let mut open_variant: Option<&str> = None;
    for (i, op) in ops.iter().enumerate() {
        let same_variant = op.variant_group.is_some() && op.variant_group == open_variant;
        // unknown write metadata writes everything it binds: never joins
        let conflict =
            op.writes_known && !same_variant && group.iter().any(|&g| conflicts(&ops[g], op));
        if i == 0 {
            barrier_before[i] = true; // the previous step's tail is real
            group = vec![i];
        } else if conflict {
            let pred = group[group.len() - 1];
            if group.len() >= 2 && ops[pred].variant_group.is_none() && !conflicts(&ops[pred], op)
            {
                // THE SWAP: the predecessor gates the group; this op runs
                // WITH it -- the pair overlaps instead of sandwiching
                barrier_before[pred] = true;
                group = vec![pred, i];
            } else {
                barrier_before[i] = true;
                group = vec![i];
            }
        } else {
            group.push(i); // joins the open group: no barrier, may overlap
        }
        open_variant = op.variant_group;
    }
    barrier_before
}

#[test]
fn conflict_with_group_not_predecessor_swaps() {
    let ops = vec![
        Op { reads: &[\"h\"], writes: &[\"x\"], writes_known: true, variant_group: None },
        Op { reads: &[\"h\"], writes: &[\"y\"], writes_known: true, variant_group: None },
        Op { reads: &[\"x\"], writes: &[\"n\"], writes_known: true, variant_group: None },
    ];
    let flags = encode_open_group(&ops);
    // the barrier lands on the PREDECESSOR, not on the reader: the reader
    // overlaps it instead of the whole prefix serializing behind its flag
    assert_eq!(flags, vec![true, true, false]);
}
"
            .to_string(),
        },
        KernelRule {
            rule_id: "bounded-cross-worker-barrier-protocol-polling-fan-in-ladder".to_string(),
            gpu_concept: "MECHANISM (a bounded device-scope polling barrier with a fan-in ladder): cross-WORKER synchronization inside ONE persistent dispatch on GPUs WITHOUT cooperative grid sync — Metal ships none, and the pin's contract for what replaces it is verbatim: 'Inside a megakernel, threadgroup barriers protect local scratch; cross-worker publication follows Metal's device-memory ordering rules. All spin loops and task queues must be bounded.' (docs/design/design.md:77-79). FOUR fan-in variants, one per schedule, picked by worker count and topology: SERIAL — 'Fixed worker lists: prepare tasks -> state-column tasks -> gated-norm tasks.' 'A worker is a threadgroup; Metal has no API for binding one to a physical core.' 'Coherent pointers and device fences publish intermediate data across groups.' (static_barrier_serial.metal:1-3) — each worker's thread 0 scans all N arrival counters itself; SIMD — 'Bounded, coherent all-worker barrier. One SIMD group polls the arrival flags in parallel; all threads still publish/observe through device-scope fences.' (static_barrier_simd.metal:1-2) — one SIMD group's 32 lanes strided-scan the arrivals; WIDE — 'Bounded device barrier with several SIMD groups polling arrival flags. All device fences are retained. A final threadgroup broadcast makes timeout handling uniform even when a different worker reports the failure first.' (static_barrier_wide.metal:1-3) — sharded polling with a second-level fan-in; LEADER-RELEASE — 'Worker zero polls arrivals; other workers poll a single release epoch.' (static_barrier_leader.metal:1) — per-waiter polling drops from O(workers) to O(1). THE INVARIANT THAT MAKES IT CORRECT UNDER REPLAY, verbatim: 'Arrival and release epochs are monotone across stages and repeated dispatches.' (leader.metal:2) — no reset ever; the count itself is the phase, because a reset would race stragglers from the previous stage; arrivals are relaxed fetch_add plus epoch compare, and a device-scope seq_cst fence plus threadgroup barrier sits on BOTH sides of every stage (each variant opens and closes with the device-scope seq_cst fence + device/threadgroup barrier pair). THE BOUND IS THE SAFETY STORY, NOT AN OPTIMIZATION: every variant spins at most one million iterations (the ++spins == 1000000u cap appears in all four) then flips a failure flag and broadcasts it — the kernel REPORTS failure instead of hanging forever; the platform contract, verbatim: 'Cross-worker waits require a schedule whose progress conditions are satisfied. Unsupported geometry is rejected, and bounded waits report failure rather than spinning indefinitely.' (docs/design/apple-gpu.md:17-18). THE COMPILER WIRES THE FAILURE PATH INTO EVERY FUSED KERNEL: the done guard is injected at kernel entry (region_fusion.py:102-104), and 'Static projection regions do not otherwise read StepState. The region still needs its done guard and bounded-barrier error path.' (region_fusion.py:87-88) — even regions that do not logically read control state get the failure path (a spin-cap failure stores the error code then sets done through the StepState atomics, :106-109); and the binding budget is a compile-time refusal, verbatim 'fused region has no Metal binding left for StepState' (region_fusion.py:91) — a fused region whose state slot lands at the 31-slot Metal binding ceiling refuses the fusion LOUDLY rather than silently exceeding it. REGIME: persistent megakernel workers coordinating stage boundaries on Apple-class GPUs without grid sync. ANTI-REGIME: where MEASURED, in-kernel global barriers may cost no less than dispatch boundaries (see fusion-scope-by-barrier-cost-not-dispatch-count — then do not fuse; the source built and measured this protocol and still kept it OFF the hot path, in the toolbox for intra-op stealing and per-chip re-evaluation); CUDA cooperative launch has a native grid sync (use it — this protocol exists for its ABSENCE). Distinct from whole-sequence-zero-barrier-single-dispatch-scan (gpu_09 — the ZERO-barrier pole: cooperation scoped to one simdgroup so no cross-worker barrier exists at all; this is the protocol FOR the case that rule refuses), from threadgroup-scope-reduction-single-writer (gpu_65 — scopes sharing to one threadgroup so a single non-atomic store replaces cross-group sync; same direction, a finer refusal), from stripe-sms-l2-semaphore-reduce (gpu_30 — an L2 semaphore ordering ONE cross-SM reduction's accumulation; this is a full all-worker STAGE barrier with monotone epochs), from barrier-free-per-dependency-wait-from-loop-counters (gpu_07 — HOST-orchestrated waits from monotone loop counters; this is DEVICE-side polling), and from ll-flag-in-data-packed-poll plus sentinel-readiness-over-separate-flag (gpu_30 — cross-KERNEL peer polling across ranks; this is intra-dispatch worker fan-in). Source: lithos-ai/lithos-metal @ 236475fa716434807668562cd878ae623c2c1fd5 (Apache-2.0) -- kernels/common/static_barrier_serial.metal:1-21 + static_barrier_simd.metal:1-28 + static_barrier_leader.metal:1-40 + static_barrier_wide.metal:1-30 + monolith/compiler/region_fusion.py:85-109 + docs/design/apple-gpu.md:14-18 + docs/design/design.md:76-79. NotBenchable here (Metal/ICB decision content; the source's barrier-cost numbers — dispatch boundary 1.6-2.7 us vs in-kernel barrier 2.1-2.6 us static, 3.1-3.9 us in the spec-compliant fenced form, 4.7-6.1 us with stealing, parity under real block work — are M5-class provenance-only, carried from the original design snapshot via the pin's pointer; the pin's docs deliberately keep device measurements in per-chip artifacts, apple-gpu.md:3-5). RIDER (Batch 260, the MULTI-DISPATCH PASS-LOOP ARRIVAL-MERGE face, trymirai/uzu @ 56dbb91e9bd6658b397041823270f36832f9234b #991 'DFlash-TfM clean up and radix top-k race fix', MIT -- this host's fence discipline at per-row/per-partition granularity across REPEATED DISPATCHES, with NO spinning at all): a radix top-k pass kernel fans each row's partitions into ONE per-row arrival counter and the LAST arriver merges -- 'METAL_FUNC bool arrive_last(device atomic_uint* count, uint expected)' fetch_adds with 'atomic_fetch_add_explicit(count, 1u, memory_order_relaxed) == expected - 1' bracketed by 'atomic_thread_fence(mem_flags::mem_device, memory_order_seq_cst, thread_scope_device)', and the ACQUIRE-side fence is CONDITIONAL -- 'if (last)' only -- the acquire that orders the merge's reads of the other partitions' relaxed partial-histogram atomics is paid by the one threadgroup that needs it; the merge gate then re-broadcasts is_last through 'threadgroup_barrier(mem_flags::mem_threadgroup | mem_flags::mem_device);' before 'if (!is_last) return;' -- every partition's threadgroup reaches the gate barrier, only the winner continues. THE HOST SHAPE: 'for pass in 0..self.passes { self.pass.encode(...); }' -- one dispatch per pass into one command buffer, every scratch buffer (histograms/prefixes/masks/ranks/counts/done/keys) allocated ONCE before the loop, so the pass-carried state (ranks/prefixes/prefix_masks) is DEVICE memory in flight during every merge window, and the last-arriver identity can change per pass. THE FIX'S LAW (the commit's one-kernel hunk, verbatim): the merge window's handoff barrier ordered only the THREADGROUP-memory second-level prefix -- old 'threadgroup_barrier(mem_flags::mem_threadgroup); higher += simd_prefixes[lid / METAL_SIMD_SIZE];' -- while the window's data flow crosses device memory; the fix widens it to 'threadgroup_barrier(mem_flags::mem_threadgroup | mem_flags::mem_device);' AND hoists the INDEPENDENT device load above the wait -- 'uint rank = pass == 0 ? k - 1 : ranks[row];' moved ABOVE the widened barrier (the load depends on nothing the barrier orders, so the scope widening costs no added latency). LAW: a barrier ordering a threadgroup-memory handoff inside a window whose data flow crosses device memory must carry the device scope (the same conjunct bit-exact-dispatch-fusion-boundary-emulation names for fusion boundaries -- here in a multi-dispatch arrival-merge context), and an independent load hoists above the barrier it does not depend on. THE RESET FACE (this host's monotone-epoch law's counter-RESET half -- the half the host's own variants refuse): uzu's counters ARE reset per pass -- 'METAL_FUNC void reset_arrival(device atomic_uint* count)' OPENS with the same device-scope seq_cst fence before 'atomic_store_explicit(count, 0u, memory_order_relaxed)', and the call site orders the winner's merged writes 'prefixes[row] |= ulong(bucket) << shift; ... ranks[row] = rank;' THEN 'threadgroup_barrier(mem_flags::mem_device);' THEN 'reset_arrival(&done_counts[row])' -- a reused arrival counter's reset is fenced on BOTH sides (its own leading fence + the device barrier between the writes and the reset), because the next pass's partitions arrive on the reset counter. Distinct-from: loop-back-edge-scratch-war-trailing-warp-barrier (gpu_108, B253 -- threadgroup-INTERNAL WAR on a reused scratch; here cross-threadgroup DEVICE handoff) and smem-slot-role-reversal-needs-barrier (gpu_07, B57 -- threadgroup-memory slot roles; here the device-memory context), and from THIS host's own variants (they never reset -- monotone epochs -- because a reset races stragglers WITHIN one dispatch; uzu's reset is safe exactly because the pass boundary is a whole-dispatch ordering; cross the two and both break). Verified: radix_top_k_small.metal:16-27 (arrive_last + reset_arrival) + :100-155 (merge gate, the fix hunk, merged writes + device barrier + reset) + backends/metal/kernel/radix_top_k_small.rs:80-91 (the pass loop) at the pin. Batch 260 / Plan 264 / Research 296. RIDER-2 (Batch 264, the CROSS-THREADGROUP COHERENT-QUALIFIER face, trymirai/uzu @ 32cd07c3f648d65e77063f3105f36c7c85bf5d81 #994 'Radix top-k cross-threadgroup coherency', MIT -- this host's own lithos design clause 'Coherent pointers and device fences publish intermediate data across groups' gaining its upstream defect instance, ONE word in the SAME kernel file the Batch-260 face quotes): the collect kernel's cross-threadgroup plain-store key buffer shipped 'device ulong* selected_keys' and fixed to 'device coherent(device) ulong* selected_keys' (radix_top_k_small.metal:164, RadixTopKSmallCollect) -- Metal's default device address space is NON-coherent (the compiler may cache accesses with only per-threadgroup coherency), so PLAIN (non-atomic) vector stores into a buffer another threadgroup reads carry no cross-threadgroup visibility guarantee through a bare device pointer; the protocol's ATOMIC surface (arrive_last/reset_arrival, explicit memory orders + fences) already carries its own ordering, the PLAIN-STORE surface is what the qualifier exists for. LAW: a cross-threadgroup handshake is correct only when BOTH axes hold -- the pointer qualified coherent AND the ordering fences/barriers at device scope (the Batch-260 face); either alone can serve stale or unordered state. Composition, never replacement: #991 owns the ORDERING axis on the merge window, #994 owns the QUALIFIER axis on the collect buffer -- same file, same radix protocol, two independent visibility defects five days apart is the class signal. Verified: the vendor clone diff --stat (1 file, +1/-1) + both spellings read at the pin. Batch 264 / Plan 268 / Research 300. Batch 223 / Plan 222 / Research 249 (drafts; Research 248 is the quote authority).".to_string(),
            ref_shape: "// naive: cross-worker stage sync by per-worker SPIN ON EVERY PEER -- each
// of W workers polls all W arrival counters every stage. Unbounded: a
// straggler (or a lost wakeup under replay) spins FOREVER, and the reset
// needed to reuse the counters races those same stragglers.
fn stage_sync_naive(my: usize, arrivals: &mut [u64], workers: usize) -> bool {
    arrivals[my] += 1; // relaxed arrival
    loop {
        let mut ready = true;
        for a in arrivals.iter().take(workers) {
            if *a < arrivals[my] {
                ready = false; // poll EVERY peer, every stage, forever
            }
        }
        if ready {
            return true; // no cap: a lost peer hangs the dispatch
        }
    }
}
"
            .to_string(),
            opt_pattern: "// optimized: LEADER-RELEASE fan-in with monotone epochs + a spin bound.
// Worker 0 polls the arrival counters; everyone else polls ONE release
// epoch -- O(1) per waiter. Epochs only ever INCREASE (no reset: the count
// itself is the phase), and a bounded spin flips a failure flag every
// worker can see instead of hanging the dispatch.
fn stage_sync_leader_release(
    my: usize,
    arrivals: &mut [u64],
    release: &mut u64,
    failed: &mut bool,
    workers: usize,
) -> bool {
    arrivals[my] += 1; // relaxed arrival; a device-scope seq_cst fence plus
    // group barrier belong on BOTH sides of the stage in the real protocol
    let my_epoch = arrivals[my];
    if my == 0 {
        let mut spins = 0u64;
        loop {
            if arrivals[..workers].iter().all(|&a| a >= my_epoch) {
                *release = my_epoch; // ONE publish; monotone, never reset
                return true;
            }
            spins += 1;
            if spins > 1_000_000 {
                *failed = true; // report, don't hang
                return false;
            }
        }
    }
    let mut spins = 0u64;
    while *release < my_epoch {
        spins += 1;
        if spins > 1_000_000 || *failed {
            return false; // the failure broadcast reaches every waiter
        }
    }
    true
}

#[test]
fn epochs_never_reset_across_stages() {
    let mut arrivals = [0u64; 2];
    let (mut release, mut failed) = (0u64, false);
    for _ in 0..3 {
        assert!(stage_sync_leader_release(0, &mut arrivals, &mut release, &mut failed, 1));
    }
    assert_eq!(release, 3); // the count is the phase -- never reset, ever
    assert!(!failed);
}

#[test]
fn spin_bound_reports_failure_instead_of_hanging() {
    let mut arrivals = [0u64; 2];
    let (mut release, mut failed) = (0u64, false);
    // worker 1 never arrives: the leader's bounded spin must REPORT, not hang
    assert!(!stage_sync_leader_release(0, &mut arrivals, &mut release, &mut failed, 2));
    assert!(failed);
}
"
            .to_string(),
        },
        KernelRule {
            rule_id: "fusion-scope-by-barrier-cost-not-dispatch-count".to_string(),
            gpu_concept: "MECHANISM (the fusion boundary is a MEASURED cost decision, never a dispatch count): on Apple-class GPUs the dispatch boundary is cheap and global sync is polling-based, and the source priced the in-kernel alternative AT the dispatch boundary before refusing it — the pin's contract form, verbatim: 'An ICB dispatch boundary can provide ordering for whole-region dependencies. The compiler chooses between an in-kernel join and separate dispatches using the complete cost of the region. Global barriers are not assumed to be cheaper just because they occur inside one kernel.' (docs/design/apple-gpu.md:49-51). The boundary settled LAYER-WISE, verbatim: 'The compiler fuses operations when their dependencies, layouts, and storage requirements permit useful reuse. In selected GDN and full-attention configurations, one layer's mixer becomes a single megakernel; the MLP remains a separate region.' (docs/design/design.md:43-45) — never a whole-model megakernel. THREE COMPANION LAWS ride the same decision: (1) the dispatch-count proxy decouples from the traffic it proxies, verbatim 'Merely reducing the number of dispatches does not guarantee lower memory traffic: cross-worker intermediates can still require device storage.' (apple-gpu.md:40-41); (2) removing a barrier is a PROOF obligation while removing a dispatch is the timing question, verbatim 'Omitting a barrier requires dependency analysis, not a timing observation.' (apple-gpu.md:64) — and mixers.md states the negative form: 'Removing a dispatch is not sufficient evidence of a latency improvement.' (docs/design/mixers.md:104); (3) the residency/progress condition, verbatim 'The compiler does not assume that worker IDs identify physical cores or that an arbitrary number of threadgroups are resident simultaneously.' (apple-gpu.md:14-15) — a megakernel whose workers must ALL be co-resident to make progress deadlocks on a smaller GPU, so residency is a progress property to be scheduled, never an assumption; and the sharing cost of getting it wrong, verbatim: 'Each dispatch and command-buffer batch is bounded. GPU sharing and preemption vary with the device and OS, so the runtime cannot rely on immediate interruption of a long-running kernel. Cancellation stops future work after the active dispatch completes.' (apple-gpu.md:53-55) — one long non-preemptible dispatch freezes every other GPU client. THE DISCIPLINE THAT KEEPS IT HONEST: the compiler builds the UNFUSED normalized program as the control for every fused region, verbatim 'The unfused normalized program is returned as the numerical and performance control.' (region_fusion.py:4-5); regions are validated 'fusion regions must be disjoint, nonempty and uniquely named' (region_fusion.py:37); the genericity fence 'Model-specific names never enter the compiler.' (region_fusion.py:30); and the validation language, verbatim: 'A candidate is evaluated as a complete mixer or layer, including intermediate traffic, synchronization, and adjacent layout preparation. Algorithm and layout improvements must be distinguished from the effect of fusion itself.' (mixers.md:107-109). THE MEASUREMENT-PROVENANCE META-LAW: 'Device-specific measurements belong in backend configurations and benchmark artifacts; the contracts below must hold independently of one machine's timing results.' (apple-gpu.md:3-5) — the doc carries the CONTRACT (never assume), the numbers live in per-chip artifacts; the original design snapshot the pin points at (design.md:10-12 -> 46b2bc4) records the probe datum itself: in-kernel barriers priced at or above the dispatch boundary in fenced and unfenced forms, the remainder erased by real block work, and a long single dispatch stalling other GPU clients for over a second — carried as provenance, never quoted as a bar. Selection authority is the backend, verbatim: 'Fusion selection belongs to the chip backend and can depend on shapes, formats, active-row bounds, and context. An available fused implementation is not automatically the default. Native kernels remain available when their complete-region cost is better or the fusion's preconditions do not hold.' (design.md:52-54). REGIME: choosing fusion scope where dispatch overhead is small and global sync is polling-based (Apple-class; a long-dispatch sharing penalty on top). ANTI-REGIME: architectures with cheap hardware grid sync (CUDA cooperative launch) or dispatch overhead large enough to dominate — there the megakernel may win; measure, never assume either way. Distinct from epilogue-must-be-workgroup-local (gpu_03 — the OWNERSHIP legality of the same boundary: what an epilogue may read without cross-workgroup sync; this is the COST side), from grid-collapsing-mega-kernel-occupancy-trap (gpu_07 — the WebGPU trap where fusion collapses the GRID into one workgroup for lack of ANY cross-workgroup sync; this prices the boundary where a cross-worker protocol EXISTS and still loses), from fusion-payback-gated-on-per-dispatch-overhead (gpu_07 — prices the SAVINGS side, dispatches saved times per-dispatch overhead, against a fused penalty; this prices the COST side, the in-kernel barrier vs the boundary it replaces, plus the sharing penalty), and from dispatch-cost-sequential-measurement-not-single-op (gpu_07 — the measurement PROTOCOL for the per-dispatch half of that ratio; compose). Pairs with bounded-cross-worker-barrier-protocol-polling-fan-in-ladder (this batch — the protocol that prices the in-kernel alternative this rule declines). Source: lithos-ai/lithos-metal @ 236475fa716434807668562cd878ae623c2c1fd5 (Apache-2.0) -- docs/design/apple-gpu.md:3-5 + 14-15 + 40-41 + 49-55 + 64 + docs/design/design.md:41-54 + docs/design/mixers.md:104-109 + monolith/compiler/region_fusion.py:4-5 + 30 + 37. NotBenchable here (Metal/ICB decision content; the probe numbers are M5-class provenance-only and live in per-chip artifacts by the doc's own provenance law). Batch 223 / Plan 222 / Research 249 (drafts; Research 248 is the quote authority).".to_string(),
            ref_shape: "// naive: fuse the WHOLE model into one megakernel because fewer-dispatches-
// is-faster -- assumed, never measured; the in-kernel global barriers
// between layers cost what dispatches cost, and the long single dispatch
// freezes every other GPU client behind a non-preemptible kernel.
fn whole_model_megakernel(tokens: &mut [f32], layers: usize) {
    launch_persistent(|worker| {
        for layer in 0..layers {
            mixer_stage(tokens, layer, worker);
            global_barrier(); // polling fan-in: costs a dispatch boundary
        }
        mlp_stage(tokens, worker);
        global_barrier();
    });
}
"
            .to_string(),
            opt_pattern: "// optimized: the boundary is decided by the MEASURED ratio, and the
// unfused normalized program stays runnable as the built-in control --
// numerics oracle AND perf baseline for every future re-measurement.
enum Scope {
    FusedLayerWise,   // one layer's mixer per dispatch
    DispatchBoundary, // the unfused control: never deleted, only re-measured
}

fn choose_scope(barrier_us: f32, dispatch_us: f32) -> Scope {
    // in-kernel barrier >= dispatch boundary => fusing buys nothing; and
    // removing a dispatch is not sufficient evidence of a latency win --
    // measure the complete region, never the dispatch count alone.
    match barrier_us < dispatch_us {
        true => Scope::FusedLayerWise,
        false => Scope::DispatchBoundary,
    }
}

fn layer_wise_pipeline(tokens: &mut [f32], layers: usize, scope: Scope) -> Vec<f32> {
    match scope {
        Scope::FusedLayerWise => {
            for layer in 0..layers {
                dispatch_mixer(tokens, layer); // barrier = the dispatch boundary
            }
            dispatch_mlp(tokens);
        }
        Scope::DispatchBoundary => run_unfused_control(tokens, layers),
    }
    tokens.to_vec()
}

#[test]
fn control_is_the_numerical_oracle_for_the_fused_region() {
    let fused = layer_wise_pipeline(&mut real_tokens(), 64, Scope::FusedLayerWise);
    let control = layer_wise_pipeline(&mut real_tokens(), 64, Scope::DispatchBoundary);
    assert_eq!(fused, control); // fused regions validate against the control
}
"
            .to_string(),
        },
    ]
}
