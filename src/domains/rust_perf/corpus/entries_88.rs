// Batch 375 — the laya round-2 distill (the staged unread-sweep candidates)
// (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634,
// Apache-2.0 — STILL the remote tip at the 2026-11-20 clone, no re-pin;
// the pin is 2 commits past the v0.5.0 tag. T0 two reader subagents
// re-derived every staged quote fresh at the pin — the prior session's
// subagent transcripts died with its thread, the snapshot's
// re-derivation note honored; citation corrections carried into the
// notes below: candidate-1's "binning around L514-521" is the measured-
// numbers DOCSTRING, the binning CODE is calibrate.py:548-551; candidate
// 2's mechanism spans two functions, round4 + decimal_quotient_to_f64,
// and its true birth is the 2026-10-19 fix commit 28adc552 — the naive
// `(value * 10_000.0).round_ties_even() / 10_000.0` it replaces survives
// verbatim in that diff and is the rule's pinned negative).
//
// rust_perf 373 → 378: R1 `gate-cut-fitted-on-serving-representation`
// (measurement-honesty, the staged A−; three faces across two languages)
// + R2 `reference-round-replicated-in-integer-arithmetic` (arithmetic,
// the staged B+/A−; the reference-runtime-parity family seed — the
// staged benchability question resolved NOT-BENCHABLE-BY-MODEL, the
// deciding metric is bit-identity with CPython and no wall validator
// hosts a Python oracle) + R3 `deepcopy-snapshot-before-untrusted-
// mutation` (allocation, the staged B+; opposite polarity of
// clone-before-mutating-loop-borrow-appeasement, adjudicated in-rule) +
// R4 `artifact-load-replaces-whole-state` (caching, the staged B+) + R5
// `lower-bound-precheck-understates-never-fabricates` (measurement-
// honesty, the staged B+). The batch's sec half (S1 `per-item-bound-
// not-batch-max`) lands in `src/domains/sec/corpus.rs` in the same
// commit. The count-neutral FACE (the reporting-round positive atom)
// is folded in-notes on R2.
//
// Honest T0 drops + stagings recorded here: the `stride-pair-leniency-
// vs-refusal` staged row (common.py window_budget L440-459, verified at
// the pin) DROPPED — a single-face admission-attribution law whose Rust
// fix space is a warning-vs-refuse policy decision with no bounded
// detector shape; the A1 `calibration-apply-covers-its-fit-stale-passes-
// through` candidate (vega.py L1086-1136, verified) STAGED to the queue
// for the next batch (three coordinated halves are a batch of their
// own); the A2 `bool-impostor-passes-a-numeric-gate` candidate
// (calibrate.py L523-538 + common.py L470-490, verified, two instances
// + two named sites) STAGED (Python bool is int is a language-axis the
// Rust face — sentinel-vs-value + untagged-serde coercion — needs its
// own round); the `per-char-regex-classification` candidate
// (laya_email.rs PyRegex, verified benchable-on-non-ASCII) STAGED (the
// fixture needs the unicode-general-category crate — the R7 lesson's
// twin, never relitigated blind).
//
// Fixtures `corpus/fixtures/b375_fixtures.rs` F1–F5 are ORIGINAL
// plain-Rust reproductions (zero repo source pasted), standalone-compiled
// AND executed BEFORE entry (8/8 green; F2's oracle is a GENERATED
// CPython table — gen_oracle.py, python3 seed 375, 151 rows bit-for-bit —
// plus the reporting-round face vector, seed 9375). NOTHING INVENTED.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

