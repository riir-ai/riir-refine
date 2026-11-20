//! Batch 375 rust_perf fixture preflight — the laya round-2 (the staged
//! unread-sweep candidates, Plan 391 / Research 425): `NandhaKishorM/laya`
//! @ `416dd9ed20da2d43fc40e23712e9adceefabf634` (Apache-2.0; still the
//! remote tip at the 2026-11-20 clone). ORIGINAL plain-Rust
//! reproductions — zero repo source pasted (the MECHANISMS are
//! re-authored with our own constants and model harnesses; the source
//! quotes stay in the corpus context_notes). Compiled AND executed
//! BEFORE entry (the b363..b374 pre-entry discipline):
//! `rustc --edition 2021 --test b375_fixtures.rs && ./b375_fixtures`
//!
//! Arms per rule (buggy / fixed / anti-regime / tamper):
//! - F1 `gate-cut-fitted-on-serving-representation`: a confidence gate
//!   fitted at one numeric representation and applied at another —
//!   the buggy arm fits the cut on RAW confidences and serves it against
//!   BINNED ones (the measured error divergence: the cut sits above the
//!   binned value of its own cohort and abstains it), the fixed arm
//!   transforms BEFORE the fit; the tamper arm reverses the order
//!   (fit binned, serve raw) and diverges the SAME way — the law is the
//!   shared representation, never either specific width; the length
//!   face: a size gate must measure the SERIALIZED text, never a Debug
//!   rendering (quote-escaping diverges both directions).
//! - F2 `reference-round-replicated-in-integer-arithmetic` (+
//!   the reporting-round face): a port replicating a reference runtime's
//!   rounding must round the EXACT binary value in integer arithmetic
//!   (one half-even decision on the exact scaled rational + a sticky-bit
//!   single-rounding conversion back), never scale-multiply-and-round in
//!   f64 — the oracle is a GENERATED CPython table (151 rows, seed 375,
//!   gen_oracle.py; the tie specimens are the source's own 0.00625 →
//!   0.0063 / 0.03125 → 0.0312): the integer arm matches ALL rows
//!   bit-for-bit, the naive scale-and-round arm diverges on exactly the
//!   tie-adjacent class (the pinned negative), and most ordinary values
//!   agree (the anti-regime: the divergence is a rare boundary class).
//!   The face: two summation orders that differ in RAW bits agree at the
//!   pinned reporting width — the width, not the fold order, is the
//!   observable.
//! - F3 `deepcopy-snapshot-before-untrusted-mutation`: a validation
//!   sized against an input must read a SNAPSHOT taken BEFORE any
//!   extension/hook code can rewrite the input in place — the buggy arm
//!   re-reads the live structure (the hook's rewrite legitimises itself,
//!   coverage is claimed while windows were re-truncated), the fixed arm
//!   snapshots first and detects, the ANTI-regime arm proves a shallow
//!   copy fails exactly like the bug (the nested containers stay
//!   shared), and the tamper arm snapshots AFTER the hook (the order is
//!   the law).
//! - F4 `artifact-load-replaces-whole-state`: loading an artifact
//!   installs the artifact's WHOLE state including its ABSENCES — the
//!   buggy arm merges (a heads-only artifact keeps the previous fit and
//!   reports calibrated over a state the directory never held), the
//!   fixed arm replaces (None clears), the face derives the status flag
//!   from the LOADED state (the tampered stale flag reds), and the
//!   anti-regime: a complete artifact makes replace and merge agree.
//! - F5 `lower-bound-precheck-understates-never-fabricates`: a cheap
//!   fast-path refusal gate may refuse ONLY on a proven lower bound —
//!   unproven abstains (0) and the exact measurer decides; the reported
//!   number is a measured understatement, never cap+1 (the fabricated
//!   count is the pinned negative: 13-char state refused as 60000 >
//!   50000); the walk is budget-bounded (census) and EXACT-typed (a
//!   value whose override could push the bound past truth abstains).
//!   REAL work model: byte-touched censuses, never a cost model.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// ═══════════════════════════════════════════════════════════════════════
// F1 — gate-cut-fitted-on-serving-representation
// ═══════════════════════════════════════════════════════════════════════

/// A model answer's confidence + its gold correctness.
struct Rec {
    conf: f64,
    correct: bool,
}

/// The serving-side BINNING map: `answer_confidence` is recalibrated
/// through it before anything reads it (quantize onto the bin ladder —
/// the values round DOWN onto a coarse 0.02 grid plus the 4-decimal
/// report width; a deterministic stand-in for the real map).
fn serve_rep(conf: f64) -> f64 {
    let q = (conf * 400.0).round() / 400.0; // the bin ladder: nearest 0.0025
    (q * 10000.0).round() / 10000.0 // the report width
}

/// Fit the abstention cut: the smallest cut reaching the error target at
/// the max coverage. Returns (cut, covered_count, errors_at_cut).
fn fit_cut(recs: &[Rec], rep: fn(f64) -> f64, target_err: f64) -> (f64, usize, usize) {
    // candidate cuts on the representation the gate will compare
    let mut cands: Vec<f64> = recs.iter().map(|r| rep(r.conf)).collect();
    cands.sort_by(|a, b| a.total_cmp(b));
    cands.dedup();
    let mut best = (f64::INFINITY, 0usize, usize::MAX);
    for &cut in &cands {
        let mut cov = 0usize;
        let mut err = 0usize;
        for r in recs {
            if rep(r.conf) >= cut {
                cov += 1;
                if !r.correct {
                    err += 1;
                }
            }
        }
        if cov == 0 {
            continue;
        }
        let e = err as f64 / cov as f64;
        if e <= target_err && cov > best.1 {
            best = (cut, cov, err);
        }
    }
    best
}

/// Serve: the gate compares the SERVING representation against the cut.
fn gate(recs: &[Rec], rep: fn(f64) -> f64, cut: f64) -> (usize, usize) {
    let mut cov = 0;
    let mut err = 0;
    for r in recs {
        if rep(r.conf) >= cut {
            cov += 1;
            if !r.correct {
                err += 1;
            }
        }
    }
    (cov, err)
}

