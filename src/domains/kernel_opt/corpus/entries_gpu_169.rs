//! Batch 417 — the NVIDIA/cuda-rust (cuda-oxide) distill batch's
//! `kernel_opt` half (Plan 413 / Research 448). Source:
//! github.com/NVIDIA/cuda-rust @ pin `f35d6ccc2e03a54e73e3796f415f11313e439d50`
//! (Apache-2.0; the verdict pin `5e5c77b19a4b…` was one commit behind — the
//! tip moved (#1866, the proofs-preview `writes_all_elems` hardening) and
//! EVERY quote was re-verified byte-exact at the NEW pin per the B414
//! re-pin discipline; #1866 touched cuda-macros/cuda_module + lean-exporter +
//! `safe_vecadd`, NONE of this batch's quote sites). THREE rules:
//! - `hardware-work-queue-steals-pending-tiles` (scheduling) — the CLC
//!   hardware work queue as the THIRD stealing mechanism (no software
//!   counter at all) + the single-lane async-response ownership rider.
//! - `launch-invariant-guard-checks-once-at-dispatch` (host-orchestration)
//!   — the launch-contract three-way placement law for a kernel safety
//!   obligation.
//! - `pre-arrive-empty-side-barriers-of-double-buffered-pipeline` (sync/init)
//!   — the init-phase asymmetry: empty-side barriers start ARRIVED.
//!
//! The batch's `rust_perf` member (`bench-ratio-omitted-when-baseline-not-live`)
//! lands in `rust_perf` `entries_105` (that family's home — the Batch-413
//! precedent). The batch's count-neutral faces route to their owners at the
//! landing: K1's L2-rasterization FACE pointer rides `entries_popcorn`'
//! owned swizzled-L2 rule (this file carries no owner edit — the pointer
//! lives in K1's notes).
//!
//! Fixtures: `corpus/fixtures/b417_fixtures.rs` — standalone-compiled AND
//! executed BEFORE entry (debug AND -O, the four-arm discipline).
//! NOTHING INVENTED — every quote read at the clone at the pin this session.
#![allow(dead_code)]

use crate::domains::kernel_opt::KernelRule;

