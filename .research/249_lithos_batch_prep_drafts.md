# Research 249 — lithos batch conversion prep (transcription-ready rule drafts)

**Status:** LANDED 2026-10-11 as Batch 223 / Plan 222 (11 rules + 3 riders, corpus 784→795; the twelve drafts adjudicated: R9 folded into R2 as the sibling-axis clause, T1's eligibility law folded into R2, T2's name-collision law folded into R8, T3 = research-note instance only; R7 and R12 confirmed RULES by fresh host reads — B187 and B105 read in full by the slice agents). Zero-cargo authoring under the Plan-337 census window (2026-10-10 ~00:3x +07): the Research 248 lithos batch's decision content is drafted HERE in corpus shape (`rule_id` / `gpu_concept` / `ref_shape` / `opt_pattern`). The batch unit RE-CLONED at the pin `.raw/lithos-b223` — HEAD == `236475fa716434807668562cd878ae623c2c1fd5` EXACTLY (zero drift). LANDED ABOVE the Research-248 revised yield (4–6 rules + 4–5 riders): every draft survived fresh dedup with zero owners, so the falsification the estimate priced never materialized — the fold set trimmed twelve drafts to eleven rules.

## Landing addendum (2026-10-11 — the T1 quote-verification ledger, the landing's owed duty)

Method: 3 authoring subagents over disjoint slices (A: R1–R4 → entries_gpu_85.rs; B: R5–R8 → entries_gpu_86.rs; C: R10–R12 → entries_gpu_87.rs) + coordinator riders (RD1→entries_gpu_08, RD2→entries_gpu_02, RD3→entries_gpu_33) + wiring (mod.rs KERNEL_CATEGORIES ×11, tests.rs pins/gates) + gates. Every quote re-verified at the pin clone BEFORE landing.