/// A synthetic cohort: correct confidences spread over [0.60, 0.80),
/// wrong ones over [0.56, 0.60) — at FULL precision a cut at 0.60
/// separates them perfectly; the serving bin map (nearest 0.05) hops
/// wrongs in [0.575, 0.60) ONTO the 0.60 bin, over the cut.
fn cohort(n_correct: usize, n_wrong: usize, seed: &mut u64) -> Vec<Rec> {
    let mut recs = Vec::with_capacity(n_correct + n_wrong);
    for i in 0..n_correct {
        let t = i as f64 / n_correct as f64;
        recs.push(Rec {
            conf: 0.5550 + t * 0.245,
            correct: true,
        });
    }
    for i in 0..n_wrong {
        let t = i as f64 / n_wrong as f64;
        recs.push(Rec {
            conf: 0.55375 + t * 0.00124 + 0.000001,
            correct: false,
        });
    }
    *seed += 1;
    recs
}

#[test]
fn f1_gate_cut_fitted_on_serving_representation() {
    let mut seed = 375u64;
    let recs = cohort(400, 120, &mut seed);
    let target = 0.10;

    // FIXED: fit on the representation the gate compares.
    let (cut_fixed, _, _) = fit_cut(&recs, serve_rep, target);
    assert!(cut_fixed.is_finite(), "fixed arm must find a cut");
    let (cov_f, err_f) = gate(&recs, serve_rep, cut_fixed);
    let err_rate_fixed = err_f as f64 / cov_f as f64;
    assert!(
        err_rate_fixed <= target,
        "fixed arm must hold its target on the serving rep: {} > {}",
        err_rate_fixed,
        target
    );

    // BUGGY: fit at full precision (the identity "representation"), serve
    // on the binned one — a cut on a scale the gate never sees. At fit
    // time the cut looks perfect; on the serving rep the hopped wrongs
    // blow the error past twice the target.
    // the full-precision cut: the minimum correct confidence (at fit time
    // it separates the cohort perfectly — 0% error at full coverage of
    // corrects)
    let cut_buggy = recs
        .iter()
        .filter(|r| r.correct)
        .map(|r| r.conf)
        .fold(f64::INFINITY, f64::min);
    let (fc_cov, fc_err) = gate(&recs, |c| c, cut_buggy);
    let fit_err_rate = fc_err as f64 / fc_cov as f64;
    assert!(
        fit_err_rate == 0.0,
        "the full-precision fit must LOOK perfect at fit time: {}",
        fit_err_rate
    );
    let (cov_b, err_b) = gate(&recs, serve_rep, cut_buggy);
    assert!(cov_b > 0, "the buggy arm must admit something to measure");
    let err_rate_buggy = err_b as f64 / cov_b as f64;
    assert!(
        err_rate_buggy > target * 2.0,
        "the naive arm must blow past twice the target on the serving rep: {} (fixed {})",
        err_rate_buggy,
        err_rate_fixed
    );

    // TAMPER (the order reversed): a cut fitted ON the binned rep, served
    // on raw — its fit-time coverage was measured on a scale the gate
    // does not serve, and it does not transfer.
    let (cut_rev, cov_fit_rev, _) = fit_cut(&recs, serve_rep, target);
    let (cov_r, _) = gate(&recs, |c| c, cut_rev);
    assert!(
        cov_r != cov_fit_rev,
        "the reversed order's fit-time coverage must not transfer: fit {} vs served {}",
        cov_fit_rev,
        cov_r
    );
}

// ── the length face: measure the SERIALIZED text, never a Debug rendering ──

/// The wire encoding: JSON-style escaping (a `"` inside a string is TWO
/// chars; a zero-width space is ONE char).
fn json_len(s: &str) -> usize {
    let mut n = 2; // the quotes
    for c in s.chars() {
        n += if c == '"' { 2 } else { c.len_utf8() };
    }
    n
}

/// The proxy rendering: Debug-style (a `"` inside stays ONE char, the
/// zero-width space renders as SIX chars of escape) — the repr-vs-json
/// divergence, both directions.
fn debug_len(s: &str) -> usize {
    let mut n = 2;
    for c in s.chars() {
        if c == '"' {
            n += 1; // Debug wraps in single quotes — no escape
        } else if c == '\u{200b}' {
            n += 6; // the escape spelling
        } else {
            n += c.len_utf8();
        }
    }
    n
}

const CAP_LEN: usize = 50;

#[test]
fn f1_length_face_measure_the_serialized_text() {
    // direction 1 (over-admission): quotes DOUBLE under the wire encoding
    // — a body AT the cap under the proxy is nearly 2x over the wire cap
    let s_quotes = "\"".repeat(48);
    assert_eq!(
        debug_len(&s_quotes),
        CAP_LEN,
        "the proxy measures it AT the cap (admits)"
    );
    assert!(
        json_len(&s_quotes) > CAP_LEN,
        "proxy-passed body must exceed the wire cap: {} > {}",
        json_len(&s_quotes),
        CAP_LEN
    );
    // direction 2 (over-refusal): a zero-width space is 6 proxy chars but
    // 1 wire char — a small body refuses under the proxy
    let s_zwsp: String = "\u{200b}".repeat(9);
    assert!(debug_len(&s_zwsp) > CAP_LEN, "{}", debug_len(&s_zwsp));
    assert!(
        json_len(&s_zwsp) < CAP_LEN,
        "proxy-refused body must fit the wire cap: {} < {}",
        json_len(&s_zwsp),
        CAP_LEN
    );
    // the law: the gate measures the encoding the consumer produces
    let bodies = [s_quotes, s_zwsp, "plain".to_string()];
    for b in &bodies {
        assert_eq!(json_len(b), json_len(b), "wire len is the gate's measure");
        // an exact gate on the wire encoding never diverges from itself
    }
}
// ═══════════════════════════════════════════════════════════════════════
// F2 — reference-round-replicated-in-integer-arithmetic
// ═══════════════════════════════════════════════════════════════════════

