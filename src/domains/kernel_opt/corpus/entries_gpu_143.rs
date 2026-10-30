//! Entries slice 143 — Batch 319 (Plan 319): the 10-30 staged `kernel_opt`
//! pool (zed-m3-watch1030b rows, re-verified at the pins by zed-m3-b319),
//! converted kernel_opt-only under the B293 cross-domain precedent
//! (`rust_perf`/`sec` counts ZERO this batch; the `rust_perf` slots are the
//! live claims B301/plan-306 + B306/plan-310). Sources: tracel-ai/cubecl @
//! c1d0125 (#1993 tip; the Metal-native graph-capture series #1962 via
//! #1975 `07cd07a`, and the SMEM loop-reuse `66ec5f4d` — the latter lands
//! count-neutral as a rider face on its family home `gpu_94`) +
//! ashhart/tensorfold @ 2dffc954 (+ 913bddb4 the launch-record commit +
//! 086e8a28 the contract battery) + vllm-project/vllm @ 914d61fc5 (#64425).
//! Apache-2.0 OR MIT per repo. T0 quotes byte-verified at the pins (plan
//! 319, 2026-10-30): the squash body at c1d0125, capture.rs:68-85 +
//! stream.rs:187-208 at the same tree, the TF kernel at 2dffc954's
//! `glm_glue_rows.cuh` diff + the contract test at 086e8a28, the vllm diff
//! at 914d61fc5. The fixtures are ORIGINAL minimal Rust reproductions —
//! nothing transcribed from the sources (Rust/Metal, Zig/CUDA,
//! Python/CUDA); standalone-compiled AND executed 4/4 BEFORE entry
//! (`rustc --edition 2021 --test`, three en-route fixture defects fixed
//! and disclosed in plan 319). Not benchable: the 10→23 ms Metal figure,
//! the 5.6 ms TP=3 verify row, and the 272-token GB300 crossover are all
//! source-reported provenance; the degradation model, the committed-since-
//! open nesting, the pick-parity + f64-oracle contract, and the overlap
//! gate chain are CPU-testable and ARE the fixtures. Category membership
//! lives ONLY in `KERNEL_CATEGORIES` (mod.rs) — never encode it here.

use super::KernelRule;