/// Batch 417 — the cuda-rust kernel rules.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── K1 — hardware work-queue stealing: the pending set lives in
        // hardware, the steal is one instruction, and the async response
        // has ONE owning lane. ──
        KernelRule {
            rule_id: "hardware-work-queue-steals-pending-tiles".to_string(),
            gpu_concept: "MECHANISM (hardware work-queue stealing; category — scheduling, the \
stealing family's third mechanism): a persistent tile loop's pending work set moves OUT of \
software entirely — instead of a global `atomicAdd` fetch-next-tile counter, the launch is a \
FULL grid (1 CTA per tile) and a finished CTA issues `clc_try_cancel`; the hardware pending \
queue hands back a canceled cluster's first ctaid and the thief serially processes ALL \
cluster-size tiles from that one steal. THE SOURCE, verbatim at the pin (NVIDIA/cuda-rust \
cuda-oxide/crates/rustc-codegen-cuda/examples/gemm_sol/src/main.rs, Phase 4B): 'steal \
pending work via `clc_try_cancel` instead of `atomicAdd` on a global counter' + 'No \
`tile_counter` parameter: hardware manages the pending queue, zero contention.' + \
'Cluster-aware stealing: `clc_try_cancel` returns the first ctaid of a stolen cluster. \
Each CTA serially processes all `CLUSTER_SIZE` tiles from that cluster.' THE RIDER \
(load-bearing, generalizes past CLC — SINGLE-LANE ASYNC-RESPONSE OWNERSHIP): the steal's \
response arrives through the ASYNC proxy, so exactly ONE lane owns the whole \
request/decode cycle and the result is broadcast — 'CLC writes the response through the \
async proxy. Keep all response access in lane 0, then broadcast registers to the warp so \
no lane can observe stale shared memory' (body: lane 0 fences, arms the 16-byte-response \
mbarrier with expect_tx, issues clc_try_cancel, spins mbarrier_try_wait_parity, decodes \
`clc_query_is_canceled` / `clc_query_get_first_ctaid_x`, re-fences; then \
`warp::shuffle_sync` broadcasts both words so the branch is warp-uniform). The in-source \
name: 'Single-lane response ownership: lane 0 waits for and decodes the async CLC \
response, then broadcasts the result to its warp so every lane takes the same path.' \
ADJUDICATION (compose-not-merge, the stealing owners read both directions at the landing \
HEAD): chunk-downsize-work-stealing-microtile-floor (B187, gpu_55) owns GRAIN selection \
(halve-while-underfilled, microtile floor) + thread-id pre-assignment + a PLAIN shared-\
counter fetch_add steal on the CPU threadpool — software counter, host side; \
own-slice-coprime-stride-steal-claimant-invariant (B223, gpu_86) owns per-slice DEVICE \
cursors + the coprime-stride victim fan-out + the claimant-invariance fence elision + the \
measured enable gate — still software atomics, one per slice. THIS rule owns the axis \
BOTH leave open: the pending set lives in HARDWARE (zero counter traffic, zero shared-\
state contention — the counter the other two rules optimize AROUND is deleted, not tuned), \
and the steal is one instruction whose RESPONSE is async. Compose: B187 picks the grain, \
B223 fans the victims, THIS removes the counter they both contend on — the three \
mechanisms stack (grain -> victim topology -> queue substrate), never merge. THE \
L2-RASTERIZATION FACE (pointer, not a rule): CLC's 'Column-major tile rasterization: \
linear ctaid maps to (row, col) for L2 locality' is a positive instance of the OWNED \
swizzled-L2 tile-mapping rule in entries_popcorn (three formula shapes + the group-width \
derivation + the operand-footprint order rider) — record the instance there, never \
re-own the axis. REGIME: sm_100a persistent tile loops (Blackwell CLC) — the anti-regime \
is every older arch, where the software counters of B187/B223 remain CORRECT and CLC does \
not exist; the second anti-regime is any steal whose consumer reduces per-thief state \
(claimant-DEPENDENT outputs — B223's fence law still binds). FOREIGN-SILICON DISCLOSURE \
(standing): the win metric is device-side contention on sm_100a hardware — NotBenchable \
by domain (the dial9/136 annotation, never a demotion; our boxes are sm_89 + Metal). The \
async-response ownership half is the TRANSFERABLE half: any per-worker async steal/alloc/\
queue response (host threadpools included) has one owning waiter + a broadcast, never \
every lane spinning on shared response memory. Distinct from gpu_30's cross-PEER sentinel \
protocol (that is cross-KERNEL readiness polling; THIS is intra-grid work distribution), \
from own-slice-coprime-stride (B223 — software cursors + visit order; THIS is the \
counter-free substrate beneath both), and from the B187 grain law (what a steal CARRIES, \
not where the queue lives). Source: NVIDIA/cuda-rust @ f35d6ccc2e03a54e73e3796f415f11313e439d50 \
(Apache-2.0) — gemm_sol/src/main.rs:2011-2055 (the Phase-4B doc: the full-grid + \
cluster-steal + lane-0-ownership + column-major + zero-contention clauses), :2251-2278 \
(the body: lane-0 fence/arm/issue/spin/decode/re-fence + the shuffle_sync broadcast), \
:51 (the clc_* intrinsic imports). Quotes byte-verified at the clone at the pin this \
session. Batch 417 / Plan 413 / Research 448. BATCH-425 FACE (the TK 2.0 PERSISTENT-GRID DELIVERY face, HazyResearch/ThunderKittens @ 67845f5fa48d MIT, Plan 419 / Research 453 — the second CLC surface, compose never merge): the owner owns the CANCEL-STEAL arm (a full grid where finished CTAs cancel pending work out of the hardware queue); ThunderKittens ships the PERSISTENT-DELIVERY arm of the same hardware queue — a persistent grid where the hardware HANDS each CTA its next task through clc.schedule/clc.query with a depth-1 query pipeline (CLC_PIPE_DEPTH, schedule_arrived/finished mbarriers, bf16_b200_gemm.cu:88,117-181), one issuer per cluster (cta_rank == 0's warp 2), the success-bit terminating every role's loop, and the queried tile overlapped with the current task's compute (the query pipeline is the schedule-side analog of the load ring). The pending set lives in hardware in BOTH arms — the family law is unchanged; the face records the delivery surface so a reviewer of a persistent CLC kernel finds the corpus under the steal rule."
                .to_string(),
            ref_shape: r"// NAIVE: a global atomic counter hands out tiles; every
// finished worker contends on it, and the handoff value is read
// by every thread through shared memory.
static TILE_COUNTER: AtomicU32 = AtomicU32::new(0);

fn persistent_loop(tid: usize, nthreads: usize, tiles: usize) {
    loop {
        // every worker: one contended RMW per steal
        let t = TILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        if t >= tiles { break; }
        process(t);
    }
}

// the async-steal defect cousin: every lane polls the response slot
// and some lane can read it BEFORE the writer's fence lands.
let stolen = SHARED_RESPONSE.load(Ordering::Relaxed);
if stolen != INVALID { process_cluster(stolen); }
"
            .to_string(),
            opt_pattern: r"// BOUND: the pending set lives in hardware — the launch is a
// full grid (1 CTA per tile) and a finished CTA STEALS with one
// instruction; the async response has ONE owning lane + a broadcast.
fn steal_loop(lane: usize, cluster_size: usize) -> StealOutcome {
    if lane == 0 {
        // lane 0 owns the whole async cycle: fence, arm, issue, spin, decode
        fence_async_proxy();
        mbarrier_arrive_expect_tx(&CLC_BAR, 1, 16);
        clc_try_cancel(RESP_PTR, &mut CLC_BAR);
        while !mbarrier_try_wait_parity(&CLC_BAR, parity) {}
        let canceled = clc_query_is_canceled(RESP_LO, RESP_HI);
        let first = clc_query_get_first_ctaid_x(RESP_LO, RESP_HI);
        fence_async_proxy();
        (canceled, first) // decoded locally
    } else {
        (0, 0) // never touches the response memory
    }
}

fn apply_steal((canceled, first): (u32, u32), lane: usize, cluster_size: usize) {
    // ONE broadcast, both words: the branch is warp-uniform, no lane
    // ever reads the shared response slot.
    let (c, f) = (shuffle_sync(canceled, 0), shuffle_sync(first, 0));
    if c != 0 {
        for ci in 0..cluster_size {
            process(first + ci); // serial over the stolen cluster
        }
    }
}
"
            .to_string(),
        },
        // ── K2 — the launch-contract placement law: a kernel safety
        // obligation goes where it is checkable ONCE. ──
        KernelRule {
            rule_id: "launch-invariant-guard-checks-once-at-dispatch".to_string(),
            gpu_concept: "MECHANISM (guard placement; category — host-orchestration, the \
launch-contract split): a kernel safety obligation has THREE lawful homes, named by what \
can CHECK it — (a) a host-CHECKABLE precondition (a relation over buffer lengths, dims, \
scalars — everything computable from the launch arguments alone) belongs at the DISPATCH \
site, evaluated ONCE per launch and refusing with a typed error naming the failed \
relation; (b) a device-PROVABLE property (in-bounds access GIVEN the precondition, \
disjoint writes, barrier uniformity) belongs in the body/verify clause; (c) a per-thread \
guard whose condition is LAUNCH-INVARIANT (it reads only lengths and dims, never device \
data) is redundant work — the dispatch site already owns it, and N threads re-derive it \
per launch for nothing. THE SOURCE, verbatim at the pin (NVIDIA/cuda-rust \
cuda-oxide/crates/cuda-macros/src/lib.rs, the `#[launch_contract(requires = ...)]` doc): \
'`requires = (relation, ...)` writes down, next to the kernel, the size relationships its \
buffers and scalars must satisfy for every access to be in bounds. The generated launcher \
checks them once per launch, on the CPU, and refuses to launch (with a typed error naming \
the failed relation) instead of letting an undersized buffer fault mid-kernel' + 'Every \
checked launcher ... evaluates every relation once at launch time, before any argument is \
handed to the driver.' The grammar law that makes the hoist SOUND (the tiny-grammar \
clause, verbatim): 'Every identifier is validated against the kernel's actual parameter \
list, so a typo is a compile error instead of a check against the wrong value. And every \
`+`/`-`/`*` compiles to checked `u64` arithmetic, so a huge `m * k` cannot wrap around to \
a small number and falsely pass.' (the wrap case is the hoist's own trap: a 32-bit \
`m * k` overflow would pass a wrong precondition the in-body guard would have caught — \
hoist WITH checked widening or not at all). THE GENERIC RUST FIX-SPACE (without \
cuda-oxide): move the kernel-body guard keyed only on launch parameters (lengths, dims — \
computable host-side) to the dispatch call site; keep body guards for device-data-\
dependent conditions. REGIME: kernels whose bodies re-derive per-thread invariants the \
launcher already enforces. ANTI-REGIMES: guards on device-side data (must stay in-body — \
the host cannot see it); relations needing per-thread values; and a launch whose only \
caller is a single checked site that already validates (the hoist is a no-op there). \
ADJACENT-DISTINCT (read both directions at the landing HEAD): nodecode-arm-bounds-\
overhead-elimination (B44, gpu_44) is the MEASUREMENT methodology that prices removing a \
body obligation entirely (a stripped variant bounds every arm that keeps it functioning) \
— K2 is the PLACEMENT decision upstream of any variant: first decide WHERE the obligation \
lives, then B44 prices whether it lives at all; uniform-exit (gpu_127) is the device-side \
skip-and-zero CONTRACT for out-of-range work — the body half of the split (K2's clause \
(b)); captured-graph-init-unconditional-and-stream-ordered (B22, gpu_22) is the capture-\
time work-omission correctness law — a guard evaluated at the WRONG TIME (capture, once) \
omitting work needed under other data, where K2 is a guard evaluated at the RIGHT time \
(dispatch, once per launch) for a condition that cannot change between launches of the \
same geometry. Grep evidence (3+ vocabularies): 'launch invariant|hoist.*dispatch|launch \
contract|guard placement' over the corpus — the hits are the tuned-table rules \
(entries_gpu_34 config tables, a different axis) and the three adjacents above; ZERO \
owner of the placement law. NotBenchable-by-domain (the win metric is device-side \
per-thread work eliminated on foreign silicon; the host-side half is a one-time-per-launch \
check every SM thread stops paying). Source: NVIDIA/cuda-rust @ \
f35d6ccc2e03a54e73e3796f415f11313e439d50 (Apache-2.0) — \
cuda-oxide/crates/cuda-macros/src/lib.rs:691-729 (the requires contract: once-per-launch, \
CPU-side, typed refusal, the tiny grammar, checked u64 arithmetic) + :718-722 (every \
relation once at launch time before any argument reaches the driver). Quotes \
byte-verified at the clone at the pin this session. Batch 417 / Plan 413 / Research 448."
                .to_string(),
            ref_shape: r"// NAIVE: the launch precondition re-derived per thread,
// in the body, where every one of N threads pays for it — and the
// same relation is ALSO what the dispatcher should have refused.
fn dispatch_naive(a: &[f32], c: &mut [f32], m: usize, k: usize) {
    // no host-side check: an undersized `c` faults MID-KERNEL
    launch_kernel(a.as_ptr(), c.as_mut_ptr(), m, k);
}

#[kernel]
unsafe fn body_naive(a: *const f32, c: *mut f32, m: u32, k: u32) {
    let idx = thread_index();
    // per-thread re-derivation of a LAUNCH-INVARIANT condition:
    // m, k, and the lengths never change between threads
    if (idx as u64) < (m as u64) * (k as u64)
        && (m as u64) * (k as u64) <= C_LEN
    {
        c[idx] = a[idx] * 2.0;
    }
}
"
            .to_string(),
            opt_pattern: r#"// BOUND: the launch-invariant relation is checked ONCE at
// dispatch (checked u64 arithmetic — a wrap cannot falsely pass);
// the body keeps only the device-provable clause.
fn dispatch_checked(a: &[f32], c: &mut [f32], m: usize, k: usize) -> Result<(), LaunchError> {
    // every relation ONCE, widened to u64 with checked arithmetic:
    // a false relation refuses WITH THE RELATION NAMED
    let need = (m as u64)
        .checked_mul(k as u64)
        .ok_or(LaunchError::SizeRequirementOverflow("m * k"))?;
    if (c.len() as u64) < need {
        return Err(LaunchError::SizeRequirementViolated {
            relation: "c.len() >= m * k",
            lhs: c.len() as u64,
            rhs: need,
        });
    }
    launch_kernel(a.as_ptr(), c.as_mut_ptr(), m, k);
    Ok(())
}

#[kernel]
unsafe fn body_checked(a: *const f32, c: *mut f32, m: u32, k: u32) {
    let idx = thread_index();
    // the body keeps ONLY what the dispatch could not check:
    // in-bounds GIVEN the precondition is the device-provable half
    if (idx as u64) < (m as u64) * (k as u64) {
        c[idx] = a[idx] * 2.0;
    }
}
"#
            .to_string(),
        },
        // ── K4 — the init-phase asymmetry: an empty-side barrier starts
        // ARRIVED, or the first fill stalls on a release that never was. ──
        KernelRule {
            rule_id: "pre-arrive-empty-side-barriers-of-double-buffered-pipeline".to_string(),
            gpu_concept: "MECHANISM (barrier init-phase asymmetry; category — sync/init, the \
mbarrier family): a double-buffered produce/consume pipeline's barrier PAIR carries an \
asymmetric init contract — the FULL-side barriers start UN-ARRIVED (the data genuinely \
absent: the consumer must wait for a real production) while the EMPTY-side barriers start \
PRE-SIGNALED (the buffers genuinely free: the producer's first fill must not be gated on \
a consumer release that has not happened yet). THE SOURCE, verbatim at the pin \
(NVIDIA/cuda-rust cuda-oxide/crates/rustc-codegen-cuda/examples/gemm_sol/src/main.rs, \
Phase 3 warp-specialized): 'Pre-signal MMA_BAR0/1 so the producer can start filling \
buffers immediately without waiting for a consumer release.' (body: `if tid == 0 { \
mbarrier_arrive(&raw const MMA_BAR0); mbarrier_arrive(&raw const MMA_BAR1); }` — tid 0 \
arrives BOTH empty barriers between the init fence and the first sync). The defect it \
kills: a naive init leaves BOTH sides un-arrived, so the producer's first \
empty-barrier wait spins on phase parity no consumer will set until AFTER its first \
full-barrier wait — which waits on the producer. A deadlock-shaped stall (or, in \
best-effort variants, a lost first-fill overlap window: the pipeline's startup latency \
is one full stage instead of zero). THE ONE-LINE FIX SHAPE: at init, arrive once on \
every empty-side barrier (the arriving thread is the ONE designated init thread after \
the init fence, before the first sync); full-side barriers are never pre-arrived. \
REGIME: any double-buffered produce/consume pipeline over fixed stage pairs (mbarrier \
on CUDA; event/semaphore pairs on other stacks). ANTI-REGIMES: single-buffered (one \
barrier pair — no asymmetry to encode); pipelines whose first phase is genuinely \
data-gated (the producer must wait for real upstream data on tick 0 — pre-arriving \
would let it read garbage); barriers whose arrival count > 1 at init (pre-arrival must \
complete the FULL expected arrival count or use expect_tx to keep the phase arithmetic \
exact). ADJUDICATION (the mbarrier owners, read at the landing HEAD): \
mbarrier-staged-in-structural-padding (B106) owns the SIZING trick (hiding barrier \
bytes in padding); gpu_141 owns the DECLARATION-ORDER fragmentation axis (barriers \
declared before slot data so the free walk leaves one run); gpu_36 owns the COMPLETION \
lifecycle (pre-arm expect_tx for the IN-FLIGHT transfer's byte count before issuing \
it — arming a barrier for THIS stage's data, not phase-priming the EMPTY side for the \
NEXT fill); the TMA/mma stage-pipeline rules own the steady-state wait/arrive protocol. \
ZERO owner of the INIT-PHASE axis (grep 'pre-signal|pre-arriv|empty barrier|initial \
phase' over the corpus — gpu_141's 'empty barrier' mentions are the declaration-order \
rule's own vocabulary, not init phase). The asymmetry is small, bounded, one init \
block — landed as a RULE at the verdict's C+ grade with the landing read recorded: the \
fix shape is one arrive per empty barrier, the defect is a startup stall, and the \
anti-regimes are named; the coordinator may still fold it to a face on the \
stage-pipeline family if the falsifier finds the stall unrepresentable in fixtures. \
NotBenchable-by-domain (the win is device-side startup overlap on foreign silicon; \
the defect shape IS CPU-testable and the fixtures pin it). Source: NVIDIA/cuda-rust @ \
f35d6ccc2e03a54e73e3796f415f11313e439d50 (Apache-2.0) — \
gemm_sol/src/main.rs:1201-1218 (the Phase-3 init block: barrier init, the init fence, \
the pre-signal arrives, the first sync). Quotes byte-verified at the clone at the pin \
this session. Batch 417 / Plan 413 / Research 448."
                .to_string(),
            ref_shape: r"// NAIVE: both barrier sides start UN-ARRIVED — the
// producer's first fill waits on an empty-side release that no
// consumer will send until after its own first full-side wait.
fn init_naive(barriers: &mut Pipeline) {
    mbarrier_init(&mut barriers.full0, 1);
    mbarrier_init(&mut barriers.full1, 1);
    mbarrier_init(&mut barriers.empty0, 1);
    mbarrier_init(&mut barriers.empty1, 1);
    fence_init_release();
    sync_threads();
    // producer, tick 0: waits empty0 for parity no one set —
    // the consumer cannot arrive until it has consumed, and it
    // cannot consume until the producer filled. Startup stalls.
    mbarrier_wait(&barriers.empty0, 0);
}
"
            .to_string(),
            opt_pattern: r"// BOUND: the EMPTY side starts ARRIVED (the buffers are
// genuinely free); the FULL side starts un-arrived (data genuinely
// absent). One designated thread pre-arrives between the init
// fence and the first sync.
fn init_bound(barriers: &mut Pipeline, tid: usize) {
    if tid == 0 {
        mbarrier_init(&mut barriers.full0, 1);
        mbarrier_init(&mut barriers.full1, 1);
        mbarrier_init(&mut barriers.empty0, 1);
        mbarrier_init(&mut barriers.empty1, 1);
    }
    fence_init_release();
    sync_threads();
    // the pre-signal: empty means free — arrive BOTH once, so the
    // producer's first wait passes immediately; full stays closed.
    if tid == 0 {
        mbarrier_arrive(&barriers.empty0);
        mbarrier_arrive(&barriers.empty1);
    }
    sync_threads();
    // producer, tick 0: the first fill starts now, no stall.
    mbarrier_wait(&barriers.empty0, 0);
}
"
            .to_string(),
        },
    ]
}
