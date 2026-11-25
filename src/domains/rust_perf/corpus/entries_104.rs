// Batch 418 — the tunazip exact-numerics transcription batch (Plan 414 /
// Research 449). Source: github.com/nathanrs/tunazip @ pin
// 571509687de3bd7301af0f79d75893b3e83d2417 (tip UNMOVED at T0 and re-checked
// at authoring — HEAD == pin twice; MIT + NOTICE verified in the clone;
// fork of Tufa Labs' tufazip <- Bellard NNCP <- Nayuki, all MIT-cited).
// ONE rule: R1 exact-accumulation-via-operand-mantissa-bound
// (rust_perf/arithmetic, decision-content NotBenchable — the pre-registered
// G2 falsifier FIRED at the landing cell, measured bases recorded in-rule).
// R2 (__dmul_rn per-intrinsic rounding) landed as a count-neutral RIDER on
// kernel_opt gpu_83 hip-affine-contract-off-row-invariance; R3 (stream-key
// domain fold + integer threshold) as a count-neutral FACE on gpu_03
// counter-derived-seed-stateless-noise; R4 (owned-transcendental
// cross-executor construction) as a count-neutral FACE on entries_88
// reference-round-replicated-in-integer-arithmetic. R6 (Decimal-oracle
// constants) DROP recorded in Research 449 (positive instance of house
// oracle-fixture practice).
//
// Dedup re-verified at the landing HEAD (3+ vocabularies per atom, every
// hit read and ruled — the Batch-409 lesson): R1 zero owners across
// mantissa/significant-bit/reduction-order/order-free/exact-accumulation/
// 2^53-budget/operand-bound/shared-exponent/power-of-two-scale sweeps —
// entries_56 "order-free" is anti-regime prose (acts idempotent AND
// order-free), entries_16 order-insensitive is a CFG fixpoint worklist,
// gpu_112 "exact accumulation" is a dyadic fixture comment, gpu_121
// exact-matmul is the int8-screen + exact-rescore certificate rule (error
// BOUNDS for screening, not operand bounding for order-free accumulation),
// gpu_101 exact-sum is test-value modulus design, entries_17/gpu_20/
// gpu_102/gpu_148 power-of-two-scale are the EXACT-RESCALE axis (pow2 as
// exponent shift, zero rounding — range, not order), gpu_12
// virtual-thread-folding-preserves-reduction-tree is the TREE-REPLAY pole
// (keep the order when parallelizing — R1 frees the order instead).
// R2's host confirmed the four-owner FMA-contraction census (gpu_152's
// owner quote: bit-equal-fusion / gdn-cross-phase / hip-affine /
// moe-router-weight-fold); hex-float vocabulary zero-owned (one unrelated
// clippy proposer hit). R3's stream-key/domain-separation/dropout-threshold
// vocabularies zero-owned outside gpu_03 (the threshold hits gpu_86/08/42/
// 161 are cost-table/verify thresholds, never RNG).
//
// G2 cell (the pre-registered demotion bar, m3 macOS aarch64 rustc -O,
// min-of-31 interleaved closures, load 6.3-8.7): quantized-unrolled-4 vs
// raw-sequential 1.1x / 1.0x / 1.0x at inner 4096/16384/65536 — PARITY at
// best, never a win — and 2.9x / 2.8x / 2.8x against the equally-order-free
// raw-unrolled-4 control (the quantization pass is real work). The bench
// claim DEMOTED to decision-content (rule stands, bench_spec None) per the
// plan's pre-registration; the win is portability + order-freedom, not time.
//
// Fixture preflight: b418_perf_fixtures.rs 5/5 arms all-ok at debug AND -O
// BEFORE entry (the gate caught THREE arm defects: the G2 timing arm's
// dead-result raw loops compiled to nothing — ratios against a deleted loop
// read inf until black_box; Python's bit_length() has no Rust spelling —
// 64 - leading_zeros(); and F4's first cancellation construction could not
// cancel — products -A*A are sign-definite — rebuilt as the
// perturbation-lost carrier, which is also the sharper disclosure). F4's
// measured nuance folded into R1's losing-regime wording: the pow2 grid is
// LOGARITHMIC, so magnitude alone is not the loss axis — a logspace +/-12
// decade row measured only 3.6e-5 relative (recorded, inside the benign
// envelope), while a 2^-20 perturbation on a 2^0 carrier is lost ENTIRELY
// (delta 1.0): the losing regime is a small signal inside an operand
// relative to a large peak in the SAME reduction, i.e. the carrier's grid
// ulp cannot resolve it.