/// The reference replica: CPython's `round(x, 4)`, exactly — the EXACT
/// binary value is rounded half-to-even at four decimals in INTEGER
/// arithmetic (no `x * 10_000.0`, whose product is itself rounded), and
/// the result is the double nearest that decimal via a single-rounding
/// (sticky-bit) conversion. ORIGINAL re-authoring (our own structure;
/// the law's shape: decompose → one half-even decision on the exact
/// rational → single-rounding conversion back).
fn round4_reference(value: f64) -> f64 {
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
        return value; // integral (or zero): exact at four decimals; ±0.0 keeps its sign
    }
    let shift = (-exponent) as u32;
    let scaled = (mantissa as u128) * 10_000; // the EXACT |value|·10^4 as scaled / 2^shift
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

/// The double nearest `quotient / 10_000` — correctly rounded for any
/// quotient < 2^67: below 2^53 the integer is exact and one IEEE division
/// is the single rounding; beyond it, divide in integers with 40 extra
/// bits and fold the remainder into a sticky bit so the conversion
/// rounds ONCE (converting first would round twice).
fn quotient_to_f64_once(quotient: u128) -> f64 {
    if quotient < (1u128 << 53) {
        return quotient as f64 / 10_000.0;
    }
    const EXTRA: u32 = 40;
    let numerator = quotient << EXTRA;
    let mut q = numerator / 10_000;
    if numerator % 10_000 != 0 {
        q |= 1;
    }
    q as f64 / (1u64 << EXTRA) as f64
}

/// The pinned negative: the naive scale-and-round — three roundings, the
/// first on a rounded product that destroys tie-adjacency.
fn round4_naive(value: f64) -> f64 {
    (value * 10_000.0).round_ties_even() / 10_000.0
}

fn f64_bits(x: f64) -> u64 {
    x.to_bits()
}

