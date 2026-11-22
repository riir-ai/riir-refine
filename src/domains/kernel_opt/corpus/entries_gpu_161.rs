//! Batch 398 — the mlx-serve RETURN-VISIT delta batch (Plan 398 / Research
//! 432): ddalcu/mlx-serve @ tip `96311b2638cf2f0ebdc7a2e39c85882a4fc4654a`
//! (2026-11-22 "Inkling 2 Mini", MIT; the verdict pin `2fe9be83` ONE commit
//! earlier — test-only delta), the Zig LLM server for Apple Silicon wrapping
//! MLX + embedded llama.cpp. The controlling PRIOR VISIT: Research 274 /
//! Batch 242 landed 31 rules @ `1015f78` (2026-10-14) — every candidate here
//! survived the B242 dedup re-run (coordinator-verified). EIGHT
//! decision-content laws: the IOGPU interactivity-preemption dispatch budget
//! (the LEAD), the dtype-constant accidental promotion (two independent
//! incidents in one source), the measured round-cost speculation controller
//! (the retired boot ladder riding its notes as the negative package), the
//! THREE-RULE speculation-gate cluster (the metric's denominator
//! normalization + the arch-normalized floor + the sticky decline with the
//! tail-match re-enable), the MXU/NAX exactness-scoping law (tile-scoped so
//! the op stays exact, error-shaped to match the incumbent, judged relative
//! to stock), and the verify-width laws (named scalars over indexed
//! accumulator arrays; a supported-width LIST, never a templated else). All
//! suggestion-only NotBenchable-by-domain (`bench_spec: None` — the win
//! metrics live in macOS IOGPU/MLX/NAX/serving-controller runtimes no
//! validator hosts; the Batch-136 annotation class; the verdict's
//! criterion-1 miss recorded in Research 432 §GOAT). Measured bases are
//! FOREIGN provenance (M3 Max / M4 Max / M5 Ultra, their packs) — never a
//! bar. Sec +1 `mdns-discovery-fed-egress-unauthenticated` (the corpus's
//! FIRST zero-config-discovery-fed-egress family seed, `DiscoveryEgress`
//! APPEND-ONLY) and `rust_perf` +1 `prefix-commit-priced-worthiness-typed-
//! outcome` (`entries_93`) land in their own files. Faces count-neutral on
//! their owners: F1 ordinal-map, F4 reference-round (+ the F15
//! exactness-contract table), F5 kv-dequant, F6 reuseaddr, F7 break-even-
//! flip (the SECOND face — B380's contrast arm stands), F8 latency-
//! histogram, F9 credential-argv (POSITIVE INSTANCE), F10 browser-reachable,
//! F11 gpu-residency (the ADMISSION half), F12 reserve-then-commit, F14
//! agent-child, F15 untrusted-download (POSITIVE INSTANCE), R8
//! admission-clones (the pad-to-qualify remedy direction), R10 bit-equal-
//! fusion (the carry protocol + cache-drift contrast), token-~55 on the B242
//! sigmoid-LUT rider, qmv2 atoms on the B379 lop3 rider. The 15-row
//! measured-negative package folds in-rule (anti-regimes/notes) — never
//! rules. Fixtures: `corpus/fixtures/b398_fixtures.rs` + the sec/perf twins
//! — standalone-compiled AND executed BEFORE entry, debug AND -O, original
//! Rust reproductions (never pasted Zig/Metal source; mlx-serve MIT, its
//! credited third-party kernels Apache-2.0/MIT per NOTICE —
//! transcription-only). T0 re-pin: full clone (the verdict's shallow pin
//! could not resolve the delta server-side; T0's full clone resolves 603
//! commits since B242's pin), tip ONE commit past the verdict pin. Outbound-
//! PR lane: EMPTY by construction (every mechanism is their own landed
//! decision). Batch 398 / Plan 398 / Research 432.

use crate::domains::kernel_opt::corpus::KernelRule;

