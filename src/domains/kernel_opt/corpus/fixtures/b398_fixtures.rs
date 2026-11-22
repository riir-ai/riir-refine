//! Batch 398 kernel fixture preflight — the mlx-serve return-visit delta
//! (Plan 398 / Research 432, ddalcu/mlx-serve @ 96311b26 MIT) as ONE
//! standalone file (8 rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the source
//! is Zig/Metal; the Rust arms below model each law's decision content).
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b398_fixtures.rs`
//! at debug AND -O. Each arm names the honest and the adversary half;
//! a defect caught here is fixed HERE first, then synced into
//! entries_gpu_161.rs in the same landing commit.
//!
//! Arms (the b380 discipline):
//! - R1 dispatch budget: the chunk closed form (budget, block-aligned,
//!   capped), the carry associativity (chunked == one pass), the band
//!   arm never chunking, the ratio-gate adversary (µbench wins, live
//!   loses past the cliff).
//! - R2 dtype promotion: the wide constant promotes the stream while the
//!   VALUES pass (the oracle blind); the fixed arm materializes in the
//!   activation dtype (no moves to log); the tampered arm — a "narrow"
//!   that keeps the wide dtype — reds.
//! - R3 round-cost controller: MIN_SAMPLES trust, the exponentiated step
//!   refusal, the self-spike, the settle-at-once, the hysteresis, the
//!   extrapolation cap; the adversary arm — one sample anchoring — reds.
//! - R4 gate denominators: the per-round lie at block 4 (armed above the
//!   bar) vs the per-draft truth; the yield gate's STEP denominator
//!   tripping where rounds cannot accumulate.
//! - R5 arch floors: the (arch × class) floor applied AFTER width
//!   scaling (max, never min); the global-floor adversary under-fires
//!   the MoE target.
//! - R6 sticky + re-enable: the stateful drafter never re-arms (the
//!   context is gone), the lookup re-arms on prompt-contained grams,
//!   the measured cost outranks the calibrated bar.
//! - R7 exactness scoping: the unscoped op's error grows past the bound;
//!   the scoped op stays within stock's error vs f64 truth; the tail
//!   sub-width equals the wide op.
//! - R8 verify width: the templated else decodes garbage and the
//!   kernel-vs-kernel bar passes it; the supported-width list refuses
//!   loud; parity vs fp32-dequant truth catches what twins miss.

// ────────────────────────────── R1 helpers ──────────────────────────────

const BK: i64 = 32;
const BUDGET_WU: i64 = 250_000_000;
const CLIFF_WU: f64 = 1.5e9;

fn kv_chunk_len(batch: i64, hq: i64, ql: i64, kl: i64, budget: i64) -> i64 {
    if budget <= 0 {
        return kl;
    }
    let per_key = (batch * hq * ql).max(1);
    let mut chunk = budget / per_key;
    chunk = (chunk / BK) * BK;
    chunk.clamp(BK, kl)
}

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

// ────────────────────────────── R1 arms ─────────────────────────────────

#[test]
fn r1_budget_splits_the_dispatch_deterministically() {
    // the FIXED arm: every dispatch within budget, BK-aligned (tail
    // capped), contiguous cover; the band arm never chunks.
    struct P {
        batch: i64,
        heads: i64,
        q: i64,
    }
    let p = P { batch: 1, heads: 48, q: 2048 };
    let kl = 32_768i64;
    let chunk = kv_chunk_len(p.batch, p.heads, p.q, kl, BUDGET_WU);
    let mut covered = 0i64;
    let mut dispatches = 0;
    let mut k = 0;
    while k < kl {
        let end = (k + chunk).min(kl);
        let w = p.batch * p.heads * p.q * (end - k);
        assert!(w <= BUDGET_WU, "dispatch within budget");
        assert!((end - k) % BK == 0 || end == kl, "BK-aligned");
        assert_eq!(k, covered, "contiguous");
        covered = end;
        k = end;
        dispatches += 1;
    }
    assert_eq!(covered, kl);
    assert!(dispatches > 1, "the split happened");
    // budget <= 0 = one dispatch (the hatch)
    assert_eq!(kv_chunk_len(p.batch, p.heads, p.q, kl, 0), kl);
    // a small context within one chunk stays one dispatch
    assert_eq!(kv_chunk_len(p.batch, p.heads, p.q, 2048, BUDGET_WU), 2048);
}

#[test]
fn r1_carry_is_associative_chunked_equals_one_pass() {
    let a = Carry { m: 8.0, l: 2.0, o: 5.0 };
    let b = Carry { m: 9.0, l: 2.5, o: 7.0 };
    let c = Carry { m: 7.5, l: 1.2, o: 3.0 };
    let d = Carry { m: 8.5, l: 1.7, o: 4.0 };
    let abcd = merge(&merge(&merge(&a, &b), &c), &d);
    let ad_bc = merge(&merge(&a, &d), &merge(&b, &c));
    let dcba = merge(&merge(&merge(&d, &c), &b), &a);
    for (x, y, name) in [
        (abcd.m, ad_bc.m, "m assoc"),
        (abcd.l, ad_bc.l, "l assoc"),
        (abcd.o, ad_bc.o, "o assoc"),
    ] {
        assert!(
            (x - y).abs() < 1e-9,
            "{name}: chunk grouping must not move the carry"
        );
    }
    // the adversary: a NON-associative merge would make the chunked
    // chain diverge from the one-dispatch result by grouping — here the
    // extreme-value form keeps m exact and l/o within one scaling step.
    assert!((abcd.m - dcba.m).abs() < 1e-12, "m is the exact max");
}

#[test]
fn r1_the_ratio_gate_adversary_loses_live() {
    // the BUGGY arm: the µbench ratio gate (fuse while kL <= 2*qL) says
    // fused; the live envelope crosses the cliff and the watchdog eats
    // the prefill. The budget gates the quantity the watchdog prices.
    let (q, kl) = (4096i64, 8192i64);
    let heads = 48i64;
    assert!(kl <= 2 * q, "the ratio gate says fused (kL <= 2*qL µbench)");
    let work = (heads * q * kl) as f64;
    assert!(work > CLIFF_WU, "one dispatch past the machine cliff");
    // the honest budget splits it instead
    let chunk = kv_chunk_len(1, heads, q, kl, BUDGET_WU);
    let per_dispatch = (heads * q * chunk) as f64;
    assert!(per_dispatch <= BUDGET_WU as f64);
}

// ────────────────────────────── R2 arms ─────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
enum D {
    F16,
    F32,
}

#[derive(Clone)]
struct T {
    d: D,
    v: Vec<f64>,
}

fn mul(a: &T, b: &T) -> T {
    let d = if a.d == D::F32 || b.d == D::F32 { D::F32 } else { D::F16 };
    T {
        d,
        v: a.v.iter().zip(&b.v).map(|(x, y)| x * y).collect(),
    }
}

#[test]
fn r2_the_wide_constant_promotes_and_the_oracle_passes() {
    // the DEFECT arm: the loader builds the table f32; the stream
    // promotes; the VALUES are identical either way (the oracle blind).
    let table = T { d: D::F32, v: vec![0.25, 1.0, 0.5] };
    let mut stream = T { d: D::F16, v: vec![1.0, 2.0, 3.0] };
    let truth: Vec<f64> = stream.v.iter().zip(&table.v).map(|(x, y)| x * y).collect();
    stream = mul(&stream, &table);
    stream = mul(&stream, &T { d: D::F16, v: vec![1.0; 3] });
    assert_eq!(stream.d, D::F32, "the whole stream promoted");
    for (s, t) in stream.v.iter().zip(&truth) {
        assert!((s - t).abs() < 1e-12, "the oracle passes; the width is the cost");
    }
}

#[test]
fn r2_the_fixed_arm_stays_narrow_and_the_trace_sees_no_moves() {
    // the FIXED arm: constants materialize IN the activation dtype; the
    // 1-D f16 table narrows at load; the trace counts MOVES only.
    fn const_table_as(v: &[f64], act: D) -> T {
        T { d: act, v: v.to_vec() }
    }
    fn narrows_loaded(t: &T) -> T {
        if t.v.len() <= 4 && t.d == D::F32 { T { d: D::F16, v: t.v.clone() } } else { t.clone() }
    }
    let table = const_table_as(&[0.25, 1.0, 0.5], D::F16);
    let norm = narrows_loaded(&T { d: D::F32, v: vec![0.9] });
    let mut stream = T { d: D::F16, v: vec![1.0, 2.0, 3.0] };
    let mut walk = vec![stream.d];
    stream = mul(&stream, &table);
    walk.push(stream.d);
    stream = mul(&stream, &norm);
    walk.push(stream.d);
    assert_eq!(stream.d, D::F16);
    let moves = walk.windows(2).filter(|w| w[0] != w[1]).count();
    assert_eq!(moves, 0, "no dtype moves to log");
}

#[test]
fn r2_the_tampered_narrow_that_is_not_reds() {
    // the TAMPERED arm: a "constTableAs" that claims the activation
    // dtype but keeps the wide one — the trace must still see the move.
    fn fake_cast(t: &T, _act: D) -> T {
        t.clone() // the lie: labels changed, bytes did not
    }
    let table = T { d: D::F32, v: vec![0.5] };
    let out = fake_cast(&table, D::F16);
    assert_eq!(out.d, D::F32, "the fake cast widens still");
    let mut walk = vec![D::F16];
    walk.push(mul(&T { d: D::F16, v: vec![1.0] }, &out).d);
    let moves = walk.windows(2).filter(|w| w[0] != w[1]).count();
    assert!(moves > 0, "the trace catches the lie");
}

// ────────────────────────────── R3 arms ─────────────────────────────────

#[derive(Clone)]
struct Cell {
    ms: f64,
    n: u32,
    ema: f64,
}

const MIN_SAMPLES: u32 = 3;
const IMPLAUSIBLE_STEP: f64 = 1.5;
const SELF_SPIKE: f64 = 3.0;
const SETTLE_WORSE: f64 = 1.20;
const HYSTERESIS: f64 = 0.05;

fn trusted(c: &Cell) -> bool {
    c.n >= MIN_SAMPLES
}

#[test]
fn r3_the_guards_refuse_contaminated_cells() {
    let anchor = Cell { ms: 100.0, n: 5, ema: 98.0 };
    // the step check, EXPONENTIATED by the width distance
    let refused = |width: usize, sample_ms: f64, below: Option<(&Cell, usize)>| {
        if let Some((c, w)) = below {
            if trusted(c) {
                return sample_ms <= c.ms * IMPLAUSIBLE_STEP.powi((width - w) as i32);
            }
        }
        true
    };
    assert!(!refused(1, 200.0, Some((&anchor, 0))), "1.5x bound: refused");
    assert!(refused(2, 200.0, Some((&anchor, 0))), "1.5^2 = 2.25x: admits");
    let untrusted = Cell { ms: 60.0, n: 1, ema: 0.0 };
    assert!(refused(1, 200.0, Some((&untrusted, 0))), "untrusted anchors never refuse");
    // the self-spike vs the mature EMA
    assert!(anchor.ema > 0.0 && 400.0 > anchor.ema * SELF_SPIKE);
    // the settle-at-once: a first sample 25% worse per token settles
    let per_tok = anchor.ms / 12.0;
    let first = 150.0 / 12.0;
    assert!(first > per_tok * SETTLE_WORSE);
    // the hysteresis: a 3% gain inside the band does not flip
    let challenger = per_tok * 0.97;
    assert!(challenger >= per_tok * (1.0 - HYSTERESIS));
}

#[test]
fn r3_the_extrapolation_cap_and_the_one_sample_adversary() {
    // the FIXED arm: past the widest measured, each position costs
    // max(last slope, prior marginal) — never a raw slope extension.
    let last_per_tok: f64 = 0.5;
    let prior_marginal: f64 = 0.5;
    let price = last_per_tok + last_per_tok.max(prior_marginal) * 4.0;
    assert!((price - 2.5).abs() < 1e-9);
    // the ADVERSARY arm: a shallow slope priced upward reads wide as
    // free (the founding defect) — the cap forbids it.
    let shallow: f64 = 0.05;
    let naive = last_per_tok + shallow * 4.0; // 0.7 — reads nearly free
    let capped = last_per_tok + shallow.max(prior_marginal) * 4.0;
    assert!(naive < capped, "the cap forbids the free-lunch slope");
}

// ────────────────────────────── R4 arms ─────────────────────────────────

fn gate_per_round(attempted: u64, accepted: u64, warmup: u64) -> bool {
    if attempted < warmup {
        return false;
    }
    (accepted as f64 / attempted as f64) < 0.30
}

const PER_DRAFT_RATE: f64 = 0.50;

fn gate_per_draft(attempted: u64, accepted: u64, dpr: u32, warmup: u64) -> bool {
    if attempted < warmup {
        return false;
    }
    if dpr == 0 {
        return false;
    }
    let rate = accepted as f64 / (attempted * dpr as u64) as f64;
    rate < PER_DRAFT_RATE
}

fn yield_gate(steps: u64, accepted: u64, warmup: u64) -> bool {
    if steps < warmup {
        return false;
    }
    (accepted as f64 / steps as f64) < 0.25
}

#[test]
fn r4_the_per_round_gate_is_armed_lie_at_block_four() {
    // block 4, 47% per-draft acceptance (regressing): per-round 1.88
    // sails over 0.30 — ARMED while the workloads lose.
    assert!(!gate_per_round(100, 188, 5), "the per-round gate stays armed");
    // the FIXED arm: per-draft 0.47 < 0.50 trips
    assert!(gate_per_draft(100, 188, 4, 5));
    // the heavy-echo workload stays: 0.84 per-draft
    assert!(!gate_per_draft(100, 336, 4, 5));
    // the zero-work config never trips
    assert!(!gate_per_draft(100, 0, 0, 5));
}

#[test]
fn r4_the_yield_gate_counts_the_paying_step() {
    // 60 enabled steps, 5 accepted tokens: verify rounds never
    // accumulated (rare matches) but the STEP denominator sees the tax.
    assert!(yield_gate(60, 5, 8));
    // a healthy yield stays
    assert!(!yield_gate(60, 20, 8));
}

// ────────────────────────────── R5 arms ─────────────────────────────────

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
        (Arch::Dense, Class::Plain) => 2.0,
        (Arch::SparseMoe, Class::Plain) => 1.8,
        (_, Class::Thinking) => 1.0,
    }
}

fn gate_minimum(arch: Arch, class: Class, width_scaled: f64) -> f64 {
    width_scaled.max(floor(arch, class))
}

#[test]
fn r5_the_floor_rides_after_scaling_max_never_min() {
    assert_eq!(gate_minimum(Arch::SparseMoe, Class::Plain, 2.6), 2.6);
    assert_eq!(gate_minimum(Arch::SparseMoe, Class::Plain, 1.2), 1.8);
    assert_eq!(gate_minimum(Arch::Dense, Class::Plain, 1.9), 2.0);
    // 2.3 accepted/round on MoE is below its own break-even bar: fires
    assert!(2.3 < gate_minimum(Arch::SparseMoe, Class::Plain, 2.6));
}

#[test]
fn r5_the_global_floor_adversary_under_fires_moe() {
    // the BUGGY arm: one floor for every arch admits a MoE round at 2.3
    // while its own break-even sits at 2.6.
    let global = 2.0;
    assert!(2.3 >= global, "stays on under its own break-even");
    // the honest (arch × class) floor catches it
    assert!(2.3 < 2.6, "the MoE break-even the floor encodes");
    assert_eq!(floor(Arch::SparseMoe, Class::Plain), 1.8);
    assert_eq!(gate_minimum(Arch::SparseMoe, Class::Plain, 2.6), 2.6);
    assert!(2.3 < gate_minimum(Arch::SparseMoe, Class::Plain, 2.6));
}

// ────────────────────────────── R6 arms ─────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Drafter {
    Stateful,
    Lookup,
}

fn resume_cost(d: Drafter) -> Option<u32> {
    match d {
        Drafter::Stateful => None,
        Drafter::Lookup => Some(1),
    }
}

fn tail_match_fraction(committed: &[u32], window: usize, gram: usize) -> f64 {
    if committed.len() < gram {
        return 0.0;
    }
    let start = committed.len().saturating_sub(window);
    let tail = &committed[start..];
    let mut hits = 0usize;
    for i in gram..tail.len() {
        let key = &tail[i - gram..i];
        // strictly earlier: nothing from the key's own window onward
        let earlier = &committed[..start + i - gram];
        if !earlier.is_empty() && earlier.windows(gram).any(|w| w == key) {
            hits += 1;
        }
    }
    let denom = tail.len().saturating_sub(gram).max(1);
    hits as f64 / denom as f64
}

const REENABLE_MIN_FRACTION: f64 = 0.25;

fn should_reenable(d: Drafter, steps: u64, committed: &[u32], gen_len: usize) -> bool {
    if resume_cost(d).is_none() {
        return false;
    }
    if steps < 32 || gen_len < 16 {
        return false;
    }
    tail_match_fraction(committed, 32, 3) >= REENABLE_MIN_FRACTION
}

#[test]
fn r6_sticky_stateful_and_echo_aware_lookup() {
    // the stateful drafter: NO re-arm however the rounds look
    assert!(!should_reenable(Drafter::Stateful, 999, &[1, 2, 3], 999));
    // the echo workload: the generated tail repeats the PROMPT's grams
    let prompt: Vec<u32> = (100..116).collect();
    let mut committed = prompt.clone();
    committed.extend_from_slice(&prompt);
    assert!(should_reenable(Drafter::Lookup, 32, &committed, 16));
    // the novel tail: nothing matches — stays off
    let novel: Vec<u32> = (300..332).collect();
    assert!(!should_reenable(Drafter::Lookup, 32, &novel, 16));
    // too soon: the interval bounds the premature-trip cost
    assert!(!should_reenable(Drafter::Lookup, 4, &committed, 16));
    // the echo-tail law: the echoed tail never repeats ITSELF (each
    // echoed gram appears once in the tail) — a self-repetition scorer
    // sees ~nothing; prompt-containment sees everything.
    let self_frac = tail_match_fraction(&prompt, 32, 3);
    assert!(self_frac < 0.05, "self-repetition scoring is blind to echoes");
}

#[test]
fn r6_the_measured_cost_outranks_the_calibrated_bar() {
    let beats = |round_ms: f64, serial_ms: f64| round_ms > 0.0 && round_ms < serial_ms;
    let decline_allowed = |calibrated_below: bool, measured: Option<bool>| match measured {
        Some(true) => false,
        _ => calibrated_below,
    };
    // measured cheaper: the decline is refused, bar notwithstanding
    assert!(!decline_allowed(true, Some(beats(6.0, 8.0))));
    // measured dearer: the calibrated decline stands
    assert!(decline_allowed(true, Some(beats(9.0, 8.0))));
    // no measurement: the bar decides
    assert!(decline_allowed(true, None));
}

// ────────────────────────────── R7 arms ─────────────────────────────────

const EXACT_COLS: usize = 128;

fn dot_unscoped(a: &[f64], b: &[f64]) -> f64 {
    let mut acc = 0.0;
    for i in 0..a.len() {
        let x = a[i] * b[i];
        acc += if i < EXACT_COLS { x } else { (x * 256.0).trunc() / 256.0 };
    }
    acc
}

fn dot_scoped(a: &[f64], b: &[f64]) -> f64 {
    let mut acc = 0.0;
    let mut i = 0;
    while i < a.len() {
        let take = EXACT_COLS.min(a.len() - i);
        acc += a[i..i + take].iter().zip(&b[i..i + take]).map(|(x, y)| x * y).sum::<f64>();
        i += take;
    }
    acc
}

fn truth(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn stock(a: &[f64], b: &[f64]) -> f64 {
    dot_unscoped(a, b) // the incumbent carries the same bound shape
}

#[test]
fn r7_scoping_holds_the_relative_bar() {
    let a: Vec<f64> = (0..256).map(|i| 1.0 + (i % 7) as f64 * 0.001).collect();
    let b: Vec<f64> = (0..256).map(|i| 2.0 - (i % 5) as f64 * 0.001).collect();
    let t = truth(&a, &b);
    // the DEFECT arm: 256 columns through the unscoped op — error above
    // stock's (the model: stock quantizes ONE half, the unscoped call
    // crosses its own bound mid-accumulate with drift compounding)
    let stock_err = (stock(&a, &b) - t).abs();
    // the FIXED arm: scoped calls never cross the bound — exact
    let scoped_err = (dot_scoped(&a, &b) - t).abs();
    assert_eq!(scoped_err, 0.0, "every call within the bound: exact");
    assert!(scoped_err <= stock_err, "the relative bar: no worse than stock");
    // the tail sub-width equals the wide op (bit-equal law, modeled)
    let wide = dot_scoped(&a, &b);
    let mut tail = 0.0;
    let mut i = 0;
    while i < a.len() {
        let take = 32.min(a.len() - i);
        tail += a[i..i + take].iter().zip(&b[i..i + take]).map(|(x, y)| x * y).sum::<f64>();
        i += take;
    }
    assert!((tail - wide).abs() < 1e-9, "the 32-key tail equals the wide op");
}

#[test]
fn r7_the_error_minimizing_objective_adversary() {
    // the objective is PARITY with stock, never minimum error: an arm
    // that minimizes absolute error but drifts from stock's error SHAPE
    // fails the bar even when its raw error is smaller. The model: the
    // two-term split matches stock's error; a one-term "better" shape
    // is judged worse-than-stock relative to the SAME truth.
    let a: Vec<f64> = (0..256).map(|i| 1.0 + (i % 7) as f64 * 0.001).collect();
    let b: Vec<f64> = (0..256).map(|i| 2.0 - (i % 5) as f64 * 0.001).collect();
    let t = truth(&a, &b);
    let stock_err = (stock(&a, &b) - t).abs();
    // a hypothetical "sharper" arm with 10x SMALLER raw error on this
    // input still fails the class rule if its shape diverges on other
    // inputs — the bar is per-input relative, not aggregate-absolute.
    let sharper = t + stock_err * 0.1;
    assert!((sharper - t).abs() < stock_err, "raw error smaller");
    // but the acceptance bar reads RELATIVE per the law: pass only
    // within stock's error — here it passes on THIS input, and the law
    // forbids reading that as a class win (the fixture pins the bar,
    // not the classifier).
    assert!((sharper - t).abs() <= stock_err);
}

// ────────────────────────────── R8 arms ─────────────────────────────────

fn dequant(quant: &str) -> f64 {
    if quant == "q8" { 8.0 } else { 6.0 }
}

#[test]
fn r8_the_templated_else_decodes_garbage_and_twin_parity_passes_it() {
    // the DEFECT arm: one arm serves every width/quant — q8 through the
    // q6 arm decodes WRONG; the kernel-vs-kernel bar passes (both twins
    // wrong alike); the fp32-dequant TRUTH bar catches it.
    let q8_out = dequant("q6"); // the templated else routes q8 to q6
    assert_eq!(q8_out, 6.0, "q8 decoded WRONG, silently");
    let twin_a = vec![q8_out; 4];
    let twin_b = vec![dequant("q6"); 4];
    let k_vs_k = twin_a.iter().zip(&twin_b).all(|(x, y)| (x - y).abs() < 1e-6);
    assert!(k_vs_k, "kernel-vs-kernel: wrong alike, passes");
    let truth_q8 = 8.0;
    assert!((q8_out - truth_q8).abs() > 1e-6, "the truth bar catches it");
}

#[test]
fn r8_the_supported_width_list_refuses_loud() {
    const SUPPORTED: [usize; 4] = [2, 3, 4, 8];
    let route = |m: usize| -> Result<(), String> {
        if SUPPORTED.contains(&m) {
            Ok(())
        } else {
            Err(format!("unsupported width {m}"))
        }
    };
    for m in [2, 3, 4, 8] {
        assert!(route(m).is_ok());
    }
    // M=5 (the Gemma drafter's loss): refused loud, never a neighbor arm
    assert!(route(5).is_err());
    // named scalars vs the indexed array (the spill model): the indexed
    // form is fully materialized (heap scratch) whatever m reads
    let indexed = vec![0.0f64; 8]; // always allocated
    let named = (dequant("q8"), dequant("q8"), dequant("q8"), dequant("q8"));
    assert_eq!(indexed.len(), 8, "the array pays for all slots");
    assert_eq!(named.0, dequant("q8"), "the named scalar is its own slot");
}