/// The generated CPython oracle (gen_oracle.py, python3 seed 375): rows
/// of (input bits, round(x, 4) output bits).
// generated by gen_oracle.py (python3 seed 375) - 151 rows
const ORACLE: [(u64, u64); 151] = [
    (0x3f7999999999999a, 0x3f79ce075f6fd220),
    (0x3fa0000000000000, 0x3f9ff2e48e8a71de),
    (0xbf7999999999999a, 0xbf79ce075f6fd220),
    (0xbfa0000000000000, 0xbf9ff2e48e8a71de),
    (0x3f0a36e2eb1c432d, 0x3f1a36e2eb1c432d),
    (0x3f23a92a30553261, 0x3f1a36e2eb1c432d),
    (0x3f799999a078d190, 0x3f79ce075f6fd220),
    (0x3f79999992ba61a3, 0x3f79652bd3c36113),
    (0x4004001a36e2eb1c, 0x4004000000000000),
    (0x4003ffe5c91d14e4, 0x4004000000000000),
    (0x3bc79ca10c924223, 0x0000000000000000),
    (0x3b90000000000000, 0x0000000000000000),
    (0x3c22725dd1d243ac, 0x0000000000000000),
    (0x3c20000000000000, 0x0000000000000000),
    (0x3c30000000000000, 0x0000000000000000),
    (0x0000000000000001, 0x0000000000000000),
    (0x0000000000000005, 0x0000000000000000),
    (0x41c0624dd2f1a9fc, 0x41c0624dd2f1a9fc),
    (0x41c0624dd2f1ad43, 0x41c0624dd2f1ad43),
    (0x41c1e1a300000000, 0x41c1e1a300000000),
    (0x41cdcd6500000000, 0x41cdcd6500000000),
    (0x41d26580b487e5c9, 0x41d26580b487e5c9),
    (0xc1c65a0bc0000000, 0xc1c65a0bc0000000),
    (0x4200000000000000, 0x4200000000000000),
    (0x426d1a94a2000000, 0x426d1a94a2000000),
    (0xc204b230cac00000, 0xc204b230cac00000),
    (0x42026580b757e69b, 0x42026580b757e69b),
    (0x0000000000000000, 0x0000000000000000),
    (0x8000000000000000, 0x8000000000000000),
    (0x4008000000000000, 0x4008000000000000),
    (0xc045000000000000, 0xc045000000000000),
    (0x4090000000000000, 0x4090000000000000),
    (0x7ff0000000000000, 0x7ff0000000000000),
    (0xfff0000000000000, 0xfff0000000000000),
    (0x7ff8000000000000, 0x7ff8000000000000),
    (0xc080a4980665a21a, 0xc080a497f62b6ae8),
    (0xc07f508b202f2ef8, 0xc07f508b43958106),
    (0xc05f473a0dd65ce0, 0xc05f4739c0ebedfa),
    (0x40814f3ae577e280, 0x40814f3afb7e9100),
    (0xc07d4f02ae128c9c, 0xc07d4f02de00d1b7),
    (0x4043cce2eb173900, 0x4043cce3bcd35a86),
    (0x408db3112afedbf6, 0x408db3113404ea4b),
    (0x406cf58f4f9a4878, 0x406cf58ef34d6a16),
    (0x4087cb1574fb8ba4, 0x4087cb15810624dd),
    (0xc06c045c92000134, 0xc06c045c91d14e3c),
    (0x407578dfb097e9f8, 0x407578dfa43fe5c9),
    (0x407a5e47d952f638, 0x407a5e47ae147ae1),
    (0x4085a5eeb753de2c, 0x4085a5eecbfb15b5),
    (0xc068e50c9b124310, 0xc068e50cb295e9e2),
    (0xc089f616a6af0d22, 0xc089f616bb98c7e3),
    (0xc054279d73be3258, 0xc054279db22d0e56),
    (0xc07920fb7cadde3a, 0xc07920fb7e90ff97),
    (0xc0888819de7a0ee0, 0xc0888819ce075f70),
    (0xc030b826177e1a00, 0xc030b8240b780347),
    (0xc08169b2973fb18c, 0xc08169b295e9e1b1),
    (0x407b8758f9e44d1c, 0x407b8758e219652c),
    (0xc07544ddec69d97a, 0xc07544de00d1b717),
    (0x404f512ff8010f40, 0x404f5130be0ded29),
    (0x408222fe34be09cc, 0x408222fe28240b78),
    (0xc06b1f715e19e868, 0xc06b1f71758e2196),
    (0xc06482be1bbcfb3c, 0xc06482be0ded288d),
    (0xc08b734696f07aa5, 0xc08b7346a7ef9db2),
    (0xc0874c4d16f2ed5c, 0xc0874c4d013a92a3),
    (0xc08686cefd559478, 0xc08686cf0d844d01),
    (0x407b788ce91e72d8, 0x407b788ce703afb8),
    (0x408a4a4f1e58c2c8, 0x408a4a4f0d844d01),
    (0xc089a61d3df69e1c, 0xc089a61d495182aa),
    (0xc06a49bb8f3d0c8c, 0xc06a49bb2fec56d6),
    (0x4082ed5b7624435a, 0x4082ed5b8bac710d),
    (0xc0843705fbad20a4, 0xc0843705f06f6944),
    (0xc07b30319c978f90, 0xc07b30318fc50481),
    (0xbff38e1bfc6b7400, 0xbff38e219652bd3c),
    (0xc088663868533e40, 0xc088663851eb851f),
    (0xc082ee607b9f9e89, 0xc082ee6075f6fd22),
    (0xc064456e6999a2bc, 0xc064456e2eb1c433),
    (0xc08e1df6073f429a, 0xc08e1df5f6fd21ff),
    (0xc08ba1128ff405a2, 0xc08ba112a3055326),
    (0x4089eded20b52e9a, 0x4089eded288ce704),
    (0x40847d862c7b50bc, 0x40847d8624dd2f1b),
    (0x4089f0b1a1ca52d8, 0x4089f0b18fc50481),
    (0x40689fefc7a588f8, 0x40689fef9db22d0e),
    (0xc080803f40a9f680, 0xc080803f487fcb92),
    (0x4080183a37c07d18, 0x4080183a29c779a7),
    (0xc00886c96041e300, 0xc00886c226809d49),
    (0x40875873af811ba2, 0x40875873b645a1cb),
    (0x408baac1951321a8, 0x408baac189374bc7),
    (0xc080488657dbc543, 0xc0804886594af4f1),
    (0xc07cbbef9b18dc9e, 0xc07cbbef9db22d0e),
    (0x4085ee085191d830, 0x4085ee086594af4f),
    (0xc062202e1f7fb2f4, 0xc062202de00d1b71),
    (0x4084ae1d1eee4320, 0x4084ae1d14e3bcd3),
    (0xc0867f61b4188a96, 0xc0867f61b089a027),
    (0x408b628805d60802, 0x408b6287fcb923a3),
    (0x4052259338726a40, 0x405225930be0ded3),
    (0xc0828698368589b4, 0xc08286982a9930be),
    (0x3fd43f351c2dbf64, 0x3fd43fe5c91d14e4),
    (0x3fd83156dc2d03ba, 0x3fd83126e978d4fe),
    (0x3fad1170d6ff2530, 0x3fad14e3bcd35a86),
    (0x3fe7ee8b68904b44, 0x3fe7eecbfb15b574),
    (0x3fd6f0514e2b96f0, 0x3fd6f0068db8bac7),
    (0x3fe30fd7a52d9b17, 0x3fe30ff972474539),
    (0x3fe1833a53a7aa11, 0x3fe1837b4a2339c1),
    (0x3fea78717f500956, 0x3fea786c226809d5),
    (0x3fc8f31fa06cdd18, 0x3fc8f27bb2fec56d),
    (0x3fead37b731d8fa6, 0x3fead35a858793de),
    (0x3fe5acce9208887c, 0x3fe5ac710cb295ea),
    (0x3fe7550bc953a7db, 0x3fe754c985f06f69),
    (0x3fe28928ff2b4201, 0x3fe288ce703afb7f),
    (0x3fce777e23f6ef98, 0x3fce76c8b4395810),
    (0x3fd12158b60691da, 0x3fd121ff2e48e8a7),
    (0x3fdacef756326964, 0x3fdacf41f212d773),
    (0x3fe1cd76974a50f3, 0x3fe1cd35a858793e),
    (0x3fd31aeee89051ac, 0x3fd31a9fbe76c8b4),
    (0x3fcfff22fb3cf13c, 0x3fd0000000000000),
    (0x3fec9f5fd50f798d, 0x3fec9fbe76c8b439),
    (0x3fee4db26d45bef4, 0x3fee4dd2f1a9fbe7),
    (0xbfcb9275bcad73b8, 0xbfcb923a29c779a7),
    (0xbfe95109d8afe316, 0xbfe950b0f27bb2ff),
    (0x3fb8580c417f7130, 0x3fb858793dd97f63),
    (0xbfdbc82330f833dc, 0xbfdbc84b5dcc63f1),
    (0xbfee7816842a8a56, 0xbfee786c226809d5),
    (0x3fe20584ee1ccda6, 0x3fe205bc01a36e2f),
    (0xbfdcfb6390196cbc, 0xbfdcfaacd9e83e42),
    (0xbfeaab4a6dd6a806, 0xbfeaab367a0f9097),
    (0x3faf2efa92fe84c0, 0x3faf2e48e8a71de7),
    (0xbfe3722625a9b8ea, 0xbfe372474538ef35),
    (0x3fe399e30e51656c, 0x3fe399999999999a),
    (0x42537fb9321a5ff8, 0x42537fb9321a5ff8),
    (0xc2438f0e5bb8f108, 0xc2438f0e5bb8f109),
    (0x425e73d733ae9ee0, 0x425e73d733ae9ee0),
    (0x425e7ba660e2bf74, 0x425e7ba660e2bf75),
    (0x4261a0cb5180ea84, 0x4261a0cb5180ea84),
    (0xc25b8b176772ce82, 0xc25b8b176772ce82),
    (0x42241e78b2d04720, 0x42241e78b2d0471e),
    (0x42637c9f074f2f58, 0x42637c9f074f2f58),
    (0xc23be2b66c444bc0, 0xc23be2b66c444bc0),
    (0xc2575f700a374bb0, 0xc2575f700a374bb0),
    (0x4247bb932151de28, 0x4247bb932151de28),
    (0xc26b65b3a98c5a3c, 0xc26b65b3a98c5a3c),
    (0x40204ced916872b0, 0x40204ce703afb7e9),
    (0xc040952a30553261, 0xc04095288ce703b0),
    (0x402fd88ce703afb8, 0x402fd89374bc6a7f),
    (0xc0283e5604189375, 0xc0283e5c91d14e3c),
    (0xc03d66c56d5cfaad, 0xc03d66c8b4395810),
    (0x402795aee631f8a1, 0x402795b573eab368),
    (0xc03e33ac710cb296, 0xc03e33afb7e90ff9),
    (0x404164346dc5d639, 0x40416436113404ea),
    (0x40372d182a9930be, 0x40372d14e3bcd35b),
    (0xc03e6ce3bcd35a86, 0xc03e6ce703afb7e9),
    (0x4039e4ac083126e9, 0x4039e4a8c154c986),
    (0xc043c303afb7e910, 0xc043c305532617c2),
];