// ── R1: fit the gate cut on the representation the gate compares ──
pub fn r1_gate_cut_fitted_on_serving_representation() -> PerfRule {
    PerfRule {
        rule_id: "gate-cut-fitted-on-serving-representation".to_string(),
        name: "a gate's cut fitted at one numeric representation and applied at another gates a quantity nothing measured: fit or derive any threshold on EXACTLY the representation the comparator will read at serve time — bin, quantize, or round BEFORE the cut is chosen, never after; a full-precision cut can sit above the rounded value of its own cohort (0.8333 < 0.83333…) and abstain all of it, and the same cut applied to binned values admits wrong answers it measured as rejected (their measured 9.8% error holds 25.6% once the cut is compared binned); the length face: a size gate measures the SERIALIZED text the consumer produces, never a proxy rendering (quote-escaping doubles admission at the ceiling; escape spelling refuses a sixth of the limit)".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: the cut fitted at full precision, served against the
// binned/rounded values the runtime reports. The cut is a
// number on a scale the gate never sees: it can sit ABOVE
// the rounded value of its own cohort (abstaining it) and
// BELOW the binned value of answers it measured as wrong.
fn fit_cut(recs: &[Rec], target_err: f64) -> f64 {
    // candidates at FULL precision; errors measured at full
    // precision too
    let mut cands: Vec<f64> = recs.iter().map(|r| r.conf).collect();
    cands.sort_by(|a, b| a.total_cmp(b));
    let mut best = (f64::INFINITY, 0usize);
    for &cut in &cands {
        let mut cov = 0; let mut err = 0;
        for r in recs {
            if r.conf >= cut { cov += 1; if !r.correct { err += 1; } }
        }
        if cov > 0 && (err as f64) / (cov as f64) <= target_err && cov > best.1 {
            best = (cut, cov);
        }
    }
    best.0 // a full-precision number
}

fn gate(recs: &[Rec], cut: f64) -> (usize, usize) {
    // SERVE: the runtime bins/rounds before anything reads it
    let mut cov = 0; let mut err = 0;
    for r in recs {
        if serve_rep(r.conf) >= cut { cov += 1; if !r.correct { err += 1; } }
    }
    (cov, err)
}"#.to_string(),
        fixed_code: r#"// FIXED: transform BEFORE the cut is chosen — candidates AND
// errors on the serving representation, so the fitted cut is
// a number on the scale the gate actually compares.
fn fit_cut_on_serving_rep(recs: &[Rec], target_err: f64) -> f64 {
    let mut cands: Vec<f64> = recs.iter().map(|r| serve_rep(r.conf)).collect();
    cands.sort_by(|a, b| a.total_cmp(b));
    cands.dedup();
    let mut best = (f64::INFINITY, 0usize);
    for &cut in &cands {
        let mut cov = 0; let mut err = 0;
        for r in recs {
            if serve_rep(r.conf) >= cut { cov += 1; if !r.correct { err += 1; } }
        }
        if cov > 0 && (err as f64) / (cov as f64) <= target_err && cov > best.1 {
            best = (cut, cov);
        }
    }
    best.0
}

fn gate(recs: &[Rec], cut: f64) -> (usize, usize) {
    let mut cov = 0; let mut err = 0;
    for r in recs {
        if serve_rep(r.conf) >= cut { cov += 1; if !r.correct { err += 1; } }
    }
    (cov, err)
}

// the length face: the gate measures the WIRE encoding
fn state_wire_len(s: &str) -> usize { /* JSON-style escaping */
    let mut n = 2;
    for c in s.chars() { n += if c == '"' { 2 } else { c.len_utf8() }; }
    n
}
fn state_gate(body: &str, cap: usize) -> bool { state_wire_len(body) <= cap }"#.to_string(),
        context_notes: Some("Grade A− — suggestion-only, honestly NotBenchable-by-domain (the harm is a gate verdict made on numbers the gate never sees — an error-rate axis a wall/alloc validator cannot host; the metric-mismatch class, `bench_spec: None`). SOURCE (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634, Apache-2.0 — still the remote tip at the 2026-11-20 clone, no re-pin; T0 re-derived fresh at the pin, the prior session's transcripts dead). THE LAW, calibrate.py:516-517 at the pin verbatim: 'the runtime recalibrates `answer_confidence` through that map before anything reads it, so a cut fitted without it is a cut on a scale the gate never sees.' THE MEASURED PREMISE (the docstring's own numbers, calibrate.py:518-521 verbatim): 'Measured on 1,200 synthetic 12-option records at `target_error=0.10`: the cut fitted without a map holds 9.8% error over 50% coverage on un-binned confidences, and admits 94.5% of answers at 25.6% error once the same number is compared against binned ones.' THE ROUNDING FACE (calibrate.py:552-555 verbatim): '`_decode_answers` reports `answer_confidence` rounded to 4 decimals, after binning, and the gate compares that. A cut picked at full precision can sit above the rounded value of its own cohort (bin 5/6 is reported as 0.8333 < 0.83333...), which abstains all of it.' — with the transform-before-fit code at :548-551 ('The runtime bins before anyone reads `answer_confidence` (`Agent._decode_answers`), so the cut has to be chosen on the binned scale or it gates a different quantity.'), and the serving-side faces at onnx_agent.py:757/768 (`round(confidence_from_probs(p, k), 4)`) and serve.py:288. THE LENGTH FACE (the third instance, serve.py:1031-1034 verbatim): '`repr` quotes a value with `\\'` and leaves a `\"` inside it one character, where JSON escapes it to the two characters `\\\"`. A `{{\"body\": '\"' * 49988}}` state has a `len(str())` of exactly 50000, so it passed the gate, and then serialized to 99988 -- the documented 50000-character limit admitting very nearly twice what it says.' — and the over-refusal direction :1035-1037 (a zero-width space renders six proxy chars, one wire char: 'a state that serializes to 8344 characters -- a sixth of the limit -- was refused with \"state too large (50004 > 50000 chars)\"'). THE FIX-SPACE LAW: fit/derive on the comparator's representation — TRANSFORM-ORDER is the atom (the tamper arm proves either order mismatch fails, not just the naive one); the length face spells it as 'measure the exact wire encoding (serde_json output, not Debug/Display)'. THE TWO-LANGUAGES LEG: only indirectly source-verifiable from the Python tree (the SDK-goldens comment, common.py:1015-1017 — the laya-rust/dotnet/ts ports must reproduce these numbers); carried as the fixture's representation-pair generality, not a cross-language gate quote. REGIME: any thresholded gate whose compared quantity is transformed between fit and serve (calibration bins, report rounding, quantization, serialization escaping/lengths, unit conversions); fit-order vs serve-order mismatches of ANY kind. ANTI-REGIMES: gate and fit already share one representation (the fixture's fixed arm IS the no-change case); transforms with no measured ordering effect (bucketing adds fitting burden for nothing); gates on values never transformed downstream. DISTINCT-FROM (T1 re-greps at HEAD, 3 vocabularies: gate.cut/cut.fitted/threshold.representation/binned.cut = ZERO owners; repr.length/serialized.length = zero owners): `confidence-threshold-bucketed-by-answer-shape` (entries_87) owns the POOL axis (one flat threshold across answer shapes — bucket the threshold); THIS owns the REPRESENTATION axis (one threshold, two scales — transform before fit); compose (a well-calibrated gate needs both: per-shape buckets fit on the serving representation); `calibrated-dispatch`'s registration law owns evidence thresholds for a RETRIEVAL narrowing — decision-gate placement, not numeric representation; sec's `capacity-exceeded-silent-fallback-not-refusal` owns the lying-CAP refusal — THIS owns the gate/fit scale mismatch (an honest cap measured on the wrong encoding). Fixture: b375_fixtures.rs F1 (the fit-looks-perfect/serve-blows-2x-target pin + the fixed arm holds target + the reversed-order tamper's fit-coverage non-transfer + the length face both directions). Plan 391 / Research 425 (round 2).".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R2: a port replicates the reference runtime's rounding in integers ──
pub fn r2_reference_round_replicated_in_integer_arithmetic() -> PerfRule {
    PerfRule {
        rule_id: "reference-round-replicated-in-integer-arithmetic".to_string(),
        name: "a port that must match a reference runtime's rounding bit-for-bit rounds the EXACT binary value in integer arithmetic — decompose the double (sign/exponent/mantissa), form the exact scaled rational in a wide integer (|x|·10^4 = scaled/2^shift, scaled < 2^53·10^4 < 2^67), make ONE half-to-even decision on the exact value, and convert back with a single rounding (below 2^53 one IEEE division; beyond it divide in integers with spare bits and fold the remainder into a STICKY BIT so the conversion rounds once) — never scale-multiply-and-round in f64, whose product is itself rounded and destroys tie-adjacency (0.00625's double sits a hair above the 62.5 tie; the product collapses it to exactly 62.5 and half-even picks 62 — the reference rounds the exact value and gives 63; measured ~2% of values differ)".to_string(),
        category: "arithmetic".to_string(),
        buggy_code: r#"// NAIVE: the port's round-to-4 as three f64 roundings — the
// first on a ROUNDED product. Tie-adjacency dies in the
// product: values whose exact scaled rational sits a hair
// above/below a half collapse ONTO the tie and half-even
// picks the neighbour the reference never picked.
fn round4_port(value: f64) -> f64 {
    (value * 10_000.0).round_ties_even() / 10_000.0
}

// the consumer: every reported confidence passes through it,
// and the cross-runtime goldens compare these numbers
fn report(conf: f64) -> f64 {
    round4_port(conf)
}"#.to_string(),
        fixed_code: r#"// FIXED: the exact-value replica. One half-even decision on the
// exact scaled rational; the conversion back rounds ONCE via the
// sticky bit. Bit-identical to CPython's round(x, 4) on the
// generated 151-row oracle (incl. -0.0, subnormals, the sticky
// path past 2^53, the tie specimens 0.00625→0.0063 /
// 0.03125→0.0312), NaN/inf pass through.
fn round4_exact(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let bits = value.to_bits();
    let negative = bits >> 63 == 1;
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, exponent) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), biased - 1075)
    };
    if exponent >= 0 || mantissa == 0 {
        return value; // integral (or ±0.0): exact at four decimals
    }
    let shift = (-exponent) as u32;
    let scaled = (mantissa as u128) * 10_000; // the EXACT |x|·10^4
    let quotient: u128 = if shift >= 128 {
        0
    } else {
        let whole = scaled >> shift;
        let remainder = scaled & ((1u128 << shift) - 1);
        let half = 1u128 << (shift - 1);
        if remainder > half || (remainder == half && whole & 1 == 1) {
            whole + 1
        } else {
            whole
        }
    };
    let magnitude = quotient_to_f64_once(quotient);
    if negative { -magnitude } else { magnitude }
}

fn quotient_to_f64_once(quotient: u128) -> f64 {
    if quotient < (1u128 << 53) {
        return quotient as f64 / 10_000.0; // exact: one IEEE division
    }
    const EXTRA: u32 = 40;
    let numerator = quotient << EXTRA;
    let mut q = numerator / 10_000;
    if numerator % 10_000 != 0 {
        q |= 1; // the sticky bit: the conversion rounds once
    }
    q as f64 / (1u64 << EXTRA) as f64
}"#.to_string(),
        context_notes: Some("Grade B+ (the strong end of the staged band) — decision-content, honestly NotBenchable-by-MODEL (the deciding metric is bit-identity with CPython's round(x, 4) — a Python oracle no wall/alloc validator hosts; the honest wall expectation is TIE-to-integer-path-SLOWER on the common [0,1) path (the integer arm does strictly more work and the sticky path is dead at every in-crate call site, reachable only via the pub API at |v| ≥ ~5.4976e8) — bench_spec: None, the B372 funnel-shift MIXED shape is NOT this: there both arms were correct and perf decided; here the naive arm is WRONG on the tie class and correctness decides). SOURCE (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634, Apache-2.0 — still the remote tip; T0 re-derived fresh). THE LAW (laya-rust/src/calibration.rs:154-159 at the pin verbatim): 'Python's `round(x, 4)`, exactly: the *exact binary value* of `x` is rounded half to even at four decimals (so `round(0.00625, 4)` is `0.0063` because the double is a hair above the tie, while `round(0.03125, 4)` is `0.0312` because that one is an exact tie), and the result is the double nearest that decimal, as CPython's `float.__round__` returns it. The sign is kept, so a negative value that rounds to zero gives `-0.0`. NaN and the infinities pass through unchanged. Integer arithmetic only: no `x * 10_000.0`, whose product is itself rounded.' THE MECHANISM IS TWO FUNCTIONS: `round4` (:160-195, the decompose + the exact scaled rational + the one half-even decision) + `decimal_quotient_to_f64` (:198-212, the sticky-bit single-rounding conversion; the doc :197 verbatim: 'The double nearest `quotient / 10_000`, correctly rounded for any `quotient < 2^67`' — with :203-204 naming the double-rounding: 'Beyond 2^53 converting first would round twice. Divide in integers with 40 extra bits and fold the remainder into a sticky bit; the conversion then rounds once.'). THE PINNED NEGATIVE (verbatim from the fix diff, commit 28adc552, 2026-10-19 — the mechanism's true birth, three weeks after the file's 2026-09-28 scaffold, verified via git log at T0): the pre-fix line WAS `(value * 10_000.0).round_ties_even() / 10_000.0`, and the commit message carries the measured defect rate: 'The old scale-and-round differed on about 2% of values (0.00625 gave 0.0062, Python 0.0063).' — the pre-fix comment had documented-and-dismissed the divergence ('well within the parity tolerances'), the exact documented-dismissal shape the rule kills. THE ORACLE (tests/round4_tests.rs + tests/round4_python_cases.txt): 20,000 Python-generated (input bits, output bits) rows — 'against Python's builtin `round(x, 4)`, bit for bit … so `-0.0` and one-ulp differences are caught'; our fixture carries the GENERATED 151-row slice (gen_oracle.py, seed 375) incl. both tie specimens + -0.0 + subnormals + the sticky path + the quotient-0 arm. CALL-SITE HEAT (the naive-port regime): laya_engine.rs L635/910/913/923(per-label)/940(per-probability `map(Calibration::round4)`)/941/963/964 — every probability of every answer; the sibling wrappers (`python_sum` confidence.rs:213-239 the Neumaier-compensated CPython sum port; `PythonJson::repr` python_json.rs:144-232 the shortest-round-trip float repr; the `float(str)` parse lang_temperatures.rs) are the FAMILY: reference-runtime-parity, four mechanisms in one crate — the family SEEDS here, siblings cited never re-owned. FACE (count-neutral, folded — the reporting-round positive atom): common.py:1011-1017 verbatim: 'summing in a different order moves the last bits (1.7e-15 over 136k measured distributions). Both callers `round(..., 4)` this, so that drift is invisible today; it is the SDK goldens in laya-rust, laya-dotnet and laya-ts that have to keep reproducing these numbers, and there is no reason to spend bit-exactness on a microsecond.' — where a value crosses to consumers at a pinned decimal width, the WIDTH (not the interior fold order) is the contract; the complement of the house Issue-940 law (sync-crossing reductions stay order-pinned; locally-consumed values behind a pinned reporting width are order-free) — in-rule adjudicated: Issue-940 owns the raw-sync half, this face the pinned-width-consumer half, compose. REGIME: any port replicating a reference runtime's arithmetic bit-for-bit (cross-runtime goldens, SDK parity lanes, replay determinism against a Python/JS reference; the decimal-quantize family at any width). ANTI-REGIMES: no reference to match (use the plain form; the integer machinery is pure cost); values already integral at the target width; a lossy-tolerant consumer (the parity tolerance the pre-fix comment leaned on is a DECISION, and this rule exists for when the goldens say it is not one). DISTINCT-FROM (T1 re-greps, 3 vocabularies: python.round/round4/sticky.bit = ZERO owners; correctly.rounded = entries_03 prose mentions + kernel exact-tie family, none the law; reference.parity = one kernel hit, a different axis): the kernel_opt exact-tie/exact-ordering family (gpu_158 et al.) owns EXACTNESS of OUR OWN numerics — THIS owns replication of a REFERENCE's numerics (the direction B366's rounding-bugs family does not cover either: that fixes our bugs, this matches theirs); `u128-funnel-shift-combine` (entries_76) owns the two-shift UB shape — adjacent arithmetic, different law; sec's Numerics family (gelu-tanh cancellation et al.) owns precision-loss bugs — THIS owns cross-runtime tie-class divergence. Fixture: b375_fixtures.rs F2 (the 151-row oracle bit-for-bit + the naive arm diverges on the tie specimen + the rare-class anti-regime (<25% divergence) + the reporting-round face arms). Plan 391 / Research 425 (round 2). FACE (Batch 398, Plan 398 / Research 432, count-neutral — the composed-chain rounding instances + the exactness-CONTRACT table): ddalcu/mlx-serve @ 96311b26 (MIT) lands three atoms on this rule's reference-parity axis — (a) the composed-chain rounding hazard: gdn_prework's ones-weight rms_norm T-cast chain and hc_prefill_mix.metal:11-12 'MLX mean materializes its reciprocal in the input dtype before multiply. The rounding matters for HC=3/5/6/7; FP32 division is a different operation' — a composed op whose intermediate (the reciprocal) materializes at a different rounding than the reference's formulation is a DIFFERENT operation on the HC cases that matter; (b) the JIT-vs-metallib twin (docs/CUDA.md:81-89): a JIT 'bf16 + bf16' add vs the metallib's add, ONE ULP apart — the same arithmetic, two compilation regimes, the reference-parity question is compilation-shape-dependent; (c) the F15 EXACTNESS-CONTRACT TABLE rides here (the Issue-940 published-contract form): the source publishes a per-plugin contract table (exact_drafted / exact_resumed / exact_concurrent, enforced by src/abi/exactness_test.zig, undeclared classes named honestly) — the published-contract discipline this rule's reference law implies: every exactness claim is a DECLARED row with an enforcement test, never prose. FACE (Batch 398, Plan 398 / Research 432, count-neutral — the composed-chain rounding instances + the exactness-CONTRACT table): ddalcu/mlx-serve @ 96311b26 (MIT) lands three atoms on this rule's reference-parity axis — (a) the composed-chain rounding hazard: gdn_prework's ones-weight rms_norm T-cast chain and hc_prefill_mix.metal:11-12 'MLX mean materializes its reciprocal in the input dtype before multiply. The rounding matters for HC=3/5/6/7; FP32 division is a different operation' — a composed op whose intermediate (the reciprocal) materializes at a different rounding than the reference's formulation is a DIFFERENT operation on the HC cases that matter; (b) the JIT-vs-metallib twin (docs/CUDA.md:81-89): a JIT 'bf16 + bf16' add vs the metallib's add, ONE ULP apart — the same arithmetic, two compilation regimes, the reference-parity question is compilation-shape-dependent; (c) the F15 EXACTNESS-CONTRACT TABLE rides here (the Issue-940 published-contract form): the source publishes a per-plugin contract table (exact_drafted / exact_resumed / exact_concurrent, enforced by src/abi/exactness_test.zig, undeclared classes named honestly) — the published-contract discipline this rule's reference law implies: every exactness claim is a DECLARED row with an enforcement test, never prose.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R3: the validation snapshot a hook cannot rewrite ──
pub fn r3_deepcopy_snapshot_before_untrusted_mutation() -> PerfRule {
    PerfRule {
        rule_id: "deepcopy-snapshot-before-untrusted-mutation".to_string(),
        name: "a validation sized against an input must read a SNAPSHOT taken BEFORE extension/hook code can rewrite the input in place — hand plugins an owned (deep) snapshot of any state a later assertion reads, never re-validate a structure a callback can still mutate: `==` over the caller's mapping cannot see an in-place rewrite (a hook adding options to a nested dict mutates the same container both sides read), a SHALLOW copy shares the nested containers and misses it exactly the same way, and the harm is silent — the budget check passes, the windows are re-truncated, and usage reports them all read while measured tokens never reached the model".to_string(),
        category: "allocation".to_string(),
        buggy_code: r#"// NAIVE: the scan sizes against the questions, hooks run, and
// the budget check compares the counts — read through the SAME
// live structure the hook widened. Both sides move together:
// the check passes, windows re-truncate, coverage is claimed.
fn run_scan(questions: &[Question], window_cap: usize) -> Report {
    let windows = plan_windows(questions);
    run_hooks(questions); // the UNTRUSTED extension surface
    let asked: Vec<usize> = questions.iter().map(|q| q.option_count()).collect();
    // asked == current: the check sees itself
    if questions.iter().map(|q| q.option_count()).collect::<Vec<_>>() == asked {
        return Report::full_coverage(windows); // the lie
    }
    Report::truncated()
}"#.to_string(),
        fixed_code: r#"// FIXED: snapshot (DEEP — nested containers copied) BEFORE any
// hook can run; the check reads the snapshot against the live
// structure and a rewrite is a detected mismatch, never a lie.
fn run_scan(questions: &[Question], window_cap: usize) -> Report {
    let windows = plan_windows(questions);
    let asked: Vec<Vec<String>> = questions.iter()
        .map(|q| q.options_snapshot()) // deep: the nested lists copied
        .collect();
    run_hooks(questions); // may widen in place — the snapshot cannot move
    let current: Vec<usize> = questions.iter().map(|q| q.option_count()).collect();
    let asked_counts: Vec<usize> = asked.iter().map(|v| v.len()).collect();
    if current == asked_counts {
        Report::full_coverage(windows)
    } else {
        Report::rewrite_detected() // the widened input re-plans LOUDLY
    }
}"#.to_string(),
        context_notes: Some("Grade B+ — decision-content, honestly NotBenchable (correctness: the harm is coverage claimed over tokens never read — a validation-integrity axis, `bench_spec: None`). SOURCE (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634, Apache-2.0 — still the remote tip; T0 re-derived fresh). THE LAW (onnx_agent.py:546-552 at the pin verbatim): 'Snapshot the questions the scan was just sized against, BEFORE any start hook can rewrite them, as the torch `predict_long` does (5f29170). `==` over the caller's mapping cannot see an in-place rewrite: a hook that adds options to `ctx.questions[q][\"criteria\"]` -- the `widen_for_high_cardinality` pattern in `docs/hooks/patterns.md` -- mutates the same nested dict, so both sides change together, `_check_scan_budget` returns early, and the windows are re-truncated while `usage[\"windows\"]` reports them all read. A shallow copy shares the nested dicts and misses it exactly the same way.' — the snapshot at :553 (`asked = copy.deepcopy(questions)`), the MEASURED harm at :567-570 verbatim: 'Without it a document short enough to fit one window was silently truncated by a re-budgeting hook and still reported `windows: 1`, i.e. \"the model read all of it\" -- measured, 138 of 240 state tokens never reached the model, while a longer document on the identical input hard-failed.' THE SECOND INSTANCE (same file, verified): the states snapshot at :588-594 ('A copy, so a start hook that mutates `ctx.states` in place cannot shift `starts.' — `results = self.predict_batch(list(windows), …)`) — two snapshot sites in one function family. THE RUST FACE: hand plugins an OWNED or Arc-frozen snapshot of any state a later assertion reads; aliasing + interior mutability is the native shape of 'both sides change together'. DISTINCT-FROM (T1 re-greps, 3 vocabularies: snapshot.before/owned.snapshot/deep.copy — the hits are the drafter-context retention law (entries_14), the measurement-integrity snapshot (entries_82), the borrow-appeasement clone (entries_72), none THIS; validation.snapshot/pre-hook.copy = zero owners) — ⛔ ADJUDICATION vs the family, both directions: `clone-before-mutating-loop-borrow-appeasement` (entries_72) owns the borrow-appeasement clone (the clone whose ONLY purpose is ending a borrow before a mutating loop — NIGHTLY redundant_clone passes it, the POSITION is the signal); THIS owns the validation-integrity snapshot (the clone's purpose is that a later assertion reads values that cannot move — it must never be elided, and a shallow spelling is the pinned negative); opposite purposes, same genus (a deliberate clone with a lifetime purpose), compose in a lint's allow/deny reading. `mem-take-field-checkout-empty-window` (entries_72) owns the take/check-out window (nested readers see the EMPTY default) — THIS owns the pre-hook snapshot (readers see the PRE-HOOK values); different hazard sides. `eval-window-state-restore-roundtrip-detection` (entries_82) owns MEASUREMENT integrity (a scored subject must not be moved away for a measurement — its 'snapshot-before-build call-site order' is the measurement half); THIS owns VALIDATION integrity (a check must not read through a callback-mutable alias) — adjacent-distinct, the order law composes. `capacity-gated-exact-undo-loud-degrade` (entries_82) owns undo availability by byte budget — the snapshot COST axis, not the integrity axis. REGIME: any validate-then-extend path over mutable input (lifecycle hooks, pre/post processors, user-supplied callbacks, config overlays) where an assertion sized on the input is evaluated after extension code ran. ANTI-REGIMES: single-threaded input a callback structurally cannot reach (owned &mut with no re-entrant path — the snapshot is dead weight); immutable input (Arc/[T] — nothing to snapshot); checks that re-derive everything from the POST-hook state and own no pre-hook claim. Fixture: b375_fixtures.rs F3 (the aliasing claim pin + the snapshot detect + the SHARED-lists shallow negative + the late-snapshot tamper). Plan 391 / Research 425 (round 2).".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R4: an artifact load REPLACES the whole state, absences included ──
pub fn r4_artifact_load_replaces_whole_state() -> PerfRule {
    PerfRule {
        rule_id: "artifact-load-replaces-whole-state".to_string(),
        name: "loading an artifact installs the artifact's WHOLE state, absences included: a section the artifact simply never carried must CLEAR the consumer's previously-fitted state for that section, never silently merge (the merge reports `calibrated: true` for a state the directory never held, and the route rides a band the artifact's heads were never measured against) — model the artifact as a whole-state struct where `None` fields mean clear, derive status flags from the LOADED value, and give merge semantics its own explicit, loud API".to_string(),
        category: "caching".to_string(),
        buggy_code: r#"// NAIVE: merge semantics — absent sections keep the previous
// fit. A heads-only artifact (a VALID artifact: the saver omits
// calibration when the agent was never calibrated) silently
// re-arms the old fit.
fn load(state: &mut AgentState, artifact: &Artifact) {
    if artifact.heads.is_some() {
        state.heads = artifact.heads.clone();
    }
    if artifact.calibration.is_some() {
        state.calibration = artifact.calibration.clone();
    }
    // state.calibrated stays true: the route rides band B, which
    // this directory's heads were never measured against
}"#.to_string(),
        fixed_code: r#"// FIXED: replace semantics — the artifact IS the whole state;
// `None` clears. Downstream flags derive from the LOADED value.
fn load(state: &mut AgentState, artifact: &Artifact) {
    state.heads = artifact.heads.clone();
    state.calibration = artifact.calibration.clone(); // None clears the stale fit
    state.binning = artifact.binning.clone();
}

fn calibrated(state: &AgentState) -> bool {
    state.calibration.is_some() // derived from the LOADED state, never stored
}"#.to_string(),
        context_notes: Some("Grade B+ — decision-content, honestly NotBenchable (the harm is a wrong decision riding a stale band — a state-composition axis, `bench_spec: None`). SOURCE (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634, Apache-2.0 — still the remote tip; T0 re-derived fresh). THE LAW (vega.py:1027-1031 at the pin verbatim): 'A heads-only policy is a valid artefact (`save_calibration` omits calibration.json when the agent was never calibrated). Loading one onto an agent that carried a previous fit used to keep that fit's calibration and report `calibrated: True` for a state this directory never held -- and the route then rode a band the directory's heads were never measured against. What was loaded is what the directory holds.' — the clear at :1032 (`self.calibration = None`) and the derived flag at :1033 (`\"calibrated\": self.calibration is not None`). THE SECOND INSTANCE (cross-file, verified — lifts the candidate): calibrate.py:779-786 `apply_calibration_payload` verbatim: '`binning_map` is installed whatever the payload holds, including nothing: a payload with no ``binning_map`` key -- every file written before histogram binning existed, and every agent that saved before :meth:`Agent.fit_binning` ran -- installs `None`, which clears a map this object already fitted. The file is the whole calibration state, not a patch onto the current one.' — carried by onnx_agent.py:267-278 `load_calibration` → `apply_calibration_payload(self, payload)`, one shared implementation, two consumer surfaces. DISTINCT-FROM (T1 re-greps, 3 vocabularies: replace.whole/load.replaces/stale.merge = ZERO rust_perf owners; whole.state = the serving-checkpoint-lifecycle laws (entries_20/86), different axis; artifact.load = the sec trust-ladder neighborhood, the TRUST gate not the INSTALL semantics) — ⛔ ADJUDICATION vs the family, both directions: `frozen-artifact-plus-delta-tail-overlay` (entries_81) owns FRESHNESS COMPOSITION over an immutable base (cache the compiled artifact, overlay the bounded tail — the base never moves); THIS owns INSTALL semantics (the loaded artifact replaces the consumer's state wholesale — absences clear); compose (a hot-swap that installs the base + overlay still installs WHOLE, and an overlay's absent sections must not resurrect the previous install's). `stale-proposal-merges-against-shown-base` (sec) owns the merge-BASE freshness of a review proposal — the merge axis there is WHICH BASE a diff applies to, not whether absence clears; `execution-artifact-trust-ladder-before-deserialize` (sec) owns the TRUST gate before loading — compose (trust gates the load, this law defines what the load means); `resume-revalidates-provenance-before-reuse` (entries_05) owns build-artifact resume identity — the provenance axis, not install semantics; instinct A2 (monotonic atomic hot-swap, never a weighted blend) is the SAME whole-artifact instinct on the serving-posture surface — cited as family corroboration, not double-counted (different repo, different substrate). REGIME: any artifact/config/policy loader whose source format carries optional sections (model directories, calibration bundles, feature configs, saved sessions) consumed onto state that may already hold a previous version. ANTI-REGIMES: a format where absence is IMPOSSIBLE (all sections mandatory — nothing to clear); explicit patch APIs (the caller names each section — the merge is the loud contract); state freshly constructed per load (nothing previous to leak). Fixture: b375_fixtures.rs F4 (the heads-only artifact: replace clears + the route falls to default + the merge rides the stale band + the stored-flag lie + the complete-artifact anti-regime). Plan 391 / Research 425 (round 2).".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R5: a fast-path refusal gate refuses only on a proven bound ──
pub fn r5_lower_bound_precheck_understates_never_fabricates() -> PerfRule {
    PerfRule {
        rule_id: "lower-bound-precheck-understates-never-fabricates".to_string(),
        name: "a cheap fast-path gate that short-circuits an expensive serialize-then-refuse path may refuse ONLY on a PROVEN lower bound — unproven abstains (typed zero/None) and the exact measurer decides, so the fast path can never flip a verdict; the reported number is a measured UNDERSTATEMENT the refusal can quote truthfully, never `cap + 1` (the fabricated count answers 'state too large (50001 > 50000 chars)' for a 60 012-character state — and an earlier revision reintroduced exactly that); the probe prices its own cost (a bounded walk, values-inspected census disclosed), and its soundness is protected by exact-type checks — a value whose override could push the bound past truth abstains instead of pricing".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: an UNSOUND proxy (counts keys and values, ignores
// escaping — and panics into a huge number on non-plain values)
// with a FABRICATED report: once past the cap it quotes cap+1
// no matter what the state really serializes to.
fn gate(v: &Value) -> Result<usize, usize> {
    let proxy = unsound_proxy_len(v);
    if proxy > CAP {
        return Err(CAP + 1); // a count nothing measured
    }
    let exact = wire_len(v);
    if exact > CAP { Err(exact) } else { Ok(exact) }
}"#.to_string(),
        fixed_code: r#"// FIXED: the budget-bounded exact-type walk. Refuse only on the
// proven bound (the walk's own counted total); unproven returns 0
// and the exact measurer decides; Other (the subclass/set/cycle
// face) ABSTAINS — a bound that can exceed truth is worse than
// no gate.
fn gate(v: &Value) -> Result<usize, usize> {
    let over = lower_bound_walk(v);
    if over > 0 {
        return Err(over); // proven: serializing the rest cannot flip it
    }
    let exact = wire_len(v);
    if exact > CAP { Err(exact) } else { Ok(exact) }
}

fn lower_bound_walk(v: &Value) -> usize {
    let mut total = 0; let mut budget = PROBE_BUDGET;
    let mut stack = vec![v];
    while let Some(item) = stack.pop() {
        if budget == 0 { return if total > CAP { total } else { 0 }; }
        budget -= 1;
        match item {
            Value::Str(s) => { total += s.len(); if total > CAP { return total; } }
            Value::Dict(items) => stack.extend(items.iter().map(|(_, val)| val)),
            Value::List(items) => stack.extend(items.iter()),
            Value::Other => return 0, // could exceed truth: abstain
        }
    }
    if total > CAP { total } else { 0 }
}"#.to_string(),
        context_notes: Some("Grade B+ — suggestion-only, honestly NotBenchable (the refuse-path pricing is real and in-source measured — a wall validator CAN host the fast-path delta — but the DECIDING content is the honesty law (never fabricate; the abstain direction; exact types) carried as annotations either way, with a Tie/Slower arm on small states expected BY DESIGN (~0.01 ms tax on accepted states, the source's own disclosure) — landed `bench_spec: None`, the honest call: a benchable spec would measure a win the rule's fixtures must NOT promise (the win exists only on oversized bodies, and the rule must stay honest on the common case). SOURCE (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634, Apache-2.0 — still the remote tip; T0 re-derived fresh). THE LAW (serve.py:970-977 at the pin verbatim): 'Non-zero only when exceeding `cap` is certain, and 0 for \"not proven\" -- never the other way round, so a 0 sends the state on to the exact `json.dumps` measurement and no verdict changes.' / 'The value is returned rather than a bool so the 413 can quote a number something measured. It understates the true length (see below), so the message is a true statement about the state -- unlike reporting `cap + 1`, which would answer \"state too large (50001 > 50000 chars)\" for a 60 012-character state. That is the same fabricated-count flaw this gate replaces, and an earlier revision of this branch reintroduced it.' THE BUDGET (serve.py:959-963 verbatim): 'Values the walk in `_state_length_lower_bound_over` will examine before giving up. This bounds the values *inspected*, not the total work: `stack.extend` pushes a container's elements without charging them against the budget, so a 2 MiB body of many small values costs ~0.43 ms of walking before falling through to the dump it was going to pay anyway (~6% on top). An ACCEPTED state is capped at MAX_STATE_CHARS, where the walk is ~0.01 ms.' — the walk itself at :996-1020, with the EXACT-TYPES arm verbatim: 'EXACT types only, deliberately. A `dict` subclass may override `values()` while `json.dumps` reads the real items, which would let this \"lower bound\" exceed the true length -- measured: a 13-character state refused as `60000 > 50000`.' THE PRICE (the in-source measurement, :989-994): the refuse path 5.687 ms → 0.0003 ms on a 2 MiB state; the composition at :1052-1059 ('Already past the cap on a lower bound, so serializing the rest cannot change the verdict. The bound is what gets reported: it is a measured number and it understates, so the 413 never claims a length larger than the state really has.') and the except history at :1063-1071 (the old 413 reported 'a count nothing had measured'). DISTINCT-FROM (T1 re-greps, 3 vocabularies: lower.bound = the B369 measurement rule + kernel algorithmic lemmas, none THIS; precheck = one kernel hit (gpu_94), a different mechanism; fabricate/precheck.understate/cheap.gate.soundness = zero owners) — ⛔ ADJUDICATION vs the family, both directions: `work-removed-build-is-not-a-valid-lower-bound` (entries_84) owns the MEASUREMENT lower bound (a build that skips work measures a different program — executed-work census before any bound); THIS owns the REFUSAL-GATE lower bound (a cheap precheck may refuse on a proven understate, never on a proxy that can exceed truth) — same vocabulary, different subject: measurement validity vs gate soundness, neither subsumes; `request-amplification-guard-before-expensive-path` (sec, B374 R8) owns the refuse-BEFORE-work placement (the 413 precedes tokenization/inference) — COMPOSE (this rule governs how such a precheck may refuse early on a bound: proven, measured, understated; the guard governs that the refusal exists and precedes the expensive path); `capacity-exceeded-silent-fallback-not-refusal` (sec) owns the lying-cap face (a cap that silently drops) — THIS owns the false-PRECISION face (a gate that quotes counts it never measured). REGIME: any fast-path refuse/admit gate over expensive-to-measure inputs exactly (serialized-size gates, payload scans, plan-cost prechecks, quota preflights). ANTI-REGIMES: the input is cheap to measure exactly (skip the precheck — it is pure overhead); the fast path only ADMITS (admission on an unproven bound is the same flip in the other direction — the law is one-sided refusals); a bool gate that discards the measured number (the reported bound is the point — the 413 quotes truth). Fixture: b375_fixtures.rs F5 (the fabricated cap+1 pinned negative on a tiny state + the Other-abstain reaches the exact measurer + the proven refusal understates + the budget census + the poisoned-walk abstain + the ordinary-state anti-regime). Plan 391 / Research 425 (round 2).".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

pub fn b375_rules() -> Vec<PerfRule> {
    vec![
        r1_gate_cut_fitted_on_serving_representation(),
        r2_reference_round_replicated_in_integer_arithmetic(),
        r3_deepcopy_snapshot_before_untrusted_mutation(),
        r4_artifact_load_replaces_whole_state(),
        r5_lower_bound_precheck_understates_never_fabricates(),
    ]
}