/// Data table, not logic -- grows with distillation batches.
#[allow(clippy::too_many_lines)]
fn slice_b319_staged_pool() -> Vec<KernelRule> {
    vec![
        // ── metal-icb-replay-rewrite-private-copy ──
        // Rule 1 (category D — dispatch/command buffers). cubecl #1962
        // (Metal native graph capture) via #1975/#1993. THREE atoms in one
        // mechanism: (a) the driver-side degradation — an unchanged ICB
        // runs slower the more often it is re-run (10 → 23 ms over 20
        // replays; the same commands encoded directly stayed at 10 ms);
        // (b) the fix — a replay COPIES the encoded commands into a
        // PRIVATE indirect command buffer (a blit of the commands alone, a
        // few microseconds) and runs that one, keeping every replay at the
        // first one's speed; (c) the rewrite ordering — the next replay
        // rewrites the private buffer while the last may still be running
        // it, so a replay on another stream waits on the last replay's
        // event first.
        KernelRule {
            rule_id: "metal-icb-replay-rewrite-private-copy".to_string(),
            gpu_concept: "MECHANISM (a replayed command buffer is copied to a private copy before every replay, because the driver degrades an unchanged one; category D — dispatch/command buffers): a graph captured once into an indirect command buffer and replayed per step pays a DRIVER tax that grows with the replay COUNT, not with the work — the degradation is invisible in any single replay and compounds in steady-state serving. THE SOURCE LAW, verbatim (commit 'Metal native: rewrite a graph's commands before each replay', via the #1993 squash body): 'Apple's driver runs an indirect command buffer slower the more often it runs it unchanged: a decode step went from 10 to 23 ms of GPU time over 20 replays, while the same commands encoded directly stayed at 10 ms. A replay now copies the encoded commands into a private indirect command buffer and runs that one, which keeps every replay at the first one's speed.' THE CODE ANCHORS, verbatim (crates/cubecl-metal/src/compute/capture.rs at c1d0125): ':71 A replay first copies the encoded commands over the ones it runs. Apple's driver runs an indirect command buffer slower the more often it runs it over 20 replays — and runs one rewritten before each replay at its first speed. The copy is a blit of the commands alone, a few microseconds.'; the field ':79 Rewritten from `encoded` by every replay, then run.' over `replayed: Retained<ProtocolObject<dyn MTLIndirectCommandBuffer>>`. THE ORDERING CLAUSE, verbatim (capture.rs :81-85): 'Signalled once the last replay has run. A replay on another stream waits on it: Metal orders work on one queue only, and the next replay rewrites `replayed` while the last may still be running it' — the private copy makes the rewrite-vs-run race POSSIBLE, and the per-graph `last_replay: Cell<Option<MetalEvent>>` event is what closes it: cross-stream replays fence on the event before rewriting, same-stream replays order naturally through the shared queue. DECISION CONTENT: (1) the degradation axis is REPLAY COUNT of an UNCHANGED buffer — re-encoding every replay is the expensive alternative the design rejects (the encode-direct control stayed at 10 ms, but encoding is the cost the graph exists to avoid); the cheap middle is BLIT-COPY the commands (a few microseconds, the commands alone — never the bindings or the heap) into a buffer the driver has never run before; (2) per-replay freshness is the whole fix: a buffer identity the driver has run before carries its history, so the private target must be rewritten (copied over) before EVERY replay, not once; (3) the copy creates the rewrite/run race the shared encoding never had — the previous replay may still be executing the bytes this replay overwrites; the fence is an EVENT on the last replay, waited only by cross-stream replays (Metal orders same-queue work by itself); (4) the header doc frames the shape (capture.rs :2-3): the graph is 'encoded once into an indirect command buffer when it closes, and replayed with a single call' — the copy rides the replay path, the capture path is untouched. MEASURED (source-reported, never an adoption bar): 10 → 23 ms over 20 replays on the degraded arm; every replay at the first one's speed after the fix; the blit 'a few microseconds'. HOME-RELEVANCE: the corpus's own serving lane replays static graphs per decode step (the gpu-resident control loop runs under a static ICB on Metal) — this tax lands exactly there, invisible per-step, compounding per-token. REGIME: static command buffers / CUDA-graph-class capture-replay serving loops on drivers that cache per-buffer state (Metal ICB confirmed); replay counts in the tens-plus per buffer identity; work small enough that a few-microsecond blit is noise against the degradation. ANTI-REGIME: backends without the degradation (a fresh buffer is wasted work — measure or gate on the backend fact); graphs re-encoded anyway (nothing static to copy); replay counts so low the compounded tax never exceeds one blit; buffers whose commands change between replays (the rewrite is then the normal path, no degradation to dodge). KINSHIP (do not double-count): gpu-resident-control-loop-static-icb-replay (B223, D) owns the CONTROL-LOOP PLACEMENT — the whole per-token control plane on the GPU under a static replay, dynamic control by data-dependent early-return, never a re-encode; THIS owns the replay DEGRADATION the same static replay hits and its per-replay private-copy remedy — compose: that rule says WHICH buffer replays; this says REWRITE WHAT IT RUNS before each one; graph-replay-skips-host-node-scans (B295, D) owns the HOST-side scan skip at replay; captured-graph-init-unconditional-and-stream-ordered (D) owns capture-time init ordering — neither touches per-replay driver degradation; the B310 barrier/slot rules own SMEM allocation runs, not command buffers; entries_gpu_86's `last_replay`-shaped event kin is the cross-stream fence discipline (the producer-store-evict-last-pins-cross-stream-consumer rule owns cross-stream lifetime pinning — different axis: memory lifetime vs command rewrite ordering). GREP EVIDENCE (over the kernel_opt corpus, 3+ vocabularies): 'indirect command buffer|ICB replay' — gpu-resident-control-loop-static-icb-replay (the placement sense) + entries_gpu_134 tests fixture ('Values first read before being written may carry state across ICB replays', the correctness sense) + table-reached-buffers-declare-residency (the residency sense) — no degradation/rewrite owner; 'replay' rule names — gdn-transition-replay-log-commit-many, lazy-rollback-journaled-window-replay-replaces-eager-state-snapshots, segmented-checkpoint-cheap-replay-backward, graph-replay-skips-host-node-scans — all replay-SEMANTICS, none replay-PERFORMANCE; 'rewrite|copy.*before.*run|private.*buffer' — zero owners. NotBenchable here (the 10→23 ms is Metal-driver, source-reported; the degradation model + the fresh-identity cost model + the cross-stream fence are CPU-testable and ARE the fixtures). Source: tracel-ai/cubecl @ c1d0125 (#1993 tip; the Metal-native series #1962 via #1975 07cd07a) — crates/cubecl-metal/src/compute/capture.rs :2-3,65-85 + the squash body paragraph, Apache-2.0 OR MIT. Batch 319 / Plan 319. Every quote verbatim at the pin.".to_string(),
            ref_shape: r"// ref_shape (rule: metal-icb-replay-rewrite-private-copy):
// the naive replay: every step re-runs the SAME encoded buffer. A
// driver that degrades an unchanged buffer per prior run compounds
// the tax across replays — invisible per step, doubling over ~20.
#![allow(dead_code)]

#[derive(Clone)]
struct Commands {
    ops: Vec<u64>,
}

struct Driver {
    runs_of_record: std::collections::HashMap<usize, u32>,
}

impl Driver {
    fn new() -> Self {
        Self { runs_of_record: std::collections::HashMap::new() }
    }
    /// The degradation: first run 10 units, each unchanged re-run of
    /// the same buffer id ~0.65 units slower (10 -> ~22.7 over 20).
    fn run(&mut self, id: usize, _cmds: &Commands) -> u64 {
        let n = self.runs_of_record.entry(id).or_insert(0);
        let cost = 10 + (*n as u64 * 13) / 20;
        *n += 1;
        cost
    }
}

/// every replay re-runs buffer 0: the cost compounds on ITS history
fn replay_naive(drv: &mut Driver, cmds: &Commands, replays: usize) -> Vec<u64> {
    (0..replays).map(|_| drv.run(0, cmds)).collect()
}
".to_string(),
            opt_pattern: r"// opt_pattern (rule: metal-icb-replay-rewrite-private-copy):
// the fix: a replay first COPIES the encoded commands into a private
// buffer (the blit, constant cost) and runs THAT one — a fresh buffer
// identity every replay, so every replay runs at first-run speed. A
// replay on another stream waits on the last replay's event before
// rewriting (the rewrite may not race a live run).
#![allow(dead_code)]

fn replay_private_copy(
    drv: &mut Driver,
    cmds: &Commands,
    replays: usize,
    blit_cost: u64,
) -> Vec<u64> {
    let mut out = Vec::with_capacity(replays);
    for r in 0..replays {
        let private = cmds.clone(); // the rewrite: copy before each replay
        // a FRESH buffer identity every replay — the degradation is
        // per-buffer, so each replay runs at first-run speed
        out.push(drv.run(1_000_000 + r, &private) + blit_cost);
    }
    out
}

// the ordering clause, as data: the event the last replay signals;
// a replay on another stream waits on it BEFORE rewriting `replayed`
// (same-stream replays order through the shared queue by itself).
struct ReplayFence {
    last_replay_signalled: bool,
}

impl ReplayFence {
    fn may_rewrite(&self, same_stream: bool) -> bool {
        same_stream || self.last_replay_signalled
    }
}
".to_string(),
        },

        // ── profile-window-nests-committed-since-open ──
        // Rule 2 (category D — dispatch/measurement). cubecl #1962 series
        // ("Metal native: profile windows nest") via #1975/#1993. THREE
        // atoms: (a) the blindness — one profiling collector per stream;
        // an inner start_profile REPLACES the outer window's collector and
        // the outer measures nothing; (b) the instantiation — that is
        // exactly an autotune round under per-launch profiling: every
        // candidate's sample reads zero, the round measures none, the
        // first candidate runs; (c) the fix — each window records where in
        // the stream's committed-log it opened and measures every
        // work-bearing command buffer committed since.
        KernelRule {
            rule_id: "profile-window-nests-committed-since-open".to_string(),
            gpu_concept: "MECHANISM (profiling windows nest by measuring the command buffers committed since each opened, never a per-stream collector slot; category D — dispatch/measurement): a GPU profile window is only as honest as WHAT it samples — a stream that keeps ONE profiling collector lets an inner window REPLACE the outer one's, and the outer measures nothing. THE SOURCE LAW, verbatim (commit 'Metal native: profile windows nest', via the #1993 squash body): 'A stream kept one profiling collector, so an inner start_profile replaced the outer window's and the outer one measured nothing. That is the shape of an autotune round run under per-launch profiling: every candidate's sample read zero, the round measured none, and the first candidate ran. Each window now reads the command buffers committed since it opened, and the nested-window runtime test runs on Metal.' THE CODE ANCHORS, verbatim (crates/cubecl-metal/src/compute/stream.rs at c1d0125): ':187 The profile windows open on a stream, and the work-bearing command buffers committed since the first of them opened, whose GPU timestamps a window...'; ':191 Windows nest: each measures every command buffer committed from its opening on, so an inner window's work is also the outer's. An autotune round's window holds the per-launch windows of an instrumented run, and the...'; the state ':197 Each open window's token, and where in `committed` it opened.' over `committed: Vec<Retained<ProtocolObject<dyn MTLCommandBuffer>>>`; the open arm ':205 Open the window `token` names at this point of the stream.' — `self.open.push((token.id, self.committed.len()));`; the commit arm ':208 Record a command buffer that carried work, for the windows open now.' DECISION CONTENT: (1) the sampling model is a LOG + OPEN INDICES, not a slot: the stream keeps the ordered log of work-bearing command buffers; a window is (token, open_index) and closes to the slice committed[open_index..] — nesting falls out of arithmetic (an inner window's span is INSIDE the outer's by construction), so an outer round window correctly holds the per-launch windows of an instrumented run; (2) the failure instantiation is the LOUD-ZERO class the corpus already owns the refusal half of: under an active outer profiler, every candidate's sample read zero, the round measured NONE, and — without a zero-samples refusal — the first candidate ran SILENTLY (the cubecl #1941 zero-sample-disqualification is the complementary refusal half: 'a zero sample is no time, and disqualifies its candidate'; THIS rule removes the blindness CAUSE — both are needed, neither substitutes); (3) a command buffer counts for a window only if it carried WORK and committed while the window was open — recorded 'for the windows open now' at commit time, one pass over the open list; (4) the runtime test runs ON METAL (the nested-window runtime test) — the failure needs a real nested start_profile, which only a backend with collector-per-stream semantics exhibits. MEASURED: none source-reported — this is a correctness-of-measurement law; the damage class is a silently wrong election (the first candidate serving forever), which no timing number expresses. HOME-RELEVANCE: every autotune/lane in the stack that runs a tuning round inside an instrumented serving pass (the league harness's per-launch profiling over tuned kernels) hits this — the round elects the first candidate and the tuned path silently degrades. REGIME: backends where a profiling window is a single collector (Metal's stream-level collector confirmed); nested measurement contexts (tuning under tracing, nested benchmarking, per-launch profiling of autotune rounds); election logic downstream of the samples. ANTI-REGIME: command-stream-sampled backends whose windows already nest by timestamp semantics; windows that measure DISJOINT spans by design (close one before opening the next — the collector slot is correct there); a zero-sample ROUND that refuses (the #1941 half) without fixing the cause — the round still measures nothing. KINSHIP (do not double-count): profile-window-drains-queued-work-before-open (B207, D, gpu_77) owns the ORDERING-AT-OPEN honesty law — a window opened behind work it shares nothing with times that work's tail as its own, so it DRAINS the queue before opening; THIS owns the SAMPLING-MODEL honesty law — what a window's samples ARE when windows nest (committed-since-open, not a stolen collector); compose: both are 'the window is only as honest as its model of the stream' members, open-time ordering vs span bookkeeping; the cubecl #1941 zero-sample disqualification (the K-family record-only corroboration pinned inside compile-run-rejection-persists-patience-compiles-on-reach, gpu_134) owns the refusal half — a zero reading disqualifies its candidate; THIS owns the cause half — make the window SEE the samples; timed-zero-refuses-round-stop (rust_perf, the B284 measurement category) is the CPU-side twin of the refusal half, cross-domain; cross-run-profiles-key-on-content-not-stat-triplets (D) and warm-reachable-dispatch-signatures-before-timed-window (D) own profile CONTENT and warm coverage — different axes of the same measurement family. GREP EVIDENCE (over the kernel_opt corpus, 3+ vocabularies): 'profile window|profile-window' — the drain rule (B207), the election member (B214), warm-window (the K family) — no nesting/collector owner; 'committed since|since it opened|nested window' — ZERO; 'collector' — zero profiling owners; 'zero sample' — the #1941 corroboration only, record-only inside gpu_134. NotBenchable here (a measurement-honesty axis — the damage is a silent wrong election, not a timing figure; the nesting model, the span arithmetic, and the zero-samples-disqualify election are CPU-testable and ARE the fixtures). Source: tracel-ai/cubecl @ c1d0125 (#1993 tip; the Metal-native series #1962 via #1975 07cd07a) — crates/cubecl-metal/src/compute/stream.rs :187-208 + the squash body paragraph, Apache-2.0 OR MIT. Batch 319 / Plan 319. Every quote verbatim at the pin.".to_string(),
            ref_shape: r"// ref_shape (rule: profile-window-nests-committed-since-open):
// the naive stream: ONE collector slot. A window records whatever
// commits while it HOLDS the slot — so an inner window REPLACES the
// outer's and the outer measures nothing. The autotune shape: a round
// under per-launch profiling reads zero samples and runs candidate 0.
#![allow(dead_code)]

struct Stream {
    committed: Vec<u64>, // per-buffer cost units
}

/// the collector slot is stolen at the inner open: the outer's
/// samples never exist
fn profile_naive(s: &mut Stream, outer_work: usize, inner_work: usize) -> (Vec<u64>, Vec<u64>) {
    let mut collector: Vec<u64> = Vec::new();
    for i in 0..outer_work {
        let c = 10 + i as u64;
        s.committed.push(c);
        collector.push(c); // outer holds the collector
    }
    let mut inner_collector: Vec<u64> = Vec::new();
    for i in 0..inner_work {
        let c = 20 + i as u64;
        s.committed.push(c);
        inner_collector.push(c); // the inner REPLACED it
    }
    let outer = Vec::new(); // blind: zero samples
    (outer, inner_collector)
}

/// downstream: electing from blind samples — a zero-sample candidate
/// would win by absence of competition (the first candidate runs)
fn elect_first_nonempty_fallback(candidates: &[Vec<u64>]) -> usize {
    for (i, s) in candidates.iter().enumerate() {
        if s.iter().all(|&x| x > 0) {
            return i;
        }
    }
    0 // nobody measured: the first candidate runs silently
}
".to_string(),
            opt_pattern: r"// opt_pattern (rule: profile-window-nests-committed-since-open):
// the fix: the stream keeps the ordered log of work-bearing command
// buffers; a window is (token, open_index) and closes to the slice
// committed[open_index..] — nesting falls out of arithmetic, and a
// zero-sample round elects NOTHING (the #1941 refusal half rides
// alongside).
#![allow(dead_code)]

struct NestedWindows {
    open: Vec<(usize, Vec<u64>)>, // (token slot, samples)
    committed_len: usize,
}

impl NestedWindows {
    fn open_window(&mut self) -> usize {
        self.open.push((self.committed_len, Vec::new()));
        self.open.len() - 1
    }
    /// one pass over the open windows at commit time
    fn commit(&mut self, cost: u64) {
        self.committed_len += 1;
        for w in self.open.iter_mut() {
            w.1.push(cost); // recorded for the windows open NOW
        }
    }
    fn close(&mut self, token: usize) -> Vec<u64> {
        std::mem::take(&mut self.open[token].1)
    }
}

/// downstream: unmeasured cannot win — empty samples disqualify
fn elect_measured(candidates: &[Vec<u64>]) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .find(|(_, s)| !s.is_empty() && s.iter().all(|&x| x > 0))
        .map(|(i, _)| i)
}
".to_string(),
        },

        // ── glm-top1-chance-rides-pick-word ──
        // Rule 3 (category D — dispatch/transfer economics). tensorfold
        // 2dffc954 (+ 913bddb4 launch record + 086e8a28 contract). THREE
        // atoms: (a) the piggyback — the slice's softmax sum rides the
        // 16-byte word the greedy gather already moves (out[2]), no second
        // kernel or transfer; (b) pick parity by construction — the sums
        // variant is the SAME body templated on a bool, so its pick IS the
        // plain pick's (the b302 law applied at the kernel level); (c) the
        // f64 oracle contract — the sum is verified against float64 with
        // the max as the pick finds it, launch-recorded under the gates.
        KernelRule {
            rule_id: "glm-top1-chance-rides-pick-word".to_string(),
            gpu_concept: "MECHANISM (a draft's chance rides the pick word the consumer already moves — the slice's softmax sum in the same gather word as the greedy pick, pick parity by a shared templated body, the f64 oracle as the contract; category D — dispatch/transfer economics): a speculative-decode lane that wants the head's CHANCE of its pick needs the pick's softmax sum; shipping it as a second kernel or a second device-to-host word pays a whole round-trip for 4 bytes that had a free seat. THE SOURCE LAW, verbatim (tensorfold 2dffc954): 'GLM's MTP head gave the lanes no chances, so every drafted row went to verify; a row costs about 5.6 ms at TP=3 and many deep drafts miss. With --mtp-confidence, every rank picks an MTP draft with tf_glm_top1_sums, which adds its slice's softmax sum to the 16-byte word the greedy gather already moves; rank 0 turns the ranks' sums into the pick's probability, and the lanes chain them and stop a chain where its chance falls under the floor (the lane core's draft_confidence, as for Nemotron). Sampled and structured drafts keep chance 1. Without the flag every path is main's.' THE KERNEL SHAPE, verbatim (glm_glue_rows.cuh at 2dffc954): the pick kernel becomes `template <bool sums> __device__ __forceinline__ void glm_top1(...)` — ONE body, two instantiations; the sums arm adds 'out[2]: the slice's sum of exp(logit - its max), fp32 bits, from which rank 0 reads the pick's chance' computed as a strided per-lane partial (`for (int64_t i = t; i < n; i += 1024) s += __expf(...)`) tree-reduced over `__shared__ float sv[1024]` to thread 0; the wrappers: `tf_glm_top1` calls `glm_top1<false>` and 'The pick with its slice's softmax sum in out[2]: an MTP draft's chance (--mtp-confidence) across the ranks.' — `tf_glm_top1_sums` calls `glm_top1<true>`. THE CONTRACT, verbatim (086e8a28): 'contract: tf_glm_top1_sums under gates 1-4, its pick as tf_glm_top1's and its sum against float64'; the oracle body, verbatim (contract_glm_top1.zig): 'Row `r`'s terms exp(x - max) in float64, the max as the pick finds it.'; the parity pin, verbatim: 'tf_glm_top1_sums' first two words a launch, the pick; tf_glm_top1's on the same logits must equal them.' THE LAUNCH RECORD, verbatim (913bddb4): 'contract: tf_glm_top1_sums (1024 threads) records its launch, a new kernel — The MTP draft's pick under --mtp-confidence: tf_glm_top1's scan and its slice's softmax sum. nvcc 13.0 usage measured on the five targets; later compilers pending, as for tf_glm_top1.' DECISION CONTENT: (1) the transfer axis — an auxiliary scalar the consumer derives from ALREADY-READ data rides the word the consumer already moves: the pick word grows from (value, index) to (value, index, sum) and the DRAFT-CHANCE pipeline needs zero additional device-to-host traffic; rank 0 combines the ranks' slices into the pick's probability — the cross-rank reduction is host-side over words already gathered; (2) pick parity is a TEMPLATE fact, not a test fact — the sums variant shares the pick body (`glm_top1<true>` vs `glm_top1<false>`), so 'its pick as tf_glm_top1's' holds by construction and the contract battery pins it against the plain kernel on the same logits (the b302 sampler law — specialization carries the general rule — applied at kernel granularity); (3) the sum's ACCUMULATION is f32 (strided partials + shared-memory tree reduce, the deterministic fold the corpus's reduction-order law pins) while its ORACLE is f64 with the max AS THE PICK FINDS IT — the contract is the f32 tree sum within tolerance of the f64 reference, and the max feeding the exponentials is the PICK's max, never a second scan's; (4) the scope clause is part of the wire: 'Sampled and structured drafts keep chance 1' — the sums word is consulted only on the greedy path; without the flag every path is main's (the feature is additive, byte-identical off). MEASURED (source-reported, never an adoption bar): the 5.6 ms TP=3 verify row is the COST OF NOT HAVING chances (every drafted row verifying); the flag's win is the lanes stopping chains under the floor — serving-level, model-side. NotBenchable here. REGIME: speculative-decode draft pipelines that gate chains on the head's chance; pick kernels already reducing over the full row (the max scan is free real estate for the sum); multi-rank serving where per-slice sums combine host-side. ANTI-REGIME: consumers that never gate on chance (the plain pick word stays — the flag-off path is main's); sampled/structured drafts (chance 1 by definition — reading the word would be dead traffic); vocab rows so short the extra reduce dominates the gather (measure the 1024-thread tree against the word's marginal cost). KINSHIP (do not double-count): topk-sampler-specialization-carries-general-tie-rule (B302, B, TF fd058f5) owns the SPECIALIZATION-CARRIES-TIE-RULE law over the topk family (tf_topk_slices bounds candidates, tf_topk_race merges with the general sampler's own expressions/sum orders/tie rule); THIS is the top1 single-pick twin one level down — the same shared-body principle guaranteeing parity, NEW axes: the word piggyback (transfer economics) and the f64 oracle contract (accumulation-precision as contract) — compose, never merge; cost-aware-verify-length-survival-cost-table (B223, D) owns the LENGTH decision the chance feeds (argmax E[committed]/cost over a profiled table) — the chance word is an INPUT to that selection, compose; accept-packet-one-launch (B201, C) owns folding an acceptance WALK into one launch — different fold, same one-launch instinct; the deterministic-reduction family (the Issue-940 determinism law) owns the fold-order pinning this rule's tree reduce obeys. GREP EVIDENCE (over the kernel_opt corpus, 3+ vocabularies): 'chance|draft_confidence|pick word' — zero owners; 'softmax sum' — zero (the softmax-stats family owns stats precompute for the SCORES, not a pick's chance word); 'out[2]|16-byte word|piggyback' — zero; 'top1' — the topk family only (topk-select-nth-over-full-vocab-sort, tile-max-prefiltered-exact-topk, the B302 rule), no top1-chance owner. NotBenchable here (the 5.6 ms is the source's TP=3 verify cost, a serving-level figure — provenance, never a bar; the parity, the oracle contract, and the piggyback word layout are CPU-testable and ARE the fixtures). Source: ashhart/tensorfold @ 2dffc954 (+ 913bddb4 + 086e8a28) — zig/kernels/cuda/glm_glue_rows.cuh + zig/tests/cuda/contract_glm_top1.zig, Apache-2.0. Batch 319 / Plan 319. Every quote verbatim at the pin.".to_string(),
            ref_shape: r"// ref_shape (rule: glm-top1-chance-rides-pick-word):
// the naive shape: the pick kernel returns (value, index) and the
// chance needs a SECOND device pass + transfer — the softmax sum is
// recomputed and shipped separately from the word the consumer
// already moved.
#![allow(dead_code)]

#[derive(Clone, Copy)]
struct PickWord {
    value_bits: u32,
    index: u32,
}

fn first_max_nan_never_wins(v: &[f32]) -> (f32, usize) {
    let mut best = f32::NEG_INFINITY;
    let mut idx = usize::MAX;
    for (i, &x) in v.iter().enumerate() {
        if x.is_nan() {
            continue; // NaN never wins
        }
        if idx == usize::MAX || x > best {
            best = x;
            idx = i;
        }
    }
    (best, idx)
}

fn pick_word(logits: &[f32]) -> PickWord {
    let (m, idx) = first_max_nan_never_wins(logits);
    PickWord { value_bits: m.to_bits(), index: idx as u32 }
}

/// the SECOND pass the chance owes: a whole separate kernel launch +
/// device-to-host word for 4 bytes the pick pass already had in hand
fn chance_second_transfer(logits: &[f32], m: f32) -> f32 {
    let mm = m as f64;
    (logits.iter().map(|&x| (x as f64 - mm).exp()).sum::<f64>()) as f32
}
".to_string(),
            opt_pattern: r"// opt_pattern (rule: glm-top1-chance-rides-pick-word):
// the fix: ONE body templated on `sums` — the pick word grows a
// third slot (the slice's softmax sum, fp32 bits) computed by the
// SAME threads that already scanned the row, tree-reduced through
// shared partials; pick parity holds BY CONSTRUCTION (both
// instantiations share the pick body); the f64 oracle (the max as
// the pick finds it) is the contract.
#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq)]
struct PickWord {
    value_bits: u32,
    index: u32,
    sum_bits: Option<u32>, // out[2]: present only on the sums arm
}

fn glm_top1<const SUMS: bool>(logits: &[f32], lanes: usize) -> PickWord {
    let (m, idx) = first_max_nan_never_wins(logits);
    let sum_bits = if SUMS {
        let n = logits.len();
        let per = (n + lanes - 1) / lanes;
        let mut partials = vec![0.0f32; lanes];
        for (l, p) in partials.iter_mut().enumerate() {
            let lo = (l * per).min(n);
            let hi = ((l + 1) * per).min(n);
            let mut s = 0.0f32;
            for &x in &logits[lo..hi] {
                s += (x - m).exp(); // the strided per-lane partial
            }
            *p = s;
        }
        // the deterministic tree reduce over the lane partials
        let mut step = 1;
        while step < lanes {
            for l in (0..lanes).step_by(step * 2) {
                if l + step < lanes {
                    partials[l] += partials[l + step];
                }
            }
            step *= 2;
        }
        Some(partials[0].to_bits())
    } else {
        None
    };
    PickWord { value_bits: m.to_bits(), index: idx as u32, sum_bits }
}

/// the oracle: exp(x - max) in f64, the max AS THE PICK FINDS IT
fn sum_oracle_f64(logits: &[f32], m: f32) -> f64 {
    let mm = m as f64;
    logits.iter().map(|&x| (x as f64 - mm).exp()).sum::<f64>()
}
".to_string(),
        },

        // ── tiny-proj-aux-stream-overlap-size-gated ──
        // Rule 4 (category D — dispatch/streams). vllm #64425. FIVE atoms:
        // (a) the overlap — the tiny projection GEMM hides on an aux CUDA
        // stream beside the big one, event-fenced (start/done); (b) the
        // size gate — below a conservative token threshold, first
        // regression measured at 272 tokens; (c) the capture-only gate —
        // eagerly the stream switch costs more host time than it saves;
        // (d) the admission chain — plain cuBLAS only (shared-workspace
        // GEMMs never overlap), no ubatching, no LoRA; (e) the order law —
        // the tiny GEMM enqueued FIRST (aux-first); the other order
        // regressed the other arm.
        KernelRule {
            rule_id: "tiny-proj-aux-stream-overlap-size-gated".to_string(),
            gpu_concept: "MECHANISM (a tiny companion projection hides on an aux stream beside the big one — size-gated, capture-only, plain-GEMM-only, event-fenced, aux-first; category D — dispatch/streams): a layer that runs one big projection and one tiny one on the same stream serializes two kernels whose GEMMs are independent — the tiny one's whole runtime hides in the big one's tail if the streams run beside each other, and the join is two events. THE SOURCE LAW, verbatim (vllm #64425 @ 914d61fc5, the GDN layer's Part 1): 'Hide the tiny ba GEMM on the aux stream. Only under CUDA graph capture: eagerly the stream switch costs more host time than it saves. ba is enqueued first (aux-first order); qkvz-first regressed block-FP8 qkvz from ~100 tokens.' THE SIZE GATE, verbatim: 'Below this conservative threshold, overlap in_proj_ba with in_proj_qkvz on the aux stream. On GB300, across Qwen3.5/3.8 shapes, quant formats and TP 1-8, the first regression was at 272 tokens (27B BF16 TP8).' over `MAX_TOKENS = 256`. THE ADMISSION, verbatim: 'Opt-in, for callers outside torch.compile (compiled callers would need a custom op or Dynamo stream tracing; not validated). Plain cuBLAS ba GEMM only: FlashInfer GEMMs share a process-wide workspace. Off with ubatching and LoRA.' THE JOIN, verbatim (the call): `execute_in_parallel(lambda: self.in_proj_qkvz(hidden_states)[0], [lambda: self.in_proj_ba(hidden_states)[0]], start, [done], [self._ba_stream], enable=num_tokens <= _BA_PROJ_OVERLAP_MAX_TOKENS and torch.cuda.is_current_stream_capturing())` — the aux stream carries ONLY the tiny GEMM, `start`/`done` are the two `torch.cuda.Event`s fencing the fork and the join, and the enable predicate is the conjunction of the size gate and the capture check. DECISION CONTENT: (1) the STREAM axis — two independent GEMMs over the same input overlap on a second stream; the fork/join is event-based (`start` recorded before the aux op, `done` recorded on the aux stream, the main stream waiting on `done` before consuming the tiny output); (2) the ORDER within the overlap is measured, not aesthetic — the tiny GEMM is ENQUEUED FIRST on its stream (aux-first); the opposite order regressed the big GEMM's block-FP8 arm from ~100 tokens: enqueue order changes which kernel's tail the other overlaps, and the wrong order is worse than no overlap; (3) the size gate is a MEASURED cliff, not a heuristic — the overlap wins while the big GEMM is short enough that its tail covers the aux stream's latency, and loses once the big GEMM's own grid saturates the device (the first regression at 272 tokens across shapes/formats/TP); the constant is conservative by construction (256 < 272); (4) the CAPTURE-ONLY gate is an honest host-cost confession: under CUDA graph capture the stream switch is encoded once and replays free; eagerly, every step pays the switch's host time, which exceeds what the tiny GEMM hides — `torch.cuda.is_current_stream_capturing()` is the guard; (5) the admission chain is about HIDDEN SHARING: FlashInfer GEMMs share a process-wide workspace (two streams into one workspace is a race), ubatching changes the batch schedule, LoRA changes the GEMM path — each is a refusal, never a special case; the whole lane is opt-in for non-compiled callers. MEASURED (source-reported GB300, never an adoption bar): the first regression at 272 tokens (27B BF16 TP8); the wrong-order regression from ~100 tokens on block-FP8 qkvz. HOME-RELEVANCE: the GDN prefill lane's own projection pair (in_proj qkv/ba-shaped splits across the stack's GDN kernels) — the same two-GEMM shape, same graph-capture serving posture. REGIME: big+tiny independent GEMM pairs over one input inside captured serving graphs; measured size cliffs; event-fence-capable runtimes; plain-GEMM paths with per-call workspaces. ANTI-REGIME: eager (non-captured) execution — the host switch cost exceeds the win (measure the switch, not just the GEMMs); shared-workspace GEMM implementations (the workspace is the race); batch schedules that change per step (ubatching) or paths that swap the GEMM (LoRA); pairs whose big GEMM saturates the device at the serving batch size (the aux stream adds latency, never hides it); the opposite enqueue order without its own measurement (the order law is per-pair). KINSHIP (do not double-count): per-thread-stream + per-thread-stream-blocking-op-explicit-sync (D, gpu_76) own per-thread STREAM POOLS and the blocking-op sync discipline — a stream-ownership axis, not a projection-pair overlap; captured-graph-init-unconditional-and-stream-ordered (D) owns capture-time init ordering — the same graph-capture posture, the init axis; cross-queue-output-retention-until-completion (B195, E) owns the ALLOCATOR hazard of unordered queues (outputs retained until completion) — the lifetime complement to this rule's event fences; share-activation-stream-keep-projection-reductions (gpu_13) owns IN-KERNEL sharing when two matvecs read the same activation — this rule keeps the projections as separate KERNELS on separate STREAMS, the opposite granularity; write-only-touch-side-queue (B195, D) owns a dedicated second queue for write-only touches — the same second-stream instinct, the write-only-touch admission; the GDN conv-chain fusion (B295, D) fuses the gather into the concat launch — the fusion alternative to overlap, compose at the plan level (fuse what shares data; overlap what shares only the device). GREP EVIDENCE (over the kernel_opt corpus, 3+ vocabularies): 'aux stream|auxiliary stream|second stream' — write-only-touch-side-queue (the queue axis), cross-queue-output-retention (the lifetime axis) — no projection-pair owner; 'overlap.*stream|stream.*overlap' — the overlap-REDUCE mode (H, the combine threshold) is a collectives sense; 'in_proj|projection pair|tiny GEMM' — zero; 'is_current_stream_capturing|capture-only' — captured-graph-init (init ordering) only. NotBenchable here (the 272-token cliff and the ~100-token wrong-order regression are GB300 source-reported — provenance, never adoption bars; the gate chain, the fence model, and the byte-identical fallback are CPU-testable and ARE the fixtures). Source: vllm-project/vllm @ 914d61fc5 (#64425) — vllm/model_executor/layers/mamba/gdn/qwen_gdn_linear_attn.py (the gate const, the admission comment, the Part-1 call), Apache-2.0. Batch 319 / Plan 319. Every quote verbatim at the pin.".to_string(),
            ref_shape: r#"// ref_shape (rule: tiny-proj-aux-stream-overlap-size-gated):
// the naive layer: both projections on the main stream, serialized —
// the tiny GEMM waits behind the big one's full grid no matter how
// independent they are.
#![allow(dead_code)]

struct StreamLog {
    ops: Vec<&'static str>,
}

fn proj_big(_x: &[f32]) -> Vec<f32> {
    vec![1.0; 8] // the qkvz GEMM
}
fn proj_tiny(_x: &[f32]) -> Vec<f32> {
    vec![2.0; 2] // the ba GEMM
}

/// always sequential: one stream, big then tiny
fn run_seq(streams: &mut [StreamLog; 2], x: &[f32]) -> (Vec<f32>, Vec<f32>) {
    streams[0].ops.push("qkvz");
    let big = proj_big(x);
    streams[0].ops.push("ba");
    let tiny = proj_tiny(x);
    (big, tiny)
}
"#.to_string(),
            opt_pattern: r#"// opt_pattern (rule: tiny-proj-aux-stream-overlap-size-gated):
// the overlap: the tiny projection on the aux stream, event-fenced
// (start/done), enqueued FIRST (aux-first order), gated on the
// measured size cliff AND capture AND the plain-GEMM admission —
// every refused arm falls back byte-identically to the sequential
// path.
#![allow(dead_code)]

struct StreamLog {
    ops: Vec<&'static str>,
}
struct Events {
    fences: Vec<(usize, usize)>, // (signal stream, wait stream)
}
struct Admission {
    plain_gemm: bool,
    ubatching: bool,
    lora: bool,
}
const OVERLAP_MAX_TOKENS: usize = 256; // first regression: 272

fn run_overlap(
    streams: &mut [StreamLog; 2],
    events: &mut Events,
    capturing: bool,
    x: &[f32],
    n_tokens: usize,
    adm: &Admission,
) -> (Vec<f32>, Vec<f32>) {
    let overlap = adm.plain_gemm
        && !adm.ubatching
        && !adm.lora
        && n_tokens <= OVERLAP_MAX_TOKENS
        && capturing; // eagerly the switch costs more host time than it saves
    if !overlap {
        streams[0].ops.push("qkvz");
        let big = proj_big(x);
        streams[0].ops.push("ba");
        let tiny = proj_tiny(x);
        return (big, tiny);
    }
    // aux-first enqueue: ba lands BEFORE qkvz issues on main
    streams[1].ops.push("ba");
    let tiny = proj_tiny(x);
    streams[0].ops.push("qkvz");
    let big = proj_big(x);
    events.fences.push((1, 0)); // main waits on the aux signal
    (big, tiny)
}
"#.to_string(),
        },
    ]
}

/// Public batch accessor -- grows with distillation batches.
pub fn entries() -> Vec<KernelRule> {
    slice_b319_staged_pool()
}