// generated by gen_oracle.py — the reporting-round face vector (seed 9375):
// two summation orders that differ in raw bits by 1 ulp but agree at the
// pinned 4-decimal reporting width.
const FACE_VECTOR: [u64; 8] = [
    0x401917a3a256cb72,
    0x4005c26c5d8cd8e6,
    0x40227de2bab11ac4,
    0x3fff41a886b51c62,
    0x4017906c965ae732,
    0x4010ab40871de04c,
    0x4002c6647916aedc,
    0x4012daa043c030f4,
];
const FACE_SUMS: (u64, u64) = (0x4042a7d120be208c, 0x4042a7d120be208d);
const FACE_ROUNDED: (u64, u64) = (0x4042a7d21ff2e48f, 0x4042a7d21ff2e48f);

#[test]
fn f2_round4_matches_the_python_oracle_bit_for_bit() {
    for (i, &(in_bits, want_bits)) in ORACLE.iter().enumerate() {
        let x = f64::from_bits(in_bits);
        let got = round4_reference(x);
        assert_eq!(
            f64_bits(got),
            want_bits,
            "row {i}: input 0x{in_bits:016x} ({x}) — got 0x{:016x}, want 0x{want_bits:016x}",
            f64_bits(got)
        );
    }
}

#[test]
fn f2_naive_diverges_reference_agrees() {
    let mut naive_div = 0usize;
    for &(in_bits, want_bits) in ORACLE.iter() {
        let x = f64::from_bits(in_bits);
        if !x.is_finite() || x == 0.0 {
            continue;
        }
        let n = round4_naive(x);
        if f64_bits(n) != want_bits {
            naive_div += 1;
        }
    }
    assert!(
        naive_div >= 1,
        "the pinned negative must reproduce: the naive arm must diverge somewhere"
    );
    // the anti-regime half: the divergence is a RARE boundary class, not a
    // wide error — most ordinary values agree.
    let finite_rows = ORACLE
        .iter()
        .filter(|&&(b, _)| {
            let x = f64::from_bits(b);
            x.is_finite() && x != 0.0
        })
        .count();
    assert!(
        (naive_div as f64) < 0.25 * finite_rows as f64,
        "naive diverged on {naive_div}/{finite_rows} — that is not the tie class"
    );
    // the tie specimen must be among the divergent rows
    let tie = f64::from_bits(0x3f7999999999999a); // 0.00625
    assert_ne!(
        f64_bits(round4_naive(tie)),
        f64_bits(round4_reference(tie)),
        "0.00625 must be a divergent row for the naive arm"
    );
    assert_eq!(
        f64_bits(round4_reference(tie)),
        0x3f79ce075f6fd220,
        "the reference replica must round 0.00625 to 0.0063 like CPython"
    );
}

#[test]
fn f2_reporting_round_face_two_orders_one_width() {
    // the face: the vector where two summation orders differ in RAW bits
    // (1 ulp) but agree at the pinned reporting width — the width, not
    // the fold order, is the observable at the boundary.
    let v: Vec<f64> = FACE_VECTOR.iter().map(|b| f64::from_bits(*b)).collect();
    let a = v.iter().sum::<f64>();
    let b: f64 = v.iter().rev().sum();
    assert_ne!(
        f64_bits(a),
        f64_bits(b),
        "the two orders must differ in raw bits"
    );
    assert_eq!(
        f64_bits(a),
        FACE_SUMS.0,
        "order A bits must match the generator"
    );
    assert_eq!(
        f64_bits(b),
        FACE_SUMS.1,
        "order B bits must match the generator"
    );
    let ra = round4_reference(a);
    let rb = round4_reference(b);
    assert_eq!(f64_bits(ra), FACE_ROUNDED.0);
    assert_eq!(f64_bits(rb), FACE_ROUNDED.1);
    assert_eq!(
        ra, rb,
        "at the pinned reporting width the 1-ulp order drift is invisible"
    );
}
// ═══════════════════════════════════════════════════════════════════════
// F3 — deepcopy-snapshot-before-untrusted-mutation
// ═══════════════════════════════════════════════════════════════════════

/// A question whose options may be WIDENED by a hook (the untrusted
/// extension surface). The nested list sits behind Rc<RefCell<..>> so a
/// SHALLOW copy can share it (the subtlety the law hangs on).
struct Question {
    options: Rc<RefCell<Vec<String>>>,
}

impl Question {
    fn new(options: &[&str]) -> Self {
        Question {
            options: Rc::new(RefCell::new(
                options.iter().map(|s| s.to_string()).collect(),
            )),
        }
    }
    fn option_count(&self) -> usize {
        self.options.borrow().len()
    }
}

/// DEEP snapshot: the nested lists are copied — later mutation of the
/// live questions cannot move the snapshot.
fn deep_snapshot(qs: &[Question]) -> Vec<Vec<String>> {
    qs.iter().map(|q| q.options.borrow().clone()).collect()
}

/// SHALLOW snapshot: the Vec of handles is copied; the NESTED lists are
/// shared (clone of Rc clones the handle, not the data).
fn shallow_snapshot(qs: &[Question]) -> Vec<Rc<RefCell<Vec<String>>>> {
    qs.iter().map(|q| Rc::clone(&q.options)).collect()
}

/// The untrusted hook: adds options to question 0 (the widen pattern).
fn widen_hook(qs: &[Question]) {
    let mut b = qs[0].options.borrow_mut();
    b.push("EXTRA_A".to_string());
    b.push("EXTRA_B".to_string());
}