- **Slice A (R1–R4):** 40+ quotes OK at file:line. ONE DROPPED-to-paraphrase: the R4 probe sentence ('Our M3 Pro and M5 Pro probes found that in-kernel global barriers were no cheaper than dispatch boundaries...') is NOT at the pin — the pin's docs carry the contract form ('An ICB dispatch boundary can provide ordering... Global barriers are not assumed to be cheaper just because they occur inside one kernel', apple-gpu.md:49-51); the probe substance (barrier µs pricing, the long-dispatch stall) lives in the original design snapshot @ `46b2bc4` which design.md:10-12 points to. Carried as attributed provenance paraphrase, never quoted, never a bar. NEW verbatim found during verification and landed: mixers.md:104 'Removing a dispatch is not sufficient evidence of a latency improvement.' (joined R4). Attribution corrections: the R3 done-guard injection is region_fusion.py:85-109 (NOT emit.py as the draft said); R1's coherence content is ALL in static_fusion.py (no docs/design coherence section exists at the pin); design §5.1/§5.12 numbering resolves to the snapshot @ 46b2bc4 (the pin's docs carry no numbered sections); barriers.py is exactly 74 lines (draft claim confirmed).
- **Slice B (R5–R8):** ~30 quotes OK, ZERO dropped. Refinements: the R5 entry-guard canonical two-conjunct form (`if(st->done || st->t_this_step==0u) return;`) is gqa_prefill.metal:17 — advance.metal/spec_ops.metal open `if (i != 0 || st->done) return;` (attribution refined, wording unchanged); the R8 arithmetic is 16 copies TOTAL = 96 KB *per capacity token* ≈ 12.9 GB at 128K (the draft's '16×96 KB' phrasing misread as multiplication); R8 stats publish per (head, chunk, ROW), not the draft's 'row-group'; R7's no-gain quote lives in steal.metal citing decode-kernels.md §7 which is ABSENT from the pin tree — the measured tables live in the committed probe results (probes/results/Apple-M3-Pro p10) — both cited. R7 ADJUDICATION: RULE CONFIRMED (B187 `chunk-downsize-work-stealing-microtile-floor` read in full: it owns halve-while-underfilled + pre-assignment + a plain shared-counter steal — the coprime victim fan-out, claimant-invariance fence elision, and the measured-scope gate are genuinely unowned). R6's kin rules (`selector-lattice-beam-over-greedy-walk`, `spec-copy-window-aimd-ramp-alone-gated`) are rust_perf-domain — named in-rule with the domain to prevent a wrong-tree grep.
- **Slice C (R10–R12):** all quotes OK, ONE corrected in-kind: the R11 full cache-key string lives at autotune.py:33-41 (`gemv_key`), NOT gemv_tuning.py (that file is the manual crew-geometry tuner with no cache key), and the tail segments (`|shared-scales{d}|scale-order-{o}|P{stat_parts}`) are CONDITIONAL APPENDS — default axes elided, non-default named — the rule transcribes the conditional structure as the actual mechanism. R12 ADJUDICATION: RULE CONFIRMED (B105 `gpu-resident-moe-routing-no-host-sync` read in full: it owns the host-sync boundary of the routing pipeline — R12 owns the device-side scheduling discipline between grouping and down; B105's route table is literally R12's input). R12 disclosure: the retire arm is opt-in (`retire_idle` defaults False; the default posture bounds the loop) — disclosed in-rule. R10 digest hybrid disclosed (content digests on fp8/nvfp4/moe lanes vs source-file identity on weight_windows).
- **Riders (coordinator):** RD1/RD2/RD3 quotes verified at the pin by the coordinator before appending (gdn_mixer.metal:29-39 + prefill.py:11-14; fuse_norm.py:2-27 + mlp_fusion.py:14-15 + mixers.md:101-104; static_fusion.py:13-56 fp8_tile_decoder). RD3 note: the subtract form's subnormal arm is `(a<<20)+(121u<<23))-(1.0f/64.0f)` — the draft's description accurate.
- **En-route catches (the compile gate is the oracle):** the slice-B fixtures' opt_pattern fields carried REAL newlines inside Rust string literals + 6 interior raw double quotes (the agent's rustfmt --check claim was STALE — the recorded lesson class, third instance) — repaired by the coordinator via scripted join+escape; one batch223 gate needle respelled to the landed text ('never an interpolated or extrapolated cost' — the draft's 'must not extrapolate outside its supported row range' is the DRAFT'S phrasing, not the rule's).
- **Wiring:** mod.rs KERNEL_CATEGORIES A+3/C+1/D+3/E+2/H+1/K+1 (a first-pass E-array append landed in the G array — the sed-window straddle; moved pre-gate), tests.rs count pins 784→795 ×5 sites + category array + batch223_rules_parse_and_needles_landed (11×parse+needles) + batch223_riders_landed_on_their_hosts (3×needles) + the batch29 exact-concept needle extended with the RD2 rider text, router.rs expert-total pin. 8 doc_markdown mechanicals healed by `cargo refine --fix --write --verify` (compile-gated, recorded).
- **Retrieval floors:** `kernel+docker` router-annotation arm re-measured 101/171 (59.1%) vs floor 102 — re-pinned 96/171 with the pre-batch A/B attribution (detached worktree at e7f2a9ee measured EXACTLY 102/171 — the batch costs ONE span, the B137 attractor mechanism); floor = measured − 5 per the Plan-030 convention. No other floor moved.

Gates: batch223 2/2; kernel_opt+latent_retrieval lib 3118/0 (3 ignored = the documented freeze-regen e2es); default lib 2493/0; clippy -D ×3 postures clean; freeze/score-bench NOT owed (zero clippy-corpus delta, P30). Commit `1c63ec7f` (slice C's early single-file push, compile-neutral pre-wiring) + the batch landing commit. Ephemeral clone `.raw/lithos-b223` removed at close.

## Provenance + discipline

- **Quote authority is Research 248, never this file.** Every quote below was read at the pin `lithos-ai/lithos-metal @ 236475fa716434807668562cd878ae623c2c1fd5` during the verdict session (kernels/docs/probes) and the compiler-addendum session (all 23 `monolith/compiler/` files, ephemeral clone, removed at close). The batch unit RE-CLONES at the pin and re-verifies every quoted line before landing (the B193 freshness precedent — the repo is 15 commits, one day old at the pin; freshness re-adjudication is mandatory, not optional).
- **Fixtures are ORIGINAL minimal Rust sketches** expressing each law (license hygiene law — no transliteration of lithos Metal/Python; the corpus precedent: CUDA/Metal sources sketch in plain Rust).
- **Dedup evidence at prep time** (this session, zero-cargo): two-vocabulary greps at verdict time (Research 248 §Dedup evidence) + fresh re-greps — `coherent|__ldg|read-only cache` hits only the NCCL peer-sentinel rule's visibility clause (entries_gpu_30, cross-kernel readiness — different mechanism) and the TensorFold low-bit load-coalescing rule (entries_gpu_56, load DIRECTION, not coherence qualification); `variant_group|variant group` and `whole-prefix|whole prefix` return ZERO rule-content owners; the verdict's barrier-vocabulary grep found no cross-worker barrier PROTOCOL owner (hits are elimination/reduction/ordering/ownership families). No candidate has an owner.
- **Numbering:** `.research/.highwater` read 248 at allocation; this file is 249; dual_allocation_gate green (0 twin / 0 independent / 0 counter) at allocation.
- **Batch/Plan numbers are placeholders** — "(at landing)" — because the conversion set may consume Batch 213; the lithos batch takes the next number at its own landing. Protocol A step 0 (plan-first `.plans/` allocation + fresh dedup grep) runs AT CONVERSION per the Research-236/239 precedent.

## Proposed batch shape (the batch adjudicates; grades from Research 248)

| Draft | Proposed rule_id | Cat | Grade | Disposition |
|---|---|---|---|---|
| R1 | `coherence-scope-by-write-set-ordinary-loads-on-immutable` | A | B+ | RULE (headliner) |
| R2 | `barrier-on-reader-open-group-conflict-swap-overlap` | D | B+ | RULE (headliner) |
| R3 | `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder` | E | B+ | RULE |
| R4 | `fusion-scope-by-barrier-cost-not-dispatch-count` | C | B+ | RULE |
| R5 | `gpu-resident-control-loop-static-icb-replay` | D | B+ | RULE |
| R6 | `cost-aware-verify-length-survival-cost-table` | D | B− | RULE |
| R7 | `own-slice-coprime-stride-steal-claimant-invariant` | E | B− | rider-or-rule vs B187 (adjudicate at conversion) |
| R8 | `attention-partition-two-knob-shared-partial-workspace` | A | B−/B | RULE (upgraded by the addendum's 12 GB motivation) |
| R9 | `encode-order-scheduling-bus-bound-first` | D | B− | RULE or clause folding into R2 (pairs with it) |
| R10 | `derived-pack-immutable-original-content-addressed-atomic` | A | B− | RULE |
| R11 | `autotune-fidelity-synthetic-carries-producer-params` | K | B−→C+ | RULE (measurement discipline) |
| R12 | `moe-readiness-queue-no-grid-barrier-producer-scans-before-retire` | H | B− | rider-or-rule vs `gpu-resident-moe-routing-no-host-sync` |
| RD1 | → host `gdn-transition-replay-log-commit-many` | G | B | RIDER (parity-slot commit-by-recompute) |
| RD2 | → host = norm-commute carrier (locate at conversion) | C | C+ | RIDER (prefold half) |
| RD3 | → host `lut-free-bit-spread-vadd4-unpack` (Batch 212) | B | rider | RIDER (fp8 subnormal density-split face) |
| T1 | `read-before-write-carries-state-across-replays` | A | C+ | C-tier: rule / instance-note (adjudicate) |
| T2 | `params-record-program-unique-identity` | D | C+ | C-tier: rider on the shared-workspace family (adjudicate) |
| T3 | `ir-coverage-gate-fails-at-compile-with-missing-list` | — | C | C-tier: instance-note (our green-zero family) |

---

## R1 — `coherence-scope-by-write-set-ordinary-loads-on-immutable` (A, B+ — HEADLINER)

**gpu_concept draft:**

> Pointer-qualification on GPUs with a non-coherent read-only/weight cache path (Metal `coherent` vs ordinary `device`; the CUDA face is the same decision the compiler makes for `__ldg`/`const __restrict__` — read-only data cache vs L1): inside one persistent or multi-stage dispatch, read data PROVABLY IMMUTABLE through ORDINARY (non-coherent) accesses and everything else through COHERENT ones — ordinary access keeps the weight-cache path, coherent qualification DEFEATS it. The classification is a WRITE-SET decision made per POINTER, never blanket: the staging pass lifts every `device` pointer to coherent by default, then strips coherence per-pointer where the dispatch cannot change the data — (1) packed weights ("uint4 pointers in this kernel address immutable packed weights only"), (2) the KV prefix strictly before the current position ("This helper only reads keys strictly before the current position. No stage in this dispatch writes that range. Tail loads/stores and every cross-task intermediate retain coherent device accesses"), (3) external inputs published by the preceding dispatch ("The preceding dispatch publishes inputs; internal data stays coherent"). TWO GRANULARITIES BY PURPOSE in the same pass: conflict/barrier decisions classify at BUFFER granularity CONSERVATIVELY ("Classify by the entire buffer name so aliases at different offsets remain coherent whenever any task writes to that allocation"), while coherence decisions are per-POINTER FINE — two views of ONE weight block take DIFFERENT qualifiers (the narrow-scale reinterpret keeps ordinary; the activation view stays coherent). Padded-speculation rider: clamped loads (`min(idx, T_act-1)`) make out-of-bounds speculation past a select LEGAL — "Ordinary cached reads may be speculated past a select". REGIME: single-dispatch persistent kernels where an immutable class is provable from the write-set (weights; append-only history read below the write cursor; prior-dispatch outputs). ANTI-REGIME: any alias the write-set cannot classify stays coherent (the conservative default); racing scratch never qualifies; and a kernel whose weight pointer is REUSED for scratch reinterprets into the coherent class. Kinship, compose: the same staging pass's conservative half is `barrier-on-reader-open-group-conflict-swap-overlap` (buffer-granular conflict classification vs this pointer-granular coherence); distinct from every L2 cache-POLICY rule (evict-first/evict-last decide a line's LIFETIME; this decides the CACHE PATH via the coherence qualifier — no policy operand exists on Metal). Direct consumers: riir-infer-laya's MSL kernels + riir-gpu (per-op Metal decode dispatches where weights are provably immutable across a dispatch). Source: lithos-ai/lithos-metal @ 236475fa — monolith/compiler/static_fusion.py `stage()` (the coherent-default lift + per-pointer strip + the three immutable classes, quotes verbatim) + docs/design §coherence granularity. Batch (at landing) / Plan (Protocol A step 0) / Research 248 addendum A.

**ref_shape draft:**

```rust
// naive: ONE coherence posture for every pointer in the dispatch — every
// load qualified coherent, including weights nothing in the kernel writes.
// The read-only weight cache is defeated for the whole dispatch.
fn stage_all_coherent(
    weights: &[u128],    // 200 MB of packed weights — never written here
    kv_prefix: &[f32],   // rows [0..pos) — never written this step
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
```

**opt_pattern draft:**

```rust
// optimized: the staging pass classifies by WRITE-SET, per POINTER.
//   immutable across the dispatch -> ORDINARY loads (weight-cache path kept):
//     packed weights | KV prefix strictly before `pos` | prior-dispatch inputs
//   everything else (scratch, cross-task intermediates, tail rows) -> coherent.
#[derive(Clone, Copy, PartialEq)]
enum Scope {
    Ordinary, // non-coherent: the read-only/weight-cache path serves it
    Coherent, // racing or written: full coherence
}

fn classify(immutable_class: bool) -> Scope {
    // conservative default: only a PROVEN immutable class strips coherence
    if immutable_class { Scope::Ordinary } else { Scope::Coherent }
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
        // select is legal — "ordinary cached reads may be speculated past
        // a select" — and the clamp keeps the read inside the prefix
        acc += ordinary_load(w) as f32 * ordinary_load(&kv_prefix[i.min(pos - 1)]);
    }
    scratch[0] = acc; // coherent store: the write set, unchanged
    acc
}

fn ordinary_load<T: Copy>(x: &T) -> T { *x } // non-coherent path
```

**Re-verify at pin:** static_fusion.py `stage()` — the coherent-default lift, the three immutable-class quotes, the buffer-vs-pointer granularity pair, the padded-speculation clamp line; docs/design coherence section.

---

## R2 — `barrier-on-reader-open-group-conflict-swap-overlap` (D, B+ — HEADLINER)

**gpu_concept draft:**

> Dependency minimization for STATIC command buffers with whole-prefix wait semantics — an encoded command's barrier waits for EVERY command before it in the buffer, not just its predecessor (Metal ICB semantics; CUDA graph capture's node-edge semantics are the same shape): place barriers with an OPEN-GROUP conflict pass instead of flagging every op. The flag on the READER is what orders a dependency — v0 flagged every op, and the measured v0-to-minimal arc is the measure-then-remove-switch discipline in its purest form. The algorithm: an op joins the ops-since-last-barrier (the open group) unless it CONFLICTS with any member — WAR/WAW/RAW detected at buffer granularity, and a row view ALIASES ITS WHOLE BASE (the conservative reading). The SWAP is the overlap payoff: when op i conflicts with the GROUP but not its immediate predecessor, the barrier moves ONTO the predecessor and the conflicting pair runs TOGETHER ("a gate GEMV encoded before its mixer core still overlaps the core rather than the projection it followed"). Three finishing clauses: mutually-exclusive per-variant dispatches join the group WITHOUT a check (variant_group — only one will ever run); the step's FIRST op always waits (the cross-step tail is real); and an op with no write metadata is treated as WRITING EVERYTHING IT BINDS (conservative default — unknown never joins the group). REGIME: any static replay lane with prefix-wait barriers — Metal ICB, CUDA graph capture, pre-encoded command lists. ANTI-REGIME: dynamically-encoded streams (no static order to reason over). Kinship: the cuda-graph family owns the replay CONTRACT — this owns the dependency SET inside one capture; compose with `coherence-scope-by-write-set-ordinary-loads-on-immutable` (the same staging pass's fine-granularity half) and with `encode-order-scheduling-bus-bound-first` (the sibling axis — which of two groupable ops encodes first). Source: lithos-ai/lithos-metal @ 236475fa — monolith/compiler/barriers.py (whole file, 74 lines) + design §5.1/§5.12 + upstream issue #29 + the tools/bench measurement (2026-09-24). Batch (at landing) / Plan (Protocol A step 0) / Research 248 addendum B.

**ref_shape draft:**

```rust
// naive: v0 — a barrier before EVERY encoded op. Correct under whole-prefix
// semantics by construction, and it serializes everything: a gate GEMV
// waits for the projection that merely ENCODED before it, not the op it
// actually depends on.
#[derive(Clone)]
struct Op {
    name: &'static str,
    reads: &'static [&'static str],
    writes: &'static [&'static str],
}

fn encode_v0(ops: &[Op]) -> Vec<String> {
    let mut buf = Vec::new();
    for op in ops {
        buf.push(format!("barrier -- whole prefix waits here"));
        buf.push(format!("dispatch {}", op.name));
    }
    buf
}
```

**opt_pattern draft:**

```rust
// optimized: OPEN-GROUP placement. Track the ops since the last barrier;
// a new op joins the group unless it WAR/WAW/RAW-conflicts with a member
// (buffer-granular; a row view aliases its whole base). On conflict with
// the GROUP but not the immediate predecessor, the barrier moves ONTO the
// predecessor — the conflicting pair runs TOGETHER instead of sandwiching
// the whole prefix.
fn conflicts(a: &Op, b: &Op) -> bool {
    let w = |x: &Op| x.writes.iter().chain(x.reads.iter());
    a.writes.iter().any(|wr| w(b).any(|r| r == wr))      // WAR or WAW
        || b.writes.iter().any(|wr| a.reads.contains(wr)) // RAW
}

fn encode_open_group(ops: &[Op]) -> Vec<String> {
    let mut buf = Vec::new();
    let mut group: Vec<&Op> = Vec::new(); // ops since the last barrier
    for (i, op) in ops.iter().enumerate() {
        // unknown write metadata is treated as writing everything it binds:
        // an op with empty `writes` that binds buffers never joins the group
        let joins = !group.iter().any(|g| conflicts(g, op)) && !op.writes.is_empty();
        if !joins && i > 0 {
            // THE SWAP: gate the PREDECESSOR, not this op — the pair overlaps
            let pred = &ops[i - 1];
            buf.push(format!("barrier before {} (swapped)", pred.name));
            buf.push(format!("dispatch {} + {} overlap", pred.name, op.name));
            group.clear();
            group.push(op);
        } else {
            buf.push(format!("dispatch {}", op.name));
            group.push(op);
        }
    }
    buf
}
```

**Re-verify at pin:** barriers.py in full (74 lines) — reader-flag ordering, the open-group join rule, the swap, variant_group, the first-op-waits clause, the conservative no-writes default; design.md §5.1/§5.12; issue #29; the tools/bench date.

---

## R3 — `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder` (E, B+)

**gpu_concept draft:**

> Cross-WORKER synchronization inside ONE persistent dispatch on GPUs WITHOUT cooperative grid sync (Metal has none): a bounded device-scope polling barrier with FOUR fan-in variants picked by worker count and topology — SERIAL (thread 0 of one group scans all N arrival counters), SIMD (one SIMD group's 32 lanes strided-scan the arrivals), WIDE (N SIMD groups each scanning a shard, then a second-level fan-in), LEADER-RELEASE (worker 0 alone polls arrivals and publishes ONE release epoch; every other worker polls that single variable — per-waiter polling drops from O(workers) to O(1)). The invariants that make it correct under replay: arrival and release EPOCHS are MONOTONE across stages and repeated dispatches ("arrival and release epochs are monotone across stages and repeated dispatches" — no reset; a reset races stragglers from the previous stage); arrivals are relaxed fetch_add + epoch compare; a device-scope seq_cst fence plus threadgroup barrier sits on BOTH sides of each stage. The BOUND is the safety story, not an optimization: a 1M-spin cap flips a failure flag and broadcasts it — the kernel REPORTS failure (error + done) instead of hanging forever ("bounded waits report failure rather than spinning indefinitely"); the done guard sits at EVERY kernel entry (compiler-injected — even regions that do not logically read control state get the failure path wired), and a compile-time cap (state slots ≥ 31) refuses the fusion LOUDLY rather than silently exceeding the Metal binding budget ("fused region has no Metal binding left"). REGIME: persistent megakernel workers coordinating stage boundaries on Apple-class GPUs. ANTI-REGIME: where MEASURED, in-kernel global barriers may cost no less than dispatch boundaries (see `fusion-scope-by-barrier-cost-not-dispatch-count` — then do not fuse); CUDA cooperative launch has a native grid sync (use it — this protocol exists for its ABSENCE). GOAT posture: NotBenchable on our box (Metal/ICB; source numbers M5-class provenance-only); OPTIONAL staged GOAT = a CubeCL-on-Metal polling-fan-in-ladder A/B (the B204→T11 pattern) — the batch decides whether to stage it. Source: lithos-ai/lithos-metal @ 236475fa — kernels/common/static_barrier_{serial,simd,leader,wide}.metal + monolith/compiler/region_fusion.py (the injected failure path + the ≥31 refusal) + emit.py (the done-guard injection). Batch (at landing) / Plan (Protocol A step 0) / Research 248 candidate 1.

**ref_shape draft:**

```rust
// naive: cross-worker stage sync by per-worker SPIN ON EVERY PEER — each
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
```

**opt_pattern draft:**

```rust
// optimized: LEADER-RELEASE fan-in with monotone epochs + a spin bound.
// Worker 0 polls the arrival counters; everyone else polls ONE release
// epoch — O(1) per waiter. Epochs only ever INCREASE (no reset: the count
// itself is the phase), and a bounded spin flips a failure flag every
// worker can see instead of hanging the dispatch.
fn stage_sync_leader_release(
    my: usize,
    arrivals: &mut [u64],
    release: &mut u64,
    failed: &mut bool,
    workers: usize,
) -> bool {
    arrivals[my] += 1; // relaxed arrival; seq_cst fence + group barrier
    // belong on BOTH sides of the stage in the real protocol
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
    } else {
        let mut spins = 0u64;
        while *release < my_epoch {
            spins += 1;
            if spins > 1_000_000 || *failed {
                return false; // the failure broadcast reaches every waiter
            }
        }
        true
    }
}
```

**Re-verify at pin:** static_barrier_{serial,simd,leader,wide}.metal — the four variant shapes, the epoch monotonicity quote, the 1M-spin bound + failure-flag quote, the fence/barrier placement; region_fusion.py — the injected done-guard + failure path + the ≥31 state-slot refusal quote.

---

## R4 — `fusion-scope-by-barrier-cost-not-dispatch-count` (C, B+)

**gpu_concept draft:**

> The kernel FUSION BOUNDARY is decided by MEASURED barrier cost, never by dispatch count: on M3 Pro and M5 Pro probes, in-kernel global barriers were NO CHEAPER than dispatch boundaries — and long dispatches delay other GPU clients ("Our M3 Pro and M5 Pro probes found that in-kernel global barriers were no cheaper than dispatch boundaries, and long dispatches could delay other GPU clients") — so the boundary settled LAYER-WISE: one layer's mixer per dispatch, MLP separate, never a whole-model megakernel. Three companion laws ride the same decision: merely reducing the number of dispatches does NOT guarantee lower memory traffic ("cross-worker intermediates can still require device storage"); OMITTING a barrier requires DEPENDENCY ANALYSIS, not a timing observation; and the runtime never assumes an arbitrary number of threadgroups are resident simultaneously (the progress condition — a megakernel whose workers must all be co-resident to make progress deadlocks on a smaller GPU). And the discipline that keeps it honest: the compiler builds the UNFUSED normalized program as the numerical AND performance CONTROL for every fused region (regions validated disjoint/nonempty/unique; "Model-specific names never enter the compiler"). REGIME: choosing fusion scope on Apple-class GPUs where dispatch overhead is small and global sync is polling-based; the M3 Pro datum is OUR silicon class. ANTI-REGIME: architectures with cheap hardware grid sync (CUDA cooperative launch) or dispatch overhead large enough to dominate — there the megakernel may win; measure, never assume either way. Kinship: `epilogue-must-be-workgroup-local` owns the OWNERSHIP legality of the same boundary (this is the COST side); pairs with the barrier protocol (R3) that prices the in-kernel alternative it declines. Source: lithos-ai/lithos-metal @ 236475fa — docs/design/apple-gpu.md + design.md §Fusion boundaries + region_fusion.py (the unfused control + region validation). Batch (at landing) / Plan (Protocol A step 0) / Research 248 candidate 2.

**ref_shape draft:**

```rust
// naive: fuse the WHOLE model into one megakernel because "fewer
// dispatches is faster" — assumed, never measured; the in-kernel global
// barriers between layers cost what dispatches cost, and the long single
// dispatch delays every other GPU client.
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
```

**opt_pattern draft:**

```rust
// optimized: the boundary is LAYER-WISE because the probes measured the
// in-kernel barrier at dispatch-boundary cost on this silicon class —
// and the unfused program is kept as the built-in control.
enum Scope {
    FusedLayerWise, // one layer's mixer per dispatch
    UnfusedControl, // the normalized program: numerics AND perf baseline
}

fn choose_scope(measured_barrier_vs_dispatch: f32) -> Scope {
    // in-kernel barrier >= dispatch boundary => fusing buys nothing;
    // the control stays runnable for every future re-measurement
    if measured_barrier_vs_dispatch < 0.9 {
        Scope::FusedLayerWise
    } else {
        Scope::FusedLayerWise // still layer-wise: the datum says no win
    }
}

fn layer_wise_pipeline(tokens: &mut [f32], layers: usize) {
    for layer in 0..layers {
        dispatch_mixer(tokens, layer); // barrier = the dispatch boundary
    }
    dispatch_mlp(tokens);
}
```

**Re-verify at pin:** docs/design/apple-gpu.md + design.md fusion-boundary section (the probe quote verbatim), the three companion clauses, region_fusion.py's unfused-control + validation lines.

---

## R5 — `gpu-resident-control-loop-static-icb-replay` (D, B+)

**gpu_concept draft:**

> The generation CONTROL LOOP — advance, accept-scan, EOS, stop, position — lives ON THE GPU under a STATIC pre-encoded command buffer: the command sequence is encoded ONCE and replayed; dynamic control is data-dependent early-return at every kernel entry (`if done || t_this_step == 0 return` — "A later replay can return without doing work"), never a re-encode. A ONE-THREAD serial closer kernel mutates control state between stages: publish the token to a sequence-tagged ring (`(seq<<32)|tok`), set EOS/stop_at, bump position; the accept-scan commits the accepted prefix + its bonus, and REJECTED positions' KV rows are simply OVERWRITTEN by the next pass — logical length is the only truth, no rollback pass exists. Guards fire BEFORE the hazard they guard: error+done are written before any cache over-run (ctx_cap clamps the verify width BEFORE the caches can exceed it), ring-overflow is checked before the write. The host's per-token work is ZERO — every decision per token is data on the GPU. REGIME: decode serving loops on any static-replay runtime (Metal ICB, CUDA graphs). ANTI-REGIME: control that genuinely needs a host decision per token — but audit each such "decision" for on-device computability first. Kinship: distinct from `cuda-graph-scalar-via-device-pointer` (scalar VALUE plumbing across captures — this is the LOOP), from `gpu-resident-moe-routing-no-host-sync` (one pipeline's routing — this is the loop the pipelines live under), from the ds4 GPU-side draft-selector rider (ONE decision — this is EVERY decision per token). Source: lithos-ai/lithos-metal @ 236475fa — kernels/common/advance.metal + spec_ops.metal (verify_select / accept_scan) + design.md §Runtime program + the StepState shape. Batch (at landing) / Plan (Protocol A step 0) / Research 248 candidate 3.

**ref_shape draft:**

```rust
// naive: the generation loop lives on the HOST — every token round-trips
// a control decision through the CPU, and the replayed command list is
// re-encoded (or re-parameterized) whenever control flow changes.
fn decode_host_loop(mut token: u32, steps: usize) -> Vec<u32> {
    let mut out = Vec::new();
    for _ in 0..steps {
        let done = host_advance(&mut token); // host reads, decides, writes
        if done {
            break;
        }
        reencode_commands(token); // control change => re-encode the replay
        out.push(token);
    }
    out
}

fn host_advance(_t: &mut u32) -> bool { false }
fn reencode_commands(_t: u32) {}
```

**opt_pattern draft:**

```rust
// optimized: the loop is GPU DATA under a STATIC replay. Every kernel
// opens with the same data-dependent early return; a one-thread closer
// kernel is the only control-state writer; rejected KV rows are simply
// overwritten next pass (logical length is the only truth). The host
// never runs per token.
#[derive(Clone, Copy)]
struct StepState {
    done: bool,
    t_this_step: u32,
    pos: u32,
    error: u32,
}

fn kernel_entry(st: &StepState) -> bool {
    // injected at EVERY kernel entry: a later replay can return without
    // doing work — the command sequence is never re-encoded
    !(st.done || st.t_this_step == 0 || st.error != 0)
}

fn closer_advance(st: &mut StepState, ring: &mut Vec<(u64, u32)>, tok: u32) {
    // guards BEFORE the hazard: ring capacity checked before the write
    if ring.len() >= ring.capacity() {
        st.error = 2; // ring overflow: error + done, no partial state
        st.done = true;
        return;
    }
    ring.push((((st.pos as u64) << 32) | tok as u64, tok));
    st.pos += 1;
    st.t_this_step = st.t_this_step.wrapping_add(1);
}
```

**Re-verify at pin:** advance.metal + spec_ops.metal (the entry guard, the ring tag format, the accept_scan overwrite contract), design.md §Runtime program, the ctx_cap-before-overrun ordering.

---

## R6 — `cost-aware-verify-length-survival-cost-table` (D, B−)

**gpu_concept draft:**

> Speculative-decoding VERIFY LENGTH is an expected-committed-per-cost maximization over a PROFILED cost table, not a fixed constant: L = argmax_l (1 + Σ_{i≤l} a_i) / cost[l], where a_i = Π_{j≤i} c_j is the SURVIVAL probability of draft token i (c_j = the per-position acceptance probability) and cost[l] = the measured relative cost of a (1+l)-row target pass vs a 1-row pass, taken from a per-(format, path) table over the pack's DOMINANT weight format (max total nbytes; accelerator rows switch at accelerator_min_t). Two hard clauses: the table MUST NOT EXTRAPOLATE outside its supported row range — a lookup miss returns None and the caller falls back to a FIXED threshold (FALLBACK_THRESHOLD = 0.5, "verifying the whole block costs ×5 at T = 8" as the motivating datum), never an extrapolated cost; and the context cap clamps L BEFORE any cache can over-run (the draft width can never exceed the reserved KV/scratch). A confident-prefix threshold mode ships beside the table for callers without profiles. The whole selection is a ONE-THREAD GPU-side op — no host round-trip in the decode loop. REGIME: spec-decode engines with variable draft widths and measurable per-width costs. ANTI-REGIME: fixed-width verify (no table needed) or hosts without cost profiles (use the threshold mode). Kinship: `selector-lattice-beam-over-greedy-walk` owns the WALK structure (this is the LENGTH decision under it); `spec-copy-window-aimd-ramp-alone-gated` owns the WINDOW adaptation (compose — the window bounds what L may take); the no-extrapolation clause is the same species as every measured-table-with-supported-range law in the corpus. Source: lithos-ai/lithos-metal @ 236475fa — kernels/common/spec_ops.metal `verify_select` + monolith/compiler verify_costs (dominant-format rows, accelerator_min_t, FALLBACK_THRESHOLD). Batch (at landing) / Plan (Protocol A step 0) / Research 248 candidate 4.

**ref_shape draft:**

```rust
// naive: verify length is a CONSTANT — every step verifies the full draft
// width whether the drafter is hot (wasted target rows) or cold (still
// paying the fixed width), with no cost model and no survival weighting.
const VERIFY_LEN: usize = 8;

fn choose_l_naive(_survival: &[f32]) -> usize {
    VERIFY_LEN // one size for every drafter state and every shape
}
```

**opt_pattern draft:**

```rust
// optimized: L = argmax_l E[committed](l) / cost[l], from a PROFILED
// table that refuses to extrapolate (miss -> None -> fixed-threshold
// fallback), clamped by the context cap BEFORE any cache can over-run.
fn cost_at(cost_table: &[(usize, f32)], l: usize) -> Option<f32> {
    // supported rows only — never interpolate, never extrapolate
    cost_table.iter().find(|(row, _)| *row == l).map(|(_, c)| *c)
}

fn choose_l(
    survival: &[f32],   // a_i = prod c_j: probability draft i is still alive
    cost_table: &[(usize, f32)],
    ctx_cap: usize,
) -> usize {
    let mut best = 1usize;
    let mut best_score = f64::NEG_INFINITY;
    for l in 1..=survival.len().min(ctx_cap) {
        // ctx_cap clamps BEFORE the lookup: no width past the reservation
        let Some(cost) = cost_at(cost_table, l) else {
            break; // table ended: FALLBACK, never extrapolate
        };
        let committed = 1.0 + survival[..l].iter().sum::<f32>();
        let score = committed as f64 / cost as f64;
        if score > best_score {
            best_score = score;
            best = l;
        }
    }
    best
}
```

**Re-verify at pin:** spec_ops.metal verify_select (the argmax form + survival product), verify_costs (dominant format, accelerator_min_t, the None path + FALLBACK_THRESHOLD = 0.5 + the ×5 motivating quote), the ctx_cap-before-cache ordering.

---

## R7 — `own-slice-coprime-stride-steal-claimant-invariant` (E, B− — rider-or-rule vs B187)

**gpu_concept draft:**

> Work-stealing refinement for tail-skewed persistent loops, gated on MEASURED skew: each worker first claims a CONTIGUOUS own slice from its OWN device atomic cursor (the uncontended CAS fast path — most workers never contend); only thieves then visit victims in a per-thief COPRIME stride so concurrent thieves fan out over DIFFERENT victims instead of queueing on one ("thieves fan out over different victims"); surplus workers own NOTHING (steal-only); missing workers cost only their slice. The fence elision is the subtle half: CLAIMANT-INVARIANCE — a block's output does not depend on WHICH worker claimed it, and the op's consumer is the NEXT dispatch — licenses running a stolen block with NO fence beyond the claim's own atomicity. And the gate is honest: enable per op ONLY where a trace shows tail skew AND an A/B gain — the source's own datum is that none of its dense targets' ops qualifies ("none of the dense targets' ops does"). REGIME: persistent loops with per-unit cost variance (ragged expert sizes, data-dependent per-block exits). ANTI-REGIME: uniform-cost partitions — the own-slice CAS alone is optimal there and the stealing machinery is pure overhead. Kinship: `chunk-downsize-work-stealing-microtile-floor` (B187) owns halve-while-underfilled + pre-assignment + atomic steal — the COPRIME victim fan-out, the claimant-invariance fence elision, and the measured-scope gate are the unowned atoms; ADJUDICATE rider-vs-rule against a fresh B187 read at conversion. Source: lithos-ai/lithos-metal @ 236475fa — kernels/common/steal.metal + probes/results p10. Batch (at landing) / Plan (Protocol A step 0) / Research 248 candidate 5.

**ref_shape draft:**

```rust
// naive: all workers contend on ONE global cursor — every claim is a
// contended CAS on the same word, and a stolen block runs behind a full
// fence "just in case" the claimant mattered.
fn claim_naive(cursor: &mut u64, n: usize) -> Option<usize> {
    let i = *cursor;
    if i < n as u64 {
        *cursor += 1; // everyone queues on this one word
        Some(i as usize)
    } else {
        None
    }
}

fn run_claimed_naive(block: usize) {
    full_fence(); // unneeded: the output cannot depend on the claimant
    compute(block);
}

fn full_fence() {}
fn compute(_b: usize) {}
```

**opt_pattern draft:**

```rust
// optimized: contiguous OWN slice behind a PER-WORKER cursor (uncontended
// fast path), then COPRIME-stride theft — thief t visits victims
// (t + k*stride_t) so concurrent thieves hit different victims. The fence
// elides because of CLAIMANT-INVARIANCE: the block's output is independent
// of who claimed it, and its consumer is the next dispatch.
fn claim_own_then_steal(
    mine: &mut u64,
    victims: &mut [u64],
    my_id: usize,
    n: usize,
) -> Option<usize> {
    // fast path: my own contiguous slice, my own cursor — no contention
    let i = *mine;
    if i < n as u64 {
        *mine += 1;
        return Some(i as usize);
    }
    // theft: coprime stride per thief fans thieves out over victims
    let stride = my_id * 2 + 1; // odd strides are pairwise coprime-ish per worker
    let v = (my_id * 7 + 3) % victims.len(); // deterministic victim start
    let j = victims[v];
    if j < n as u64 {
        victims[v] += 1;
        return Some(j as usize);
    }
    let _ = stride;
    None
}

fn run_claimed(block: usize) {
    // NO fence: claimant-invariant output + next-dispatch consumer
    compute(block);
}
```

**Re-verify at pin:** steal.metal (own-slice cursors, the coprime victim stride, the claimant-invariance comment, surplus/missing worker clauses), probes p10 (the measured no-gain-on-dense-targets datum).

---

## R8 — `attention-partition-two-knob-shared-partial-workspace` (A, B−/B — upgraded)

**gpu_concept draft:**

> Split/partial attention parallelism has TWO INDEPENDENT knobs, and the partials WORKSPACE obeys a cross-layer sharing law. Knob one: TILE size = the local staging footprint (how much K/V a worker holds). Knob two: PARTITION size = the partial-result count (how many workers split the sequence). They tune SEPARATELY and in OPPOSITE directions ("Smaller partitions expose more parallel work; larger partitions reduce partial writes and merge work"). Partials carry ONLINE-SOFTMAX statistics (running max + denominator per row), published per (head, chunk, row-group), so a later merge renormalizes WITHOUT re-reading K/V. The cross-LAYER half is the memory law: sequential attention layers SHARE ONE WORKSPACE per partial-result kind, sized for the LARGEST consumer, with dependency barriers ordering the reuse — never a separate context-sized workspace retained per layer. The measured motivation the verdict lacked: 16 per-layer copies of the partial workspace on the 27B = 96 KB per capacity token = 12 GB at a 128K context — ALONE over the M5 Max budget; the SAFETY argument is layer ORDER ("each layer's core reads the previous layers' output, so one buffer per kind serves them all; the barrier pass orders the reuse"); the mechanism is one-buffer-per-name, sized max, grown in place. REGIME: split-attention prefill on memory-bound parts; layered models whose layers emit same-kind partials. ANTI-REGIME: cross-STREAM consumers (no barrier-orderable reuse) or genuinely overlapping partial lifetimes (then per-layer storage is required). Kinship: the split-K / online-softmax families own the accumulation mechanics (ambient here); `moe-per-expert-mixed-dispatch-worst-case-workspace` (B186) owns worst-case reservation PER EXPERT — this is the cross-LAYER SHARING half, ordered by barriers. Source: lithos-ai/lithos-metal @ 236475fa — docs/design/mixers.md + kernels/common/gqa_prefill.metal + monolith/compiler/shared_partials.py (16 copies × 96 KB = 12 GB @ 128K) + emit.py `scratch(shared=True)`. Batch (at landing) / Plan (Protocol A step 0) / Research 248 candidate 7.

**ref_shape draft:**

```rust
// naive: every layer allocates its OWN context-sized partial workspace —
// 16 layers x the same (heads, chunks, rows) shape = 16x the bytes, and
// at a 128K context the copies alone exceed the memory budget.
struct Partials {
    maxima: Vec<f32>,    // running max per (head, chunk, row-group)
    denominators: Vec<f32>,
}

fn layer_prefill_naive(layers: usize) -> Vec<Partials> {
    (0..layers)
        .map(|_| Partials {
            maxima: vec![0.0; 1024],      // a fresh context-sized buffer
            denominators: vec![0.0; 1024], // PER LAYER — 16 copies
        })
        .collect()
}
```

**opt_pattern draft:**

```rust
// optimized: ONE workspace per partial KIND, sized for the largest
// consumer, shared across sequential layers — safe because layer order
// serializes the lifetimes (each layer reads the previous layer's output),
// and the barrier pass orders the reuse. Grow in place, never re-allocate.
#[derive(Default)]
struct SharedPartials {
    maxima: Vec<f32>,
    denominators: Vec<f32>,
}

impl SharedPartials {
    fn ensure(&mut self, n: usize) {
        // sized for the largest consumer; grown in place, reused by every
        // later layer of the same kind
        if self.maxima.len() < n {
            self.maxima.resize(n, f32::NEG_INFINITY);
            self.denominators.resize(n, 0.0);
        }
    }
}

fn layer_prefill_shared(layers: usize, per_layer: usize, ws: &mut SharedPartials) {
    ws.ensure(per_layer); // ONE buffer serves all 16 layers
    for layer in 0..layers {
        // barrier-ordered reuse: this layer's core reads the previous
        // layer's output, so the scratch lifetimes never overlap
        attention_partial_into(ws, layer);
        merge_barrier();
    }
}

fn attention_partial_into(_ws: &SharedPartials, _layer: usize) {}
fn merge_barrier() {}
```

**Re-verify at pin:** mixers.md (the two-knob quote), gqa_prefill.metal (the per-(head, chunk, row-group) stats shape), shared_partials.py (the 16×96KB = 12GB motivation + the layer-order safety quote), emit.py `scratch(shared=True)` (one-buffer-per-name, sized max, grow-in-place).

---

## R9 — `encode-order-scheduling-bus-bound-first` (D, B− — pairs with R2)

**gpu_concept draft:**

> In a static command buffer where EXECUTION FOLLOWS ENCODE ORDER, the encode order is a scheduling AXIS distinct from barrier placement: when a bus-bound sibling dispatch would otherwise follow the big core it feeds, MOVING IT IN FRONT creates pair overlap no dependency pass can express — the pair overlaps the core itself instead of the projection it followed ("Move each sibling gate GEMV in front of the mixer core it was emitted after (profiles where the bus-bound dispatch must be encoded first for the pair to overlap)"). This composes with the barrier pass: the barrier pass decides WHICH commands may run together; encode order decides the ORDER THEY START. Profile-CONDITIONAL by construction — apply only where the profile shows the bus-bound dispatch must be encoded first for the pair to overlap. REGIME: static replay lanes (ICB / graphs) with measurable encode-order execution and a small bus-bound dispatch paired with a big core. ANTI-REGIME: out-of-order execution (the axis does not exist there), or dependencies that forbid the reordering. Kinship: `barrier-on-reader-open-group-conflict-swap-overlap` owns WHICH ops group; this owns WHERE in the group the small dispatch encodes; both are dependency-minimization faces of the same staging pass. Source: lithos-ai/lithos-metal @ 236475fa — monolith/compiler/emit.py `_bus_first()`. Batch (at landing) / Plan (Protocol A step 0) / Research 248 addendum C.

**ref_shape draft:**

```rust
// naive: encode order = authoring order. The tiny bus-bound gate GEMV
// encodes after the huge mixer core, so under encode-order execution it
// waits behind the whole core instead of overlapping it.
fn encode_naive() -> Vec<&'static str> {
    vec!["mixer_core_5ms", "gate_gemv_bus_bound_100us", "projection"]
}
```

**opt_pattern draft:**

```rust
// optimized: the bus-bound sibling MOVES IN FRONT of the core it feeds —
// profile-conditional: only where the profile says the pair overlaps by
// encoding the bus-bound dispatch first. Dependencies unchanged.
fn encode_bus_first(profile_says_overlap: bool) -> Vec<&'static str> {
    if profile_says_overlap {
        vec!["gate_gemv_bus_bound_100us", "mixer_core_5ms", "projection"]
    } else {
        vec!["mixer_core_5ms", "gate_gemv_bus_bound_100us", "projection"]
    }
}
```

**Re-verify at pin:** emit.py `_bus_first()` — the move + the profile-conditional quote.

---

## R10 — `derived-pack-immutable-original-content-addressed-atomic` (A, B−)

**gpu_concept draft:**

> Kernel-friendly weight REPACKS never mutate the original pack: the ORIGINAL stays immutable; DERIVED, page-aligned files hold the SAME codes reordered for the kernel's access pattern (row tile, reduction iteration, operand slot, lane, consecutive codes); the conversion runs at COMPILE time, outside the timed GPU program ("Conversion happens at compilation, outside the timed GPU program"); and derived files are CONTENT-ADDRESSED, published ATOMICALLY — a reader sees the old or the new file, never a half-written one. The cleanup half is where the memory win lives: after projections acquire derived layouts, the ORIGINAL windows shrink to their used spans — with two deliberate skips (skip the shrink when the span is ≤ 1 MiB; skip when the active fraction is ≥ 80%) and the clause that makes it mandatory when it applies: "A handful of norms must not keep gigabytes of replaced matrix payload resident in every ICB." One binding per original window is retained; NO tensor bytes change (the derived file is a REORDER, not a requantization). REGIME: engines with compile-time weight preprocessing serving multiple access-pattern consumers of one pack. ANTI-REGIME: runtime repacking (the conversion must stay out of the timed path or the rule's premise dies). Kinship: composes with the freeze/thaw + content-addressed pack laws (our substrate: NeuronShard freeze, ARTB vessels); the LOSSLESS + compile-time clauses are the fence separating this from every lossy-requantization rule. Source: lithos-ai/lithos-metal @ 236475fa — monolith/compiler/{fp8_tiles,nvfp4_tiles,moe_tiles,weight_windows}.py (compact_weights: the span-shrink + both skips + the gigabytes quote + one-binding retention). Batch (at landing) / Plan (Protocol A step 0) / Research 248 addendum D.

**ref_shape draft:**

```rust
// naive: repack IN PLACE for the new kernel's access pattern — the
// original layout is gone, every other consumer breaks, and the repack
// runs inside the serving path.
fn repack_in_place(pack: &mut [u8]) {
    // mutate the only copy: a half-written pack is unbootable, and the
    // repack time lands inside the timed program
    permute_for_kernel(pack);
}

fn permute_for_kernel(_p: &mut [u8]) {}
```

**opt_pattern draft:**

```rust
// optimized: the original pack is IMMUTABLE; a derived, content-addressed
// file holds the same codes in kernel order; conversion is compile-time;
// publication is atomic; and the original's unused spans shrink AFTER the
// derived layout exists (with the two skips).
#[derive(Clone, Copy, PartialEq)]
enum PackState {
    Original,
    Derived { active_fraction: f32 },
}

fn repack_original_immutable(
    original: &[u8],
    state: PackState,
    used_span_bytes: usize,
) -> Option<Vec<u8>> {
    // derived layout: SAME codes, kernel order — lossless by construction
    let derived = permute_copy(original);
    if let PackState::Derived { active_fraction } = state {
        let shrinkable = original.len() - used_span_bytes;
        // the two deliberate skips
        if used_span_bytes <= 1 << 20 || active_fraction >= 0.8 {
            return Some(derived);
        }
        let _ = shrinkable; // shrink the original's windows to the used span
    }
    Some(derived) // atomic publish is the caller's contract, never in-place
}

fn permute_copy(p: &[u8]) -> Vec<u8> { p.to_vec() }
```

**Re-verify at pin:** the four tiles/windows modules — the immutability + compile-time + atomic-publish quotes, compact_weights' ≤1 MiB / ≥80% skips + the gigabytes quote + one-binding retention.

---

## R11 — `autotune-fidelity-synthetic-carries-producer-params` (K, B−→C+)

**gpu_concept draft:**

> An autotuner's synthetic benchmark is EVIDENCE only if it reproduces the program's real data conditions — four clauses: (1) PRODUCER PARAMS — if the tuned variant's cost depends on a producer's parameterization (a fused norm folding per-SIMD-group partial sums-of-squares), the benchmark must allocate the producer's REAL count ("the benchmark must see the program's count" — a synthetic that under-allocates the partials measures a different kernel); (2) STREAM, NOT CACHE — each timing must stream enough replicated weight bytes (≥ 256 MB) that DRAM bandwidth, not cache hits, is what is measured ("≥ 256 MB streamed per timing"); (3) THE BAR IS THE DEFAULT PLUS NOISE — a candidate replaces the default only when it beats it by MORE than a fixed noise margin (NOISE_MARGIN = 0.03; min-of-5 after ≥ 30 ms warm) — conservative-vs-best is DELIBERATE: an autotuner that flips on noise poisons every downstream cell; (4) THE CACHE KEY IS THE FULL MECHANISM SIGNATURE per (chip, pack layout) — every axis that could change the winner is NAMED in the key (`gemv|{format}|{n}x{k}|R{rows}|{lane_order}|{scale_placement}|T{t}|{epilogue}|{norm}|shared-scales{d}|scale-order-{o}|P{stat_parts}`), or a stale entry answers for a different kernel. REGIME: any autotuner or bench harness choosing among kernel variants. ANTI-REGIME: none — these are discipline clauses, not a regime choice. Kinship: our G2 box-state law (the box is part of every latency claim) and the Bench-948 flip-matrix discipline are the same species host-side; this is the corpus's autotune-facing statement, with the full-signature cache key as the new atom. Source: lithos-ai/lithos-metal @ 236475fa — monolith/compiler/autotune.py (the producer-count clause, the 256 MB stream, NOISE_MARGIN, warm/min-of-5) + gemv_tuning.py (the cache-key format string). Batch (at landing) / Plan (Protocol A step 0) / Research 248 addendum E.

**ref_shape draft:**

```rust
// naive: the autotuner's synthetic under-allocates the producer's partials
// and measures from a cache-hot weight copy — it elects a variant that
// loses in the real program, and the cache key hides the difference.
fn autotune_naive(candidates: &[&str]) -> &str {
    let mut best = candidates[0];
    let mut best_t = f64::INFINITY;
    for c in candidates {
        let t = time_variant(c, /*partial_count=*/ 4, /*weights_mb=*/ 8.0);
        if t < best_t {
            best_t = t; // no noise margin: cache-warm noise elects the winner
            best = c;
        }
    }
    best
}

fn time_variant(_v: &str, _partials: usize, _mb: f64) -> f64 { 1.0 }
```

**opt_pattern draft:**

```rust
// optimized: the synthetic sees the PROGRAM'S producer params, streams
// >= 256 MB per timing, replaces the default only beyond a noise margin
// (min-of-5 after warm), and the cache key names EVERY mechanism axis.
const NOISE_MARGIN: f64 = 0.03;

fn autotune_fidelity(candidates: &[&str], program_partials: usize) -> String {
    let mut best = "default".to_string();
    let mut best_t = f64::INFINITY;
    for c in candidates {
        // (1) the program's real producer count — never a smaller synthetic
        // (2) >= 256 MB replicated weights per timing: DRAM, not cache
        let t = median_of_5(time_variant(c, program_partials, 256.0));
        // (3) conservative-vs-best: beat the default by MORE than noise
        if t < best_t * (1.0 - NOISE_MARGIN) {
            best_t = t;
            best = format!("{c}");
        }
    }
    // (4) the full signature IS the key — no stale entry answers for a
    // different kernel
    format!("gemv|R{}|epilogue|norm|P{} -> {}", program_partials, program_partials, best)
}

fn median_of_5(t: f64) -> f64 { t }
fn time_variant(_v: &str, _partials: usize, _mb: f64) -> f64 { 1.0 }
```

**Re-verify at pin:** autotune.py (the producer-count clause quote, the 256 MB stream quote, NOISE_MARGIN = 0.03, min-of-5 + ≥30 ms warm), gemv_tuning.py (the cache-key string verbatim).

---

## R12 — `moe-readiness-queue-no-grid-barrier-producer-scans-before-retire` (H, B− — rider-or-rule vs gpu-resident-moe-routing-no-host-sync)

**gpu_concept draft:**

> Grouped-MoE dispatch WITHOUT a grid-wide barrier, on a readiness queue: a bounded worker loop picks either READY down-work or an UNCLAIMED gate tile — never a wait on a named peer ("There is no grid-wide barrier or wait for a particular worker. A bounded worker loop chooses ready down work or an unclaimed gate tile."). Two correctness clauses carry it: the PRECEDING grouping dispatch RESETS the queue counters before every replay (never reset an ACTIVE queue — the same setup-vs-flight law the static-fusion queue setup carries); and when all gate jobs are claimed but some experts are still unready, each unready expert's LAST PRODUCER — the worker that just published that expert's grouping result — SCANS THE READY WORK ITSELF before retiring ("Its last producer continues after publication and scans ready work before retiring. Consequently idle workers can leave without losing future down jobs, freeing execution slots instead of polling other groups."). Numerics ride along: expert results round to BF16 PER THREADGROUP, then the final weighted reduction combines in the ORIGINAL FP32 order ("The router and final weighted reduction retain their original numerical order"). And the metadata-undo law: the original operations are retained as JSON metadata so an explicit tuning recipe can recover the UNFUSED program after automatic chip selection. REGIME: grouped MoE with ragged expert readiness on persistent/dispatch-free runtimes. ANTI-REGIME: dense uniform experts (a plain grouped GEMM has no readiness skew). Kinship: `gpu-resident-moe-routing-no-host-sync` owns the routing-no-host-sync CONTRACT — this is the QUEUE discipline under it (ADJUDICATE rider-vs-rule at conversion); `moe-per-expert-mixed-dispatch-worst-case-workspace` (B186) owns the workspace axis. Source: lithos-ai/lithos-metal @ 236475fa — monolith/compiler/{moe_ready,moe_down,moe_fusion}.py. Batch (at landing) / Plan (Protocol A step 0) / Research 248 addendum I.

**ref_shape draft:**

```rust
// naive: a grid-wide barrier before the down pass, and idle workers SPIN
// on named peers until every expert is ready — execution slots burn on
// polling, and resetting the queue mid-flight corrupts the next replay.
fn moe_naive(expert_ready: &[bool], workers: usize) {
    loop {
        if expert_ready.iter().all(|&r| r) {
            break;
        }
        // every worker polls EVERY expert: no one retires early
    }
    grid_barrier();
    down_pass(workers);
}

fn grid_barrier() {}
fn down_pass(_w: usize) {}
```

**opt_pattern draft:**

```rust
// optimized: a bounded worker loop picks ready down-work or an unclaimed
// gate tile — never a named-peer wait. The producer that publishes the
// last unready expert's readiness SCANS the ready work ITSELF before
// retiring, so idle workers may leave without losing future down jobs.
fn moe_ready_queue(
    expert_ready: &mut [bool],
    gate_claimed: &mut usize,
    gate_total: usize,
    my_worker: usize,
) -> Option<Job> {
    // counters were RESET by the preceding grouping dispatch — before the
    // replay, never mid-flight
    loop {
        if let Some(job) = ready_down_job(expert_ready) {
            return Some(job); // ready down-work first
        }
        if *gate_claimed < gate_total {
            *gate_claimed += 1;
            return Some(Job::Gate); // else claim a gate tile
        }
        // the last producer scans ready work before retiring: this worker
        // may LEAVE — future down jobs are not lost with it
        if my_worker == 0 && expert_ready.iter().any(|&r| !r) {
            continue; // the producing worker keeps scanning once more
        }
        return None; // retire: execution slot freed, not spent polling
    }
}

enum Job {
    Gate,
    Down(usize),
}

fn ready_down_job(_ready: &[bool]) -> Option<Job> { None }
```

**Re-verify at pin:** moe_ready.py (the no-grid-barrier quote, the reset-before-replay clause, the last-producer-scans quote), moe_down.py (the BF16-per-threadgroup + original-order FP32 combine quote), moe_fusion.py (the JSON metadata-undo law).

---

## Rider drafts

### RD1 → host `gdn-transition-replay-log-commit-many` (G family; locate host file at conversion by grep)

**Rider text draft:**

> RIDER (lithos-ai/lithos-metal @ 236475fa, Apache-2.0 — the PARITY-SLOT commit-by-recompute form): the family gains the double-buffered-slot member — SLOTS=2 recurrence states indexed BY STEP PARITY: the step's pass reads slot (step&1) and writes the OTHER, so the writer never aliases the window the readers replay (the measured failure that motivates it: with one slot and value-heads > key-heads, v-head blocks sharing a key head's conv window race — "a block that finishes first overwrites the window a slower sibling is still reading … a flaky oracle test under GPU contention"). COMMIT then needs NO journal and NO snapshot: a commit-mode re-run of the SAME kernel re-runs the committed tokens from the READ slot, overwriting the WRITE slot — rejected speculative tokens simply never reach persistent state. Two discipline clauses: placeholder bindings the kernel never touches must stay TRULY unwritten (the early version stored through a placeholder binding past a 16-byte value — "intermittent wrong-token / hang", found only under MTL_SHADER_VALIDATION=1 — the validation layer is part of the test matrix); and on the prompt side the parity invariant licenses dead-op ELIMINATION of the commit op entirely ("Forward GDN already stores the complete recurrence in the next parity slot").

**Re-verify at pin:** gdn_mixer.metal header (SLOTS/COMMIT + the race story + the placeholder-binding failure note), prefill.py `specialize_prompt` (the commit-op elimination quote).

### RD2 → host = the norm-commute carrier (fusion/epilogue family; locate at conversion by grep)

**Rider text draft:**

> RIDER (lithos-ai/lithos-metal @ 236475fa — the PREFOLD half of norm-commute): the commuted norm ships in two halves — the producer GEMV emits per-block PARTIAL sums-of-squares (STAT_OUT) and the consumer sums the n_blocks partials IN BLOCK ORDER ("the same statistic, deterministic, one dispatch fewer per norm"; a producer feeds ONE statistic — no double-claim), and the reciprocal-RMS fold itself runs ONCE in a separate one-SIMD-group kernel so every downstream projection applies the SAME scales after their FP32 product ("preserving the commuted-norm contract" — one fold, not per-consumer refolds). The honesty pair rides the rider: moving BF16 rounding changes the numerical path (the fused posture is default-ON WITH an opt-out switch), "Removing a dispatch is not sufficient evidence of a latency improvement", and the tests must distinguish agreement with the REORDERED formula from agreement with the ORIGINAL rounding boundaries.

**Re-verify at pin:** passes/fuse_norm.py (STAT_OUT + block order + one-statistic clauses), mlp_fusion.py `prefold_post_norm` (the one-SIMD-group fold + contract quote), mixers.md §Input-normalization fusion (the honesty pair verbatim).

### RD3 → host `lut-free-bit-spread-vadd4-unpack` (Batch 212; kernels-side file, locate at conversion)

**Rider text draft:**

> RIDER (lithos-ai/lithos-metal @ 236475fa — the FP8-SUBNORMAL density-split face, independently re-derived): the family's density-aware split CONFIRMED on a second engine with five exact FP8→BF16 decode variants (half-operand / vector / half / bits / lut / subtract), all sharing the shape: the COMMON case is pure bit arithmetic — the float-exponent form `(a<<4)+(120u<<7)` or the subtract form `(a<<20)+(121u<<23) − 1/64` — while the 8 SUBNORMAL inputs (a<8) take the split arm (a packed two-per-uint LUT or a tiny float product). Instance confirmation of the family's own "density-aware in-tree" law — lithos re-derived it independently; not a new family, a second in-tree witness.

**Re-verify at pin:** static_fusion.py `fp8_tile_decoder` — the five variant names + the two bit-arithmetic forms + the a<8 subnormal split.

---

## C-tier notes (the batch adjudicates rule / rider / instance-note)

- **T1 `read-before-write-carries-state-across-replays` (C+, eligibility predicate):** scratch-reuse eligibility is a PROVEN liveness interval, never absence-of-evidence — "Only a value with a known producer and a subsequent consumer is eligible. Unknown write metadata is deliberately insufficient to prove production. Values first read before being written may carry state across ICB replays." Keep-list = persistent state + initialized + inputs + observable outputs; slot packing prefers an already-large-enough allocation, else grows the closest. Source: arena.py `reuse_arenas`. Adjudicate: small A-category rule or instance-note under the arena/scratch family.
- **T2 `params-record-program-unique-identity` (C+, rider):** session buffers share BY NAME across programs for weights/constants; mutable config records must carry PROGRAM-UNIQUE identity — the measured failure (#113): a dynamic-T program and a static program at its t_max both said 'T8', and a static T=8 program's o_proj tiles read another GEMV's record (24576 rows into a 4096-row output — OOB loads AND stores, 16-second steps). Source: emit.py #113. Adjudicate: rider on the shared-workspace family (R8 / B186) or a small D rule.
- **T3 `ir-coverage-gate-fails-at-compile-with-missing-list` (C):** a program whose op has no kernel binding for the target profile DOES NOT BUILD — the check runs on the IR before emission and fails at compile time with the LIST of what is missing, never at run time. Source: coverage.py. Instance of our green-zero family (the `#![cfg]`-gated target that prints `ok. 0 passed`) — likely instance-note, not a kernel rule.

---

## GOAT postures (the batch finalizes)

- All twelve drafts: **NotBenchable-annotated** on our box — Metal/ICB-specific decision content; every source number is M5-class provenance-only, never an adoption bar (Research 248 caveat).
- **R3 optional staged GOAT:** a CubeCL-on-Metal polling-fan-in-ladder A/B (the B204→T11 pattern) — stage only if the batch wants a measured datum; NotBenchable is the default posture.
- **R8's 12 GB figure** is arithmetic (16 copies × 96 KB × capacity tokens), verifiable by multiplication — the annotation carries the arithmetic, not a box measurement.

## Provenance line for the batch landing

Source: lithos-ai/lithos-metal @ `236475fa716434807668562cd878ae623c2c1fd5` (Apache-2.0; blog lithosai.com/blog/lithos-metal; MPK-lineage task model, DSpark spec decoding; third_party/NOTICE carries MPK/Mirage/MLX/llama.cpp headers). Fixtures in this file are original minimal Rust (license hygiene law).