// The corpus entry IS the payload (data, not logic) — the kernel build fn
// convention (the entries_popcorn precedent).
#[allow(clippy::too_many_lines)]
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — the batch lead (Research 432's B+): one dispatch whose work
        // scales with the scanned extent trips the macOS IOGPU interactivity
        // watchdog at long context; a WORK-UNIT budget splits the key axis
        // into BK-aligned chunks with exact online-softmax carry. ─────────
        KernelRule {
            rule_id: "io-gpu-preemption-dispatch-budget".to_string(),
            gpu_concept: "MECHANISM (dispatch/host-orchestration — macOS IOGPU interactivity-preemption avoidance on the DISPATCH side; the batch lead): a GPU dispatch whose work scales with the scanned extent (attention prefill: work = batch·Hq·qL·kL over the KV scan) monopolizes the device long enough at high kv_len to trip macOS IOGPU interactivity preemption — the system reclaims the dispatch's interactivity budget mid-kernel and long-context prefill collapses, and the penalty SCALES WITH DISPATCH LENGTH. The remedy is a WORK-UNIT BUDGET on the dispatch, with the split axis chosen so the split is EXACT: above the budget the host splits the KEY axis into separately-dispatched BK-aligned chunks — each a normal-length dispatch — carrying the online-softmax state across them bit-identically. The source's budget block, verbatim (src/sdk/blocks/attention/attn256.zig:101-111 @ 96311b26): 'One long dispatch scanning the whole KV monopolizes the GPU long enough at high kv_len to trip macOS IOGPU interactivity preemption, collapsing long-context prefill (oMLX issue #2225: the M3 Max cliff sits near ~1.5e9 work units ≈ 50 ms/dispatch; design ported from oMLX's qwen35_fa256_attention.py, Apache-2.0, oMLX by jundot). Above the budget (work = batch·Hq·qL·kL) the CAUSAL arm splits the key axis into separately-dispatched BK-aligned chunks with exact online-softmax carry (fp32 m/l/O buffers chained between dispatches — bit-identical to the single-dispatch result). The band arm never chunks: its in-kernel block skip already bounds per-dispatch work by the window.' THE BUDGET IS A MEASURED CONSTANT, verbatim (:113): 'pub const FUSED256_DEFAULT_DISPATCH_BUDGET: i64 = 250_000_000;' — 'The budget mirrors oMLX's 250M fallback (~23 ms/dispatch on the M4 Max at the kernel's measured ~1.1e10 work-units/s).' — the WORK-UNIT count, not a millisecond guess, is the portable quantity: the machine's cliff (the M3 Max's ~1.5e9 wu ≈ 50 ms) and the app's budget (250M wu ≈ 23 ms on the M4 Max) are separately measured numbers on different silicon, and neither is derived from the other. THE CARRY IS THE EXACTNESS HALF, verbatim (src/kernels/attention/attn256.metal:24-30): 'Budgeted kv-chunk dispatch (causal arm): this dispatch covers keys [k_begin, k_end); online-softmax state (m/l/unnormalized O) rides fp32 carry buffers between dispatches — exact register precision, so the chunked chain is bit-identical to one long dispatch. phase bit 0 = carry-in present, bit 1 = final chunk (normalize + bf16 store).' — a phase bitmask tells each chunked dispatch whether it has carry-in and whether it is final (normalize + store) or interior (carry out only), so ONE kernel serves every position in the chain and the host composes the split without a second kernel. THE CHUNK LENGTH IS A CLOSED FORM, verbatim (:115-127): 'kv-axis chunk length for the budgeted causal dispatch: the largest BK(32)-multiple keeping work-per-dispatch = batch·hq·ql·chunk within budget, floored at one BK block, capped at kl. budget <= 0 = one dispatch.' — round DOWN to the block multiple (never exceed the budget), floor at one block (a sub-block chunk is pure launch overhead), cap at kl (never an empty range); budget <= 0 disables the split entirely (the escape hatch). Engagement is COUNTED, never inferred (the dispatch-count test seam, :129-135). THE MEASURED WIN, verbatim (:86-99): 'the historical net-LOSS of every pre-budget ratio-gated variant (8K: fused-to-ratio-4 231.5, fused-to-ratio-2 232.9, composed 234.7 tok/s — monotonic: more fused = slower, despite the v2 kernel winning the µbench at kL <= 2*qL) was the macOS IOGPU interactivity-preemption class: one long dispatch scanning the whole KV gets preempted, and the penalty scales with dispatch length. With the key axis split into budget-sized dispatches (see FUSED256_DEFAULT_DISPATCH_BUDGET) the causal arm WINS the same same-session A/B on the 27B: +2.9%/+2.3%/+4.6% at 8K/16K/32K.' THE FOUNDING ANTI-REGIME IS THE PRE-BUDGET RATIO GATE (negative 7, folded in-rule): the variants gated on a kL:qL RATIO lost MONOTONICALLY (more fused = slower) on the live envelope while the same v2 kernel WON the µbench envelope (kL <= 2*qL) — the µbench envelope ≠ live envelope under preemption: the thing the ratio gated (fused width) was never the thing that lost (dispatch length); the budget gates the QUANTITY the watchdog actually prices. The kernel's own rejected variants corroborate the split shape (the notes package, measured never rules: BK=8 ping-pong — per-block rescale doubles; V-from-global fragments; K-only double-buffer; Q-smem BQ=32 — SLC thrash at long kL): the budget changed the DISPATCH SHAPE, not the kernel's inner schedule, and that is where the win lived. REGIME: long-context prefill (or any batched scan) on macOS where one dispatch's work grows with the scanned extent and the interactivity watchdog preempts long kernels; the measured basis (their silicon, provenance never a bar): +2.9/+2.3/+4.6% at 8K/16K/32K on the 27B; the sliding-band arm's own win (gemma-26B 99K: 317 → 712 tok/s) is the in-kernel block-skip arm, which this rule EXCLUDES from chunking. ANTI-REGIME: an arm whose in-kernel block skip already bounds per-dispatch work (the band arm: 'has no such competition... rides the master switch alone and never chunks' — the window IS the budget there); short context under the cliff (one dispatch is fastest; chunking pays launch overhead for nothing); non-macOS backends with differently-priced long-kernel watchdogs (re-derive the budget constant per platform, never port 250M); decode (per-step dispatch work is tiny — a prefill/batched-scan class). ⛔ KINSHIP (adjudicated both directions): fail-stop-deadline-trap-bounded-cross-rank-wait (gpu_148) owns the FAIL-STOP ENFORCEMENT axis — a cross-rank wait whose deadline EXPIRED faults the device deliberately; THIS owns PREVENTION (keep every dispatch under the machine's interactivity budget so the stall never forms; compose: the budget prevents the wait, the deadline bounds the wait that remains); graph-retrace-on-preemption (gpu_14) owns RECOVERY (re-record after a preemption has struck; compose on a Metal graph lane); gpu-residency-summed-beyond-process-and-cgroup-accounting (rust_perf entries_84) owns ADMISSION (does the load FIT before it starts) — THIS owns DISPATCH SHAPE (does one submitted unit monopolize the device mid-run); different lifecycle stages, compose on a serving lane; graph-captured prefill note: chunking multiplies dispatches per step, so a captured graph bakes the chunk count — re-capture on kv_len class change (the captured-graph family's shape axis, cited). NotBenchable-here (the win metric is macOS IOGPU interactivity-preemption behavior — a watchdog no validator hosts; the Batch-136 annotation class). The fixture models the budget split deterministically (the chunk closed form, the carry associativity vs a modeled whole-scan state, the band-arm exclusion). Source: ddalcu/mlx-serve @ 96311b2638cf2f0ebdc7a2e39c85882a4fc4654a (tip 2026-11-22 'Inkling 2 Mini', MIT; the verdict pin 2fe9be83 one commit earlier, test-only delta) — src/sdk/blocks/attention/attn256.zig:86-99 (the plain-causal doc + the pre-budget losses + the measured win), :101-111 (the budget block + the oMLX #2225 cliff), :113 (the 250M constant + the M4 Max pricing), :115-127 (fused256KvChunkLen closed form + the budget<=0 hatch), :129-135 (the dispatch-count test seam); src/kernels/attention/attn256.metal:24-30 (the carry buffers + the phase bitmask). Batch 398 / Plan 398 / Research 432."
                .to_string(),
            ref_shape: r#"// ref_shape: the whole KV scan is ONE dispatch — work = batch*H*Q*kL
// grows with the extent, and a live-envelope ratio gate (fuse while
// kL <= 2*qL) picks the dispatch shape from a µbench envelope the
// watchdog never reads. At 32K the dispatch crosses the machine's
// interactivity cliff and prefill collapses.

const M3_MAX_CLIFF_WU: f64 = 1.5e9; // the machine's cliff (oMLX #2225)

struct Prefill {
    batch: i64,
    heads: i64,
    q_len: i64,
}

impl Prefill {
    // one dispatch per prefill, however long the KV is
    fn dispatch_work(&self, kv_len: i64) -> f64 {
        (self.batch * self.heads * self.q_len * kv_len) as f64
    }
}

// the µbench ratio gate: fused while kL <= 2*qL — wins µbench, loses live.
fn ratio_gate_fused(q_len: i64, kv_len: i64) -> bool {
    kv_len <= 2 * q_len
}

let p = Prefill { batch: 1, heads: 48, q_len: 2048 };
let kv_len = 32_768;
let work = p.dispatch_work(kv_len); // 3.2e9 — far past the cliff
assert!(ratio_gate_fused(p.q_len, kv_len), "µbench says fused");
assert!(work > M3_MAX_CLIFF_WU, "one monopolizing dispatch");
"#.to_string(),
            opt_pattern: r#"// opt_pattern: a work-unit budget gates the DISPATCH (not the kernel);
// above it the key axis splits into BK-aligned chunks chosen by the
// closed form, each an ordinary dispatch; online-softmax state (m/l/
// unnormalized O) carries across chunks through a merge associativity
// the fp32 buffers preserve; a phase bitmask marks carry-in/final; an
// arm whose in-kernel block skip already bounds its work never chunks.

const BK: i64 = 32;
const DISPATCH_BUDGET_WU: i64 = 250_000_000; // ~23 ms on the M4 Max

fn kv_chunk_len(batch: i64, hq: i64, ql: i64, kl: i64, budget: i64) -> i64 {
    if budget <= 0 {
        return kl; // the hatch: one dispatch
    }
    let per_key = (batch * hq * ql).max(1);
    let mut chunk = budget / per_key;
    chunk = (chunk / BK) * BK; // round DOWN to the block multiple
    chunk.clamp(BK, kl) // floor at one block, cap at kl
}

// the carry: (m, l, o) as the fp32 buffers stand; the online-softmax
// merge is associative, so merging chunk states in any grouping
// reproduces the whole-scan state (bit-identical in exact arithmetic —
// the fp32 buffers are the precision contract).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Carry {
    m: f64,
    l: f64,
    o: f64,
}

fn merge(a: &Carry, b: &Carry) -> Carry {
    let m = a.m.max(b.m);
    let ea = (a.m - m).exp();
    let eb = (b.m - m).exp();
    Carry {
        m,
        l: a.l * ea + b.l * eb,
        o: a.o * ea + b.o * eb,
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Interior, // carry-in present, carry-out only
    Final,    // carry-in present, normalize + store
}

struct Prefill {
    batch: i64,
    heads: i64,
    q_len: i64,
}

impl Prefill {
    fn causal_chain(&self, kv_len: i64) -> Vec<(i64, i64, Phase)> {
        let chunk = kv_chunk_len(self.batch, self.heads, self.q_len, kv_len, DISPATCH_BUDGET_WU);
        let mut chain = Vec::new();
        let mut k = 0i64;
        while k < kv_len {
            let end = (k + chunk).min(kv_len);
            let phase = if end == kv_len { Phase::Final } else { Phase::Interior };
            chain.push((k, end, phase));
            k = end;
        }
        chain
    }

    // the band arm: in-kernel block skip bounds its work — never chunks
    fn band_chain(&self, kv_len: i64) -> Vec<(i64, i64, Phase)> {
        vec![(0, kv_len, Phase::Final)]
    }
}

let p = Prefill { batch: 1, heads: 48, q_len: 2048 };
let kv_len = 32_768;
let chain = p.causal_chain(kv_len);
let mut covered = 0i64;
for (k0, k1, phase) in &chain {
    let w = p.batch * p.heads * p.q_len * (k1 - k0);
    assert!(w <= DISPATCH_BUDGET_WU, "every dispatch within budget");
    assert!((k1 - k0) % BK == 0 || k1 == kv_len, "BK-aligned (tail capped)");
    assert_eq!(*k0, covered, "contiguous cover, no gap");
    covered = *k1;
    if *k1 == kv_len {
        assert!(*phase == Phase::Final);
    }
}
assert_eq!(covered, kv_len);
// carry associativity: chunked chain == one pass, any grouping
let a = Carry { m: 8.0, l: 2.0, o: 5.0 };
let b = Carry { m: 9.0, l: 2.5, o: 7.0 };
let c = Carry { m: 7.5, l: 1.2, o: 3.0 };
let ab_c = merge(&merge(&a, &b), &c);
let a_bc = merge(&a, &merge(&b, &c));
assert!((ab_c.m - a_bc.m).abs() < 1e-12);
assert!((ab_c.l - a_bc.l).abs() < 1e-9);
// the band arm: one dispatch regardless of budget
assert_eq!(p.band_chain(kv_len).len(), 1);
"#.to_string(),
        },
        // ── R2 — the accidental graph-wide promotion from a constant's
        // dtype: two independent incidents in one source (the Laguna f32
        // table 3x; the qwen4 scalar whose oracle PASSED) + the f16-table
        // narrow-at-load twin. ──────────────────────────────────────────
        KernelRule {
            rule_id: "constant-materializes-in-activation-dtype".to_string(),
            gpu_concept: "MECHANISM (dtype/layout — the ACCIDENTAL graph-wide promotion from a constant's dtype): a scalar or table the LOADER materializes in one dtype (the loader's convenience dtype, f32) and multiplies into narrower activations promotes the WHOLE residual stream — every downstream op upcasts and every later weight read pays the widened read — while op count, kernels, KV and host syncs stay innocent; the graph LOOKS unchanged and the math oracle still passes. TWO INDEPENDENT INCIDENTS in one source pin the class. The table form, verbatim (docs/gotchas/engine-mlx.md:348-356 @ 96311b26): '### A load-time f32 constant table made Laguna 3x slower — Defect: Laguna XS decoded at 40.5 ms/token, 2.36x slower than the mlxfast-challenge tree's unoptimized baseline on the same silicon. Cause: the YaRN mscale table was built f32 and multiplied into bf16 q/k, so the whole residual stream ran f32 from the first full-attention layer and every later weight read was upcast; op count, kernels, KV and host syncs were all innocent. Found by a rung ladder (diagProjBench) where the reconstruction diverged from the shipped function. Fix: constTableAs returns the table in the activation dtype (also the stock MLX path's numerics). 40.5 → 13.4 ms/token, within 2.2% of the frontier. Measure the reference tree interleaved in the same window: a mid-session 1.62x machine slowdown hit both alike. Ruled out by live A/B: command-buffer batching, wired limit, the pool cap, compiled forward, an intra-step async-eval ladder (worse at every stride: we already overlap at the step boundary). Guard: constTableAs hands a load-time f32 table back in the activation dtype.' The scalar twin is the SNEAKIER instance, verbatim (:845-848): '(1) mlx_array_new_float(0.25) is a real f32 array, so the hyper-connection inject weight and the whole stream ran f32 (the oracle passed at 0.99996 either way; [dtype-trace] had said so in every log)' — a single constant built by a constructor that always yields the widest dtype promotes the stream exactly like the table, and the math oracle CANNOT see it: the values are identical, only the dtype walk differs; the trace had logged it and nobody read the log. The third form narrows the OTHER direction, verbatim (:358-362): '### Per-channel f16 tables turned an f16 pack's residual f32 — Defect: Bonsai 27B ran 14.8% slow. Cause: the checkpoint is f16 (its config says bf16; read the checkpoint), and 1-D f16 norm weights promoted a bf16 residual at layer 0. Fix: narrowsLoadedF16 narrows 1-D f16 tables at load (matmul operands untouched: that is a kernel-selection change). Every forward path now arms a [dtype-trace] that logs only where the dtype moves (lint pins). Measurement trap: the first boot after another large load and thermal soak each reversed the sign; only paired A/Bs with a cooldown were stable.' — 1-D norm/bias tables are the promotion SOURCE at layer 0; narrow them AT LOAD, never the matmul operands (that changes kernel selection, a different axis); the fix also arms a dtype-MOVES-ONLY trace ('logs only where the dtype moves (lint pins)' — a trace that logs every op is noise nobody reads; the promotion-hunting trace logs the boundary crossings). THE CALIBRATION CASCADE rides the notes, verbatim (:362-366): '### What a 3x forward fix invalidated: two defaults and a spec gate — Defect: decisions calibrated on the slow Laguna forward were wrong on the fixed one.' — anything tuned on the promoted stream (a default, a spec gate threshold) must be re-measured after the dtype fix: the fix invalidates its own calibration history. THE MEASUREMENT FACE (positive instances, folded): the interleaved reference ('Measure the reference tree interleaved in the same window: a mid-session 1.62x machine slowdown hit both alike') is the bench_preflight interleaved-A/B shape; and the trap ('the first boot after another large load and thermal soak each reversed the sign; only paired A/Bs with a cooldown were stable') is the bench_preflight settle/cooldown discipline arrived at independently. REGIME: any graph runtime where constants enter from a loader (checkpoint tables, rope/YaRN scales, inject weights, 1-D norms) with a dtype independent of the activations; the measured bases (their silicon): 40.5 → 13.4 ms/token (3.0x), 14.8% on Bonsai 27B, the oracle-passing f32 stream. ANTI-REGIME: a DELIBERATE precision raise before a reduction (the upcast-before-reduction family — accuracy-motivated, chosen; THIS is the accident, and the fix removes it without changing accepted numerics — 'also the stock MLX path's numerics': match the reference's dtype walk); a constant the graph genuinely needs wide (accumulator constants — narrow where the CONSUMER reads, never blindly); matmul-operand narrowing (kernel-selection change — a different axis, refused at load). ⛔ KINSHIP (adjudicated both directions): upcast-before-reduction owns the DELIBERATE raise (accuracy-motivated, reduction-site) — THIS owns the ACCIDENTAL promotion (loader-dtype-motivated, stream-wide): opposite intents, opposite fixes; the grep evidence: 66 'promot' hits in the rust_perf corpus are all upcast-before-reduction-family prose — the accidental axis is unowned; reference-round-replicated-in-integer-arithmetic (rust_perf entries_88) owns ROUNDING replication of a reference's arithmetic — THIS owns dtype-walk promotion (the values pass, the WIDTH differs); decay-gate-dtype-one-point-zero-lock (entries_gpu_97) owns a dtype VALUE hazard at 1.0 (a gate exactly 1.0 never forgets) — a value edge, not a width-promotion axis. NotBenchable-here (the win is a graph-wide dtype walk in an MLX-class runtime; the Rust face is a promotion lattice a validator does not host). The fixture models the promotion: a wide constant multiplied into a narrow stream promotes every later op while the VALUES pass; the fix casts at load; the trace counts dtype MOVES only. Source: ddalcu/mlx-serve @ 96311b2638cf2f0ebdc7a2e39c85882a4fc4654a (MIT) — docs/gotchas/engine-mlx.md:348-356 (the Laguna table incident + the fix + the interleaved-reference face + the ruled-out list + the guard), :358-362 (the Bonsai f16-table incident + narrowsLoadedF16 + the dtype-trace + the trap), :362-366 (the calibration cascade), :845-856 (the qwen4 scalar twin + scalarOf + the oracle-passes disclosure + the full 41.9 → 57.4 tok/s three-part fix). Batch 398 / Plan 398 / Research 432."
                .to_string(),
            ref_shape: r#"// ref_shape: the loader builds the scale table f32 and multiplies it
// into the f16 stream — the multiply promotes, every later op carries
// the wide dtype, and the math oracle still passes (values identical;
// only the walk differs). The [dtype-trace] had said so in every log.

#[derive(Clone, Copy, PartialEq, Debug)]
enum D {
    F16,
    F32,
}

#[derive(Clone)]
struct Tensor {
    d: D,
    vals: Vec<f64>,
}

// the loader's convenience constructor: ALWAYS the wide dtype
fn const_table_f32(vals: &[f64]) -> Tensor {
    Tensor {
        d: D::F32,
        vals: vals.to_vec(),
    }
}

fn mul(a: &Tensor, b: &Tensor) -> Tensor {
    // promotion: the wider dtype wins the result
    let d = if a.d == D::F32 || b.d == D::F32 { D::F32 } else { D::F16 };
    Tensor {
        d,
        vals: a.vals.iter().zip(&b.vals).map(|(x, y)| x * y).collect(),
    }
}

let table = const_table_f32(&[0.25, 1.0, 0.5]); // built f32 by the loader
let mut stream = Tensor { d: D::F16, vals: vec![1.0, 2.0, 3.0] };
let truth: Vec<f64> = stream
    .vals
    .iter()
    .zip(&table.vals)
    .map(|(x, y)| x * y)
    .collect();
stream = mul(&stream, &table); // promotes HERE
stream = mul(&stream, &Tensor { d: D::F16, vals: vec![1.0; 3] }); // still wide
assert_eq!(stream.d, D::F32, "the whole stream promoted");
// the oracle: values identical either way — it cannot see the promotion
for (s, t) in stream.vals.iter().zip(&truth) {
    assert!((s - t).abs() < 1e-12, "the oracle passes; the width is the cost");
}
"#.to_string(),
            opt_pattern: r#"// opt_pattern: the constant is materialized IN THE ACTIVATION DTYPE
// at load (constTableAs / scalarOf(v, dtype)); 1-D f16 tables narrow at
// load (matmul operands untouched); a dtype-trace counts MOVES only;
// anything tuned on the old stream is re-measured after the fix.

#[derive(Clone, Copy, PartialEq, Debug)]
enum D {
    F16,
    F32,
}

#[derive(Clone)]
struct Tensor {
    d: D,
    vals: Vec<f64>,
}

fn const_table_as(vals: &[f64], activation: D) -> Tensor {
    // the values narrow exactly at load; the stream never widens
    Tensor {
        d: activation,
        vals: vals.to_vec(),
    }
}

fn scalar_of(v: f64, activation: D) -> Tensor {
    Tensor { d: activation, vals: vec![v] }
}

fn narrows_loaded_f16(t: &Tensor) -> Tensor {
    // 1-D tables narrow at load; matmul operands untouched
    if t.vals.len() <= 4 && t.d == D::F32 {
        Tensor { d: D::F16, vals: t.vals.clone() }
    } else {
        t.clone()
    }
}

fn mul(a: &Tensor, b: &Tensor) -> Tensor {
    let d = if a.d == D::F32 || b.d == D::F32 { D::F32 } else { D::F16 };
    Tensor {
        d,
        vals: a.vals.iter().zip(&b.vals).map(|(x, y)| x * y).collect(),
    }
}

// the trace: counts dtype MOVES only (lint pins), never ops
fn dtype_moves(path: &[D]) -> usize {
    path.windows(2).filter(|w| w[0] != w[1]).count()
}

let table = const_table_as(&[0.25, 1.0, 0.5], D::F16);
let inject = scalar_of(0.25, D::F16);
let norm = narrows_loaded_f16(&Tensor { d: D::F32, vals: vec![0.9] });
let mut stream = Tensor { d: D::F16, vals: vec![1.0, 2.0, 3.0] };
let mut walk = vec![stream.d];
stream = mul(&stream, &table);
walk.push(stream.d);
stream = mul(&stream, &inject);
walk.push(stream.d);
stream = mul(&stream, &norm);
walk.push(stream.d);
assert_eq!(stream.d, D::F16, "the stream stays narrow");
assert_eq!(dtype_moves(&walk), 0, "no moves to log");
"#.to_string(),
        },
        // ── R3 — the measured round-cost speculation controller: width
        // priced from a measured per-(width × KV bucket) table with
        // contamination guards; the retired boot ladder rides the notes.
        // ────────────────────────────────────────────────────────────────
        KernelRule {
            rule_id: "measured-round-cost-spec-controller".to_string(),
            gpu_concept: "MECHANISM (measurement-honesty / serving control — the measured-cost speculation controller): speculation WIDTH choices read a MEASURED per-model table — round milliseconds and emitted tokens per (width × KV bucket), sampled from solo single-chunk rounds, blended by EMAs with stale-cell handling — and the EV plan reads TIME from the table while a width TRIAL explores; the table is guarded against contaminated cells and extrapolated by law, never by raw slope. The source's design section, verbatim (docs/gotchas/engine-mlx.md:826-836 @ 96311b26): 'Defect: (design) no chip row is right across packs (peer sweeps put the best DFlash block at 8/6/none/5/4 across five cells). Fix: engines/mlx/spec/round_cost.zig, a per-model table of round ms and emitted tokens per width and KV bucket from solo single-chunk rounds, EMAs with stale-cell blending; the EV plan reads time from it; a width trial explores. Traps found only by the simulated-loop test: extrapolating a cliff's slope downward priced narrower widths at zero; a shallow slope extrapolated upward priced wide widths as free (past the widest measured, each position costs max(last slope, prior marginal)); a struct literal assigning m_lo then reading it for m_hi aliased (build plans from a scalar); one sample per cell let a transition-inflated round anchor the plan (MIN_SAMPLES 3, transitions dropped, 5% hysteresis); a width ≥ 20% worse per token on its first sample is settled at once. The per-silicon row stays the cold-start cap (mtp_depth_free). Persistence is opt-in, keyed on chip/model/quant/OS/engine build. The DFlash WidthChooser stays opt-in: a MoE verify's cost is content-dependent.' THE CONTAMINATION GUARDS are the pricing table's own admission control, verbatim (:1006-1012): '...interleaved prefill drops the round clock; folds and loads refuse samples past IMPLAUSIBLE_STEP 1.5x of the nearest trusted narrower cell, SELF_SPIKE 3x of a mature cell, and (load sweep only) IMPLAUSIBLE_WIDER 1.25x of the nearest wider cell; per-token cost divides by the monotone envelope planTok. Uniform contamination is invisible to ratios: delete the table.' — the constants live in round_cost.zig (:89 MIN_SAMPLES 3, :134 IMPLAUSIBLE_STEP 1.5, :139 IMPLAUSIBLE_WIDER 1.25, :142 SELF_SPIKE 3.0), and the step check is EXPONENTIATED by the width distance (:253-260): 'ms at width exceeds IMPLAUSIBLE_STEP per step over the nearest trusted narrower cell' → 'ms > below.ms * IMPLAUSIBLE_STEP^(width - w)', skipping untrusted anchors ('if (below.n < MIN_SAMPLES) continue') — a cell two widths narrower allows 1.5², one width allows 1.5; 'trusted' means MIN_SAMPLES met. THE KEYSTONE LAW is the uniform-contamination verdict: ratios between cells cannot detect contamination that hits all cells alike ('Uniform contamination is invisible to ratios: delete the table') — the remedy is deleting the table and falling back to the per-silicon cap, never re-weighting. THE PER-SILICON CAP is the fallback layer, verbatim (:735-738): 'Defect: NO_WIDE_LANE_BLOCK_CAP = 5 was an M4 measurement applied to every non-NAX machine, blocking oMLX's measured M3 Ultra block-8 win. Fix: blockCapForMachine(chip) keyed on the sysctl brand string (Ultra and Max share a GPU arch name): M3 Ultra 8, else 5; resolveBlockSize takes the cap as a parameter (hermetic tests); the ready line names the row.' — a per-MACHINE table keyed on the brand string, hermetic tests over the keying, and the ready line NAMING the row chosen (the receipt law). THE RETIRED BOOT LADDER rides the notes as the founding negative (the table exists because the ladder failed), verbatim (:810-818): 'Defect: hand-typed per-silicon caps and four refits of the EV cost surface, each fitted at one context length. A load-time probe of the verify ladder reproduced the M4 caps but decided nothing better, and its two extra terms lost: a learned per-token KV term made deep positions look cheaper (the attention each query row does grows with context, O(k x L)) and bought extensions nobody accepted (-2.7% at 21k); the fitted marginals under-priced depth 2.3x. Cause: every width decision is accepted tokens over round time and a forward ladder measures cost alone (a forward is not a round). Kept from it: a request-scoped learner can never see cross-request variation (state belongs on the model); contention only adds time; skipped test arms read as passes when their model env is unset.' — 'a forward is not a round' is the UNIT law: the measured unit must be the ROUND (drafts + verify + emission), never the forward; and the kept lessons are negative-package gold (request-scoped learners cannot see cross-request variation; skipped test arms read as passes when their model env is unset — the green-zero rule's serving twin). The wider measurement-honesty package corroborates in-notes (measured, never rules): the lm_head prune won µbench and 3 of 20 live pairs and was deleted — 'a head readout is judged live, never by µbench'; an ablation leaving an output unwritten measures NaN routing ('garbage activations route every MoE layer to the same cached experts; ablate with sane values' — the sane-stand-ins law); the µbench traps (hot weights hid a latency-bound read; a prompt nonce naming each boot's arm re-rolled generations; an A/B arm silently running the other arm's config). REGIME: speculation width / block-size / any serving control priced per (config × workload bucket) where the pricing data comes from the system's own rounds; the simulated-loop test is the authoring instrument ('Traps found only by the simulated-loop test'). ANTI-REGIME: a boot-time load probe (measures forwards, not rounds — the retired ladder); a GLOBAL per-silicon constant applied across machines (the NO_WIDE_LANE_BLOCK_CAP defect); µbench-gated adoption of a live cost behavior. ⛔ KINSHIP (adjudicated both directions): speculation-scheduler-break-even-flip (gpu_29) owns the ENGAGE/BYPASS FLIP axis (cumulative saved-vs-extra, family dependence, margin band) — THIS owns the WHICH-WIDTH pricing axis (a measured table + EV plan + guards; compose: gpu_29 decides whether to speculate this request, this table prices which width); specdec-composite-decomposed-into-per-step-time-and-accepted-length (rust_perf entries_84) owns the composite-metric DECOMPOSITION axis (accepted ÷ step_time derived, never the judge) — THIS owns the per-cell MEASURED TABLE the decomposition feeds from; resource-limit-first-dispatch-probe-decline-latch (entries_gpu_97) owns the resource-limit latch (decline on a resource limit, per-width memo) — a different decision class, cross-ref; the calibration-gate vocabulary (per-group evidence, pre-registered bars, verdict-first) is the epistemic frame the guards implement. NotBenchable-here (the win is serving-runtime width selection priced from the engine's own rounds). The fixture models the table + guards deterministically (MIN_SAMPLES trust, the exponentiated step refusal, the self-spike, the ≥20% settle, the 5% hysteresis, the extrapolation cap). Source: ddalcu/mlx-serve @ 96311b2638cf2f0ebdc7a2e39c85882a4fc4654a (MIT) — docs/gotchas/engine-mlx.md:810-818 (the retired boot ladder + 'a forward is not a round' + the kept lessons), :826-836 (the measured table + the traps + the caps + the opt-ins), :1006-1012 (the contamination guards + 'Uniform contamination is invisible to ratios: delete the table' + the planTok envelope), :735-738 (blockCapForMachine + the ready-line receipt), src/engines/mlx/spec/round_cost.zig:89 (MIN_SAMPLES 3) + :134 (IMPLAUSIBLE_STEP 1.5) + :139 (IMPLAUSIBLE_WIDER 1.25) + :142 (SELF_SPIKE 3.0) + :234/:253-260 (the trusted-neighbor exponentiated check). Batch 398 / Plan 398 / Research 432."
                .to_string(),
            ref_shape: r#"// ref_shape: the width plan extrapolates slopes from the table's edge
// and anchors on ONE sample per cell — a cliff's slope priced downward
// reads narrower widths as free; a shallow slope priced upward reads
// wide widths as free; a transition-inflated first sample anchors the
// plan; contaminated cells are indistinguishable from real ones.

#[derive(Clone)]
struct Cell {
    ms: f64, // round milliseconds at this width/bucket
    n: u32,  // samples collected
}

struct Table {
    cells: Vec<Vec<Cell>>, // [width][kv_bucket]
}

impl Table {
    fn per_tok(&self, w: usize, b: usize) -> f64 {
        self.cells[w][b].ms / 12.0 // fixed token yield, for the model
    }

    // BUG: extrapolate the slope past the widest measured width
    fn plan_price(&self, width: usize, b: usize) -> f64 {
        let w = width.min(self.cells.len() - 1);
        if w == width {
            return self.per_tok(w, b);
        }
        let slope = self.per_tok(w, b) - self.per_tok(w - 1, b);
        // a shallow slope priced upward: wider reads (nearly) free
        self.per_tok(w, b) + slope * ((width - w) as f64)
    }

    // BUG: any single sample anchors the plan (no MIN_SAMPLES)
    fn adopt_wider(&self, w: usize, b: usize) -> bool {
        self.per_tok(w, b) < self.per_tok(w - 1, b)
    }
}

let mut cells = vec![vec![Cell { ms: 100.0, n: 1 }; 2]; 3];
cells[2][0] = Cell { ms: 300.0, n: 1 }; // a transition-inflated sample
let t = Table { cells };
// the slope extrapolation: width 5 priced cheaper than width 4
assert!(t.plan_price(5, 0) < t.plan_price(4, 0), "wide reads free");
assert!(t.adopt_wider(2, 0), "one inflated sample anchors");
"#.to_string(),
            opt_pattern: r#"// opt_pattern: MIN_SAMPLES anchors everything trusted; a first sample
// >= 20% worse per token settles the width AT ONCE; refusals price
// against the nearest TRUSTED neighbor (the step check EXPONENTIATED
// by the width distance; the self-spike vs the mature EMA); past the
// widest measured, each position costs max(last slope, prior marginal)
// — never a raw extrapolation; adoption clears a hysteresis band.

const MIN_SAMPLES: u32 = 3;
const IMPLAUSIBLE_STEP: f64 = 1.5;
const SELF_SPIKE: f64 = 3.0;
const SETTLE_WORSE: f64 = 1.20;
const HYSTERESIS: f64 = 0.05;

#[derive(Clone)]
struct Cell {
    ms: f64,
    n: u32,
    ema: f64, // the mature EMA the self-spike prices against
}

fn trusted(c: &Cell) -> bool {
    c.n >= MIN_SAMPLES
}

fn per_tok(c: &Cell) -> f64 {
    c.ms / 12.0
}

/// the exponentiated step refusal: scan down to the nearest TRUSTED
/// narrower cell; refuse past IMPLAUSIBLE_STEP^(width - w).
fn sample_admitted(width: usize, sample_ms: f64, below: Option<(&Cell, usize)>) -> bool {
    if let Some((c, w)) = below {
        if trusted(c) {
            let bound = c.ms * IMPLAUSIBLE_STEP.powi((width - w) as i32);
            return sample_ms <= bound;
        }
    }
    true // no trusted anchor: admit, keep sampling (never refuse blind)
}

fn self_spike(sample_ms: f64, mature_ema: f64) -> bool {
    mature_ema > 0.0 && sample_ms > mature_ema * SELF_SPIKE
}

/// past the widest measured width: each position costs
/// max(last slope per position, prior marginal per position).
fn extrapolated_price(last_per_tok: f64, prior_marginal: f64, extra_widths: usize) -> f64 {
    let slope = last_per_tok; // the last slope IS the per-position cost floor
    last_per_tok + slope.max(prior_marginal) * extra_widths as f64
}

let trusted_cell = Cell { ms: 100.0, n: 5, ema: 98.0 };
// a 2.0x jump over a trusted narrower cell: refused (1.5^1 = 1.5x bound)
assert!(!sample_admitted(1, 200.0, Some((&trusted_cell, 0))));
// two widths away: 1.5^2 = 2.25x bound — the same sample admits
assert!(sample_admitted(2, 200.0, Some((&trusted_cell, 0))));
// an UNTRUSTED anchor (n < 3) never refuses: admit, keep sampling
let untrusted = Cell { ms: 60.0, n: 1, ema: 0.0 };
assert!(sample_admitted(1, 200.0, Some((&untrusted, 0))));
// a 4x self-spike over the mature EMA: refused
assert!(self_spike(400.0, 98.0));
// a first sample 25% worse per token: settled at once
let incumbent = per_tok(&trusted_cell);
let first = 12.5; // 150 ms / 12 tokens
assert!(first > incumbent * SETTLE_WORSE, "settled immediately");
// adoption clears 5% hysteresis
let challenger = per_tok(&trusted_cell) * 0.97;
assert!(challenger >= incumbent * (1.0 - HYSTERESIS), "within band: no flip");
// the extrapolation cap: extra positions cost the slope, never less
let p = extrapolated_price(0.5, 0.5, 4);
assert!((p - 2.5).abs() < 1e-9, "max(last slope, prior marginal) per position");
"#.to_string(),
        },
        // ── R4 — the speculation gate's DENOMINATOR law: normalize by the
        // drafts-per-round the configuration moves, and count the unit the
        // tax lands on (the two-metric split). ────────────────────────────
        KernelRule {
            rule_id: "specdec-gate-metric-normalized-by-drafts-per-round".to_string(),
            gpu_concept: r#"MECHANISM (serving control — the speculation gate's DENOMINATOR law): an acceptance gate's metric must be normalized by the denominator that MOVES WITH CONFIGURATION (drafts per round), and the tax-paying unit must BE the denominator — a gate that counts only verify ROUNDS never sees a workload whose cost lands on every STEP. The founding incident is the mis-normalized gate, verbatim (src/engines/mlx/spec/gates.zig:18-28 @ 96311b26): 'History: pre-v5 this gate compared accepted/attempted (per-round average) against 0.30 — but with block_size=4 the max value of that ratio is 3.0, so the 0.30 threshold corresponded to ~10% per-draft probability, well below where verify+draft overhead actually breaks even. Empirically creative-content workloads regress at 22-47% per-draft acceptance while the gate stayed off (per-round avg 0.66-1.58, all above 0.30). Switching to a per-draft probability with threshold 0.50 cleanly cuts off the regressing tail while leaving heavy-echo workloads (84-97% per-draft) running unmolested.' — the metric's RANGE moves with block_size, so an ABSOLUTE threshold on it silently re-prices itself with configuration: 0.30 was meant as an acceptance floor and armed as ~10%; regressing workloads (22-47% per-draft) read 0.66-1.58 per-round and sailed through ARMED. The fix divides by the config quantity explicitly, verbatim (:35-42): 'drafts_per_round is the number of drafted tokens proposed in each verify (= block_size - 1 for the drafter, or pld_draft_len for PLD); we divide accepts by attempts × drafts_per_round to get the per-draft acceptance probability' + the degenerate-config guard: 'drafts_per_round == 0 is treated as "no speculative work happens per round" → never trip (defensive — current callers always pass >= 1).' Threshold RUNTIME_GATE_MIN_PER_DRAFT_RATE 0.50 (:28). THE SECOND DENOMINATOR LAW is the two-metric split, verbatim (:88-98): 'The per-draft gate above only counts verify ROUNDS, so a workload where the n-gram lookup rarely matches never accumulates enough "attempts" to trip it — yet every no-match step pays PLD's unpipelined cold forward (measured −14% vs the async-pipelined next() on creative content). The yield gate instead counts EVERY enabled-mode nextPld step: if the speculation is yielding fewer than YIELD_GATE_MIN_YIELD extra (drafted, accepted) tokens per step after YIELD_GATE_WARMUP steps, the cold-path tax outweighs the wins → disable. Paired with specShouldReenable, which flips PLD back on when the generated tail turns repetitive.' — same system, two gates, two denominators, because the COST lands on different units: verify rounds for the drafter path, every enabled step for the lookup path; a gate whose denominator misses the paying unit under-counts its own tax to zero. THE WARMUP IS A PRICED QUANTITY, verbatim (:98-107): 'Warmup 8 (not higher): the re-enable check bounds the cost of a premature trip to ≤SPEC_REENABLE_INTERVAL pipelined-fallback steps, so we can gate early and recover the pipeline sooner on novel content.' + the doc row: 'Every warmup step pays PLD's UNPIPELINED cold forward (plus a synchronous host read of the sampled token) against next()'s async-pipelined step, so the tax is a fraction of the AR step cost; 32 was calibrated before the Laguna mscale fix made that step ~3x cheaper. See the yield-gate warmup is 8 test for the sweep.' — the warmup FOLLOWS the per-step cost (32 → 8 after the step got 3x cheaper) and is swept by test; and the per-draft gate's own warmup carries its trade verbatim (:14-17): 'Lower values make the gate trip sooner, reducing regression-tail damage at the cost of fewer chances for slow-warmup workloads to amortize spec overhead.' The sampled-draft corollary rides the notes (measured, never a rule): acceptance 1 − TV(p, q) needs the DRAFT distribution close in shape — sampled/sharpened drafts lose to greedy on low-entropy content — so a gate fed sampled drafts reads a structurally lower rate; the gate cannot rescue a drafter that samples against the metric's shape assumption. REGIME: any gated adaptive path whose metric's range or tax-paying unit moves with configuration (speculation gates, batch gates, retry gates). ANTI-REGIME: a metric whose denominator is config-stable (a per-token rate over a fixed token unit); a gate whose cost lands on the same unit it counts. ⛔ KINSHIP (adjudicated both directions): specdec-composite-decomposed-into-per-step-time-and-accepted-length (rust_perf entries_84) owns the DERIVED-COMPOSITE axis (tok/s = accepted ÷ step_time; the composite is derived, never the judge) — THIS owns the THRESHOLD's DENOMINATOR (which unit the bar reads; compose: the composite supplies the rate, the gate's denominator decides what it means); speculation-scheduler-break-even-flip (gpu_29) owns the engage/bypass flip on cumulative cost — a different decision from a different metric; resource-limit-first-dispatch-probe-decline-latch (entries_gpu_97) is a resource-limit latch, cross-ref only; composes specdec-gate-threshold-arch-normalized (the FLOOR law — that rule says WHICH floor per arch/class, this says WHICH UNIT the bar reads). NotBenchable-here (the win is a serving-runtime gate's cutoff behavior). The fixture pins the arithmetic: block 4, per-round 0.66 above 0.30 (the armed lie) vs per-draft 0.165-0.47 below 0.50 (the truth); the yield gate's step denominator trips where the round denominator cannot accumulate. Source: ddalcu/mlx-serve @ 96311b2638cf2f0ebdc7a2e39c85882a4fc4654a (MIT) — src/engines/mlx/spec/gates.zig:14-17 (the warmup trade), :18-28 (the pre-v5 history + the 0.50 fix), :29-53 (runtimeGateShouldDisable + the drafts_per_round division + the zero guard), :88-98 (the yield-gate two-metric split + the −14% cold-path measure), :98-107 (the priced warmup 8 + the 32-was-calibrated note + the sweep test), :108-110 (YIELD_GATE_MIN_YIELD 0.25 + yieldGateShouldDisable). Batch 398 / Plan 398 / Research 432."
               "#.to_string(),
            ref_shape: r#"// ref_shape: the gate compares accepted/attempted (per-ROUND average)
// against an absolute bar. With block_size 4 the ratio's max is 3.0, so
// a 0.30 bar is ~10% per-draft: regressing workloads (22-47% per-draft)
// read 0.66-1.58 per-round and sail through ARMED.

fn gate_should_disable_buggy(attempted: u64, accepted: u64, warmup: u64) -> bool {
    if attempted < warmup {
        return false;
    }
    let avg = accepted as f64 / attempted as f64; // per-round average
    avg < 0.30 // the range moves with block_size; the bar does not
}

// block 4, 47% per-draft acceptance: 100 rounds, 188 accepted drafts
let regressing = gate_should_disable_buggy(100, 188, 5);
assert!(!regressing, "per-round 1.88 > 0.30: the gate stays ARMED");
"#.to_string(),
            opt_pattern: r#"// opt_pattern: the gate divides by the CONFIG quantity (drafts per
// round) and counts the unit that pays the tax (every enabled step for
// the lookup path); the zero-work config never trips; the warmup is
// priced and swept.

const RUNTIME_GATE_MIN_PER_DRAFT_RATE: f64 = 0.50;
const RUNTIME_GATE_WARMUP: u64 = 5;
const YIELD_GATE_MIN_YIELD: f64 = 0.25;
const YIELD_GATE_WARMUP: u64 = 8;

fn gate_should_disable(attempted: u64, accepted: u64, drafts_per_round: u32) -> bool {
    if attempted < RUNTIME_GATE_WARMUP {
        return false;
    }
    if drafts_per_round == 0 {
        return false; // no spec work per round: never trip (defensive)
    }
    let proposed = attempted * drafts_per_round as u64;
    let rate = accepted as f64 / proposed as f64; // per-draft probability
    rate < RUNTIME_GATE_MIN_PER_DRAFT_RATE
}

// the yield gate's denominator is the STEP the tax lands on
fn yield_gate_should_disable(steps_total: u64, accepted: u64) -> bool {
    if steps_total < YIELD_GATE_WARMUP {
        return false;
    }
    (accepted as f64 / steps_total as f64) < YIELD_GATE_MIN_YIELD
}

// block 4, the same workload: per-draft 0.47 < 0.50 — the gate TRIPS.
assert!(gate_should_disable(100, 188, 4), "per-draft 0.47: disabled");
// the heavy-echo workload stays unmolested: 84% per-draft
assert!(!gate_should_disable(100, 336, 4), "0.84 stays on");
// zero-work config never trips
assert!(!gate_should_disable(100, 0, 0));
// the lookup path: 60 enabled steps, 5 accepted tokens — verify rounds
// never accumulated (rare matches); the STEP denominator sees the tax.
assert!(yield_gate_should_disable(60, 5), "0.083/step < 0.25: disabled");
"#.to_string(),
        },
        // ── R5 — the ARCH-normalized threshold: the break-even floor is a
        // property of the verify architecture (dense vs sparse/MoE) and the
        // request class, applied after width scaling. ─────────────────────
        KernelRule {
            rule_id: "specdec-gate-threshold-arch-normalized".to_string(),
            gpu_concept: "MECHANISM (serving control — the ARCH-normalized threshold): a speculation break-even threshold is a property of the VERIFY ARCHITECTURE (dense trunk vs sparse/MoE) and the REQUEST CLASS (thinking vs plain), never one global number — and the floor is applied AFTER width scaling as an absolute per-arch minimum. The source's MoE floor, verbatim (src/engines/mlx/spec/gates.zig:63-70 @ 96311b26): '/// Absolute floor for a SPARSE target, applied after the width scaling. See scheduler.dflashGateMinimum for the measurement it comes from — a MoE verify reads every expert its block's positions route to, so its break-even acceptance is several times a dense trunk's.' + 'pub const DFLASH_MOE_GATE_MIN_ACCEPTED_PER_ROUND: f32 = 1.8;' — a MoE verify pays expert routing + gather per block position, so the acceptance that breaks even is HIGHER than a dense trunk's at the same width; the floor number is the measured break-even boundary, and it is a FLOOR: the width scaling may raise the bar, never lower it below the arch's minimum. THE CLASS NORMALIZATION beside it: the dense floor DFLASH_GATE_MIN_ACCEPTED_PER_ROUND 2.0 vs the thinking floor DFLASH_THINKING_GATE_MIN_ACCEPTED_PER_ROUND 1.0 (:63) — the same gate, three calibrated numbers, each owned by an (arch × class) cell. THE PER-ROUND METRIC ITSELF IS ARCH-MOTIVATED, verbatim (:55-62): 'DFlash has different economics from sequential-token drafters: one assistant call proposes the whole block and the M5 NAX verify lane makes wide blocks cheap. Per-draft percentage therefore penalizes block 16 even when it is decisively profitable. The stable cross-width signal is accepted drafts per verify round. M5 Muse sweeps put code/tool traffic at 4.4-15.0 accepted/round and the regressing prose/vision class at 1.0-1.5; two is the measured break-even boundary. M4 block-5 model-card workloads accepting 62-86% remain above it as well.' — the metric choice is justified BY the arch (the NAX verify lane makes wide blocks cheap, so per-draft percentage punishes exactly the widths the hardware made cheap); the boundary (2.0) is read off the measured classes (4.4-15.0 profitable vs 1.0-1.5 regressing); and the boundary TRANSFERS (M4 block-5's 62-86% remain above it) — the threshold's evidence is named, classed, and the transfer checked. REGIME: any adaptive control whose break-even depends on the underlying execution architecture (dense vs routed experts; scalar vs tensor-tile verify lanes) or request class. ANTI-REGIME: one threshold across arches (the founding defect shape — a MoE under-fires its whole life); a threshold tuned per workload but not per arch (the arch moves the break-even, the workload moves the distribution — both axes need their own normalizer). ⛔ KINSHIP: composes specdec-gate-metric-normalized-by-drafts-per-round (the DENOMINATOR law — that rule says WHICH UNIT the bar reads, this says WHICH FLOOR per arch/class); the calibration-gate vocabulary (per-group thresholds, pre-registered evidence) is the frame; resource-limit-first-dispatch-probe-decline-latch (entries_gpu_97) cross-ref only. NotBenchable-here. The fixture pins the floor application order: width scaling FIRST, arch floor SECOND (max, never min), and the (arch × class) cell selects the base. Source: ddalcu/mlx-serve @ 96311b2638cf2f0ebdc7a2e39c85882a4fc4654a (MIT) — src/engines/mlx/spec/gates.zig:55-62 (the per-round metric + the measured classes + the transfer), :63 (the thinking floor), :64-70 (the MoE floor + the absolute-floor-after-scaling law), :71-79 (dflashGateShouldDisable). Batch 398 / Plan 398 / Research 432."
                .to_string(),
            ref_shape: r#"// ref_shape: ONE break-even floor for every arch and class — a MoE
// verify (expert routing + gather per block position) and a dense trunk
// priced alike, and a thinking class answers with the plain floor.

fn gate_minimum_buggy() -> f64 {
    2.0 // one number, every target
}

// a MoE verify at block 8: measured break-even ~2.6 accepted/round —
// the global floor admits anything above 2.0, so 2.3 accepted/round
// stays ARMED while below its own break-even.
let moe_observed = 2.3;
assert!(moe_observed >= gate_minimum_buggy(), "stays on under its own break-even");
"#.to_string(),
            opt_pattern: r"// opt_pattern: the floor is per (arch × class), applied AFTER width
// scaling as an ABSOLUTE minimum — scaling may raise it, never drop it
// below the arch's measured break-even boundary.

#[derive(Clone, Copy, PartialEq)]
enum Arch {
    Dense,
    SparseMoe,
}

#[derive(Clone, Copy, PartialEq)]
enum Class {
    Plain,
    Thinking,
}

fn floor(arch: Arch, class: Class) -> f64 {
    match (arch, class) {
        // measured break-evens: a MoE verify reads every expert its
        // block routes to (several times a dense trunk's); thinking
        // tolerates lower.
        (Arch::Dense, Class::Plain) => 2.0,
        (Arch::SparseMoe, Class::Plain) => 1.8,
        (_, Class::Thinking) => 1.0,
    }
}

fn gate_minimum(arch: Arch, class: Class, width_scaled: f64) -> f64 {
    // the floor rides AFTER scaling: max, never min
    width_scaled.max(floor(arch, class))
}

// scaling raised the MoE bar above its floor
assert_eq!(gate_minimum(Arch::SparseMoe, Class::Plain, 2.6), 2.6);
// bad scaling cannot undercut the arch floor
assert_eq!(gate_minimum(Arch::SparseMoe, Class::Plain, 1.2), 1.8);
// 2.3 accepted/round on MoE is BELOW its floor bar: the gate fires
assert!(2.3 < gate_minimum(Arch::SparseMoe, Class::Plain, 2.6));
// the dense floor stays 2.0 at the same scaling
assert_eq!(gate_minimum(Arch::Dense, Class::Plain, 1.9), 2.0);
".to_string(),
        },
        // ── R6 — the STICKY decline per drafter class + the content-signal
        // re-enable (tail-match over the committed sequence) + the
        // measured-cost-outranks-calibrated-bar clause. ────────────────────
        KernelRule {
            rule_id: "specdec-gate-sticky-decline-tail-match-re-enable".to_string(),
            gpu_concept: "MECHANISM (serving control — the STICKY decline + the content-signal re-enable): a mid-request speculation decline is STICKY per drafter class — stickiness FOLLOWING the resume cost, not the failure — and the paired re-enable reads a CONTENT signal the outcome metric structurally cannot see. THE STICKY HALF, verbatim (src/engines/mlx/spec/gates.zig:135-141 @ 96311b26): 'DFlash runtime economics gate. Sticky within the request: once a DFLASH_GATE_WARMUP-round sample proves the block-parallel path yields less than its width-normalized request-class threshold, subsequent ticks use the regular pipelined decoder through nextDflash's entry fallback.' and the reason it MUST be sticky, verbatim (:112-114): 'Decode plain in the batched group. Sticky for DFlash (plain rounds do not extend the assistant context); PLD's periodic re-enable check resumes once the slot is solo.' — DFlash's drafter carries an ASSISTANT CONTEXT built across its rounds; plain rounds never extend it, so a mid-request re-engage has nothing to resume from: stickiness follows the RESUME COST (infinite here), not the sample. PLD (prompt-lookup) pays no such resume cost — its state is the committed sequence itself — so its decline is PAIRED with a re-enable check. THE MEASURED-OUTRANKS CLAUSE sits between gate and decline, verbatim (:152-153): 'The table's own measure outranks the calibrated bar: a round emitting tokens cheaper than the plain step it would yield to stays on.' — before declining, the gate consults the measured cost table (roundBeatsSerial); a round the measurement prices cheaper than the plain alternative STAYS ON however the calibrated bar reads: measurement outranks calibration at the decision boundary — the calibrated bar is the prior, the measured round is the evidence. THE RE-ENABLE HALF, verbatim (:112-131): 'While the yield gate has PLD disabled, the COMMITTED sequence (prompt + generated) is re-scored every SPEC_REENABLE_INTERVAL steps: what fraction of the recent generated positions would have had a PLD lookup hit (their key-gram appears earlier in committed)? This catches the echo workload where the model repeats PROMPT content (file edits, tool results) — self-repetition scoring misses it because the echoed tail never repeats itself. Above the threshold, PLD is worth re-engaging at the cost of one pipeline drain.' + the constants (:120-125): SPEC_REENABLE_INTERVAL 32 / SPEC_REENABLE_WINDOW 32 / SPEC_REENABLE_MIN_FRACTION 0.25 / SPEC_REENABLE_MIN_TOKENS 16; the signal is tailMatchFraction(committed, window, gram 3) over the COMMITTED sequence (prompt included) — a PROMPT-CONTAINED gram match, structurally invisible to any self-repetition scorer: the echoed tail never repeats ITSELF (each generated position is novel within the tail), it repeats the PROMPT. THE COST BOUND rides the pairing, verbatim (:98-100): 'the re-enable check bounds the cost of a premature trip to ≤SPEC_REENABLE_INTERVAL pipelined-fallback steps' — the check's interval IS the bound on how long a wrong decline costs. REGIME: mid-request adaptive declines whose resume cost differs by mechanism (a stateful drafter vs a stateless lookup) and whose recovery signal lives in the CONTENT rather than the outcome metric. ANTI-REGIME: a decline whose re-engage is free (no stickiness needed — the check alone); a content signal the outcome metric already covers (one scorer suffices); re-enabling a stateful drafter on content signals (the assistant context is gone — stickiness is structural, not policy). ⛔ KINSHIP (adjudicated both directions): speculation-scheduler-break-even-flip (gpu_29) owns the WITHIN-REQUEST MONOTONE bypass (the verdict is a prompt-family property; B380's face carries the SpecCtl re-probe contrast) — THIS owns the ACROSS-DECLINE RESUME axis (why the decline is sticky per drafter class + what signal re-arms it; compose: the flip governs engagement within an active request, the sticky/re-enable governs the disabled state's lifecycle); resource-limit-first-dispatch-probe-decline-latch (entries_gpu_97) owns the RESOURCE-limit latch (a per-width memo on a resource refusal — the latch there follows a resource state, here a resume cost); gpu_91's sticky is a different sticky (cache-side). NotBenchable-here. The fixture pins: the sticky stateful drafter (no re-engage despite profitable-looking rounds — the assistant context never rebuilt), the PLD re-enable (echo content: generated grams contained in the committed prompt cross 0.25 → re-engage; novel tail: the match fraction stays ~0 → stays off), and the measured-outranks ordering (a calibrated decline refused when roundBeatsSerial prices the round cheaper). Source: ddalcu/mlx-serve @ 96311b2638cf2f0ebdc7a2e39c85882a4fc4654a (MIT) — src/engines/mlx/spec/gates.zig:112-114 (the sticky-for-DFlash reason), :115-131 (the re-enable block + constants + the echo law + specShouldReenable), :135-141 (the sticky gate + the entry fallback), :143-155 (yieldSpecToCompany + the measured-outranks clause + the roundBeatsSerial consult). Batch 398 / Plan 398 / Research 432."
                .to_string(),
            ref_shape: r#"// ref_shape: the decline re-checks the SAME outcome metric that
// tripped it, for every drafter class — a stateful drafter (whose
// assistant context plain rounds never rebuilt) re-engages into
// nothing, and a lookup decline waits on a repetition signal that
// cannot see echoed PROMPT content.

fn should_reenable_buggy(recent_accept_rate: f64, floor: f64) -> bool {
    recent_accept_rate >= floor // the tripping metric, re-read
}

// echoed PROMPT content: the tail is novel within itself (never
// repeats itself), so the accept-rate recovery lags reality by the
// whole echo onset — and the stateful drafter would re-engage with
// no assistant context to resume from.
assert!(should_reenable_buggy(0.51, 0.50), "re-engage into nothing");
"#.to_string(),
            opt_pattern: r"// opt_pattern: stickiness follows the RESUME COST — a stateful
// drafter stays declined for the request (plain rounds never extend
// the assistant context), a stateless lookup pairs its decline with a
// content-signal re-enable (prompt-gram matches in the committed
// tail) on a bounded interval; the measured cost outranks the
// calibrated bar at the boundary.

#[derive(Clone, Copy, PartialEq)]
enum Drafter {
    StatefulBlocks,
    PromptLookup,
}

fn resume_cost(d: Drafter) -> Option<u32> {
    match d {
        // plain rounds never extend the assistant context: no resume
        Drafter::StatefulBlocks => None,
        // state is the committed sequence: re-enable is a drain away
        Drafter::PromptLookup => Some(1),
    }
}

/// fraction of the tail's positions whose 3-gram appears EARLIER IN
/// committed (prompt included) — the echo signal.
fn tail_match_fraction(committed: &[u32], window: usize, gram: usize) -> f64 {
    if committed.len() < gram {
        return 0.0;
    }
    let start = committed.len().saturating_sub(window);
    let tail = &committed[start..];
    let mut hits = 0usize;
    for i in gram..tail.len() {
        let key = &tail[i - gram..i];
        let earlier = &committed[..start + i];
        if earlier.windows(gram).any(|w| w == key) {
            hits += 1;
        }
    }
    let denom = tail.len().saturating_sub(gram).max(1);
    hits as f64 / denom as f64
}

const REENABLE_INTERVAL: u64 = 32;
const REENABLE_MIN_FRACTION: f64 = 0.25;
const REENABLE_MIN_TOKENS: usize = 16;

fn should_reenable(
    d: Drafter,
    steps_since_decline: u64,
    committed: &[u32],
    generated_len: usize,
) -> bool {
    if resume_cost(d).is_none() {
        return false; // sticky: the context is gone, no resume
    }
    if steps_since_decline < REENABLE_INTERVAL {
        return false;
    }
    if generated_len < REENABLE_MIN_TOKENS {
        return false;
    }
    tail_match_fraction(committed, 32, 3) >= REENABLE_MIN_FRACTION
}

// the measured-outranks ordering: a calibrated decline is REFUSED
// when the measured table prices the round cheaper than serial.
fn round_beats_serial(round_ms: f64, serial_ms: f64) -> Option<bool> {
    if round_ms > 0.0 { Some(round_ms < serial_ms) } else { None }
}
fn decline_allowed(calibrated_below: bool, measured: Option<bool>) -> bool {
    match measured {
        Some(true) => false, // measured cheaper: stays on, bar notwithstanding
        _ => calibrated_below,
    }
}

// sticky: a profitable-looking round NEVER re-arms the stateful drafter
assert!(!should_reenable(Drafter::StatefulBlocks, 999, &[1, 2, 3], 999));
// echo workload: the generated tail repeats the PROMPT's grams —
// 16 prompt tokens, 16 generated echoes; the match fraction crosses 0.25
let prompt: Vec<u32> = (100..116).collect();
let mut committed = prompt.clone();
committed.extend_from_slice(&prompt);
assert!(should_reenable(Drafter::PromptLookup, 32, &committed, 16));
// novel tail: nothing matches the prompt — stays off
let novel: Vec<u32> = (300..332).collect();
assert!(!should_reenable(Drafter::PromptLookup, 32, &novel, 16));
// too soon: the interval bounds the cost of a premature trip
assert!(!should_reenable(Drafter::PromptLookup, 4, &committed, 16));
// the boundary ordering: measured cheaper beats a calibrated decline
assert!(!decline_allowed(true, round_beats_serial(6.0, 8.0)));
assert!(decline_allowed(true, round_beats_serial(9.0, 8.0)));
".to_string(),
        },
        // ── R7 — the MXU/NAX exactness-scoping law: adopt the tensor op by
        // SCOPING the tile to its exactness bound, tail blocks proven
        // bit-equal, error shaped to MATCH stock, bar relative to stock.
        // ────────────────────────────────────────────────────────────────
        KernelRule {
            rule_id: "mxu-op-exactness-scoping".to_string(),
            gpu_concept: "MECHANISM (numerics/dispatch — cooperative-tensor op adoption by EXACTNESS SCOPING): an MXU/NAX-class cooperative tensor op is adopted by SCOPING THE TILE to where the op is exact — not by proving the whole shape exact and not by minimizing error. FOUR laws: (1) the exactness BOUND is discovered and honored — the gather-attention PV op 'is exact up to 128 output columns: the head dimension goes in two halves' (two f32 destination tensors of 64 columns each); (2) the TAIL outside the tile geometry is scored at a sub-width proven BIT-EQUAL to the wide op ('scores 32 keys at a time: each equals the TK-key op's bit for bit'); (3) operand decomposition MATCHES THE INCUMBENT'S ERROR SHAPE — the softmax weights split into two bf16 terms where 'two match stock's error, one is worse': the goal is parity with stock, never minimal error; (4) the acceptance bar is RELATIVE to the incumbent ('error vs float64 no worse than stock'), never absolute and never kernel-vs-kernel. The anchors, verbatim (src/kernels/attention/lane_attn_partial.metal:29 @ 96311b26): '// the op is exact up to 128 output columns: the head dimension goes in two halves' — with the split visible right below (:30-33, the Olo/Ohi cooperative-tensor f32 destinations zeroed in two 64-element halves); verbatim (src/kernels/attention/lane_attn_tail.metal:25): '// scores 32 keys at a time: each equals the TK-key op's bit for bit' — the tail dispatch's narrower score op equals the wide op bit-for-bit, so the score sheet is uniform across chunk widths. The acceptance record, verbatim (docs/gotchas/engine-mlx.md:1018-1029): 'Defect: (levers) the indexer score ran four memory-bound ops per row chunk over a 256 MB f32 sheet (~3 s per prefill chunk at 1M); the gather attention ran at ~3 TFLOP/s. Fix: msv_qsa_score reproduces MLX's tf32 k-loop order and the sequential 4-head sum bit for bit (every operand is already bf16-valued); msv_qsa_nax_precise (PR #385) runs the gather on NAX cooperative tensors (macOS 26.3 + G17, its own predicate qsaNaxEligible), softmax weights split into two bf16 terms (two match stock's error, one is worse): +12.6% prefill at 162k. Bar: error vs float64 no worse than stock (tests/qsa_nax_precision.py). The score must stay on the NAX kernel at every width so decode, verify and prefill pick the same blocks.' — four laws in one record: the tf32-order mirror for the score sheet ('every operand is already bf16-valued' — reproduce the reference's REDUCTION REGIME bit for bit); the two-term split chosen because it matches stock's error (the one-term shape is WORSE than stock — precision-maximizing decomposition is the wrong objective); the relative bar (error-vs-f64 vs STOCK, the test named); and the cross-width consistency requirement (one score kernel at every width so decode/verify/prefill select the same blocks — selection consistency, not just per-call parity). REGIME: cooperative-tensor adoption on hardware whose tensor ops carry shape-dependent exactness bounds (Apple NAX/MXU-class; the measured +12.6% prefill at 162k on G17 is provenance, never a bar); sibling to the 1146 Metal tensor stack's laws on the same substrate class. ANTI-REGIME: a probe-gated capability arm whose op runs inexact past the bound (the scoping IS the admission — arm only where exact); an error-MINIMIZING decomposition (objective mismatch: parity, not precision); a per-width score kernel change (selection divergence across decode/verify/prefill). ⛔ KINSHIP (adjudicated both directions): bit-equal-fusion-mirrors-reference-reduction-regime (entries_gpu_99) owns the MIRROR LAW (fuse/transform so the reduction regime matches the reference's) — THIS owns the TILE-SCOPING axis (where the tensor op itself is exact + the sub-width tail proven bit-equal; the score-sheet tf32 mirror here is a positive instance OF the mirror law, cited not re-owned; the R10 carry-protocol face lands on the _99 rule); cooperative-tensor-mma-capability-route (gpu_18/B105) owns the capability PROBE arm (probe-then-route) — compose: the probe arms the hardware, the scoping keeps the op exact; trellis-gemm-narrow-m-plan-finalist-and-paired-k-chunks (gpu_120) owns the plan-finalist eligibility axis — compose as the eligibility table over scoped shapes. NotBenchable-here (the bar is error-vs-f64 vs stock on Apple NAX cooperative tensors — no validator hosts the hardware). The fixture models a tensor op with a 128-column exactness bound: the buggy arm runs past the bound (error above stock's); the fixed arm splits at the bound and the bar compares vs STOCK's error. Source: ddalcu/mlx-serve @ 96311b2638cf2f0ebdc7a2e39c85882a4fc4654a (MIT) — src/kernels/attention/lane_attn_partial.metal:29 (the 128-column bound + the two-half head dim), :30-33 (the Olo/Ohi f32 destination tensors), src/kernels/attention/lane_attn_tail.metal:25 (the 32-key tail bit-equal to the TK op), docs/gotchas/engine-mlx.md:1018-1029 (the score-sheet mirror + the two-term split + the relative bar + the cross-width consistency). Batch 398 / Plan 398 / Research 432."
                .to_string(),
            ref_shape: r#"// ref_shape: the tensor op is armed for the shape without scoping —
// 256 output columns through one op whose exactness bound is 128, the
// error grows past the bound, and the bar is 'smaller error than the
// other arm' (kernel-vs-kernel) instead of 'no worse than stock vs f64'.

const EXACT_COLUMNS: usize = 128;

/// model: past the exactness bound the op loses low bits (bf16-class
/// quantization of the accumulator), so error grows with columns.
fn tensor_dot_unscoped(a: &[f64], b: &[f64], cols: usize) -> f64 {
    let mut acc = 0.0f64;
    for i in 0..cols {
        let x = a[i] * b[i];
        // exact inside the bound; low bits lost past it
        acc += if i < EXACT_COLUMNS { x } else { (x * 256.0).trunc() / 256.0 };
    }
    acc
}

fn f64_truth(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn stock_impl(a: &[f64], b: &[f64]) -> f64 {
    // the incumbent (stock) also quantizes past the bound, the same way
    let mut acc = 0.0f64;
    for i in 0..a.len() {
        let x = a[i] * b[i];
        acc += if i < EXACT_COLUMNS { x } else { (x * 256.0).trunc() / 256.0 };
    }
    acc
}

let a: Vec<f64> = (0..256).map(|i| 1.0 + (i % 7) as f64 * 0.001).collect();
let b: Vec<f64> = (0..256).map(|i| 2.0 - (i % 5) as f64 * 0.001).collect();
let truth = f64_truth(&a, &b);
let err = (tensor_dot_unscoped(&a, &b, 256) - truth).abs();
let stock_err = (stock_impl(&a, &b) - truth).abs();
assert!(err > stock_err * 2.0, "error grows past the bound");
"#.to_string(),
            opt_pattern: r#"// opt_pattern: scope the tile to the exactness bound (the head dim
// in two halves, each within the bound); the tail dispatch scores a
// sub-width PROVEN bit-equal to the wide op; the bar is error vs
// STOCK's error against f64 truth — never vs the sibling arm.

const EXACT_COLUMNS: usize = 128;

fn tensor_dot_scoped(a: &[f64], b: &[f64]) -> f64 {
    // the head dim goes in halves: never a call past the bound
    let mut acc = 0.0f64;
    let mut i = 0;
    while i < a.len() {
        let take = EXACT_COLUMNS.min(a.len() - i);
        acc += a[i..i + take]
            .iter()
            .zip(&b[i..i + take])
            .map(|(x, y)| x * y)
            .sum::<f64>();
        i += take;
    }
    acc
}

/// the tail dispatch scores TAIL_KEYS keys; each sub-op must equal
/// the wide op bit for bit (modeled: same arithmetic, chunked).
fn tail_bit_equal_wide(a: &[f64], b: &[f64], tail_keys: usize) -> bool {
    let wide = tensor_dot_scoped(a, b);
    let mut tail = 0.0f64;
    let mut i = 0;
    while i < a.len() {
        let take = tail_keys.min(a.len() - i);
        tail += a[i..i + take]
            .iter()
            .zip(&b[i..i + take])
            .map(|(x, y)| x * y)
            .sum::<f64>();
        i += take;
    }
    (tail - wide).abs() < 1e-9
}

fn f64_truth(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn stock_impl(a: &[f64], b: &[f64]) -> f64 {
    let mut acc = 0.0f64;
    for i in 0..a.len() {
        let x = a[i] * b[i];
        acc += if i < EXACT_COLUMNS { x } else { (x * 256.0).trunc() / 256.0 };
    }
    acc
}

let a: Vec<f64> = (0..256).map(|i| 1.0 + (i % 7) as f64 * 0.001).collect();
let b: Vec<f64> = (0..256).map(|i| 2.0 - (i % 5) as f64 * 0.001).collect();
let truth = f64_truth(&a, &b);
let scoped_err = (tensor_dot_scoped(&a, &b) - truth).abs();
let stock_err = (stock_impl(&a, &b) - truth).abs();
// the relative bar: no worse than stock vs f64 truth
assert!(scoped_err <= stock_err, "error vs f64 no worse than stock");
// the tail sub-width is bit-equal to the wide op
assert!(tail_bit_equal_wide(&a, &b, 32));
"#.to_string(),
        },
        // ── R8 — the verify-width routing laws: named scalars over indexed
        // accumulator arrays; a supported-width LIST, never a templated
        // else; parity vs fp32 dequant truth. ────────────────────────────
        KernelRule {
            rule_id: "verify-width-named-scalars-supported-width-list".to_string(),
            gpu_concept: "MECHANISM (codegen/dispatch — the verify-width routing laws): multi-row verify shapes (M 2..8) route per width through an EXPLICIT supported-width table — split-K for narrow M, a wide tile for lm_head-class N, a tensor tile where the probe arms it — and two codegen laws govern the kernels the table dispatches: ACCUMULATORS ARE NAMED SCALARS (an indexed accumulator array spills to local memory ~10x) and EVERY SUPPORTED WIDTH HAS ITS OWN ARM (a templated else is not a supported-width list — a width that compiled through the wrong arm returned garbage). The source's record, verbatim (docs/gotchas/engine-mlx.md:266-276 @ 96311b26): 'Fix: verifyQmm routes eligible affine qmms through split-K (M 2..7), a wide tile for lm_head-class N, and the NAX m16 tile on G17 (M 8..16; mixed widths shape-gated per width after a sweep). Rules: codegen named scalars with literal accumulator indices (an indexed array spills 10x); M = 8 is the plain-SIMD cliff; the NAX kernel is never built where the probe is false; a templated width else is not a supported-width list (q8 compiled through the q6 arm and returned garbage: each width needs its own arm and parity row); A/B each adopted shape on its own model (the lane lost on the Gemma drafter's M = 5); rebuild the exe before a live A/B. Parity: no-worse-than fp32 dequant truth, never kernel-vs-kernel (a kernel-vs-kernel bound went red on the first CI GPU). Guard: VerifyQmmParity, the lane table test.' — seven laws in one record: the routing ladder (split-K M 2..7 / wide tile for lm_head N / NAX m16 on the tensor-capable class, mixed widths shape-gated per width after a sweep); the named-scalar codegen law (the accumulator array is a register-allocation hazard — literal indices keep the values in registers); the cliff (M 8 is where plain SIMD gives way — the boundary the table is built around); the probe honesty (never build the tensor kernel where the probe is false — dead kernels are build failures, not silent wrong answers); the supported-width-list law (q8 through the q6 arm = garbage; each width needs its own arm AND its own parity row); the per-model A/B (the lane LOST on one drafter's M = 5 — adoption is per-model, never assumed from the ladder); the rebuild discipline (a stale exe invalidates the live A/B); and the parity oracle (fp32 dequant TRUTH, never kernel-vs-kernel — a kernel-vs-kernel bound went red on the first CI GPU: two kernels can be wrong ALIKE). The measured basis: wide verify 71.9 → 109 → 119.5 tok/s at 4 lanes (their silicon, provenance). The anti-regime rides in-rule (measured, never a rule): wide verify at N = 1 on M4 Max — wider blocks raise acceptance and lose throughput; DFlash b8 drops novel content below serial — the law is a VERIFY (multi-row) law; plain N=1 decode keeps its own width. REGIME: codegen'd width-specialized kernels dispatched from a per-width table (verify qmm, batched matvec ladders). ANTI-REGIME: N = 1 plain decode (wider is worse); an unspecialized single arm (the M 2..8 dead zone is the point); a kernel-vs-kernel parity bar. ⛔ KINSHIP (adjudicated both directions): spill-fragility-law-accumulator-regcap-cliff (gpu_44) owns the REGISTER-PRESSURE CLIFF axis (accumulator/regcap pressure starving occupancy — the detection lens and the tile/role reshaping remedy); THIS owns the CODEGEN remedy + the supported-width-list law (named scalars with literal indices, per-width arms — the fix there reshapes the kernel, the fix here is how the codegen NAMES and ENUMERATES; compose on a codegen'd kernel: name the accumulators AND respect the regcap cliff); trellis-gemm-narrow-m-plan-finalist-and-paired-k-chunks (gpu_120) owns the plan-finalist eligibility axis (shape-keyed finalist selection + paired K chunks) — compose: the width table IS an eligibility table; the codegen laws are unowned there. NotBenchable-here (the win is Metal register allocation + verify throughput). The fixture models: an indexed accumulator array forcing a spill (modeled as heap-backed scratch) vs named scalars staying in registers; a width table with per-width arms refusing an unsupported width LOUDLY vs a templated else decoding garbage; parity vs fp32-dequant truth catching what kernel-vs-kernel misses. Source: ddalcu/mlx-serve @ 96311b2638cf2f0ebdc7a2e39c85882a4fc4654a (MIT) — docs/gotchas/engine-mlx.md:266-276 (the verify-width record: the ladder + the seven laws + the parity oracle + the guard). Batch 398 / Plan 398 / Research 432."
                .to_string(),
            ref_shape: r#"// ref_shape: one templated arm serves every width — the accumulator
// is an INDEXED ARRAY (codegens to local memory, ~10x spill), q8
// compiles through the q6 arm and returns garbage, and the parity bar
// compares kernels (wrong alike, both pass).

fn verify_qmm_templated(m: usize, quant: &str) -> Vec<f64> {
    // the accumulator array: indexed writes spill (modeled: heap scratch)
    let mut acc = vec![0.0f64; 8];
    let spills = acc.len(); // the indexed array is ALWAYS fully allocated
    for i in 0..m.min(8) {
        acc[i] = dequant_arm(quant); // ONE arm for every width/quant
    }
    acc[..m.max(1)].to_vec()
}

fn dequant_arm(_quant: &str) -> f64 {
    6.0 // the q6 arm's decode — q8 values decode WRONG through it
}

// kernel-vs-kernel parity: both kernels share the wrong arm, both pass
fn parity_kernel_vs_kernel(a: &[f64], b: &[f64]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6)
}

let q8_out = verify_qmm_templated(4, "q8");
let q6_out = verify_qmm_templated(4, "q6");
assert_eq!(q8_out[0], 6.0, "q8 decoded WRONG, silently");
assert!(parity_kernel_vs_kernel(&q8_out, &q6_out), "wrong alike: passes");
"#.to_string(),
            opt_pattern: r"// opt_pattern: a supported-width LIST with per-width arms (an
// unsupported width refuses LOUD, never compiles through a neighbor);
// accumulators as NAMED scalars (literal indices — no indexed array);
// parity against fp32 dequant TRUTH, never kernel-vs-kernel.

const SUPPORTED_M: [usize; 4] = [2, 3, 4, 8]; // the list, per-width arms

#[derive(Clone, Copy, PartialEq)]
enum Quant {
    Q8,
    Q6,
}

#[derive(Debug, PartialEq)]
enum QmmErr {
    UnsupportedWidth(usize),
}

fn dequant_q8() -> f64 {
    8.0
}

fn dequant_q6() -> f64 {
    6.0
}

fn verify_qmm(m: usize, quant: Quant) -> Result<Vec<f64>, QmmErr> {
    if !SUPPORTED_M.contains(&m) {
        // loud refusal — never compile through a neighbor arm
        return Err(QmmErr::UnsupportedWidth(m));
    }
    // named scalars: a0..a3 with literal indices — registers, no spill
    let (a0, a1, a2, a3) = match quant {
        Quant::Q8 => (dequant_q8(), dequant_q8(), dequant_q8(), dequant_q8()),
        Quant::Q6 => (dequant_q6(), dequant_q6(), dequant_q6(), dequant_q6()),
    };
    Ok(vec![a0, a1, a2, a3][..m].to_vec())
}

/// parity vs fp32 dequant TRUTH — the oracle is the truth, not a twin
fn parity_vs_truth(got: &[f64], truth: &[f64]) -> bool {
    got.iter().zip(truth).all(|(x, t)| (x - t).abs() <= 1e-6)
}

// M=5 (between the split-K rung and the cliff): the lane LOST on one
// drafter's M=5 — unsupported here, refused loud, never assumed.
assert_eq!(verify_qmm(5, Quant::Q8), Err(QmmErr::UnsupportedWidth(5)));
// q8 through its OWN arm: correct decode
let q8 = verify_qmm(4, Quant::Q8).unwrap();
assert_eq!(q8[0], 8.0);
// truth parity catches what kernel-vs-kernel missed
let truth = [8.0, 8.0, 8.0, 8.0];
assert!(parity_vs_truth(&q8, &truth));
let wrong = [6.0, 6.0, 6.0, 6.0];
assert!(!parity_vs_truth(&wrong, &truth));
".to_string(),
        },
    ]
}