#[test]
fn f3_deepcopy_snapshot_before_untrusted_mutation() {
    let qs = vec![Question::new(&["a", "b", "c"]), Question::new(&["x", "y"])];
    let window_cap = 10;

    // FIXED: snapshot BEFORE the hook; the check reads the snapshot.
    let asked = deep_snapshot(&qs);
    let asked_counts: Vec<usize> = asked.iter().map(|v| v.len()).collect();
    widen_hook(&qs);
    let (claimed, _) = scan_and_check_with_counts(&asked_counts, &qs, window_cap);
    assert_eq!(
        claimed, 0,
        "the fixed arm must DETECT the rewrite (windows re-truncated, coverage refused)"
    );

    // BUGGY: no snapshot — `asked` IS the live structure (the aliasing),
    // so the check reads the counts the hook already widened on BOTH
    // sides: the rewrite legitimises itself and full coverage is claimed
    // while the extra options never reached any window.
    let qs2 = vec![Question::new(&["a", "b", "c"]), Question::new(&["x", "y"])];
    widen_hook(&qs2);
    let aliased: Vec<usize> = qs2.iter().map(|q| q.option_count()).collect(); // == current
    let (claimed2, _) = scan_and_check_with_counts(&aliased, &qs2, window_cap);
    assert_eq!(
        claimed2, 2,
        "the buggy arm must claim full coverage over the widened structure"
    );

    // ANTI-REGIME (the shallow-copy-also-fails subtlety): a shallow copy
    // shares the nested lists — it fails EXACTLY like the bug.
    let qs3 = vec![Question::new(&["a", "b", "c"]), Question::new(&["x", "y"])];
    let shallow = shallow_snapshot(&qs3);
    let shallow_counts: Vec<usize> = shallow.iter().map(|r| r.borrow().len()).collect();
    widen_hook(&qs3);
    // read "through the snapshot" AFTER the hook: the shared lists moved.
    let post_counts: Vec<usize> = shallow.iter().map(|r| r.borrow().len()).collect();
    assert_ne!(
        shallow_counts, post_counts,
        "the shallow snapshot must share the nested lists (both moved)"
    );
    // the check reads the counts THROUGH the shallow snapshot — the
    // shared lists moved with the hook, so the "snapshot" sees the same
    // widened state the live side reports (the aliasing again).
    let via_shallow: Vec<usize> = shallow.iter().map(|r| r.borrow().len()).collect();
    let (claimed3, _) = scan_and_check_with_counts(&via_shallow, &qs3, window_cap);
    assert_eq!(
        claimed3, 2,
        "the shallow arm must fail like the bug: the shared lists moved with the hook"
    );

    // TAMPER: snapshot AFTER the hook — the order is the law; it fails
    // exactly like the bug.
    let qs4 = vec![Question::new(&["a", "b", "c"]), Question::new(&["x", "y"])];
    let sized4: Vec<usize> = qs4.iter().map(|q| q.option_count()).collect();
    let _ = sized4;
    widen_hook(&qs4);
    let late_counts: Vec<usize> = qs4.iter().map(|q| q.option_count()).collect();
    let (claimed4, _) = scan_and_check_with_counts(&late_counts, &qs4, window_cap);
    assert_eq!(claimed4, 2, "the late snapshot must fail like the bug");
}

/// The FIXED-side check: the sized counts come from the SNAPSHOT, the
/// current counts from the live structure — a rewrite is a mismatch.
fn scan_and_check_with_counts(
    sized_against: &[usize],
    qs: &[Question],
    window_cap: usize,
) -> (usize, usize) {
    let mut current = Vec::with_capacity(qs.len());
    for q in qs {
        current.push(q.option_count());
    }
    let all_fit = current
        .iter()
        .zip(sized_against.iter())
        .all(|(c, s)| c <= s);
    if all_fit {
        let windows = sized_against.len();
        let tokens = sized_against
            .iter()
            .sum::<usize>()
            .min(window_cap * windows);
        (windows, tokens)
    } else {
        (0, 0)
    }
}
// ═══════════════════════════════════════════════════════════════════════
// F4 — artifact-load-replaces-whole-state
// ═══════════════════════════════════════════════════════════════════════

/// The artifact the directory holds: each section present or ABSENT (a
/// heads-only artifact omits the calibration — absence is content).
#[derive(Clone)]
struct Artifact {
    heads: Option<Vec<String>>,
    calibration: Option<Vec<f64>>, // the band table
    binning: Option<HashMap<String, usize>>,
}

/// The agent's live state: what a previous fit installed.
#[derive(Clone)]
struct AgentState {
    heads: Option<Vec<String>>,
    calibration: Option<Vec<f64>>,
    binning: Option<HashMap<String, usize>>,
    calibrated_flag: bool, // the stale-flag tamper arm's surface
}

impl AgentState {
    fn previously_fitted() -> Self {
        let mut binning = HashMap::new();
        binning.insert("choice".to_string(), 4);
        AgentState {
            heads: Some(vec!["old_head".to_string()]),
            calibration: Some(vec![0.1, 0.9]), // band B: never re-measured
            binning: Some(binning),
            calibrated_flag: true,
        }
    }
}

/// BUGGY: merge semantics — absent sections keep the previous fit.
fn load_merge(state: &mut AgentState, artifact: &Artifact) {
    if artifact.heads.is_some() {
        state.heads = artifact.heads.clone();
    }
    if artifact.calibration.is_some() {
        state.calibration = artifact.calibration.clone();
    }
    if artifact.binning.is_some() {
        state.binning = artifact.binning.clone();
    }
}

/// FIXED: replace semantics — the artifact is the whole state; a None
/// section CLEARS the previously-fitted one.
fn load_replace(state: &mut AgentState, artifact: &Artifact) {
    state.heads = artifact.heads.clone();
    state.calibration = artifact.calibration.clone();
    state.binning = artifact.binning.clone();
}

/// The decision: the calibration band the route rides.
fn route_band(state: &AgentState) -> &'static str {
    match &state.calibration {
        Some(bands) if !bands.is_empty() => "measured-band",
        _ => "default",
    }
}

/// The status flag the ROUTE reports — FIXED derives it from the loaded
/// state; the tampered arm keeps a stored bool.
fn calibrated_reported(state: &AgentState, derive_from_state: bool) -> bool {
    if derive_from_state {
        state.calibration.is_some()
    } else {
        state.calibrated_flag
    }
}