use crate::domains::rust_perf::PerfRule;

pub fn b418_rules() -> Vec<PerfRule> {
    vec![
        // ── R1: order-free exact accumulation via operand mantissa bound ──
        PerfRule {
            rule_id: "exact-accumulation-via-operand-mantissa-bound".to_string(),
            name: "a reduction that must be bit-identical under ANY order (parallel chunks, atomics, split-K, tree) bounds each operand's mantissa to b bits under ONE shared power-of-two scale per row/column, so every product and partial sum is exactly representable and every reduction order yields the same bits — the third posture: the neighbours either PIN the reduction order or spend exactness on a narrower dtype; this FREES the order".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "// A parallel/chunked reduction whose BITS depend on the chunking — and\n// a tolerance check that waves the drift through. Every rerun with a\n// different worker count (or split-K width, or atomic arrival order)\n// can produce a different low-bit result.\nfn dot_any_order(a: &[f64], b: &[f64], workers: usize) -> f64 {\n    let chunk = a.len().div_ceil(workers);\n    let mut partials: Vec<f64> = Vec::with_capacity(workers);\n    for c in 0..workers {\n        let lo = c * chunk;\n        let hi = ((c + 1) * chunk).min(a.len());\n        let mut acc = 0.0f64;\n        for i in lo..hi {\n            acc += a[i] * b[i];\n        }\n        partials.push(acc);\n    }\n    partials.iter().sum()\n}\n\nfn matches(a: f64, b: f64) -> bool {\n    (a - b).abs() <= (a.abs() + b.abs()) * 1e-12 // tolerance: blind to the\n    // low-bit drift a determinism-gated consumer cannot absorb\n}".to_string(),
            fixed_code: "// Order-free exact accumulation: quantize each operand to B-bit\n// mantissas under ONE shared power-of-two scale per operand set (the\n// peak lifts the largest element to the grid top), then accumulate —\n// every product k_a*k_b*2^(ea+eb-2B) and every partial sum stays an\n// exact multiple of the grid unit while inner * 2^(b1+b2) <= 2^53, so\n// ANY order (and any worker count) reproduces these bits.\nconst B: i32 = 16; // mantissa bits per operand (the 16+16 row)\nconst F64_SIG_BITS: u64 = 53;\n\nfn peak_exponent(row: &[f64]) -> i32 {\n    let mut e: i32 = -1022;\n    for &x in row {\n        if x != 0.0 {\n            let ex = ((x.abs().to_bits() >> 52) & 0x7ff) as i32 - 1022;\n            e = e.max(ex);\n        }\n    }\n    e\n}\n\nfn pow2(e: i32) -> f64 {\n    f64::from_bits(((e + 1023) as u64) << 52)\n}\n\nfn grid_round(x: f64, scale: f64, inverse: f64) -> f64 {\n    (x * scale).round_ties_even() * inverse\n}\n\n/// The loud precondition: refuse past the budget BY NAME, admit AT it.\nfn check_budget(inner: usize, b1: i32, b2: i32) -> Result<(), String> {\n    if (inner as u64) > (1u64 << (F64_SIG_BITS - (b1 + b2) as u64)) {\n        Err(format!(\"inner dimension {inner} overflows the exact float53 range\"))\n    } else {\n        Ok(())\n    }\n}\n\nfn dot_exact_any_order(a: &[f64], b: &[f64], workers: usize) -> Result<f64, String> {\n    check_budget(a.len(), B, B)?; // 16+16: inner <= 2^21\n    let ea = peak_exponent(a);\n    let eb = peak_exponent(b);\n    let (sa, ia) = (pow2(B - ea), pow2(ea - B));\n    let (sb, ib) = (pow2(B - eb), pow2(eb - B));\n    let chunk = a.len().div_ceil(workers);\n    let mut partials: Vec<f64> = Vec::with_capacity(workers);\n    for c in 0..workers {\n        let lo = c * chunk;\n        let hi = ((c + 1) * chunk).min(a.len());\n        let mut acc = 0.0f64;\n        for i in lo..hi {\n            acc += grid_round(a[i], sa, ia) * grid_round(b[i], sb, ib);\n        }\n        partials.push(acc);\n    }\n    Ok(partials.iter().sum())\n}\n\nfn bit_identical(a: f64, b: f64) -> bool {\n    a.to_bits() == b.to_bits() // the compare is BITS, never a tolerance\n}".to_string(),
            context_notes: Some("NotBenchable-by-model (bench_spec: None — the win metric is CROSS-ORDER BIT-IDENTITY on determinism-gated surfaces, never latency; the pre-registered G2 falsifier FIRED at the landing cell and the bench claim demoted to decision content per the plan: quantized-unrolled-4 vs raw-sequential = 1.1x / 1.0x / 1.0x at inner 4096 / 16384 / 65536 — parity at best, never a win — and 2.9x / 2.8x / 2.8x against the equally-order-free raw-unrolled-4 control (m3 macOS aarch64, rustc -O, min-of-31 interleaved closures, load 6.3-8.7; the quantization pass is real work and the portability win is the point — the Batch-136 annotation class, measured bases intact). SOURCE (nathanrs/tunazip @ 571509687de3bd7301af0f79d75893b3e83d2417, tip UNMOVED at T0 + re-checked at authoring, MIT; tunazip <- tufazip <- NNCP <- Nayuki all MIT-cited in NOTICE; original fixtures, never source code): the fork's whole contribution is making train-while-coding arithmetic bit-exact across GPU models (H100 -> B200/A100/L40S/A10/T4 decode byte-exact; a float-mode file decodes 99.9% garbage off the authoring GPU — the blog's motivating measurement, PROVENANCE never a bar). THE MECHANISM (detops/gemm.py): 'exact_matmul' grid_rounds operands to <= 16 significant bits with ONE shared power-of-two scale per a-row / b-column (peak_of = +/-inf-norm over the reduction dim, fixed.py:67-68; scale = pow2(BITS - exponent) built by EXPONENT-FIELD bit manipulation, never libm — frexp-free and device-identical by construction, fixed.py:33-35 + the host pow2 twin '((exponent + 1023) << 52).view(float64)'), then an f64 matmul. THE LOUD PRECONDITION (gemm.py:23-24 verbatim): 'if a.shape[-1] * 2 ** (bits_a + bits_b) > FLOAT64_EXACT_INTEGER_LIMIT: raise ValueError(f\"inner dimension {a.shape[-1]} overflows the exact float64 range\")' — FLOAT64_EXACT_INTEGER_LIMIT = 2 ** 53. THE ARITHMETIC (re-derived independently, verdict rounds 0 + 2 concur): after scaling, every operand is an integer k (|k| < 2^(b1+b2)... per-side |k_a| < 2^b1, |k_b| < 2^b2) times one fixed pow2 per row/column pair, so EVERY product in a cell's sum is an integer multiple of ONE grid unit and the partial sums are integers < inner * 2^(b1+b2) — exactly representable in f64 while 'inner_dim * 2^(b1+b2) <= 2^p' holds, hence EVERY summation order computes the same exact value. Budget rows (reviewer-re-derived): f64 (p=53): 16+16-bit operands -> inner <= 2^21 (the blog's row — serves the ORACLE / G5-reference lane where f64 is the reference substrate); f32 (p=24): 8+8 -> inner <= 2^8 = 256; 12+12 -> inner <= 1, NO accumulation room — refuse (our serving surfaces are f32: the transferable band is the low-bit-operand class or the int64 variant, NOT the f64 16/16 row on wide inner dims). THE INT64 VARIANT TRADES THE BUDGET, NEVER FREE (fixed.py:83-89 'exact_sum', the reviewer-pinned wording): 'bits = min(SUM_FRACTION_BITS, INT64_SUM_BITS - (terms - 1).bit_length())' — min(40, 62 - ceil_log2 terms): the significand budget buys the ACCUMULATOR's overflow budget instead (quantized integers stay < 2^bits so the int64 sum of `terms` of them cannot wrap), and the quantization to that width is ITSELF LOSSY — 62 - log2(terms) bits is an overflow budget, never 'free exactness'. THE LOSSY SIDE (round-1 correction, binding): a shared power-of-two exponent per row/column is a LOSSY transform, and the fixture measured the honest nuance — the pow2 grid is LOGARITHMIC (a per-row peak lift), so wide dynamic range ALONE is not the loss axis (a logspace +/-12-decade row measured only 3.6e-5 relative delta, inside the benign envelope) — the losing regime is a SMALL SIGNAL INSIDE AN OPERAND relative to a LARGE PEAK IN THE SAME REDUCTION: a 2^-20 perturbation on a 2^0 carrier sits below the carrier's ~2^-17 grid ulp and was lost ENTIRELY (delta 1.0, b418 F4). The lossy-surface promotion law binds: per-family deployed-path retention (conditional), never bit-equality-against-i128 alone — that certifies DETERMINISM, not QUALITY; the quality leg is the source's own 'assert_matches_float' det-vs-float template (their GRADIENT_TOLERANCE 1e-6 is THEIR provenance, never our bar; our benign envelope is derived: 4*sqrt(n)*2^-(B+1)). THE DISTINCT-ROLE SENTENCE (verbatim, the fold-guard agreed in verdict round 2): 'The neighbours either PIN the reduction order (shape-only-plan-row-invariant-kernel gpu_56/B188, and the house katgpt-rs Issue 940 law) or use exactness to LICENSE a narrower dtype (exact-widening-enables-fp16-pv-accumulation gpu_48/B178, deferred-bias-ternary-dp4a-exact-isum gpu_49/B180); R1 is the third posture — bounding the operand mantissas makes every reduction order (parallel, atomic, split-K, tree) produce identical bits, freeing the order instead of pinning it or spending the exactness on dtype.' FURTHER KIN (cited, never merged): rademacher-signs-over-gaussian-projection (gpu_04 — exact-valued operand CHOICE axis), the ane pow2-prescale family + gpu_102/gpu_148 (exact RESCALE by pow2 — range axis, zero rounding; R1 rounds INTO the grid), entries_88 reference-round-replicated-in-integer-arithmetic (reference-parity CONSTRUCTION for one rounding decision — R1 is accumulation-order freedom), gpu_12 virtual-thread-folding-preserves-reduction-tree (the tree-replay pole: replay the original order when parallelizing — R1 makes replay unnecessary while the budget holds), int8-screen-error-bound-certifies-exact-rescore (gpu_121 — error BOUNDS for screening, not operand bounding). DEDUP RE-VERIFIED at the landing HEAD (3+ vocabularies, every hit read): mantissa / significant-bit / reduction-order / order-free / order-insensitive / exact-accumulation / exact-matmul / exact-sum / 2^53-budget / operand-bound / shared-exponent / power-of-two-scale sweeps — zero owner of operand-bounding-for-order-insensitive-accumulation (the hit-by-hit rulings live in this file's header). THE ADVERSARIAL-FIXTURE SPEC (residue read, folded): exactness claims are proven against the integer oracle on the two adversarial fixtures, never random alone — all-max (monotone partial sums, bounded: 127^2 * 1024 < 2^24) and alternating-sign cancellation; b418_perf_fixtures.rs F1 runs all three classes x four orders x the i128 reference (F2 the thread-invariance shape, F3 the loud refusal, F4 the lossy disclosure + the benign envelope, F5 the int64 overflow budget). REGIME: oracle / G5-reference lanes (f64 as the reference substrate), deterministic-replay and cross-device-parity verification paths, parallel folds on determinism-gated surfaces where the budget holds. ANTI-REGIMES: f32 serving paths whose inner dims exceed the 8+8 budget of 256 (refuse, never silently narrow); results riding cancellation or sub-grid signals (the losing regime — the transform is lossy there); consumers needing the unquantized values' full precision. UPSTREAM: present at the pin (their landed decision — decision content only, never a PR; the outbound-PR lane is EMPTY by construction). Batch 418 / Plan 414 / Research 449.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