#[test]
fn f4_artifact_load_replaces_whole_state() {
    // the heads-only artifact: a valid artifact that never held a
    // calibration (absence is content, not corruption)
    let heads_only = Artifact {
        heads: Some(vec!["h1".to_string(), "h2".to_string()]),
        calibration: None,
        binning: None,
    };

    // FIXED: replace — the loaded state is what the directory holds.
    let mut fixed = AgentState::previously_fitted();
    load_replace(&mut fixed, &heads_only);
    assert!(
        fixed.calibration.is_none(),
        "replace must CLEAR the absent section"
    );
    assert_eq!(
        route_band(&fixed),
        "default",
        "the route must fall to the default, never ride a band the artifact never held"
    );
    assert!(
        !calibrated_reported(&fixed, true),
        "the derived flag must read calibrated:false for a state the artifact never held"
    );

    // BUGGY: merge — the previous fit survives; the route rides band B
    // the directory's heads were never measured against.
    let mut merged = AgentState::previously_fitted();
    load_merge(&mut merged, &heads_only);
    assert!(
        merged.calibration.is_some(),
        "merge keeps the stale fit (the bug)"
    );
    assert_eq!(
        route_band(&merged),
        "measured-band",
        "the merged route rides the stale band"
    );
    assert!(
        calibrated_reported(&merged, true),
        "the merge arm reports calibrated:true over a state the directory never held (the bug)"
    );
    assert_eq!(
        route_band(&merged),
        "measured-band",
        "and the route rides band B the directory's heads were never measured against"
    );

    // FACE (the tamper): a loader that REPLACES the state but keeps a
    // STORED flag reports calibrated over a state that is not — the
    // derived-from-loaded-state law removes the lie.
    let mut liar = AgentState::previously_fitted();
    load_replace(&mut liar, &heads_only);
    assert!(
        calibrated_reported(&liar, false) && !calibrated_reported(&liar, true),
        "the stale stored flag lies: true over a cleared state; the derived flag reads false"
    );

    // ANTI-REGIME: a COMPLETE artifact makes replace and merge agree.
    let complete = Artifact {
        heads: Some(vec!["n1".to_string()]),
        calibration: Some(vec![0.3, 0.7]),
        binning: Some(HashMap::new()),
    };
    let mut a = AgentState::previously_fitted();
    let mut b = AgentState::previously_fitted();
    load_replace(&mut a, &complete);
    load_merge(&mut b, &complete);
    assert_eq!(
        route_band(&a),
        route_band(&b),
        "complete artifact: both semantics agree"
    );
    assert_eq!(a.heads, b.heads);
}
// ═══════════════════════════════════════════════════════════════════════
// F5 — lower-bound-precheck-understates-never-fabricates
// ═══════════════════════════════════════════════════════════════════════

/// A JSON-ish state value. `Other` covers everything the EXACT walk
/// refuses to price (the subclass/set/cycle face): the encoder decides.
enum Value {
    Str(String),
    Dict(Vec<(String, Value)>),
    List(Vec<Value>),
    Other,
}

/// The exact measurer (the serialize-and-measure the fast path saves).
fn wire_len(v: &Value) -> usize {
    match v {
        Value::Str(s) => {
            let mut n = 2;
            for c in s.chars() {
                n += if c == '"' { 2 } else { c.len_utf8() };
            }
            n
        }
        Value::Dict(items) => {
            let mut n = 2;
            for (i, (k, val)) in items.iter().enumerate() {
                if i > 0 {
                    n += 1;
                }
                n += wire_len(&Value::Str(k.clone())) + 1 + wire_len(val);
            }
            n
        }
        Value::List(items) => {
            let mut n = 2;
            for (i, val) in items.iter().enumerate() {
                if i > 0 {
                    n += 1;
                }
                n += wire_len(val);
            }
            n
        }
        Value::Other => 0, // the encoder refuses or renders — out of the model
    }
}

const CAP_STATE: usize = 50_000;
const PROBE_BUDGET: usize = 64;

/// FIXED: the budget-bounded lower-bound walk. EXACT types only —
/// anything else abstains (0) and the encoder decides. The budget bounds
/// values INSPECTED; unproven is 0, never a guess.
fn lower_bound(v: &Value, census: &mut usize) -> usize {
    let mut total = 0usize;
    let mut budget = PROBE_BUDGET;
    let mut stack: Vec<&Value> = vec![v];
    while let Some(item) = stack.pop() {
        if budget == 0 {
            return if total > CAP_STATE { total } else { 0 }; // unproven tail: abstain unless already proven
        }
        budget -= 1;
        *census += 1;
        match item {
            Value::Str(s) => {
                total += wire_len(&Value::Str(s.clone()));
                if total > CAP_STATE {
                    return total;
                }
            }
            Value::Dict(items) => stack.extend(items.iter().map(|(_, val)| val)),
            Value::List(items) => stack.extend(items.iter()),
            // EXACT types only: an Other (the subclass/set/cycle face)
            // might make this "lower bound" exceed the truth — abstain.
            Value::Other => return 0,
        }
    }
    if total > CAP_STATE { total } else { 0 }
}

/// The FIXED gate: refuse only on the proven bound; the reported number
/// IS the bound (a measured understatement); abstain hands to the exact
/// measurer, which decides correctly every time.
fn gate_fixed(v: &Value, census: &mut usize) -> Result<usize, usize> {
    let over = lower_bound(v, census);
    if over > 0 {
        return Err(over); // proven: serializing the rest cannot change the verdict
    }
    let exact = wire_len(v);
    if exact > CAP_STATE {
        Err(exact)
    } else {
        Ok(exact)
    }
}

/// BUGGY: the fabricated-count gate — an UNSOUND proxy (counts the KEYS
/// too) that, once past the cap, reports cap + 1 no matter what the
/// state actually serializes to.
fn gate_buggy(v: &Value) -> Result<usize, usize> {
    let proxy = unsound_proxy_len(v);
    if proxy > CAP_STATE {
        return Err(CAP_STATE + 1); // the fabricated count
    }
    let exact = wire_len(v);
    if exact > CAP_STATE {
        Err(exact)
    } else {
        Ok(exact)
    }
}

/// An unsound proxy: counts keys AND values AND ignores escaping — it
/// can report MORE than the true length (the 13-char-refused-as-60000
/// class when a small value rides a pathological path).
fn unsound_proxy_len(v: &Value) -> usize {
    match v {
        Value::Str(s) => s.len(),
        Value::Dict(items) => {
            let mut n = 2;
            for (k, val) in items {
                n += k.len() + 1 + unsound_proxy_len(val);
            }
            n
        }
        Value::List(items) => {
            let mut n = 2;
            for val in items {
                n += unsound_proxy_len(val);
            }
            n
        }
        Value::Other => usize::MAX / 2, // the proxy panics/errs into a huge number
    }
}

fn big_state(entries: usize, val_len: usize) -> Value {
    Value::Dict(
        (0..entries)
            .map(|i| (format!("k{i}"), Value::Str("x".repeat(val_len))))
            .collect(),
    )
}

#[test]
fn f5_lower_bound_understates_never_fabricates() {
    // the pinned negative: a SMALL state whose UNSOUND proxy explodes —
    // the fabricated gate refuses quoting a count nothing measured
    // (Other is a 13-char-class state; the proxy reports a huge number).
    let small_with_other = Value::Dict(vec![
        ("a".to_string(), Value::Str("tiny".to_string())),
        ("b".to_string(), Value::Other),
    ]);
    assert_eq!(wire_len(&small_with_other), 17, "the true length is tiny");
    let buggy = gate_buggy(&small_with_other);
    match buggy {
        Err(reported) => {
            assert!(
                reported > wire_len(&small_with_other),
                "the fabricated count must exceed the true length: {reported} > {}",
                wire_len(&small_with_other)
            );
            assert_eq!(reported, CAP_STATE + 1, "the fabrication is cap+1");
        }
        Ok(_) => panic!("the buggy arm must refuse on the unbound proxy"),
    }

    // FIXED on the same input: the walk hits the Other, ABSTAINS (0), the
    // exact measurer decides — and it's a size question for a
    // serializable state, so it's admitted with the true length.
    let mut census = 0usize;
    match gate_fixed(&small_with_other, &mut census) {
        Ok(len) => assert_eq!(len, 17, "the abstain path must reach the exact measurer"),
        Err(_) => panic!("a tiny state must not refuse"),
    }

    // the proven-refusal arm: a genuinely oversized state — the bound is
    // proven (the walk saw enough), the reported number UNDERSTATES.
    let big = big_state(2000, 40); // ~90k chars
    let mut census = 0usize;
    let reported = match gate_fixed(&big, &mut census) {
        Err(n) => n,
        Ok(_) => panic!("an oversized state must refuse"),
    };
    let true_len = wire_len(&big);
    assert!(
        reported <= true_len,
        "the reported bound must UNDERSTATE: {reported} <= {true_len}"
    );
    assert!(
        reported > CAP_STATE,
        "the bound must be proven past the cap"
    );
    // the reported number is something MEASURED: the walk counted exactly
    // the strings it charged.
    assert!(
        reported <= CAP_STATE + 2000 * 44 + 4,
        "the bound is the walked total, not a guess"
    );

    // the budget arm: the probe's own cost is bounded — the census counts
    // values INSPECTED, bounded by PROBE_BUDGET (+ the values popped past
    // the budget check read none).
    let huge = big_state(50_000, 4);
    let mut census = 0usize;
    let _ = gate_fixed(&huge, &mut census);
    assert!(
        census <= PROBE_BUDGET + 2,
        "the probe must be budget-bounded: census {census} > {}",
        PROBE_BUDGET + 2
    );

    // REAL work model: on a huge state the probe's byte census is
    // bounded while the serialize-and-measure path touches every byte —
    // the fast path saves the exact walk when the answer is admit.
    let list_state = Value::List(
        (0..30_000)
            .map(|_| Value::Str("wwwww".to_string()))
            .collect(),
    );
    let mut census_list = 0usize;
    match gate_fixed(&list_state, &mut census_list) {
        Err(reported) => {
            assert_eq!(
                reported,
                wire_len(&list_state),
                "the abstain path reaches the exact measurer"
            );
            assert!(
                census_list <= PROBE_BUDGET + 2,
                "the refused state's probe is also budget-bounded: {census_list}"
            );
        }
        Ok(_) => panic!("the list state is over the cap"),
    }
    // and an admitted state's probe is bounded too — the budget bounds the
    // walk whatever the verdict.
    let small_list = Value::List((0..8).map(|_| Value::Str("x".repeat(5))).collect());
    let mut census_small = 0usize;
    match gate_fixed(&small_list, &mut census_small) {
        Ok(len) => assert_eq!(
            len,
            wire_len(&small_list),
            "the admit path reaches the exact measurer"
        ),
        Err(_) => panic!("the small list state is under the cap"),
    }

    // the tamper: an "Other-like" value inside a Dict must make the walk
    // ABSTAIN, never price it (the subclass face — its override could
    // push the bound past truth).
    let poisoned = Value::Dict(vec![
        ("a".to_string(), Value::Str("x".repeat(40_000))),
        ("b".to_string(), Value::Other),
        ("c".to_string(), Value::Str("x".repeat(40_000))),
    ]);
    let mut census = 0usize;
    let out = lower_bound(&poisoned, &mut census);
    assert_eq!(out, 0, "the poisoned walk must abstain on Other");
    // and the gate still decides CORRECTLY through the exact measurer:
    assert!(wire_len(&poisoned) > CAP_STATE);
    match gate_fixed(&poisoned, &mut census) {
        Err(n) => assert_eq!(
            n,
            wire_len(&poisoned),
            "the exact measurer's refusal is exact"
        ),
        Ok(_) => panic!("an oversized poisoned state must still refuse — via the exact arm"),
    }

    // the anti-regime: small states — the gate admits with the exact
    // length; the fixed and buggy arms agree on the admit side.
    let small = Value::Dict(vec![("k".to_string(), Value::Str("v".to_string()))]);
    let mut census = 0usize;
    let f = gate_fixed(&small, &mut census);
    let b = gate_buggy(&small);
    assert_eq!(
        f, b,
        "on ordinary states both arms agree (the fast path is invisible)"
    );
    assert!(matches!(f, Ok(_)));
}
